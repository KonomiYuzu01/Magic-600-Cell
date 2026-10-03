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
const AMP_GRID = [0, 0.1, 0.3, 1, 3], BETA_GRID = [0, 0.1, 0.3, 1];
// Refinement steps of the length search between grid values (plan section 5.3).
const REFINE = [Math.sqrt(1.5), 1.5 ** 0.25];
const median = (values) => {
  const s = Array.from(values).sort((x, y) => x - y), n = s.length;
  return n % 2 ? s[(n - 1) / 2] : (s[n / 2 - 1] + s[n / 2]) / 2;
};
// One kernel group per parameter: both features of a circular parameter share a length (plan section 5.3).
const groupsOf = (space) => featureSlices(space).map(({ start, count }) => ({ start, count, circular: count === 2 }));

// Feasible pools by geometry and `map`, then by scene table and size: the
// learners of all families share them, and scenes without limits share one pool.
const pools = new WeakMap();

// The family a record belongs to; a record without one belongs to the first family.
export function recordFamily(space, record) {
  if (record && record.family !== undefined) return typeof record.family === "string" ? record.family : null;
  return familiesOf(space)[0].id;
}

// Stored hyperparameters (plan section 7, `model/<family>`) name the parameter
// table they were fitted on: the id, kind and range of every parameter, in order,
// which fix the encoding that the length scales measure.
const tableKey = (space) => space.params.map((p) => `${p.id}:${p.kind}:${p.min}:${p.max}`);
const modelShape = (space) => ({ dim: featureDim(space), scenes: SCENES.length, groups: groupsOf(space) });

// The document that stores one family's hyperparameters after a finished search.
export function hyperDoc(space, family, hyper, answers) {
  return {
    version: 1, family, params: tableKey(space), answers,
    hyper: { lengths: Array.from(hyper.lengths), rho: hyper.rho, eps: hyper.eps, amp: hyper.amp, beta: hyper.beta },
  };
}

// The hyperparameters of a stored document, or null unless the document is this
// family's, on this parameter table, complete and within the model's ranges.
export function storedHyper(space, doc, family) {
  const plain = (x) => x !== null && typeof x === "object" && !Array.isArray(x);
  if (!plain(doc) || doc.version !== 1 || doc.family !== family || !plain(doc.hyper)) return null;
  const key = tableKey(space);
  if (!Array.isArray(doc.params) || doc.params.length !== key.length || doc.params.some((p, i) => p !== key[i])) return null;
  const { lengths, rho, eps, amp, beta } = doc.hyper;
  if (!Array.isArray(lengths) || ![...lengths, rho, eps, amp, beta].every((x) => typeof x === "number")) return null;
  try {
    return createModel({ ...modelShape(space), hyper: { lengths, rho, eps, amp, beta } }).hyper;
  } catch {
    return null;
  }
}

