// Jumble witness viewer (workstream J2). Research prototype; look, colour and motion values are
// provisional. Legality, grip status and certificates are read from the selected scene, which
// export_scene.py writes from the exact Q(sqrt 5) witness or J1 simulator. Nothing here decides legality: poses
// between exact states are an uncertified float preview along the twist's one-parameter family.
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

const $ = (id) => document.getElementById(id);
const TYPED = { float32: Float32Array, uint8: Uint8Array, uint16: Uint16Array, uint32: Uint32Array, int32: Int32Array };
const ALPHA = 121 / 125;
const SCALE = 3;                       // 3D units per facet distance in the Global view
const F_OFF = 1, F_DIM = 2, F_EMPH = 4, F_FOCUS = 8, F_HIDE = 16;
const SCENES = { witness: 'scene.json', s4: 'scene-s4.json' };

function hashSelection() {
  const m = /^#(s4-)?s(\d+)(?:-g(\d+))?(?:-p(\d+))?$/.exec(location.hash || '');
  return m ? { scene: m[1] ? 's4' : 'witness', state: Number(m[2]),
    grip: m[3] == null ? null : Number(m[3]), piece: m[4] == null ? null : Number(m[4]) } : null;
}
const sceneId = hashSelection()?.scene || 'witness';

THREE.ColorManagement.enabled = false;   // CSS token colours go to the shaders unchanged
const app = { ready: false, errors: [] };
window.jumbleViewer = app;
window.addEventListener('error', (e) => reportError(e.message || String(e)));
window.addEventListener('unhandledrejection', (e) => reportError(String(e.reason && e.reason.message || e.reason)));

function reportError(msg) {
  app.errors.push(msg);
  const box = $('error');
  box.hidden = false;
  box.textContent = `The viewer stopped: ${msg}. Reload the page; if it persists, regenerate the scene with export_scene.py.`;
}

// ---------------------------------------------------------------------------------------------
// 4D linear algebra (row-major 4x4 in Float64Array(16))

const dot4 = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
const norm4 = (a) => Math.sqrt(dot4(a, a));
const scale4 = (a, s) => [a[0] * s, a[1] * s, a[2] * s, a[3] * s];
const sub4 = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]];
const add4 = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]];
const unit4 = (a) => scale4(a, 1 / norm4(a));
function matVec(m, x) {
  return [
    m[0] * x[0] + m[1] * x[1] + m[2] * x[2] + m[3] * x[3],
    m[4] * x[0] + m[5] * x[1] + m[6] * x[2] + m[7] * x[3],
    m[8] * x[0] + m[9] * x[1] + m[10] * x[2] + m[11] * x[3],
    m[12] * x[0] + m[13] * x[1] + m[14] * x[2] + m[15] * x[3],
  ];
}
function matMul(a, b) {
  const out = new Float64Array(16);
  for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) {
    let s = 0;
    for (let k = 0; k < 4; k++) s += a[r * 4 + k] * b[k * 4 + c];
    out[r * 4 + c] = s;
  }
  return out;
}
// R(phi) = I + (cos phi - 1)(u u^T + v v^T) + sin phi (v u^T - u v^T): the exact twist's plane,
// rotated by a partial angle. R(theta) is the exported twist (to float rounding).
function rotFamily(u, v, phi) {
  const c = Math.cos(phi) - 1, s = Math.sin(phi);
  const m = new Float64Array(16);
  for (let r = 0; r < 4; r++) for (let k = 0; k < 4; k++) {
    m[r * 4 + k] = (r === k ? 1 : 0) + c * (u[r] * u[k] + v[r] * v[k]) + s * (v[r] * u[k] - u[r] * v[k]);
  }
  return m;
}
function perpBasis(a) {
  // three orthonormal vectors spanning a-perp, by Gram-Schmidt against the standard basis
  const out = [];
  for (let i = 0; i < 4 && out.length < 3; i++) {
    let v = [0, 0, 0, 0];
    v[i] = 1;
    v = sub4(v, scale4(a, dot4(a, v)));
    for (const b of out) v = sub4(v, scale4(b, dot4(b, v)));
    const n = norm4(v);
    if (n > 1e-6) out.push(scale4(v, 1 / n));
  }
  return out;
}
function det4(rows) {
  const m = rows;
  const d3 = (a, b, c) => a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0]);
  let s = 0;
  for (let j = 0; j < 4; j++) {
    const minor = [1, 2, 3].map((i) => m[i].filter((_, k) => k !== j));
    s += (j % 2 ? -1 : 1) * m[0][j] * d3(minor[0], minor[1], minor[2]);
  }
  return s;
}
function jacobiEigen3(a) {
  // symmetric 3x3: returns eigenvectors (columns) sorted by eigenvalue, largest first
  const A = a.map((r) => r.slice());
  const V = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
  for (let sweep = 0; sweep < 30; sweep++) {
    let off = 0;
    for (let p = 0; p < 3; p++) for (let q = p + 1; q < 3; q++) off += A[p][q] * A[p][q];
    if (off < 1e-24) break;
    for (let p = 0; p < 3; p++) for (let q = p + 1; q < 3; q++) {
      if (Math.abs(A[p][q]) < 1e-30) continue;
      const th = (A[q][q] - A[p][p]) / (2 * A[p][q]);
      const t = Math.sign(th || 1) / (Math.abs(th) + Math.sqrt(th * th + 1));
      const c = 1 / Math.sqrt(t * t + 1), s = t * c;
      for (let k = 0; k < 3; k++) {
        const akp = A[k][p], akq = A[k][q];
        A[k][p] = c * akp - s * akq; A[k][q] = s * akp + c * akq;
      }
      for (let k = 0; k < 3; k++) {
        const apk = A[p][k], aqk = A[q][k];
        A[p][k] = c * apk - s * aqk; A[q][k] = s * apk + c * aqk;
      }
      for (let k = 0; k < 3; k++) {
        const vkp = V[k][p], vkq = V[k][q];
        V[k][p] = c * vkp - s * vkq; V[k][q] = s * vkp + c * vkq;
      }
    }
  }
  const order = [0, 1, 2].sort((i, j) => A[j][j] - A[i][i]);
  return order.map((i) => [V[0][i], V[1][i], V[2][i]]);
}

// ---------------------------------------------------------------------------------------------
// scene data

async function loadScene() {
  const file = SCENES[sceneId];
  const hRes = await fetch(file);
  if (!hRes.ok) throw new Error(`${file} could not be loaded (HTTP ${hRes.status})`);
  const header = await hRes.json();
  const bRes = await fetch(header.bin.file);
  if (!bRes.ok) throw new Error(`${header.bin.file} could not be loaded (HTTP ${bRes.status})`);
  // hosts that serve no binary type get the same bytes as base64 text (header.bin.encoding)
  const buf = header.bin.encoding === 'base64'
    ? Uint8Array.from(atob((await bRes.text()).trim()), (ch) => ch.charCodeAt(0)).buffer
    : await bRes.arrayBuffer();
  if (buf.byteLength !== header.bin.bytes) throw new Error(`${header.bin.file} does not match ${file}`);
  const hash = await crypto.subtle.digest('SHA-256', buf);
  const digest = Array.from(new Uint8Array(hash), (b) => b.toString(16).padStart(2, '0')).join('');
  if (digest !== header.bin.sha256) throw new Error(`Scene data SHA-256 digest does not match ${file}`);
  const A = {};
  for (const [name, d] of Object.entries(header.bin.arrays)) A[name] = new TYPED[d.dtype](buf, d.offset, d.length);
  return buildModel(header, A);
}

