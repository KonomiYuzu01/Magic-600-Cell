import test from "node:test";
import assert from "node:assert/strict";
import { buildGeometry } from "../geometry.js";
import { createLearner, recordFamily, storedHyper, hyperDoc } from "../learner.js";
import { buildPool, classPairsFor } from "../candidates.js";
import { defaultSpace, sobol, fromUnit, encode, featureSlices, withinScene, sceneSpace } from "../../core/space.js";
import { predict } from "../../core/gp.js";

// Evidence: synthetic looks and answers; no stored session data.
const geometry = buildGeometry();
const space = defaultSpace();
const points = sobol(64, space.params.length, 1).map((u) => fromUnit(space, u));
const options = { cap: 12, candidates: 48, hyperEvals: 0, poolDraws: 1024 };
const record = (a, b, answer = "A", scene = "solving") => ({ lookA: points[a], lookB: points[b], scene, answer, t: "", session: "s0" });
const key = (look) => JSON.stringify(look);
// Looks that pass the solving scene's hard checks, as every shown look does. Only such a best
// look joins a round's candidates, where the regret test can assess it.
const feasible = buildPool(sceneSpace(space, "solving"), classPairsFor(geometry), { draws: 2048 });
const shown = (a, b, answer = "A") => ({ ...record(0, 1, answer), lookA: feasible[a], lookB: feasible[b] });

// Posterior mean per shown look, keyed by look, and the observations as look-key pairs.
function snapshot(learner) {
  const { state } = learner;
  const mean = predict(state.model, state.model.looks).mean;
  const means = new Map(state.looks.map((l, i) => [key(l.look) + "|" + l.scene, mean[i]]));
  const obs = state.model._observations.map((o) => [key(state.looks[o.a].look), key(state.looks[o.b].look), o.outcome].join(" ")).sort();
  return { means, obs, looks: state.looks.length, pruned: state.pruned };
}

test("the cap keeps the comparisons among the most recently shown looks", () => {
  const records = Array.from({ length: 10 }, (_, i) => record(2 * i, 2 * i + 1, i % 2 ? "B" : "A"));
  const learner = createLearner(geometry, options);
  learner.reset(space, records);
  const { state } = learner;
  assert.equal(state.looks.length, 12);
  assert.equal(state.pruned, 4);
  assert.equal(state.comparisons, 6);
  assert.deepEqual(new Set(state.looks.map((l) => key(l.look))), new Set(points.slice(8, 20).map(key)));
  const msg = learner.propose("solving");
  assert.equal(msg.type, "pair");
  assert.deepEqual(msg.counts, { answers: 10, used: 6, looks: 12, pruned: 4, skipped: 0 });
});

test("reaching the cap during a session rebuilds the model as a fresh fit of all records", () => {
  // Records 0-4 show ten looks; 5 and 6 add looks and hit the cap; 7-9 reuse shown looks.
  const records = [record(0, 1), record(2, 3, "B"), record(4, 5), record(6, 7, "same"), record(8, 9),
    record(10, 11, "B"), record(12, 13), record(12, 10), record(11, 13, "B"), record(10, 13, "same")];
  const live = createLearner(geometry, options);
  live.reset(space, records.slice(0, 5));
  const rebuilt = records.slice(5).map((r) => live.answer(r).rebuilt);
  assert.deepEqual(rebuilt, [false, true, false, false, false]);
  const fresh = createLearner(geometry, options);
  fresh.reset(space, records);
  const a = snapshot(live), b = snapshot(fresh);
  assert.deepEqual(a.obs, b.obs);
  assert.equal(a.looks, b.looks);
  assert.equal(a.pruned, b.pruned);
  for (const [k, m] of b.means) assert.ok(Math.abs(a.means.get(k) - m) < 1e-8, k);
});

test("records that do not fit the parameter table are skipped and counted", () => {
  const { glow, ...noGlow } = points[1];
  const records = [record(0, 1), { ...record(2, 3), lookB: noGlow }, record(4, 5, "A", "party"), record(6, 7, "maybe"),
    record(8, 8), record(9, 9, "bad")];
  const learner = createLearner(geometry, options);
  learner.reset(space, records);
  assert.equal(learner.state.skipped, 4);
  assert.equal(learner.state.comparisons, 1);
  assert.equal(learner.state.rejected.length, 2);
  learner.answer({ ...record(10, 11), lookA: { ...points[10], hueRotation: "red" } });
  assert.equal(learner.propose("solving").counts.skipped, 5);
  assert.throws(() => learner.propose("party"));
});

