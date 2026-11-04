// ---- round 3: the invisible brushes in the page (r_showBrushes), the multiple selection, linked caulk and the
// nearby texture
//
// The brush view draws the faces the game's r_showBrushes draws, sorted and coloured by the game's own code
// (api/brushes: clip, trigger, caulk, nodraw..., and the water, lava and slime volumes), each kind on or off. A
// click on a caulk or nodraw face it draws selects that face. Shift+click (or Ctrl+click) adds a surface or a face
// to the selection or takes it out; a shader then applies to all of them in one step to undo, and so does Hide.
// Linked caulk asks the game for every face joined to the selected one that players see (api/link); the nearby
// texture asks it which shader the drawn surfaces round the selected faces use (api/suggest).

// on, the faces read (for map), a read under way, the kinds shown (chosen: by the viewer or kept), the meshes and the
// faces a click selects
const sjkBrush = { on: false, data: null, map: '', loading: false, shown: new Set(), chosen: false, meshes: [], picks: [] };
const SJK_BRUSH_KEY = 'sjkMapEditBrushes';

// the view on or off, and the kinds shown, kept in this browser
function sjkBrushSave() {
	try { localStorage.setItem(SJK_BRUSH_KEY, JSON.stringify({ on: sjkBrush.on, shown: sjkBrush.chosen ? [...sjkBrush.shown] : null })); } catch (e) { /* not kept */ }
}
function sjkBrushRestore() {
	try {
		const kept = JSON.parse(localStorage.getItem(SJK_BRUSH_KEY) || 'null');
		if (kept && Array.isArray(kept.shown)) { sjkBrush.shown = new Set(kept.shown); sjkBrush.chosen = true; }
		if (kept && kept.on) sjkBrushToggle(true);
	} catch (e) { /* the defaults */ }
}

function sjkBrushToggle(on) {
	sjkBrush.on = on === undefined ? !sjkBrush.on : on;
	$('btnBrushes').classList.toggle('on', sjkBrush.on);
	$('brushCard').hidden = !sjkBrush.on;
	// on again after a failed read: read again
	if (sjkBrush.on && !sjkBrush.data && !sjkBrush.loading) sjkBrush.map = '';
	sjkBrushSave();
	sjkRenderBrushCard();
}

// the faces of the map loaded, once for it (a failure is not tried again until the view is turned on again)
async function sjkBrushLoad() {
	if (sjkBrush.loading || !loadedMap) return;
	const map = sjkBrush.map = loadedMap;
	sjkBrush.loading = true;
	sjkBrushBuild(null);
	sjkText('brushCount', 'Reading the brushes...');
	try {
		const res = await fetch('api/brushes');
		const data = await res.json();
		if (data.ok === false || !data.kinds) throw new Error(data.error || 'HTTP ' + res.status);
		// another map came meanwhile: read for it
		if (loadedMap === map) sjkBrushBuild(data); else sjkBrush.map = '';
	} catch (e) {
		toast('The brushes could not be read: ' + e.message, false);
	}
	sjkBrush.loading = false;
	sjkRenderBrushCard();
}

// the faces of each kind as triangles and outline segments, and the faces a click may select with their planes
function sjkBrushBuild(data) {
	for (const m of sjkBrush.meshes) if (m) { sjkGeoFree(m.fill); sjkGeoFree(m.lines); }
	Object.assign(sjkBrush, { data: null, meshes: [], picks: [] });
	if (!data) return;
	const tris = data.kinds.map(() => []), lines = data.kinds.map(() => []), picks = [];
	for (const [kind, brush, side, flat, normal] of data.faces) {
		const n = flat.length / 3, p = i => [flat[i * 3], flat[i * 3 + 1], flat[i * 3 + 2]];
		if (n < 3) continue;
		for (let i = 2; i < n; i++) tris[kind].push(...p(0), ...p(i - 1), ...p(i));
		for (let i = 0; i < n; i++) lines[kind].push(...p(i), ...p((i + 1) % n));
		// a face a click may select, with its plane (the normal out of its brush: its front)
		if (data.kinds[kind].selects && brush >= 0) {
			const a = p(0);
			picks.push({ kind, brush, side, flat, normal, d: normal[0] * a[0] + normal[1] * a[1] + normal[2] * a[2] });
		}
	}
	sjkBrush.data = data;
	sjkBrush.meshes = data.kinds.map((k, i) => tris[i].length ? { fill: sjkGeo(tris[i], gl.TRIANGLES), lines: sjkGeo(lines[i], gl.LINES) } : null);
	sjkBrush.picks = picks;
	// the first time: every kind the map has (r_showBrushes 2), as caulk is what the editor is for; not the sky, which
	// would cover the view (a click on the sky selects its side without it)
	if (!sjkBrush.chosen) { sjkBrush.shown = new Set(data.kinds.filter(k => k.count && k.name !== 'sky').map(k => k.name)); sjkBrush.chosen = true; }
}

