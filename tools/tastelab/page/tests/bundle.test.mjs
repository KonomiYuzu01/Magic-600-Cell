import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { validateBundle, sha256Hex } from "../bundle.js";

// Synthetic bytes only. The fixture hashes use Node's independent SHA-256.
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
const encode = (value) => new TextEncoder().encode(value);
function fixture(count = 2) {
  const files = new Map(), data = Buffer.alloc(count * 1024), items = [];
  for (let i = 0; i < count; i++) {
    const bytes = Uint8Array.of(0xff, 0xd8, 0xff, i & 255, i >> 8);
    const thumbSha256 = digest(bytes);
    files.set(thumbSha256 + ".jpg", bytes);
    items.push({ imageId: digest(`synthetic original ${i}`), thumbSha256, width: 512, height: 384, bytes: bytes.length,
      source: "wikimedia", sourceId: `synthetic-${i}`, title: "Synthetic item", credit: "Synthetic creator",
      pageUrl: "https://commons.wikimedia.org/wiki/File:Synthetic.jpg", licence: "CC-BY-SA-4.0",
      licenceUrl: "https://creativecommons.org/licenses/by-sa/4.0/" });
    data.writeUInt16LE(0x3c00, i * 1024 + (i % 512) * 2);
  }
  const manifest = { format: "tastelab-images", version: 1, id: "20261004T120000Z-0123abcd", createdAt: "2026-10-04T12:00:00Z",
    model: { id: "synthetic-model", weightsSha256: "ab".repeat(32) }, items,
    embeddings: { dtype: "float16", dim: 512, count, data: data.toString("base64") } };
  files.set("manifest.json", encode(JSON.stringify(manifest)));
  return { manifest, files };
}
function syncManifest({ manifest, files }) {
  const manifestText = JSON.stringify(manifest);
  if (files.has("manifest.json")) files.set("manifest.json", encode(manifestText));
  return manifestText;
}
function changeBits(manifest, offset, bits) {
  const bytes = Buffer.from(manifest.embeddings.data, "base64");
  bytes.writeUInt16LE(bits, offset);
  manifest.embeddings.data = bytes.toString("base64");
}
function replaceThumbnail(manifest, files, bytes) {
  const item = manifest.items[0];
  files.delete(item.thumbSha256 + ".jpg");
  item.thumbSha256 = digest(bytes);
  item.bytes = bytes.length;
  files.set(item.thumbSha256 + ".jpg", bytes);
}

test("valid synthetic bundle preserves image identity, metadata and item-ordered Float32 vectors", async () => {
  const f = fixture(), result = await validateBundle(syncManifest(f), f.files);
  assert.equal(result.ok, true);
  assert.deepEqual(Object.keys(result.bundle), ["id", "createdAt", "model", "items", "vectors"]);
  assert.deepEqual(result.bundle.items, f.manifest.items);
  assert.deepEqual(result.bundle.model, f.manifest.model);
  assert.equal(result.bundle.id, f.manifest.id);
  assert.equal(result.bundle.createdAt, f.manifest.createdAt);
  assert.ok(result.bundle.vectors instanceof Float32Array);
  assert.equal(result.bundle.vectors.length, 1024);
  assert.deepEqual([...result.bundle.vectors.slice(0, 3)], [1, 0, 0]);
  assert.deepEqual([...result.bundle.vectors.slice(512, 515)], [0, 1, 0]);
  assert.notEqual(result.bundle.items[0].imageId, result.bundle.items[0].thumbSha256, "TLPLAN-01 original identity differs from thumbnail hash");
});