test("relevance is the inverse of each parameter's length scale, one per kernel group", () => {
  const learner = createLearner(geometry, options);
  learner.reset(space, [record(0, 1)]);
  const slices = featureSlices(space);
  const L = learner.state.model.hyper.lengths;
  assert.equal(L.length, slices.length);
  const group = (id) => slices.findIndex((s) => s.id === id);
  L[group("hueRotation")] = 0.1;
  L[group("glow")] = 0.25;
  const rel = Object.fromEntries(learner.propose("solving").relevance.map((r) => [r.id, r.value]));
  assert.equal(rel.hueRotation, 10);
  assert.equal(rel.glow, 4);
  assert.equal(rel.gap, 1);
});

test("settled needs the regret test, a stable best in this scene and another session in the same region", () => {
  const always = () => ({ settled: true, regret: 0.01 });
  const make = (settledRecords, session = "s2") => {
    const learner = createLearner(geometry, { ...options, cap: 40, settledTest: always, stableAnswers: 15 });
    learner.reset(space, [shown(0, 1), shown(0, 2)], { session, settledRecords });
    return learner;
  };
  // Look 0 wins every answer, so it stays the best; one answer in another scene does not count.
  const others = Math.min(20, feasible.length - 1);
  const run = (learner, answers) => {
    for (let i = 0; i < answers; i++) {
      learner.answer(shown(0, 1 + (i % others)));
      if (i === 5) learner.answer(record(30, 31, "A", "celebrating"));
    }
    return learner.propose("solving");
  };
  const probe = make([]);
  const near = { scene: "solving", session: "s1", best: { ...feasible[0], glow: feasible[0].glow + 0.01 } };
  assert.ok(probe.sameRegion(feasible[0], near.best), "the near look must lie within one length scale");
  const far = { scene: "solving", session: "s1", best: points.find((p) => !probe.sameRegion(feasible[0], p)) };
  assert.ok(far.best, "some look must lie outside one length scale");
  const early = run(make([near]), 14);
  assert.equal(early.sessionSettled, false);
  assert.equal(early.settled, false);
  const confirmed = run(make([near]), 15);
  assert.deepEqual(confirmed.best, feasible[0]);
  assert.equal(confirmed.sessionSettled, true);
  assert.equal(confirmed.settled, true);
  assert.equal(run(make([far]), 15).settled, false);
  assert.equal(run(make([{ ...near, session: "s2" }]), 15).settled, false);
  assert.equal(run(make([{ ...near, scene: "inspecting" }]), 15).settled, false);
  assert.equal(run(make([]), 15).settled, false);
});

test("a stable best may move within its region but not leave it during the last answers", () => {
  const always = () => ({ settled: true, regret: 0.01 });
  const learner = createLearner(geometry, { ...options, cap: 40, settledTest: always, stableAnswers: 4 });
  learner.reset(space, [shown(0, 1), shown(0, 2)], { session: "s2" });
  const best = learner.best("solving");
  const near = { ...best, glow: best.glow + 0.01 };
  const far = points.find((p) => !learner.sameRegion(best, p));
  assert.ok(learner.sameRegion(best, near) && far);
  const history = learner.state.bestHistory[0];
  history.push(near, best, near, best);
  assert.equal(learner.propose("solving").stable, true, "different looks in one region");
  history.push(best, far, best, best);
  assert.equal(learner.propose("solving").stable, false, "a best outside the region among the last answers");
  history.push(best, best);
  assert.equal(learner.propose("solving").stable, true);
  history.push(null, best, best, best);
  assert.equal(learner.propose("solving").stable, false, "an answer without a best");
});