// over the map, after it: each kind's faces see-through in its colour, their outlines brighter; hidden behind what
// is in front of them, as in the game
function sjkDrawBrushes() {
	if (!sjkBrush.on) return;
	if (sjkBrush.map !== loadedMap) sjkBrushLoad();
	if (!sjkBrush.data) return;
	gl.useProgram(sjkOverlay.p);
	gl.uniformMatrix4fv(sjkOverlay.u.uMvp, false, viewProj);
	gl.uniformMatrix4fv(sjkOverlay.u.uModel, false, SJK_IDENTITY);
	gl.uniform1f(sjkOverlay.u.uBias, 0);
	gl.enable(gl.BLEND);
	gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
	gl.disable(gl.CULL_FACE);
	gl.depthMask(false);
	gl.depthFunc(gl.LEQUAL);
	gl.enable(gl.DEPTH_TEST);
	gl.enable(gl.POLYGON_OFFSET_FILL);
	gl.polygonOffset(-1, -2);
	sjkBrush.data.kinds.forEach((k, i) => {
		const m = sjkBrush.meshes[i];
		if (!m || !sjkBrush.shown.has(k.name)) return;
		gl.uniform4fv(sjkOverlay.u.uColor, [k.colour[0], k.colour[1], k.colour[2], 0.3]);
		gl.bindVertexArray(m.fill.vao);
		gl.drawArrays(gl.TRIANGLES, 0, m.fill.count);
	});
	gl.disable(gl.POLYGON_OFFSET_FILL);
	sjkBrush.data.kinds.forEach((k, i) => {
		const m = sjkBrush.meshes[i];
		if (!m || !sjkBrush.shown.has(k.name)) return;
		sjkOverlayDraw(m.lines, SJK_IDENTITY, [k.colour[0], k.colour[1], k.colour[2], 0.85], 0.0003);
	});
	gl.depthMask(true);
	gl.depthFunc(gl.LESS);
	gl.disable(gl.BLEND);
	gl.bindVertexArray(null);
}

// the caulk, nodraw or sky face of the brush view a click's ray meets first, from its front
function sjkBrushPick(ray) {
	if (!sjkBrush.on || !sjkBrush.data) return null;
	const kinds = sjkBrush.data.kinds, e = ray.eye, d = ray.dir;
	let best = null, bestT = Infinity;
	for (const f of sjkBrush.picks) {
		if (!sjkBrush.shown.has(kinds[f.kind].name)) continue;
		const along = f.normal[0] * d[0] + f.normal[1] * d[1] + f.normal[2] * d[2];
		if (along > -1e-6) continue;
		const t = (f.d - (f.normal[0] * e[0] + f.normal[1] * e[1] + f.normal[2] * e[2])) / along;
		if (t < 0 || t >= bestT) continue;
		const hit = [e[0] + d[0] * t, e[1] + d[1] * t, e[2] + d[2] * t], fl = f.flat, n = fl.length / 3;
		// inside the convex polygon: on the same side of every edge, whichever way its corners turn
		let plus = false, minus = false;
		for (let i = 0; i < n; i++) {
			const j = (i + 1) % n, a = [fl[i * 3], fl[i * 3 + 1], fl[i * 3 + 2]], b = [fl[j * 3], fl[j * 3 + 1], fl[j * 3 + 2]];
			const ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]], ah = [hit[0] - a[0], hit[1] - a[1], hit[2] - a[2]];
			const c = [ab[1] * ah[2] - ab[2] * ah[1], ab[2] * ah[0] - ab[0] * ah[2], ab[0] * ah[1] - ab[1] * ah[0]];
			const side = c[0] * f.normal[0] + c[1] * f.normal[1] + c[2] * f.normal[2];
			if (side > 0.01) plus = true;
			if (side < -0.01) minus = true;
		}
		const inside = !(plus && minus);
		if (inside) { best = f; bestT = t; }
	}
	return best && { brush: best.brush, side: best.side, t: bestT };
}

