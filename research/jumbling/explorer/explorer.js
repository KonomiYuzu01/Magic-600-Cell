// Viewer for research/jumbling/explorer: grip orbits on S^3 by stereographic projection.
// Data: explorer-results.json (numbers) and points/<menu>__<closure>.json (samples), both
// written by explore.py --preset.
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

const $ = (id) => document.getElementById(id);
const THRESHOLD_DEG = 46.8;
const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
const KIND_TEXT = { lattice: '600 lattice grips twist', all: 'every grip twists' };
const FAMILY_TEXT = { control: 'Control', group: 'Groups', realignment: 'Realignment classes', plane: 'Plane rotations' };

const state = {
  doc: null, menus: new Map(), runs: new Map(), kind: 'lattice', menu: null,
  depth: 1, points: new Map(), groups: [], lattice: null,
};

// ------------------------------------------------------------------ formatting

const fmtInt = new Intl.NumberFormat('en-US');
const SUP = { '-': '⁻', 0: '⁰', 1: '¹', 2: '²', 3: '³', 4: '⁴', 5: '⁵', 6: '⁶', 7: '⁷', 8: '⁸', 9: '⁹' };
const sup = (n) => String(n).split('').map((c) => SUP[c]).join('');
function big(x) {
  if (x < 1e6) return fmtInt.format(x);
  const e = Math.floor(Math.log10(x));
  return `${(x / 10 ** e).toFixed(2)} × 10${sup(e)}`;
}
function angle(deg) {
  if (deg === null || deg === undefined) return '–';
  if (deg >= 0.01) return `${deg.toFixed(3)}°`;
  const [m, e] = deg.toExponential(2).split('e');
  return `${m} × 10${sup(Number(e))}°`;
}
function token(name) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}
function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === 'class') node.className = v;
    else node.setAttribute(k, v);
  }
  for (const c of children) node.append(c);
  return node;
}
function svg(tag, attrs = {}) {
  const node = document.createElementNS('http://www.w3.org/2000/svg', tag);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  return node;
}

function decode(b64, scale) {
  const bin = atob(b64);
  const view = new DataView(new ArrayBuffer(bin.length));
  for (let i = 0; i < bin.length; i++) view.setUint8(i, bin.charCodeAt(i));
  const out = new Float32Array(bin.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = view.getInt16(2 * i, true) / scale;
  return out;
}

async function loadJSON(path) {
  const r = await fetch(path);
  if (!r.ok) throw new Error(`could not load ${path} (HTTP ${r.status})`);
  return r.json();
}

// ------------------------------------------------------------------ 3D scene

const stage = $('stage');
const canvas = $('scene');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(36, 4 / 3, 0.01, 60);
camera.position.set(0.95, 0.62, 1.12);
const controls = new OrbitControls(camera, canvas);
controls.enableDamping = !reduceMotion;
controls.minDistance = 0.12;
controls.maxDistance = 9;
controls.autoRotateSpeed = 0.6;

function sprite(kind) {
  const c = document.createElement('canvas');
  c.width = c.height = 64;
  const g = c.getContext('2d');
  g.fillStyle = '#ffffff';
  g.strokeStyle = '#ffffff';
  g.beginPath();
  if (kind === 'disc') {
    g.arc(32, 32, 27, 0, Math.PI * 2);
    g.fill();
  } else {
    g.lineWidth = 9;
    g.arc(32, 32, 24, 0, Math.PI * 2);
    g.stroke();
  }
  return new THREE.CanvasTexture(c);
}
const DISC = sprite('disc');
const RING = sprite('ring');

const ballRadius = Math.tan((THRESHOLD_DEG * Math.PI) / 360);
const ball = new THREE.Mesh(
  new THREE.SphereGeometry(ballRadius, 48, 32),
  new THREE.MeshBasicMaterial({ transparent: true, opacity: 0.07, depthWrite: false }),
);
const ballRim = new THREE.LineLoop(
  new THREE.BufferGeometry().setFromPoints(
    Array.from({ length: 128 }, (_, i) => {
      const t = (i / 128) * Math.PI * 2;
      return new THREE.Vector3(Math.cos(t) * ballRadius, 0, Math.sin(t) * ballRadius);
    }),
  ),
  new THREE.LineBasicMaterial({ transparent: true, opacity: 0.45 }),
);
scene.add(ball, ballRim);

const baseGeo = new THREE.BufferGeometry();
baseGeo.setAttribute('position', new THREE.Float32BufferAttribute([0, 0, 0], 3));
const basePole = new THREE.Points(baseGeo, new THREE.PointsMaterial({
  size: 0.1, map: RING, transparent: true, alphaTest: 0.2, depthWrite: false, depthTest: false,
}));
basePole.renderOrder = 2;
scene.add(basePole);

function resize() {
  const w = stage.clientWidth;
  const h = stage.clientHeight;
  if (!w || !h) return;
  renderer.setSize(w, h, false);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}
new ResizeObserver(resize).observe(stage);

function frame() {
  controls.autoRotate = $('spin').checked && !reduceMotion;
  controls.update();
  renderer.render(scene, camera);
  requestAnimationFrame(frame);
}

function depthColor(d) {
  return token(`--d${Math.min(Math.max(d, 1), 8)}`);
}

function applyTheme() {
  renderer.setClearColor(new THREE.Color(token('--viz-bg')), 1);
  ball.material.color.set(token('--accent'));
  ballRim.material.color.set(token('--accent'));
  basePole.material.color.set(token('--base'));
  if (state.lattice) state.lattice.material.color.set(token('--lattice'));
  for (const g of state.groups) g.points.material.color.set(depthColor(g.depth));
  renderLegend();
}
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', applyTheme);
new MutationObserver(applyTheme).observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });

