import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { assembleImageExport, createRatingState, reduceRating, keyAction, validateNote, cleanNote, pairKey,
  ratingDocument, pairDocument, loadImageDocuments, createImagesUI } from "../images-ui.js";
import { handleMessage } from "../images-worker.js";

const digest = (value) => createHash("sha256").update(value).digest("hex");
const a = digest("original A"), b = digest("original B"), c = digest("original C");
const time = "2026-10-04T12:00:00Z";
const rating = (imageId = a, verdict = "like", note = null, love = false) => ({ imageId, verdict, love, note, ratedAt: time });
const pair = (likedImageId = a, dislikedImageId = b, note = "Different contrast") => ({ likedImageId, dislikedImageId, note, notedAt: time });

test("version 4 preserves every version 2 field and exports exactly the image contract", () => {
  const base = { kind: "tastelab-export", version: 2, space: { params: [] }, families: ["f1"], comparisons: [{ answer: "A" }],
    sessions: [{ settled: {} }], presets: [{ look: {} }], shared: [], costHeavy: { params: ["gloss"], note: "cost" } };
  const input = { bundles: [{ id: "bundle-id", items: [{ imageId: a, thumbSha256: digest("thumb"), filename: "secret.jpg" }],
    vectors: new Float32Array(512), thumbnails: [Uint8Array.of(1)] }],
    ratings: [{ ...rating(), vectors: [1], thumbnail: "private", filename: "private.jpg" }],
    pairs: [{ ...pair(), similarity: 0.8, files: ["private"] }] };
  const before = structuredClone({ base, input }), result = assembleImageExport(base, input);
  assert.deepEqual(result, { ...base, version: 4, images: { bundles: [{ bundleId: "bundle-id", items: 1 }], ratings: [rating()], pairs: [pair()] } });
  for (const key of Object.keys(base).filter((key) => key !== "version")) assert.deepEqual(result[key], base[key]);
  assert.deepEqual({ base, input }, before);
  const json = JSON.stringify(result);
  for (const forbidden of ["thumbnail", "thumbSha256", "vectors", "filename", "private", "embeddings"]) assert.equal(json.includes(forbidden), false, forbidden);
});

test("legacy and malformed stored love flags load and export as false", () => {
  const records = [
    { imageId: a, verdict: "like", note: null, ratedAt: time },
    { ...rating(b), love: "true" },
    rating(c, "dislike", null, true),
  ];
  const before = structuredClone(records), expected = [rating(a), rating(b), rating(c, "dislike")];
  assert.deepEqual([...createRatingState(records).ratings.values()], expected);
  const result = assembleImageExport({ version: 2 }, { bundles: [], ratings: records, pairs: [] });
  assert.equal(result.version, 4);
  assert.deepEqual(result.images.ratings, expected);
  assert.ok(result.images.ratings.every((record) => typeof record.love === "boolean"));
  assert.deepEqual(records, before);
});

test("export merges unsaved notes, ratings and pairs, and honours unsuccessful undo deletions", () => {
  const result = assembleImageExport({ version: 2 }, {
    bundles: [], ratings: [rating(a), rating(b, "dislike")], pairs: [pair()],
    ratingChanges: new Map([[a, null], [b, rating(b, "dislike", "local note")], [c, rating(c, "like", null, true)]]),
    pairChanges: new Map([[pairKey(a, b), pair(a, b, "local pair note")], [pairKey(c, b), pair(c, b)]]),
  });
  assert.deepEqual(result.images, { bundles: [], ratings: [rating(b, "dislike", "local note"), rating(c, "like", null, true)],
    pairs: [pair(a, b, "local pair note"), pair(c, b)] });
  assert.deepEqual(assembleImageExport({ version: 2 }, { bundles: [], ratings: [], pairs: [] }).images, { bundles: [], ratings: [], pairs: [] });
});

