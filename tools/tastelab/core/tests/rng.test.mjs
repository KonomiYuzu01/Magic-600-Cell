import test from 'node:test';
import assert from 'node:assert/strict';
import {createRng} from '../rng.js';

test('Mulberry32 known values and deterministic mixed draws', () => {
  const known = createRng(1);
  assert.equal(known.next(), 0.6270739405881613);
  assert.equal(known.next(), 0.002735721180215478);
  assert.equal(known.next(), 0.5274470399599522);
  const a = createRng(123), b = createRng(123);
  for (let i = 0; i < 200; i++) {
    assert.equal(a.next(), b.next());
    assert.equal(a.normal(), b.normal());
    assert.equal(a.int(17), b.int(17));
  }
  assert.notEqual(createRng(123).next(), createRng(124).next());
});

test('uniform bounds, integer bounds and Gaussian moments', () => {
  const rng = createRng(42);
  let sum = 0, squares = 0;
  const counts = new Array(7).fill(0);
  for (let i = 0; i < 50000; i++) {
    const u = rng.next();
    assert.ok(u >= 0 && u < 1);
    counts[rng.int(7)]++;
    const z = rng.normal();
    sum += z;
    squares += z * z;
  }
  assert.ok(Math.abs(sum / 50000) < 0.02);
  assert.ok(Math.abs(squares / 50000 - 1) < 0.03);
  assert.ok(counts.every(n => Math.abs(n / 50000 - 1 / 7) < 0.01));
  assert.equal(rng.int(1), 0);
  assert.ok(rng.int(4294967296) < 4294967296);
  for (const n of [0, -1, 2.5, Infinity, 4294967297]) assert.throws(() => rng.int(n));
  assert.throws(() => createRng(NaN));
});
