import test from "node:test";
import assert from "node:assert/strict";
import {
  KIND, VERSION, TEXT_MAX, ATTRIBUTE_KINDS, BRIDGES, BRIDGE_LABELS,
  PLACES, PLACE_LABELS, DIRECTIONS, STATUSES, AUTHORS,
  emptyDoc, validateDoc, parseDoc, serializeDoc, mergeFolder, autoPairs,
  nextId, removePair, removeCard, cardFromPair, summarize,
} from "../model.js";
import { defaultSpace } from "../../core/space.js";

// Evidence: synthetic reference metadata only; no images or personal documents.
function reference(id, file, extra = {}) {
  return {
    id, file, sha256: null, bytes: null, subject: "", source: "", notes: "",
    attributes: null, missing: false, ...extra,
  };
}

function fixture() {
  return {
    kind: "tastelab-references",
    version: 1,
    updated: "2026-10-02T12:34:56.000Z",
    references: [
      reference("r1", "01-like-a.jpg", {
        sha256: "a".repeat(64), bytes: 123,
        subject: "A building", source: "Owner's reference", notes: "Look at the treatment",
        attributes: { palette: { hue: 0.4, relations: { contrast: 0.8 } }, material: 0.3, motion: null },
      }),
      reference("r2", "01-dislike-b.png", {
        sha256: "b".repeat(64), bytes: 456, subject: "Another building",
        source: "A second reference", notes: "Too busy", attributes: { rhythm: 0.7 },
      }),
      reference("r3", "03-like-c.jpg", { subject: "A ring", missing: true }),
    ],
    pairs: [{
      id: "p1", liked: "r1", disliked: "r2", property: "Quiet repeated edges",
      feeling: "Calm focus", notes: "Keep the rhythm", kinds: ["palette", "structure"],
      connections: ["r3"],
    }],
    cards: [{
      id: "c1", interest: "Ordered rings", bridge: { kind: "symmetry", text: "Repeated orbits" },
      place: { kind: "theme", text: "A calm working view" }, refs: ["r1", "r3"], pairs: ["p1"],
      g2: "Make structure readable",
      g3: [{ param: "chroma", direction: "lower" }, { param: "lightness", direction: "higher" }],
      h01: "The linked views", status: "draft", author: "owner",
    }],
  };
}

function set(doc, path, value) {
  const parts = path.replace(/\[(\d+)\]/g, ".$1").split(".");
  const key = parts.pop();
  const parent = parts.reduce((object, part) => object[part], doc);
  parent[key] = value;
}

function problemAt(doc, path) {
  const problems = validateDoc(doc);
  assert.ok(problems.some((problem) => problem.startsWith(`${path}:`)),
    `Expected a problem at ${path}; got ${JSON.stringify(problems)}`);
}

function freeze(value) {
  if (value && typeof value === "object") {
    Object.values(value).forEach(freeze);
    Object.freeze(value);
  }
  return value;
}

test("the exported vocabularies are frozen and every choice validates", () => {
  assert.equal(KIND, "tastelab-references");
  assert.equal(VERSION, 1);
  assert.equal(TEXT_MAX, 2000);
  assert.deepEqual(ATTRIBUTE_KINDS, ["palette", "rhythm", "material", "typography", "structure", "motion"]);
  assert.deepEqual(BRIDGES, ["symmetry", "quaternions", "hopf-fibres", "stage-order", "instrument", "other"]);
  assert.deepEqual(BRIDGE_LABELS, {
    symmetry: "Symmetry", quaternions: "Quaternions", "hopf-fibres": "Hopf fibres",
    "stage-order": "Stage order", instrument: "Instrument metaphor", other: "Other",
  });
  assert.deepEqual(PLACES, ["theme", "signature-moment", "theory-figure", "icon-motif", "celebration", "other"]);
  assert.deepEqual(PLACE_LABELS, {
    theme: "Theme", "signature-moment": "Signature moment", "theory-figure": "Theory-book figure",
    "icon-motif": "Icon motif", celebration: "Celebration", other: "Other",
  });
  assert.deepEqual(DIRECTIONS, ["higher", "lower", "keep"]);
  assert.deepEqual(STATUSES, ["draft", "confirmed"]);
  assert.deepEqual(AUTHORS, ["owner", "agent"]);
  for (const value of [ATTRIBUTE_KINDS, BRIDGES, BRIDGE_LABELS, PLACES, PLACE_LABELS, DIRECTIONS, STATUSES, AUTHORS]) {
    assert.ok(Object.isFrozen(value));
    assert.throws(() => { value.extra = true; }, TypeError);
  }
  for (const [path, choices] of [
    ["cards[0].bridge.kind", BRIDGES], ["cards[0].place.kind", PLACES],
    ["cards[0].g3[0].direction", DIRECTIONS], ["cards[0].status", STATUSES], ["cards[0].author", AUTHORS],
  ]) {
    for (const choice of choices) {
      const doc = fixture();
      set(doc, path, choice);
      assert.deepEqual(validateDoc(doc), []);
    }
  }
  const doc = fixture();
  doc.pairs[0].kinds = [...ATTRIBUTE_KINDS];
  doc.cards[0].g3 = defaultSpace().params.map((param) => ({ param: param.id, direction: "keep" }));
  assert.deepEqual(validateDoc(doc), []);
});

