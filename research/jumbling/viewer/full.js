// Full W-J drawing through magic600-jumbling-render/1. No state edits or legality decisions.
// Geometry, poses, lattice flags and moving ids reach GLSL as data textures. One triangle
// draw instances the unchanged 30,480-vertex mesh 600 times. All look values are provisional.
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

THREE.ColorManagement.enabled = false;
const $ = (id) => document.getElementById(id);
const COUNTS = { pieces: 177120, slots: 259800, cells: 600, base_stickers: 433, base_vertices: 30480 };
const MENUS = ['S4', 'I_a', 'I_b'];
const DTYPES = { '<f4': Float32Array, '<i4': Int32Array, '<u4': Uint32Array };
const B64 = new URLSearchParams(location.search).has('b64');
const app = { ready: false, errors: [], model: null, playing: false, view: { menu: 'S4', state: 'end', t: 0 } };
window.fullViewer = app;
let catalog, geometry, overlay, state, renderer, scene, camera, controls, mesh, material, width = 1024;
let stateTextures = [], staticTextures = [], markerMeshes = [], selectedCertificate = null;
let serial = 0, dirty = true, lastTime = 0;

function requireValue(condition, message) { if (!condition) throw new Error(message); }
const equal = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const sameIdentity = (a, b) => a && b && Object.keys(a).length === Object.keys(b).length
  && Object.keys(a).every((k) => a[k] === b[k]);
const hexDigest = (s) => typeof s === 'string' && /^[a-f0-9]{64}$/.test(s);
const finite4 = (v) => Array.isArray(v) && v.length === 4 && v.every(Number.isFinite);
const validId = (p) => Number.isInteger(p) && p >= 0 && p < COUNTS.pieces;
const dot = (a, b) => a.reduce((s, x, i) => s + x * b[i], 0);

function checkDimension(h, file) {
  requireValue(h.dimension === 4, `${file}: dimension ${h.dimension} is unsupported; this drawing implements dimension 4`);
}
async function jsonFile(path) {
  const response = await fetch(path);
  requireValue(response.ok, `${path}: HTTP ${response.status}. Generate the requested data with export_full.py`);
  return response.json();
}
async function bytesFile(path, binary = true) {
  async function b64() {
    const response = await fetch(`${path}.b64`);
    requireValue(response.ok, `${path}.b64: HTTP ${response.status}`);
    const text = (await response.text()).replace(/\s/g, '');
    return Uint8Array.from(atob(text), (c) => c.charCodeAt(0)).buffer;
  }
  if (binary && B64) return b64();
  let response;
  try { response = await fetch(path); } catch (e) {
    if (binary) return b64();
    throw e;
  }
  if (!response.ok && binary) return b64();
  requireValue(response.ok, `${path}: HTTP ${response.status}`);
  // An integrity failure is never retried through another encoding.
  return response.arrayBuffer();
}
async function checkedBytes(path, desc, binary = true) {
  requireValue(crypto.subtle && hexDigest(desc.sha256), `${path}: Web Crypto or a valid SHA-256 is missing`);
  const bytes = await bytesFile(path, binary);
  const hash = await crypto.subtle.digest('SHA-256', bytes);
  const sha = Array.from(new Uint8Array(hash), (b) => b.toString(16).padStart(2, '0')).join('');
  requireValue(sha === desc.sha256, `${path}: SHA-256 mismatch`);
  if (desc.bytes != null) requireValue(bytes.byteLength === desc.bytes, `${path}: byte count mismatch`);
  return bytes;
}
async function arrayFile(path, desc, dtype, shape) {
  requireValue(desc && desc.dtype === dtype && equal(desc.shape, shape), `${path}: type or shape mismatch`);
  const bytes = await checkedBytes(path, desc);
  requireValue(bytes.byteLength === shape.reduce((a, b) => a * b, 1) * 4, `${path}: array byte count mismatch`);
  const a = new DTYPES[dtype](bytes);
  if (dtype === '<f4') requireValue(a.every(Number.isFinite), `${path}: non-finite float`);
  return a;
}
function exactFiles(files, names, where) {
  requireValue(files && equal(Object.keys(files).sort(), names.slice().sort()), `${where}: unexpected array files`);
}
function sameArrayFiles(a, b) {
  return a && b && equal(Object.keys(a).sort(), Object.keys(b).sort()) && Object.keys(a).every((k) =>
    a[k].sha256 === b[k].sha256 && a[k].dtype === b[k].dtype && equal(a[k].shape, b[k].shape));
}

async function loadGeometry() {
  const h = await jsonFile('full/geometry.json');
  checkDimension(h, 'geometry.json');
  requireValue(h.format === 'magic600-jumbling-geometry/1' && Object.entries(COUNTS).every(([k, n]) => h[k] === n),
    'geometry.json: expected the full 600-cell geometry');
  const specs = {
    'mesh_vertices.f32': ['<f4', [30480, 4]], 'mesh_sticker.u32': ['<u4', [30480]],
    'mesh_centers.f32': ['<f4', [433, 4]], 'cell_frames.f32': ['<f4', [600, 4, 4]],
    'slot_piece.u32': ['<u4', [259800]],
  };
  exactFiles(h.files, [...Object.keys(specs), 'mesh.json'], 'geometry.json');
  const entries = await Promise.all(Object.entries(specs).map(async ([name, [type, shape]]) =>
    [name, await arrayFile(`full/${name}`, h.files[name], type, shape)]));
  const A = Object.fromEntries(entries);
  const m = JSON.parse(new TextDecoder().decode(await checkedBytes('full/mesh.json', h.files['mesh.json'], false)));
  requireValue(Object.entries(COUNTS).filter(([k]) => k !== 'pieces').every(([k, n]) => m[k] === n), 'mesh.json: counts mismatch');
  requireValue(finite4(m.normal) && Number.isFinite(m.normal_length) && m.normal_length > 0, 'mesh.json: invalid normal');
  requireValue(Array.isArray(m.offsets) && m.offsets.length === 434 && m.offsets[0] === 0 && m.offsets[433] === 30480,
    'mesh.json: invalid offsets');
  for (let local = 0; local < 433; local++) {
    const a = m.offsets[local], b = m.offsets[local + 1];
    requireValue(Number.isInteger(a) && Number.isInteger(b) && b > a && (b - a) % 3 === 0, 'mesh.json: invalid triangle list');
    for (let vi = a; vi < b; vi++) requireValue(A['mesh_sticker.u32'][vi] === local, 'mesh_sticker.u32: offset mismatch');
  }
  const pieces = new Set(A['slot_piece.u32']);
  requireValue(pieces.size === 177120 && [...pieces].every(validId), 'slot_piece.u32: must cover all 177120 pieces');
  return { header: h, mesh: m, A };
}

