// ---- the view: the fly camera, a click on the map, the selection drawn over it, copy and paste, and a frame
//
// The frame is VisionJK's (js/frame.js) with only the map: the map with its shader scripts once they are read
// (the plain view of lightmaps and images before), the models of misc_model_static, SJK's placed models and
// shown faces, the boomboxes, boxes and toys, what is see-through, the flares, and over it all the selection.

const keys = new Set();
cam.fly = true;

// W A S D (or the arrows) along the view, Q and E (or C and Space) down and up, Shift three times as fast; the
// wheel sets the speed
function moveCamera(dt) {
	const y = cam.yaw * Math.PI / 180, d = camDir(), v = cam.flySpeed * (keys.has('shift') ? 3 : 1) * dt;
	const fwd = (keys.has('w') || keys.has('arrowup') ? 1 : 0) - (keys.has('s') || keys.has('arrowdown') ? 1 : 0);
	const right = (keys.has('d') || keys.has('arrowright') ? 1 : 0) - (keys.has('a') || keys.has('arrowleft') ? 1 : 0);
	const up = (keys.has('e') || keys.has(' ') ? 1 : 0) - (keys.has('q') || keys.has('c') ? 1 : 0);
	cam.pos[0] += (d[0] * fwd + Math.sin(y) * right) * v;
	cam.pos[1] += (d[1] * fwd - Math.cos(y) * right) * v;
	cam.pos[2] += (d[2] * fwd + up) * v;
}

// the plain view (js/frame.js drawWorldPlain, as in the game: its light, no haze) until the scripts are read
function drawWorldPlain() {
	gl.useProgram(mapProg.p);
	gl.uniformMatrix4fv(mapProg.u.uMvp, false, viewProj);
	gl.uniform3fv(mapProg.u.uEye, eye);
	gl.uniform1f(mapProg.u.uFogFar, 4000);
	gl.uniform1i(mapProg.u.uTex, 0);
	gl.uniform1i(mapProg.u.uLm, 1);
	gl.uniform1i(mapProg.u.uView, 3);
	gl.disable(gl.CULL_FACE);
	for (const b of world.batches) {
		const drawn = remapOf(b.shaderName);
		let t = world.textures.get(drawn) || world.textures.get(b.shaderName);
		if (!t || t.failed) t = b.editorTex || (b.editorTex = editorImage(b.shaderName)) || t;
		if (!t) continue;
		gl.activeTexture(gl.TEXTURE0);
		gl.bindTexture(gl.TEXTURE_2D, t.tex);
		gl.activeTexture(gl.TEXTURE1);
		gl.bindTexture(gl.TEXTURE_2D, b.lm || grayTex);
		gl.uniform1i(mapProg.u.uUseLm, b.lm ? 1 : 0);
		gl.uniform1i(mapProg.u.uAlphaTest, t.alpha ? 1 : 0);
		gl.bindVertexArray(b.vao);
		gl.drawElements(gl.TRIANGLES, b.count, gl.UNSIGNED_INT, 0);
	}
	gl.activeTexture(gl.TEXTURE0);
}

// a mesh of js/gl.js at a point, in one colour
function sjkMark(mesh, pos, yaw, scale, color) {
	const c = Math.cos(yaw), s = Math.sin(yaw);
	gl.useProgram(flatProg.p);
	gl.uniformMatrix4fv(flatProg.u.uMvp, false, viewProj);
	gl.uniformMatrix4fv(flatProg.u.uModel, false, new Float32Array([c * scale, s * scale, 0, 0, -s * scale, c * scale, 0, 0, 0, 0, scale, 0, pos[0], pos[1], pos[2], 1]));
	gl.uniform4fv(flatProg.u.uColor, color);
	gl.bindVertexArray(mesh.vao);
	gl.drawElements(gl.TRIANGLES, mesh.count, gl.UNSIGNED_SHORT, 0);
}