test("emptyDoc returns independent, valid empty documents", () => {
  const expected = { kind: "tastelab-references", version: 1, updated: null, references: [], pairs: [], cards: [] };
  const first = emptyDoc(), second = emptyDoc();
  assert.deepEqual(first, expected);
  assert.deepEqual(validateDoc(first), []);
  first.references.push(reference("r1", "a.jpg"));
  assert.deepEqual(second, expected);
});

test("a complete document round trips with ordered keys and a fresh timestamp", () => {
  const original = fixture();
  assert.deepEqual(validateDoc(original), []);
  // Reverse schema key order to exercise canonical serialization.
  const reverse = (object) => Object.fromEntries(Object.entries(object).reverse());
  const doc = reverse(original);
  doc.references = doc.references.map(reverse);
  doc.pairs = doc.pairs.map(reverse);
  doc.cards = doc.cards.map((card) => ({
    ...reverse(card), bridge: reverse(card.bridge), place: reverse(card.place), g3: card.g3.map(reverse),
  }));
  const before = structuredClone(doc);
  const text = serializeDoc(freeze(doc), new Date("2026-10-03T14:15:16.123Z"));
  const result = parseDoc(text);
  assert.deepEqual(result.problems, []);
  assert.deepEqual(result.doc, { ...original, updated: "2026-10-03T14:15:16.123Z" });
  assert.deepEqual(doc, before);
  assert.equal(text, JSON.stringify(result.doc, null, 2) + "\n");
  assert.deepEqual(Object.keys(result.doc), ["kind", "version", "updated", "references", "pairs", "cards"]);
  assert.deepEqual(Object.keys(result.doc.references[0]),
    ["id", "file", "sha256", "bytes", "subject", "source", "notes", "attributes", "missing"]);
  assert.deepEqual(Object.keys(result.doc.pairs[0]),
    ["id", "liked", "disliked", "property", "feeling", "notes", "kinds", "connections"]);
  assert.deepEqual(Object.keys(result.doc.cards[0]),
    ["id", "interest", "bridge", "place", "refs", "pairs", "g2", "g3", "h01", "status", "author"]);
  assert.deepEqual(Object.keys(result.doc.cards[0].bridge), ["kind", "text"]);
  assert.deepEqual(Object.keys(result.doc.cards[0].place), ["kind", "text"]);
  assert.deepEqual(Object.keys(result.doc.cards[0].g3[0]), ["param", "direction"]);
});

test("updated accepts null and ISO calendar dates and timestamps", () => {
  for (const updated of [null, "2026-10-03", "2026-10-03T12:30Z", "2026-10-03T12:30:45+01:00", "2024-02-29T00:00:00.000Z"]) {
    assert.deepEqual(validateDoc({ ...fixture(), updated }), []);
  }
});