function buildLattice(data) {
  const pos = decode(data.b64, data.scale);
  const n = pos.length / 3;
  const col = new Float32Array(n * 4);
  for (let i = 0; i < n; i++) {
    const a = data.angle_deg[i];
    const alpha = a <= 60 ? 1 : Math.max(0.22, 1 - (a - 60) / 75);
    col.set([1, 1, 1, alpha], i * 4);
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.BufferAttribute(pos, 3));
  geo.setAttribute('color', new THREE.BufferAttribute(col, 4));
  const pts = new THREE.Points(geo, new THREE.PointsMaterial({
    size: 0.04, map: RING, vertexColors: true, transparent: true, alphaTest: 0.05, depthWrite: false,
  }));
  pts.renderOrder = 1;
  scene.add(pts);
  state.lattice = pts;
}

function clearGroups() {
  for (const g of state.groups) {
    scene.remove(g.points);
    g.points.geometry.dispose();
    g.points.material.dispose();
  }
  state.groups = [];
}

function buildGroups(data) {
  clearGroups();
  for (const d of data.depths) {
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(decode(d.b64, data.scale), 3));
    const pts = new THREE.Points(geo, new THREE.PointsMaterial({
      size: d.depth <= 1 ? 0.028 : 0.017, map: DISC, transparent: true, alphaTest: 0.35,
    }));
    pts.material.color.set(depthColor(d.depth));
    scene.add(pts);
    state.groups.push({ depth: d.depth, count: d.count, points: pts });
  }
}

function applyDepth() {
  const only = $('only-depth').checked;
  const show = (g) => (only ? g.depth === state.depth : g.depth <= state.depth);
  for (const g of state.groups) g.points.visible = show(g);
  const shown = state.groups.filter(show).reduce((s, g) => s + g.count, 0);
  const op = only ? '=' : '≤';
  $('depth-op').textContent = op;
  $('stage-note').textContent = `${state.menu} · ${state.kind === 'all' ? 'every grip' : 'lattice grips'}`
    + ` · depth ${op} ${state.depth} · ${fmtInt.format(shown)} points`;
}

function renderLegend() {
  const box = $('legend');
  box.replaceChildren();
  for (const g of state.groups) {
    const dot = el('i', { class: 'dot' });
    dot.style.background = depthColor(g.depth);
    box.append(el('span', {}, dot, `depth ${g.depth}`));
  }
  box.append(el('span', {}, el('i', { class: 'ring' }), 'lattice pole'));
  box.append(el('span', {}, el('i', { class: 'ring base' }), 'base pole n₀'));
}

// ------------------------------------------------------------------ run facts and table

function runKey(menu, kind) { return `${menu}__${kind}`; }
function currentRun() { return state.runs.get(runKey(state.menu, state.kind)); }

function lastComplete(run) {
  const done = run.levels.filter((l) => l.complete);
  return done[done.length - 1];
}

