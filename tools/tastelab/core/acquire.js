import {predict, observations, outcomeProbabilities, logPhi, createJointSampler} from './gp.js';

const distance = (a, b) => Math.sqrt(a.reduce((sum, x, i) => sum + (x - b[i]) ** 2, 0));
const entropy = probabilities => Object.values(probabilities).reduce((sum, p) => sum - (p > 0 ? p * Math.log(p) : 0), 0);
const argmax = values => {
  let best = -1;
  for (let i = 0; i < values.length; i++) if (best < 0 || values[i] > values[best]) best = i;
  return best;
};
const sceneNumber = scene => typeof scene === 'string' ? ['solving', 'inspecting', 'celebrating'].indexOf(scene) : scene;

// Equal-weight normal quantiles, computed once for deterministic quadrature.
const quantiles = Array.from({length: 16}, (_, k) => {
  const probability = (k + 0.5) / 16;
  let low = -8, high = 8;
  for (let i = 0; i < 60; i++) {
    const middle = (low + high) / 2;
    if (Math.exp(logPhi(middle)) < probability) low = middle;
    else high = middle;
  }
  return (low + high) / 2;
});
export function mutualInfo(meanDiff, variance, eps) {
  if (variance <= 1e-14) return 0;
  const average = {A: 0, B: 0, same: 0};
  let conditionalEntropy = 0;
  const sd = Math.sqrt(variance);
  for (const z of quantiles) {
    const probabilities = outcomeProbabilities(meanDiff + sd * z, eps);
    for (const key of Object.keys(average)) average[key] += probabilities[key] / quantiles.length;
    conditionalEntropy += entropy(probabilities) / quantiles.length;
  }
  return Math.max(0, entropy(average) - conditionalEntropy);
}
export function expectedInfo(model, pointA, pointB) {
  const {mean, cov} = predict(model, [pointA, pointB]);
  return mutualInfo(mean[0] - mean[1], cov[0] + cov[3] - 2 * cov[1],
    model._posterior?.hyper.eps ?? model.hyper.eps);
}

// `prediction` may carry predict(model, candidates) when the caller already has it,
// so that one round computes the candidates' joint posterior once.
const checkedPrediction = (prediction, count) => {
  if (!prediction?.mean || !prediction?.cov || prediction.mean.length !== count || prediction.cov.length !== count * count) {
    throw new RangeError('prediction does not match the candidates');
  }
  return prediction;
};
export function nextPair(model, candidates, rng, {
  anchorRate = 0.5, freePairs = 3000, rejected = [], rejectRadius = 0.1, prediction: given = null,
} = {}) {
  if (!Number.isFinite(anchorRate) || anchorRate < 0 || anchorRate > 1 ||
      !Number.isInteger(freePairs) || freePairs < 0 || !Number.isFinite(rejectRadius) || rejectRadius < 0) {
    throw new RangeError('acquisition options');
  }
  const rejectedFeatures = rejected.map(look => {
    const features = typeof look === 'number' ? model.looks[look]?.features : look?.features ?? look;
    if (!features || features.length !== model.dim || Array.from(features).some(x => !Number.isFinite(x))) {
      throw new RangeError('rejected look');
    }
    return features;
  });
  const excluded = point => rejectedFeatures.some(features => distance(point.features, features) <= rejectRadius);
  // Validate every point, including excluded ones, without modifying the model.
  const prediction = given ? checkedPrediction(given, candidates.length) : predict(model, candidates);
  const eligible = candidates.map((_, i) => i).filter(i => !excluded(candidates[i]));
  if (eligible.length < 2) throw new RangeError('at least two non-rejected candidates required');
  // A round compares looks in one scene. If multiple scenes are supplied,
  // choose the scene with the strongest available posterior-mean incumbent.
  const viable = eligible.filter(i => eligible.some(j => j !== i &&
    sceneNumber(candidates[j].scene) === sceneNumber(candidates[i].scene)));
  if (!viable.length) throw new RangeError('two candidates in the same scene required');
  const anchor = viable.reduce((best, i) => prediction.mean[i] > prediction.mean[best] ? i : best, viable[0]);
  const scene = sceneNumber(candidates[anchor].scene);
  const pool = eligible.filter(i => sceneNumber(candidates[i].scene) === scene);
  const eps = model._posterior?.hyper.eps ?? model.hyper.eps;
  const {mean, cov} = prediction, count = candidates.length;
  const information = (a, b) => mutualInfo(mean[a] - mean[b],
    cov[a * count + a] + cov[b * count + b] - 2 * cov[a * count + b], eps);
  let partner = -1, bestInfo = -Infinity;
  for (const i of pool) {
    if (i === anchor) continue;
    const info = information(anchor, i);
    if (info > bestInfo) { bestInfo = info; partner = i; }
  }
  let pair = {a: anchor, b: partner, kind: 'mi'};
  if (rng.next() < anchorRate) return pair;
  for (let t = 0; t < freePairs; t++) {
    const i = rng.int(pool.length);
    let j = rng.int(pool.length - 1);
    j += j >= i;
    const a = pool[i], b = pool[j], info = information(a, b);
    if (info > bestInfo) { bestInfo = info; pair = {a, b, kind: 'free'}; }
  }
  return pair;
}

// `best` is the index of the reported best look among the candidates; by default
// the candidate with the highest posterior mean.
export function settled(model, candidates, rng, {draws = 200, tau = 0.05, prediction: given = null, best: bestIndex = null, reference} = {}) {
  if (!Number.isInteger(draws) || draws < 1 || !Number.isFinite(tau) || tau < 0 ||
      !Number.isFinite(reference) || !candidates.length) {
    throw new RangeError('settled options or candidates');
  }
  const prediction = given ? checkedPrediction(given, candidates.length) : predict(model, candidates);
  const best = bestIndex ?? argmax(prediction.mean);
  if (!Number.isInteger(best) || best < 0 || best >= candidates.length) throw new RangeError('best');
  // Fix the normalization before sampling (TL-P11).
  const range = prediction.mean[argmax(prediction.mean)] - reference;
  if (range <= 0) return {settled: false, regret: 1};
  const sample = typeof prediction.draw === 'function' ? rng => prediction.draw(rng) : createJointSampler(prediction);
  let loss = 0;
  for (let i = 0; i < draws; i++) {
    const utilities = sample(rng);
    loss += utilities[argmax(utilities)] - utilities[best];
  }
  const regret = Math.min(1, loss / draws / range);
  return {settled: observations(model).length > 0 && regret <= tau, regret};
}
