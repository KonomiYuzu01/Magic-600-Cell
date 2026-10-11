import test from "node:test";
import assert from "node:assert/strict";
import { setImmediate as tick } from "node:timers/promises";
import { createLocalDb } from "../local-db.js";
import { defaultSpace } from "../../core/space.js";

const STANDALONE_STATUS = "Answers are stored in this browser only. Clearing this site's data deletes them; export to keep a copy.";
const TAB_STATUS = "Storage is not available here; answers stay in this tab.";
const OWNER_STATUS = "Only the owner of this page can record answers.";
const NO_WEBGL = "WebGL 2 is not available in this browser.";
const look = Object.fromEntries(defaultSpace().params.map((p) => [p.id, p.kind === "integer" ? Math.round((p.min + p.max) / 2) : (p.min + p.max) / 2]));
const comparisons = ["A", "B"].map((answer, i) => ({
  answer, lookA: look, lookB: look, family: "f1", scene: "solving", session: "fixture", t: `2026-10-10T12:00:0${i}.000Z`,
}));
const presetData = { presets: [{ family: "f1", scene: "solving", look }], shared: [] };
const compare = (a, b) => a < b ? -1 : a > b ? 1 : 0;

// `gate`, when given, holds the first store read until it resolves.
function artifactDb(records = comparisons, gate = null) {
  const reads = [];
  function query(name, field = null, direction = "asc", filters = [], count = Infinity) {
    return {
      orderBy(next, order = "asc") { return query(name, next, order, filters, count); },
      where(key, op, value) {
        assert.ok(["==", ">"].includes(op));
        return query(name, field, direction, [...filters, [key, op, value]], count);
      },
      limit(next) { return query(name, field, direction, filters, Math.min(next, 1000)); },
      async get() {
        reads.push(name);
        const rows = (name === "comparisons" ? records : []).map((body, i) => ({ id: String(i).padStart(4, "0"), body }))
          .filter(({ body }) => filters.every(([key, op, value]) => op === "==" ? body[key] === value : body[key] > value))
          .sort((a, b) => (field ? compare(a.body[field], b.body[field]) * (direction === "desc" ? -1 : 1) : 0) || compare(a.id, b.id))
          .slice(0, count);
        const docs = rows.map(({ id, body }) => ({ id, data: () => structuredClone(body) }));
        return { docs, size: docs.length };
      },
    };
  }
  return { reads, collection: query, doc(path) {
    assert.equal(path, "space/current");
    return { async get() { await gate; reads.push(path); return { exists: false }; } };
  } };
}

// The answer adapter uses the real createLocalDb. The separate Images cache is
// unavailable in these fixtures, as it can be in either page mode.
function fakeIndexedDB({ unavailable = false } = {}) {
  const opens = [], values = new Map();
  let created = false;
  const connection = {
    close() {},
    objectStoreNames: { contains: () => created },
    createObjectStore() { created = true; return { createIndex() {} }; },
    transaction() {
      const transaction = { abort() {} };
      const operation = (read, write = () => {}) => {
        const request = {};
        queueMicrotask(() => {
          request.result = structuredClone(read());
          request.onsuccess();
          queueMicrotask(() => { write(); transaction.oncomplete(); });
        });
        return request;
      };
      transaction.objectStore = () => ({
        get: (key) => operation(() => values.get(JSON.stringify(key))),
        put(body) {
          const stored = structuredClone(body), key = JSON.stringify([body.collection, body.id]);
          return operation(() => key, () => values.set(key, stored));
        },
        index: () => ({ getAll: (name) => operation(() => [...values.values()].filter((value) => value.collection === name)) }),
      });
      return transaction;
    },
  };
  return { opens, indexedDB: { open(name, version) {
    opens.push(name);
    if (unavailable || name === "tastelab-images") throw new Error("Browser storage is unavailable.");
    assert.equal(name, "tastelab-answers");
    assert.equal(version, 1);
    const request = { result: connection };
    queueMicrotask(() => {
      if (!created) request.onupgradeneeded();
      request.onsuccess();
    });
    return request;
  } } };
}

