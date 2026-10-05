import { validateBundle, MAX_BUNDLE_FILES, MAX_MANIFEST_BYTES, MAX_SELECTION_BYTES, THUMB_MAX_BYTES } from "./bundle.js";

const fail = (message) => ({ ok: false, errors: [message] });
const plainName = (name) => typeof name === "string" && name.length > 0 && name !== "." && name !== ".."
  && !/[/\\\u0000-\u001f\u007f-\u009f]/u.test(name);

export async function readSelection(fileList) {
  const selected = Array.from(fileList), names = new Set();
  if (selected.length > MAX_BUNDLE_FILES) return fail("A bundle selection may contain at most 301 files.");
  if (selected.some((file) => !Number.isSafeInteger(file.size) || file.size < 0)) return fail("Selected file sizes must be nonnegative integers.");
  if (selected.reduce((total, file) => total + file.size, 0) > MAX_SELECTION_BYTES) return fail("Selected files exceed the total bundle byte bound.");
  let folder = null, flat = null;
  for (const file of selected) {
    if (!plainName(file.name) || names.has(file.name)) return fail("Selected file names must be unique and direct.");
    names.add(file.name);
    // Only the writer's manifest and hash-named JPEGs are valid. In particular,
    // embeddings live inside the bounded manifest, so refuse any extra file
    // (including embeddings.bin) before allocating its bytes.
    if (file.name !== "manifest.json" && !/^[0-9a-f]{64}\.jpg$/.test(file.name)) return fail("Select only manifest.json and its SHA-256-named thumbnails; embeddings belong in the manifest.");
    if (file.size > (file.name === "manifest.json" ? MAX_MANIFEST_BYTES : THUMB_MAX_BYTES)) return fail("Selected manifest or thumbnail exceeds its byte bound.");
    const path = file.webkitRelativePath ?? "", isFlat = path === "";
    if (flat !== null && flat !== isFlat) return fail("Selected files must be directly in one folder.");
    flat = isFlat;
    if (!isFlat) {
      if (typeof path !== "string") return fail("Selected folder path is invalid.");
      const parts = path.split("/");
      if (parts.length !== 2 || !plainName(parts[0]) || parts[1] !== file.name || (folder !== null && folder !== parts[0])) return fail("Selected files must be directly in one folder.");
      folder = parts[0];
    }
  }
  if (!names.has("manifest.json")) return fail("Select manifest.json with its thumbnails.");
  try {
    const files = new Map();
    for (const file of selected) {
      const bytes = new Uint8Array(await file.arrayBuffer());
      if (bytes.byteLength > (file.name === "manifest.json" ? MAX_MANIFEST_BYTES : THUMB_MAX_BYTES)) return fail("Read manifest or thumbnail exceeds its byte bound.");
      files.set(file.name, bytes);
    }
    const manifestText = new TextDecoder("utf-8", { fatal: true }).decode(files.get("manifest.json"));
    return { ok: true, manifestText, files };
  } catch { return fail("Selected files could not be read as a UTF-8 bundle."); }
}

export async function openBundle(selection, cache) {
  if (!selection || selection.ok === false) return selection || fail("No bundle selected.");
  const result = await validateBundle(selection.manifestText, selection.files);
  if (!result.ok) return result;
  let cached = true;
  try { await cache.put(result.bundle.id, { manifestText: selection.manifestText, files: [...selection.files] }); }
  catch { cached = false; }
  return { ok: true, bundle: result.bundle, cached };
}

export async function loadCached(cache) {
  const bundles = [], dropped = [];
  try {
    for (const key of await cache.keys()) {
      const value = await cache.get(key);
      let result = { ok: false };
      if (value && Array.isArray(value.files) && value.files.every((entry) => Array.isArray(entry) && entry.length === 2)) {
        const files = new Map(value.files);
        if (files.size === value.files.length) result = await validateBundle(value.manifestText, files);
      }
      if (result.ok && result.bundle.id === key) bundles.push(result.bundle);
      else { await cache.delete(key); dropped.push(key); }
    }
    return { bundles, dropped, unavailable: false };
  } catch { return { bundles: [], dropped, unavailable: true }; }
}

export async function closeBundle(id, cache) {
  await cache.delete(id);
}

export function mergeBundles(bundles) {
  const order = [], vectors = new Map(), items = new Map();
  for (const bundle of bundles) {
    bundle.items.forEach((item, index) => {
      if (items.has(item.imageId)) return;
      order.push(item.imageId);
      items.set(item.imageId, item);
      vectors.set(item.imageId, bundle.vectors.subarray(index * 512, (index + 1) * 512));
    });
  }
  return { order, vectors, items };
}
