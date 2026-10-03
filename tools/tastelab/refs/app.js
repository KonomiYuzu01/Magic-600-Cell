// Taste Lab references (owner design decision 4A): the owner's liked and
// disliked reference pairs and nexus cards, on the owner's computer only. The
// page reads the images of a folder the owner opens, measures palette
// relations and structure here, and keeps references.json in that folder.
// Images are never copied, uploaded or stored: the document holds the owner's
// notes and the measured numbers, and agents read only the document.
import { describeImage, compareImages } from "./attributes.js";
import {
  emptyDoc, parseDoc, serializeDoc, mergeFolder, autoPairs, nextId, removePair, removeCard, cardFromPair, summarize,
  ATTRIBUTE_KINDS, BRIDGES, BRIDGE_LABELS, PLACES, PLACE_LABELS, DIRECTIONS, TEXT_MAX,
} from "./model.js";
import { defaultSpace } from "../core/space.js";

const DOC_FILE = "references.json";
const COPY = /^references-conflict-\d{8}-\d{6}-[0-9a-f]{6}\.json$/; // the page's version, kept when Save meets a changed file
const IMAGE = /\.(jpe?g|png|webp|gif|avif|bmp)$/i;
const SIDE = 256; // longest side of the copy that is measured
const PARAMS = defaultSpace().params.map((p) => p.id);
const PARAM_LABELS = {
  hueRotation: "Hue rotation", hueSpread: "Hue spread", lightness: "Lightness", lightnessAlt: "Lightness alternation",
  chroma: "Chroma", classes: "Colour classes", bgLightness: "Background lightness", bgHue: "Background tint hue",
  bgTint: "Background tint", gap: "Cell gap", edgeWeight: "Edge weight", edgeBrightness: "Edge brightness",
  gloss: "Gloss", glow: "Glow", fog: "Fog", turnMs: "Twist duration", easeA: "Easing start", easeB: "Easing end",
};
const MEASURE_LABELS = {
  "lightness.p50": "Lightness (median)", contrast: "Contrast", "chroma.p50": "Chroma (median)",
  chromaticShare: "Coloured share", hueSpread: "Hue spread", hueCount: "Number of hues", warmShare: "Warm share",
  "background.L": "Background lightness", edgeDensity: "Edge density", "symmetry.mirrorX": "Left-right symmetry",
  "symmetry.mirrorY": "Top-bottom symmetry", "symmetry.rot180": "Half-turn symmetry",
  "orientation.coherence": "Directional structure",
};

const $ = (id) => document.getElementById(id);
const intro = $("intro");
// baseline: the text of references.json as the page last read or wrote it (null: there was none).
const state = { dir: null, name: "", urls: new Map(), doc: null, baseline: null, problems: null, dirty: false, rev: 0, busy: false, tab: "pairs" };

// el("tag", {class, text, on<event>, value, checked, ...attributes}, ...children)
function el(tag, props = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(props)) {
    if (value === undefined || value === null || value === false) continue;
    if (key === "class") node.className = value;
    else if (key === "text") node.textContent = value;
    else if (key === "value") node.value = value;
    else if (key === "checked") node.checked = Boolean(value);
    else if (key.startsWith("on")) node.addEventListener(key.slice(2), value);
    else node.setAttribute(key, value === true ? "" : String(value));
  }
  for (const child of children.flat()) if (child !== null && child !== undefined && child !== false) node.append(child);
  return node;
}

function status(text, kind = "") {
  $("status").textContent = text;
  $("status").dataset.kind = kind;
}

function updateBar() {
  const doc = state.doc;
  $("save").disabled = !doc || !state.dirty || state.busy;
  $("save").textContent = state.dirty ? "Save*" : "Save";
  $("copy").disabled = !doc || state.busy;
  $("open").disabled = state.busy;
  $("folder").textContent = doc
    ? `${state.name}: ${count(doc.references.length, "image")}, ${count(doc.pairs.length, "pair")}, ${count(doc.cards.length, "card")}`
    : "";
}

