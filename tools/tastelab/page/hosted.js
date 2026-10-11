import { MAX_MANIFEST_BYTES, MAX_ITEMS, THUMB_MAX_BYTES, validateBundle } from "./bundle.js";

export const INDEX_MAX_BYTES = 65536, MAX_HOSTED = 100, THUMB_CONCURRENCY = 6;
export const REQUEST_TIMEOUT_MS = 30000, HOSTED_MAX_BYTES = 512 * 1024 * 1024;
export const REQUEST = { cache: "no-cache", credentials: "same-origin", redirect: "error" };

const failure = (message) => Object.assign(new Error(message), { hosted: true });
const idOk = (id) => typeof id === "string" && id.length === 25 && /^[0-9]{8}T[0-9]{6}Z-[0-9a-f]{8}$/.test(id);
const hashOk = (hash) => typeof hash === "string" && hash.length === 64 && /^[0-9a-f]{64}$/.test(hash);
function decode(bytes) {
  try { return new TextDecoder("utf-8", { fatal: true }).decode(bytes); }
  catch { throw failure("The file is not valid UTF-8."); }
}
function indexIds(bytes) {
  let index;
  try { index = JSON.parse(decode(bytes)); }
  catch { throw failure("The bundle index is not valid UTF-8 JSON."); }
  if (!index || Object.keys(index).length !== 3 || !["format", "version", "bundles"].every((key) => Object.hasOwn(index, key))
      || index.format !== "tastelab-bundle-index" || index.version !== 1 || !Array.isArray(index.bundles)
      || index.bundles.length > MAX_HOSTED || index.bundles.some((id, i, ids) => !idOk(id) || (i > 0 && ids[i - 1] >= id))) {
    throw failure("The bundle index is invalid.");
  }
  return index.bundles;
}
function manifestItems(text, id) {
  let manifest;
  try { manifest = JSON.parse(text); }
  catch { throw failure("The manifest is not valid JSON."); }
  if (!manifest || manifest.id !== id) throw failure("The manifest id does not match the index.");
  if (!Array.isArray(manifest.items) || !manifest.items.length || manifest.items.length > MAX_ITEMS) {
    throw failure("The manifest item count is invalid.");
  }
  const hashes = new Set();
  for (const item of manifest.items) {
    if (!item || !hashOk(item.thumbSha256) || hashes.has(item.thumbSha256)
        || !Number.isInteger(item.bytes) || item.bytes < 1 || item.bytes > THUMB_MAX_BYTES) {
      throw failure("Thumbnail hashes or byte counts are invalid.");
    }
    hashes.add(item.thumbSha256);
  }
  return manifest.items;
}

export async function loadHostedBundles({
  fetch, base, knownIds = new Set(), onBundle = () => {}, onProgress = () => {},
  timeoutMs = REQUEST_TIMEOUT_MS, maxBytes = HOSTED_MAX_BYTES,
}) {
  const root = new URL(base);
  if (!["http:", "https:"].includes(root.protocol)) return { status: "absent" };
  const urlFor = (name) => new URL("bundles/" + name, root).href;

  // A controller stays registered until the headers and streaming read settle.
  async function request(url, bound, controllers) {
    const controller = new AbortController(), { signal } = controller;
    controllers?.add(controller);
    let reader, response;
    const abortRead = () => { if (reader) void reader.cancel().catch(() => {}); };
    signal.addEventListener("abort", abortRead);
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
      response = await fetch(url, { ...REQUEST, signal });
      signal.throwIfAborted();
      if (response.redirected || (response.url && response.url !== url)) throw failure("The response was redirected.");
      if (response.status < 200 || response.status >= 300) {
        await response.body?.cancel();
        return { status: response.status, bytes: new Uint8Array() };
      }
      const length = response.headers.get("Content-Length");
      if (length !== null && Number(length) > bound) throw failure("The file exceeds its byte limit.");
      const chunks = [];
      let size = 0;
      if (response.body) {
        reader = response.body.getReader();
        for (;;) {
          const { done, value } = await reader.read();
          signal.throwIfAborted();
          if (done) break;
          size += value.byteLength;
          if (size > bound) throw failure("The file exceeds its byte limit.");
          chunks.push(value);
        }
      }
      const bytes = new Uint8Array(size);
      let offset = 0;
      for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
      return { status: response.status, bytes };
    } catch (error) {
      if (reader) await reader.cancel().catch(() => {});
      else if (response?.body) await response.body.cancel().catch(() => {});
      if (error?.hosted) throw error;
      throw Object.assign(failure("The request could not be completed."), { unreachable: true });
    } finally {
      clearTimeout(timer);
      signal.removeEventListener("abort", abortRead);
      reader?.releaseLock();
      controllers?.delete(controller);
    }
  }

  let ids;
  try {
    const response = await request(urlFor("index.json"), INDEX_MAX_BYTES);
    if (response.status === 404) return { status: "absent" };
    if (response.status < 200 || response.status >= 300) throw failure("The bundle index could not be read.");
    ids = indexIds(response.bytes);
  } catch (error) {
    return error.unreachable ? { status: "unreachable" } : { status: "unavailable", error: error.message };
  }

  const skipped = ids.filter((id) => knownIds.has(id)), pending = ids.filter((id) => !knownIds.has(id));
  const result = { status: "loaded", imported: [], refused: [], skipped, stopped: null, notAttempted: [] };
  let used = 0, done = 0;
  onProgress({ done, total: pending.length });
  for (const [position, id] of pending.entries()) {
    try {
      const response = await request(urlFor(id + "/manifest.json"), MAX_MANIFEST_BYTES);
      if (response.status < 200 || response.status >= 300) throw failure("The manifest could not be read.");
      const manifestText = decode(response.bytes), items = manifestItems(manifestText, id);
      const size = response.bytes.length + items.reduce((total, item) => total + item.bytes, 0);
      if (used + size > maxBytes) return { ...result, stopped: "budget", notAttempted: pending.slice(position) };
      used += size;
      const files = new Map([["manifest.json", response.bytes]]), controllers = new Set();
      let next = 0, failed = null;
      async function thumbnails() {
        while (!failed && next < items.length) {
          const item = items[next++], name = item.thumbSha256 + ".jpg";
          try {
            const thumbnail = await request(urlFor(id + "/" + name), item.bytes, controllers);
            if (thumbnail.status < 200 || thumbnail.status >= 300) throw failure("A thumbnail could not be read.");
            if (thumbnail.bytes.length !== item.bytes) throw failure("A thumbnail has the wrong byte count.");
            files.set(name, thumbnail.bytes);
          } catch (error) {
            if (!failed) {
              failed = error;
              for (const controller of controllers) controller.abort();
            }
          }
        }
      }
      await Promise.all(Array.from({ length: Math.min(THUMB_CONCURRENCY, items.length) }, thumbnails));
      if (failed) throw failed;
      const validated = await validateBundle(manifestText, files);
      if (!validated.ok) throw failure("The bundle failed validation.");
      // The caller caches the bundle, after deciding that its id is still the site's.
      onBundle({ bundle: validated.bundle, manifestText }, files);
      result.imported.push(id);
    } catch (error) {
      result.refused.push({ id, error: error?.hosted ? error.message : "The bundle could not be opened." });
    }
    onProgress({ done: ++done, total: pending.length });
  }
  return result;
}
