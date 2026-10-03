// Pure model for annotated reference pairs and nexus cards. The document never carries image data.
import { defaultSpace } from "../core/space.js";

export const KIND = "tastelab-references";
export const VERSION = 1;
export const TEXT_MAX = 2000;
export const ATTRIBUTE_KINDS = Object.freeze(["palette", "rhythm", "material", "typography", "structure", "motion"]);
export const BRIDGES = Object.freeze(["symmetry", "quaternions", "hopf-fibres", "stage-order", "instrument", "other"]);
export const BRIDGE_LABELS = Object.freeze({
  symmetry: "Symmetry", quaternions: "Quaternions", "hopf-fibres": "Hopf fibres",
  "stage-order": "Stage order", instrument: "Instrument metaphor", other: "Other",
});
export const PLACES = Object.freeze(["theme", "signature-moment", "theory-figure", "icon-motif", "celebration", "other"]);
export const PLACE_LABELS = Object.freeze({
  theme: "Theme", "signature-moment": "Signature moment", "theory-figure": "Theory-book figure",
  "icon-motif": "Icon motif", celebration: "Celebration", other: "Other",
});
export const DIRECTIONS = Object.freeze(["higher", "lower", "keep"]);
export const STATUSES = Object.freeze(["draft", "confirmed"]);
export const AUTHORS = Object.freeze(["owner", "agent"]);

const DOC_KEYS = ["kind", "version", "updated", "references", "pairs", "cards"];
const REF_KEYS = ["id", "file", "sha256", "bytes", "subject", "source", "notes", "attributes", "missing"];
const PAIR_KEYS = ["id", "liked", "disliked", "property", "feeling", "notes", "kinds", "connections"];
const CARD_KEYS = ["id", "interest", "bridge", "place", "refs", "pairs", "g2", "g3", "h01", "status", "author"];
const ID_PREFIXES = { references: "r", pairs: "p", cards: "c" };
const PARAM_IDS = new Set(defaultSpace().params.map((param) => param.id));
const MEDIA_DATA = /data:[^\s;,/]+\/[^\s;,/]+(?:;[^,\r\n]*)?;base64,/i;
const ATTRIBUTE_KEY = /^[A-Za-z][A-Za-z0-9]{0,31}$/;

function plain(value) {
  if (value === null || typeof value !== "object") return false;
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

function clone(value) {
  if (Array.isArray(value)) return value.map(clone);
  if (plain(value)) return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, clone(item)]));
  return value;
}

const ordered = (value, keys) => Object.fromEntries(keys.map((key) => [key, value[key]]));

function isoDate(value) {
  const match = /^(\d{4}|[+-]\d{6})-(\d{2})-(\d{2})(?:T\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?)?$/i.exec(value);
  if (!match || !Number.isFinite(Date.parse(value))) return false;
  const year = Number(match[1]), month = Number(match[2]), day = Number(match[3]);
  const date = new Date(0);
  date.setUTCFullYear(year, month - 1, day);
  return date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day;
}

export function emptyDoc() {
  return { kind: KIND, version: VERSION, updated: null, references: [], pairs: [], cards: [] };
}