function buildModel(header, A) {
  const P = A.piece_ids.length, S = A.sticker_piece.length, K = header.states.length;
  const m = { header, A, P, S, K, c: header.c, d: header.d };
  m.vert = (i, j) => {
    const o = (A.vert_start[i] + j) * 4;
    return [A.verts[o], A.verts[o + 1], A.verts[o + 2], A.verts[o + 3]];
  };
  m.nVerts = (i) => A.vert_start[i + 1] - A.vert_start[i];
  m.cells = new Map(header.cells.map((c) => [c.id, c]));
  m.stickers = [];
  m.pieceStickers = Array.from({ length: P }, () => []);
  let gv = 0, nt = 0;
  for (let s = 0; s < S; s++) {
    const p = A.sticker_piece[s];
    const map = new Map(), local = [], tris = [];
    for (let t = A.tri_start[s] * 3; t < A.tri_start[s + 1] * 3; t++) {
      const j = A.tris[t];
      if (!map.has(j)) { map.set(j, local.length); local.push(j); }
      tris.push(map.get(j));
    }
    const cen = [0, 0, 0, 0];
    for (const j of local) { const v = m.vert(p, j); for (let k = 0; k < 4; k++) cen[k] += v[k] / local.length; }
    const edges = [];
    for (let e = A.edge_start[p]; e < A.edge_start[p + 1]; e++) {
      const a = A.edges[2 * e], b = A.edges[2 * e + 1];
      if (map.has(a) && map.has(b)) edges.push(map.get(a), map.get(b));
    }
    m.stickers.push({ s, p, host: A.sticker_host[s], local, map, tris, edges, cen, gv0: gv, t0: nt });
    m.pieceStickers[p].push(s);
    gv += local.length;
    nt += tris.length / 3;
  }
  m.nGV = gv;
  m.nTris = nt;
  m.poses = header.poses.map((p) => Float64Array.from(p.m));
  m.latticePoses = header.poses.map((p) => Float64Array.from(p.nearest_lattice));
  m.statePose = (k, i) => A.state_pose[k * P + i];
  m.onLattice = (k, i) => A.state_lattice[k * P + i] === 1;
  m.certs = [];
  for (let k = 0; k < K; k++) {
    const byGrip = new Map();
    for (let r = A.cert_start[k]; r < A.cert_start[k + 1]; r++) {
      const g = A.cert_grip[r];
      if (!byGrip.has(g)) byGrip.set(g, []);
      byGrip.get(g).push(r);
    }
    m.certs.push(byGrip);
  }
  m.cert = (r) => ({ piece: A.cert_piece[r], vb: A.cert_vertex_below[r], va: A.cert_vertex_above[r],
                     hb: A.cert_h_below[r], ha: A.cert_h_above[r] });
  m.grips = header.grips.patch;
  m.gripUnit = (e) => header.grips.units[String(e)];
  m.pieceById = new Map();
  A.piece_ids.forEach((id, i) => m.pieceById.set(id, i));
  return m;
}

// ---------------------------------------------------------------------------------------------
// state

let M = null;                          // the model
const view = {
  t: 2,                                // position along the sequence; integers are exact states
  grip: 0,                             // selected grip (pole id) or null
  piece: null,                         // focus piece (index) or null
  proj: 'persp', eye: 'mid', dist: 3, cell: 0.78, sticker: 0.86,
  show: 'all', grips: true, cage: true, ghost: true, dim: true, cull: true,
};
let frame = null;                      // per-piece poses at view.t
let PAL = [];                          // host-cell colour classes, read from the CSS tokens
const dirty = { pose: true, flags: true, local: true, panels: true, home: true };
let localKey = '';
let localFrame = null;

function stateAt(t) {
  let k = Math.floor(t + 1e-9), f = t - k;
  if (k >= M.K - 1) { k = M.K - 1; f = 0; }
  if (f < 1e-6) f = 0;
  return { k, f };
}

function computeFrame(t) {
  const { k, f } = stateAt(t);
  const tw = f > 0 ? M.header.twists[k] : null;
  const R = tw ? rotFamily(tw.u, tw.v, tw.theta * f) : null;
  const cache = new Map(), mats = new Array(M.P), moving = new Uint8Array(M.P);
  for (let i = 0; i < M.P; i++) {
    const a = M.statePose(k, i);
    const mv = tw && M.statePose(k + 1, i) !== a;
    const key = mv ? a + 1000 : a;
    let mat = cache.get(key);
    if (!mat) { mat = mv ? matMul(R, M.poses[a]) : M.poses[a]; cache.set(key, mat); }
    mats[i] = mat;
    moving[i] = mv ? 1 : 0;
  }
  return { t, k, f, exact: f === 0, twist: tw, mats, moving };
}

function isOff(p) {
  // Every moving piece has an uncertified intermediate pose, including retained turns.
  return frame.moving[p] === 1 || !M.onLattice(frame.k, p);
}

function straddleSet(k, grip) {
  const set = new Set();
  const recs = grip == null ? null : M.certs[k].get(grip);
  if (recs) for (const r of recs) set.add(M.A.cert_piece[r]);
  return set;
}

function certFor(k, grip, piece) {
  const recs = grip == null ? null : M.certs[k].get(grip);
  if (!recs) return null;
  for (const r of recs) if (M.A.cert_piece[r] === piece) return M.cert(r);
  return null;
}

// ---------------------------------------------------------------------------------------------
// theme tokens

function token(name) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || '#888888';
}
function colour(name) { return new THREE.Color(token(name)); }

// ---------------------------------------------------------------------------------------------
// shaders

const STICKER_VS = `
attribute vec3 aColor;
attribute float aFlags;
varying vec3 vColor;
varying float vFlags;
varying vec3 vView;
void main() {
  vec4 mv = modelViewMatrix * vec4(position, 1.0);
  vView = mv.xyz;
  vColor = aColor;
  vFlags = aFlags;
  gl_Position = projectionMatrix * mv;
}`;
const STICKER_FS = `
uniform vec3 uBg;
uniform vec3 uAccent;
uniform vec3 uEmph;
uniform float uAlpha;
uniform float uPx;
varying vec3 vColor;
varying float vFlags;
varying vec3 vView;
float bit(float f, float b) { return mod(floor(f / b + 0.01), 2.0); }
void main() {
  if (bit(vFlags, 16.0) > 0.5) discard;
  vec3 n = normalize(cross(dFdx(vView), dFdy(vView)));
  vec3 l = normalize(vec3(0.35, 0.62, 0.70));
  float lam = abs(dot(n, l));
  vec3 col = vColor * (0.40 + 0.62 * lam);
  if (bit(vFlags, 1.0) > 0.5) {
    float s = step(0.5, fract((gl_FragCoord.x + gl_FragCoord.y) / (7.0 * uPx)));
    col *= mix(1.0, 0.58, s);
  }
  if (bit(vFlags, 4.0) > 0.5) col = mix(col, uEmph, 0.18);
  if (bit(vFlags, 8.0) > 0.5) col = mix(col, uAccent, 0.5);
  if (bit(vFlags, 2.0) > 0.5) col = mix(col, uBg, 0.6);
  gl_FragColor = vec4(col, uAlpha);
}`;
const GRIP_VS = `
attribute float aState;
attribute float aSel;
uniform float uSize;
varying float vState;
varying float vSel;
void main() {
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  float k = aState > 1.5 ? 1.0 : aState > 0.5 ? 0.72 : 0.6;
  gl_PointSize = uSize * k * (aSel > 0.5 ? 1.8 : 1.0);
  vState = aState;
  vSel = aSel;
}`;
const GRIP_FS = `
uniform vec3 uOk;
uniform vec3 uBlock;
uniform vec3 uNeutral;
uniform vec3 uInk;
uniform vec3 uAccent;
varying float vState;
varying float vSel;
void main() {
  vec2 p = gl_PointCoord * 2.0 - 1.0;
  float r = length(p);
  vec3 col = uNeutral;
  float a = 0.0;
  if (vState < 0.5) {
    a = 1.0 - smoothstep(0.30, 0.40, r);
  } else if (vState < 1.5) {
    a = smoothstep(0.40, 0.50, r) * (1.0 - smoothstep(0.84, 0.94, r));
    col = r > 0.74 ? uInk : uOk;
  } else {
    float dm = abs(p.x) + abs(p.y);
    a = 1.0 - smoothstep(0.86, 0.96, dm);
    col = dm > 0.66 ? uInk : uBlock;
  }
  if (vSel > 0.5) {
    float halo = smoothstep(0.90, 0.95, r) * (1.0 - smoothstep(0.98, 1.0, r));
    if (halo > a) { a = halo; col = uAccent; }
  }
  if (a < 0.03) discard;
  gl_FragColor = vec4(col, a);
}`;
const DOT_VS = `
attribute vec3 aColor;
uniform float uSize;
varying vec3 vColor;
void main() {
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  gl_PointSize = uSize;
  vColor = aColor;
}`;
const DOT_FS = `
uniform vec3 uInk;
varying vec3 vColor;
void main() {
  vec2 p = gl_PointCoord * 2.0 - 1.0;
  float r = length(p);
  if (r > 1.0) discard;
  gl_FragColor = vec4(r > 0.62 ? uInk : vColor, 1.0);
}`;

function stickerMaterial(alpha) {
  return new THREE.ShaderMaterial({
    vertexShader: STICKER_VS, fragmentShader: STICKER_FS, side: THREE.DoubleSide,
    transparent: alpha < 1, depthWrite: alpha >= 1,
    uniforms: { uBg: { value: new THREE.Color() }, uAccent: { value: new THREE.Color() }, uEmph: { value: new THREE.Color() },
                uAlpha: { value: alpha }, uPx: { value: 1 } },
  });
}
function dotMaterial(size) {
  return new THREE.ShaderMaterial({
    vertexShader: DOT_VS, fragmentShader: DOT_FS, depthTest: false, transparent: true,
    uniforms: { uSize: { value: size }, uInk: { value: new THREE.Color() } },
  });
}

