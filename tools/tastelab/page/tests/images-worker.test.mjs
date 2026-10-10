import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
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

test("love changes only pair order, preserving readiness, next selection and the actual fitted model", () => {
  const vectors = new Map(), ratings = new Map();
  for (let i = 0; i < 10; i++) {
    vectors.set("r" + i, i < 5 ? vector(1, 0) : vector(0, 1));
    ratings.set("r" + i, i < 5 ? "like" : "dislike");
  }
  vectors.set("uncertain", vector(Math.SQRT1_2, Math.SQRT1_2));
  vectors.set("far", vector(-1, 0));
  ratings.set("closed", "like");
  const notedPairs = new Set();
  for (let i = 0; i < 5; i++) for (let j = 5; j < 10; j++) {
    if (!([0, 4].includes(i) && [5, 6].includes(j))) notedPairs.add("r" + i + "|r" + j);
  }
  const message = { id: 6, order: [...vectors.keys()], vectors, ratings, skips: new Map(), ratedCount: 10, notedPairs };
  const fits = [], forwarded = [];
  // Observe real dependency calls from the worker source; its private model is
  // deliberately absent from the public message contract.
  const source = readFileSync(new URL("../images-worker.js", import.meta.url), "utf8")
    .replace(/^import .*;\r?\n/u, "").replace("export function handleMessage", "function handleMessage");
  const observed = runInNewContext(source + "\nhandleMessage;", {
    nextImage, isReady,
    fitLogistic(X, y, options) {
      const model = fitLogistic(X, y, options);
      fits.push({ X: structuredClone(X), y: structuredClone(y), options: structuredClone(options), model });
      return model;
    },
    suggestPairs(options) {
      forwarded.push(new Set(options.loved ?? []));
      return suggestPairs(options);
    },
  });
  for (const ready of [true, false]) {
    if (!ready) ratings.delete("r1");
    for (const ratedCount of [10, 12]) {
      message.ratedCount = ratedCount;
      const withLove = { ...message, loved: new Set(["r4", "closed", "uncertain"]) }, before = structuredClone(withLove);
      const plain = handleMessage(message), loved = handleMessage(withLove);
      assert.equal(plain.ready, ready);
      assert.equal(loved.ready, plain.ready);
      assert.equal(plain.next, ready && ratedCount % 3 !== 0 ? "uncertain" : "far");
      assert.equal(loved.next, plain.next);
      assert.deepEqual(plain.pairs.map((p) => [p.likedImageId, p.dislikedImageId]), [
        ["r0", "r5"], ["r0", "r6"], ["r4", "r5"], ["r4", "r6"],
      ]);
      assert.deepEqual(loved.pairs, [plain.pairs[2], plain.pairs[3], plain.pairs[0], plain.pairs[1]]);
      assert.deepEqual(handleMessage({ ...message, loved: new Set() }), plain);
      fits.length = forwarded.length = 0;
      assert.deepEqual(structuredClone(observed(message)), plain);
      assert.deepEqual(structuredClone(observed(withLove)), loved);
      assert.deepEqual(forwarded, [new Set(), new Set(["r4"])]);
      assert.equal(fits.length, ready ? 2 : 0);
      if (ready) assert.deepEqual(fits[1], fits[0], "training rows, binary labels and fitted model are identical");
      assert.deepEqual(withLove, before);
    }
  }
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
