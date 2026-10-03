// Taste Lab page: shows two looks, records the owner's choice in the
// artifact's owner-only store, and asks the model worker for the next pair.
// Each theme family is learned on its own (plan section 4.5).
import { buildGeometry, properColouring } from "./geometry.js";
import { palette, background, defaultSpace, validateSpace, familiesOf, SCENES } from "../core/space.js";
import { Preview } from "./preview.js";
import { COST_HEAVY } from "./presets.js";

const $ = (id) => document.getElementById(id);
const use = (name) => (window.claude && window.claude.use ? window.claude.use(name) : Promise.resolve(null));
const LABELS = {
  hueRotation: "Hue rotation", hueSpread: "Hue spread", lightness: "Lightness", lightnessAlt: "Lightness alternation",
  chroma: "Chroma", classes: "Colour classes", bgLightness: "Background lightness", bgHue: "Background tint hue",
  bgTint: "Background tint", gap: "Cell gap", edgeWeight: "Edge weight", edgeBrightness: "Edge brightness",
  gloss: "Gloss", glow: "Glow", fog: "Fog", turnMs: "Twist duration", easeA: "Easing start", easeB: "Easing end",
};

const geo = buildGeometry();
const session = Math.random().toString(36).slice(2, 10);
const PAGE = 500; // documents per read; the store answers at most 1000 per query
const state = { scene: "solving", family: null, records: [], ids: [], sessions: [], models: {}, pair: null, busy: true, db: null, space: defaultSpace(), turnCell: 0 };
const colourings = new Map();

function cellColours(look) {
  const k = look.classes;
  if (!colourings.has(k)) colourings.set(k, properColouring(20, geo.ringAdjacency, k));
  const colours = colourings.get(k);
  const pal = palette(look);
  const out = new Float32Array(600 * 3);
  for (let c = 0; c < 600; c++) {
    const rgb = pal[colours[geo.ringOf[c]]].mapped.srgb;
    out.set(rgb, 3 * c);
  }
  return out;
}

let previews = [];
function showPair(msg) {
  state.pair = msg;
  const turnCell = Math.floor(Math.random() * 600);
  [msg.lookA, msg.lookB].forEach((look, i) => {
    previews[i].setTurnCell(turnCell);
    previews[i].setLook(look, cellColours(look), background(look).srgb);
  });
  $("count").textContent = String(msg.counts.answers);
  $("countWord").textContent = msg.counts.answers === 1 ? "answer" : "answers";
  $("countFamily").textContent = familiesOf(state.space).length > 1 ? `for ${familyName(msg.family)}` : "";
  $("kind").textContent = msg.kind === "mi" ? "Testing the current favourite" : "Exploring";
  // The settled test's expected regret, relative to the predicted range (plan section 5.6).
  $("settled").textContent = msg.settled ? "Settled for this scene"
    : msg.sessionSettled ? "Settled in this session; confirm in another session"
      : `Not settled: expected room for improvement ${Math.round(100 * msg.settledRegret)}%`;
  const notes = [];
  if (msg.counts.pruned) notes.push(`${msg.counts.pruned} older answers are outside the model's 400-look window.`);
  if (msg.counts.skipped) notes.push(`${msg.counts.skipped} answers do not fit the current parameter table and are not used.`);
  $("pruned").hidden = !notes.length;
  $("pruned").textContent = notes.join(" ");
  recordSettled(msg);
  renderRelevance(msg.relevance, msg.counts.answers, msg.fitted);
  if (!state.statusSticky) $("status").textContent = "";
  setBusy(false);
}

// Relevance of the shown family's model; `answers` counts that family's answers.
// The ranking is provisional until 160 answers (plan section 6). It waits for
// length scales from a finished hyperparameter search, in this view or stored by
// an earlier one: before that, every parameter has the starting length scale and
// the order means nothing.
const RANKING_AT = 160;
function renderRelevance(rel, answers, fitted) {
  const list = $("matters");
  list.replaceChildren();
  const note = (text) => {
    const li = document.createElement("li");
    li.className = "note";
    li.textContent = text;
    list.append(li);
  };
  if (answers < 10) return note(`Appears after 10 answers (${answers} so far).`);
  if (!fitted) return note(`Appears once the model is fitted (${answers} answers so far).`);
  if (answers < RANKING_AT) note(`Provisional until ${RANKING_AT} answers (${answers} so far).`);
  const max = Math.max(...rel.map((r) => r.value), 1e-9);
  for (const r of [...rel].sort((a, b) => b.value - a.value).slice(0, 8)) {
    const li = document.createElement("li");
    const name = document.createElement("span");
    name.textContent = LABELS[r.id] || r.id;
    const bar = document.createElement("span");
    bar.className = "bar";
    bar.style.setProperty("--w", `${Math.round((100 * r.value) / max)}%`);
    li.append(name, bar);
    list.append(li);
  }
}

