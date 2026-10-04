import test from 'node:test';
import assert from 'node:assert/strict';
import {createModel, addLook, addObservation, fit, predict, outcomeProbabilities} from '../gp.js';
import {createRng} from '../rng.js';
import {nextPair, mutualInfo, expectedInfo, settled} from '../acquire.js';

// Evidence: synthetic points and answers only.
const points = [0, 0.25, 0.5, 0.75, 1].map(x => ({features: new Float64Array([x]), scene: 0}));
const median = values => Array.from(values).sort((a, b) => a - b)[Math.floor(values.length / 2)];
function trained(count = 100) {
  const model = createModel({dim: 1});
  for (const point of points) addLook(model, point.features, point.scene);
  for (let i = 0; i < count; i++) addObservation(model, {a: 4, b: i % 4, outcome: 'A'});
  assert.equal(fit(model).converged, true);
  return model;
}
function pairInfo(model, prediction, a, b) {
  const {mean, cov} = prediction, n = mean.length;
  return mutualInfo(mean[a] - mean[b], cov[a * n + a] + cov[b * n + b] - 2 * cov[a * n + b],
    model._posterior?.hyper.eps ?? model.hyper.eps);
}
function monteCarloInfo(mean, variance, eps, seed) {
  const rng = createRng(seed), count = 100000;
  const average = {A: 0, B: 0, same: 0};
  const entropy = p => Object.values(p).reduce((sum, x) => sum - (x > 0 ? x * Math.log(x) : 0), 0);
  let conditional = 0;
  for (let i = 0; i < count; i++) {
    const p = outcomeProbabilities(mean + Math.sqrt(variance) * rng.normal(), eps);
    for (const key of Object.keys(average)) average[key] += p[key] / count;
    conditional += entropy(p) / count;
  }
  return entropy(average) - conditional;
}

test('mutual information is zero at zero variance and symmetric in the mean difference', () => {
  for (const variance of [0, -1, 1e-14]) assert.equal(mutualInfo(0, variance, 0.2), 0);
  for (const mean of [0.7, 3]) {
    assert.ok(Math.abs(mutualInfo(mean, 2, 0.2) - mutualInfo(-mean, 2, 0.2)) < 1e-12);
  }
});

test('mutual information increases with variance and decreases far from a tie', () => {
  const byVariance = [0.01, 0.1, 1, 4, 16].map(v => mutualInfo(0, v, 0.2));
  const byMean = [0, 1, 3, 8].map(m => mutualInfo(m, 2, 0.2));
  assert.ok(byVariance.every((v, i) => i === 0 || v > byVariance[i - 1]));
  assert.ok(byMean.every((v, i) => i === 0 || v < byMean[i - 1]));
  assert.ok(byMean.every(v => v >= 0));
});

test('quantile quadrature agrees with seeded Monte Carlo within 0.01', () => {
  for (const [mean, variance, eps] of [[0, 1, 0.2], [1, 2, 0.8], [-2, 4, 0.05]]) {
    const actual = mutualInfo(mean, variance, eps);
    const estimate = monteCarloInfo(mean, variance, eps, 41);
    assert.ok(Math.abs(actual - estimate) < 0.01, `${mean}, ${variance}, ${eps}: ${actual} vs ${estimate}`);
  }
});

test('expected information is deterministic and decreases with consistent data', () => {
  const prior = createModel({dim: 1});
  const uncertain = expectedInfo(prior, points[0], points[4]);
  assert.ok(uncertain > 0.1);
  assert.equal(expectedInfo(prior, points[0], points[0]), 0);
  assert.equal(expectedInfo(prior, points[0], points[4]), uncertain);
  const model = trained(800);
  const certain = expectedInfo(model, points[0], points[4]);
  assert.ok(certain < 0.02, `certain MI ${certain}`);
  assert.ok(uncertain > 5 * certain);
  assert.equal(certain, pairInfo(model, predict(model, [points[0], points[4]]), 0, 1));
  model.hyper = {...model.hyper, eps: 1.5};
  assert.equal(expectedInfo(model, points[0], points[4]), certain);
});

