import {SCENES} from './space.js';

const SQRT2 = Math.SQRT2;
const LOG_SQRT_2PI = 0.5 * Math.log(2 * Math.PI);
const logPhiDensity = x => -0.5 * x * x - LOG_SQRT_2PI;
// A plain loop, in the same order as a left-to-right reduce: the posterior
// covariance of a round spends most of its time here.
function dot(a, b) {
  let sum = 0;
  for (let i = 0; i < a.length; i++) sum += a[i] * b[i];
  return sum;
}
const maxAbs = a => a.reduce((value, x) => Math.max(value, Math.abs(x)), 0);

function polynomial(x, coefficients, leadingOne = false) {
  let value = leadingOne ? 1 : 0;
  for (const coefficient of coefficients) value = value * x + coefficient;
  return value;
}
// Cephes' double-precision erf/erfc rational approximations. Evaluate erfc in
// log space in the tails so neither the likelihood nor its derivatives underflow.
const ERF_T = [9.604973739870516, 90.02601972038427, 2232.005345946843, 7003.325141128051, 55592.30130103949];
const ERF_U = [33.56171416475031, 521.3579497801527, 4594.323829709801, 22629.000061389095, 49267.39426086359];
const ERFC_P = [2.461969814735305e-10, 0.5641895648310688, 7.463210564422699, 48.63719709856814,
  196.5208329560771, 526.4451949954773, 934.5285271719576, 1027.5518868951572, 557.5353353693993];
const ERFC_Q = [13.228195115474499, 86.70721408859897, 354.9377788878199, 975.7085017432055,
  1823.9091668790973, 2246.3376081871097, 1656.6630919416135, 557.5353408177277];
const ERFC_R = [0.5641895835477551, 1.275366707599781, 5.019050422511805, 6.160210979930536,
  7.409742699504489, 2.9788666537210024];
const ERFC_S = [2.2605286322011726, 9.396035249380014, 12.048953980809666, 17.08144507475659,
  9.608968090632859, 3.369076451000815];

export function logPhi(x) {
  if (x === -Infinity) return -Infinity;
  if (x === Infinity) return 0;
  if (!Number.isFinite(x)) throw new TypeError('finite normal argument required');
  if (x > 0) return Math.log1p(-Math.exp(logPhi(-x)));
  const y = -x / SQRT2;
  if (y < 1) {
    const erf = y * polynomial(y * y, ERF_T) / polynomial(y * y, ERF_U, true);
    return Math.log1p(-erf) - Math.LN2;
  }
  const numerator = polynomial(y, y < 8 ? ERFC_P : ERFC_R);
  const denominator = polynomial(y, y < 8 ? ERFC_Q : ERFC_S, true);
  return -y * y + Math.log(numerator / denominator) - Math.LN2;
}
function logDifference(big, small) {
  return big + Math.log(-Math.expm1(small - big));
}

export function likelihoodTerms(d, eps, outcome) {
  if (!Number.isFinite(d) || !Number.isFinite(eps) || eps <= 0) throw new RangeError('likelihood arguments');
  if (outcome === 'A' || outcome === 'B') {
    const sign = outcome === 'A' ? 1 : -1;
    const t = (sign * d - eps) / SQRT2;
    const logp = logPhi(t);
    const ratio = Math.exp(logPhiDensity(t) - logp);
    return {logp, gradient: sign * ratio / SQRT2, hessian: -0.5 * ratio * (t + ratio)};
  }
  if (outcome !== 'same') throw new RangeError('outcome must be A, B or same');
  const lo = (-eps - d) / SQRT2;
  const hi = (eps - d) / SQRT2;
  // Reflect an interval in the positive tail before subtracting CDFs.
  const logp = lo >= 0
    ? logDifference(logPhi(-lo), logPhi(-hi))
    : logDifference(logPhi(hi), logPhi(lo));
  const lowRatio = Math.exp(logPhiDensity(lo) - logp);
  const highRatio = Math.exp(logPhiDensity(hi) - logp);
  const gradient = (lowRatio - highRatio) / SQRT2;
  const hessian = (lo * lowRatio - hi * highRatio) / 2 - gradient * gradient;
  return {logp, gradient, hessian};
}
export function outcomeProbabilities(d, eps = 0.2) {
  const A = Math.exp(likelihoodTerms(d, eps, 'A').logp);
  const B = Math.exp(likelihoodTerms(d, eps, 'B').logp);
  const same = Math.exp(likelihoodTerms(d, eps, 'same').logp);
  const total = A + B + same;
  return {A: A / total, B: B / total, same: same / total};
}

