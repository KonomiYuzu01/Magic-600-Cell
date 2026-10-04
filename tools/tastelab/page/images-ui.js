import { readSelection, openBundle, loadCached, closeBundle, mergeBundles } from "./library.js";
import { isReady } from "./images.js";
import { createIdbCache } from "./idb-cache.js";

export const pairKey = (likedImageId, dislikedImageId) => `${likedImageId}_${dislikedImageId}`;
// The ratings importer (tools/tastelab/fetch.py) takes a rating note of 1 to 140
// characters or null and a pair note of 1 to 280, without control characters.
export const cleanNote = (text) => text.replace(/\s+/gu, " ").trim();
export function validateNote(note, kind = "rating") {
  return typeof note === "string" && note.length <= (kind === "pair" ? 280 : 140) && !/\p{Cc}/u.test(note)
    && (kind !== "pair" || note.trim().length > 0);
}

// An empty rating note is stored and exported as null.
export function ratingDocument(record) {
  const { imageId, verdict, note, ratedAt } = record;
  return { path: `imageRatings/${imageId}`, body: { imageId, verdict, note: note || null, ratedAt } };
}

export function pairDocument(record) {
  const { likedImageId, dislikedImageId, note, notedAt } = record;
  return { path: `imagePairs/${pairKey(likedImageId, dislikedImageId)}`, body: { likedImageId, dislikedImageId, note, notedAt } };
}

export function createRatingState(records = []) {
  return { ratings: new Map(records.map((record) => [record.imageId, ratingDocument(record).body])), skips: new Map(), history: [], changes: new Map() };
}

// Only this tab's likes and dislikes enter its undo history. Null changes are
// deletion markers. Keep all changes since load, even after successful saves,
// so an export's storage read cannot replace what this tab currently shows.
export function reduceRating(state, action) {
  const ratings = new Map(state.ratings), skips = new Map(state.skips), changes = new Map(state.changes), history = [...state.history];
  if (action.type === "rate") {
    if (!["like", "dislike"].includes(action.verdict) || !validateNote(action.note ?? "")) throw new RangeError("Invalid image rating.");
    const record = ratingDocument(action).body;
    ratings.set(record.imageId, record);
    changes.set(record.imageId, record);
    skips.delete(record.imageId);
    history.push(record.imageId);
  } else if (action.type === "skip") {
    skips.set(action.imageId, ratings.size);
  } else if (action.type === "note") {
    if (!validateNote(action.note)) throw new RangeError("Image notes must have at most 140 characters.");
    if (!ratings.has(action.imageId)) return state;
    const previous = ratings.get(action.imageId);
    // ratedAt orders the record's last change, verdict or note, for the importer.
    // A stationary or backwards clock still advances a note edit by 1 ms.
    const ratedAt = new Date(Math.max(Date.parse(action.ratedAt), Date.parse(previous.ratedAt) + 1)).toISOString();
    const record = ratingDocument({ ...previous, note: action.note, ratedAt }).body;
    ratings.set(action.imageId, record);
    changes.set(action.imageId, record);
  } else if (action.type === "undo") {
    if (!history.length) return state;
    const imageId = history.pop();
    ratings.delete(imageId);
    changes.set(imageId, null);
  } else return state;
  return { ratings, skips, history, changes };
}

export function keyAction(event, tab) {
  if (event.repeat || event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || event.target?.isContentEditable
      || event.target?.closest?.("input, textarea, select, [contenteditable]")) return null;
  const key = event.key.toLowerCase();
  const keys = tab === "looks"
    ? { a: "A", b: "B", s: "same", x: "bad", z: "undo", w: "note", f: "family", 1: "scene-1", 2: "scene-2", 3: "scene-3" }
    : tab === "images" ? { arrowright: "like", arrowleft: "dislike", arrowdown: "skip", w: "note", z: "undo" } : {};
  return Object.hasOwn(keys, key) ? keys[key] : null;
}

function mergeRecords(records, changes, key, document) {
  const merged = new Map(records.map((record) => [key(record), document(record).body]));
  for (const [id, record] of changes) {
    if (record === null) merged.delete(id);
    else merged.set(id, document(record).body);
  }
  return [...merged.values()];
}