async function loadOverlay(menu) {
  const dir = `full/${menu}/`;
  const h = await jsonFile(`${dir}overlay.json`);
  checkDimension(h, 'overlay.json');
  requireValue(h.format === 'magic600-jumbling-overlay/1' && h.menu === menu
    && h.menu_identity === catalog.menus[menu].identity && sameIdentity(h.model_identity, geometry.header.model_identity),
  'overlay.json: model or menu identity mismatch');
  requireValue(typeof h.grip_status === 'string' && /^[ab]{600}$/.test(h.grip_status), 'overlay.json: incomplete grip status');
  const blocked = Array.from(h.grip_status, (s, i) => s === 'b' ? i : -1).filter((i) => i >= 0);
  requireValue(Array.isArray(h.certificates) && equal(h.certificates.map((c) => c.grip), blocked), 'overlay.json: incomplete certificates');
  for (const c of h.certificates) requireValue(validId(c.piece) && finite4(c.point_below_float) && finite4(c.point_above_float),
    'overlay.json: invalid certificate');
  const sw = h.sweep;
  requireValue(sw && Number.isInteger(sw.grip) && sw.grip >= 0 && sw.grip < 600 && finite4(sw.plane_u) && finite4(sw.plane_v)
    && Number.isFinite(sw.angle) && sw.angle > 0 && sw.angle < Math.PI
    && Number.isInteger(sw.from_revision) && sw.to_revision === sw.from_revision + 1
    && Math.abs(dot(sw.plane_u, sw.plane_u) - 1) < 1e-8 && Math.abs(dot(sw.plane_v, sw.plane_v) - 1) < 1e-8
    && Math.abs(dot(sw.plane_u, sw.plane_v)) < 1e-8, 'overlay.json: invalid rotation family');
  requireValue(Array.isArray(sw.samples) && sw.samples.length === 16 && sw.samples.every((s) =>
    validId(s.piece) && ['start', 'mid', 'end'].every((k) => finite4(s[k]))), 'overlay.json: invalid sweep samples');
  requireValue(Number.isInteger(sw.moving.count) && sw.moving.count > 0 && sw.moving.count <= 177120
    && sw.moving.file === 'moving.i32', 'overlay.json: invalid moving count');
  exactFiles(h.files, ['moving.i32', 'sweep_home.f32', 'sweep_pieces.i32'], 'overlay.json');
  const [moving, home, pieces] = await Promise.all([
    arrayFile(`${dir}moving.i32`, h.files['moving.i32'], '<i4', [sw.moving.count]),
    arrayFile(`${dir}sweep_home.f32`, h.files['sweep_home.f32'], '<f4', [16, 4]),
    arrayFile(`${dir}sweep_pieces.i32`, h.files['sweep_pieces.i32'], '<i4', [16]),
  ]);
  requireValue(h.files['moving.i32'].sha256 === sw.moving.sha256
    && moving.every((p, i) => validId(p) && (i === 0 || moving[i - 1] < p)), 'overlay.json: moving ids or SHA-256 mismatch');
  const inside = new Set(moving);
  requireValue(pieces.every((p, i) => p === sw.samples[i].piece && inside.has(p)), 'overlay.json: samples outside moving set');
  return { header: h, moving, home, pieces };
}

async function loadState(menu, stage, o) {
  const entry = stage === 'solved' ? catalog.solved : catalog.menus[menu].states[stage];
  requireValue(entry, `${menu}/${stage}: state was not exported`);
  const path = stage === 'solved' ? 'full/solved/' : `full/${menu}/${stage}/`;
  const h = await jsonFile(`${path}header.json`);
  checkDimension(h, 'header.json');
  requireValue(h.format === 'magic600-jumbling-render/1' && h.pieces === 177120
    && Number.isInteger(h.poses) && h.poses >= 1 && h.poses <= 177121
    && Number.isInteger(h.revision) && h.revision >= 0 && hexDigest(h.digest), 'header.json: invalid state counts or digest');
  requireValue(sameIdentity(h.model_identity, geometry.header.model_identity) && h.contract_revision === o.header.contract_revision
    && (stage === 'solved' || h.menu_identity === o.header.menu_identity), 'header.json: state identity or contract mismatch');
  requireValue(h.digest === entry.digest && h.revision === entry.revision && h.fixture.stage === (stage === 'solved' ? 'start' : stage),
    'header.json: catalog revision mismatch');
  if (stage === 'end') requireValue(h.digest === o.header.end.digest && h.revision === o.header.end.records, 'header.json: end survey mismatch');
  if (stage === 'sweep-before') requireValue(h.digest === o.header.sweep.before.digest && h.revision === o.header.sweep.from_revision
    && sameArrayFiles(h.files, o.header.sweep.before.arrays), 'header.json: sweep revision mismatch');
  exactFiles(h.files, ['pose_index.i32', 'poses.f32', 'pose_lattice.i32'], 'header.json');
  const [index, poses, lattice] = await Promise.all([
    arrayFile(`${path}pose_index.i32`, h.files['pose_index.i32'], '<i4', [177120]),
    arrayFile(`${path}poses.f32`, h.files['poses.f32'], '<f4', [h.poses, 4, 4]),
    arrayFile(`${path}pose_lattice.i32`, h.files['pose_lattice.i32'], '<i4', [h.poses]),
  ]);
  requireValue(index.every((p) => p >= 0 && p < h.poses) && lattice.every((k) => k >= -1 && k < 7200), 'header.json: invalid pose index or lattice id');
  requireValue(poses.slice(0, 16).every((x, i) => Math.abs(x - (i % 5 === 0 ? 1 : 0)) < 1e-7) && lattice[0] === 0,
    'header.json: pose zero must be the identity');
  let off = 0;
  for (const p of index) if (lattice[p] < 0) off++;
  requireValue(off === entry.off_lattice && (stage !== 'end' || off === o.header.end.off_lattice), 'header.json: lattice count differs from fixture');
  return { header: h, index, poses, lattice, off };
}

