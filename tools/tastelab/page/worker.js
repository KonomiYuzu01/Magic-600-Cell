// Taste Lab model worker: runs the learners off the page's main thread, one
// per theme family (plan section 4.5). Only the shown family's hyperparameter
// search runs, one evaluation per task between messages, so that the next pair
// never waits for a whole search (plan section 5.5).
import { buildGeometry } from "./geometry.js";
import { createLearner, recordFamily } from "./learner.js";
import { defaultSpace, familiesOf, SCENES } from "../core/space.js";
import { confirmedBy, sharedSummary } from "./presets.js";

const geometry = buildGeometry();
const learners = new Map();
let ctx = null; // the space, records, settled records, session and seed of the last init
let shown = null; // the family whose search runs when idle
let idleTimer = null;
// Learners with a search under way, and learners whose search has finished since
// they were built: until then every parameter has the starting length scale, so the
// pair says that its relevance ranking is not fitted yet.
const searching = new WeakSet();
const fitted = new WeakSet();

const fail = (err, id) => self.postMessage({ type: "error", id, message: String(err && err.message ? err.message : err) });

// A family's learner, built from every record of this page view on first use.
function learnerFor(family) {
  if (!learners.has(family)) {
    const index = familiesOf(ctx.space).findIndex((f) => f.id === family);
    if (index < 0) throw new RangeError("family");
    const learner = createLearner(geometry, { deferHyper: true });
    learner.reset(ctx.space, ctx.records, { seed: ctx.seed + index, session: ctx.session, settledRecords: ctx.settled, family });
    learners.set(family, learner);
  }
  return learners.get(family);
}

function propose(family, scene) {
  shown = family;
  const learner = learnerFor(family);
  return { ...learner.propose(scene), fitted: fitted.has(learner) };
}

// One evaluation of a learner's pending search; true while work remains.
function step(learner) {
  if (learner.idle()) {
    searching.add(learner);
    return true;
  }
  if (searching.delete(learner)) fitted.add(learner);
  return false;
}

// The export's starting points: every family's searches finish first.
function presets(settled) {
  const out = [];
  for (const f of familiesOf(ctx.space)) {
    const learner = learnerFor(f.id);
    while (step(learner));
    for (const scene of SCENES) {
      const look = learner.best(scene);
      const own = (r) => r && r.scene === scene && recordFamily(ctx.space, r) === f.id;
      out.push({
        family: f.id,
        name: f.name,
        scene,
        look,
        answers: ctx.records.filter(own).length,
        settled: confirmedBy(look, settled.filter(own), learner.sameRegion),
      });
    }
  }
  return { presets: out, shared: sharedSummary(ctx.space, out) };
}

function runIdle() {
  idleTimer = null;
  try {
    if (shown !== null && learners.has(shown) && step(learners.get(shown))) scheduleIdle();
  } catch (err) {
    fail(err);
  }
}

function scheduleIdle() {
  if (idleTimer === null) idleTimer = setTimeout(runIdle, 0);
}

self.onmessage = (e) => {
  const m = e.data;
  try {
    if (m.type === "init") {
      learners.clear();
      ctx = { space: m.space || defaultSpace(), records: (m.records || []).slice(), settled: m.settled || [], session: m.session || null, seed: m.seed || 1 };
      self.postMessage(propose(m.family || familiesOf(ctx.space)[0].id, m.scene));
    } else if (m.type === "answer") {
      ctx.records.push(m.record);
      // A family without a learner yet reads the record from ctx when first shown.
      const family = recordFamily(ctx.space, m.record);
      const learner = learners.get(family);
      // A rebuild at the look cap starts again from the starting hyperparameters.
      if (learner && learner.answer(m.record).rebuilt) fitted.delete(learner);
      self.postMessage(propose(m.family, m.scene));
    } else if (m.type === "next") {
      self.postMessage(propose(m.family, m.scene));
    } else if (m.type === "presets") {
      self.postMessage({ type: "presets", id: m.id, ...presets(m.settled || []) });
    }
  } catch (err) {
    fail(err, m && m.id);
  }
  scheduleIdle();
};
