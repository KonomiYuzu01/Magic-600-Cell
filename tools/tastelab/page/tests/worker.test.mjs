import test from "node:test";
import assert from "node:assert/strict";
import { defaultSpace, sobol, fromUnit, SCENES } from "../../core/space.js";

// Evidence: synthetic looks and answers; no stored session data. The worker
// module runs with a stand-in for the Web Worker global.
// Replies to each message, and the documents that finished searches post.
const out = [], models = [];
globalThis.self = { postMessage: (m) => (m.type === "model" ? models : out).push(m) };
await import("../worker.js");
const send = (data) => {
  out.length = 0;
  self.onmessage({ data });
  assert.equal(out.length, 1);
  return out[0];
};
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

const space = defaultSpace();
const points = sobol(16, space.params.length, 1).map((u) => fromUnit(space, u));
const record = (a, b, family, answer = "A") => ({ lookA: points[a], lookB: points[b], family, scene: "solving", answer, t: "", session: "s1" });

test("the worker keeps one learner per family and routes every answer to its family", () => {
  const records = [record(0, 1), record(2, 3, "f1"), record(4, 5, "f2")];
  let msg = send({ type: "init", space, records, settled: [], session: "s2", scene: "solving", family: "f1", seed: 7 });
  assert.equal(msg.type, "pair");
  assert.equal(msg.family, "f1");
  assert.equal(msg.counts.answers, 2);
  // The second family's learner is built on first use from every record so far.
  msg = send({ type: "next", scene: "solving", family: "f2" });
  assert.equal(msg.family, "f2");
  assert.equal(msg.counts.answers, 1);
  msg = send({ type: "answer", record: record(6, 7, "f2", "B"), scene: "solving", family: "f2" });
  assert.equal(msg.counts.answers, 2);
  // An answer of the family not shown goes to that family's learner only.
  msg = send({ type: "answer", record: record(8, 9, "f1"), scene: "solving", family: "f2" });
  assert.equal(msg.family, "f2");
  assert.equal(msg.counts.answers, 2);
  assert.equal(send({ type: "next", scene: "solving", family: "f1" }).counts.answers, 3);
  const err = send({ type: "next", scene: "solving", family: "f3" });
  assert.equal(err.type, "error");
  assert.equal(err.message, "family");
});

test("presets give every family and scene its best look, answer count and settled state", () => {
  const settled = [{ family: "f1", scene: "solving", session: "s0", best: points[0] }];
  const msg = send({ type: "presets", id: "p1", settled });
  assert.equal(msg.type, "presets");
  assert.equal(msg.id, "p1");
  assert.equal(msg.presets.length, 2 * SCENES.length);
  const at = (family, scene) => msg.presets.find((x) => x.family === family && x.scene === scene);
  assert.equal(at("f1", "solving").answers, 3);
  assert.equal(at("f2", "solving").answers, 2);
  assert.equal(at("f1", "solving").name, "Family 1");
  assert.ok(at("f1", "solving").look && at("f2", "solving").look);
  assert.equal(at("f1", "inspecting").look, null, "no answer in this scene yet");
  assert.equal(at("f1", "solving").settled, false, "one session does not settle");
  // The inspecting scene has no look yet, so no parameter is marked either way.
  assert.equal(msg.shared.length, space.params.length);
  for (const s of msg.shared) assert.equal(s.shared, null, s.id);
});

test("a pair says whether its family's hyperparameter search has finished since the learner was built", async () => {
  const records = Array.from({ length: 12 }, (_, i) => record(i, i + 1, "f1", i % 3 ? "A" : "B"));
  records.push(record(13, 14, "f2"), record(14, 15, "f2", "B"));
  let msg = send({ type: "init", space, records, settled: [], session: "s3", scene: "solving", family: "f1", seed: 11 });
  assert.equal(msg.type, "pair");
  assert.deepEqual([msg.fitted, msg.searched], [false, false], "the search after a load has not run yet");
  models.length = 0;
  // The search runs one evaluation per task between messages.
  for (let i = 0; i < 20000 && !msg.fitted; i++) {
    await tick();
    if (i % 200 === 199) msg = send({ type: "next", scene: "solving", family: "f1" });
  }
  assert.deepEqual([msg.fitted, msg.searched], [true, true]);
  // The finished search posts its hyperparameters for the page to store.
  assert.deepEqual(models.map((m) => [m.family, m.doc.answers]), [["f1", 12]]);
  // Another family's learner is built on first use and searches on its own.
  assert.equal(send({ type: "next", scene: "solving", family: "f2" }).fitted, false);
  send({ type: "presets", id: "p3", settled: [] });
  assert.equal(send({ type: "next", scene: "solving", family: "f2" }).fitted, true, "presets finish the search of a family with nothing fitted");
  assert.deepEqual(models.map((m) => m.family), ["f1", "f2"]);
  assert.equal(send({ type: "next", scene: "solving", family: "f1" }).fitted, true);
});