test("rating and note reducers are pure, skip stays local, and undo deletes only this tab's last rating", () => {
  let state = createRatingState([rating(a)]);
  const loaded = state, initial = structuredClone(state);
  assert.equal(reduceRating(state, { type: "undo" }), state, "loaded ratings are outside this tab's undo history");
  state = reduceRating(state, { type: "skip", imageId: b });
  assert.equal(state.skips.get(b), 1);
  assert.equal(state.changes.size, 0);
  const skippedState = state, skipped = structuredClone(state);
  state = reduceRating(state, { type: "rate", ...rating(b, "dislike") });
  assert.equal(state.skips.has(b), false);
  assert.deepEqual(state.history, [b]);
  assert.deepEqual(state.changes.get(b), rating(b, "dislike"));
  const ratedState = state, rated = structuredClone(state);
  state = reduceRating(state, { type: "note", imageId: b, note: "why", ratedAt: time });
  assert.deepEqual(state.ratings.get(b), { ...rating(b, "dislike", "why"), ratedAt: "2026-10-04T12:00:00.001Z" });
  assert.deepEqual(state.history, [b]);
  const notedState = state, beforeUndo = structuredClone(state);
  state = reduceRating(state, { type: "undo" });
  assert.deepEqual([...state.ratings.values()], [rating(a)]);
  assert.equal(state.changes.get(b), null);
  assert.deepEqual(state.history, []);
  // Each state passed to the reducer is unchanged by it.
  assert.deepEqual(loaded, initial);
  assert.deepEqual(skippedState, skipped);
  assert.deepEqual(ratedState, rated);
  assert.deepEqual(notedState, beforeUndo);
  assert.equal(skipped.skips.has(b), true);
  assert.equal(beforeUndo.ratings.has(b), true);
  state = reduceRating(state, { type: "rate", ...rating(c) });
  state = reduceRating(state, { type: "note", imageId: c, note: "", ratedAt: time });
  assert.equal(state.ratings.get(c).note, null, "a cleared note is null");
});

test("love stays a like through note edits and undo removes the whole rating", () => {
  const initial = createRatingState(), before = structuredClone(initial), loved = rating(a, "like", null, true);
  const rated = reduceRating(initial, { type: "rate", ...loved });
  assert.deepEqual(rated.ratings.get(a), loved);
  assert.deepEqual(rated.changes.get(a), loved);
  assert.deepEqual(rated.history, [a]);
  const noted = reduceRating(rated, { type: "note", imageId: a, note: "The light", ratedAt: time });
  assert.deepEqual(noted.ratings.get(a), { ...loved, note: "The light", ratedAt: "2026-10-04T12:00:00.001Z" });
  const undone = reduceRating(noted, { type: "undo" });
  assert.equal(undone.ratings.has(a), false);
  assert.equal(undone.changes.get(a), null);
  assert.deepEqual(undone.history, []);
  assert.deepEqual(initial, before);
  assert.deepEqual(rated.ratings.get(a), loved);
});

test("love is boolean and true requires a like verdict", () => {
  const initial = createRatingState();
  const invalid = { name: "RangeError", message: "Invalid image rating." };
  assert.throws(() => reduceRating(initial, { type: "rate", ...rating(a, "dislike", null, true) }), invalid);
  for (const love of [null, 0, 1, "true", "false", [], {}]) {
    assert.throws(() => reduceRating(initial, { type: "rate", ...rating(), love }), invalid);
  }
  const legacy = { type: "rate", imageId: a, verdict: "like", ratedAt: time };
  assert.deepEqual(reduceRating(initial, legacy).ratings.get(a), rating());
  for (const verdict of ["like", "dislike"]) {
    assert.deepEqual(reduceRating(initial, { type: "rate", ...rating(a, verdict) }).ratings.get(a), rating(a, verdict));
  }
  assert.equal(initial.ratings.size, 0);
});

for (const clock of ["later", "unchanged", "backwards"]) test(`adding, replacing and clearing a note advance ratedAt with a ${clock} clock`, () => {
  let state = reduceRating(createRatingState(), { type: "rate", ...rating() });
  for (const note of ["First explanation", "Replacement explanation", ""]) {
    const previous = state, before = structuredClone(state), last = Date.parse(state.ratings.get(a).ratedAt);
    const ratedAt = new Date(last + (clock === "later" ? 1000 : clock === "backwards" ? -1000 : 0)).toISOString();
    state = reduceRating(state, { type: "note", imageId: a, note, ratedAt });
    assert.equal(Date.parse(state.ratings.get(a).ratedAt), last + (clock === "later" ? 1000 : 1));
    assert.equal(state.ratings.get(a).note, note || null);
    assert.deepEqual(previous, before, "the reducer does not mutate its input");
  }
});

