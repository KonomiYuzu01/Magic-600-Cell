import test from 'node:test';
import assert from 'node:assert/strict';
import {createModel, addLook, addObservation, observations, removeLast, fit, fitHyper,
  logEvidence, predict, sampleJoint, createJointSampler, serialize, deserialize,
  logPhi, outcomeProbabilities, likelihoodTerms} from '../gp.js';
import {defaultSpace, encode} from '../space.js';
import {createRng} from '../rng.js';

const close = (a, b, tolerance = 1e-8) => {
  assert.equal(a.length, b.length);
  a.forEach((value, i) => assert.ok(Math.abs(value - b[i]) <= tolerance, `${i}: ${value} != ${b[i]}`));
};
const outcome = (d, eps = 0.2) => Math.abs(d) < eps ? 'same' : d > 0 ? 'A' : 'B';
function randomModel(seed = 7, count = 30) {
  const rng = createRng(seed);
  const model = createModel({dim: 2});
  for (let i = 0; i < 14; i++) addLook(model, [rng.next(), rng.next()], i % 3);
  const utility = i => 3 * Math.sin(model.looks[i].features[0] * Math.PI) - model.looks[i].features[1];
  for (let i = 0; i < count; i++) {
    const a = rng.int(14);
    let b;
    do b = rng.int(14); while (a === b);
    addObservation(model, {a, b, outcome: outcome(utility(a) - utility(b) + Math.SQRT2 * rng.normal())});
  }
  return model;
}

test('normal CDF references, normalized symmetric outcomes and finite tails', () => {
  assert.ok(Math.abs(Math.exp(logPhi(-1)) - 0.15865525393145707) < 1e-15);
  assert.ok(Math.abs(Math.exp(logPhi(-8)) / 6.22096057427178e-16 - 1) < 1e-13);
  assert.ok(Math.abs(logPhi(-40) - (-804.6084420137539)) < 1e-10);
  for (const eps of [0.01, 0.2, 2]) for (const d of [-100, -4, -1, 0, 1, 4, 100]) {
    const p = outcomeProbabilities(d, eps), reverse = outcomeProbabilities(-d, eps);
    assert.ok(Math.abs(p.A + p.B + p.same - 1) < 1e-14);
    assert.ok(Math.abs(p.A - reverse.B) < 1e-14);
    assert.ok(Math.abs(p.same - reverse.same) < 1e-14);
    for (const answer of ['A', 'B', 'same']) {
      const terms = likelihoodTerms(d, eps, answer);
      assert.ok(Object.values(terms).every(Number.isFinite));
      assert.ok(terms.hessian <= 1e-8);
    }
  }
  assert.throws(() => likelihoodTerms(0, 0.2, 'bad'));
});

test('finite differences verify all three likelihood gradients and Hessians', () => {
  const step = 1e-4;
  for (const eps of [0.01, 0.2, 2]) for (const d of [-40, -4, -0.4, 0, 0.4, 4, 40]) {
    for (const answer of ['A', 'B', 'same']) {
      const term = likelihoodTerms(d, eps, answer);
      const plus = likelihoodTerms(d + step, eps, answer);
      const minus = likelihoodTerms(d - step, eps, answer);
      const gradient = (plus.logp - minus.logp) / (2 * step);
      const hessian = (plus.gradient - minus.gradient) / (2 * step);
      assert.ok(Math.abs(gradient - term.gradient) < 2e-6, `${answer}, d=${d}, eps=${eps}, gradient`);
      assert.ok(Math.abs(hessian - term.hessian) < 2e-5, `${answer}, d=${d}, eps=${eps}, hessian`);
    }
  }
});

test('kernel has the specified amplitude, ARD lengths and scene coupling', () => {
  const model = createModel({dim: 2, hyper: {signal: 2, lengths: new Float64Array([0.25, 0.75]), rho: 0.4}});
  const points = [{features: [0, 0], scene: 'solving'}, {features: [0, 0], scene: 1},
    {features: [0.25, 0.75], scene: 0}];
  const result = predict(model, points);
  close(result.mean, [0, 0, 0]);
  close(result.cov, [4, 1.6, 4 / Math.E, 1.6, 4, 1.6 / Math.E, 4 / Math.E, 1.6 / Math.E, 4]);
  const expectedPrior = Math.log(2) + Math.log(5) + Math.log(4) - 4 * 0.5 * Math.log(2 * Math.PI);
  assert.ok(Math.abs(logEvidence(createModel({dim: 1})) - expectedPrior) < 1e-12);
});

