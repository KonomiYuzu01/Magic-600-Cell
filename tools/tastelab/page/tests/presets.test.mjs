import test from "node:test";
import assert from "node:assert/strict";
import { COST_HEAVY, rangeDifference, confirmedBy, sharedSummary } from "../presets.js";
import { defaultSpace, sobol, fromUnit, SCENES } from "../../core/space.js";

// Evidence: synthetic looks only; no stored session data.
const space = defaultSpace();
const byId = Object.fromEntries(space.params.map((p) => [p.id, p]));
const base = fromUnit(space, sobol(1, space.params.length, 1)[0]);

test("differences are in units of the range, circular ones the shorter way round", () => {
  assert.equal(rangeDifference(byId.hueRotation, 350, 10), 20 / 360);
  assert.equal(rangeDifference(byId.hueRotation, 10, 370), 0);
  assert.equal(rangeDifference(byId.gap, 0, 0.3), 1);
  assert.equal(rangeDifference(byId.classes, 5, 6), 1 / 4);
  assert.deepEqual([...COST_HEAVY], ["gloss", "glow", "fog", "edgeWeight"]);
});

test("an exported look is settled when the settled bests of two sessions share its region", () => {
  const near = (a, b) => Math.abs(a.glow - b.glow) < 0.05;
  const look = { glow: 0.3 };
  const s1 = { session: "s1", best: { glow: 0.31 } };
  assert.equal(confirmedBy(look, [s1, { ...s1, best: { glow: 0.29 } }, { session: "s2", best: { glow: 0.6 } }], near), false);
  assert.equal(confirmedBy(look, [s1, { session: "s2", best: { glow: 0.28 } }], near), true);
  assert.equal(confirmedBy(null, [s1, { session: "s2", best: { glow: 0.3 } }], near), false);
  assert.equal(confirmedBy(look, [null, { session: "s2" }], near), false);
});

test("the shared summary marks a parameter within a tenth of its range in every scene", () => {
  const presets = (second) => ["f1", "f2"].flatMap((family) => SCENES.map((scene) => ({
    family, scene, look: family === "f1" ? base : second(scene),
  })));
  const close = (scene) => ({ ...base, chroma: base.chroma + 0.01, hueRotation: (base.hueRotation + (scene === "celebrating" ? 50 : 20)) % 360 });
  const summary = Object.fromEntries(sharedSummary(space, presets(close)).map((s) => [s.id, s]));
  assert.equal(summary.chroma.shared, true);
  assert.equal(summary.hueRotation.shared, false, "50 degrees in one scene is more than a tenth");
  assert.equal(summary.gap.shared, true);
  const celebrating = summary.hueRotation.scenes.find((s) => s.scene === "celebrating");
  assert.deepEqual(celebrating.values, { f1: base.hueRotation, f2: (base.hueRotation + 50) % 360 });
  assert.ok(Math.abs(celebrating.difference - 50 / 360) < 1e-12);
  // A family without a look in some scene leaves the mark undecided.
  const missing = presets(close).map((x) => (x.family === "f2" && x.scene === "inspecting" ? { ...x, look: null } : x));
  for (const s of sharedSummary(space, missing)) assert.equal(s.shared, null, s.id);
  assert.equal(sharedSummary(space, presets(close).filter((x) => x.family === "f1")), null);
});