const violations = [
  ["wrong kind", "kind", "other"],
  ["wrong version", "version", 2],
  ["non-ISO updated", "updated", "October 3, 2026"],
  ["impossible ISO day", "updated", "2026-02-30T00:00:00Z"],
  ["bad reference id", "references[0].id", "r0"],
  ["leading zero in an id", "references[0].id", "r01"],
  ["id beyond six digits", "references[0].id", "r1000000"],
  ["bad pair id", "pairs[0].id", "r1"],
  ["bad card id", "cards[0].id", "c-1"],
  ["empty file name", "references[0].file", ""],
  ["slash in file name", "references[0].file", "a/b.jpg"],
  ["backslash in file name", "references[0].file", "a\\b.jpg"],
  ["colon in file name", "references[0].file", "c:x.jpg"],
  ["dot file name", "references[0].file", "."],
  ["parent file name", "references[0].file", ".."],
  ["control character in file name", "references[0].file", "a\u0001.jpg"],
  ["DEL in file name", "references[0].file", "a\u007f.jpg"],
  ["C1 control in file name", "references[0].file", "a\u0085.jpg"],
  ["file name over 255 characters", "references[0].file", "a".repeat(256)],
  ["short sha256", "references[0].sha256", "a".repeat(63)],
  ["uppercase sha256", "references[0].sha256", "A".repeat(64)],
  ["non-hex sha256", "references[0].sha256", "g".repeat(64)],
  ["negative bytes", "references[0].bytes", -1],
  ["fractional bytes", "references[0].bytes", 1.5],
  ["infinite bytes", "references[0].bytes", Infinity],
  ["attributes containing a string", "references[0].attributes.palette", "blue"],
  ["attributes containing NaN", "references[0].attributes.palette", NaN],
  ["attributes containing infinity", "references[0].attributes.palette", Infinity],
  ["attributes depth 4", "references[0].attributes", { a: { b: { c: { d: 1 } } } }, "references[0].attributes.a.b.c"],
  ["attributes over 64 keys", "references[0].attributes", Object.fromEntries(Array.from({ length: 65 }, (_, i) => [`k${i}`, i]))],
  ["bad attribute key", "references[0].attributes", { "bad-key": 1 }, "references[0].attributes.bad-key"],
  ["long attribute key", "references[0].attributes", { ["a".repeat(33)]: 1 }, `references[0].attributes.${"a".repeat(33)}`],
  ["attributes containing an array", "references[0].attributes.palette", [1]],
  ["attributes root must be an object", "references[0].attributes", 1],
  ["pair references must differ", "pairs[0].disliked", "r1"],
  ["unknown liked reference", "pairs[0].liked", "r9"],
  ["unknown disliked reference", "pairs[0].disliked", "r9"],
  ["unknown attribute kind", "pairs[0].kinds", ["colour"], "pairs[0].kinds[0]"],
  ["repeated attribute kind", "pairs[0].kinds", ["palette", "palette"], "pairs[0].kinds[1]"],
  ["unknown connection", "pairs[0].connections", ["r9"], "pairs[0].connections[0]"],
  ["connection to liked reference", "pairs[0].connections", ["r1"], "pairs[0].connections[0]"],
  ["connection to disliked reference", "pairs[0].connections", ["r2"], "pairs[0].connections[0]"],
  ["repeated connection", "pairs[0].connections", ["r3", "r3"], "pairs[0].connections[1]"],
  ["unknown bridge kind", "cards[0].bridge.kind", "ring"],
  ["unknown place kind", "cards[0].place.kind", "wallpaper"],
  ["unknown card reference", "cards[0].refs", ["r9"], "cards[0].refs[0]"],
  ["repeated card reference", "cards[0].refs", ["r1", "r1"], "cards[0].refs[1]"],
  ["unknown card pair", "cards[0].pairs", ["p9"], "cards[0].pairs[0]"],
  ["repeated card pair", "cards[0].pairs", ["p1", "p1"], "cards[0].pairs[1]"],
  ["unknown G3 parameter", "cards[0].g3[0].param", "unknown"],
  ["repeated G3 parameter", "cards[0].g3[1].param", "chroma"],
  ["unknown direction", "cards[0].g3[0].direction", "up"],
  ["unknown status", "cards[0].status", "ready"],
  ["unknown author", "cards[0].author", "reviewer"],
  ["text over TEXT_MAX", "pairs[0].property", "a".repeat(2001)],
  ["unknown top-level key", "extra", true],
  ["unknown reference key", "references[0].extra", true],
  ["unknown pair key", "pairs[0].extra", true],
  ["unknown card key", "cards[0].extra", true],
  ["unknown bridge key", "cards[0].bridge.extra", true],
  ["unknown place key", "cards[0].place.extra", true],
  ["unknown G3 key", "cards[0].g3[0].extra", true],
];

for (const [name, path, value, problemPath = path] of violations) {
  test(`validation reports the path for ${name}`, () => {
    const doc = fixture();
    set(doc, path, value);
    problemAt(doc, problemPath);
  });
}

