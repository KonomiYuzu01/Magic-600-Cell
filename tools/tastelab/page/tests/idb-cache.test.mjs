import test from "node:test";
import assert from "node:assert/strict";
import { createIdbCache } from "../idb-cache.js";

// In-memory IDB requests: no browser data, filesystem or network.
function fakeIndexedDB({ blocked = false, openError = false, transactionError = false, transactionThrow = false, requestError = false } = {}) {
  const values = new Map(), connections = [], events = [], requests = [];
  let created = false;
  const event = () => {
    const value = { prevented: false, preventDefault() { this.prevented = true; } };
    events.push(value);
    return value;
  };
  const indexedDB = {
    open(name, version) {
      assert.equal(name, "tastelab-images"); assert.equal(version, 1);
      const request = {};
      const connection = {
        closed: 0,
        close() { this.closed++; },
        objectStoreNames: { contains: () => created },
        createObjectStore(name) { assert.equal(name, "bundles"); created = true; },
        transaction(name, mode) {
          assert.equal(name, "bundles"); assert.ok(["readonly", "readwrite"].includes(mode));
          if (transactionThrow) throw new Error("transaction throws");
          const transaction = { error: null, aborted: false, abort() { this.aborted = true; } };
          function operation(fn) {
            const req = {};
            queueMicrotask(() => {
              if (requestError) {
                req.error = new Error("request failed");
                req.onerror(event());
                transaction.onabort(event());
              } else {
                req.result = fn();
                req.onsuccess();
                if (transactionError) {
                  transaction.error = new Error("transaction failed");
                  transaction.onerror(event());
                  transaction.onabort(event());
                } else transaction.oncomplete();
              }
            });
            return req;
          }
          transaction.objectStore = () => ({
            get: (key) => operation(() => structuredClone(values.get(key))),
            put: (value, key) => operation(() => { values.set(key, structuredClone(value)); return key; }),
            delete: (key) => operation(() => values.delete(key)),
            getAllKeys: () => operation(() => [...values.keys()].sort()),
          });
          return transaction;
        },
      };
      connections.push(connection);
      requests.push(request);
      request.result = connection;
      queueMicrotask(() => {
        if (blocked) request.onblocked(event());
        else if (openError) { request.error = new Error("open failed"); request.onerror(event()); }
        else {
          if (!created) request.onupgradeneeded();
          request.onsuccess();
        }
      });
      return request;
    },
  };
  return { indexedDB, values, connections, requests, events };
}

test("one object store puts, gets, lists and deletes cloned bundle values", async () => {
  const fake = fakeIndexedDB(), cache = createIdbCache(fake.indexedDB);
  const value = { manifestText: "{}", files: [["thumb.jpg", Uint8Array.of(1, 2, 3)]] };
  await cache.put("b", value);
  await cache.put("a", { manifestText: "other", files: [] });
  const loaded = await cache.get("b");
  assert.deepEqual(loaded, value);
  loaded.files[0][1][0] = 9;
  assert.deepEqual(await cache.get("b"), value);
  assert.deepEqual(await cache.keys(), ["a", "b"]);
  await cache.delete("b");
  assert.equal(await cache.get("b"), undefined);
  assert.deepEqual(await cache.keys(), ["a"]);
  assert.ok(fake.connections.every((db) => db.closed === 1));
});

test("a throwing open rejects every cache method without a synchronous throw", async () => {
  const cache = createIdbCache({ open() { throw new Error("open throws"); } });
  for (const call of [() => cache.get("a"), () => cache.put("a", {}), () => cache.delete("a"), () => cache.keys()]) {
    let pending;
    assert.doesNotThrow(() => { pending = call(); });
    await assert.rejects(pending, /open throws/);
  }
});

test("a blocked open rejects promptly and closes a later successful connection", async () => {
  const fake = fakeIndexedDB({ blocked: true }), cache = createIdbCache(fake.indexedDB);
  await assert.rejects(cache.keys(), /blocked/);
  fake.requests[0].onsuccess();
  assert.equal(fake.connections[0].closed, 1);
});

test("an open error is handled and returned as a rejection", async () => {
  const fake = fakeIndexedDB({ openError: true });
  await assert.rejects(createIdbCache(fake.indexedDB).get("a"), /open failed/);
  assert.ok(fake.events.every((event) => event.prevented));
});

test("a failing transaction after request success rejects without an uncaught error", async () => {
  const fake = fakeIndexedDB({ transactionError: true });
  await assert.rejects(createIdbCache(fake.indexedDB).put("a", {}), /transaction failed/);
  assert.equal(fake.connections[0].closed, 1);
  assert.ok(fake.events.every((event) => event.prevented));
});

test("a throwing transaction and a request error both reject and close the connection", async () => {
  for (const options of [{ transactionThrow: true }, { requestError: true }]) {
    const fake = fakeIndexedDB(options);
    await assert.rejects(createIdbCache(fake.indexedDB).delete("a"), /transaction throws|request failed/);
    assert.equal(fake.connections[0].closed, 1);
    assert.ok(fake.events.every((event) => event.prevented));
  }
});