// Texture rows all have the same width. Scalar integer arrays use R32I/R32UI; 4-vectors
// use RGBA32F. A facet consumes four column texels, a pose four row texels.
function dataTexture(values, channels, unsigned = false) {
  const height = Math.max(1, Math.ceil(values.length / channels / width));
  requireValue(height <= renderer.capabilities.maxTextureSize, 'This GPU cannot hold the full contract arrays');
  const padded = new values.constructor(width * height * channels);
  padded.set(values);
  const integer = values instanceof Int32Array || values instanceof Uint32Array;
  const texture = new THREE.DataTexture(padded, width, height,
    integer ? THREE.RedIntegerFormat : THREE.RGBAFormat,
    integer ? (unsigned ? THREE.UnsignedIntType : THREE.IntType) : THREE.FloatType);
  texture.internalFormat = integer ? (unsigned ? 'R32UI' : 'R32I') : 'RGBA32F';
  texture.minFilter = texture.magFilter = THREE.NearestFilter;
  texture.generateMipmaps = false;
  texture.unpackAlignment = 1;
  texture.needsUpdate = true;
  return texture;
}

// The mesh and the 16-point float readback probe use this SAME vertex program. Probe home
// centroids are separately hashed, engine-derived inputs; they are not the fixture's posed
// start samples. Thus the check exercises GPU pose indexing, matrix order and sweep composition.
const VERTEX = `
precision highp float;
precision highp int;
precision highp sampler2D;
precision highp isampler2D;
precision highp usampler2D;
uniform sampler2D uVertices, uCenters, uFrames, uPoses, uProbeHome;
uniform usampler2D uStickers, uSlotPiece;
uniform isampler2D uIndex, uLattice, uMoving, uProbePieces;
uniform int uWidth, uFilter, uIsolate, uFocus, uProjection;
uniform float uRadius, uFacetShrink, uStickerShrink, uAngle;
uniform vec4 uNormal, uPlaneU, uPlaneV, uPole, uBasis0, uBasis1, uBasis2;
uniform mat4 uQ, viewMatrix, projectionMatrix;
uniform bool uProbe;
out vec3 vProjected;
out vec4 vWorld, vClip;
flat out int vClass, vOff, vFocus;
ivec2 address(int i) { return ivec2(i % uWidth, i / uWidth); }
vec4 vectorAt(sampler2D t, int i) { return texelFetch(t, address(i), 0); }
int intAt(isampler2D t, int i) { return texelFetch(t, address(i), 0).r; }
uint uintAt(usampler2D t, int i) { return texelFetch(t, address(i), 0).r; }
vec4 posePoint(int pose, vec4 x) {
  int i = pose * 4;
  return vec4(dot(vectorAt(uPoses, i), x), dot(vectorAt(uPoses, i+1), x),
              dot(vectorAt(uPoses, i+2), x), dot(vectorAt(uPoses, i+3), x));
}
vec4 sweepPoint(vec4 x) {
  float a = dot(x, uPlaneU), b = dot(x, uPlaneV);
  float c = cos(uAngle)-1.0, s = sin(uAngle);
  return x + (c*a-s*b)*uPlaneU + (s*a+c*b)*uPlaneV;
}
vec3 project4(vec4 x) {
  if (uProjection == 0) return 1.18 * x.xyz / (1.18-x.w);
  return vec3(dot(x,uBasis0), dot(x,uBasis1), dot(x,uBasis2)) / (1.0-dot(x,uPole));
}
void main() {
  int piece;
  vec4 home;
  vClass = gl_InstanceID;
  if (uProbe) {
    piece = intAt(uProbePieces, gl_VertexID);
    home = vectorAt(uProbeHome, gl_VertexID) / uRadius;
  } else {
    int local = int(uintAt(uStickers, gl_VertexID));
    int slot = gl_InstanceID * 433 + local;
    piece = int(uintAt(uSlotPiece, slot));
    vec4 center = vectorAt(uCenters, local);
    vec4 vertex = vectorAt(uVertices, gl_VertexID);
    vec4 shrunk = (uNormal + uFacetShrink*(center-uNormal)
                 + uFacetShrink*uStickerShrink*(vertex-center)) / uRadius;
    int f = gl_InstanceID * 4;
    home = mat4(vectorAt(uFrames,f), vectorAt(uFrames,f+1),
                vectorAt(uFrames,f+2), vectorAt(uFrames,f+3)) * shrunk;
  }
  int pose = intAt(uIndex, piece);
  vOff = intAt(uLattice, pose) < 0 ? 1 : 0;
  vFocus = piece == uFocus ? 1 : 0;
  bool hidden = (uFilter == 1 && vOff == 0) || (uFilter == 2 && vOff == 1)
             || (uFilter == 3 && piece != uIsolate);
  vec4 world = posePoint(pose, home);
  if (intAt(uMoving, piece) != 0 && uAngle != 0.0) world = sweepPoint(world);
  vWorld = world;
  vProjected = project4(uQ * world);
  vClip = projectionMatrix * viewMatrix * vec4(vProjected,1.0);
  gl_Position = uProbe ? vec4(2.0*(float(gl_VertexID)+0.5)/16.0-1.0,0.0,0.0,1.0) : vClip;
  if (hidden && !uProbe) gl_Position = vec4(2.0,2.0,2.0,1.0);
  gl_PointSize = 1.0;
}`;
const FRAGMENT = `
precision highp float;
precision highp int;
in vec3 vProjected;
in vec4 vWorld, vClip;
flat in int vClass, vOff, vFocus;
uniform bool uTint, uProbe;
uniform int uFocus, uProbeOutput;
uniform float uRadius;
uniform vec3 uTintColor, uHighlight;
out vec4 outColor;
vec3 hsv(float h, float s, float v) {
  vec3 p = abs(fract(vec3(h)+vec3(0.0,2.0/3.0,1.0/3.0))*6.0-3.0);
  return v * mix(vec3(1.0), clamp(p-1.0,0.0,1.0), s);
}
void main() {
  if (uProbe) {
    if (uProbeOutput == 0) outColor = vWorld * uRadius;
    else if (uProbeOutput == 1) outColor = vec4(vProjected,1.0);
    else outColor = vClip / vClip.w;
    return;
  }
  vec3 color = hsv(fract(float(vClass)*0.61803398875),0.61,0.90);
  vec3 n = normalize(cross(dFdx(vProjected), dFdy(vProjected)));
  color *= 0.5 + 0.5*abs(dot(n,normalize(vec3(0.3,0.5,1.0))));
  if (uTint && vOff != 0) color = mix(color,uTintColor,0.4);
  if (uFocus >= 0) color = vFocus != 0 ? mix(color,uHighlight,0.8) : color*0.22;
  outColor = vec4(color,1.0);
}`;

