import {mapToGamut, deltaE, CVD_KINDS, simulateCvd, linearToOklab} from './color.js';

export const SCENES = Object.freeze(['solving', 'inspecting', 'celebrating']);
const PARAMS = [
  ['hueRotation', 'Palette', 0, 360, 'circular'],
  ['hueSpread', 'Palette', 60, 360, 'linear'],
  ['lightness', 'Palette', 0.45, 0.85, 'linear'],
  ['lightnessAlt', 'Palette', 0, 0.2, 'linear'],
  ['chroma', 'Palette', 0.04, 0.20, 'linear'],
  ['classes', 'Palette', 4, 8, 'integer'],
  ['bgLightness', 'Background', 0.05, 0.95, 'linear'],
  ['bgHue', 'Background', 0, 360, 'circular'],
  ['bgTint', 'Background', 0, 0.05, 'linear'],
  ['gap', 'Geometry', 0, 0.3, 'linear'],
  ['edgeWeight', 'Geometry', 0, 3, 'linear'],
  ['edgeBrightness', 'Geometry', 0, 1, 'linear'],
  ['gloss', 'Material', 0, 1, 'linear'],
  ['glow', 'Material', 0, 1, 'linear'],
  ['fog', 'Material', 0, 1, 'linear'],
  ['turnMs', 'Motion', 150, 900, 'linear'],
  ['easeA', 'Motion', 0, 1, 'linear'],
  ['easeB', 'Motion', 0, 1, 'linear'],
];
export function defaultSpace() {
  return {params: PARAMS.map(([id, group, min, max, kind]) => ({id, group, min, max, kind}))};
}

export function validateSpace(space) {
  if (!space || !Array.isArray(space.params) || !space.params.length) throw new TypeError('params required');
  const ids = new Set();
  for (const p of space.params) {
    if (!p || typeof p.id !== 'string' || !p.id || ids.has(p.id) ||
        typeof p.group !== 'string' || !p.group || !Number.isFinite(p.min) ||
        !Number.isFinite(p.max) || p.min >= p.max || !['linear', 'integer', 'circular'].includes(p.kind) ||
        (p.kind === 'integer' && (!Number.isInteger(p.min) || !Number.isInteger(p.max))) ||
        (p.kind === 'circular' && (p.min !== 0 || p.max !== 360))) {
      throw new TypeError('malformed parameter');
    }
    ids.add(p.id);
  }
}

export function canonical(space, look) {
  validateSpace(space);
  const result = {...look};
  for (const p of space.params) {
    if (!Number.isFinite(look?.[p.id])) throw new TypeError(`invalid ${p.id}`);
    if (p.kind === 'circular') result[p.id] = ((look[p.id] % 360) + 360) % 360;
    if (p.kind === 'integer') result[p.id] = Math.round(look[p.id]);
  }
  return result;
}
export function featureDim(space) {
  validateSpace(space);
  return space.params.reduce((n, p) => n + (p.kind === 'circular' ? 2 : 1), 0);
}
export function encode(space, look) {
  const values = canonical(space, look);
  const features = [];
  for (const p of space.params) {
    if (p.kind === 'circular') {
      const h = values[p.id] * Math.PI / 180;
      features.push(Math.cos(h) / (2 * Math.PI), Math.sin(h) / (2 * Math.PI));
    } else features.push((values[p.id] - p.min) / (p.max - p.min));
  }
  return Float64Array.from(features);
}

