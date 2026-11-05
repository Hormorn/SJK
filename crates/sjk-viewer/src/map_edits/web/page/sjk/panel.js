// ---- the panel: the in-game editor's pages (Selected, Place, Edits), World (what other parts of SJK stand on the
// map) and its foot (scope, answer, Publish)
//
// Every button asks the game for the command the in-game panel's button runs (sjkEdit), so both stay one editor.

const SJK_ROWS = 200, SJK_TILES = 120;
const SJK_TABS = ['selected', 'place', 'edits', 'world'];
let sjkTab = 'selected';

function sjkShowTab(tab) {
	sjkTab = tab;
	for (const b of document.querySelectorAll('#tabs button')) b.classList.toggle('on', b.dataset.tab === tab);
	for (const t of SJK_TABS) $('tab-' + t).hidden = t !== tab;
	if (tab === 'selected') sjkRenderShaders();
	if (tab === 'place') sjkRenderModels();
	sjkRenderRecent();
}

// A text set only when it changed: the panel is drawn again with every state (about once a second), and a text
// written anew drops what the mouse selected in it, so nothing could be selected to copy.
function sjkText(id, text) {
	const el = $(id);
	if (el.textContent !== text) el.textContent = text;
}
// The same for a block of HTML: written only when it changed.
function sjkHtml(el, html) {
	if (el.dataset.html !== html) { el.dataset.html = html; el.innerHTML = html; }
}

const SJK_KINDS = { model: 'Map prop', placed: 'Placed model', surface: 'Surface', face: 'Hidden face (caulk or nodraw)', other: 'Not editable' };
// a sky side the game selected (a face of the sky's brush): covered with a texture, or its sky swapped
const sjkIsSky = sel => !!(sel && sel.kind === 'face' && sel.target && sel.target.sky);
const sjkSkySel = () => sjkIsSky(sjk.state && sjk.state.selection);
const sjkKindName = sel => sjkIsSky(sel) ? 'Sky (a side of the sky\'s brush)' : SJK_KINDS[sel.kind] || sel.kind;
// the sky chosen in the sky list
sjk.chosenSky = '';

// The selection named plainly: what it is, its shader, its model, its edit
function sjkFacts(sel) {
	if (!sel) return '';
	const t = sel.target || {}, rows = [['Kind', sjkKindName(sel)]];
	if (t.placed !== undefined && t.placed !== null) rows.push(['Edit', String(t.placed)]);
	if (t.model) rows.push(['Model', t.model]);
	if (t.classname && sel.kind === 'model') rows.push(['Entity', t.entity + ' (' + t.classname + ')']);
	if (t.shader) rows.push([sel.kind === 'model' || sel.kind === 'placed' ? 'Shader aimed at' : 'Shader', t.shader]);
	if (t.surface !== undefined) rows.push(['Surface', t.surface + (t.surface_kind ? ' (' + t.surface_kind + ')' : '')]);
	if (t.group && t.group.length > 1) rows.push(['Whole model', t.group.length + ' surfaces (drawn fainter)']);
	if (t.brush !== undefined) rows.push(['Brush side', t.brush + ' / ' + t.side]);
	const remap = t.shader && sjk.state.remaps.find(([from]) => from.toLowerCase() === t.shader.toLowerCase());
	if (remap) rows.push([sjkIsSky(sel) ? 'Sky swapped for' : 'Drawn as', remap[1]]);
	if (sel.hit) rows.push(['At', sel.hit.map(v => Math.round(v)).join(' ')]);
	return rows.map(([k, v]) => '<dt>' + k + '</dt><dd>' + escapeHtml(v) + '</dd>').join('');
}

