// Synthetic regular 600-cell geometry for Taste Lab, not the cut-sticker model.
// Construction: research/audit/verify_regular_geometry.py (Z[phi], radius 2).
// Helices and cover order: tools/tastelab/sim/geometry_fixture.py. Enumerate
// closed Boerdijk-Coxeter helices, then take the first exact cover, searching
// the lowest uncovered cell and sorted helices, whose ring graph is 7-regular.

export const PHI = (1 + Math.sqrt(5)) / 2;

function compareTuples(a, b) {
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return a[i] - b[i];
  }
  return 0;
}

function permutations(values) {
  if (values.length === 0) return [[]];
  const result = [];
  for (let i = 0; i < values.length; i++) {
    const rest = values.filter((_, j) => i !== j);
    for (const tail of permutations(rest)) result.push([values[i], ...tail]);
  }
  return result;
}

function exactVertices() {
  // Flatten the four [a, b] pairs. Numeric lexicographic order is exactly
  // Python's tuple-of-pairs order; sorting floating values would change IDs.
  const vertices = new Map();
  const add = (v) => vertices.set(v.join(","), v);
  for (let axis = 0; axis < 4; axis++) {
    for (const sign of [-1, 1]) {
      const v = Array(8).fill(0);
      v[2 * axis] = 2 * sign;
      add(v);
    }
  }
  for (const a of [-1, 1]) for (const b of [-1, 1]) {
    for (const c of [-1, 1]) for (const d of [-1, 1]) {
      add([a, 0, b, 0, c, 0, d, 0]);
    }
  }
  const evenOrders = permutations([0, 1, 2, 3]).filter((p) => {
    let inversions = 0;
    for (let i = 0; i < 4; i++) for (let j = i + 1; j < 4; j++) {
      if (p[i] > p[j]) inversions++;
    }
    return inversions % 2 === 0;
  });
  for (const a of [-1, 1]) for (const b of [-1, 1]) {
    for (const c of [-1, 1]) {
      const v = [[0, 0], [a, 0], [0, b], [-c, c]];
      for (const order of evenOrders) add(order.flatMap((i) => v[i]));
    }
  }
  return [...vertices.values()].sort(compareTuples);
}

function exactDot(v, w) {
  let a = 0, b = 0;
  for (let i = 0; i < 8; i += 2) {
    // (a + b*phi)(c + d*phi), with phi^2 = phi + 1.
    a += v[i] * w[i] + v[i + 1] * w[i + 1];
    b += v[i] * w[i + 1] + v[i + 1] * w[i] + v[i + 1] * w[i + 1];
  }
  return [a, b];
}

function ringPairs(faces, ringOf) {
  const pairs = new Map();
  for (const [x, y] of faces.values()) {
    const a = Math.min(ringOf[x], ringOf[y]);
    const b = Math.max(ringOf[x], ringOf[y]);
    if (a !== b) pairs.set(`${a},${b}`, [a, b]);
  }
  return [...pairs.values()].sort(compareTuples);
}

function buildRings(cells, faces) {
  const unique = new Map();
  for (let start = 0; start < cells.length; start++) {
    for (const initial of permutations(cells[start])) {
      let window = initial;
      let current = start;
      const visited = [];
      for (let step = 0; step < 30; step++) {
        visited.push(current);
        const face = window.slice(1).sort((a, b) => a - b);
        const owners = faces.get(face.join(","));
        current = owners[0] === current ? owners[1] : owners[0];
        const next = cells[current].find((v) => !face.includes(v));
        window = [window[1], window[2], window[3], next];
      }
      if (compareTuples(window, initial) === 0 && new Set(visited).size === 30) {
        const ring = visited.sort((a, b) => a - b);
        unique.set(ring.join(","), ring);
      }
    }
  }
  const candidates = [...unique.values()].sort(compareTuples);
  const bits = Array.from({length: cells.length}, (_, i) => 1n << BigInt(i));
  const masks = candidates.map((ring) => ring.reduce((mask, n) => mask | bits[n], 0n));
  const byCell = Array.from({length: cells.length}, () => []);
  candidates.forEach((ring, i) => ring.forEach((n) => byCell[n].push(i)));
  const choice = [];

  function sevenRegular() {
    const ringOf = new Int32Array(cells.length);
    choice.forEach((r, i) => candidates[r].forEach((n) => { ringOf[n] = i; }));
    const degree = new Int32Array(choice.length);
    for (const [a, b] of ringPairs(faces, ringOf)) { degree[a]++; degree[b]++; }
    return degree.every((d) => d === 7);
  }

  function search(used) {
    if (choice.length === 20) {
      return sevenRegular() ? choice.map((r) => candidates[r]) : null;
    }
    let n = 0;
    while ((used & bits[n]) !== 0n) n++;
    for (const r of byCell[n]) {
      if ((used & masks[r]) !== 0n) continue;
      choice.push(r);
      const found = search(used | masks[r]);
      if (found !== null) return found;
      choice.pop();
    }
    return null;
  }

  const cover = search(0n);
  if (cover === null) throw new Error("No 7-regular 600-cell ring cover found");
  return cover;
}

