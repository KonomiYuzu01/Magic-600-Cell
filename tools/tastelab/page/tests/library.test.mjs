import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readSelection, openBundle, loadCached, closeBundle, mergeBundles } from "../library.js";
import { nextImage, suggestPairs } from "../images.js";

// Everything is synthetic and held in memory, including the File and cache fakes.
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
const encode = (value) => new TextEncoder().encode(value);
function fixture(suffix = "0", indices = [0, 1]) {
  const files = new Map(), data = Buffer.alloc(indices.length * 1024);
  const items = indices.map((index, row) => {
    const bytes = Uint8Array.of(0xff, 0xd8, 0xff, index), thumbSha256 = digest(bytes);
    files.set(thumbSha256 + ".jpg", bytes);
    data.writeUInt16LE(0x3c00, row * 1024 + index * 2);
    return { imageId: digest(`synthetic original ${index}`), thumbSha256, width: 512, height: 384, bytes: bytes.length,
      source: "wikimedia", sourceId: `synthetic-${index}`, title: `Bundle ${suffix}`, credit: "Synthetic creator",
      pageUrl: "https://commons.wikimedia.org/wiki/File:Synthetic.jpg", licence: "CC-BY-SA-4.0",
      licenceUrl: "https://creativecommons.org/licenses/by-sa/4.0/" };
  });
  const manifest = { format: "tastelab-images", version: 1, id: `20261004T120000Z-0123abc${suffix}`, createdAt: "2026-10-04T12:00:00Z",
    model: { id: "synthetic-model", weightsSha256: "ab".repeat(32) }, items,
    embeddings: { dtype: "float16", dim: 512, count: indices.length, data: data.toString("base64") } };
  const manifestText = JSON.stringify(manifest);
  files.set("manifest.json", encode(manifestText));
  return { manifest, manifestText, files };
}
function fileList(files, folder = "bundle") {
  return [...files].map(([name, bytes]) => ({ name, size: bytes.length, webkitRelativePath: folder === null ? "" : folder + "/" + name,
    async arrayBuffer() { return bytes.slice().buffer; } }));
}
function memoryCache() {
  const values = new Map(), calls = [];
  return {
    values, calls,
    async keys() { calls.push(["keys"]); return [...values.keys()]; },
    async get(key) { calls.push(["get", key]); return structuredClone(values.get(key)); },
    async put(key, value) { calls.push(["put", key]); values.set(key, structuredClone(value)); },
    async delete(key) { calls.push(["delete", key]); values.delete(key); },
  };
}

test("readSelection accepts one folder or a flat file selection with no relative paths", async () => {
  const f = fixture();
  for (const folder of ["bundle", null]) {
    const selected = fileList(f.files, folder), result = await readSelection(selected);
    assert.equal(result.ok, true);
    assert.equal(result.manifestText, f.manifestText);
    assert.deepEqual(result.files, f.files);
    assert.ok(result.files instanceof Map);
    assert.ok([...result.files.values()].every((bytes) => bytes instanceof Uint8Array));
  }
  const selected = fileList(f.files, null); selected.forEach((file) => { delete file.webkitRelativePath; });
  assert.equal((await readSelection(selected)).ok, true);
});

const selectionRefusals = [
  ["repeated names", (files) => { files.push({ ...files[0] }); }],
  ["nested folder", (files) => { files[0].webkitRelativePath = "bundle/nested/" + files[0].name; }],
  ["two different folders", (files) => { files[0].webkitRelativePath = "other/" + files[0].name; }],
  ["one path segment", (files) => { files[0].webkitRelativePath = files[0].name; }],
  ["empty folder segment", (files) => { files[0].webkitRelativePath = "/" + files[0].name; }],
  ["parent folder segment", (files) => { files[0].webkitRelativePath = "../" + files[0].name; }],
  ["backslash path", (files) => { files[0].webkitRelativePath = "bundle\\" + files[0].name; }],
  ["mismatched leaf name", (files) => { files[0].webkitRelativePath = "bundle/other.jpg"; }],
  ["mixed flat and folder paths", (files) => { files[0].webkitRelativePath = ""; }],
  ["non-string relative path", (files) => { files[0].webkitRelativePath = 1; }],
  ["non-direct file name", (files) => { files[0].name = "nested/image.jpg"; }],
  ["file name control", (files) => { files[0].name += "\n"; }],
  ["missing manifest", (files) => { files.splice(files.findIndex((file) => file.name === "manifest.json"), 1); }],
  ["empty selection", (files) => { files.length = 0; }],
];
for (const [name, mutate] of selectionRefusals) test(`selection refuses ${name} before reading bytes`, async () => {
  const files = fileList(fixture().files); mutate(files);
  let reads = 0;
  files.forEach((file) => { file.arrayBuffer = async () => { reads++; return new ArrayBuffer(0); }; });
  const result = await readSelection(files);
  assert.equal(result.ok, false); assert.ok(result.errors.length); assert.equal(reads, 0);
});

