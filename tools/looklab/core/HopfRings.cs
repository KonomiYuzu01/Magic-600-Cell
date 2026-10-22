using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;

namespace LookLab.Core;

public sealed record HopfCover(string Side, int Index, int[] Helices, double[] J);
public sealed record HopfFacts(IReadOnlyDictionary<int, int> RightClasses, IReadOnlyDictionary<int, int> LeftClasses,
    int RightCovers, int LeftCovers, bool CoversPartitionHelices, int CoverGraphDegree, int CoverGraphEdges,
    int ScrewOrder, double ScrewDegrees, (double Angle, int Count)[] BaseAngles, double AdjacentBaseAngle,
    bool HalfStepsIntegral, bool ParityAlternates, bool Brick, int BeadClasses, double ChartDeterminant, int ChainsAlongJ);

public sealed class HopfRings
{
    public string Side { get; }
    public double[] J { get; }
    public double[] K { get; }
    public double[] JK { get; }
    public int[] RingHelix { get; }
    public int[][] Rings { get; }
    public int[] RingOf { get; } = new int[600];
    public int[] HalfStep { get; } = new int[600];
    public double[][] Origins { get; } = new double[20][];
    public double[][] BasePoints { get; } = new double[20][];
    public double[][] E1 { get; } = new double[20][];
    public double[][] E2 { get; } = new double[20][];
    public (int A, int B)[] Graph { get; }
    public double[][] Bead { get; }
    public IReadOnlyList<HopfCover> Covers { get; }
    public double[][] RightInvariants { get; } = new double[240][];
    public double[][] LeftInvariants { get; } = new double[240][];
    public HopfFacts Facts { get; }
    public int ChainsAlongJ => Facts.ChainsAlongJ;
    private readonly double[][] charts = new double[600][];