function cssColor(name) { return new THREE.Color(getComputedStyle(document.documentElement).getPropertyValue(name).trim()); }
function makeRenderer() {
  const canvas = $('canvas');
  const context = canvas.getContext('webgl2', { antialias: true, preserveDrawingBuffer: true });
  requireValue(context, 'WebGL2 is required for the full-state drawing');
  renderer = new THREE.WebGLRenderer({ canvas, context, antialias: true, preserveDrawingBuffer: true });
  renderer.setPixelRatio(1);
  renderer.debug.onShaderError = (gl, program, vs, fs) => {
    throw new Error(`Shader error: ${gl.getProgramInfoLog(program)} ${gl.getShaderInfoLog(vs)} ${gl.getShaderInfoLog(fs)}`);
  };
  canvas.addEventListener('webglcontextlost', (e) => { e.preventDefault(); fail('The WebGL context was lost; reload the page'); });
  width = Math.min(1024, renderer.capabilities.maxTextureSize);
  scene = new THREE.Scene();
  // SPEC section 3 step 7: zoom=1.15, eye z=5, near=.05, far=100.
  camera = new THREE.PerspectiveCamera(2 * Math.atan(1 / 1.15) * 180 / Math.PI, 1, 0.05, 100);
  camera.position.set(0, 0, 5);
  controls = new OrbitControls(camera, canvas);
  controls.enableDamping = false;
  controls.minDistance = 0.05;
  controls.maxDistance = 50;
  controls.addEventListener('change', () => { dirty = true; });
  const uniforms = {};
  const names = { uVertices: 'mesh_vertices.f32', uStickers: 'mesh_sticker.u32',
    uCenters: 'mesh_centers.f32', uFrames: 'cell_frames.f32', uSlotPiece: 'slot_piece.u32' };
  for (const [uniform, name] of Object.entries(names)) {
    const a = geometry.A[name];
    const texture = dataTexture(a, name.endsWith('.f32') ? 4 : 1, name.endsWith('.u32'));
    staticTextures.push(texture);
    uniforms[uniform] = { value: texture };
  }
  Object.assign(uniforms, {
    uWidth: { value: width }, uRadius: { value: geometry.mesh.normal_length },
    uNormal: { value: new THREE.Vector4(...geometry.mesh.normal) },
    uFacetShrink: { value: 0.76 }, uStickerShrink: { value: 0.82 }, uAngle: { value: 0 },
    uPlaneU: { value: new THREE.Vector4() }, uPlaneV: { value: new THREE.Vector4() },
    uQ: { value: new THREE.Matrix4() }, uPole: { value: new THREE.Vector4(0, 0, 0, 1) },
    uBasis0: { value: new THREE.Vector4(1, 0, 0, 0) }, uBasis1: { value: new THREE.Vector4(0, 1, 0, 0) },
    uBasis2: { value: new THREE.Vector4(0, 0, 1, 0) }, uProjection: { value: 0 },
    uFilter: { value: 0 }, uIsolate: { value: 0 }, uFocus: { value: -1 },
    uTint: { value: false }, uTintColor: { value: cssColor('--tint') }, uHighlight: { value: cssColor('--highlight') },
    uProbe: { value: false }, uProbeOutput: { value: 0 },
    uIndex: { value: null }, uPoses: { value: null }, uLattice: { value: null }, uMoving: { value: null },
    uProbeHome: { value: null }, uProbePieces: { value: null },
  });
  material = new THREE.RawShaderMaterial({ glslVersion: THREE.GLSL3, vertexShader: VERTEX, fragmentShader: FRAGMENT,
    uniforms, side: THREE.DoubleSide, depthTest: true, depthWrite: true, blending: THREE.NoBlending });
  const g = new THREE.InstancedBufferGeometry();
  // Three uses this immutable base attribute only to obtain the draw's vertex count.
  // GLSL fetches the vertices from uVertices; there is no expanded per-facet vertex buffer.
  g.setAttribute('position', new THREE.BufferAttribute(geometry.A['mesh_vertices.f32'], 4));
  g.instanceCount = 600;
  mesh = new THREE.Mesh(g, material);
  mesh.frustumCulled = false;
  mesh.visible = false;
  scene.add(mesh);
  new ResizeObserver(resize).observe($('stage'));
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => { theme(); dirty = true; });
  theme();
  resize();
}
function theme() {
  if (!renderer) return;
  renderer.setClearColor(cssColor('--view-bg'), 1);
  material.uniforms.uTintColor.value.copy(cssColor('--tint'));
  material.uniforms.uHighlight.value.copy(cssColor('--highlight'));
  markerMeshes.forEach((point, i) => point.material.color.copy(cssColor(i === 0 ? '--below' : '--above')));
}
function resize() {
  if (!renderer) return;
  const rect = $('stage').getBoundingClientRect();
  renderer.setSize(Math.max(1, rect.width), Math.max(1, rect.height), false);
  camera.aspect = rect.width / Math.max(1, rect.height);
  camera.updateProjectionMatrix();
  dirty = true;
}