// ---------------------------------------------------------------------------------------------
// Global view

const G = {};

function eyeDir() {
  const nc = M.gripUnit(M.c), nd = M.gripUnit(M.d);
  if (view.eye === 'c') return nc;
  if (view.eye === 'd') return nd;
  return unit4(add4(nc, nd));
}

function makeProjector() {
  const e = eyeDir();
  const nc = M.gripUnit(M.c), nd = M.gripUnit(M.d);
  let b1 = sub4(nd, nc);
  b1 = unit4(sub4(b1, scale4(e, dot4(e, b1))));
  const rest = perpBasis(e).map((v) => sub4(v, scale4(b1, dot4(b1, v))));
  let b2 = unit4(rest.reduce((best, v) => (norm4(v) > norm4(best) ? v : best)));
  let b3 = rest.map((v) => sub4(v, scale4(b2, dot4(b2, v)))).reduce((best, v) => (norm4(v) > norm4(best) ? v : best));
  b3 = unit4(b3);
  if (det4([e, b1, b2, b3]) < 0) b3 = scale4(b3, -1);
  const D = view.dist;
  const eye4 = scale4(e, D);
  const proj = view.proj;
  const fn = (x, out, o) => {
    let s;
    if (proj === 'persp') {
      s = SCALE * (D - 1) / (D - dot4(e, x));
    } else {
      const r = norm4(x);
      s = SCALE * 2 / (r + dot4(e, x));
    }
    out[o] = s * dot4(b1, x);
    out[o + 1] = s * dot4(b2, x);
    out[o + 2] = s * dot4(b3, x);
  };
  return { fn, e, eye4, persp: proj === 'persp' };
}

function setupGlobal() {
  G.renderer = new THREE.WebGLRenderer({ canvas: $('gCanvas'), antialias: true });
  G.renderer.outputColorSpace = THREE.LinearSRGBColorSpace;
  G.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  G.scene = new THREE.Scene();
  G.camera = new THREE.PerspectiveCamera(36, 1, 0.01, 200);
  G.camera.position.set(1.7, 1.9, 6.3);
  G.controls = new OrbitControls(G.camera, $('gCanvas'));
  G.controls.addEventListener('change', () => { needRender = true; });

  // stickers: one merged geometry, positions recomputed from 4D on every pose or projection change
  const geom = new THREE.BufferGeometry();
  G.pos = new Float32Array(M.nGV * 3);
  G.col = new Float32Array(M.nGV * 3);
  G.flags = new Float32Array(M.nGV);
  G.home = new Float32Array(M.nGV * 4);
  const index = new Uint32Array(M.nTris * 3);
  G.triSticker = new Uint32Array(M.nTris);
  for (const st of M.stickers) {
    for (let i = 0; i < st.tris.length; i++) index[st.t0 * 3 + i] = st.gv0 + st.tris[i];
    for (let i = 0; i < st.tris.length / 3; i++) G.triSticker[st.t0 + i] = st.s;
  }
  geom.setAttribute('position', new THREE.BufferAttribute(G.pos, 3));
  geom.setAttribute('aColor', new THREE.BufferAttribute(G.col, 3));
  geom.setAttribute('aFlags', new THREE.BufferAttribute(G.flags, 1));
  geom.setIndex(new THREE.BufferAttribute(index, 1));
  G.geom = geom;
  G.mat = stickerMaterial(1);
  G.mesh = new THREE.Mesh(geom, G.mat);
  G.mesh.frustumCulled = false;
  G.scene.add(G.mesh);

  // lattice cage: world-fixed tetrahedral cells hosting stickers (edges subdivided for stereographic)
  const SUB = 8;
  G.cageSub = SUB;
  G.cages = [];
  for (const [mine, opacity] of [[true, 0.6], [false, 0.16]]) {
    const edges = [];
    for (const cell of M.header.cells) {
      if ((cell.id === M.c || cell.id === M.d) !== mine) continue;
      const T = cell.tetra;
      for (let i = 0; i < 4; i++) for (let j = i + 1; j < 4; j++) edges.push([cell.centre, T[i], T[j]]);
    }
    const pos = new Float32Array(edges.length * SUB * 2 * 3);
    const cg = new THREE.BufferGeometry();
    cg.setAttribute('position', new THREE.BufferAttribute(pos, 3));
    const mat = new THREE.LineBasicMaterial({ transparent: true, opacity, depthWrite: false });
    const obj = new THREE.LineSegments(cg, mat);
    obj.frustumCulled = false;
    G.scene.add(obj);
    G.cages.push({ edges, pos, mat, obj });
  }

  // focus outline and lattice ghost
  G.focusMat = new THREE.LineBasicMaterial({ depthTest: false, transparent: true, opacity: 0.95 });
  G.focus = new THREE.LineSegments(new THREE.BufferGeometry(), G.focusMat);
  G.focus.renderOrder = 5;
  G.focus.frustumCulled = false;
  G.scene.add(G.focus);
  G.ghostMat = new THREE.LineDashedMaterial({ depthTest: false, transparent: true, opacity: 0.85, dashSize: 0.03, gapSize: 0.025 });
  G.ghost = new THREE.LineSegments(new THREE.BufferGeometry(), G.ghostMat);
  G.ghost.renderOrder = 4;
  G.ghost.frustumCulled = false;
  G.scene.add(G.ghost);

  // grip markers at the projected poles (world-fixed grips)
  const gg = new THREE.BufferGeometry();
  G.gripPos = new Float32Array(M.grips.length * 3);
  G.gripState = new Float32Array(M.grips.length);
  G.gripSel = new Float32Array(M.grips.length);
  gg.setAttribute('position', new THREE.BufferAttribute(G.gripPos, 3));
  gg.setAttribute('aState', new THREE.BufferAttribute(G.gripState, 1));
  gg.setAttribute('aSel', new THREE.BufferAttribute(G.gripSel, 1));
  G.gripMat = new THREE.ShaderMaterial({
    vertexShader: GRIP_VS, fragmentShader: GRIP_FS, transparent: true, depthTest: false,
    uniforms: { uSize: { value: 15 }, uOk: { value: new THREE.Color() }, uBlock: { value: new THREE.Color() },
                uNeutral: { value: new THREE.Color() }, uInk: { value: new THREE.Color() }, uAccent: { value: new THREE.Color() } },
  });
  G.gripPts = new THREE.Points(gg, G.gripMat);
  G.gripPts.renderOrder = 10;
  G.gripPts.frustumCulled = false;
  G.scene.add(G.gripPts);

  // certificate points
  const cpg = new THREE.BufferGeometry();
  G.certPos = new Float32Array(6);
  G.certCol = new Float32Array(6);
  cpg.setAttribute('position', new THREE.BufferAttribute(G.certPos, 3));
  cpg.setAttribute('aColor', new THREE.BufferAttribute(G.certCol, 3));
  G.certMat = dotMaterial(11);
  G.certPts = new THREE.Points(cpg, G.certMat);
  G.certPts.renderOrder = 11;
  G.certPts.frustumCulled = false;
  G.scene.add(G.certPts);

  addPicking($('gCanvas'), pickGlobal);
}

function refreshHome() {
  // cell shrink toward the host cell centre and sticker shrink toward the sticker centroid, in
  // the piece's home frame; the pose then carries both centres with the piece
  const sc = view.cell, ss = view.sticker;
  for (const st of M.stickers) {
    const C = M.cells.get(st.host).centre;
    for (let j = 0; j < st.local.length; j++) {
      const x = M.vert(st.p, st.local[j]);
      const o = (st.gv0 + j) * 4;
      for (let k = 0; k < 4; k++) {
        const y = st.cen[k] + ss * (x[k] - st.cen[k]);
        G.home[o + k] = C[k] + sc * (y - C[k]);
      }
    }
  }
}

