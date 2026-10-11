import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { loadHostedBundles, INDEX_MAX_BYTES, MAX_HOSTED, THUMB_CONCURRENCY, REQUEST_TIMEOUT_MS,
  HOSTED_MAX_BYTES, REQUEST } from "../hosted.js";
import { MAX_MANIFEST_BYTES, MAX_ITEMS, THUMB_MAX_BYTES } from "../bundle.js";

const base = "https://site.invalid/taste/";
const digest = (value) => createHash("sha256").update(value).digest("hex");
const encode = (value) => new TextEncoder().encode(JSON.stringify(value));
const bundleId = (number) => `20261011T120000Z-${number.toString(16).padStart(8, "0")}`;
const index = (ids) => ({ format: "tastelab-bundle-index", version: 1, bundles: ids });
const turn = () => new Promise((resolve) => setImmediate(resolve));
async function until(check) {
  for (let i = 0; i < 100 && !check(); i++) await turn();
  assert.ok(check(), "The expected asynchronous step occurred.");
}

function fixture(number, count = 2) {
  const files = new Map(), data = Buffer.alloc(count * 1024);
  const items = Array.from({ length: count }, (_, row) => {
    const bytes = Uint8Array.of(0xff, 0xd8, 0xff, number, row), thumbSha256 = digest(bytes);
    files.set(thumbSha256 + ".jpg", bytes);
    data.writeUInt16LE(0x3c00, row * 1024 + row * 2);
    return { imageId: digest(`image-${number}-${row}`), thumbSha256, width: 512, height: 384, bytes: bytes.length,
      source: "wikimedia", sourceId: `synthetic-${number}-${row}`, title: "Synthetic image", credit: "Synthetic creator",
      pageUrl: "https://commons.wikimedia.org/wiki/File:Synthetic.jpg", licence: "CC0-1.0",
      licenceUrl: "https://creativecommons.org/publicdomain/zero/1.0/" };
  });
  const manifest = { format: "tastelab-images", version: 1, id: bundleId(number), createdAt: "2026-10-11T12:00:00Z",
    model: { id: "synthetic", weightsSha256: "ab".repeat(32) }, items,
    embeddings: { dtype: "float16", dim: 512, count, data: data.toString("base64") } };
  files.set("manifest.json", encode(manifest));
  return { id: manifest.id, manifest, files };
}

function site(bundles = [fixture(1), fixture(2)]) {
  const resources = new Map([[base + "bundles/index.json", encode(index(bundles.map((bundle) => bundle.id)))]]), calls = [];
  for (const bundle of bundles) for (const [name, bytes] of bundle.files) resources.set(base + `bundles/${bundle.id}/${name}`, bytes);
  const fetch = async (url, options) => {
    calls.push({ url, options });
    assert.ok(url.startsWith(base + "bundles/"));
    assert.deepEqual(Object.keys(options).sort(), ["cache", "credentials", "redirect", "signal"]);
    assert.deepEqual({ ...options, signal: undefined }, { ...REQUEST, signal: undefined });
    assert.ok(options.signal instanceof AbortSignal);
    const resource = resources.get(url);
    if (typeof resource === "function") return resource(options);
    return resource === undefined ? new Response(null, { status: 404 }) : new Response(resource);
  };
  return { bundles, resources, calls, fetch };
}

// Record the ids handed to the caller, which alone may cache them.
function opened() {
  const ids = [];
  return { ids, onBundle: ({ bundle }) => { ids.push(bundle.id); } };
}

function observed(chunks, headers = {}, extra = {}) {
  const state = { pulled: 0, cancelled: 0 };
  const body = new ReadableStream({
    pull(controller) {
      if (state.pulled < chunks.length) controller.enqueue(chunks[state.pulled++]);
      else controller.close();
    },
    cancel() { state.cancelled++; },
  }, { highWaterMark: 0 });
  const response = { status: 200, headers: new Headers(headers), redirected: false, url: "", body,
    arrayBuffer() { assert.fail("Do not read an entire response."); },
    text() { assert.fail("Do not read an entire response."); },
    json() { assert.fail("Do not read an entire response."); }, ...extra };
  return { response, state };
}