function bindState(s, o) {
  const moving = new Int32Array(COUNTS.pieces);
  o.moving.forEach((p) => { moving[p] = 1; });
  const arrays = { uIndex: [s.index, 1], uPoses: [s.poses, 4], uLattice: [s.lattice, 1],
    uMoving: [moving, 1], uProbeHome: [o.home, 4], uProbePieces: [o.pieces, 1] };
  const next = [];
  try {
    for (const [key, [a, channels]] of Object.entries(arrays)) next.push([key, dataTexture(a, channels)]);
  } catch (e) { next.forEach(([, t]) => t.dispose()); throw e; }
  // Adopt the complete revision synchronously after ALL bytes and identities passed.
  next.forEach(([key, texture]) => { material.uniforms[key].value = texture; });
  stateTextures.forEach((t) => t.dispose());
  stateTextures = next.map(([, t]) => t);
  material.uniforms.uPlaneU.value.set(...o.header.sweep.plane_u);
  material.uniforms.uPlaneV.value.set(...o.header.sweep.plane_v);
}

function cameraQ() {
  const q = Array.from({ length: 16 }, (_, i) => i % 5 === 0 ? 1 : 0);
  // This is SPEC's column-major rotate(a,b,t), including its multiplication order.
  for (const [a, b, degrees] of [[0, 3, Number($('rotateX').value)], [1, 3, Number($('rotateY').value)]]) {
    const c = Math.cos(degrees * Math.PI / 180), s = Math.sin(degrees * Math.PI / 180);
    for (let j = 0; j < 4; j++) {
      const x = q[a * 4 + j], y = q[b * 4 + j];
      q[a * 4 + j] = c * x - s * y;
      q[b * 4 + j] = s * x + c * y;
    }
  }
  return q;
}
function poleParameters() {
  let pole = $('pole').value === 'negative-w' ? [0, 0, 0, -1] : [0, 0, 0, 1];
  if ($('pole').value === 'facet-0') {
    const length = Math.sqrt(dot(geometry.mesh.normal, geometry.mesh.normal));
    pole = geometry.mesh.normal.map((x) => x / length);
  }
  const basis = [];
  for (let i = 0; i < 4 && basis.length < 3; i++) {
    let v = [0, 0, 0, 0]; v[i] = 1;
    let d = dot(v, pole); v = v.map((x, k) => x - d * pole[k]);
    for (const b of basis) { d = dot(v, b); v = v.map((x, k) => x - d * b[k]); }
    const length = Math.sqrt(dot(v, v));
    if (length > 1e-8) basis.push(v.map((x) => x / length));
  }
  return { pole, basis };
}
function matColumn(m, x) {
  return [0, 1, 2, 3].map((r) => x.reduce((s, c, k) => s + m[k * 4 + r] * c, 0));
}
function projectionParameters() {
  const p = poleParameters();
  return { kind: $('projection').value, Q: cameraQ(), ...p, d4: 1.18, radius: geometry.mesh.normal_length,
    view: camera.matrixWorldInverse.elements.slice(), camera3: camera.projectionMatrix.elements.slice() };
}
function projectPoint(assetPoint) {
  // Certificates are already posed points in the asset frame. Do NOT pose or shrink again.
  const world = matColumn(cameraQ(), assetPoint.map((x) => x / geometry.mesh.normal_length));
  if ($('projection').value === 'perspective') return world.slice(0, 3).map((x) => 1.18 * x / (1.18 - world[3]));
  const { pole, basis } = poleParameters();
  return basis.map((b) => dot(world, b) / (1 - dot(world, pole)));
}
function syncView() {
  if (!material) return;
  const u = material.uniforms;
  u.uFacetShrink.value = Number($('facetShrink').value);
  u.uStickerShrink.value = Number($('stickerShrink').value);
  u.uQ.value.fromArray(cameraQ());
  const { pole, basis } = poleParameters();
  u.uPole.value.set(...pole);
  basis.forEach((b, i) => u[`uBasis${i}`].value.set(...b));
  u.uProjection.value = $('projection').value === 'perspective' ? 0 : 1;
  u.uTint.value = $('tint').checked;
  u.uFilter.value = ['all', 'off', 'on', 'isolate'].indexOf($('filter').value);
  const id = Number($('piece').value);
  u.uIsolate.value = validId(id) ? id : -1;
  u.uFocus.value = selectedCertificate ? selectedCertificate.piece : ($('filter').value === 'isolate' ? u.uIsolate.value : -1);
  const progress = app.view.t <= 1 ? app.view.t : 2 - app.view.t;
  u.uAngle.value = app.view.state === 'sweep-before' && overlay
    ? overlay.header.sweep.angle * progress * progress * (3 - 2 * progress) : 0;
  $('facetValue').textContent = u.uFacetShrink.value.toFixed(2);
  $('stickerValue').textContent = u.uStickerShrink.value.toFixed(2);
  $('rotateXValue').textContent = `${$('rotateX').value}°`;
  $('rotateYValue').textContent = `${$('rotateY').value}°`;
  updateMarkers();
  dirty = true;
}