// what changes with every state: the header, the selection, the edits, the foot
function sjkRender() {
	const s = sjk.state;
	sjkText('mapName', s ? s.map + (s.checksum !== null && s.checksum !== undefined ? '' : ' (reading its checksum...)') : '');
	const problem = sjk.error || (s && !s.editor ? 'The map editor is off in the game: mapedit turns it on' : '') ||
		(s && !s.enabled ? 'SJK map edits are off in the game (cg_mapEdits 0): edits are made but not drawn' : '');
	sjkText('problem', problem);
	$('problem').hidden = !problem;
	$('busy').hidden = !sjk.busy;
	if (!s) return;
	// Selected
	const sel = s.selection;
	sjkText('selSummary', sel ? sel.summary : 'Click the map to select a prop, a surface, a placed model or a hidden face');
	sjkText('selKind', sel ? sjkKindName(sel) : '');
	sjkHtml($('selFacts'), sjkFacts(sel));
	const editable = sel && sel.kind !== 'other';
	$('btnHide').disabled = !editable;
	$('btnHide').textContent = sel && sel.kind === 'placed' ? 'Remove' : 'Hide';
	$('btnHideGroup').disabled = !sel || sel.kind !== 'surface';
	$('btnCopy').disabled = !sel || (sel.kind !== 'model' && sel.kind !== 'placed');
	$('btnRemap').disabled = !editable || !sjkRemapTarget();
	$('btnUnremap').disabled = !editable;
	// Place
	const current = s.edits.find(e => e.id === s.current);
	sjkText('currentName', current ? current.label : 'none: place one, or choose one in Edits');
	for (const b of document.querySelectorAll('#moves button')) b.disabled = !current;
	// Edits
	const list = $('editList'), key = JSON.stringify([s.edits.map(e => [e.id, e.label, e.active]), s.current, sjk.chosenEdit]);
	if (list.dataset.key !== key) {
		list.dataset.key = key;
		list.innerHTML = s.edits.length ? s.edits.map(e => '<div class="row' + (e.active ? '' : ' dim') + (e.id === sjk.chosenEdit ? ' sel' : '') + '" data-id="' + e.id + '" title="' +
			escapeHtml(e.describe) + '">' + e.id + ': ' + escapeHtml(e.label) + (e.id === s.current ? ' <b class="gold">moved by Place</b>' : '') + '</div>').join('')
			: '<div class="hint">No edits on this map yet.</div>';
	}
	$('btnDelete').disabled = !s.edits.some(e => e.id === sjk.chosenEdit);
	$('btnUndo').disabled = !s.undo;
	$('btnRedo').disabled = !s.redo;
	// the foot
	for (const b of document.querySelectorAll('#scope button')) b.classList.toggle('on', b.dataset.scope === s.scope);
	sjkText('scopeServer', s.server ? 'This server (' + s.server + ')' : 'This server');
	const answer = sjk.answer || null;
	const text = answer ? answer.text : s.hub ? 'Hub: ' + s.hub : '';
	sjkText('answer', text);
	$('answer').classList.toggle('err', !!(answer ? answer.error : s.hub));
	$('btnPublish').classList.toggle('gold', !!s.unpublished);
	$('btnPublish').disabled = !s.edits.length && !s.unpublished;
	sjkText('pubState', s.unpublished ? 'Changed here, not published' : s.revision ? 'Hub revision ' + s.revision : '');
	sjkRenderThings();
	sjkRenderMulti();
	sjkRenderSky();
}

// ---- the sky: a sky side selected offers the skies of the game data (their scripts have skyParms) to swap it for