function checkedHyper(dim, input = {}) {
  const hyper = {
    signal: input.signal ?? 1,
    lengths: Float64Array.from(input.lengths ?? new Float64Array(dim).fill(0.5)),
    rho: input.rho ?? 0.5,
    eps: input.eps ?? 0.2,
  };
  if (!Number.isFinite(hyper.signal) || hyper.signal <= 0 || hyper.lengths.length !== dim ||
      hyper.lengths.some(x => !Number.isFinite(x) || x < 0.05 || x > 5) ||
      !Number.isFinite(hyper.rho) || hyper.rho <= 0 || hyper.rho >= 1 ||
      !Number.isFinite(hyper.eps) || hyper.eps < 0.01 || hyper.eps > 2) {
    throw new RangeError('invalid hyperparameters');
  }
  return hyper;
}
const hyperKey = h => JSON.stringify([h.signal, ...h.lengths, h.rho, h.eps]);
function checkedPoint(model, features, scene) {
  if (typeof scene === 'string') scene = SCENES.indexOf(scene);
  if (!Number.isInteger(scene) || scene < 0 || scene >= model.scenes) throw new RangeError('scene');
  if (!features || features.length !== model.dim || Array.from(features).some(x => !Number.isFinite(x))) {
    throw new RangeError('features');
  }
  return {features: Float64Array.from(features), scene};
}
export function createModel({dim, scenes = 3, hyper = {}}) {
  if (!Number.isInteger(dim) || dim < 1 || !Number.isInteger(scenes) || scenes < 1) throw new RangeError('model dimensions');
  return {dim, scenes, hyper: checkedHyper(dim, hyper), looks: [], _observations: [], _revision: 0, _posterior: null};
}
export function addLook(model, features, scene) {
  if (model.looks.length >= 400) throw new Error('capacity');
  const point = checkedPoint(model, features, scene);
  model.looks.push(point);
  model._revision++;
  return model.looks.length - 1;
}
export function addObservation(model, {a, b, outcome}) {
  if (![a, b].every(i => Number.isInteger(i) && i >= 0 && i < model.looks.length) || a === b ||
      !['A', 'B', 'same'].includes(outcome)) throw new RangeError('observation');
  model._observations.push({a, b, outcome});
  model._revision++;
}
export function removeLast(model) {
  const removed = model._observations.pop();
  if (removed) model._revision++;
  return removed ? {...removed} : undefined;
}
export const observations = model => model._observations.map(o => ({...o}));

