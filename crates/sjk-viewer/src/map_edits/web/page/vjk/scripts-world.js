// From VisionJK's web admin page (codemp/game/webadmin/page/js/scripts-world.js at 50d7c18b, GPL-2.0, by Blackwolf;
// the panel after JKAdmin's by Lixsl), for SJK's browser map editor. SJK: the sky drawn is the one a remap swaps the
// map's for (remapOf in prepareScripted), as the game's sky follows a sky remap.
// the sky of the engine (tr_sky.cpp): a box around the eye, and the layers of clouds on a dome above it
const SKY_SIDES = [[3, -1, 2], [-3, 1, 2], [1, 3, 2], [-1, -3, 2], [-2, -1, 3], [2, -1, -3]];
const SKY_SUFFIX = ['rt', 'lf', 'bk', 'ft', 'up', 'dn'];
// SKY_RADIUS: the radius of the world the clouds are laid around (R_InitSkyTexCoords), not the size of the box
const SKY_CUTS = 8, SKY_RADIUS = 4096;

function skyVec(s, t, side) {
	const b = [s, t, 1];
	return SKY_SIDES[side].map(k => k < 0 ? -b[-k - 1] : b[k - 1]);
}

// where a direction meets the layer of clouds, as texture coordinates
function cloudSt(v, height) {
	const R = SKY_RADIUS, x = v[0], y = v[1], z = v[2];
	const p = (1 / (2 * (x * x + y * y + z * z))) * (-2 * z * R + 2 * Math.sqrt(z * z * R * R + 2 * x * x * R * height + x * x * height * height
		+ 2 * y * y * R * height + y * y * height * height + 2 * z * z * R * height + z * z * height * height));
	const px = x * p, py = y * p, pz = z * p + R, l = Math.hypot(px, py, pz);
	return [Math.acos(px / l), Math.acos(py / l)];
}

function buildSky(name) {
	// without a script there is nothing that says what the sky is
	if (!scripts.bodies.has(scKey(name))) return null;
	const mat = materialFor(name, true);
	if (!mat.sky && !mat.stages.length) return null;
	// a box one unit to each side: how large it is drawn the far plane says (drawSky)
	const N = SKY_CUTS, pos = [], st = [], cst = [], idx = [], height = mat.sky ? mat.sky.height : 512;
	for (let side = 0; side < 6; side++) {
		const base = pos.length / 3;
		for (let j = 0; j <= N; j++) {
			for (let i = 0; i <= N; i++) {
				const s = i / N * 2 - 1, t = j / N * 2 - 1, v = skyVec(s, t, side);
				pos.push(v[0], v[1], v[2]);
				st.push((s + 1) / 2, 1 - (t + 1) / 2);
				cst.push(...cloudSt(v, height));
			}
		}
		for (let j = 0; j < N; j++) {
			for (let i = 0; i < N; i++) {
				const a = base + j * (N + 1) + i;
				idx.push(a, a + 1, a + N + 1, a + 1, a + N + 2, a + N + 1);
			}
		}
	}
	const posBuf = gl.createBuffer(), ib = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, posBuf);
	gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(pos), gl.STATIC_DRAW);
	const side = coords => {
		const vao = gl.createVertexArray(), buf = gl.createBuffer();
		gl.bindVertexArray(vao);
		gl.bindBuffer(gl.ARRAY_BUFFER, posBuf);
		gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 0, 0);
		gl.bindBuffer(gl.ARRAY_BUFFER, buf);
		gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(coords), gl.STATIC_DRAW);
		gl.enableVertexAttribArray(1); gl.vertexAttribPointer(1, 2, gl.FLOAT, false, 0, 0);
		gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, ib);
		if (coords === st) gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, new Uint16Array(idx), gl.STATIC_DRAW);
		gl.bindVertexArray(null);
		return vao;
	};
	return {
		// moves: its clouds scroll or pulse, so no two pictures of it are alike (drawWorldMask)
		boxVao: side(st), cloudVao: side(cst), perSide: N * N * 6, stages: mat.stages, moves: matAnimates(mat),
		box: mat.sky && mat.sky.box ? SKY_SUFFIX.map(suffix => Object.assign(newStage(), { maps: [mat.sky.box + '_' + suffix], clamp: true })) : null
	};
}

// The quads of a shader with deformVertexes autosprite or autosprite2, which the engine turns to the viewer in
// every frame (AutospriteDeform and Autosprite2Deform in tr_shade_calc.cpp). What does not depend on the viewer
// is worked out here, once, the rest is done by the vertex shader. Four vertices and six indexes are a quad.
// autosprite: a flare. The quad is made new around its middle, as wide as the old one, the whole image on it.
//   aPos is the middle, aLm how far along the left and the up of the view a corner is.
// autosprite2: a beam or a chain. It turns around its long axis: aPos is the middle of the short edge a vertex is
//   on, aNorm the long axis and aLm.x how far to which side of it the vertex is.
// null: the surfaces are no quads, they stay as they are (the engine warns then).
const EDGE_VERTS = [[0, 1], [0, 2], [0, 3], [1, 2], [1, 3], [2, 3]];

