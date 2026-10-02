// Simulated-owner experiment (plan section 6). Deterministic per seed; prints JSON.
// Usage: node tools/tastelab/sim/experiment.mjs [--seeds 50] [--budget 60] [--setting low|full]
//        [--noise 1,2,4] [--first 0] [--candidates 512]
import { createRng } from "../core/rng.js";
import { buildPool, roundCandidates, classPairsFor } from "../page/candidates.js";
import { defaultSpace, encode, canonical, sobol, fromUnit, hardCheck, featureDim, SCENES } from "../core/space.js";
import { createModel, addLook, addObservation, fit, fitHyper, predict } from "../core/gp.js";
import { nextPair, settled } from "../core/acquire.js";
import { buildGeometry } from "../page/geometry.js";

const argv = process.argv.slice(2);
const arg = (name, dflt) => (argv.includes(name) ? argv[argv.indexOf(name) + 1] : dflt);
const SEEDS = Number(arg("--seeds", 50));
const FIRST = Number(arg("--first", 0));
const BUDGET = Number(arg("--budget", 60));
const SETTING = arg("--setting", "low");
const NOISE = arg("--noise", "1,2,4").split(",").map(Number);
const NCAND = Number(arg("--candidates", 512));
const EPS = 0.2;

const geo = buildGeometry();
const fullSpace = defaultSpace();
const pairsOf = classPairsFor(geo);
const feasible = (look) => hardCheck(look, pairsOf(look.classes).pairs).ok;

// Relevant parameters of the synthetic owner and their weights.
const RELEVANT = ["hueRotation", "chroma", "gap", "glow"];
const WEIGHTS = { hueRotation: 1, chroma: 1.5, gap: 1, glow: 1 };
// Scene-specific part: in "celebrating" the preferred glow is shifted.
const GLOW_SHIFT = { solving: 0, inspecting: 0, celebrating: 0.35 };

function unitOf(p, v) {
  return p.kind === "circular" ? (((v % 360) + 360) % 360) / 360 : (v - p.min) / (p.max - p.min);
}

function makeOwner(rng, pool) {
  // The optimum of the shared part is a feasible look drawn from the pool.
  const target = pool[rng.int(pool.length)];
  const t = {};
  for (const p of fullSpace.params) t[p.id] = unitOf(p, target[p.id]);
  const params = Object.fromEntries(fullSpace.params.map((p) => [p.id, p]));
  return {
    target,
    utility(look, scene) {
      let u = 0;
      for (const id of RELEVANT) {
        const p = params[id];
        let x = unitOf(p, look[id]), c = t[id];
        if (id === "glow") c = Math.min(1, Math.max(0, c + (c > 0.5 ? -1 : 1) * GLOW_SHIFT[scene]));
        let dx = x - c;
        if (p.kind === "circular") dx = Math.min(Math.abs(dx), 1 - Math.abs(dx));
        u -= WEIGHTS[id] * dx * dx;
      }
      return u;
    },
  };
}

const Phi = (x) => 0.5 * (1 + erf(x / Math.SQRT2));
function erf(x) {
  const s = Math.sign(x), a = Math.abs(x), t = 1 / (1 + 0.3275911 * a);
  const y = 1 - ((((1.061405429 * t - 1.453152027) * t + 1.421413741) * t - 0.284496736) * t + 0.254829592) * t * Math.exp(-a * a);
  return s * y;
}

function respond(rng, uA, uB, snr) {
  // Utilities span about [-4.5, 0]; scale so that snr is the latent signal-to-noise ratio.
  const d = (snr * (uA - uB)) / 0.5;
  const pA = Phi((d - EPS) / Math.SQRT2), pB = Phi((-d - EPS) / Math.SQRT2);
  const r = rng.next();
  return r < pA ? "A" : r < pA + pB ? "B" : "same";
}

function lowDim(look, base) {
  // Low-dimensional setting: only the relevant parameters vary; the rest stay at the target.
  const out = { ...base };
  for (const id of RELEVANT) out[id] = look[id];
  return out;
}

function runOne(seed, snr, method, pool, median) {
  const rng = createRng(1000003 * seed + 7919 * Math.round(snr * 10) + (method === "model" ? 1 : 2));
  const ownerRng = createRng(seed);
  const owner = makeOwner(ownerRng, pool);
  const scene = SCENES[seed % 3];
  const sceneIdx = SCENES.indexOf(scene);
  // u* over the feasible pool plus the target itself, for this scene.
  let uStar = owner.utility(owner.target, scene);
  for (const l of pool) uStar = Math.max(uStar, owner.utility(SETTING === "low" ? lowDim(l, owner.target) : l, scene));
  const uMed = median(owner, scene);
  const model = createModel({ dim: featureDim(fullSpace), scenes: 3 });
  const looks = [], keys = new Map();
  const lookIdx = (look) => {
    const k = JSON.stringify(look);
    if (!keys.has(k)) { keys.set(k, addLook(model, encode(fullSpace, look), sceneIdx)); looks.push(look); }
    return keys.get(k);
  };
  const map = SETTING === "low" ? (l) => lowDim(l, owner.target) : (l) => l;
  const runPool = SETTING === "low" ? lowPool(seed, owner, map) : pool;
  const regrets = [];
  let bestKey = null, stableFor = 0, falseSettled = false, settledAt = null;
  for (let q = 0; q < BUDGET; q++) {
    const best = bestOf(model, looks, sceneIdx);
    const cands = roundCandidates(fullSpace, runPool, pairsOf, rng, { best, n: NCAND, map });
    if (cands.length < 2) break;
    let A, B;
    if (method === "random") {
      const i = rng.int(cands.length);
      let j = rng.int(cands.length - 1);
      if (j >= i) j++;
      A = cands[i]; B = cands[j];
    } else {
      const pick = nextPair(model, cands.map((l) => ({ features: encode(fullSpace, l), scene: sceneIdx })), rng, {});
      A = pick.kind === "repeat" ? looks[pick.lookA] : cands[pick.a];
      B = pick.kind === "repeat" ? looks[pick.lookB] : cands[pick.b];
    }
    const outcome = respond(rng, owner.utility(A, scene), owner.utility(B, scene), snr);
    addObservation(model, { a: lookIdx(A), b: lookIdx(B), outcome });
    fit(model);
    if ((q + 1) % 10 === 0) { fitHyper(model, { maxEvals: 60 }); fit(model); }
    const nb = bestOf(model, looks, sceneIdx);
    const r = (uStar - owner.utility(nb, scene)) / Math.max(1e-12, uStar - uMed);
    regrets.push(r);
    const key = JSON.stringify(nb);
    stableFor = key === bestKey ? stableFor + 1 : 0;
    bestKey = key;
    if (method === "model" && stableFor >= 15 && settledAt === null) {
      const st = settled(model, cands.map((l) => ({ features: encode(fullSpace, l), scene: sceneIdx })), rng, { draws: 200, prob: 0.9 });
      if (st.settled) { settledAt = q + 1; if (r > 0.2) falseSettled = true; }
    }
  }
  return { regrets, rank: rankRelevant(model), settledAt, falseSettled };
}

