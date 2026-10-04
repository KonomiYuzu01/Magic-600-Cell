import { validateBundle } from "./bundle.js";

const fail = (message) => ({ ok: false, errors: [message] });
const plainName = (name) => typeof name === "string" && name.length > 0 && name !== "." && name !== ".."
  && !/[/\\\u0000-\u001f\u007f-\u009f]/u.test(name);

export async function readSelection(fileList) {
  const selected = Array.from(fileList), names = new Set();
  let folder = null, flat = null;
  for (const file of selected) {
    if (!plainName(file.name) || names.has(file.name)) return fail("Selected file names must be unique and direct.");
    names.add(file.name);
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
    for (const file of selected) files.set(file.name, new Uint8Array(await file.arrayBuffer()));
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
