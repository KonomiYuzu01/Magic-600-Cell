import test from "node:test";
import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
import {PHI, buildGeometry, properColouring, capTurn, applyMatrix} from "../geometry.js";

// Evidence: synthetic base-polytope geometry, not puzzle mechanics or rendering.
const fixture = JSON.parse(readFileSync(new URL("./fixtures/600cell.json", import.meta.url), "utf8"));
const geometry = buildGeometry();
const tolerance = 1e-12;
const vertex = (i) => geometry.vertices.subarray(4 * i, 4 * i + 4);
const center = (i) => geometry.cellCenters.subarray(4 * i, 4 * i + 4);
const dot = (a, b) => a.reduce((sum, x, i) => sum + x * b[i], 0);

function close(actual, expected, context = "") {
  assert.ok(Math.abs(actual - expected) <= tolerance, `${context}: ${actual} != ${expected}`);
}

function vectorClose(actual, expected, context = "") {
  assert.equal(actual.length, expected.length);
  for (let i = 0; i < actual.length; i++) close(actual[i], expected[i], `${context}, component ${i}`);
}

function compareTuples(a, b) {
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return a[i] - b[i];
  return 0;
}

function canonicalTuples(tuples, width, limit) {
  tuples.forEach((tuple, i) => {
    assert.equal(tuple.length, width);
    tuple.forEach((v, j) => {
      assert.ok(Number.isInteger(v) && v >= 0 && v < limit);
      if (j > 0) assert.ok(tuple[j - 1] < v);
    });
    if (i > 0) assert.ok(compareTuples(tuples[i - 1], tuple) < 0);
  });
}

function faceIncidence() {
  const faces = new Map();
  geometry.cells.forEach(([a, b, c, d], n) => {
    for (const face of [[a, b, c], [a, b, d], [a, c, d], [b, c, d]]) {
      const key = face.join(",");
      if (!faces.has(key)) faces.set(key, []);
      faces.get(key).push(n);
    }
  });
  return faces;
}

test("fixture vertex order, unit coordinates, edges and simplex counts", () => {
  assert.equal(PHI, (1 + Math.sqrt(5)) / 2);
  assert.ok(geometry.vertices instanceof Float64Array);
  assert.equal(geometry.vertices.length, 120 * 4);
  assert.equal(geometry.edges.length, 720);
  assert.equal(geometry.triangles.length, 1200);
  assert.equal(geometry.cells.length, 600);
  assert.deepEqual(geometry.cells, fixture.cells);
  fixture.vertices.forEach((v, i) => {
    vectorClose(vertex(i), v.map(([a, b]) => (a + b * PHI) / 2), `vertex ${i}`);
    close(Math.hypot(...vertex(i)), 1, `vertex ${i} norm`);
  });
  canonicalTuples(geometry.edges, 2, 120);
  canonicalTuples(geometry.triangles, 3, 120);
  canonicalTuples(geometry.cells, 4, 120);
  for (const [a, b] of geometry.edges) {
    close(Math.hypot(...Array.from(vertex(a), (x, d) => x - vertex(b)[d])), 1 / PHI, `edge ${a},${b}`);
  }
  const edges = new Set(geometry.edges.map((e) => e.join(",")));
  for (const [a, b, c] of geometry.triangles) {
    for (const edge of [[a, b], [a, c], [b, c]]) assert.ok(edges.has(edge.join(",")));
  }
});