// the placed model Place moves
function sjkDrawMarks(now) {
	const s = sjk.state;
	if (!s) return;
	const current = s.current && s.placed.find(p => p.id === s.current);
	if (current) sjkMark(ringMesh, current.origin, now / 900, 0.5, [0.85, 0.7, 0.35, 1]);
	gl.bindVertexArray(null);
}

// ---- the selection drawn: its triangles tinted and pulsing with their outline over the map (a surface, the
// rest of the model Hide whole model takes fainter, a hidden face's polygon), and a model or a thing tinted with
// a box around it. Drawn last, with the depth test (a little in front, so it wins over what it lies on); the
// outline's hidden parts show faintly through what is in front of them.

const sjkOverlay = compile(`#version 300 es
layout(location=0) in vec3 aPos; uniform mat4 uMvp; uniform mat4 uModel; uniform float uBias;
void main() { gl_Position = uMvp * uModel * vec4(aPos, 1.0); gl_Position.z -= uBias * gl_Position.w; }`,
`#version 300 es
precision mediump float; uniform vec4 uColor; out vec4 o;
void main() { o = uColor; }`);
const SJK_GOLD = [1.0, 0.78, 0.3];

// positions as a mesh of their own, drawn as triangles or lines
function sjkGeo(data, mode) {
	if (!data.length) return null;
	const vao = gl.createVertexArray();
	gl.bindVertexArray(vao);
	const buf = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, buf);
	gl.bufferData(gl.ARRAY_BUFFER, data instanceof Float32Array ? data : new Float32Array(data), gl.STATIC_DRAW);
	gl.enableVertexAttribArray(0);
	gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 0, 0);
	gl.bindVertexArray(null);
	return { vao, buf, count: data.length / 3, mode };
}
function sjkGeoFree(g) {
	if (g) { gl.deleteBuffer(g.buf); gl.deleteVertexArray(g.vao); }
}

// The triangles of the map's surfaces `numbers` and the edges of their outline (those only one triangle has),
// from the batches' own lists (a patch at its finest).
function sjkSurfaceGeo(numbers) {
	const tris = [], edges = new Map();
	const at = (b, v) => Math.round(b.pos[v * 3] * 4) + ',' + Math.round(b.pos[v * 3 + 1] * 4) + ',' + Math.round(b.pos[v * 3 + 2] * 4);
	for (const b of [...world.batches, ...world.skyBatches]) {
		if (!b.ranges || !b.pos) continue;
		for (const [n, first, count] of b.ranges) {
			if (!numbers.has(n)) continue;
			for (let i = first; i + 2 < first + count; i += 3) {
				const v = [b.tri[i], b.tri[i + 1], b.tri[i + 2]];
				if (v[0] === v[1] || v[1] === v[2] || v[0] === v[2]) continue;
				for (const x of v) tris.push(b.pos[x * 3], b.pos[x * 3 + 1], b.pos[x * 3 + 2]);
				for (let e = 0; e < 3; e++) {
					const p = v[e], q = v[(e + 1) % 3], kp = at(b, p), kq = at(b, q);
					const key = kp < kq ? kp + '|' + kq : kq + '|' + kp;
					const known = edges.get(key);
					if (known) known.n++; else edges.set(key, { n: 1, b, p, q });
				}
			}
		}
	}
	const lines = [];
	for (const { n, b, p, q } of edges.values()) {
		if (n !== 1) continue;
		lines.push(b.pos[p * 3], b.pos[p * 3 + 1], b.pos[p * 3 + 2], b.pos[q * 3], b.pos[q * 3 + 1], b.pos[q * 3 + 2]);
	}
	return { fill: sjkGeo(tris, gl.TRIANGLES), lines: sjkGeo(lines, gl.LINES) };
}