function buildSprites(b, kind) {
	const nv = b.verts, tri = b.tri;
	if ((nv & 3) || tri.length !== (nv >> 2) * 6) return null;
	const read = (buf, into) => { gl.bindBuffer(gl.ARRAY_BUFFER, buf); gl.getBufferSubData(gl.ARRAY_BUFFER, 0, into); return into; };
	const pos = read(b.posBuf, new Float32Array(nv * 3)), st = read(b.stBuf, new Float32Array(nv * 2)), col = read(b.colBuf, new Uint8Array(nv * 4));
	const at = new Float32Array(nv * 3), off = new Float32Array(nv * 2), axis = new Float32Array(nv * 3);
	let idx = tri;
	if (kind === 1) {
		// RB_AddQuadStamp: left and up, right and up, right and down, left and down
		const CORNER = [[1, 1], [-1, 1], [-1, -1], [1, -1]], ST = [0, 0, 1, 0, 1, 1, 0, 1];
		idx = new Uint32Array(tri.length);
		for (let i = 0, q = 0; i < nv; i += 4, q += 6) {
			const mid = [0, 1, 2].map(c => (pos[i * 3 + c] + pos[i * 3 + 3 + c] + pos[i * 3 + 6 + c] + pos[i * 3 + 9 + c]) / 4);
			const radius = Math.hypot(pos[i * 3] - mid[0], pos[i * 3 + 1] - mid[1], pos[i * 3 + 2] - mid[2]) * 0.707;
			for (let j = 0; j < 4; j++) {
				const v = i + j;
				at.set(mid, v * 3);
				off[v * 2] = CORNER[j][0] * radius; off[v * 2 + 1] = CORNER[j][1] * radius;
				st[v * 2] = ST[j * 2]; st[v * 2 + 1] = ST[j * 2 + 1];
				// the colour of the first vertex is the one of the whole quad
				for (let c = 0; c < 4; c++) col[v * 4 + c] = col[i * 4 + c];
			}
			idx.set([i, i + 1, i + 3, i + 3, i + 1, i + 2], q);
		}
	} else {
		at.set(pos);
		for (let i = 0, q = 0; i < nv; i += 4, q += 6) {
			// the two shortest edges are the ends of the quad
			const nums = [0, 0], lengths = [999999, 999999];
			for (let j = 0; j < 6; j++) {
				const a = (i + EDGE_VERTS[j][0]) * 3, c = (i + EDGE_VERTS[j][1]) * 3;
				const l = (pos[a] - pos[c]) ** 2 + (pos[a + 1] - pos[c + 1]) ** 2 + (pos[a + 2] - pos[c + 2]) ** 2;
				if (l < lengths[0]) { nums[1] = nums[0]; lengths[1] = lengths[0]; nums[0] = j; lengths[0] = l; }
				else if (l < lengths[1]) { nums[1] = j; lengths[1] = l; }
			}
			const mid = nums.map(n => [0, 1, 2].map(c => (pos[(i + EDGE_VERTS[n][0]) * 3 + c] + pos[(i + EDGE_VERTS[n][1]) * 3 + c]) / 2));
			const major = [mid[1][0] - mid[0][0], mid[1][1] - mid[0][1], mid[1][2] - mid[0][2]];
			for (let j = 0; j < 2; j++) {
				const v1 = i + EDGE_VERTS[nums[j]][0], v2 = i + EDGE_VERTS[nums[j]][1], l = 0.5 * Math.sqrt(lengths[j]);
				// which way round the edge is used says to which side its ends go
				let k = 0;
				for (; k < 5; k++) if (tri[q + k] === v1 && tri[q + k + 1] === v2) break;
				at.set(mid[j], v1 * 3); at.set(mid[j], v2 * 3);
				off[v1 * 2] = k === 5 ? l : -l;
				off[v2 * 2] = k === 5 ? -l : l;
			}
			for (let j = 0; j < 4; j++) axis.set(major, (i + j) * 3);
		}
	}
	const vao = gl.createVertexArray();
	gl.bindVertexArray(vao);
	const attr = (loc, data, size, type, normalized) => {
		gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
		gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
		gl.enableVertexAttribArray(loc);
		gl.vertexAttribPointer(loc, size, type, normalized, 0, 0);
	};
	attr(0, at, 3, gl.FLOAT, false);
	attr(1, st, 2, gl.FLOAT, false);
	attr(2, off, 2, gl.FLOAT, false);
	attr(3, col, 4, gl.UNSIGNED_BYTE, true);
	attr(4, axis, 3, gl.FLOAT, false);
	gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, gl.createBuffer());
	gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, idx, gl.STATIC_DRAW);
	gl.bindVertexArray(null);
	return { vao, count: idx.length };
}

// ---- the fogs of a map

// a key of the worldspawn, in large or small letters, '' without it
function worldKey(w, key) {
	const first = /\{([^}]*)\}/.exec(w.entityText || ''), k = first && new RegExp('"' + key + '"\\s+"([^"]*)"', 'i').exec(first[1]);
	return k ? k[1] : '';
}