test("the regret test concerns the reported best look and reuses the round's posterior", () => {
  const seen = [];
  const spy = (model, pts, rng, opts) => { seen.push({ pts, opts }); return { settled: false, regret: 0.5 }; };
  const learner = createLearner(geometry, { ...options, cap: 40, settledTest: spy, tau: 0.07 });
  learner.reset(space, []);
  assert.equal(learner.propose("solving").settledRegret, 1);
  assert.equal(seen.length, 0, "no test before the first comparison");
  learner.reset(space, [shown(0, 1), shown(0, 2), shown(0, 3)]);
  const msg = learner.propose("solving");
  assert.equal(seen.length, 1);
  const { pts, opts } = seen[0];
  assert.deepEqual(Array.from(pts[opts.best].features), Array.from(encode(space, msg.best)));
  assert.equal(opts.prediction.mean.length, pts.length);
  assert.equal(opts.prediction.cov.length, pts.length ** 2);
  assert.equal(opts.tau, 0.07);
  // The reference is the median posterior mean over a fixed pool sample, so it does not
  // depend on the round's candidates.
  assert.ok(Number.isFinite(opts.reference));
  learner.propose("solving");
  assert.equal(seen.length, 2);
  assert.deepEqual(Array.from(seen[1].pts[seen[1].opts.best].features), Array.from(encode(space, msg.best)));
  assert.equal(seen[1].opts.reference, opts.reference);
  assert.equal(msg.settledRegret, 0.5);
});

test("the deferred hyperparameter search proposes first and ends with the synchronous result", () => {
  const records = Array.from({ length: 10 }, (_, i) => record(2 * i, 2 * i + 1, i % 3 ? "A" : "B"));
  const sync = createLearner(geometry, { ...options, cap: 40, hyperEvals: 8 });
  sync.reset(space, records);
  const deferred = createLearner(geometry, { ...options, cap: 40, hyperEvals: 8, deferHyper: true });
  deferred.reset(space, records);
  const start = Array.from(deferred.state.model.hyper.lengths);
  assert.equal(deferred.propose("solving").type, "pair");
  assert.deepEqual(Array.from(deferred.state.model.hyper.lengths), start);
  let steps = 0;
  while (deferred.idle()) steps++;
  assert.equal(steps, 8);
  assert.equal(deferred.idle(), false);
  assert.deepEqual(Array.from(deferred.state.model.hyper.lengths), Array.from(sync.state.model.hyper.lengths));
  const a = snapshot(sync), b = snapshot(deferred);
  for (const [k, m] of a.means) assert.ok(Math.abs(b.means.get(k) - m) < 1e-10, k);
  // An answer during a search: the search finishes on its snapshot and the fit covers the answer.
  const late = createLearner(geometry, { ...options, cap: 40, hyperEvals: 8, deferHyper: true });
  late.reset(space, records);
  late.idle();
  late.answer(record(30, 31));
  while (late.idle());
  assert.deepEqual(Array.from(late.state.model.hyper.lengths), Array.from(sync.state.model.hyper.lengths));
  assert.equal(late.state.model._posterior.revision, late.state.model._revision);
  assert.equal(snapshot(late).obs.length, 11);
});

test("best() reports the proposal's best look, and map fixes parameters of every new candidate", () => {
  const fixed = (look) => ({ ...look, glow: 0.5 });
  const learner = createLearner(geometry, { ...options, cap: 40, map: fixed });
  learner.reset(space, [record(0, 1), record(0, 2), record(0, 3)]);
  assert.equal(learner.best("inspecting"), null);
  assert.throws(() => learner.best("party"));
  const shown = new Set(learner.state.looks.map((l) => key(l.look)));
  for (let i = 0; i < 10; i++) {
    const msg = learner.propose("solving");
    assert.deepEqual(learner.best("solving"), msg.best);
    for (const look of [msg.lookA, msg.lookB]) assert.ok(shown.has(key(look)) || look.glow === 0.5, key(look));
  }
  assert.deepEqual(learner.best("solving"), points[0]);
});