export function buildGeometry() {
  const exact = exactVertices();
  const vertices = new Float64Array(exact.length * 4);
  exact.forEach((v, i) => {
    for (let d = 0; d < 4; d++) vertices[4 * i + d] = (v[2 * d] + v[2 * d + 1] * PHI) / 2;
  });
  const adjacency = Array.from({length: exact.length}, () => new Set());
  const edges = [];
  for (let i = 0; i < exact.length; i++) for (let j = i + 1; j < exact.length; j++) {
    const [a, b] = exactDot(exact[i], exact[j]);
    if (a === 0 && b === 2) {
      edges.push([i, j]);
      adjacency[i].add(j);
      adjacency[j].add(i);
    }
  }
  const triangles = [];
  for (const [i, j] of edges) for (let k = j + 1; k < exact.length; k++) {
    if (adjacency[i].has(k) && adjacency[j].has(k)) triangles.push([i, j, k]);
  }
  const cells = [];
  for (const [i, j, k] of triangles) for (let l = k + 1; l < exact.length; l++) {
    if (adjacency[i].has(l) && adjacency[j].has(l) && adjacency[k].has(l)) cells.push([i, j, k, l]);
  }
  const faces = new Map();
  const cellCenters = new Float64Array(cells.length * 4);
  cells.forEach((cell, n) => {
    const sum = [0, 0, 0, 0];
    for (const v of cell) for (let d = 0; d < 4; d++) sum[d] += vertices[4 * v + d];
    cellCenters.set(normalize(sum), 4 * n);
    for (let omit = 0; omit < 4; omit++) {
      const key = cell.filter((_, i) => i !== omit).join(",");
      if (!faces.has(key)) faces.set(key, []);
      faces.get(key).push(n);
    }
  });
  const faceNeighbors = Array.from({length: cells.length}, () => []);
  for (const [a, b] of faces.values()) {
    faceNeighbors[a].push(b);
    faceNeighbors[b].push(a);
  }
  faceNeighbors.forEach((neighbors) => neighbors.sort((a, b) => a - b));
  const rings = buildRings(cells, faces);
  const ringOf = new Int32Array(cells.length);
  rings.forEach((ring, i) => ring.forEach((n) => { ringOf[n] = i; }));
  const ringAdjacency = ringPairs(faces, ringOf);
  return {vertices, edges, triangles, cells, cellCenters, faceNeighbors, rings, ringOf, ringAdjacency};
}

export function properColouring(n, pairs, k) {
  const neighbors = Array.from({length: n}, () => []);
  for (const [a, b] of pairs) {
    if (a === b) return null;
    neighbors[a].push(b);
    neighbors[b].push(a);
  }
  const colours = new Int32Array(n).fill(-1);
  function search(v) {
    if (v === n) return true;
    for (let colour = 0; colour < k; colour++) {
      if (neighbors[v].some((u) => colours[u] === colour)) continue;
      colours[v] = colour;
      if (search(v + 1)) return true;
    }
    colours[v] = -1;
    return false;
  }
  return search(0) ? colours : null;
}

function dot(v, w) {
  return v[0] * w[0] + v[1] * w[1] + v[2] * w[2] + v[3] * w[3];
}

function normalize(v) {
  const norm = Math.hypot(...v);
  return v.map((x) => x / norm);
}

function orientedNormal(c, u, p) {
  // Cofactors give q so that the orthonormal frame (c, u, p, q) has
  // positive determinant. This fixes the sign of the interpolated angle.
  return normalize([0, 1, 2, 3].map((omit) => {
    const [i, j, k] = [0, 1, 2, 3].filter((d) => d !== omit);
    const determinant = c[i] * (u[j] * p[k] - u[k] * p[j])
      - c[j] * (u[i] * p[k] - u[k] * p[i])
      + c[k] * (u[i] * p[j] - u[j] * p[i]);
    return (omit % 2 === 0 ? -1 : 1) * determinant;
  }));
}

export function capTurn(geometry, cell, t, cut = 0.9) {
  const {vertices, cells, cellCenters} = geometry;
  const c = cellCenters.subarray(4 * cell, 4 * cell + 4);
  const vertex = (n) => vertices.subarray(4 * n, 4 * n + 4);
  const v = vertex(cells[cell][0]);
  const alongC = dot(v, c);
  const u = normalize(Array.from(v, (x, i) => x - alongC * c[i]));
  const w = vertex(cells[cell][1]);
  const wc = dot(w, c), wu = dot(w, u);
  const p = normalize(Array.from(w, (x, i) => x - wc * c[i] - wu * u[i]));
  const q = orientedNormal(c, u, p);
  const angle = t * 2 * Math.PI / 3;
  const cosine = Math.cos(angle), sine = Math.sin(angle);
  const matrix = new Float64Array(16);
  for (let i = 0; i < 4; i++) for (let j = 0; j < 4; j++) {
    matrix[4 * i + j] = (i === j ? 1 : 0)
      + (cosine - 1) * (p[i] * p[j] + q[i] * q[j])
      + sine * (q[i] * p[j] - p[i] * q[j]);
  }
  const capCells = [];
  const union = new Set();
  for (let n = 0; n < cells.length; n++) {
    if (dot(cellCenters.subarray(4 * n, 4 * n + 4), c) >= cut) {
      capCells.push(n);
      for (const v of cells[n]) union.add(v);
    }
  }
  const capVertices = [...union].sort((a, b) => a - b);
  return {matrix, capCells, capVertices};
}

export function applyMatrix(matrix, vec4) {
  const result = new Float64Array(4);
  for (let i = 0; i < 4; i++) for (let j = 0; j < 4; j++) result[i] += matrix[4 * i + j] * vec4[j];
  return result;
}