function statusText(run) {
  if (run.status === 'closed') return ['closed', 'Closed: finite'];
  return ['growing', 'Growing'];
}

function reasonText(run) {
  const last = run.levels[run.levels.length - 1];
  if (run.status === 'closed') return `no new grips at depth ${last.depth}`;
  if (run.status === 'depth limit reached') return `still adding grips at the depth limit ${last.depth}`;
  return `${run.status} during depth ${last.depth} (${(100 * last.fraction_of_level_processed).toFixed(1)}% of the level)`;
}

function renderFacts() {
  const run = currentRun();
  const menu = state.menus.get(state.menu);
  const box = $('facts');
  box.replaceChildren();
  if (!run || !menu) return;
  const [cls, label] = statusText(run);
  box.append(el('h2', {}, menu.label));
  box.append(el('div', { class: 'status' }, el('span', { class: `pill ${cls}` }, label),
    el('span', { class: 'caption' }, reasonText(run))));
  const rows = [];
  rows.push(['Closure', KIND_TEXT[run.twisting]]);
  rows.push(['Menu', `${menu.size} rotations: A4 and ${menu.jumble_elements} jumble twists, ${menu.exact_q_sqrt5 ? 'exact in Q(√5)' : 'float only'}`]);
  if (menu.j1_menu_identity) {
    rows.push(['J1 menu identity', el('span', { title: menu.j1_menu_identity, 'data-j1-identity': menu.j1_menu_identity },
      `${menu.j1_menu_identity.slice(0, 16)}…`)]);
    const names = { s4: 'S4₀', i_a: 'I_a', i_b: 'I_b' };
    const relations = { equal: 'equals', contained: 'contained in', 'not-contained': 'not contained in' };
    rows.push(['J1 relation', Object.entries(menu.j1_relation)
      .map(([name, relation]) => `${relations[relation]} ${names[name]}`).join('; ')]);
  }
  if (menu.generator_angle_deg !== undefined) rows.push(['Generator angle', `${menu.generator_angle_deg.toFixed(4)}°`]);
  if (menu.family === 'realignment') {
    rows.push(['Aligned poles', `${menu.aligned_poles} of 56 (by shell ${menu.aligned_per_shell.join(', ')})`]);
  }
  if (menu.cayley_s) rows.push(['Cayley s', `${menu.cayley_s[0]}/${menu.cayley_s[1]}`]);
  const lc = lastComplete(run);
  rows.push(['Deepest complete level', `depth ${lc.depth}`]);
  rows.push(['Grips there', big(lc.cumulative_grips)]);
  if (run.twisting === 'all') rows.push(['Positions there', big(lc.cumulative_positions)]);
  if (run.growth_ratios.length) rows.push(['Growth per level', run.growth_ratios.map((r) => `×${r.toFixed(2)}`).join('  ')]);
  const withSep = run.levels.filter((l) => l.separation);
  const sepLevel = withSep[withSep.length - 1];
  const sep = sepLevel ? sepLevel.separation : null;
  if (sep) rows.push(['Smallest separation', `${angle(sep.min_separation_deg)} at depth ${sepLevel.depth}`]);
  const ex = run.exact_coincidence_check;
  if (ex.menu_exact && ex.checked) {
    rows.push(['Exact checks', `${fmtInt.format(ex.confirmed)} of ${fmtInt.format(ex.checked)} sampled coincidences confirmed (of ${big(ex.coincidences_total)})`]);
  } else if (!ex.menu_exact) {
    rows.push(['Exact checks', 'none: the menu is float only']);
  }
  if (sep && sep.closest_pair_exactly_distinct !== undefined) {
    rows.push(['Closest pair', sep.closest_pair_exactly_distinct ? 'exactly distinct' : 'NOT exactly distinct']);
  }
  const dl = el('dl');
  for (const [k, v] of rows) dl.append(el('dt', {}, k), el('dd', {}, v));
  box.append(dl);
}