test('anchor rounds choose the highest mean and its most informative partner', () => {
  const model = trained(), prediction = predict(model, points);
  const best = prediction.mean.indexOf(Math.max(...prediction.mean));
  const partners = points.map((_, i) => i).filter(i => i !== best);
  const partner = partners.reduce((a, b) => pairInfo(model, prediction, best, b) >
    pairInfo(model, prediction, best, a) ? b : a);
  const expected = {a: best, b: partner, kind: 'mi'};
  assert.deepEqual(nextPair(model, points, createRng(21), {anchorRate: 1}), expected);
  const rng = createRng(21), afterRoll = createRng(21);
  afterRoll.next();
  assert.deepEqual(nextPair(model, points, rng, {anchorRate: 0, freePairs: 0}), expected);
  assert.equal(rng.next(), afterRoll.next());
});

test('free rounds are at least as informative as the anchor pair and are reproducible', () => {
  const model = trained(), prediction = predict(model, points);
  const anchor = nextPair(model, points, createRng(9), {anchorRate: 1});
  for (const seed of [4, 21, 98]) {
    const options = {anchorRate: 0, freePairs: 3000};
    const pair = nextPair(model, points, createRng(seed), options);
    assert.ok(pairInfo(model, prediction, pair.a, pair.b) >= pairInfo(model, prediction, anchor.a, anchor.b));
    assert.notEqual(pair.a, pair.b);
    assert.ok([pair.a, pair.b].every(i => Number.isInteger(i) && i >= 0 && i < points.length));
    assert.ok(['mi', 'free'].includes(pair.kind));
    assert.deepEqual(pair, nextPair(model, points, createRng(seed), options));
  }
});

test('a strictly better free pair wins, while tied pairs keep the first anchor and partner', () => {
  const model = createModel({dim: 1}), candidates = points.slice(0, 3);
  const prediction = {mean: [3, 0, 0], cov: [0.01, 0, 0, 0, 1, 0, 0, 0, 1]};
  let calls = 0;
  const rng = {next: () => 0.75, int: n => { calls++; assert.ok(n === 3 || n === 2); return 1; }};
  assert.deepEqual(nextPair(model, candidates, rng, {anchorRate: 0, freePairs: 1, prediction}),
    {a: 1, b: 2, kind: 'free'});
  assert.equal(calls, 2);
  const tied = {mean: [0, 0, 0], cov: Array(9).fill(0)};
  assert.deepEqual(nextPair(model, candidates, createRng(3), {anchorRate: 0, freePairs: 40, prediction: tied}),
    {a: 0, b: 1, kind: 'mi'});
});

test('reject neighbourhoods exclude both candidates, including the former anchor', () => {
  const model = trained(), rng = createRng(21);
  const means = predict(model, points).mean.slice(0, 3);
  const anchor = means.indexOf(Math.max(...means));
  for (const rejected of [[points[4]], [4], [points[4].features]]) {
    for (const anchorRate of [0, 1]) {
      const pair = nextPair(model, points, rng, {rejected, rejectRadius: 0.26, anchorRate, freePairs: 80});
      assert.ok(pair.a < 3 && pair.b < 3);
      assert.notEqual(pair.a, pair.b);
      if (anchorRate === 1) assert.equal(pair.a, anchor);
    }
  }
  // Distance exactly equal to the radius is excluded too.
  const pair = nextPair(model, points, rng, {rejected: [4], rejectRadius: 0.25, anchorRate: 1});
  assert.ok(pair.a < 3 && pair.b < 3);
  assert.throws(() => nextPair(model, points, rng, {rejected: points}),
    {name: 'RangeError', message: 'at least two non-rejected candidates required'});
});