// the box of an md3 as R_ModelBounds gives it (its first frame), read from the head of its file once
const sjkBounds = new Map();
function sjkModelBounds(path) {
	const key = path.toLowerCase();
	if (sjkBounds.has(key)) return sjkBounds.get(key);
	sjkBounds.set(key, null);
	fetchGameFile(path).then(data => {
		const dv = new DataView(data);
		if (readStr(dv, 0, 4) !== 'IDP3') return;
		const frames = dv.getInt32(92, true), f = i => dv.getFloat32(frames + i * 4, true);
		sjkBounds.set(key, { mins: [f(0), f(1), f(2)], maxs: [f(3), f(4), f(5)] });
	}).catch(() => {});
	return null;
}
// the twelve edges of a box, a little bigger than it
function sjkBoxLines(b) {
	const pad = 2, lo = b.mins.map(v => v - pad), hi = b.maxs.map(v => v + pad), c = [];
	for (let i = 0; i < 8; i++) c.push([i & 1 ? hi[0] : lo[0], i & 2 ? hi[1] : lo[1], i & 4 ? hi[2] : lo[2]]);
	const out = [];
	for (let i = 0; i < 8; i++) for (const bit of [1, 2, 4]) if (!(i & bit)) out.push(...c[i], ...c[i | bit]);
	return sjkGeo(out, gl.LINES);
}

// A model of the map's misc_model_static by its entity number: its path and where it is drawn (staticList's
// matrix), and its angles and scale as its keys say.
function sjkPropOf(number) {
	if (!world) return null;
	const e = mapEnts().statics.find(x => x.num === number);
	const path = e && spawnKey(e, 'model');
	if (!path) return null;
	const st = staticList([e]), angles = [0, 0, 0];
	if (!spawnVector(e, 'angles', angles) && spawnKey(e, 'angle') !== undefined) angles[1] = spawnFloat(e, 'angle');
	return { model: path, place: st.mats[0], angles, scale: Array.from(st.scale.subarray(0, 3)) };
}

// what is selected as the overlay draws it: a key that says when to make it again, meshes, and models with
// their matrices
const sjkSel = { key: '', fill: null, lines: null, groupFill: null, models: [], fills: [], outlines: [] };
function sjkSelectionKey(s) {
	const sel = s && s.selection, thing = sjkTab === 'world' ? sjkChosenThing() : null;
	return JSON.stringify([loadedMap, sel && sel.kind, sel && sel.target, sel && sel.kind === 'placed' && s.placed.find(p => p.id === sel.target.placed), thing && thing.parts,
		sjkMany() ? sjk.multi.map(m => m.key) : null]);
}
function sjkBuildSelection() {
	const s = sjk.state, key = sjkSelectionKey(s);
	if (sjkSel.key === key && sjkSel.world === world) return;
	sjkGeoFree(sjkSel.fill); sjkGeoFree(sjkSel.lines); sjkGeoFree(sjkSel.groupFill);
	for (const m of sjkSel.models) sjkGeoFree(m.box);
	for (const g of [...sjkSel.fills, ...sjkSel.outlines]) sjkGeoFree(g);
	Object.assign(sjkSel, { key, world, fill: null, lines: null, groupFill: null, models: [], hit: null, fills: [], outlines: [] });
	const sel = s && s.selection, t = sel && sel.target;
	if (sel) sjkSel.hit = sel.hit;
	// a multiple selection: each of its faces and surfaces as the selection is drawn
	if (sjkMany()) {
		const g = sjkMultiGeo();
		sjkSel.fills = g.fills; sjkSel.outlines = g.outlines; sjkSel.hit = null;
	} else if (sel && sel.kind === 'surface') {
		const own = sjkSurfaceGeo(new Set([t.surface]));
		sjkSel.fill = own.fill; sjkSel.lines = own.lines;
		const rest = (t.group || []).filter(n => n !== t.surface);
		if (rest.length) { const g = sjkSurfaceGeo(new Set(rest)); sjkSel.groupFill = g.fill; sjkGeoFree(g.lines); }
	} else if (sel && sel.kind === 'face' && t.points && t.points.length > 2) {
		const tri = [], line = [], p = t.points;
		for (let i = 2; i < p.length; i++) tri.push(...p[0], ...p[i - 1], ...p[i]);
		for (let i = 0; i < p.length; i++) line.push(...p[i], ...p[(i + 1) % p.length]);
		sjkSel.fill = sjkGeo(tri, gl.TRIANGLES); sjkSel.lines = sjkGeo(line, gl.LINES);
	} else if (sel && sel.kind === 'placed') {
		const p = sjk.placed.find(x => x.id === t.placed);
		if (p) sjkSel.models.push({ model: p.model, place: p.place, box: null });
	} else if (sel && sel.kind === 'model') {
		const prop = sjkPropOf(t.entity);
		if (prop) sjkSel.models.push({ model: prop.model, place: prop.place, box: null });
	}
	const thing = sjkTab === 'world' ? sjkChosenThing() : null;
	const drawn = thing && sjk.things.find(x => x.thing === thing.thing && x.id === thing.id && x.label === thing.label);
	if (drawn) for (const part of drawn.drawn) sjkSel.models.push({ model: part.model, place: part.place, box: null });
}