test('Newton converges on 30 random noisy comparisons', () => {
  const model = randomModel();
  const result = fit(model);
  assert.equal(result.converged, true);
  assert.ok(result.iterations > 0 && result.iterations <= 30);
  const prediction = predict(model, model.looks);
  assert.ok(prediction.mean.every(Number.isFinite));
  assert.ok(prediction.cov.every(Number.isFinite));
  for (let i = 0; i < 14; i++) {
    assert.ok(prediction.cov[i * 14 + i] >= 0);
    for (let j = 0; j < 14; j++) assert.equal(prediction.cov[i * 14 + j], prediction.cov[j * 14 + i]);
  }
  assert.ok(Number.isFinite(logEvidence(model)));
});

test('undo and refit equal a fresh fit without that observation', () => {
  const model = randomModel(17, 31);
  assert.equal(fit(model).converged, true);
  const last = observations(model).at(-1);
  assert.deepEqual(removeLast(model), last);
  assert.equal(fit(model).converged, true);
  const fresh = createModel({dim: model.dim});
  for (const p of model.looks) addLook(fresh, p.features, p.scene);
  for (const o of observations(model)) addObservation(fresh, o);
  assert.equal(fit(fresh).converged, true);
  const points = [...model.looks, {features: [0.3, 0.7], scene: 1}];
  const a = predict(model, points), b = predict(fresh, points);
  close(a.mean, b.mean);
  close(a.cov, b.cov);
  const copy = observations(model);
  copy[0].outcome = 'bad';
  assert.notEqual(observations(model)[0].outcome, 'bad');
});

test('all ties and duplicate features fit without NaN', () => {
  const model = createModel({dim: 1});
  for (const x of [0, 0, 0.25, 0.5, 0.75, 1]) addLook(model, [x], 0);
  for (let i = 0; i < 100; i++) addObservation(model, {a: i % 5, b: 5, outcome: 'same'});
  assert.equal(fit(model).converged, true);
  const p = predict(model, model.looks);
  assert.ok(p.mean.every(x => x === 0));
  assert.ok(p.cov.every(Number.isFinite));
  assert.ok(Number.isFinite(logEvidence(model)));
  assert.ok(sampleJoint(model, model.looks, createRng(2)).every(Number.isFinite));
});

test('non-convergence keeps the entire previous posterior, including after adding a look', () => {
  const model = randomModel();
  assert.equal(fit(model).converged, true);
  const points = [{features: [0.2, 0.4], scene: 0}, {features: [0.8, 0.1], scene: 2}];
  const before = predict(model, points);
  const newLook = addLook(model, [0.5, 0.5], 1);
  addObservation(model, {a: newLook, b: 0, outcome: 'A'});
  assert.deepEqual(fit(model, {maxIter: 0}), {converged: false, iterations: 0});
  assert.deepEqual(predict(model, points), before);
  assert.deepEqual(predict(deserialize(serialize(model)), points), before);
});

test('one-dimensional noisy utility locates the optimum for most of ten seeds', () => {
  const optimum = 0.62;
  let successes = 0;
  for (let seed = 0; seed < 10; seed++) {
    const rng = createRng(seed), model = createModel({dim: 1});
    for (let i = 0; i <= 20; i++) addLook(model, [i / 20], 0);
    const utility = x => -20 * (x - optimum) ** 2;
    for (let i = 0; i < 40; i++) {
      const a = rng.int(21);
      let b;
      do b = rng.int(21); while (a === b);
      addObservation(model, {a, b, outcome: outcome(utility(a / 20) - utility(b / 20) + Math.SQRT2 * rng.normal())});
    }
    assert.equal(fit(model).converged, true, `seed ${seed}`);
    const grid = Array.from({length: 101}, (_, i) => ({features: [i / 100], scene: 0}));
    const {mean} = predict(model, grid);
    const best = mean.indexOf(Math.max(...mean)) / 100;
    if (Math.abs(best - optimum) <= 0.1 + 1e-12) successes++;
  }
  assert.ok(successes >= 7, `${successes}/10 seeds found the optimum within 0.1`);
});

