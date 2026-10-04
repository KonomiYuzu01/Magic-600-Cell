// Simulated-owner experiment (plan section 6). Deterministic per seed; prints JSON.
// Both methods run the page's learner (page/learner.js) on their own answers: "model"
// answers the learner's pairs, "random" two distinct looks drawn uniformly from the
// scene's feasible sweep. The budget is split into sessions of 30 comparisons, so that the
// two-session settled rule of plan section 5.6 is scored as the page applies it.
// Criteria 1, 2 and 4 are scored at 90 comparisons and criterion 3 at 160 (owner decision
// of 3 October 2026); random pairs run to the first checkpoint only.
// With --families 2 the synthetic owner has two theme families whose regions of regret
// at most 0.10 are disjoint; their answers are interleaved, and each family's learner
// reads its own. --budget counts the comparisons per family.
// --taus and --stables replay the settled rule for a grid of values in the same runs.
// --dump appends each finished run to a file as one JSON line; a later call with the same
// file, options and code reuses the runs already there, so an interrupted experiment resumes.
// Usage: node tools/tastelab/sim/experiment.mjs [--seeds 50] [--budget 180] [--setting low|full]
//        [--noise 1,2,4] [--first 0] [--candidates 512] [--sessions budget/30] [--families 1|2]
//        [--owner quad|cross|bump] [--tau 0.05] [--stable 10] [--taus 0.02,0.05,0.1]
//        [--stables 10,15,20] [--jobs 1] [--dump runs.jsonl]
import { fork } from "node:child_process";
import { createHash } from "node:crypto";
import { appendFileSync, existsSync, readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { createRng } from "../core/rng.js";
import { buildPool, classPairsFor } from "../page/candidates.js";
import { defaultSpace, canonical, hardCheck, featureSlices, familiesOf, sceneSpace, SCENES } from "../core/space.js";
import { buildGeometry } from "../page/geometry.js";
import { createLearner } from "../page/learner.js";

// With --jobs, child processes started by runAll run the tasks; they get "--child" first.
const CHILD = process.argv[2] === "--child";
const argv = process.argv.slice(CHILD ? 3 : 2);
const arg = (name, dflt) => (argv.includes(name) ? argv[argv.indexOf(name) + 1] : dflt);
const SEEDS = Number(arg("--seeds", 50));
const FIRST = Number(arg("--first", 0));
const BUDGET = Number(arg("--budget", 180));
const SETTING = arg("--setting", "low");
const NOISE = arg("--noise", "1,2,4").split(",").map(Number);
const NCAND = Number(arg("--candidates", 512));
const SESSIONS = Number(arg("--sessions", BUDGET / 30));
const FAMILIES = Number(arg("--families", 1));
const JOBS = Number(arg("--jobs", 1));
const DUMP = arg("--dump", null);
// The options a run's result depends on: all but the choice of runs (--seeds, --first,
// --noise), the thread count and the dump file.
const RUN_OPTIONS = (() => {
  const skip = new Set(["--seeds", "--first", "--noise", "--jobs", "--dump"]);
  const kept = [];
  for (let i = 0; i < argv.length; i++) {
    if (skip.has(argv[i])) i++;
    else kept.push(argv[i]);
  }
  return kept.join(" ");
})();
// The code a run's result depends on: this harness and the modules of core/ and page/.
const CODE = (() => {
  const root = new URL("../", import.meta.url);
  const names = ["sim/experiment.mjs"];
  for (const dir of ["core", "page"]) {
    for (const name of readdirSync(new URL(`${dir}/`, root)).filter((n) => n.endsWith(".js")).sort()) names.push(`${dir}/${name}`);
  }
  const hash = createHash("sha256");
  for (const name of names) hash.update(`${name}\0`).update(readFileSync(new URL(name, root))).update("\0");
  return hash.digest("hex").slice(0, 16);
})();
// Checkpoints of plan section 6: criteria 1, 2 and 4 (and the two-family criterion) at
// CHECK comparisons per family, criterion 3 at RANK_AT. Random pairs stop at CHECK.
const CHECK = 90, RANK_AT = 160;
const RANDOM_BUDGET = Math.min(BUDGET, CHECK);
// Settled rule values under calibration (plan section 5.6).
const TAU = Number(arg("--tau", 0.05));
const STABLE = Number(arg("--stable", 10));
// Calibration grid: the learner's pairs and best looks depend on neither value (the settled
// test draws the same random numbers for every tau), so one run replays the settled rule for
// every grid value. The replay of the learner's own values is checked against its flags.
const list = (name) => (argv.includes(name) ? arg(name).split(",").map(Number) : null);
const TAUS = list("--taus"), STABLES = list("--stables");
const GRID = TAUS || STABLES ? (TAUS || [TAU]).flatMap((tau) => (STABLES || [STABLE]).map((stable) => ({ tau, stable }))) : [];
if ([{ tau: TAU, stable: STABLE }, ...GRID].some((g) => !(g.tau >= 0 && g.tau < 1) || !Number.isInteger(g.stable) || g.stable < 1)) {
  throw new RangeError("tau values must lie in [0, 1) and stable counts must be positive integers");
}
// The utility's shape: "quad" (plan section 6) is additive and quadratic. The robustness
// checks break one assumption each: "cross" adds an interaction of chroma and cell gap,
// "bump" makes every term a bounded bump instead of a parabola.
const OWNER = arg("--owner", "quad");
const EPS = 0.2;
if (!["quad", "cross", "bump"].includes(OWNER) || (OWNER !== "quad" && Number(arg("--families", 1)) !== 1)) {
  throw new RangeError("--owner must be quad, cross or bump; the robustness checks use one family");
}
if (!["low", "full"].includes(SETTING) || !Number.isInteger(SESSIONS) || SESSIONS < 1 || !Number.isInteger(BUDGET / SESSIONS)) {
  throw new RangeError("--setting must be low or full, and the budget must split into equal sessions");
}
// Two families are run in the full space only: the low setting's sweep depends on each
// family's optimum, so the families would not share one feasible sweep.
if (![1, 2].includes(FAMILIES) || (FAMILIES === 2 && SETTING !== "full")) {
  throw new RangeError("--families must be 1, or 2 with --setting full");
}

const geo = buildGeometry();
const fullSpace = defaultSpace();
const FAMILY_IDS = familiesOf(fullSpace).map((f) => f.id).slice(0, FAMILIES);
const pairsOf = classPairsFor(geo);
const params = Object.fromEntries(fullSpace.params.map((p) => [p.id, p]));
const tables = Object.fromEntries(SCENES.map((s) => [s, sceneSpace(fullSpace, s)]));
const feasibleIn = (scene, look) => hardCheck(look, pairsOf(look.classes).pairs, undefined, tables[scene]).ok;
const pool = buildPool(fullSpace, pairsOf);
if (!pool.length) throw new Error("no feasible look in the sweep");

// The feasible sweep of a scene's table; scenes without limits share one.
const sweeps = new Map();
function sweepOf(scene) {
  const key = JSON.stringify(tables[scene]);
  if (!sweeps.has(key)) sweeps.set(key, buildPool(tables[scene], pairsOf));
  return sweeps.get(key);
}

// Relevant parameters of the synthetic owner and their weights.
const RELEVANT = ["hueRotation", "chroma", "gap", "glow"];
const WEIGHTS = { hueRotation: 1, chroma: 1.5, gap: 1, glow: 1 };
const CROSS = 0.6; // "cross": correlation of the chroma and cell gap terms
const BUMP = 0.2; // "bump": width of each term on the unit axis
// Scene-specific part: in "celebrating" the preferred glow is shifted by this much
// on the unit axis, toward the middle of the range, then into the scene's limits.
const GLOW_SHIFT = 0.35;

function unitOf(p, v) {
  return p.kind === "circular" ? (((v % 360) + 360) % 360) / 360 : (v - p.min) / (p.max - p.min);
}
const valueOf = (p, u) => p.min + u * (p.max - p.min);

function makeOwner(target) {
  // The optimum of the shared part is a feasible look drawn from the pool.
  const t = {};
  for (const p of fullSpace.params) t[p.id] = unitOf(p, target[p.id]);
  const centre = (id, scene) => {
    let c = t[id];
    if (id === "glow" && scene === "celebrating") c = t.glow >= GLOW_SHIFT ? t.glow - GLOW_SHIFT : t.glow + GLOW_SHIFT;
    if (params[id].kind !== "circular") {
      const q = tables[scene].params.find((x) => x.id === id);
      c = Math.min(unitOf(params[id], q.max), Math.max(unitOf(params[id], q.min), c));
    }
    return c;
  };
  return {
    target,
    centre,
    // The scene's optimum, utility 0: the target with the scene's centres, within the scene's limits.
    optimum(scene) {
      const look = { ...target };
      for (const q of tables[scene].params) if (q.kind !== "circular") look[q.id] = Math.min(q.max, Math.max(q.min, look[q.id]));
      for (const id of RELEVANT) look[id] = valueOf(params[id], centre(id, scene));
      return canonical(fullSpace, look);
    },
    // Utility 0 at the optimum and negative elsewhere, for every shape.
    utility(look, scene) {
      const d = {};
      for (const id of RELEVANT) {
        const p = params[id];
        let dx = unitOf(p, look[id]) - centre(id, scene);
        if (p.kind === "circular") dx = dx - Math.round(dx); // signed, the shorter way round
        d[id] = dx;
      }
      let u = 0;
      for (const id of RELEVANT) u -= WEIGHTS[id] * (OWNER === "bump" ? 1 - Math.exp(-(d[id] ** 2) / (2 * BUMP ** 2)) : d[id] ** 2);
      // A positive semi-definite cross term (correlation CROSS < 1), so the optimum stays.
      if (OWNER === "cross") u -= 2 * CROSS * Math.sqrt(WEIGHTS.chroma * WEIGHTS.gap) * d.chroma * d.gap;
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

function respond(rng, uA, uB, snr, spread) {
  // The probit noise is 1 (plan section 5.1), so the latent utility is u scaled to a
  // standard deviation of snr over the feasible sweep.
  const d = (snr * (uA - uB)) / spread;
  const pA = Phi((d - EPS) / Math.SQRT2), pB = Phi((-d - EPS) / Math.SQRT2);
  const r = rng.next();
  return r < pA ? "A" : r < pA + pB ? "B" : "same";
}

function lowDim(look, base) {
  // Low-dimensional setting: only the relevant parameters vary; the rest stay at the optimum.
  const out = { ...base };
  for (const id of RELEVANT) out[id] = look[id];
  return out;
}

// One family of the synthetic owner in one scene: its optimum, sweep and regret.
function family(id, owner, scene, seed) {
  const optimum = owner.optimum(scene);
  // u* is the scene's optimum, which passes the hard checks within the scene's limits
  // (TL-D01); the median and the spread come from the setting's own feasible sweep (TL-D02).
  if (!feasibleIn(scene, optimum)) throw new Error(`seed ${seed}: the optimum fails the hard checks`);
  const map = SETTING === "low" ? (l) => lowDim(l, optimum) : undefined;
  const sweep = map ? buildPool(tables[scene], pairsOf, { map }) : sweepOf(scene);
  const uStar = owner.utility(optimum, scene);
  const u = sweep.map((l) => owner.utility(l, scene));
  // The optimum's relevant values pass through the unit axis and back, which can move one
  // by a unit in the last place; a sweep look that is the drawn target then beats u* by
  // about 1e-32. Only a larger excess means that the optimum is wrong.
  if (u.some((x) => x > uStar + 1e-12)) throw new Error(`seed ${seed}: a look of the sweep beats the optimum`);
  const mean = u.reduce((s, x) => s + x, 0) / u.length;
  const spread = Math.sqrt(u.reduce((s, x) => s + (x - mean) ** 2, 0) / u.length);
  const D = Math.max(1e-12, uStar - medianOf(u));
  return { id, owner, optimum, map, sweep, spread, D, regret: (look) => (uStar - owner.utility(look, scene)) / D };
}

// The utilities' common weighted metric between two families' optima (circular for hue).
function distance(a, b, scene) {
  let d2 = 0;
  for (const id of RELEVANT) {
    let dx = Math.abs(a.owner.centre(id, scene) - b.owner.centre(id, scene));
    if (params[id].kind === "circular") dx = Math.min(dx, 1 - dx);
    d2 += WEIGHTS[id] * dx * dx;
  }
  return Math.sqrt(d2);
}

// The families of one seed. Two families are drawn until their regions of regret at
// most 0.10 (balls of radius sqrt(0.1 D) in the metric) are disjoint; then a collapsed
// control, one look reported for both families, must fail the recovery predicate.
function fixture(seed, scene) {
  const rng = createRng(seed);
  const draw = (id) => family(id, makeOwner(pool[rng.int(pool.length)]), scene, seed);
  if (FAMILIES === 1) return [draw(FAMILY_IDS[0])];
  for (let tries = 0; tries < 1000; tries++) {
    const [a, b] = FAMILY_IDS.map(draw);
    if (distance(a, b, scene) <= Math.sqrt(0.1 * a.D) + Math.sqrt(0.1 * b.D)) continue;
    const recovered = (look) => a.regret(look) <= 0.1 && b.regret(look) <= 0.1;
    let summed = null, least = Infinity;
    for (const look of a.sweep) {
      const s = a.regret(look) + b.regret(look);
      if (s < least) { least = s; summed = look; }
    }
    for (const look of [a.optimum, b.optimum, summed]) {
      if (recovered(look)) throw new Error(`seed ${seed}: a collapsed control passes the predicate`);
    }
    return [a, b];
  }
  throw new Error(`seed ${seed}: no two families with disjoint regions`);
}

// Learners per process and family; the low setting needs new ones per seed,
// since its candidates depend on the owner's optimum.
const cached = [];
function learnerFor(seed, k, map) {
  const key = SETTING === "low" ? seed : "full";
  if (!cached[k] || cached[k].key !== key) cached[k] = { key, learner: createLearner(geo, { candidates: NCAND, map, tau: TAU, stableAnswers: STABLE }) };
  return cached[k].learner;
}

function runOne(seed, snr, method) {
  const scene = SCENES[seed % 3];
  const fams = fixture(seed, scene);
  const runSeed = 1000003 * seed + 7919 * Math.round(snr * 10) + (method === "model" ? 1 : 2);
  const rng = createRng(runSeed);
  const learners = fams.map((f, k) => learnerFor(seed, k, f.map));
  const records = [], settledRecords = [];
  // regrets: after every answer; ranks: whether the relevance ranking is correct, after every tenth.
  const per = fams.map(() => ({ regrets: [], ranks: [], sessions: [], settledAt: null, falseSettled: false, falseSettledAt: null }));
  // Replays of the settled rule per family: the learner's own values first, then the grid.
  const replays = fams.map(() => [{ tau: TAU, stable: STABLE }, ...GRID].map((g) => ({ ...g, records: [], sessions: [], settledAt: null, falseSettledAt: null })));
  const budget = method === "random" ? RANDOM_BUDGET : BUDGET;
  for (let s = 0; s < SESSIONS && per[0].regrets.length < budget; s++) {
    const session = `s${s + 1}`;
    fams.forEach((f, k) => learners[k].reset(fullSpace, records, { seed: runSeed + s + 7919 * k, session, settledRecords: settledRecords.slice(), family: f.id }));
    const diag = fams.map(() => ({ sessionSettledAt: null, sessionSettledWrong: false }));
    const stored = fams.map(() => new Map());
    for (const r of replays.flat()) { r.stored = new Map(); r.diag = { sessionSettledAt: null, sessionSettledWrong: false }; }
    // What the page does with a proposal: store a session's settled best per family and
    // scene (app.js recordSettled) and declare "settled" when another session confirms it.
    const observe = (k, p) => {
      const f = fams[k];
      if (p.type !== "pair") throw new Error(`seed ${seed}: ${p.reason || "no pair"}`);
      if (p.family !== f.id) throw new Error(`seed ${seed}: the learner of ${f.id} proposed for ${p.family}`);
      if (p.sessionSettled && p.best) {
        stored[k].set(p.scene, p.best);
        if (diag[k].sessionSettledAt === null) diag[k].sessionSettledAt = per[k].regrets.length;
        if (f.regret(p.best) > 0.2) diag[k].sessionSettledWrong = true;
      }
      if (p.settled) {
        if (per[k].settledAt === null) per[k].settledAt = per[k].regrets.length;
        if (f.regret(p.best) > 0.2 && !per[k].falseSettled) {
          per[k].falseSettled = true;
          per[k].falseSettledAt = per[k].regrets.length;
        }
      }
      // The replays apply the tests of the learner's propose() with their own values.
      const history = learners[k].state.bestHistory[SCENES.indexOf(p.scene)];
      replays[k].forEach((r, i) => {
        const recent = history.slice(-r.stable);
        const stable = Boolean(p.best) && recent.length === r.stable && recent.every((x) => x !== null && learners[k].sameRegion(x, p.best));
        const sessionSettled = Boolean(stable && p.best && p.settledRegret <= r.tau);
        const confirmed = sessionSettled && r.records.some((x) => x.scene === p.scene && x.session !== session && learners[k].sameRegion(x.best, p.best));
        if (i === 0 && (stable !== p.stable || sessionSettled !== p.sessionSettled || confirmed !== p.settled)) {
          throw new Error(`seed ${seed}: the replay of the settled rule disagrees with the learner`);
        }
        const n = per[k].regrets.length;
        if (sessionSettled) {
          r.stored.set(p.scene, p.best);
          if (r.diag.sessionSettledAt === null) r.diag.sessionSettledAt = n;
          if (f.regret(p.best) > 0.2) r.diag.sessionSettledWrong = true;
        }
        if (confirmed) {
          if (r.settledAt === null) r.settledAt = n;
          if (f.regret(p.best) > 0.2 && r.falseSettledAt === null) r.falseSettledAt = n;
        }
      });
    };
    for (let q = 0; q < BUDGET / SESSIONS && per[0].regrets.length < budget; q++) {
      fams.forEach((f, k) => {
        let A, B;
        if (method === "random") {
          const i = rng.int(f.sweep.length);
          let j = rng.int(f.sweep.length - 1);
          if (j >= i) j++;
          A = f.sweep[i]; B = f.sweep[j];
        } else {
          const p = learners[k].propose(scene);
          observe(k, p);
          A = p.lookA; B = p.lookB;
        }
        const answer = respond(rng, f.owner.utility(A, scene), f.owner.utility(B, scene), snr, f.spread);
        const record = { lookA: A, lookB: B, family: f.id, scene, answer, session };
        records.push(record);
        // Every learner sees every answer and uses only its own family's.
        for (const l of learners) l.answer(record);
        per[k].regrets.push(f.regret(learners[k].best(scene)));
        if (per[k].regrets.length % 10 === 0) per[k].ranks.push(rankRelevant(learners[k].state.model));
      });
    }
    // The page shows the state after the session's last answer too.
    if (method === "model") fams.forEach((f, k) => observe(k, learners[k].propose(scene)));
    fams.forEach((f, k) => {
      for (const [sc, best] of stored[k]) settledRecords.push({ family: f.id, scene: sc, session, best });
      per[k].sessions.push(diag[k]);
      for (const r of replays[k]) {
        for (const [sc, best] of r.stored) r.records.push({ scene: sc, session, best });
        r.sessions.push(r.diag);
      }
    });
  }
  return {
    seed,
    families: fams.map((f, k) => ({
      ...per[k],
      // The best look shown so far, by true utility: separates the choice of pairs from the final pick.
      shown: Math.min(...learners[k].state.looks.map((l) => f.regret(l.look))),
      grid: replays[k].slice(1).map(({ tau, stable, settledAt, falseSettledAt, sessions }) => ({ tau, stable, settledAt, falseSettledAt, sessions })),
    })),
  };
}

function rankRelevant(model) {
  // True when every relevant parameter has a shorter length scale than every irrelevant one;
  // the model keeps one length per parameter (plan section 5.3).
  const L = model.hyper.lengths;
  const per = Object.fromEntries(featureSlices(fullSpace).map(({ id }, g) => [id, L[g]]));
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

function medianOf(a) {
  const s = [...a].sort((x, y) => x - y);
  return s.length % 2 ? s[(s.length - 1) / 2] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2;
}

// Runs the tasks in `JOBS` child processes; every result depends only on its task, the run
// options and the code, so runs found in the dump file under the same ones are reused.
// Processes, not worker threads: on Windows the worker threads of one process mostly waited
// on each other, and 28 of them kept about two cores busy.
async function runAll(tasks) {
  const results = new Array(tasks.length);
  const taskKey = (t) => `${t.seed}|${t.snr}|${t.method}`;
  if (DUMP && existsSync(DUMP)) {
    const saved = new Map();
    for (const line of readFileSync(DUMP, "utf8").split("\n")) {
      if (!line.trim()) continue;
      let entry;
      try { entry = JSON.parse(line); } catch { continue; }
      if (entry.options === RUN_OPTIONS && entry.code === CODE) saved.set(taskKey(entry), entry.result);
    }
    tasks.forEach((t, i) => { if (saved.has(taskKey(t))) results[i] = saved.get(taskKey(t)); });
  }
  const keep = (i, result) => {
    results[i] = result;
    if (DUMP) appendFileSync(DUMP, JSON.stringify({ options: RUN_OPTIONS, code: CODE, ...tasks[i], result }) + "\n");
  };
  const todo = tasks.map((_, i) => i).filter((i) => results[i] === undefined);
  if (JOBS <= 1) {
    for (const i of todo) keep(i, runOne(tasks[i].seed, tasks[i].snr, tasks[i].method));
    return results;
  }
  let next = 0;
  await Promise.all(Array.from({ length: Math.min(JOBS, todo.length) }, () => new Promise((resolve, reject) => {
    const child = fork(fileURLToPath(import.meta.url), ["--child", ...argv], {
      serialization: "advanced", stdio: ["ignore", "ignore", "inherit", "ipc"],
    });
    let busy = false;
    const send = () => {
      if (next >= todo.length) { child.disconnect(); resolve(); return; }
      const i = todo[next++];
      busy = true;
      child.send({ i, ...tasks[i] });
    };
    child.on("message", (m) => {
      busy = false;
      if (m.error) { reject(new Error(m.error)); return; }
      keep(m.i, m.result);
      send();
    });
    child.on("error", reject);
    child.on("exit", (code) => { if (busy) reject(new Error(`a child process exited with code ${code} during a run`)); });
    send();
  })));
  return results;
}

// Regret after q answers and ranking after q answers (q a multiple of 10), per run.
const regretAt = (runs, q) => runs.map((r) => r.regrets[Math.min(q, r.regrets.length) - 1]);
const rankAt = (runs, q) => runs.map((r) => Boolean(r.ranks[Math.min(q, 10 * r.ranks.length) / 10 - 1]));
const share = (values) => values.filter(Boolean).length / values.length;
const tens = (limit) => Array.from({ length: Math.floor(limit / 10) }, (_, i) => 10 * (i + 1));

// The single-family summary of plan section 6, from runs that each hold one family.
// Criteria 1, 2 and 4 at the checkpoint, criterion 3 at RANK_AT; `pass` is null for a
// criterion whose checkpoint the budget does not reach.
function summary(res) {
  const check = Math.min(CHECK, BUDGET);
  const model = regretAt(res.model, check), random = regretAt(res.random, check);
  const by = (q, key) => share(res.model.map((r) => r[key] !== null && r[key] <= q));
  const ranked = BUDGET >= RANK_AT ? rankAt(res.model, RANK_AT) : null;
  const p = wilcoxonOneSided(model, random);
  const reached = check === CHECK;
  return {
    checkpoint: check,
    medianRegret: { model: medianOf(model), random: medianOf(random) },
    regretAtMost010: { model: share(model.map((x) => x <= 0.1)), random: share(random.map((x) => x <= 0.1)) },
    wilcoxonP: p,
    falseSettled: by(check, "falseSettledAt"),
    settled: by(check, "settledAt"),
    relevantRankedFirst: {
      at: ranked ? RANK_AT : null,
      model: ranked ? share(ranked) : null,
      atCheckpoint: { model: share(rankAt(res.model, check)), random: share(rankAt(res.random, check)) },
    },
    pass: {
      regret: reached ? medianOf(model) <= 0.1 : null,
      beatsRandom: reached ? p < 0.01 : null,
      falseSettled: reached ? by(check, "falseSettledAt") <= 0.1 : null,
      ranking: ranked ? share(ranked) >= 0.8 : null,
    },
    // After the whole budget: diagnostics beyond the checkpoints.
    final: { at: BUDGET, medianRegret: medianOf(regretAt(res.model, BUDGET)), falseSettled: by(BUDGET, "falseSettledAt"), settled: by(BUDGET, "settledAt") },
    // Session-level diagnostics: one session's settled state before another session confirms it.
    sessionSettled: res.model[0].sessions.map((_, s) => ({
      session: s + 1,
      reached: share(res.model.map((r) => r.sessions[s].sessionSettledAt !== null)),
      wrong: share(res.model.map((r) => r.sessions[s].sessionSettledWrong)),
    })),
    // The settled rule's calibration grid (plan section 5.6), from the replays.
    settledGrid: GRID.map((g, i) => {
      const runs = res.model.map((r) => r.grid[i]);
      const at = (q, key) => share(runs.map((r) => r[key] !== null && r[key] <= q));
      return {
        ...g,
        falseSettled: at(check, "falseSettledAt"),
        settled: at(check, "settledAt"),
        final: { falseSettled: at(BUDGET, "falseSettledAt"), settled: at(BUDGET, "settledAt") },
        sessionSettled: runs[0].sessions.map((_, s) => ({
          session: s + 1,
          reached: share(runs.map((r) => r.sessions[s].sessionSettledAt !== null)),
          wrong: share(runs.map((r) => r.sessions[s].sessionSettledWrong)),
        })),
      };
    }),
    curve: tens(BUDGET).map((q) => ({
      q,
      model: medianOf(regretAt(res.model, q)),
      random: q <= RANDOM_BUDGET ? medianOf(regretAt(res.random, q)) : null,
      ranked: share(rankAt(res.model, q)),
    })),
    bestShown: { model: medianOf(res.model.map((r) => r.shown)), random: medianOf(res.random.map((r) => r.shown)) },
    perSeed: {
      seeds: res.model.map((r) => r.seed), model, random,
      rankedFirst: ranked, falseSettled: res.model.map((r) => r.falseSettledAt !== null && r.falseSettledAt <= check),
    },
  };
}

async function main() {
  const tasks = NOISE.flatMap((snr) => Array.from({ length: SEEDS }, (_, k) => ["model", "random"].map((method) => ({ seed: FIRST + k, snr, method }))).flat());
  const results = await runAll(tasks);
  const out = {
    code: CODE, setting: SETTING, owner: OWNER, budget: BUDGET, randomBudget: RANDOM_BUDGET, sessions: SESSIONS, families: FAMILIES,
    checkpoints: { criteria: CHECK, ranking: RANK_AT }, settledRule: { tau: TAU, stable: STABLE, grid: GRID },
    seeds: [FIRST, FIRST + SEEDS - 1], candidates: NCAND, pool: pool.length, eps: EPS, levels: {},
  };
  for (const snr of NOISE) {
    const of = (method) => results.filter((_, i) => tasks[i].snr === snr && tasks[i].method === method);
    const family = (method, k) => of(method).map((r) => ({ seed: r.seed, ...r.families[k] }));
    if (FAMILIES === 1) {
      out.levels[snr] = summary({ model: family("model", 0), random: family("random", 0) });
      continue;
    }
    // Two families: a seed recovers both when the larger of the two regrets at the checkpoint is at most 0.10.
    const check = Math.min(CHECK, BUDGET);
    const larger = (method, q) => of(method).map((r) => Math.max(...r.families.map((f) => f.regrets[Math.min(q, f.regrets.length) - 1])));
    const max = { model: larger("model", check), random: larger("random", check) };
    out.levels[snr] = {
      checkpoint: check,
      medianMaxRegret: { model: medianOf(max.model), random: medianOf(max.random) },
      bothRecovered: { model: share(max.model.map((x) => x <= 0.1)), random: share(max.random.map((x) => x <= 0.1)) },
      wilcoxonP: wilcoxonOneSided(max.model, max.random),
      pass: check === CHECK ? medianOf(max.model) <= 0.1 : null,
      curve: tens(BUDGET).map((q) => ({ q, model: medianOf(larger("model", q)), random: q <= RANDOM_BUDGET ? medianOf(larger("random", q)) : null })),
      perFamily: FAMILY_IDS.map((id, k) => {
        const s = summary({ model: family("model", k), random: family("random", k) });
        return { id, medianRegret: s.medianRegret, relevantRankedFirst: s.relevantRankedFirst, settled: s.settled, falseSettled: s.falseSettled, bestShown: s.bestShown, settledGrid: s.settledGrid };
      }),
      perSeed: { seeds: of("model").map((r) => r.seed), model: max.model, random: max.random },
    };
  }
  console.log(JSON.stringify(out, null, 2));
}

if (CHILD) {
  process.on("message", (task) => {
    let reply;
    try {
      reply = { i: task.i, result: runOne(task.seed, task.snr, task.method) };
    } catch (err) {
      reply = { i: task.i, error: String((err && err.stack) || err) };
    }
    process.send(reply);
  });
  process.on("disconnect", () => process.exit(0));
} else {
  main().then(() => process.exit(0), (err) => { console.error(err); process.exit(1); });
}