function setBusy(b) {
  state.busy = b;
  document.body.classList.toggle("busy", b);
}

const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });
let presetRequest = null;
worker.onmessage = (e) => {
  const m = e.data;
  const forPresets = presetRequest && m.id === presetRequest.id;
  if (m.type === "pair") showPair(m);
  else if (m.type === "nopair") { $("status").textContent = m.reason; setBusy(false); }
  else if (m.type === "model") saveModel(m);
  else if (m.type === "presets" && forPresets) { presetRequest.resolve(m); presetRequest = null; }
  else if (m.type === "error" && forPresets) { presetRequest.reject(new Error(m.message)); presetRequest = null; }
  else if (m.type === "error") { $("status").textContent = "The model stopped: " + m.message; setBusy(false); }
};

async function save(record) {
  if (!state.db) return null;
  const id = `${Date.now().toString(36)}-${session}`;
  try {
    await state.db.doc(`comparisons/${id}`).set(record);
    return id;
  } catch (err) {
    $("status").textContent = `This answer was not saved (${err.code || "error"}). Answers continue in this tab only.`;
    return null;
  }
}

// Each finished hyperparameter search is stored in model/<family>, and the next
// load or undo starts from it (plan section 7). A failed write only costs that
// faster start, so it is not reported.
let modelWrite = Promise.resolve();
function saveModel(m) {
  state.models[m.family] = m.doc;
  if (!state.db) return;
  modelWrite = modelWrite.then(() => state.db.doc(`model/${m.family}`).set(m.doc)).catch(() => {});
}

// A session that settles stores its best look per family and scene in
// sessions/<id>; another session that settles in the same region confirms it
// (plan section 5.6).
const sessionDoc = { started: new Date().toISOString(), settled: {} };
let sessionWrite = Promise.resolve();
function recordSettled(msg) {
  if (!state.db || !msg.sessionSettled || !msg.best) return;
  const byScene = sessionDoc.settled[msg.family] || (sessionDoc.settled[msg.family] = {});
  const entry = byScene[msg.scene];
  if (entry && JSON.stringify(entry.best) === JSON.stringify(msg.best)) return;
  byScene[msg.scene] = { best: msg.best, t: new Date().toISOString() };
  const body = JSON.parse(JSON.stringify(sessionDoc));
  state.sessions = state.sessions.filter((s) => s.id !== session).concat([{ id: session, ...body }]);
  sessionWrite = sessionWrite.then(() => state.db.doc(`sessions/${session}`).set(body))
    .catch(() => { delete byScene[msg.scene]; });
}

// Settled bests as { family: { scene: entry } }; a session stored before theme
// families has { scene: entry }, and its entries belong to the first family.
function settledRecords() {
  const out = [];
  for (const s of state.sessions) {
    for (const [key, value] of Object.entries(s.settled || {})) {
      if (!value || typeof value !== "object") continue;
      if ("best" in value) out.push({ scene: key, session: s.id, best: value.best });
      else for (const [scene, e] of Object.entries(value)) out.push({ family: key, scene, session: s.id, best: e && e.best });
    }
  }
  return out;
}

function init() {
  worker.postMessage({
    type: "init", space: state.space, records: state.records, settled: settledRecords(), models: state.models,
    session, scene: state.scene, family: state.family, seed: Date.now(),
  });
}

async function answer(kind) {
  if (state.busy || !state.pair) return;
  setBusy(true);
  closeReason();
  const record = { lookA: state.pair.lookA, lookB: state.pair.lookB, family: state.family, scene: state.scene, answer: kind, t: new Date().toISOString(), session };
  const id = await save(record);
  state.records.push(record);
  state.ids.push(id);
  worker.postMessage({ type: "answer", record, scene: state.scene, family: state.family });
}

