// From VisionJK's web admin page (codemp/game/webadmin/page/js/map.js at 50d7c18b, GPL-2.0, by Blackwolf;
// the panel after JKAdmin's by Lixsl), for SJK's browser map editor. SJK: changed: the batches keep those ranges and their vertices' places (pos, for the selection's outline), and a patch a map edit hides keeps its triangles without an area.
/* ------------------------------------------------------------------ */
/* map                                                                 */
/* ------------------------------------------------------------------ */

let world = null;		// { batches: [{vao, count, tex, lm, alpha}], bounds, pick }
let loadedMap = '';
// how long the last map took here, in milliseconds: reading the file, making the meshes and the lightmaps, and
// the images that were handed to the graphics card since (their number, and the time spent on it)
const loadTimes = { parse: 0, meshes: 0, images: 0, imageCount: 0 };

async function loadMap(name) {
	loadedMap = name;
	$('loading').style.display = '';
	$('loading').textContent = 'Loading ' + name + '...';
	world = null;
	// the images and the stages that hold them are those of a map: the ones of the last map are let go
	gameImages.clear();
	imageQueue.length = 0;
	scripts.materials.clear();
	try {
		const res = await fetch('api/map?name=' + encodeURIComponent(name));
		if (!res.ok) throw new Error('HTTP ' + res.status);
		const data = await res.arrayBuffer(), t0 = performance.now();
		const bsp = parseBsp(data);
		if (loadedMap !== name) return;
		const t1 = performance.now();

		// R_LoadLightmaps: no mipmaps, and the edge is not wrapped around
		const lightmapTex = bsp.lightmaps.map(d => makeTexture(128, 128, d, { rgb: true, clamp: true, mip: false }));
		const textures = new Map(), grids = [];
		const all = bsp.batches.map(b => {
			const vao = gl.createVertexArray();
			gl.bindVertexArray(vao);
			const attr = (loc, data, size, type, normalized) => {
				const buf = gl.createBuffer();
				gl.bindBuffer(gl.ARRAY_BUFFER, buf);
				gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
				gl.enableVertexAttribArray(loc);
				gl.vertexAttribPointer(loc, size, type, normalized, 0, 0);
				return buf;
			};
			const posBuf = attr(0, b.pos, 3, gl.FLOAT, false);
			const stBuf = attr(1, b.st, 2, gl.FLOAT, false);
			attr(2, b.lm, 2, gl.FLOAT, false);
			const colBuf = attr(3, b.col, 4, gl.UNSIGNED_BYTE, true);
			attr(4, b.norm, 3, gl.FLOAT, false);
			const tri = b.idx, idxBuf = gl.createBuffer();
			gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, idxBuf);
			// the triangles of a patch change with how far away it is (updateGridLod)
			gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, tri, b.grids.length ? gl.DYNAMIC_DRAW : gl.STATIC_DRAW);
			gl.bindVertexArray(null);
			const shaderName = bsp.shaders[b.shader].name;
			if (!b.sky && !textures.has(shaderName)) textures.set(shaderName, gameImage(shaderName, false));
			for (const g of b.grids) { g.idxBuf = idxBuf; grids.push(g); }
			// posBuf and tri: the wireframe of the editor makes its lines of them when it is asked for. With stBuf,
			// colBuf and verts they are also what the quads that turn to the viewer are made of (buildSprites).
			// tri has every triangle of a patch, whatever of it is drawn at the moment.
			return { vao, count: tri.length, ranges: b.ranges, pos: b.pos, shaderName, fog: b.fog, model: b.model, lm: b.lightmap >= 0 ? lightmapTex[b.lightmap] : null, posBuf, stBuf, colBuf, idxBuf, verts: b.verts, tri, wire: null, sprites: undefined, sky: b.sky };
		});
		// what the map file marks as sky waits for the scripts (settleSky)
		const batches = all.filter(b => !b.sky);
		loadTimes.parse = t1 - t0; loadTimes.meshes = performance.now() - t1; loadTimes.images = 0; loadTimes.imageCount = 0;

		world = { tree: bspTree(data, bsp.brushes.planes), entityText: bsp.entityText, batches, textures, bounds: bsp.bounds, pick: bsp.pick, sky: bsp.sky, models: bsp.models, brushes: bsp.brushes, shaders: bsp.shaders, surfs: bsp.surfs, stats: bsp.stats, grid: bsp.grid, fogs: bsp.fogs, fogInfo: [], globalFog: null, wires: new Map(), scripted: null, skyDraw: null,
			grids, flares: bsp.flares, flareDraw: null, skyBatches: all.filter(b => b.sky), skySettled: false };
		settleSky(world);
		prepareScripted(world);
		resetCamera();
		$('loading').style.display = 'none';
	} catch (e) {
		$('loading').textContent = 'Could not load the map: ' + e.message;
	}
}