test("stored hyperparameters serve the first pair after a load or an undo until the new search ends", async () => {
  // The documents of the searches above, as the store returns them.
  const stored = Object.fromEntries(models.map((m) => [m.family, JSON.parse(JSON.stringify(m.doc))]));
  assert.ok(stored.f1 && stored.f2);
  const records = Array.from({ length: 12 }, (_, i) => record(i, i + 1, "f1", i % 3 ? "A" : "B"));
  const init = (extra) => send({ type: "init", space, records, settled: [], models: stored, session: "s4", scene: "solving", family: "f1", seed: 11, ...extra });
  let msg = init({});
  assert.deepEqual([msg.fitted, msg.searched], [true, false]);
  assert.deepEqual(msg.relevance.map((r) => r.value), stored.f1.hyper.lengths.map((l) => 1 / l));
  // An undo re-initializes the worker while the search is pending: the new learner
  // starts from the stored values, and only its own search posts a document.
  await tick();
  msg = init({ records: records.slice(0, 11), seed: 12 });
  assert.deepEqual([msg.fitted, msg.searched], [true, false]);
  models.length = 0;
  for (let i = 0; i < 20000 && !models.length; i++) await tick();
  assert.equal(models.length, 1);
  const doc = models[0].doc;
  assert.equal(models[0].family, "f1");
  assert.deepEqual([doc.version, doc.family, doc.answers, typeof doc.t], [1, "f1", 11, "string"]);
  msg = send({ type: "next", scene: "solving", family: "f1" });
  assert.deepEqual([msg.fitted, msg.searched], [true, true]);
  // A document stored for another family or another parameter table is ignored.
  msg = init({ models: { f1: stored.f2, f2: stored.f1 } });
  assert.deepEqual([msg.fitted, msg.searched], [false, false]);
  const linear = space.params.findIndex((p) => p.kind === "linear");
  const wider = { ...space, params: space.params.map((p, i) => (i === linear ? { ...p, max: p.max + 1 } : p)) };
  assert.equal(init({ space: wider }).fitted, false);
});

const exportRecords = () => {
  const records = Array.from({ length: 12 }, (_, i) => record(i, i + 1, "f1", i % 3 ? "A" : "B"));
  records.push(record(13, 14, "f2"), record(14, 15, "f2", "B"));
  return records;
};
const at = (msg, family) => msg.presets.find((x) => x.family === family && x.scene === "solving");
const flags = (msg) => ["f1", "f2"].flatMap((f) => [at(msg, f).fitted, at(msg, f).searched]);
// Both families' documents after their searches, which an export without stored values finishes.
function storedDocs(session, seed) {
  send({ type: "init", space, records: exportRecords(), settled: [], session, scene: "solving", family: "f1", seed });
  models.length = 0;
  const msg = send({ type: "presets", id: `all-${session}`, settled: [] });
  assert.deepEqual(flags(msg), [true, true, true, true]);
  assert.deepEqual(models.map((m) => m.family).sort(), ["f1", "f2"]);
  return Object.fromEntries(models.map((m) => [m.family, JSON.parse(JSON.stringify(m.doc))]));
}