// The brush a fog fills. The map file names it, and so does the page where js/bsp.js keeps that number (f.brush).
// Where it does not: the first brush with the shader of the fog and the side the fog ends at that no fog before
// it took. -1: none.
function fogBrush(w, f, taken) {
	if (f.brush !== undefined) return f.brush;
	const B = w.brushes, P = B.planes, n = f.plane, any = !n.some(v => v !== 0);
	for (let i = 0; i * 3 + 2 < B.brushes.length; i++) {
		const sh = w.shaders[B.brushes[i * 3 + 2]];
		if (taken.has(i) || !sh || sh.name !== f.shader) continue;
		for (let s = B.brushes[i * 3], end = s + B.brushes[i * 3 + 1]; s < end; s++) {
			const p = B.sides[s * 3] * 4;
			if (any || (P[p] === -n[0] && P[p + 1] === -n[1] && P[p + 2] === -n[2] && P[p + 3] === -n[3])) { taken.add(i); return i; }
		}
	}
	return -1;
}

// the pass that draws the fog of a brush onto a surface (RB_FogPass): the image of the fog in its colour, blended
// by how thick the fog is there, where the surface was drawn (equal) or over what is behind it
function fogPassStage(color, equal) {
	return Object.assign(newStage(), { tex: fogTex, tcGen: 3, blend: [gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA], depthWrite: false, depthEqual: equal, rgbGen: CGEN.const, rgbConst: color });
}

// The fogs of the map as R_LoadFogs sets them up: colour and depth from the script of their shader, the box of
// the brush they fill (the whole map without one), and the side they end at.
const WORLD_MAX = 65536, FOG_EXP2 = Math.sqrt(-Math.log(1 / 255));
function prepareFogs(w) {
	const taken = new Set(), B = w.brushes, P = B.planes;
	w.globalFog = null;
	w.fogInfo = w.fogs.map(f => {
		// a shader without fogParms: the engine takes a red fog, 250 deep
		const parms = materialFor(f.shader, 'model').fog || { color: [1, 0, 0], depth: 250 };
		const info = { shader: f.shader, global: false, color: parms.color, depth: parms.depth, tcScale: 1 / (Math.max(1, parms.depth) * 8), plane: f.plane,
			mins: [-WORLD_MAX, -WORLD_MAX, -WORLD_MAX], maxs: [WORLD_MAX, WORLD_MAX, WORLD_MAX], mode: 0, parm: new Float32Array(2), pass: null };
		// the colour of the pass is the one of the fog cut down to bytes (ColorBytes4)
		const bytes = parms.color.map(c => Math.floor(c * 255) / 255);
		info.pass = [fogPassStage(bytes, true), fogPassStage(bytes, false)];
		if (f.global) {
			// the last one of the file is the fog of the whole map (worldData.globalFog)
			if (w.globalFog) w.globalFog.global = false;
			info.global = true;
			w.globalFog = info;
		} else {
			// a brush has its six sides along the axes first
			const brush = fogBrush(w, f, taken), first = brush >= 0 ? B.brushes[brush * 3] : -1;
			if (first >= 0 && (first + 5) * 3 < B.sides.length) {
				for (let i = 0; i < 3; i++) {
					info.mins[i] = -P[B.sides[(first + i * 2) * 3] * 4 + 3];
					info.maxs[i] = P[B.sides[(first + i * 2 + 1) * 3] * 4 + 3];
				}
			}
		}
		return info;
	});
	const g = w.globalFog;
	if (!g) return;
	// The fog of the whole map is the fog of OpenGL (RB_IterateStagesGeneric). GL_EXP2, as thick as leaves 1/255
	// of a colour at the depth of the fog. With "fogstart" in the worldspawn the client game asks for a fog in a
	// range (cg_linearFogOverride, RE_SetRangedFog): GL_LINEAR from there to the distance the map is culled at.
	const start = parseFloat(worldKey(w, 'fogstart')) || 0, cull = parseFloat(worldKey(w, 'distancecull')), end = isFinite(cull) ? cull : 6000;
	if (!start) {
		g.mode = 1;
		g.parm[0] = FOG_EXP2 / g.depth;
	} else {
		// below zero it is the range the scope of a rifle asks for: from the depth of the fog, or nearer where
		// that leaves less than the range before the end
		const from = start > 0 ? start : end - g.depth < -start ? Math.max(16, end + start) : g.depth;
		g.mode = 2;
		g.parm[0] = end;
		g.parm[1] = 1 / (end - from);
	}
}

// The fog an entity is in (R_ComputeFogNum, R_GComputeFogNum, R_SpriteFogNum): the first one of the map whose box
// the ball around it reaches into. The fog of the whole map has a box around everything. null: none.
function fogAt(origin, radius) {
	for (const f of world.fogInfo) {
		// the fog of the whole map can be taken away for a view (a sky portal with "onlyfoghere")
		if (f.global && f !== world.globalFog) continue;
		let j = 0;
		for (; j < 3; j++) if (origin[j] - radius >= f.maxs[j] || origin[j] + radius <= f.mins[j]) break;
		if (j === 3) return f;
	}
	return null;
}

