using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Numerics;

namespace LookLab.Core;

public sealed class CellStructure
{
    public double[] Normals { get; }
    public int[][] FaceNeighbors { get; }
    public int[] VertexPositions { get; }
    public int[][] VertexCells { get; }
    public int[][] CellVertices { get; }
    public double[][] Vertices4 { get; }
    public int[][] Rings { get; }
    public int[][] RingChains { get; }
    public int[] RingOf { get; }
    public (int A, int B)[] RingGraph { get; }

    private CellStructure(double[] normals, int[] positions, int[][] vertexCells, int[][] cellVertices)
    {
        Normals = normals;
        VertexPositions = positions;
        VertexCells = vertexCells;
        CellVertices = cellVertices;
        FaceNeighbors = new int[Geometry.Cells][];
        for (int c = 0; c < Geometry.Cells; c++)
        {
            double[] dots = Enumerable.Range(0, Geometry.Cells).Select(n => Geometry.Dot(normals.AsSpan(c * 4, 4), normals.AsSpan(n * 4, 4))).ToArray();
            int[] ordered = Enumerable.Range(0, Geometry.Cells).Where(n => n != c).OrderByDescending(n => dots[n]).ThenBy(n => n).ToArray();
            double top = dots[ordered[0]];
            int[] neighbors = ordered.Where(n => Math.Abs(dots[n] - top) < 1e-9).ToArray();
            if (neighbors.Length != 4 || top - dots[ordered[4]] < 1e-6)
                throw Json.Error("normals.npy", "cell " + c + " needs four top-dot neighbors and a clear gap to the fifth");
            FaceNeighbors[c] = neighbors.OrderBy(n => n).ToArray();
        }
        for (int c = 0; c < Geometry.Cells; c++) foreach (int n in FaceNeighbors[c])
        {
            if (!FaceNeighbors[n].Contains(c)) throw Json.Error("adjacency", "asymmetric relation");
            if (CellVertices[c].Intersect(CellVertices[n]).Count() != 3) throw Json.Error("cell_vertices", "face neighbors must share exactly three vertices");
        }
        Vertices4 = DeriveVertices();
        Rings = BuildRings();
        RingOf = new int[Geometry.Cells];
        Array.Fill(RingOf, -1);
        for (int ring = 0; ring < Rings.Length; ring++) foreach (int cell in Rings[ring])
        {
            if (RingOf[cell] != -1) throw Json.Error("rings", "cover is not disjoint");
            RingOf[cell] = ring;
        }
        if (Rings.Length != 20 || Rings.Any(r => r.Length != 30) || RingOf.Any(r => r < 0))
            throw Json.Error("rings", "expected an exact cover by twenty rings of thirty cells");
        RingGraph = Pairs(RingOf);
        if (Enumerable.Range(0, 20).Any(r => RingGraph.Count(p => p.A == r || p.B == r) != 7))
            throw Json.Error("rings", "ring graph must be 7-regular");
        RingChains = Rings.Select(Chain).ToArray();
        for (int k = 4; k <= 8; k++)
            if (ProperColouring(20, RingGraph, k) == null) throw Json.Error("classes", "no colouring for " + k);
    }

    public static CellStructure Load(byte[] verifiedModel)
    {
        using var memory = new MemoryStream(verifiedModel, false);
        using var archive = new ZipArchive(memory, ZipArchiveMode.Read);
        double[] normals = NpyReader.Entry(archive, "normals.npy", "<f8", 600, 4).Doubles();
        int[] orbits = NpyReader.Entry(archive, "orbit_id.npy", "<i2", 177120).Integers();
        int[] offsets = NpyReader.Entry(archive, "face_offsets.npy", "<i4", 177121).Integers();
        int[] faces = NpyReader.Entry(archive, "face_values.npy", "<i2", 259800).Integers();
        if (offsets[0] != 0 || offsets[^1] != faces.Length || offsets.Zip(offsets.Skip(1)).Any(p => p.First > p.Second))
            throw Json.Error("face_offsets.npy", "invalid incidence offsets");
        int[] positions = Enumerable.Range(0, orbits.Length).Where(p => orbits[p] == 34).ToArray();
        if (positions.Length != 120) throw Json.Error("vertex_positions", "vertex orbit must contain 120 positions");
        var vertexCells = new int[120][];
        var cellVertices = Enumerable.Range(0, 600).Select(_ => new List<int>()).ToArray();
        for (int v = 0; v < 120; v++)
        {
            int p = positions[v];
            int[] cells = faces.AsSpan(offsets[p], offsets[p + 1] - offsets[p]).ToArray();
            if (cells.Length != 20 || cells.Distinct().Count() != 20 || cells.Any(c => c < 0 || c >= 600))
                throw Json.Error("vertex_cells", "each vertex must meet twenty distinct cells");
            vertexCells[v] = cells;
            foreach (int c in cells) cellVertices[c].Add(v);
        }
        if (cellVertices.Any(v => v.Count != 4)) throw Json.Error("cell_vertices", "each cell must meet four vertices");
        return new CellStructure(normals, positions, vertexCells, cellVertices.Select(v => v.ToArray()).ToArray());
    }