// The sky is what the script of a shader says it is (skyParms), not what the map file marks as sky: the engine
// draws a surface as sky when its shader has skyParms and as a surface like any other when it has none
// (mp/ffa4 has such: black walls that the fog of the map colours). So once the scripts are there, the batches
// that were kept apart for the mark become batches of the map where the script is no sky, and the sky of the
// map is the first shader that is one. true: the map has more batches now.
function settleSky(w) {
	if (w.skySettled || scripts.state !== 2) return false;
	w.skySettled = true;
	const isSky = b => !!materialFor(b.shaderName, !!b.lm).sky;
	const plain = w.skyBatches.filter(b => !isSky(b)), sky = w.skyBatches.find(isSky);
	w.sky = sky ? sky.shaderName : '';
	for (const b of plain) {
		if (!w.textures.has(b.shaderName)) w.textures.set(b.shaderName, gameImage(b.shaderName, false));
		w.batches.push(b);
	}
	return plain.length > 0;
}

// ---- how much of a curved surface is drawn (RB_SurfaceGrid and LodErrorForVolume in rd-vanilla/tr_surface.cpp)

const LOD_CURVE_ERROR = 250;		// r_lodCurveError
const lodCols = new Uint8Array(GRID_MAX), lodRows = new Uint8Array(GRID_MAX), lodIdx = new Uint32Array((GRID_MAX - 1) * (GRID_MAX - 1) * 6);

// A row or a column of a grid is drawn while the error it stands for (see subdividePatch) is not above
// r_lodCurveError over the distance: how far in front of the eye the middle of its group of patches is, less the
// radius of that group, and never less than 1. So a pipe has fewer sides the further away it is, exactly as in
// the game. Called once a frame: a grid is only touched when the camera has moved it over one of its limits,
// and then the triangles it keeps are written into its part of the index list. That part stays as long as it
// is: what is left over are triangles without an area.
// look false: every row and column, for the plain view of the editor.
function updateGridLod(now, look) {
	if (!world || !world.grids.length || !viewProj) return;
	const m = viewProj;
	let bound = null;
	for (const g of world.grids) {
		let error = 1e30;
		if (look) {
			let x = g.lodOrigin[0], y = g.lodOrigin[1], z = g.lodOrigin[2];
			// a brush model that is an entity: where that is
			const p = g.model ? moverPlaces(g.model, now)[0] : null;
			if (p) { const lx = x, ly = y, lz = z; x = p[0] * lx + p[4] * ly + p[8] * lz + p[12]; y = p[1] * lx + p[5] * ly + p[9] * lz + p[13]; z = p[2] * lx + p[6] * ly + p[10] * lz + p[14]; }
			// the last row of the matrix is the distance along the line of sight
			const d = Math.abs(m[3] * x + m[7] * y + m[11] * z + m[15]) - g.lodRadius;
			error = LOD_CURVE_ERROR / (d < 1 ? 1 : d);
		}
		if (error >= g.low && error < g.high) continue;
		// SJK: a patch a map edit hides keeps its triangles without an area (sjkHideSurfaces)
		if (sjkHidden.has(g.surface)) continue;
		let nc = 1, nr = 1, low = 0, high = Infinity;
		for (let i = 1; i < g.width - 1; i++) { const e = g.widthLodError[i]; if (e <= error) { lodCols[nc++] = i; if (e > low) low = e; } else if (e < high) high = e; }
		for (let i = 1; i < g.height - 1; i++) { const e = g.heightLodError[i]; if (e <= error) { lodRows[nr++] = i; if (e > low) low = e; } else if (e < high) high = e; }
		lodCols[nc++] = g.width - 1; lodRows[nr++] = g.height - 1;
		g.low = low; g.high = high; g.cols = nc; g.rows = nr;
		const all = (g.width - 1) * (g.height - 1) * 6;
		lodIdx.fill(g.base, gridIndexes(lodIdx, 0, g.base, g.width, lodCols, nc, lodRows, nr), all);
		if (bound !== g.idxBuf) {
			// the list of a vertex array is bound with it: none is, so that none is changed
			if (!bound) gl.bindVertexArray(null);
			gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, bound = g.idxBuf);
		}
		gl.bufferSubData(gl.ELEMENT_ARRAY_BUFFER, g.first * 4, lodIdx, 0, all);
	}
}
// ---- the images of the game as textures