export function validateDoc(doc) {
  const problems = [];
  const problem = (path, message) => problems.push(`${path || "doc"}: ${message}`);
  function object(value, keys, path) {
    if (!plain(value)) { problem(path, "expected a plain object"); return false; }
    for (const key of Object.keys(value)) {
      if (!keys.includes(key)) problem(path ? `${path}.${key}` : key, "unknown key");
    }
    return true;
  }
  function text(value, path) {
    if (typeof value !== "string") { problem(path, "expected a string"); return false; }
    if (value.length > TEXT_MAX) problem(path, `text exceeds ${TEXT_MAX} characters`);
    if (MEDIA_DATA.test(value)) problem(path, "embedded base64 media is not allowed");
    return true;
  }
  function list(value, path) {
    if (Array.isArray(value)) return value;
    problem(path, "expected an array");
    return [];
  }
  function choice(value, path, choices) {
    if (text(value, path) && !choices.includes(value)) problem(path, `unknown value ${value}`);
  }
  function known(value, path, ids, label) {
    if (!text(value, path)) return false;
    if (!ids.has(value)) problem(path, `unknown ${label} ${value}`);
    return true;
  }
  function uniqueList(value, path, ids, label, excluded = []) {
    const seen = new Set();
    list(value, path).forEach((item, i) => {
      const itemPath = `${path}[${i}]`;
      if (!known(item, itemPath, ids, label)) return;
      if (seen.has(item)) problem(itemPath, `duplicate ${label} ${item}`);
      if (excluded.includes(item)) problem(itemPath, "connection is one of the pair's own references");
      seen.add(item);
    });
  }
  function id(value, path, prefix, seen) {
    if (!text(value, path)) return;
    if (!new RegExp(`^${prefix}[1-9][0-9]{0,5}$`).test(value)) problem(path, "invalid id");
    if (seen.has(value)) problem(path, `duplicate id ${value}`);
    seen.add(value);
  }
  function attributes(value, path, depth = 1) {
    if (value === null) return;
    if (!plain(value)) { problem(path, "expected a numeric tree or null"); return; }
    // Count the root attributes object as level 1; each nested object adds a level.
    if (depth > 3) { problem(path, "numeric tree exceeds 3 levels"); return; }
    const entries = Object.entries(value);
    if (entries.length > 64) problem(path, "numeric tree exceeds 64 keys");
    for (const [key, item] of entries) {
      const itemPath = `${path}.${key}`;
      if (!ATTRIBUTE_KEY.test(key)) problem(itemPath, "invalid attribute key");
      if (item === null) continue;
      if (typeof item === "number") {
        if (!Number.isFinite(item)) problem(itemPath, "expected a finite number");
      } else if (plain(item)) attributes(item, itemPath, depth + 1);
      else problem(itemPath, "expected a finite number, null or numeric tree");
    }
  }

  if (!object(doc, DOC_KEYS, "")) return problems;
  if (text(doc.kind, "kind") && doc.kind !== KIND) problem("kind", `expected ${KIND}`);
  if (doc.version !== VERSION) problem("version", `expected ${VERSION}`);
  if (doc.updated !== null && text(doc.updated, "updated") && !isoDate(doc.updated)) {
    problem("updated", "expected an ISO 8601 date or timestamp, or null");
  }
  const references = list(doc.references, "references");
  const pairs = list(doc.pairs, "pairs");
  const cards = list(doc.cards, "cards");
  const refIds = new Set(references.filter(plain).map((ref) => ref.id));
  const pairIds = new Set(pairs.filter(plain).map((pair) => pair.id));
  const referenceSeen = new Set(), pairSeen = new Set(), cardSeen = new Set();

  references.forEach((ref, i) => {
    const path = `references[${i}]`;
    if (!object(ref, REF_KEYS, path)) return;
    id(ref.id, `${path}.id`, "r", referenceSeen);
    if (text(ref.file, `${path}.file`) && (!ref.file.length || ref.file.length > 255 ||
        /[/\\:\u0000-\u001f\u007f-\u009f]/.test(ref.file) || ref.file === "." || ref.file === "..")) {
      problem(`${path}.file`, "invalid file name");
    }
    if (ref.sha256 !== null && text(ref.sha256, `${path}.sha256`) && !/^[0-9a-f]{64}$/.test(ref.sha256)) {
      problem(`${path}.sha256`, "expected 64 lowercase hex digits or null");
    }
    if (ref.bytes !== null && (!Number.isInteger(ref.bytes) || ref.bytes < 0)) {
      problem(`${path}.bytes`, "expected a non-negative integer or null");
    }
    for (const key of ["subject", "source", "notes"]) text(ref[key], `${path}.${key}`);
    attributes(ref.attributes, `${path}.attributes`);
    if (typeof ref.missing !== "boolean") problem(`${path}.missing`, "expected a boolean");
  });

  pairs.forEach((pair, i) => {
    const path = `pairs[${i}]`;
    if (!object(pair, PAIR_KEYS, path)) return;
    id(pair.id, `${path}.id`, "p", pairSeen);
    known(pair.liked, `${path}.liked`, refIds, "reference");
    known(pair.disliked, `${path}.disliked`, refIds, "reference");
    if (pair.liked === pair.disliked) problem(`${path}.disliked`, "liked and disliked references must differ");
    for (const key of ["property", "feeling", "notes"]) text(pair[key], `${path}.${key}`);
    uniqueList(pair.kinds, `${path}.kinds`, new Set(ATTRIBUTE_KINDS), "attribute kind");
    uniqueList(pair.connections, `${path}.connections`, refIds, "reference", [pair.liked, pair.disliked]);
  });

  cards.forEach((card, i) => {
    const path = `cards[${i}]`;
    if (!object(card, CARD_KEYS, path)) return;
    id(card.id, `${path}.id`, "c", cardSeen);
    for (const key of ["interest", "g2", "h01"]) text(card[key], `${path}.${key}`);
    for (const [key, kinds] of [["bridge", BRIDGES], ["place", PLACES]]) {
      if (object(card[key], ["kind", "text"], `${path}.${key}`)) {
        choice(card[key].kind, `${path}.${key}.kind`, kinds);
        text(card[key].text, `${path}.${key}.text`);
      }
    }
    uniqueList(card.refs, `${path}.refs`, refIds, "reference");
    uniqueList(card.pairs, `${path}.pairs`, pairIds, "pair");
    const paramsSeen = new Set();
    list(card.g3, `${path}.g3`).forEach((entry, j) => {
      const entryPath = `${path}.g3[${j}]`;
      if (!object(entry, ["param", "direction"], entryPath)) return;
      if (known(entry.param, `${entryPath}.param`, PARAM_IDS, "G3 parameter")) {
        if (paramsSeen.has(entry.param)) problem(`${entryPath}.param`, `duplicate G3 parameter ${entry.param}`);
        paramsSeen.add(entry.param);
      }
      choice(entry.direction, `${entryPath}.direction`, DIRECTIONS);
    });
    choice(card.status, `${path}.status`, STATUSES);
    choice(card.author, `${path}.author`, AUTHORS);
  });
  return problems;
}