// The batches of the map in the order and with the stages the game draws them.
// The order (R_AddDrawSurf, the sort key): the sort of the shader, then the order in which the shaders were
// registered, then the entity (a brush model that is one before the map itself), then the fog. A shader of the
// engine is a name with a lightmap, registered when the first surface that has both is loaded (R_LoadSurfaces),
// and the batches of the map are in the order of their first surface: so that order is the place of the first
// batch with the same name and lightmap. Not known here: the number the game gives an entity, the brush models
// are taken in the order of the map file.
function prepareScripted(w) {
	if (scripts.state !== 2 || !w) return;
	prepareFogs(w);
	const list = [], first = new Map();
	w.batches.forEach((b, i) => {
		let byLm = first.get(b.shaderName);
		if (!byLm) first.set(b.shaderName, byLm = new Map());
		if (!byLm.has(b.lm)) byLm.set(b.lm, i);
		// a shader that is swapped is drawn with the script of what it is swapped for, in its own place and with
		// its own lightmap (js/editor-remaps.js)
		const mat = materialFor(remapOf(b.shaderName), !!b.lm), order = byLm.get(b.lm), fog = (b.fog >= 0 && w.fogInfo[b.fog]) || null;
		if (mat.order > order) mat.order = order;
		// a shader without stages is still drawn where it is the surface of a fog: as that fog (SS_FOG)
		if (mat.nodraw || (!mat.stages.length && !(mat.fogPass && fog && !fog.global))) return;
		const e = { b, mat, i, order, fog, vao: b.vao, count: b.count, sprite: 0 };
		const kind = mat.deforms.some(d => d.kind === 'autosprite') ? 1 : mat.deforms.some(d => d.kind === 'autosprite2') ? 2 : 0;
		if (kind) {
			if (b.sprites === undefined) b.sprites = buildSprites(b, kind);
			if (b.sprites) Object.assign(e, { vao: b.sprites.vao, count: b.sprites.count, sprite: kind });
		}
		list.push(e);
	});
	list.sort((x, y) => x.mat.sort - y.mat.sort || x.order - y.order || !x.b.model - !y.b.model || x.b.model - y.b.model || x.b.fog - y.b.fog || x.i - y.i);
	w.skyDraw = w.sky ? buildSky(remapOf(w.sky)) || buildSky(w.sky) : null;

	w.scripted = list;
}

const WAVE_FUNC = { sin: 1, triangle: 2, square: 3, sawtooth: 4, inversesawtooth: 5 };
const dfWave = new Float32Array(4), dfBulge = new Float32Array(4), dfMove = new Float32Array(3);

// the deformVertexes of a shader for the program that draws it next, at the time t
function setDeform(prog, mat, t) {
	// most have none: nothing to send when the program has none set either
	if (!mat.deforms.length && !prog.deformed) return;
	let func = 0;
	dfWave.fill(0); dfBulge.fill(0); dfMove.fill(0);
	for (const d of mat.deforms) {
		if (d.kind === 'wave') {
			const x = d.wave.phase + t * d.wave.freq;
			func = WAVE_FUNC[d.wave.func] || 1;
			// a wave that does not run is the same for every vertex
			dfWave[0] = d.wave.freq ? d.spread : 0; dfWave[1] = d.wave.base; dfWave[2] = d.wave.amp; dfWave[3] = x - Math.floor(x);
		} else if (d.kind === 'bulge') {
			dfBulge[0] = d.width; dfBulge[1] = d.height; dfBulge[2] = (t * d.speed) % (Math.PI * 2); dfBulge[3] = d.width === 0 && d.speed === 0 ? 1 : 0;
		} else if (d.kind === 'move') {
			const by = waveValue(d.wave, t);
			for (let i = 0; i < 3; i++) dfMove[i] += d.vec[i] * by;
		}
	}
	prog.deformed = mat.deforms.length > 0;
	gl.uniform1i(prog.u.uWaveFunc, func);
	gl.uniform4fv(prog.u.uWave, dfWave);
	gl.uniform4fv(prog.u.uBulge, dfBulge);
	gl.uniform3fv(prog.u.uMove, dfMove);
}

// where the camera looks, and its left and its up, in the map
function cameraAxes() {
	const at = (x, y, z) => { const v = M4.transform(invViewProj, [x, y, z]); return [v[0] / v[3], v[1] / v[3], v[2] / v[3]]; };
	const from = (a, b) => { const d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]], l = Math.hypot(d[0], d[1], d[2]) || 1; return [d[0] / l, d[1] / l, d[2] / l]; };
	const o = at(0, 0, 0);
	return { fwd: from(at(0, 0, -1), at(0, 0, 1)), left: from(o, at(-1, 0, 0)), up: from(o, at(0, 1, 0)) };
}

// How far away the far plane of the camera is, from its matrix: the row of the depth is a times the row below
// it, which is the depth before the planes come into it, plus b, and the far plane is at b / (a + 1).
function cameraFar() {
	const m = viewProj, a = -(m[2] * m[3] + m[6] * m[7] + m[10] * m[11]) / (m[3] * m[3] + m[7] * m[7] + m[11] * m[11]);
	return (m[14] + a * m[15]) / (a + 1);
}

// ---- for the comparison with the game: what of the map animates on its own

