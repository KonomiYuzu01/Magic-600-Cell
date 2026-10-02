// Taste Lab page: shows two looks, records the owner's choice in the
// artifact's owner-only store, and asks the model worker for the next pair.
import { buildGeometry, properColouring } from "./geometry.js";
import { palette, background, defaultSpace, SCENES } from "../core/space.js";
import { Preview } from "./preview.js";

const $ = (id) => document.getElementById(id);
const use = (name) => (window.claude && window.claude.use ? window.claude.use(name) : Promise.resolve(null));
const LABELS = {
  hueRotation: "Hue rotation", hueSpread: "Hue spread", lightness: "Lightness", lightnessAlt: "Lightness alternation",
  chroma: "Chroma", classes: "Colour classes", bgLightness: "Background lightness", bgHue: "Background tint hue",
  bgTint: "Background tint", gap: "Sticker gap", edgeWeight: "Edge weight", edgeBrightness: "Edge brightness",
  gloss: "Gloss", glow: "Glow", fog: "Fog", turnMs: "Turn duration", easeA: "Easing start", easeB: "Easing end",
};

const geo = buildGeometry();
const session = Math.random().toString(36).slice(2, 10);
const state = { scene: "solving", records: [], ids: [], pair: null, busy: true, db: null, space: defaultSpace(), turnCell: 0 };
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
  $("kind").textContent = msg.kind === "info" ? "Testing a question" : msg.kind === "repeat" ? "Checking consistency" : "Exploring";
  $("settled").textContent = msg.settled ? "Settled for this scene" : `Confidence ${Math.round(100 * msg.settledProb)}%`;
  $("pruned").hidden = !msg.counts.pruned;
  $("pruned").textContent = `${msg.counts.pruned} older answers are outside the model's 400-look window.`;
  renderRelevance(msg.relevance);
  if (!state.statusSticky) $("status").textContent = "";
  setBusy(false);
}

function renderRelevance(rel) {
  const list = $("matters");
  list.replaceChildren();
  if (state.records.length < 10) {
    const li = document.createElement("li");
    li.textContent = `Appears after 10 answers (${state.records.length} so far).`;
    list.append(li);
    return;
  }
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
worker.onmessage = (e) => {
  const m = e.data;
  if (m.type === "pair") showPair(m);
  else if (m.type === "nopair") { $("status").textContent = m.reason; setBusy(false); }
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

async function answer(kind) {
  if (state.busy || !state.pair) return;
  setBusy(true);
  const record = { lookA: state.pair.lookA, lookB: state.pair.lookB, scene: state.scene, answer: kind, t: new Date().toISOString(), session };
  const id = await save(record);
  state.records.push(record);
  state.ids.push(id);
  worker.postMessage({ type: "answer", record, scene: state.scene });
}

async function undo() {
  if (state.busy || !state.records.length) return;
  setBusy(true);
  state.records.pop();
  const id = state.ids.pop();
  if (id && state.db) {
    try { await state.db.doc(`comparisons/${id}`).delete(); } catch (err) { $("status").textContent = "The last answer could not be removed from storage."; }
  }
  worker.postMessage({ type: "init", space: state.space, records: state.records, scene: state.scene, seed: Date.now() });
}

function setScene(scene) {
  if (state.busy || scene === state.scene) return;
  state.scene = scene;
  for (const b of document.querySelectorAll("[data-scene]")) b.setAttribute("aria-pressed", String(b.dataset.scene === scene));
  setBusy(true);
  worker.postMessage({ type: "next", scene });
}

async function exportData() {
  const downloads = await use("downloads");
  const data = JSON.stringify({ kind: "tastelab-export", version: 1, space: state.space, comparisons: state.records, best: state.pair && state.pair.best }, null, 2);
  if (!downloads) { $("status").textContent = "Export is not available in this view."; return; }
  try { await downloads.save({ filename: "tastelab-export.json", data }); $("status").textContent = "Exported."; }
  catch (err) { if (err.code !== "declined") $("status").textContent = `Export failed (${err.code}).`; }
}

document.addEventListener("keydown", (e) => {
  if (e.target.closest && e.target.closest("input, textarea, select")) return;
  const k = e.key.toLowerCase();
  if (k === "a") answer("A");
  else if (k === "b") answer("B");
  else if (k === "s") answer("same");
  else if (k === "x") answer("bad");
  else if (k === "z") undo();
  else if (k === "1" || k === "2" || k === "3") setScene(SCENES[Number(k) - 1]);
  else return;
  e.preventDefault();
});
for (const [id, kind] of [["pickA", "A"], ["pickB", "B"], ["same", "same"], ["bad", "bad"]]) $(id).addEventListener("click", () => answer(kind));
$("undo").addEventListener("click", undo);
$("export").addEventListener("click", exportData);
for (const b of document.querySelectorAll("[data-scene]")) b.addEventListener("click", () => setScene(b.dataset.scene));

async function loadRecords(db) {
  const out = [];
  const snap = await db.collection("comparisons").orderBy("t").get();
  snap.docs.forEach((d) => { out.push(d.data()); state.ids.push(d.id); });
  return out;
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
  if (db && user && user.isOwner && !user.isOwner()) {
    $("status").textContent = "Only the owner of this page can record answers.";
    state.statusSticky = true;
  } else if (db) {
    state.db = db;
    try {
      const sp = await db.doc("space/current").get();
      if (sp.exists) state.space = sp.data();
      state.records = await loadRecords(db);
    } catch (err) {
      $("status").textContent = `Stored answers could not be read (${err.code || "error"}).`;
    }
  } else {
    $("status").textContent = "Storage is not available here; answers stay in this tab.";
    state.statusSticky = true;
  }
  worker.postMessage({ type: "init", space: state.space, records: state.records, scene: state.scene, seed: Date.now() });
}

boot();