export function assembleImageExport(base, { bundles, ratings, pairs, ratingChanges = new Map(), pairChanges = new Map() }) {
  return {
    ...base, version: 3,
    images: {
      bundles: bundles.map((bundle) => ({ bundleId: bundle.id, items: bundle.items.length })),
      ratings: mergeRecords(ratings, ratingChanges, (record) => record.imageId, ratingDocument),
      pairs: mergeRecords(pairs, pairChanges, (record) => pairKey(record.likedImageId, record.dislikedImageId), pairDocument),
    },
  };
}

const PAGE = 500;
async function readPages(collection, field) {
  const records = [];
  let last = null;
  for (;;) {
    let query = collection.orderBy(field).limit(PAGE);
    if (last !== null) query = query.where(field, ">", last);
    const snap = await query.get();
    records.push(...snap.docs.map((doc) => doc.data()));
    if (snap.size < PAGE) return records;
    last = records.at(-1)[field];
  }
}

export async function loadImageDocuments(db, name) {
  const collection = db.collection(name);
  if (name === "imageRatings") return readPages(collection, "imageId");
  // A liked image can have more than 500 pairs. Finish the boundary image's
  // pairs separately before moving on; timestamp ties never lose documents.
  const pairs = new Map();
  let last = null;
  for (;;) {
    let query = collection.orderBy("likedImageId").limit(PAGE);
    if (last !== null) query = query.where("likedImageId", ">", last);
    const snap = await query.get();
    for (const doc of snap.docs) {
      const record = doc.data();
      pairs.set(pairKey(record.likedImageId, record.dislikedImageId), record);
    }
    if (snap.size < PAGE) return [...pairs.values()];
    last = snap.docs.at(-1).data().likedImageId;
    for (const record of await readPages(collection.where("likedImageId", "==", last), "dislikedImageId")) {
      pairs.set(pairKey(record.likedImageId, record.dislikedImageId), record);
    }
  }
}

