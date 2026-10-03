// Transferable palette and structure attributes: reads pixels only, never stores or returns them.
import {srgbToLinear, linearToOklab} from "../core/color.js";

export const CHROMA_MIN = 0.04;
export const THRESHOLDS = Object.freeze({
  "lightness.p50": 0.05,
  contrast: 0.08,
  "chroma.p50": 0.02,
  chromaticShare: 0.10,
  hueSpread: 15,
  hueCount: 1,
  warmShare: 0.15,
  "background.L": 0.08,
  edgeDensity: 0.25,
  "symmetry.mirrorX": 0.10,
  "symmetry.mirrorY": 0.10,
  "symmetry.rot180": 0.10,
  "orientation.coherence": 0.15,
});
export const HINT_PARAMS = Object.freeze({
  "lightness.p50": "lightness",
  "chroma.p50": "chroma",
  hueSpread: "hueSpread",
  "background.L": "bgLightness",
});

const round = value => Math.round(value * 1e4) / 1e4 || 0;
const degrees = radians => radians * 180 / Math.PI;
const clamp01 = value => Math.min(1, Math.max(0, value));
const percentile = (sorted, q) => sorted[Math.floor(q * (sorted.length - 1))];

function roundNumbers(value) {
  if (value === null) return null;
  if (typeof value === "number") return round(value);
  return Object.fromEntries(Object.entries(value).map(([key, child]) => [key, roundNumbers(child)]));
}

function huePeaks(histogram, weight) {
  const smoothed = histogram.map((value, index) =>
    (histogram[(index + 11) % 12] + 2 * value + histogram[(index + 1) % 12]) / (4 * weight));
  if (Math.max(...smoothed) - Math.min(...smoothed) < 0.01) return 12;
  let peaks = 0;
  for (let start = 0; start < 12; start++) {
    const value = smoothed[start];
    const previous = smoothed[(start + 11) % 12];
    if (Math.abs(value - previous) <= 1e-12) continue;
    let end = (start + 1) % 12;
    while (Math.abs(smoothed[end] - value) <= 1e-12) end = (end + 1) % 12;
    if (value >= 0.06 && previous < value && smoothed[end] < value) peaks++;
  }
  return peaks;
}

function structure(map, width, height) {
  const mean = map.reduce((sum, value) => sum + value, 0) / map.length;
  let deviation = 0;
  let mirrorX = 0;
  let mirrorY = 0;
  let rot180 = 0;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const index = y * width + x;
      deviation += Math.abs(map[index] - mean);
      mirrorX += Math.abs(map[index] - map[y * width + width - 1 - x]);
      mirrorY += Math.abs(map[index] - map[(height - 1 - y) * width + x]);
      rot180 += Math.abs(map[index] - map[map.length - 1 - index]);
    }
  }
  const symmetry = difference => deviation / map.length < 1e-6 ? 1 : clamp01(1 - difference / (2 * deviation));
  let density = 0;
  let count = 0;
  let sxx = 0;
  let syy = 0;
  let sxy = 0;
  for (let y = 1; y < height - 1; y++) {
    for (let x = 1; x < width - 1; x++) {
      const index = y * width + x;
      const top = index - width;
      const bottom = index + width;
      const gx = (map[top + 1] - map[top - 1] + 2 * (map[index + 1] - map[index - 1])
        + map[bottom + 1] - map[bottom - 1]) / 8;
      const gy = (map[bottom - 1] - map[top - 1] + 2 * (map[bottom] - map[top])
        + map[bottom + 1] - map[top + 1]) / 8;
      density += Math.hypot(gx, gy);
      count++;
      sxx += gx * gx;
      syy += gy * gy;
      sxy += gx * gy;
    }
  }
  // The tensor's eigenvalue sum is its trace, and their difference is this hypot.
  const trace = sxx + syy;
  const coherence = trace < 1e-12 ? 0 : clamp01(Math.hypot(sxx - syy, 2 * sxy) / trace);
  const angle = coherence < 0.1 ? null : (degrees(0.5 * Math.atan2(2 * sxy, sxx - syy)) + 90) % 180;
  return {
    edgeDensity: count ? density / count : 0,
    symmetry: {mirrorX: symmetry(mirrorX), mirrorY: symmetry(mirrorY), rot180: symmetry(rot180)},
    orientation: {coherence, angle},
  };
}