function kernel(a, b, hyper) {
  let distance = 0;
  for (let d = 0; d < hyper.lengths.length; d++) distance += ((a.features[d] - b.features[d]) / hyper.lengths[d]) ** 2;
  return hyper.signal ** 2 * Math.exp(-0.5 * distance) * (a.scene === b.scene ? 1 : hyper.rho);
}
function covariance(points, hyper) {
  const n = points.length;
  const matrix = new Float64Array(n * n);
  for (let i = 0; i < n; i++) {
    for (let j = 0; j <= i; j++) matrix[i * n + j] = matrix[j * n + i] = kernel(points[i], points[j], hyper);
  }
  return matrix;
}
function cholesky(matrix, n) {
  if (!n) return new Float64Array();
  let scale = 0;
  for (let i = 0; i < n; i++) scale = Math.max(scale, Math.abs(matrix[i * n + i]));
  const base = Math.max(scale, 1e-12) * 1e-10;
  for (let attempt = 0; attempt < 9; attempt++) {
    const jitter = attempt === 0 ? 0 : base * 10 ** (attempt - 1);
    const L = new Float64Array(n * n);
    let ok = true;
    for (let i = 0; i < n && ok; i++) {
      for (let j = 0; j <= i; j++) {
        let value = matrix[i * n + j] + (i === j ? jitter : 0);
        for (let k = 0; k < j; k++) value -= L[i * n + k] * L[j * n + k];
        if (i === j) {
          if (!(value > 0) || !Number.isFinite(value)) { ok = false; break; }
          L[i * n + j] = Math.sqrt(value);
        } else L[i * n + j] = value / L[j * n + j];
      }
    }
    if (ok) return L;
  }
  throw new Error('covariance is not positive definite');
}
function lowerSolve(L, b, n) {
  const x = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    let value = b[i];
    for (let j = 0; j < i; j++) value -= L[i * n + j] * x[j];
    x[i] = value / L[i * n + i];
  }
  return x;
}
function upperSolve(L, b, n) {
  const x = new Float64Array(n);
  for (let i = n - 1; i >= 0; i--) {
    let value = b[i];
    for (let j = i + 1; j < n; j++) value -= L[j * n + i] * x[j];
    x[i] = value / L[i * n + i];
  }
  return x;
}
function lowerMultiply(L, x, n) {
  const y = new Float64Array(n);
  for (let i = 0; i < n; i++) for (let j = 0; j <= i; j++) y[i] += L[i * n + j] * x[j];
  return y;
}
function likelihoodAt(model, f, derivatives = false) {
  const n = f.length;
  const gradient = derivatives ? new Float64Array(n) : null;
  const W = derivatives ? new Float64Array(n * n) : null;
  let logp = 0;
  for (const {a, b, outcome} of model._observations) {
    const term = likelihoodTerms(f[a] - f[b], model.hyper.eps, outcome);
    logp += term.logp;
    if (derivatives) {
      gradient[a] += term.gradient;
      gradient[b] -= term.gradient;
      // The probit and interval likelihoods are log-concave. Clamp only
      // floating point cancellation in an otherwise nonnegative curvature.
      const w = Math.max(0, -term.hessian);
      W[a * n + a] += w;
      W[b * n + b] += w;
      W[a * n + b] -= w;
      W[b * n + a] -= w;
    }
  }
  return {logp, gradient, W};
}
function precision(L, W, n) {
  // I + L^T W L, computed in O(n^3), independent of the observation count.
  const WL = new Float64Array(n * n);
  for (let i = 0; i < n; i++) {
    for (let k = 0; k < n; k++) {
      const w = W[i * n + k];
      if (w === 0) continue;
      for (let j = 0; j <= k; j++) WL[i * n + j] += w * L[k * n + j];
    }
  }
  const H = new Float64Array(n * n);
  for (let i = 0; i < n; i++) {
    for (let j = 0; j <= i; j++) {
      let value = i === j ? 1 : 0;
      for (let k = i; k < n; k++) value += L[k * n + i] * WL[k * n + j];
      H[i * n + j] = H[j * n + i] = value;
    }
  }
  return cholesky(H, n);
}
function modeState(model, L, z) {
  const n = z.length;
  const f = lowerMultiply(L, z, n);
  const terms = likelihoodAt(model, f, true);
  const gradient = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    gradient[i] = -z[i];
    for (let j = i; j < n; j++) gradient[i] += L[j * n + i] * terms.gradient[j];
  }
  return {...terms, f, gradient, objective: terms.logp - dot(z, z) / 2};
}

export function fit(model, {maxIter = 30, tol = 1e-6} = {}) {
  if (!Number.isInteger(maxIter) || maxIter < 0 || !Number.isFinite(tol) || tol <= 0) throw new RangeError('fit options');
  checkedHyper(model.dim, model.hyper);
  const n = model.looks.length;
  const L = cholesky(covariance(model.looks, model.hyper), n);
  // Starting at zero makes undo independent of the history of warm starts.
  let z = new Float64Array(n);
  let state = modeState(model, L, z);
  let iterations = 0;
  while (maxAbs(state.gradient) > tol && iterations < maxIter) {
    const H = precision(L, state.W, n);
    const step = upperSolve(H, lowerSolve(H, state.gradient, n), n);
    const slope = dot(state.gradient, step);
    let accepted = false;
    let rate = 1;
    for (let trial = 0; trial < 25; trial++, rate *= 0.5) {
      const candidate = z.map((x, i) => x + rate * step[i]);
      const f = lowerMultiply(L, candidate, n);
      const objective = likelihoodAt(model, f).logp - dot(candidate, candidate) / 2;
      if (Number.isFinite(objective) && objective >= state.objective + 1e-4 * rate * slope - 1e-12) {
        z = candidate;
        accepted = true;
        break;
      }
    }
    iterations++;
    if (!accepted) return {converged: false, iterations};
    state = modeState(model, L, z);
  }
  if (maxAbs(state.gradient) > tol || !Number.isFinite(state.objective)) return {converged: false, iterations};
  // Commit all fit state together. A failed attempt leaves predictions intact,
  // even if new looks or observations have been appended since the last fit.
  model._posterior = {
    looks: model.looks.map(p => ({features: Float64Array.from(p.features), scene: p.scene})),
    hyper: checkedHyper(model.dim, model.hyper), revision: model._revision,
    L, H: precision(L, state.W, n), z, alpha: upperSolve(L, z, n), f: state.f,
    objective: state.objective,
  };
  return {converged: true, iterations};
}

