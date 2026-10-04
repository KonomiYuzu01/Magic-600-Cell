// All ordering comes from the open bundles; selection uses no clock or RNG.
const dot = (a, b) => {
  let sum = 0;
  for (let i = 0; i < a.length; i++) sum += a[i] * b[i];
  return sum;
};
const sigmoid = (z) => z >= 0 ? 1 / (1 + Math.exp(-z)) : Math.exp(z) / (1 + Math.exp(z));

export function isReady(ratings) {
  let likes = 0, dislikes = 0;
  for (const rating of ratings.values()) {
    if (rating === "like") likes++;
    if (rating === "dislike") dislikes++;
  }
  return likes >= 5 && dislikes >= 5;
}

export function predict(model, vector) {
  return sigmoid(dot(model.w, vector) + model.b);
}

export function nextImage({ order, vectors, ratings, skips, ratedCount, model }) {
  const farthest = !isReady(ratings) || !model || ratedCount % 3 === 0;
  const rated = [...vectors.keys()].filter((id) => ratings.has(id));
  let best = null, bestScore = -Infinity;
  for (const id of order) {
    if (ratings.has(id) || (skips.has(id) && ratedCount - skips.get(id) < 50)) continue;
    const vector = vectors.get(id);
    let score = Infinity;
    if (farthest) {
      for (const other of rated) score = Math.min(score, 1 - dot(vector, vectors.get(other)));
    } else score = -Math.abs(predict(model, vector) - 0.5);
    if (score > bestScore) { best = id; bestScore = score; }
  }
  return best;
}

// Cholesky solve of a positive definite Hessian, using its lower triangle.
function solve(hessian, gradient) {
  const size = gradient.length;
  for (let i = 0; i < size; i++) {
    for (let j = 0; j <= i; j++) {
      let value = hessian[i * size + j];
      for (let k = 0; k < j; k++) value -= hessian[i * size + k] * hessian[j * size + k];
      if (i === j) {
        if (!(value > 0)) throw new RangeError("Logistic Hessian is not positive definite.");
        hessian[i * size + j] = Math.sqrt(value);
      } else hessian[i * size + j] = value / hessian[j * size + j];
    }
  }
  const delta = Float64Array.from(gradient);
  for (let i = 0; i < size; i++) {
    for (let j = 0; j < i; j++) delta[i] -= hessian[i * size + j] * delta[j];
    delta[i] /= hessian[i * size + i];
  }
  for (let i = size - 1; i >= 0; i--) {
    for (let j = i + 1; j < size; j++) delta[i] -= hessian[j * size + i] * delta[j];
    delta[i] /= hessian[i * size + i];
  }
  return delta;
}

export function fitLogistic(X, y, { lambda = 1, maxSteps = 50, tol = 1e-6, init } = {}) {
  if (!X.length || X.length !== y.length || !y.every((value) => value === 0 || value === 1)) throw new RangeError("Logistic data must have rows and binary labels.");
  const positives = y.reduce((sum, value) => sum + value, 0), negatives = y.length - positives;
  if (!positives || !negatives) throw new RangeError("Logistic data must contain both classes.");
  const dim = X[0].length, size = dim + 1;
  if (!Number.isFinite(lambda) || lambda <= 0 || !Number.isInteger(maxSteps) || maxSteps < 0 || !Number.isFinite(tol) || tol <= 0
      || X.some((row) => row.length !== dim || !Array.from(row).every(Number.isFinite))
      || (init && (init.w.length !== dim || !Array.from(init.w).every(Number.isFinite) || !Number.isFinite(init.b)))) throw new RangeError("Logistic dimensions or options are invalid.");
  const w = init ? Float64Array.from(init.w) : new Float64Array(dim);
  let b = init ? init.b : 0, steps = 0;
  // Minimise sum_i classWeight_i * logLoss_i + lambda/2 * ||w||².
  // The bias is never penalised; all arithmetic in the fit stays float64.
  while (true) {
    const gradient = new Float64Array(size), hessian = new Float64Array(size * size);
    for (let j = 0; j < dim; j++) { gradient[j] = lambda * w[j]; hessian[j * size + j] = lambda; }
    for (let i = 0; i < X.length; i++) {
      const row = X[i], p = predict({ w, b }, row);
      const weight = X.length / (2 * (y[i] ? positives : negatives));
      const residual = weight * (p - y[i]), curvature = weight * p * (1 - p);
      for (let j = 0; j < size; j++) {
        const xj = j === dim ? 1 : row[j];
        gradient[j] += residual * xj;
        for (let k = 0; k <= j; k++) hessian[j * size + k] += curvature * xj * (k === dim ? 1 : row[k]);
      }
    }
    const gradNorm = Math.hypot(...gradient);
    if (gradNorm < tol || steps >= maxSteps) return { w, b, steps, gradNorm };
    const delta = solve(hessian, gradient);
    for (let j = 0; j < dim; j++) w[j] -= delta[j];
    b -= delta[dim];
    steps++;
  }
}

export function suggestPairs({ vectors, ratings, notedPairs, limit = 10 }) {
  if (limit <= 0) return [];
  const liked = [], disliked = [];
  for (const id of vectors.keys()) {
    if (ratings.get(id) === "like") liked.push(id);
    if (ratings.get(id) === "dislike") disliked.push(id);
  }
  const pairs = [];
  for (const likedImageId of liked) {
    for (const dislikedImageId of disliked) {
      if (!notedPairs.has(likedImageId + "|" + dislikedImageId)) pairs.push({ likedImageId, dislikedImageId, similarity: dot(vectors.get(likedImageId), vectors.get(dislikedImageId)) });
    }
  }
  // Stable sort retains liked order, then disliked order, for equal scores.
  return pairs.sort((a, b) => b.similarity - a.similarity).slice(0, limit);
}
