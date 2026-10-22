using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using LookLab.Core;

internal static partial class Program
{
    private static GlobalLayout? globalLayout;
    private static GlobalLayout Layouts => globalLayout ??= new GlobalLayout(Mesh.CellStructure);
    private static readonly ProjectionKind[] LL7Kinds = { ProjectionKind.Hemispheres, ProjectionKind.HopfRings, ProjectionKind.RingSheet };
    private static double[] LL7Unit(double[] x) => x.Select(v => v / Math.Sqrt(Geometry.Dot(x, x))).ToArray();
    private static double[] LL7Centre(int cell) => LL7Unit(Mesh.CellStructure.Normals.AsSpan(cell * 4, 4).ToArray());
    private static double LL7Length(double[] x) => Math.Sqrt(Geometry.Dot(x, x));
    private static double[] LL7Difference(double[] a, double[] b) => a.Select((x, i) => x - b[i]).ToArray();
    private static void LL7Near(double[] a, double[] b, double tolerance = 1e-9)
    {
        Equal(a.Length, b.Length);
        for (int i = 0; i < a.Length; i++) Near(a[i], b[i], tolerance);
    }
    private static double[][] LL7Corners(int cell, double cs)
    {
        double[] centre = LL7Centre(cell);
        return Mesh.CellStructure.CellVertices[cell].OrderBy(v => v).Select(v =>
        {
            double[] vertex = LL7Unit(Mesh.CellStructure.Vertices4[v]);
            double d = Geometry.Dot(vertex, centre);
            return vertex.Select((x, i) => centre[i] + cs * (x / d - centre[i])).ToArray();
        }).ToArray();
    }
    private static double[] LL7Q()
    {
        double[] q = Geometry.Identity();
        Geometry.Rotate(q, 0, 2, .4); Geometry.Rotate(q, 1, 3, -.7); Geometry.Rotate(q, 2, 3, .3);
        return q;
    }
    private static double[] LL7Spin()
    {
        double[] spin = Geometry.Identity();
        Geometry.Rotate(spin, 0, 2, .5); Geometry.Rotate(spin, 1, 2, -.3);
        return spin;
    }
    private static IEnumerable<double[]> LL7Rotations()
    {
        var random = new Random(70707);
        for (int i = 0; i < 20; i++)
        {
            double[] q = Geometry.Identity();
            for (int a = 0; a < 4; a++) for (int b = a + 1; b < 4; b++) Geometry.Rotate(q, a, b, (2 * random.NextDouble() - 1) * Math.PI);
            yield return q;
        }
    }
    private static double LL7Det(double[][] rows)
    {
        double[][] m = rows.Select(r => (double[])r.Clone()).ToArray();
        double det = 1;
        for (int i = 0; i < m.Length; i++)
        {
            int pivot = Enumerable.Range(i, m.Length - i).OrderByDescending(j => Math.Abs(m[j][i])).First();
            if (m[pivot][i] == 0) return 0;
            if (pivot != i) { (m[pivot], m[i]) = (m[i], m[pivot]); det = -det; }
            det *= m[i][i];
            for (int j = i + 1; j < m.Length; j++)
            {
                double factor = m[j][i] / m[i][i];
                for (int k = i + 1; k < m.Length; k++) m[j][k] -= factor * m[i][k];
            }
        }
        return det;
    }