test("each family's learner reads only its own records; a record without a family belongs to the first", () => {
  const records = [record(0, 1), { ...record(2, 3), family: "f2" }, { ...record(4, 5), family: "f9" }, { ...record(6, 7, "B"), family: "f1" }];
  const first = createLearner(geometry, { ...options, cap: 40 });
  first.reset(space, records);
  assert.equal(first.state.family, "f1");
  assert.equal(first.state.comparisons, 2);
  assert.equal(first.state.skipped, 1, "a family the table does not list is skipped");
  const second = createLearner(geometry, { ...options, cap: 40 });
  second.reset(space, records, { family: "f2" });
  assert.equal(second.state.comparisons, 1);
  assert.deepEqual(new Set(second.state.looks.map((l) => key(l.look))), new Set([key(points[2]), key(points[3])]));
  // Another family's answer is neither used nor counted.
  assert.deepEqual(first.answer({ ...record(8, 9), family: "f2" }), { rebuilt: false });
  const msg = first.propose("solving");
  assert.equal(msg.family, "f1");
  assert.deepEqual(msg.counts, { answers: 2, used: 2, looks: 4, pruned: 0, skipped: 1 });
  first.answer({ ...record(10, 11), family: "f1" });
  assert.equal(first.propose("solving").counts.answers, 3);
  assert.throws(() => second.reset(space, records, { family: "f3" }), RangeError);
  // A table without families has one family, the first of the defaults.
  const single = { params: space.params };
  assert.equal(recordFamily(single, record(0, 1)), "f1");
  const only = createLearner(geometry, { ...options, cap: 40 });
  only.reset(single, records);
  assert.equal(only.state.comparisons, 2);
  assert.equal(only.state.skipped, 2);
});

test("in a scene with limits the candidates and the reported best stay within them", () => {
  const outside = { ...points[0], glow: 0.9, turnMs: 800 };
  assert.ok(!withinScene(space, outside, "celebrating") && withinScene(space, outside, "solving"));
  const inside = [1, 2, 3, 4, 5].map((i) => ({ ...points[i], glow: 0.3, turnMs: 400 }));
  // The look outside the limits wins every answer, so it is the posterior best in both scenes.
  const records = ["celebrating", "solving"].flatMap((scene) =>
    inside.map((look) => ({ lookA: outside, lookB: look, scene, answer: "A", t: "", session: "s0" })));
  const learner = createLearner(geometry, { ...options, cap: 40 });
  learner.reset(space, records);
  assert.deepEqual(learner.best("solving"), outside);
  const best = learner.best("celebrating");
  assert.ok(best && withinScene(space, best, "celebrating"), "the reported best obeys the limits");
  for (let i = 0; i < 40; i++) {
    const msg = learner.propose("celebrating");
    assert.equal(msg.type, "pair");
    for (const look of [msg.lookA, msg.lookB]) assert.ok(withinScene(space, look, "celebrating"), key(look));
  }
});

// Hyperparameters that differ from the starting values and from every search result
// below, as a stored document or the model before a rebuild supplies them.
const carriedHyper = () => ({
  lengths: Float64Array.from(featureSlices(space), (_, g) => 0.3 + 0.1 * (g % 5)), rho: 0.9, eps: 0.5, amp: 0.3, beta: 0.1,
});
const sameMeans = (a, b) => {
  const x = snapshot(a), y = snapshot(b);
  assert.deepEqual(y.obs, x.obs);
  for (const [k, m] of x.means) assert.ok(Math.abs(y.means.get(k) - m) < 1e-10, k);
};

test("carried hyperparameters serve until the search started at reset ends, which ends as without them", () => {
  const records = Array.from({ length: 10 }, (_, i) => record(2 * i, 2 * i + 1, i % 3 ? "A" : "B"));
  const opts = { ...options, cap: 40, hyperEvals: 8 };
  const results = [];
  const plain = createLearner(geometry, { ...opts, onSearch: (h) => results.push(h) });
  plain.reset(space, records);
  assert.equal(results.length, 1, "the synchronous search ends inside reset");
  assert.deepEqual(results[0], plain.state.model.hyper);
  const carried = carriedHyper();
  const sync = createLearner(geometry, opts);
  sync.reset(space, records, { hyper: carried });
  assert.deepEqual(sync.state.model.hyper, plain.state.model.hyper);
  sameMeans(plain, sync);
  const found = [];
  const deferred = createLearner(geometry, { ...opts, deferHyper: true, onSearch: (h) => found.push(h) });
  deferred.reset(space, records, { hyper: carried });
  assert.deepEqual(deferred.state.model.hyper, carried);
  const first = deferred.propose("solving");
  assert.deepEqual([first.fitted, first.searched], [true, false]);
  assert.deepEqual(first.relevance.map((r) => r.value), Array.from(carried.lengths, (l) => 1 / l));
  while (deferred.idle());
  assert.equal(found.length, 1);
  assert.deepEqual(deferred.state.model.hyper, plain.state.model.hyper);
  sameMeans(plain, deferred);
  assert.equal(deferred.propose("solving").searched, true);
  // Without carried values, a pair is not fitted before the search ends.
  const cold = createLearner(geometry, { ...opts, deferHyper: true });
  cold.reset(space, records);
  const pair = cold.propose("solving");
  assert.deepEqual([pair.fitted, pair.searched], [false, false]);
  assert.throws(() => createLearner(geometry, opts).reset(space, records, { hyper: { ...carried, rho: 1 } }), RangeError);
});