export function parseDoc(text) {
  let doc;
  try { doc = JSON.parse(text); }
  catch (error) { return { doc: null, problems: [`JSON: ${error.message}`] }; }
  return { doc, problems: validateDoc(doc) };
}

function assertValid(doc) {
  const problems = validateDoc(doc);
  if (problems.length) throw new Error(problems.join("\n"));
}

export function serializeDoc(doc, now = new Date()) {
  assertValid(doc);
  const result = {
    ...ordered(doc, DOC_KEYS),
    updated: now.toISOString(),
    references: doc.references.map((ref) => ordered(ref, REF_KEYS)),
    pairs: doc.pairs.map((pair) => ordered(pair, PAIR_KEYS)),
    cards: doc.cards.map((card) => ({
      ...ordered(card, CARD_KEYS),
      bridge: ordered(card.bridge, ["kind", "text"]),
      place: ordered(card.place, ["kind", "text"]),
      g3: card.g3.map((entry) => ordered(entry, ["param", "direction"])),
    })),
  };
  return JSON.stringify(result, null, 2) + "\n";
}

export function nextId(doc, collection) {
  if (!Object.hasOwn(ID_PREFIXES, collection)) throw new RangeError(`unknown collection ${collection}`);
  const max = doc[collection].reduce((largest, item) => Math.max(largest, Number(item.id.slice(1))), 0);
  return `${ID_PREFIXES[collection]}${max + 1}`;
}

export function mergeFolder(doc, files) {
  const byFile = new Map();
  for (const entry of files) {
    if (byFile.has(entry.file)) throw new RangeError(`duplicate file name ${entry.file}`);
    byFile.set(entry.file, entry);
  }
  const result = clone(doc);
  const existing = new Set(result.references.map((ref) => ref.file));
  for (const ref of result.references) {
    const entry = byFile.get(ref.file);
    ref.missing = !entry;
    if (!entry) continue;
    if (ref.sha256 !== entry.sha256) {
      ref.sha256 = entry.sha256;
      ref.bytes = entry.bytes;
      ref.attributes = clone(entry.attributes);
    } else if (ref.attributes === null && entry.attributes !== null) {
      ref.attributes = clone(entry.attributes);
    }
  }
  for (const entry of files) {
    if (existing.has(entry.file)) continue;
    result.references.push({
      id: nextId(result, "references"), file: entry.file, sha256: entry.sha256, bytes: entry.bytes,
      subject: "", source: "", notes: "", attributes: clone(entry.attributes), missing: false,
    });
  }
  return result;
}

