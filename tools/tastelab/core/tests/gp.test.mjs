import test from 'node:test';
import assert from 'node:assert/strict';
import {createModel, addLook, addObservation, observations, removeLast, fit, fitHyper, createHyperSearch,
  logEvidence, hyperPrior, predict, predictMean, sampleJoint, createJointSampler, serialize, deserialize,
  logPhi, outcomeProbabilities, likelihoodTerms} from '../gp.js';
import {defaultSpace, encode, featureSlices, featureDim} from '../space.js';
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

const mixedGroups = [
  {start: 0, count: 1, circular: false}, {start: 1, count: 2, circular: true},
  {start: 3, count: 1, circular: false}, {start: 4, count: 2, circular: true},
];
function randomPoint(rng, groups = mixedGroups, scene = rng.int(3)) {
  return {features: groups.flatMap(group => {
    const h = 2 * Math.PI * rng.next();
    return group.circular ? [Math.cos(h) / (2 * Math.PI), Math.sin(h) / (2 * Math.PI)] : [rng.next()];
  }), scene};
}
function referenceKernel(a, b, groups, hyper) {
  const terms = groups.map(({start, count, circular}, g) => {
    const indices = Array.from({length: count}, (_, i) => start + i);
    const centre = circular ? 0 : 0.5;
    const dot = indices.reduce((sum, d) => sum + (a.features[d] - centre) * (b.features[d] - centre), 0);
    const distance = indices.reduce((sum, d) => sum + (a.features[d] - b.features[d]) ** 2, 0);
    const s = dot / hyper.lengths[g] ** 2;
    return {s, quadratic: s ** 2, smooth: Math.exp(-distance / (2 * hyper.lengths[g] ** 2))};
  });
  const total = terms.reduce((sum, term) => sum + term.s, 0);
  const additive = terms.reduce((sum, term) => sum + 2 * term.s + hyper.amp * term.smooth, 0);
  const quadratic = terms.reduce((sum, term) => sum + term.quadratic, 0);
  return (additive + (1 - hyper.beta) * quadratic + hyper.beta * total ** 2) * (a.scene === b.scene ? 1 : hyper.rho);
}
function preferenceFixture(seed, saturation) {
  const rng = createRng(seed), model = createModel({dim: 6, scenes: 1});
  for (let i = 0; i < 64; i++) addLook(model, Array.from({length: 6}, () => rng.next()), 0);
  const utility = i => [0, 1].reduce((sum, p) => {
    const d = model.looks[i].features[p] - [0.35, 0.65][p];
    return sum - (saturation ? 1 - Math.exp(-d * d / (2 * 0.2 ** 2)) : 20 * d * d);
  }, 0);
  for (let i = 0; i < 300; i++) {
    const a = rng.int(64);
    let b;
    do b = rng.int(64); while (a === b);
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

test('mixed-group kernel equals an independent formula on random pairs in both scene cases', () => {
  const rng = createRng(20261003);
  for (const hyper of [
    {lengths: [0.05, 0.3, 1.1, 50], rho: 0.2, amp: 0, beta: 0},
    {lengths: [0.5, 2, 4, 0.1], rho: 0.4, amp: 0.3, beta: 0.3},
    {lengths: [1, 1, 0.75, 2.5], rho: 0.9, amp: 3, beta: 1},
    {lengths: [4, 0.05, 16, 0.33], rho: 0.5, amp: 10, beta: 0.1},
  ]) {
    const model = createModel({dim: 6, groups: mixedGroups, hyper});
    for (let i = 0; i < 50; i++) {
      const a = randomPoint(rng, mixedGroups, 0), b = randomPoint(rng, mixedGroups, 0);
      for (const scene of [0, 1]) {
        b.scene = scene;
        const prediction = predict(model, [a, b]);
        const expected = referenceKernel(a, b, mixedGroups, hyper);
        close(prediction.mean, [0, 0]);
        close([prediction.cov[1]], [expected], 1e-10 * Math.max(1, Math.abs(expected)));
      }
    }
  }
});

test('amp and beta zero give the unnormalized additive quadratic kernel', () => {
  const rng = createRng(20);
  const model = createModel({dim: 6, groups: mixedGroups, hyper: {lengths: [0.33, 0.75, 1.7, 4], rho: 0.4}});
  for (let i = 0; i < 50; i++) {
    const a = randomPoint(rng), b = randomPoint(rng);
    const expected = mixedGroups.reduce((sum, group, g) => {
      const s = group.circular
        ? (a.features[group.start] * b.features[group.start] + a.features[group.start + 1] * b.features[group.start + 1]) / model.hyper.lengths[g] ** 2
        : (a.features[group.start] - 0.5) * (b.features[group.start] - 0.5) / model.hyper.lengths[g] ** 2;
      return sum + 2 * s + s ** 2;
    }, 0) * (a.scene === b.scene ? 1 : model.hyper.rho);
    close([predict(model, [a, b]).cov[1]], [expected]);
  }
});

test('60-point prior covariances factor for every smooth and interaction weight', () => {
  const rng = createRng(60), points = Array.from({length: 60}, () => randomPoint(rng));
  for (const amp of [0, 0.1, 0.3, 1, 3]) for (const beta of [0, 0.1, 0.3, 1]) {
    const model = createModel({dim: 6, groups: mixedGroups, hyper: {lengths: [0.22, 0.5, 1.7, 4], amp, beta}});
    const prediction = predict(model, points);
    assert.ok(prediction.cov.every(Number.isFinite));
    // The existing Cholesky jitter also factors the finite-rank additive case.
    assert.ok(createJointSampler(prediction)(rng).every(Number.isFinite));
  }
});

test('a length of 50 switches a linear group off across its unit interval', () => {
  const rng = createRng(50);
  for (const amp of [0, 0.1, 0.3, 1, 3]) for (const beta of [0, 0.1, 0.3, 1]) {
    const model = createModel({dim: 6, groups: mixedGroups, hyper: {lengths: [50, 0.5, 0.75, 1.1], amp, beta}});
    for (let i = 0; i < 10; i++) {
      const a = randomPoint(rng), b = randomPoint(rng);
      a.features[0] = 0.5;
      const baseline = predict(model, [a, b]).cov[1];
      for (let step = 0; step <= 20; step++) {
        a.features[0] = step / 20;
        assert.ok(Math.abs(predict(model, [a, b]).cov[1] - baseline) < 1e-2);
      }
    }
  }
});

test('without observations evidence equals the length, rho and epsilon prior', () => {
  const defaultModel = createModel({dim: 1});
  const expectedDefault = Math.log(20) - 1 / 3 - 0.5 * Math.log(3) - 3 * 0.5 * Math.log(2 * Math.PI);
  close([hyperPrior(defaultModel.hyper), logEvidence(defaultModel)], [expectedDefault, expectedDefault], 1e-12);
  const model = createModel({dim: 6, groups: mixedGroups,
    hyper: {lengths: [0.1, 0.75, 2.5, 50], rho: 0.4, eps: 0.3, amp: 0.3, beta: 0.1}});
  const h = model.hyper, logDensityConstant = 0.5 * Math.log(2 * Math.PI);
  const logMedian = Math.SQRT2 + 0.5 * Math.log(model.dim);
  const lengthsPrior = Array.from(h.lengths).reduce((sum, length) => sum - Math.log(length)
    - 0.5 * ((Math.log(length) - logMedian) / Math.sqrt(3)) ** 2 - Math.log(Math.sqrt(3)) - logDensityConstant, 0);
  const logit = Math.log(h.rho / (1 - h.rho));
  const expected = lengthsPrior - 0.5 * logit ** 2 - Math.log(h.rho * (1 - h.rho)) - logDensityConstant
    - Math.log(h.eps) - 0.5 * Math.log(h.eps / 0.2) ** 2 - logDensityConstant;
  close([hyperPrior(h, model.dim), logEvidence(model)], [expected, expected], 1e-12);
  const rng = createRng(9);
  for (let i = 0; i < 10; i++) {
    const p = randomPoint(rng);
    addLook(model, p.features, p.scene);
  }
  close([logEvidence(model)], [expected], 1e-12);
  assert.equal(hyperPrior({...h, amp: 10, beta: 1}, model.dim), hyperPrior(h, model.dim));
});

test('groups cover the features in order, are copied, and default to linear groups', () => {
  const groups = mixedGroups.map(g => ({...g}));
  const model = createModel({dim: 6, groups});
  assert.deepEqual(model.groups, mixedGroups);
  groups[0].start = 1;
  groups.pop();
  assert.deepEqual(model.groups, mixedGroups);
  const linear = Array.from({length: 3}, (_, start) => ({start, count: 1, circular: false}));
  const implicit = createModel({dim: 3}), explicit = createModel({dim: 3, groups: linear});
  assert.deepEqual(implicit.groups, linear);
  assert.equal(serialize(implicit), serialize(explicit));
  for (const invalid of [
    [], {}, [null], [{start: 1, count: 1, circular: false}],
    [{start: 0, count: 1, circular: false}, {start: 2, count: 2, circular: true}],
    [{start: 0, count: 2, circular: true}, {start: 1, count: 1, circular: false}],
    [{start: 0, count: 1, circular: true}], [{start: 0, count: 2, circular: false}],
    [{start: 0, count: 3, circular: false}], [{start: 0, count: 1}],
    [{start: 0, count: 1, circular: 'false'}],
    [{start: 0, count: 2, circular: true}],
    Array.from({length: 4}, (_, start) => ({start, count: 1, circular: false})),
  ]) assert.throws(() => createModel({dim: 3, groups: invalid}), RangeError);
});

test('hyperparameters are per group, bounded, copied, and ignore signal', () => {
  const model = createModel({dim: 6, groups: mixedGroups, hyper: {signal: NaN}});
  assert.deepEqual(model.hyper, {lengths: new Float64Array([1, 1, 1, 1]), rho: 0.5, eps: 0.2, amp: 0, beta: 0});
  const lengths = [0.05, 50, 1, 1];
  const bounded = createModel({dim: 6, groups: mixedGroups, hyper: {lengths, rho: 0.1, eps: 2, amp: 10, beta: 1}});
  lengths[0] = 1;
  assert.equal(bounded.hyper.lengths[0], 0.05);
  for (const [key, values] of [
    ['lengths', [[], [1, 1, 1, 1, 1, 1], [0.049, 1, 1, 1], [50.01, 1, 1, 1], [NaN, 1, 1, 1], ['1', 1, 1, 1]]],
    ['rho', [0, 1, -0.1, NaN, Infinity, '0.5']], ['eps', [0.009, 2.01, NaN, Infinity]],
    ['amp', [-0.01, 10.01, NaN, Infinity]], ['beta', [-0.01, 1.01, NaN, Infinity]],
  ]) for (const value of values) {
    assert.throws(() => createModel({dim: 6, groups: mixedGroups, hyper: {[key]: value}}),
      {name: 'RangeError', message: 'invalid hyperparameters'});
  }
});

test('changing amp or beta invalidates the evidence posterior', () => {
  const model = randomModel();
  assert.equal(fit(model).converged, true);
  for (const [key, value] of [['amp', 0.3], ['beta', 0.3]]) {
    const previous = model._posterior;
    model.hyper[key] = value;
    assert.ok(Number.isFinite(logEvidence(model)));
    assert.notEqual(model._posterior, previous);
    assert.equal(model._posterior.hyper[key], value);
  }
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
    const rng = createRng(seed), model = createModel({dim: 1, hyper: {lengths: [0.3]}});
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
  const groups = featureSlices(space).map(({start, count}) => ({start, count, circular: count === 2}));
  const model = createModel({dim: featureDim(space), groups, hyper: {amp: 0.3, beta: 0.3}});
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
    const rng = createRng(42), model = createModel({dim: 1, scenes: 2, hyper: {lengths: [0.3]}});
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
    assert.ok(evaluations > 0 && evaluations <= 500);
    assert.ok(logEvidence(model) >= before);
    assert.ok(model.hyper.lengths.every(x => x >= 0.05 && x <= 50));
    assert.ok(model.hyper.eps >= 0.01 && model.hyper.eps <= 2);
    fitted.push(model.hyper.rho);
    const saved = serialize(model);
    assert.equal(fitHyper(model, {maxEvals: 0}), 0);
    assert.equal(serialize(model), saved);
  }
  assert.ok(fitted[0] > 0.65, `shared rho ${fitted[0]}`);
  assert.ok(fitted[1] < 0.35, `opposite rho ${fitted[1]}`);
});

const hyperOf = model => JSON.stringify({...model.hyper, lengths: Array.from(model.hyper.lengths)});
test('search obeys its budget, only accepts grid lengths, and keeps the default amp and beta', () => {
  const lengthGrid = [0.1, 0.3, 1, 50];
  for (const maxEvals of [0, 1, 2, 7, 500]) {
    const model = createModel({dim: 2, hyper: {lengths: [0.8, 0.8]}});
    const search = createHyperSearch(model, {maxEvals, lengthGrid});
    let steps = 0;
    while (true) {
      const before = serialize(model), previous = model.hyper.lengths.slice();
      const ran = search.step();
      assert.equal(serialize(model), before);
      if (!ran) break;
      assert.equal(search.evaluations, ++steps);
      assert.ok(search.evaluations <= maxEvals);
      search.adopt();
      model.hyper.lengths.forEach((length, g) => {
        if (length !== previous[g]) assert.ok(lengthGrid.includes(length));
      });
      assert.equal(model.hyper.amp, 0);
      assert.equal(model.hyper.beta, 0);
    }
    assert.equal(search.step(), false);
    assert.equal(search.adopt(), steps);
    assert.ok(steps <= maxEvals);
    if (maxEvals <= 7) assert.equal(steps, maxEvals);
  }
});

test('a pass visits every grid length and applies log moves from the latest accepted point', () => {
  const model = createModel({dim: 2, hyper: {lengths: [0.8, 0.8]}});
  const evaluations = fitHyper(model, {lengthGrid: [0.1, 0.3, 1, 50], passes: 1});
  assert.equal(evaluations, 17);
  close(model.hyper.lengths, [0.3, 0.3]);
  close([model.hyper.rho, model.hyper.eps], [0.5, 0.2 * Math.exp(-1.1)], 1e-12);
});

test('search skips clamped moves and logarithmically equal lengths', () => {
  const near = createModel({dim: 1});
  assert.equal(fitHyper(near, {lengthGrid: [1 + 1e-10], passes: 1}), 9);
  assert.equal(near.hyper.lengths[0], 1);
  const rho = createModel({dim: 1, hyper: {rho: 1 - Number.EPSILON}});
  assert.equal(fitHyper(rho, {lengthGrid: [1], passes: 1, maxEvals: 2}), 2);
  assert.ok(rho.hyper.rho < 1 - Number.EPSILON);
  const eps = createModel({dim: 1, hyper: {eps: 2}});
  assert.equal(fitHyper(eps, {lengthGrid: [1], passes: 1, maxEvals: 6}), 6);
  close([eps.hyper.eps], [2 * Math.exp(-0.8)], 1e-12);
});

test('invalid hyper search budgets, passes and grids throw RangeErrors', () => {
  const model = createModel({dim: 1});
  for (const options of [
    {maxEvals: -1}, {maxEvals: 1.5}, {maxEvals: NaN}, {maxEvals: Infinity},
    {passes: 0}, {passes: -1}, {passes: 1.5}, {passes: NaN}, {passes: Infinity},
    {lengthGrid: []}, {lengthGrid: null}, {lengthGrid: [0.049]}, {lengthGrid: [50.01]},
    {lengthGrid: [NaN]}, {lengthGrid: [Infinity]}, {lengthGrid: ['1']}, {lengthGrid: new Array(1)},
    {ampGrid: []}, {ampGrid: [-0.01]}, {ampGrid: [10.01]}, {ampGrid: [NaN]},
    {betaGrid: []}, {betaGrid: [-0.01]}, {betaGrid: [1.01]}, {betaGrid: [NaN]},
    {refine: null}, {refine: [1]}, {refine: [0.5]}, {refine: [10.01]}, {refine: [NaN]}, {refine: [Infinity]},
    {refine: ['2']}, {refine: new Array(1)},
  ]) assert.throws(() => createHyperSearch(model, options), RangeError);
});

test('refinement moves each length at most one step per factor from the grid result and never lowers the evidence', () => {
  const options = {lengthGrid: [0.15, 0.75, 2.5, 50], passes: 1};
  const grid = randomModel(11), refined = randomModel(11), capped = randomModel(11);
  const gridEvals = fitHyper(grid, options);
  const refinedEvals = fitHyper(refined, {...options, refine: [2, 1.25]});
  assert.ok(refinedEvals > gridEvals && refinedEvals <= gridEvals + 2 * 2 * 2, `${gridEvals} ${refinedEvals}`);
  assert.ok(logEvidence(refined) >= logEvidence(grid) - 1e-9);
  const steps = [0.5, 1, 2].flatMap(a => [0.8, 1, 1.25].map(b => a * b));
  refined.hyper.lengths.forEach((length, g) => {
    assert.ok(steps.some(step => Math.abs(length / (grid.hyper.lengths[g] * step) - 1) < 1e-12), `${g}: ${length}`);
  });
  assert.ok(refined.hyper.lengths.some((length, g) => length !== grid.hyper.lengths[g]));
  assert.equal(fitHyper(capped, {...options, refine: [2, 1.25], maxEvals: gridEvals + 1}), gridEvals + 1);
});

test('a stepped hyper search equals fitHyper and, after new answers, refits on the current data', () => {
  const whole = randomModel(11), stepped = randomModel(11);
  assert.equal(fit(whole).converged, true);
  assert.equal(fit(stepped).converged, true);
  const options = {maxEvals: 40, lengthGrid: [0.15, 0.75, 2.5, 50], ampGrid: [0, 0.1, 0.3], betaGrid: [0, 0.3, 1], passes: 3};
  const evaluations = fitHyper(whole, options);
  const search = createHyperSearch(stepped, options);
  let steps = 0;
  while (search.step()) {
    steps++;
    assert.equal(search.evaluations, steps);
  }
  assert.equal(search.step(), false);
  assert.equal(search.adopt(), evaluations);
  assert.equal(serialize(stepped), serialize(whole));
  // An answer arrives during the search: the search keeps its snapshot, and adopting
  // its result refits on every answer, as a fresh fit with those hyperparameters does.
  const late = randomModel(11);
  assert.equal(fit(late).converged, true);
  const pending = createHyperSearch(late, options);
  pending.step();
  addObservation(late, {a: 0, b: 1, outcome: 'A'});
  assert.equal(fit(late).converged, true);
  while (pending.step());
  pending.adopt();
  assert.equal(hyperOf(late), hyperOf(whole));
  assert.equal(late._posterior.revision, late._revision);
  const fresh = randomModel(11);
  addObservation(fresh, {a: 0, b: 1, outcome: 'A'});
  fresh.hyper = whole.hyper;
  assert.equal(fit(fresh).converged, true);
  close(predict(late, late.looks).mean, predict(fresh, fresh.looks).mean, 1e-10);
  assert.throws(() => createHyperSearch(late, {maxEvals: -1}));
});

test('noisy separable quadratic preferences rank the two relevant groups in most seeds', t => {
  let ranked = 0, zeroAmp = 0;
  for (let seed = 0; seed < 6; seed++) {
    const model = preferenceFixture(seed, false);
    assert.ok(fitHyper(model) <= 500);
    const lengths = Array.from(model.hyper.lengths);
    if (Math.max(...lengths.slice(0, 2)) < Math.min(...lengths.slice(2))) ranked++;
    const extended = preferenceFixture(seed, false);
    assert.ok(fitHyper(extended, {ampGrid: [0, 0.1, 0.3, 1, 3]}) <= 500);
    if (extended.hyper.amp === 0) zeroAmp++;
  }
  t.diagnostic(`seeds 0-5, 64 looks, 300 comparisons: relevance ${ranked}/6; extended quadratic amp=0 ${zeroAmp}/6`);
  assert.ok(ranked > 3, `${ranked}/6 seeds ranked both relevant groups ahead of every irrelevant group`);
});

test('noisy saturating preferences select a smooth term in most seeds', t => {
  let positiveAmp = 0;
  for (let seed = 0; seed < 6; seed++) {
    const model = preferenceFixture(seed, true);
    assert.ok(fitHyper(model, {ampGrid: [0, 0.1, 0.3, 1, 3]}) <= 500);
    if (model.hyper.amp > 0) positiveAmp++;
  }
  t.diagnostic(`seeds 0-5, 64 looks, 300 comparisons: saturating amp>0 ${positiveAmp}/6`);
  assert.ok(positiveAmp > 3, `${positiveAmp}/6 seeds selected amp>0`);
});

test('predictMean equals the mean of predict, before and after a fit', () => {
  const model = randomModel();
  const points = [{features: [0.25, 0.25], scene: 0}, {features: [0.6, 0.4], scene: 2}, {features: [0.9, 0.1], scene: 1}];
  assert.deepEqual(predictMean(model, points), predict(model, points).mean);
  assert.equal(fit(model).converged, true);
  assert.deepEqual(predictMean(model, points), predict(model, points).mean);
  assert.throws(() => predictMean(model, [{features: [0.5], scene: 0}]));
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

test('version 2 round trips mixed groups and every hyperparameter, rejecting old and malformed models', () => {
  const model = createModel({dim: 6, groups: mixedGroups,
    hyper: {lengths: [0.22, 0.75, 1.7, 4], rho: 0.35, eps: 0.1, amp: 1, beta: 0.3}});
  const rng = createRng(22);
  for (let i = 0; i < 14; i++) {
    const p = randomPoint(rng);
    addLook(model, p.features, p.scene);
  }
  for (let i = 0; i < 40; i++) {
    const a = rng.int(14);
    let b;
    do b = rng.int(14); while (a === b);
    addObservation(model, {a, b, outcome: outcome(3 * (model.looks[a].features[0] - model.looks[b].features[0]) + Math.SQRT2 * rng.normal())});
  }
  assert.equal(fit(model).converged, true);
  const json = serialize(model), data = JSON.parse(json), restored = deserialize(json);
  assert.equal(data.version, 2);
  assert.deepEqual(data.groups, mixedGroups);
  assert.deepEqual(restored.groups, model.groups);
  assert.equal(hyperOf(restored), hyperOf(model));
  assert.equal(serialize(restored), json);
  const points = Array.from({length: 6}, () => randomPoint(rng));
  assert.deepEqual(predict(restored, points), predict(model, points));
  assert.deepEqual(predictMean(restored, points), predictMean(model, points));
  assert.throws(() => deserialize({...data, version: 1}), TypeError);
  const malformed = mixedGroups.map(g => ({...g}));
  malformed[1].start = 2;
  assert.throws(() => deserialize({...data, groups: malformed}), RangeError);
  assert.throws(() => deserialize({...data, hyper: {...data.hyper, lengths: [1, 1, 1, 1, 1, 1]}}), RangeError);
  assert.throws(() => deserialize({...data, posterior: {...data.posterior, hyper: {...data.posterior.hyper, amp: 11}}}), RangeError);
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

test('a search can start away from the model, and found reports a converged evaluation', () => {
  const plain = randomModel(13), shifted = randomModel(13);
  assert.equal(fit(plain).converged, true);
  const start = plain.hyper;
  shifted.hyper = {...plain.hyper, lengths: plain.hyper.lengths.map(l => l * 3), rho: 0.9};
  assert.equal(fit(shifted).converged, true);
  const options = {maxEvals: 12, lengthGrid: [0.15, 0.75, 2.5, 50]};
  const a = createHyperSearch(plain, options), b = createHyperSearch(shifted, {...options, start});
  assert.equal(b.found, false);
  b.step();
  assert.equal(b.found, true);
  while (a.step());
  while (b.step());
  a.adopt();
  b.adopt();
  assert.equal(serialize(shifted), serialize(plain));
  // Before its first evaluation a search has nothing to adopt and leaves the model alone.
  const untouched = randomModel(13);
  assert.equal(fit(untouched).converged, true);
  const before = serialize(untouched);
  const unused = createHyperSearch(untouched, {...options, start: shifted.hyper});
  assert.equal(unused.found, false);
  unused.adopt();
  assert.equal(serialize(untouched), before);
  assert.throws(() => createHyperSearch(plain, {...options, start: {rho: 1}}), RangeError);
});