// A shader that looks different from one moment to the next whatever the view: images that scroll, turn or
// change, colours that follow a wave, vertices that move, the clouds of a sky. The picture of the page and the
// one of the game are never taken at the very same time, so these surfaces are left out of the comparison.
const stageAnimates = st => st.maps.length > 1 || st.rgbGen === CGEN.wave || st.alphaGen === AGEN.wave || st.tcMods.some(m => m.kind !== 'scale' && m.kind !== 'transform');
function matAnimates(mat) {
	return mat.deforms.length > 0 || mat.stages.some(stageAnimates);
}
let refMasking = false;
function refMask(on) { refMasking = !!on; }
// the map in two colours: white where what is drawn animates, black where it does not
function drawWorldMask(now) {
	// Where nothing of the map is drawn the sky shows: left out where its clouds move, compared where it is a
	// box of images or nothing at all.
	const sky = world.skyDraw && world.skyDraw.moves ? 1 : 0;
	gl.clearColor(sky, sky, sky, 1);
	gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
	gl.enable(gl.DEPTH_TEST);
	gl.disable(gl.BLEND);
	gl.depthMask(true);
	gl.disable(gl.CULL_FACE);
	gl.useProgram(lineProg.p);
	const list = world.scripted || world.batches.map(b => ({ b, mat: null, vao: b.vao, count: b.count }));
	// what is solid first and into the depth, then what is see-through over it without
	for (const late of [false, true]) {
		for (const e of list) {
			const blended = !!e.mat && e.mat.sort > SC_SORT_SEE_THROUGH;
			if (blended !== late || (e.mat && e.mat.nodraw)) continue;
			const moves = !!e.mat && (matAnimates(e.mat) || !!e.mat.sky);
			if (late && !moves) continue;
			gl.depthMask(!late);
			gl.uniform4f(lineProg.u.uColor, moves ? 1 : 0, moves ? 1 : 0, moves ? 1 : 0, 1);
			gl.bindVertexArray(e.vao);
			for (const place of moverPlaces(e.b.model, now)) {
				gl.uniformMatrix4fv(lineProg.u.uMvp, false, place ? M4.mul(viewProj, place) : viewProj);
				gl.drawElements(gl.TRIANGLES, e.count, gl.UNSIGNED_INT, 0);
			}
		}
	}
	gl.depthMask(true);
	// The client draws a small icon into the top left corner also without its HUD. It is no part of the scene.
	gl.enable(gl.SCISSOR_TEST);
	gl.scissor(0, canvas.height - 40, 56, 40);
	gl.clearColor(1, 1, 1, 1);
	gl.clear(gl.COLOR_BUFFER_BIT);
	gl.disable(gl.SCISSOR_TEST);
	gl.clearColor(0.05, 0.05, 0.06, 1);
}

// A sky portal was drawn for this frame (drawSkyPortal in js/world-ents.js): the sky of the map is left as it is.
let skyPortalOn = false;
// Which side of a surface is drawn (GL_Cull in tr_backend.cpp): 0 the front, 1 the back, 2 both. The front of a
// triangle of the game is the side its corners run clockwise on. Seen from outside, a room is open that way:
// its walls and its ceiling face inwards.
let cullSides = true;
function setCull(side) {
	if (side === 2 || !cullSides) { gl.disable(gl.CULL_FACE); return; }
	gl.enable(gl.CULL_FACE);
	gl.frontFace(gl.CW);
	gl.cullFace(side === 1 ? gl.FRONT : gl.BACK);
}

// ---- a stage drawn

const FOG_BLACK = [0, 0, 0], FOG_WHITE = [1, 1, 1];
const stTcA = new Float32Array(3), stTcB = new Float32Array(3), stTurb = new Float32Array(2), stFogPlane = new Float32Array(4);
// The colour of the entity that is drawn (shaderRGBA), for rgbGen entity and alphaGen entity: white unless the
// one who draws says otherwise (drawModelScripted).
const stEntity = [1, 1, 1, 1], NO_ENTITY = [0, 0, 0, 0];
// The map itself is drawn. Its entity is the world of the engine (tr.worldEntity), which has no colour and no
// light of a place: what asks for either is black there, as in the game.
let stWorld = false;
// where the brush model that is drawn stands (its matrix), null for the map as the file has it
let stPlace = null;
// What the entity that is drawn asks of every stage, whatever the stage says (renderfx of its refEntity_t,
// RB_IterateStagesGeneric). stTint (RF_RGB_TINT): its colour and not the rgbGen of the stage. stForceAlpha
// (RF_FORCE_ENT_ALPHA): blended over what is there by its alpha, with no alpha test and without writing depth.
let stTint = false, stForceAlpha = false;
// The frame the entity that is drawn gives its shaders (RF_SETANIMINDEX with skinNum in R_BindAnimatedImage: a
// mover with EF_SHADER_ANIM, CG_Mover). -1: an animMap shows the image the time gives.
let stFrame = -1;

// the images a stage names, asked for the first time they are wanted
const stageImages = st => st.images || (st.images = st.maps.map(name => gameImage(name, st.clamp)));

// The image a stage shows at the time t. null: it has none, or it is not there (yet) and the stage would lie over
// what is drawn (blended): grey where it covers something, nothing where it would lie over it.
function stageTexture(st, t, lm, blended) {
	if (st.lightmap) return lm || whiteTex;
	if (st.white) return whiteTex;
	if (st.tex) return st.tex;
	const images = stageImages(st), n = images.length, frame = stFrame >= 0 ? stFrame : Math.floor(t * st.animFreq);
	if (!n) return null;
	const h = images[n < 2 ? 0 : st.oneShot ? Math.min(n - 1, Math.max(0, frame)) : ((frame % n) + n) % n];
	return !h.ready && blended ? null : h.tex;
}

