import test from "node:test";
import assert from "node:assert/strict";
import {performance} from "node:perf_hooks";
import {srgbToLinear, linearToOklab, oklabToLinear, linearToSrgb} from "../../core/color.js";
import {createRng} from "../../core/rng.js";
import {CHROMA_MIN, THRESHOLDS, HINT_PARAMS, describeImage, compareImages} from "../attributes.js";

const close = (actual, expected, tolerance = 1e-4) => {
  assert.ok(Math.abs(actual - expected) <= tolerance, `${actual} != ${expected}`);
};
const lab = rgb => linearToOklab(rgb.map(value => srgbToLinear(value / 255)));
const rounded = value => Math.round(value * 1e4) / 1e4;

function image(width, height, pixel) {
  const data = new Uint8ClampedArray(4 * width * height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const rgba = pixel(x, y);
      data.set(rgba.length === 3 ? [...rgba, 255] : rgba, 4 * (y * width + x));
    }
  }
  return {width, height, data};
}
const solid = (width, height, rgb) => image(width, height, () => rgb);
const grey = value => solid(64, 64, [value, value, value]);

function palette(description) {
  const {lightness, contrast, chroma, chromaticShare, hueMean, hueSpread, hueCount, warmShare} = description;
  return {lightness, contrast, chroma, chromaticShare, hueMean, hueSpread, hueCount, warmShare};
}

function hueRgb(degrees) {
  const angle = degrees * Math.PI / 180;
  return oklabToLinear([0.65, 0.08 * Math.cos(angle), 0.08 * Math.sin(angle)])
    .map(value => Math.round(255 * linearToSrgb(value)));
}

test("describeImage rejects invalid dimensions, data types and lengths", () => {
  const valid = solid(8, 8, [128, 128, 128]);
  for (const invalid of [null, undefined, 5, {}, [],
    ...[7, 1025, 8.5, "8", NaN, Infinity].flatMap(size => [
      {...valid, width: size}, {...valid, height: size},
    ]),
    {...valid, data: Array.from(valid.data)},
    {...valid, data: new Uint16Array(256)},
    {...valid, data: new Int8Array(256)},
    {...valid, data: new Float32Array(256)},
    {...valid, data: new Uint8Array(255)},
    {...valid, data: new Uint8ClampedArray(257)},
    {...valid, data: null},
  ]) {
    assert.throws(() => describeImage(invalid), RangeError);
  }
  assert.deepEqual(describeImage({...valid, data: new Uint8Array(valid.data)}), describeImage(valid));
  assert.equal(describeImage(solid(1024, 8, [128, 128, 128])).version, 1);
});

test("alpha 128 counts as opaque and fewer than 64 opaque pixels are rejected", () => {
  assert.throws(() => describeImage(solid(8, 8, [255, 0, 0, 127])), {
    name: "RangeError", message: "too few opaque pixels",
  });
  const sparse = solid(8, 8, [128, 128, 128, 128]);
  sparse.data[3] = 127;
  assert.throws(() => describeImage(sparse), {name: "RangeError", message: "too few opaque pixels"});
  sparse.data[3] = 128;
  assert.equal(describeImage(sparse).version, 1);
});

test("uniform grey has neutral palette and no directional structure", () => {
  const description = describeImage(grey(128));
  assert.equal(description.version, 1);
  assert.ok(description.chroma.p50 < 0.005);
  assert.equal(description.chromaticShare, 0);
  assert.equal(description.hueMean, null);
  assert.equal(description.hueSpread, null);
  assert.equal(description.hueCount, 0);
  assert.equal(description.warmShare, null);
  assert.equal(description.contrast, 0);
  assert.equal(description.edgeDensity, 0);
  assert.deepEqual(description.symmetry, {mirrorX: 1, mirrorY: 1, rot180: 1});
  assert.deepEqual(description.orientation, {coherence: 0, angle: null});
  close(description.lightness.p50, lab([128, 128, 128])[0], 1e-3);
});

test("red and blue halves have two hues, chroma-weighted warmth and vertical stripes", () => {
  const description = describeImage(image(64, 64, x => x < 32 ? [255, 0, 0] : [0, 0, 255]));
  const redChroma = Math.hypot(...lab([255, 0, 0]).slice(1));
  const blueChroma = Math.hypot(...lab([0, 0, 255]).slice(1));
  assert.equal(description.chromaticShare, 1);
  assert.equal(description.hueCount, 2);
  assert.ok(description.hueSpread > 60);
  close(description.warmShare, redChroma / (redChroma + blueChroma), 1e-3);
  assert.ok(description.symmetry.mirrorX < 0.5);
  assert.equal(description.symmetry.mirrorY, 1);
  assert.ok(description.orientation.coherence > 0.9);
  assert.equal(description.orientation.angle, 90);
});

