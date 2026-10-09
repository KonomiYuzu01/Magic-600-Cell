using System;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using LookLab.App;
using LookLab.App.LL5;
using LookLab.Core;

internal static partial class Program
{
    private static void LL5CvdReference()
    {
        foreach (string mode in Colour.Modes)
            foreach (double[] linear in CvdCheckData.LinearSamples)
                CvdCheckData.Require(CvdTransform.Apply(linear.Select(x => (float)x).ToArray(), CvdTransform.Rows(mode), mode != "normal"),
                    CvdCheckData.Reference(linear, mode), "LL5 coefficients");
        Refuse(() => CvdTransform.Rows("unknown"), "CVD");
        // Negative coefficients must survive the upload; clamping is after
        // the multiplication, not on each coefficient or in encoded sRGB.
        Check(CvdTransform.Rows("protan")[0][2] < 0, "CVD coefficient was clamped");
        float[] red = { 1, 0, 0 };
        Check(CvdTransform.Apply(red, CvdTransform.Rows("protan"), true)[2] == 0, "CVD output clamp missing");
        Check(CvdTransform.Apply(red, CvdTransform.Rows("protan"), false)[0] == 1, "normal preview changed the colour");
    }

    private static void LL5ShaderProbe()
    {
        string drawing = Read("app/shaders/cells.gdshader");
        var source = new ShaderSource(drawing);
        Check(source.Fragment().Contains(source.FragmentBody, StringComparison.Ordinal), "CVD probe changed the drawing fragment body");
        Check(source.FragmentBody.Contains("if (cvd_enabled)", StringComparison.Ordinal) &&
            source.FragmentBody.Contains("dot(cvd_r0.xyz, ALBEDO)", StringComparison.Ordinal) &&
            source.FragmentBody.Contains("dot(cvd_r1.xyz, ALBEDO)", StringComparison.Ordinal) &&
            source.FragmentBody.Contains("dot(cvd_r2.xyz, ALBEDO)", StringComparison.Ordinal), "CVD drawing step missing");
        Equal(source.Uniforms.Single(u => u.Name == "cvd_enabled").Type, "bool");
        Check(drawing.Contains("uniform bool cvd_enabled = false;", StringComparison.Ordinal), "CVD default affects normal or geometry draws");
        foreach (int row in Enumerable.Range(0, 3)) Equal(source.Uniforms.Single(u => u.Name == "cvd_r" + row).Type, "vec4");
        string vertex = CvdProbeSource.Vertex(source, CvdCheckData.LinearSamples.Length);
        foreach (ShaderVarying varying in source.Varyings)
            Check(vertex.Contains($"layout(location={varying.Location}) out {varying.Type} {varying.Name};", StringComparison.Ordinal), "probe varying does not match drawing input");
        Check(vertex.Contains("sticker_colour=sample_colours[i].rgb", StringComparison.Ordinal) && vertex.Contains("/13.0", StringComparison.Ordinal), "probe does not upload one known colour per pixel");
        var values = source.Uniforms.Where(u => u.Type != "sampler2D").ToDictionary(u => u.Name,
            u => new float[u.Type == "mat4" ? 16 : u.Type == "vec4" ? 4 : 1]);
        values["cvd_enabled"][0] = 1;
        float[][] rows = CvdTransform.Rows("tritan");
        for (int i = 0; i < 3; i++) Array.Copy(rows[i], values["cvd_r" + i], 3);
        float[] packed = source.PackParameters(name => values[name]);
        foreach (int i in Enumerable.Range(0, 3))
            Sequence(packed.Skip(source.Uniforms.Single(u => u.Name == "cvd_r" + i).Offset * 4).Take(3), rows[i]);
        values["cvd_r0"] = new float[4];
        float[] wrong = source.PackParameters(name => values[name]);
        Equal(packed.Zip(wrong).Count(p => p.First != p.Second), 3);
    }