test("hosted request constants match the published contract", () => {
  assert.equal(INDEX_MAX_BYTES, 65536); assert.equal(MAX_HOSTED, 100); assert.equal(THUMB_CONCURRENCY, 6);
  assert.equal(REQUEST_TIMEOUT_MS, 30000); assert.equal(HOSTED_MAX_BYTES, 512 * 1024 * 1024);
  assert.deepEqual(REQUEST, { cache: "no-cache", credentials: "same-origin", redirect: "error" });
});

for (const protocol of ["file:///taste/", "data:text/plain,test"]) test(`a ${protocol.split(":")[0]} base makes no request`, async () => {
  assert.deepEqual(await loadHostedBundles({ base: protocol, fetch() { assert.fail("No request is allowed."); } }), { status: "absent" });
});

for (const [status, expected] of [[404, "absent"], [500, "unavailable"]]) test(`an index HTTP ${status} is ${expected}`, async () => {
  const fake = site(); fake.resources.set(base + "bundles/index.json", () => new Response(null, { status }));
  const result = await loadHostedBundles({ ...fake, base });
  assert.equal(result.status, expected); assert.equal(fake.calls.length, 1);
  if (expected === "unavailable") assert.equal(typeof result.error, "string");
});

test("an offline index is unreachable without exposing the fetch error", async () => {
  assert.deepEqual(await loadHostedBundles({ base, fetch: async () => { throw new Error("private URL and title"); } }), { status: "unreachable" });
});

test("two validated bundles reach onBundle with their manifest text and downloaded files in index order", async () => {
  const fake = site(), handed = [], progress = [];
  const result = await loadHostedBundles({ ...fake, base,
    onBundle: (result, files) => handed.push({ result, files }), onProgress: (value) => progress.push(value) });
  const ids = fake.bundles.map((bundle) => bundle.id);
  assert.deepEqual(result, { status: "loaded", imported: ids, refused: [], skipped: [], stopped: null, notAttempted: [] });
  assert.deepEqual(progress, [{ done: 0, total: 2 }, { done: 1, total: 2 }, { done: 2, total: 2 }]);
  for (const [i, { result: bundle, files }] of handed.entries()) {
    assert.equal(bundle.bundle.id, ids[i]); assert.equal(bundle.bundle.items.length, 2);
    assert.equal(bundle.manifestText, new TextDecoder().decode(fake.bundles[i].files.get("manifest.json")));
    assert.deepEqual(files, fake.bundles[i].files);
  }
  assert.equal(handed.length, 2);
  assert.equal(new Set(fake.calls.map(({ options }) => options.signal)).size, fake.calls.length, "each request owns a controller");
});

test("known ids are skipped without any bundle request and excluded from progress", async () => {
  const fake = site(), known = fake.bundles[0].id, progress = [];
  const result = await loadHostedBundles({ ...fake, base, knownIds: new Set([known]), onProgress: (value) => progress.push(value) });
  assert.deepEqual(result.skipped, [known]); assert.deepEqual(result.imported, [fake.bundles[1].id]);
  assert.equal(fake.calls.some(({ url }) => url.includes(known)), false);
  assert.deepEqual(progress, [{ done: 0, total: 1 }, { done: 1, total: 1 }]);
});

test("wrong thumbnail bytes refuse only that bundle and retain safe error text", async () => {
  const fake = site(), first = fake.bundles[0], handed = opened();
  const name = first.manifest.items[0].thumbSha256 + ".jpg";
  fake.resources.set(base + `bundles/${first.id}/${name}`, Uint8Array.of(0xff, 0xd8, 0xff, 99, 0));
  const result = await loadHostedBundles({ ...fake, base, onBundle: handed.onBundle });
  assert.deepEqual(result.imported, [fake.bundles[1].id]); assert.equal(result.refused[0].id, first.id);
  assert.doesNotMatch(result.refused[0].error, /Synthetic|https?:/); assert.deepEqual(handed.ids, result.imported);
});

