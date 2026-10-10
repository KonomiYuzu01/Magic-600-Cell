import test from "node:test";
import assert from "node:assert/strict";
import { createLocalDb } from "../local-db.js";
import { loadImageDocuments } from "../images-ui.js";

// In-memory IDB requests, with writes committed only on transaction complete.
// Both adapters share the same store; no browser data, filesystem or network.
function fakeIndexedDB({ openFailure = null } = {}) {
  const values = new Map(), connections = [], requests = [], transactions = [], events = [];
  let created = false, storeName, keyPath, nextOutcome = null;
  const indexes = new Map();
  const event = () => {
    const value = { prevented: false, preventDefault() { this.prevented = true; } };
    events.push(value);
    return value;
  };
  const keyFor = (body) => keyPath.map((field) => body[field]);
  const indexedDB = {
    open(name, version) {
      assert.equal(name, "tastelab-answers");
      assert.equal(version, 1);
      if (openFailure === "throw") throw new Error("open throws");
      const request = {};
      const connection = {
        closed: 0,
        close() { this.closed++; },
        objectStoreNames: { contains: (name) => created && name === storeName },
        createObjectStore(name, options) {
          assert.equal(created, false, "there is only one object store");
          created = true;
          storeName = name;
          keyPath = options.keyPath;
          return { createIndex(name, field) { indexes.set(name, field); } };
        },
        transaction(name, mode) {
          assert.equal(name, storeName);
          assert.ok(["readonly", "readwrite"].includes(mode));
          const outcome = nextOutcome;
          nextOutcome = null;
          if (outcome === "throw") throw new Error("transaction throws");
          let commit = () => {};
          const transaction = { error: null, aborted: false, requests: 0,
            abort() { this.aborted = true; } };
          transactions.push(transaction);
          function operation(read, write = () => {}) {
            const req = {};
            transaction.requests++;
            commit = write;
            queueMicrotask(() => {
              if (outcome === "requestError") {
                req.error = new Error("request failed");
                req.onerror(event());
                transaction.onabort(event());
              } else {
                req.result = read();
                req.onsuccess();
                // Keep request success separate from transaction completion.
                queueMicrotask(() => {
                  if (outcome === "abort" || transaction.aborted) {
                    transaction.error = new Error("transaction aborted");
                    transaction.onabort(event());
                  } else if (outcome === "error") {
                    transaction.error = new Error("transaction failed");
                    transaction.onerror(event());
                    transaction.onabort(event());
                  } else {
                    commit();
                    transaction.oncomplete();
                  }
                });
              }
            });
            return req;
          }
          transaction.objectStore = (name) => {
            assert.equal(name, storeName);
            return {
              get: (key) => operation(() => structuredClone(values.get(JSON.stringify(key)))),
              put(body) {
                assert.equal(mode, "readwrite");
                const stored = structuredClone(body), key = keyFor(stored);
                return operation(() => key, () => values.set(JSON.stringify(key), stored));
              },
              delete(key) {
                assert.equal(mode, "readwrite");
                return operation(() => undefined, () => values.delete(JSON.stringify(key)));
              },
              index(name) {
                assert.ok(indexes.has(name));
                return { getAll(key) {
                  return operation(() => structuredClone([...values.values()].filter((value) => value[indexes.get(name)] === key)));
                } };
              },
            };
          };
          return transaction;
        },
      };
      request.result = connection;
      connections.push(connection);
      requests.push(request);
      queueMicrotask(() => {
        if (openFailure === "blocked") request.onblocked(event());
        else if (openFailure === "error") { request.error = new Error("open failed"); request.onerror(event()); }
        else {
          if (!created) request.onupgradeneeded();
          request.onsuccess();
        }
      });
      return request;
    },
  };
  return { indexedDB, values, connections, requests, transactions, events,
    failNext(outcome) { nextOutcome = outcome; } };
}

const ids = async (query) => (await query.get()).docs.map((doc) => doc.id);
const numbered = (n) => String(n).padStart(4, "0");

test("local documents round trip across adapters with isolated collections and fresh copies", async () => {
  const fake = fakeIndexedDB(), db = await createLocalDb(fake.indexedDB), other = await createLocalDb(fake.indexedDB);
  const doc = db.doc("comparisons/a"), body = { answer: "A", nested: { values: [1, null, true] } };
  const expected = structuredClone(body), pending = doc.set(body);
  body.nested.values[0] = 9;
  await pending;
  const snap = await other.doc("comparisons/a").get();
  assert.equal(snap.exists, true);
  assert.equal(snap.id, "a");
  assert.deepEqual(snap.data(), expected);
  snap.data().nested.values[0] = 8;
  assert.deepEqual(snap.data(), expected);
  assert.deepEqual((await doc.get()).data(), expected);
  await db.doc("sessions/a").set({ started: "now" });
  await doc.set({ replacement: true });
  assert.deepEqual((await other.doc("comparisons/a").get()).data(), { replacement: true });
  await doc.delete();
  await doc.delete();
  const missing = await doc.get();
  assert.equal(missing.exists, false);
  assert.equal(missing.id, "a");
  assert.equal(missing.data(), undefined);
  assert.deepEqual((await other.doc("sessions/a").get()).data(), { started: "now" });
});

