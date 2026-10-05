import test from "node:test";
import assert from "node:assert/strict";
import { handleMessage } from "../images-worker.js";
import { nextImage, fitLogistic, isReady, suggestPairs } from "../images.js";

const vector = (...values) => Float32Array.from(values);
function direct(message) {
  const { order, vectors, skips, ratedCount, notedPairs } = message;
  const ratings = new Map(order.filter((id) => message.ratings.has(id)).map((id) => [id, message.ratings.get(id)]));
  const ready = isReady(ratings), ids = [...ratings.keys()];
  const model = ready ? fitLogistic(ids.map((id) => vectors.get(id)), ids.map((id) => ratings.get(id) === "like" ? 1 : 0)) : null;
  return { type: "suggestions", id: message.id, ready,
    next: nextImage({ order, vectors, ratings, skips, ratedCount, model }),
    pairs: suggestPairs({ vectors, ratings, notedPairs, limit: 10 }) };
}

test("the message handler matches direct farthest-point and skipped-image selection", () => {
  const vectors = new Map([["a", vector(1, 0)], ["b", vector(0, 1)], ["c", vector(-1, 0)], ["d", vector(0, -1)]]);
  const message = { id: 1, order: [...vectors.keys()], vectors, ratings: new Map(), skips: new Map(), ratedCount: 0, notedPairs: new Set() };
  const before = structuredClone(message);
  assert.deepEqual(handleMessage(message), direct(message));
  assert.equal(handleMessage(message).next, "a");
  message.ratings.set("a", "like");
  message.skips.set("c", 1);
  message.ratedCount = 1;
  assert.deepEqual(handleMessage(message), direct(message));
  assert.equal(handleMessage(message).next, "b");
  message.ratedCount = 51;
  assert.equal(handleMessage(message).next, "c");
  assert.equal(before.ratings.size, 0);
});

test("the worker fits ready ratings and matches direct uncertainty and every-third exploration", () => {
  // Likes along x and dislikes along y: "uncertain" lies on the decision boundary,
  // while "far" is the farthest from every rated image but clearly disliked.
  const vectors = new Map(), ratings = new Map();
  for (let i = 0; i < 10; i++) {
    vectors.set(`r${i}`, i < 5 ? vector(1, 0) : vector(0, 1));
    ratings.set(`r${i}`, i < 5 ? "like" : "dislike");
  }
  vectors.set("uncertain", vector(Math.SQRT1_2, Math.SQRT1_2));
  vectors.set("far", vector(-1, 0));
  const message = { id: 2, order: [...vectors.keys()], vectors, ratings, skips: new Map(), ratedCount: 10, notedPairs: new Set(["r0|r5"]) };
  for (const ratedCount of [10, 11, 12, 13, 14, 15]) {
    message.ratedCount = ratedCount;
    const before = structuredClone(message), result = handleMessage(message);
    assert.deepEqual(result, direct(message));
    assert.equal(result.ready, true);
    assert.equal(result.pairs.length, 10);
    assert.ok(result.pairs.every((pair) => pair.likedImageId !== "r0" || pair.dislikedImageId !== "r5"));
    assert.deepEqual(message, before);
    assert.equal(result.next, ratedCount % 3 === 0 ? "far" : "uncertain", `ratedCount ${ratedCount}`);
  }
});

test("closed bundles' ratings do not make a model ready without their vectors", () => {
  const ratings = new Map(Array.from({ length: 10 }, (_, i) => [`r${i}`, i < 5 ? "like" : "dislike"]));
  const message = { id: 3, order: ["r0", "next"], vectors: new Map([["r0", vector(1, 0)], ["next", vector(-1, 0)]]),
    ratings, skips: new Map(), ratedCount: 10, notedPairs: new Set() };
  assert.deepEqual(handleMessage(message), direct(message));
  assert.deepEqual(handleMessage(message), { type: "suggestions", id: 3, ready: false, next: "next", pairs: [] });
});

test("empty and exhausted libraries return no image or pairs", () => {
  const message = { id: 4, order: [], vectors: new Map(), ratings: new Map(), skips: new Map(), ratedCount: 0, notedPairs: new Set() };
  assert.deepEqual(handleMessage(message), { type: "suggestions", id: 4, ready: false, next: null, pairs: [] });
  message.order = ["a"]; message.vectors.set("a", vector(1)); message.ratings.set("a", "like");
  assert.equal(handleMessage(message).next, null);
});

test("worker errors carry the request id instead of escaping the message handler", () => {
  assert.equal(handleMessage({ id: 5, order: null }).type, "error");
  assert.equal(handleMessage({ id: 5, order: null }).id, 5);
});