    public HopfRings(CellStructure structure)
    {
        int[][] helices = structure.Helices;
        Require(helices.Length == 240, "expected 240 helices");
        double[][] centres = Enumerable.Range(0, 600).Select(c => Unit(structure.Normals.AsSpan(c * 4, 4).ToArray())).ToArray();
        double[][] vertices = structure.Vertices4.Select(Unit).ToArray();
        int[][] chains = helices.Select(structure.Chain).ToArray();
        var u = new double[240][]; var v = new double[240][];
        for (int h = 0; h < 240; h++)
        {
            var a = new double[4]; var b = new double[4];
            for (int step = 0; step < 30; step++) for (int i = 0; i < 4; i++)
            {
                double t = step * Math.PI / 15;
                a[i] += Math.Cos(t) * centres[chains[h][step]][i];
                b[i] += Math.Sin(t) * centres[chains[h][step]][i];
            }
            Require(Math.Abs(Geometry.Dot(a, b)) / (Norm(a) * Norm(b)) < 1e-9 && Math.Abs(Norm(a) / Norm(b) - 1) < 1e-9,
                "Fourier vectors must be orthogonal and equal in length");
            u[h] = Unit(a); v[h] = Unit(b);
            RightInvariants[h] = Multiply(Conj(u[h]), v[h]);
            LeftInvariants[h] = Multiply(v[h], Conj(u[h]));
            foreach (double[] invariant in new[] { RightInvariants[h], LeftInvariants[h] })
                Require(Math.Abs(invariant[0]) < 1e-9 && Math.Abs(Norm(invariant) - 1) < 1e-9, "invariant must be pure and unit");
        }
        List<int[]> right = Classes(RightInvariants), left = Classes(LeftInvariants);
        IReadOnlyDictionary<int, int> rightSizes = ClassSizes(right), leftSizes = ClassSizes(left);
        var covers = new List<HopfCover>();
        foreach (var (side, groups, invariants) in new[] { ("right", right, RightInvariants), ("left", left, LeftInvariants) })
        {
            int index = 0;
            foreach (int[] group in groups.Where(g => g.Length == 20).OrderBy(g => g[0]))
            {
                Require(group.SelectMany(h => helices[h]).OrderBy(c => c).SequenceEqual(Enumerable.Range(0, 600)), "cover must partition cells");
                var graph = RingGraph(structure, group.Select(h => helices[h]).ToArray());
                Require(graph.Length == 30 && Enumerable.Range(0, 20).All(r => graph.Count(p => p.A == r || p.B == r) == 3),
                    "cover graph must be 3-regular with 30 edges");
                covers.Add(new HopfCover(side, index++, group, (double[])invariants[group[0]].Clone()));
            }
            Require(index == 6, "expected six covers per side");
        }
        Require(covers.SelectMany(c => c.Helices).OrderBy(h => h).SequenceEqual(Enumerable.Range(0, 240)), "covers must partition helices");
        Covers = covers.AsReadOnly();
        HopfCover[] chosen = covers.Where(c => c.Helices.Contains(0)).ToArray();
        Require(chosen.Length == 1, "helix zero must belong to exactly one cover");
        Side = chosen[0].Side;
        Require(Side == "right", "chosen cover is left; mirrored chart is not defined");
        J = (double[])RightInvariants[0].Clone();
        int[] cover = chosen[0].Helices;
        var coverGraph = RingGraph(structure, cover.Select(h => helices[h]).ToArray());
        var path = new List<int> { Array.IndexOf(cover, 0) };
        bool Search()
        {
            if (path.Count == 20) return true;
            int last = path[^1];
            foreach (int next in coverGraph.Where(p => p.A == last || p.B == last).Select(p => p.A == last ? p.B : p.A).OrderBy(r => cover[r]))
            {
                if (path.Contains(next)) continue;
                path.Add(next);
                if (Search()) return true;
                path.RemoveAt(path.Count - 1);
            }
            return false;
        }
        Require(Search(), "no Hamiltonian ring path");
        RingHelix = path.Select(r => cover[r]).ToArray();
        Rings = new int[20][];
        for (int r = 0; r < 20; r++)
        {
            int h = RingHelix[r];
            double[] Project(double[] x) => Enumerable.Range(0, 4).Select(i => Geometry.Dot(x, u[h]) * u[h][i] + Geometry.Dot(x, v[h]) * v[h][i]).ToArray();
            foreach (double[] x in new[] { Multiply(u[h], J), Multiply(v[h], J) })
                Require(Distance(x, Project(x)) < 1e-9, "ring plane is not a right fibre");
            Origins[r] = Unit(Project(r == 0 ? centres[chains[h][0]] : Origins[r - 1]));
            double[] tangent = Multiply(Origins[r], J);
            foreach (int c in helices[h])
            {
                double t = Math.Atan2(Geometry.Dot(centres[c], tangent), Geometry.Dot(centres[c], Origins[r])) * 180 / Math.PI;
                double half = ((t % 360 + 360) % 360) / 6;
                Require(Math.Abs(half - Math.Round(half)) < 1e-9, "nonintegral half-step");
                HalfStep[c] = (int)Math.Round(half) % 60;
                RingOf[c] = r;
            }
            Rings[r] = helices[h].OrderBy(c => HalfStep[c]).ToArray();
            Require(Rings[r].Select(c => HalfStep[c]).Distinct().Count() == 30 && Rings[r].Select(c => HalfStep[c] % 2).Distinct().Count() == 1,
                "ring must have thirty distinct half-steps of one parity");
            Require(r == 0 || HalfStep[Rings[r][0]] % 2 != HalfStep[Rings[r - 1][0]] % 2, "path parity must alternate");
            for (int i = 0; i < 30; i++) Require(structure.FaceNeighbors[Rings[r][i]].Contains(Rings[r][(i + 1) % 30]), "half-step order is not face-connected");
        }
        Graph = RingGraph(structure, Rings);
        for (int r = 0; r < 19; r++)
        {
            int[] offsets = Rings[r].SelectMany(c => structure.FaceNeighbors[c].Where(n => RingOf[n] == r + 1)
                .Select(n => (HalfStep[n] - HalfStep[c] + 90) % 60 - 30)).ToArray();
            Require(offsets.Length == 20 && offsets.All(d => d == -1 || d == 1) && offsets.Distinct().Count() == 2, "path lacks brick pattern");
        }
        int along = 0, order = 0; double degrees = 0;
        foreach (int h in RingHelix)
        {
            int[] steps = Enumerable.Range(0, 30).Select(i => (HalfStep[chains[h][(i + 1) % 30]] - HalfStep[chains[h][i]] + 60) % 60).ToArray();
            Require(steps.All(s => s == 2) || steps.All(s => s == 58), "canonical chain changes direction along J");
            if (steps[0] == 2) along++;
            (order, degrees) = Screw(chains[h].Select(c => centres[c]).ToArray(), centres);
        }
        for (int r = 0; r < 20; r++)
        {
            BasePoints[r] = Multiply(Multiply(Origins[r], J), Conj(Origins[r])).Skip(1).ToArray();
            Require(Math.Abs(Norm(BasePoints[r]) - 1) < 1e-9, "base point must be unit");
        }
        var expectedAngles = new[] { (0.0, 1), (41.81, 3), (70.53, 6), (109.47, 6), (138.19, 3), (180.0, 1) };
        (double Angle, int Count)[] Angles(int r) => BasePoints.Select(b => Math.Round(Angle(BasePoints[r], b), 2)).GroupBy(a => a)
            .OrderBy(g => g.Key).Select(g => (g.Key, g.Count())).ToArray();
        for (int r = 0; r < 20; r++) Require(Angles(r).SequenceEqual(expectedAngles), "base angle multiset differs from a dodecahedron");
        double[] adjacent = Graph.Select(p => Math.Round(Angle(BasePoints[p.A], BasePoints[p.B]), 2)).Distinct().ToArray();
        Require(adjacent.Length == 1 && adjacent[0] == 41.81, "adjacent base angle differs");
        K = Array.Empty<double>();
        for (int axis = 1; axis < 4; axis++)
        {
            double[] candidate = Enumerable.Range(0, 4).Select(i => (i == axis ? 1.0 : 0) - J[axis] * J[i]).ToArray();
            if (Norm(candidate) <= .5) continue;
            K = Unit(candidate); break;
        }
        Require(K.Length == 4, "no transverse quaternion K");
        JK = Multiply(J, K);
        for (int r = 0; r < 20; r++)
        {
            E1[r] = Multiply(Multiply(Origins[r], K), Conj(Origins[r])).Skip(1).ToArray();
            E2[r] = Multiply(Multiply(Origins[r], JK), Conj(Origins[r])).Skip(1).ToArray();
        }
        var beads = new double[600][][];
        double determinant = 0;
        for (int c = 0; c < 600; c++)
        {
            double a = (18 - 72 * HalfStep[c]) * Math.PI / 180;
            double[] e = Multiply(centres[c], new[] { Math.Cos(a), Math.Sin(a) * J[1], Math.Sin(a) * J[2], Math.Sin(a) * J[3] });
            double[][] frame = { centres[c], Multiply(centres[c], J), Multiply(e, K), Multiply(e, JK) };
            for (int i = 0; i < 4; i++) for (int j = 0; j < 4; j++)
                Require(Math.Abs(Geometry.Dot(frame[i], frame[j]) - (i == j ? 1 : 0)) < 1e-9, "chart frame is not orthonormal");
            determinant = Determinant(frame);
            Require(Math.Abs(determinant - 1) < 1e-9, "chart determinant must be +1");
            charts[c] = frame.Skip(1).SelectMany(row => row).ToArray();
            beads[c] = structure.CellVertices[c].OrderBy(vx => vx).Select(vx =>
            {
                double d = Geometry.Dot(vertices[vx], centres[c]);
                double[] point = vertices[vx].Select((x, i) => x / d - centres[c][i]).ToArray();
                return frame.Skip(1).Select(row => Geometry.Dot(row, point)).ToArray();
            }).ToArray();
        }
        Bead = beads[0];
        foreach (double[][] bead in beads)
        {
            int[] matches = bead.Select(p => Array.FindIndex(Bead, b => Distance(p, b) < 1e-9)).ToArray();
            Require(matches.All(i => i >= 0) && matches.Distinct().Count() == 4, "cell chart points differ from the reference bead");
        }
        Facts = new HopfFacts(rightSizes, leftSizes, covers.Count(c => c.Side == "right"), covers.Count(c => c.Side == "left"), true,
            coverGraph.Count(p => p.A == 0 || p.B == 0), coverGraph.Length, order, degrees, Angles(0), adjacent[0], true, true, true, 1,
            Math.Round(determinant, 9), along);
    }