function sjkOverlayDraw(g, model, color, bias) {
	if (!g) return;
	gl.uniformMatrix4fv(sjkOverlay.u.uModel, false, model);
	gl.uniform4fv(sjkOverlay.u.uColor, color);
	gl.uniform1f(sjkOverlay.u.uBias, bias);
	gl.bindVertexArray(g.vao);
	gl.drawArrays(g.mode, 0, g.count);
}

// outlines: faint where something is in front of them, bright where they are seen
function sjkOutline(g, model, glow) {
	gl.disable(gl.DEPTH_TEST);
	sjkOverlayDraw(g, model, [SJK_GOLD[0], SJK_GOLD[1], SJK_GOLD[2], 0.3], 0);
	gl.enable(gl.DEPTH_TEST);
	sjkOverlayDraw(g, model, [1, 0.93, 0.7, 0.75 + 0.25 * glow], 0.0005);
}

function sjkDrawSelection(now) {
	sjkBuildSelection();
	const glow = 0.5 + 0.5 * Math.sin(now / 240);
	const fillColor = [SJK_GOLD[0], SJK_GOLD[1], SJK_GOLD[2], 0.28 + 0.3 * glow];
	gl.useProgram(sjkOverlay.p);
	gl.uniformMatrix4fv(sjkOverlay.u.uMvp, false, viewProj);
	gl.enable(gl.BLEND);
	gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
	gl.disable(gl.CULL_FACE);
	gl.depthMask(false);
	gl.depthFunc(gl.LEQUAL);
	gl.enable(gl.DEPTH_TEST);
	gl.enable(gl.POLYGON_OFFSET_FILL);
	gl.polygonOffset(-1, -4);
	let drew = false;
	if (sjkSel.groupFill) sjkOverlayDraw(sjkSel.groupFill, SJK_IDENTITY, [SJK_GOLD[0], SJK_GOLD[1], SJK_GOLD[2], 0.08 + 0.08 * glow], 0);
	if (sjkSel.fill) { sjkOverlayDraw(sjkSel.fill, SJK_IDENTITY, fillColor, 0); drew = true; }
	for (const g of sjkSel.fills) { sjkOverlayDraw(g, SJK_IDENTITY, fillColor, 0); drew = true; }
	for (const m of sjkSel.models) {
		const model = staticNow(m.model);
		if (model) {
			gl.uniformMatrix4fv(sjkOverlay.u.uModel, false, m.place);
			gl.uniform4fv(sjkOverlay.u.uColor, [SJK_GOLD[0], SJK_GOLD[1], SJK_GOLD[2], 0.1 + 0.15 * glow]);
			gl.uniform1f(sjkOverlay.u.uBias, 0);
			for (const mesh of model.meshes) {
				gl.bindVertexArray(mesh.vao);
				gl.drawElements(gl.TRIANGLES, mesh.count, gl.UNSIGNED_INT, 0);
			}
		}
		if (!m.box) { const b = sjkModelBounds(m.model); if (b) m.box = sjkBoxLines(b); }
		if (m.box) { sjkOutline(m.box, m.place, glow); drew = true; }
	}
	gl.disable(gl.POLYGON_OFFSET_FILL);
	if (sjkSel.lines) sjkOutline(sjkSel.lines, SJK_IDENTITY, glow);
	for (const g of sjkSel.outlines) sjkOutline(g, SJK_IDENTITY, glow);
	gl.depthMask(true);
	gl.depthFunc(gl.LESS);
	gl.disable(gl.BLEND);
	gl.bindVertexArray(null);
	// nothing of its own to draw (a mover, a model still loading): the point it was chosen at
	if (!drew && sjkSel.hit) sjkMark(cubeMesh, sjkSel.hit, now / 700, 0.5, [0.85, 0.7, 0.35, 1]);
	gl.bindVertexArray(null);
}

