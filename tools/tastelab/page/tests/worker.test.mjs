import test from "node:test";
import assert from "node:assert/strict";
import { defaultSpace, sobol, fromUnit, SCENES } from "../../core/space.js";

// Evidence: synthetic looks and answers; no stored session data. The worker
// module runs with a stand-in for the Web Worker global.
const out = [];
globalThis.self = { postMessage: (m) => out.push(m) };
await import("../worker.js");
const send = (data) => {
  out.length = 0;
  self.onmessage({ data });
  assert.equal(out.length, 1);
  return out[0];
};

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