test("invalid document paths throw synchronously without a transaction", async () => {
  const fake = fakeIndexedDB(), db = await createLocalDb(fake.indexedDB);
  for (const path of [null, undefined, 1, {}, "", "comparisons", "/a", "comparisons/", "comparisons/a/b", "comparisons//a", "comparisons/a/"]) {
    assert.throws(() => db.doc(path), { code: "invalid_path" });
  }
  assert.equal(fake.transactions.length, 0);
});

test("non-object and non-JSON bodies reject before a transaction and preserve stored data", async () => {
  const fake = fakeIndexedDB(), db = await createLocalDb(fake.indexedDB), doc = db.doc("comparisons/a");
  await doc.set({ original: true });
  const circular = {}; circular.self = circular;
  const invalid = [null, undefined, [], 1, true, "text", () => {}, new Date(), new Map(),
    { value: undefined }, { value: () => {} }, { value: Symbol("x") }, { value: 1n },
    { value: NaN }, { value: Infinity }, circular];
  const count = fake.transactions.length;
  for (const body of invalid) {
    let pending;
    assert.doesNotThrow(() => { pending = doc.set(body); });
    await assert.rejects(pending, { code: "invalid_body" });
  }
  assert.equal(fake.transactions.length, count);
  assert.deepEqual((await doc.get()).data(), { original: true });
});

async function queryFixture() {
  const fake = fakeIndexedDB(), db = await createLocalDb(fake.indexedDB);
  for (const [id, body] of [
    ["b", { score: 2, group: "same", label: "a" }], ["a", { score: 2, group: "same", label: "Z" }],
    ["c", { score: 10, group: "other", label: "z" }], ["d", { score: 1, group: "same", label: "ä" }],
    ["e", { score: "2", group: "same" }], ["f", { group: "same" }], ["g", { score: null }],
    ["h", { score: false }], ["i", { score: {} }], ["j", { score: [] }],
  ]) await db.doc(`ratings/${id}`).set(body);
  await db.doc("other/a").set({ score: 100 });
  return { fake, db, collection: db.collection("ratings") };
}

test("where supports equality, greater-than, inclusive bounds, AND and matching types", async () => {
  const { collection } = await queryFixture();
  assert.deepEqual(await ids(collection.where("score", "==", 2)), ["a", "b"]);
  assert.deepEqual(await ids(collection.where("score", ">", 2)), ["c"]);
  assert.deepEqual(await ids(collection.where("score", ">=", 2)), ["a", "b", "c"]);
  assert.deepEqual(await ids(collection.where("score", "==", "2")), ["e"]);
  assert.deepEqual(await ids(collection.where("score", ">", "1")), ["e"]);
  assert.deepEqual(await ids(collection.where("score", ">=", "2")), ["e"]);
  assert.deepEqual(await ids(collection.where("label", ">", "Z")), ["b", "c", "d"]);
  assert.deepEqual(await ids(collection.where("score", ">=", 2).where("group", "==", "same")), ["a", "b"]);
  assert.deepEqual(await ids(collection.where("missing", "==", 1)), []);
  assert.deepEqual(await ids(collection.where("score", "==", false)), []);
});

test("queries order numbers and strings, tie by id, exclude missing order fields and limit last", async () => {
  const { fake, collection } = await queryFixture();
  const numeric = collection.where("score", ">=", 1);
  assert.deepEqual(await ids(numeric.orderBy("score")), ["d", "a", "b", "c"]);
  assert.deepEqual(await ids(numeric.orderBy("score", "desc")), ["c", "a", "b", "d"]);
  assert.deepEqual(await ids(collection.orderBy("label")), ["a", "b", "c", "d"]);
  assert.deepEqual(await ids(collection.orderBy("label", "desc")), ["d", "c", "b", "a"]);
  assert.deepEqual(new Set(await ids(collection.orderBy("score"))), new Set(["a", "b", "c", "d", "e"]));
  assert.deepEqual(await ids(collection.limit(1).where("score", ">", 2)), ["c"]);
  assert.deepEqual(await ids(collection.orderBy("score").limit(2).where("score", ">=", 2)), ["a", "b"]);
  const snap = await collection.orderBy("label").limit(2).get();
  assert.equal(snap.size, snap.docs.length);
  assert.equal(snap.size, 2);
  snap.docs[0].data().label = "changed";
  assert.equal(snap.docs[0].data().label, "Z");
  assert.ok(fake.transactions.every((transaction) => transaction.requests === 1), "one request per collection read");
  for (const limit of [0, -1, 1.5, NaN, Infinity, "2"]) assert.throws(() => collection.limit(limit));
});

