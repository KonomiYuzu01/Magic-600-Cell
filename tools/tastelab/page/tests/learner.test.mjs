import test from "node:test";
import assert from "node:assert/strict";
import { buildGeometry } from "../geometry.js";
import { createLearner, recordFamily } from "../learner.js";
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