const invalidIndexes = [
  ["extra key", { ...index([]), extra: true }], ["wrong format", { ...index([]), format: "other" }],
  ["wrong version", { ...index([]), version: 2 }], ["unsorted", index([bundleId(2), bundleId(1)])],
  ["duplicate", index([bundleId(1), bundleId(1)])], ["bad id", index(["../escape"])],
  ["id with newline", index([bundleId(1) + "\n"])], ["101 ids", index(Array.from({ length: 101 }, (_, i) => bundleId(i)))],
  ["non-array", { ...index([]), bundles: {} }], ["null", null], ["invalid JSON", Uint8Array.of(0x7b)],
  ["invalid UTF-8", Uint8Array.of(0xc3, 0x28)], ["oversized", new Uint8Array(INDEX_MAX_BYTES + 1)],
];
for (const [name, value] of invalidIndexes) test(`an index with ${name} is unavailable before any bundle request`, async () => {
  const fake = site(); fake.resources.set(base + "bundles/index.json", value instanceof Uint8Array ? value : encode(value));
  const result = await loadHostedBundles({ ...fake, base });
  assert.equal(result.status, "unavailable"); assert.equal(fake.calls.length, 1);
  assert.ok(result.error.length < 150); assert.doesNotMatch(result.error, /https?:|escape/);
});

test("an empty index is loaded with zero progress", async () => {
  const fake = site([]), progress = [];
  const result = await loadHostedBundles({ ...fake, base, onProgress: (value) => progress.push(value) });
  assert.deepEqual(result.imported, []); assert.deepEqual(progress, [{ done: 0, total: 0 }]);
});

const invalidManifests = [
  ["bad hash", (manifest) => { manifest.items[0].thumbSha256 = "../escape"; }],
  ["uppercase hash", (manifest) => { manifest.items[0].thumbSha256 = "AB".repeat(32); }],
  ["duplicate hash", (manifest) => { manifest.items[1].thumbSha256 = manifest.items[0].thumbSha256; }],
  ...[0, 1.5, THUMB_MAX_BYTES + 1, "5"].map((bytes) => ["bad bytes " + bytes, (manifest) => { manifest.items[0].bytes = bytes; }]),
  ["no items", (manifest) => { manifest.items = []; }], ["non-array items", (manifest) => { manifest.items = {}; }],
  ["too many items", (manifest) => { manifest.items = Array(MAX_ITEMS + 1).fill(manifest.items[0]); }],
  ["null item", (manifest) => { manifest.items[0] = null; }],
];
for (const [name, change] of invalidManifests) test(`manifest preconditions refuse ${name} without thumbnails`, async () => {
  const bundle = fixture(1); change(bundle.manifest); bundle.files.set("manifest.json", encode(bundle.manifest));
  const fake = site([bundle]), handed = opened();
  const result = await loadHostedBundles({ ...fake, base, onBundle: handed.onBundle });
  assert.equal(result.refused.length, 1); assert.equal(fake.calls.length, 2); assert.deepEqual(handed.ids, []);
});

for (const [name, bytes] of [["invalid JSON", Uint8Array.of(0x7b)], ["invalid UTF-8", Uint8Array.of(0xc3, 0x28)]]) {
  test(`a manifest with ${name} is refused without thumbnails`, async () => {
    const bundle = fixture(1); bundle.files.set("manifest.json", bytes); const fake = site([bundle]);
    const result = await loadHostedBundles({ ...fake, base });
    assert.equal(result.refused.length, 1); assert.equal(fake.calls.length, 2);
  });
}