const frameErrors = new Set();
let sjkBefore = performance.now();
function frame(now) {
	try {
		drawFrame(now);
	} catch (e) {
		const text = String(e && e.stack || e);
		if (!frameErrors.has(text)) { frameErrors.add(text); console.error('a frame failed: ' + text); }
		gl.bindVertexArray(null);
		gl.depthMask(true); gl.depthFunc(gl.LESS); gl.depthRange(0, 1);
		gl.colorMask(true, true, true, true);
		gl.disable(gl.BLEND); gl.disable(gl.CULL_FACE); gl.disable(gl.POLYGON_OFFSET_FILL);
	}
	requestAnimationFrame(frame);
}
function drawFrame(now) {
	const dt = Math.min(0.1, (now - sjkBefore) / 1000);
	sjkBefore = lastFrame = now;
	if (!document.hidden) moveCamera(dt);
	if (canvas.width !== Math.max(1, Math.round(canvas.clientWidth * canvasScale()))) resize();
	updateCamera();
	frameNow = now;
	if (world) {
		if (settleSky(world)) prepareScripted(world);
		updateGridLod(now, gameLook() && !!world.scripted);
		// the edits of the state that came before the map was read
		if (sjk.world !== world) sjkApply();
	}
	gl.viewport(0, 0, canvas.width, canvas.height);
	gl.clearColor(0.03, 0.035, 0.05, 1);
	gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
	gl.enable(gl.DEPTH_TEST);
	if (!world) return;
	const scripted = gameLook() && world.scripted;
	lateModels.length = 0;
	deferLate = !!scripted;
	if (scripted) {
		skyPortalOn = drawSkyPortal(now);
		drawWorldScripted(now, false);
	} else {
		drawWorldPlain();
	}
	drawWorldEnts(now);
	sjkGizmoPreview();
	sjkDrawEdits();
	sjkDrawBrushes();
	sjkDrawMarks(now);
	if (scripted) {
		drawWorldScripted(now, true);
		drawFlares(now);
	}
	deferLate = false;
	sjkDrawSelection(now);
	sjkDrawGizmo(now);
}

// ---- mouse and keys on the map
//
// The view's keys act only while the view has focus (a click or a drag on it gives it): never while a field of
// the panel has it, nor on text selected in the panel, so Ctrl+A, C, V and X stay the field's and the page's.