function updateGlobalGeometry() {
  const P = makeProjector();
  G.projector = P;
  const tmp = [0, 0, 0, 0];
  for (const st of M.stickers) {
    const m = frame.mats[st.p];
    for (let j = 0; j < st.local.length; j++) {
      const o = (st.gv0 + j) * 4;
      tmp[0] = G.home[o]; tmp[1] = G.home[o + 1]; tmp[2] = G.home[o + 2]; tmp[3] = G.home[o + 3];
      P.fn(matVec(m, tmp), G.pos, (st.gv0 + j) * 3);
    }
  }
  G.geom.attributes.position.needsUpdate = true;

  // cage: cell shrink only, identity pose
  const sc = view.cell, SUB = G.cageSub;
  const pt = (C, a, b, u) => {
    const x = [0, 0, 0, 0];
    for (let k = 0; k < 4; k++) x[k] = C[k] + sc * (a[k] + u * (b[k] - a[k]) - C[k]);
    return x;
  };
  for (const cage of G.cages) {
    let o = 0;
    for (const [C, a, b] of cage.edges) {
      for (let s = 0; s < SUB; s++) {
        P.fn(pt(C, a, b, s / SUB), cage.pos, o); o += 3;
        P.fn(pt(C, a, b, (s + 1) / SUB), cage.pos, o); o += 3;
      }
    }
    cage.obj.geometry.attributes.position.needsUpdate = true;
    cage.obj.visible = view.cage;
  }

  // grips
  M.grips.forEach((e, i) => P.fn(M.gripUnit(e), G.gripPos, i * 3));
  G.gripPts.geometry.attributes.position.needsUpdate = true;
  updateFocusLines();
}

function stickerLines(p, pose, out) {
  const P = G.projector;
  const a = [0, 0, 0], b = [0, 0, 0];
  for (const s of M.pieceStickers[p]) {
    const st = M.stickers[s];
    const pt = (i) => {
      const o = (st.gv0 + i) * 4;
      return matVec(pose, [G.home[o], G.home[o + 1], G.home[o + 2], G.home[o + 3]]);
    };
    for (let e = 0; e < st.edges.length; e += 2) {
      P.fn(pt(st.edges[e]), a, 0);
      P.fn(pt(st.edges[e + 1]), b, 0);
      out.push(a[0], a[1], a[2], b[0], b[1], b[2]);
    }
  }
}

function updateFocusLines() {
  const p = view.piece;
  const f = [], g = [];
  if (p != null) {
    stickerLines(p, frame.mats[p], f);
    if (view.ghost && frame.exact && !M.onLattice(frame.k, p)) stickerLines(p, M.latticePoses[M.statePose(frame.k, p)], g);
  }
  G.focus.geometry.dispose();
  G.focus.geometry = new THREE.BufferGeometry();
  G.focus.geometry.setAttribute('position', new THREE.Float32BufferAttribute(f, 3));
  G.ghost.geometry.dispose();
  G.ghost.geometry = new THREE.BufferGeometry();
  G.ghost.geometry.setAttribute('position', new THREE.Float32BufferAttribute(g, 3));
  G.ghost.computeLineDistances();

  // certificate points of the focus piece for the selected grip at an exact state
  const cert = frame.exact ? certFor(frame.k, view.grip, p) : null;
  G.certPts.visible = !!cert;
  if (cert) {
    const m = frame.mats[p];
    // Certificates belong to region vertices; sticker shrink must never move their points.
    G.certWorld = [matVec(m, M.vert(p, cert.vb)), matVec(m, M.vert(p, cert.va))];
    G.certWorld.forEach((x, i) => G.projector.fn(x, G.certPos, i * 3));
    G.certPts.geometry.attributes.position.needsUpdate = true;
  }
}

function updateGlobalFlags() {
  const { k, exact } = frame;
  const strad = exact ? straddleSet(k, view.grip) : new Set();
  const dimOthers = view.dim && strad.size > 0;
  const P = G.projector;
  for (const st of M.stickers) {
    const p = st.p;
    let f = isOff(p) ? F_OFF : 0;
    if (strad.has(p)) f |= F_EMPH;
    if (p === view.piece) f |= F_FOCUS;
    if (dimOthers && !(f & (F_EMPH | F_FOCUS))) f |= F_DIM;
    if (view.show === 'cd' && st.host !== M.c && st.host !== M.d) f |= F_HIDE;
    if (view.show === 'straddle' && !(f & (F_EMPH | F_FOCUS))) f |= F_HIDE;
    if (view.cull && P.persp) {
      const n = matVec(frame.mats[p], M.cells.get(st.host).centre);
      if (dot4(n, P.eye4) <= 1) f |= F_HIDE;
    }
    st.hidden = !!(f & F_HIDE);
    for (let j = 0; j < st.local.length; j++) G.flags[st.gv0 + j] = f;
  }
  G.geom.attributes.aFlags.needsUpdate = true;
  const status = M.header.states[k].grip_status;
  M.grips.forEach((e, i) => {
    G.gripState[i] = exact ? (status[e] === 'b' ? 2 : 1) : 0;
    G.gripSel[i] = e === view.grip ? 1 : 0;
  });
  G.gripPts.geometry.attributes.aState.needsUpdate = true;
  G.gripPts.geometry.attributes.aSel.needsUpdate = true;
  G.gripPts.visible = view.grips;
}

function applyThemeColours() {
  const bg = colour('--view-bg');
  PAL = [0, 1, 2, 3, 4, 5, 6, 7].map((i) => colour(`--cell-${i}`));
  for (const st of M.stickers) {
    const c = PAL[M.cells.get(st.host).colour % PAL.length];
    for (let j = 0; j < st.local.length; j++) G.col.set([c.r, c.g, c.b], (st.gv0 + j) * 3);
  }
  G.geom.attributes.aColor.needsUpdate = true;
  for (const R of [G, L]) {
    R.renderer.setClearColor(bg, 1);
  }
  for (const mat of [G.mat, L.mat, L.otherMat, L.ghostFill]) {
    mat.uniforms.uBg.value.copy(bg);
    mat.uniforms.uAccent.value.copy(colour('--accent'));
    mat.uniforms.uEmph.value.copy(colour('--block'));
    mat.uniforms.uPx.value = G.renderer.getPixelRatio();
  }
  for (const cage of G.cages) cage.mat.color.copy(colour('--cage'));
  G.focusMat.color.copy(colour('--accent'));
  G.ghostMat.color.copy(colour('--ghost'));
  const gu = G.gripMat.uniforms;
  gu.uOk.value.copy(colour('--ok')); gu.uBlock.value.copy(colour('--block')); gu.uNeutral.value.copy(colour('--muted'));
  gu.uInk.value.copy(colour('--ink')); gu.uAccent.value.copy(colour('--accent'));
  G.gripMat.uniforms.uSize.value = 13 * G.renderer.getPixelRatio();
  const below = colour('--below'), above = colour('--above');
  for (const R of [G, L]) {
    R.certCol.set([below.r, below.g, below.b, above.r, above.g, above.b]);
    R.certPts.geometry.attributes.aColor.needsUpdate = true;
    R.certMat.uniforms.uInk.value.copy(colour('--ink'));
    R.certMat.uniforms.uSize.value = 11 * R.renderer.getPixelRatio();
  }
  L.edgeMat.color.copy(colour('--ink'));
  L.ghostMat.color.copy(colour('--ghost'));
  L.planeMat.color.copy(colour('--plane'));
  L.gridMat.color.copy(colour('--plane'));
  dirty.local = true;
  needRender = true;
}

// ---------------------------------------------------------------------------------------------
// Local view

const L = {};

function setupLocal() {
  L.renderer = new THREE.WebGLRenderer({ canvas: $('lCanvas'), antialias: true });
  L.renderer.outputColorSpace = THREE.LinearSRGBColorSpace;
  L.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  L.scene = new THREE.Scene();
  L.camera = new THREE.PerspectiveCamera(34, 1, 0.001, 100);
  L.controls = new OrbitControls(L.camera, $('lCanvas'));
  L.controls.addEventListener('change', () => { needRender = true; });
  L.group = new THREE.Group();
  L.scene.add(L.group);
  L.mat = stickerMaterial(0.8);
  L.otherMat = stickerMaterial(0.2);
  L.ghostFill = stickerMaterial(0.12);
  L.edgeMat = new THREE.LineBasicMaterial({ transparent: true, opacity: 0.9 });
  L.ghostMat = new THREE.LineDashedMaterial({ transparent: true, opacity: 0.9, dashSize: 0.06, gapSize: 0.045 });
  L.planeMat = new THREE.MeshBasicMaterial({ transparent: true, opacity: 0.12, side: THREE.DoubleSide, depthWrite: false });
  L.gridMat = new THREE.LineBasicMaterial({ transparent: true, opacity: 0.4 });
  const cpg = new THREE.BufferGeometry();
  L.certPos = new Float32Array(6);
  L.certCol = new Float32Array(6);
  cpg.setAttribute('position', new THREE.BufferAttribute(L.certPos, 3));
  cpg.setAttribute('aColor', new THREE.BufferAttribute(L.certCol, 3));
  L.certMat = dotMaterial(11);
  L.certPts = new THREE.Points(cpg, L.certMat);
  L.certPts.renderOrder = 11;
  L.certPts.frustumCulled = false;
  L.scene.add(L.certPts);
  L.pickables = [];
  addPicking($('lCanvas'), pickLocal);
}

