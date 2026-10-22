using System;
using System.Collections.Generic;
using System.Linq;

namespace LookLab.Core;

public sealed class GlobalLayout
{
    public const double RingRadius = 2.62, BaseRadius = 8.063, RingScale = 0.28;
    public const double SheetDx = 0.68, SheetDy = 0.57, SheetScale = 0.6;
    public const double HemisphereOffset = 1.75, HemisphereScale = 1.4, HemisphereClamp = 0.25;
    public const int LayoutRows = 10;
    public HopfRings Hopf { get; }
    private readonly double[][] centres = new double[600][], ringCharts = new double[600][], sheetCharts = new double[600][];

    public GlobalLayout(CellStructure structure)
    {
        Hopf = new HopfRings(structure);
        double[][] orientation = { Hopf.E1[0], Hopf.E2[0], Hopf.BasePoints[0] };
        for (int cell = 0; cell < 600; cell++)
        {
            double[] centre = structure.Normals.AsSpan(cell * 4, 4).ToArray();
            double length = Math.Sqrt(Geometry.Dot(centre, centre));
            centres[cell] = centre.Select(x => x / length).ToArray();
            int r = Hopf.RingOf[cell], h = Hopf.HalfStep[cell];
            double t = h * Math.PI / 30;
            double[] n = new double[3], tangent = new double[3], position = new double[3];
            for (int i = 0; i < 3; i++)
            {
                n[i] = Math.Cos(t) * Hopf.E1[r][i] + Math.Sin(t) * Hopf.E2[r][i];
                tangent[i] = -Math.Sin(t) * Hopf.E1[r][i] + Math.Cos(t) * Hopf.E2[r][i];
                position[i] = BaseRadius * Hopf.BasePoints[r][i] + RingRadius * n[i];
            }
            double[][] columns = { tangent, n, Hopf.BasePoints[r].Select(x => -x).ToArray() };
            double[] b = Hopf.Chart(cell), ring = new double[16], sheet = new double[16];
            for (int row = 0; row < 3; row++)
            {
                for (int col = 0; col < 4; col++)
                {
                    for (int k = 0; k < 3; k++) ring[row * 4 + col] += RingScale * Geometry.Dot(orientation[row], columns[k]) * b[k * 4 + col];
                    sheet[row * 4 + col] = SheetScale * b[row * 4 + col];
                }
                ring[12 + row] = RingScale * Geometry.Dot(orientation[row], position);
            }
            sheet[12] = SheetScale * SheetDx * (h / 2.0 - 14.75);
            sheet[13] = -SheetScale * SheetDy * (r - 9.5);
            ringCharts[cell] = ring; sheetCharts[cell] = sheet;
        }
    }

    public static bool IsLayout(ProjectionKind kind) => kind >= ProjectionKind.Hemispheres;

    private static void Finite(ReadOnlySpan<double> values, int count, string id)
    {
        if (values.Length != count) throw Json.Error(id, "expected " + count + " finite values");
        foreach (double value in values) if (!double.IsFinite(value)) throw Json.Error(id, "expected " + count + " finite values");
    }

    public double[] Place(ProjectionKind kind, int chartCell, ReadOnlySpan<double> posed, ReadOnlySpan<double> q, ReadOnlySpan<double> spin)
    {
        if (!IsLayout(kind) || kind > ProjectionKind.RingSheet) throw Json.Error("projection", "GlobalLayout needs a layout kind");
        if (chartCell < 0 || chartCell >= 600) throw Json.Error("cell", "outside 0 to 599");
        Finite(posed, 4, "posed"); Finite(q, 16, "Q"); Finite(spin, 16, "spin");
        if (kind == ProjectionKind.Hemispheres)
        {
            double[] y = Geometry.Multiply(q, posed), centre = Geometry.Multiply(q, centres[chartCell]);
            double side = centre[3] >= 0 ? 1 : -1, denominator = Math.Max(1 + side * y[3], HemisphereClamp);
            return new[] { HemisphereScale * (-side * y[0] / denominator - side * HemisphereOffset),
                HemisphereScale * y[1] / denominator, HemisphereScale * y[2] / denominator };
        }
        double[] chart = kind == ProjectionKind.HopfRings ? ringCharts[chartCell] : sheetCharts[chartCell];
        var placed = new double[3];
        for (int row = 0; row < 3; row++) placed[row] = Geometry.Dot(chart.AsSpan(row * 4, 4), posed) + chart[12 + row];
        if (kind == ProjectionKind.RingSheet) return placed;
        var result = new double[3];
        for (int row = 0; row < 3; row++) for (int col = 0; col < 3; col++) result[row] += spin[col * 4 + row] * placed[col];
        return result;
    }