// A load locks the page; a save does not, and an edit made while it runs stays unsaved.
function setBusy(busy, lock = false) {
  state.busy = busy;
  document.body.classList.toggle("busy", busy);
  document.body.classList.toggle("loading", busy && lock);
  $("main").inert = busy && lock;
  updateBar();
}

function changed() {
  state.rev++;
  state.dirty = true;
  updateBar();
}

const hex = (buffer) => Array.from(new Uint8Array(buffer), (b) => b.toString(16).padStart(2, "0")).join("");
const count = (n, word) => `${n} ${word}${n === 1 ? "" : "s"}`;
const two = (n) => String(n).padStart(2, "0");
const stamp = (d) => `${d.getFullYear()}${two(d.getMonth() + 1)}${two(d.getDate())}-${two(d.getHours())}${two(d.getMinutes())}${two(d.getSeconds())}`;
const fmt = (x) => (typeof x !== "number" ? "-" : String(Math.abs(x) >= 10 ? Math.round(x) : Math.round(x * 100) / 100));
const pct = (x) => (typeof x !== "number" ? "-" : `${Math.round(100 * x)}%`);

// Palette and structure numbers of one image, from a copy whose longest side is
// at most SIDE pixels; null when the browser cannot decode it or it is too small.
async function measure(file) {
  let bitmap = null;
  try {
    bitmap = await createImageBitmap(file);
    const scale = Math.min(1, SIDE / Math.max(bitmap.width, bitmap.height));
    const width = Math.round(bitmap.width * scale), height = Math.round(bitmap.height * scale);
    if (width < 8 || height < 8) return null;
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d", { willReadFrequently: true });
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = "high";
    ctx.drawImage(bitmap, 0, 0, width, height);
    const { data } = ctx.getImageData(0, 0, width, height);
    return describeImage({ width, height, data });
  } catch {
    return null;
  } finally {
    if (bitmap) bitmap.close();
  }
}

async function openFolder() {
  if (state.busy) return;
  if (state.dirty && !confirm("Discard the unsaved changes?")) return;
  if (typeof window.showDirectoryPicker !== "function") {
    $("fallback").click();
    return;
  }
  let dir;
  try {
    dir = await window.showDirectoryPicker({ id: "tastelab-references", mode: "readwrite" });
  } catch (err) {
    if (!err || err.name !== "AbortError") status(`The folder could not be opened (${err && (err.name || err.message)}).`, "error");
    return;
  }
  await loading(async () => {
    const files = [], copies = [];
    let text = null;
    for await (const handle of dir.values()) {
      if (handle.kind !== "file") continue;
      if (handle.name === DOC_FILE) text = await (await handle.getFile()).text();
      else if (COPY.test(handle.name)) copies.push(handle.name);
      else if (IMAGE.test(handle.name)) files.push(await handle.getFile());
    }
    await load(dir, dir.name, files, text, copies);
  });
}

// The text of a file in the folder, or null when there is none.
async function readText(dir, name) {
  try {
    return await (await (await dir.getFileHandle(name)).getFile()).text();
  } catch (err) {
    if (err && err.name === "NotFoundError") return null;
    throw err;
  }
}

// The browser writes a temporary file and replaces the target only on close.
async function writeText(dir, name, text) {
  const writable = await (await dir.getFileHandle(name, { create: true })).createWritable();
  try {
    await writable.write(text);
    await writable.close();
  } catch (err) {
    await writable.abort().catch(() => {});
    throw err;
  }
}

// Writes a conflict copy under a name no file has: the time to the second and a
// random part, so that two tabs saving in the same second never share a name.
async function writeCopy(dir, text) {
  for (let attempt = 0; attempt < 8; attempt++) {
    const name = `references-conflict-${stamp(new Date())}-${hex(crypto.getRandomValues(new Uint8Array(3)))}.json`;
    if ((await readText(dir, name)) !== null) continue;
    await writeText(dir, name, text);
    return name;
  }
  throw new Error("No free name for a conflict copy");
}