function sjkRenderBrushCard() {
	const data = sjkBrush.data, list = $('brushKinds');
	if (!data) { sjkHtml(list, ''); return; }
	const total = data.faces.length, shown = data.kinds.filter(k => sjkBrush.shown.has(k.name)).reduce((n, k) => n + k.count, 0);
	sjkText('brushCount', shown + ' of ' + total + ' faces shown');
	sjkHtml(list, data.kinds.filter(k => k.count).map(k => {
		const c = k.colour.map(v => Math.round(v * 255)).join(',');
		return '<label title="' + escapeHtml(k.label) + (k.selects ? ' (a click selects it)' : '') + '"><input type="checkbox" data-kind="' + k.name + '"' +
			(sjkBrush.shown.has(k.name) ? ' checked' : '') + '><i style="background:rgb(' + c + ')"></i>' + k.name + ' <span>' + k.count + '</span></label>';
	}).join(''));
	for (const box of list.querySelectorAll('input')) box.checked = sjkBrush.shown.has(box.dataset.kind);
}
$('btnBrushes').addEventListener('click', () => sjkBrushToggle());
$('brushKinds').addEventListener('change', e => {
	const box = e.target.closest('input');
	if (!box) return;
	if (box.checked) sjkBrush.shown.add(box.dataset.kind); else sjkBrush.shown.delete(box.dataset.kind);
	sjkBrushSave();
	sjkRenderBrushCard();
});
for (const b of document.querySelectorAll('#brushSets button')) b.addEventListener('click', () => {
	const kinds = sjkBrush.data ? sjkBrush.data.kinds : [];
	const set = b.dataset.set;
	sjkBrush.shown = new Set(kinds.filter(k => set === 'all' || (set === 'play' && k.mode === 1) || (set === 'caulk' && k.selects && k.name !== 'sky')).map(k => k.name));

	sjkBrushSave();
	sjkRenderBrushCard();
});

// ---- the multiple selection: what the page adds to the game's own selection

// what a selection of the game is, as a member of the set: a surface or a hidden face, else nothing
function sjkItemOf(sel) {
	const t = sel && sel.target;
	if (!t) return null;
	if (sel.kind === 'surface') return { key: 's' + t.surface, kind: 'surface', surface: t.surface, shader: t.shader };
	if (sel.kind === 'face') return { key: 'f' + t.brush + '/' + t.side, kind: 'face', brush: t.brush, side: t.side, shader: t.shader, points: t.points || [], sky: !!t.sky };
	// placed models join a set for the gizmo (gizmo.js), which moves them together
	if (sel.kind === 'placed' && t.placed !== null && t.placed !== undefined) return { key: 'p' + t.placed, kind: 'placed', placed: t.placed };
	return null;
}
const sjkSelKey = sel => sel ? sel.kind + JSON.stringify([sel.target && sel.target.surface, sel.target && sel.target.brush, sel.target && sel.target.side, sel.target && sel.target.entity, sel.target && sel.target.placed]) : '';
const sjkSetFaces = () => sjk.multi.filter(m => m.kind === 'face').map(m => [m.brush, m.side]);
const sjkSetSurfaces = () => sjk.multi.filter(m => m.kind === 'surface').map(m => m.surface);
const sjkMany = () => sjk.multi.length > 1;

// a state from the game: a selection made elsewhere (in the game, by an edit) starts the set again
function sjkSelectionSeen(s) {
	const key = sjkSelKey(s.selection);
	if (key === sjk.selKey) return;
	sjk.selKey = key;
	if (sjk.ownSelect) return;
	const item = sjkItemOf(s.selection);
	sjk.multi = item ? [item] : [];
	sjkMultiChanged();
}

// a click on the map: select what it meets (the brush view's face first); with Shift or Ctrl, add it to the set or
// take it out
async function sjkSelectClick(ray, adding) {
	const body = Object.assign({ op: 'select' }, ray);
	const face = sjkBrushPick(ray);
	if (face) body.face = [face.brush, face.side];
	const before = sjk.multi.length ? null : sjkItemOf(sjk.state && sjk.state.selection);
	sjk.ownSelect = true;
	let answer;
	try { answer = await sjkEdit(body); } finally { sjk.ownSelect = false; }
	const s = sjk.state, item = sjkItemOf(s && s.selection);
	sjk.selKey = sjkSelKey(s && s.selection);
	if (!adding) sjk.multi = item ? [item] : [];
	else if (!answer.ok || !item) toast(answer.ok ? 'Only surfaces, hidden faces and placed models join a multiple selection' : answer.error || 'Nothing there', false);
	else {
		if (before && before.key !== item.key) sjk.multi = [before];
		const at = sjk.multi.findIndex(m => m.key === item.key);
		if (at >= 0) sjk.multi.splice(at, 1); else sjk.multi.push(item);
		toast(sjk.multi.length + ' selected');
	}
	sjkMultiChanged();
}

