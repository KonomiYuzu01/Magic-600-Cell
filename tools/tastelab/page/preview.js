// WebGL 2 preview of one Taste Lab look: the 600 cells of the 600-cell,
// projected from 4D, coloured by ring class, with one animated cap turn.
// Synthetic geometry for taste comparisons; not the puzzle's mechanical model.
import { applyMatrix, capTurn } from "./geometry.js";

const VS = `#version 300 es
in vec3 aPos; in vec3 aNormal; in vec3 aColor; in vec3 aBary; in float aDepth4;
uniform mat4 uView; uniform mat4 uProj;
out vec3 vColor; out vec3 vNormal; out vec3 vBary; out float vFog; out vec3 vEye; out float vDepth4;
void main() {
  vec4 eye = uView * vec4(aPos, 1.0);
  vEye = eye.xyz; vNormal = mat3(uView) * aNormal; vColor = aColor; vBary = aBary; vDepth4 = aDepth4;
  vFog = -eye.z;
  gl_Position = uProj * eye;
}`;

const FS = `#version 300 es
precision highp float;
in vec3 vColor; in vec3 vNormal; in vec3 vBary; in float vFog; in vec3 vEye; in float vDepth4;
uniform vec3 uBg; uniform float uGloss; uniform float uGlow; uniform float uFog;
uniform float uEdgeWeight; uniform float uEdgeBright;
out vec4 outColor;
vec3 toLinear(vec3 c) { return mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(0.04045, c)); }
vec3 toSrgb(vec3 c) { return mix(c * 12.92, 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, step(0.0031308, c)); }
void main() {
  vec3 n = normalize(vNormal);
  vec3 v = normalize(-vEye);
  if (dot(n, v) < 0.0) n = -n;
  vec3 l = normalize(vec3(0.4, 0.7, 0.6));
  vec3 base = toLinear(vColor);
  float diff = 0.35 + 0.65 * max(dot(n, l), 0.0);
  vec3 h = normalize(l + v);
  float spec = uGloss * pow(max(dot(n, h), 0.0), mix(8.0, 96.0, uGloss));
  float rim = uGlow * pow(1.0 - max(dot(n, v), 0.0), 2.0);
  vec3 c = base * diff + vec3(spec) + base * rim * 1.5;
  if (uEdgeWeight > 0.0) {
    vec3 w = fwidth(vBary);
    vec3 e = smoothstep(vec3(0.0), w * uEdgeWeight, vBary);
    float edge = 1.0 - min(min(e.x, e.y), e.z);
    c = mix(c, mix(vec3(0.0), vec3(1.0), uEdgeBright), edge * 0.85);
  }
  float fog = 1.0 - exp(-uFog * 0.6 * max(vFog - 1.5, 0.0));
  vec3 bg = toLinear(uBg);
  c = mix(c, bg, clamp(fog, 0.0, 1.0));
  outColor = vec4(toSrgb(clamp(c, 0.0, 1.0)), 1.0);
}`;

function compile(gl, type, src) {
  const s = gl.createShader(type);
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
  return s;
}

function perspective(fov, aspect, near, far) {
  const f = 1 / Math.tan(fov / 2), nf = 1 / (near - far);
  return new Float32Array([f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, (far + near) * nf, -1, 0, 0, 2 * far * near * nf, 0]);
}

function viewMatrix(yaw, pitch, dist) {
  const cy = Math.cos(yaw), sy = Math.sin(yaw), cp = Math.cos(pitch), sp = Math.sin(pitch);
  // Column-major: rotate about y by yaw, then about x by pitch, then move back by dist.
  return new Float32Array([cy, sp * sy, -cp * sy, 0, 0, cp, sp, 0, sy, -sp * cy, cp * cy, 0, 0, 0, -dist, 1]);
}

// Cubic Bezier easing through (0,0), (a,0), (1-b,1), (1,1).
export function ease(t, a, b) {
  const x1 = a, x2 = 1 - b;
  const bx = (s) => 3 * (1 - s) * (1 - s) * s * x1 + 3 * (1 - s) * s * s * x2 + s * s * s;
  const by = (s) => 3 * (1 - s) * s * s + s * s * s;
  let lo = 0, hi = 1;
  for (let i = 0; i < 40; i++) {
    const mid = (lo + hi) / 2;
    if (bx(mid) < t) lo = mid; else hi = mid;
  }
  return by((lo + hi) / 2);
}

// 4D perspective: the point (0,0,0,1) is nearest the viewer; w = -1 is farthest.
const EYE_W = 2.2;
const CULL_W = 0.5;
function project(p) {
  const s = 1 / (EYE_W - p[3]);
  return [p[0] * s * 2.4, p[1] * s * 2.4, p[2] * s * 2.4, p[3]];
}

