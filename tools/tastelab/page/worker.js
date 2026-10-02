// Taste Lab model worker: rebuilds the preference model from stored
// comparisons, fits it, and proposes the next pair of looks.
import { createRng } from "../core/rng.js";
import { defaultSpace, encode, canonical, featureDim, SCENES } from "../core/space.js";
import { createModel, addLook, addObservation, fit, fitHyper, predict } from "../core/gp.js";
import { nextPair, settled } from "../core/acquire.js";
import { buildGeometry } from "./geometry.js";
import { buildPool, roundCandidates, classPairsFor } from "./candidates.js";

const CANDIDATES = 512;
const LOOK_CAP = 400;
const geo = buildGeometry();
const pairsOf = classPairsFor(geo);
let poolFor = null;

function feasiblePool(space) {
  const key = JSON.stringify(space);
  if (!poolFor || poolFor.key !== key) poolFor = { key, pool: buildPool(space, pairsOf) };
  return poolFor.pool;
}

function lookKey(look) {
  return JSON.stringify(look);
}

let state = null;

function reset(space, records, seed) {
  const sp = space || defaultSpace();
  state = {
    space: sp,
    records: [],
    model: createModel({ dim: featureDim(sp), scenes: SCENES.length }),
    looks: [],
    lookIndex: new Map(),
    rejected: [],
    rng: createRng(seed >>> 0),
    answersSinceHyper: 0,
    bestHistory: [],
    pruned: 0,
  };
  // Cap: fit only the comparisons among the 400 most recently shown looks.
  const keep = new Set();
  const used = new Set();
  for (let i = records.length - 1; i >= 0; i--) {
    const r = records[i];
    if (r.answer === "bad") { keep.add(i); continue; }
    const ka = lookKey(canonical(sp, r.lookA)) + "|" + r.scene, kb = lookKey(canonical(sp, r.lookB)) + "|" + r.scene;
    const extra = (used.has(ka) ? 0 : 1) + (used.has(kb) || ka === kb ? 0 : 1);
    if (used.size + extra > LOOK_CAP) continue;
    used.add(ka); used.add(kb); keep.add(i);
  }
  state.pruned = records.length - keep.size;
  state.allRecords = records.slice();
  records.forEach((r, i) => { if (keep.has(i)) ingest(r, false); });
  if (state.records.length) {
    fit(state.model);
    fitHyper(state.model, { maxEvals: 60 });
    fit(state.model);
  }
}

function lookId(look, scene) {
  const key = lookKey(look) + "|" + scene;
  if (state.lookIndex.has(key)) return state.lookIndex.get(key);
  if (state.looks.length >= LOOK_CAP) return -1;
  const idx = addLook(state.model, encode(state.space, look), scene);
  state.looks.push({ look, scene });
  state.lookIndex.set(key, idx);
  return idx;
}

function ingest(record, refit) {
  const scene = SCENES.indexOf(record.scene);
  const A = canonical(state.space, record.lookA), B = canonical(state.space, record.lookB);
  state.records.push(record);
  if (record.answer === "bad") {
    state.rejected.push(encode(state.space, A), encode(state.space, B));
    return;
  }
  let a = lookId(A, scene), b = lookId(B, scene);
  if (a < 0 || b < 0) {
    // Cap reached during a session: rebuild from every stored record.
    const all = state.allRecords.slice();
    reset(state.space, all, state.rng.int(2 ** 31));
    return;
  }
  addObservation(state.model, { a, b, outcome: record.answer });
  if (refit) {
    const r = fit(state.model);
    state.answersSinceHyper++;
    if (state.answersSinceHyper >= 10) {
      fitHyper(state.model, { maxEvals: 60 });
      fit(state.model);
      state.answersSinceHyper = 0;
    }
    return r;
  }
}

function bestLook(scene) {
  // The look with the highest posterior mean in this scene; null before any data.
  const idx = state.looks.map((l, i) => (l.scene === scene ? i : -1)).filter((i) => i >= 0);
  if (!idx.length || !state.records.length) return null;
  const { mean } = predict(state.model, idx.map((i) => ({ features: encode(state.space, state.looks[i].look), scene })));
  let best = 0;
  for (let j = 1; j < idx.length; j++) if (mean[j] > mean[best]) best = j;
  return idx[best];
}

function candidates(scene) {
  const best = bestLook(scene);
  return roundCandidates(state.space, feasiblePool(state.space), pairsOf, state.rng, {
    best: best === null ? null : state.looks[best].look, n: CANDIDATES,
  });
}

function propose(sceneName) {
  const scene = SCENES.indexOf(sceneName);
  const looks = candidates(scene);
  if (looks.length < 2) return { type: "nopair", reason: "No look in this region passes the colour checks." };
  const points = looks.map((l) => ({ features: encode(state.space, l), scene }));
  const pick = nextPair(state.model, points, state.rng, { rejected: state.rejected, rejectRadius: 0.1 });
  let A, B;
  if (pick.kind === "repeat") {
    A = state.looks[pick.lookA].look;
    B = state.looks[pick.lookB].look;
  } else {
    A = looks[pick.a];
    B = looks[pick.b];
  }
  if (state.rng.next() < 0.5) [A, B] = [B, A];
  const st = state.records.length ? settled(state.model, points, state.rng, { draws: 200, prob: 0.9 }) : { settled: false, prob: 0 };
  const best = bestLook(scene);
  state.bestHistory.push(best === null ? null : lookKey(state.looks[best].look));
  const recent = state.bestHistory.slice(-15);
  const stable = recent.length === 15 && recent.every((k) => k !== null && k === recent[0]);
  return {
    type: "pair",
    scene: sceneName,
    lookA: A,
    lookB: B,
    kind: pick.kind,
    best: best === null ? null : state.looks[best].look,
    settledProb: st.prob,
    settled: Boolean(st.settled && stable),
    relevance: relevance(),
    counts: { answers: state.records.length, looks: state.looks.length, pruned: state.pruned },
  };
}

function relevance() {
  // Inverse length scale per parameter; circular parameters use the larger of their two features.
  const L = state.model.hyper.lengths;
  const out = [];
  let f = 0;
  for (const p of state.space.params) {
    if (p.kind === "circular") { out.push({ id: p.id, value: 1 / Math.min(L[f], L[f + 1]) }); f += 2; }
    else { out.push({ id: p.id, value: 1 / L[f] }); f += 1; }
  }
  return out;
}

self.onmessage = (e) => {
  const m = e.data;
  try {
    if (m.type === "init") {
      reset(m.space, m.records || [], m.seed || 1);
      self.postMessage(propose(m.scene));
    } else if (m.type === "answer") {
      state.allRecords.push(m.record);
      ingest(m.record, true);
      self.postMessage(propose(m.scene));
    } else if (m.type === "next") {
      self.postMessage(propose(m.scene));
    }
  } catch (err) {
    self.postMessage({ type: "error", message: String(err && err.message ? err.message : err) });
  }
};
