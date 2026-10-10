import test from "node:test";
import assert from "node:assert/strict";
import { chooseStorage, saveFile } from "../platform.js";

function browserDownload() {
  const blobs = [], anchors = [], urls = [], revoked = [], scheduled = [], events = [];
  class Blob {
    constructor(parts, options) { this.parts = parts; this.type = options.type; blobs.push(this); }
  }
  const document = {
    body: { appendChild(anchor) { events.push("append"); anchor.attached = true; } },
    createElement(tag) {
      assert.equal(tag, "a");
      const anchor = {
        click() { assert.equal(this.attached, true); events.push("click"); },
        remove() { this.attached = false; events.push("remove"); },
      };
      anchors.push(anchor);
      return anchor;
    },
  };
  const URL = {
    createObjectURL(blob) { assert.equal(blob, blobs.at(-1)); urls.push("blob:export"); return urls.at(-1); },
    revokeObjectURL(url) { revoked.push(url); },
  };
  const schedule = (callback, delay) => { scheduled.push({ callback, delay }); };
  return { document, URL, Blob, schedule, blobs, anchors, urls, revoked, scheduled, events };
}

test("artifact storage selection never opens local storage even without sign-in capabilities", async () => {
  for (const claude of [{}, { use: () => null }]) {
    let opened = 0;
    assert.deepEqual(await chooseStorage({ claude, openLocal() { opened++; throw new Error("must not open"); } }),
      { mode: "artifact", standalone: false });
    assert.equal(opened, 0);
  }
});

test("standalone selection opens local storage once and returns its database", async () => {
  const db = {}, calls = [];
  assert.deepEqual(await chooseStorage({ claude: undefined, async openLocal() { calls.push("open"); return db; } }),
    { mode: "standalone", standalone: true, db });
  assert.deepEqual(calls, ["open"]);
});

for (const failure of ["reject", "throw"]) test(`standalone stays standalone when local storage ${failure}s`, async () => {
  const openLocal = () => {
    if (failure === "throw") throw new Error("unavailable");
    return Promise.reject(new Error("unavailable"));
  };
  assert.deepEqual(await chooseStorage({ claude: null, openLocal }), { mode: "none", standalone: true });
});

test("saveFile uses the downloads capability exactly once, including in standalone mode", async () => {
  for (const standalone of [false, true]) {
    const browser = browserDownload(), calls = [];
    await saveFile({ ...browser, standalone, downloads: { async save(body) { calls.push(body); } } }, "answers.json", "data");
    assert.deepEqual(calls, [{ filename: "answers.json", data: "data" }]);
    assert.equal(browser.blobs.length, 0);
    assert.equal(browser.anchors.length, 0);
  }
});

test("downloads rejection propagates without starting a browser download", async () => {
  const browser = browserDownload(), declined = Object.assign(new Error("declined"), { code: "declined" });
  await assert.rejects(saveFile({ ...browser, standalone: true, downloads: { async save() { throw declined; } } }, "answers.json", "data"),
    (error) => error === declined);
  assert.equal(browser.blobs.length, 0);
  assert.equal(browser.anchors.length, 0);
});

test("standalone browser download creates a JSON Blob, clicks and removes a hidden anchor, then revokes later", async () => {
  const browser = browserDownload(), data = '{"answer":"A"}';
  await saveFile({ ...browser, standalone: true, downloads: null }, "tastelab-export.json", data);
  assert.equal(browser.blobs.length, 1);
  assert.deepEqual(browser.blobs[0].parts, [data]);
  assert.equal(browser.blobs[0].type, "application/json");
  assert.equal(browser.anchors.length, 1);
  const anchor = browser.anchors[0];
  assert.equal(anchor.href, "blob:export");
  assert.equal(anchor.download, "tastelab-export.json");
  assert.equal(anchor.hidden, true);
  assert.equal(anchor.attached, false);
  assert.deepEqual(browser.events, ["append", "click", "remove"]);
  assert.equal(browser.scheduled.length, 1);
  assert.equal(browser.scheduled[0].delay, 60000);
  assert.deepEqual(browser.revoked, []);
  browser.scheduled[0].callback();
  assert.deepEqual(browser.revoked, ["blob:export"]);
});

test("artifact export without a downloads capability rejects without a Blob or anchor", async () => {
  const browser = browserDownload();
  await assert.rejects(saveFile({ ...browser, standalone: false, downloads: null }, "answers.json", "data"), { code: "unavailable" });
  assert.equal(browser.blobs.length, 0);
  assert.equal(browser.anchors.length, 0);
  assert.equal(browser.scheduled.length, 0);
});

test("TSA-001: unavailable standalone storage still allows browser export", async () => {
  const browser = browserDownload();
  const storage = await chooseStorage({ openLocal: () => Promise.reject(new Error("storage disabled")) });
  await saveFile({ ...browser, standalone: storage.standalone, downloads: null }, "answers.json", "data");
  assert.equal(storage.mode, "none");
  assert.deepEqual(browser.events, ["append", "click", "remove"]);
});

test("TSA-001: an artifact without downloads never opens local storage or starts a browser export", async () => {
  const browser = browserDownload();
  let opened = 0;
  const storage = await chooseStorage({ claude: {}, openLocal() { opened++; return {}; } });
  await assert.rejects(saveFile({ ...browser, standalone: storage.standalone, downloads: null }, "answers.json", "data"), { code: "unavailable" });
  assert.equal(opened, 0);
  assert.equal(browser.blobs.length, 0);
  assert.equal(browser.anchors.length, 0);
});