test("horizontal black and white bands have horizontal orientation", () => {
  const description = describeImage(image(64, 64, (x, y) => {
    const value = Math.floor(y / 4) % 2 ? 255 : 0;
    return [value, value, value];
  }));
  assert.ok(description.orientation.coherence > 0.9);
  close(description.orientation.angle, 0, 1);
  assert.equal(description.symmetry.mirrorX, 1);
});

test("diagonal orientation respects x rightwards and y downwards", () => {
  for (const [slope, angle] of [[1, 135], [-1, 45]]) {
    const description = describeImage(image(32, 32, (x, y) => {
      const value = 4 * (x + slope * y + (slope === -1 ? 31 : 0));
      return [value, value, value];
    }));
    assert.equal(description.orientation.coherence, 1);
    assert.equal(description.orientation.angle, angle);
  }
});

test("a seeded image mirrored across x has exact left-right symmetry", () => {
  const rng = createRng(48);
  const left = Array.from({length: 32 * 64}, () => [rng.int(256), rng.int(256), rng.int(256)]);
  const description = describeImage(image(64, 64, (x, y) => left[y * 32 + Math.min(x, 63 - x)]));
  close(description.symmetry.mirrorX, 1, 1e-9);
  assert.ok(description.symmetry.mirrorY < 1);
});

test("smooth images retain edge density and hue count after 2 by 2 box downscaling", () => {
  const rng = createRng(73);
  const phases = Array.from({length: 3}, () => rng.next() * 2 * Math.PI);
  const source = image(256, 128, (x, y) => [
    170 + 35 * Math.sin(x / 32 + phases[0]) * Math.cos(y / 28),
    70 + 18 * Math.sin(x / 40 + y / 35 + phases[1]),
    50 + 15 * Math.cos(x / 36 - y / 24 + phases[2]),
  ]);
  const smaller = image(128, 64, (x, y) => [0, 1, 2].map(channel => {
    let sum = 0;
    for (let dy = 0; dy < 2; dy++) {
      for (let dx = 0; dx < 2; dx++) {
        sum += source.data[4 * ((2 * y + dy) * source.width + 2 * x + dx) + channel];
      }
    }
    return Math.round(sum / 4);
  }));
  const a = describeImage(source);
  const b = describeImage(smaller);
  assert.ok(a.edgeDensity > 0);
  assert.ok(Math.abs(a.edgeDensity - b.edgeDensity) / Math.max(a.edgeDensity, b.edgeDensity) <= 0.15);
  assert.equal(a.hueCount, b.hueCount);
  assert.ok(a.hueCount > 0);
});

test("transparent colours do not affect palette attributes", () => {
  const left = (x, y) => y < 32 ? [255, 0, 0] : [0, 0, 255];
  const expected = palette(describeImage(image(32, 64, left)));
  for (const hidden of [[0, 255, 0, 0], [255, 255, 255, 127]]) {
    const description = describeImage(image(64, 64, (x, y) => x < 32 ? left(x, y) : hidden));
    assert.deepEqual(palette(description), expected);
  }
});

test("the opaque border frame describes the background rather than a central square", () => {
  const description = describeImage(image(100, 100, (x, y) =>
    x >= 25 && x < 75 && y >= 25 && y < 75 ? [255, 0, 0] : [255, 255, 255]));
  close(description.background.L, lab([255, 255, 255])[0], 1e-3);
  assert.ok(description.background.C < 0.01);
  close(description.lightness.p90, lab([255, 255, 255])[0], 1e-3);
});

test("background is null below 16 opaque frame pixels", () => {
  const interior = (x, y) => x >= 8 && x < 16 && y >= 8 && y < 16;
  for (const frameCount of [15, 16]) {
    const description = describeImage(image(32, 32, (x, y) =>
      interior(x, y) || (y === 0 && x < frameCount) ? [255, 255, 255] : [255, 0, 0, 0]));
    assert.deepEqual(description.background, frameCount === 15 ? null : {L: 1, C: 0});
  }
});

test("percentiles use floor rank and contrast is derived before rounding", () => {
  const description = describeImage(image(8, 8, (x, y) => {
    const value = 4 * (y * 8 + x);
    return [value, value, value];
  }));
  const p10 = lab([24, 24, 24])[0];
  const p50 = lab([124, 124, 124])[0];
  const p90 = lab([224, 224, 224])[0];
  assert.deepEqual(description.lightness, {p10: rounded(p10), p50: rounded(p50), p90: rounded(p90)});
  assert.equal(description.contrast, rounded(p90 - p10));
});

