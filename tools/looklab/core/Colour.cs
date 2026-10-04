using System;
using System.Collections.Generic;
using System.Linq;

namespace LookLab.Core;

public sealed record GamutColour(bool Ok, string? Reason, double Chroma, double[] Linear, double[] Srgb, double[] Lab);
public sealed record PaletteThresholds(double Distance, double BackgroundLightness);
public sealed record PaletteModeReport(string Mode, double MinDistance, double MinBackgroundLightness,
    bool? MeetsDistance, bool? MeetsBackground);
public sealed record PaletteReport(IReadOnlyList<PaletteModeReport> Modes, IReadOnlyList<(int A, int B)> ClassPairs, int SameRingAdjacencies);

public static class Colour
{
    private static readonly double[][] M1 = {
        new[] { 0.4122214708, 0.5363325363, 0.0514459929 }, new[] { 0.2119034982, 0.6806995451, 0.1073969566 }, new[] { 0.0883024619, 0.2817188376, 0.6299787005 }
    };
    private static readonly double[][] M2 = {
        new[] { 0.2104542553, 0.7936177850, -0.0040720468 }, new[] { 1.9779984951, -2.4285922050, 0.4505937099 }, new[] { 0.0259040371, 0.7827717662, -0.8086757660 }
    };
    private static readonly double[][] I1 = Inverse(M1), I2 = Inverse(M2);
    private static readonly Dictionary<string, double[][]> Cvd = new(StringComparer.Ordinal)
    {
        ["protan"] = new[] { new[] { 0.152286, 1.052583, -0.204868 }, new[] { 0.114503, 0.786281, 0.099216 }, new[] { -0.003882, -0.048116, 1.051998 } },
        ["deutan"] = new[] { new[] { 0.367322, 0.860646, -0.227968 }, new[] { 0.280085, 0.672501, 0.047413 }, new[] { -0.011820, 0.042940, 0.968881 } },
        ["tritan"] = new[] { new[] { 1.255528, -0.076749, -0.178779 }, new[] { -0.078411, 0.930809, 0.147602 }, new[] { 0.004733, 0.691367, 0.303900 } }
    };
    public static readonly IReadOnlyList<string> Modes = Array.AsReadOnly(new[] { "normal", "protan", "deutan", "tritan" });

    private static double[][] Inverse(double[][] m)
    {
        double a = m[0][0], b = m[0][1], c = m[0][2], d = m[1][0], e = m[1][1], f = m[1][2], g = m[2][0], h = m[2][1], i = m[2][2];
        double det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
        return new[] { new[] { e * i - f * h, c * h - b * i, b * f - c * e }, new[] { f * g - d * i, a * i - c * g, c * d - a * f },
            new[] { d * h - e * g, b * g - a * h, a * e - b * d } }.Select(row => row.Select(x => x / det).ToArray()).ToArray();
    }

    private static double[] Multiply(double[][] matrix, double[] vector)
    {
        if (vector.Length != 3 || vector.Any(x => !double.IsFinite(x))) throw Json.Error("colour", "three finite components required");
        return matrix.Select(row => row[0] * vector[0] + row[1] * vector[1] + row[2] * vector[2]).ToArray();
    }

    public static double SrgbToLinear(double x) => x <= 0.04045 ? x / 12.92 : Math.Pow((x + 0.055) / 1.055, 2.4);
    public static double LinearToSrgb(double x) => x <= 0.0031308 ? 12.92 * x : 1.055 * Math.Pow(x, 1 / 2.4) - 0.055;
    public static double[] LinearToOklab(double[] rgb) => Multiply(M2, Multiply(M1, rgb).Select(Math.Cbrt).ToArray());
    public static double[] OklabToLinear(double[] lab) => Multiply(I1, Multiply(I2, lab).Select(x => x * x * x).ToArray());
    public static double[] Oklch(double l, double c, double h)
    {
        double angle = ((h % 360 + 360) % 360) * Math.PI / 180;
        return new[] { l, c * Math.Cos(angle), c * Math.Sin(angle) };
    }
    public static OklchColour OklabToOklch(double[] lab)
    {
        if (lab.Length != 3 || lab.Any(x => !double.IsFinite(x))) throw Json.Error("colour", "three finite components required");
        double angle = Math.Atan2(lab[2], lab[1]) * 180 / Math.PI;
        return new OklchColour(lab[0], Math.Sqrt(lab[1] * lab[1] + lab[2] * lab[2]), (angle % 360 + 360) % 360);
    }