for (const collection of ["references", "pairs", "cards"]) {
  test(`validation rejects duplicate ${collection} ids`, () => {
    const doc = fixture();
    doc[collection].push(structuredClone(doc[collection][0]));
    problemAt(doc, `${collection}[${doc[collection].length - 1}].id`);
  });
}

const textPaths = [
  "kind", "updated", "references[0].id", "references[0].file", "references[0].sha256",
  "references[0].subject", "references[0].source", "references[0].notes",
  "pairs[0].id", "pairs[0].liked", "pairs[0].disliked", "pairs[0].property", "pairs[0].feeling",
  "pairs[0].notes", "pairs[0].kinds[0]", "pairs[0].connections[0]",
  "cards[0].id", "cards[0].interest", "cards[0].bridge.kind", "cards[0].bridge.text",
  "cards[0].place.kind", "cards[0].place.text", "cards[0].refs[0]", "cards[0].pairs[0]",
  "cards[0].g2", "cards[0].g3[0].param", "cards[0].g3[0].direction", "cards[0].h01",
  "cards[0].status", "cards[0].author",
];

for (const path of textPaths) {
  test(`validation rejects embedded base64 media at ${path}`, () => {
    for (const media of [
      "data:image/png;base64,aA==", "prefix DATA:IMAGE/SVG+XML;BASE64,aA== suffix",
      "data:image/vendor~type;base64,aA==", "data:image/png;charset=utf-8;base64,aA==",
    ]) {
      const doc = fixture();
      set(doc, path, media);
      problemAt(doc, path);
    }
  });
}

const wrongTypes = [
  ["kind", 1], ["version", "1"], ["updated", {}],
  ["references", {}], ["pairs", "x"], ["cards", null],
  ["references[0]", []], ["pairs[0]", null], ["cards[0]", 42],
  ["references[0].id", false], ["references[0].file", 1], ["references[0].sha256", []],
  ["references[0].bytes", "123"], ["references[0].subject", null], ["references[0].source", {}],
  ["references[0].notes", []], ["references[0].attributes", []], ["references[0].missing", "false"],
  ["pairs[0].id", 1], ["pairs[0].liked", null], ["pairs[0].disliked", []],
  ["pairs[0].property", 1], ["pairs[0].feeling", {}], ["pairs[0].notes", false],
  ["pairs[0].kinds", {}], ["pairs[0].connections", null],
  ["pairs[0].kinds[0]", {}], ["pairs[0].connections[0]", null],
  ["cards[0].id", null], ["cards[0].interest", 1], ["cards[0].bridge", []], ["cards[0].place", false],
  ["cards[0].bridge.kind", 1], ["cards[0].bridge.text", {}],
  ["cards[0].place.kind", null], ["cards[0].place.text", []],
  ["cards[0].refs", "r1"], ["cards[0].pairs", {}], ["cards[0].refs[0]", 1], ["cards[0].pairs[0]", {}],
  ["cards[0].g2", null], ["cards[0].g3", {}], ["cards[0].g3[0]", false],
  ["cards[0].g3[0].param", {}], ["cards[0].g3[0].direction", 1],
  ["cards[0].h01", []], ["cards[0].status", false], ["cards[0].author", 1],
];

for (const [path, value] of wrongTypes) {
  test(`wrong type at ${path} is a problem rather than an exception`, () => {
    const doc = fixture();
    set(doc, path, value);
    assert.doesNotThrow(() => validateDoc(doc));
    problemAt(doc, path);
  });
}

test("validateDoc never throws for arbitrary JSON roots and nested garbage", () => {
  const values = [null, 42, "x", [], {}, {
    kind: [], version: {}, updated: false, references: [null, 42, [], {}],
    pairs: [false, "x", {}, []], cards: [[], null, {}],
  }];
  for (const value of values) {
    assert.doesNotThrow(() => validateDoc(value));
    assert.ok(validateDoc(value).length > 0);
  }
});

test("validateDoc never coerces malformed list entries to strings", () => {
  const doc = fixture();
  const garbage = { toString: null, valueOf: null };
  doc.pairs[0].kinds = [garbage, garbage];
  doc.pairs[0].connections = [garbage, garbage];
  doc.cards[0].g3 = [{ param: garbage, direction: "keep" }, { param: garbage, direction: "keep" }];
  assert.doesNotThrow(() => validateDoc(doc));
  problemAt(doc, "pairs[0].kinds[1]");
  problemAt(doc, "pairs[0].connections[1]");
  problemAt(doc, "cards[0].g3[1].param");
});