test('mixed-scene rounds use the highest-mean anchor that has a partner in its scene', () => {
  const model = createModel({dim: 1});
  const candidates = [points[0], {...points[1], scene: 'solving'},
    {...points[2], scene: 'inspecting'}, {...points[3], scene: 1}, {...points[4], scene: 2}];
  const prediction = {mean: [0, 1, 3, 2, 10], cov: Array(25).fill(0)};
  for (const anchorRate of [0, 1]) {
    const pair = nextPair(model, candidates, createRng(4), {anchorRate, freePairs: 50, prediction});
    assert.deepEqual(pair, {a: 2, b: 3, kind: 'mi'});
  }
  assert.throws(() => nextPair(model, [candidates[0], candidates[2], candidates[4]], createRng(4)),
    {name: 'RangeError', message: 'two candidates in the same scene required'});
});

test('a supplied prediction gives the same pair and settled result as computing it', () => {
  const model = trained(60), prediction = predict(model, points);
  for (const anchorRate of [0, 0.5, 1]) {
    const options = {anchorRate, freePairs: 60};
    assert.deepEqual(nextPair(model, points, createRng(8), {...options, prediction}),
      nextPair(model, points, createRng(8), options));
  }
  const reference = median(prediction.mean);
  assert.deepEqual(settled(model, points, createRng(3), {prediction, reference}),
    settled(model, points, createRng(3), {reference}));
  assert.throws(() => nextPair(model, points, createRng(8), {prediction: predict(model, points.slice(1))}), /prediction/);
  assert.throws(() => settled(model, points, createRng(3), {reference, prediction: {mean: prediction.mean}}), /prediction/);
});

test('invalid acquisition options and rejected looks throw RangeErrors', () => {
  const model = trained(), rng = createRng(2);
  for (const anchorRate of [-0.1, 1.1, NaN, Infinity]) {
    assert.throws(() => nextPair(model, points, rng, {anchorRate}), RangeError);
  }
  for (const freePairs of [-1, 1.5, NaN, Infinity]) {
    assert.throws(() => nextPair(model, points, rng, {freePairs}), RangeError);
  }
  for (const rejectRadius of [-1, NaN, Infinity]) {
    assert.throws(() => nextPair(model, points, rng, {rejectRadius}), RangeError);
  }
  for (const rejected of [[99], [1.5], [[0, 1]], [{features: [NaN]}], [null]]) {
    assert.throws(() => nextPair(model, points, rng, {rejected}), RangeError);
  }
  for (const candidates of [[], [points[0]]]) assert.throws(() => nextPair(model, candidates, rng), RangeError);
  assert.throws(() => nextPair(model, [{features: [NaN], scene: 0}, points[1]], rng), RangeError);
});

test('settled is false before learning and true after a long consistent run', () => {
  const model = createModel({dim: 1}), round = [points[0], points[2], points[4]];
  assert.deepEqual(settled(model, round, createRng(2), {reference: 0}), {settled: false, regret: 1});
  for (const point of round) addLook(model, point.features, point.scene);
  for (let i = 0; i < 800; i++) addObservation(model, {a: 2, b: i % 2, outcome: 'A'});
  assert.equal(fit(model).converged, true);
  const reference = median(predict(model, round).mean);
  const result = settled(model, round, createRng(2), {reference});
  assert.equal(result.settled, true);
  assert.ok(result.regret >= 0 && result.regret <= 0.05, `regret ${result.regret}`);
});

test('settled tests the reported best look and caps the regret for a poor choice', () => {
  const model = trained(800), prediction = predict(model, points);
  const reference = median(prediction.mean), top = prediction.mean.indexOf(Math.max(...prediction.mean));
  assert.deepEqual(settled(model, points, createRng(2), {reference, best: top}),
    settled(model, points, createRng(2), {reference}));
  assert.equal(settled(model, points, createRng(2), {reference, best: top}).settled, true);
  const other = settled(model, points, createRng(2), {reference, best: 0});
  assert.equal(other.settled, false);
  assert.ok(other.regret > 0.5 && other.regret <= 1, `regret ${other.regret}`);
});