test("Sobel density and symmetry normalisation match a single vertical step", () => {
  const description = describeImage(image(8, 8, x => x < 4 ? [0, 0, 0] : [255, 255, 255]));
  close(description.edgeDensity, 1 / 6);
  assert.deepEqual(description.symmetry, {mirrorX: 0, mirrorY: 1, rot180: 0});
  assert.deepEqual(description.orientation, {coherence: 1, angle: 90});
});

test("cells without opaque pixels use the global lightness mean", () => {
  const filled = describeImage(image(24, 8, x => x < 8 ? [0, 0, 0] : x < 16 ? [255, 255, 255] : [99, 0, 234, 0]));
  close(filled.edgeDensity, 1.5 / 22);
  assert.deepEqual(filled.orientation, {coherence: 1, angle: 90});
  assert.equal(filled.symmetry.mirrorY, 1);
});

test("a very thin reduced map has no interior gradients", () => {
  const description = describeImage(image(1024, 8, x => x < 512 ? [0, 0, 0] : [255, 255, 255]));
  assert.equal(description.edgeDensity, 0);
  assert.deepEqual(description.orientation, {coherence: 0, angle: null});
  assert.equal(description.symmetry.mirrorY, 1);
});

test("adjacent hues across the circular boundary form one peak", () => {
  const colours = [hueRgb(15), hueRgb(345)];
  const description = describeImage(image(64, 64, x => colours[x % 2]));
  assert.equal(description.hueCount, 1);
  assert.ok(description.hueMean < 5 || description.hueMean > 355);
  assert.equal(description.warmShare, 1);
});

test("nearly flat hue histograms have twelve peaks", () => {
  const colours = Array.from({length: 12}, (_, index) => hueRgb(15 + 30 * index));
  const description = describeImage(image(96, 8, x => colours[x % 12]));
  assert.equal(description.hueCount, 12);
  assert.ok(description.hueSpread > 150);
});

test("hue measures use the unrounded chromatic share at the one percent cutoff", () => {
  for (const count of [40, 41]) {
    const description = describeImage(image(64, 64, (x, y) =>
      y * 64 + x < count ? [255, 0, 0] : [128, 128, 128]));
    assert.equal(description.chromaticShare, rounded(count / 4096));
    assert.equal(description.hueCount, count === 40 ? 0 : 1);
    assert.equal(description.hueMean === null, count === 40);
    assert.equal(description.hueSpread === null, count === 40);
    assert.equal(description.warmShare, count === 40 ? null : 1);
  }
  assert.equal(CHROMA_MIN, 0.04);
});

test("compareImages reports lighter liked images and the associated G3 hint", () => {
  const liked = describeImage(grey(200));
  const disliked = describeImage(grey(80));
  const comparison = compareImages(liked, disliked);
  assert.deepEqual(comparison.differences.find(entry => entry.measure === "lightness.p50"), {
    measure: "lightness.p50", liked: liked.lightness.p50, disliked: disliked.lightness.p50,
    delta: rounded(liked.lightness.p50 - disliked.lightness.p50), direction: "higher",
  });
  assert.deepEqual(comparison.hints.find(entry => entry.param === "lightness"), {
    param: "lightness", direction: "higher", from: "lightness.p50",
  });
  assert.deepEqual(compareImages(liked, liked), {differences: [], hints: []});
});

test("comparison thresholds are inclusive, ordered and use relative edge density", () => {
  const disliked = describeImage(grey(128));
  const liked = structuredClone(disliked);
  Object.assign(disliked, {hueSpread: 0, hueCount: 0, warmShare: 0, edgeDensity: 0.003});
  Object.assign(liked, {
    lightness: {...liked.lightness, p50: 0.75}, contrast: 0.08,
    chroma: {...liked.chroma, p50: 0.02}, chromaticShare: 0.10,
    hueSpread: 15, hueCount: 1, warmShare: 0.15,
    background: {...liked.background, L: 0.75}, edgeDensity: 0.004,
    symmetry: {mirrorX: 0.75, mirrorY: 0.75, rot180: 0.75},
    orientation: {coherence: 0.15, angle: 0},
  });
  const measures = ["lightness.p50", "contrast", "chroma.p50", "chromaticShare", "hueSpread",
    "hueCount", "warmShare", "background.L", "edgeDensity", "symmetry.mirrorX",
    "symmetry.mirrorY", "symmetry.rot180", "orientation.coherence"];
  const comparison = compareImages(liked, disliked);
  assert.deepEqual(comparison.differences.map(entry => entry.measure), measures);
  assert.deepEqual(comparison.hints, [
    {param: "lightness", direction: "higher", from: "lightness.p50"},
    {param: "chroma", direction: "higher", from: "chroma.p50"},
    {param: "hueSpread", direction: "higher", from: "hueSpread"},
    {param: "bgLightness", direction: "higher", from: "background.L"},
  ]);
  assert.equal(comparison.differences.find(entry => entry.measure === "edgeDensity").delta, 0.001);
  assert.equal(comparison.differences.find(entry => entry.measure === "symmetry.mirrorX").direction, "lower");
  liked.edgeDensity = 0.0039;
  assert.ok(!compareImages(liked, disliked).differences.some(entry => entry.measure === "edgeDensity"));
  assert.ok(Object.isFrozen(THRESHOLDS));
  assert.ok(Object.isFrozen(HINT_PARAMS));
});