    private static void LL5PalettePresentation()
    {
        PaletteReport report = Colour.Report(Schema.Defaults, Mesh.CellStructure);
        string text = PaletteDisplay.Text(report, null);
        Check(!text.Contains("(pass)", StringComparison.Ordinal) && !text.Contains("(fail)", StringComparison.Ordinal), "panel chose its own thresholds");
        foreach (PaletteModeReport mode in report.Modes)
        {
            Check(text.Contains(mode.Mode + ":", StringComparison.Ordinal), "missing CVD report mode");
            Check(text.Contains(mode.MinDistance!.Value.ToString("G6", CultureInfo.InvariantCulture), StringComparison.Ordinal), "missing class distance");
            Check(text.Contains(mode.MinBackgroundLightness!.Value.ToString("G6", CultureInfo.InvariantCulture), StringComparison.Ordinal), "missing background lightness");
        }
        foreach (var pair in report.ClassPairs) Check(text.Contains($"{pair.A}/{pair.B}", StringComparison.Ordinal), "missing comparison pair");
        Check(text.Contains($"Same-ring face adjacencies: {report.SameRingAdjacencies}", StringComparison.Ordinal), "same-ring count missing");
        foreach (double threshold in new[] { 0.0, 2.0 })
        {
            var marks = new PaletteThresholds(threshold, threshold);
            string marked = PaletteDisplay.Text(Colour.Report(Schema.Defaults, Mesh.CellStructure, marks), marks);
            Check(marked.Contains(threshold == 0 ? "(pass)" : "(fail)", StringComparison.Ordinal), "caller marks were not displayed");
        }
        ParameterSet invalid = Schema.Defaults.With("lightness", new NumberValue(.85)).With("lightnessAlt", new NumberValue(.2));
        PaletteReport failed = Colour.Report(invalid, Mesh.CellStructure);
        string failure = PaletteDisplay.Text(failed, null);
        Check(failed.GamutFailures.Count > 0 && failure.Contains("unavailable", StringComparison.Ordinal), "failed palette invents distances");
        foreach (GamutFailure gamut in failed.GamutFailures)
            Check(failure.Contains("class " + gamut.ClassIndex + ": " + gamut.Reason, StringComparison.Ordinal), "failure class or reason missing");
        Check(!failure.Contains("(fail)", StringComparison.Ordinal), "gamut failure assigns threshold marks without caller input");
        var backgroundFailure = failed with { GamutFailures = new[] { new GamutFailure(null, "synthetic reason") } };
        Check(PaletteDisplay.Text(backgroundFailure, null).Contains("background: synthetic reason", StringComparison.Ordinal), "background gamut reason missing");
    }

    private static void LL5CostPresentation()
    {
        CostTable table = CostTable.Load(FileAt("data/cost.json"), Schema);
        var rows = CostDisplay.Rows(Schema, Schema.Defaults, table);
        Equal(rows.Count, Schema.Parameters.Count(p => p.CostFeature != null));
        Check(rows.All(r => r.Cost == "not measured"), "null cost replaced with a number");
        Equal(CostDisplay.Summary(rows), "cost not measured");
        Check(CostDisplay.Rows(Schema, Schema.Defaults, table, false).All(r => r.Cost != "not measured"), "null-branch injected fault has no effect");
        var document = JsonNode.Parse(Read("data/cost.json"))!;
        document["features"]!["edgeWeight"] = new JsonObject { ["milliseconds"] = 1.25, ["source"] = "synthetic full-detail fixture" };
        document["features"]!["unused feature"] = new JsonObject { ["milliseconds"] = 99, ["source"] = "unmapped fixture" };
        table = CostTable.Parse(document.ToJsonString(), Schema);
        ParameterSet changed = Schema.Defaults.With("edgeWeight", new NumberValue(.5));
        var measuredRows = CostDisplay.Rows(Schema, changed, table);
        CostRow measured = measuredRows.Single(r => r.Parameter == "edgeWeight");
        Equal(measured.Feature, Schema["edgeWeight"].CostFeature); Equal(measured.Setting, "0.5");
        Equal(measured.Cost, "1.25 ms (source: synthetic full-detail fixture)");
        Check(CostDisplay.Summary(measuredRows).Contains(measured.Cost, StringComparison.Ordinal), "meter status loses measured source");
        Check(!measuredRows.Any(r => r.Feature == "unused feature"), "meter shows a feature the preset does not set");
        Equal(measuredRows.Count(r => r.Cost == "not measured"), rows.Count - 1);
        Equal(CostDisplay.Measurement(new CostMeasurement(0, "zero fixture")), "0 ms (source: zero fixture)");
    }

    private static void LL5HarnessTests()
    {
        var run = Run("python", new[] { "-B", FileAt("tests/ll5_harness_test.py") });
        Check(run.Code == 0, run.Out + run.Error);
        Check(System.Text.RegularExpressions.Regex.IsMatch(run.Error, @"Ran [1-9][0-9]* tests"), "harness cases did not run");
        Console.WriteLine(run.Error.Split('\n').First(line => line.StartsWith("Ran ", StringComparison.Ordinal)).Trim());
        var extra = Run("python", new[] { "-B", FileAt("check.py"), "extra" }); Equal(extra.Code, 2);
    }
}