// the tcMods of a stage at the time t as one matrix, each applied to what the ones before it made
function stageTcMods(st, t) {
	let a = 1, b = 0, c = 0, d = 0, e = 1, f = 0;
	stTurb[0] = 0;
	for (const m of st.tcMods) {
		const n = m.n;
		let k;
		if (m.kind === 'scroll') { const x = n[0] * t, y = n[1] * t; k = [1, 0, x - Math.floor(x), 0, 1, y - Math.floor(y)]; }
		else if (m.kind === 'scale') k = [n[0], 0, 0, 0, n[1], 0];
		else if (m.kind === 'rotate') { const r = -n[0] * t * Math.PI / 180, cs = Math.cos(r), sn = Math.sin(r); k = [cs, -sn, 0.5 - 0.5 * cs + 0.5 * sn, sn, cs, 0.5 - 0.5 * sn - 0.5 * cs]; }
		else if (m.kind === 'stretch') { const w = waveValue(m.wave, t), p = w ? 1 / w : 1; k = [p, 0, 0.5 - 0.5 * p, 0, p, 0.5 - 0.5 * p]; }
		else if (m.kind === 'transform') k = [n[0], n[2], n[4], n[1], n[3], n[5]];
		else { stTurb[0] = n[1]; stTurb[1] = n[2] + t * n[3]; continue; }
		[a, b, c, d, e, f] = [k[0] * a + k[1] * d, k[0] * b + k[1] * e, k[0] * c + k[1] * f + k[2], k[3] * a + k[4] * d, k[3] * b + k[4] * e, k[3] * c + k[4] * f + k[5]];
	}
	stTcA[0] = a; stTcA[1] = b; stTcA[2] = c; stTcB[0] = d; stTcB[1] = e; stTcB[2] = f;
}

// What a program was last sent of the uniforms of a stage. One stage is mostly like the one before it, and a
// uniform is only sent when it changes.
const stageSet = prog => prog.set || (prog.set = { color: [-1, 0, 0, 0] });
function stageInt(prog, name, v) {
	if (prog.set[name] !== v) { prog.set[name] = v; gl.uniform1i(prog.u[name], v); }
}

// What a program needs before the stages of a view are drawn with it: how thick the fog of the whole map is, and
// an image on the second unit also where no stage has two.
function stageBegin(prog, globalFog) {
	stageSet(prog);
	if (globalFog) gl.uniform2fv(prog.u.uFogParm, globalFog.parm);
	gl.activeTexture(gl.TEXTURE1);
	gl.bindTexture(gl.TEXTURE_2D, whiteTex);
	gl.activeTexture(gl.TEXTURE0);
}

// Where the fog of a brush is, for the vertex shader (RB_CalcFogTexCoords): the side it ends at and how far
// behind that side the eye is. For a brush model that an entity moved the side is turned into its space.
function stageFogPlane(prog, fog) {
	const set = prog.set, place = prog === stageProg ? stPlace : null, n = fog.plane;
	if (set.fog === fog && set.fogPlace === place && set.fogX === eye[0] && set.fogY === eye[1] && set.fogZ === eye[2]) return;
	set.fog = fog; set.fogPlace = place; set.fogX = eye[0]; set.fogY = eye[1]; set.fogZ = eye[2];
	if (place) {
		const m = place;
		stFogPlane[0] = m[0] * n[0] + m[1] * n[1] + m[2] * n[2];
		stFogPlane[1] = m[4] * n[0] + m[5] * n[1] + m[6] * n[2];
		stFogPlane[2] = m[8] * n[0] + m[9] * n[1] + m[10] * n[2];
		stFogPlane[3] = n[3] - (m[12] * n[0] + m[13] * n[1] + m[14] * n[2]);
	} else {
		stFogPlane.set(n);
	}
	gl.uniform4fv(prog.u.uFogPlane, stFogPlane);
	gl.uniform2f(prog.u.uFogSt, fog.tcScale, eye[0] * n[0] + eye[1] * n[1] + eye[2] * n[2] - n[3]);
}