export function describeImage(image) {
  if (!image || typeof image !== "object" || Array.isArray(image)) throw new RangeError("invalid image");
  const {width, height, data} = image;
  if (!Number.isInteger(width) || width < 8 || width > 1024
    || !Number.isInteger(height) || height < 8 || height > 1024
    || !(data instanceof Uint8ClampedArray || data instanceof Uint8Array)
    || data.length !== 4 * width * height) {
    throw new RangeError("invalid image");
  }
  const scale = Math.min(1, 128 / Math.max(width, height));
  const mapWidth = Math.max(1, Math.round(width * scale));
  const mapHeight = Math.max(1, Math.round(height * scale));
  const map = new Float64Array(mapWidth * mapHeight);
  const counts = new Uint32Array(map.length);
  const lightness = [];
  const chroma = [];
  const borderLightness = [];
  const borderChroma = [];
  const thickness = Math.floor(0.05 * Math.min(width, height)) + 1;
  const histogram = new Array(12).fill(0);
  let sumL = 0;
  let chromaticCount = 0;
  let weight = 0;
  let sumA = 0;
  let sumB = 0;
  let warmWeight = 0;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const index = 4 * (y * width + x);
      if (data[index + 3] < 128) continue;
      const [L, a, b] = linearToOklab([
        srgbToLinear(data[index] / 255),
        srgbToLinear(data[index + 1] / 255),
        srgbToLinear(data[index + 2] / 255),
      ]);
      const C = Math.hypot(a, b);
      lightness.push(L);
      chroma.push(C);
      sumL += L;
      const cell = Math.floor((y + 0.5) / height * mapHeight) * mapWidth
        + Math.floor((x + 0.5) / width * mapWidth);
      map[cell] += L;
      counts[cell]++;
      if (x < thickness || x >= width - thickness || y < thickness || y >= height - thickness) {
        borderLightness.push(L);
        borderChroma.push(C);
      }
      if (C >= CHROMA_MIN) {
        const hue = (degrees(Math.atan2(b, a)) + 360) % 360;
        chromaticCount++;
        weight += C;
        sumA += a;
        sumB += b;
        histogram[Math.floor(hue / 30)] += C;
        if (hue >= 330 || hue < 110) warmWeight += C;
      }
    }
  }
  if (lightness.length < 64) throw new RangeError("too few opaque pixels");
  const meanL = sumL / lightness.length;
  for (let index = 0; index < map.length; index++) {
    map[index] = counts[index] ? map[index] / counts[index] : meanL;
  }
  const chromaticShare = chromaticCount / lightness.length;
  let hueMean = null;
  let hueSpread = null;
  let hueCount = 0;
  let warmShare = null;
  if (chromaticShare >= 0.01) {
    const R = clamp01(Math.hypot(sumA, sumB) / weight);
    hueMean = R < 1e-9 ? null : (degrees(Math.atan2(sumB, sumA)) + 360) % 360;
    hueSpread = R < 1e-9 ? 180 : Math.min(180, degrees(Math.sqrt(-2 * Math.log(R))));
    hueCount = huePeaks(histogram, weight);
    warmShare = warmWeight / weight;
  }
  lightness.sort((a, b) => a - b);
  chroma.sort((a, b) => a - b);
  let background = null;
  if (borderLightness.length >= 16) {
    borderLightness.sort((a, b) => a - b);
    borderChroma.sort((a, b) => a - b);
    background = {L: percentile(borderLightness, 0.5), C: percentile(borderChroma, 0.5)};
  }
  const p10 = percentile(lightness, 0.1);
  const p50 = percentile(lightness, 0.5);
  const p90 = percentile(lightness, 0.9);
  const description = roundNumbers({
    version: 1,
    lightness: {p10, p50, p90},
    contrast: p90 - p10,
    chroma: {p50: percentile(chroma, 0.5), p90: percentile(chroma, 0.9)},
    chromaticShare, hueMean, hueSpread, hueCount, warmShare, background,
    ...structure(map, mapWidth, mapHeight),
  });
  // Rounding can reach the excluded endpoint of a circular interval.
  if (description.hueMean !== null) description.hueMean %= 360;
  if (description.orientation.angle !== null) description.orientation.angle %= 180;
  return description;
}

const record = value => value !== null && typeof value === "object" && !Array.isArray(value);
const numberInRange = (value, min = 0, max = Infinity) => Number.isFinite(value) && value >= min && value <= max;
const hasNumbers = (value, keys, min = 0, max = Infinity) =>
  record(value) && keys.every(key => numberInRange(value[key], min, max));

function validDescription(value) {
  return record(value) && value.version === 1
    && hasNumbers(value.lightness, ["p10", "p50", "p90"], 0, 1)
    && numberInRange(value.contrast, 0, 1)
    && hasNumbers(value.chroma, ["p50", "p90"])
    && numberInRange(value.chromaticShare, 0, 1)
    && (value.hueMean === null || (numberInRange(value.hueMean, 0, 360) && value.hueMean < 360))
    && (value.hueSpread === null || numberInRange(value.hueSpread, 0, 180))
    && Number.isInteger(value.hueCount) && numberInRange(value.hueCount, 0, 12)
    && (value.warmShare === null || numberInRange(value.warmShare, 0, 1))
    && (value.background === null || (hasNumbers(value.background, ["L"], 0, 1) && hasNumbers(value.background, ["C"])))
    && numberInRange(value.edgeDensity)
    && hasNumbers(value.symmetry, ["mirrorX", "mirrorY", "rot180"], 0, 1)
    && hasNumbers(value.orientation, ["coherence"], 0, 1)
    && (value.orientation.angle === null || (numberInRange(value.orientation.angle, 0, 180) && value.orientation.angle < 180));
}

function measureValue(description, measure) {
  return measure.split(".").reduce((value, key) => value === null ? null : value[key], description);
}

export function compareImages(liked, disliked) {
  if (!validDescription(liked) || !validDescription(disliked)) throw new RangeError("invalid image description");
  const differences = [];
  const hints = [];
  for (const [measure, threshold] of Object.entries(THRESHOLDS)) {
    const a = measureValue(liked, measure);
    const b = measureValue(disliked, measure);
    if (a === null || b === null) continue;
    const delta = a - b;
    const magnitude = measure === "edgeDensity" ? Math.abs(delta) / Math.max(a, b, 1e-9) : Math.abs(delta);
    // Four-decimal inputs can land just below an equal threshold in binary arithmetic.
    if (magnitude < threshold && threshold - magnitude > Number.EPSILON) continue;
    const direction = a > b ? "higher" : "lower";
    differences.push({measure, liked: a, disliked: b, delta: round(delta), direction});
    if (Object.hasOwn(HINT_PARAMS, measure)) hints.push({param: HINT_PARAMS[measure], direction, from: measure});
  }
  return {differences, hints};
}