test("a search replaced before it ends hands its replacement the same start with or without carried values", () => {
  const records = Array.from({ length: 4 }, (_, i) => record(2 * i, 2 * i + 1, i % 2 ? "B" : "A"));
  const scenes = ["solving", "inspecting", "celebrating"];
  const answers = Array.from({ length: 10 }, (_, i) => record(20 + 2 * i, 21 + 2 * i, i % 3 ? "A" : "B", scenes[i % 3]));
  const run = (hyper, steps) => {
    const learner = createLearner(geometry, { ...options, cap: 40, hyperEvals: 12, deferHyper: true });
    learner.reset(space, records, { hyper });
    for (let i = 0; i < steps; i++) learner.idle();
    // The tenth answer starts a new search, which takes over from the pending one.
    for (const r of answers) learner.answer(r);
    assert.equal(learner.propose("solving").searched, false, "the replacement has not ended");
    while (learner.idle());
    assert.equal(learner.propose("solving").searched, true);
    return learner;
  };
  for (const steps of [0, 1, 5]) {
    const cold = run(null, steps), warm = run(carriedHyper(), steps);
    assert.deepEqual(warm.state.model.hyper, cold.state.model.hyper, `${steps} steps before the answers`);
    sameMeans(cold, warm);
  }
});

test("nothing is settled before the search started at the last reset has ended", () => {
  const always = () => ({ settled: true, regret: 0.01 });
  const other = [{ scene: "solving", session: "s1", best: feasible[0] }];
  const make = (records, deferHyper) => {
    const learner = createLearner(geometry, { ...options, cap: 40, settledTest: always, stableAnswers: 3, hyperEvals: 4, deferHyper });
    learner.reset(space, records, { session: "s2", settledRecords: other });
    return learner;
  };
  const flags = (msg) => [msg.stable, msg.searched, msg.sessionSettled, msg.settled];
  // Look 0 wins every answer, so it stays the best.
  const loaded = [shown(0, 4), shown(0, 5)];
  const sync = make(loaded, false);
  for (let i = 1; i <= 3; i++) sync.answer(shown(0, i));
  assert.deepEqual(flags(sync.propose("solving")), [true, true, true, true]);
  const deferred = make(loaded, true);
  for (let i = 1; i <= 3; i++) deferred.answer(shown(0, i));
  const before = deferred.propose("solving");
  assert.deepEqual(flags(before), [true, false, false, false]);
  assert.equal(before.settledRegret, 0.01, "the regret is still reported");
  while (deferred.idle());
  assert.deepEqual(flags(deferred.propose("solving")), [true, true, true, true]);
  // A view without stored comparisons runs its first search after ten answers.
  const fresh = make([], true);
  for (let i = 1; i <= 10; i++) fresh.answer(shown(0, i));
  assert.deepEqual(flags(fresh.propose("solving")), [true, false, false, false]);
  while (fresh.idle());
  assert.deepEqual(flags(fresh.propose("solving")), [true, true, true, true]);
});

test("a rebuild at the cap keeps the hyperparameters until its search ends as a fresh reset's", () => {
  // As in the cap test: record 6 hits the cap and 7-9 reuse shown looks.
  const records = [record(0, 1), record(2, 3, "B"), record(4, 5), record(6, 7, "same"), record(8, 9),
    record(10, 11, "B"), record(12, 13), record(12, 10), record(11, 13, "B"), record(10, 13, "same")];
  const opts = { ...options, hyperEvals: 30 };
  const live = createLearner(geometry, { ...opts, deferHyper: true });
  live.reset(space, records.slice(0, 5));
  while (live.idle());
  assert.equal(live.answer(records[5]).rebuilt, false);
  const kept = live.state.model.hyper;
  const starting = { lengths: new Float64Array(featureSlices(space).length).fill(1), rho: 0.5, eps: 0.2, amp: 0, beta: 0 };
  assert.notDeepEqual(kept, starting, "the search has moved away from the starting values");
  assert.equal(live.answer(records[6]).rebuilt, true);
  assert.deepEqual(live.state.model.hyper, kept);
  for (const r of records.slice(7)) assert.equal(live.answer(r).rebuilt, false);
  const pair = live.propose("solving");
  assert.deepEqual([pair.fitted, pair.searched], [true, false]);
  while (live.idle());
  const fresh = createLearner(geometry, opts);
  fresh.reset(space, records.slice(0, 7));
  for (const r of records.slice(7)) fresh.answer(r);
  assert.deepEqual(live.state.model.hyper, fresh.state.model.hyper);
  sameMeans(fresh, live);
  assert.equal(live.propose("solving").searched, true);
});

