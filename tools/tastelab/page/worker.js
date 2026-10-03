// Taste Lab model worker: runs the learners off the page's main thread, one
// per theme family (plan section 4.5). Only the shown family's hyperparameter
// search runs, one evaluation per task between messages, so that the next pair
// never waits for a whole search (plan section 5.5).
import { buildGeometry } from "./geometry.js";
import { createLearner, recordFamily, storedHyper, hyperDoc } from "./learner.js";
import { defaultSpace, familiesOf, SCENES } from "../core/space.js";
import { confirmedBy, sharedSummary } from "./presets.js";

const geometry = buildGeometry();
const learners = new Map();
let ctx = null; // the space, records, settled records, stored models, session and seed of the last init
let shown = null; // the family whose search runs when idle
let idleTimer = null;

const fail = (err, id) => self.postMessage({ type: "error", id, message: String(err && err.message ? err.message : err) });

// A family's learner, built from every record of this page view on first use.
// It starts with the family's stored hyperparameters, and each finished search
// posts its result for the page to store (plan section 7).
function learnerFor(family) {
  if (!learners.has(family)) {
    const index = familiesOf(ctx.space).findIndex((f) => f.id === family);
    if (index < 0) throw new RangeError("family");
    const space = ctx.space;
    const learner = createLearner(geometry, {
      deferHyper: true,
      onSearch: (hyper) => self.postMessage({
        type: "model", family, doc: { ...hyperDoc(space, family, hyper, learner.state.answers), t: new Date().toISOString() },
      }),
    });
    const hyper = storedHyper(space, ctx.models[family], family);
    learner.reset(space, ctx.records, { seed: ctx.seed + index, session: ctx.session, settledRecords: ctx.settled, family, hyper });
    learners.set(family, learner);
  }
  return learners.get(family);
}

function propose(family, scene) {
  shown = family;
  return learnerFor(family).propose(scene);
}

// The export's starting points, under each family's hyperparameters in force
// (plan section 9). Only a family without fitted ones finishes its pending search
// first (plan section 7).
function presets(settled) {
  const out = [];
  for (const f of familiesOf(ctx.space)) {
    const learner = learnerFor(f.id);
    if (!learner.state.fitted) while (learner.idle());
    const { fitted, searched } = learner.state;
    for (const scene of SCENES) {
      const look = learner.best(scene);
      const own = (r) => r && r.scene === scene && recordFamily(ctx.space, r) === f.id;
      out.push({
        family: f.id,
        name: f.name,
        scene,
        look,
        answers: ctx.records.filter(own).length,
        // As on the page, nothing is settled before a search since the load has ended.
        settled: searched && confirmedBy(look, settled.filter(own), learner.sameRegion),
        fitted,
        searched,
      });
    }
  }
  return { presets: out, shared: sharedSummary(ctx.space, out) };
}

function runIdle() {
  idleTimer = null;
  try {
    if (shown !== null && learners.has(shown) && learners.get(shown).idle()) scheduleIdle();
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
      ctx = {
        space: m.space || defaultSpace(), records: (m.records || []).slice(), settled: m.settled || [],
        models: m.models || {}, session: m.session || null, seed: m.seed || 1,
      };
      self.postMessage(propose(m.family || familiesOf(ctx.space)[0].id, m.scene));
    } else if (m.type === "answer") {
      ctx.records.push(m.record);
      // A family without a learner yet reads the record from ctx when first shown.
      const family = recordFamily(ctx.space, m.record);
      const learner = learners.get(family);
      if (learner) learner.answer(m.record);
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
