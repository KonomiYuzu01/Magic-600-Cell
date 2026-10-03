import test from 'node:test';
import assert from 'node:assert/strict';
import {createModel, addLook, addObservation, fit, predict} from '../gp.js';
import {createRng} from '../rng.js';
import {nextPair, expectedInfo, settled} from '../acquire.js';

const points = [0, 0.25, 0.5, 0.75, 1].map(x => ({features: new Float64Array([x]), scene: 0}));
function trained(count = 100) {
  const model = createModel({dim: 1});
  for (const point of points) addLook(model, point.features, point.scene);
  for (let i = 0; i < count; i++) addObservation(model, {a: 4, b: i % 4, outcome: 'A'});
  assert.equal(fit(model).converged, true);
  return model;
}

test('Thompson and information pairs have distinct candidate indices and include the incumbent', () => {
  const model = trained();
  const means = predict(model, points).mean;
  const best = means.indexOf(Math.max(...means));
  const rng = createRng(21);
  for (const infoRate of [0, 1]) for (let i = 0; i < 20; i++) {
    const pair = nextPair(model, points, rng, {infoRate, repeatRate: 0});
    assert.equal(pair.kind, infoRate ? 'info' : 'thompson');
    assert.equal(pair.a, best);
    assert.notEqual(pair.a, pair.b);
    assert.ok([pair.a, pair.b].every(i => Number.isInteger(i) && i >= 0 && i < points.length));
  }
});

test('reject neighbourhoods are excluded, including the former incumbent', () => {
  const model = trained(), rng = createRng(21);
  for (const rejected of [[points[4]], [4], [points[4].features]]) {
    for (let i = 0; i < 10; i++) {
      const pair = nextPair(model, points, rng, {rejected, rejectRadius: 0.26, repeatRate: 0});
      assert.ok(pair.a < 3 && pair.b < 3);
      assert.notEqual(pair.a, pair.b);
    }
  }
  assert.throws(() => nextPair(model, points, rng, {rejected: points, repeatRate: 0}), /two/);
  assert.throws(() => nextPair(model, points, rng, {infoRate: 0.8, repeatRate: 0.3}));
});

test('repeat returns an observed pair as look indices and respects rejections', () => {
  const model = createModel({dim: 1});
  addLook(model, [0.75], 0); addLook(model, [1], 0);
  addObservation(model, {a: 1, b: 0, outcome: 'A'});
  assert.equal(fit(model).converged, true);
  const pair = nextPair(model, points, createRng(4), {repeatRate: 1, infoRate: 0});
  assert.deepEqual(pair, {a: 1, b: 0, kind: 'repeat', lookA: 1, lookB: 0});
  const next = nextPair(model, points, createRng(4), {
    repeatRate: 1, infoRate: 0, rejected: [model.looks[1]], rejectRadius: 0.01,
  });
  assert.equal(next.kind, 'thompson');
  assert.ok(next.a !== 4 && next.b !== 4);
  const otherScene = points.map(p => ({...p, scene: 1}));
  assert.equal(nextPair(model, otherScene, createRng(4), {repeatRate: 1, infoRate: 0}).kind, 'thompson');
});

test('mixed-scene rounds never compare across scenes', () => {
  const model = trained();
  const candidates = [...points, ...points.map(p => ({...p, scene: 'inspecting'}))];
  const pair = nextPair(model, candidates, createRng(4), {repeatRate: 0});
  assert.equal(candidates[pair.a].scene, candidates[pair.b].scene);
});

test('expected information vanishes for a known difference and decreases with consistent data', () => {
  const prior = createModel({dim: 1});
  const uncertain = expectedInfo(prior, points[0], points[4], createRng(1), 2048);
  assert.ok(uncertain > 0.1);
  assert.equal(expectedInfo(prior, points[0], points[0], createRng(1)), 0);
  const model = trained(800);
  const certain = expectedInfo(model, points[0], points[4], createRng(1), 2048);
  assert.ok(certain < 0.02, `certain MI ${certain}`);
  assert.ok(uncertain > 5 * certain);
  assert.throws(() => expectedInfo(model, points[0], points[1], createRng(1), 0));
});

test('settled is false before learning and true after a long consistent run', () => {
  const model = createModel({dim: 1});
  const round = [points[0], points[2], points[4]];
  assert.equal(settled(model, round, createRng(2)).settled, false);
  for (const point of round) addLook(model, point.features, point.scene);
  for (let i = 0; i < 800; i++) addObservation(model, {a: 2, b: i % 2, outcome: 'A'});
  assert.equal(fit(model).converged, true);
  const result = settled(model, round, createRng(2));
  assert.equal(result.settled, true);
  assert.ok(result.prob >= 0.9);
  assert.ok(result.prob <= 1);
  assert.throws(() => settled(model, [], createRng(2)));
});

test('a supplied prediction gives the same pair and settled result as computing it', () => {
  const model = trained(60);
  const prediction = predict(model, points);
  for (const infoRate of [0, 1]) {
    assert.deepEqual(nextPair(model, points, createRng(8), {infoRate, repeatRate: 0, prediction}),
      nextPair(model, points, createRng(8), {infoRate, repeatRate: 0}));
  }
  assert.deepEqual(settled(model, points, createRng(3), {prediction}), settled(model, points, createRng(3)));
  assert.throws(() => nextPair(model, points, createRng(8), {prediction: predict(model, points.slice(1))}), /prediction/);
  assert.throws(() => settled(model, points, createRng(3), {prediction: {mean: prediction.mean}}), /prediction/);
});

test('settled tests the reported best look, not the candidate with the highest mean', () => {
  const model = trained(800);
  const means = predict(model, points).mean;
  const top = means.indexOf(Math.max(...means));
  assert.deepEqual(settled(model, points, createRng(2), {best: top}), settled(model, points, createRng(2)));
  assert.equal(settled(model, points, createRng(2), {best: top}).settled, true);
  const other = settled(model, points, createRng(2), {best: (top + 1) % points.length});
  assert.equal(other.settled, false);
  assert.ok(other.prob < 0.1, `prob ${other.prob}`);
  for (const best of [-1, points.length, 1.5]) assert.throws(() => settled(model, points, createRng(2), {best}), /best/);
});
