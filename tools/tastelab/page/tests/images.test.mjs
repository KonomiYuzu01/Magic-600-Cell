import test from "node:test";
import assert from "node:assert/strict";
import { isReady, nextImage, fitLogistic, predict, suggestPairs } from "../images.js";

const vector = (...values) => Float32Array.from(values);
const close = (actual, expected, tol = 1e-8) => assert.ok(Math.abs(actual - expected) <= tol, `${actual} versus ${expected}`);
const cardinal = new Map([["a", vector(1, 0)], ["b", vector(0, 1)], ["c", vector(-1, 0)], ["d", vector(0, -1)]]);
const pick = (options = {}) => nextImage({ order: [...cardinal.keys()], vectors: cardinal, ratings: new Map(), skips: new Map(), ratedCount: 0, ...options });

test("farthest point starts with the first eligible image and ties follow order", () => {
  assert.equal(pick(), "a");
  assert.equal(pick({ skips: new Map([["a", 0]]) }), "b");
  assert.equal(pick({ ratings: new Map([["a", "like"]]), ratedCount: 1 }), "c");
  const ratings = new Map([["a", "like"], ["c", "dislike"]]);
  assert.equal(pick({ ratings, ratedCount: 2 }), "b");
  assert.equal(pick({ order: ["a", "c", "d", "b"], ratings, ratedCount: 2 }), "d");
});

test("farthest point maximises the minimum distance to every available rated image", () => {
  const vectors = new Map([["r1", vector(1, 0)], ["r2", vector(0, 1)],
    ["near", vector(0.8, 0.6)], ["opposite-one", vector(-1, 0)], ["far-both", vector(-Math.SQRT1_2, -Math.SQRT1_2)]]);
  const ratings = new Map([["r2", "dislike"], ["r1", "like"], ["closed-image", "like"]]);
  assert.equal(nextImage({ order: [...vectors.keys()], vectors, ratings, skips: new Map(), ratedCount: 3 }), "far-both");
  assert.equal(pick({ ratings: new Map([["closed-image", "like"]]), ratedCount: 1 }), "a");
});

test("skip returns after exactly 50 further ratings and never returns a rated image", () => {
  const skips = new Map([["a", 10]]), order = ["a"];
  assert.equal(pick({ order, skips, ratedCount: 59 }), null);
  assert.equal(pick({ order, skips, ratedCount: 60 }), "a");
  assert.equal(pick({ order, skips, ratedCount: 61 }), "a");
  assert.equal(pick({ order, skips, ratings: new Map([["a", "dislike"]]), ratedCount: 60 }), null);
});

test("readiness requires at least five likes and five dislikes", () => {
  const ratings = new Map();
  assert.equal(isReady(ratings), false);
  for (let i = 0; i < 5; i++) ratings.set(`l${i}`, "like");
  for (let i = 0; i < 4; i++) ratings.set(`d${i}`, "dislike");
  assert.equal(isReady(ratings), false);
  ratings.set("d4", "dislike");
  assert.equal(isReady(ratings), true);
  ratings.delete("l4");
  assert.equal(isReady(ratings), false);
  for (let i = 0; i < 20; i++) ratings.set(`d${i}`, "dislike");
  assert.equal(isReady(ratings), false);
  ratings.set("l4", "like");
  assert.equal(isReady(ratings), true);
});

test("ready selection alternates farthest at ratedCount multiples of three with uncertainty", () => {
  const ratings = new Map(), vectors = new Map();
  for (let i = 0; i < 10; i++) { ratings.set(`r${i}`, i < 5 ? "like" : "dislike"); vectors.set(`r${i}`, vector(1, 0)); }
  vectors.set("far", vector(-1, 0)); vectors.set("uncertain-first", vector(0, 1)); vectors.set("uncertain-second", vector(0, -1));
  const options = { order: [...vectors.keys()], vectors, ratings, skips: new Map(), model: { w: [5, 0], b: 0 } };
  for (const ratedCount of [12, 15, 18]) assert.equal(nextImage({ ...options, ratedCount }), "far");
  for (const ratedCount of [10, 11, 13, 14, 16, 17]) assert.equal(nextImage({ ...options, ratedCount }), "uncertain-first");
  ratings.delete("r0");
  assert.equal(nextImage({ ...options, ratedCount: 11 }), "far", "before readiness the supplied model does not change selection");
});

