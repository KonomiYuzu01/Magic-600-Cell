import {predict, observations, outcomeProbabilities, createJointSampler} from './gp.js';

const distance = (a, b) => Math.sqrt(a.reduce((sum, x, i) => sum + (x - b[i]) ** 2, 0));
const entropy = probabilities => Object.values(probabilities).reduce((sum, p) => sum - (p > 0 ? p * Math.log(p) : 0), 0);
const argmax = (values, exclude = -1) => {
  let best = -1;
  for (let i = 0; i < values.length; i++) if (i !== exclude && (best < 0 || values[i] > values[best])) best = i;
  return best;
};
const sceneNumber = scene => typeof scene === 'string' ? ['solving', 'inspecting', 'celebrating'].indexOf(scene) : scene;

function information(mean, variance, eps, rng, samples) {
  if (variance <= 1e-14) return 0;
  const average = {A: 0, B: 0, same: 0};
  let conditionalEntropy = 0;
  const sd = Math.sqrt(Math.max(0, variance));
  for (let i = 0; i < samples; i++) {
    const probabilities = outcomeProbabilities(mean + sd * rng.normal(), eps);
    for (const key of Object.keys(average)) average[key] += probabilities[key] / samples;
    conditionalEntropy += entropy(probabilities) / samples;
  }
  return Math.max(0, entropy(average) - conditionalEntropy);
}
export function expectedInfo(model, pointA, pointB, rng, samples = 64) {
  if (!Number.isInteger(samples) || samples < 1) throw new RangeError('samples');
  const {mean, cov} = predict(model, [pointA, pointB]);
  return information(mean[0] - mean[1], cov[0] + cov[3] - 2 * cov[1],
    model._posterior?.hyper.eps ?? model.hyper.eps, rng, samples);
}

export function nextPair(model, candidates, rng, {
  infoRate = 0.2, repeatRate = 0.05, rejected = [], rejectRadius = 0.1,
} = {}) {
  if (![infoRate, repeatRate, rejectRadius].every(x => Number.isFinite(x) && x >= 0) ||
      infoRate + repeatRate > 1) throw new RangeError('acquisition options');
  const rejectedFeatures = rejected.map(look => {
    const features = typeof look === 'number' ? model.looks[look]?.features : look?.features ?? look;
    if (!features || features.length !== model.dim || Array.from(features).some(x => !Number.isFinite(x))) {
      throw new RangeError('rejected look');
    }
    return features;
  });
  const excluded = point => rejectedFeatures.some(features => distance(point.features, features) <= rejectRadius);
  // Validate every point, including excluded ones, without modifying the model.
  const prediction = predict(model, candidates);
  const eligible = candidates.map((_, i) => i).filter(i => !excluded(candidates[i]));
  const scenes = new Set(eligible.map(i => sceneNumber(candidates[i].scene)));
  const roll = rng.next();
  const repeats = observations(model).filter(o =>
    model.looks[o.a].scene === model.looks[o.b].scene &&
    scenes.has(model.looks[o.a].scene) &&
    !excluded(model.looks[o.a]) && !excluded(model.looks[o.b]));
  if (roll < repeatRate && repeats.length) {
    const {a, b} = repeats[rng.int(repeats.length)];
    return {a, b, kind: 'repeat', lookA: a, lookB: b};
  }
  if (eligible.length < 2) throw new RangeError('at least two non-rejected candidates required');
  // A round compares looks in one scene. If multiple scenes are supplied,
  // choose the scene with the strongest available posterior-mean incumbent.
  const viable = eligible.filter(i => eligible.some(j => j !== i &&
    sceneNumber(candidates[j].scene) === sceneNumber(candidates[i].scene)));
  if (!viable.length) throw new RangeError('two candidates in the same scene required');
  let anchor = viable.reduce((best, i) => prediction.mean[i] > prediction.mean[best] ? i : best, viable[0]);
  const scene = sceneNumber(candidates[anchor].scene);
  const pool = eligible.filter(i => sceneNumber(candidates[i].scene) === scene);
  // Prefer the best previously shown look when it is in the round's pool.
  // The caller includes its incumbent to guarantee that it can be returned as
  // a candidate index; otherwise the pool's highest mean is the incumbent.
  const shown = model.looks.filter(p => p.scene === scene && !excluded(p));
  if (shown.length) {
    const means = predict(model, shown).mean;
    const best = shown[argmax(means)];
    const index = pool.find(i => distance(candidates[i].features, best.features) < 1e-12);
    if (index !== undefined) anchor = index;
  }
  const localAnchor = pool.indexOf(anchor);
  const mean = Float64Array.from(pool, i => prediction.mean[i]);
  const cov = new Float64Array(pool.length ** 2);
  for (let i = 0; i < pool.length; i++) {
    for (let j = 0; j < pool.length; j++) cov[i * pool.length + j] = prediction.cov[pool[i] * candidates.length + pool[j]];
  }
  const eps = model._posterior?.hyper.eps ?? model.hyper.eps;
  if (roll >= repeatRate && roll < repeatRate + infoRate) {
    let best = -1;
    let bestInfo = -Infinity;
    for (let i = 0; i < pool.length; i++) {
      if (i === localAnchor) continue;
      const variance = cov[localAnchor * pool.length + localAnchor] + cov[i * pool.length + i] -
        2 * cov[localAnchor * pool.length + i];
      const info = information(mean[localAnchor] - mean[i], variance, eps, rng, 64);
      if (info > bestInfo) { bestInfo = info; best = i; }
    }
    return {a: anchor, b: pool[best], kind: 'info'};
  }
  const draw = createJointSampler({mean, cov})(rng);
  return {a: anchor, b: pool[argmax(draw, localAnchor)], kind: 'thompson'};
}

export function settled(model, candidates, rng, {draws = 200, prob = 0.9} = {}) {
  if (!Number.isInteger(draws) || draws < 1 || !Number.isFinite(prob) || prob < 0 || prob > 1 || !candidates.length) {
    throw new RangeError('settled options or candidates');
  }
  const prediction = predict(model, candidates);
  const best = argmax(prediction.mean);
  const sample = createJointSampler(prediction);
  let wins = 0;
  for (let i = 0; i < draws; i++) {
    const utilities = sample(rng);
    if (utilities.every((value, j) => j === best || utilities[best] > value)) wins++;
  }
  const probability = wins / draws;
  return {settled: observations(model).length > 0 && probability >= prob, prob: probability};
}