// `settledTest` replaces the regret test of plan section 5.6 in unit tests only.
// `map` fixes parameters of every candidate (the experiment's low-dimensional setting).
// With `deferHyper`, the hyperparameter search runs in idle() steps instead of
// inside reset() and answer(), so that the worker can propose the next pair first.
// `ampGrid` and `betaGrid` are the values the search tries for the weights of the
// kernel's smooth part and of its interactions (plan section 5.3).
// `onSearch(hyper)` is called whenever a search ends with a result.
export function createLearner(geometry, {
  cap = 400, candidates = 512, stableAnswers = 10, tau = 0.05, hyperEvals = 600, poolDraws = 16384,
  settledTest = settled, map = identity, deferHyper = false, ampGrid = AMP_GRID, betaGrid = BETA_GRID,
  anchorRate = 0.5, freePairs = 3000, axis = 4, multi = 24, referenceLooks = 1024, onSearch = null,
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

  // `hyper`, the result of an earlier search (a stored one, or the model's before a
  // rebuild at the cap), serves pairs until the search started here ends; that
  // search still starts from the starting values (plan section 9).
  function reset(space, records, { seed = 1, session = null, settledRecords = [], family = null, hyper = null } = {}) {
    const sp = space || defaultSpace();
    const families = familiesOf(sp);
    const fam = family === null ? families[0].id : family;
    if (!families.some((f) => f.id === fam)) throw new RangeError("family");
    const model = createModel(modelShape(sp));
    const base = model.hyper;
    if (hyper) model.hyper = createModel({ ...modelShape(sp), hyper }).hyper;
    state = {
      space: sp,
      family: fam,
      familyIds: new Set(families.map((f) => f.id)),
      session,
      settledRecords,
      model,
      // Every search starts from `base`: the result of the last search here, else
      // the starting values. Searches therefore never depend on carried values.
      base,
      // True once a search has ended here, or when values were carried in.
      fitted: Boolean(hyper),
      // True once a search started here has ended; the settled rule waits for it.
      searched: !hyperEvals,
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
      // Per scene: the previous round's candidate with the highest posterior mean, shown or not.
      predicted: SCENES.map(() => null),
      reference: new Map(),
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
    if (state.search) handOver(false);
    state.search = createHyperSearch(state.model, { maxEvals: hyperEvals, ampGrid, betaGrid, refine: REFINE, start: state.base });
    if (!deferHyper) while (idle());
  }

  // Applies the best point of the pending search, which has `ended` or is being
  // replaced. Once a search ends, the model holds what it would hold without
  // carried values: the best point found, else `base`.
  function handOver(ended) {
    const search = state.search, found = search.found;
    state.search = null;
    search.adopt();
    if (found) state.base = state.model.hyper;
    if (!ended) return;
    if (state.model.hyper !== state.base) {
      state.model.hyper = state.base;
      fit(state.model);
    }
    state.searched = state.fitted = true;
    if (found && onSearch) onSearch(state.model.hyper);
  }

  // Runs one evaluation of the pending search; true while work remains.
  function idle() {
    if (!state || !state.search) return false;
    if (state.search.step()) return true;
    handOver(true);
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
      // The cap is reached during a session: rebuild from every stored record. Fitted
      // hyperparameters serve until the rebuild's search ends.
      const { bestHistory, predicted, session, settledRecords, family, fitted } = state;
      reset(state.space, state.allRecords, {
        seed: state.rng.int(2 ** 31), session, settledRecords, family, hyper: fitted ? state.model.hyper : null,
      });
      state.bestHistory = bestHistory;
      state.predicted = predicted;
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
    state.bestHistory[parsed.scene].push(best === null ? null : state.looks[best].look);
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

  // Two looks are in the same region when their scaled distance under the fitted
  // length scales, the square root of the sum of ((x_i - x'_i) / l_i)^2 over the
  // encoded features, is at most 1 (plan section 9).
  function sameRegion(a, b) {
    let fa, fb;
    try { fa = encode(state.space, a); fb = encode(state.space, b); } catch { return false; }
    const L = state.model.hyper.lengths;
    let d = 0;
    featureSlices(state.space).forEach(({ start, count }, g) => {
      for (let f = start; f < start + count; f++) d += ((fa[f] - fb[f]) / L[g]) ** 2;
    });
    return d <= 1;
  }

  function relevance() {
    // Inverse length scale per parameter (one length per parameter, plan section 5.3).
    const L = state.model.hyper.lengths;
    return featureSlices(state.space).map(({ id }, g) => ({ id, value: 1 / L[g] }));
  }

  // The median posterior mean over a fixed sample of the scene's feasible pool:
  // the reference of the predicted range in the settled test (plan section 5.6).
  function referenceMean(scene, table) {
    if (!state.reference.has(scene)) {
      const sample = feasiblePool(table).slice(0, referenceLooks);
      state.reference.set(scene, sample.map((l) => ({ features: encode(state.space, l), scene })));
    }
    return median(predictMean(state.model, state.reference.get(scene)));
  }

  function propose(sceneName) {
    const scene = SCENES.indexOf(sceneName);
    if (scene < 0) throw new RangeError("scene");
    const best = bestLook(scene);
    const bestValue = best === null ? null : state.looks[best].look;
    // Candidates come from the scene's table (plan section 4.1); encoding uses the full table.
    // They centre on the predicted optimum and include the best shown look (plan section 5.4).
    const table = sceneSpace(state.space, sceneName);
    const centre = state.predicted[scene] ?? bestValue;
    const looks = roundCandidates(table, feasiblePool(table), pairsOf, state.rng, {
      best: centre, n: candidates, map, include: bestValue ? [bestValue] : [], axis, multi,
    });
    if (looks.length < 2) return { type: "nopair", reason: "No look in this region passes the colour checks." };
    const points = looks.map((l) => ({ features: encode(state.space, l), scene }));
    // One joint posterior over the round's candidates serves the next pair and the settled test.
    const prediction = predict(state.model, points);
    if (state.comparisons) {
      let top = 0;
      for (let i = 1; i < looks.length; i++) if (prediction.mean[i] > prediction.mean[top]) top = i;
      state.predicted[scene] = looks[top];
    }
    const pick = nextPair(state.model, points, state.rng, {
      anchorRate, freePairs, rejected: state.rejected, rejectRadius: 0.1, prediction,
    });
    let A = looks[pick.a], B = looks[pick.b];
    if (state.rng.next() < 0.5) [A, B] = [B, A];
    // Settled (plan section 5.6): the expected regret of the reported best, relative
    // to the predicted range, is at most tau; the best after each of the last
    // answers in this scene lies in the current best's region; and another session
    // settled in the same region. Each test needs fitted length scales, so nothing
    // is settled before a search started at the last reset has ended.
    const bestIndex = bestValue ? looks.findIndex((l) => lookKey(l) === lookKey(bestValue)) : -1;
    const st = state.comparisons && bestIndex >= 0
      ? settledTest(state.model, points, state.rng, {
        draws: 200, tau, prediction, best: bestIndex, reference: referenceMean(scene, table),
      })
      : { settled: false, regret: 1 };
    const recent = state.bestHistory[scene].slice(-stableAnswers);
    const stable = Boolean(bestValue) && recent.length === stableAnswers && recent.every((l) => l !== null && sameRegion(l, bestValue));
    const sessionSettled = Boolean(state.searched && st.settled && stable && bestValue);
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
      settledRegret: st.regret,
      stable,
      sessionSettled,
      settled: confirmed,
      fitted: state.fitted,
      searched: state.searched,
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
