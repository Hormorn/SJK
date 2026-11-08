// ---- the Lights layer: SJK's added lights and the map's own real-time lamps
//
// Sol, 08/11/2026: "for mapedit_web I want to see lights so I can move them, delete them or add new ones". Added
// lights (map edits of kind `light`) are drawn by the game as dynamic lights, for every SJK player on every preset:
// bulbs in their colour here, chosen by a click, moved by the gizmo (one `set_light` at the end of a drag), their
// colour, reach and strength set on the Lights page. The map's lamps are the real-time lighting's (Experimental
// only), one marker per fixture (a surface, a surface of a prop's model, or a steady effect; never one per patch):
// a lamp edit turns one off or moves it, and the game applies it when the map's world is built again (its next
// load, or vid_restart force), so the page says which lamps wait for that.

const sjkLights = {
	on: false, lamps: null, lampsFor: '', loading: false, chosen: null, placing: false, preview: null,
	// what a new light is
	color: [1, 0.85, 0.65], radius: 320, intensity: 1
};
const SJK_LIGHT_PICK = 14;

// the page's own choices, kept by this browser
function sjkLightsSave() {
	try { localStorage.setItem('sjkMapEditLights', JSON.stringify({ on: sjkLights.on })); } catch (e) { /* not kept */ }
}
function sjkLightsRestore() {
	try {
		const kept = JSON.parse(localStorage.getItem('sjkMapEditLights') || 'null');
		if (kept && kept.on) sjkLightsToggle(true);
	} catch (e) { /* off */ }
}
function sjkLightsToggle(on) {
	sjkLights.on = on === undefined ? !sjkLights.on : on;
	$('btnLights').classList.toggle('on', sjkLights.on);
	sjkLightsSave();
	if (sjkLights.on) sjkLoadLamps();
}

// the map's lamp fixtures as the game's world found them (once per map; again when the Lights page is opened)
async function sjkLoadLamps(force) {
	if (!loadedMap || sjkLights.loading || (!force && sjkLights.lampsFor === loadedMap)) return;
	sjkLights.loading = true;
	const map = loadedMap;
	try {
		const res = await fetch('api/lamps');
		const lamps = await res.json();
		if (lamps.ok === false) throw new Error(lamps.error);
		sjkLights.lamps = lamps;
		sjkLights.lampsFor = map;
	} catch (e) {
		sjkLights.lamps = null;
	}
	sjkLights.loading = false;
	sjkRenderLights();
}

// ---- what the edits say

const sjkLightOf = id => (sjk.state && sjk.state.lights || []).find(l => l.id === id) || null;
const sjkV3 = v => Array.isArray(v) && v.length === 3 ? v : [0, 0, 0];
const sjkMoved = v => v.some(x => Math.abs(x) > 1e-4);
// the lamp edits of the document that hold here (a server's own wins), by fixture
function sjkLampEdits() {
	const out = new Map(), s = sjk.state;
	if (!s) return out;
	const edits = s.edits.filter(e => e.active && e.edit.kind === 'lamp');
	// the every-server edits first: one for this server wins over them
	edits.sort((a, b) => (a.edit.server ? 1 : 0) - (b.edit.server ? 1 : 0));
	for (const e of edits) out.set(e.edit.fixture, { id: e.id, off: !!e.edit.off, offset: sjkV3(e.edit.offset) });
	return out;
}
// what the game's world was built with
function sjkLampApplied() {
	const a = sjkLights.lamps && sjkLights.lamps.applied, out = new Map();
	if (!a) return out;
	for (const id of a.off || []) out.set(id, { off: true, offset: [0, 0, 0] });
	for (const [id, offset] of a.moved || []) out.set(id, { off: false, offset });
	return out;
}
// each fixture with what its edit wants and whether the world shows it yet
function sjkLampRows() {
	const fixtures = (sjkLights.lamps && sjkLights.lamps.fixtures) || [], wanted = sjkLampEdits(), built = sjkLampApplied();
	return fixtures.map(f => {
		const w = wanted.get(f.id), b = built.get(f.id);
		const off = !!(w && w.off), offset = w ? w.offset : [0, 0, 0];
		const was = b || { off: false, offset: [0, 0, 0] };
		const pending = off !== was.off || offset.some((v, i) => Math.abs(v - was.offset[i]) > 1e-3);
		return Object.assign({}, f, { edit: w ? w.id : null, off, offset, pending, at: gzAdd(f.position, off ? [0, 0, 0] : offset) });
	});
}
function sjkLampRow(id) { return sjkLampRows().find(r => r.id === id) || null; }