const viewFocused = () => document.activeElement === canvas;
let drag = null;
canvas.addEventListener('contextmenu', e => e.preventDefault());
canvas.addEventListener('pointerdown', e => {
	canvas.focus({ preventScroll: true });
	drag = { x: e.clientX, y: e.clientY, moved: false, button: e.button };
	canvas.setPointerCapture(e.pointerId);
});
canvas.addEventListener('focus', () => $('hud').classList.add('focused'));
canvas.addEventListener('blur', () => { $('hud').classList.remove('focused'); keys.clear(); });
// a button of the panel hands the keys back to the view; a field keeps them
$('panel').addEventListener('click', e => {
	if (e.target.closest('button') && !String(getSelection())) canvas.focus({ preventScroll: true });
});
canvas.addEventListener('pointermove', e => {
	const r = canvas.getBoundingClientRect();
	sjk.mouse = { x: e.clientX - r.left, y: e.clientY - r.top };
	if (!drag) return;
	const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
	if (!drag.moved && Math.hypot(dx, dy) < 4) return;
	drag.moved = true;
	drag.x = e.clientX;
	drag.y = e.clientY;
	// any button held looks around
	cam.yaw -= dx * 0.25;
	cam.pitch = Math.max(-89, Math.min(89, cam.pitch + dy * 0.25));
});
canvas.addEventListener('pointerup', e => {
	const was = drag;
	drag = null;
	if (!was || was.moved || was.button !== 0 || !world || !viewProj) return;
	const r = canvas.getBoundingClientRect(), ray = sjkRay(e.clientX - r.left, e.clientY - r.top);
	// a thing chosen on World goes where the click is; on Place with a model chosen the model; else a click on a
	// boombox, box or toy chooses it, and anywhere else selects what the game's pick finds
	if (sjk.placeThing) {
		const thing = sjk.placeThing;
		sjk.placeThing = null;
		sjkEdit(Object.assign({ op: 'place_thing', thing, server: $('thingServer').checked && thing !== 'boombox', escort: $('thingEscort').checked && thing === 'toy' },
			thing === 'toy' ? { kind: $('toyKind').value } : {}, ray));
	} else if (sjkTab === 'place' && sjk.chosenModel) sjkEdit(Object.assign({ op: 'place', model: sjk.chosenModel }, ray));
	else {
		const thing = sjkThingOnRay(ray);
		if (thing) {
			sjk.chosenThing = { thing: thing.thing, id: thing.id, label: thing.label };
			sjkShowTab('world');
			sjkRender();
			return;
		}
		sjkSelectClick(ray, e.shiftKey || e.ctrlKey || e.metaKey);
		if (sjkTab === 'edits' || sjkTab === 'world') sjkShowTab('selected');
	}
});

// the boombox, box or toy nearest along a ray, within a ball around each
function sjkThingOnRay(ray) {
	const d = ray.dir, l = Math.hypot(d[0], d[1], d[2]) || 1;
	let best = null, bestAt = Infinity;
	for (const t of sjk.things) {
		const v = [t.origin[0] - ray.eye[0], t.origin[1] - ray.eye[1], t.origin[2] + 12 - ray.eye[2]];
		const along = (v[0] * d[0] + v[1] * d[1] + v[2] * d[2]) / l;
		if (along < 0) continue;
		const off = v[0] * v[0] + v[1] * v[1] + v[2] * v[2] - along * along;
		if (off <= 20 * 20 && along < bestAt) { best = t; bestAt = along; }
	}
	return best;
}
canvas.addEventListener('wheel', e => {
	e.preventDefault();
	cam.flySpeed = Math.max(50, Math.min(4000, cam.flySpeed * (e.deltaY < 0 ? 1.2 : 1 / 1.2)));
	toast('Speed ' + Math.round(cam.flySpeed));
}, { passive: false });
window.addEventListener('keydown', e => {
	if (!viewFocused()) return;
	const k = e.key.toLowerCase(), mod = e.ctrlKey || e.metaKey;
	if (mod && k === 'z') { e.preventDefault(); sjkEdit({ op: e.shiftKey ? 'redo' : 'undo' }); return; }
	if (mod && k === 'y') { e.preventDefault(); sjkEdit({ op: 'redo' }); return; }
	// the copy and paste events follow (below); if the browser sends none, the clipboard is asked directly
	if (mod && k === 'c') { sjk.copyPending = true; setTimeout(sjkCopyFallback, 80); return; }
	if (mod && k === 'v') { sjk.pastePending = true; setTimeout(sjkPasteFallback, 80); return; }
	if (mod || e.altKey) return;
	if (e.key === '?') { e.preventDefault(); sjkHudFold(); return; }
	if (k === 'escape') { if (!sjkStopPlacing()) sjkClearSelection(); return; }
	if (k === 'b') { sjkBrushToggle(); return; }
	if (k === 'delete' && sjk.chosenEdit !== null && sjkTab === 'edits') { sjkEdit({ op: 'delete', id: sjk.chosenEdit }); return; }
	if (k === 'h' && sjk.state && (sjk.state.selection || sjkMany())) { sjkHideSelection(); return; }
	keys.add(k);
	if (k === ' ' || k.startsWith('arrow')) e.preventDefault();
});
// Delete on the list of edits, once a row was clicked
$('editList').addEventListener('keydown', e => {
	if (e.key === 'Delete' && sjk.chosenEdit !== null) { e.preventDefault(); sjkEdit({ op: 'delete', id: sjk.chosenEdit }); }
});