test("missing required keys are reported at their paths", () => {
  for (const path of ["", "references[0]", "pairs[0]", "cards[0]", "cards[0].bridge", "cards[0].place", "cards[0].g3[0]"]) {
    const original = fixture();
    const object = path ? path.replace(/\[(\d+)\]/g, ".$1").split(".").reduce((value, key) => value[key], original) : original;
    for (const key of Object.keys(object)) {
      const doc = fixture();
      const target = path ? path.replace(/\[(\d+)\]/g, ".$1").split(".").reduce((value, part) => value[part], doc) : doc;
      delete target[key];
      problemAt(doc, path ? `${path}.${key}` : key);
    }
  }
});

test("valid boundaries include zero bytes, null metadata, 64 attribute keys and depth 3", () => {
  const doc = fixture();
  doc.references[0].id = "r999999";
  doc.pairs[0].liked = "r999999";
  doc.cards[0].refs[0] = "r999999";
  doc.references[0].file = "a".repeat(255);
  doc.references[0].bytes = 0;
  doc.references[0].sha256 = null;
  doc.references[0].attributes = Object.fromEntries(Array.from({ length: 64 }, (_, i) => [`k${i}`, i]));
  doc.references[0].attributes.k0 = { nested: { ["a".repeat(32)]: -0.5, nil: null } };
  doc.pairs[0].property = "a".repeat(2000);
  assert.deepEqual(validateDoc(doc), []);
});

test("parseDoc returns the parsed value and validation problems", () => {
  const result = parseDoc(JSON.stringify(fixture()));
  assert.deepEqual(result, { doc: fixture(), problems: [] });
  for (const value of [null, 42, "x", [], {}, { kind: "wrong" }]) {
    const parsed = parseDoc(JSON.stringify(value));
    assert.deepEqual(parsed.doc, value);
    assert.ok(parsed.problems.length > 0);
  }
});

test("parseDoc returns one problem and a null document for invalid JSON", () => {
  for (const text of ["", "{", "{\"kind\":}"]) {
    const result = parseDoc(text);
    assert.equal(result.doc, null);
    assert.equal(result.problems.length, 1);
    assert.equal(typeof result.problems[0], "string");
  }
});

test("serializeDoc rejects invalid documents and lists their problems", () => {
  const doc = fixture();
  doc.version = 2;
  doc.pairs[0].property = null;
  assert.throws(() => serializeDoc(doc), (error) => {
    assert.ok(error instanceof Error);
    assert.match(error.message, /version:/);
    assert.match(error.message, /pairs\[0\]\.property:/);
    return true;
  });
});

test("serializeDoc uses a current timestamp when now is omitted", () => {
  const before = Date.now();
  const parsed = parseDoc(serializeDoc(emptyDoc()));
  const after = Date.now();
  assert.deepEqual(parsed.problems, []);
  assert.ok(Date.parse(parsed.doc.updated) >= before && Date.parse(parsed.doc.updated) <= after);
});

test("mergeFolder keeps owner text and attributes for an unchanged file", () => {
  const doc = fixture();
  const files = [{ file: "01-like-a.jpg", sha256: "a".repeat(64), bytes: 999, attributes: { palette: 0.9 } }];
  const beforeDoc = structuredClone(doc), beforeFiles = structuredClone(files);
  const result = mergeFolder(freeze(doc), freeze(files));
  assert.deepEqual(result.references[0], beforeDoc.references[0]);
  assert.deepEqual(result.pairs, beforeDoc.pairs);
  assert.deepEqual(result.cards, beforeDoc.cards);
  assert.equal(result.updated, beforeDoc.updated);
  assert.deepEqual(doc, beforeDoc);
  assert.deepEqual(files, beforeFiles);
  assert.notEqual(result, doc);
});

test("mergeFolder replaces hash, bytes and attributes only when the hash changes", () => {
  const doc = fixture();
  const files = [{ file: "01-like-a.jpg", sha256: "c".repeat(64), bytes: 789, attributes: { rhythm: { spacing: 0.2 } } }];
  const beforeDoc = structuredClone(doc), beforeFiles = structuredClone(files);
  const result = mergeFolder(freeze(doc), freeze(files));
  assert.deepEqual(result.references[0], {
    ...beforeDoc.references[0], sha256: "c".repeat(64), bytes: 789, attributes: { rhythm: { spacing: 0.2 } },
  });
  assert.deepEqual(doc, beforeDoc);
  assert.deepEqual(files, beforeFiles);
});