// Without folder access (browsers other than Chrome and Edge) the owner picks the
// images and references.json as files, and Save downloads the document.
async function openFiles(event) {
  const picked = [...event.target.files];
  event.target.value = "";
  if (!picked.length) return;
  await loading(async () => {
    const docFile = picked.find((f) => f.name === DOC_FILE);
    await load(null, "selected files", picked.filter((f) => IMAGE.test(f.name)), docFile ? await docFile.text() : null);
  });
}

// One load at a time, with the page locked; a failed load keeps the open document.
async function loading(task) {
  if (state.busy) return;
  setBusy(true, true);
  try {
    await task();
  } catch (err) {
    status(`The folder could not be read (${err.name || err.message}).`, "error");
  } finally {
    setBusy(false);
    render();
  }
}

async function load(dir, name, files, text, copies = []) {
  status("");
  let doc = emptyDoc();
  if (text !== null) {
    const parsed = parseDoc(text);
    if (parsed.problems.length) {
      // Without a document the page cannot save over the file.
      Object.assign(state, { dir: null, doc: null, baseline: null, problems: parsed.problems, dirty: false });
      return;
    }
    doc = parsed.doc;
  }
  const before = JSON.stringify(doc);
  files.sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }));
  const known = new Map(doc.references.map((r) => [r.file, r]));
  const entries = [];
  for (const [i, file] of files.entries()) {
    status(`Measuring ${i + 1} of ${files.length}: ${file.name}`);
    const sha256 = hex(await crypto.subtle.digest("SHA-256", await file.arrayBuffer()));
    const ref = known.get(file.name);
    const attributes = ref && ref.sha256 === sha256 && ref.attributes ? ref.attributes : await measure(file);
    entries.push({ file: file.name, sha256, bytes: file.size, attributes });
  }
  const merged = mergeFolder(doc, entries);
  const fresh = new Set(merged.references.slice(doc.references.length).map((r) => r.id));
  doc = autoPairs(merged).doc;
  // Only images new to the document are paired by name, so a pair the owner deleted stays deleted.
  doc.pairs = doc.pairs.filter((p, i) => i < merged.pairs.length || fresh.has(p.liked) || fresh.has(p.disliked));
  for (const url of state.urls.values()) URL.revokeObjectURL(url);
  state.urls = new Map(files.map((f) => [f.name, URL.createObjectURL(f)]));
  Object.assign(state, { dir, name, doc, baseline: text, problems: null, dirty: JSON.stringify(doc) !== before });
  const unread = doc.references.filter((r) => !r.missing && !r.attributes).length;
  const notes = [];
  if (unread) notes.push(`${count(unread, "image")} could not be measured; notes still work.`);
  if (copies.length) notes.push(`Conflict copies of ${DOC_FILE} in this folder: ${copies.sort().join(", ")}. Merge what you need, then delete them.`);
  status(notes.join(" "));
}

function renderProblems(problems) {
  return el("section", { class: "problems" },
    el("h2", { text: `${DOC_FILE} in this folder is not valid` }),
    el("p", { text: "The page does not change it. Fix or move the file, then open the folder again." }),
    el("ul", {}, problems.slice(0, 50).map((p) => el("li", { text: p }))));
}

async function save() {
  if (!state.doc || state.busy) return;
  const rev = state.rev;
  let text;
  try {
    text = serializeDoc(state.doc);
  } catch (err) {
    status(err.message, "error");
    return;
  }
  setBusy(true);
  try {
    if (!state.dir) {
      // The page cannot see whether the download arrived, so the changes stay unsaved.
      const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
      el("a", { href: url, download: DOC_FILE }).click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      status(`Download of ${DOC_FILE} started; keep it next to the images. The page cannot see whether it arrived, so the changes still count as unsaved.`);
      return;
    }
    // A file changed after the page read it (another tab, a program or an agent) is not overwritten.
    // The browser offers no lock, so a write between this check and the save itself goes unseen.
    const current = await readText(state.dir, DOC_FILE);
    if (current !== null && current !== state.baseline) {
      if (!confirm(`${DOC_FILE} changed in this folder after the page read it, for example in another tab or by an agent. The page does not overwrite it.\n\nOK saves this page's version next to it as a new conflict copy. Cancel saves nothing.`)) {
        status(`Not saved: ${DOC_FILE} changed in the folder after the page read it.`, "error");
        return;
      }
      const copy = await writeCopy(state.dir, text);
      status(`${DOC_FILE} was kept. This page's version is in ${copy}; open the folder again to work from ${DOC_FILE}.`, "error");
    } else {
      await writeText(state.dir, DOC_FILE, text);
      state.baseline = text;
      status(`Saved ${DOC_FILE}.`);
    }
    state.doc.updated = JSON.parse(text).updated;
    state.dirty = state.rev !== rev;
  } catch (err) {
    status(`Not saved (${err.name || err.message}).`, "error");
  } finally {
    setBusy(false);
  }
}