const sjkSkyNames = () => new Set(((sjk.lists && sjk.lists.skies) || []).map(k => k.name.toLowerCase()));
function sjkSkyTile(name, picture) {
	return '<div class="tile' + (name === sjk.chosenSky ? ' sel' : '') + '" data-name="' + escapeHtml(name) + '" title="' + escapeHtml(name) + '"><i class="thumb" data-thumb="' +
		escapeHtml(picture || name) + '"></i><span>' + escapeHtml(name.slice(name.lastIndexOf('/') + 1)) + '</span></div>';
}
function sjkRenderSky() {
	const s = sjk.state, sel = s && s.selection, on = sjkSkySel();
	$('skyBox').hidden = !on;
	if (!on) return;
	const own = sel.target.shader, swapped = s.remaps.find(([from]) => from.toLowerCase() === own.toLowerCase());
	const skies = (sjk.lists && sjk.lists.skies) || [], pictures = new Map(skies.map(k => [k.name.toLowerCase(), k.picture]));
	const box = $('skyList');
	if (!sjk.lists) sjkHtml(box, '<div class="hint">Reading the skies...</div>');
	else {
		const words = sjkWords($('skyFind').value), all = skies.filter(k => sjkMatches(k.name, words)), shown = all.slice(0, SJK_TILES);
		sjkHtml(box, shown.map(k => sjkSkyTile(k.name, k.picture)).join('') || '<div class="hint">No sky has that in its name.</div>');
		thumbWatch(box);
		sjkText('skyCount', all.length + ' skies' + (all.length > shown.length ? ', the first ' + shown.length + ' shown' : '') +
			(s.skies && s.skies.length > 1 ? '; this map has ' + s.skies.length : ''));
	}
	// Recent: the skies among the shaders used lately
	const r = sjk.recent, recent = r && r.remember ? r.shaders.filter(n => pictures.has(n.toLowerCase())) : [];
	$('skyRecentBox').hidden = !recent.length;
	sjkHtml($('skyRecent'), recent.map(n => sjkSkyTile(n, pictures.get(n.toLowerCase()))).join(''));
	thumbWatch($('skyRecent'));
	const chosen = sjk.chosenSky && sjk.chosenSky.toLowerCase() !== (swapped ? swapped[1] : own).toLowerCase();
	$('btnSky').disabled = !chosen;
	$('btnSkyAll').disabled = !sjk.chosenSky;
	$('btnSkyBack').disabled = !swapped;
	sjkText('skyHint', (swapped ? 'Drawn as ' + swapped[1] + ' now. ' : '') + 'Draw another shader, below, covers the selected sky side with a texture instead.');
}
function sjkSkyClick(e) {
	const tile = e.target.closest('.tile');
	if (!tile || String(getSelection())) return;
	// a second click on the chosen one swaps, as a shader's tile remaps
	if (sjk.chosenSky === tile.dataset.name && sjkSkySel()) { sjkEdit({ op: 'sky', shader: tile.dataset.name }); return; }
	sjk.chosenSky = tile.dataset.name;
	for (const t of document.querySelectorAll('#skyList .tile, #skyRecent .tile')) t.classList.toggle('sel', t.dataset.name === sjk.chosenSky);
	sjkRender();
}

// ---- World: the boomboxes, mystery boxes and toys on the map

const SJK_THING_NAMES = { boombox: 'your boombox', mystery_box: 'a mystery box', toy: 'a toy' };

function sjkRenderThings() {
	const s = sjk.state, w = s && s.things;
	if (!w) return;
	const chosen = sjk.chosenThing;
	const list = w.list.map(t => '<div class="row' + (chosen && chosen.thing === t.thing && chosen.id === t.id && chosen.label === t.label ? ' sel' : '') +
		'" data-thing="' + t.thing + '" data-id="' + t.id + '" data-label="' + escapeHtml(t.label) + '" title="' + escapeHtml(t.parts.map(p => p.model).join(', ')) + '">' +
		escapeHtml(t.label) + ' <span class="count">' + t.origin.map(v => Math.round(v)).join(' ') + '</span></div>').join('');
	sjkHtml($('thingList'), list || '<div class="hint">Nothing of the kind on this map.</div>');
	const thing = sjkChosenThing();
	$('btnThingRemove').disabled = !thing || (thing.thing === 'boombox' && !thing.mine);
	$('btnThingFly').disabled = !thing;
	const kinds = w.toy_kinds || [], sel = $('toyKind');
	const key = kinds.join('\n');
	if (sel.dataset.key !== key) {
		const was = sel.value;
		sel.dataset.key = key;
		sel.innerHTML = kinds.length ? kinds.map(k => '<option>' + escapeHtml(k) + '</option>').join('') : '<option value="">no kinds from the hub</option>';
		if (kinds.includes(was)) sel.value = was;
	}
	for (const b of document.querySelectorAll('#thingPlace button')) {
		const t = b.dataset.thing;
		b.classList.toggle('on', sjk.placeThing === t);
		b.disabled = t !== 'boombox' && (!w.on_server || (t === 'toy' && !kinds.length));
	}
	sjkText('thingHint', sjk.placeThing ? 'Click the floor where ' + SJK_THING_NAMES[sjk.placeThing] + ' goes (Esc stops).' :
		!w.on_server ? 'Mystery boxes and toys stand on a server\'s map: join one in the game to place them.' :
		'Choose what to place, then click the floor where it goes. Boxes and toys are the hub\'s: it places and removes them but cannot move one (remove it and place it again).');
}

