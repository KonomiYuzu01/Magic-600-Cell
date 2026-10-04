// Palette feasibility sweep (plan section 4.3). Deterministic; prints JSON.
// Usage: node tools/tastelab/sim/sweep.mjs [--points 4096]
import { buildGeometry, properColouring } from "../page/geometry.js";
import { defaultSpace, sobol, fromUnit, canonical, hardCheck } from "../core/space.js";
import { mapToGamut, deltaE, simulateCvd, linearToOklab, CVD_KINDS } from "../core/color.js";

const args = process.argv.slice(2);
const N = Number(args[args.indexOf("--points") + 1]) || 4096;
const geo = buildGeometry();
const space = defaultSpace();
const PALETTE = new Set(["hueRotation", "hueSpread", "lightness", "lightnessAlt", "chroma", "bgLightness", "bgHue", "bgTint"]);

function pairsFor(k, colours = properColouring(20, geo.ringAdjacency, k)) {
  if (!colours) return null;
  const seen = new Map();
  for (const [i, j] of geo.ringAdjacency) {
    const a = Math.min(colours[i], colours[j]), b = Math.max(colours[i], colours[j]);
    seen.set(`${a},${b}`, [a, b]);
  }
  return [...seen.values()];
}

function middle(look) {
  const out = { ...look };
  for (const p of space.params) if (!PALETTE.has(p.id) && p.id !== "classes") out[p.id] = (p.min + p.max) / 2;
  return out;
}

// Graph-coloured palettes with k classes, the phase 1 rule.
const byK = {};
for (let k = 4; k <= 8; k++) {
  const pairs = pairsFor(k);
  const units = sobol(N, space.params.length, 1);
  let ok = 0, best = 0;
  const reasons = {};
  for (const u of units) {
    const look = canonical(space, { ...middle(fromUnit(space, u)), classes: k });
    const r = hardCheck(look, pairs);
    if (r.ok) ok++;
    else {
      // Count each look once per kind of failure (gamut, background, or the limiting vision view).
      const kinds = new Set(r.reasons.map((why) => (why.startsWith("deltaE:") ? why.split(":").slice(0, 2).join(":") : why.split(":")[0])));
      for (const kind of kinds) reasons[kind] = (reasons[kind] || 0) + 1;
    }
    const worst = Math.min(...Object.values(r.minDeltaE));
    if (Number.isFinite(worst)) best = Math.max(best, worst);
  }
  byK[k] = { feasible: ok, of: N, bestWorstViewDeltaE: Number(best.toFixed(4)), rejections: reasons };
}

// The rejected rule: 20 classes, one per ring, hues in ring order.
const views = (lin) => [lin, ...CVD_KINDS.map((k) => simulateCvd(lin, k))].map(linearToOklab);
let feasible20 = 0, best20 = 0;
for (const u of sobol(N, space.params.length, 1)) {
  const look = canonical(space, fromUnit(space, u));
  const cols = [];
  for (let i = 0; i < 20; i++) {
    const L = look.lightness + (i % 2 ? look.lightnessAlt : -look.lightnessAlt);
    const m = mapToGamut(L, look.chroma, look.hueRotation + (look.hueSpread * i) / 20);
    if (!m.ok) { cols.length = 0; break; }
    cols.push(views(m.linear));
  }
  if (!cols.length) continue;
  let worst = Infinity;
  for (const [a, b] of geo.ringAdjacency) for (let v = 0; v < 4; v++) worst = Math.min(worst, deltaE(cols[a][v], cols[b][v]));
  if (worst >= 0.08) feasible20++;
  best20 = Math.max(best20, worst);
}

console.log(JSON.stringify({
  points: N,
  threshold: { deltaE: 0.08, bgL: 0.2 },
  ringGraph: { rings: 20, pairs: geo.ringAdjacency.length },
  graphColoured: byK,
  twentyClasses: { feasible: feasible20, of: N, bestWorstViewDeltaE: Number(best20.toFixed(4)) },
}, null, 2));