test("selection read failures and invalid UTF-8 return errors without a partial selection", async () => {
  const f = fixture(), files = fileList(f.files);
  files[0].arrayBuffer = async () => { throw new Error("read failed"); };
  const failed = await readSelection(files);
  assert.equal(failed.ok, false); assert.equal(failed.files, undefined);
  f.files.set("manifest.json", Uint8Array.of(0xff));
  assert.equal((await readSelection(fileList(f.files))).ok, false);
});

test("open, cache, reload, reopen and close use only the injected browser cache", async () => {
  const f = fixture(), cache = memoryCache(), selection = await readSelection(fileList(f.files));
  const opened = await openBundle(selection, cache);
  assert.equal(opened.ok, true); assert.equal(opened.cached, true);
  assert.deepEqual(cache.calls, [["put", f.manifest.id]]);
  assert.deepEqual(cache.values.get(f.manifest.id), { manifestText: f.manifestText, files: [...f.files] });
  assert.deepEqual(await loadCached(cache), { bundles: [opened.bundle], dropped: [], unavailable: false });
  assert.equal((await openBundle(selection, cache)).ok, true);
  assert.equal(cache.values.size, 1, "reopening an id does not duplicate the cache entry");
  await closeBundle(f.manifest.id, cache);
  assert.deepEqual(await loadCached(cache), { bundles: [], dropped: [], unavailable: false });
});

test("invalid selection or bundle never writes to the cache", async () => {
  const cache = memoryCache(), selection = await readSelection([]);
  assert.deepEqual(await openBundle(selection, cache), selection);
  const f = fixture(); f.files.get(f.manifest.items[0].thumbSha256 + ".jpg")[3] ^= 1;
  const result = await openBundle(await readSelection(fileList(f.files)), cache);
  assert.equal(result.ok, false); assert.ok(result.errors.length);
  assert.deepEqual(cache.calls, []); assert.equal(cache.values.size, 0);
});

test("a rejecting put still opens the complete validated bundle with cached false", async () => {
  const f = fixture(), cache = memoryCache();
  cache.put = async () => { throw new Error("quota"); };
  const opened = await openBundle(await readSelection(fileList(f.files)), cache);
  assert.equal(opened.ok, true); assert.equal(opened.cached, false);
  assert.deepEqual(opened.bundle.items, f.manifest.items); assert.equal(cache.values.size, 0);
});

for (const corruption of ["manifest", "thumbnail"]) test(`a cached ${corruption} changed after opening is dropped while other bundles still open`, async () => {
  const a = fixture("0"), b = fixture("1", [2, 3]), cache = memoryCache();
  await openBundle(await readSelection(fileList(a.files)), cache);
  const other = await openBundle(await readSelection(fileList(b.files)), cache);
  const entry = cache.values.get(a.manifest.id);
  if (corruption === "manifest") {
    const manifest = JSON.parse(entry.manifestText); manifest.items[0].source = "archive";
    entry.manifestText = JSON.stringify(manifest);
    entry.files.find(([name]) => name === "manifest.json")[1] = encode(entry.manifestText);
  } else entry.files.find(([name]) => name !== "manifest.json")[1][3] ^= 1;
  assert.deepEqual(await loadCached(cache), { bundles: [other.bundle], dropped: [a.manifest.id], unavailable: false });
  assert.equal(cache.values.has(a.manifest.id), false); assert.equal(cache.values.has(b.manifest.id), true);
  assert.ok(cache.calls.some(([method, key]) => method === "delete" && key === a.manifest.id));
});

test("an empty cache returns no bundles so the UI can request the folder, without changing answers", async () => {
  const ratings = new Map([["id", "like"]]), notes = new Map([["id", "keep"]]), before = structuredClone({ ratings, notes });
  assert.deepEqual(await loadCached(memoryCache()), { bundles: [], dropped: [], unavailable: false });
  assert.deepEqual({ ratings, notes }, before);
  assert.deepEqual(mergeBundles([]), { order: [], vectors: new Map(), items: new Map() });
});