function sjkStopPlacing() {
	if (!sjk.placeThing && !sjk.chosenModel) return false;
	sjk.placeThing = null;
	sjk.chosenModel = '';
	for (const r of document.querySelectorAll('#modelList .row, #modelRecent .row')) r.classList.remove('sel');
	sjkRender();
	toast('Stopped placing');
	return true;
}

// ---- the controls' card: folded with ? or a click on its head, kept so in this browser

function sjkHudFold(fold) {
	const hud = $('hud'), folded = fold === undefined ? !hud.classList.contains('folded') : fold;
	hud.classList.toggle('folded', folded);
	try { localStorage.setItem('sjkMapEditHudFolded', folded ? '1' : '0'); } catch (e) { /* not kept */ }
}
$('hudHead').addEventListener('click', () => sjkHudFold());
try { sjkHudFold(localStorage.getItem('sjkMapEditHudFolded') === '1'); } catch (e) { /* unfolded */ }

// ---- copy and paste in the view
//
// Ctrl+C copies the selection: a placed model or a map prop with its scale and turn, or a surface's shader (the
// one drawn, a remap's when it has one), as its plain text (the model's or the shader's path) and the whole of
// it for this page. Ctrl+V pastes a model as a placed copy where the pointer is, or a shader as a remap (or a
// shown face) of the selection. A path copied elsewhere pastes the same way.

const SJK_CLIP_TYPE = 'application/x-sjk-mapedit';

function sjkClipOfSelection() {
	const s = sjk.state, sel = s && s.selection, t = sel && sel.target;
	if (!sel) return null;
	if (sel.kind === 'placed') {
		const p = s.placed.find(x => x.id === t.placed);
		if (p) return { kind: 'model', model: p.model, angles: p.angles, scale: p.scale, text: p.model };
	}
	if (sel.kind === 'model') {
		const prop = sjkPropOf(t.entity);
		if (prop) return { kind: 'model', model: prop.model, angles: prop.angles, scale: prop.scale, text: prop.model };
		if (t.model && /\.md3$/i.test(t.model)) return { kind: 'model', model: t.model, text: t.model };
	}
	if (t && t.shader && (sel.kind === 'surface' || sel.kind === 'face')) {
		const remap = s.remaps.find(([from]) => from.toLowerCase() === t.shader.toLowerCase());
		const shader = remap ? remap[1] : t.shader;
		return { kind: 'shader', shader, text: shader };
	}
	return null;
}