class Element {
  children = []; listeners = new Map(); textContent = ""; hidden = false; disabled = false; value = "";
  style = { setProperty() {} }; dataset = {}; classes = new Set();
  classList = {
    toggle: (name, enabled) => { if (enabled) this.classes.add(name); else this.classes.delete(name); },
    contains: (name) => this.classes.has(name),
  };
  constructor(tagName = "div", gl = null) { this.tagName = tagName; this.gl = gl; this.clicks = 0; }
  addEventListener(type, listener) { this.listeners.set(type, [...(this.listeners.get(type) || []), listener]); }
  append(...children) { for (const child of children) this.appendChild(child); }
  appendChild(child) { this.children.push(child); child.parent = this; return child; }
  replaceChildren(...children) { this.children = []; this.append(...children); }
  setAttribute(name, value) { this[name] = value; }
  focus() {}
  blur() {}
  remove() { this.parent.children = this.parent.children.filter((child) => child !== this); this.parent = null; this.removed = true; }
  click() { this.clicks++; }
  getContext(kind) { assert.equal(kind, "webgl2"); return this.gl; }
  async fire(type) {
    const event = { target: this, currentTarget: this, preventDefault() {} };
    for (const listener of this.listeners.get(type) || []) await listener(event);
  }
}

async function until(predicate, description) {
  for (let turn = 0; turn < 100; turn++) {
    if (predicate()) return;
    await tick();
  }
  assert.fail("App did not settle: " + description);
}

// `early` runs right after the import, before boot settles; without `webgl`
// the canvases give no context and boot stops at the Preview error.
async function withApp(name, { claude, indexedDB, early = null, webgl = true }, run) {
  const keys = ["document", "window", "Worker", "indexedDB", "URL", "Blob", "requestAnimationFrame", "setTimeout"];
  const saved = keys.map((key) => Object.getOwnPropertyDescriptor(globalThis, key));
  const NativeURL = globalThis.URL, NativeBlob = globalThis.Blob;
  const elements = new Map(), workers = [], blobs = [], anchors = [], urls = [], revoked = [], scheduled = [], frames = [];
  // Constants are harmless numbers; shader/program status and no-op methods
  // are truthy so both real Preview constructors succeed.
  const gl = new Proxy({}, { get: (_, key) => key === key.toUpperCase() ? 0 : () => true });
  const $ = (id) => {
    if (!elements.has(id)) elements.set(id, new Element("div", webgl ? gl : null));
    return elements.get(id);
  };
  const document = {
    getElementById: $, querySelectorAll: () => [], addEventListener() {}, body: new Element("body"),
    createElement(tag) { const element = new Element(tag); if (tag === "a") anchors.push(element); return element; },
  };
  globalThis.document = document;
  globalThis.window = claude ? { claude } : {};
  globalThis.indexedDB = indexedDB;
  globalThis.Blob = class extends NativeBlob {
    constructor(parts, options) { super(parts, options); blobs.push(this); }
  };
  globalThis.URL = class extends NativeURL {
    static createObjectURL(blob) { assert.equal(blob, blobs.at(-1)); const url = "blob:export-" + urls.length; urls.push(url); return url; }
    static revokeObjectURL(url) { revoked.push(url); }
  };
  globalThis.requestAnimationFrame = (callback) => { frames.push(callback); return frames.length; };
  globalThis.setTimeout = (callback, delay) => { scheduled.push({ callback, delay }); return scheduled.length; };
  globalThis.Worker = class {
    messages = [];
    constructor(url, options) { this.url = url; assert.equal(options.type, "module"); workers.push(this); }
    postMessage(message) {
      this.messages.push(structuredClone(message));
      assert.ok(this.url.pathname.endsWith("/worker.js"), "No image work is needed without a bundle.");
      let reply;
      if (message.type === "init") reply = {
        type: "pair", family: message.family, scene: message.scene, lookA: look, lookB: look, kind: "explore",
        counts: { answers: message.records.length, pruned: 0, skipped: 0 }, relevance: [], fitted: false, settledRegret: 0.1,
      };
      else if (message.type === "presets") reply = { type: "presets", id: message.id, ...presetData };
      else assert.fail("Unexpected worker request: " + message.type);
      queueMicrotask(() => this.onmessage({ data: reply }));
    }
  };
  try {
    await import(new NativeURL(`../app.js?case=${name}`, import.meta.url));
    const worker = workers.find((worker) => worker.url.pathname.endsWith("/worker.js"));
    if (early) await early({ $ });
    if (webgl) {
      await until(() => worker?.messages.some((message) => message.type === "init") && !document.body.classList.contains("busy"), "boot and first pair");
      assert.equal(frames.length, 2, "Both previews started.");
    } else await until(() => $("status").textContent === NO_WEBGL, "boot without WebGL 2");
    const init = worker.messages.find((message) => message.type === "init");
    await run({ $, worker, init, blobs, anchors, urls, revoked, scheduled });
  } finally {
    await tick();
    keys.forEach((key, i) => { if (saved[i]) Object.defineProperty(globalThis, key, saved[i]); else delete globalThis[key]; });
  }
}