async function copySummary() {
  if (!state.doc) return;
  try {
    await navigator.clipboard.writeText(summarize(state.doc));
    status("Summary copied: text only, no images.");
  } catch (err) {
    status(`The summary could not be copied (${err.name || err.message}).`, "error");
  }
}

const refById = (id) => state.doc.references.find((r) => r.id === id);
const refLabel = (ref) => (ref ? ref.file + (ref.missing ? " (missing)" : "") : "(none)");

function thumb(ref, cls = "thumb") {
  const url = ref && !ref.missing && state.urls.get(ref.file);
  if (!url) return el("div", { class: `${cls} none`, text: ref ? (ref.missing ? "Missing" : "No preview") : "None" });
  return el("img", {
    class: cls, src: url, alt: ref.file, loading: "lazy", decoding: "async",
    onerror: (e) => e.target.replaceWith(el("div", { class: `${cls} none`, text: "No preview" })),
  });
}

function textField(label, obj, key, { multiline = false, placeholder = "" } = {}) {
  const input = multiline
    ? el("textarea", { rows: 2, maxlength: TEXT_MAX, placeholder })
    : el("input", { type: "text", maxlength: TEXT_MAX, placeholder });
  input.value = obj[key];
  input.addEventListener("input", () => { obj[key] = input.value; changed(); });
  return el("label", { class: "field" }, el("span", { text: label }), input);
}

function textInput(obj, key, label) {
  const input = el("input", { type: "text", maxlength: TEXT_MAX, "aria-label": label, placeholder: label });
  input.value = obj[key];
  input.addEventListener("input", () => { obj[key] = input.value; changed(); });
  return input;
}

function choice(options, value, label, onChange) {
  const select = el("select", { "aria-label": label }, options.map(([v, text]) => el("option", { value: v, text })));
  select.value = value;
  select.addEventListener("change", () => onChange(select.value));
  return select;
}

// Chips for a list of ids, each removable, and a menu that adds one of `options`.
function chipList(title, ids, label, options, onChange) {
  const list = el("div", { class: "chips" });
  for (const id of ids) {
    list.append(el("span", { class: "chip" }, label(id),
      el("button", { type: "button", class: "remove", "aria-label": `Remove ${label(id)}`, text: "×", onclick: () => onChange(ids.filter((x) => x !== id)) })));
  }
  if (options.length) list.append(choice([["", "Add…"], ...options], "", `Add to ${title}`, (v) => { if (v) onChange([...ids, v]); }));
  return el("div", { class: "field" }, el("span", { text: title }), list);
}

function render() {
  for (const b of document.querySelectorAll("[data-tab]")) b.setAttribute("aria-selected", String(b.dataset.tab === state.tab));
  const main = $("main");
  if (!state.doc) {
    main.replaceChildren(state.problems ? renderProblems(state.problems) : intro);
  } else if (state.tab === "pairs") {
    main.replaceChildren(renderPairs());
  } else if (state.tab === "cards") {
    main.replaceChildren(renderCards());
  } else {
    main.replaceChildren(renderReferences());
  }
  updateBar();
}