test('circular encoding gives unchanged predictions after a full turn', () => {
  const space = defaultSpace();
  const look = Object.fromEntries(space.params.map(p => [p.id, (p.min + p.max) / 2]));
  const a = {...look, hueRotation: 20}, b = {...look, hueRotation: 160};
  const model = createModel({dim: encode(space, a).length});
  addLook(model, encode(space, a), 0);
  addLook(model, encode(space, b), 0);
  for (let i = 0; i < 20; i++) addObservation(model, {a: 0, b: 1, outcome: 'A'});
  assert.equal(fit(model).converged, true);
  const point = rotation => ({features: encode(space, {...a, hueRotation: rotation}), scene: 0});
  assert.deepEqual(predict(model, [point(20)]), predict(model, [point(380)]));
});

test('hyper fitting learns smaller rho for opposing scenes and larger rho for shared preferences', () => {
  const fitted = [];
  for (const opposite of [false, true]) {
    const rng = createRng(42), model = createModel({dim: 1, scenes: 2, hyper: {signal: 3}});
    for (let scene = 0; scene < 2; scene++) for (let i = 0; i < 5; i++) addLook(model, [i / 4], scene);
    for (let i = 0; i < 160; i++) {
      const scene = i % 2, a = rng.int(5);
      let b;
      do b = rng.int(5); while (a === b);
      const difference = 6 * (a - b) / 4 * (opposite && scene === 1 ? -1 : 1) + Math.SQRT2 * rng.normal();
      addObservation(model, {a: scene * 5 + a, b: scene * 5 + b, outcome: outcome(difference)});
    }
    assert.equal(fit(model).converged, true);
    const before = logEvidence(model);
    const evaluations = fitHyper(model);
    assert.ok(evaluations > 0 && evaluations <= 60);
    assert.ok(logEvidence(model) >= before);
    assert.ok(model.hyper.lengths.every(x => x >= 0.05 && x <= 5));
    assert.ok(model.hyper.eps >= 0.01 && model.hyper.eps <= 2);
    fitted.push(model.hyper.rho);
    const saved = serialize(model);
    assert.equal(fitHyper(model, {maxEvals: 0}), 0);
    assert.equal(serialize(model), saved);
  }
  assert.ok(fitted[0] > 0.65, `shared rho ${fitted[0]}`);
  assert.ok(fitted[1] < 0.35, `opposite rho ${fitted[1]}`);
});

test('joint draws reproduce posterior covariance and serialized models preserve predictions', () => {
  const model = randomModel();
  assert.equal(fit(model).converged, true);
  const points = [{features: [0.25, 0.25], scene: 0}, {features: [0.6, 0.4], scene: 0}];
  const prediction = predict(model, points);
  const draw = createJointSampler(prediction), rng = createRng(99);
  const sum = [0, 0], product = [0, 0, 0];
  for (let i = 0; i < 30000; i++) {
    const sample = draw(rng);
    sum[0] += sample[0]; sum[1] += sample[1];
    product[0] += sample[0] ** 2; product[1] += sample[0] * sample[1]; product[2] += sample[1] ** 2;
  }
  const mean = sum.map(x => x / 30000);
  close(mean, prediction.mean, 0.02);
  close([product[0] / 30000 - mean[0] ** 2, product[1] / 30000 - mean[0] * mean[1],
    product[2] / 30000 - mean[1] ** 2], [prediction.cov[0], prediction.cov[1], prediction.cov[3]], 0.02);
  const json = serialize(model);
  assert.deepEqual(predict(deserialize(json), points), prediction);
  assert.deepEqual(predict(deserialize(JSON.parse(json)), points), prediction);
  assert.deepEqual(observations(deserialize(json)), observations(model));
  assert.throws(() => deserialize('{"version":2}'));
});

test('look capacity and input checks', () => {
  const model = createModel({dim: 1});
  const features = new Float64Array([0.1]);
  addLook(model, features, 0);
  features[0] = 7;
  assert.equal(model.looks[0].features[0], 0.1);
  for (let i = 1; i < 400; i++) assert.equal(addLook(model, [i / 400], 0), i);
  assert.throws(() => addLook(model, [0], 0), /capacity/);
  assert.equal(model.looks.length, 400);
  assert.throws(() => addObservation(model, {a: 0, b: 400, outcome: 'A'}));
  assert.throws(() => addObservation(model, {a: 0, b: 1, outcome: 'bad'}));
  assert.throws(() => createModel({dim: 1, hyper: {rho: 1}}));
  assert.throws(() => predict(model, [{features: [NaN], scene: 0}]));
  const empty = createModel({dim: 1});
  assert.equal(removeLast(empty), undefined);
  assert.throws(() => addLook(empty, [0], 3));
  assert.throws(() => addLook(empty, [0, 1], 0));
});