test("THB-005: a manifest of a different index id never reaches onBundle", async () => {
  const a = fixture(1), b = fixture(2), fake = site([a]), handed = opened();
  fake.resources.set(base + `bundles/${a.id}/manifest.json`, b.files.get("manifest.json"));
  for (const [name, bytes] of b.files) if (name !== "manifest.json") fake.resources.set(base + `bundles/${a.id}/${name}`, bytes);
  const result = await loadHostedBundles({ ...fake, base, onBundle: handed.onBundle });
  assert.deepEqual(result.imported, []); assert.equal(result.refused[0].id, a.id);
  assert.equal(fake.calls.length, 2); assert.deepEqual(handed.ids, []);
});

for (const stage of ["index", "manifest", "thumbnail"]) for (const declared of [true, false]) {
  test(`THB-003: ${stage} oversize ${declared ? "Content-Length cancels before pulling" : "cancels at the crossing chunk"}`, async () => {
    const bundle = fixture(1), fake = site([bundle]), item = bundle.manifest.items[0];
    const bound = stage === "index" ? INDEX_MAX_BYTES : stage === "manifest" ? MAX_MANIFEST_BYTES : item.bytes;
    const url = base + "bundles/" + (stage === "index" ? "index.json" : `${bundle.id}/${stage === "manifest" ? "manifest.json" : item.thumbSha256 + ".jpg"}`);
    const tracked = observed([new Uint8Array(bound), Uint8Array.of(1), Uint8Array.of(2)], declared ? { "Content-Length": String(bound + 1) } : {});
    fake.resources.set(url, () => tracked.response);
    const result = await loadHostedBundles({ ...fake, base });
    assert.equal(stage === "index" ? result.status : result.refused.length, stage === "index" ? "unavailable" : 1);
    assert.equal(tracked.state.pulled, declared ? 0 : 2); assert.equal(tracked.state.cancelled, 1);
  });
}

test("THB-003: a null body is zero thumbnail bytes and refuses only its bundle", async () => {
  const fake = site(), first = fake.bundles[0];
  fake.resources.set(base + `bundles/${first.id}/${first.manifest.items[0].thumbSha256}.jpg`, () => new Response(null));
  const result = await loadHostedBundles({ ...fake, base });
  assert.equal(result.refused[0].id, first.id); assert.deepEqual(result.imported, [fake.bundles[1].id]);
});

test("THB-003: the budget stops before thumbnails of the crossing bundle or requests for later ids", async () => {
  const fake = site([fixture(1), fixture(2), fixture(3)]), [a, b, c] = fake.bundles;
  const size = (bundle) => bundle.files.get("manifest.json").length + bundle.manifest.items.reduce((total, item) => total + item.bytes, 0);
  const progress = [], result = await loadHostedBundles({ ...fake, base, maxBytes: size(a) + size(b) - 1,
    onProgress: (value) => progress.push(value) });
  assert.deepEqual(result.imported, [a.id]); assert.equal(result.stopped, "budget"); assert.deepEqual(result.notAttempted, [b.id, c.id]);
  assert.deepEqual(fake.calls.filter(({ url }) => url.includes(b.id)).map(({ url }) => url), [base + `bundles/${b.id}/manifest.json`]);
  assert.equal(fake.calls.some(({ url }) => url.includes(c.id)), false);
  assert.deepEqual(progress, [{ done: 0, total: 3 }, { done: 1, total: 3 }]);
});