function posedRegion(p, mat) {
  const out = [];
  for (let j = 0; j < M.nVerts(p); j++) out.push(matVec(mat, M.vert(p, j)));
  return out;
}

function computeLocalFrame(mode, grip, p) {
  const k = frame.k;
  const pts = posedRegion(p, frame.mats[p]);
  let a, o, host = null;
  if (mode === 'grip') {
    a = M.gripUnit(grip);
    o = ALPHA;
  } else {
    host = M.stickers[M.pieceStickers[p][0]].host;
    a = matVec(M.latticePoses[M.statePose(k, p)], M.cells.get(host).centre);
    o = 1;
  }
  const x0 = [0, 0, 0, 0];
  for (const x of pts) for (let i = 0; i < 4; i++) x0[i] += x[i] / pts.length;
  const Q = perpBasis(a);
  const cov = [[0, 0, 0], [0, 0, 0], [0, 0, 0]];
  for (const x of pts) {
    const y = sub4(x, x0);
    const q = Q.map((b) => dot4(b, y));
    for (let i = 0; i < 3; i++) for (let j = 0; j < 3; j++) cov[i][j] += q[i] * q[j];
  }
  const ev = jacobiEigen3(cov);
  const comb = (w) => add4(add4(scale4(Q[0], w[0]), scale4(Q[1], w[1])), scale4(Q[2], w[2]));
  const b1 = unit4(comb(ev[0])), b2 = unit4(comb(ev[1]));
  // scale: the focus piece spans about two units
  let r = 1e-9;
  for (const x of pts) {
    const y = sub4(x, x0);
    r = Math.max(r, Math.hypot(dot4(b1, y), dot4(a, x) - o - (dot4(a, x0) - o), dot4(b2, y)));
  }
  return { mode, grip, host, a, o, x0, b1, b2, k: 1 / r, raw: r };
}

function toLocal(F, x) {
  const y = sub4(x, F.x0);
  return [dot4(F.b1, y) * F.k, (dot4(F.a, x) - F.o) * F.k, dot4(F.b2, y) * F.k];
}