// Esc: the set and the game's selection dropped
function sjkClearSelection() {
	if (!sjk.multi.length && !(sjk.state && sjk.state.selection)) return false;
	sjk.multi = [];
	sjk.linkNote = '';
	sjkEdit({ op: 'clear' });
	sjkMultiChanged();
	return true;
}

// a shader on the selection: on a set, every face drawn with it and every surface's shader remapped, one step
function sjkApplyShader(shader) {
	if (sjkMany()) return sjkEdit({ op: 'apply_many', faces: sjkSetFaces(), surfaces: sjkSetSurfaces(), shader });
	return sjkEdit({ op: 'remap', shader });
}
function sjkHideSelection() {
	if (sjkMany()) return sjkEdit({ op: 'hide_many', faces: sjkSetFaces(), surfaces: sjkSetSurfaces() });
	return sjkEdit({ op: 'hide' });
}

// the set drawn as the selection is: its faces' polygons and its surfaces' triangles
function sjkMultiGeo() {
	const tri = [], line = [];
	for (const m of sjk.multi) {
		if (m.kind !== 'face' || !m.points || m.points.length < 3) continue;
		const p = m.points;
		for (let i = 2; i < p.length; i++) tri.push(...p[0], ...p[i - 1], ...p[i]);
		for (let i = 0; i < p.length; i++) line.push(...p[i], ...p[(i + 1) % p.length]);
	}
	const fills = [sjkGeo(tri, gl.TRIANGLES)], outlines = [sjkGeo(line, gl.LINES)];
	const surfaces = new Set(sjkSetSurfaces());
	if (surfaces.size) { const g = sjkSurfaceGeo(surfaces); fills.push(g.fill); outlines.push(g.lines); }
	return { fills: fills.filter(Boolean), outlines: outlines.filter(Boolean) };
}

// ---- linked caulk and the nearby texture

// the face links and fills start from: the selected one, else the set's first
function sjkAnchorFace() {
	const sel = sjk.state && sjk.state.selection, item = sjkItemOf(sel);
	if (item && item.kind === 'face') return item;
	return sjk.multi.find(m => m.kind === 'face') || null;
}
// the faces a fill draws: the set's, or the selected face
function sjkFillFaces() {
	const faces = sjk.multi.filter(m => m.kind === 'face');
	if (faces.length) return faces;
	const item = sjkItemOf(sjk.state && sjk.state.selection);
	return item && item.kind === 'face' ? [item] : [];
}

async function sjkLink(grow) {
	const a = sjkAnchorFace();
	if (!a) return;
	sjk.busy++;
	sjkRender();
	try {
		const res = await fetch('api/link?brush=' + a.brush + '&side=' + a.side + '&grow=' + grow);
		const j = await res.json();
		if (j.ok === false || !j.faces) throw new Error(j.error || 'HTTP ' + res.status);
		sjk.multi = j.faces.map(f => ({ key: 'f' + f.brush + '/' + f.side, kind: 'face', brush: f.brush, side: f.side, shader: f.shader, points: f.points, sky: !!f.sky }));
		sjk.linkNote = j.faces.length + (j.faces.length === 1 ? ' face' : ' faces') + (grow === 'plane' ? ' on its plane' : ' joined') +
			(j.buried ? ', ' + j.buried + ' unseen left out' : '') + (j.capped ? '; stopped at ' + j.limit + ', link again from a face at the edge' : '');
		toast(sjk.linkNote, !j.capped);
	} catch (e) {
		sjk.linkNote = 'Not linked: ' + e.message;
	}
	sjk.busy--;
	sjkMultiChanged();
}

let sjkSuggestTimer = 0;
function sjkMultiChanged() {
	sjkRender();
	clearTimeout(sjkSuggestTimer);
	sjkSuggestTimer = setTimeout(sjkSuggest, 150);
}

// the shaders round the faces a fill would draw, asked again when they change
async function sjkSuggest() {
	const faces = sjkFillFaces(), key = faces.map(f => f.key).join(',');
	if (sjk.suggest && sjk.suggest.key === key) return;
	sjk.suggest = { key, candidates: null, chosen: '' };
	if (!faces.length) { sjkRender(); return; }
	sjkRender();
	try {
		const res = await fetch('api/suggest', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ faces: faces.map(f => [f.brush, f.side]) }) });
		const j = await res.json();
		if (!sjk.suggest || sjk.suggest.key !== key) return;
		if (j.ok === false || !j.candidates) throw new Error(j.error || 'HTTP ' + res.status);
		sjk.suggest.candidates = j.candidates;
		sjk.suggest.chosen = j.candidates.length ? j.candidates[0].shader : '';
	} catch (e) {
		if (sjk.suggest && sjk.suggest.key === key) Object.assign(sjk.suggest, { candidates: [], error: e.message });
	}
	sjkRender();
}