test("mergeFolder fills null attributes even when the hash is unchanged", () => {
  const doc = fixture();
  doc.references[0].attributes = null;
  const result = mergeFolder(doc, [{ file: "01-like-a.jpg", sha256: "a".repeat(64), bytes: 123, attributes: { material: 0.2 } }]);
  assert.deepEqual(result.references[0].attributes, { material: 0.2 });
  const noAttributes = mergeFolder(doc, [{ file: "01-like-a.jpg", sha256: "a".repeat(64), bytes: 123, attributes: null }]);
  assert.equal(noAttributes.references[0].attributes, null);
  assert.equal(doc.references[0].attributes, null);
});

test("mergeFolder marks absent files missing and clears the mark on return", () => {
  const doc = fixture();
  const absent = mergeFolder(doc, []);
  assert.ok(absent.references.every((ref) => ref.missing));
  assert.deepEqual(absent.pairs, doc.pairs);
  assert.deepEqual(absent.cards, doc.cards);
  const returned = mergeFolder(absent, [{ file: "01-like-a.jpg", sha256: "a".repeat(64), bytes: 123, attributes: null }]);
  assert.equal(returned.references[0].missing, false);
  assert.equal(returned.references[1].missing, true);
  assert.equal(absent.references[0].missing, true);
  assert.equal(returned.references[0].subject, "A building");
});

test("mergeFolder keeps reference order and appends fresh ids in file order", () => {
  const doc = emptyDoc();
  doc.references = [reference("r8", "old.jpg"), reference("r2", "present.jpg")];
  const files = [
    { file: "z.png", sha256: null, bytes: 10, attributes: null },
    { file: "present.jpg", sha256: null, bytes: null, attributes: null },
    { file: "a.webp", sha256: "d".repeat(64), bytes: 20, attributes: { material: 0.1 } },
  ];
  const beforeDoc = structuredClone(doc), beforeFiles = structuredClone(files);
  const result = mergeFolder(freeze(doc), freeze(files));
  assert.deepEqual(result.references, [
    reference("r8", "old.jpg", { missing: true }), reference("r2", "present.jpg"),
    reference("r9", "z.png", { bytes: 10 }),
    reference("r10", "a.webp", { sha256: "d".repeat(64), bytes: 20, attributes: { material: 0.1 } }),
  ]);
  assert.deepEqual(validateDoc(result), []);
  assert.deepEqual(doc, beforeDoc);
  assert.deepEqual(files, beforeFiles);
});

test("mergeFolder rejects duplicate file names without changing inputs", () => {
  const doc = fixture();
  const file = { file: "new.jpg", sha256: null, bytes: 0, attributes: null };
  const files = [file, { ...file }];
  const beforeDoc = structuredClone(doc), beforeFiles = structuredClone(files);
  assert.throws(() => mergeFolder(freeze(doc), freeze(files)), RangeError);
  assert.deepEqual(doc, beforeDoc);
  assert.deepEqual(files, beforeFiles);
});

test("autoPairs detects roles, excludes ambiguity and missing files, and sorts numerically", () => {
  const doc = emptyDoc();
  const names = [
    "10-like-a.jpg", "10-dislike-b.jpg", "09-like-a.jpg", "09-dislike-b.jpg",
    "01-like-x.jpg", "01-dislike-y.png", "2 Like.webp", "2_DISLIKE.jpeg",
    "03-喜欢.jpg", "03-不喜欢.png", "04-likeable.jpg", "04-dislike.jpg",
    "05-like-a.jpg", "05-like-b.jpg", "05-dislike.jpg", "06-like.jpg", "06-dislike.jpg",
    "07-like.jpg", "07-dislike.jpg", "08-like.jpg", "08-dislikeable.jpg",
  ];
  doc.references = names.map((file, i) => reference(`r${i + 1}`, file, { missing: i === 16 }));
  doc.pairs = [{
    id: "p4", liked: "r18", disliked: "r19", property: "Existing", feeling: "", notes: "",
    kinds: [], connections: [],
  }];
  const before = structuredClone(doc);
  const result = autoPairs(freeze(doc));
  assert.equal(result.added, 5);
  assert.deepEqual(result.doc.pairs.slice(1), [
    { id: "p5", liked: "r5", disliked: "r6", property: "", feeling: "", notes: "", kinds: [], connections: [] },
    { id: "p6", liked: "r7", disliked: "r8", property: "", feeling: "", notes: "", kinds: [], connections: [] },
    { id: "p7", liked: "r9", disliked: "r10", property: "", feeling: "", notes: "", kinds: [], connections: [] },
    { id: "p8", liked: "r3", disliked: "r4", property: "", feeling: "", notes: "", kinds: [], connections: [] },
    { id: "p9", liked: "r1", disliked: "r2", property: "", feeling: "", notes: "", kinds: [], connections: [] },
  ]);
  assert.deepEqual(result.doc.pairs[0], before.pairs[0]);
  assert.deepEqual(validateDoc(result.doc), []);
  assert.deepEqual(doc, before);
  const again = autoPairs(result.doc);
  assert.equal(again.added, 0);
  assert.deepEqual(again.doc, result.doc);
  assert.notEqual(again.doc, result.doc);
});