function assertExport(data, expected, worker) {
  assert.equal(data.kind, "tastelab-export");
  assert.deepEqual(data.comparisons, expected);
  assert.deepEqual(data.presets, presetData.presets);
  assert.deepEqual(data.shared, presetData.shared);
  assert.equal(worker.messages.filter((message) => message.type === "presets").length, 1);
}

async function browserExport({ $, worker, blobs, anchors, urls, revoked, scheduled }, expected) {
  await $("export").fire("click");
  assert.equal($("status").textContent, "Exported.");
  assert.equal(blobs.length, 1);
  assert.equal(blobs[0].type, "application/json");
  assertExport(JSON.parse(await blobs[0].text()), expected, worker);
  assert.equal(anchors.length, 1);
  const anchor = anchors[0];
  assert.equal(anchor.download, "tastelab-export.json");
  assert.equal(anchor.href, urls[0]);
  assert.equal(anchor.hidden, true);
  assert.equal(anchor.clicks, 1);
  assert.equal(anchor.removed, true);
  assert.equal(urls.length, 1);
  assert.deepEqual(revoked, []);
  assert.equal(scheduled.length, 1);
  assert.equal(scheduled[0].delay, 60000);
  scheduled[0].callback();
  assert.deepEqual(revoked, urls);
}

test("real app boots the artifact owner store and exports through downloads", async () => {
  const db = artifactDb(), browser = fakeIndexedDB(), saves = [], used = [];
  const capabilities = { db, user: { async isOwner() { return true; } }, downloads: { async save(body) { saves.push(body); } } };
  const claude = { async use(name) { used.push(name); return capabilities[name]; } };
  await withApp("artifact", { claude, indexedDB: browser.indexedDB }, async ({ $, worker, init, blobs, anchors, urls, scheduled }) => {
    assert.deepEqual(init.records, comparisons);
    assert.equal(db.reads.filter((name) => name === "comparisons").length, 1);
    // Images still attempts its separate cache; artifact answers never use IDB.
    assert.deepEqual(browser.opens, ["tastelab-images"]);
    assert.equal($("status").textContent, "");
    await $("export").fire("click");
    assert.equal($("status").textContent, "Exported.");
    assert.equal(saves.length, 1);
    assert.equal(saves[0].filename, "tastelab-export.json");
    assertExport(JSON.parse(saves[0].data), comparisons, worker);
    assert.deepEqual(used, ["user", "db", "downloads"]);
    assert.equal(blobs.length, 0);
    assert.equal(anchors.length, 0);
    assert.equal(urls.length, 0);
    assert.equal(scheduled.length, 0);
  });
});

test("real app boots standalone browser storage and exports its stored comparisons", async () => {
  const browser = fakeIndexedDB(), db = await createLocalDb(browser.indexedDB);
  for (let i = comparisons.length - 1; i >= 0; i--) await db.doc(`comparisons/${i}`).set(comparisons[i]);
  browser.opens.length = 0;
  await withApp("standalone-storage", { indexedDB: browser.indexedDB }, async (app) => {
    assert.deepEqual(app.init.records, comparisons);
    assert.deepEqual(browser.opens, ["tastelab-answers", "tastelab-images"]);
    assert.equal(app.$("status").textContent, STANDALONE_STATUS);
    await browserExport(app, comparisons);
  });
});

test("TSA-001: real app exports in standalone mode when browser storage throws", async () => {
  const browser = fakeIndexedDB({ unavailable: true });
  await withApp("standalone-no-storage", { indexedDB: browser.indexedDB }, async (app) => {
    assert.deepEqual(app.init.records, []);
    assert.deepEqual(browser.opens, ["tastelab-answers", "tastelab-images"]);
    assert.equal(app.$("status").textContent, TAB_STATUS);
    await browserExport(app, []);
  });
});

// A tie group of 1000 cannot be read completely (tie_overflow).
const tied = Array.from({ length: 1000 }, (_, i) => ({ ...comparisons[i % 2], t: comparisons[0].t }));
const READ_ERROR = "Stored answers could not be read (tie_overflow).";
const EXPORT_REFUSED = "Export is not available: stored answers could not be read.";

