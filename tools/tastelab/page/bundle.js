import { licenceOk, pageUrlOk } from "./licences.js";

// bundle.py writes at most 300 items, with JPEG thumbnails of at most 200 KiB.
export const MAX_ITEMS = 300, THUMB_MAX_BYTES = 204800;
// The writer embeds 512 float16 values per item in manifest.json: at most
// 307200 raw bytes, or 409600 base64 bytes. No separate embeddings file is valid.
export const MAX_EMBEDDING_BYTES = MAX_ITEMS * 512 * 2;
// 8 MiB allows the embedded base64 plus all 300 items' bounded text and URLs,
// including the writer's indented JSON and escaped Unicode (up to 12 bytes per
// non-BMP character), with room for keys and metadata. It bounds JSON parsing.
export const MAX_MANIFEST_BYTES = 8 * 1024 * 1024;
// One manifest and one thumbnail per item; embeddings already occupy the
// manifest allowance, so the total bound is 69828608 bytes, without double counting.
export const MAX_BUNDLE_FILES = MAX_ITEMS + 1;
export const MAX_SELECTION_BYTES = MAX_MANIFEST_BYTES + MAX_ITEMS * THUMB_MAX_BYTES;

const ROOT_KEYS = ["format", "version", "id", "createdAt", "model", "items", "embeddings"];
const ITEM_KEYS = ["imageId", "thumbSha256", "width", "height", "bytes", "source", "sourceId", "title", "credit", "pageUrl", "licence", "licenceUrl"];
const fail = (message) => ({ ok: false, errors: [message] });
const exactKeys = (value, keys) => value !== null && typeof value === "object" && !Array.isArray(value)
  && Object.keys(value).length === keys.length && keys.every((key) => Object.hasOwn(value, key));
const text = (value, min, max) => typeof value === "string" && value.length >= min && value.length <= max
  && !/[\u0000-\u001f\u007f-\u009f]/u.test(value);
const hash = (value) => typeof value === "string" && value.length === 64 && /^[0-9a-f]{64}$/.test(value);
const integer = (value, max) => Number.isInteger(value) && value >= 1 && value <= max;

function utcTime(value) {
  if (typeof value !== "string") return false;
  const match = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/.exec(value);
  if (match?.[0] !== value) return false;
  const time = new Date(value);
  // Date parses some impossible calendar dates by rolling into the next month.
  return Number.isFinite(time.getTime()) && time.toISOString().slice(0, 19) === value.slice(0, 19);
}

function float16(bits) {
  const sign = bits & 0x8000 ? -1 : 1, exponent = (bits >> 10) & 31, fraction = bits & 1023;
  if (exponent === 31) return fraction ? NaN : sign * Infinity;
  if (exponent === 0) return sign * fraction * 2 ** -24;
  return sign * (1 + fraction / 1024) * 2 ** (exponent - 15);
}

