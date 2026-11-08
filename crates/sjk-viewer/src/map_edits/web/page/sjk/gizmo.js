// ---- the gizmo: Blender's handles to move, rotate and scale placed models (and your own boombox)
//
// Sol, 04/11/2026: "add gizmos on the web/ingame to move on an axis the selection or rotate it, scale it, the
// classic stuff like in Blender". The maths are the game's (map_edits/gizmo.rs). Move: an arrow per axis (X red, Y
// green, Z blue: the world's, or the model's own with Local), a square per plane, the centre in the view's plane.
// Rotate: a ring per axis and one round the view. Scale: an arm with a box end per axis of the model, the centre
// for all three. A drag on a handle, or G, R or T and then the mouse (X, Y, Z constrain, twice for the model's own
// axis, Shift with them for the plane without it; a number typed sets the amount; Enter or a click puts it down,
// Esc or a right click takes it back), changes the page's drawing only; its end sends one `set` (mapedit_set:
// every model's exact origin, angles and scale), so a drag is one step to undo and one write of the document.
// Ctrl snaps (the bar's steps: 8 units, 15 degrees, 0.1 of scale), Shift is fine (a tenth).
//
// Several placed models (Shift+click adds them) turn and scale together about their middle. Your boombox (chosen on
// World) is stood again where a drag ends (`set_thing`), on the floor under it: it moves along the floor and turns
// about Z only. Mystery boxes and toys are the hub's, which places and removes them but has no move (a remove and a
// place would make a new box, its opened state and id lost): they get no gizmo.

const GZ_ARM = 0.12;
const GZ_RING = 48;
const GZ_COLORS = [[0.95, 0.27, 0.33], [0.55, 0.82, 0.2], [0.25, 0.55, 0.98]];
const GZ_WORLD = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
const GZ_FINE = 0.1;
const gz = { mode: 'move', local: false, snap: { grid: 8, angle: 15, scale: 0.1 }, hover: null, active: null, preview: null, says: '', mouse: null, buf: null, vao: null, boxes: new Map() };

// ---- vectors and JA's angles

const gzAdd = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const gzSub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const gzMul = (a, s) => [a[0] * s, a[1] * s, a[2] * s];
const gzDot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const gzCross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const gzLen = a => Math.hypot(a[0], a[1], a[2]);
const gzNorm = a => { const l = gzLen(a); return l > 1e-9 ? gzMul(a, 1 / l) : [0, 0, 0]; };
const gzSnapped = (v, step) => step > 0 ? Math.round(v / step) * step : v;

// AnglesToAxis: the model's forward, left and up (pitch, yaw, roll in degrees)
function gzAxesOf(angles) {
	const r = Math.PI / 180, sp = Math.sin(angles[0] * r), cp = Math.cos(angles[0] * r), sy = Math.sin(angles[1] * r), cy = Math.cos(angles[1] * r),
		sr = Math.sin(angles[2] * r), cr = Math.cos(angles[2] * r);
	return [[cp * cy, cp * sy, -sp], [sr * sp * cy - cr * sy, sr * sp * sy + cr * cy, sr * cp], [cr * sp * cy + sr * sy, cr * sp * sy - sr * cy, cr * cp]];
}
// the angles of those axes, each in 0..360 (straight up or down, all yaw)
function gzAnglesOf([f, l, u]) {
	const pitch = Math.asin(Math.max(-1, Math.min(1, -f[2])));
	const [yaw, roll] = Math.cos(pitch) > 1e-4 ? [Math.atan2(f[1], f[0]), Math.atan2(l[2], u[2])] : [Math.atan2(-l[0], l[1]), 0];
	return [pitch, yaw, roll].map(a => { const d = ((a * 180 / Math.PI) % 360 + 360) % 360; return 360 - d < 1e-3 ? 0 : d; });
}
// v turned by deg about the unit axis (right-handed, Rodrigues)
function gzTurn(axis, deg, v) {
	const t = deg * Math.PI / 180, c = Math.cos(t), s = Math.sin(t);
	return gzAdd(gzAdd(gzMul(v, c), gzMul(gzCross(axis, v), s)), gzMul(axis, gzDot(axis, v) * (1 - c)));
}
const gzClampScale = s => Math.max(0.01, Math.min(64, s));
const gzPivot = list => gzMul(list.reduce((sum, p) => gzAdd(sum, p.origin), [0, 0, 0]), 1 / list.length);

// the models after a change about the pivot: { move: v } | { turn: axis, deg } | { scale: factor, along }
function gzApply(start, pivot, ch) {
	return start.map(p => {
		const out = { id: p.id, origin: p.origin.slice(), angles: p.angles.slice(), scale: p.scale.slice() };
		if (ch.move) out.origin = gzAdd(p.origin, ch.move);
		else if (ch.turn) {
			out.origin = gzAdd(pivot, gzTurn(ch.turn, ch.deg, gzSub(p.origin, pivot)));
			out.angles = gzAnglesOf(gzAxesOf(p.angles).map(a => gzTurn(ch.turn, ch.deg, a)));
		} else if (ch.scale !== undefined) {
			const f = Math.max(1e-3, ch.scale), off = gzSub(p.origin, pivot);
			if (!ch.along) {
				out.origin = gzAdd(pivot, gzMul(off, f));
				out.scale = p.scale.map(s => gzClampScale(s * f));
			} else {
				out.origin = gzAdd(p.origin, gzMul(ch.along, gzDot(off, ch.along) * (f - 1)));
				const axes = gzAxesOf(p.angles);
				let k = 0;
				for (let i = 1; i < 3; i++) if (Math.abs(gzDot(axes[i], ch.along)) > Math.abs(gzDot(axes[k], ch.along))) k = i;
				out.scale[k] = gzClampScale(p.scale[k] * f);
			}
		}
		return out;
	});
}