test("selection returns null when no image is eligible and leaves all input state unchanged", () => {
  assert.equal(pick({ order: [] }), null);
  assert.equal(pick({ ratings: new Map([...cardinal.keys()].map((id) => [id, "like"])) }), null);
  assert.equal(pick({ skips: new Map([...cardinal.keys()].map((id) => [id, 0])) }), null);
  const options = { order: [...cardinal.keys()], vectors: cardinal, ratings: new Map([["a", "like"]]), skips: new Map([["b", 0]]), ratedCount: 1 };
  const before = structuredClone(options);
  assert.equal(nextImage(options), nextImage(options));
  assert.deepEqual(options, before);
});

test("prediction is stable at extreme logits and includes the bias", () => {
  close(predict({ w: [0], b: 0 }, vector(1)), 0.5);
  close(predict({ w: [1], b: Math.log(3) }, vector(0)), 0.75);
  assert.equal(predict({ w: [1000], b: 0 }, vector(1)), 1);
  assert.equal(predict({ w: [-1000], b: 0 }, vector(1)), 0);
});

// Independent reference: coordinate minimisation by bisection on each
// derivative, without a Hessian or Newton steps. Includes the free bias.
function referenceFit(X, y, lambda) {
  const dim = X[0].length, theta = Array(dim + 1).fill(0), counts = [y.filter((v) => v === 0).length, y.filter((v) => v === 1).length];
  const derivative = (coordinate, value) => {
    const trial = theta.slice(); trial[coordinate] = value;
    let sum = coordinate === dim ? 0 : lambda * value;
    for (let i = 0; i < X.length; i++) {
      const z = trial[dim] + X[i].reduce((total, x, j) => total + x * trial[j], 0);
      sum += (1 / (1 + Math.exp(-z)) - y[i]) * (X.length / (2 * counts[y[i]])) * (coordinate === dim ? 1 : X[i][coordinate]);
    }
    return sum;
  };
  for (let round = 0; round < 1000; round++) {
    const before = theta.slice();
    for (let j = 0; j <= dim; j++) {
      let lo = -20, hi = 20;
      assert.ok(derivative(j, lo) < 0 && derivative(j, hi) > 0);
      for (let iteration = 0; iteration < 80; iteration++) {
        const mid = (lo + hi) / 2;
        if (derivative(j, mid) > 0) hi = mid; else lo = mid;
      }
      theta[j] = (lo + hi) / 2;
    }
    if (Math.max(...theta.map((value, j) => Math.abs(value - before[j]))) < 1e-13) return theta;
  }
  assert.fail("independent reference did not converge");
}

test("Newton fit agrees within 1e-8 with independent bisection for balanced weights and free bias", () => {
  const X = [[-1, 0.5], [-0.6, 0.8], [0.5, -0.3], [0.9, 0.2], [0.1, -0.9]], y = [0, 0, 0, 1, 1];
  const before = structuredClone({ X, y });
  for (const lambda of [1, 0.4, 3]) {
    const expected = referenceFit(X, y, lambda), result = fitLogistic(X, y, { lambda, tol: 1e-12 });
    assert.ok(result.gradNorm < 1e-12);
    assert.ok(result.steps > 0 && result.steps < 50);
    result.w.forEach((value, j) => close(value, expected[j]));
    close(result.b, expected.at(-1));
    assert.ok(Math.abs(result.b) > 0.01, "this reference case exercises an unpenalised nonzero bias");
  }
  assert.deepEqual({ X, y }, before);
});

test("class balancing matches the independent symmetric scalar optimum", () => {
  const X = [[-1], [1], [1], [1]], y = [0, 1, 1, 1];
  let lo = 0, hi = 4;
  for (let i = 0; i < 100; i++) {
    const mid = (lo + hi) / 2;
    if (mid - 4 / (1 + Math.exp(mid)) > 0) hi = mid; else lo = mid;
  }
  const result = fitLogistic(X, y, { tol: 1e-12 });
  close(result.w[0], (lo + hi) / 2);
  close(result.b, 0);
  const constant = fitLogistic([[1], [1], [1], [1]], y);
  close(constant.w[0], 0); close(constant.b, 0);
  assert.equal(constant.steps, 0);
});