for (const method of ["keys", "get", "delete"]) test(`a cache whose ${method} rejects is unavailable and returns no partial bundles`, async () => {
  const cache = memoryCache(), f = fixture(), g = fixture("1", [2, 3]);
  await openBundle(await readSelection(fileList(f.files)), cache);
  await openBundle(await readSelection(fileList(g.files)), cache);
  const original = cache[method];
  if (method === "delete") cache.values.set(g.manifest.id, { manifestText: "{}", files: [] });
  cache[method] = async (...args) => {
    if (method !== "get" || args[0] === g.manifest.id) throw new Error("cache blocked");
    return original(...args);
  };
  const result = await loadCached(cache);
  assert.equal(result.unavailable, true); assert.deepEqual(result.bundles, []); assert.deepEqual(result.dropped, []);
});

const cacheRefusals = [
  ["missing cache value", () => undefined],
  ["missing files", () => ({ manifestText: "{}" })],
  ["non-array files", () => ({ manifestText: "{}", files: {} })],
  ["malformed file tuple", (entry) => ({ ...entry, files: [null] })],
  ["extra tuple element", (entry) => ({ ...entry, files: [["manifest.json", encode(entry.manifestText), "extra"]] })],
  ["repeated cached name", (entry) => ({ ...entry, files: [...entry.files, entry.files[0]] })],
  ["manifest bytes disagree", (entry) => ({ ...entry, files: entry.files.map(([name, bytes]) => [name, name === "manifest.json" ? encode("{}") : bytes]) })],
];
for (const [name, mutate] of cacheRefusals) test(`cache revalidation drops ${name}`, async () => {
  const f = fixture(), cache = memoryCache();
  await openBundle(await readSelection(fileList(f.files)), cache);
  cache.values.set(f.manifest.id, mutate(cache.values.get(f.manifest.id)));
  assert.deepEqual(await loadCached(cache), { bundles: [], dropped: [f.manifest.id], unavailable: false });
  assert.equal(cache.values.size, 0);
});

test("a cached manifest id must match its cache key", async () => {
  const f = fixture(), cache = memoryCache();
  await openBundle(await readSelection(fileList(f.files)), cache);
  cache.values.set("wrong-id", cache.values.get(f.manifest.id)); cache.values.delete(f.manifest.id);
  assert.deepEqual(await loadCached(cache), { bundles: [], dropped: ["wrong-id"], unavailable: false });
});

test("close deletes only its bundle from cache, preserves answers, and reports cache rejection", async () => {
  const f = fixture(), cache = memoryCache(), ratings = new Map([[f.manifest.items[0].imageId, "like"]]);
  await openBundle(await readSelection(fileList(f.files)), cache);
  const before = structuredClone(ratings); cache.calls.length = 0;
  await closeBundle(f.manifest.id, cache);
  assert.deepEqual(cache.calls, [["delete", f.manifest.id]]); assert.deepEqual(ratings, before);
  cache.delete = async () => { throw new Error("cache blocked"); };
  await assert.rejects(closeBundle(f.manifest.id, cache), /cache blocked/);
  assert.deepEqual(ratings, before);
});

test("TLPLAN-01 merged rows, vectors, ratings and pairs use imageId rather than thumbnail hash", async () => {
  const f = fixture(), cache = memoryCache(), opened = await openBundle(await readSelection(fileList(f.files)), cache);
  const merged = mergeBundles([opened.bundle]), [a, b] = opened.bundle.items;
  assert.deepEqual(merged.order, [a.imageId, b.imageId]);
  assert.deepEqual([...merged.items.keys()], merged.order); assert.deepEqual([...merged.vectors.keys()], merged.order);
  assert.notEqual(a.imageId, a.thumbSha256); assert.equal(merged.items.has(a.thumbSha256), false);
  const ratings = new Map([[a.imageId, "like"]]);
  assert.equal(nextImage({ ...merged, ratings, skips: new Map(), ratedCount: 1 }), b.imageId);
  ratings.set(b.imageId, "dislike");
  assert.deepEqual(suggestPairs({ vectors: merged.vectors, ratings, notedPairs: new Set() }),
    [{ likedImageId: a.imageId, dislikedImageId: b.imageId, similarity: 0 }]);
  assert.equal(ratings.has(a.thumbSha256), false);
});