// rays (screenRay's { origin, dir }) against an axis line and a plane
function gzAlongAxis(ray, point, axis) {
	const u = ray.dir, v = axis, w = gzSub(ray.origin, point);
	const a = gzDot(u, u), b = gzDot(u, v), c = gzDot(v, v), d = gzDot(u, w), e = gzDot(v, w), den = a * c - b * b;
	return Math.abs(den) > 1e-6 * a * c ? (a * e - b * d) / den : null;
}
function gzOnPlane(ray, point, normal) {
	const facing = gzDot(ray.dir, normal);
	if (Math.abs(facing) < 1e-6) return null;
	const t = gzDot(gzSub(point, ray.origin), normal) / facing;
	return t > 0 ? gzAdd(ray.origin, gzMul(ray.dir, t)) : null;
}
// degrees from `from` to `to` round `c` on the screen (y down), counter-clockwise positive
function gzScreenTurn(c, from, to) {
	const angle = p => Math.atan2(-(p[1] - c[1]), p[0] - c[0]);
	let d = (angle(to) - angle(from)) * 180 / Math.PI;
	if (d > 180) d -= 360; else if (d < -180) d += 360;
	return d;
}

// ---- what the gizmo is on

// the placed models of the set (the last added first: its axes are the local ones, Blender's active object, as in
// the game), else the selected one, else (on Place) the current one; or your boombox chosen on World
function gzTargets() {
	const s = sjk.state;
	if (!s) return null;
	const ids = sjk.multi.filter(m => m.kind === 'placed').map(m => m.placed).reverse();
	if (!ids.length && s.selection && s.selection.kind === 'placed' && s.selection.target) ids.push(s.selection.target.placed);
	if (!ids.length && sjkTab === 'place' && s.current) ids.push(s.current);
	const list = ids.map(id => s.placed.find(p => p.id === id)).filter(Boolean)
		.map(p => ({ id: p.id, origin: p.origin.slice(), angles: p.angles.slice(), scale: p.scale.slice() }));
	if (list.length) return { kind: 'placed', list };
	const t = sjkTab === 'world' ? sjkChosenThing() : null;
	if (t && t.thing === 'boombox' && t.mine && t.parts && t.parts[0]) {
		return { kind: 'boombox', list: [{ id: 0, origin: t.parts[0].origin.slice(), angles: [0, t.parts[0].angles[1], 0], scale: [1, 1, 1] }], size: t.parts[0].scale };
	}
	return null;
}

// what a boombox takes: along the floor and about Z
const gzBoomboxTakes = h => (h.k === 'axis' && h.i < 2) || (h.k === 'plane' && h.i === 2) || h.k === 'free' || (h.k === 'ring' && h.i === 2);

// ---- the gizmo laid out on the screen (CSS pixels)