test("SHA-256 uses exactly a Uint8Array view's bytes", async () => {
  assert.equal(await sha256Hex(encode("abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
  assert.equal(await sha256Hex(Uint8Array.of(0, 97, 98, 99, 0).subarray(1, 4)), digest("abc"));
});

for (const [name, value] of [["invalid JSON", "{"], ["non-text manifest", null], ["null root", "null"], ["array root", "[]"]]) {
  test(`bundle refuses ${name}`, async () => {
    const result = await validateBundle(value, fixture().files);
    assert.equal(result.ok, false);
    assert.ok(result.errors.length);
    assert.equal(result.bundle, undefined);
  });
}

const refusals = [
  ["extra manifest key", (m) => { m.extra = true; }],
  ["missing manifest key", (m) => { delete m.createdAt; }],
  ["extra model key", (m) => { m.model.extra = true; }],
  ["missing model key", (m) => { delete m.model.weightsSha256; }],
  ["non-object model", (m) => { m.model = []; }],
  ["extra item key", (m) => { m.items[0].extra = true; }],
  ["missing item key", (m) => { delete m.items[0].title; }],
  ["non-object item", (m) => { m.items[0] = null; }],
  ["extra embeddings key", (m) => { m.embeddings.extra = true; }],
  ["missing embeddings key", (m) => { delete m.embeddings.data; }],
  ["non-object embeddings", (m) => { m.embeddings = null; }],
  ["wrong format", (m) => { m.format = "other"; }],
  ["wrong version", (m) => { m.version = 2; }],
  ["string version", (m) => { m.version = "1"; }],
  ["malformed bundle id", (m) => { m.id = "20261004T120000Z-0123ABCd"; }],
  ["bundle id newline", (m) => { m.id += "\n"; }],
  ["non-string bundle id", (m) => { m.id = null; }],
  ["non-UTC time", (m) => { m.createdAt = "2026-10-04T12:00:00+00:00"; }],
  ["time without timezone", (m) => { m.createdAt = "2026-10-04T12:00:00"; }],
  ["invalid calendar time", (m) => { m.createdAt = "2026-02-30T12:00:00Z"; }],
  ["invalid time of day", (m) => { m.createdAt = "2026-10-04T24:00:00Z"; }],
  ["time newline", (m) => { m.createdAt += "\n"; }],
  ["non-string time", (m) => { m.createdAt = 0; }],
  ["empty model id", (m) => { m.model.id = ""; }],
  ["long model id", (m) => { m.model.id = "x".repeat(101); }],
  ["non-string model id", (m) => { m.model.id = 1; }],
  ["model id control", (m) => { m.model.id = "x\u0085"; }],
  ["uppercase weights hash", (m) => { m.model.weightsSha256 = "AB".repeat(32); }],
  ["short weights hash", (m) => { m.model.weightsSha256 = "a".repeat(63); }],
  ["weights hash newline", (m) => { m.model.weightsSha256 += "\n"; }],
  ["zero items", (m) => { m.items = []; }],
  ["more than 300 items", (m) => { m.items = Array(301).fill(m.items[0]); }],
  ["non-array items", (m) => { m.items = {}; }],
  ["duplicate image ids", (m) => { m.items[1].imageId = m.items[0].imageId; }],
  ["uppercase image id", (m) => { m.items[0].imageId = m.items[0].imageId.toUpperCase(); }],
  ["non-hex image id", (m) => { m.items[0].imageId = "g".repeat(64); }],
  ["image id newline", (m) => { m.items[0].imageId += "\n"; }],
  ["short image id", (m) => { m.items[0].imageId = "a".repeat(63); }],
  ["non-string image id", (m) => { m.items[0].imageId = 1; }],
  ["uppercase thumbnail hash", (m) => { m.items[0].thumbSha256 = m.items[0].thumbSha256.toUpperCase(); }],
  ["non-hex thumbnail hash", (m) => { m.items[0].thumbSha256 = "g".repeat(64); }],
  ["thumbnail hash newline", (m) => { m.items[0].thumbSha256 += "\n"; }],
  ["duplicate thumbnail names", (m) => { m.items[1].thumbSha256 = m.items[0].thumbSha256; }],
  ["empty source id", (m) => { m.items[0].sourceId = ""; }],
  ["long source id", (m) => { m.items[0].sourceId = "x".repeat(201); }],
  ["source id control", (m) => { m.items[0].sourceId = "x\u0000"; }],
  ["non-string source id", (m) => { m.items[0].sourceId = 0; }],
  ["long title", (m) => { m.items[0].title = "x".repeat(301); }],
  ["title control", (m) => { m.items[0].title = "x\n"; }],
  ["non-string title", (m) => { m.items[0].title = null; }],
  ["empty credit", (m) => { m.items[0].credit = ""; }],
  ["blank credit", (m) => { m.items[0].credit = " "; }],
  ["long credit", (m) => { m.items[0].credit = "x".repeat(301); }],
  ["credit control", (m) => { m.items[0].credit = "x\u007f"; }],
  ["non-string credit", (m) => { m.items[0].credit = null; }],
  ["class B archive source canary", (m) => { m.items[0].source = "archive"; }],
  ["non-string source", (m) => { m.items[0].source = ["wikimedia"]; }],
  ["non-coercible source", (m) => { m.items[0].source = { toString: null }; }],
  ["class B private-reference licence canary", (m) => { m.items[0].licence = "private-reference"; }],
  ["class B safebooru page canary", (m) => { m.items[0].pageUrl = "https://safebooru.org/index.php"; }],
  ["unapproved licence URL", (m) => { m.items[0].licenceUrl = "https://evil.invalid/licence"; }],
  ["wrong source for public-domain", (m) => { m.items[0].source = "met"; m.items[0].licence = "public-domain"; m.items[0].licenceUrl = "https://commons.wikimedia.org/wiki/Test"; }],
  ["unsafe page URL", (m) => { m.items[0].pageUrl = "https://commons.wikimedia.org:443/wiki/Test"; }],
  ["missing manifest file", (m, f) => { f.delete("manifest.json"); }],
  ["missing thumbnail", (m, f) => { f.delete(m.items[0].thumbSha256 + ".jpg"); }],
  ["extra file", (m, f) => { f.set("extra.jpg", Uint8Array.of(1)); }],
  ["wrong thumbnail name", (m, f) => { const name = m.items[0].thumbSha256 + ".jpg"; f.set("other.jpg", f.get(name)); f.delete(name); }],
  ["non-Uint8Array file", (m, f) => { f.set(m.items[0].thumbSha256 + ".jpg", [255, 216, 255]); }],
  ["declared file size mismatch", (m) => { m.items[0].bytes++; }],
  ["thumbnail hash mismatch", (m, f) => { f.get(m.items[0].thumbSha256 + ".jpg")[3] ^= 1; }],
  ["JPEG marker mismatch", (m, f) => { replaceThumbnail(m, f, Uint8Array.of(0xff, 0xd8, 0, 1)); }],
  ["truncated JPEG marker", (m, f) => { replaceThumbnail(m, f, Uint8Array.of(0xff, 0xd8)); }],
  ["wrong embeddings dtype", (m) => { m.embeddings.dtype = "float32"; }],
  ["wrong embeddings dimension", (m) => { m.embeddings.dim = 511; }],
  ["string embeddings dimension", (m) => { m.embeddings.dim = "512"; }],
  ["wrong embeddings count", (m) => { m.embeddings.count++; }],
  ["string embeddings count", (m) => { m.embeddings.count = "2"; }],
  ["non-string embeddings data", (m) => { m.embeddings.data = 1; }],
  ["malformed base64", (m) => { m.embeddings.data = "!" + m.embeddings.data.slice(1); }],
  ["base64 whitespace", (m) => { m.embeddings.data = "\n" + m.embeddings.data.slice(1); }],
  ["too few embedding bytes", (m) => { m.embeddings.data = Buffer.alloc(2047).toString("base64"); }],
  ["too many embedding bytes", (m) => { m.embeddings.data = Buffer.alloc(2049).toString("base64"); }],
  ["malformed base64 padding", (m) => { m.embeddings.data = m.embeddings.data.slice(0, -1) + "A"; }],
  ["NaN embedding", (m) => { changeBits(m, 0, 0x7e00); }],
  ["positive infinite embedding", (m) => { changeBits(m, 0, 0x7c00); }],
  ["negative infinite embedding", (m) => { changeBits(m, 0, 0xfc00); }],
  ["embedding norm too low", (m) => { changeBits(m, 0, 0x3beb); }],
  ["embedding norm too high", (m) => { changeBits(m, 0, 0x3c0b); }],
];
for (const field of ["width", "height", "bytes"]) {
  const maximum = field === "bytes" ? 204800 : 512;
  for (const [suffix, value] of [["zero", 0], ["negative", -1], ["over cap", maximum + 1], ["fraction", 1.5], ["string", "1"]]) {
    refusals.push([`${field} ${suffix}`, (m) => { m.items[0][field] = value; }]);
  }
}
for (const [name, mutate] of refusals) test(`bundle refuses ${name}`, async () => {
  const f = fixture();
  mutate(f.manifest, f.files);
  const result = await validateBundle(syncManifest(f), f.files);
  assert.equal(result.ok, false);
  assert.ok(result.errors.length && result.errors.every((error) => typeof error === "string"));
  assert.equal(result.bundle, undefined, "one invalid item refuses the entire bundle");
});

test("bundle refuses a non-Map files argument and mismatched manifest bytes", async () => {
  const f = fixture(), manifestText = syncManifest(f);
  assert.equal((await validateBundle(manifestText, [...f.files])).ok, false);
  f.files.set("manifest.json", encode("{}"));
  assert.equal((await validateBundle(manifestText, f.files)).ok, false);
  f.files.set("manifest.json", Uint8Array.of(0xff));
  assert.equal((await validateBundle(manifestText, f.files)).ok, false);
});

test("item counts 1 and 300, text maxima, dimensions and 204800 bytes are inclusive", async () => {
  for (const count of [1, 300]) {
    const f = fixture(count), item = f.manifest.items[0];
    f.manifest.model.id = "m".repeat(100);
    item.sourceId = "s".repeat(200); item.title = ""; item.credit = "c".repeat(300);
    item.width = 1; item.height = 512;
    const bytes = new Uint8Array(204800); bytes.set([0xff, 0xd8, 0xff]);
    replaceThumbnail(f.manifest, f.files, bytes);
    assert.equal((await validateBundle(syncManifest(f), f.files)).ok, true);
    item.title = "t".repeat(300);
    assert.equal((await validateBundle(syncManifest(f), f.files)).ok, true);
  }
});

test("float16 decodes little-endian normals, negative zero and subnormals by hand", async () => {
  const f = fixture();
  changeBits(f.manifest, 0, 0xbc00);
  changeBits(f.manifest, 2, 0x8000);
  changeBits(f.manifest, 4, 0x0001);
  changeBits(f.manifest, 6, 0x83ff);
  const result = await validateBundle(syncManifest(f), f.files);
  assert.ok(result.ok);
  assert.equal(result.bundle.vectors[0], -1);
  assert.ok(Object.is(result.bundle.vectors[1], -0));
  assert.equal(result.bundle.vectors[2], 2 ** -24);
  assert.equal(result.bundle.vectors[3], -1023 * 2 ** -24);
});

test("unit norm tolerance and fractional UTC calendar times are accepted", async () => {
  for (const bits of [0x3bec, 0x3c0a]) {
    const f = fixture();
    changeBits(f.manifest, 0, bits);
    f.manifest.createdAt = "2024-02-29T23:59:59.123456Z";
    assert.equal((await validateBundle(syncManifest(f), f.files)).ok, true);
  }
});