    public double[] PlaceMoving(ProjectionKind kind, int cell, ReadOnlySpan<double> posed, ReadOnlySpan<double> q, ReadOnlySpan<double> spin,
        TurnData turn, double theta, bool moving)
    {
        if (!double.IsFinite(theta)) throw Json.Error("theta", "finite angle required");
        double[] own = Place(kind, cell, posed, q, spin);
        if (!moving || theta == 0) return own;
        bool inverse = theta * turn.Angle < 0;
        int target = turn.TargetCells(inverse).Span[cell];
        double blend = Math.Clamp(Math.Abs(theta / turn.Angle), 0, 1);
        double[] destination = Place(kind, target, posed, q, spin);
        for (int i = 0; i < 3; i++) own[i] = (1 - blend) * own[i] + blend * destination[i];
        return own;
    }

    public float[] CellLayout(TurnData turn)
    {
        var data = new float[600 * LayoutRows * 4];
        for (int cell = 0; cell < 600; cell++)
        {
            for (int i = 0; i < 4; i++) data[cell * 4 + i] = (float)centres[cell][i];
            data[(600 + cell) * 4] = turn.TargetCells(false).Span[cell];
            data[(600 + cell) * 4 + 1] = turn.TargetCells(true).Span[cell];
            for (int row = 0; row < 4; row++) for (int i = 0; i < 4; i++)
            {
                data[((row + 2) * 600 + cell) * 4 + i] = (float)ringCharts[cell][row * 4 + i];
                data[((row + 6) * 600 + cell) * 4 + i] = (float)sheetCharts[cell][row * 4 + i];
            }
        }
        return data;
    }

    public static void Drag(double[] q, double[] spin, double ax, double ay)
    {
        Finite(q, 16, "Q"); Finite(spin, 16, "spin");
        if (!double.IsFinite(ax) || !double.IsFinite(ay)) throw Json.Error("rotate", "finite drag angles required");
        Geometry.Rotate(q, 0, 3, ax); Geometry.Rotate(q, 1, 3, ay);
        Geometry.Rotate(spin, 0, 2, ax);
        double c = Math.Cos(ay), s = Math.Sin(ay);
        for (int j = 0; j < 4; j++)
        {
            double y = spin[j * 4 + 1], z = spin[j * 4 + 2];
            spin[j * 4 + 1] = c * y - s * z;
            spin[j * 4 + 2] = s * y + c * z;
        }
    }

    public CellColouring Colouring(ParameterSet look, CellStructure structure)
    {
        int classes = look.Integer("classes");
        bool hopf = look.Structure(1).Projection is ProjectionKind.HopfRings or ProjectionKind.RingSheet;
        int[]? colours = hopf ? EquitableColouring(20, Hopf.Graph, classes) : CellStructure.ProperColouring(20, structure.RingGraph, classes);
        return new CellColouring(hopf ? Hopf.RingOf : structure.RingOf, colours ?? throw Json.Error("classes", "no proper ring colouring"));
    }

    public static int[]? EquitableColouring(int count, IEnumerable<(int A, int B)> pairs, int classes)
    {
        if (count < 1 || classes < 1) throw Json.Error("classes", "positive counts required");
        var neighbors = Enumerable.Range(0, count).Select(_ => new List<int>()).ToArray();
        foreach (var (a, b) in pairs)
        {
            if (a < 0 || b < 0 || a >= count || b >= count) throw Json.Error("ring graph", "invalid vertex");
            if (a == b) return null;
            neighbors[a].Add(b); neighbors[b].Add(a);
        }
        int low = count / classes, high = low + (count % classes == 0 ? 0 : 1);
        int[] colours = Enumerable.Repeat(-1, count).ToArray(), used = new int[classes];
        bool Search(int vertex)
        {
            if (used.Sum(n => Math.Max(0, low - n)) > count - vertex) return false;
            if (vertex == count) return true;
            for (int colour = 0; colour < classes; colour++)
            {
                if (used[colour] == high || neighbors[vertex].Any(n => colours[n] == colour)) continue;
                colours[vertex] = colour; used[colour]++;
                if (Search(vertex + 1)) return true;
                used[colour]--;
            }
            colours[vertex] = -1;
            return false;
        }
        return Search(0) ? colours : null;
    }
}