// One stage of a shader drawn (RB_IterateStagesGeneric with ComputeColors and ComputeTexCoords). lm: the lightmap
// of the surface. fog: the fog the surface is in (an entry of world.fogInfo), null for none.
function drawStage(st, t, lm, vao, count, type, offset, prog = stageProg, fog = null) {
	// the second of two stages the engine draws as one was drawn with the first
	if (st.merged) return;
	const two = st.multi, tex = stageTexture(st, t, lm, st.blend), tex2 = two ? stageTexture(two, t, lm, st.blend) : null;
	if (!tex || (two && !tex2)) return;

	if (stForceAlpha) {
		gl.enable(gl.BLEND); gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
		gl.depthMask(false);
		gl.depthFunc(gl.LEQUAL);
	} else {
		if (st.blend) { gl.enable(gl.BLEND); gl.blendFunc(st.blend[0], st.blend[1]); } else gl.disable(gl.BLEND);
		gl.depthMask(st.depthWrite);
		gl.depthFunc(st.depthEqual ? gl.EQUAL : gl.LEQUAL);
	}

	const u = prog.u, set = stageSet(prog);
	stageInt(prog, 'uTcGen', st.tcGen);
	// most stages have no tcMod: nothing to send while the program has none either
	if (st.tcMods.length || set.tc !== 0) {
		stageTcMods(st, t);
		gl.uniform3fv(u.uTcA, stTcA); gl.uniform3fv(u.uTcB, stTcB); gl.uniform2fv(u.uTurb, stTurb);
		set.tc = st.tcMods.length;
	}
	stageInt(prog, 'uMulti', two ? st.env : 0);
	if (two) {
		stageInt(prog, 'uTcGen2', two.tcGen);
		if (two.tcMods.length || set.tc2 !== 0) {
			stageTcMods(two, t);
			gl.uniform3fv(u.uTcA2, stTcA); gl.uniform3fv(u.uTcB2, stTcB); gl.uniform2fv(u.uTurb2, stTurb);
			set.tc2 = two.tcMods.length;
		}
		gl.activeTexture(gl.TEXTURE1);
		gl.bindTexture(gl.TEXTURE_2D, tex2);
		gl.activeTexture(gl.TEXTURE0);
	}

	// The colour that is the same for every vertex. What the vertices bring is the vertex shader's (rgbMode and
	// alphaMode of the stage).
	let r = 1, g = 1, b = 1, a = 1;
	const cg = stTint ? CGEN.entity : st.rgbGen, ag = stForceAlpha ? AGEN.entity : st.alphaGen, model = prog === modelStageProg, dark = stWorld && !model, ent = dark ? NO_ENTITY : stEntity;
	if (cg === CGEN.wave) r = g = b = Math.min(1, Math.max(0, waveValue(st.rgbWave, t)));
	else if (cg === CGEN.const) { r = st.rgbConst[0]; g = st.rgbConst[1]; b = st.rgbConst[2]; }
	else if (cg === CGEN.entity || cg === CGEN.lightingdiffuseentity) { r = ent[0]; g = ent[1]; b = ent[2]; }
	else if (cg === CGEN.oneminusentity) { r = 1 - ent[0]; g = 1 - ent[1]; b = 1 - ent[2]; }
	// RB_CalcDiffuseColor with the light of the world: black, and an alpha of nothing where no alphaGen follows
	else if (cg === CGEN.lightingdiffuse && dark) r = g = b = a = 0;
	if (ag === AGEN.const) a = st.alphaConst;
	else if (ag === AGEN.wave) a = Math.min(1, Math.max(0, waveValue(st.alphaWave, t)));
	else if (ag === AGEN.entity) a = ent[3];
	else if (ag === AGEN.oneminusentity) a = 1 - ent[3];
	else if (ag !== AGEN.skip) a = 1;
	const c = set.color;
	if (c[0] !== r || c[1] !== g || c[2] !== b || c[3] !== a) { c[0] = r; c[1] = g; c[2] = b; c[3] = a; gl.uniform4f(u.uColor, r, g, b, a); }
	// a model has no colours at its vertices, and what is no model no light of a place: white for a stage that asks
	stageInt(prog, 'uRgbVertex', !stTint && (st.rgbMode === 3) === model ? st.rgbMode : 0);
	stageInt(prog, 'uAlphaVertex', stForceAlpha || (model && st.alphaMode === 2) ? 0 : st.alphaMode);
	if (st.alphaMode === 4 && set.portal !== st.portalRange) { set.portal = st.portalRange; gl.uniform1f(u.uPortalRange, st.portalRange || 256); }
	stageInt(prog, 'uAlphaFunc', stForceAlpha ? 0 : st.alphaFunc);

	// The fog of the whole map is the fog of OpenGL, on every shader that takes a fog: the stage fades to the
	// colour of the fog, or to black where it adds to what is there and to white where it multiplies it
	// (mGLFogColorOverride), so that what is under it comes out as the colour of the fog.
	const to = fog && fog.global && st.fogPass ? (st.fogOverride === 1 ? FOG_BLACK : st.fogOverride === 2 ? FOG_WHITE : fog.color) : null;
	const mode = to ? fog.mode : 0;
	if (set.fogTo !== to || set.fogMode !== mode) {
		set.fogTo = to; set.fogMode = mode;
		if (to) gl.uniform4f(u.uFog, to[0], to[1], to[2], mode); else gl.uniform4f(u.uFog, 0, 0, 0, 0);
	}
	// in any fog, what blends fades out with it (adjustColorsForFog)
	const adjust = fog ? st.adjustFog : 0;
	stageInt(prog, 'uFogAdj', adjust);
	if (adjust || (fog && st.tcGen === 3)) stageFogPlane(prog, fog);

	gl.bindTexture(gl.TEXTURE_2D, tex);
	gl.bindVertexArray(vao);
	if (st.noDepth) gl.disable(gl.DEPTH_TEST);
	gl.drawElements(gl.TRIANGLES, count, type, offset);
	if (st.noDepth) gl.enable(gl.DEPTH_TEST);
}

function drawSky(sky, t, fog) {
	// Around the eye wherever it is, behind everything, and as large as the far plane lets it be (MakeSkyVec in
	// tr_sky.cpp): how far away that is says how much of a fog lies on the clouds.
	const size = cameraFar() / 1.75;
	gl.uniformMatrix4fv(stageProg.u.uMvp, false, M4.mul(viewProj, new Float32Array([size, 0, 0, 0, 0, size, 0, 0, 0, 0, size, 0, eye[0], eye[1], eye[2], 1])));
	gl.disable(gl.DEPTH_TEST);
	// the sky has no colours at its vertices: white for a stage that asks for them
	gl.vertexAttrib4f(3, 1, 1, 1, 1);
	// the box is drawn as its images are, also in a fog (DrawSkyBox)
	if (sky.box) {
		for (let side = 0; side < 6; side++) drawStage(sky.box[side], t, null, sky.boxVao, sky.perSide, gl.UNSIGNED_SHORT, side * sky.perSide * 2);
	}
	// The clouds are on every side but the one below, and they are drawn as every surface is
	// (RB_StageIteratorGeneric): with the fog of the whole map on them.
	for (const st of sky.stages) {
		drawStage(st, t, null, sky.cloudVao, sky.perSide * 5, gl.UNSIGNED_SHORT, 0, stageProg, fog);
		if (st.noDepth) gl.disable(gl.DEPTH_TEST);
	}
	gl.enable(gl.DEPTH_TEST);
}

