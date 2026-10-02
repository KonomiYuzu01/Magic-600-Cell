import test from 'node:test';
import assert from 'node:assert/strict';
import {SCENES, defaultSpace, validateSpace, encode, featureDim, canonical, sobol,
  fromUnit, palette, background, hardCheck} from '../space.js';
import {mapToGamut} from '../color.js';

const space = defaultSpace();
const midpoint = () => Object.fromEntries(space.params.map(p => [p.id, (p.min + p.max) / 2]));
const fixture = () => ({...midpoint(), hueRotation: 15, hueSpread: 360, lightness: 0.5,
  lightnessAlt: 0.05, chroma: 0.2, classes: 4, bgLightness: 0.05, bgTint: 0});
const clique = [[0, 1], [0, 2], [0, 3], [1, 2], [1, 3], [2, 3]];

test('the complete plan table, independent copies and space validation', () => {
  assert.deepEqual(SCENES, ['solving', 'inspecting', 'celebrating']);
  assert.deepEqual(space.params.map(p => [p.id, p.min, p.max, p.kind]), [
    ['hueRotation', 0, 360, 'circular'], ['hueSpread', 60, 360, 'linear'],
    ['lightness', 0.45, 0.85, 'linear'], ['lightnessAlt', 0, 0.2, 'linear'],
    ['chroma', 0.04, 0.20, 'linear'], ['classes', 4, 8, 'integer'],
    ['bgLightness', 0.05, 0.95, 'linear'], ['bgHue', 0, 360, 'circular'],
    ['bgTint', 0, 0.05, 'linear'], ['gap', 0, 0.3, 'linear'],
    ['edgeWeight', 0, 3, 'linear'], ['edgeBrightness', 0, 1, 'linear'],
    ['gloss', 0, 1, 'linear'], ['glow', 0, 1, 'linear'], ['fog', 0, 1, 'linear'],
    ['turnMs', 150, 900, 'linear'], ['easeA', 0, 1, 'linear'], ['easeB', 0, 1, 'linear'],
  ]);
  assert.equal(featureDim(space), 20);
  const copy = defaultSpace();
  copy.params[0].max = 99;
  assert.equal(defaultSpace().params[0].max, 360);
  for (const bad of [null, {}, {params: []}, {params: [space.params[0], space.params[0]]},
    {params: [{...space.params[1], min: 400}]}, {params: [{...space.params[0], max: 180}]},
    {params: [{...space.params[5], min: 4.5}]}, {params: [{...space.params[1], kind: 'string'}]}]) {
    assert.throws(() => validateSpace(bad));
  }
});

test('circular encoding wraps exactly and crosses the seam continuously', () => {
  const look = midpoint();
  assert.deepEqual(encode(space, {...look, hueRotation: 0}), encode(space, {...look, hueRotation: 360}));
  const a = encode(space, {...look, hueRotation: 359});
  const b = encode(space, {...look, hueRotation: 1});
  assert.ok(Math.hypot(...a.map((x, i) => x - b[i])) < 0.006);
  const quarter = encode(space, {...look, hueRotation: 90});
  assert.ok(Math.abs(quarter[0]) < 1e-12);
  assert.ok(Math.abs(quarter[1] - 1 / (2 * Math.PI)) < 1e-12);
  const fixed = canonical(space, {...look, hueRotation: -1, classes: 4.4});
  assert.equal(fixed.hueRotation, 359);
  assert.equal(fixed.classes, 4);
  assert.equal(look.hueRotation, 180);
  assert.throws(() => encode(space, {...look, lightness: NaN}));
});

test('Joe–Kuo Sobol published initial points, skipping and all 32 dimensions', () => {
  // Gray-code order, including the origin (Joe–Kuo D(6)).
  const initial = [[0, 0, 0], [0.5, 0.5, 0.5], [0.75, 0.25, 0.25], [0.25, 0.75, 0.75],
    [0.375, 0.375, 0.625], [0.875, 0.875, 0.125], [0.625, 0.125, 0.875], [0.125, 0.625, 0.375]];
  assert.deepEqual(sobol(8, 3).map(x => Array.from(x)), initial);
  assert.deepEqual(sobol(3, 3, 5), sobol(8, 3).slice(5));
  assert.deepEqual(Array.from(sobol(1, 5, 4)[0]), [0.375, 0.375, 0.625, 0.875, 0.375]);
  const points = sobol(1024, 32);
  for (const point of points) assert.ok(point.every(x => x >= 0 && x < 1));
  for (let d = 0; d < 32; d++) assert.equal(new Set(points.map(p => p[d])).size, 1024);
  assert.ok(sobol(1, 32, 4294967295)[0].every(x => x >= 0 && x < 1));
  assert.throws(() => sobol(2, 1, 4294967295));
  assert.throws(() => sobol(1, 33));
  assert.throws(() => sobol(-1, 1));
});

test('unit mapping covers ranges and gives integer classes without bias', () => {
  const first = fromUnit(space, new Float64Array(18));
  space.params.forEach(p => assert.equal(first[p.id], p.min));
  const last = fromUnit(space, new Float64Array(18).fill(1 - 1e-10));
  assert.equal(last.classes, 8);
  space.params.forEach(p => assert.ok(last[p.id] >= p.min && last[p.id] <= p.max));
  const counts = Array(5).fill(0);
  for (let i = 0; i < 100; i++) {
    const unit = new Float64Array(18).fill((i + 0.5) / 100);
    counts[fromUnit(space, unit).classes - 4]++;
  }
  assert.deepEqual(counts, [20, 20, 20, 20, 20]);
  assert.throws(() => fromUnit(space, new Float64Array(18).fill(1)));
  assert.throws(() => fromUnit(space, [0]));
});

test('palette and background use the shared gamut mapping', () => {
  const look = fixture();
  const colours = palette(look);
  assert.equal(colours.length, 4);
  assert.deepEqual(colours.map(c => c.h), [15, 105, 195, 285]);
  assert.deepEqual(colours.map(c => c.L), [0.45, 0.55, 0.45, 0.55]);
  for (const c of colours) assert.deepEqual(c.mapped, mapToGamut(c.L, c.C, c.h));
  assert.deepEqual(background(look), mapToGamut(look.bgLightness, look.bgTint, look.bgHue));
});

test('hard checks fail gamut first, detect equal touching classes, and accept a four-clique', () => {
  const look = fixture();
  const failed = hardCheck({...look, lightness: 0.85, lightnessAlt: 0.2, edgeBrightness: -1}, clique);
  assert.deepEqual(failed.reasons, ['gamut']);
  const equal = hardCheck({...look, lightness: 0.85, lightnessAlt: 0.15}, [[1, 3]]);
  assert.equal(equal.ok, false);
  assert.equal(equal.minDeltaE.normal, 0);
  assert.ok(equal.reasons.some(r => r.startsWith('deltaE:')));
  const accepted = hardCheck(look, clique);
  assert.equal(accepted.ok, true, accepted.reasons.join(', '));
  assert.ok(Object.values(accepted.minDeltaE).every(x => x >= 0.08));
  assert.ok(accepted.minBgL >= 0.20);
  assert.ok(hardCheck({...look, glow: 2}, clique).reasons.includes('range:glow'));
  assert.ok(hardCheck({...look, bgLightness: 0.5}, clique).reasons.includes('background'));
  assert.equal(hardCheck(look, clique, {deltaE: 0.1, bgL: 0.2}).ok, false);
  assert.throws(() => hardCheck(look, [[0, 9]]));
});