    private double[][] DeriveVertices()
    {
        var vertices = new double[120][];
        for (int v = 0; v < 120; v++)
        {
            var direction = new double[4];
            double plane = 0;
            foreach (int c in VertexCells[v])
            {
                for (int i = 0; i < 4; i++) direction[i] += Normals[c * 4 + i] / 20;
                plane += Geometry.Dot(Normals.AsSpan(c * 4, 4), Normals.AsSpan(c * 4, 4)) / 20;
            }
            double denominator = VertexCells[v].Sum(c => Geometry.Dot(Normals.AsSpan(c * 4, 4), direction)) / 20;
            if (denominator <= 0) throw Json.Error("vertices", "invalid incident cell planes");
            vertices[v] = direction.Select(x => x * plane / denominator).ToArray();
            for (int c = 0; c < 600; c++)
            {
                double residual = Geometry.Dot(Normals.AsSpan(c * 4, 4), vertices[v]) - Geometry.Dot(Normals.AsSpan(c * 4, 4), Normals.AsSpan(c * 4, 4));
                if (residual > 1e-9 || (Math.Abs(residual) < 1e-9) != VertexCells[v].Contains(c))
                    throw Json.Error("vertices", "retained planes disagree with incidence");
            }
        }
        double radius2 = Geometry.Dot(vertices[0], vertices[0]);
        if (vertices.Any(v => Math.Abs(Geometry.Dot(v, v) - radius2) > 1e-8)) throw Json.Error("vertices", "unequal radii");
        for (int c = 0; c < 600; c++) for (int d = 0; d < 4; d++)
            if (Math.Abs(CellVertices[c].Sum(v => vertices[v][d]) / 4 - Normals[c * 4 + d]) > 1e-9)
                throw Json.Error("vertices", "tetrahedral center disagrees with cell pole");
        return vertices;
    }

    private (int A, int B)[] Pairs(int[] owner)
    {
        var pairs = new HashSet<(int, int)>();
        for (int c = 0; c < 600; c++) foreach (int n in FaceNeighbors[c])
            if (owner[c] != owner[n]) pairs.Add((Math.Min(owner[c], owner[n]), Math.Max(owner[c], owner[n])));
        return pairs.OrderBy(p => p.Item1).ThenBy(p => p.Item2).ToArray();
    }

    private static (int, int, int) Face(int a, int b, int c)
    {
        int[] values = { a, b, c };
        Array.Sort(values);
        return (values[0], values[1], values[2]);
    }

