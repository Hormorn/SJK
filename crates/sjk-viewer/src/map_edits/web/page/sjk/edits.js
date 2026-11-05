// ---- SJK's map edits: what the game says (api/edits), drawn on the page's map, and the edits asked of it
//
// The game is the editor: the page asks it for its state about once a second, and every edit is a POST that the
// game runs as its own editor command (api/edit). So the page never keeps edits of its own: what it draws is
// what the game draws, hidden surfaces and props, remaps, placed models and shown faces.

const SJK_POLL_MSEC = 700;
// the last state, the game's lists (models to place, shaders a remap may draw), and the page's choices
const sjk = { state: null, error: '', lists: null, listsFor: '', world: null, keys: {}, placed: [], faces: [], things: [], viewTaken: '', chosenModel: '', chosenShader: '', chosenEdit: null,
	busy: 0, recent: null, chosenThing: null, placeThing: null, clip: null,
	// the multiple selection (brushes.js): its members, the game's selection it last saw, a click of its own under
	// way, the nearby shaders of its faces and the last link's note
	multi: [], selKey: '', ownSelect: false, suggest: null, linkNote: '' };

async function sjkPoll() {
	try {
		const res = await fetch('api/edits');
		if (!res.ok) throw new Error(res.status === 403 ? 'this page has an old token: run mapedit_web again' : 'HTTP ' + res.status);
		const s = await res.json();
		if (s.ok === false) throw new Error(s.error);
		sjk.error = '';
		sjkTake(s);
	} catch (e) {
		sjk.error = e.message && e.message !== 'Failed to fetch' ? e.message : 'SJK is not answering: the editor is off, or the game was closed';
	}
	sjkRender();
	setTimeout(sjkPoll, SJK_POLL_MSEC);
}

// A new state: the map it is on, then what its edits draw.
function sjkTake(s) {
	sjk.state = s;
	state = { map: s.map, gametype: 0 };
	if (s.map && s.map !== loadedMap) loadMap(s.map);
	if (s.map && sjk.listsFor !== s.map) sjkLoadLists(s.map);
	sjkApply();
	sjkSelectionSeen(s);
}

// Once the map is there: the camera where the player is (the first time for a map), and the edits drawn.
function sjkApply() {
	const s = sjk.state;
	if (!s || !world || loadedMap !== s.map) return;
	if (sjk.world !== world) {
		sjk.world = world;
		sjk.keys = {};
		// loadMap puts the camera above the map (resetCamera): this page always flies, from where it was, or for a
		// map seen the first time from the player's eyes, or from above its middle without them (in a menu)
		if (sjk.viewTaken !== s.map) {
			sjk.viewTaken = s.map;
			const b = world.bounds;
			if (s.view) sjkGoTo(s.view);
			else sjkGoTo({ eye: [(b.mins[0] + b.maxs[0]) / 2, (b.mins[1] + b.maxs[1]) / 2, b.maxs[2]], yaw: 45, pitch: 60 });
		}
		cam.fly = true;
	}
	const changed = (name, value) => {
		const key = JSON.stringify(value);
		if (sjk.keys[name] === key) return false;
		sjk.keys[name] = key;
		return true;
	};
	if (changed('surfaces', s.hidden_surfaces)) sjkHideSurfaces(new Set(s.hidden_surfaces));
	if (changed('props', s.hidden_props)) {
		sjkHiddenProps.clear();
		for (const n of s.hidden_props) sjkHiddenProps.add(n);
		// the list of the static models is made again (mapEntsFor)
		if (mapEntsNow) mapEntsNow.gametype = -1;
	}
	if (changed('remaps', s.remaps) || remap.seen !== world) {
		remap.list = s.remaps.map(([old, to]) => ({ old, new: to, own: true }));
		remapApply();
	}
	sjk.placed = s.placed.map(p => ({ id: p.id, model: p.model, place: sjkPlace(p.origin, p.angles, p.scale) }));
	if (changed('faces', s.faces)) sjkBuildFaces(s.faces);
	// the boomboxes, mystery boxes and toys: each part where the game draws it
	if (changed('things', s.things)) {
		sjk.things = (s.things ? s.things.list : []).map(t => Object.assign({}, t, {
			drawn: t.parts.map(p => ({ model: p.model, place: sjkPlace(p.origin, p.angles, [p.scale, p.scale, p.scale]) }))
		}));
	}
}

// where a placed model is drawn: the axes of its angles, each scaled by its own number (as staticList does)
function sjkPlace(origin, angles, scale) {
	const m = anglesMatrix(origin, angles, 1);
	for (let a = 0; a < 3; a++) for (let c = 0; c < 3; c++) m[a * 4 + c] *= scale[a];
	return m;
}