    private static void LL7HopfFacts()
    {
        HopfRings h = Layouts.Hopf; CellStructure s = Mesh.CellStructure; HopfFacts f = h.Facts;
        Equal(s.Helices.Length, 240);
        foreach (int[] helix in s.Helices) Sequence(helix, helix.OrderBy(c => c));
        for (int i = 1; i < 240; i++) Check(s.Helices[i - 1].Zip(s.Helices[i]).First(p => p.First != p.Second) is var first && first.First < first.Second, "helix order");
        foreach (var sizes in new[] { f.RightClasses, f.LeftClasses })
        {
            Equal(sizes.Values.Sum(), 16); Equal(sizes.Count, 2); Equal(sizes[12], 10); Equal(sizes[20], 6);
        }
        Equal(f.RightCovers, 6); Equal(f.LeftCovers, 6); Equal(h.Covers.Count, 12); Check(f.CoversPartitionHelices, "cover partition");
        Sequence(h.Covers.SelectMany(c => c.Helices).OrderBy(i => i), Enumerable.Range(0, 240));
        foreach (string side in new[] { "right", "left" })
        {
            HopfCover[] covers = h.Covers.Where(c => c.Side == side).ToArray();
            Sequence(covers.Select(c => c.Index), Enumerable.Range(0, 6));
            Sequence(covers.Select(c => c.Helices[0]), covers.Select(c => c.Helices[0]).OrderBy(i => i));
        }
        Sequence(h.Covers.Select(c => c.Side), Enumerable.Repeat("right", 6).Concat(Enumerable.Repeat("left", 6)));
        foreach (HopfCover cover in h.Covers)
        {
            Sequence(cover.Helices, cover.Helices.OrderBy(i => i));
            Sequence(cover.Helices.SelectMany(i => s.Helices[i]).OrderBy(c => c), Enumerable.Range(0, 600));
            var owner = new int[600];
            for (int r = 0; r < 20; r++) foreach (int cell in s.Helices[cover.Helices[r]]) owner[cell] = r;
            var pairs = Enumerable.Range(0, 600).SelectMany(c => s.FaceNeighbors[c].Where(n => owner[n] > owner[c]).Select(n => (owner[c], owner[n]))).Distinct().ToArray();
            Equal(pairs.Length, 30);
            for (int r = 0; r < 20; r++) Equal(pairs.Count(p => p.Item1 == r || p.Item2 == r), 3);
        }
        Equal(f.CoverGraphDegree, 3); Equal(f.CoverGraphEdges, 30);
        HopfCover chosen = h.Covers.Single(c => c.Helices.Contains(0));
        Equal(h.Side, "right"); Equal(chosen.Side, "right"); LL7Near(h.J, h.RightInvariants[0]);
        Sequence(h.RingHelix.Select(i => Array.IndexOf(chosen.Helices, i)), new[] { 0, 2, 1, 7, 5, 11, 10, 4, 6, 3, 9, 8, 13, 12, 16, 19, 17, 14, 15, 18 });
        Sequence(chosen.Helices.Select(i => h.HalfStep[s.Helices[i][0]] % 2), new[] { 0, 0, 1, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 0, 1, 1 });
        Equal(f.ScrewOrder, 30); Near(f.ScrewDegrees, 12, 1e-9);
        Sequence(f.BaseAngles, new[] { (0.0, 1), (41.81, 3), (70.53, 6), (109.47, 6), (138.19, 3), (180.0, 1) }); Near(f.AdjacentBaseAngle, 41.81);
        Check(f.HalfStepsIntegral && f.ParityAlternates && f.Brick, "phase or brick facts"); Equal(f.BeadClasses, 1); Near(f.ChartDeterminant, 1);
        Equal(f.ChainsAlongJ, 12); Equal(h.ChainsAlongJ, 12);
        Sequence(h.Rings.SelectMany(r => r).OrderBy(c => c), Enumerable.Range(0, 600));
        int brickPairs = 0, seam = 0;
        for (int r = 0; r < 20; r++)
        {
            int[] ring = h.Rings[r];
            Sequence(ring.Select(c => h.HalfStep[c]), ring.Select(c => h.HalfStep[c]).OrderBy(i => i));
            for (int i = 0; i < 30; i++)
            {
                Equal(h.RingOf[ring[i]], r); Check(s.FaceNeighbors[ring[i]].Contains(ring[(i + 1) % 30]), "half-step neighbours");
                Equal((h.HalfStep[ring[(i + 1) % 30]] - h.HalfStep[ring[i]] + 60) % 60, 2);
            }
            var angles = h.BasePoints.Select(b => Math.Round(Math.Acos(Math.Clamp(Geometry.Dot(h.BasePoints[r], b), -1, 1)) * 180 / Math.PI, 2))
                .GroupBy(a => a).OrderBy(g => g.Key).Select(g => (g.Key, g.Count()));
            Sequence(angles, f.BaseAngles);
            if (r == 19) continue;
            var facePairs = ring.SelectMany(c => s.FaceNeighbors[c].Where(n => h.RingOf[n] == r + 1).Select(n => (c, n))).ToArray();
            Equal(facePairs.Length, 20); brickPairs += facePairs.Length;
            Sequence(facePairs.Select(p => (h.HalfStep[p.n] - h.HalfStep[p.c] + 90) % 60 - 30).Distinct().OrderBy(x => x), new[] { -1, 1 });
            seam += facePairs.Count(p => Math.Abs(h.HalfStep[p.n] - h.HalfStep[p.c]) == 59);
        }
        Equal(brickPairs, 380); Equal(seam, 6);
        double[][] expected = { new[] { -.32492, .236068, .076703 }, new[] { -.076703, -.236068, -.32492 },
            new[] { .32492, .236068, -.076703 }, new[] { .076703, -.236068, .32492 } };
        for (int i = 0; i < 4; i++) LL7Near(h.Bead[i], expected[i], 5e-7);
        int[] oldHelices = s.Rings.Select(r => Array.FindIndex(s.Helices, x => x.SequenceEqual(r))).ToArray();
        var memberships = h.Covers.Select(c => (c.Side, c.Index, Count: c.Helices.Intersect(oldHelices).Count())).Where(c => c.Count > 0).ToArray();
        Equal(memberships.Length, 2); Equal(memberships.Single(c => c.Side == "right").Count, 10); Equal(memberships.Single(c => c.Side == "left").Count, 10);
        double Spread(double[][] invariants) => oldHelices.Max(i => Math.Min(LL7Length(LL7Difference(invariants[i], invariants[oldHelices[0]])),
            LL7Length(invariants[i].Select((x, j) => x + invariants[oldHelices[0]][j]).ToArray())));
        Near(Spread(h.RightInvariants), .640852, 5e-7); Near(Spread(h.LeftInvariants), 1.274682, 5e-7);
        Console.WriteLine($"LL7_HOPF_FACTS covers=6+6 chainsAlongJ={f.ChainsAlongJ} brick={brickPairs} seam={seam} oldRings=10+10 jSpread={Spread(h.RightInvariants):F6},{Spread(h.LeftInvariants):F6}");
        Console.WriteLine("LL7_BEAD " + string.Join("; ", h.Bead.Select(b => string.Join(",", b.Select(x => x.ToString("F9", System.Globalization.CultureInfo.InvariantCulture))))));
    }

    private static void LL7Charts()
    {
        HopfRings h = Layouts.Hopf;
        for (int cell = 0; cell < 600; cell++)
        {
            double[] centre = LL7Centre(cell), chart = h.Chart(cell);
            double[][] frame = new[] { centre }.Concat(chart.Chunk(4)).ToArray();
            for (int i = 0; i < 4; i++) for (int j = 0; j < 4; j++) Near(Geometry.Dot(frame[i], frame[j]), i == j ? 1 : 0, 1e-9);
            Near(LL7Det(frame), 1, 1e-9);
            double[][] points = LL7Corners(cell, 1).Select(p => frame.Skip(1).Select(row => Geometry.Dot(row, LL7Difference(p, centre))).ToArray()).ToArray();
            int[] matches = points.Select(p => Array.FindIndex(h.Bead, b => LL7Length(LL7Difference(p, b)) < 1e-9)).ToArray();
            Check(matches.All(i => i >= 0) && matches.Distinct().Count() == 4, "bead mismatch at " + cell);
        }
        Refuse(() => h.Chart(-1), "hopf"); Refuse(() => h.Chart(600), "hopf");
    }

    private static double LL7Jacobian(Func<double[], double[]> map, double[] centre, double[][] basis)
    {
        const double step = 1e-6;
        double[][] columns = basis.Select(b =>
        {
            double[] plus = map(centre.Select((x, i) => x + step * b[i]).ToArray()), minus = map(centre.Select((x, i) => x - step * b[i]).ToArray());
            return plus.Select((x, i) => (x - minus[i]) / (2 * step)).ToArray();
        }).ToArray();
        return LL7Det(columns);
    }
    private static void LL7Orientation()
    {
        double[] identity = Geometry.Identity(), fixedQ = LL7Q(), fixedSpin = LL7Spin();
        double minimum = double.PositiveInfinity;
        for (int cell = 0; cell < 600; cell++)
        {
            double[] centre = LL7Centre(cell); double[][] basis = Layouts.Hopf.Chart(cell).Chunk(4).ToArray();
            foreach (ProjectionKind kind in LL7Kinds) foreach (bool rotated in new[] { false, true })
            {
                double det = LL7Jacobian(x => Layouts.Place(kind, cell, x, rotated ? fixedQ : identity, rotated ? fixedSpin : identity), centre, basis);
                Check(det > 0, $"negative layout Jacobian: {kind} cell {cell}: {det:R}"); minimum = Math.Min(minimum, det);
            }
            foreach (double[] q in new[] { identity, fixedQ })
            {
                double det = LL7Jacobian(x => { double[] y = Geometry.Multiply(q, x); return y.Take(3).Select(v => v / (1 - y[3])).ToArray(); }, centre, basis);
                Check(det > 0, "negative stereographic Jacobian at " + cell);
            }
        }
        Console.WriteLine($"LL7_ORIENTATION minLayoutDet={minimum:G9} all=positive");
    }

