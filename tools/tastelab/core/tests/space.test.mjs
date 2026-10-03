import test from 'node:test';
import assert from 'node:assert/strict';
import {SCENES, defaultSpace, validateSpace, encode, featureDim, featureSlices, canonical, sobol,
  fromUnit, palette, background, hardCheck, familiesOf, sceneSpace, withinScene, MAX_FAMILIES} from '../space.js';
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
    {params: [{...space.params[5], min: 4.5}]}, {params: [{...space.params[1], kind: 'string'}]},
    {params: [{...space.params[5], min: 3}]}, {params: [{...space.params[5], max: 9}]},
    {params: [{...space.params[5], kind: 'linear'}]}]) {
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

test('feature slices follow the encoding order', () => {
  const slices = featureSlices(space);
  assert.deepEqual(slices.slice(0, 3), [{id: 'hueRotation', start: 0, count: 2}, {id: 'hueSpread', start: 2, count: 1},
    {id: 'lightness', start: 3, count: 1}]);
  assert.deepEqual(slices.find(s => s.id === 'bgHue'), {id: 'bgHue', start: 8, count: 2});
  const last = slices[slices.length - 1];
  assert.equal(last.start + last.count, featureDim(space));
  const look = midpoint();
  const features = encode(space, look);
  const moved = encode(space, {...look, glow: look.glow + 0.25});
  const glow = slices.find(s => s.id === 'glow');
  features.forEach((x, i) => assert.equal(x !== moved[i], i === glow.start, `feature ${i}`));
});

test('hard checks use the ranges of the given parameter table', () => {
  const look = {...fixture(), glow: 2};
  const wide = {params: space.params.map(p => p.id === 'glow' ? {...p, max: 2} : p)};
  assert.ok(hardCheck(look, clique).reasons.includes('range:glow'));
  assert.equal(hardCheck(look, clique, undefined, wide).ok, true);
  const narrow = {params: space.params.map(p => p.id === 'gloss' ? {...p, max: 0.2} : p)};
  assert.ok(hardCheck({...fixture(), gloss: 0.5}, clique, undefined, narrow).reasons.includes('range:gloss'));
  assert.throws(() => hardCheck(fixture(), clique, undefined, {params: []}));
});

test('theme families: two by default, one for a table without them, and validated', () => {
  assert.deepEqual(familiesOf(space), [{id: 'f1', name: 'Family 1'}, {id: 'f2', name: 'Family 2'}]);
  assert.deepEqual(familiesOf({params: space.params}), [{id: 'f1', name: 'Family 1'}]);
  const copy = defaultSpace();
  copy.families[0].name = 'Observatory';
  assert.equal(defaultSpace().families[0].name, 'Family 1');
  validateSpace({...space, families: [{id: 'calm', name: 'Calm'}]});
  const many = Array.from({length: MAX_FAMILIES + 1}, (_, i) => ({id: 'f' + i, name: 'F' + i}));
  for (const families of [[], many, [{id: 'f1', name: 'A'}, {id: 'f1', name: 'B'}], [{id: 'F1', name: 'A'}],
    [{id: 'f1', name: ' '}], [{id: 'f1'}], [{id: 'f1', name: 'x'.repeat(41)}], ['f1'], {f1: 'A'}]) {
    assert.throws(() => validateSpace({...space, families}), /families/);
  }
});

test('scene limits narrow the celebrating ranges only and keep the encoding', () => {
  const celebrating = sceneSpace(space, 'celebrating');
  const range = (sp, id) => { const p = sp.params.find(q => q.id === id); return [p.min, p.max]; };
  assert.deepEqual(range(celebrating, 'glow'), [0, 0.5]);
  assert.deepEqual(range(celebrating, 'turnMs'), [150, 600]);
  assert.deepEqual(range(celebrating, 'gloss'), [0, 1]);
  for (const scene of ['solving', 'inspecting']) assert.deepEqual(sceneSpace(space, scene).params, space.params);
  assert.deepEqual(sceneSpace({params: space.params}, 'celebrating').params, space.params);
  assert.throws(() => sceneSpace(space, 'party'), /scene/);
  const look = {...fixture(), glow: 0.7, turnMs: 500};
  assert.equal(withinScene(space, look, 'solving'), true);
  assert.equal(withinScene(space, look, 'celebrating'), false);
  assert.equal(withinScene(space, {...look, glow: 0.5}, 'celebrating'), true);
  assert.equal(withinScene(space, {...look, glow: 0.5, turnMs: 601}, 'celebrating'), false);
  assert.equal(withinScene(space, {...look, glow: undefined}, 'solving'), false);
  assert.ok(hardCheck({...fixture(), glow: 0.7}, clique, undefined, celebrating).reasons.includes('range:glow'));
  validateSpace({...space, sceneLimits: {}});
  validateSpace({...space, sceneLimits: {solving: {classes: {min: 5, max: 6}}}});
  for (const sceneLimits of [[], {party: {glow: {max: 0.5}}}, {celebrating: {nope: {max: 1}}},
    {celebrating: {hueRotation: {max: 90}}}, {celebrating: {glow: {max: 1.5}}}, {celebrating: {glow: {min: 0.5, max: 0.5}}},
    {celebrating: {glow: {}}}, {celebrating: {glow: {max: 0.5, step: 1}}}, {celebrating: {classes: {max: 6.5}}},
    {celebrating: {glow: 0.5}}, {celebrating: []}]) {
    assert.throws(() => validateSpace({...space, sceneLimits}), /scene limits/);
  }
});