// The optional one-line reason for the last answer (plan section 7). It is
// stored with that answer in the owner-only store and exported with it.
let reasonWrite = Promise.resolve();
function openReason() {
  if (state.busy) return;
  if (!state.records.length) { $("status").textContent = "Answer a pair first; the reason belongs to the last answer."; return; }
  $("reason").value = state.records[state.records.length - 1].reason || "";
  $("reasonForm").hidden = false;
  $("reason").focus();
}

function closeReason() {
  $("reasonForm").hidden = true;
}

function saveReason() {
  const i = state.records.length - 1;
  if (i >= 0) {
    const record = state.records[i];
    const text = $("reason").value.trim().slice(0, 140);
    if (text) record.reason = text;
    else delete record.reason;
    const id = state.ids[i];
    if (id && state.db) {
      const body = JSON.parse(JSON.stringify(record));
      reasonWrite = reasonWrite.then(() => state.db.doc(`comparisons/${id}`).set(body))
        .catch((err) => { $("status").textContent = `The reason was not saved (${err.code || "error"}).`; });
    }
  }
  closeReason();
}

async function undo() {
  if (state.busy || !state.records.length) return;
  setBusy(true);
  closeReason();
  state.records.pop();
  const id = state.ids.pop();
  if (id && state.db) {
    try { await state.db.doc(`comparisons/${id}`).delete(); } catch (err) { $("status").textContent = "The last answer could not be removed from storage."; }
  }
  init();
}

function setScene(scene) {
  if (state.busy || scene === state.scene) return;
  state.scene = scene;
  for (const b of document.querySelectorAll("[data-scene]")) b.setAttribute("aria-pressed", String(b.dataset.scene === scene));
  setBusy(true);
  worker.postMessage({ type: "next", scene, family: state.family });
}

const familyName = (id) => (familiesOf(state.space).find((f) => f.id === id) || { name: id }).name;

// One button per theme family of the table; F shows the next family.
function renderFamilies() {
  const families = familiesOf(state.space);
  const group = $("families");
  group.replaceChildren();
  group.hidden = families.length < 2;
  for (const f of families) {
    const b = document.createElement("button");
    b.type = "button";
    b.dataset.family = f.id;
    b.setAttribute("aria-pressed", String(f.id === state.family));
    b.textContent = f.name;
    b.addEventListener("click", () => setFamily(f.id));
    group.append(b);
  }
  const hint = document.createElement("kbd");
  hint.textContent = "F";
  hint.title = "Next family";
  group.append(hint);
}

function setFamily(family) {
  if (state.busy || family === state.family) return;
  closeReason();
  state.family = family;
  for (const b of document.querySelectorAll("[data-family]")) b.setAttribute("aria-pressed", String(b.dataset.family === family));
  setBusy(true);
  worker.postMessage({ type: "next", scene: state.scene, family });
}

function nextFamily() {
  const ids = familiesOf(state.space).map((f) => f.id);
  if (ids.length > 1) setFamily(ids[(ids.indexOf(state.family) + 1) % ids.length]);
}

// The starting points of every family and scene, computed by the worker.
function requestPresets() {
  return new Promise((resolve, reject) => {
    const id = `presets-${Date.now().toString(36)}`;
    presetRequest = { id, resolve, reject };
    worker.postMessage({ type: "presets", id, settled: settledRecords() });
  });
}

async function exportData() {
  if (state.exporting) return;
  const downloads = await use("downloads");
  if (!downloads) { $("status").textContent = "Export is not available in this view."; return; }
  state.exporting = true;
  $("status").textContent = "Preparing the export.";
  try {
    const result = await requestPresets();
    const data = JSON.stringify({
      kind: "tastelab-export",
      version: 2,
      space: state.space,
      families: familiesOf(state.space),
      comparisons: state.records,
      sessions: state.sessions,
      presets: result.presets,
      shared: result.shared,
      costHeavy: {
        params: COST_HEAVY,
        note: "Parameters whose frame-time cost H-06 measures before a preset reaches G3. Taste Lab measures no performance; this export makes no performance claim.",
      },
    }, null, 2);
    await downloads.save({ filename: "tastelab-export.json", data });
    $("status").textContent = "Exported.";
  } catch (err) {
    if (err.code !== "declined") $("status").textContent = err.code ? `Export failed (${err.code}).` : `Export failed: ${err.message}`;
  } finally {
    state.exporting = false;
  }
}