// the panel's part of all this: the set's count, the link buttons, the suggestion and Fill
function sjkRenderMulti() {
	const many = sjkMany(), faces = sjk.multi.filter(m => m.kind === 'face').length, surfaces = sjk.multi.length - faces;
	$('multiBox').hidden = !many;
	sjkText('multiCount', many ? sjk.multi.length + ' selected (' + faces + (faces === 1 ? ' face, ' : ' faces, ') + surfaces + (surfaces === 1 ? ' surface)' : ' surfaces)') : '');
	sjkText('multiHint', surfaces && many ? 'A remap affects every surface with this shader, here or anywhere on the map.' : 'Shift+click adds or takes out; Esc clears.');
	if (many) sjkText('selKind', 'Multiple selection');
	$('btnHide').textContent = many ? 'Hide ' + sjk.multi.length : $('btnHide').textContent;
	if (many) $('btnHide').disabled = false;
	const sky = sjkSkySel();
	$('btnRemap').textContent = many ? 'Apply to ' + sjk.multi.length : sky ? 'Cover side' : 'Remap';
	$('btnUnremap').textContent = many ? 'Take back from ' + sjk.multi.length : sky ? 'Take back' : 'Remove remap';
	if (many) { $('btnRemap').disabled = !sjkRemapTarget(); $('btnUnremap').disabled = false; $('btnHideGroup').disabled = true; $('btnCopy').disabled = true; }
	const anchor = sjkAnchorFace();
	$('linkBox').hidden = !anchor;
	sjkText('linkHead', anchor && anchor.sky ? 'Linked sky' : 'Linked caulk');
	sjkText('linkNote', sjk.linkNote || 'From the face: every face joined to it that players see.');
	const fill = sjkFillFaces(), sug = sjk.suggest;
	$('fillBox').hidden = !fill.length;
	const box = $('fillTiles');
	if (!fill.length) { sjkHtml(box, ''); return; }
	if (!sug || !sug.candidates) {
		sjkHtml(box, '<div class="hint">Looking round ' + fill.length + (fill.length === 1 ? ' face...' : ' faces...') + '</div>');
		$('btnFill').disabled = true;
		sjkText('btnFill', 'Fill');
		return;
	}
	if (!sug.candidates.length) {
		sjkHtml(box, '<div class="hint">' + (sug.error ? escapeHtml(sug.error) : 'No drawn surface touches ' + (fill.length === 1 ? 'this face' : 'these faces') + ': choose a shader below.') + '</div>');
		$('btnFill').disabled = true;
		sjkText('btnFill', 'Fill');
		return;
	}
	sjkHtml(box, sug.candidates.map((c, i) => '<div class="tile' + (i === 0 ? ' best' : '') + (c.shader === sug.chosen ? ' sel' : '') + '" data-name="' + escapeHtml(c.shader) + '" title="' + escapeHtml(c.shader) + '"><i class="thumb" data-thumb="' +
		escapeHtml(c.shader) + '"></i><span>' + escapeHtml(c.shader.slice(c.shader.lastIndexOf('/') + 1)) + '</span><b>' + Math.round(c.share * 100) + '%</b></div>').join(''));
	if (box.dataset.watched !== box.dataset.html) { box.dataset.watched = box.dataset.html; thumbWatch(box); }
	$('btnFill').disabled = !sug.chosen;
	sjkText('btnFill', 'Fill ' + (fill.length === 1 ? 'it' : fill.length) + ' with ' + sug.chosen.slice(sug.chosen.lastIndexOf('/') + 1));
	$('btnFill').title = 'Draw ' + (fill.length === 1 ? 'the face' : 'the ' + fill.length + ' faces') + ' with ' + sug.chosen + ': one step to undo';
}

$('btnLinkPlane').addEventListener('click', () => sjkLink('plane'));
$('btnLinkSurface').addEventListener('click', () => sjkLink('surface'));
$('btnMultiClear').addEventListener('click', () => sjkClearSelection());
$('fillTiles').addEventListener('click', e => {
	const tile = e.target.closest('.tile');
	if (!tile || !sjk.suggest || String(getSelection())) return;
	sjk.suggest.chosen = tile.dataset.name;
	sjkRender();
});
$('btnFill').addEventListener('click', () => {
	const faces = sjkFillFaces(), shader = sjk.suggest && sjk.suggest.chosen;
	if (faces.length && shader) sjkEdit({ op: 'apply_many', faces: faces.map(f => [f.brush, f.side]), shader });
});
