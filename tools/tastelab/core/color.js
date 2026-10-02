// Björn Ottosson's 2021 OKLab matrices (https://bottosson.github.io/posts/oklab/).
const M1 = [
  [0.4122214708, 0.5363325363, 0.0514459929],
  [0.2119034982, 0.6806995451, 0.1073969566],
  [0.0883024619, 0.2817188376, 0.6299787005],
];
const M2 = [
  [0.2104542553, 0.7936177850, -0.0040720468],
  [1.9779984951, -2.4285922050, 0.4505937099],
  [0.0259040371, 0.7827717662, -0.8086757660],
];

function inverse3(m) {
  const [[a, b, c], [d, e, f], [g, h, i]] = m;
  const det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
  return [
    [e * i - f * h, c * h - b * i, b * f - c * e],
    [f * g - d * i, a * i - c * g, c * d - a * f],
    [d * h - e * g, b * g - a * h, a * e - b * d],
  ].map(row => row.map(x => x / det));
}
// Inverting the published forward coefficients avoids rounding error from
// independently rounded inverse coefficients in a colour round trip.
const I1 = inverse3(M1);
const I2 = inverse3(M2);
const multiply = (m, v) => m.map(row => row.reduce((sum, x, i) => sum + x * v[i], 0));

export const srgbToLinear = x => x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4;
export const linearToSrgb = x => x <= 0.0031308 ? 12.92 * x : 1.055 * x ** (1 / 2.4) - 0.055;
export const linearToOklab = rgb => multiply(M2, multiply(M1, rgb).map(Math.cbrt));
export const oklabToLinear = lab => multiply(I1, multiply(I2, lab).map(x => x ** 3));

export function oklch(L, C, hDeg) {
  const h = (((hDeg % 360) + 360) % 360) * Math.PI / 180;
  return [L, C * Math.cos(h), C * Math.sin(h)];
}

const inGamut = rgb => rgb.every(x => Number.isFinite(x) && x >= 0 && x <= 1);

export function mapToGamut(L, C, hDeg) {
  if (!Number.isFinite(L) || L < 0 || L > 1) return {ok: false, reason: 'lightness'};
  if (!Number.isFinite(C) || C < 0 || !Number.isFinite(hDeg)) {
    return {ok: false, reason: 'colour'};
  }
  const result = (chroma, linear) => ({
    ok: true, C: chroma, linear, srgb: linear.map(linearToSrgb), lab: linearToOklab(linear),
  });
  // Only zero chroma is possible at the endpoints. The rounded published
  // matrices put nominal neutral white a few 1e-7 outside RGB; use exact white.
  if (L === 0 || L === 1) return result(0, [L, L, L]);
  const neutral = oklabToLinear([L, 0, 0]);
  if (!inGamut(neutral)) return {ok: false, reason: 'gamut'};
  const requested = oklabToLinear(oklch(L, C, hDeg));
  if (inGamut(requested)) return result(C, requested);
  let low = 0;
  let high = C;
  let linear = neutral;
  for (let i = 0; i < 40; i++) {
    const mid = (low + high) / 2;
    const rgb = oklabToLinear(oklch(L, mid, hDeg));
    if (inGamut(rgb)) {
      low = mid;
      linear = rgb;
    } else high = mid;
  }
  return result(low, linear);
}

export const deltaE = (a, b) => Math.hypot(...a.map((x, i) => x - b[i]));
export const CVD_KINDS = Object.freeze(['protan', 'deutan', 'tritan']);
// Machado, Oliveira & Fernandes (2009), severity 1.0, linear RGB.
// https://www.inf.ufrgs.br/~oliveira/pubs_files/CVD_Simulation/CVD_Simulation.html
const CVD = {
  protan: [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
  deutan: [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]],
  tritan: [[1.255528, -0.076749, -0.178779], [-0.078411, 0.930809, 0.147602], [0.004733, 0.691367, 0.303900]],
};
export function simulateCvd(linear, kind) {
  if (!Object.hasOwn(CVD, kind)) throw new RangeError('unknown CVD kind');
  return multiply(CVD[kind], linear).map(x => Math.min(1, Math.max(0, x)));
}