function renderPairs() {
  const doc = state.doc;
  const usable = doc.references.filter((r) => !r.missing);
  const wrap = el("section", { class: "list" }, el("div", { class: "tools" },
    el("button", { type: "button", text: "New pair", disabled: usable.length < 2, onclick: newPair }),
    el("span", { class: "hint", text: "Each pair shows the liked image first. Write what differs in how they look, not in what they show." })));
  if (!doc.pairs.length) {
    wrap.append(el("p", { class: "empty", text: usable.length < 2
      ? "This folder has fewer than two images."
      : "No pairs yet. Name files like 01-like-….jpg and 01-dislike-….png and open the folder again, or add a pair by hand." }));
  }
  for (const pair of doc.pairs) wrap.append(renderPair(pair));
  return wrap;
}

function newPair() {
  const usable = state.doc.references.filter((r) => !r.missing);
  if (usable.length < 2) return;
  const pair = { id: nextId(state.doc, "pairs"), liked: usable[0].id, disliked: usable[1].id, property: "", feeling: "", notes: "", kinds: [], connections: [] };
  state.doc = { ...state.doc, pairs: [...state.doc.pairs, pair] };
  changed();
  render();
}

function renderPair(pair) {
  const doc = state.doc;
  const side = (key) => {
    const other = key === "liked" ? pair.disliked : pair.liked;
    const options = doc.references.filter((r) => r.id !== other).map((r) => [r.id, refLabel(r)]);
    return el("figure", { class: `side ${key}` }, thumb(refById(pair[key])),
      el("figcaption", {}, el("span", { class: "role", text: key === "liked" ? "Liked" : "Disliked" }),
        choice(options, pair[key], key === "liked" ? "Liked image" : "Disliked image", (id) => {
          pair[key] = id;
          pair.connections = pair.connections.filter((c) => c !== pair.liked && c !== pair.disliked);
          changed();
          render();
        })));
  };
  const kinds = el("fieldset", { class: "kinds" }, el("legend", { text: "What should transfer" }),
    ATTRIBUTE_KINDS.map((kind) => {
      const box = el("input", { type: "checkbox", checked: pair.kinds.includes(kind) });
      box.addEventListener("change", () => {
        pair.kinds = ATTRIBUTE_KINDS.filter((k) => (k === kind ? box.checked : pair.kinds.includes(k)));
        changed();
      });
      return el("label", { class: "chip" }, box, ` ${kind}`);
    }));
  const others = doc.references.filter((r) => r.id !== pair.liked && r.id !== pair.disliked && !pair.connections.includes(r.id));
  const connections = chipList("Connections to other references", pair.connections, (id) => refLabel(refById(id)),
    others.map((r) => [r.id, refLabel(r)]), (ids) => { pair.connections = ids; changed(); render(); });
  return el("article", { class: "pair", id: pair.id },
    el("header", {}, el("h2", { text: pair.id }), el("div", { class: "actions" },
      el("button", { type: "button", text: "Make nexus card", onclick: () => {
        state.doc = cardFromPair(state.doc, pair.id).doc;
        state.tab = "cards";
        changed();
        render();
      } }),
      el("button", { type: "button", class: "danger", text: "Delete pair", onclick: () => {
        if (!confirm(`Delete pair ${pair.id}? Cards keep their other links.`)) return;
        state.doc = removePair(state.doc, pair.id);
        changed();
        render();
      } }))),
    el("div", { class: "sides" }, side("liked"), side("disliked")),
    textField("Property: the specific visual property you mean", pair, "property"),
    textField("Feeling: the feeling you want", pair, "feeling"),
    kinds, connections,
    textField("Notes", pair, "notes", { multiline: true }),
    renderDifferences(refById(pair.liked), refById(pair.disliked)));
}