    private static bool InGamut(double[] rgb) => rgb.All(x => double.IsFinite(x) && x >= 0 && x <= 1);
    private static GamutColour Mapped(double chroma, double[] linear) => new(true, null, chroma, linear, linear.Select(LinearToSrgb).ToArray(), LinearToOklab(linear));
    private static GamutColour Failed(string reason) => new(false, reason, 0, Array.Empty<double>(), Array.Empty<double>(), Array.Empty<double>());

    public static GamutColour MapToGamut(double l, double c, double h)
    {
        if (!double.IsFinite(l) || l < 0 || l > 1) return Failed("lightness");
        if (!double.IsFinite(c) || c < 0 || !double.IsFinite(h)) return Failed("colour");
        if (l == 0 || l == 1) return Mapped(0, new[] { l, l, l });
        double[] neutral = OklabToLinear(new[] { l, 0.0, 0.0 });
        if (!InGamut(neutral)) return Failed("gamut");
        double[] requested = OklabToLinear(Oklch(l, c, h));
        if (InGamut(requested)) return Mapped(c, requested);
        double low = 0, high = c;
        double[] linear = neutral;
        for (int i = 0; i < 40; i++)
        {
            double mid = (low + high) / 2;
            double[] rgb = OklabToLinear(Oklch(l, mid, h));
            if (InGamut(rgb)) { low = mid; linear = rgb; } else high = mid;
        }
        return Mapped(low, linear);
    }

    public static double Distance(double[] a, double[] b) => Math.Sqrt(a.Select((x, i) => (x - b[i]) * (x - b[i])).Sum());
    public static double[] SimulateCvd(double[] linear, string kind)
    {
        if (!Cvd.TryGetValue(kind, out double[][]? matrix)) throw Json.Error("CVD", "unknown kind " + kind);
        return Multiply(matrix, linear).Select(x => Math.Clamp(x, 0, 1)).ToArray();
    }

    public static GamutColour[] Palette(ParameterSet look)
    {
        int classes = look.Integer("classes");
        if (classes < 4 || classes > 8) throw Json.Error("classes", "expected 4 to 8");
        return Enumerable.Range(0, classes).Select(i => MapToGamut(look.Number("lightness") + (i % 2 == 1 ? look.Number("lightnessAlt") : -look.Number("lightnessAlt")),
            look.Number("chroma"), look.Number("hueRotation") + look.Number("hueSpread") * i / classes)).ToArray();
    }

    public static GamutColour Background(ParameterSet look) => MapToGamut(look.Number("bgLightness"), look.Number("bgTint"), look.Number("bgHue"));

    public static PaletteReport Report(ParameterSet look, CellStructure structure, PaletteThresholds? thresholds = null)
    {
        if (thresholds != null && (!double.IsFinite(thresholds.Distance) || !double.IsFinite(thresholds.BackgroundLightness) || thresholds.Distance < 0 || thresholds.BackgroundLightness < 0))
            throw Json.Error("thresholds", "finite nonnegative thresholds required");
        GamutColour[] colours = Palette(look);
        GamutColour background = Background(look);
        if (!background.Ok || colours.Any(c => !c.Ok)) throw Json.Error("palette", "gamut mapping failed");
        int[] classes = CellStructure.ProperColouring(20, structure.RingGraph, colours.Length) ?? throw Json.Error("classes", "no proper ring colouring");
        var pairs = new HashSet<(int, int)>();
        int sameRing = 0;
        for (int c = 0; c < 600; c++) foreach (int n in structure.FaceNeighbors[c].Where(n => n > c))
        {
            int a = structure.RingOf[c], b = structure.RingOf[n];
            if (a == b) { sameRing++; continue; }
            int ca = classes[a], cb = classes[b];
            pairs.Add((Math.Min(ca, cb), Math.Max(ca, cb)));
        }
        (int A, int B)[] classPairs = pairs.OrderBy(p => p.Item1).ThenBy(p => p.Item2).ToArray();
        var reports = new List<PaletteModeReport>();
        foreach (string mode in Modes)
        {
            double[][] labs = colours.Select(c => mode == "normal" ? c.Lab : LinearToOklab(SimulateCvd(c.Linear, mode))).ToArray();
            double[] bg = mode == "normal" ? background.Lab : LinearToOklab(SimulateCvd(background.Linear, mode));
            double minDistance = classPairs.Select(p => Distance(labs[p.A], labs[p.B])).DefaultIfEmpty(double.PositiveInfinity).Min();
            double minBackground = labs.Min(lab => Math.Abs(lab[0] - bg[0]));
            reports.Add(new PaletteModeReport(mode, minDistance, minBackground, thresholds == null ? null : minDistance >= thresholds.Distance,
                thresholds == null ? null : minBackground >= thresholds.BackgroundLightness));
        }
        return new PaletteReport(reports.AsReadOnly(), Array.AsReadOnly(classPairs), sameRing);
    }
}