test("null hue and background measures are skipped and hueMean is never compared", () => {
  const achromatic = describeImage(grey(128));
  const chromatic = describeImage(solid(64, 64, [255, 0, 0]));
  chromatic.background = null;
  for (const comparison of [compareImages(achromatic, chromatic), compareImages(chromatic, achromatic)]) {
    assert.ok(!comparison.differences.some(entry => ["hueSpread", "warmShare", "background.L"].includes(entry.measure)));
  }
  const changedHue = structuredClone(chromatic);
  changedHue.hueMean = (changedHue.hueMean + 120) % 360;
  assert.deepEqual(compareImages(chromatic, changedHue), {differences: [], hints: []});
});

test("comparison includes exact decimal thresholds despite binary subtraction", () => {
  const disliked = describeImage(grey(128));
  disliked.lightness.p50 = 0.4;
  const liked = structuredClone(disliked);
  liked.lightness.p50 = 0.45;
  liked.symmetry.mirrorX = 0.9;
  assert.deepEqual(compareImages(liked, disliked).differences, [
    {measure: "lightness.p50", liked: 0.45, disliked: 0.4, delta: 0.05, direction: "higher"},
    {measure: "symmetry.mirrorX", liked: 0.9, disliked: 1, delta: -0.1, direction: "lower"},
  ]);
});

test("compareImages rejects wrong versions and malformed descriptions on either side", () => {
  const valid = describeImage(grey(128));
  for (const invalid of [null, undefined, [], {}, {version: 1}, {...valid, version: 2},
    {...valid, contrast: NaN}, {...valid, edgeDensity: Infinity},
    {...valid, contrast: -0.1}, {...valid, edgeDensity: -0.1},
    {...valid, chromaticShare: 1.1}, {...valid, hueCount: 1.5},
    {...valid, hueMean: 360}, {...valid, hueSpread: 181}, {...valid, warmShare: -0.1},
    {...valid, chromaticShare: "0"}, {...valid, hueMean: undefined},
    {...valid, lightness: {p10: 0, p50: 0}}, {...valid, chroma: null},
    {...valid, symmetry: {mirrorX: 1, mirrorY: 1}},
    {...valid, orientation: {coherence: 0, angle: "90"}},
    {...valid, orientation: {coherence: 1, angle: 180}},
    {...valid, background: {L: 0.5}},
  ]) {
    assert.throws(() => compareImages(invalid, valid), RangeError);
    assert.throws(() => compareImages(valid, invalid), RangeError);
  }
  assert.deepEqual(compareImages(JSON.parse(JSON.stringify(valid)), valid), {differences: [], hints: []});
});

test("descriptions are deterministic, rounded JSON data and contain no input pixels", () => {
  const rng = createRng(17);
  const source = image(16, 16, () => [rng.int(256), rng.int(256), rng.int(256)]);
  const pixelsBefore = source.data.slice();
  const description = describeImage(source);
  assert.deepEqual(description, JSON.parse(JSON.stringify(description)));
  assert.deepEqual(description, describeImage(source));
  assert.deepEqual(source.data, pixelsBefore);
  assert.deepEqual(Object.keys(description), ["version", "lightness", "contrast", "chroma", "chromaticShare",
    "hueMean", "hueSpread", "hueCount", "warmShare", "background", "edgeDensity", "symmetry", "orientation"]);
  function check(value) {
    if (typeof value === "number") {
      assert.ok(Number.isFinite(value));
      assert.equal(value, rounded(value));
    } else if (value !== null) {
      assert.equal(Object.getPrototypeOf(value), Object.prototype);
      Object.values(value).forEach(check);
    }
  }
  check(description);
  for (const value of [[0, 0, 0], [255, 255, 255], [255, 0, 0]]) {
    const uniform = describeImage(solid(8, 8, value));
    assert.deepEqual(uniform, JSON.parse(JSON.stringify(uniform)));
  }
});

test("a 256 by 256 seeded noise fixture is described in under two seconds", () => {
  const rng = createRng(91);
  const source = image(256, 256, () => [rng.int(256), rng.int(256), rng.int(256)]);
  const start = performance.now();
  const description = describeImage(source);
  const elapsed = performance.now() - start;
  assert.equal(description.version, 1);
  assert.ok(elapsed < 2000, `description took ${elapsed.toFixed(1)} ms`);
});