function renderDifferences(liked, disliked) {
  const box = el("div", { class: "measured" }, el("h3", { text: "Measured on this computer" }));
  if (!liked || !disliked || !liked.attributes || !disliked.attributes) {
    box.append(el("p", { class: "hint", text: "One of the images has no measurement." }));
    return box;
  }
  let result;
  try {
    result = compareImages(liked.attributes, disliked.attributes);
  } catch {
    box.append(el("p", { class: "hint", text: "The measurements cannot be compared (another version)." }));
    return box;
  }
  if (!result.differences.length) {
    box.append(el("p", { class: "hint", text: "No clear difference in palette or structure; the property may be material, typography, rhythm or motion." }));
    return box;
  }
  box.append(el("ul", {}, result.differences.map((d) => el("li", {
    text: `${MEASURE_LABELS[d.measure] || d.measure}: liked ${d.direction} (${fmt(d.liked)} against ${fmt(d.disliked)})`,
  }))));
  if (result.hints.length) {
    box.append(el("p", { class: "hint", text: `Hints for the look parameters: ${result.hints.map((h) => `${PARAM_LABELS[h.param] || h.param} ${h.direction}`).join(", ")}. Only hints; your comparisons decide.` }));
  }
  return box;
}

function renderCards() {
  const wrap = el("section", { class: "list" }, el("div", { class: "tools" },
    el("button", { type: "button", text: "New card", onclick: newCard }),
    el("span", { class: "hint", text: "A card names an interest, its bridge to the project and the place it could take in the product." })));
  if (!state.doc.cards.length) wrap.append(el("p", { class: "empty", text: "No nexus cards yet. Make one from a pair, or start an empty card." }));
  for (const card of state.doc.cards) wrap.append(renderCard(card));
  return wrap;
}

function newCard() {
  const card = {
    id: nextId(state.doc, "cards"), interest: "", bridge: { kind: "other", text: "" }, place: { kind: "other", text: "" },
    refs: [], pairs: [], g2: "", g3: [], h01: "", status: "draft", author: "owner",
  };
  state.doc = { ...state.doc, cards: [...state.doc.cards, card] };
  changed();
  render();
}

function renderCard(card) {
  const doc = state.doc;
  const kindRow = (obj, kinds, labels, title, placeholder) => el("div", { class: "field" }, el("span", { text: title }),
    el("div", { class: "row" },
      choice(kinds.map((k) => [k, labels[k]]), obj.kind, `${title}: kind`, (v) => { obj.kind = v; changed(); }),
      textInput(obj, "text", placeholder)));
  const confirmed = el("input", { type: "checkbox", checked: card.status === "confirmed" });
  confirmed.addEventListener("change", () => { card.status = confirmed.checked ? "confirmed" : "draft"; changed(); render(); });
  return el("article", { class: `card ${card.status}`, id: card.id },
    el("header", {}, el("h2", { text: card.author === "agent" ? `${card.id} (drafted by an agent)` : card.id }),
      el("div", { class: "actions" }, el("label", { class: "chip" }, confirmed, " Confirmed"),
        el("button", { type: "button", class: "danger", text: "Delete card", onclick: () => {
          if (!confirm(`Delete card ${card.id}?`)) return;
          state.doc = removeCard(state.doc, card.id);
          changed();
          render();
        } }))),
    card.refs.length ? el("div", { class: "strip" }, card.refs.map((id) => thumb(refById(id), "mini"))) : null,
    textField("Interest", card, "interest"),
    kindRow(card.bridge, BRIDGES, BRIDGE_LABELS, "Bridge to the project", "How it connects"),
    kindRow(card.place, PLACES, PLACE_LABELS, "Place in the product", "Where it could appear"),
    chipList("References", card.refs, (id) => refLabel(refById(id)),
      doc.references.filter((r) => !card.refs.includes(r.id)).map((r) => [r.id, refLabel(r)]),
      (ids) => { card.refs = ids; changed(); render(); }),
    chipList("Pairs", card.pairs, (id) => id,
      doc.pairs.filter((p) => !card.pairs.includes(p.id)).map((p) => [p.id, `${p.id}: ${refLabel(refById(p.liked))}`]),
      (ids) => { card.pairs = ids; changed(); render(); }),
    textField("G2: manifesto choice", card, "g2"),
    renderG3(card),
    textField("H-01: visual feature", card, "h01"));
}