// First 32 dimensions of Joe–Kuo D(6), new-joe-kuo-6.21201.
// Each row is [degree, polynomial coefficients, initial odd numerators].
// https://web.maths.unsw.edu.au/~fkuo/sobol/
const DIRECTIONS = [
  [1, 0, [1]], [2, 1, [1, 3]], [3, 1, [1, 3, 1]], [3, 2, [1, 1, 1]],
  [4, 1, [1, 1, 3, 3]], [4, 4, [1, 3, 5, 13]], [5, 2, [1, 1, 5, 5, 17]],
  [5, 4, [1, 1, 5, 5, 5]], [5, 7, [1, 1, 7, 11, 19]], [5, 11, [1, 1, 5, 1, 1]],
  [5, 13, [1, 1, 1, 3, 11]], [5, 14, [1, 3, 5, 5, 31]],
  [6, 1, [1, 3, 3, 9, 7, 49]], [6, 13, [1, 1, 1, 15, 21, 21]],
  [6, 16, [1, 3, 1, 13, 27, 49]], [6, 19, [1, 1, 1, 15, 7, 5]],
  [6, 22, [1, 3, 1, 15, 13, 25]], [6, 25, [1, 1, 5, 5, 19, 61]],
  [7, 1, [1, 3, 7, 11, 23, 15, 103]], [7, 4, [1, 3, 7, 13, 13, 15, 69]],
  [7, 7, [1, 1, 3, 13, 7, 35, 63]], [7, 8, [1, 3, 5, 9, 1, 25, 53]],
  [7, 14, [1, 3, 1, 13, 9, 35, 107]], [7, 19, [1, 3, 1, 5, 27, 61, 31]],
  [7, 21, [1, 1, 5, 11, 19, 41, 61]], [7, 28, [1, 3, 5, 3, 3, 13, 69]],
  [7, 31, [1, 1, 7, 13, 1, 19, 1]], [7, 32, [1, 3, 7, 5, 13, 19, 59]],
  [7, 37, [1, 1, 3, 9, 25, 29, 41]], [7, 41, [1, 3, 5, 13, 23, 1, 55]],
  [7, 42, [1, 3, 7, 3, 13, 59, 17]],
];
export function sobol(n, dim, skip = 0) {
  if (!Number.isSafeInteger(n) || n < 0 || !Number.isSafeInteger(skip) || skip < 0 ||
      n + skip > 4294967296 || !Number.isInteger(dim) || dim < 1 || dim > 32) {
    throw new RangeError('invalid Sobol size, dimension or skip');
  }
  const directions = Array.from({length: dim}, (_, d) => {
    const v = new Uint32Array(32);
    if (d === 0) {
      for (let j = 0; j < 32; j++) v[j] = 1 << (31 - j);
    } else {
      const [s, a, m] = DIRECTIONS[d - 1];
      for (let j = 0; j < s; j++) v[j] = m[j] << (31 - j);
      for (let j = s; j < 32; j++) {
        v[j] = v[j - s] ^ (v[j - s] >>> s);
        for (let k = 1; k < s; k++) if ((a >>> (s - 1 - k)) & 1) v[j] ^= v[j - k];
      }
    }
    return v;
  });
  return Array.from({length: n}, (_, i) => {
    const index = i + skip;
    const gray = (index ^ (index >>> 1)) >>> 0;
    return Float64Array.from(directions, v => {
      let bits = gray;
      let value = 0;
      let j = 0;
      while (bits) {
        if (bits & 1) value ^= v[j];
        bits >>>= 1;
        j++;
      }
      return (value >>> 0) / 4294967296;
    });
  });
}

export function fromUnit(space, u) {
  validateSpace(space);
  if (!u || u.length !== space.params.length || Array.from(u).some(x => !Number.isFinite(x) || x < 0 || x >= 1)) {
    throw new RangeError('unit point required');
  }
  return Object.fromEntries(space.params.map((p, i) => [p.id, p.kind === 'integer'
    ? p.min + Math.floor(u[i] * (p.max - p.min + 1))
    : p.min + u[i] * (p.max - p.min)]));
}
export function palette(look) {
  if (!Number.isInteger(look.classes) || look.classes < 4 || look.classes > 8) throw new RangeError('classes');
  return Array.from({length: look.classes}, (_, i) => {
    const L = look.lightness + (i % 2 ? look.lightnessAlt : -look.lightnessAlt);
    const h = look.hueRotation + look.hueSpread * i / look.classes;
    return {L, C: look.chroma, h, mapped: mapToGamut(L, look.chroma, h)};
  });
}
export const background = look => mapToGamut(look.bgLightness, look.bgTint, look.bgHue);

export function hardCheck(look, classPairs, thresholds = {deltaE: 0.08, bgL: 0.20}) {
  const limits = {deltaE: 0.08, bgL: 0.20, ...thresholds};
  if (Object.values(limits).some(x => !Number.isFinite(x) || x < 0)) throw new RangeError('thresholds');
  const minDeltaE = {normal: Infinity, protan: Infinity, deutan: Infinity, tritan: Infinity};
  const failure = reason => ({ok: false, reasons: [reason], minDeltaE, minBgL: Infinity});
  if (!Number.isInteger(look?.classes) || look.classes < 4 || look.classes > 8) return failure('range:classes');
  const colours = palette(look).map(c => c.mapped);
  const bg = background(look);
  if (!bg.ok || colours.some(c => !c.ok)) return failure('gamut');
  const reasons = [];
  for (const p of defaultSpace().params) {
    const x = look[p.id];
    if (!Number.isFinite(x) || x < p.min || x > p.max || (p.kind === 'integer' && !Number.isInteger(x))) {
      reasons.push(`range:${p.id}`);
    }
  }
  const views = {normal: colours.map(c => c.lab)};
  for (const kind of CVD_KINDS) views[kind] = colours.map(c => linearToOklab(simulateCvd(c.linear, kind)));
  for (const pair of classPairs) {
    if (!Array.isArray(pair) || pair.length !== 2 || pair.some(i => !Number.isInteger(i) || i < 0 || i >= colours.length)) {
      throw new RangeError('class pair');
    }
    for (const [kind, labs] of Object.entries(views)) {
      const distance = deltaE(labs[pair[0]], labs[pair[1]]);
      minDeltaE[kind] = Math.min(minDeltaE[kind], distance);
      if (distance < limits.deltaE) reasons.push(`deltaE:${kind}:${pair[0]}-${pair[1]}`);
    }
  }
  const minBgL = Math.min(...colours.map(c => Math.abs(c.lab[0] - bg.lab[0])));
  if (minBgL < limits.bgL) reasons.push('background');
  return {ok: reasons.length === 0, reasons, minDeltaE, minBgL};
}