export async function sha256Hex(bytes) {
  const digest = new Uint8Array(await globalThis.crypto.subtle.digest("SHA-256", bytes));
  return Array.from(digest, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export async function validateBundle(manifestText, files) {
  if (typeof manifestText !== "string") return fail("manifest.json must be text.");
  const manifestBytes = files instanceof Map ? files.get("manifest.json") : null;
  if (manifestText.length > MAX_MANIFEST_BYTES || (manifestBytes instanceof Uint8Array && manifestBytes.byteLength > MAX_MANIFEST_BYTES)) return fail("manifest.json exceeds its byte bound.");
  let manifest;
  try { manifest = JSON.parse(manifestText); }
  catch { return fail("manifest.json must contain valid JSON."); }
  if (!exactKeys(manifest, ROOT_KEYS)) return fail("Manifest keys are invalid.");
  if (manifest.format !== "tastelab-images" || manifest.version !== 1) return fail("Unsupported bundle format or version.");
  if (!text(manifest.id, 25, 25) || !/^[0-9]{8}T[0-9]{6}Z-[0-9a-f]{8}$/.test(manifest.id)) return fail("Bundle id is invalid.");
  if (!utcTime(manifest.createdAt)) return fail("createdAt must be an ISO 8601 UTC time.");
  if (!exactKeys(manifest.model, ["id", "weightsSha256"])) return fail("Model keys are invalid.");
  if (!text(manifest.model.id, 1, 100) || !hash(manifest.model.weightsSha256)) return fail("Model id or weightsSha256 is invalid.");
  if (!Array.isArray(manifest.items) || !integer(manifest.items.length, MAX_ITEMS)) return fail("A bundle must contain 1 to 300 items.");

  const imageIds = new Set(), names = new Set(["manifest.json"]);
  for (const item of manifest.items) {
    if (!exactKeys(item, ITEM_KEYS)) return fail("Item keys are invalid.");
    if (!hash(item.imageId) || imageIds.has(item.imageId)) return fail("imageId must be unique lowercase SHA-256 hex.");
    imageIds.add(item.imageId);
    if (!hash(item.thumbSha256)) return fail("thumbSha256 must be lowercase SHA-256 hex.");
    const name = item.thumbSha256 + ".jpg";
    if (names.has(name)) return fail("Each item must have its own thumbnail file.");
    names.add(name);
    if (!integer(item.width, 512) || !integer(item.height, 512) || !integer(item.bytes, THUMB_MAX_BYTES)) return fail("Item width, height or bytes is outside its integer bounds.");
    if (!text(item.sourceId, 1, 200) || !text(item.title, 0, 300) || !text(item.credit, 1, 300) || !item.credit.trim()) return fail("Item text or credit is invalid.");
    if (!licenceOk(item.source, item.licence, item.licenceUrl)) return fail("Item source, licence or licenceUrl is not class A.");
    if (!pageUrlOk(item.source, item.pageUrl)) return fail("Item pageUrl is invalid for its source.");
  }

  if (!(files instanceof Map) || files.size !== names.size || [...files.keys()].some((name) => !names.has(name))
      || [...files.values()].some((bytes) => !(bytes instanceof Uint8Array))) return fail("Files must contain exactly manifest.json and one Uint8Array thumbnail per item.");
  try {
    if (new TextDecoder("utf-8", { fatal: true }).decode(files.get("manifest.json")) !== manifestText) return fail("manifest.json bytes disagree with its text.");
  } catch { return fail("manifest.json must be UTF-8."); }
  for (const item of manifest.items) {
    const bytes = files.get(item.thumbSha256 + ".jpg");
    if (bytes.length !== item.bytes) return fail("Thumbnail file size does not equal item.bytes.");
    if (bytes[0] !== 0xff || bytes[1] !== 0xd8 || bytes[2] !== 0xff) return fail("Thumbnail must start with JPEG bytes FF D8 FF.");
    try {
      if (await sha256Hex(bytes) !== item.thumbSha256) return fail("Thumbnail SHA-256 does not match its file name and thumbSha256.");
    } catch { return fail("Thumbnail SHA-256 could not be checked."); }
  }

  const embeddings = manifest.embeddings;
  if (!exactKeys(embeddings, ["dtype", "dim", "count", "data"])) return fail("Embeddings keys are invalid.");
  if (embeddings.dtype !== "float16" || embeddings.dim !== 512 || embeddings.count !== manifest.items.length) return fail("Embeddings dtype, dim or count is invalid.");
  const byteCount = embeddings.count * 1024;
  if (byteCount > MAX_EMBEDDING_BYTES || typeof embeddings.data !== "string" || embeddings.data.length !== 4 * Math.ceil(byteCount / 3)
      || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(embeddings.data)) return fail("Embeddings must be base64 of exactly count × 1024 bytes.");
  let decoded;
  try { decoded = atob(embeddings.data); }
  catch { return fail("Embeddings base64 is invalid."); }
  if (decoded.length !== byteCount) return fail("Embeddings decoded byte count is invalid.");
  const vectors = new Float32Array(embeddings.count * 512);
  for (let row = 0; row < embeddings.count; row++) {
    let normSquared = 0;
    for (let col = 0; col < 512; col++) {
      const index = row * 512 + col, offset = index * 2;
      const value = float16(decoded.charCodeAt(offset) | (decoded.charCodeAt(offset + 1) << 8));
      if (!Number.isFinite(value)) return fail("Embeddings must contain only finite values.");
      vectors[index] = value;
      normSquared += value * value;
    }
    if (Math.abs(Math.sqrt(normSquared) - 1) > 1e-2) return fail("Each embedding must have unit norm ± 1e-2.");
  }
  const { id, createdAt, model, items } = manifest;
  return { ok: true, bundle: { id, createdAt, model, items, vectors } };
}