function clearMarkers() {
  markerMeshes.forEach((point) => { scene.remove(point); point.geometry.dispose(); point.material.dispose(); });
  markerMeshes = [];
  $('markers').replaceChildren();
}
function updateMarkers() {
  if (!selectedCertificate || !camera) return;
  const rect = $('stage').getBoundingClientRect();
  camera.updateMatrixWorld();
  markerMeshes.forEach((point, i) => {
    const side = i === 0 ? 'below' : 'above';
    const projected = projectPoint(selectedCertificate[`point_${side}_float`]);
    const positions = point.geometry.attributes.position;
    positions.setXYZ(0, ...projected);
    positions.needsUpdate = true;
    const ndc = new THREE.Vector3(...projected).project(camera);
    const label = $('markers').children[i];
    point.visible = projected.every(Number.isFinite);
    label.hidden = !point.visible || !Number.isFinite(ndc.x) || !Number.isFinite(ndc.y) || ndc.z < -1 || ndc.z > 1;
    label.style.left = `${(ndc.x + 1) * rect.width / 2}px`;
    label.style.top = `${(1 - ndc.y) * rect.height / 2}px`;
    point.userData = { side, asset: selectedCertificate[`point_${side}_float`].slice(), projected, ndc: ndc.toArray() };
  });
}
function clearCertificate() {
  selectedCertificate = null;
  clearMarkers();
  $('blocked').selectedIndex = -1;
  $('isolate').disabled = $('clear').disabled = true;
  $('certificate').textContent = app.view.state === 'end'
    ? 'Select a blocked grip to highlight its straddling piece and its two certificate points.'
    : 'Certificates are supplied for the end state only.';
  syncView();
}
function selectGrip(grip) {
  if (!app.ready || app.view.state !== 'end') return;
  clearCertificate();
  const cert = overlay.header.certificates.find((c) => c.grip === grip);
  if (!cert) return;
  selectedCertificate = cert;
  $('blocked').value = String(grip);
  $('piece').value = String(cert.piece);
  $('filter').value = 'all';
  $('certificate').textContent = `Grip ${grip} · straddling piece ${cert.piece}. Below and above are engine certificate points; their signs are exact.`;
  $('isolate').disabled = $('clear').disabled = false;
  for (const side of ['below', 'above']) {
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute([0, 0, 0], 3));
    const point = new THREE.Points(g, new THREE.PointsMaterial({ color: cssColor(`--${side}`), size: 9,
      sizeAttenuation: false, depthTest: false, depthWrite: false }));
    point.frustumCulled = false;
    point.renderOrder = 10;
    scene.add(point);
    markerMeshes.push(point);
    const label = document.createElement('span');
    label.className = `marker ${side}`;
    label.textContent = side;
    $('markers').append(label);
  }
  syncView();
}

