// Export presets (plan sections 4.5 and 7): the best look per theme family and
// scene, its settled state, and the parameters the families share.
import { SCENES } from "../core/space.js";

// Parameters whose frame-time cost H-06 measures before a preset reaches G3.
// Taste Lab measures no performance; the export makes no performance claim.
export const COST_HEAVY = Object.freeze(["gloss", "glow", "fog", "edgeWeight"]);

// The difference between two values of one parameter in units of its range;
// a circular parameter takes the shorter way round.
export function rangeDifference(p, a, b) {
  let d = Math.abs(a - b);
  if (p.kind === "circular") {
    d %= 360;
    d = Math.min(d, 360 - d);
  }
  return d / (p.max - p.min);
}

// Settled for the export: the settled bests of at least two sessions lie in
// the same region as the exported look (plan section 5.6).
export function confirmedBy(look, settledRecords, sameRegion) {
  if (!look) return false;
  const sessions = new Set();
  for (const r of settledRecords) if (r && r.best && sameRegion(r.best, look)) sessions.add(r.session);
  return sessions.size >= 2;
}

// Per parameter and scene: each family's best value and the largest difference
// between the families in units of the parameter's range. A parameter within
// `within` in every scene is shared; when a family has no look in some scene the
// mark is null. Null with fewer than two families.
export function sharedSummary(space, presets, { within = 0.1 } = {}) {
  const families = [...new Set(presets.map((x) => x.family))];
  if (families.length < 2) return null;
  return space.params.map((p) => {
    const scenes = SCENES.map((scene) => {
      const values = {};
      for (const f of families) {
        const x = presets.find((y) => y.family === f && y.scene === scene);
        values[f] = x && x.look ? x.look[p.id] : null;
      }
      const known = Object.values(values);
      let difference = null;
      if (known.every((v) => v !== null)) {
        difference = 0;
        for (let i = 0; i < known.length; i++) {
          for (let j = i + 1; j < known.length; j++) difference = Math.max(difference, rangeDifference(p, known[i], known[j]));
        }
      }
      return { scene, values, difference };
    });
    const decided = scenes.every((s) => s.difference !== null);
    return { id: p.id, scenes, shared: decided ? scenes.every((s) => s.difference <= within) : null };
  });
}