// The images the engine makes no mipmaps for: the ones a stage names after its shader has said nomipmaps
// (ParseShader and ParseStage in tr_shader.cpp). The parser of the scripts does not keep that word, so the texts
// are gone through for it here, once they are there.
let noMipImages = null;
function imageHasMips(name) {
	if (!noMipImages) {
		if (scripts.state !== 2) return true;
		noMipImages = new Set();
		for (const body of scripts.bodies.values()) {
			if (!/nomipmaps/i.test(body)) continue;
			let depth = 0, off = false;
			for (const a of scriptLines(body)) {
				const k = a[0].toLowerCase();
				if (k === '{') depth++;
				else if (k === '}') depth--;
				else if (depth === 0 && k === 'nomipmaps') off = true;
				else if (depth === 1 && off && /^(clamp)?map$|animmap$/.test(k)) for (const image of a.slice(k.endsWith('animmap') ? 2 : 1)) noMipImages.add(scKey(image));
			}
		}
	}
	return !noMipImages.has(scKey(name));
}

// The file of an image of the game, by its name with or without an ending: the ending the name has first, then
// jpg, png and tga, the order of the engine (R_LoadImage). { base: the name without an ending, file: with the
// one it has, tga, img: what decodeTga gives for a tga and a bitmap for the others, w, h }. null: there is none,
// or the server does not hand out its folder.
async function fetchImageFile(name) {
	const m = /\.(tga|jpg|png)$/i.exec(name), base = m ? name.slice(0, m.index) : name;
	const endings = [m ? m[0].toLowerCase() : '', '.jpg', '.png', '.tga'].filter((e, i, all) => e && all.indexOf(e) === i);
	let unanswered = false;
	for (const ext of endings) {
		try {
			let res, data;
			try {
				res = await fetch('api/file?name=' + encodeURIComponent(base + ext));
				if (!res.ok) { if (res.status !== 404 && res.status !== 400) unanswered = true; continue; }
				data = await res.arrayBuffer();
			} catch (e) {
				unanswered = true;
				continue;
			}
			// as it is in the file: the stages of a shader blend with the alpha themselves
			const tga = ext === '.tga', img = tga ? decodeTga(data) : await createImageBitmap(new Blob([data]), { premultiplyAlpha: 'none', colorSpaceConversion: 'none' });
			if (!img) continue;
			return { base, file: base + ext, tga, img, w: tga ? img.w : img.width, h: tga ? img.h : img.height };
		} catch (e) { /* a file that is no image: the next ending */ }
	}
	// the server did not answer (it changes the map, say): that is not "there is none", the caller may ask again
	if (unanswered) throw new Error('no answer for ' + name);
	return null;
}

// An image of the game as a texture. null: there is none (fetchImageFile), or its sides are not powers of two:
// such an image the engine refuses (R_FindImageFile), and the shader that names it is drawn as one that has an
// error.
async function fetchImage(name, clamp) {
	const f = await fetchImageFile(name);
	if (!f || (f.w & (f.w - 1)) || (f.h & (f.h - 1))) return null;
	const t0 = performance.now(), opts = { clamp, mip: imageHasMips(f.base) };
	let tex;
	if (f.tga) {
		tex = makeTexture(f.w, f.h, f.img.data, opts);
	} else {
		tex = gl.createTexture();
		gl.bindTexture(gl.TEXTURE_2D, tex);
		gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE, f.img);
		finishTexture(f.w, f.h, opts);
	}
	loadTimes.images += performance.now() - t0; loadTimes.imageCount++;
	return { tex, alpha: f.tga && f.img.alpha, w: f.w, h: f.h };
}