export function createImagesUI({ setStatus }) {
  const $ = (id) => document.getElementById(id);
  const cache = createIdbCache(), files = new Map(), writes = new Map();
  let bundles = [], merged = mergeBundles([]), thumbnails = new Map();
  let ratingState = createRatingState(), pairs = new Map(), pairChanges = new Map();
  let db = null, loading = true, answersReady = false, readingAnswers = false, opening = false, working = false, current = null, revision = 0;
  let ratingUrls = [], pairUrls = [], noteImageId = null, shownPair = null, worker = null;

  const cacheUnavailable = () => { $("imageCacheNote").hidden = false; };
  const revoke = (urls) => { for (const url of urls) URL.revokeObjectURL(url); };
  function refuse(errors) {
    $("imageRefusals").replaceChildren();
    for (const error of errors) {
      const li = document.createElement("li");
      li.textContent = error;
      $("imageRefusals").append(li);
    }
  }

  function renderImage(imageId, urls) {
    const item = merged.items.get(imageId), bytes = thumbnails.get(imageId);
    const figure = document.createElement("figure"), image = document.createElement("img");
    const url = URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" }));
    urls.push(url);
    image.src = url;
    image.alt = item.title || "Image reference";
    const caption = document.createElement("figcaption");
    caption.className = "image-credit";
    for (const text of [item.title || "Untitled", item.credit]) {
      const line = document.createElement("span");
      line.textContent = text;
      caption.append(line);
    }
    for (const [text, href] of [[item.source, item.pageUrl], [item.licence, item.licenceUrl]]) {
      const link = document.createElement("a");
      link.textContent = text;
      link.href = href;
      link.target = "_blank";
      link.rel = "noopener noreferrer";
      caption.append(link);
    }
    figure.append(image, caption);
    return figure;
  }

  function controls() {
    for (const id of ["imageLike", "imageDislike", "imageSkip"]) $(id).disabled = loading || !answersReady || opening || working || !current;
    $("imageUndo").disabled = loading || !answersReady || !ratingState.history.length;
    $("imageNoteButton").disabled = loading || !answersReady || !ratingState.history.length;
    for (const id of ["imageNote", "imagePairNote", "imagePairSave"]) $(id).disabled = loading || !answersReady;
    $("imageAnswersRetry").hidden = !db || answersReady;
    $("imageAnswersRetry").disabled = loading || readingAnswers;
    $("bundleFolder").disabled = $("bundleFiles").disabled = loading || opening;
  }

  function refuseAnswerChange() {
    if (answersReady) return false;
    setStatus(readingAnswers ? "Stored image answers are being read. Rating is paused so they are not overwritten."
      : "Stored image answers could not be read. Rating is paused so they are not overwritten. Retry reading stored answers.");
    return true;
  }

  function progress() {
    const available = new Map(merged.order.filter((id) => ratingState.ratings.has(id)).map((id) => [id, ratingState.ratings.get(id).verdict]));
    $("imageLikes").textContent = String([...available.values()].filter((value) => value === "like").length);
    $("imageDislikes").textContent = String([...available.values()].filter((value) => value === "dislike").length);
    $("imageSkipped").textContent = String(merged.order.filter((id) => ratingState.skips.has(id)).length);
    $("imageModel").textContent = isReady(available) ? "Model ready" : "Model needs 5 likes and 5 dislikes.";
  }

  function closeNote() { $("imageNoteForm").hidden = true; noteImageId = null; }
  function closePair() {
    $("imagePairDetail").hidden = true;
    $("imagePairViews").replaceChildren();
    revoke(pairUrls); pairUrls = []; shownPair = null;
  }

  function renderPairs(suggestions) {
    $("imagePairs").replaceChildren();
    for (const pair of suggestions) {
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = `Like: ${merged.items.get(pair.likedImageId).title || "Untitled"} / Dislike: ${merged.items.get(pair.dislikedImageId).title || "Untitled"}`;
      button.addEventListener("click", () => {
        closePair();
        shownPair = pair;
        $("imagePairViews").append(renderImage(pair.likedImageId, pairUrls), renderImage(pair.dislikedImageId, pairUrls));
        $("imagePairNote").value = "";
        $("imagePairDetail").hidden = false;
        $("imagePairNote").focus();
      });
      $("imagePairs").append(button);
    }
    $("imagePairsHint").hidden = suggestions.length > 0;
  }

  function refresh() {
    merged = mergeBundles(bundles);
    thumbnails = new Map();
    for (const bundle of bundles) for (const item of bundle.items) {
      if (!thumbnails.has(item.imageId)) thumbnails.set(item.imageId, files.get(bundle.id).get(item.thumbSha256 + ".jpg"));
    }
    $("openBundles").replaceChildren();
    for (const bundle of bundles) {
      const li = document.createElement("li"), label = document.createElement("span"), button = document.createElement("button");
      label.textContent = `${bundle.id} · ${bundle.items.length} items`;
      button.type = "button";
      button.textContent = "Close";
      button.addEventListener("click", async () => {
        bundles = bundles.filter((open) => open.id !== bundle.id);
        files.delete(bundle.id);
        closePair();
        refresh();
        try { await closeBundle(bundle.id, cache); }
        catch { cacheUnavailable(); setStatus("Closed in this tab. The cache could not be cleared; this bundle may open again next time."); }
      });
      li.append(label, button);
      $("openBundles").append(li);
    }
    revoke(ratingUrls); ratingUrls = [];
    $("imageView").replaceChildren();
    current = null;
    progress();
    revision++;
    if (worker && merged.order.length && !loading) {
      working = true;
      $("imageEmpty").textContent = "Choosing the next image.";
      renderPairs([]);
      worker.postMessage({
        id: revision, order: merged.order, vectors: merged.vectors,
        ratings: new Map([...ratingState.ratings].map(([id, record]) => [id, record.verdict])),
        skips: ratingState.skips, ratedCount: ratingState.ratings.size,
        notedPairs: new Set([...pairs.values()].map((pair) => pair.likedImageId + "|" + pair.dislikedImageId)),
      });
    } else {
      working = false;
      $("imageEmpty").textContent = bundles.length ? "Image suggestions are unavailable." : "Open a bundle folder to start rating images.";
      renderPairs([]);
    }
    $("imageEmpty").hidden = false;
    controls();
  }

  try {
    worker = new Worker(new URL("./images-worker.js", import.meta.url), { type: "module" });
    worker.onmessage = (event) => {
      const message = event.data;
      if (message.id !== revision) return;
      working = false;
      if (message.type === "error") {
        $("imageEmpty").textContent = "The image model stopped.";
        setStatus("The image model stopped: " + message.message);
      } else {
        current = message.next;
        $("imageView").replaceChildren();
        revoke(ratingUrls); ratingUrls = [];
        if (current) $("imageView").append(renderImage(current, ratingUrls));
        $("imageEmpty").hidden = Boolean(current);
        $("imageEmpty").textContent = "No eligible images remain. Skipped images return after 50 further ratings; open another bundle to continue.";
        renderPairs(message.pairs);
      }
      controls();
    };
    worker.onerror = (event) => {
      event.preventDefault();
      worker = null;
      current = null; working = false;
      setStatus("Image suggestions could not run in this browser.");
      controls();
    };
  } catch { setStatus("Image suggestions could not run in this browser."); }

  // Serialize every write to a document, including a later undo. A failure
  // leaves its local change available to the worker and the export.
  function write(path, body) {
    const pending = (writes.get(path) || Promise.resolve()).then(async () => {
      if (!db) { setStatus("Storage is not available; this image answer stays in this tab."); return; }
      try {
        const doc = db.doc(path);
        if (body === null) await doc.delete();
        else await doc.set(body);
      } catch (error) {
        setStatus(`This image answer was not saved (${error.code || "error"}). It stays in this tab and will be included in the export.`);
      }
    });
    writes.set(path, pending);
  }

  function saveRating(imageId) {
    const record = ratingState.changes.get(imageId);
    const path = `imageRatings/${imageId}`;
    write(path, record);
  }

  function action(kind) {
    if (loading || refuseAnswerChange()) return;
    if (["like", "dislike", "skip"].includes(kind)) {
      if (working || opening || !current) return;
      closeNote();
      const imageId = current;
      ratingState = reduceRating(ratingState, { type: kind === "skip" ? "skip" : "rate", imageId, verdict: kind, ratedAt: new Date().toISOString() });
      if (kind !== "skip") saveRating(imageId);
      refresh();
    } else if (kind === "undo") {
      const imageId = ratingState.history.at(-1);
      if (!imageId) return;
      closeNote(); closePair();
      ratingState = reduceRating(ratingState, { type: "undo" });
      saveRating(imageId);
      refresh();
    } else if (kind === "note") {
      noteImageId = ratingState.history.at(-1);
      if (!noteImageId) { setStatus("Rate an image first; the note belongs to the last rating."); return; }
      $("imageNote").value = ratingState.ratings.get(noteImageId).note ?? "";
      $("imageNoteForm").hidden = false;
      $("imageNote").focus();
    }
  }

  async function openFiles(input) {
    if (loading || opening || !input.files.length) return;
    opening = true;
    controls();
    try {
      const selection = await readSelection(input.files), result = await openBundle(selection, cache);
      if (!result.ok) { refuse(result.errors); return; }
      refuse([]);
      const index = bundles.findIndex((bundle) => bundle.id === result.bundle.id);
      if (index < 0) bundles.push(result.bundle);
      else bundles[index] = result.bundle;
      files.set(result.bundle.id, selection.files);
      if (!result.cached) cacheUnavailable();
      closePair();
      refresh();
    } catch { refuse(["The bundle could not be opened. Select its folder again."]); }
    // Release the input's focus, so the rating keys work at once.
    finally { opening = false; input.value = ""; input.blur(); controls(); }
  }

  for (const [id, kind] of [["imageLike", "like"], ["imageDislike", "dislike"], ["imageSkip", "skip"], ["imageUndo", "undo"], ["imageNoteButton", "note"]]) {
    $(id).addEventListener("click", () => action(kind));
  }
  for (const id of ["bundleFolder", "bundleFiles"]) $(id).addEventListener("change", () => openFiles($(id)));
  $("imageNoteForm").addEventListener("submit", (event) => {
    event.preventDefault();
    if (loading || refuseAnswerChange()) return;
    const note = cleanNote($("imageNote").value);
    if (!validateNote(note)) { setStatus("Image notes need at most 140 characters, without control characters."); return; }
    if (noteImageId && ratingState.ratings.has(noteImageId)) {
      ratingState = reduceRating(ratingState, { type: "note", imageId: noteImageId, note, ratedAt: new Date().toISOString() });
      saveRating(noteImageId);
    }
    closeNote();
  });
  $("imagePairForm").addEventListener("submit", (event) => {
    event.preventDefault();
    if (loading || refuseAnswerChange()) return;
    const note = cleanNote($("imagePairNote").value);
    if (!validateNote(note, "pair")) { setStatus("Pair notes need 1 to 280 characters, without control characters."); return; }
    if (!shownPair) return;
    const { path, body } = pairDocument({ ...shownPair, note, notedAt: new Date().toISOString() });
    const key = pairKey(body.likedImageId, body.dislikedImageId);
    pairs.set(key, body); pairChanges.set(key, body);
    write(path, body);
    closePair();
    refresh();
  });
  $("imagePairClose").addEventListener("click", closePair);
  for (const [id, close] of [["imageNote", closeNote], ["imagePairNote", closePair]]) {
    $(id).addEventListener("keydown", (event) => {
      if (event.repeat && ["Enter", "Escape"].includes(event.key)) event.preventDefault();
      else if (event.key === "Escape") { event.preventDefault(); close(); }
    });
  }

  let ready = Promise.resolve();
  async function readAnswers() {
    if (readingAnswers || answersReady) return;
    readingAnswers = true;
    controls();
    const loaded = await Promise.allSettled([loadImageDocuments(db, "imageRatings"), loadImageDocuments(db, "imagePairs")]);
    // Install the baseline together. No answer may be changed until both reads
    // succeed, so a retry cannot overwrite this tab's edits or delete old answers.
    if (loaded.every((result) => result.status === "fulfilled")) {
      ratingState = createRatingState(loaded[0].value);
      pairs = new Map(loaded[1].value.map((record) => [pairKey(record.likedImageId, record.dislikedImageId), pairDocument(record).body]));
      answersReady = true;
      setStatus("Stored image answers loaded. Rating is available.");
    } else setStatus("Stored image answers could not be read. Rating is paused so they are not overwritten. Retry reading stored answers.");
    readingAnswers = false;
    refresh();
  }
  $("imageAnswersRetry").addEventListener("click", () => { ready = readAnswers(); return ready; });

  async function load(storage, owner) {
    db = storage;
    // Without a storage capability, local-only actions cannot touch stored answers.
    answersReady = !db;
    const cachedFiles = new Map();
    const cached = await loadCached({ ...cache, async get(key) {
      const value = await cache.get(key);
      cachedFiles.set(key, value);
      return value;
    } });
    if (cached.unavailable) cacheUnavailable();
    const refusals = cached.dropped.map(() => "A cached bundle failed validation and was removed. Open its folder again.");
    for (const bundle of cached.bundles) {
      files.set(bundle.id, new Map(cachedFiles.get(bundle.id).files));
      bundles.push(bundle);
    }
    refuse(refusals);
    // Storage reads may wait or fail; opening and viewing bundles remains available.
    loading = false;
    refresh();
    if (db) {
      await readAnswers();
    } else setStatus(owner === false ? "Only the owner of this page can save image answers." : "Storage is not available here; image answers stay in this tab.");
  }

  controls();
  return {
    load(storage, owner) { ready = load(storage, owner); return ready; },
    handleKey(event) {
      const kind = keyAction(event, "images");
      if (kind) { event.preventDefault(); action(kind); }
    },
    async exportData(base, warn = setStatus) {
      await ready;
      await Promise.allSettled([...writes.values()]);
      let stored = null;
      if (db) {
        const refreshed = await Promise.allSettled([loadImageDocuments(db, "imageRatings"), loadImageDocuments(db, "imagePairs")]);
        if (refreshed.every((result) => result.status === "fulfilled")) stored = refreshed;
        else warn("Stored image answers could not be refreshed; answers saved from another tab may be missing.");
      }
      return assembleImageExport(base, { bundles,
        ratings: stored ? stored[0].value : [...ratingState.ratings.values()],
        pairs: stored ? stored[1].value : [...pairs.values()], ratingChanges: ratingState.changes, pairChanges });
    },
  };
}