    private static double[] LL7Cross(double[] a, double[] b) => new[] { a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0] };
    private static readonly (int A, int B)[] LL7Edges = { (0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3) };
    private static bool LL7Separates(double[][] a, double[][] b, double[] axis)
    {
        double length = LL7Length(axis);
        if (length < 1e-12) return false;
        axis = axis.Select(x => x / length).ToArray();
        double[] pa = a.Select(p => Geometry.Dot(p, axis)).ToArray(), pb = b.Select(p => Geometry.Dot(p, axis)).ToArray();
        return pa.Max() < pb.Min() - 1e-10 || pb.Max() < pa.Min() - 1e-10;
    }
    private static bool LL7Separate3D(double[][] a, double[][] b)
    {
        foreach (double[][] tetra in new[] { a, b }) for (int omit = 0; omit < 4; omit++)
        {
            double[][] face = tetra.Where((_, i) => i != omit).ToArray();
            if (LL7Separates(a, b, LL7Cross(LL7Difference(face[1], face[0]), LL7Difference(face[2], face[0])))) return true;
        }
        foreach (var ea in LL7Edges) foreach (var eb in LL7Edges)
            if (LL7Separates(a, b, LL7Cross(LL7Difference(a[ea.A], a[ea.B]), LL7Difference(b[eb.A], b[eb.B])))) return true;
        return false;
    }
    private static bool LL7Separate2D(double[][] a, double[][] b)
    {
        foreach (double[][] tetra in new[] { a, b }) foreach (var edge in LL7Edges)
        {
            double[] d = LL7Difference(tetra[edge.A], tetra[edge.B]);
            if (LL7Separates(a, b, new[] { -d[1], d[0], 0.0 })) return true;
        }
        return false;
    }
    private static void LL7Separation()
    {
        double[] identity = Geometry.Identity();
        double[][][] corners = Enumerable.Range(0, 600).Select(c => LL7Corners(c, 1)).ToArray();
        double[][][] ring = Enumerable.Range(0, 600).Select(c => corners[c].Select(p => Layouts.Place(ProjectionKind.HopfRings, c, p, identity, identity)).ToArray()).ToArray();
        double[][][] sheet = Enumerable.Range(0, 600).Select(c => corners[c].Select(p => Layouts.Place(ProjectionKind.RingSheet, c, p, identity, identity)).ToArray()).ToArray();
        double[][] centres = ring.Select(t => Enumerable.Range(0, 3).Select(i => t.Average(p => p[i])).ToArray()).ToArray();
        double radius = ring.SelectMany((t, c) => t.Select(p => LL7Length(LL7Difference(p, centres[c])))).Max();
        int ringPairs = 0, sheetPairs = 0;
        for (int a = 0; a < 600; a++) for (int b = a + 1; b < 600; b++)
        {
            if (LL7Length(LL7Difference(centres[a], centres[b])) < 2 * radius + 1e-10)
            {
                ringPairs++;
                Check(LL7Separate3D(ring[a], ring[b]), $"Hopf tetrahedra intersect: {a}, {b}");
            }
            if (Math.Abs(Layouts.Hopf.RingOf[a] - Layouts.Hopf.RingOf[b]) <= 1)
            {
                sheetPairs++;
                Check(LL7Separate2D(sheet[a], sheet[b]), $"sheet cells overlap: {a}, {b}");
            }
        }
        Check(ringPairs > 0 && sheetPairs > 0, "separation search was empty");
        Console.WriteLine($"LL7_SEPARATION ringPairs={ringPairs} sheetPairs={sheetPairs} beadRadius={radius / GlobalLayout.RingScale:G9}");
    }

    private static double[] LL7RestAnchor(int slot, double gap)
    {
        int local = slot % Geometry.StickersPerCell;
        var point = new double[4];
        for (int i = 0; i < 4; i++) point[i] = (Mesh.BaseNormal.Span[i] + (1 - gap) *
            (Mesh.StickerAnchors.Span[local * 4 + i] - Mesh.BaseNormal.Span[i])) / Mesh.Radius;
        return Geometry.Multiply(Mesh.CellFrames.Span.Slice(slot / Geometry.StickersPerCell * 16, 16), point);
    }
    private static void LL7Fit()
    {
        double[] identity = Geometry.Identity(); StructureValues p = Schema.Defaults.Structure(1.6);
        double gap = Schema.Defaults.Number("gap"), ringRadius = 0, sheetX = 0, sheetY = 0, hemiX = 0, hemiY = 0;
        double minimumDenominator = double.PositiveInfinity, partnerExtent = 0, hemisphereRadius = 0;
        var allAnchors = Enumerable.Range(0, Geometry.Slots).Select(slot => LL7RestAnchor(slot, gap)).ToArray();
        double[][][] full = Enumerable.Range(0, 600).Select(c => LL7Corners(c, 1)).ToArray();
        double[][][] shrunken = Enumerable.Range(0, 600).Select(c => LL7Corners(c, p.Cs)).ToArray();
        for (int c = 0; c < 600; c++)
        {
            foreach (double[] corner in full[c]) ringRadius = Math.Max(ringRadius, LL7Length(Layouts.Place(ProjectionKind.HopfRings, c, corner, identity, identity)));
            foreach (double[] corner in shrunken[c])
            {
                Projection projected = ViewFraming.Camera(Layouts.Place(ProjectionKind.RingSheet, c, corner, identity, identity), p);
                sheetX = Math.Max(sheetX, Math.Abs(projected.NdcX)); sheetY = Math.Max(sheetY, Math.Abs(projected.NdcY));
            }
        }
        foreach (double[] q in new[] { identity }.Concat(LL7Rotations()))
        {
            for (int c = 0; c < 600; c++)
            {
                double side = Geometry.Multiply(q, LL7Centre(c))[3] >= 0 ? 1 : -1;
                foreach (double[] corner in full[c])
                {
                    double[] y = Geometry.Multiply(q, corner);
                    double denominator = 1 + side * y[3];
                    minimumDenominator = Math.Min(minimumDenominator, denominator);
                    hemisphereRadius = Math.Max(hemisphereRadius, Math.Sqrt(y.Take(3).Sum(x => x * x)) / denominator);
                    partnerExtent = Math.Max(partnerExtent, -y[0] / denominator);
                }
                foreach (double[] corner in shrunken[c])
                {
                    Projection projected = ViewFraming.Camera(Layouts.Place(ProjectionKind.Hemispheres, c, corner, q, identity), p);
                    hemiX = Math.Max(hemiX, Math.Abs(projected.NdcX)); hemiY = Math.Max(hemiY, Math.Abs(projected.NdcY));
                }
            }
            for (int slot = 0; slot < Geometry.Slots; slot++)
            {
                Projection projected = ViewFraming.Camera(Layouts.Place(ProjectionKind.Hemispheres, slot / Geometry.StickersPerCell, allAnchors[slot], q, identity), p);
                hemiX = Math.Max(hemiX, Math.Abs(projected.NdcX)); hemiY = Math.Max(hemiY, Math.Abs(projected.NdcY));
            }
        }
        Console.WriteLine($"LL7_FIT ringRadius={ringRadius:G12} sheetNdc={sheetX:G12},{sheetY:G12} hemisphereNdc={hemiX:G12},{hemiY:G12} minDenominator={minimumDenominator:G12} qCount=21 anchorsPerQ={allAnchors.Length} partnerExtent={partnerExtent:G9} hemisphereRadius={hemisphereRadius:G9}");
        Check(ringRadius <= 2.5, $"RingScale={GlobalLayout.RingScale}: radius {ringRadius:R} exceeds 2.5");
        Check(sheetX <= .9 && sheetY <= .9, $"SheetScale={GlobalLayout.SheetScale}: NDC {sheetX:R}, {sheetY:R} exceeds .9");
        Check(hemiX <= .9 && hemiY <= .9, $"HemisphereScale={GlobalLayout.HemisphereScale}: NDC {hemiX:R}, {hemiY:R} exceeds .9");
        Check(minimumDenominator >= .5, "hemisphere rest denominator is below .5");
    }

    private static void LL7TurnTargets()
    {
        int[] slots = Enumerable.Range(0, Geometry.Slots).Where(Turn.IsMoving).ToArray();
        Equal(slots.Length, 4605); Equal(slots.Select(Geometry.CellOfSlot).Distinct().Count(), 57);
        int[] forward = Turn.TargetCells(false).ToArray(), inverse = Turn.TargetCells(true).ToArray();
        Equal(forward.Length, 600); Equal(inverse.Length, 600);
        Sequence(forward.OrderBy(c => c), Enumerable.Range(0, 600)); Sequence(inverse.OrderBy(c => c), Enumerable.Range(0, 600));
        Equal(forward[0], 0); Equal(inverse[0], 0);
        for (int cell = 0; cell < 600; cell++)
        {
            Equal(inverse[forward[cell]], cell); Equal(forward[inverse[cell]], cell);
            if (Enumerable.Range(cell * 433, 433).All(s => !Turn.IsMoving(s))) { Equal(forward[cell], cell); Equal(inverse[cell], cell); }
        }
        foreach (bool backwards in new[] { false, true })
        {
            uint[] before = Turn.Snapshot(0).ToArray(), after = Turn.Apply(before, backwards);
            foreach (int slot in slots)
            {
                int target = Turn.DestinationSlot(slot, backwards);
                Equal(after[target], before[slot]); Equal(target / 433, Turn.TargetCells(backwards).Span[slot / 433]);
            }
            Sequence(slots.Where(s => Turn.DestinationSlot(s, backwards) == s), new[] { 0, 6, 37, 119, 122 });
            foreach (int slot in Enumerable.Range(0, Geometry.Slots).Where(s => !Turn.IsMoving(s))) Equal(Turn.DestinationSlot(slot, backwards), slot);
        }
        var moving = new bool[Geometry.Slots]; moving[0] = moving[1] = true;
        Refuse(() => TurnData.BuildTargets(moving, new[] { 0, 1 }, new[] { 433, 866 }), "moving slots of cell 0 reach different cells");
        Refuse(() => Turn.DestinationSlot(-1, false), "slot"); Refuse(() => Turn.DestinationSlot(Geometry.Slots, true), "slot");
    }

    private static double[] LL7TurnPoint(double[] point, double theta)
    {
        double[] posed = (double[])point.Clone();
        double x = Geometry.Dot(posed, Turn.PlaneU.Span), y = Geometry.Dot(posed, Turn.PlaneV.Span);
        double du = (Math.Cos(theta) - 1) * x - Math.Sin(theta) * y, dv = Math.Sin(theta) * x + (Math.Cos(theta) - 1) * y;
        for (int i = 0; i < 4; i++) posed[i] += du * Turn.PlaneU.Span[i] + dv * Turn.PlaneV.Span[i];
        return posed;
    }
    private static void LL7BlendHandoff()
    {
        int[] slots = Enumerable.Range(0, Geometry.Slots).Where(Turn.IsMoving).ToArray();
        double[] q = LL7Q(), spin = LL7Spin(); var summaries = new List<string>();
        foreach (double gap in new[] { .24, 0 })
        {
            var rest = slots.ToDictionary(s => s, s => LL7RestAnchor(s, gap));
            foreach (bool inverse in new[] { false, true })
            {
                double theta = (inverse ? -1 : 1) * Turn.Angle;
                var residuals = new List<double>(); double fixedMax = 0;
                foreach (int slot in slots)
                {
                    int destination = Turn.DestinationSlot(slot, inverse), cell = slot / 433, targetCell = destination / 433;
                    double[] posed = LL7TurnPoint(rest[slot], theta), target = rest[destination];
                    double residual = LL7Length(LL7Difference(posed, target)); residuals.Add(residual);
                    if (slot == destination) fixedMax = Math.Max(fixedMax, residual);
                    Check(residual <= 1e-5, $"retained input turn residual {residual:R} at {slot}, inverse={inverse}, gap={gap}");
                    foreach (ProjectionKind kind in LL7Kinds)
                    {
                        double[] placed = Layouts.PlaceMoving(kind, cell, posed, q, spin, Turn, theta, true);
                        double[] expected = Layouts.Place(kind, targetCell, target, q, spin);
                        double bound = kind == ProjectionKind.HopfRings ? GlobalLayout.RingScale : GlobalLayout.SheetScale;
                        if (kind == ProjectionKind.Hemispheres)
                        {
                            double[] a = Geometry.Multiply(q, posed), b = Geometry.Multiply(q, target);
                            double side = Geometry.Multiply(q, LL7Centre(targetCell))[3] >= 0 ? 1 : -1;
                            double delta = Math.Min(Math.Max(1 + side * a[3], GlobalLayout.HemisphereClamp), Math.Max(1 + side * b[3], GlobalLayout.HemisphereClamp));
                            double radius = Math.Max(Math.Sqrt(a.Take(3).Sum(x => x * x)), Math.Sqrt(b.Take(3).Sum(x => x * x)));
                            bound = GlobalLayout.HemisphereScale * (1 / delta + radius / (delta * delta));
                        }
                        Check(LL7Length(LL7Difference(placed, expected)) <= bound * residual + 1e-9, $"blend handoff {kind} at {slot}");
                        Sequence(Layouts.PlaceMoving(kind, cell, posed, q, spin, Turn, 0, true), Layouts.Place(kind, cell, posed, q, spin));
                        Sequence(Layouts.PlaceMoving(kind, cell, posed, q, spin, Turn, theta, false), Layouts.Place(kind, cell, posed, q, spin));
                    }
                }
                residuals.Sort();
                summaries.Add($"gap={gap} inverse={inverse} max={residuals[^1]:G10} median={residuals[residuals.Count / 2]:G10} unpermutedMax={fixedMax:G10}");
            }
        }
        Console.WriteLine("LL7_BLEND_HANDOFF " + string.Join("; ", summaries));
    }

    private static double[] LL7Posed(int globalVertex, StructureValues p, double theta)
    {
        int cell = globalVertex / Geometry.BaseVertices, vertex = globalVertex % Geometry.BaseVertices, local = Mesh.BaseStickerIds.Span[vertex];
        var point = new double[4];
        for (int i = 0; i < 4; i++) point[i] = (Mesh.BaseNormal.Span[i] + p.Cs * (Mesh.StickerAnchors.Span[local * 4 + i] - Mesh.BaseNormal.Span[i]) +
            p.Cs * p.Ss * (Mesh.BaseVertexData.Span[vertex * 4 + i] - Mesh.StickerAnchors.Span[local * 4 + i])) / Mesh.Radius;
        double[] world = Geometry.Multiply(Mesh.CellFrames.Span.Slice(cell * 16, 16), point);
        return theta != 0 && Turn.IsMoving(cell * 433 + local) ? LL7TurnPoint(world, theta) : world;
    }
    private static void LL7ProjectOverload()
    {
        var random = new Random(70707);
        var firstVertex = new int[433]; Array.Fill(firstVertex, -1);
        for (int v = 0; v < Geometry.BaseVertices; v++) if (firstVertex[Mesh.BaseStickerIds.Span[v]] < 0) firstVertex[Mesh.BaseStickerIds.Span[v]] = v;
        int[] moving = Enumerable.Range(0, Geometry.Slots).Where(Turn.IsMoving).ToArray();
        int[] vertices = Enumerable.Range(0, 200).Select(i => i < 100 ? random.Next(600 * Geometry.BaseVertices) :
            moving[random.Next(moving.Length)]).ToArray();
        for (int i = 100; i < 200; i++) vertices[i] = vertices[i] / 433 * Geometry.BaseVertices + firstVertex[vertices[i] % 433];
        double[] q = LL7Q(), spin = LL7Spin(); StructureValues p = Schema.Defaults.Structure(1.6);
        foreach (int vertex in vertices) foreach (double theta in new[] { 0, .6, -Turn.Angle / 2 })
        {
            foreach (ProjectionKind kind in new[] { ProjectionKind.Perspective, ProjectionKind.Stereographic, ProjectionKind.Orthographic })
                Equal(Mesh.Project(vertex, p with { Projection = kind }, q, Turn, theta, Layouts, spin), Mesh.Project(vertex, p with { Projection = kind }, q, Turn, theta));
            int cell = vertex / Geometry.BaseVertices, slot = cell * 433 + Mesh.BaseStickerIds.Span[vertex % Geometry.BaseVertices];
            double[] posed = LL7Posed(vertex, p, theta);
            foreach (ProjectionKind kind in LL7Kinds)
            {
                StructureValues values = p with { Projection = kind };
                Projection expected = ViewFraming.Camera(Layouts.PlaceMoving(kind, cell, posed, q, spin, Turn, theta, Turn.IsMoving(slot)), values);
                Equal(Mesh.Project(vertex, values, q, Turn, theta, Layouts, spin), expected);
                Refuse(() => Mesh.Project(vertex, values, q, Turn, theta), "layout kind needs a GlobalLayout");
            }
        }
        Refuse(() => Mesh.Project(0, p, q, Turn, 0, Layouts, new double[3]), "spin");
        Refuse(() => Mesh.Project(0, p, q, Turn, 0, Layouts, Enumerable.Repeat(double.NaN, 16).ToArray()), "spin");
        foreach (ProjectionKind kind in new[] { ProjectionKind.Perspective, ProjectionKind.Stereographic, ProjectionKind.Orthographic })
            Refuse(() => Layouts.Place(kind, 0, new double[4], q, spin), "projection");
        Refuse(() => Layouts.Place(ProjectionKind.HopfRings, 600, new double[4], q, spin), "cell");
        Refuse(() => Layouts.Place(ProjectionKind.HopfRings, -1, new double[4], q, spin), "cell");
        Refuse(() => Layouts.Place(ProjectionKind.HopfRings, 0, new double[3], q, spin), "posed");
        Refuse(() => Layouts.Place(ProjectionKind.HopfRings, 0, new[] { double.NaN, 0, 0, 0 }, q, spin), "posed");
        Refuse(() => Layouts.Place(ProjectionKind.HopfRings, 0, new double[4], new double[3], spin), "Q");
        Refuse(() => Layouts.Place(ProjectionKind.RingSheet, 0, new double[4], q, new double[3]), "spin");
    }

    private static void LL7CellLayout()
    {
        float[] data = Layouts.CellLayout(Turn); Equal(GlobalLayout.LayoutRows, 10); Equal(data.Length, 600 * 10 * 4);
        double[] identity = Geometry.Identity();
        for (int cell = 0; cell < 600; cell++)
        {
            double[] centre = LL7Centre(cell);
            for (int i = 0; i < 4; i++) Near(data[cell * 4 + i], centre[i], 1e-7);
            Equal(data[(600 + cell) * 4], (float)Turn.TargetCells(false).Span[cell]);
            Equal(data[(600 + cell) * 4 + 1], (float)Turn.TargetCells(true).Span[cell]);
            Equal(data[(600 + cell) * 4 + 2], 0f); Equal(data[(600 + cell) * 4 + 3], 0f);
            foreach (ProjectionKind kind in new[] { ProjectionKind.HopfRings, ProjectionKind.RingSheet })
            {
                int row = kind == ProjectionKind.HopfRings ? 2 : 6;
                Equal(data[((row + 3) * 600 + cell) * 4 + 3], 0f);
                foreach (double[] point in new[] { centre }.Concat(LL7Corners(cell, 1 - Schema.Defaults.Number("gap"))))
                {
                    double[] expected = Layouts.Place(kind, cell, point, identity, identity);
                    for (int i = 0; i < 3; i++)
                    {
                        double value = data[((row + 3) * 600 + cell) * 4 + i];
                        for (int col = 0; col < 4; col++) value += data[((row + i) * 600 + cell) * 4 + col] * point[col];
                        Near(value, expected[i], 1e-5 * Math.Max(1, Math.Abs(expected[i])));
                    }
                }
            }
        }
    }

    private static void LL7LayoutFraming()
    {
        double[] anchors = ViewFraming.MovingRestAnchors(Mesh, Turn, Schema.Defaults.Number("gap")), q = LL7Q(), spin = LL7Spin();
        int[] cells = ViewFraming.MovingRestCells(Turn);
        Sequence(cells, Enumerable.Range(0, Geometry.Slots).Where(Turn.IsMoving).Select(s => s / 433)); Equal(anchors.Length, cells.Length * 4);
        foreach (ProjectionKind kind in Enum.GetValues<ProjectionKind>())
        {
            StructureValues p = Schema.Defaults.Structure(1.6) with { Projection = kind };
            FocusFrame frame = ViewFraming.Target(anchors, cells, q, spin, p, Layouts);
            if (!GlobalLayout.IsLayout(kind)) Equal(frame, ViewFraming.Target(anchors, q, p));
            else
            {
                Bounded(frame); Check(frame.M > 1, "layout framing is neutral: " + kind);
                Equal(ViewFraming.Target(Array.Empty<double>(), Array.Empty<int>(), q, spin, p, Layouts), FocusFrame.Neutral);
            }
        }
        Refuse(() => ViewFraming.Target(anchors, Array.Empty<int>(), q, spin, Schema.Defaults.Structure(1.6) with { Projection = ProjectionKind.RingSheet }, Layouts), "anchors");
    }

    private static void LL7Drag()
    {
        double[] q = Geometry.Identity(), spin = Geometry.Identity();
        GlobalLayout.Drag(q, spin, .2, 0); Check(Geometry.Multiply(spin, new[] { 0.0, 0, 1, 0 })[0] > 0, "yaw sign");
        q = Geometry.Identity(); spin = Geometry.Identity();
        GlobalLayout.Drag(q, spin, 0, .2); Check(Geometry.Multiply(spin, new[] { 0.0, 0, 1, 0 })[1] < 0, "pitch sign");
        q = LL7Q(); spin = LL7Spin(); double[] expectedQ = (double[])q.Clone();
        var random = new Random(70707);
        for (int step = 0; step < 200; step++)
        {
            double ax = (2 * random.NextDouble() - 1) * .2, ay = (2 * random.NextDouble() - 1) * .2;
            GlobalLayout.Drag(q, spin, ax, ay);
            Geometry.Rotate(expectedQ, 0, 3, ax); Geometry.Rotate(expectedQ, 1, 3, ay); Sequence(q, expectedQ);
            for (int i = 0; i < 4; i++) for (int j = 0; j < 4; j++) Near(Geometry.Dot(spin.AsSpan(i * 4, 4), spin.AsSpan(j * 4, 4)), i == j ? 1 : 0, 1e-12);
            Near(LL7Det(spin.Chunk(4).ToArray()), 1, 1e-12);
            for (int i = 0; i < 4; i++) { Equal(spin[12 + i], i == 3 ? 1.0 : 0); Equal(spin[i * 4 + 3], i == 3 ? 1.0 : 0); }
        }
    }

    private static void LL7Colouring()
    {
        CellStructure s = Mesh.CellStructure; HopfRings h = Layouts.Hopf;
        for (int k = 4; k <= 8; k++)
        {
            int[] equitable = GlobalLayout.EquitableColouring(20, h.Graph, k) ?? throw new Exception("no equitable colouring for " + k);
            Sequence(equitable, GlobalLayout.EquitableColouring(20, h.Graph, k)!);
            Check(h.Graph.All(p => equitable[p.A] != equitable[p.B]), "equitable colouring is not proper");
            Sequence(equitable.Distinct().OrderBy(c => c), Enumerable.Range(0, k));
            for (int colour = 0; colour < k; colour++)
            {
                int count = equitable.Count(c => c == colour);
                Check(count == 20 / k || count == (20 + k - 1) / k, "class is not equitable");
            }
            foreach (ProjectionKind kind in Enum.GetValues<ProjectionKind>())
            {
                string projection = Schema["projection"].Options![(int)kind];
                ParameterSet look = Schema.Defaults.With("classes", new IntegerValue(k)).With("projection", new EnumValue(projection));
                CellColouring colouring = Layouts.Colouring(look, s);
                bool hopf = kind is ProjectionKind.HopfRings or ProjectionKind.RingSheet;
                Sequence(colouring.RingOf, hopf ? h.RingOf : s.RingOf);
                Sequence(colouring.RingClass, hopf ? equitable : CellStructure.ProperColouring(20, s.RingGraph, k)!);
                PaletteReport report = Colour.Report(look, s, colouring);
                if (hopf)
                {
                    var pairs = new HashSet<(int, int)>(); int same = 0;
                    for (int c = 0; c < 600; c++) foreach (int n in s.FaceNeighbors[c].Where(n => n > c))
                    {
                        if (colouring.RingOf[c] == colouring.RingOf[n]) { same++; continue; }
                        int a = colouring.ClassOf(c), b = colouring.ClassOf(n); pairs.Add((Math.Min(a, b), Math.Max(a, b)));
                    }
                    Sequence(report.ClassPairs, pairs.OrderBy(p => p.Item1).ThenBy(p => p.Item2));
                    Equal(same, 600); Equal(report.SameRingAdjacencies, same);
                    Sequence(Enumerable.Range(0, 600).Select(colouring.ClassOf).Distinct().OrderBy(c => c), Enumerable.Range(0, k));
                }
                ParameterSet perspective = look.With("projection", new EnumValue("perspective"));
                PaletteReport old = Colour.Report(look, s), shared = Colour.Report(look, s, Layouts.Colouring(perspective, s));
                Sequence(old.Modes, shared.Modes); Sequence(old.ClassPairs, shared.ClassPairs); Equal(old.SameRingAdjacencies, shared.SameRingAdjacencies);
                Sequence(old.GamutFailures, shared.GamutFailures);
            }
        }
        ParameterSet badGamut = Schema.Defaults.With("lightness", new NumberValue(.85)).With("lightnessAlt", new NumberValue(.2));
        PaletteReport failure = Colour.Report(badGamut, s), sharedFailure = Colour.Report(badGamut, s, Layouts.Colouring(badGamut, s));
        Sequence(failure.GamutFailures, sharedFailure.GamutFailures); Sequence(failure.Modes, sharedFailure.Modes);
        Sequence(failure.ClassPairs, sharedFailure.ClassPairs); Equal(failure.SameRingAdjacencies, sharedFailure.SameRingAdjacencies);
        Refuse(() => GlobalLayout.EquitableColouring(0, h.Graph, 4), "classes");
        Refuse(() => GlobalLayout.EquitableColouring(20, h.Graph, 0), "classes");
        Refuse(() => GlobalLayout.EquitableColouring(20, new[] { (0, 20) }, 4), "ring graph");
        Check(GlobalLayout.EquitableColouring(1, new[] { (0, 0) }, 4) == null, "self-loop accepted");
        Check(GlobalLayout.EquitableColouring(3, new[] { (0, 1), (1, 2), (0, 2) }, 2) == null, "impossible colouring accepted");
        Refuse(() => Colour.Report(Schema.Defaults, s, new CellColouring(new int[599], new[] { 0 })), "colouring");
        Refuse(() => Colour.Report(Schema.Defaults, s, new CellColouring(Enumerable.Repeat(-1, 600).ToArray(), new[] { 0 })), "colouring");
        Refuse(() => Colour.Report(Schema.Defaults, s, new CellColouring(new int[600], Array.Empty<int>())), "colouring");
        Refuse(() => Colour.Report(Schema.Defaults, s, new CellColouring(new int[600], new[] { 6 })), "colouring");
        Refuse(() => Colour.Report(Schema.Defaults, s, new CellColouring(new int[600], new[] { -1 })), "colouring");
    }

    private static void LL7G1Presets()
    {
        Preset defaults = Presets.Load(FileAt("presets/default.json"), Schema);
        var presets = new[] { ("perspective", "G1 perspective", "perspective"), ("stereographic", "G1 stereographic", "stereographic"),
            ("orthographic", "G1 orthographic", "orthographic"), ("hemispheres", "G1 two hemispheres", "hemispheres"),
            ("hopf-rings", "G1 Hopf rings", "hopfRings"), ("ring-sheet", "G1 ring sheet", "ringSheet") };
        Equal(Schema.Parameters.Count, 42);
        Sequence(Schema["projection"].Options!, presets.Select(p => p.Item3));
        for (int i = 0; i < presets.Length; i++)
        {
            var (file, name, kind) = presets[i]; string path = FileAt("presets/g1-" + file + ".json");
            Preset preset = Presets.Load(path, Schema); Equal(preset.Name, name); Equal(preset.Family, null); Equal(preset.Scene, null);
            Sequence(Presets.CanonicalBytes(preset, Schema), File.ReadAllBytes(path), "noncanonical G1 preset: " + file);
            Equal(preset.Params.Choice("projection"), kind); Equal((int)preset.Params.Structure(1.6).Projection, i);
            foreach (var parameter in defaults.Params.Values)
            {
                if (parameter.Key == "projection") continue;
                if (kind == "orthographic" && parameter.Key == "fieldOfView") Near(preset.Params.Number(parameter.Key), 2 * Math.Atan(1 / 3.27), 1e-12);
                else Equal(preset.Params.Values[parameter.Key], parameter.Value, file + ": changed " + parameter.Key);
            }
        }
    }

    private static void LL7HopfFixture()
    {
        HopfRings h = Layouts.Hopf; CellStructure s = Mesh.CellStructure; HopfFacts f = h.Facts;
        using var document = JsonDocument.Parse(Read("fixtures/hopf-rings.json")); var x = document.RootElement;
        double[] Doubles(JsonElement e) => e.EnumerateArray().Select(v => v.GetDouble()).ToArray();
        int[] Integers(JsonElement e) => e.EnumerateArray().Select(v => v.GetInt32()).ToArray();
        Equal(x.GetProperty("format").GetString(), "magic600-look-hopf-rings"); Equal(x.GetProperty("version").GetInt32(), 1);
        Equal(AssetCatalogue.Digest(File.ReadAllBytes(Path.Combine(Root, "assets/model.npz"))), x.GetProperty("modelDigest").GetString());
        Equal(x.GetProperty("helixCount").GetInt32(), s.Helices.Length); Equal(x.GetProperty("side").GetString(), h.Side);
        LL7Near(Doubles(x.GetProperty("J")), h.J); LL7Near(Doubles(x.GetProperty("K")), h.K);
        Equal(x.GetProperty("rings").GetArrayLength(), 20);
        for (int r = 0; r < 20; r++)
        {
            JsonElement ring = x.GetProperty("rings")[r];
            Equal(ring.GetProperty("helix").GetInt32(), h.RingHelix[r]);
            Sequence(Integers(ring.GetProperty("cells")), h.Rings[r]);
            Sequence(Integers(ring.GetProperty("halfSteps")), h.Rings[r].Select(c => h.HalfStep[c]));
            LL7Near(Doubles(ring.GetProperty("origin")), h.Origins[r]); LL7Near(Doubles(ring.GetProperty("basePoint")), h.BasePoints[r]);
        }
        Sequence(Integers(x.GetProperty("ringOf")), h.RingOf); Sequence(Integers(x.GetProperty("halfStep")), h.HalfStep);
        Sequence(x.GetProperty("graph").EnumerateArray().Select(p => (p[0].GetInt32(), p[1].GetInt32())), h.Graph);
        for (int i = 0; i < 4; i++) LL7Near(Doubles(x.GetProperty("bead")[i]), h.Bead[i]);
        JsonElement facts = x.GetProperty("facts");
        Sequence(facts.EnumerateObject().Select(p => p.Name), new[] { "rightClasses", "leftClasses", "rightCovers", "leftCovers",
            "coversPartitionHelices", "coverGraphDegree", "coverGraphEdges", "screwOrder", "screwDegrees", "baseAngles", "adjacentBaseAngle",
            "halfStepsIntegral", "parityAlternates", "brick", "chainsAlongJ", "chainsAgainstJ", "rowFacePairs", "seamFacePairs",
            "beadClasses", "chartDeterminant" });
        foreach (var (name, sizes) in new[] { ("rightClasses", f.RightClasses), ("leftClasses", f.LeftClasses) })
            Sequence(facts.GetProperty(name).EnumerateObject().Select(p => (int.Parse(p.Name), p.Value.GetInt32())), sizes.OrderBy(p => p.Key).Select(p => (p.Key, p.Value)));
        Equal(facts.GetProperty("rightCovers").GetInt32(), f.RightCovers); Equal(facts.GetProperty("leftCovers").GetInt32(), f.LeftCovers);
        Equal(facts.GetProperty("coversPartitionHelices").GetBoolean(), f.CoversPartitionHelices);
        Equal(facts.GetProperty("coverGraphDegree").GetInt32(), f.CoverGraphDegree); Equal(facts.GetProperty("coverGraphEdges").GetInt32(), f.CoverGraphEdges);
        Equal(facts.GetProperty("screwOrder").GetInt32(), f.ScrewOrder); Near(facts.GetProperty("screwDegrees").GetDouble(), f.ScrewDegrees, 1e-9);
        JsonElement[] angles = facts.GetProperty("baseAngles").EnumerateArray().ToArray(); Equal(angles.Length, f.BaseAngles.Length);
        for (int i = 0; i < angles.Length; i++) { Near(angles[i][0].GetDouble(), f.BaseAngles[i].Angle, 1e-9); Equal(angles[i][1].GetInt32(), f.BaseAngles[i].Count); }
        Near(facts.GetProperty("adjacentBaseAngle").GetDouble(), f.AdjacentBaseAngle, 1e-9);
        foreach (var (name, value) in new[] { ("halfStepsIntegral", f.HalfStepsIntegral), ("parityAlternates", f.ParityAlternates), ("brick", f.Brick) })
            Equal(facts.GetProperty(name).GetBoolean(), value);
        Equal(facts.GetProperty("chainsAlongJ").GetInt32(), f.ChainsAlongJ); Equal(facts.GetProperty("chainsAgainstJ").GetInt32(), 20 - f.ChainsAlongJ);
        int[] rowPairs = Enumerable.Range(0, 19).Select(r => h.Rings[r].Sum(c => s.FaceNeighbors[c].Count(n => h.RingOf[n] == r + 1))).Distinct().ToArray();
        int seam = Enumerable.Range(0, 19).Sum(r => h.Rings[r].Sum(c => s.FaceNeighbors[c].Count(n => h.RingOf[n] == r + 1 && Math.Abs(h.HalfStep[n] - h.HalfStep[c]) == 59)));
        Equal(rowPairs.Length, 1); Equal(facts.GetProperty("rowFacePairs").GetInt32(), rowPairs[0]); Equal(facts.GetProperty("seamFacePairs").GetInt32(), seam);
        Equal(facts.GetProperty("beadClasses").GetInt32(), f.BeadClasses); Near(facts.GetProperty("chartDeterminant").GetDouble(), f.ChartDeterminant, 1e-9);
        // LH-05: where the retained Look Lab rings fall among the Hopf covers.
        JsonElement old = x.GetProperty("lookLabRings");
        int[] oldHelices = s.Rings.Select(r => Array.FindIndex(s.Helices, helix => helix.SequenceEqual(r))).ToArray();
        Check(oldHelices.All(i => i >= 0), "a Look Lab ring is not a helix"); Equal(old.GetProperty("rings").GetInt32(), oldHelices.Length);
        var byCover = h.Covers.Select(c => (c.Side, c.Index, Chosen: c.Helices.Contains(0), Rings: c.Helices.Intersect(oldHelices).Count()))
            .Where(c => c.Rings > 0).OrderBy(c => c.Side, StringComparer.Ordinal).ThenBy(c => c.Index).ToArray();
        Sequence(old.GetProperty("byCover").EnumerateArray().Select(c => (c.GetProperty("side").GetString()!, c.GetProperty("cover").GetInt32(),
            c.GetProperty("chosen").GetBoolean(), c.GetProperty("rings").GetInt32())), byCover);
        double Spread(double[][] invariants) => oldHelices.Max(i => Math.Min(LL7Length(LL7Difference(invariants[i], invariants[oldHelices[0]])),
            LL7Length(invariants[i].Select((v, j) => v + invariants[oldHelices[0]][j]).ToArray())));
        Near(old.GetProperty("jSpread").GetProperty("right").GetDouble(), Spread(h.RightInvariants), 1e-6);
        Near(old.GetProperty("jSpread").GetProperty("left").GetDouble(), Spread(h.LeftInvariants), 1e-6);
        Equal(old.GetProperty("graphDegree").GetInt32(), s.RingGraph.Count(p => p.A == 0 || p.B == 0));
        Equal(old.GetProperty("hopfCover").GetBoolean(), byCover.Length == 1);
        Console.WriteLine($"LL7_HOPF_FIXTURE rings=20 lookLabRingsByCover={string.Join("+", byCover.Select(c => c.Rings))} hopfCover={byCover.Length == 1}");
    }
}
