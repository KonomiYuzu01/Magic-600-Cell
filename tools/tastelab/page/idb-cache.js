// Bundles stay in this browser. Every operation rejects asynchronously when
// IndexedDB is missing, blocked or fails; the library can still open a folder.
const DATABASE = "tastelab-images", STORE = "bundles";

export function createIdbCache(indexedDB) {
  function open() {
    return new Promise((resolve, reject) => {
      let settled = false;
      const fail = (error) => { settled = true; reject(error); };
      const request = (indexedDB ?? globalThis.indexedDB).open(DATABASE, 1);
      request.onblocked = () => fail(new Error("The browser cache is blocked."));
      request.onerror = (event) => {
        event.preventDefault();
        fail(request.error || new Error("The browser cache could not be opened."));
      };
      request.onupgradeneeded = () => {
        try {
          if (settled) request.transaction.abort();
          else if (!request.result.objectStoreNames.contains(STORE)) request.result.createObjectStore(STORE);
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
  }

  async function run(mode, operation) {
    const db = await open();
    return new Promise((resolve, reject) => {
      let transaction, settled = false, result;
      const fail = (error, event) => {
        event?.preventDefault();
        if (settled) return;
        settled = true;
        try { transaction?.abort(); } catch { /* it may already have aborted */ }
        db.close();
        reject(error || new Error("The browser cache transaction failed."));
      };
      try {
        transaction = db.transaction(STORE, mode);
        transaction.onerror = (event) => fail(transaction.error, event);
        transaction.onabort = (event) => fail(transaction.error, event);
        transaction.oncomplete = () => {
          if (settled) return;
          settled = true;
          db.close();
          resolve(result);
        };
        const request = operation(transaction.objectStore(STORE));
        request.onerror = (event) => fail(request.error, event);
        request.onsuccess = () => { result = request.result; };
      } catch (error) { fail(error); }
    });
  }

  return {
    async get(key) { return run("readonly", (store) => store.get(key)); },
    async put(key, value) { return run("readwrite", (store) => store.put(value, key)); },
    async delete(key) { return run("readwrite", (store) => store.delete(key)); },
    async keys() { return run("readonly", (store) => store.getAllKeys()); },
  };
}