test('nonpositive predicted ranges return regret 1 without drawing', () => {
  const model = trained(), candidates = points.slice(0, 2);
  const prediction = {mean: [0, 1], cov: Array(4).fill(0), draw: () => assert.fail('unexpected draw')};
  for (const reference of [1, 2]) {
    assert.deepEqual(settled(model, candidates, createRng(2), {prediction, reference}),
      {settled: false, regret: 1});
  }
});

test('regret averages losses before capping, using the highest mean for its fixed range', () => {
  const model = trained(1), candidates = points.slice(0, 3);
  let draw = 0;
  const prediction = {mean: [1, 3, 2], cov: Array(9).fill(0), draw: () =>
    Float64Array.of(0, draw++ % 2 ? 1.25 : 0.25, 0)};
  const options = {prediction, reference: 2, best: 0, draws: 2};
  assert.deepEqual(settled(model, candidates, createRng(2), {...options, tau: 0.75}),
    {settled: true, regret: 0.75});
  assert.deepEqual(settled(model, candidates, createRng(2), {...options, tau: 0.74}),
    {settled: false, regret: 0.75});
});

test('settled uses a prediction draw function and still requires an observation', () => {
  const model = createModel({dim: 1}), candidates = points.slice(0, 2), rng = createRng(2);
  let calls = 0;
  const prediction = {mean: [0, 1], cov: Array(4).fill(0), draw: drawRng => {
    assert.equal(drawRng, rng);
    calls++;
    return Float64Array.of(-1, 2);
  }};
  assert.deepEqual(settled(model, candidates, rng, {prediction, reference: 0, draws: 7}),
    {settled: false, regret: 0});
  for (const point of candidates) addLook(model, point.features, point.scene);
  addObservation(model, {a: 1, b: 0, outcome: 'A'});
  assert.deepEqual(settled(model, candidates, rng, {prediction, reference: 0, draws: 7}),
    {settled: true, regret: 0});
  assert.equal(calls, 14);
});

test('TL-P11 fixed-range regret stays finite and stable near a draw maximum of zero', () => {
  const model = trained(1);
  const candidates = Array.from({length: 512}, (_, i) => ({features: [i / 511], scene: 0}));
  const mean = Float64Array.from(candidates, p => p.features[0] - 0.5);
  let nearReference = 0;
  const prediction = {mean, cov: new Float64Array(512 ** 2), draw: rng => {
    const c = 0.5 + rng.next();
    const near = rng.next() < 0.25;
    const a = near ? -1 / c + mean[256] : -0.75 + 1.5 * rng.next();
    const values = mean.map(z => z + c * (-z * z + a * z));
    if (near) {
      assert.ok(Math.abs(Math.max(...values)) < 1e-8);
      nearReference++;
    }
    return values;
  }};
  const estimate = draws => settled(model, candidates, createRng(11), {prediction, reference: 0, best: 511, draws});
  const short = estimate(200), long = estimate(20000);
  assert.ok(nearReference > 0);
  for (const result of [short, long]) assert.ok(Number.isFinite(result.regret) && result.regret >= 0 && result.regret <= 1);
  assert.ok(Math.abs(short.regret - long.regret) < 0.05, `${short.regret} vs ${long.regret}`);
});

test('invalid settled best, reference, draws, tau and candidates throw RangeErrors', () => {
  const model = trained(), rng = createRng(2), reference = 0;
  for (const best of [-1, points.length, 1.5, NaN, Infinity, '0']) {
    assert.throws(() => settled(model, points, rng, {reference, best}), RangeError);
  }
  for (const reference of [undefined, null, NaN, Infinity, -Infinity, '0']) {
    assert.throws(() => settled(model, points, rng, {reference}), RangeError);
  }
  for (const draws of [0, -1, 1.5, NaN, Infinity]) {
    assert.throws(() => settled(model, points, rng, {reference, draws}), RangeError);
  }
  for (const tau of [-1, NaN, Infinity]) assert.throws(() => settled(model, points, rng, {reference, tau}), RangeError);
  assert.throws(() => settled(model, [], rng, {reference}), RangeError);
  assert.equal(settled(model, points, rng, {reference, tau: 2}).settled, true);
});