function pieceMeshLocal(F, p, mat, material, pickInfo, flagOverride) {
  const pos = [], col = [], flags = [], idx = [];
  const off = flagOverride != null ? flagOverride : isOff(p) ? F_OFF : 0;
  for (const s of M.pieceStickers[p]) {
    const st = M.stickers[s];
    const base = pos.length / 3;
    const c = PAL[M.cells.get(st.host).colour % PAL.length];
    for (const j of st.local) {
      pos.push(...toLocal(F, matVec(mat, M.vert(p, j))));
      col.push(c.r, c.g, c.b);
      flags.push(off);
    }
    for (const t of st.tris) idx.push(base + t);
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  g.setAttribute('aColor', new THREE.Float32BufferAttribute(col, 3));
  g.setAttribute('aFlags', new THREE.Float32BufferAttribute(flags, 1));
  g.setIndex(idx);
  const mesh = new THREE.Mesh(g, material);
  if (pickInfo != null) { mesh.userData.piece = pickInfo; L.pickables.push(mesh); }
  return mesh;
}

function regionEdgesLocal(F, p, mat, material) {
  const pos = [];
  const A = M.A;
  for (let e = A.edge_start[p]; e < A.edge_start[p + 1]; e++) {
    pos.push(...toLocal(F, matVec(mat, M.vert(p, A.edges[2 * e]))), ...toLocal(F, matVec(mat, M.vert(p, A.edges[2 * e + 1]))));
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  const ls = new THREE.LineSegments(g, material);
  if (material.isLineDashedMaterial) ls.computeLineDistances();
  return ls;
}

function clearLocal() {
  for (const ch of [...L.group.children]) {
    L.group.remove(ch);
    if (ch.geometry) ch.geometry.dispose();
  }
  L.pickables = [];
  L.certPts.visible = false;
  L.tags = [];
}

function buildLocal() {
  clearLocal();
  const p = view.piece;
  const hint = $('lHint');
  if (p == null) {
    $('lWhat').textContent = 'no focus piece';
    hint.textContent = 'Select a blocked grip or a piece in Global.';
    localKey = '';
    return;
  }
  const mode = view.grip != null ? 'grip' : 'piece';
  const key = `${mode}:${view.grip}:${p}`;
  const refit = key !== localKey || (frame.exact && localFrame && localFrame.state !== frame.k);
  if (refit) {
    localFrame = computeLocalFrame(mode, view.grip, p);
    localFrame.state = frame.k;
  }
  const mat = frame.mats[p];
  const cur = posedRegion(p, mat).reduce((s, x) => add4(s, x), [0, 0, 0, 0]);
  const F = { ...localFrame, x0: scale4(cur, 1 / M.nVerts(p)) };
  L.group.add(pieceMeshLocal(F, p, mat, L.mat, null));
  L.group.add(regionEdgesLocal(F, p, mat, L.edgeMat));

  // extent of the focus piece in local units
  const pts = posedRegion(p, mat).map((x) => toLocal(F, x));
  const box = new THREE.Box3();
  for (const q of pts) box.expandByPoint(new THREE.Vector3(...q));
  box.expandByPoint(new THREE.Vector3(box.min.x, 0, box.min.z));
  const size = Math.max(box.max.x - box.min.x, box.max.z - box.min.z, 1.2) * 1.3;

  // anchor plane: the cut of the selected grip (h = 0) or the facet of the lattice slot
  const cx = (box.min.x + box.max.x) / 2, cz = (box.min.z + box.max.z) / 2;
  const plane = new THREE.Mesh(new THREE.PlaneGeometry(size, size), L.planeMat);
  plane.rotation.x = -Math.PI / 2;
  plane.position.set(cx, 0, cz);
  L.group.add(plane);
  const grid = new THREE.GridHelper(size, 8);
  grid.material = L.gridMat;
  grid.position.set(cx, 0, cz);
  L.group.add(grid);
  L.tags.push({ at: [cx - size * 0.42, 0, cz + size * 0.3], cls: 'axis',
    text: mode === 'grip' ? `cut of grip ${gripName(view.grip)}: h = 0` : `facet of cell ${F.host}` });

  if (mode === 'grip') {
    $('lWhat').innerHTML = `piece ${M.A.piece_ids[p]} against the cut of grip ${gripName(view.grip)} · vertical: <span class="math">h</span>`;
    const cert = frame.exact ? certFor(frame.k, view.grip, p) : null;
    if (cert) {
      const qb = toLocal(F, matVec(mat, M.vert(p, cert.vb)));
      const qa = toLocal(F, matVec(mat, M.vert(p, cert.va)));
      L.certPos.set([...qb, ...qa]);
      L.certPts.geometry.attributes.position.needsUpdate = true;
      L.certPts.visible = true;
      L.tags.push({ at: qb, cls: 'below', text: `h = ${fmtSigned(cert.hb, 5)}` });
      L.tags.push({ at: qa, cls: 'above', text: `h = ${fmtSigned(cert.ha, 5)}` });
    }
    // neighbouring straddling pieces, dimmed and selectable
    if (frame.exact) {
      const recs = M.certs[frame.k].get(view.grip) || [];
      const x0 = F.x0;
      const near = recs.map((r) => M.A.cert_piece[r]).filter((q) => q !== p).map((q) => {
        const c = posedRegion(q, frame.mats[q]).reduce((s, x) => add4(s, x), [0, 0, 0, 0]);
        return [norm4(sub4(scale4(c, 1 / M.nVerts(q)), x0)), q];
      }).sort((u, v) => u[0] - v[0]).slice(0, 12);
      for (const [, q] of near) L.group.add(pieceMeshLocal(F, q, frame.mats[q], L.otherMat, q, F_DIM));
    }
    const status = frame.exact ? M.header.states[frame.k].grip_status[view.grip] : null;
    hint.textContent = !frame.exact ? 'Float preview: no certificate between exact states.'
      : status === 'b' ? (cert ? 'Exact certificate: one vertex below the cut, one above.' : 'This piece does not straddle; pick a highlighted piece.')
        : 'Grip admissible here: no piece straddles its cut.';
  } else {
    $('lWhat').innerHTML = `piece ${M.A.piece_ids[p]} against the facet of cell ${F.host}`
      + (frame.exact ? ' at its lattice slot' : ' · float preview');
    const lat = M.latticePoses[M.statePose(frame.k, p)];
    if (view.ghost && !M.onLattice(frame.k, p)) {
      L.group.add(pieceMeshLocal(F, p, lat, L.ghostFill, null));
      L.group.add(regionEdgesLocal(F, p, lat, L.ghostMat));
    }
    hint.textContent = !frame.exact
      ? `${frame.moving[p] ? 'Moving' : 'Stationary'} (float preview): lattice status is available only at exact states.`
      : M.onLattice(frame.k, p) ? 'On the lattice: the piece sits in a slot.'
        : 'Dashed: nearest lattice slot. Above the plane: outside the 600-cell.';
  }

  if (refit) {
    const c = new THREE.Vector3((box.min.x + box.max.x) / 2, (Math.min(box.min.y, 0) + Math.max(box.max.y, 0)) / 2, (box.min.z + box.max.z) / 2);
    const rad = Math.max(box.getSize(new THREE.Vector3()).length() / 2, size / 2);
    const dist = rad / Math.sin(THREE.MathUtils.degToRad(L.camera.fov / 2)) * 1.08;
    const dir = new THREE.Vector3(0.62, 0.42, 1).normalize();
    L.camera.position.copy(c).addScaledVector(dir, dist);
    L.camera.near = dist / 100; L.camera.far = dist * 10;
    L.camera.updateProjectionMatrix();
    L.controls.target.copy(c);
    L.controls.update();
  }
  localKey = key;
}

// ---------------------------------------------------------------------------------------------
// picking

let needRender = true;

function addPicking(canvas, handler) {
  let down = null;
  canvas.addEventListener('pointerdown', (e) => { down = [e.clientX, e.clientY]; });
  canvas.addEventListener('pointerup', (e) => {
    if (!down) return;
    const moved = Math.hypot(e.clientX - down[0], e.clientY - down[1]);
    down = null;
    if (moved < 5) handler(e);
  });
}

function ndc(e, canvas) {
  const r = canvas.getBoundingClientRect();
  return new THREE.Vector2(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
}

function pickGlobal(e) {
  const canvas = $('gCanvas');
  const rect = canvas.getBoundingClientRect();
  // grips first: nearest marker within 14 px
  if (view.grips) {
    let best = null, bd = 14;
    const v = new THREE.Vector3();
    M.grips.forEach((g, i) => {
      v.set(G.gripPos[i * 3], G.gripPos[i * 3 + 1], G.gripPos[i * 3 + 2]).project(G.camera);
      const x = (v.x + 1) / 2 * rect.width, y = (1 - v.y) / 2 * rect.height;
      const d = Math.hypot(x - (e.clientX - rect.left), y - (e.clientY - rect.top));
      if (d < bd) { bd = d; best = g; }
    });
    if (best != null) { selectGrip(best); return; }
  }
  const ray = new THREE.Raycaster();
  ray.setFromCamera(ndc(e, canvas), G.camera);
  G.geom.computeBoundingSphere();
  const hits = ray.intersectObject(G.mesh, false);
  for (const h of hits) {
    const st = M.stickers[G.triSticker[h.faceIndex]];
    if (!st.hidden) { selectPiece(st.p); return; }
  }
}

function pickLocal(e) {
  if (!L.pickables.length) return;
  const ray = new THREE.Raycaster();
  ray.setFromCamera(ndc(e, $('lCanvas')), L.camera);
  const hits = ray.intersectObjects(L.pickables, false);
  if (hits.length) selectPiece(hits[0].object.userData.piece);
}

// ---------------------------------------------------------------------------------------------
// selection, timeline and panels

function gripName(e) {
  if (e === M.c) return `${e} (c)`;
  if (e === M.d) return `${e} (d)`;
  return String(e);
}

function fmtSigned(x, digits) {
  const s = Math.abs(x).toFixed(digits);
  return (x < 0 ? '−' : x > 0 ? '+' : '') + s;
}

function twistHTML(label) {
  return label.replace('T_d', 'T<sub>d</sub>').replace('^-1', '<sup>−1</sup>');
}

function selectGrip(e) {
  view.grip = e;
  const k = stateAt(view.t).k;
  const recs = M.certs[k].get(e);
  if (recs && recs.length && (view.piece == null || !straddleSet(k, e).has(view.piece))) view.piece = M.A.cert_piece[recs[0]];
  markDirty();
}

function selectPiece(p) {
  view.piece = p;
  markDirty();
}

function clearGrip() {
  view.grip = null;
  markDirty();
}

function markDirty() {
  dirty.flags = dirty.local = dirty.panels = true;
  needRender = true;
}

function setT(t, fromUser) {
  view.t = Math.max(0, Math.min(M.K - 1, t));
  if (fromUser) stopAnim();
  dirty.pose = dirty.flags = dirty.local = dirty.panels = true;
  needRender = true;
}

const anim = { active: false, target: 0, seg: 0, dir: 1, u: 0, hold: 0, play: false };
const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

function startAnim(target, play) {
  target = Math.max(0, Math.min(M.K - 1, target));
  if (Math.abs(target - view.t) < 1e-9) return;
  anim.active = true; anim.play = play; anim.target = target; anim.hold = 0;
  anim.dir = target > view.t ? 1 : -1;
  if (anim.dir > 0) { anim.seg = Math.floor(view.t + 1e-9); anim.u = view.t - anim.seg; }
  else { anim.seg = Math.ceil(view.t - 1e-9) - 1; anim.u = anim.seg + 1 - view.t; }
  updatePlayButton();
}
function stopAnim() { anim.active = false; anim.play = false; updatePlayButton(); }
function ease(u) { return u * u * (3 - 2 * u); }

function tickAnim(dt) {
  if (!anim.active) return;
  if (anim.hold > 0) { anim.hold -= dt; return; }
  const tw = M.header.twists[anim.seg];
  const dur = reduceMotion ? 1e-6 : 0.8 + 1.6 * tw.angle_deg / 120;
  anim.u = Math.min(1, anim.u + dt / dur);
  const p = ease(anim.u);
  const t = anim.dir > 0 ? anim.seg + p : anim.seg + 1 - p;
  if (anim.u >= 1) {
    const at = anim.dir > 0 ? anim.seg + 1 : anim.seg;
    setT(at, false);
    if (at === anim.target) { stopAnim(); return; }
    anim.seg += anim.dir;
    anim.u = 0;
    anim.hold = anim.play ? 1.1 : 0;
    return;
  }
  setT(t, false);
}

function updatePlayButton() {
  const b = $('bPlay');
  b.textContent = anim.active ? '❚❚' : '▶︎';
  b.setAttribute('aria-label', anim.active ? 'Pause' : 'Play');
}

function buildStops() {
  const box = $('stops');
  box.innerHTML = '';
  const n = M.K - 1;
  for (let k = 0; k < M.K; k++) {
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'stop state';
    b.style.left = `${(k / n) * 100}%`;
    b.textContent = `S${k}`;
    b.title = M.header.states[k].label;
    b.addEventListener('click', () => { stopAnim(); startAnim(k, false); });
    box.appendChild(b);
    if (k < n) {
      const s = document.createElement('span');
      s.className = 'stop twist';
      s.style.left = `${((k + 0.5) / n) * 100}%`;
      s.innerHTML = twistHTML(M.header.twists[k].label);
      box.appendChild(s);
    }
  }
}

function updatePanels() {
  const { k, f } = stateAt(view.t);
  const st = M.header.states[k];
  const exact = f === 0;
  if (exact) writeHash();
  // timeline readout
  for (const b of document.querySelectorAll('.stop.state')) b.setAttribute('aria-current', String(exact && b.textContent === `S${k}`));
  $('scrub').value = view.t;
  const now = $('now');
  if (exact) {
    now.innerHTML = `<span><b>S${k}</b> · ${st.label.replace('T_d', 'T<sub>d</sub>').replace('^-1', '<sup>−1</sup>')} · exact state</span>`
      + `<span>off-lattice pieces <b class="num">${st.off_lattice_pieces.toLocaleString('en-US')}</b></span>`
      + `<span>distinct poses <b class="num">${Object.keys(st.pieces_per_pose).length}</b></span>`;
  } else {
    const tw = M.header.twists[k];
    now.innerHTML = `<span><b>S${k} → S${k + 1}</b> · <span class="math">${twistHTML(tw.label)}</span> · ${tw.kind}, ${tw.angle_deg.toFixed(5)}°</span>`
      + `<span>preview angle <b class="num">${(tw.angle_deg * f).toFixed(3)}°</b> (float, uncertified)</span>`
      + `<span>moving pieces <b class="num">${tw.moved.toLocaleString('en-US')}</b></span>`;
  }
  for (const [id, badge] of [['gBadge', 'g'], ['lBadge', 'l']]) {
    const el = $(id);
    el.className = `badge ${exact ? 'exact' : 'preview'}`;
    el.textContent = exact ? `S${k} · exact state` : `S${k}→S${k + 1} · float preview, uncertified`;
  }

  // grips
  $('gripTitle').textContent = exact ? `Grips at S${k}` : `Grips (status at exact states only)`;
  $('gripCounts').innerHTML = `<span><b class="num">${st.admissible}</b> admissible</span><span><b class="num">${st.blocked}</b> blocked</span>`
    + `<span class="mono">of 600 · ${M.grips.length} in this patch</span>`;
  const chips = $('gripChips');
  chips.innerHTML = '';
  const list = [M.c, M.d, ...st.blocked_grips.filter((e) => e !== M.c && e !== M.d)];
  for (const e of list) {
    const b = document.createElement('button');
    b.type = 'button';
    const blocked = st.grip_status[e] === 'b';
    b.className = `chip ${blocked ? 'blocked' : ''}`;
    b.textContent = e === M.c ? `${e} c` : e === M.d ? `${e} d` : String(e);
    b.title = `grip ${gripName(e)}: ${blocked ? 'blocked' : 'admissible'} at S${k}`;
    b.setAttribute('aria-pressed', String(e === view.grip));
    b.addEventListener('click', () => (view.grip === e ? clearGrip() : selectGrip(e)));
    chips.appendChild(b);
  }
  const gi = $('gripInfo');
  if (view.grip == null) {
    gi.innerHTML = '<dt>Grip</dt><dd>none selected</dd>';
  } else {
    const e = view.grip;
    const blocked = st.grip_status[e] === 'b';
    const recs = M.certs[k].get(e) || [];
    let html = `<dt>Grip</dt><dd class="mono">${gripName(e)} <button type="button" id="bClearGrip" class="chip">clear</button></dd>`;
    html += `<dt>Status</dt><dd>${exact ? `${blocked ? 'blocked' : 'admissible'} at S${k} (exact)` : 'in motion; no status'}</dd>`;
    if (exact && blocked) html += `<dt>Straddling</dt><dd class="num">${recs.length.toLocaleString('en-US')} pieces</dd>`;
    const att = M.header.attempts.find((a) => a.state === k && a.grip === e);
    if (exact && att) html += `<dt>Attempt</dt><dd>twist rejected; configuration unchanged</dd>`;
    gi.innerHTML = html;
    $('bClearGrip').addEventListener('click', clearGrip);
  }

  // focus piece
  const pi = $('pieceInfo');
  const pager = $('pager');
  pager.innerHTML = '';
  if (view.piece == null) {
    pi.innerHTML = '<dt>Piece</dt><dd>none selected</dd>';
  } else {
    const p = view.piece;
    const flags = M.A.piece_flags[p];
    const caps = flags === 3 ? 'c and d' : flags === 1 ? 'c' : 'd';
    const hosts = M.pieceStickers[p].map((s) => M.stickers[s].host);
    const pose = M.header.poses[M.statePose(k, p)];
    const onLat = M.onLattice(k, p);
    let html = `<dt>Piece</dt><dd class="mono">${M.A.piece_ids[p]}</dd>`;
    html += `<dt>Caps</dt><dd>${caps}</dd><dt>Host cells</dt><dd class="mono">${hosts.join(', ')}</dd>`;
    const poseText = !exact ? `${frame.moving[p] ? 'moving' : 'stationary'} (float preview)`
      : onLat ? 'on the lattice' : `off-lattice, ${pose.residual_deg[0].toFixed(3)}° from the nearest lattice pose (float)`;
    html += `<dt>${exact ? `Pose at S${k}` : 'Pose'}</dt><dd>${poseText}</dd>`;
    if (view.grip != null) {
      const cert = exact ? certFor(k, view.grip, p) : null;
      const hs = posedRegion(p, frame.mats[p]).map((x) => dot4(M.gripUnit(view.grip), x) - ALPHA);
      html += `<dt>h range</dt><dd class="num">[${fmtSigned(Math.min(...hs), 5)}, ${fmtSigned(Math.max(...hs), 5)}] (float display)</dd>`;
      if (cert) html += `<dt>Certificate</dt><dd class="num">vertex ${cert.vb}: h = ${fmtSigned(cert.hb, 5)}; vertex ${cert.va}: h = ${fmtSigned(cert.ha, 5)} (exact signs)</dd>`;
    }
    pi.innerHTML = html;
    if (view.grip != null && exact) {
      const recs = M.certs[k].get(view.grip) || [];
      const idx = recs.findIndex((r) => M.A.cert_piece[r] === p);
      if (recs.length) {
        const prev = document.createElement('button');
        prev.type = 'button'; prev.textContent = '◀︎'; prev.setAttribute('aria-label', 'Previous straddling piece');
        const next = document.createElement('button');
        next.type = 'button'; next.textContent = '▶︎'; next.setAttribute('aria-label', 'Next straddling piece');
        const lab = document.createElement('span');
        lab.className = 'mono';
        lab.textContent = idx >= 0 ? `straddling piece ${idx + 1} of ${recs.length}` : `${recs.length} straddling pieces`;
        const go = (d) => selectPiece(M.A.cert_piece[recs[((idx < 0 ? 0 : idx + d) + recs.length) % recs.length]]);
        prev.addEventListener('click', () => go(-1));
        next.addEventListener('click', () => go(1));
        pager.append(prev, lab, next);
      }
    }
  }
}

function writeHash() {
  const { k, f } = stateAt(view.t);
  if (f !== 0) return;
  let h = `${sceneId === 's4' ? 's4-' : ''}s${k}`;
  if (view.grip != null) h += `-g${view.grip}`;
  if (view.piece != null) h += `-p${M.A.piece_ids[view.piece]}`;
  try { history.replaceState(null, '', `#${h}`); } catch (err) { /* ignore: hash is a convenience */ }
}

function readHash() {
  const selected = hashSelection();
  if (!selected || selected.scene !== sceneId) return false;
  view.t = Math.min(M.K - 1, selected.state);
  view.grip = M.grips.includes(selected.grip) ? selected.grip : null;
  view.piece = M.pieceById.get(selected.piece) ?? null;
  if (view.grip != null && view.piece == null) {
    const recs = M.certs[view.t].get(view.grip);
    if (recs) view.piece = M.A.cert_piece[recs[0]];
  }
  if (view.piece == null) view.piece = M.pieceById.get(0);   // the centre piece of cap c
  return true;
}

function wireControls() {
  $('sceneSel').addEventListener('change', (e) => {
    if (e.target.value === sceneId) return;
    stopAnim();
    // Each scene is loaded and digest-checked before its model and renderers are built.
    location.hash = `#${e.target.value === 's4' ? 's4-' : ''}s${Math.round(view.t)}-g${M.c}`;
    location.reload();
  });
  $('scrub').addEventListener('input', (e) => setT(Number(e.target.value), true));
  $('bPlay').addEventListener('click', () => {
    if (anim.active) { stopAnim(); return; }
    if (view.t >= M.K - 1 - 1e-9) setT(0, true);
    startAnim(M.K - 1, true);
  });
  $('bFwd').addEventListener('click', () => { stopAnim(); startAnim(Math.floor(view.t + 1e-9) + 1, false); });
  $('bBack').addEventListener('click', () => { stopAnim(); startAnim(Math.ceil(view.t - 1e-9) - 1, false); });
  const seg = (proj) => {
    view.proj = proj;
    $('pPersp').setAttribute('aria-pressed', String(proj === 'persp'));
    $('pStereo').setAttribute('aria-pressed', String(proj === 'stereo'));
    $('rDist').disabled = proj !== 'persp';
    dirty.pose = dirty.flags = true;
    needRender = true;
  };
  $('pPersp').addEventListener('click', () => seg('persp'));
  $('pStereo').addEventListener('click', () => seg('stereo'));
  $('eyeSel').addEventListener('change', (e) => { view.eye = e.target.value; dirty.pose = dirty.flags = true; needRender = true; });
  $('showSel').addEventListener('change', (e) => { view.show = e.target.value; dirty.flags = true; needRender = true; });
  const range = (id, out, key, home) => $(id).addEventListener('input', (e) => {
    view[key] = Number(e.target.value);
    $(out).textContent = view[key].toFixed(2);
    if (home) dirty.home = true;
    dirty.pose = dirty.flags = true;
    needRender = true;
  });
  range('rDist', 'oDist', 'dist', false);
  range('rCell', 'oCell', 'cell', true);
  range('rSticker', 'oSticker', 'sticker', true);
  const check = (id, key, what) => $(id).addEventListener('change', (e) => {
    view[key] = e.target.checked;
    for (const w of what) dirty[w] = true;
    needRender = true;
  });
  check('cGrips', 'grips', ['flags']);
  check('cCage', 'cage', ['pose']);
  check('cGhost', 'ghost', ['pose', 'local']);
  check('cDim', 'dim', ['flags']);
  check('cCull', 'cull', ['flags']);
  window.addEventListener('keydown', (e) => {
    const tag = (e.target && e.target.tagName) || '';
    if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') return;
    if (tag === 'BUTTON' && e.key === ' ') return;
    if (e.key === ' ') { e.preventDefault(); $('bPlay').click(); }
    if (e.key === 'ArrowRight') $('bFwd').click();
    if (e.key === 'ArrowLeft') $('bBack').click();
  });
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  if (mq.addEventListener) mq.addEventListener('change', applyThemeColours);
  new MutationObserver(applyThemeColours).observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
}

function resize(R, stage) {
  const w = stage.clientWidth, h = stage.clientHeight;
  if (!w || !h) return;
  R.renderer.setSize(w, h, false);
  R.camera.aspect = w / h;
  R.camera.updateProjectionMatrix();
  needRender = true;
}

function placeTags(R, overlay, tags) {
  overlay.textContent = '';
  const rect = overlay.getBoundingClientRect();
  const v = new THREE.Vector3();
  for (const t of tags) {
    v.set(t.at[0], t.at[1], t.at[2]).project(R.camera);
    if (v.z > 1 || Math.abs(v.x) > 1.05 || Math.abs(v.y) > 1.05) continue;
    const el = document.createElement('span');
    el.className = `tag ${t.cls || ''}`;
    el.textContent = t.text;
    overlay.appendChild(el);
    // keep the label inside the view: the tag is centred on its point unless that would clip it
    const half = el.offsetWidth / 2;
    const x = ((v.x + 1) / 2) * rect.width;
    el.style.left = `${Math.min(Math.max(x, half + 4), rect.width - half - 4)}px`;
    el.style.top = `${((1 - v.y) / 2) * rect.height}px`;
  }
}

function globalTags() {
  const tags = [];
  if (view.grips) {
    M.grips.forEach((e, i) => {
      if (e === M.c || e === M.d || e === view.grip) {
        const name = e === M.c ? 'c' : e === M.d ? 'd' : String(e);
        tags.push({ at: [G.gripPos[i * 3], G.gripPos[i * 3 + 1], G.gripPos[i * 3 + 2]], cls: e === view.grip ? 'sel' : '', text: e === view.grip && (e === M.c || e === M.d) ? `${name} = ${e}` : name });
      }
    });
  }
  return tags;
}

function render() {
  if (dirty.home) { refreshHome(); dirty.home = false; dirty.pose = true; }
  if (dirty.pose) {
    frame = computeFrame(view.t);
    updateGlobalGeometry();
    dirty.pose = false;
    dirty.flags = dirty.local = true;
  }
  if (dirty.flags) { updateGlobalFlags(); updateFocusLines(); dirty.flags = false; }
  if (dirty.local) { buildLocal(); dirty.local = false; }
  if (dirty.panels) { updatePanels(); dirty.panels = false; }
  G.renderer.render(G.scene, G.camera);
  L.renderer.render(L.scene, L.camera);
  placeTags(G, $('gOverlay'), globalTags());
  placeTags(L, $('lOverlay'), L.tags || []);
  $('gWhat').textContent = view.proj === 'persp'
    ? `4D perspective, eye at distance ${view.dist.toFixed(2)} along ${view.eye === 'mid' ? 'the c–d midpoint' : `pole ${view.eye}`}`
    : `stereographic from the antipode of ${view.eye === 'mid' ? 'the c–d midpoint' : `pole ${view.eye}`}`;
}

function loop(now) {
  const dt = loop.last ? Math.min(0.1, (now - loop.last) / 1000) : 0;
  loop.last = now;
  tickAnim(dt);
  if (needRender) {
    needRender = false;
    render();
    app.frames = (app.frames || 0) + 1;
  }
  requestAnimationFrame(loop);
}

async function main() {
  try {
    M = await loadScene();
  } catch (err) {
    reportError(err.message);
    $('gBadge').textContent = 'no data';
    $('lBadge').textContent = 'no data';
    return;
  }
  if (!readHash()) {
    view.t = 2;
    view.grip = M.c;
    const recs = M.certs[2].get(M.c);
    view.piece = recs ? M.A.cert_piece[recs[0]] : null;
  }
  $('sceneSel').value = sceneId;
  $('sceneMeta').textContent = `600-cell-Full geometry · cut depth 121/125 · caps of grips c = ${M.c} and d = ${M.d} · ${M.P.toLocaleString('en-US')} pieces`;
  $('sceneSource').textContent = `Source: ${M.header.source} through export_scene.py.`;
  $('scrub').max = M.K - 1;
  setupGlobal();
  setupLocal();
  applyThemeColours();
  buildStops();
  wireControls();
  const ro = new ResizeObserver(() => { resize(G, $('gStage')); resize(L, $('lStage')); });
  ro.observe($('gStage'));
  ro.observe($('lStage'));
  resize(G, $('gStage'));
  resize(L, $('lStage'));
  G.controls.target.set(0, 0, 0);
  G.controls.update();
  dirty.home = dirty.pose = dirty.flags = dirty.local = dirty.panels = true;
  render();
  app.ready = true;
  app.scene = sceneId;
  app.model = { pieces: M.P, stickers: M.S, states: M.K, triangles: M.nTris };
  app.goto = (t) => { stopAnim(); setT(t, true); render(); };
  app.selectGrip = (e) => { selectGrip(e); render(); };
  app.selectPiece = (id) => { selectPiece(M.pieceById.get(id)); render(); };
  app.clearGrip = () => { clearGrip(); render(); };
  app.set = (opts) => { Object.assign(view, opts); dirty.home = dirty.pose = dirty.flags = dirty.local = dirty.panels = true; render(); };
  app.view = view;
  // The actual 4D inputs and projected buffer of the Global certificate markers, for checks.
  app.globalCertificate = () => G.certPts.visible ? {
    world: G.certWorld.map((x) => x.slice()), projected: Array.from(G.certPos),
  } : null;
  app.localCertificate = () => L.certPts.visible ? Array.from(L.certPos) : null;
  app.gripScreen = (e) => {
    const i = M.grips.indexOf(e);
    const v = new THREE.Vector3(G.gripPos[i * 3], G.gripPos[i * 3 + 1], G.gripPos[i * 3 + 2]).project(G.camera);
    const r = $('gCanvas').getBoundingClientRect();
    return { x: r.left + (v.x + 1) / 2 * r.width, y: r.top + (1 - v.y) / 2 * r.height };
  };
  // for the headless check: share of pixels that differ from the background, per view
  app.coverage = () => {
    const out = {};
    for (const [name, R] of [['global', G], ['local', L]]) {
      R.renderer.render(R.scene, R.camera);
      const gl = R.renderer.getContext();
      const w = gl.drawingBufferWidth, h = gl.drawingBufferHeight;
      const px = new Uint8Array(w * h * 4);
      gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, px);
      const bg = new THREE.Color();
      R.renderer.getClearColor(bg);
      const b = [bg.r * 255, bg.g * 255, bg.b * 255];
      let n = 0;
      for (let i = 0; i < px.length; i += 4) {
        if (Math.abs(px[i] - b[0]) + Math.abs(px[i + 1] - b[1]) + Math.abs(px[i + 2] - b[2]) > 24) n++;
      }
      out[name] = n / (w * h);
    }
    return out;
  };
  // the end of every animated family R(theta) * pose(before) against the exported pose(after)
  app.familyCheck = () => {
    let worst = 0;
    const seen = new Set();
    M.header.twists.forEach((tw, k) => {
      const R = rotFamily(tw.u, tw.v, tw.theta);
      for (let i = 0; i < M.P; i++) {
        const a = M.statePose(k, i), b = M.statePose(k + 1, i);
        if (a === b || seen.has(`${k}:${a}:${b}`)) continue;
        seen.add(`${k}:${a}:${b}`);
        const X = matMul(R, M.poses[a]), Y = M.poses[b];
        for (let j = 0; j < 16; j++) worst = Math.max(worst, Math.abs(X[j] - Y[j]));
      }
    });
    return worst;
  };
  requestAnimationFrame(loop);
}

main();