function panel() {
  const h = state.header;
  $('menuIdentity').textContent = `${overlay.header.menu_identity.slice(0, 12)}…`;
  $('menuIdentity').title = overlay.header.menu_identity;
  $('contract').textContent = h.contract_revision;
  $('revision').textContent = String(h.revision);
  $('digest').textContent = `${h.digest.slice(0, 16)}…`;
  $('digest').title = h.digest;
  $('offLattice').textContent = `${state.off} / 177120${app.view.state === 'sweep-before' ? ' (base arrays)' : ''}`;
  $('slots').textContent = '259800 / 259800';
  $('blockedCount').textContent = app.view.state === 'end' ? String(overlay.header.certificates.length) : '— (end survey only)';
  $('blocked').replaceChildren();
  if (app.view.state === 'end') for (const c of overlay.header.certificates) {
    const option = document.createElement('option');
    option.value = String(c.grip);
    option.textContent = `Grip ${c.grip} · piece ${c.piece}`;
    $('blocked').append(option);
  }
  $('blocked').selectedIndex = -1;
  $('blocked').disabled = app.view.state !== 'end';
}
function pause() {
  app.playing = false;
  $('play').textContent = 'Play sweep';
  $('play').setAttribute('aria-pressed', 'false');
}
function setTime(t) {
  requireValue(app.ready && app.view.state === 'sweep-before', 'Load the before state to scrub its sweep');
  app.view.t = Math.max(0, Math.min(2, t));
  $('scrub').value = String(app.view.t);
  const phase = app.view.t <= 1 ? 'twist' : 'inverse';
  $('motion').textContent = `Grip ${overlay.header.sweep.grip} · ${phase} · sweep ${app.view.t.toFixed(3)} / 2 · base revision ${state.header.revision}`;
  syncView();
}
function fail(error) {
  const message = error instanceof Error ? error.message : String(error);
  app.errors.push(message);
  app.ready = false;
  app.model = null;
  app.drawn = null;
  pause();
  if (mesh) mesh.visible = false;
  clearMarkers();
  selectedCertificate = null;
  $('error').hidden = false;
  $('error').textContent = `Load error: ${message}. No state is drawn.`;
  $('badge').textContent = 'No data · drawing stopped';
  for (const id of ['play', 'scrub', 'blocked', 'isolate', 'clear']) $(id).disabled = true;
  for (const id of ['menuIdentity', 'contract', 'revision', 'digest', 'offLattice', 'slots', 'blockedCount']) $(id).textContent = '—';
  if (renderer) renderer.render(scene, camera);
  dirty = false;
}
async function loadSelection(menu, stage, writeHash = true) {
  requireValue(catalog.menus[menu], `Menu ${menu} was not exported`);
  if (stage !== 'solved' && !catalog.menus[menu].states[stage]) stage = catalog.menus[menu].states.end ? 'end' : 'solved';
  const generation = ++serial;
  app.ready = false;
  app.errors = [];
  app.model = null;
  app.drawn = null;
  pause();
  app.view = { menu, state: stage, t: 0 };
  $('menu').value = menu;
  $('state').value = stage;
  Array.from($('state').options).forEach((option) => { option.disabled = option.value !== 'solved' && !catalog.menus[menu].states[option.value]; });
  $('scrub').value = '0';
  $('badge').textContent = 'Loading verified arrays…';
  $('error').hidden = true;
  if (mesh) mesh.visible = false;
  clearMarkers();
  selectedCertificate = null;
  $('blocked').replaceChildren();
  for (const id of ['play', 'scrub', 'blocked', 'isolate', 'clear']) $(id).disabled = true;
  for (const id of ['menuIdentity', 'contract', 'revision', 'digest', 'offLattice', 'slots', 'blockedCount']) $(id).textContent = '—';
  if (renderer) renderer.render(scene, camera);
  try {
    const o = await loadOverlay(menu);
    const s = await loadState(menu, stage, o);
    if (generation !== serial) return false;
    if (!renderer) makeRenderer();
    bindState(s, o);
    state = s;
    overlay = o;
    app.model = { ...COUNTS, poses: s.header.poses };
    app.header = s.header;
    app.overlay = o.header;
    app.offLattice = s.off;
    panel();
    clearCertificate();
    $('play').disabled = $('scrub').disabled = !catalog.menus[menu].states['sweep-before'];
    $('motion').textContent = 'Sweep: twist → inverse. The base arrays remain at the before state.';
    if (writeHash) history.replaceState(null, '', `#${menu}-${stage}`);
    app.ready = true;
    mesh.visible = true;
    draw();
    return true;
  } catch (e) {
    if (generation === serial) fail(e);
    return false;
  }
}
function draw() {
  if (!renderer || !app.ready) return;
  syncView();
  renderer.render(scene, camera);
  // Only this mesh draws triangles. Points do not inflate the instance count.
  app.drawn = { triangles: renderer.info.render.triangles, calls: renderer.info.render.calls,
    instances: renderer.info.render.triangles / (mesh.geometry.attributes.position.count / 3),
    verticesPerInstance: mesh.geometry.attributes.position.count };
  app.focusPiece = material.uniforms.uFocus.value;
  $('badge').textContent = app.view.state === 'sweep-before' && app.view.t !== 0 && app.view.t !== 2
    ? 'Swept motion · uncertified float preview of the before state'
    : 'uncertified float drawing of an exact J1 state';
  dirty = false;
}

// ReadPixels is used only by the headless evidence check, never as a performance metric.
app.coverage = () => {
  if (!renderer) return 0;
  if (app.ready) draw();
  const gl = renderer.getContext(), w = gl.drawingBufferWidth, h = gl.drawingBufferHeight;
  const pixels = new Uint8Array(w * h * 4);
  gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
  const bg = renderer.getClearColor(new THREE.Color());
  const background = [bg.r, bg.g, bg.b].map((x) => Math.round(x * 255));
  let drawn = 0;
  for (let i = 0; i < pixels.length; i += 4) if ([0, 1, 2].some((k) => Math.abs(pixels[i + k] - background[k]) > 3)) drawn++;
  return drawn / (w * h);
};
app.markerData = () => markerMeshes.map((point) => ({ ...point.userData,
  uploaded: Array.from(point.geometry.attributes.position.array) }));
app.projectionParameters = () => { camera.updateMatrixWorld(); return projectionParameters(); };
app.sampleSweep = () => {
  requireValue(app.ready && app.view.state === 'sweep-before', 'The GPU sweep probe needs the before state');
  requireValue(renderer.extensions.has('EXT_color_buffer_float'), 'The GPU sweep check requires EXT_color_buffer_float');
  syncView();
  camera.updateMatrixWorld();
  const parameters = projectionParameters();
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.Float32BufferAttribute(new Float32Array(16 * 3), 3));
  const points = new THREE.Points(g, material);
  points.frustumCulled = false;
  const probeScene = new THREE.Scene();
  probeScene.add(points);
  const target = new THREE.WebGLRenderTarget(16, 1, { type: THREE.FloatType, format: THREE.RGBAFormat,
    minFilter: THREE.NearestFilter, magFilter: THREE.NearestFilter, depthBuffer: false, stencilBuffer: false });
  const results = [];
  try {
    material.uniforms.uProbe.value = true;
    renderer.setRenderTarget(target);
    for (let mode = 0; mode < 3; mode++) {
      material.uniforms.uProbeOutput.value = mode;
      renderer.render(probeScene, camera);
      const pixels = new Float32Array(16 * 4);
      renderer.readRenderTargetPixels(target, 0, 0, 16, 1, pixels);
      results.push(pixels);
    }
    requireValue(renderer.getContext().getError() === 0, 'GPU sweep readback failed');
  } finally {
    material.uniforms.uProbe.value = false;
    material.uniforms.uProbeOutput.value = 0;
    renderer.setRenderTarget(null);
    target.dispose();
    g.dispose();
    draw();
  }
  return { parameters, t: app.view.t, rows: Array.from(overlay.pieces, (piece, i) => ({ piece,
    world: Array.from(results[0].slice(i * 4, i * 4 + 4)),
    projected: Array.from(results[1].slice(i * 4, i * 4 + 3)),
    ndc: Array.from(results[2].slice(i * 4, i * 4 + 3)) })) };
};
app.goto = loadSelection;
app.setTime = setTime;
app.selectGrip = selectGrip;
app.clearGrip = clearCertificate;