// ---- drawing: bulbs and lamp markers the same size on screen whatever the distance

function sjkMarkScale(p) {
	const d = Math.hypot(p[0] - eye[0], p[1] - eye[1], p[2] - eye[2]);
	return Math.max(0.15, d / 700);
}
// the circles of a light's reach about each axis, as lines
const sjkReach = { key: '', g: null };
function sjkReachGeo(origin, radius) {
	const key = origin.join(',') + ':' + radius;
	if (sjkReach.key === key) return sjkReach.g;
	sjkGeoFree(sjkReach.g);
	const out = [], n = 64;
	for (let axis = 0; axis < 3; axis++) {
		for (let k = 0; k < n; k++) {
			for (const t of [k / n, (k + 1) / n]) {
				const a = t * Math.PI * 2, c = Math.cos(a) * radius, s = Math.sin(a) * radius;
				const p = axis === 0 ? [0, c, s] : axis === 1 ? [c, 0, s] : [c, s, 0];
				out.push(origin[0] + p[0], origin[1] + p[1], origin[2] + p[2]);
			}
		}
	}
	sjkReach.key = key;
	sjkReach.g = sjkGeo(out, gl.LINES);
	return sjkReach.g;
}

// After the selection: the markers, faint through walls and full where seen; the chosen one gold-ringed with its reach.
function sjkDrawLights(now) {
	if (!sjkLights.on && sjkTab !== 'lights') return;
	const s = sjk.state;
	if (!s) return;
	const chosen = sjkLights.chosen, glow = 0.5 + 0.5 * Math.sin(now / 240), pv = sjkLights.preview;
	gl.enable(gl.BLEND);
	gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
	gl.disable(gl.CULL_FACE);
	gl.depthMask(false);
	const pass = (seen) => {
		if (seen) { gl.enable(gl.DEPTH_TEST); gl.depthFunc(gl.LEQUAL); } else gl.disable(gl.DEPTH_TEST);
		const alpha = seen ? 1 : 0.28;
		for (const r of sjkLampRows()) {
			const sel = chosen && chosen.kind === 'lamp' && chosen.id === r.id;
			const at = pv && pv.kind === 'lamp' && pv.id === r.id ? pv.origin : r.at;
			const c = r.off ? [0.35, 0.35, 0.38] : r.color.map(v => 0.35 + 0.65 * Math.min(1, v));
			sjkMark(cubeMesh, at, 0, sjkMarkScale(at) * (sel ? 0.9 : 0.6), [...c, alpha * (r.off ? 0.7 : 0.9)]);
			if (sel) sjkMark(ringMesh, at, now / 900, sjkMarkScale(at) * 0.55, [1, 0.78, 0.3, alpha]);
		}
		for (const l of s.lights || []) {
			const sel = chosen && chosen.kind === 'light' && chosen.id === l.id;
			const at = pv && pv.kind === 'light' && pv.id === l.id ? pv.origin : l.origin;
			const c = l.color.map(v => Math.min(1, v * Math.max(0.6, Math.min(1.4, l.intensity))));
			sjkMark(diamondMesh, at, now / 1200, sjkMarkScale(at) * (sel ? 1.4 : 1.0), [...c, alpha]);
			if (sel) sjkMark(ringMesh, at, now / 900, sjkMarkScale(at) * 0.8, [1, 0.78, 0.3, alpha * (0.6 + 0.4 * glow)]);
		}
	};
	pass(false);
	pass(true);
	// the chosen light's reach
	const l = chosen && chosen.kind === 'light' && sjkLightOf(chosen.id);
	if (l) {
		const at = pv && pv.kind === 'light' && pv.id === l.id ? pv.origin : l.origin;
		gl.useProgram(sjkOverlay.p);
		gl.uniformMatrix4fv(sjkOverlay.u.uMvp, false, viewProj);
		sjkOutline(sjkReachGeo(at, l.radius), SJK_IDENTITY, glow * 0.4);
	}
	gl.depthMask(true);
	gl.depthFunc(gl.LESS);
	gl.enable(gl.DEPTH_TEST);
	gl.disable(gl.BLEND);
	gl.bindVertexArray(null);
}