export class Preview {
  constructor(canvas, geometry) {
    this.canvas = canvas;
    this.geo = geometry;
    const gl = canvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });
    if (!gl) throw new Error("WebGL 2 is not available in this browser.");
    this.gl = gl;
    const prog = gl.createProgram();
    gl.attachShader(prog, compile(gl, gl.VERTEX_SHADER, VS));
    gl.attachShader(prog, compile(gl, gl.FRAGMENT_SHADER, FS));
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog));
    this.prog = prog;
    this.loc = {};
    for (const n of ["aPos", "aNormal", "aColor", "aBary", "aDepth4"]) this.loc[n] = gl.getAttribLocation(prog, n);
    for (const n of ["uView", "uProj", "uBg", "uGloss", "uGlow", "uFog", "uEdgeWeight", "uEdgeBright"]) this.loc[n] = gl.getUniformLocation(prog, n);
    this.vao = gl.createVertexArray();
    this.buf = gl.createBuffer();
    // 600 cells x 4 faces x 3 vertices x (3 pos + 3 normal + 3 colour + 3 bary + 1 depth).
    this.stride = 13;
    this.data = new Float32Array(600 * 12 * this.stride);
    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buf);
    gl.bufferData(gl.ARRAY_BUFFER, this.data.byteLength, gl.DYNAMIC_DRAW);
    const F = 4, st = this.stride * F;
    const attr = (name, size, off) => {
      gl.enableVertexAttribArray(this.loc[name]);
      gl.vertexAttribPointer(this.loc[name], size, gl.FLOAT, false, st, off * F);
    };
    attr("aPos", 3, 0); attr("aNormal", 3, 3); attr("aColor", 3, 6); attr("aBary", 3, 9); attr("aDepth4", 1, 12);
    this.look = null;
    this.cellColors = null;
    this.turnCell = 0;
    this.turnStart = 0;
    this.yaw = 0.6;
    this.raf = 0;
  }

  // look: parameter object; cellColors: Float32Array(600*3) sRGB in [0,1]; bg: [r,g,b] sRGB.
  setLook(look, cellColors, bg) {
    this.look = look;
    this.cellColors = cellColors;
    this.bg = bg;
    this.turnStart = performance.now();
  }

  setTurnCell(cell) {
    this.turnCell = cell;
  }

  start() {
    const loop = (now) => {
      this.draw(now);
      this.raf = requestAnimationFrame(loop);
    };
    this.raf = requestAnimationFrame(loop);
  }

  stop() {
    cancelAnimationFrame(this.raf);
  }

  build(now) {
    const { geo, look } = this;
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const period = look.turnMs + 900;
    const phase = ((now - this.turnStart) % period) / look.turnMs;
    const t = reduced ? 0 : Math.min(Math.max(phase, 0), 1);
    const turn = capTurn(geo, this.turnCell, ease(t, look.easeA, look.easeB));
    const inCap = new Uint8Array(600);
    for (const c of turn.capCells) inCap[c] = 1;
    const shrink = 1 - look.gap;
    const V = geo.vertices;
    let o = 0;
    const d = this.data;
    const p4 = [[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]];
    for (let c = 0; c < 600; c++) {
      const cell = geo.cells[c];
      const cen = [0, 0, 0, 0];
      for (let k = 0; k < 4; k++) {
        const v = cell[k];
        let p = [V[4 * v], V[4 * v + 1], V[4 * v + 2], V[4 * v + 3]];
        if (inCap[c]) p = applyMatrix(turn.matrix, p);
        p4[k] = p;
        for (let j = 0; j < 4; j++) cen[j] += p[j] / 4;
      }
      if (cen[3] > CULL_W) {
        // Cells nearest the 4D eye would enclose the rest; leave them out.
        this.data.fill(0, o, o + 12 * this.stride);
        o += 12 * this.stride;
        continue;
      }
      const q = p4.map((p) => project(p.map((x, j) => cen[j] + (x - cen[j]) * shrink)));
      const col = [this.cellColors[3 * c], this.cellColors[3 * c + 1], this.cellColors[3 * c + 2]];
      const faces = [[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]];
      for (const f of faces) {
        const a = q[f[0]], b = q[f[1]], e = q[f[2]];
        const u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]], w = [e[0] - a[0], e[1] - a[1], e[2] - a[2]];
        const n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
        const len = Math.hypot(n[0], n[1], n[2]) || 1;
        const bary = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
        [a, b, e].forEach((p, i) => {
          d[o++] = p[0]; d[o++] = p[1]; d[o++] = p[2];
          d[o++] = n[0] / len; d[o++] = n[1] / len; d[o++] = n[2] / len;
          d[o++] = col[0]; d[o++] = col[1]; d[o++] = col[2];
          d[o++] = bary[i][0]; d[o++] = bary[i][1]; d[o++] = bary[i][2];
          d[o++] = p[3];
        });
      }
    }
  }

  draw(now) {
    const { gl, look } = this;
    if (!look) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const w = Math.max(1, Math.round(this.canvas.clientWidth * dpr)), h = Math.max(1, Math.round(this.canvas.clientHeight * dpr));
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    this.build(now);
    gl.viewport(0, 0, w, h);
    gl.clearColor(this.bg[0], this.bg[1], this.bg[2], 1);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    gl.enable(gl.DEPTH_TEST);
    gl.useProgram(this.prog);
    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buf);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, this.data);
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const yaw = this.yaw + (reduced ? 0 : now * 0.00012);
    gl.uniformMatrix4fv(this.loc.uView, false, viewMatrix(yaw, 0.35, 4.2));
    gl.uniformMatrix4fv(this.loc.uProj, false, perspective(0.8, w / h, 0.1, 40));
    gl.uniform3fv(this.loc.uBg, this.bg);
    gl.uniform1f(this.loc.uGloss, look.gloss);
    gl.uniform1f(this.loc.uGlow, look.glow);
    gl.uniform1f(this.loc.uFog, look.fog);
    gl.uniform1f(this.loc.uEdgeWeight, look.edgeWeight);
    gl.uniform1f(this.loc.uEdgeBright, look.edgeBrightness);
    gl.drawArrays(gl.TRIANGLES, 0, 600 * 12);
  }
}