test("two bundles show a shared image once, in opening and item order; closing one keeps the other copy", async () => {
  const a = fixture("0", [0, 1]), b = fixture("1", [1, 2]), cache = memoryCache();
  const openedA = await openBundle(await readSelection(fileList(a.files)), cache);
  const openedB = await openBundle(await readSelection(fileList(b.files)), cache);
  const before = structuredClone([openedA.bundle, openedB.bundle]), shared = a.manifest.items[1].imageId;
  const merged = mergeBundles([openedA.bundle, openedB.bundle]);
  assert.deepEqual(merged.order, [a.manifest.items[0].imageId, shared, b.manifest.items[1].imageId]);
  assert.equal(merged.items.size, 3); assert.equal(merged.items.get(shared).title, "Bundle 0");
  assert.equal(merged.vectors.get(shared)[1], 1); assert.equal(merged.vectors.get(b.manifest.items[1].imageId)[2], 1);
  assert.equal(merged.vectors.get(shared).length, 512);
  assert.deepEqual([openedA.bundle, openedB.bundle], before);
  const reversed = mergeBundles([openedB.bundle, openedA.bundle]);
  assert.deepEqual(reversed.order, [shared, b.manifest.items[1].imageId, a.manifest.items[0].imageId]);
  assert.equal(reversed.items.get(shared).title, "Bundle 1");
  await closeBundle(openedA.bundle.id, cache);
  const remaining = mergeBundles((await loadCached(cache)).bundles);
  assert.deepEqual(remaining.order, b.manifest.items.map((item) => item.imageId));
  assert.equal(remaining.items.get(shared).title, "Bundle 1");
});

const manifestMaxBytes = 8 * 1024 * 1024, thumbMaxBytes = 204800, embeddingMaxBytes = 300 * 512 * 2;
const sizeRefusals = [
  ["oversized manifest", (files) => { files.find((file) => file.name === "manifest.json").size = manifestMaxBytes + 1; }],
  ["oversized thumbnail", (files) => { files[0].size = thumbMaxBytes + 1; }],
  ["oversized embeddings file", (files) => { files.push({ name: "embeddings.bin", size: embeddingMaxBytes + 1, webkitRelativePath: "bundle/embeddings.bin" }); }],
  ["too many files", (files) => {
    for (let i = files.length; i < 302; i++) files.push({ name: digest(`extra ${i}`) + ".jpg", size: 1, webkitRelativePath: "" });
    files.forEach((file) => { file.webkitRelativePath = ""; });
  }],
  ["oversized total", (files) => {
    files.length = 0;
    files.push({ name: "manifest.json", size: manifestMaxBytes + 1, webkitRelativePath: "" });
    for (let i = 0; i < 300; i++) files.push({ name: digest(`full ${i}`) + ".jpg", size: thumbMaxBytes, webkitRelativePath: "" });
  }],
];
for (const [name, mutate] of sizeRefusals) test(`selection refuses ${name} before any arrayBuffer call`, async () => {
  const files = fileList(fixture().files); mutate(files);
  let reads = 0;
  files.forEach((file) => { file.arrayBuffer = async () => { reads++; return new ArrayBuffer(0); }; });
  const result = await readSelection(files);
  assert.equal(result.ok, false); assert.ok(result.errors.length); assert.equal(reads, 0);
  if (name === "oversized total") assert.match(result.errors[0], /total/i);
});

test("selection refuses a separate embeddings file even within the vector byte bound", async () => {
  const files = fileList(fixture().files);
  files.push({ name: "embeddings.bin", size: embeddingMaxBytes, webkitRelativePath: "bundle/embeddings.bin" });
  let reads = 0;
  files.forEach((file) => { file.arrayBuffer = async () => { reads++; return new ArrayBuffer(0); }; });
  assert.equal((await readSelection(files)).ok, false); assert.equal(reads, 0);
});

test("selection refuses a small separate embeddings file by its name", async () => {
  const files = fileList(fixture().files);
  files.push({ name: "embeddings.bin", size: 1024, webkitRelativePath: "bundle/embeddings.bin" });
  let reads = 0;
  files.forEach((file) => { file.arrayBuffer = async () => { reads++; return new ArrayBuffer(0); }; });
  assert.equal((await readSelection(files)).ok, false); assert.equal(reads, 0);
});

test("bounded valid selections read their files and still open", async () => {
  const f = fixture(), files = fileList(f.files), cache = memoryCache();
  let reads = 0;
  files.forEach((file) => { const read = file.arrayBuffer; file.arrayBuffer = async () => { reads++; return read(); }; });
  const selected = await readSelection(files);
  assert.equal(selected.ok, true); assert.equal(reads, files.length);
  assert.equal((await openBundle(selected, cache)).ok, true);
});

test("selection checks actual manifest and thumbnail lengths even if declared sizes are smaller", async () => {
  for (const name of ["manifest.json", fixture().manifest.items[0].thumbSha256 + ".jpg"]) {
    const files = fileList(fixture().files), file = files.find((file) => file.name === name);
    file.arrayBuffer = async () => new ArrayBuffer((name === "manifest.json" ? manifestMaxBytes : thumbMaxBytes) + 1);
    const result = await readSelection(files);
    assert.equal(result.ok, false); assert.equal(result.files, undefined);
  }
});