function renderTable() {
  const run = currentRun();
  const t = $('levels');
  t.replaceChildren();
  if (!run) return;
  const head = ['Depth', 'New grips', 'Grips', 'Positions', 'Orbit reps', 'Images', 'Coinciding', 'Min separation', 'Median NN', 'Level'];
  t.append(el('thead', {}, el('tr', {}, ...head.map((h) => el('th', { scope: 'col' }, h)))));
  const body = el('tbody');
  for (const l of run.levels) {
    const s = l.separation || {};
    body.append(el('tr', {},
      el('td', {}, String(l.depth)),
      el('td', {}, big(l.new_grips)),
      el('td', {}, big(l.cumulative_grips)),
      el('td', {}, big(l.cumulative_positions)),
      el('td', {}, fmtInt.format(l.cumulative_representatives)),
      el('td', {}, fmtInt.format(l.images_computed)),
      el('td', {}, fmtInt.format(l.images_coinciding)),
      el('td', {}, angle(s.min_separation_deg)),
      el('td', {}, angle(s.median_nearest_neighbour_deg_approx)),
      el('td', {}, l.complete ? 'complete' : `cut at ${(100 * l.fraction_of_level_processed).toFixed(1)}%`),
    ));
  }
  t.append(body);
}

// ------------------------------------------------------------------ growth chart

const tip = el('div', { class: 'tooltip', hidden: '' });
$('chart').append(tip);

function drawChart() {
  const wrap = $('chart');
  const W = Math.max(280, wrap.clientWidth);
  const H = Math.round(Math.min(360, Math.max(230, W * 0.4)));
  const narrow = W < 560;
  const m = { l: 52, r: narrow ? 14 : 132, t: 18, b: 38 };
  const runs = [...state.runs.values()].filter((r) => r.twisting === state.kind);
  const maxDepth = Math.max(...runs.map((r) => r.levels[r.levels.length - 1].depth));
  const maxLog = Math.ceil(Math.log10(Math.max(...runs.flatMap((r) => r.levels.map((l) => l.cumulative_grips)))));
  const minLog = 2;
  const x = (d) => m.l + (d / maxDepth) * (W - m.l - m.r);
  const y = (v) => m.t + ((maxLog - Math.log10(v)) / (maxLog - minLog)) * (H - m.t - m.b);
  const root = svg('svg', { viewBox: `0 0 ${W} ${H}`, width: W, height: H, role: 'img',
    'aria-label': 'Cumulative grips by BFS depth, log scale' });
  const grid = svg('g', { class: 'grid' });
  const axis = svg('g', { class: 'axis' });
  for (let e = minLog; e <= maxLog; e++) {
    grid.append(svg('line', { x1: m.l, x2: W - m.r, y1: y(10 ** e), y2: y(10 ** e) }));
    const t = svg('text', { x: m.l - 8, y: y(10 ** e) + 4, 'text-anchor': 'end' });
    t.textContent = `10${sup(e)}`;
    axis.append(t);
  }
  for (let d = 0; d <= maxDepth; d++) {
    const t = svg('text', { x: x(d), y: H - m.b + 16, 'text-anchor': 'middle' });
    t.textContent = String(d);
    axis.append(t);
  }
  const xt = svg('text', { x: (m.l + W - m.r) / 2, y: H - 4, 'text-anchor': 'middle', class: 'axis-title' });
  xt.textContent = 'BFS depth';
  const yt = svg('text', { x: m.l, y: 10, 'text-anchor': 'start', class: 'axis-title' });
  yt.textContent = 'grips on S³';
  root.append(grid, axis, xt, yt);

  const line = (levels) => levels.map((l, i) => `${i ? 'L' : 'M'}${x(l.depth).toFixed(1)},${y(l.cumulative_grips).toFixed(1)}`).join('');
  const ctx = svg('g');
  for (const r of runs) {
    if (r.menu === state.menu) continue;
    const g = svg('g');
    const hit = svg('path', { d: line(r.levels), class: 'ctx-hit' });
    const vis = svg('path', { d: line(r.levels), class: 'ctx' });
    const title = svg('title');
    title.textContent = state.menus.get(r.menu).label;
    hit.append(title);
    hit.addEventListener('click', () => selectMenu(r.menu));
    g.append(hit, vis);
    ctx.append(g);
  }
  root.append(ctx);

  const run = currentRun();
  if (run) {
    const done = run.levels.filter((l) => l.complete);
    const cut = run.levels.filter((l) => !l.complete);
    root.append(svg('path', { d: line(done), class: 'sel' }));
    if (cut.length) root.append(svg('path', { d: line([done[done.length - 1], ...cut]), class: 'sel partial' }));
    for (const l of run.levels) {
      const cx = x(l.depth);
      const cy = y(l.cumulative_grips);
      root.append(svg('circle', { cx, cy, r: 4.5, class: l.complete ? 'mark' : 'mark open' }));
      const hit = svg('circle', { cx, cy, r: 12, fill: 'transparent' });
      hit.addEventListener('pointerenter', () => {
        tip.textContent = `depth ${l.depth}: ${l.complete ? '' : '≥ '}${big(l.cumulative_grips)} grips (+${big(l.new_grips)})`;
        tip.hidden = false;
        const tw = tip.offsetWidth;
        tip.style.left = `${Math.min(Math.max(cx - tw / 2, 0), W - tw)}px`;
        tip.style.top = `${cy - 40}px`;
      });
      hit.addEventListener('pointerleave', () => { tip.hidden = true; });
      root.append(hit);
    }
    const last = run.levels[run.levels.length - 1];
    if (!narrow) {
      const lab = svg('text', { x: x(last.depth) + 10, y: y(last.cumulative_grips) + 4, class: 'end-label' });
      lab.textContent = `${last.complete ? '' : '≥ '}${big(last.cumulative_grips)}`;
      root.append(lab);
    }
  }
  const old = wrap.querySelector('svg');
  if (old) old.replaceWith(root);
  else wrap.prepend(root);
}
new ResizeObserver(() => { if (state.doc) drawChart(); }).observe($('chart'));

