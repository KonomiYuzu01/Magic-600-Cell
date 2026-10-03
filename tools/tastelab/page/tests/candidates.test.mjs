import test from "node:test";
import assert from "node:assert/strict";
import { defaultSpace, fromUnit, canonical, hardCheck } from "../../core/space.js";
import { createRng } from "../../core/rng.js";
import { buildGeometry } from "../geometry.js";
import { buildPool, classPairsFor, roundCandidates, toUnit } from "../candidates.js";

// Evidence: synthetic geometry and looks, not rendering or puzzle mechanics.
const space = defaultSpace();
const pairsOf = classPairsFor(buildGeometry());
const pool = buildPool(space, pairsOf, { draws: 1024 });
const key = (look) => JSON.stringify(look);
const check = (look) => hardCheck(look, pairsOf(look.classes).pairs, undefined, space).ok;

// Original roundCandidates, retained here to prove default output and RNG use.
function legacyRound(space, pool, pairsOf, rng, { best = null, n = 512, local = 256, tries = 1024, sigma = 0.08, map = (l) => l } = {}) {
  const out = [];
  const keys = new Set();
  const push = (look) => {
    const k = JSON.stringify(look);
    if (!keys.has(k)) { keys.add(k); out.push(look); }
  };
  if (best) {
    push(best);
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
  const order = pool.map((_, i) => i);
  for (let i = order.length - 1; i > 0; i--) {
    const j = rng.int(i + 1);
    [order[i], order[j]] = [order[j], order[i]];
  }
  for (const i of order) {
    if (out.length >= n) break;
    push(pool[i]);
  }
  return out;
}

test("default options preserve the original looks, order and RNG consumption", () => {
  assert.ok(pool.length >= 8);
  for (const best of [null, pool[0]]) {
    const rng = createRng(7), originalRng = createRng(7);
    const actual = roundCandidates(space, pool, pairsOf, rng, { best });
    assert.deepEqual(actual, legacyRound(space, pool, pairsOf, originalRng, { best }));
    assert.deepEqual([rng.next(), rng.normal(), rng.int(17), rng.normal()],
      [originalRng.next(), originalRng.normal(), originalRng.int(17), originalRng.normal()]);
    assert.ok(actual.every(check));
  }
});

test("all round candidates pass the hard checks and are unique", () => {
  const out = roundCandidates(space, pool, pairsOf, createRng(12), {
    best: pool[0], n: 12, local: 4, tries: 32, include: pool.slice(0, 3), axis: 4, multi: 24,
  });
  assert.ok(out.length > 12);
  assert.ok(out.every(check));
  assert.equal(new Set(out.map(key)).size, out.length);
});

test("a best look that fails the hard checks centres the moves but is not returned", () => {
  const best = { ...pool[0], gap: -1 };
  assert.equal(check(best), false);
  const out = roundCandidates(space, pool, pairsOf, createRng(21), { best, n: 6, local: 3, tries: 64, axis: 2, multi: 4 });
  assert.ok(out.length >= 6);
  assert.ok(out.every(check));
  assert.ok(!out.some((look) => key(look) === key(best)));
});

test("axis moves change at most one parameter from the round-tripped centre", () => {
  const best = pool[0], centre = canonical(space, fromUnit(space, toUnit(space, best)));
  const out = roundCandidates(space, pool, pairsOf, createRng(42), { best, n: 1, local: 0, axis: 4, multi: 0 });
  assert.deepEqual(out[0], best);
  const moves = out.slice(1);
  assert.ok(moves.length > 0 && moves.length <= 4 * space.params.length);
  for (const look of moves) {
    assert.ok(space.params.filter((p) => look[p.id] !== centre[p.id]).length <= 1);
    assert.ok(check(look));
  }
});

test("multi moves change at most three parameters from the round-tripped centre", () => {
  const best = pool[0], centre = canonical(space, fromUnit(space, toUnit(space, best)));
  const out = roundCandidates(space, pool, pairsOf, createRng(42), { best, n: 1, local: 0, axis: 0, multi: 48 });
  const moves = out.slice(1);
  assert.ok(moves.length > 0 && moves.length <= 48);
  for (const look of moves) {
    assert.ok(space.params.filter((p) => look[p.id] !== centre[p.id]).length <= 3);
    assert.ok(check(look));
  }
});

test("include adds new feasible looks once, after the pool fill and before moves", () => {
  const options = { best: pool[0], n: 3, local: 0 };
  const baseline = roundCandidates(space, pool, pairsOf, createRng(14), options);
  const seen = new Set(baseline.map(key));
  const added = pool.filter((look) => !seen.has(key(look))).slice(0, 2);
  const invalid = { ...pool[0], gap: -1 };
  const include = [added[0], added[0], baseline[0], invalid, added[1]];
  const rng = createRng(14), baselineRng = createRng(14);
  roundCandidates(space, pool, pairsOf, baselineRng, options);
  const out = roundCandidates(space, pool, pairsOf, rng, { ...options, include });
  assert.deepEqual(out, [...baseline, ...added]);
  assert.equal(rng.next(), baselineRng.next());
  const moved = roundCandidates(space, pool, pairsOf, createRng(14), { ...options, include, axis: 1, multi: 4 });
  assert.deepEqual(moved.slice(0, out.length), out);
  assert.equal(new Set(moved.map(key)).size, moved.length);
});

test("map fixes parameters on local, axis and multi moves", () => {
  const map = (look) => ({ ...look, gloss: 0.25, glow: 0.2, fog: 0.4 });
  const fixedPool = pool.map(map), best = fixedPool[0];
  for (const options of [{ local: 3 }, { local: 0, axis: 4 }, { local: 0, multi: 48 }]) {
    const out = roundCandidates(space, fixedPool, pairsOf, createRng(42), { best, n: 1, map, ...options });
    assert.ok(out.length > 1);
    for (const look of out) {
      assert.equal(look.gloss, 0.25);
      assert.equal(look.glow, 0.2);
      assert.equal(look.fog, 0.4);
      assert.ok(check(look));
    }
  }
});

test("axis and multi options consume no RNG and add no moves without a best look", () => {
  const rng = createRng(18), baselineRng = createRng(18);
  assert.deepEqual(roundCandidates(space, pool, pairsOf, rng, { n: 5, axis: 4, multi: 24 }),
    roundCandidates(space, pool, pairsOf, baselineRng, { n: 5 }));
  assert.equal(rng.next(), baselineRng.next());
});