// the thing chosen in the list, as the game has it now
function sjkChosenThing() {
	const c = sjk.chosenThing, w = sjk.state && sjk.state.things;
	return c && w ? w.list.find(t => t.thing === c.thing && t.id === c.id && t.label === c.label) || null : null;
}

function sjkFlyTo(o) {
	const d = camDir();
	cam.fly = true;
	cam.pos = [o[0] - d[0] * 160, o[1] - d[1] * 160, o[2] - d[2] * 160 + 16];
}

// ---- the Recent rows: the shaders and models used lately, kept by the game (api/recent)

function sjkRenderRecent() {
	const r = sjk.recent;
	for (const box of document.querySelectorAll('.rememberRecent')) box.checked = !!(r && r.remember);
	const shaders = r && r.remember ? r.shaders : [], models = r && r.remember ? r.models : [];
	$('shaderRecentBox').hidden = !shaders.length;
	$('modelRecentBox').hidden = !models.length;
	const sb = $('shaderRecent');
	sjkHtml(sb, shaders.map(n => '<div class="tile' + (n === sjk.chosenShader ? ' sel' : '') + '" data-name="' + escapeHtml(n) + '" title="' + escapeHtml(n) + '"><i class="thumb" data-thumb="' +
		escapeHtml(n) + '"></i><span>' + escapeHtml(n.slice(n.lastIndexOf('/') + 1)) + '</span></div>').join(''));
	thumbWatch(sb);
	sjkHtml($('modelRecent'), models.map(m => '<div class="row' + (m === sjk.chosenModel ? ' sel' : '') + '" data-model="' + escapeHtml(m) + '" title="' + escapeHtml(m) + '">' +
		escapeHtml(m.replace(/^models\//, '')) + '</div>').join(''));
}

// the lists from the game: shown on their pages
function sjkRenderLists() {
	sjkRenderShaders();
	sjkRenderModels();
	sjkRenderSky();
}

const sjkWords = text => text.toLowerCase().split(/\s+/).filter(Boolean);
const sjkMatches = (name, words) => words.every(w => name.toLowerCase().includes(w));

function sjkRenderModels() {
	const box = $('modelList');
	if (!sjk.lists) { box.innerHTML = '<div class="hint">Reading the models...</div>'; return; }
	const words = sjkWords($('modelFind').value), all = sjk.lists.models.filter(m => sjkMatches(m, words)), shown = all.slice(0, SJK_ROWS);
	box.innerHTML = shown.map(m => '<div class="row' + (m === sjk.chosenModel ? ' sel' : '') + '" data-model="' + escapeHtml(m) + '" title="' + escapeHtml(m) + '">' +
		escapeHtml(m.replace(/^models\//, '')) + '</div>').join('') + (all.length > shown.length ? '<div class="hint">' + (all.length - shown.length) + ' more: search to narrow</div>' : '') ||
		'<div class="hint">No model has that in its name.</div>';
	$('modelCount').textContent = all.length + ' models';
}

// what a remap draws: the shader chosen in the list, or the one typed
const sjkRemapTarget = () => ($('shaderFind').value.trim() && !sjk.lists?.shaders.includes(sjk.chosenShader) ? $('shaderFind').value.trim() : sjk.chosenShader) || '';

function sjkRenderShaders() {
	const box = $('shaderList');
	if (!sjk.lists) { box.innerHTML = '<div class="hint">Reading the shaders...</div>'; return; }
	const words = sjkWords($('shaderFind').value), all = sjk.lists.shaders.filter(m => sjkMatches(m, words)), shown = all.slice(0, SJK_TILES);
	box.innerHTML = shown.map(n => '<div class="tile' + (n === sjk.chosenShader ? ' sel' : '') + '" data-name="' + escapeHtml(n) + '" title="' + escapeHtml(n) + '"><i class="thumb" data-thumb="' +
		escapeHtml(n) + '"></i><span>' + escapeHtml(n.slice(n.lastIndexOf('/') + 1)) + '</span></div>').join('') ||
		'<div class="hint">No shader has that in its name; Remap tries the name as typed.</div>';
	$('shaderCount').textContent = all.length + ' shaders' + (all.length > shown.length ? ', the first ' + shown.length + ' shown' : '');
	thumbWatch(box);
}

// the arrows of Place: 8 units along the world axis nearest the camera's right and ahead, so steps add up
// straight (as the in-game panel moves); Shift: 1 unit, 1 degree, a tenth of the growth
function sjkNudge(right, away) {
	const y = cam.yaw * Math.PI / 180, r = [Math.sin(y), -Math.cos(y)], f = [Math.cos(y), Math.sin(y)];
	const axis = v => Math.abs(v[0]) >= Math.abs(v[1]) ? [Math.sign(v[0]), 0] : [0, Math.sign(v[1])];
	const a = axis(r), b = axis(f);
	return [a[0] * right + b[0] * away, a[1] * right + b[1] * away];
}
function sjkMove(e, kind) {
	const fine = e.shiftKey, step = fine ? 1 : 8;
	const moves = {
		up: [0, 0, step], down: [0, 0, -step],
		left: [...sjkNudge(-step, 0), 0], right: [...sjkNudge(step, 0), 0],
		away: [...sjkNudge(0, step), 0], closer: [...sjkNudge(0, -step), 0]
	};
	if (kind === 'turnl' || kind === 'turnr') return sjkEdit({ op: 'turn', degrees: (fine ? 1 : 15) * (kind === 'turnl' ? 1 : -1) });
	if (kind === 'bigger' || kind === 'smaller') { const g = fine ? 1.01 : 1.1; return sjkEdit({ op: 'scale', factor: kind === 'bigger' ? g : 1 / g }); }
	if (kind === 'remove') return sjk.state && sjk.state.current ? sjkEdit({ op: 'delete', id: sjk.state.current }) : null;
	return sjkEdit({ op: 'move', offset: moves[kind] });
}

function sjkPublish() {
	const s = sjk.state;
	if (!s) return;
	if (!confirm('Send this map\'s ' + s.edits.length + ' edits to the SJK hub for every SJK player? It replaces the hub\'s list for ' + s.map + '.')) return;
	sjkEdit({ op: 'publish' });
}

// ---- the panel's controls

for (const b of document.querySelectorAll('#tabs button')) b.addEventListener('click', () => sjkShowTab(b.dataset.tab));
$('btnHide').addEventListener('click', () => sjkHideSelection());
$('btnHideGroup').addEventListener('click', () => sjkEdit({ op: 'hide', group: true }));
$('btnCopy').addEventListener('click', () => {
	const sel = sjk.state && sjk.state.selection;
	if (!sel || !sel.target.model) return;
	sjk.chosenModel = sel.target.model;
	sjk.placeThing = null;
	$('modelFind').value = '';
	sjkShowTab('place');
	toast('Click the map where the copy goes');
});
$('btnRemap').addEventListener('click', () => { const to = sjkRemapTarget(); if (to) sjkApplyShader(to); });
$('btnUnremap').addEventListener('click', () => sjkApplyShader('-'));
$('skyList').addEventListener('click', sjkSkyClick);
$('skyRecent').addEventListener('click', sjkSkyClick);
$('skyFind').addEventListener('input', () => { clearTimeout(sjkTyped); sjkTyped = setTimeout(sjkRenderSky, 150); });
$('btnSky').addEventListener('click', () => { if (sjk.chosenSky) sjkEdit({ op: 'sky', shader: sjk.chosenSky }); });
$('btnSkyAll').addEventListener('click', () => { if (sjk.chosenSky) sjkEdit({ op: 'sky', shader: sjk.chosenSky, all: true }); });
$('btnSkyBack').addEventListener('click', () => sjkEdit({ op: 'sky', shader: '-' }));

let sjkTyped = 0;
$('shaderFind').addEventListener('input', () => { clearTimeout(sjkTyped); sjkTyped = setTimeout(() => { sjkRenderShaders(); sjkRender(); }, 150); });
function sjkTileClick(e) {
	const tile = e.target.closest('.tile');
	// a click that ends a selection of text chooses nothing
	if (!tile || String(getSelection())) return;
	// a second click on the chosen one remaps, as in the game
	if (sjk.chosenShader === tile.dataset.name && sjk.state && (sjk.state.selection || sjkMany())) { sjkApplyShader(tile.dataset.name); return; }
	sjk.chosenShader = tile.dataset.name;
	for (const t of document.querySelectorAll('#shaderList .tile, #shaderRecent .tile')) t.classList.toggle('sel', t.dataset.name === sjk.chosenShader);
	sjkRender();
}
$('shaderList').addEventListener('click', sjkTileClick);
$('shaderRecent').addEventListener('click', sjkTileClick);
$('modelFind').addEventListener('input', () => { clearTimeout(sjkTyped); sjkTyped = setTimeout(sjkRenderModels, 150); });
function sjkModelClick(e) {
	const row = e.target.closest('.row');
	if (!row || String(getSelection())) return;
	sjk.chosenModel = row.dataset.model;
	sjk.placeThing = null;
	for (const r of document.querySelectorAll('#modelList .row, #modelRecent .row')) r.classList.toggle('sel', r.dataset.model === sjk.chosenModel);
}
$('modelList').addEventListener('click', sjkModelClick);
$('modelRecent').addEventListener('click', sjkModelClick);
for (const box of document.querySelectorAll('.rememberRecent')) box.addEventListener('change', () => sjkSetRecent({ op: 'remember', on: box.checked }));
// World
$('thingList').addEventListener('click', e => {
	const row = e.target.closest('.row');
	if (!row || String(getSelection())) return;
	sjk.chosenThing = { thing: row.dataset.thing, id: +row.dataset.id, label: row.dataset.label };
	sjkRender();
});
$('thingList').addEventListener('dblclick', e => {
	const row = e.target.closest('.row');
	const thing = row && sjk.state.things.list.find(t => t.thing === row.dataset.thing && t.id === +row.dataset.id);
	if (thing) sjkFlyTo(thing.origin);
});
$('btnThingFly').addEventListener('click', () => { const t = sjkChosenThing(); if (t) sjkFlyTo(t.origin); });
$('btnThingRemove').addEventListener('click', () => {
	const t = sjkChosenThing();
	if (!t) return;
	if (t.thing !== 'boombox' && !confirm('Remove ' + t.label + ' from this map for every SJK player?')) return;
	sjkEdit({ op: 'remove_thing', thing: t.thing, id: t.thing === 'boombox' ? 0 : t.id });
	sjk.chosenThing = null;
});
for (const b of document.querySelectorAll('#thingPlace button')) b.addEventListener('click', () => {
	sjk.placeThing = sjk.placeThing === b.dataset.thing ? null : b.dataset.thing;
	sjk.chosenModel = '';
	sjkRender();
});
for (const b of document.querySelectorAll('#moves button')) b.addEventListener('click', e => sjkMove(e, b.dataset.move));
$('editList').addEventListener('click', e => {
	const row = e.target.closest('.row');
	if (!row || String(getSelection())) return;
	const id = +row.dataset.id, edit = sjk.state.edits.find(x => x.id === id);
	sjk.chosenEdit = id;
	sjkRender();
	// a placed model: the one Place moves
	if (edit && edit.edit.kind === 'place') sjkEdit({ op: 'current', id });
});
$('editList').addEventListener('dblclick', e => {
	const row = e.target.closest('.row');
	const edit = row && sjk.state.edits.find(x => x.id === +row.dataset.id);
	// a double click flies to a placed model
	if (edit && edit.edit.kind === 'place') sjkFlyTo(edit.edit.origin);
});
$('btnDelete').addEventListener('click', () => { if (sjk.chosenEdit !== null) sjkEdit({ op: 'delete', id: sjk.chosenEdit }); });
$('btnUndo').addEventListener('click', () => sjkEdit({ op: 'undo' }));
$('btnRedo').addEventListener('click', () => sjkEdit({ op: 'redo' }));
for (const b of document.querySelectorAll('#scope button')) b.addEventListener('click', () => sjkEdit({ op: 'scope', scope: b.dataset.scope }));
$('btnPublish').addEventListener('click', sjkPublish);
$('btnMyView').addEventListener('click', () => { if (sjk.state && sjk.state.view) sjkGoTo(sjk.state.view); });