test("unit cell centers, two cells per face and four distinct face neighbors", () => {
  assert.ok(geometry.cellCenters instanceof Float64Array);
  assert.equal(geometry.cellCenters.length, 600 * 4);
  geometry.cells.forEach((cell, n) => {
    const sum = [0, 0, 0, 0];
    for (const v of cell) for (let d = 0; d < 4; d++) sum[d] += vertex(v)[d];
    const norm = Math.hypot(...sum);
    vectorClose(center(n), sum.map((x) => x / norm), `center ${n}`);
    close(Math.hypot(...center(n)), 1, `center ${n} norm`);
  });
  const faces = faceIncidence();
  assert.equal(faces.size, 1200);
  assert.deepEqual([...faces.keys()].map((f) => f.split(",").map(Number)).sort(compareTuples), geometry.triangles);
  const expected = Array.from({length: 600}, () => []);
  for (const owners of faces.values()) {
    assert.equal(owners.length, 2);
    const [a, b] = owners;
    assert.notEqual(a, b);
    expected[a].push(b);
    expected[b].push(a);
  }
  assert.equal(geometry.faceNeighbors.length, 600);
  geometry.faceNeighbors.forEach((neighbors, n) => {
    assert.equal(neighbors.length, 4);
    assert.equal(new Set(neighbors).size, 4);
    assert.deepEqual(neighbors, expected[n].sort((a, b) => a - b));
  });
});

test("fixture rings form a disjoint cover of closed face-connected cycles", () => {
  assert.deepEqual(geometry.rings, fixture.rings);
  assert.equal(geometry.rings.length, 20);
  assert.ok(geometry.ringOf instanceof Int32Array);
  assert.equal(geometry.ringOf.length, 600);
  const covered = new Set();
  geometry.rings.forEach((ring, r) => {
    assert.equal(ring.length, 30);
    assert.equal(new Set(ring).size, 30);
    const members = new Set(ring);
    ring.forEach((n, i) => {
      if (i > 0) assert.ok(ring[i - 1] < n);
      assert.ok(!covered.has(n));
      covered.add(n);
      assert.equal(geometry.ringOf[n], r);
      assert.equal(geometry.faceNeighbors[n].filter((x) => members.has(x)).length, 2);
    });
    const reached = new Set([ring[0]]);
    const queue = [ring[0]];
    for (let i = 0; i < queue.length; i++) {
      for (const n of geometry.faceNeighbors[queue[i]]) {
        if (members.has(n) && !reached.has(n)) { reached.add(n); queue.push(n); }
      }
    }
    assert.equal(reached.size, 30);
  });
  assert.equal(covered.size, 600);
});

test("ring adjacency is exactly the 7-regular, 70-pair face-sharing graph", () => {
  const expected = new Map();
  for (const [x, y] of faceIncidence().values()) {
    const a = geometry.ringOf[x], b = geometry.ringOf[y];
    if (a !== b) {
      const pair = [a, b].sort((i, j) => i - j);
      expected.set(pair.join(","), pair);
    }
  }
  assert.deepEqual(geometry.ringAdjacency, [...expected.values()].sort(compareTuples));
  assert.equal(geometry.ringAdjacency.length, 70);
  canonicalTuples(geometry.ringAdjacency, 2, 20);
  const degrees = new Int32Array(20);
  for (const [a, b] of geometry.ringAdjacency) { degrees[a]++; degrees[b]++; }
  assert.ok(degrees.every((d) => d === 7));
});

test("colouring is deterministic, backtracks, succeeds at four and fails at three", () => {
  const colours = properColouring(20, geometry.ringAdjacency, 4);
  assert.ok(colours instanceof Int32Array);
  assert.equal(colours.length, 20);
  assert.ok(colours.every((c) => c >= 0 && c < 4));
  for (const [a, b] of geometry.ringAdjacency) assert.notEqual(colours[a], colours[b]);
  assert.deepEqual(properColouring(20, [...geometry.ringAdjacency].reverse(), 4), colours);
  assert.equal(properColouring(20, geometry.ringAdjacency, 3), null);
  const path = [[0, 2], [2, 3], [3, 1]];
  assert.deepEqual(properColouring(4, path, 2), new Int32Array([0, 1, 1, 0]));
  assert.equal(properColouring(1, [[0, 0]], 4), null);
  assert.equal(properColouring(1, [], 0), null);
  assert.deepEqual(properColouring(0, [], 0), new Int32Array(0));
});

