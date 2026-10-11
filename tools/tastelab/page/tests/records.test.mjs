import test from "node:test";
import assert from "node:assert/strict";
import { createLocalDb } from "../local-db.js";
import { loadRecords, PAGE, TIE_LIMIT } from "../records.js";

// Only the requests used by createLocalDb; all data stays in memory.
function fakeIndexedDB() {
  const values = new Map();
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
  return { open(name, version) {
    assert.equal(name, "tastelab-answers");
    assert.equal(version, 1);
    const request = { result: connection };
    queueMicrotask(() => {
      if (!created) request.onupgradeneeded();
      request.onsuccess();
    });
    return request;
  } };
}

const compare = (a, b) => a < b ? -1 : a > b ? 1 : 0;
const ordered = (rows) => [...rows].sort((a, b) => compare(a.body.t, b.body.t) || compare(a.id, b.id));
const numbered = (n) => String(n).padStart(4, "0");
const documents = (times) => times.map((t, i) => ({ id: "answer-" + numbered(times.length - i), body: { t, answer: "A", fixture: i } }));
const distinct = (count, start = 0) => Array.from({ length: count }, (_, i) => numbered(start + i));

async function localDb(rows) {
  const db = await createLocalDb(fakeIndexedDB());
  for (const { id, body } of rows) await db.doc(`comparisons/${id}`).set(body);
  return db;
}

// Artifact queries cap explicit limits at 1000 and always break ties by id.
function artifactDb(rows) {
  function query(field = null, filters = [], count = Infinity) {
    return {
      orderBy(next) { return query(next, filters, count); },
      where(key, op, value) {
        assert.ok(["==", ">"].includes(op));
        return query(field, [...filters, [key, op, value]], count);
      },
      limit(next) { return query(field, filters, Math.min(next, 1000)); },
      async get() {
        const selected = rows.filter(({ body }) => filters.every(([key, op, value]) =>
          op === "==" ? body[key] === value : body[key] > value))
          .sort((a, b) => (field ? compare(a.body[field], b.body[field]) : 0) || compare(a.id, b.id)).slice(0, count);
        const docs = selected.map(({ id, body }) => ({ id, data: () => structuredClone(body) }));
        return { docs, size: docs.length };
      },
    };
  }
  return { collection(name) { assert.equal(name, "comparisons"); return query(); } };
}

for (const [name, makeDb] of [["local", localDb], ["artifact", artifactDb]]) {
  for (const [label, times] of [
    ["501 comparisons with 500 timestamp ties and one later answer", [...Array(500).fill("same-time"), "z-later"]],
    ["a 600-answer tie across a middle page boundary", [...distinct(300), ...Array(600).fill(numbered(300)), ...distinct(100, 301)]],
    ["exactly 500 distinct timestamps", distinct(500)],
    ["exactly 500 tied timestamps", Array(500).fill("same-time")],
    ["no comparisons", []],
  ]) test(`${name} Looks loader returns ${label} once in time and id order`, async () => {
    assert.equal(PAGE, 500);
    assert.equal(TIE_LIMIT, 1000);
    const rows = documents(times), db = await makeDb(rows), expected = ordered(rows);
    const { records, ids } = await loadRecords(db);
    assert.equal(records.length, rows.length);
    assert.equal(new Set(ids).size, rows.length);
    assert.deepEqual(records, expected.map(({ body }) => body));
    assert.deepEqual(ids, expected.map(({ id }) => id));
  });

  test(`${name} Looks loader rejects a potentially incomplete 1000-answer tie`, async () => {
    const db = await makeDb(documents(Array(1000).fill("same-time")));
    await assert.rejects(loadRecords(db), { code: "tie_overflow" });
  });

  test(`${name} Looks loader matches an unpaged ordered read across several full pages`, async () => {
    const db = await makeDb(documents(distinct(1603)));
    const unpaged = await db.collection("comparisons").orderBy("t").get();
    assert.equal(unpaged.size, 1603);
    const loaded = await loadRecords(db);
    assert.deepEqual(loaded.records, unpaged.docs.map((doc) => doc.data()));
    assert.deepEqual(loaded.ids, unpaged.docs.map((doc) => doc.id));
    assert.equal(new Set(loaded.ids).size, 1603);
  });
}