test("autoPairs normalizes leading zeros and uses only non-missing references", () => {
  const doc = emptyDoc();
  doc.references = [
    reference("r1", "0003_like.more.jpg"), reference("r2", "3DISLIKE.png"),
    reference("r3", "03-like-copy.png", { missing: true }),
    reference("r4", "0喜欢"), reference("r5", "00不喜欢"),
  ];
  const { doc: result, added } = autoPairs(doc);
  assert.equal(added, 2);
  assert.deepEqual(result.pairs.map((pair) => [pair.id, pair.liked, pair.disliked]),
    [["p1", "r4", "r5"], ["p2", "r1", "r2"]]);
});

test("autoPairs keeps distinct integers beyond Number's exact range", () => {
  const doc = emptyDoc();
  doc.references = [
    reference("r1", "9007199254740993-like.jpg"), reference("r2", "9007199254740993-dislike.jpg"),
    reference("r3", "9007199254740992-like.jpg"), reference("r4", "9007199254740992-dislike.jpg"),
  ];
  const { doc: result, added } = autoPairs(doc);
  assert.equal(added, 2);
  assert.deepEqual(result.pairs.map((pair) => [pair.liked, pair.disliked]), [["r3", "r4"], ["r1", "r2"]]);
});

test("nextId uses the largest suffix, not collection order, count or holes", () => {
  const doc = emptyDoc();
  assert.equal(nextId(doc, "references"), "r1");
  assert.equal(nextId(doc, "pairs"), "p1");
  assert.equal(nextId(doc, "cards"), "c1");
  doc.references = [reference("r8", "a.jpg"), reference("r2", "b.jpg")];
  const pair = fixture().pairs[0], card = fixture().cards[0];
  doc.pairs = [{ ...pair, id: "p12" }, { ...pair, id: "p3" }];
  doc.cards = [{ ...card, id: "c9" }, { ...card, id: "c4" }];
  const before = structuredClone(doc);
  assert.equal(nextId(freeze(doc), "references"), "r9");
  assert.equal(nextId(doc, "pairs"), "p13");
  assert.equal(nextId(doc, "cards"), "c10");
  assert.deepEqual(doc, before);
});

test("removePair removes its card links while preserving the remaining pairs", () => {
  const doc = fixture();
  doc.pairs.push({ ...structuredClone(doc.pairs[0]), id: "p3" });
  doc.cards[0].pairs.push("p3");
  doc.cards.push({ ...structuredClone(doc.cards[0]), id: "c2", pairs: ["p1"] });
  const before = structuredClone(doc);
  const result = removePair(freeze(doc), "p1");
  assert.deepEqual(result.pairs, [before.pairs[1]]);
  assert.deepEqual(result.cards[0].pairs, ["p3"]);
  assert.deepEqual(result.cards[1].pairs, []);
  assert.deepEqual(result.cards[0].refs, before.cards[0].refs);
  assert.deepEqual(result.references, before.references);
  assert.deepEqual(validateDoc(result), []);
  assert.deepEqual(doc, before);
});

test("removeCard removes only the named card and preserves its sources", () => {
  const doc = fixture();
  doc.cards.push({ ...structuredClone(doc.cards[0]), id: "c3" });
  const before = structuredClone(doc);
  const result = removeCard(freeze(doc), "c1");
  assert.deepEqual(result.cards, [before.cards[1]]);
  assert.deepEqual(result.references, before.references);
  assert.deepEqual(result.pairs, before.pairs);
  assert.deepEqual(validateDoc(result), []);
  assert.deepEqual(doc, before);
});