// ------------------------------------------------------------------ selection

async function loadRunPoints() {
  const key = runKey(state.menu, state.kind);
  if (!state.points.has(key)) state.points.set(key, await loadJSON(`points/${key}.json`));
  return state.points.get(key);
}

async function refresh() {
  const run = currentRun();
  const data = await loadRunPoints();
  buildGroups(data);
  const maxDepth = Math.max(0, ...data.depths.map((d) => d.depth));
  const slider = $('depth');
  slider.max = String(maxDepth);
  state.depth = Math.min(Math.max(state.depth, 1), maxDepth);
  if (state.depth === 0 && maxDepth > 0) state.depth = 1;
  slider.value = String(state.depth);
  $('depth-out').textContent = String(state.depth);
  applyDepth();
  renderLegend();
  renderFacts();
  renderTable();
  drawChart();
  if (run && history.replaceState) history.replaceState(null, '', `#${state.menu}`);
}

function selectMenu(name) {
  state.menu = name;
  $('menu').value = name;
  state.depth = 2;
  refresh().catch(showError);
}

function showError(err) {
  $('facts').replaceChildren(el('p', { class: 'error' }, String(err.message || err)));
}

function populateMenus() {
  const sel = $('menu');
  const groups = new Map();
  for (const m of state.menus.values()) {
    if (!groups.has(m.family)) groups.set(m.family, el('optgroup', { label: FAMILY_TEXT[m.family] || m.family }));
    const label = m.family === 'group' ? `${m.label} · group · ${m.size} elements` : m.label;
    groups.get(m.family).append(el('option', { value: m.name }, label));
  }
  for (const g of groups.values()) sel.append(g);
}

async function main() {
  state.doc = await loadJSON('explorer-results.json');
  for (const m of state.doc.menus) state.menus.set(m.name, m);
  for (const r of state.doc.runs) state.runs.set(runKey(r.menu, r.twisting), r);
  populateMenus();
  buildLattice(await loadJSON('points/lattice.json'));
  const fromHash = location.hash.slice(1);
  state.menu = state.menus.has(fromHash) ? fromHash
    : state.menus.has('class-00') ? 'class-00' : state.menus.keys().next().value;
  $('menu').value = state.menu;
  state.depth = 2;
  $('menu').addEventListener('change', (e) => selectMenu(e.target.value));
  for (const r of document.querySelectorAll('input[name="kind"]')) {
    r.addEventListener('change', (e) => { state.kind = e.target.value; refresh().catch(showError); });
  }
  $('depth').addEventListener('input', (e) => {
    state.depth = Number(e.target.value);
    $('depth-out').textContent = String(state.depth);
    applyDepth();
  });
  $('only-depth').addEventListener('change', applyDepth);
  $('show-lattice').addEventListener('change', (e) => { state.lattice.visible = e.target.checked; });
  $('show-ball').addEventListener('change', (e) => { ball.visible = e.target.checked; ballRim.visible = e.target.checked; });
  resize();
  applyTheme();
  await refresh();
  requestAnimationFrame(frame);
}

main().catch(showError);