function hyperPrior(h) {
  const logNormal = (value, median) => -Math.log(value) - (Math.log(value / median) ** 2) / 2 - LOG_SQRT_2PI;
  let value = logNormal(h.signal, 1) + logNormal(h.eps, 0.2);
  for (const length of h.lengths) value += logNormal(length, 0.5);
  const logit = Math.log(h.rho) - Math.log1p(-h.rho);
  return value - logit * logit / 2 - Math.log(h.rho) - Math.log1p(-h.rho) - LOG_SQRT_2PI;
}
export function logEvidence(model) {
  const p = model._posterior;
  if (!p || p.revision !== model._revision || hyperKey(p.hyper) !== hyperKey(model.hyper)) {
    if (!fit(model).converged) return -Infinity;
  }
  const posterior = model._posterior;
  let value = posterior.objective + hyperPrior(posterior.hyper);
  for (let i = 0; i < posterior.z.length; i++) value -= Math.log(posterior.H[i * posterior.z.length + i]);
  return value;
}

// The hyperparameter search as a resumable object: each step() runs one
// evaluation on the looks and comparisons present when the search started, so
// a worker can spread the search over idle time. adopt() applies the best
// hyperparameters; when answers arrived meanwhile, it refits on the current data.
export function createHyperSearch(model, {maxEvals = 60} = {}) {
  if (!Number.isInteger(maxEvals) || maxEvals < 0) throw new RangeError('maxEvals');
  const snapshot = {looks: model.looks.slice(), observations: model._observations.slice(), revision: model._revision};
  const parameters = h => [Math.log(h.signal), ...Array.from(h.lengths, Math.log),
    Math.log(h.rho / (1 - h.rho)), Math.log(h.eps)];
  const bound = (x, i) => i === 0 ? x : i <= model.dim
    ? Math.min(Math.log(5), Math.max(Math.log(0.05), x))
    : i === model.dim + 1 ? Math.min(36, Math.max(-36, x))
      : Math.min(Math.log(2), Math.max(Math.log(0.01), x));
  const toHyper = x => checkedHyper(model.dim, {
    signal: Math.exp(x[0]), lengths: x.slice(1, model.dim + 1).map(v => Math.exp(v)),
    rho: 1 / (1 + Math.exp(-x[model.dim + 1])), eps: Math.exp(x[model.dim + 2]),
  });
  let evaluations = 0;
  const evaluate = x => {
    evaluations++;
    const trial = createModel({dim: model.dim, scenes: model.scenes, hyper: toHyper(x)});
    trial.looks = snapshot.looks;
    trial._observations = snapshot.observations;
    trial._revision = snapshot.revision;
    const result = fit(trial);
    return {trial, value: result.converged ? logEvidence(trial) : -Infinity};
  };
  let best = null;
  // Bounded coordinate search in log/logit coordinates; the budget includes
  // the initial fit and failed trials. No unsuccessful trial changes the model.
  function* steps() {
    let x = parameters(model.hyper);
    best = evaluate(x);
    yield;
    let step = 0.8;
    while (evaluations < maxEvals && step >= 0.025) {
      let improved = false;
      for (let i = 0; i < x.length && evaluations < maxEvals; i++) {
        const base = x.slice();
        for (const sign of [1, -1]) {
          if (evaluations >= maxEvals) break;
          const candidate = base.slice();
          candidate[i] = bound(candidate[i] + sign * step, i);
          if (candidate[i] === base[i]) continue;
          const result = evaluate(candidate);
          if (result.value > best.value + 1e-9) {
            best = result;
            x = candidate;
            improved = true;
          }
          yield;
        }
      }
      if (!improved) step *= 0.5;
    }
  }
  const runner = maxEvals ? steps() : null;
  let done = !runner;
  return {
    // Runs one evaluation; false once the search has finished.
    step() {
      if (!done) done = runner.next().done;
      return !done;
    },
    get evaluations() { return evaluations; },
    adopt() {
      if (best && Number.isFinite(best.value)) {
        model.hyper = best.trial.hyper;
        if (model._revision === snapshot.revision) model._posterior = best.trial._posterior;
        else fit(model);
      }
      return evaluations;
    },
  };
}
export function fitHyper(model, {maxEvals = 60} = {}) {
  const search = createHyperSearch(model, {maxEvals});
  while (search.step());
  return search.adopt();
}

