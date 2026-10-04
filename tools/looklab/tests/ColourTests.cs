using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using LookLab.Core;

internal static partial class Program
{
    private static void ColourConversions()
    {
        double[][] samples = { new[] { 0.0, 0.0, 0.0 }, new[] { 1.0, 1.0, 1.0 }, new[] { 1.0, 0.0, 0.0 }, new[] { .04, .5, .83 }, new[] { -.01, .3, 1.1 } };
        foreach (double[] rgb in samples)
        {
            double[] lab = Colour.LinearToOklab(rgb), result = Colour.OklabToLinear(lab);
            for (int i = 0; i < 3; i++) Near(result[i], rgb[i], 1e-12);
            OklchColour lch = Colour.OklabToOklch(lab);
            double[] fromLch = Colour.Oklch(lch.L, lch.C, lch.H);
            for (int i = 0; i < 3; i++) Near(fromLch[i], lab[i], 1e-12);
        }
        foreach (double srgb in new[] { 0, .003, .04045, .3, 1.0 }) Near(Colour.LinearToSrgb(Colour.SrgbToLinear(srgb)), srgb, 3e-8);
        Sequence(Colour.Oklch(.7, .1, -60), Colour.Oklch(.7, .1, 300));
        for (int l = 0; l <= 10; l++) for (int h = 0; h < 360; h += 20)
        {
            GamutColour mapped = Colour.MapToGamut(l / 10.0, .4, h);
            Check(mapped.Ok && mapped.Chroma <= .4 && mapped.Linear.All(x => x >= 0 && x <= 1), "gamut mapping failed");
        }
        Sequence(Colour.MapToGamut(1, .2, 30).Linear, new[] { 1.0, 1.0, 1.0 });
        Equal(Colour.MapToGamut(0, .2, 30).Chroma, 0.0);
        Check(!Colour.MapToGamut(-.1, .1, 0).Ok && !Colour.MapToGamut(.5, double.NaN, 0).Ok && !Colour.MapToGamut(.5, .1, double.PositiveInfinity).Ok, "invalid colour accepted");
        GamutColour reduced = Colour.MapToGamut(.95, .4, 20); Check(reduced.Chroma < .4, "out-of-gamut chroma was not reduced");
        Refuse(() => Colour.SimulateCvd(new[] { 1.0, 0.0, 0.0 }, "unknown"), "CVD");
        double[] bg = Colour.Background(Schema.Defaults).Srgb;
        for (int i = 0; i < 3; i++) Near(bg[i], new[] { .13, .145, .16 }[i], 1e-12);
    }

    private static void PaletteGraph()
    {
        var s = Mesh.CellStructure;
        PaletteReport report = Colour.Report(Schema.Defaults, s);
        Equal(report.SameRingAdjacencies, 600); Equal(report.Modes.Count, 4);
        Check(report.ClassPairs.All(p => p.A < p.B), "same-class or duplicate pairs in distance graph");
        int[] colouring = CellStructure.ProperColouring(20, s.RingGraph, Schema.Defaults.Integer("classes"))!;
        var expected = s.RingGraph.Select(p => (Math.Min(colouring[p.A], colouring[p.B]), Math.Max(colouring[p.A], colouring[p.B]))).Distinct().OrderBy(p => p.Item1).ThenBy(p => p.Item2);
        Sequence(report.ClassPairs, expected);
        Check(report.Modes.All(m => m.MinDistance > 0 && m.MinBackgroundLightness > 0 && m.MeetsDistance == null && m.MeetsBackground == null), "core invented thresholds");
        Check(Colour.Report(Schema.Defaults, s, new PaletteThresholds(0, 0)).Modes.All(m => m.MeetsDistance == true && m.MeetsBackground == true), "caller thresholds not applied");
        Check(Colour.Report(Schema.Defaults, s, new PaletteThresholds(10, 10)).Modes.All(m => m.MeetsDistance == false && m.MeetsBackground == false), "caller thresholds not applied");
        Refuse(() => Colour.Report(Schema.Defaults, s, new PaletteThresholds(-1, 0)), "thresholds");
        Refuse(() => Colour.Report(Schema.Defaults.With("classes", new IntegerValue(3)), s), "classes");
    }

    private static Dictionary<string, object> TasteLook(ParameterSet p) => ParameterSchema.TasteIds.ToDictionary(id => id, id => p.Values[id] switch
    {
        NumberValue n => (object)n.Value, IntegerValue n => n.Value, _ => throw new Exception("unexpected Taste Lab type")
    });

    private static void VectorNear(double[] actual, JsonElement expected, double tolerance = 1e-11)
    {
        Equal(actual.Length, expected.GetArrayLength());
        for (int i = 0; i < actual.Length; i++) Near(actual[i], expected[i].GetDouble(), tolerance);
    }

