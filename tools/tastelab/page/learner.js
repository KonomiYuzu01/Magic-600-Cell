// Taste Lab learner: the state behind the model worker. It rebuilds the
// preference model of one theme family from stored comparisons within the
// look cap, refits it after each answer and proposes the next pair (plan
// sections 4.5 and 5.2 to 5.6).
import { createRng } from "../core/rng.js";
import { defaultSpace, encode, canonical, featureDim, featureSlices, familiesOf, sceneSpace, withinScene, SCENES } from "../core/space.js";
import { createModel, addLook, addObservation, fit, createHyperSearch, predict, predictMean } from "../core/gp.js";
import { nextPair, settled } from "../core/acquire.js";
import { buildPool, roundCandidates, classPairsFor } from "./candidates.js";

const ANSWERS = ["A", "B", "same", "bad"];
const lookKey = (look) => JSON.stringify(look);
const identity = (l) => l;

// Feasible pools by geometry and `map`, then by scene table and size: the
// learners of all families share them, and scenes without limits share one pool.
const pools = new WeakMap();

// The family a record belongs to; a record without one belongs to the first family.
export function recordFamily(space, record) {
  if (record && record.family !== undefined) return typeof record.family === "string" ? record.family : null;
  return familiesOf(space)[0].id;
}

// `settledTest` replaces the joint-draw test of plan section 5.6 in unit tests only.
// `map` fixes parameters of every candidate (the experiment's low-dimensional setting).
// With `deferHyper`, the hyperparameter search runs in idle() steps instead of
// inside reset() and answer(), so that the worker can propose the next pair first.
export function createLearner(geometry, {
  cap = 400, candidates = 512, stableAnswers = 15, hyperEvals = 60, poolDraws = 16384, settledTest = settled,
  map = identity, deferHyper = false,
} = {}) {
  const pairsOf = classPairsFor(geometry);
  let state = null;

  function feasiblePool(table) {
    if (!pools.has(geometry)) pools.set(geometry, new WeakMap());
    const byMap = pools.get(geometry);
    if (!byMap.has(map)) byMap.set(map, new Map());
    const byKey = byMap.get(map);
    const key = poolDraws + "|" + JSON.stringify(table);
    if (!byKey.has(key)) byKey.set(key, buildPool(table, pairsOf, { draws: poolDraws, map }));
    return byKey.get(key);
  }

  // A record's looks in the current parameter table, or null when the record
  // does not fit it (another table, an unknown scene or answer, equal looks).
  function parse(space, record) {
    const scene = SCENES.indexOf(record && record.scene);
    if (scene < 0 || !ANSWERS.includes(record.answer)) return null;
    try {
      const project = (look) => {
        const c = canonical(space, look);
        return Object.fromEntries(space.params.map((p) => [p.id, c[p.id]]));
      };
      const A = project(record.lookA), B = project(record.lookB);
      if (record.answer !== "bad" && lookKey(A) === lookKey(B)) return null;
      return { scene, A, B };
    } catch {
      return null;
    }
  }

  // Whose record this is: "own", "other" (another family of the table) or "unknown".
  function owner(record) {
    const family = recordFamily(state.space, record);
    if (family === state.family) return "own";
    return state.familyIds.has(family) ? "other" : "unknown";
  }

  function reset(space, records, { seed = 1, session = null, settledRecords = [], family = null } = {}) {
    const sp = space || defaultSpace();
    const families = familiesOf(sp);
    const fam = family === null ? families[0].id : family;
    if (!families.some((f) => f.id === fam)) throw new RangeError("family");
    state = {
      space: sp,
      family: fam,
      familyIds: new Set(families.map((f) => f.id)),
      session,
      settledRecords,
      model: createModel({ dim: featureDim(sp), scenes: SCENES.length }),
      allRecords: records.slice(),
      answers: 0,
      used: 0,
      comparisons: 0,
      looks: [],
      lookIndex: new Map(),
      rejected: [],
      rng: createRng(seed >>> 0),
      answersSinceHyper: 0,
      search: null,
      bestHistory: SCENES.map(() => []),
      pruned: 0,
      skipped: 0,
    };
    // Another family's records are neither used nor skipped; a record of a
    // family the table does not list does not fit it and is skipped.
    const whose = records.map(owner);
    const parsed = records.map((r, i) => (whose[i] === "own" ? parse(sp, r) : null));
    state.answers = whose.filter((w) => w === "own").length;
    // Cap: fit only the comparisons among the most recently shown looks.
    const keep = new Set();
    const shown = new Set();
    for (let i = records.length - 1; i >= 0; i--) {
      if (whose[i] === "other") continue;
      const p = parsed[i];
      if (!p) { state.skipped++; continue; }
      if (records[i].answer === "bad") { keep.add(i); continue; }
      const ka = lookKey(p.A) + "|" + p.scene, kb = lookKey(p.B) + "|" + p.scene;
      const extra = (shown.has(ka) ? 0 : 1) + (shown.has(kb) ? 0 : 1);
      if (shown.size + extra > cap) { state.pruned++; continue; }
      shown.add(ka); shown.add(kb); keep.add(i);
    }
    records.forEach((r, i) => { if (keep.has(i)) ingest(r, parsed[i]); });
    if (state.comparisons) {
      fit(state.model);
      refitHyper();
    }
  }

  // Starts the hyperparameter search of plan section 5.3. A search still
  // running hands over its best point so far, so its work is not lost.
  function refitHyper() {
    if (!hyperEvals) return;
    if (state.search) state.search.adopt();
    state.search = createHyperSearch(state.model, { maxEvals: hyperEvals });
    if (!deferHyper) while (idle());
  }

  // Runs one evaluation of the pending search; true while work remains.
  function idle() {
    if (!state || !state.search) return false;
    if (state.search.step()) return true;
    state.search.adopt();
    state.search = null;
    return false;
  }

  function lookId(look, scene) {
    const key = lookKey(look) + "|" + scene;
    if (state.lookIndex.has(key)) return state.lookIndex.get(key);
    if (state.looks.length >= cap) return -1;
    const idx = addLook(state.model, encode(state.space, look), scene);
    state.looks.push({ look, scene });
    state.lookIndex.set(key, idx);
    return idx;
  }

  // Adds one parsed record to the model without refitting; false at the cap.
  function ingest(record, parsed) {
    if (record.answer === "bad") {
      state.rejected.push(encode(state.space, parsed.A), encode(state.space, parsed.B));
      state.used++;
      return true;
    }
    const a = lookId(parsed.A, parsed.scene), b = lookId(parsed.B, parsed.scene);
    if (a < 0 || b < 0) return false;
    addObservation(state.model, { a, b, outcome: record.answer });
    state.used++;
    state.comparisons++;
    return true;
  }

  function answer(record) {
    state.allRecords.push(record);
    const whose = owner(record);
    if (whose === "other") return { rebuilt: false };
    if (whose === "own") state.answers++;
    const parsed = whose === "own" ? parse(state.space, record) : null;
    if (!parsed) { state.skipped++; return { rebuilt: false }; }
    let rebuilt = false;
    if (!ingest(record, parsed)) {
      // The cap is reached during a session: rebuild from every stored record.
      const { bestHistory, session, settledRecords, family } = state;
      reset(state.space, state.allRecords, { seed: state.rng.int(2 ** 31), session, settledRecords, family });
      state.bestHistory = bestHistory;
      rebuilt = true;
    } else if (record.answer !== "bad") {
      fit(state.model);
      if (++state.answersSinceHyper >= 10) {
        refitHyper();
        state.answersSinceHyper = 0;
      }
    }
    // Stability counts answers per scene, not proposals or scene changes.
    const best = bestLook(parsed.scene);
    state.bestHistory[parsed.scene].push(best === null ? null : lookKey(state.looks[best].look));
    return { rebuilt };
  }

  function bestLook(scene) {
    // The shown look with the highest posterior mean in this scene, within the
    // scene's limits; null before any comparison.
    const idx = [];
    state.looks.forEach((l, i) => { if (l.scene === scene && withinScene(state.space, l.look, SCENES[scene])) idx.push(i); });
    if (!idx.length || !state.comparisons) return null;
    const mean = predictMean(state.model, idx.map((i) => state.model.looks[i]));
    let best = 0;
    for (let j = 1; j < idx.length; j++) if (mean[j] > mean[best]) best = j;
    return idx[best];
  }

  // Two looks are in the same region when their kernel correlation under the
  // fitted length scales is at least exp(-1/2), that is, a scaled distance of at most 1.
  function sameRegion(a, b) {
    let fa, fb;
    try { fa = encode(state.space, a); fb = encode(state.space, b); } catch { return false; }
    const L = state.model.hyper.lengths;
    let d = 0;
    for (let i = 0; i < L.length; i++) d += ((fa[i] - fb[i]) / L[i]) ** 2;
    return d <= 1;
  }

  function relevance() {
    // Inverse length scale per parameter; a circular parameter uses its shorter feature length.
    const L = state.model.hyper.lengths;
    return featureSlices(state.space).map(({ id, start, count }) => {
      let shortest = Infinity;
      for (let f = start; f < start + count; f++) shortest = Math.min(shortest, L[f]);
      return { id, value: 1 / shortest };
    });
  }

  function propose(sceneName) {
    const scene = SCENES.indexOf(sceneName);
    if (scene < 0) throw new RangeError("scene");
    const best = bestLook(scene);
    const bestValue = best === null ? null : state.looks[best].look;
    // Candidates come from the scene's table (plan section 4.1); encoding uses the full table.
    const table = sceneSpace(state.space, sceneName);
    const looks = roundCandidates(table, feasiblePool(table), pairsOf, state.rng, { best: bestValue, n: candidates, map });
    if (looks.length < 2) return { type: "nopair", reason: "No look in this region passes the colour checks." };
    const points = looks.map((l) => ({ features: encode(state.space, l), scene }));
    // One joint posterior over the round's candidates serves the next pair and the settled test.
    const prediction = predict(state.model, points);
    const options = { rejected: state.rejected, rejectRadius: 0.1, prediction };
    let pick = nextPair(state.model, points, state.rng, options);
    // A repeat shows a stored pair again; one outside the scene's limits (stored
    // before the limits changed) gives way to a new pair.
    if (pick.kind === "repeat" && ![pick.lookA, pick.lookB].every((i) => withinScene(state.space, state.looks[i].look, sceneName))) {
      pick = nextPair(state.model, points, state.rng, { ...options, repeatRate: 0 });
    }
    let A, B;
    if (pick.kind === "repeat") {
      A = state.looks[pick.lookA].look;
      B = state.looks[pick.lookB].look;
    } else {
      A = looks[pick.a];
      B = looks[pick.b];
    }
    if (state.rng.next() < 0.5) [A, B] = [B, A];
    // Settled (plan section 5.6): the reported best beats every candidate of the
    // round in the joint draws (roundCandidates puts it first), it is unchanged for
    // the last answers in this scene, and another session settled in the same region.
    const bestIndex = bestValue ? looks.findIndex((l) => lookKey(l) === lookKey(bestValue)) : -1;
    const st = state.comparisons && bestIndex >= 0
      ? settledTest(state.model, points, state.rng, { draws: 200, prob: 0.9, prediction, best: bestIndex })
      : { settled: false, prob: 0 };
    const recent = state.bestHistory[scene].slice(-stableAnswers);
    const stable = recent.length === stableAnswers && recent.every((k) => k !== null && k === recent[0]);
    const sessionSettled = Boolean(st.settled && stable && bestValue);
    const confirmed = sessionSettled && state.settledRecords.some((r) =>
      r && r.scene === sceneName && recordFamily(state.space, r) === state.family &&
      r.session !== state.session && sameRegion(r.best, bestValue));
    return {
      type: "pair",
      family: state.family,
      scene: sceneName,
      lookA: A,
      lookB: B,
      kind: pick.kind,
      best: bestValue,
      settledProb: st.prob,
      sessionSettled,
      settled: confirmed,
      relevance: relevance(),
      counts: { answers: state.answers, used: state.used, looks: state.looks.length, pruned: state.pruned, skipped: state.skipped },
    };
  }

  // The best shown look of a scene, or null before any comparison in it.
  function best(sceneName) {
    const scene = SCENES.indexOf(sceneName);
    if (scene < 0) throw new RangeError("scene");
    const idx = bestLook(scene);
    return idx === null ? null : state.looks[idx].look;
  }

  return {
    reset,
    answer,
    propose,
    best,
    idle,
    sameRegion: (a, b) => sameRegion(a, b),
    get state() { return state; },
  };
}