export function predict(model, points) {
  const test = points.map(p => checkedPoint(model, p.features, p.scene));
  const p = model._posterior;
  const hyper = p?.hyper ?? model.hyper;
  const count = test.length;
  const mean = new Float64Array(count);
  const cov = covariance(test, hyper);
  if (!p || !p.looks.length) return {mean, cov};
  const n = p.looks.length;
  const h = [];
  const q = [];
  for (let j = 0; j < count; j++) {
    const cross = Float64Array.from(p.looks, look => kernel(look, test[j], hyper));
    mean[j] = dot(cross, p.alpha);
    h.push(lowerSolve(p.L, cross, n));
    q.push(lowerSolve(p.H, h[j], n));
  }
  for (let i = 0; i < count; i++) {
    for (let j = 0; j <= i; j++) {
      let value = cov[i * count + j] - dot(h[i], h[j]) + dot(q[i], q[j]);
      if (i === j) value = Math.max(0, value);
      cov[i * count + j] = cov[j * count + i] = value;
    }
  }
  return {mean, cov};
}
// Posterior means only, without the covariance that predict() computes:
// enough to choose the best look.
export function predictMean(model, points) {
  const test = points.map(p => checkedPoint(model, p.features, p.scene));
  const p = model._posterior;
  const hyper = p?.hyper ?? model.hyper;
  const mean = new Float64Array(test.length);
  if (!p || !p.looks.length) return mean;
  for (let j = 0; j < test.length; j++) {
    mean[j] = dot(Float64Array.from(p.looks, look => kernel(look, test[j], hyper)), p.alpha);
  }
  return mean;
}
// Acquisition reuses this factor for hundreds of draws of the same posterior.
export function createJointSampler({mean, cov}) {
  const n = mean.length;
  if (cov.length !== n * n) throw new RangeError('covariance shape');
  const L = cholesky(cov, n);
  return rng => {
    const noise = Float64Array.from({length: n}, () => rng.normal());
    const draw = lowerMultiply(L, noise, n);
    return draw.map((x, i) => x + mean[i]);
  };
}
export const sampleJoint = (model, points, rng) => createJointSampler(predict(model, points))(rng);

const hyperJSON = h => ({...h, lengths: Array.from(h.lengths)});
const pointsJSON = points => points.map(p => ({features: Array.from(p.features), scene: p.scene}));
export function serialize(model) {
  const p = model._posterior;
  return JSON.stringify({
    version: 1, dim: model.dim, scenes: model.scenes, hyper: hyperJSON(model.hyper),
    looks: pointsJSON(model.looks), observations: observations(model), revision: model._revision,
    posterior: p ? {
      looks: pointsJSON(p.looks), hyper: hyperJSON(p.hyper), revision: p.revision,
      L: Array.from(p.L), H: Array.from(p.H), z: Array.from(p.z),
      alpha: Array.from(p.alpha), f: Array.from(p.f), objective: p.objective,
    } : null,
  });
}
export function deserialize(json) {
  const data = typeof json === 'string' ? JSON.parse(json) : json;
  if (!data || data.version !== 1 || !Array.isArray(data.looks) || !Array.isArray(data.observations)) {
    throw new TypeError('model format');
  }
  const model = createModel(data);
  for (const p of data.looks) addLook(model, p.features, p.scene);
  for (const o of data.observations) addObservation(model, o);
  if (!Number.isSafeInteger(data.revision) || data.revision < 0) throw new TypeError('revision');
  model._revision = data.revision;
  if (data.posterior) {
    const p = data.posterior;
    if (!Array.isArray(p.looks) || p.looks.length > 400 || !Number.isSafeInteger(p.revision) ||
        p.revision < 0 || p.revision > data.revision || !Number.isFinite(p.objective)) throw new TypeError('posterior');
    const n = p.looks.length;
    const arrays = {};
    for (const key of ['L', 'H', 'z', 'alpha', 'f']) {
      const size = key === 'L' || key === 'H' ? n * n : n;
      if (!Array.isArray(p[key]) || p[key].length !== size || p[key].some(x => !Number.isFinite(x))) throw new TypeError('posterior arrays');
      arrays[key] = Float64Array.from(p[key]);
    }
    for (const key of ['L', 'H']) {
      for (let i = 0; i < n; i++) {
        if (arrays[key][i * n + i] <= 0) throw new TypeError('posterior factor');
        for (let j = i + 1; j < n; j++) if (arrays[key][i * n + j] !== 0) throw new TypeError('posterior factor');
      }
    }
    model._posterior = {
      ...arrays, looks: p.looks.map(point => checkedPoint(model, point.features, point.scene)),
      hyper: checkedHyper(model.dim, p.hyper), revision: p.revision, objective: p.objective,
    };
  }
  return model;
}