    private static void ColourJs()
    {
        double[][] rgb = { new[] { 0.0, 0.0, 0.0 }, new[] { 1.0, 1.0, 1.0 }, new[] { 1.0, 0.0, 0.0 }, new[] { .13, .5, .95 }, new[] { -.05, .2, 1.1 } };
        double[][] lch = { new[] { 0.0, .3, 0.0 }, new[] { 1.0, .3, 360.0 }, new[] { .5, .4, 20.0 }, new[] { .75, .15, -70.0 }, new[] { .99, .5, 280.0 } };
        ParameterSet[] looks = {
            Schema.Defaults,
            Schema.Defaults.With("classes", new IntegerValue(4)).With("hueRotation", new NumberValue(352)).With("hueSpread", new NumberValue(72)).With("lightness", new NumberValue(.5)).With("lightnessAlt", new NumberValue(.15)).With("chroma", new NumberValue(.2)).With("bgLightness", new NumberValue(.9)),
            Schema.Defaults.With("classes", new IntegerValue(8)).With("hueRotation", new NumberValue(143)).With("hueSpread", new NumberValue(300)).With("lightness", new NumberValue(.82)).With("lightnessAlt", new NumberValue(.08)),
            Schema.Defaults.With("classes", new IntegerValue(7)).With("lightnessAlt", new NumberValue(0)).With("bgTint", new NumberValue(.05))
        };
        PaletteReport[] reports = looks.Select(p => Colour.Report(p, Mesh.CellStructure)).ToArray();
        using JsonDocument result = Node(new { task = "colour", rgb, lch, looks = looks.Select(TasteLook), pairs = reports.Select(r => r.ClassPairs.Select(p => new[] { p.A, p.B })) });
        JsonElement reference = result.RootElement;
        for (int i = 0; i < rgb.Length; i++)
        {
            VectorNear(Colour.LinearToOklab(rgb[i]), reference.GetProperty("conversions")[i].GetProperty("lab"));
            VectorNear(Colour.OklabToLinear(Colour.LinearToOklab(rgb[i])), reference.GetProperty("conversions")[i].GetProperty("roundTrip"));
            foreach (string mode in Colour.Modes.Skip(1)) VectorNear(Colour.SimulateCvd(rgb[i], mode), reference.GetProperty("conversions")[i].GetProperty("cvd").GetProperty(mode));
        }
        for (int i = 0; i < lch.Length; i++)
        {
            GamutColour mapped = Colour.MapToGamut(lch[i][0], lch[i][1], lch[i][2]); var expected = reference.GetProperty("mapped")[i];
            Equal(mapped.Ok, expected.GetProperty("ok").GetBoolean()); Near(mapped.Chroma, expected.GetProperty("C").GetDouble());
            VectorNear(mapped.Linear, expected.GetProperty("linear")); VectorNear(mapped.Srgb, expected.GetProperty("srgb")); VectorNear(mapped.Lab, expected.GetProperty("lab"));
        }
        for (int i = 0; i < looks.Length; i++)
        {
            var expected = reference.GetProperty("reports")[i]; GamutColour[] palette = Colour.Palette(looks[i]);
            for (int c = 0; c < palette.Length; c++) { VectorNear(palette[c].Lab, expected.GetProperty("palette")[c].GetProperty("lab")); VectorNear(palette[c].Srgb, expected.GetProperty("palette")[c].GetProperty("srgb")); }
            VectorNear(Colour.Background(looks[i]).Linear, expected.GetProperty("bg").GetProperty("linear"));
            foreach (PaletteModeReport mode in reports[i].Modes)
            {
                var oracle = expected.GetProperty("modes").GetProperty(mode.Mode);
                Near(mode.MinDistance, oracle.GetProperty("distance").GetDouble(), 1e-11);
                Near(mode.MinBackgroundLightness, oracle.GetProperty("background").GetDouble(), 1e-11);
                Near(mode.MinDistance, expected.GetProperty("hardCheck").GetProperty("minDeltaE").GetProperty(mode.Mode).GetDouble(), 1e-11);
                if (mode.Mode == "normal") Near(mode.MinBackgroundLightness, expected.GetProperty("hardCheck").GetProperty("minBgL").GetDouble(), 1e-11);
            }
        }
        foreach (JsonElement p in reference.GetProperty("ranges").EnumerateArray())
        {
            var definition = Schema[p.GetProperty("id").GetString()!]; Near(definition.Min!.Value, p.GetProperty("min").GetDouble()); Near(definition.Max!.Value, p.GetProperty("max").GetDouble());
        }
    }

    private static void EaseTests()
    {
        foreach (double a in new[] { 0, .25, .5, .75, 1.0 }) foreach (double b in new[] { 0, .25, .5, .75, 1.0 })
        {
            Near(Easing.Ease(0, a, b), 0, 1e-12); Near(Easing.Ease(1, a, b), 1, 1e-12);
            double previous = -1;
            for (int i = 0; i <= 1000; i++) { double value = Easing.Ease(i / 1000.0, a, b); Check(value >= previous, "easing decreased"); Check(value >= 0 && value <= 1, "easing outside unit range"); previous = value; }
        }
        Near(Easing.TurnAngle(-10, 190, Math.PI, .2, .3), 0, 1e-12);
        Near(Easing.TurnAngle(400, 190, Math.PI, .2, .3), Math.PI, 1e-12);
        Near(Easing.TurnAngle(95, 190, Math.PI, .2, .2), Math.PI / 2, 1e-11);
        Refuse(() => Easing.Ease(.5, 1.1, 0), "ease"); Refuse(() => Easing.TurnAngle(10, 0, Math.PI, .2, .2), "turnMs");
    }

    private static void EaseJs()
    {
        var points = new List<double[]>();
        foreach (double a in new[] { 0, .1, 1.0 / 3, .7, 1.0 }) foreach (double b in new[] { 0, .2, 1.0 / 3, .9, 1.0 })
            foreach (double t in new[] { 0, .0001, .1, .5, .9, .9999, 1.0 }) points.Add(new[] { t, a, b });
        using var reference = Node(new { task = "easing", values = points });
        for (int i = 0; i < points.Count; i++) Near(Easing.Ease(points[i][0], points[i][1], points[i][2]), reference.RootElement[i].GetDouble(), 1e-9);
    }
}