// the marker under the pointer (CSS pixels): the nearest to the eye within a few pixels
function sjkLightAt(x, y) {
	if (!sjkLights.on && sjkTab !== 'lights') return null;
	let best = null, bestAt = Infinity;
	const offer = (kind, id, at) => {
		const p = project(at);
		if (!p || Math.hypot(p[0] - x, p[1] - y) > SJK_LIGHT_PICK) return;
		const d = Math.hypot(at[0] - eye[0], at[1] - eye[1], at[2] - eye[2]);
		// an added light wins over a lamp at the same spot
		const rank = d - (kind === 'light' ? 32 : 0);
		if (rank < bestAt) { best = { kind, id }; bestAt = rank; }
	};
	for (const l of (sjk.state && sjk.state.lights) || []) offer('light', l.id, l.origin);
	for (const r of sjkLampRows()) offer('lamp', r.id, r.at);
	return best;
}

function sjkChooseLight(c) {
	sjkLights.chosen = c;
	if (sjkTab !== 'lights') sjkShowTab('lights');
	sjkRenderLights();
}

// what the gizmo moves on the Lights page: the chosen light or lamp, moved only
function sjkLightTarget() {
	if (sjkTab !== 'lights' || !sjkLights.chosen) return null;
	const c = sjkLights.chosen;
	if (c.kind === 'light') {
		const l = sjkLightOf(c.id);
		return l ? { kind: 'light', list: [{ id: l.id, origin: l.origin.slice(), angles: [0, 0, 0], scale: [1, 1, 1] }] } : null;
	}
	const r = sjkLampRow(c.id);
	return r && !r.off ? { kind: 'lamp', fixture: r.id, base: r.position, list: [{ id: 0, origin: r.at.slice(), angles: [0, 0, 0], scale: [1, 1, 1] }] } : null;
}
// the end of a gizmo's move: the light set where it was put, or the lamp's offset from where the map has it
function sjkLightMoved(a, origin) {
	if (a.kind === 'light') {
		const l = sjkLightOf(a.start[0].id);
		if (!l) return null;
		return sjkEdit({ op: 'set_light', id: l.id, origin, color: l.color, radius: l.radius, intensity: l.intensity });
	}
	const offset = origin.map((v, i) => Math.round((v - a.base[i]) * 100) / 100);
	return sjkEdit(sjkMoved(offset) ? { op: 'lamp', fixture: a.fixture, action: 'move', offset } : { op: 'lamp', fixture: a.fixture, action: 'reset' });
}

// ---- adding a light: where the next click (or the view's centre) meets the map, in front of it

async function sjkAddLight(ray) {
	sjkLights.placing = false;
	const answer = await sjkEdit(Object.assign({ op: 'add_light', color: sjkLights.color, radius: sjkLights.radius, intensity: sjkLights.intensity }, ray));
	const id = answer.ok && answer.lines && /^(\d+):/.exec(answer.lines[answer.lines.length - 1] || '');
	if (id) sjkChooseLight({ kind: 'light', id: +id[1] });
	sjkRenderLights();
}

// ---- the page

const sjkHex = c => '#' + c.map(v => Math.round(Math.max(0, Math.min(1, v)) * 255).toString(16).padStart(2, '0')).join('');
const sjkUnhex = h => [1, 3, 5].map(i => parseInt(h.slice(i, i + 2), 16) / 255);
const sjkRound = (v, n = 0) => (Math.round(v * 10 ** n) / 10 ** n).toString();