test("cardFromPair copies liked and connection refs and appends owner draft defaults", () => {
  const doc = fixture();
  doc.cards[0].id = "c8";
  const before = structuredClone(doc);
  const result = cardFromPair(freeze(doc), "p1");
  assert.equal(result.id, "c9");
  assert.deepEqual(result.doc.cards, [before.cards[0], {
    id: "c9", interest: "", bridge: { kind: "other", text: "" }, place: { kind: "other", text: "" },
    refs: ["r1", "r3"], pairs: ["p1"], g2: "", g3: [], h01: "", status: "draft", author: "owner",
  }]);
  assert.deepEqual(result.doc.references, before.references);
  assert.deepEqual(result.doc.pairs, before.pairs);
  assert.deepEqual(validateDoc(result.doc), []);
  assert.deepEqual(doc, before);
});

test("unknown ids throw RangeError and leave the document unchanged", () => {
  const doc = fixture();
  const before = structuredClone(doc);
  freeze(doc);
  assert.throws(() => removePair(doc, "p9"), RangeError);
  assert.throws(() => removeCard(doc, "c9"), RangeError);
  assert.throws(() => cardFromPair(doc, "p9"), RangeError);
  assert.deepEqual(doc, before);
});

test("summarize reports annotated pairs and labelled nexus cards from document metadata", () => {
  const doc = fixture();
  doc.pairs[0].property = "Quiet\r\nrepeated\tedges";
  doc.cards[0].interest = "Ordered\nrings";
  doc.cards[0].bridge.text = "Repeated\torbits";
  doc.cards[0].place.text = "A calm\nworking view";
  doc.cards[0].g2 = "Make\nstructure readable";
  doc.cards[0].h01 = "The\tlinked views";
  doc.pairs[0].notes = "Keep\nthe rhythm";
  const before = structuredClone(doc);
  const text = summarize(freeze(doc));
  assert.equal(text, [
    "Taste Lab references: 3 references, 1 pairs, 1 cards",
    "p1: liked 01-like-a.jpg vs disliked 01-dislike-b.png | property: Quiet repeated edges | feeling: Calm focus | kinds: palette, structure | connections: 03-like-c.jpg (missing) | notes: Keep the rhythm",
    "c1 (draft, owner): interest: Ordered rings | bridge: Symmetry: Repeated orbits | place: Theme: A calm working view | refs: 01-like-a.jpg, 03-like-c.jpg (missing) | pairs: p1 | G2: Make structure readable | G3: chroma lower, lightness higher | H-01: The linked views",
  ].join("\n"));
  assert.doesNotMatch(text, /data:/i);
  assert.doesNotMatch(text, /\t|\r/);
  assert.deepEqual(doc, before);
});

test("summarize marks missing liked and disliked references and omits empty fields", () => {
  const doc = fixture();
  doc.references[0].missing = true;
  doc.references[1].missing = true;
  Object.assign(doc.pairs[0], { property: "", feeling: "", notes: "", kinds: [], connections: [] });
  Object.assign(doc.cards[0], { interest: "", g2: "", g3: [], h01: "", refs: [], pairs: [] });
  const text = summarize(doc);
  assert.match(text, /liked 01-like-a\.jpg \(missing\) vs disliked 01-dislike-b\.png \(missing\)/);
  assert.doesNotMatch(text, /property:|feeling:|notes:|kinds:|connections:|interest:|refs:|pairs:|G2:|G3:|H-01:/);
  assert.equal(summarize(emptyDoc()), "Taste Lab references: 0 references, 0 pairs, 0 cards");
});

test("summarize labels every bridge and place and never emits embedded media", () => {
  for (const kind of BRIDGES) {
    const doc = fixture();
    doc.cards[0].bridge.kind = kind;
    assert.ok(summarize(doc).includes(`bridge: ${BRIDGE_LABELS[kind]}: Repeated orbits`));
  }
  for (const kind of PLACES) {
    const doc = fixture();
    doc.cards[0].place.kind = kind;
    assert.ok(summarize(doc).includes(`place: ${PLACE_LABELS[kind]}: A calm working view`));
  }
  const doc = fixture();
  doc.cards[0].interest = "data:image/png;base64,aA==";
  assert.throws(() => summarize(doc), Error);
});