const gzArmPixels = () => canvas.clientHeight * GZ_ARM;
const gzReach = () => Math.max(7, canvas.clientHeight / 110);
// an arm in world units at p: the same size on screen whatever the distance
function gzArm(p) {
	const d = Math.max(1, gzLen(gzSub(p, eye))), side = gzNorm(gzCross(camDir(), [0, 0, 1]));
	const step = d * 0.05, a = project(p), b = project(gzAdd(p, gzMul(gzLen(side) ? side : [0, 1, 0], step)));
	if (!a || !b) return null;
	const px = Math.hypot(a[0] - b[0], a[1] - b[1]);
	return px > 1e-3 ? step / px * gzArmPixels() : null;
}
function gzFrame(list, basis) {
	if (!list || !list.length) return null;
	const pivot = gzPivot(list), own = gzAxesOf(list[0].angles), arm = gzArm(pivot), centre = project(pivot);
	if (!arm || !centre) return null;
	return { pivot, own, basis: basis || (gz.local ? own : GZ_WORLD), arm, centre, toEye: gzNorm(gzSub(eye, pivot)) };
}
const gzAxisShown = (f, a) => Math.abs(gzDot(a, f.toEye)) < 0.97;
const gzPlaneShown = (f, n) => Math.abs(gzDot(n, f.toEye)) > 0.2;
function gzPlaneCorners(f, i) {
	const u = f.basis[(i + 1) % 3], v = f.basis[(i + 2) % 3], at = (a, b) => gzAdd(f.pivot, gzMul(gzAdd(gzMul(u, a), gzMul(v, b)), f.arm));
	return [at(0.3, 0.3), at(0.55, 0.3), at(0.55, 0.55), at(0.3, 0.55)];
}
function gzRingPoints(f, i) {
	const u = f.basis[(i + 1) % 3], v = f.basis[(i + 2) % 3], out = [];
	for (let k = 0; k <= GZ_RING; k++) {
		const t = k / GZ_RING * Math.PI * 2;
		out.push(gzAdd(f.pivot, gzMul(gzAdd(gzMul(u, Math.cos(t)), gzMul(v, Math.sin(t))), f.arm)));
	}
	return out;
}
const gzCentreRadius = () => gzArmPixels() * 0.13;
const gzViewRingRadius = () => gzArmPixels() * 1.2;
function gzSegDist(p, a, b) {
	const ab = [b[0] - a[0], b[1] - a[1]], l2 = ab[0] * ab[0] + ab[1] * ab[1];
	const t = l2 > 0 ? Math.max(0, Math.min(1, ((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / l2)) : 0;
	return Math.hypot(p[0] - a[0] - ab[0] * t, p[1] - a[1] - ab[1] * t);
}
function gzInside(p, q) {
	let pos = 0, neg = 0;
	for (let i = 0; i < q.length; i++) {
		const a = q[i], b = q[(i + 1) % q.length], c = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
		if (c >= 0) pos++; if (c <= 0) neg++;
	}
	return pos === q.length || neg === q.length;
}
const gzKey = h => h ? h.k + (h.i === undefined ? '' : h.i) : '';

// the handle under the mouse: the centre first, then the squares, the arms and the rings
function gzPick(f, mode, m, boombox) {
	if (!f) return null;
	const reach = gzReach(), nearCentre = Math.hypot(m[0] - f.centre[0], m[1] - f.centre[1]) <= Math.max(gzCentreRadius(), reach);
	const seg = (a, b) => { const pa = project(a), pb = project(b); return pa && pb ? gzSegDist(m, pa, pb) : Infinity; };
	const takes = h => !boombox || gzBoomboxTakes(h);
	let best = null, bestAt = Infinity;
	const offer = (d, h) => { if (d <= reach && d < bestAt && takes(h)) { best = h; bestAt = d; } };
	if (mode === 'move') {
		if (nearCentre) return { k: 'free' };
		let plane = null;
		for (let i = 0; i < 3; i++) {
			if (!gzPlaneShown(f, f.basis[i]) || !takes({ k: 'plane', i })) continue;
			const q = gzPlaneCorners(f, i).map(project);
			if (q.every(Boolean) && gzInside(m, q) && (plane === null || Math.abs(gzDot(f.basis[i], f.toEye)) > Math.abs(gzDot(f.basis[plane], f.toEye)))) plane = i;
		}
		if (plane !== null) return { k: 'plane', i: plane };
		for (let i = 0; i < 3; i++) if (gzAxisShown(f, f.basis[i])) offer(seg(gzAdd(f.pivot, gzMul(f.basis[i], f.arm * 0.2)), gzAdd(f.pivot, gzMul(f.basis[i], f.arm * 1.05))), { k: 'axis', i });
	} else if (mode === 'scale') {
		if (boombox) return null;
		if (nearCentre) return { k: 'uniform' };
		for (let i = 0; i < 3; i++) if (gzAxisShown(f, f.own[i])) offer(seg(gzAdd(f.pivot, gzMul(f.own[i], f.arm * 0.2)), gzAdd(f.pivot, gzMul(f.own[i], f.arm * 1.05))), { k: 'scale', i });
	} else {
		for (let i = 0; i < 3; i++) {
			const pts = gzRingPoints(f, i).map(project);
			let d = Infinity;
			for (let k = 1; k < pts.length; k++) if (pts[k - 1] && pts[k]) d = Math.min(d, gzSegDist(m, pts[k - 1], pts[k]));
			offer(d, { k: 'ring', i });
		}
		offer(Math.abs(Math.hypot(m[0] - f.centre[0], m[1] - f.centre[1]) - gzViewRingRadius()), { k: 'view' });
	}
	return best;
}

// ---- a transform under way

// X, Y or Z pressed (with Shift, the plane): the world's axis, then the model's own, then none
function gzPressed(con, i, plane) {
	const kind = plane ? 'plane' : 'axis';
	if (con.kind === kind && con.i === i) return con.local ? { kind: 'none' } : { kind, i, local: true };
	return { kind, i, local: false };
}

function gzBegin(how, targets, at) {
	const f = gzFrame(targets.list);
	if (!f) return false;
	gz.active = { how, kind: targets.kind, size: targets.size, start: targets.list, pivot: f.pivot, own: f.own, basis: f.basis,
		begin: at, last: at, turned: 0, typed: '', change: null };
	gz.hover = null;
	gzFollow(at, false, false);
	return true;
}

// what the change is made of
function gzControl(a) {
	const axisOf = (i, local) => local ? a.own[i] : GZ_WORLD[i];
	const boombox = a.kind === 'boombox';
	const h = a.how.handle, con = a.how.keys ? a.how.con : null, mode = a.how.keys;
	if (h) {
		if (h.k === 'axis') return { k: 'moveAxis', axis: a.basis[h.i] };
		if (h.k === 'plane') return { k: 'movePlane', n: a.basis[h.i], u: a.basis[(h.i + 1) % 3], v: a.basis[(h.i + 2) % 3] };
		if (h.k === 'free') return boombox ? { k: 'movePlane', n: [0, 0, 1], u: [1, 0, 0], v: [0, 1, 0] } : { k: 'moveView' };
		if (h.k === 'ring') return { k: 'turn', axis: a.basis[h.i] };
		if (h.k === 'view') return { k: 'turn', axis: null };
		if (h.k === 'scale') return { k: 'scaleAxis', axis: a.own[h.i], dist: false };
		return { k: 'scaleUniform', dist: false };
	}
	if (boombox) {
		// along the floor, about Z
		if (mode === 'rotate') return { k: 'turn', axis: [0, 0, 1] };
		if (con.kind === 'axis' && con.i < 2) return { k: 'moveAxis', axis: GZ_WORLD[con.i] };
		return { k: 'movePlane', n: [0, 0, 1], u: [1, 0, 0], v: [0, 1, 0] };
	}
	if (mode === 'move') {
		if (con.kind === 'axis') return { k: 'moveAxis', axis: axisOf(con.i, con.local) };
		if (con.kind === 'plane') return { k: 'movePlane', n: axisOf(con.i, con.local), u: axisOf((con.i + 1) % 3, con.local), v: axisOf((con.i + 2) % 3, con.local) };
		return { k: 'moveView' };
	}
	if (mode === 'rotate') return { k: 'turn', axis: con.kind === 'none' ? null : axisOf(con.i, con.local) };
	return con.kind === 'none' ? { k: 'scaleUniform', dist: true } : { k: 'scaleAxis', axis: axisOf(con.i, con.local), dist: true };
}

function gzTyped(a) {
	const t = a.typed.trim(), neg = t.startsWith('-'), digits = t.replace(/^-/, '');
	if (!digits) return null;
	const v = parseFloat(digits);
	return isFinite(v) ? (neg ? -v : v) : null;
}

function gzSnapFactor(f, snap, typed) {
	if (!snap || typed) return f;
	const s = 1 + gzSnapped(f - 1, gz.snap.scale);
	return Math.abs(s) < 1e-3 ? gz.snap.scale : s;
}

// the change for the mouse at m, Ctrl (snap) and Shift (fine) held
function gzChange(a, m, snap, fine) {
	const centre = project(a.pivot) || a.begin, ray = p => screenRay(p[0], p[1]), typed = gzTyped(a), c = gzControl(a), by = fine ? GZ_FINE : 1;
	if (c.k === 'turn') { a.turned += gzScreenTurn(centre, a.last, m); a.last = m; }
	if (c.k === 'moveAxis') {
		let t = typed;
		if (t === null) {
			const p = q => gzAlongAxis(ray(q), a.pivot, c.axis) || 0;
			t = (p(m) - p(a.begin)) * by;
			if (snap) t = gzSnapped(t, gz.snap.grid);
		}
		return { move: gzMul(c.axis, t) };
	}
	if (c.k === 'movePlane' || c.k === 'moveView') {
		if (typed !== null) return { move: gzMul(c.k === 'movePlane' ? c.u : GZ_WORLD[0], typed) };
		const n = c.k === 'movePlane' ? c.n : camDir(), hit = q => gzOnPlane(ray(q), a.pivot, n);
		const now = hit(m), then = hit(a.begin), off = now && then ? gzMul(gzSub(now, then), by) : [0, 0, 0];
		const step = snap ? gz.snap.grid : 0, axes = c.k === 'movePlane' ? [c.u, c.v] : GZ_WORLD;
		return { move: axes.reduce((sum, ax) => gzAdd(sum, gzMul(ax, gzSnapped(gzDot(off, ax), step))), [0, 0, 0]) };
	}
	if (c.k === 'turn') {
		const towards = gzSub(eye, a.pivot), axis = c.axis || gzNorm(towards), sign = c.axis ? (gzDot(c.axis, towards) >= 0 ? 1 : -1) : 1;
		let deg = typed !== null ? typed : a.turned * sign * by;
		if (snap && typed === null) deg = gzSnapped(deg, gz.snap.angle);
		return { turn: axis, deg };
	}
	let f = typed;
	if (f === null) {
		if (c.dist) {
			const from = Math.max(16, Math.hypot(a.begin[0] - centre[0], a.begin[1] - centre[1]));
			f = 1 + (Math.hypot(m[0] - centre[0], m[1] - centre[1]) / from - 1) * by;
		} else if (c.k === 'scaleAxis') {
			const p = q => gzAlongAxis(ray(q), a.pivot, c.axis) || 0, from = p(a.begin);
			f = Math.abs(from) < 1e-3 ? 1 : 1 + (p(m) / from - 1) * by;
		} else {
			f = 1 + ((m[0] - a.begin[0]) - (m[1] - a.begin[1])) / gzArmPixels() * by;
		}
	}
	return { scale: gzSnapFactor(f, snap, typed !== null), along: c.k === 'scaleAxis' ? c.axis : null };
}

function gzDescribe(a, ch) {
	const typed = a.typed ? ' (typed ' + a.typed + ')' : '', r = v => v.toFixed(1);
	const name = axis => {
		let best = 'X', at = -1;
		for (let i = 0; i < 3; i++) for (const local of [false, true]) {
			const d = Math.abs(gzDot(local ? a.own[i] : GZ_WORLD[i], axis)) + (local ? 0 : 1e-4);
			if (d > at) { at = d; best = 'XYZ'[i] + (local ? ' local' : ''); }
		}
		return best;
	};
	if (ch.move) return 'Move ' + ch.move.map(r).join(' ') + typed;
	if (ch.turn) return 'Rotate ' + r(ch.deg) + ' degrees about ' + name(ch.turn) + typed;
	return 'Scale ' + ch.scale.toFixed(2) + (ch.along ? ' along ' + name(ch.along) : '') + typed;
}

// the mouse at m: the change previewed on the page's drawing
function gzFollow(m, snap, fine) {
	const a = gz.active;
	if (!a) return;
	a.change = gzChange(a, m, snap, fine);
	gz.preview = gzApply(a.start, a.pivot, a.change);
	gz.says = gzDescribe(a, a.change);
	gzRenderBar();
}

const gzRound = (v, step) => Math.round(v / step) * step;
// the end of a transform: one `set` (or `set_thing`) when it changed something, or back as it was
async function gzEnd(keep) {
	const a = gz.active;
	if (!a) return;
	gz.active = null;
	const ch = a.change, moved = ch && (ch.move ? gzLen(ch.move) > 1e-4 : ch.turn ? Math.abs(ch.deg) > 1e-4 : Math.abs(ch.scale - 1) > 1e-5);
	if (!keep || !moved) { gz.preview = null; sjkSel.key = ''; gz.says = keep ? '' : 'Transform cancelled'; gzRenderBar(); return; }
	const list = gz.preview.map(p => ({ id: p.id, origin: p.origin.map(v => gzRound(v, 0.01)), angles: p.angles.map(v => gzRound(v, 0.01) % 360), scale: p.scale.map(v => gzRound(v, 0.0001)) }));
	gz.preview = list;
	gz.says = '';
	gzRenderBar();
	try {
		if (a.kind === 'boombox') await sjkEdit({ op: 'set_thing', thing: 'boombox', origin: list[0].origin, yaw: list[0].angles[1] });
		else await sjkEdit({ op: 'set', edits: list.map(p => ({ id: p.id, origin: p.origin, angles: p.angles, scale: p.scale })) });
	} finally {
		// the state read after the answer draws them where the game put them
		if (!gz.active) { gz.preview = null; sjkSel.key = ''; }
	}
}

// ---- the preview on the page's drawing, and the gizmo over it all

// Before the edits are drawn: the models of a transform under way (or waiting for the game's answer) where it puts
// them, and the selection's tint with them.
function sjkGizmoPreview() {
	if (!gz.preview) return;
	for (const p of gz.preview) {
		if (p.id === 0 && (gz.active ? gz.active.kind : gzLastKind) === 'boombox') {
			const t = sjk.things.find(x => x.thing === 'boombox' && x.mine);
			const s = t && t.parts[0] ? t.parts[0].scale : 1;
			if (t && t.drawn[0]) t.drawn[0].place = sjkPlace(p.origin, p.angles, [s, s, s]);
		} else {
			const q = sjk.placed.find(x => x.id === p.id);
			if (q) q.place = sjkPlace(p.origin, p.angles, p.scale);
		}
	}
	// the selection's tint is made again from these places (a set's placed models are outlined by the gizmo)
	if (!sjkMany()) sjkSel.key = '';
}
let gzLastKind = 'placed';

function gzBand(out, a, b, w) {
	const s = gzMul(gzNorm(gzCross(gzSub(b, a), gzSub(eye, a))), w);
	const a0 = gzSub(a, s), a1 = gzAdd(a, s), b0 = gzSub(b, s), b1 = gzAdd(b, s);
	out.push(...a0, ...a1, ...b1, ...a0, ...b1, ...b0);
}
function gzCircle(out, centre, r, w) {
	const right = gzNorm(gzCross(camDir(), [0, 0, 1])), up = gzCross(right, camDir());
	let prev = null;
	for (let k = 0; k <= GZ_RING; k++) {
		const t = k / GZ_RING * Math.PI * 2, p = gzAdd(centre, gzAdd(gzMul(right, Math.cos(t) * r), gzMul(up, Math.sin(t) * r)));
		if (prev) gzBand(out, prev, p, w);
		prev = p;
	}
}

function gzParts(f, mode, lit, boombox) {
	const parts = [], upp = f.arm / gzArmPixels(), w = upp * 2, litKey = gzKey(lit);
	const colour = (i, h, alpha = 0.95) => litKey === gzKey(h) ? [1, 0.82, 0.3, 1] : i === null ? [0.92, 0.92, 0.92, 0.85] : [...GZ_COLORS[i], alpha];
	const takes = h => !boombox || gzBoomboxTakes(h);
	const right = gzNorm(gzCross(camDir(), [0, 0, 1])), up = gzCross(right, camDir());
	if (mode === 'move' || mode === 'scale') {
		const axes = mode === 'move' ? f.basis : f.own;
		for (let i = 0; i < 3; i++) {
			const a = axes[i], h = mode === 'move' ? { k: 'axis', i } : { k: 'scale', i };
			if (!gzAxisShown(f, a) || !takes(h)) continue;
			const tris = [], base = gzAdd(f.pivot, gzMul(a, f.arm * 0.82)), tip = gzAdd(f.pivot, gzMul(a, f.arm));
			gzBand(tris, gzAdd(f.pivot, gzMul(a, f.arm * 0.2)), mode === 'move' ? base : tip, w);
			if (mode === 'move') {
				const side = gzMul(gzNorm(gzCross(a, gzSub(eye, base))), upp * 6);
				tris.push(...tip, ...gzAdd(base, side), ...gzSub(base, side));
			} else {
				const h5 = upp * 5, c = (x, y) => gzAdd(tip, gzAdd(gzMul(right, x * h5), gzMul(up, y * h5)));
				tris.push(...c(-1, -1), ...c(1, -1), ...c(1, 1), ...c(-1, -1), ...c(1, 1), ...c(-1, 1));
			}
			parts.push({ tris, color: colour(i, h) });
		}
		if (mode === 'move') for (let i = 0; i < 3; i++) {
			const h = { k: 'plane', i };
			if (!gzPlaneShown(f, f.basis[i]) || !takes(h)) continue;
			const q = gzPlaneCorners(f, i);
			parts.push({ tris: [...q[0], ...q[1], ...q[2], ...q[0], ...q[2], ...q[3]], color: colour(i, h, 0.45) });
		}
		const centre = [];
		gzCircle(centre, f.pivot, gzCentreRadius() * upp, w);
		parts.push({ tris: centre, color: colour(null, mode === 'move' ? { k: 'free' } : { k: 'uniform' }) });
	} else {
		for (let i = 0; i < 3; i++) {
			const h = { k: 'ring', i };
			if (!takes(h)) continue;
			const tris = [], pts = gzRingPoints(f, i);
			for (let k = 1; k < pts.length; k++) gzBand(tris, pts[k - 1], pts[k], w);
			parts.push({ tris, color: colour(i, h) });
		}
		if (takes({ k: 'view' })) {
			const tris = [];
			gzCircle(tris, f.pivot, gzViewRingRadius() * upp, w);
			parts.push({ tris, color: colour(null, { k: 'view' }) });
		}
	}
	const dot = [];
	gzCircle(dot, f.pivot, upp * 2, upp * 2);
	parts.push({ tris: dot, color: [0.92, 0.92, 0.92, 0.9] });
	return parts;
}

// a G, R or T transform: the line its constraint follows, and one from the pivot to the mouse
function gzKeyParts(a, f) {
	const parts = [], upp = f.arm / gzArmPixels(), w = upp * 1.2, c = gzControl(a);
	const axis = c.k === 'moveAxis' || c.k === 'scaleAxis' ? c.axis : c.k === 'turn' ? c.axis : c.k === 'movePlane' ? c.n : null;
	if (axis) {
		const tris = [], far = f.arm * 40;
		gzBand(tris, gzSub(f.pivot, gzMul(axis, far)), gzAdd(f.pivot, gzMul(axis, far)), w);
		// in the colour of the world's axis it is nearest
		let i = 0;
		for (let k = 1; k < 3; k++) if (Math.abs(axis[k]) > Math.abs(axis[i])) i = k;
		parts.push({ tris, color: [...GZ_COLORS[i], 0.7] });
	}
	if (gz.mouse) {
		const r = screenRay(gz.mouse[0], gz.mouse[1]), p = gzOnPlane(r, f.pivot, camDir());
		if (p) { const tris = []; gzBand(tris, f.pivot, p, w * 0.6); parts.push({ tris, color: [1, 1, 1, 0.55] }); }
	}
	const dot = [];
	gzCircle(dot, f.pivot, upp * 3, upp * 3);
	parts.push({ tris: dot, color: [1, 0.82, 0.3, 1] });
	return parts;
}

// After the selection: the gizmo, on top of everything (no depth test), the same size whatever the distance; and
// the boxes of a set's placed models.
function sjkDrawGizmo(now) {
	const a = gz.active, targets = a ? null : gzTargets();
	const list = a ? gz.preview || a.start : targets && targets.list;
	if (a) gzLastKind = a.kind; else if (targets) gzLastKind = targets.kind;
	gl.useProgram(sjkOverlay.p);
	gl.uniformMatrix4fv(sjkOverlay.u.uMvp, false, viewProj);
	gl.enable(gl.BLEND);
	gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
	gl.disable(gl.CULL_FACE);
	gl.depthMask(false);
	// a set's placed models outlined, as the selection is
	if (sjkMany()) {
		const glow = 0.5 + 0.5 * Math.sin(now / 240);
		for (const m of sjk.multi) {
			if (m.kind !== 'placed') continue;
			const p = sjk.placed.find(x => x.id === m.placed);
			if (!p) continue;
			let g = gz.boxes.get(p.model);
			if (!g) { const b = sjkModelBounds(p.model); if (b) { g = sjkBoxLines(b); gz.boxes.set(p.model, g); } }
			if (g) sjkOutline(g, p.place, glow);
		}
	}
	const f = list && gzFrame(list, a ? a.basis : null);
	if (f) {
		const parts = a && a.how.keys ? gzKeyParts(a, f) : gzParts(f, gz.mode, a ? a.how.handle : gz.hover, (a ? a.kind : targets.kind) === 'boombox');
		const data = new Float32Array(parts.reduce((n, p) => n + p.tris.length, 0));
		let at = 0;
		for (const p of parts) { data.set(p.tris, at); p.first = at / 3; p.count = p.tris.length / 3; at += p.tris.length; }
		if (!gz.vao) {
			gz.vao = gl.createVertexArray();
			gl.bindVertexArray(gz.vao);
			gz.buf = gl.createBuffer();
			gl.bindBuffer(gl.ARRAY_BUFFER, gz.buf);
			gl.enableVertexAttribArray(0);
			gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 0, 0);
		}
		gl.bindVertexArray(gz.vao);
		gl.bindBuffer(gl.ARRAY_BUFFER, gz.buf);
		gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
		gl.disable(gl.DEPTH_TEST);
		gl.uniformMatrix4fv(sjkOverlay.u.uModel, false, SJK_IDENTITY);
		gl.uniform1f(sjkOverlay.u.uBias, 0);
		for (const p of parts) {
			gl.uniform4fv(sjkOverlay.u.uColor, p.color);
			gl.drawArrays(gl.TRIANGLES, p.first, p.count);
		}
		gl.enable(gl.DEPTH_TEST);
		gz.drawn = parts.length;
	} else gz.drawn = 0;
	gl.depthMask(true);
	gl.depthFunc(gl.LESS);
	gl.disable(gl.BLEND);
	gl.bindVertexArray(null);
}

// ---- mouse and keys: before the view's own (capture on the window), only what the gizmo takes

const gzAt = e => { const r = canvas.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
function gzStop(e) { e.stopImmediatePropagation(); e.preventDefault(); }

window.addEventListener('pointerdown', e => {
	if (e.target !== canvas || !world || !viewProj) return;
	const m = gzAt(e);
	gz.mouse = m;
	if (gz.active) {
		// G, R or T: a click puts it down, a right click takes it back; a handle's drag: a right click takes it back
		if (gz.active.how.keys && e.button === 0) gzEnd(true);
		else if (e.button === 2) gzEnd(false);
		canvas.focus({ preventScroll: true });
		gzStop(e);
		return;
	}
	if (e.button !== 0 || e.shiftKey || e.ctrlKey || e.metaKey) return;
	const t = gzTargets();
	const h = t && gzPick(gzFrame(t.list), gz.mode, m, t.kind === 'boombox');
	if (!h) return;
	canvas.focus({ preventScroll: true });
	canvas.setPointerCapture(e.pointerId);
	gzBegin({ handle: h }, t, m);
	gzStop(e);
}, true);
window.addEventListener('pointermove', e => {
	if (!world || !viewProj) return;
	if (e.target !== canvas && !gz.active) return;
	const m = gzAt(e);
	gz.mouse = m;
	if (gz.active) { gzFollow(m, e.ctrlKey || e.metaKey, e.shiftKey); gzStop(e); return; }
	const t = gzTargets();
	const h = t && gzPick(gzFrame(t.list), gz.mode, m, t.kind === 'boombox');
	if (gzKey(h) !== gzKey(gz.hover)) { gz.hover = h; canvas.style.cursor = h ? 'pointer' : ''; }
}, true);
window.addEventListener('pointerup', e => {
	if (!gz.active || !gz.active.how.handle) return;
	if (e.button === 0) gzEnd(true);
	gzStop(e);
}, true);

window.addEventListener('keydown', e => {
	if (document.activeElement !== canvas) return;
	const a = gz.active, k = e.key.toLowerCase();
	if (a) {
		gzStop(e);
		if (k === 'escape') { gzEnd(false); return; }
		if (!a.how.keys) return;
		if (k === 'enter' || k === ' ') { gzEnd(true); return; }
		if (e.repeat && k !== 'backspace') return;
		if (k === 'x' || k === 'y' || k === 'z') a.how.con = gzPressed(a.how.con, 'xyz'.indexOf(k), e.shiftKey);
		else if (k === 'backspace') a.typed = a.typed.slice(0, -1);
		else if (k === '-') a.typed = a.typed.startsWith('-') ? a.typed.slice(1) : '-' + a.typed;
		else if (/^[0-9]$/.test(k) || k === '.' || k === ',') { if (a.typed.length < 16) a.typed += k === ',' ? '.' : k; }
		else return;
		gzFollow(gz.mouse || a.begin, e.ctrlKey || e.metaKey, false);
		return;
	}
	if (e.ctrlKey || e.metaKey || e.altKey) return;
	if (k === '1' || k === '2' || k === '3') { gzSetMode(['move', 'rotate', 'scale'][+k - 1]); gzStop(e); return; }
	if (k === 'l') { gzSetLocal(!gz.local); gzStop(e); return; }
	if (k === 'g' || k === 'r' || k === 't') {
		gzStop(e);
		const t = gzTargets(), mode = { g: 'move', r: 'rotate', t: 'scale' }[k];
		if (!t) { toast('Select a placed model (or choose your boombox on World) first', false); return; }
		if (t.kind === 'boombox' && mode === 'scale') { toast('The boombox keeps its size: it moves and turns', false); return; }
		const m = gz.mouse || [canvas.clientWidth / 2, canvas.clientHeight / 2];
		if (!gzBegin({ keys: mode, con: { kind: 'none' } }, t, m)) toast('The gizmo is out of view', false);
	}
}, true);
// a held key the view lost (focus elsewhere) stops nothing; leaving the page takes a transform back
window.addEventListener('blur', () => { if (gz.active) gzEnd(false); });

// ---- the bar: the mode, world or local axes, the snapping steps; kept by this browser

const gzBar = document.createElement('div');
gzBar.id = 'gizmoBar';
gzBar.innerHTML = '<span class="gzHead">Gizmo</span>' +
	'<button data-gz="move" title="Move: arrows, squares, the centre (1; G moves with the mouse)">Move <kbd>1</kbd></button>' +
	'<button data-gz="rotate" title="Rotate: rings about X, Y, Z and the view (2; R rotates with the mouse)">Rotate <kbd>2</kbd></button>' +
	'<button data-gz="scale" title="Scale: an arm per axis of the model, the centre for all (3; T scales with the mouse)">Scale <kbd>3</kbd></button>' +
	'<button id="gzLocal" title="The model\'s own axes instead of the world\'s (L)">Local <kbd>L</kbd></button>' +
	'<label title="Ctrl held snaps moves to this many units">Snap <input id="gzGrid" type="number" min="0.01" step="1"></label>' +
	'<label title="Ctrl held snaps turns to this many degrees"><input id="gzAngle" type="number" min="0.01" step="1">&deg;</label>' +
	'<label title="Ctrl held snaps scale to this step">&times;<input id="gzScale" type="number" min="0.01" step="0.05"></label>' +
	'<span id="gzSays"></span>';
$('view').appendChild(gzBar);
const gzStyle = document.createElement('style');
gzStyle.textContent = '#gizmoBar { position: absolute; left: 12px; top: 12px; display: flex; flex-wrap: wrap; gap: 6px; align-items: center; max-width: calc(100% - 200px); }' +
	'#gizmoBar button, #gizmoBar label, #gizmoBar .gzHead, #gzSays { background: rgba(10, 12, 18, .86); border: 1px solid var(--line); border-radius: 4px; padding: 3px 7px; font-size: 12px; color: var(--muted); }' +
	'#gizmoBar button.on { color: var(--text); border-color: var(--gold, #c9a14a); }' +
	'#gizmoBar label { display: flex; align-items: center; gap: 4px; padding: 1px 6px; }' +
	'#gizmoBar input { width: 48px; margin: 0; padding: 2px 4px; font-size: 12px; height: auto; min-height: 0; line-height: 1.2; }' +
	'#gzSays:empty { display: none; } #gzSays { color: var(--text); }';
document.head.appendChild(gzStyle);

function gzSave() {
	try { localStorage.setItem('sjkMapEditGizmo', JSON.stringify({ mode: gz.mode, local: gz.local, snap: gz.snap })); } catch (e) { /* not kept */ }
}
function gzSetMode(mode) { gz.mode = mode; gz.hover = null; gzSave(); gzRenderBar(); }
function gzSetLocal(on) { gz.local = on; gz.hover = null; gzSave(); gzRenderBar(); toast(on ? 'Gizmo: the model\'s own axes' : 'Gizmo: the world\'s axes'); }
function gzRenderBar() {
	for (const b of gzBar.querySelectorAll('button[data-gz]')) b.classList.toggle('on', b.dataset.gz === gz.mode);
	$('gzLocal').classList.toggle('on', gz.local);
	const says = $('gzSays');
	if (says.textContent !== gz.says) says.textContent = gz.says;
}
for (const b of gzBar.querySelectorAll('button[data-gz]')) b.addEventListener('click', () => gzSetMode(b.dataset.gz));
$('gzLocal').addEventListener('click', () => gzSetLocal(!gz.local));
for (const [id, key] of [['gzGrid', 'grid'], ['gzAngle', 'angle'], ['gzScale', 'scale']]) {
	$(id).addEventListener('change', () => {
		const v = parseFloat($(id).value);
		if (isFinite(v) && v > 0) { gz.snap[key] = v; gzSave(); } else $(id).value = gz.snap[key];
	});
}
try {
	const kept = JSON.parse(localStorage.getItem('sjkMapEditGizmo') || 'null');
	if (kept) {
		if (['move', 'rotate', 'scale'].includes(kept.mode)) gz.mode = kept.mode;
		gz.local = !!kept.local;
		for (const key of ['grid', 'angle', 'scale']) if (kept.snap && kept.snap[key] > 0) gz.snap[key] = kept.snap[key];
	}
} catch (e) { /* the defaults */ }
$('gzGrid').value = gz.snap.grid; $('gzAngle').value = gz.snap.angle; $('gzScale').value = gz.snap.scale;
gzRenderBar();

// the controls card names the gizmo's keys
{
	const dl = document.querySelector('#hudBody dl');
	for (const [keys, does] of [['G / R / T', 'move, rotate, scale with the mouse (placed model, your boombox)'],
		['X Y Z', 'then: along an axis, twice its own; Shift: the plane'], ['0-9 Enter', 'then: type the amount, put it down (Esc takes it back)'],
		['1 2 3 / L', 'gizmo: move, rotate, scale / own axes'], ['Ctrl / Shift', 'while dragging: snap / fine']]) {
		const dt = document.createElement('dt'), dd = document.createElement('dd');
		dt.textContent = keys; dd.textContent = does;
		dl.append(dt, dd);
	}
}

// The page's names stay inside its function; the headless check (`&test` in the address) reaches them here.
if (new URLSearchParams(location.search).has('test')) {
	window.sjkTest = { gz, sjk, gzTargets, gzFrame, gzPick, gzKey, gzRingPoints, gzAdd, gzMul, project, sjkShowTab, sjkEdit, canvas, cam, camDir, gzPivot };
}