test("a rebuild at the cap before any search has ended carries nothing", () => {
  // As in the cap test: record 6 hits the cap; the search after the reset has not run.
  const records = [record(0, 1), record(2, 3, "B"), record(4, 5), record(6, 7, "same"), record(8, 9), record(10, 11, "B"), record(12, 13)];
  const live = createLearner(geometry, { ...options, hyperEvals: 30, deferHyper: true });
  live.reset(space, records.slice(0, 5));
  assert.equal(live.answer(records[5]).rebuilt, false);
  assert.equal(live.answer(records[6]).rebuilt, true);
  const starting = { lengths: new Float64Array(featureSlices(space).length).fill(1), rho: 0.5, eps: 0.2, amp: 0, beta: 0 };
  assert.deepEqual(live.state.model.hyper, starting);
  const pair = live.propose("solving");
  assert.deepEqual([pair.fitted, pair.searched], [false, false], "the starting values are not presented as fitted");
  while (live.idle());
  assert.deepEqual([live.propose("solving").fitted, live.propose("solving").searched], [true, true]);
});

test("stored hyperparameters are used only when complete and stored for this family and parameter table", () => {
  const learner = createLearner(geometry, { ...options, cap: 40, hyperEvals: 8 });
  learner.reset(space, Array.from({ length: 10 }, (_, i) => record(2 * i, 2 * i + 1, i % 3 ? "A" : "B")));
  const hyper = learner.state.model.hyper;
  // Documents pass through JSON, as the store keeps them.
  const doc = JSON.parse(JSON.stringify(hyperDoc(space, "f1", hyper, 10)));
  assert.deepEqual(doc.hyper.lengths, Array.from(hyper.lengths));
  assert.deepEqual(storedHyper(space, doc, "f1"), hyper);
  assert.equal(storedHyper(space, doc, "f2"), null, "another family's document");
  const linear = space.params.findIndex((p) => p.kind === "linear");
  const table = (change) => ({ ...space, params: space.params.map((p, i) => (i === linear ? { ...p, ...change } : p)) });
  assert.deepEqual(storedHyper(table({}), doc, "f1"), hyper, "an unchanged table");
  const p = space.params[linear];
  for (const change of [{ max: p.max + (p.max - p.min) }, { min: p.min - 1 }, { id: "renamed" }, { kind: "integer" }]) {
    assert.equal(storedHyper(table(change), doc, "f1"), null, JSON.stringify(change));
  }
  const swapped = { ...space, params: [space.params[1], space.params[0], ...space.params.slice(2)] };
  assert.equal(storedHyper(swapped, doc, "f1"), null, "another order");
  const h = doc.hyper;
  const malformed = [
    null, [], "doc", { ...doc, version: 2 }, { ...doc, family: undefined }, { ...doc, params: doc.params.slice(1) },
    { ...doc, params: undefined }, { ...doc, hyper: undefined }, { ...doc, hyper: {} }, { ...doc, hyper: [] },
    { ...doc, hyper: { ...h, lengths: h.lengths.slice(1) } }, { ...doc, hyper: { ...h, lengths: { ...h.lengths } } },
    { ...doc, hyper: { ...h, lengths: h.lengths.map(() => 0.01) } }, { ...doc, hyper: { ...h, rho: 1 } },
    { ...doc, hyper: { ...h, eps: "0.2" } },
    ...["lengths", "rho", "eps", "amp", "beta"].flatMap((k) => [{ ...doc, hyper: { ...h, [k]: undefined } }, { ...doc, hyper: { ...h, [k]: null } }]),
  ];
  for (const bad of malformed) assert.equal(storedHyper(space, bad, "f1"), null, JSON.stringify(bad));
});