// Surfaces a map edit hides draw nothing: their triangles in the batch's list get every corner on the first
// vertex, as the game does with its own index buffer. A patch keeps that (updateGridLod skips it); one shown
// again gets its triangles back and is made to choose its detail anew.
function sjkHideSurfaces(hidden) {
	const batches = new Set([...world.batches, ...world.skyBatches]);
	gl.bindVertexArray(null);
	for (const b of batches) {
		for (const [n, first, count, base] of b.ranges || []) {
			const was = sjkHidden.has(n), now = hidden.has(n);
			if (was === now) continue;
			gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, b.idxBuf);
			gl.bufferSubData(gl.ELEMENT_ARRAY_BUFFER, first * 4, now ? new Uint32Array(count).fill(base) : b.tri.subarray(first, first + count));
			if (!now) for (const g of world.grids) if (g.surface === n) g.low = Infinity;
		}
	}
	sjkHidden.clear();
	for (const n of hidden) sjkHidden.add(n);
}

// A shown face (a caulk or nodraw side drawn with a shader): the polygon and its texture coordinates as the game
// makes them, drawn as a model of one mesh.
function sjkBuildFaces(faces) {
	sjk.faces = faces.map(f => {
		const n = f.points.length, pos = new Float32Array(f.points.flat()), uv = new Float32Array(f.uv.flat());
		const norm = new Float32Array(n * 3);
		for (let i = 0; i < n; i++) norm.set(f.normal, i * 3);
		const idx = [];
		for (let i = 2; i < n; i++) idx.push(0, i - 1, i, 0, i, i - 1);
		const face = { id: f.id, model: null };
		mainTexture(f.shader).then(tex => {
			face.model = { meshes: [{ vao: modelVao(pos, norm, uv, new Uint32Array(idx)), count: idx.length, tex, shaderName: f.shader, mat: null }], tags: {} };
		});
		return face;
	});
}

const SJK_IDENTITY = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
// the placed models and the shown faces, after the map
function sjkDrawEdits() {
	for (const p of sjk.placed) {
		const model = staticNow(p.model);
		if (model) drawStatic(model, p.place);
	}
	for (const f of sjk.faces) if (f.model) drawStatic(f.model, SJK_IDENTITY);
	for (const t of sjk.things) for (const p of t.drawn) {
		const model = staticNow(p.model);
		if (model) drawStatic(model, p.place);
	}
}

// ---- the Recent rows (api/recent): the game keeps them, in a file of its own, so they outlive the page and its port

async function sjkLoadRecent() {
	try {
		const res = await fetch('api/recent');
		const recent = await res.json();
		if (recent.ok === false) throw new Error(recent.error);
		sjk.recent = recent;
	} catch (e) { /* the next edit tries again */ }
	sjkRenderRecent();
}

async function sjkSetRecent(change) {
	try {
		const res = await fetch('api/recent', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(change) });
		const recent = await res.json();
		if (recent.ok === false) throw new Error(recent.error);
		sjk.recent = recent;
	} catch (e) {
		toast('The game did not keep that: ' + e.message, false);
	}
	sjkRenderRecent();
}

// the camera at a view the game gives: an eye, a yaw and a pitch
function sjkGoTo(v) {
	cam.fly = true;
	cam.follow = -1;
	cam.pos = v.eye.slice();
	cam.yaw = v.yaw;
	cam.pitch = v.pitch;
}

async function sjkLoadLists(map) {
	sjk.listsFor = map;
	try {
		const res = await fetch('api/lists');
		const lists = await res.json();
		if (lists.ok === false) throw new Error(lists.error);
		sjk.lists = lists;
		sjkRenderLists();
	} catch (e) {
		sjk.listsFor = '';
	}
}

// Ask the game for an edit. Its answer shows at the panel's foot, and the state is read again at once.
async function sjkEdit(body) {
	sjk.busy++;
	sjkRender();
	let answer;
	try {
		const res = await fetch('api/edit', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });
		answer = res.headers.get('content-type') === 'application/json' ? await res.json() : { ok: false, error: await res.text() };
	} catch (e) {
		answer = { ok: false, error: 'SJK did not answer: ' + e.message };
	}
	sjk.busy--;
	sjk.answer = answer.ok ? { text: (answer.lines || []).join(' '), error: false } : { text: answer.error || 'refused', error: true };
	sjk.answerAt = performance.now();
	try {
		const res = await fetch('api/edits');
		if (res.ok) sjkTake(await res.json());
	} catch (e) { /* the next poll */ }
	sjkRender();
	// a remap, a shown face or a placed model goes first in the Recent rows
	if (answer.ok && (body.op === 'remap' || body.op === 'sky' || body.op === 'place' || body.op === 'apply_many')) sjkLoadRecent();
	return answer;
}

// The ray of a click on the map, from the camera, for the game to select or place along.
function sjkRay(x, y) {
	const ray = screenRay(x, y);
	return { eye: eye.slice(), dir: ray.dir, forward: camDir() };
}