async function refusedExport({ $, worker, init, blobs, anchors, urls }) {
  assert.deepEqual(init.records, []);
  assert.equal($("status").textContent, READ_ERROR, "The read error outlives the first pair.");
  await $("export").fire("click");
  assert.equal($("status").textContent, EXPORT_REFUSED);
  assert.equal(worker.messages.filter((message) => message.type === "presets").length, 0);
  assert.equal(blobs.length, 0);
  assert.equal(anchors.length, 0);
  assert.equal(urls.length, 0);
}

test("TSS-001: an unread artifact history keeps its notice and is never exported", async () => {
  const db = artifactDb(tied), browser = fakeIndexedDB(), saves = [];
  const capabilities = { db, user: { async isOwner() { return true; } }, downloads: { async save(body) { saves.push(body); } } };
  const claude = { async use(name) { return capabilities[name]; } };
  await withApp("artifact-unread", { claude, indexedDB: browser.indexedDB }, async (app) => {
    await refusedExport(app);
    assert.equal(saves.length, 0);
  });
});

test("TSS-001: an unread standalone history keeps its notice and is never downloaded", async () => {
  const browser = fakeIndexedDB(), db = await createLocalDb(browser.indexedDB);
  for (const [i, body] of tied.entries()) await db.doc(`comparisons/${String(i).padStart(4, "0")}`).set(body);
  await withApp("standalone-unread", { indexedDB: browser.indexedDB }, refusedExport);
});

test("TSS-001: an export clicked during boot waits for the read and is refused", async () => {
  let release, giveDownloads = null, clicked;
  const gate = new Promise((resolve) => { release = resolve; });
  const db = artifactDb(tied, gate), browser = fakeIndexedDB(), saves = [];
  const capabilities = { db, user: { async isOwner() { return true; } } };
  const downloads = { async save(body) { saves.push(body); } };
  // Downloads answer only when the test lets them, after the failed read.
  const claude = { use(name) {
    if (name !== "downloads") return Promise.resolve(capabilities[name]);
    return new Promise((resolve) => { giveDownloads = () => resolve(downloads); });
  } };
  const early = async ({ $ }) => {
    clicked = $("export").fire("click");
    await tick();
    release();
  };
  await withApp("artifact-early-export", { claude, indexedDB: browser.indexedDB, early }, async ({ $, worker, init }) => {
    if (giveDownloads) giveDownloads();
    await clicked;
    assert.deepEqual(init.records, []);
    assert.equal($("status").textContent, EXPORT_REFUSED);
    assert.equal(worker.messages.filter((message) => message.type === "presets").length, 0);
    assert.equal(saves.length, 0);
  });
});

test("TSS-001: without WebGL 2 the stored answers stay unread and are never exported", async () => {
  const db = artifactDb(), browser = fakeIndexedDB(), saves = [];
  const capabilities = { db, user: { async isOwner() { return true; } }, downloads: { async save(body) { saves.push(body); } } };
  const claude = { async use(name) { return capabilities[name]; } };
  await withApp("artifact-no-webgl", { claude, indexedDB: browser.indexedDB, webgl: false }, async ({ $, worker, init }) => {
    assert.equal(init, undefined);
    assert.deepEqual(db.reads, []);
    await $("export").fire("click");
    assert.equal($("status").textContent, EXPORT_REFUSED);
    assert.equal(worker.messages.filter((message) => message.type === "presets").length, 0);
    assert.equal(saves.length, 0);
  });
});

test("without WebGL 2 or browser storage there is nothing unread and the export still runs", async () => {
  const browser = fakeIndexedDB({ unavailable: true });
  await withApp("standalone-no-webgl", { indexedDB: browser.indexedDB, webgl: false }, async (app) => {
    assert.equal(app.init, undefined);
    await browserExport(app, []);
  });
});

test("real app refuses artifact answer reads for a non-owner", async () => {
  const db = artifactDb(), browser = fakeIndexedDB();
  const claude = { async use(name) { return { db, user: { async isOwner() { return false; } } }[name]; } };
  await withApp("artifact-non-owner", { claude, indexedDB: browser.indexedDB }, async ({ $, init }) => {
    assert.equal($("status").textContent, OWNER_STATUS);
    assert.deepEqual(init.records, []);
    assert.deepEqual(db.reads, []);
    assert.deepEqual(browser.opens, ["tastelab-images"]);
  });
});