test("successive undo operations respect rating order without undoing a skip", () => {
  let state = createRatingState();
  for (const record of [rating(a), rating(b, "dislike")]) state = reduceRating(state, { type: "rate", ...record });
  state = reduceRating(state, { type: "skip", imageId: c });
  state = reduceRating(state, { type: "undo" });
  assert.deepEqual([...state.ratings.keys()], [a]);
  assert.equal(state.skips.get(c), 2);
  state = reduceRating(state, { type: "undo" });
  assert.equal(state.ratings.size, 0);
  assert.deepEqual([...state.changes.values()], [null, null]);
});

test("rating and pair note validation enforces both boundaries", () => {
  assert.equal(validateNote(""), true);
  assert.equal(validateNote("x".repeat(140)), true);
  assert.equal(validateNote("x".repeat(141)), false);
  assert.equal(validateNote("", "pair"), false);
  assert.equal(validateNote("  ", "pair"), false);
  assert.equal(validateNote("x", "pair"), true);
  assert.equal(validateNote("x".repeat(280), "pair"), true);
  assert.equal(validateNote("x".repeat(281), "pair"), false);
  assert.equal(validateNote(null), false);
  for (const control of ["\t", "\n", "\u0000", "\u0007", "\u007f", "\u0085"]) {
    assert.equal(validateNote(`a${control}b`), false);
    assert.equal(validateNote(`a${control}b`, "pair"), false);
  }
  assert.equal(validateNote("Ünïcode ✓ note"), true);
  assert.equal(cleanNote("  Too\tbusy \n\r here  "), "Too busy here");
  assert.equal(cleanNote(" \t "), "");
  assert.throws(() => reduceRating(createRatingState(), { type: "rate", ...rating(a, "skip") }), RangeError);
  assert.throws(() => reduceRating(createRatingState(), { type: "rate", ...rating(a, "like", "x".repeat(141)) }), RangeError);
});

test("the document key uses original image ids in liked-disliked order, with exact bodies", () => {
  assert.equal(pairKey(a, b), `${a}_${b}`);
  assert.notEqual(pairKey(a, b), pairKey(b, a));
  assert.deepEqual(ratingDocument({ ...rating(), thumbnail: "canary" }), { path: `imageRatings/${a}`, body: rating() });
  assert.deepEqual(ratingDocument({ imageId: a, verdict: "like", ratedAt: time }).body, rating());
  assert.deepEqual(ratingDocument(rating(a, "like", "")).body, rating(), "an empty note is null");
  assert.deepEqual(pairDocument({ ...pair(), similarity: 1 }), { path: `imagePairs/${a}_${b}`, body: pair() });
});

const event = (key, extra = {}) => ({ key, repeat: false, target: { closest: () => null }, ...extra });
test("key mappings are isolated by tab, retain Looks keys, and ignore repeat and input editing", () => {
  const looks = { A: "A", b: "B", s: "same", x: "bad", z: "undo", W: "note", f: "family", 1: "scene-1", 2: "scene-2", 3: "scene-3" };
  const images = { ArrowRight: "like", ArrowUp: "love", ArrowLeft: "dislike", ArrowDown: "skip", w: "note", Z: "undo" };
  for (const [tab, mapping] of [["looks", looks], ["images", images]]) for (const [key, action] of Object.entries(mapping)) {
    assert.equal(keyAction(event(key), tab), action);
    assert.equal(keyAction(event(key, { repeat: true }), tab), null);
    for (const selector of ["input", "textarea", "select", "[contenteditable]"]) {
      assert.equal(keyAction(event(key, { target: { closest: (value) => value.includes(selector) ? {} : null } }), tab), null);
    }
    assert.equal(keyAction(event(key, { target: { isContentEditable: true } }), tab), null);
    assert.equal(keyAction(event(key, { defaultPrevented: true }), tab), null);
    for (const modifier of ["altKey", "ctrlKey", "metaKey"]) assert.equal(keyAction(event(key, { [modifier]: true }), tab), null);
    assert.equal(keyAction(event(key, { shiftKey: true }), tab), action);
  }
  for (const key of ["A", "B", "S", "X", "F", "1", "2", "3"]) assert.equal(keyAction(event(key), "images"), null);
  for (const key of ["ArrowRight", "ArrowUp", "ArrowLeft", "ArrowDown"]) assert.equal(keyAction(event(key), "looks"), null);
  assert.equal(keyAction(event("q"), "images"), null);
  assert.equal(keyAction(event("A", { target: { closest: (value) => value.includes("[role='tablist']") ? {} : null } }), "looks"), "A");
});