    private int[][] BuildRings()
    {
        var faces = new Dictionary<(int, int, int), List<int>>();
        for (int cell = 0; cell < 600; cell++) for (int omit = 0; omit < 4; omit++)
        {
            int[] face = CellVertices[cell].Where((_, i) => i != omit).ToArray();
            var key = Face(face[0], face[1], face[2]);
            if (!faces.TryGetValue(key, out List<int>? owners)) faces.Add(key, owners = new List<int>());
            owners.Add(cell);
        }
        if (faces.Count != 1200 || faces.Values.Any(v => v.Count != 2)) throw Json.Error("faces", "two owners per face required");
        var unique = new SortedSet<int[]>(Comparer<int[]>.Create(CompareTuples));
        for (int start = 0; start < 600; start++) foreach (int[] initial in Permutations(CellVertices[start]))
        {
            int[] window = (int[])initial.Clone();
            int current = start;
            var visited = new int[30];
            for (int step = 0; step < 30; step++)
            {
                visited[step] = current;
                List<int> owners = faces[Face(window[1], window[2], window[3])];
                current = owners[0] == current ? owners[1] : owners[0];
                int next = CellVertices[current].Single(v => v != window[1] && v != window[2] && v != window[3]);
                window = new[] { window[1], window[2], window[3], next };
            }
            if (window.SequenceEqual(initial) && visited.Distinct().Count() == 30)
            {
                Array.Sort(visited);
                unique.Add(visited);
            }
        }
        int[][] candidates = unique.ToArray();
        BigInteger[] bits = Enumerable.Range(0, 600).Select(n => BigInteger.One << n).ToArray();
        BigInteger[] masks = candidates.Select(r => r.Aggregate(BigInteger.Zero, (mask, n) => mask | bits[n])).ToArray();
        var byCell = Enumerable.Range(0, 600).Select(_ => new List<int>()).ToArray();
        for (int r = 0; r < candidates.Length; r++) foreach (int c in candidates[r]) byCell[c].Add(r);
        var choice = new List<int>();
        bool Search(BigInteger used)
        {
            if (choice.Count == 20)
            {
                var owner = new int[600];
                for (int r = 0; r < 20; r++) foreach (int c in candidates[choice[r]]) owner[c] = r;
                var pairs = Pairs(owner);
                return Enumerable.Range(0, 20).All(r => pairs.Count(p => p.A == r || p.B == r) == 7);
            }
            int n = 0;
            while ((used & bits[n]) != 0) n++;
            foreach (int r in byCell[n])
            {
                if ((used & masks[r]) != 0) continue;
                choice.Add(r);
                if (Search(used | masks[r])) return true;
                choice.RemoveAt(choice.Count - 1);
            }
            return false;
        }
        if (!Search(BigInteger.Zero)) throw Json.Error("rings", "no 7-regular ring cover found");
        return choice.Select(r => candidates[r]).ToArray();
    }

    private int[] Chain(int[] ring)
    {
        if (ring.Any(c => FaceNeighbors[c].Count(ring.Contains) != 2)) throw Json.Error("rings", "not a closed face-neighbor chain");
        var chain = new List<int> { ring[0] };
        int previous = -1, current = ring[0];
        do
        {
            int next = FaceNeighbors[current].First(n => ring.Contains(n) && n != previous);
            previous = current;
            current = next;
            if (current != ring[0]) chain.Add(current);
        } while (current != ring[0] && chain.Count <= 30);
        if (chain.Count != 30 || chain.Distinct().Count() != 30) throw Json.Error("rings", "disconnected chain");
        return chain.ToArray();
    }

    private static int CompareTuples(int[] a, int[] b)
    {
        for (int i = 0; i < Math.Min(a.Length, b.Length); i++) if (a[i] != b[i]) return a[i].CompareTo(b[i]);
        return a.Length.CompareTo(b.Length);
    }

    private static IEnumerable<int[]> Permutations(int[] values)
    {
        if (values.Length == 0) { yield return Array.Empty<int>(); yield break; }
        for (int i = 0; i < values.Length; i++) foreach (int[] tail in Permutations(values.Where((_, j) => j != i).ToArray()))
            yield return new[] { values[i] }.Concat(tail).ToArray();
    }

    public static int[]? ProperColouring(int count, IEnumerable<(int A, int B)> pairs, int classes)
    {
        if (count < 1 || classes < 1) throw Json.Error("classes", "positive counts required");
        var neighbors = Enumerable.Range(0, count).Select(_ => new List<int>()).ToArray();
        foreach (var (a, b) in pairs)
        {
            if (a < 0 || b < 0 || a >= count || b >= count) throw Json.Error("ring graph", "invalid vertex");
            if (a == b) return null;
            neighbors[a].Add(b);
            neighbors[b].Add(a);
        }
        var colours = Enumerable.Repeat(-1, count).ToArray();
        bool Search(int v)
        {
            if (v == count) return true;
            for (int colour = 0; colour < classes; colour++)
            {
                if (neighbors[v].Any(u => colours[u] == colour)) continue;
                colours[v] = colour;
                if (Search(v + 1)) return true;
            }
            colours[v] = -1;
            return false;
        }
        return Search(0) ? colours : null;
    }
}