function sjkRenderLights() {
	if (sjkTab !== 'lights') return;
	const s = sjk.state, lamps = sjkLights.lamps, c = sjkLights.chosen;
	// the note: what players see
	const realtime = lamps && lamps.realtime;
	sjkText('lightsNote', 'Added lights are dynamic lights every SJK player sees on every preset, ' + (s ? s.lights_budget || 8 : 8) +
		' at most at once (the nearest). The map\'s lamps light only the Experimental preset\'s real-time lighting; a lamp edit applies when the map is loaded again (or vid_restart force). Baked lightmaps never change.');
	$('btnAddLight').classList.toggle('on', sjkLights.placing);
	sjkText('addHint', sjkLights.placing ? 'Click the map where the light goes (Esc stops).' : '');
	// added lights
	const lights = (s && s.lights) || [];
	sjkHtml($('lightList'), lights.map(l => '<div class="row' + (c && c.kind === 'light' && c.id === l.id ? ' sel' : '') + '" data-light="' + l.id + '">' +
		'<i class="swatch" style="background:' + sjkHex(l.color) + '"></i>' + l.id + ': r' + sjkRound(l.radius) + (Math.abs(l.intensity - 1) > 1e-3 ? ' x' + sjkRound(l.intensity, 2) : '') +
		' <span class="count">' + l.origin.map(v => Math.round(v)).join(' ') + '</span></div>').join('') || '<div class="hint">No added lights on this map.</div>');
	const light = c && c.kind === 'light' && sjkLightOf(c.id);
	$('lightProps').hidden = !light;
	if (light) {
		// a field being typed in keeps what is typed
		const set = (id, v) => { if (document.activeElement !== $(id)) $(id).value = v; };
		set('lightColor', sjkHex(light.color));
		set('lightRadius', sjkRound(light.radius));
		set('lightIntensity', sjkRound(light.intensity, 2));
		sjkText('lightAt', 'Light ' + light.id + ' at ' + light.origin.map(v => Math.round(v)).join(' '));
	}
	// the map's lamps
	const rows = sjkLampRows(), words = sjkWords($('lampFind').value);
	const shown = rows.filter(r => sjkMatches(r.id + ' ' + r.name, words)).slice(0, SJK_ROWS);
	sjkText('lampCount', lamps ? rows.length + ' fixtures' : '');
	sjkHtml($('lampList'), !lamps ? '<div class="hint">' + (sjkLights.loading ? 'Reading the lamps...' : 'The game did not send its lamps.') + '</div>' :
		!rows.length ? '<div class="hint">' + (realtime ? 'This map has no real-time lamps.' : 'No lamps: they are found on the Experimental preset only (real-time lighting). Switch to it in the game to see and edit them.') + '</div>' :
		shown.map(r => '<div class="row' + (r.off ? ' dim' : '') + (c && c.kind === 'lamp' && c.id === r.id ? ' sel' : '') + '" data-lamp="' + escapeHtml(r.id) + '" title="' + escapeHtml(r.name) + '">' +
			'<i class="swatch" style="background:' + (r.off ? '#333' : sjkHex(r.color)) + '"></i>' + escapeHtml(r.id) + ' ' + escapeHtml(r.name.slice(r.name.lastIndexOf('/') + 1)) +
			(r.off ? ' <b class="gold">off</b>' : sjkMoved(r.offset) ? ' <b class="gold">moved</b>' : '') + (r.pending ? ' <span class="count">(on reload)</span>' : '') + '</div>').join(''));
	const lamp = c && c.kind === 'lamp' && sjkLampRow(c.id);
	$('lampProps').hidden = !lamp;
	if (lamp) {
		sjkHtml($('lampFacts'), [['Fixture', lamp.id], ['Kind', { surface: 'BSP surface', prop: 'Prop model surface', effect: 'Steady effect' }[lamp.kind] || lamp.kind],
			['Name', lamp.name || '?'], ['Lamps', lamp.lamps + ' patches'], ['Power', sjkRound(lamp.power, 2)], ['Reach', sjkRound(lamp.radius)],
			['At', lamp.position.map(v => Math.round(v)).join(' ')], ['Moved by', lamp.offset.map(v => sjkRound(v, 1)).join(' ')]]
			.map(([k, v]) => '<dt>' + k + '</dt><dd>' + escapeHtml(String(v)) + '</dd>').join(''));
		$('btnLampOff').textContent = lamp.off ? 'Turn on' : 'Turn off';
		$('btnLampReset').disabled = lamp.edit === null;
		sjkText('lampPending', lamp.pending ? 'Waits for the map\'s next load (or vid_restart force) to show in the game.' : lamp.edit !== null ? 'Shown in the game as edited.' : '');
	}
}