function fakeDB(ratings = [], pairs = [], onWrite = async () => {}, onRead = async () => {}) {
  const values = new Map([...ratings.map(ratingDocument), ...pairs.map(pairDocument)].map(({ path, body }) => [path, body]));
  const reads = [], writes = [];
  function collection(name, field = null, filters = [], size = 500) {
    return {
      orderBy(next) { return collection(name, next, filters, size); },
      where(key, op, value) { return collection(name, field, [...filters, [key, op, value]], size); },
      limit(next) { assert.ok(next <= 500); return collection(name, field, filters, next); },
      async get() {
        reads.push({ name, field, filters, size });
        const rows = [...values].filter(([path]) => path.startsWith(name + "/")).map(([path, body]) => ({ id: path.slice(name.length + 1), body }))
          .filter(({ body }) => filters.every(([key, op, value]) => op === ">" ? body[key] > value : body[key] === value))
          .sort((a, b) => a.body[field] < b.body[field] ? -1 : a.body[field] > b.body[field] ? 1 : 0).slice(0, size);
        const snapshot = rows.map(({ id, body }) => ({ id, body: structuredClone(body) }));
        await onRead(name);
        return { size: snapshot.length, docs: snapshot.map(({ id, body }) => ({ id, data: () => structuredClone(body) })) };
      },
    };
  }
  return { values, reads, writes, collection,
    doc(path) {
      assert.match(path, /^image(?:Ratings|Pairs)\//);
      return {
        async set(body) { writes.push({ path, body: structuredClone(body) }); await onWrite("set", path, body); values.set(path, structuredClone(body)); },
        async delete() { writes.push({ path, body: null }); await onWrite("delete", path); values.delete(path); },
      };
    },
  };
}

test("paged reads include all ratings and pairs even when over 500 pairs share a liked id and timestamp", async () => {
  const id = (n) => n.toString(16).padStart(64, "0");
  const ratings = Array.from({ length: 1101 }, (_, i) => rating(id(i)));
  const pairs = Array.from({ length: 1202 }, (_, i) => pair(id(i < 601 ? 0 : 1), id(i)));
  const db = fakeDB(ratings, pairs);
  assert.deepEqual(await loadImageDocuments(db, "imageRatings"), ratings);
  assert.deepEqual(await loadImageDocuments(db, "imagePairs"), pairs);
  assert.ok(db.reads.length > 6);
  assert.ok(db.reads.every((read) => read.size === 500));
  assert.equal(db.writes.length, 0);
});

// Minimal DOM and worker fakes exercise the actual controller's asynchronous
// path. Thumbnail bytes and bundle manifests are synthetic and stay in memory.
class Element {
  children = []; listeners = new Map(); hidden = false; disabled = false; value = ""; textContent = "";
  constructor(tagName = "div") { this.tagName = tagName; }
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) { this.children = children; }
  addEventListener(type, listener) { this.listeners.set(type, listener); }
  focus() { this.focused = true; }
  blur() { this.focused = false; this.blurred = true; }
  async fire(type, data = {}) {
    const e = { target: this, preventDefault() { this.defaultPrevented = true; }, ...data };
    await this.listeners.get(type)?.(e);
    return e;
  }
}
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));
function fixture(indices = [0, 1, 2]) {
  const files = [], data = Buffer.alloc(indices.length * 1024);
  const items = indices.map((index, row) => {
    const bytes = Uint8Array.of(0xff, 0xd8, 0xff, index), thumbSha256 = digest(bytes);
    files.push([thumbSha256 + ".jpg", bytes]);
    data.writeUInt16LE(0x3c00, row * 1024 + index * 2);
    return { imageId: [a, b, c][index], thumbSha256, width: 512, height: 384, bytes: bytes.length,
      source: "wikimedia", sourceId: `synthetic-${index}`, title: `Synthetic ${index}`, credit: "Synthetic creator",
      pageUrl: "https://commons.wikimedia.org/wiki/File:Synthetic.jpg", licence: "CC0-1.0",
      licenceUrl: "https://creativecommons.org/publicdomain/zero/1.0/" };
  });
  const manifest = { format: "tastelab-images", version: 1, id: "20261004T120000Z-0123abcd", createdAt: time,
    model: { id: "synthetic", weightsSha256: "ab".repeat(32) }, items,
    embeddings: { dtype: "float16", dim: 512, count: items.length, data: data.toString("base64") } };
  files.push(["manifest.json", new TextEncoder().encode(JSON.stringify(manifest))]);
  return files.map(([name, bytes]) => ({ name, size: bytes.length, webkitRelativePath: "bundle/" + name, arrayBuffer: async () => bytes.slice().buffer }));
}