test("applyMatrix uses row-major storage without changing its inputs", () => {
  const matrix = new Float64Array([0, -1, 0, 0, 1, 0, 0, 0, 0, 0, 0, -1, 0, 0, 1, 0]);
  const original = matrix.slice();
  const input = [1, 2, 3, 4];
  const output = applyMatrix(matrix, input);
  assert.ok(output instanceof Float64Array);
  assert.deepEqual(output, new Float64Array([-2, 1, -4, 3]));
  assert.deepEqual(matrix, original);
  assert.deepEqual(input, [1, 2, 3, 4]);
});

test("cap selection uses the inclusive center cut and the sorted vertex union", () => {
  const cell = 0;
  for (const cut of [-1.1, 0, 0.9, 1.1]) {
    const {capCells, capVertices} = capTurn(geometry, cell, 0.5, cut);
    const expected = geometry.cells.map((_, n) => n).filter((n) => dot(center(n), center(cell)) >= cut);
    assert.deepEqual(capCells, expected);
    assert.deepEqual(capVertices, [...new Set(expected.flatMap((n) => geometry.cells[n]))].sort((a, b) => a - b));
  }
  const boundary = dot(center(geometry.faceNeighbors[cell][0]), center(cell));
  assert.ok(capTurn(geometry, cell, 0, boundary).capCells.includes(geometry.faceNeighbors[cell][0]));
  assert.ok(capTurn(geometry, cell, 0).capCells.includes(cell));
});

test("every cell's cap interpolation is orthogonal, fixes both axes and has the stated angle", () => {
  const identity = new Float64Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
  for (let cell = 0; cell < 600; cell++) {
    const c = center(cell), v = vertex(geometry.cells[cell][0]);
    const projection = dot(v, c);
    const axis = Array.from(v, (x, d) => x - projection * c[d]);
    const norm = Math.hypot(...axis);
    const u = axis.map((x) => x / norm);
    for (const t of [0, 0.25, 0.5, 1]) {
      const {matrix, capCells} = capTurn(geometry, cell, t);
      assert.ok(matrix instanceof Float64Array);
      assert.equal(matrix.length, 16);
      if (t === 0) assert.deepEqual(matrix, identity);
      for (let i = 0; i < 4; i++) for (let j = 0; j < 4; j++) {
        let product = 0;
        for (let k = 0; k < 4; k++) product += matrix[4 * k + i] * matrix[4 * k + j];
        close(product, i === j ? 1 : 0, `orthogonality, cell ${cell}, t ${t}`);
      }
      vectorClose(applyMatrix(matrix, c), c, `fixed center ${cell}, t ${t}`);
      vectorClose(applyMatrix(matrix, u), u, `fixed vertex axis ${cell}, t ${t}`);
      close(matrix[0] + matrix[5] + matrix[10] + matrix[15], 2 + 2 * Math.cos(t * 2 * Math.PI / 3), "rotation trace");
      assert.ok(capCells.includes(cell));
    }
  }
});

test("every endpoint permutes all vertices, its tetrahedron and its cap vertices", () => {
  for (let cell = 0; cell < 600; cell++) {
    const {matrix, capVertices} = capTurn(geometry, cell, 1);
    const permutation = [];
    for (let v = 0; v < 120; v++) {
      const rotated = applyMatrix(matrix, vertex(v));
      const match = fixture.vertices.findIndex((_, n) => vertex(n).every((x, d) => Math.abs(x - rotated[d]) <= tolerance));
      assert.notEqual(match, -1, `cell ${cell}, vertex ${v}`);
      permutation.push(match);
    }
    assert.equal(new Set(permutation).size, 120);
    assert.deepEqual(capVertices.map((v) => permutation[v]).sort((a, b) => a - b), capVertices);
    const tetrahedron = geometry.cells[cell];
    assert.equal(permutation[tetrahedron[0]], tetrahedron[0]);
    assert.deepEqual(tetrahedron.map((v) => permutation[v]).sort((a, b) => a - b), tetrahedron);
    for (const v of tetrahedron.slice(1)) {
      assert.notEqual(permutation[v], v);
      assert.equal(permutation[permutation[permutation[v]]], v);
    }
  }
});