// Fog is drawn where the camera is in the map: flying, through the eyes of a player, or where a client of the
// game is (the comparison). Looking at the map from above it would hide the map. fogOff takes it away for a
// view (the map outside of a sky portal with "onlyfoghere", RDF_NOFOG).
let fogOff = false;
const fogShown = () => !fogOff && (cam.fly || view.id >= 0 || !!refCam);

// The map with its scripts, in two parts as the sort of the game has it: the sky and everything solid first,
// then, after the players and the models were drawn, what is see-through (late), so that glass and water lie
// over who stands behind them.
function drawWorldScripted(now, late) {
	const t = now / 1000, open = viewCulls();
	gl.useProgram(stageProg.p);
	gl.uniform1i(stageProg.u.uTex, 0);
	gl.uniform3fv(stageProg.u.uEye, eye);
	gl.activeTexture(gl.TEXTURE0);
	gl.disable(gl.CULL_FACE);
	// the alpha of the canvas stays 1 whatever a stage blends, or the page would shine through
	gl.colorMask(true, true, true, false);
	setDeform(stageProg, NO_DEFORM, t);
	gl.uniform1i(stageProg.u.uSprite, 0);
	const fogged = fogShown(), globalFog = fogged ? world.globalFog : null;
	stageBegin(stageProg, globalFog);
	stPlace = null;
	// with a sky portal the picture of its room is what shows where the sky is
	// (the frame begins as the colour of a fog of the whole map: drawSkyPortal in js/world-ents.js does that clear)
	if (!late && !skyPortalOn && world.skyDraw) drawSky(world.skyDraw, t, globalFog);
	gl.uniformMatrix4fv(stageProg.u.uMvp, false, viewProj);
	let sprite = 0, axes = null, placed = null;
	stWorld = true;
	for (const e of world.scripted) {
		if ((e.mat.sort > SC_SORT_BANNER) !== late) continue;
		// what a model has that the game draws before this comes first
		if (late && lateModels.length && drawLateModels(e.mat.sort, e.order)) {
			gl.useProgram(stageProg.p);
			gl.colorMask(true, true, true, false);
		}
		// a brush model that is an entity: where that is, and not at all without one
		const places = moverPlaces(e.b.model, now);
		if (!places.length) continue;
		const fog = !fogged || (e.fog && e.fog.global && !globalFog) ? null : e.fog;
		// the fog of a brush is a pass of its own over what was just drawn, on every shader that takes a fog
		const pass = fog && !fog.global && e.mat.fogPass ? fog.pass[e.mat.fogPass - 1] : null;
		// a quad that turns to the viewer has no back
		// both sides where the editor is not set to "Open"
		setCull(e.sprite || !open ? 2 : e.mat.cull);
		if (e.sprite && !axes) {
			axes = cameraAxes();
			gl.uniform3fv(stageProg.u.uViewLeft, axes.left);
			gl.uniform3fv(stageProg.u.uViewUp, axes.up);
			gl.uniform3fv(stageProg.u.uViewFwd, axes.fwd);
		}
		if (e.sprite !== sprite) gl.uniform1i(stageProg.u.uSprite, sprite = e.sprite);
		setDeform(stageProg, e.mat, t);
		if (e.mat.polygonOffset) { gl.enable(gl.POLYGON_OFFSET_FILL); gl.polygonOffset(-1, -2); }
		// as often as an entity shows the brush model, each with the frame that entity gives its shaders
		const shown = e.b.model && movers ? movers.get(e.b.model) : null;
		for (let i = 0; i < places.length; i++) {
			const place = places[i];
			stFrame = shown && shown[i] ? shown[i].frame : -1;
			if (place !== placed) {
				placed = stPlace = place;
				gl.uniformMatrix4fv(stageProg.u.uMvp, false, place ? M4.mul(viewProj, place) : viewProj);
				gl.uniform3fv(stageProg.u.uEye, place ? eyeAt(place) : eye);
			}
			for (const st of e.mat.stages) drawStage(st, t, e.b.lm, e.vao, e.count, gl.UNSIGNED_INT, 0, stageProg, fog);
			if (pass) drawStage(pass, t, null, e.vao, e.count, gl.UNSIGNED_INT, 0, stageProg, fog);
		}
		if (e.mat.polygonOffset) gl.disable(gl.POLYGON_OFFSET_FILL);
	}
	stPlace = null;
	stFrame = -1;
	stWorld = false;
	if (late) { drawLateModels(Infinity); deferLate = false; }
	gl.disable(gl.CULL_FACE);
	gl.disable(gl.BLEND);
	gl.depthMask(true);
	gl.depthFunc(gl.LESS);
	gl.colorMask(true, true, true, true);
}