for (const stage of ["index headers", "index body", "manifest headers", "thumbnail body"]) {
  test(`THB-004: stalled ${stage} time out and allow later bundles`, async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const fake = site(), first = fake.bundles[0]; let signal, pulled = false, cancelled = false;
    const url = base + "bundles/" + (stage.startsWith("index") ? "index.json" : `${first.id}/${stage.startsWith("manifest") ? "manifest.json" : first.manifest.items[0].thumbSha256 + ".jpg"}`);
    fake.resources.set(url, (options) => {
      signal = options.signal;
      if (stage.endsWith("headers")) return new Promise((resolve, reject) => signal.addEventListener("abort", () => reject(new Error("Aborted")), { once: true }));
      return { status: 200, headers: new Headers(), redirected: false, url: "",
        body: new ReadableStream({ pull() { pulled = true; }, cancel() { cancelled = true; } }, { highWaterMark: 0 }) };
    });
    const pending = loadHostedBundles({ ...fake, base, timeoutMs: 30 });
    await until(() => signal && (stage.endsWith("headers") || pulled));
    t.mock.timers.tick(30); assert.equal(signal.aborted, true, "the request timer aborted it");
    const result = await pending;
    if (stage.startsWith("index")) assert.deepEqual(result, { status: "unreachable" });
    else { assert.equal(result.refused[0].id, first.id); assert.deepEqual(result.imported, [fake.bundles[1].id]); }
    if (stage.endsWith("body")) assert.equal(cancelled, true);
    const latestSignal = fake.calls.at(-1).options.signal;
    t.mock.timers.tick(100); assert.equal(latestSignal.aborted, stage.startsWith("index"), "settled request timers are cleared");
  });
}

test("THB-004: six in-flight thumbnails abort and all settle before the next bundle starts", async () => {
  const a = fixture(1, 8), b = fixture(2), fake = site([a, b]), pending = [];
  let active = 0, peak = 0, nextStartedWith = null;
  for (const item of a.manifest.items) fake.resources.set(base + `bundles/${a.id}/${item.thumbSha256}.jpg`, (options) => {
    active++; peak = Math.max(peak, active);
    return new Promise((resolve, reject) => pending.push({ signal: options.signal, reject })).finally(() => { active--; });
  });
  fake.resources.set(base + `bundles/${b.id}/manifest.json`, () => {
    nextStartedWith = active; return new Response(b.files.get("manifest.json"));
  });
  const loading = loadHostedBundles({ ...fake, base });
  try {
    await until(() => pending.length >= 6);
    assert.equal(pending.length, 6); assert.equal(peak, 6);
    pending[0].reject(new Error("First thumbnail failed"));
    await until(() => pending.slice(1).every(({ signal }) => signal.aborted));
    assert.equal(nextStartedWith, null, "aborted requests still count until settled");
    for (const sibling of pending.slice(1)) { sibling.reject(new Error("Abort settled")); await turn(); }
    const result = await loading;
    assert.equal(nextStartedWith, 0); assert.equal(peak, 6); assert.equal(pending.length, 6, "no queued thumbnail started after failure");
    assert.equal(result.refused[0].id, a.id); assert.deepEqual(result.imported, [b.id]);
  } finally {
    for (const request of pending) request.reject(new Error("Test cleanup"));
    await loading;
  }
});

for (const stage of ["index", "manifest", "thumbnail"]) for (const change of [{ redirected: true }, { url: base + "elsewhere" }]) {
  test(`THB-007: ${stage} refuses ${Object.keys(change)[0]} redirects`, async () => {
    const a = fixture(1), fake = site([a]), name = stage === "index" ? "index.json" : `${a.id}/${stage === "manifest" ? "manifest.json" : a.manifest.items[0].thumbSha256 + ".jpg"}`;
    const url = base + "bundles/" + name, tracked = observed([fake.resources.get(url)], {}, change);
    fake.resources.set(url, () => tracked.response);
    const result = await loadHostedBundles({ ...fake, base });
    assert.equal(stage === "index" ? result.status : result.refused.length, stage === "index" ? "unavailable" : 1);
    assert.equal(tracked.state.pulled, 0); assert.equal(tracked.state.cancelled, 1);
  });
}

test("a response URL identical to its requested URL is accepted", async () => {
  const fake = site([]), url = base + "bundles/index.json", tracked = observed([fake.resources.get(url)], {}, { url });
  fake.resources.set(url, () => tracked.response);
  assert.equal((await loadHostedBundles({ ...fake, base })).status, "loaded");
});