const lowPools = new Map();
function lowPool(seed, owner, map) {
  if (!lowPools.has(seed)) lowPools.set(seed, buildPool(fullSpace, pairsOf, { map }));
  return lowPools.get(seed);
}

function bestOf(model, looks, sceneIdx) {
  const idx = looks.map((_, i) => i);
  if (!idx.length) return null;
  const pr = predict(model, idx.map((i) => ({ features: encode(fullSpace, looks[i]), scene: sceneIdx })));
  let b = 0;
  for (let i = 1; i < idx.length; i++) if (pr.mean[i] > pr.mean[b]) b = i;
  return looks[b];
}

function rankRelevant(model) {
  // True when every relevant parameter has a shorter length scale than every irrelevant one.
  const L = model.hyper.lengths;
  const per = {};
  let f = 0;
  for (const p of fullSpace.params) {
    per[p.id] = p.kind === "circular" ? Math.min(L[f], L[f + 1]) : L[f];
    f += p.kind === "circular" ? 2 : 1;
  }
  const rel = RELEVANT.map((id) => per[id]);
  const irr = fullSpace.params.map((p) => p.id).filter((id) => !RELEVANT.includes(id)).map((id) => per[id]);
  return Math.max(...rel) < Math.min(...irr);
}

function wilcoxonOneSided(x, y) {
  // H1: x < y (model regret below random). Normal approximation with tie correction.
  const d = x.map((v, i) => y[i] - v).filter((v) => v !== 0);
  const n = d.length;
  if (!n) return 1;
  const order = d.map((v, i) => [Math.abs(v), i]).sort((a, b) => a[0] - b[0]);
  const ranks = new Array(n);
  let tie = 0;
  for (let i = 0; i < n; ) {
    let j = i;
    while (j + 1 < n && order[j + 1][0] === order[i][0]) j++;
    const r = (i + j + 2) / 2;
    for (let k = i; k <= j; k++) ranks[order[k][1]] = r;
    const t = j - i + 1;
    tie += t * t * t - t;
    i = j + 1;
  }
  const W = d.reduce((s, v, i) => s + (v > 0 ? ranks[i] : 0), 0);
  const mu = (n * (n + 1)) / 4;
  const sd = Math.sqrt((n * (n + 1) * (2 * n + 1)) / 24 - tie / 48);
  return 1 - Phi((W - mu - 0.5) / sd);
}

const medianOf = (a) => { const s = [...a].sort((x, y) => x - y); return s.length % 2 ? s[(s.length - 1) / 2] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2; };

function main() {
  const pool = buildPool(fullSpace, pairsOf);
  if (!pool.length) throw new Error("no feasible look in the sweep");
  const median = (owner, scene) => medianOf(pool.map((l) => owner.utility(SETTING === "low" ? lowDim(l, owner.target) : l, scene)));
  const out = { setting: SETTING, budget: BUDGET, seeds: [FIRST, FIRST + SEEDS - 1], candidates: NCAND, pool: pool.length, eps: EPS, levels: {} };
  for (const snr of NOISE) {
    const res = { model: [], random: [] };
    for (let s = FIRST; s < FIRST + SEEDS; s++) {
      res.model.push(runOne(s, snr, "model", pool, median));
      res.random.push(runOne(s, snr, "random", pool, median));
    }
    const at = (runs, q) => runs.map((r) => r.regrets[Math.min(q, r.regrets.length) - 1]);
    const final = { model: at(res.model, BUDGET), random: at(res.random, BUDGET) };
    const curve = [10, 20, 30, 40, 50, 60].filter((q) => q <= BUDGET).map((q) => ({ q, model: medianOf(at(res.model, q)), random: medianOf(at(res.random, q)) }));
    out.levels[snr] = {
      medianRegret: { model: medianOf(final.model), random: medianOf(final.random) },
      wilcoxonP: wilcoxonOneSided(final.model, final.random),
      relevantRankedFirst: res.model.filter((r) => r.rank).length / SEEDS,
      settled: res.model.filter((r) => r.settledAt !== null).length / SEEDS,
      falseSettled: res.model.filter((r) => r.falseSettled).length / SEEDS,
      curve,
      perSeed: { model: final.model, random: final.random },
    };
  }
  console.log(JSON.stringify(out, null, 2));
}

main();
