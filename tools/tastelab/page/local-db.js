// Answers stay in this browser, separately from the cached image bundles.
const DATABASE = "tastelab-answers", STORE = "documents";
const copy = (value) => JSON.parse(JSON.stringify(value));
const plain = (value) => value !== null && typeof value === "object"
  && [Object.prototype, null].includes(Object.getPrototypeOf(value));

function validateJson(value, seen = new Set()) {
  if (value === null || typeof value === "string" || typeof value === "boolean") return;
  if (typeof value === "number" && Number.isFinite(value)) return;
  if ((!Array.isArray(value) && !plain(value)) || seen.has(value)) throw new Error("Not a JSON value.");
  seen.add(value);
  for (const entry of Object.values(value)) validateJson(entry, seen);
  seen.delete(value);
}

const comparable = (value) => typeof value === "number" || typeof value === "string";
function compare(a, b) {
  if (typeof a !== typeof b) return typeof a === "number" ? -1 : 1;
  return a < b ? -1 : a > b ? 1 : 0;
}

export async function createLocalDb(indexedDB) {
  const connection = await new Promise((resolve, reject) => {
    let settled = false;
    const fail = (error) => { settled = true; reject(error); };
    const request = indexedDB.open(DATABASE, 1);
    request.onblocked = () => fail(new Error("The browser answer store is blocked."));
    request.onerror = (event) => {
      event.preventDefault();
      fail(request.error || new Error("The browser answer store could not be opened."));
    };
    request.onupgradeneeded = () => {
      try {
        if (settled) request.transaction.abort();
        else if (!request.result.objectStoreNames.contains(STORE)) {
          const store = request.result.createObjectStore(STORE, { keyPath: ["collection", "id"] });
          store.createIndex("collection", "collection");
        }
      } catch (error) { fail(error); }
    };
    request.onsuccess = () => {
      const db = request.result;
      if (settled) { db.close(); return; }
      settled = true;
      db.onversionchange = () => db.close();
      resolve(db);
    };
  });

  function run(mode, operation) {
    return new Promise((resolve, reject) => {
      let transaction, settled = false, result;
      const fail = (error, event) => {
        event?.preventDefault();
        if (settled) return;
        settled = true;
        try { transaction?.abort(); } catch { /* it may already have aborted */ }
        reject(error || new Error("The browser answer transaction failed."));
      };
      try {
        transaction = connection.transaction(STORE, mode);
        transaction.onerror = (event) => fail(transaction.error, event);
        transaction.onabort = (event) => fail(transaction.error, event);
        transaction.oncomplete = () => {
          if (settled) return;
          settled = true;
          resolve(result);
        };
        const request = operation(transaction.objectStore(STORE));
        request.onerror = (event) => fail(request.error, event);
        request.onsuccess = () => { result = request.result; };
      } catch (error) { fail(error); }
    });
  }

  function query(name, filters = [], order = null, count = null) {
    return {
      where(field, op, value) {
        if (!["==", ">", ">="].includes(op)) throw new Error("Unsupported query operator.");
        return query(name, [...filters, { field, op, value }], order, count);
      },
      orderBy(field, direction = "asc") {
        if (!["asc", "desc"].includes(direction)) throw new Error("Invalid query direction.");
        return query(name, filters, { field, direction }, count);
      },
      limit(n) {
        if (!Number.isInteger(n) || n <= 0) throw new Error("Query limit must be a positive integer.");
        return query(name, filters, order, n);
      },
      async get() {
        const stored = await run("readonly", (store) => store.index("collection").getAll(name));
        let records = stored.filter(({ body }) => filters.every(({ field, op, value }) => {
          const actual = body[field];
          if (!comparable(actual) || typeof actual !== typeof value) return false;
          return op === "==" ? actual === value : op === ">" ? actual > value : actual >= value;
        }));
        if (order) records = records.filter(({ body }) => comparable(body[order.field]));
        records.sort((a, b) => {
          const ordered = order ? compare(a.body[order.field], b.body[order.field]) * (order.direction === "desc" ? -1 : 1) : 0;
          return ordered || compare(a.id, b.id);
        });
        if (count !== null) records = records.slice(0, count);
        const docs = records.map(({ id, body }) => ({ id, data: () => copy(body) }));
        return { docs, size: docs.length };
      },
    };
  }

  return {
    doc(path) {
      if (typeof path !== "string" || !/^[^/]+\/[^/]+$/.test(path)) {
        throw Object.assign(new Error("A document path must contain a collection and id."), { code: "invalid_path" });
      }
      const [collection, id] = path.split("/"), key = [collection, id];
      return {
        async get() {
          const record = await run("readonly", (store) => store.get(key));
          return { exists: record !== undefined, id, data: () => record === undefined ? undefined : copy(record.body) };
        },
        async set(body) {
          let stored;
          try {
            if (!plain(body)) throw new Error("Not an object.");
            validateJson(body);
            stored = copy(body);
          } catch {
            throw Object.assign(new Error("A document body must be a plain JSON object."), { code: "invalid_body" });
          }
          await run("readwrite", (store) => store.put({ collection, id, body: stored }));
        },
        async delete() { await run("readwrite", (store) => store.delete(key)); },
      };
    },
    collection(name) { return query(name); },
  };
}