async function withUI(db, run) {
  const keys = ["document", "Worker", "indexedDB", "URL"], saved = keys.map((key) => Object.getOwnPropertyDescriptor(globalThis, key));
  const elements = new Map(), status = [], workers = [], created = [], revoked = [], NativeURL = globalThis.URL;
  const ids = new Set([...readFileSync(new URL("../../index.html", import.meta.url), "utf8").matchAll(/id="([^"]+)"/g)].map((match) => match[1]));
  const $ = (id) => { assert.ok(ids.has(id), "Missing page element: " + id); if (!elements.has(id)) elements.set(id, new Element()); return elements.get(id); };
  for (const id of ["imageCacheNote", "imageNoteForm", "imagePairDetail", "imageAnswersRetry"]) $(id).hidden = true;
  globalThis.document = { getElementById: $, createElement: (tag) => new Element(tag) };
  globalThis.indexedDB = { open() { throw new Error("Cache blocked"); } };
  globalThis.URL = class extends NativeURL {
    static createObjectURL() { const url = "blob:synthetic-" + created.length; created.push(url); return url; }
    static revokeObjectURL(url) { revoked.push(url); }
  };
  globalThis.Worker = class {
    messages = []; paused = false;
    constructor(url, options) { assert.ok(url.pathname.endsWith("/images-worker.js")); assert.equal(options.type, "module"); workers.push(this); }
    postMessage(message) { this.messages.push(structuredClone(message)); if (!this.paused) queueMicrotask(() => this.reply(message)); }
    reply(message) { this.onmessage({ data: handleMessage(message) }); }
  };
  try {
    const ui = createImagesUI({ setStatus: (message) => status.push(message) });
    await ui.load(db, true); await tick();
    const open = async (files = fixture()) => { $("bundleFolder").files = files; await $("bundleFolder").fire("change"); await tick(); };
    await run({ ui, $, open, status, worker: workers[0], created, revoked });
    await tick();
  } finally {
    keys.forEach((key, i) => { if (saved[i]) Object.defineProperty(globalThis, key, saved[i]); else delete globalThis[key]; });
  }
}

test("opening without a cache renders attribution from object URLs, refuses invalid selections, and closes locally", async () => {
  await withUI(fakeDB(), async ({ ui, $, open, created, revoked }) => {
    assert.equal($("imageCacheNote").hidden, false);
    await open();
    assert.equal($("openBundles").children.length, 1);
    assert.equal($("bundleFolder").blurred, true, "the rating keys work after opening");
    const figure = $("imageView").children[0], [image, caption] = figure.children;
    assert.match(image.src, /^blob:/);
    assert.equal(caption.children[1].textContent, "Synthetic creator");
    assert.deepEqual(caption.children.slice(2).map((link) => [link.textContent, link.href]), [
      ["wikimedia", "https://commons.wikimedia.org/wiki/File:Synthetic.jpg"],
      ["CC0-1.0", "https://creativecommons.org/publicdomain/zero/1.0/"]]);
    for (const link of caption.children.slice(2)) {
      assert.equal(link.target, "_blank"); assert.equal(link.rel, "noopener noreferrer");
    }
    await open([{ name: "nested.jpg", webkitRelativePath: "bundle/nested/nested.jpg" }]);
    assert.ok($("imageRefusals").children.length);
    assert.equal($("openBundles").children.length, 1);
    await $("openBundles").children[0].children[1].fire("click");
    assert.equal($("imageView").children.length, 0);
    assert.equal($("imageLike").disabled, true);
    assert.equal($("imageLove").disabled, true);
    assert.deepEqual((await ui.exportData({ version: 2 })).images.bundles, []);
    assert.deepEqual(revoked, created);
  });
});

for (const input of ["button", "ArrowUp"]) test("Love through " + input + " saves a like with love and reaches counts, worker and export", async () => {
  const db = fakeDB();
  await withUI(db, async ({ ui, $, open, worker }) => {
    assert.equal($("imageLove").disabled, $("imageLike").disabled);
    assert.equal($("imageLove").disabled, true);
    await open();
    assert.equal($("imageLove").disabled, false);
    worker.paused = true;
    if (input === "button") await $("imageLove").fire("click");
    else ui.handleKey(event("ArrowUp", { preventDefault() {} }));
    await tick();
    assert.equal(db.writes.length, 1);
    const saved = db.values.get("imageRatings/" + a);
    assert.deepEqual(saved, { ...rating(a, "like", null, true), ratedAt: saved.ratedAt });
    assert.equal($("imageLikes").textContent, "1");
    assert.equal($("imageLoved").textContent, "1");
    assert.equal($("imageDislikes").textContent, "0");
    assert.equal($("imageLove").disabled, true);
    assert.equal($("imageLove").disabled, $("imageLike").disabled);
    const message = worker.messages.at(-1);
    assert.deepEqual(message.ratings, new Map([[a, "like"]]));
    assert.deepEqual(message.loved, new Set([a]));
    worker.reply(message);
    worker.paused = false;
    assert.equal($("imageLove").disabled, false);
    await $("imageNoteButton").fire("click");
    $("imageNote").value = "The light";
    await $("imageNoteForm").fire("submit");
    const data = await ui.exportData({ version: 2 });
    assert.equal(data.version, 4);
    assert.equal(data.images.ratings[0].love, true);
    assert.equal(data.images.ratings[0].note, "The light");
    assert.equal(db.values.get("imageRatings/" + a).love, true);
    await $("imageUndo").fire("click"); await tick();
    assert.equal(db.values.has("imageRatings/" + a), false);
    assert.equal($("imageLoved").textContent, "0");
    assert.equal($("imageLikes").textContent, "0");
    assert.deepEqual(worker.messages.at(-1).loved, new Set());
  });
});

test("stored legacy ratings load and export with love false without rewriting storage", async () => {
  const legacy = { imageId: a, verdict: "like", note: "Old note", ratedAt: time }, db = fakeDB();
  db.values.set("imageRatings/" + a, legacy);
  await withUI(db, async ({ ui, $, open, worker }) => {
    await open();
    assert.equal($("imageLikes").textContent, "1");
    assert.equal($("imageLoved").textContent, "0");
    assert.deepEqual(worker.messages.at(-1).loved, new Set());
    assert.deepEqual((await ui.exportData({ version: 2 })).images.ratings, [{ ...legacy, love: false }]);
    assert.deepEqual(db.values.get("imageRatings/" + a), legacy);
    assert.equal(db.writes.length, 0);
  });
});

test("Loved counts only open-bundle ratings while loved closed images stay in the export", async () => {
  const closed = digest("closed image");
  await withUI(fakeDB([rating(a, "like", null, true), rating(b), rating(closed, "like", null, true)]), async ({ ui, $, open }) => {
    await open();
    assert.equal($("imageLikes").textContent, "2");
    assert.equal($("imageLoved").textContent, "1");
    const data = await ui.exportData({ version: 2 });
    assert.equal(data.images.ratings.filter((record) => record.love).length, 2);
    await $("openBundles").children[0].children[1].fire("click");
    assert.equal($("imageLikes").textContent, "0");
    assert.equal($("imageLoved").textContent, "0");
    assert.equal($("imageLove").disabled, true);
  });
});

test("a failed rating write remains visible to suggestions and export without leaking bundle data", async () => {
  const db = fakeDB([], [], async () => { throw { code: "unavailable" }; });
  await withUI(db, async ({ ui, $, open, status }) => {
    await open();
    await $("imageLike").fire("click"); await tick();
    assert.match(status.at(-1), /not saved.*stays in this tab/);
    assert.equal($("imageLikes").textContent, "1");
    const data = await ui.exportData({ version: 2, comparisons: [] });
    assert.equal(data.images.ratings[0].imageId, a);
    assert.equal(data.images.ratings[0].verdict, "like");
    assert.equal(data.images.ratings[0].note, null);
    assert.deepEqual(Object.keys(data.images), ["bundles", "ratings", "pairs"]);
    assert.ok(db.writes.every(({ path }) => path.startsWith("imageRatings/")));
  });
});

test("a queued note cannot overwrite an undo; a failed deletion is also absent from export", async () => {
  let release;
  const gate = new Promise((resolve) => { release = resolve; });
  const db = fakeDB([], [], async (kind, path, body) => {
    if (kind === "set" && body.note === "queued note") await gate;
    if (kind === "delete") throw new Error("delete refused");
  });
  await withUI(db, async ({ ui, $, open }) => {
    await open(); await $("imageLike").fire("click"); await tick();
    await $("imageNoteButton").fire("click");
    $("imageNote").value = "queued note";
    await $("imageNoteForm").fire("submit"); await tick();
    await $("imageUndo").fire("click"); await tick();
    assert.equal(db.writes.length, 2, "undo waits for the note's write");
    release();
    const data = await ui.exportData({ version: 2 });
    assert.deepEqual(db.writes.map(({ body }) => body && body.note), [null, "queued note", null]);
    assert.deepEqual(data.images.ratings, []);
    assert.equal($("imageLikes").textContent, "0");
  });
});

test("pair notes show two attributed images, validate, write only their document, and exclude the suggestion", async () => {
  const db = fakeDB([rating(a), rating(b, "dislike")]);
  await withUI(db, async ({ ui, $, open, status }) => {
    await open();
    assert.equal($("imagePairs").children.length, 1);
    await $("imagePairs").children[0].fire("click");
    assert.equal($("imagePairViews").children.length, 2);
    $("imagePairNote").value = " ";
    await $("imagePairForm").fire("submit");
    assert.match(status.at(-1), /1 to 280/);
    assert.equal(db.writes.length, 0);
    $("imagePairNote").value = "a\u0007b";
    await $("imagePairForm").fire("submit");
    assert.match(status.at(-1), /control characters/);
    assert.equal(db.writes.length, 0);
    $("imagePairNote").value = " Difference\tin  colour ";
    await $("imagePairForm").fire("submit"); await tick();
    assert.equal($("imagePairs").children.length, 0);
    assert.equal($("imagePairDetail").hidden, true);
    const data = await ui.exportData({ version: 2 });
    assert.equal(data.images.pairs[0].note, "Difference in colour");
    assert.equal(db.writes[0].path, `imagePairs/${a}_${b}`);
    assert.deepEqual(Object.keys(db.writes[0].body), ["likedImageId", "dislikedImageId", "note", "notedAt"]);
  });
});

test("late worker replies after a close cannot restore an image or enable rating", async () => {
  await withUI(fakeDB(), async ({ $, open, worker }) => {
    worker.paused = true;
    await open();
    const message = worker.messages.at(-1);
    await $("openBundles").children[0].children[1].fire("click");
    worker.reply(message);
    assert.equal($("imageView").children.length, 0);
    assert.equal($("imageLike").disabled, true);
    assert.equal($("imageLove").disabled, true);
  });
});

for (const collection of ["imageRatings", "imagePairs"]) test(`a failed initial ${collection} read pauses all answer writes until retry`, async () => {
  let unavailable = true;
  const original = rating(a, "dislike", "Original explanation"), originalPair = pair(b, a);
  const db = fakeDB([original], [originalPair], undefined, async (name) => {
    if (unavailable && name === collection) throw new Error("read unavailable");
  });
  await withUI(db, async ({ ui, $, open, status }) => {
    assert.match(status.at(-1), /Stored image answers.*could not be read.*Rating is paused.*not overwritten/);
    await open();
    assert.equal($("imageView").children.length, 1, "viewing remains available");
    assert.equal($("bundleFolder").disabled, false);
    for (const id of ["imageLike", "imageLove", "imageDislike", "imageUndo", "imageNoteButton", "imageNote", "imagePairNote", "imagePairSave"]) {
      assert.equal($(id).disabled, true, id);
    }
    assert.equal($("imageAnswersRetry").hidden, false);
    unavailable = false; // Writes would now succeed, but the baseline is still unknown.
    for (const id of ["imageLike", "imageLove", "imageDislike", "imageUndo", "imageNoteButton"]) await $(id).fire("click");
    for (const key of ["ArrowRight", "ArrowUp", "ArrowLeft", "z", "w"]) ui.handleKey(event(key, { preventDefault() {} }));
    $("imageNote").value = "Overwritten";
    await $("imageNoteForm").fire("submit");
    $("imagePairNote").value = "Overwritten pair";
    await $("imagePairForm").fire("submit");
    await tick();
    assert.equal(db.writes.length, 0);
    assert.deepEqual([...db.values.values()], [original, originalPair]);
    await $("imageAnswersRetry").fire("click"); await tick();
    assert.equal($("imageAnswersRetry").hidden, true);
    assert.equal($("imageDislikes").textContent, "1");
    assert.deepEqual((await ui.exportData({ version: 2 })).images.ratings, [original]);
    await $("imageLike").fire("click"); await tick();
    assert.equal(db.values.get(`imageRatings/${b}`).verdict, "like");
    assert.deepEqual(db.values.get(`imageRatings/${a}`), original);
  });
});

test("export retains known answers and Looks data when reads and writes fail", async () => {
  let unavailable = false;
  const storedRating = rating(c, "dislike", "Stored note"), storedPair = pair(a, c);
  const fail = async () => { if (unavailable) throw new Error("storage outage"); };
  const db = fakeDB([storedRating], [storedPair], fail, fail);
  await withUI(db, async ({ ui, $, open, status }) => {
    await open(); unavailable = true;
    await $("imageLike").fire("click"); await tick();
    assert.match(status.at(-1), /not saved/);
    const base = { kind: "tastelab-export", version: 2, comparisons: [{ answer: "A" }], sessions: [{ settled: {} }], presets: [{ look: {} }] };
    const data = await ui.exportData(base);
    for (const key of ["comparisons", "sessions", "presets"]) assert.deepEqual(data[key], base[key]);
    assert.equal(data.images.ratings.find((record) => record.imageId === a).verdict, "like");
    assert.deepEqual(data.images.ratings.find((record) => record.imageId === c), storedRating);
    assert.deepEqual(data.images.pairs, [storedPair]);
    assert.match(status.at(-1), /could not be refreshed.*another tab may be missing/);
    let warning = "";
    await ui.exportData(base, (message) => { warning = message; });
    assert.match(warning, /could not be refreshed.*another tab may be missing/);
  });
});

test("export after failed initial reads still contains Looks data and warns about missing stored answers", async () => {
  const db = fakeDB([rating()], [], undefined, async () => { throw new Error("storage outage"); });
  await withUI(db, async ({ ui, $, open, status }) => {
    await open(); await $("imageLike").fire("click"); await tick();
    const base = { kind: "tastelab-export", version: 2, comparisons: [{ answer: "B" }], presets: [{ look: {} }] };
    const data = await ui.exportData(base);
    assert.deepEqual(data.images.ratings, []); assert.deepEqual(data.images.pairs, []);
    assert.deepEqual(data.comparisons, base.comparisons); assert.deepEqual(data.presets, base.presets);
    assert.equal(db.writes.length, 0);
    assert.match(status.at(-1), /could not be refreshed.*another tab may be missing/);
  });
});

for (const action of ["undo", "rate"]) test(`export reflects a saved ${action} while its ratings read is paused`, async () => {
  let paused = false, release, began;
  const gate = new Promise((resolve) => { release = resolve; }), reading = new Promise((resolve) => { began = resolve; });
  const db = fakeDB([], [], undefined, async (name) => {
    if (paused && name === "imageRatings") { began(); await gate; }
  });
  await withUI(db, async ({ ui, $, open }) => {
    await open(); await $("imageLike").fire("click"); await tick();
    assert.equal(db.values.get(`imageRatings/${a}`).verdict, "like");
    paused = true;
    const exporting = ui.exportData({ version: 2 });
    await reading;
    await $(action === "undo" ? "imageUndo" : "imageLike").fire("click"); await tick();
    if (action === "undo") assert.equal(db.values.has(`imageRatings/${a}`), false);
    else assert.equal(db.values.get(`imageRatings/${b}`).verdict, "like");
    release();
    const data = await exporting;
    if (action === "undo") assert.deepEqual(data.images.ratings, []);
    else assert.deepEqual(data.images.ratings.map((record) => record.imageId), [a, b]);
  });
});