document.addEventListener("keydown", (e) => {
  if (e.target.closest && e.target.closest("input, textarea, select")) return;
  const k = e.key.toLowerCase();
  if (k === "a") answer("A");
  else if (k === "b") answer("B");
  else if (k === "s") answer("same");
  else if (k === "x") answer("bad");
  else if (k === "z") undo();
  else if (k === "w") openReason();
  else if (k === "f") nextFamily();
  else if (k === "1" || k === "2" || k === "3") setScene(SCENES[Number(k) - 1]);
  else return;
  e.preventDefault();
});
for (const [id, kind] of [["pickA", "A"], ["pickB", "B"], ["same", "same"], ["bad", "bad"]]) $(id).addEventListener("click", () => answer(kind));
$("undo").addEventListener("click", undo);
$("why").addEventListener("click", openReason);
$("reasonForm").addEventListener("submit", (e) => { e.preventDefault(); saveReason(); });
$("reason").addEventListener("keydown", (e) => { if (e.key === "Escape") { e.preventDefault(); closeReason(); } });
$("export").addEventListener("click", exportData);
for (const b of document.querySelectorAll("[data-scene]")) b.addEventListener("click", () => setScene(b.dataset.scene));

// Every stored comparison in answer order, read in pages by time stamp.
async function loadRecords(db) {
  const records = [], ids = [], seen = new Set();
  let last = null;
  for (;;) {
    let query = db.collection("comparisons").orderBy("t").limit(PAGE);
    if (last !== null) query = query.where("t", ">=", last);
    const snap = await query.get();
    let added = 0;
    for (const d of snap.docs) {
      if (seen.has(d.id)) continue;
      seen.add(d.id);
      records.push(d.data());
      ids.push(d.id);
      added++;
    }
    if (snap.size < PAGE || !added) break;
    last = snap.docs[snap.docs.length - 1].data().t;
  }
  return { records, ids };
}

// The stored hyperparameters by family; the worker checks each against the table.
async function loadModels(db) {
  const snap = await db.collection("model").limit(PAGE).get();
  return Object.fromEntries(snap.docs.map((d) => [d.id, d.data()]));
}

async function loadSessions(db) {
  const snap = await db.collection("sessions").orderBy("started", "desc").limit(PAGE).get();
  return snap.docs.map((d) => ({ id: d.id, ...d.data() }));
}

// A stored table from before theme families takes the default families and scene limits.
function withDefaults(space) {
  const d = defaultSpace();
  return { ...space, families: space.families ?? d.families, sceneLimits: space.sceneLimits ?? d.sceneLimits };
}

// A stored parameter table is used only when it is valid and names every parameter the page draws.
function usableSpace(space) {
  try {
    validateSpace(space);
    const ids = new Set(space.params.map((p) => p.id));
    return defaultSpace().params.every((p) => ids.has(p.id));
  } catch {
    return false;
  }
}

async function boot() {
  setBusy(true);
  try {
    previews = [new Preview($("viewA"), geo), new Preview($("viewB"), geo)];
    previews.forEach((p) => p.start());
  } catch (err) {
    $("status").textContent = err.message;
    return;
  }
  const user = await use("user");
  const db = await use("db");
  const owner = user ? await user.isOwner() : null;
  if (db && owner === false) {
    $("status").textContent = "Only the owner of this page can record answers.";
    state.statusSticky = true;
  } else if (db) {
    state.db = db;
    try {
      const sp = await db.doc("space/current").get();
      if (sp.exists) {
        const stored = sp.data();
        const usable = [withDefaults(stored), stored].find(usableSpace);
        if (usable) state.space = usable;
        else { $("status").textContent = "The stored parameter table is not usable; the default table is used."; state.statusSticky = true; }
      }
      const loaded = await loadRecords(db);
      state.records = loaded.records;
      state.ids = loaded.ids;
      state.sessions = await loadSessions(db);
      try { state.models = await loadModels(db); } catch { /* the first search starts from the starting values */ }
    } catch (err) {
      $("status").textContent = `Stored answers could not be read (${err.code || "error"}).`;
    }
  } else {
    $("status").textContent = "Storage is not available here; answers stay in this tab.";
    state.statusSticky = true;
  }
  state.family = familiesOf(state.space)[0].id;
  renderFamilies();
  init();
}

boot();