test("Newton step cap counts actual full steps and reports the final gradient norm", () => {
  const X = [[-1], [1]], y = [0, 1];
  const zero = fitLogistic(X, y, { maxSteps: 0 });
  assert.equal(zero.steps, 0); assert.equal(zero.w[0], 0); assert.equal(zero.b, 0); close(zero.gradNorm, 1);
  const one = fitLogistic(X, y, { maxSteps: 1, tol: 1e-12 });
  assert.equal(one.steps, 1); close(one.w[0], 2 / 3); close(one.b, 0);
  close(one.gradNorm, Math.abs(one.w[0] - 2 / (1 + Math.exp(one.w[0]))));
  assert.ok(one.gradNorm > 1e-12);
  assert.equal(fitLogistic(X, y, { tol: 2 }).steps, 0);
});

test("warm start converges to the same fit without modifying the starting model", () => {
  const X = [[-1, 0.5], [-0.6, 0.8], [0.5, -0.3], [0.9, 0.2], [0.1, -0.9]], y = [0, 0, 0, 1, 1];
  const init = { w: Float64Array.of(0.2, -0.1), b: 0.3 }, before = structuredClone(init);
  const cold = fitLogistic(X, y, { tol: 1e-12 }), warm = fitLogistic(X, y, { init, tol: 1e-12 });
  warm.w.forEach((value, j) => close(value, cold.w[j])); close(warm.b, cold.b);
  assert.deepEqual(init, before);
  assert.notEqual(warm.w, init.w);
  assert.equal(fitLogistic(X, y, { init: cold, tol: 1e-12 }).steps, 0);
});

test("default Newton fit converges for 512-dimensional unit Float32 embeddings", () => {
  const positive = new Float32Array(512), negative = new Float32Array(512);
  positive[0] = 1; negative[0] = -1;
  const result = fitLogistic([negative, negative, positive, positive], [0, 0, 1, 1]);
  assert.equal(result.w.length, 512);
  assert.ok(result.gradNorm < 1e-6 && result.steps < 50);
  assert.ok([...result.w].every(Number.isFinite) && Number.isFinite(result.b));
  assert.ok(predict(result, positive) > 0.5 && predict(result, negative) < 0.5);
});

test("pairs rank highest cosine similarity first, then liked and disliked import order", () => {
  const vectors = new Map([["l1", vector(1, 0)], ["d1", vector(1, 0)], ["l2", vector(0, 1)],
    ["d2", vector(0.8, 0.6)], ["d3", vector(0, 1)]]);
  const ratings = new Map([["d3", "dislike"], ["l2", "like"], ["d2", "dislike"], ["d1", "dislike"], ["l1", "like"], ["closed", "like"]]);
  const pairs = suggestPairs({ vectors, ratings, notedPairs: new Set() });
  assert.deepEqual(pairs.map((p) => [p.likedImageId, p.dislikedImageId]),
    [["l1", "d1"], ["l2", "d3"], ["l1", "d2"], ["l2", "d2"], ["l1", "d3"], ["l2", "d1"]]);
  close(pairs[0].similarity, 1); close(pairs[1].similarity, 1);
  close(pairs[2].similarity, vector(0.8)[0]);
});

test("pair ties, noted pair exclusion, defaults and limits are deterministic and pure", () => {
  const vectors = new Map(), ratings = new Map();
  for (let i = 0; i < 4; i++) { vectors.set(`l${i}`, vector(1, 0)); ratings.set(`l${i}`, "like"); }
  for (let i = 0; i < 4; i++) { vectors.set(`d${i}`, vector(1, 0)); ratings.set(`d${i}`, "dislike"); }
  const notedPairs = new Set(["l0|d0", "d1|l0"]), options = { vectors, ratings, notedPairs }, before = structuredClone(options);
  const pairs = suggestPairs(options);
  assert.equal(pairs.length, 10);
  assert.deepEqual(pairs.slice(0, 5).map((p) => [p.likedImageId, p.dislikedImageId]),
    [["l0", "d1"], ["l0", "d2"], ["l0", "d3"], ["l1", "d0"], ["l1", "d1"]]);
  assert.deepEqual(suggestPairs({ ...options, limit: 2 }), pairs.slice(0, 2));
  assert.deepEqual(suggestPairs({ ...options, limit: 0 }), []);
  assert.deepEqual(suggestPairs(options), pairs);
  assert.deepEqual(options, before);
  for (let i = 0; i < 4; i++) for (let j = 0; j < 4; j++) notedPairs.add(`l${i}|d${j}`);
  assert.deepEqual(suggestPairs(options), []);
  assert.deepEqual(suggestPairs({ vectors, ratings: new Map(), notedPairs: new Set() }), []);
});