test("immutable queries chain both before and after orderBy and limit as Images paging does", async () => {
  const { collection } = await queryFixture();
  const filtered = collection.where("group", "==", "same");
  const ordered = filtered.orderBy("score"), limited = ordered.limit(1);
  const paged = limited.where("score", ">", 1);
  assert.notEqual(filtered, collection);
  assert.notEqual(ordered, filtered);
  assert.notEqual(limited, ordered);
  assert.notEqual(paged, limited);
  assert.deepEqual(await ids(paged), ["a"]);
  assert.deepEqual(await ids(collection.orderBy("score").limit(1).where("group", "==", "same").where("score", ">", 1)), ["a"]);
  assert.equal((await filtered.get()).size, 5);
  assert.equal((await collection.get()).size, 10);
  assert.deepEqual(await ids(limited), ["d"]);
});

test("Images loads all 1200 local rating documents once each", async () => {
  const fake = fakeIndexedDB(), db = await createLocalDb(fake.indexedDB), expected = [];
  for (let n = 0; n < 1200; n++) {
    const body = { imageId: numbered(n), verdict: "like", love: false, note: null, ratedAt: "same-time" };
    expected.push(body);
    await db.doc(`imageRatings/${body.imageId}`).set(body);
  }
  const loaded = await loadImageDocuments(db, "imageRatings");
  assert.deepEqual(loaded, expected);
  assert.equal(new Set(loaded.map((record) => record.imageId)).size, 1200);
});

test("Images loads more than 500 local pairs for one liked image without duplicates or gaps", async () => {
  const fake = fakeIndexedDB(), db = await createLocalDb(fake.indexedDB), expected = [];
  for (const [likedImageId, count] of [["a", 2], ["b", 1200], ["c", 2]]) for (let n = 0; n < count; n++) {
    const body = { likedImageId, dislikedImageId: numbered(n), note: "Different", notedAt: "same-time" };
    expected.push(body);
    await db.doc(`imagePairs/${likedImageId}_${body.dislikedImageId}`).set(body);
  }
  const loaded = await loadImageDocuments(db, "imagePairs");
  const key = (record) => `${record.likedImageId}_${record.dislikedImageId}`;
  assert.equal(loaded.length, expected.length);
  assert.equal(new Set(loaded.map(key)).size, expected.length);
  assert.deepEqual([...loaded].sort((a, b) => key(a) < key(b) ? -1 : 1), expected);
});

for (const operation of ["set", "delete"]) for (const outcome of ["abort", "error", "requestError"]) {
  test(`${operation} rejects on ${outcome}, including transaction failure after request success, without changing shared data`, async () => {
    const fake = fakeIndexedDB(), db = await createLocalDb(fake.indexedDB), other = await createLocalDb(fake.indexedDB);
    await db.doc("comparisons/a").set({ original: true });
    fake.failNext(outcome);
    await assert.rejects(db.doc("comparisons/a")[operation]({ replacement: true }), /transaction aborted|transaction failed|request failed/);
    assert.deepEqual((await other.doc("comparisons/a").get()).data(), { original: true });
    assert.ok(fake.events.every((event) => event.prevented));
  });
}

test("reads wait for transaction completion and reject transaction failures after request success", async () => {
  const fake = fakeIndexedDB(), db = await createLocalDb(fake.indexedDB);
  await db.doc("comparisons/a").set({ t: "now" });
  for (const outcome of ["abort", "error", "requestError", "throw"]) for (const read of [
    () => db.doc("comparisons/a").get(), () => db.collection("comparisons").get(),
  ]) {
    fake.failNext(outcome);
    await assert.rejects(read(), /transaction aborted|transaction failed|request failed|transaction throws/);
  }
});

for (const failure of ["error", "blocked", "throw"]) test(`local database open ${failure} rejects`, async () => {
  const fake = fakeIndexedDB({ openFailure: failure });
  let pending;
  assert.doesNotThrow(() => { pending = createLocalDb(fake.indexedDB); });
  await assert.rejects(pending, /open failed|blocked|open throws/);
  if (failure === "error") assert.ok(fake.events.every((event) => event.prevented));
  if (failure === "blocked") {
    fake.requests[0].onsuccess();
    assert.equal(fake.connections[0].closed, 1);
  }
});

test("documented Looks loadRecords limitation: 500 timestamp ties stop before the 501st comparison", async () => {
  const fake = fakeIndexedDB(), db = await createLocalDb(fake.indexedDB);
  for (let n = 0; n < 501; n++) await db.doc(`comparisons/${numbered(n)}`).set({ t: n < 500 ? "same-time" : "z-later" });
  // Copy of page/app.js loadRecords (base lines 378-397). Importing app.js
  // starts DOM and worker code, so its loop stays here without changing the page.
  const PAGE = 500, records = [], ids = [], seen = new Set();
  let last = null;
  for (;;) {
    let query = db.collection("comparisons").orderBy("t").limit(PAGE);
    if (last !== null) query = query.where("t", ">=", last);
    const snap = await query.get();
    let added = 0;
    for (const d of snap.docs) {
      if (seen.has(d.id)) continue;
      seen.add(d.id);
      records.push(d.data());
      ids.push(d.id);
      added++;
    }
    if (snap.size < PAGE || !added) break;
    last = snap.docs[snap.docs.length - 1].data().t;
  }
  assert.equal((await db.collection("comparisons").get()).size, 501);
  assert.equal(records.length, 500);
  assert.equal(ids.length, 500);
});