// what a text pasted from elsewhere is: a model path, a shader path, or this page's own copy
function sjkClipOfText(text) {
	text = (text || '').trim().replace(/\\/g, '/');
	if (!text || text.length > 160 || /\s/.test(text)) return null;
	if (sjk.clip && sjk.clip.text === text) return sjk.clip;
	if (/\.md3$/i.test(text)) return { kind: 'model', model: text, text };
	if (text.includes('/')) return { kind: 'shader', shader: text, text };
	return null;
}
const sjkVec3 = v => Array.isArray(v) && v.length === 3 && v.every(n => typeof n === 'number' && isFinite(n));
function sjkClipOfData(data) {
	try {
		const c = JSON.parse(data);
		if (c && c.kind === 'model' && typeof c.model === 'string') return { kind: 'model', model: c.model, text: c.model, angles: sjkVec3(c.angles) ? c.angles : undefined, scale: sjkVec3(c.scale) ? c.scale : undefined };
		if (c && c.kind === 'shader' && typeof c.shader === 'string') return { kind: 'shader', shader: c.shader, text: c.shader };
	} catch (e) { /* not ours */ }
	return null;
}

function sjkPasteClip(c) {
	if (!c) { toast('Nothing to paste: copy a model or a shader path, or a selection here', false); return; }
	if (c.kind === 'model') {
		const m = sjk.mouse || { x: canvas.clientWidth / 2, y: canvas.clientHeight / 2 };
		const body = Object.assign({ op: 'place', model: c.model }, sjkRay(m.x, m.y));
		if (c.angles) body.angles = c.angles;
		if (c.scale) body.scale = c.scale;
		sjkEdit(body);
		return;
	}
	const sel = sjk.state && sjk.state.selection;
	if (!sjkMany() && (!sel || sel.kind === 'other')) { toast('Select a surface or a face to paste ' + c.shader + ' on', false); return; }
	sjkApplyShader(c.shader);
}

document.addEventListener('copy', e => {
	if (!viewFocused() || String(getSelection())) return;
	sjk.copyPending = false;
	const c = sjkClipOfSelection();
	if (!c) { toast('Nothing selected to copy', false); return; }
	e.preventDefault();
	e.clipboardData.setData('text/plain', c.text);
	try { e.clipboardData.setData(SJK_CLIP_TYPE, JSON.stringify(c)); } catch (err) { /* the text is enough */ }
	sjk.clip = c;
	toast('Copied ' + c.text);
});
document.addEventListener('paste', e => {
	if (!viewFocused()) return;
	sjk.pastePending = false;
	e.preventDefault();
	sjkPasteClip(sjkClipOfData(e.clipboardData.getData(SJK_CLIP_TYPE)) || sjkClipOfText(e.clipboardData.getData('text/plain')));
});
function sjkCopyFallback() {
	if (!sjk.copyPending) return;
	sjk.copyPending = false;
	const c = sjkClipOfSelection();
	if (!c) { toast('Nothing selected to copy', false); return; }
	sjk.clip = c;
	(navigator.clipboard ? navigator.clipboard.writeText(c.text) : Promise.reject()).catch(() => {});
	toast('Copied ' + c.text);
}
function sjkPasteFallback() {
	if (!sjk.pastePending) return;
	sjk.pastePending = false;
	(navigator.clipboard ? navigator.clipboard.readText() : Promise.reject())
		.then(text => sjkPasteClip(sjkClipOfText(text)), () => sjkPasteClip(sjk.clip));
}
window.addEventListener('keyup', e => { keys.delete(e.key.toLowerCase()); if (e.key === 'Shift') keys.delete('shift'); });
window.addEventListener('blur', () => keys.clear());
window.addEventListener('resize', resize);

// ---- the start

if (!gl) {
	$('loading').textContent = 'This browser has no WebGL 2.';
} else {
	resize();
	// the map as the game draws it: its shader scripts (api/shaders)
	loadShaderScripts();
	sjkShowTab('selected');
	sjkPoll();
	sjkLoadRecent();
	sjkBrushRestore();
	requestAnimationFrame(frame);
}