function renderG3(card) {
  const rows = el("div", { class: "g3" });
  card.g3.forEach((entry, i) => {
    rows.append(el("div", { class: "row" }, el("span", { class: "param", text: PARAM_LABELS[entry.param] || entry.param }),
      choice(DIRECTIONS.map((d) => [d, d]), entry.direction, `${entry.param} direction`, (v) => { entry.direction = v; changed(); }),
      el("button", { type: "button", class: "remove", text: "×", "aria-label": `Remove ${entry.param}`, onclick: () => {
        card.g3 = card.g3.filter((_, j) => j !== i);
        changed();
        render();
      } })));
  });
  const unused = PARAMS.filter((p) => !card.g3.some((e) => e.param === p));
  if (unused.length) {
    rows.append(choice([["", "Add parameter…"], ...unused.map((p) => [p, PARAM_LABELS[p] || p])], "", "Add a G3 parameter", (v) => {
      if (!v) return;
      card.g3 = [...card.g3, { param: v, direction: "higher" }];
      changed();
      render();
    }));
  }
  return el("div", { class: "field" }, el("span", { text: "G3: look parameters" }), rows);
}

function renderReferences() {
  const wrap = el("section", { class: "grid" });
  if (!state.doc.references.length) wrap.append(el("p", { class: "empty", text: "No images in this folder." }));
  for (const ref of state.doc.references) {
    wrap.append(el("article", { class: ref.missing ? "ref missing" : "ref", id: ref.id },
      thumb(ref), el("h2", { text: refLabel(ref) }),
      textField("Subject: what it shows", ref, "subject"),
      textField("Source: where it is from", ref, "source"),
      textField("Notes", ref, "notes", { multiline: true }),
      renderAttributes(ref.attributes)));
  }
  return wrap;
}

function renderAttributes(a) {
  if (!a) return el("p", { class: "hint", text: "Not measured: the image could not be read." });
  if (a.version !== 1) return el("p", { class: "hint", text: "Measured with another version." });
  try {
    const rows = [
      ["Lightness (10, 50, 90%)", `${fmt(a.lightness.p10)}, ${fmt(a.lightness.p50)}, ${fmt(a.lightness.p90)}`],
      ["Contrast", fmt(a.contrast)],
      ["Chroma (50, 90%)", `${fmt(a.chroma.p50)}, ${fmt(a.chroma.p90)}`],
      ["Coloured share", pct(a.chromaticShare)],
      ["Hue mean, spread", a.hueMean === null ? "-" : `${Math.round(a.hueMean)}°, ${Math.round(a.hueSpread)}°`],
      ["Number of hues", String(a.hueCount)],
      ["Warm share", pct(a.warmShare)],
      ["Background lightness", a.background ? fmt(a.background.L) : "-"],
      ["Edge density", fmt(a.edgeDensity)],
      ["Symmetry (left-right, top-bottom, half-turn)", `${fmt(a.symmetry.mirrorX)}, ${fmt(a.symmetry.mirrorY)}, ${fmt(a.symmetry.rot180)}`],
      ["Directional structure", a.orientation.angle === null ? fmt(a.orientation.coherence) : `${fmt(a.orientation.coherence)} at ${Math.round(a.orientation.angle)}°`],
    ];
    return el("table", { class: "attrs" }, el("tbody", {}, rows.map(([k, v]) => el("tr", {}, el("th", { text: k }), el("td", { text: v })))));
  } catch {
    return el("p", { class: "hint", text: "Measured with another version." });
  }
}

$("open").addEventListener("click", openFolder);
$("fallback").addEventListener("change", openFiles);
$("save").addEventListener("click", save);
$("copy").addEventListener("click", copySummary);
for (const b of document.querySelectorAll("[data-tab]")) b.addEventListener("click", () => { state.tab = b.dataset.tab; render(); });
document.addEventListener("keydown", (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
    e.preventDefault();
    save();
  }
});
window.addEventListener("beforeunload", (e) => {
  if (state.dirty) e.preventDefault();
});
if (typeof window.showDirectoryPicker !== "function") {
  status("This browser cannot open folders: Open folder picks files instead, and Save downloads references.json. Chrome and Edge open folders.");
}
render();