test("the export uses the hyperparameters in force and settles nothing before a search since the load has ended", async () => {
  const records = exportRecords();
  const stored = storedDocs("s5", 13);
  // With stored values the export finishes no search and says where its values come from.
  // Stored lengths of 50 put every look in one region, so two sessions' settled bests
  // would confirm it, but the search since the load has not ended.
  const wide = { ...stored.f1, hyper: { ...stored.f1.hyper, lengths: stored.f1.hyper.lengths.map(() => 50) } };
  const settled = [1, 2].map((s) => ({ family: "f1", scene: "solving", session: `s${s - 1}`, best: points[4 * s] }));
  const pair = send({ type: "init", space, records, settled, models: { f1: wide, f2: stored.f2 }, session: "s6", scene: "solving", family: "f1", seed: 13 });
  assert.deepEqual([pair.fitted, pair.searched], [true, false]);
  models.length = 0;
  let msg = send({ type: "presets", id: "p6", settled });
  assert.equal(models.length, 0, "the export finishes no search");
  assert.deepEqual(flags(msg), [true, false, true, false]);
  assert.deepEqual(at(msg, "f1").look, pair.best, "the export's best look is the page's");
  assert.equal(at(msg, "f1").settled, false, "not settled before the search since the load has ended");
  // The shown family's search after the load still ends in idle time.
  assert.equal(send({ type: "next", scene: "solving", family: "f1" }).searched, false);
  for (let i = 0; i < 20000 && !models.length; i++) await tick();
  assert.deepEqual(models.map((m) => m.family), ["f1"]);
  assert.equal(send({ type: "next", scene: "solving", family: "f1" }).searched, true);
  // Then two sessions whose settled best is the exported look settle it, and one does
  // not; the family whose search is still pending stays unsettled.
  const looks = send({ type: "presets", id: "p7", settled: [] });
  const by = (family, n) => Array.from({ length: n }, (_, s) => ({ family, scene: "solving", session: `s${s}`, best: at(looks, family).look }));
  msg = send({ type: "presets", id: "p8", settled: [...by("f1", 2), ...by("f2", 2)] });
  assert.deepEqual(flags(msg), [true, true, true, false]);
  assert.deepEqual([at(msg, "f1").settled, at(msg, "f2").settled], [true, false]);
  assert.equal(at(send({ type: "presets", id: "p9", settled: by("f1", 1) }), "f1").settled, false, "one session does not settle");
});

test("a tenth answer during the search after a load leaves the stored values in force for pairs and the export", async () => {
  const stored = storedDocs("s7", 17);
  let pair = send({ type: "init", space, records: exportRecords(), settled: [], models: stored, session: "s8", scene: "solving", family: "f1", seed: 17 });
  assert.deepEqual([pair.fitted, pair.searched], [true, false]);
  // One idle evaluation, then ten answers: the tenth queues the next search while this one runs.
  await tick();
  for (let i = 0; i < 10; i++) {
    pair = send({ type: "answer", record: record(i, i + 2, "f1", i % 3 ? "B" : "A"), scene: "solving", family: "f1" });
  }
  assert.deepEqual([pair.fitted, pair.searched], [true, false]);
  assert.deepEqual(pair.relevance.map((r) => r.value), stored.f1.hyper.lengths.map((l) => 1 / l), "the stored lengths");
  const msg = send({ type: "presets", id: "p10", settled: [] });
  assert.deepEqual(flags(msg), [true, false, true, false]);
  assert.deepEqual(at(msg, "f1").look, pair.best, "the export's best look is the page's");
});

test("a search queued during a search runs after it in idle time; the export waits only for the first", async () => {
  send({ type: "init", space, records: exportRecords(), settled: [], session: "s9", scene: "solving", family: "f1", seed: 19 });
  await tick();
  for (let i = 0; i < 10; i++) send({ type: "answer", record: record(i, i + 2, "f1", i % 3 ? "B" : "A"), scene: "solving", family: "f1" });
  models.length = 0;
  // The export runs the search of a family with nothing fitted to its end, not the one queued after it.
  const msg = send({ type: "presets", id: "p11", settled: [] });
  assert.deepEqual(flags(msg), [true, true, true, true]);
  assert.deepEqual(models.map((m) => m.family).sort(), ["f1", "f2"]);
  // The idle loop then runs the queued search to its end, which posts its document too.
  for (let i = 0; i < 2000 && models.length < 3; i++) await tick();
  assert.deepEqual(models.map((m) => [m.family, m.doc.answers]), [["f1", 22], ["f2", 2], ["f1", 22]]);
});