async function seek(t) {
  pause();
  if (app.view.state !== 'sweep-before') {
    if (!await loadSelection(app.view.menu, 'sweep-before')) return;
  }
  if (app.view.state === 'sweep-before') setTime(t);
}
async function togglePlay() {
  if (app.playing) { pause(); return; }
  if (app.view.state !== 'sweep-before') {
    if (!await loadSelection(app.view.menu, 'sweep-before')) return;
  }
  if (!app.ready || app.view.state !== 'sweep-before') return;
  if (app.view.t === 2) setTime(0);
  app.playing = true;
  lastTime = performance.now();
  $('play').textContent = 'Pause sweep';
  $('play').setAttribute('aria-pressed', 'true');
}
function resetView() {
  if (!camera) return;
  $('rotateX').value = $('rotateY').value = '0';
  camera.position.set(0, 0, 5);
  controls.target.set(0, 0, 0);
  controls.update();
  syncView();
}
function isolateCertificate() {
  if (!selectedCertificate) return;
  $('filter').value = 'isolate';
  $('piece').value = String(selectedCertificate.piece);
  // Fit the two certified points in camera3, without moving either point or the piece.
  const a = new THREE.Vector3(...projectPoint(selectedCertificate.point_below_float));
  const b = new THREE.Vector3(...projectPoint(selectedCertificate.point_above_float));
  if (a.toArray().every(Number.isFinite) && b.toArray().every(Number.isFinite)) {
    const center = a.clone().add(b).multiplyScalar(0.5);
    const direction = camera.position.clone().sub(controls.target).normalize();
    controls.target.copy(center);
    camera.position.copy(center).addScaledVector(direction, Math.max(0.4, a.distanceTo(b) * 2.5));
    controls.update();
  }
  syncView();
}

$('menu').addEventListener('change', () => { loadSelection($('menu').value, $('state').value).catch(fail); });
$('state').addEventListener('change', () => { loadSelection($('menu').value, $('state').value).catch(fail); });
$('play').addEventListener('click', () => { togglePlay().catch(fail); });
$('scrub').addEventListener('input', () => { seek(Number($('scrub').value)).catch(fail); });
$('blocked').addEventListener('change', () => selectGrip(Number($('blocked').value)));
$('isolate').addEventListener('click', isolateCertificate);
$('clear').addEventListener('click', () => { $('filter').value = 'all'; clearCertificate(); });
$('resetView').addEventListener('click', resetView);
for (const id of ['projection', 'pole', 'facetShrink', 'stickerShrink', 'rotateX', 'rotateY', 'tint', 'filter', 'piece'])
  $(id).addEventListener('input', syncView);
function hashSelection() {
  const m = /^#(S4|I_a|I_b)-(solved|end|sweep-before)$/.exec(location.hash);
  return m ? { menu: m[1], stage: m[2] } : null;
}
window.addEventListener('hashchange', () => {
  const h = hashSelection();
  if (h && catalog?.menus[h.menu]) loadSelection(h.menu, h.stage, false).catch(fail);
});
window.addEventListener('error', (e) => fail(e.message));
window.addEventListener('unhandledrejection', (e) => fail(e.reason));

function animate(now) {
  requestAnimationFrame(animate);
  try {
    if (app.ready && app.playing) {
      const t = (app.view.t + (now - lastTime) / 5000) % 2;
      lastTime = now;
      setTime(t);
    }
    if (dirty && app.ready) draw();
  } catch (e) { fail(e); }
}
async function start() {
  requireValue(new Uint8Array(new Uint32Array([1]).buffer)[0] === 1, 'Little-endian typed arrays are required');
  [catalog, geometry] = await Promise.all([jsonFile('full/catalog.json'), loadGeometry()]);
  checkDimension(catalog, 'catalog.json');
  requireValue(catalog.format === 'magic600-full-viewer/1' && catalog.menus && Object.keys(catalog.menus).length
    && Object.keys(catalog.menus).every((name) => MENUS.includes(name)), 'catalog.json: invalid menus');
  requireValue(catalog.solved?.header === 'solved/header.json' && hexDigest(catalog.solved.digest), 'catalog.json: invalid solved state');
  for (const [name, item] of Object.entries(catalog.menus)) {
    requireValue(hexDigest(item.identity) && item.overlay === `${name}/overlay.json` && item.states,
      'catalog.json: invalid menu identity or overlay');
    for (const [stage, entry] of Object.entries(item.states)) requireValue(['end', 'sweep-before'].includes(stage)
      && entry.header === `${name}/${stage}/header.json` && hexDigest(entry.digest), 'catalog.json: invalid state path');
  }
  Array.from($('menu').options).forEach((option) => { option.disabled = !catalog.menus[option.value]; });
  const h = hashSelection();
  const menu = h && catalog.menus[h.menu] ? h.menu : Object.keys(catalog.menus)[0];
  await loadSelection(menu, h?.stage || 'end');
}
requestAnimationFrame(animate);
start().catch(fail);