export function autoPairs(doc) {
  const groups = new Map();
  for (const ref of doc.references) {
    if (ref.missing) continue;
    const dot = ref.file.lastIndexOf(".");
    const stem = dot < 0 ? ref.file : ref.file.slice(0, dot);
    const match = /^(\d+)[ _-]*(dislike|like|不喜欢|喜欢)(?![A-Za-z])/i.exec(stem);
    if (!match) continue;
    const number = BigInt(match[1]);
    const role = ["like", "喜欢"].includes(match[2].toLowerCase()) ? "like" : "dislike";
    if (!groups.has(number)) groups.set(number, { like: [], dislike: [] });
    groups.get(number)[role].push(ref.id);
  }
  const result = clone(doc);
  const existing = new Set(doc.pairs.map((pair) => `${pair.liked}|${pair.disliked}`));
  let added = 0;
  const sorted = [...groups].sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0);
  for (const [, roles] of sorted) {
    if (roles.like.length !== 1 || roles.dislike.length !== 1) continue;
    const liked = roles.like[0], disliked = roles.dislike[0];
    if (existing.has(`${liked}|${disliked}`)) continue;
    result.pairs.push({
      id: nextId(result, "pairs"), liked, disliked, property: "", feeling: "", notes: "",
      kinds: [], connections: [],
    });
    added++;
  }
  return { doc: result, added };
}

export function removePair(doc, id) {
  if (!doc.pairs.some((pair) => pair.id === id)) throw new RangeError(`unknown pair ${id}`);
  const result = clone(doc);
  result.pairs = result.pairs.filter((pair) => pair.id !== id);
  for (const card of result.cards) card.pairs = card.pairs.filter((pairId) => pairId !== id);
  return result;
}

export function removeCard(doc, id) {
  if (!doc.cards.some((card) => card.id === id)) throw new RangeError(`unknown card ${id}`);
  const result = clone(doc);
  result.cards = result.cards.filter((card) => card.id !== id);
  return result;
}

export function cardFromPair(doc, pairId) {
  const pair = doc.pairs.find((entry) => entry.id === pairId);
  if (!pair) throw new RangeError(`unknown pair ${pairId}`);
  const result = clone(doc);
  const id = nextId(result, "cards");
  result.cards.push({
    id, interest: "", bridge: { kind: "other", text: "" }, place: { kind: "other", text: "" },
    refs: [pair.liked, ...pair.connections], pairs: [pairId], g2: "", g3: [], h01: "",
    status: "draft", author: "owner",
  });
  return { doc: result, id };
}

export function summarize(doc) {
  assertValid(doc);
  const flat = (text) => text.replace(/[\r\n\t]+/g, " ").trim();
  const byId = new Map(doc.references.map((ref) => [ref.id, ref]));
  const file = (id) => {
    const ref = byId.get(id);
    return flat(ref.file) + (ref.missing ? " (missing)" : "");
  };
  const field = (parts, label, value) => {
    const text = flat(value);
    if (text) parts.push(`${label}: ${text}`);
  };
  const labelled = (value, labels) => labels[value.kind] + (flat(value.text) ? `: ${flat(value.text)}` : "");
  const lines = [`Taste Lab references: ${doc.references.length} references, ${doc.pairs.length} pairs, ${doc.cards.length} cards`];
  for (const pair of doc.pairs) {
    const parts = [`${pair.id}: liked ${file(pair.liked)} vs disliked ${file(pair.disliked)}`];
    field(parts, "property", pair.property);
    field(parts, "feeling", pair.feeling);
    field(parts, "kinds", pair.kinds.join(", "));
    field(parts, "connections", pair.connections.map(file).join(", "));
    field(parts, "notes", pair.notes);
    lines.push(parts.join(" | "));
  }
  for (const card of doc.cards) {
    const parts = [];
    field(parts, "interest", card.interest);
    field(parts, "bridge", labelled(card.bridge, BRIDGE_LABELS));
    field(parts, "place", labelled(card.place, PLACE_LABELS));
    field(parts, "refs", card.refs.map(file).join(", "));
    field(parts, "pairs", card.pairs.join(", "));
    field(parts, "G2", card.g2);
    field(parts, "G3", card.g3.map((entry) => `${entry.param} ${entry.direction}`).join(", "));
    field(parts, "H-01", card.h01);
    lines.push(`${card.id} (${card.status}, ${card.author}): ${parts.join(" | ")}`);
  }
  return lines.join("\n");
}
