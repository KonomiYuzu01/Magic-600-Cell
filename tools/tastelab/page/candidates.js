// Candidate looks for one round (plan section 5.4): a pool of feasible Sobol
// looks built once, plus local perturbations around the current best.
// Only looks that pass the hard checks are ever returned.
import { sobol, fromUnit, canonical, hardCheck } from "../core/space.js";
import { properColouring } from "./geometry.js";

export function classPairsFor(geometry) {
  const cache = new Map();
  return (k) => {
    if (!cache.has(k)) {
      const colours = properColouring(20, geometry.ringAdjacency, k);
      const seen = new Map();
      if (colours) {
        for (const [i, j] of geometry.ringAdjacency) {
          const a = Math.min(colours[i], colours[j]), b = Math.max(colours[i], colours[j]);
          seen.set(`${a},${b}`, [a, b]);
        }
      }
      cache.set(k, { colours, pairs: [...seen.values()] });
    }
    return cache.get(k);
  };
}

export function toUnit(space, look) {
  return space.params.map((p) => {
    const v = look[p.id];
    if (p.kind === "circular") return (((v % 360) + 360) % 360) / 360;
    if (p.kind === "integer") return Math.min(0.999999, (v - p.min + 0.5) / (p.max - p.min + 1));
    return Math.min(0.999999, Math.max(0, (v - p.min) / (p.max - p.min)));
  });
}

// Feasible looks from the first `draws` Sobol points. `map` may fix some
// parameters (the experiment's low-dimensional setting) before the check.
export function buildPool(space, pairsOf, { draws = 16384, map = (l) => l } = {}) {
  const pool = [];
  for (const u of sobol(draws, space.params.length, 1)) {
    const look = canonical(space, map(fromUnit(space, u)));
    if (hardCheck(look, pairsOf(look.classes).pairs, undefined, space).ok) pool.push(look);
  }
  return pool;
}

export function roundCandidates(space, pool, pairsOf, rng, { best = null, n = 512, local = 256, tries = 1024, sigma = 0.08, map = (l) => l, include = [], axis = 0, multi = 0 } = {}) {
  const out = [];
  const keys = new Set();
  const push = (look) => {
    const k = JSON.stringify(look);
    if (!keys.has(k)) { keys.add(k); out.push(look); }
  };
  if (best) {
    // The centre joins the round only if it passes the hard checks of this table: a look shown
    // before the table changed may fail them. The moves still start from it.
    if (hardCheck(best, pairsOf(best.classes).pairs, undefined, space).ok) push(best);
    const base = toUnit(space, best);
    for (let t = 0; t < tries && out.length < local + 1; t++) {
      const u = base.map((x, i) => {
        const y = x + sigma * rng.normal();
        return space.params[i].kind === "circular" ? ((y % 1) + 1) % 1 : Math.min(0.999999, Math.max(0, y));
      });
      const look = canonical(space, map(fromUnit(space, u)));
      if (hardCheck(look, pairsOf(look.classes).pairs, undefined, space).ok) push(look);
    }
  }
  // Fill the rest with a random subset of the feasible pool.
  const order = pool.map((_, i) => i);
  for (let i = order.length - 1; i > 0; i--) {
    const j = rng.int(i + 1);
    [order[i], order[j]] = [order[j], order[i]];
  }
  for (const i of order) {
    if (out.length >= n) break;
    push(pool[i]);
  }
  for (const look of include) {
    if (hardCheck(look, pairsOf(look.classes).pairs, undefined, space).ok) push(look);
  }
  if (best) {
    const base = toUnit(space, best);
    for (let i = 0; i < space.params.length; i++) {
      for (let t = 0; t < axis; t++) {
        const u = base.slice();
        u[i] = rng.next() * 0.999999;
        const look = canonical(space, map(fromUnit(space, u)));
        if (hardCheck(look, pairsOf(look.classes).pairs, undefined, space).ok) push(look);
      }
    }
    for (let t = 0; t < multi; t++) {
      const u = base.slice();
      const m = 2 + rng.int(2);
      for (let i = 0; i < m; i++) u[rng.int(space.params.length)] = rng.next() * 0.999999;
      const look = canonical(space, map(fromUnit(space, u)));
      if (hardCheck(look, pairsOf(look.classes).pairs, undefined, space).ok) push(look);
    }
  }
  return out;
}
