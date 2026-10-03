import test from 'node:test';
import assert from 'node:assert/strict';
import {srgbToLinear, linearToSrgb, linearToOklab, oklabToLinear,
  oklch, mapToGamut, deltaE, CVD_KINDS, simulateCvd} from '../color.js';
import {createRng} from '../rng.js';

const close = (a, b, tol = 1e-9) => {
  assert.equal(a.length, b.length);
  a.forEach((x, i) => assert.ok(Math.abs(x - b[i]) <= tol, `${i}: ${x} != ${b[i]}`));
};

test('sRGB transfer functions and RGB/OKLab round trips', () => {
  assert.equal(srgbToLinear(0), 0);
  assert.equal(srgbToLinear(1), 1);
  assert.equal(srgbToLinear(0.04045), 0.04045 / 12.92);
  assert.equal(linearToSrgb(0.0031308), 12.92 * 0.0031308);
  const rng = createRng(5);
  const colours = [[0, 0, 0], [1, 1, 1], [1, 0, 0], [0, 1, 0], [0, 0, 1],
    ...Array.from({length: 200}, () => [rng.next(), rng.next(), rng.next()])];
  for (const srgb of colours) {
    const linear = srgb.map(srgbToLinear);
    close(linear.map(linearToSrgb), srgb);
    const back = oklabToLinear(linearToOklab(linear));
    close(back, linear);
    close(back.map(linearToSrgb), srgb);
  }
});

test('Ottosson reference white and red; OKLCh and Euclidean deltaE', () => {
  // White's small residual is the rounding in the published forward matrices.
  close(linearToOklab([1, 1, 1]), [1, 0, 0], 5e-8);
  close(linearToOklab([1, 0, 0]), [0.6279553606145516, 0.224863061065974, 0.1258462985307351]);
  close(oklch(0.5, 0.2, 90), [0.5, 0, 0.2]);
  close(oklch(0.5, 0.2, 450), oklch(0.5, 0.2, 90));
  assert.equal(deltaE([0, 0, 0], [0.03, 0.04, 0]), 0.05);
});

test('gamut mapping reduces only chroma and approaches the boundary', () => {
  const mapped = mapToGamut(0.85, 0.20, 90);
  assert.ok(mapped.ok);
  assert.ok(mapped.C > 0 && mapped.C < 0.2);
  assert.ok(mapped.linear.every(x => x >= 0 && x <= 1));
  assert.ok(mapped.srgb.every(x => x >= 0 && x <= 1));
  close(mapped.lab, oklch(0.85, mapped.C, 90));
  close(mapped.lab, linearToOklab(mapped.linear));
  const beyond = oklabToLinear(oklch(0.85, mapped.C + 1e-9, 90));
  assert.ok(beyond.some(x => x < 0 || x > 1));
  assert.equal(mapToGamut(0.5, 0.01, 20).C, 0.01);
  assert.equal(mapToGamut(1.05, 0.1, 0).ok, false);
  assert.equal(mapToGamut(-0.01, 0.1, 0).ok, false);
  assert.equal(mapToGamut(0.5, -0.1, 0).ok, false);
  assert.equal(mapToGamut(NaN, 0.1, 0).ok, false);
  close(mapToGamut(0, 0.2, 0).linear, [0, 0, 0]);
  close(mapToGamut(1, 0.2, 0).linear, [1, 1, 1]);
});

test('Machado severity-one matrices preserve grey and clamp linear RGB', () => {
  assert.deepEqual(CVD_KINDS, ['protan', 'deutan', 'tritan']);
  for (const kind of CVD_KINDS) {
    for (const x of [0, 0.2, 0.5, 1]) close(simulateCvd([x, x, x], kind), [x, x, x], 1.1e-6);
    assert.ok(simulateCvd([1, 0, 0], kind).every(x => x >= 0 && x <= 1));
  }
  close(simulateCvd([1, 0, 0], 'protan'), [0.152286, 0.114503, 0]);
  close(simulateCvd([0, 1, 0], 'deutan'), [0.860646, 0.672501, 0.042940]);
  close(simulateCvd([0, 0, 1], 'tritan'), [0, 0.147602, 0.303900]);
  assert.throws(() => simulateCvd([0, 0, 0], 'unknown'));
});