    public double[] Chart(int cell)
    {
        if (cell < 0 || cell >= 600) throw Json.Error("hopf", "cell outside 0 to 599");
        return (double[])charts[cell].Clone();
    }

    private static void Require(bool condition, string message)
    {
        if (!condition) throw Json.Error("hopf", message);
    }
    private static double Norm(double[] a) => Math.Sqrt(Geometry.Dot(a, a));
    private static double[] Unit(double[] a)
    {
        double n = Norm(a);
        Require(double.IsFinite(n) && n > 0, "nonzero finite vector required");
        return a.Select(x => x / n).ToArray();
    }
    private static double Distance(double[] a, double[] b) => Math.Sqrt(a.Select((x, i) => (x - b[i]) * (x - b[i])).Sum());
    private static double Angle(double[] a, double[] b) => Math.Acos(Math.Clamp(Geometry.Dot(a, b) / (Norm(a) * Norm(b)), -1, 1)) * 180 / Math.PI;
    private static double[] Conj(double[] a) => new[] { a[0], -a[1], -a[2], -a[3] };
    private static double[] Multiply(double[] a, double[] b) => new[] {
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0] };

    private static List<int[]> Classes(double[][] invariants)
    {
        var groups = new List<List<int>>();
        for (int h = 0; h < invariants.Length; h++)
        {
            List<int>? group = groups.FirstOrDefault(g => Math.Min(Distance(invariants[h], invariants[g[0]]),
                Distance(invariants[h], invariants[g[0]].Select(x => -x).ToArray())) < 1e-6);
            if (group == null) groups.Add(group = new List<int>());
            group.Add(h);
        }
        return groups.Select(g => g.ToArray()).ToList();
    }

    private static IReadOnlyDictionary<int, int> ClassSizes(List<int[]> groups)
    {
        var sizes = groups.GroupBy(g => g.Length).ToDictionary(g => g.Key, g => g.Count());
        Require(groups.Count == 16 && sizes.Count == 2 && sizes.GetValueOrDefault(12) == 10 && sizes.GetValueOrDefault(20) == 6,
            "expected ten classes of twelve and six of twenty per side");
        return new ReadOnlyDictionary<int, int>(sizes);
    }

    private static (int A, int B)[] RingGraph(CellStructure structure, int[][] rings)
    {
        var owner = new int[600];
        for (int r = 0; r < rings.Length; r++) foreach (int c in rings[r]) owner[c] = r;
        var pairs = new HashSet<(int, int)>();
        for (int c = 0; c < 600; c++) foreach (int n in structure.FaceNeighbors[c])
            if (owner[c] != owner[n]) pairs.Add((Math.Min(owner[c], owner[n]), Math.Max(owner[c], owner[n])));
        return pairs.OrderBy(p => p.Item1).ThenBy(p => p.Item2).ToArray();
    }

    private static double Determinant(double[][] rows)
    {
        double[][] m = rows.Select(row => (double[])row.Clone()).ToArray();
        double determinant = 1;
        for (int i = 0; i < 4; i++)
        {
            int pivot = Enumerable.Range(i, 4 - i).OrderByDescending(j => Math.Abs(m[j][i])).First();
            if (pivot != i) { (m[i], m[pivot]) = (m[pivot], m[i]); determinant = -determinant; }
            determinant *= m[i][i];
            if (m[i][i] == 0) return 0;
            for (int j = i + 1; j < 4; j++)
            {
                double factor = m[j][i] / m[i][i];
                for (int k = i + 1; k < 4; k++) m[j][k] -= factor * m[i][k];
            }
        }
        return determinant;
    }

    private static (int Order, double Degrees) Screw(double[][] chain, double[][] centres)
    {
        var normal = new double[4, 8]; var cross = new double[4, 4];
        for (int row = 0; row < 4; row++)
        {
            normal[row, row + 4] = 1;
            for (int col = 0; col < 4; col++) for (int k = 0; k < 30; k++)
            {
                normal[row, col] += chain[k][row] * chain[k][col];
                cross[row, col] += chain[(k + 1) % 30][row] * chain[k][col];
            }
        }
        for (int i = 0; i < 4; i++)
        {
            int pivot = Enumerable.Range(i, 4 - i).OrderByDescending(j => Math.Abs(normal[j, i])).First();
            Require(Math.Abs(normal[pivot, i]) > 1e-12, "singular screw normal equations");
            for (int col = 0; col < 8; col++) (normal[i, col], normal[pivot, col]) = (normal[pivot, col], normal[i, col]);
            double divisor = normal[i, i];
            for (int col = 0; col < 8; col++) normal[i, col] /= divisor;
            for (int row = 0; row < 4; row++)
            {
                if (row == i) continue;
                double factor = normal[row, i];
                for (int col = 0; col < 8; col++) normal[row, col] -= factor * normal[i, col];
            }
        }
        var matrix = new double[16];
        for (int row = 0; row < 4; row++) for (int col = 0; col < 4; col++) for (int k = 0; k < 4; k++)
            matrix[col * 4 + row] += cross[row, k] * normal[k, col + 4];
        for (int k = 0; k < 30; k++) Require(Distance(Geometry.Multiply(matrix, chain[k]), chain[(k + 1) % 30]) < 1e-9, "screw residual");
        for (int i = 0; i < 4; i++) for (int j = 0; j < 4; j++)
            Require(Math.Abs(Geometry.Dot(matrix.AsSpan(i * 4, 4), matrix.AsSpan(j * 4, 4)) - (i == j ? 1 : 0)) < 1e-9, "screw is not orthogonal");
        (long, long, long, long) Key(double[] x) => ((long)Math.Round(x[0] * 1e8), (long)Math.Round(x[1] * 1e8), (long)Math.Round(x[2] * 1e8), (long)Math.Round(x[3] * 1e8));
        var lookup = centres.ToDictionary(Key);
        var mapped = new HashSet<(long, long, long, long)>();
        foreach (double[] centre in centres)
        {
            double[] image = Geometry.Multiply(matrix, centre);
            Require(lookup.TryGetValue(Key(image), out double[]? target) && Distance(image, target) < 1e-9, "screw does not preserve the centres");
            mapped.Add(Key(image));
        }
        Require(mapped.Count == 600, "screw is not a centre permutation");
        double[] power = Geometry.Identity();
        int order = 0;
        for (int n = 1; n <= 30; n++)
        {
            var next = new double[16];
            for (int col = 0; col < 4; col++) for (int row = 0; row < 4; row++) for (int k = 0; k < 4; k++)
                next[col * 4 + row] += matrix[k * 4 + row] * power[col * 4 + k];
            power = next;
            if (power.Select((x, i) => Math.Abs(x - (i % 5 == 0 ? 1 : 0))).Max() < 1e-9) { order = n; break; }
        }
        Require(order == 30, "screw order must be thirty");
        // The chain's dominant Fourier plane fixes the positive 12-degree advance.
        double[] u = new double[4], v = new double[4];
        for (int k = 0; k < 30; k++) for (int i = 0; i < 4; i++)
        {
            u[i] += Math.Cos(k * Math.PI / 15) * chain[k][i];
            v[i] += Math.Sin(k * Math.PI / 15) * chain[k][i];
        }
        u = Unit(u); v = Unit(v);
        double Phase(double[] x) => Math.Atan2(Geometry.Dot(x, v), Geometry.Dot(x, u)) * 180 / Math.PI;
        double degrees = (Phase(chain[1]) - Phase(chain[0]) + 360) % 360;
        Require(Math.Abs(degrees - 12) < 1e-9, "screw step must be twelve degrees");
        return (order, Math.Round(degrees, 9));
    }
}