// ---- the page's controls

$('btnLights').addEventListener('click', () => sjkLightsToggle());
$('btnAddLight').addEventListener('click', () => {
	sjkLights.placing = !sjkLights.placing;
	sjk.placeThing = null;
	sjk.chosenModel = '';
	if (sjkLights.placing && !sjkLights.on) sjkLightsToggle(true);
	sjkRenderLights();
});
$('btnAddLightHere').addEventListener('click', () => sjkAddLight(sjkRay(canvas.clientWidth / 2, canvas.clientHeight / 2)));
$('lightList').addEventListener('click', e => {
	const row = e.target.closest('.row');
	if (!row || String(getSelection())) return;
	sjkChooseLight({ kind: 'light', id: +row.dataset.light });
});
$('lightList').addEventListener('dblclick', e => {
	const row = e.target.closest('.row'), l = row && sjkLightOf(+row.dataset.light);
	if (l) sjkFlyTo(l.origin);
});
$('lampList').addEventListener('click', e => {
	const row = e.target.closest('.row');
	if (!row || String(getSelection())) return;
	sjkChooseLight({ kind: 'lamp', id: row.dataset.lamp });
});
$('lampList').addEventListener('dblclick', e => {
	const row = e.target.closest('.row'), r = row && sjkLampRow(row.dataset.lamp);
	if (r) sjkFlyTo(r.at);
});
$('lampFind').addEventListener('input', () => { clearTimeout(sjkTyped); sjkTyped = setTimeout(sjkRenderLights, 150); });
// a light's fields: each change is one `set_light`, one step to undo
function sjkLightFields() {
	const c = sjkLights.chosen, l = c && c.kind === 'light' && sjkLightOf(c.id);
	if (!l) return;
	const radius = parseFloat($('lightRadius').value), intensity = parseFloat($('lightIntensity').value), color = sjkUnhex($('lightColor').value);
	if (!isFinite(radius) || !isFinite(intensity)) { toast('A radius and an intensity are numbers', false); return; }
	const r = Math.max(16, Math.min(2048, radius)), i = Math.max(0.05, Math.min(8, intensity));
	const c3 = color.every(v => v === 0) ? [0.01, 0.01, 0.01] : color;
	sjkLights.color = c3; sjkLights.radius = r; sjkLights.intensity = i;
	sjkEdit({ op: 'set_light', id: l.id, origin: l.origin, color: c3, radius: r, intensity: i });
}
for (const id of ['lightColor', 'lightRadius', 'lightIntensity']) $(id).addEventListener('change', sjkLightFields);
$('btnLightDelete').addEventListener('click', () => {
	const c = sjkLights.chosen;
	if (c && c.kind === 'light') { sjkEdit({ op: 'delete', id: c.id }); sjkLights.chosen = null; }
});
$('btnLightFly').addEventListener('click', () => { const c = sjkLights.chosen, l = c && sjkLightOf(c.id); if (l) sjkFlyTo(l.origin); });
$('btnLampOff').addEventListener('click', () => {
	const c = sjkLights.chosen, r = c && c.kind === 'lamp' && sjkLampRow(c.id);
	if (r) sjkEdit({ op: 'lamp', fixture: r.id, action: r.off ? 'on' : 'off' });
});
$('btnLampReset').addEventListener('click', () => {
	const c = sjkLights.chosen;
	if (c && c.kind === 'lamp') sjkEdit({ op: 'lamp', fixture: c.id, action: 'reset' });
});
$('btnLampFly').addEventListener('click', () => { const c = sjkLights.chosen, r = c && c.kind === 'lamp' && sjkLampRow(c.id); if (r) sjkFlyTo(r.at); });
$('btnLampsReload').addEventListener('click', () => sjkLoadLamps(true));
