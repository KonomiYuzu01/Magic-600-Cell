using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json.Nodes;
using System.Xml.Linq;
using LookLab.Core;

internal static partial class Program
{
    private static void HarnessTests()
    {
        var run = Run("python", new[] { "-B", FileAt("tests/harness_test.py") });
        Check(run.Code == 0, run.Out + run.Error); Check(run.Error.Contains("Ran 6 tests", StringComparison.Ordinal), "harness cases did not all run");
        var extra = Run("python", new[] { "-B", FileAt("check.py"), "extra" }); Equal(extra.Code, 2);
    }

    private static void BuildContract()
    {
        var props = XDocument.Parse(Read("Directory.Build.props"));
        var expected = new Dictionary<string, string> { ["TargetFramework"] = "net8.0", ["Nullable"] = "enable", ["ImplicitUsings"] = "disable", ["UseAppHost"] = "false", ["InvariantGlobalization"] = "true", ["NuGetAudit"] = "false", ["Deterministic"] = "true", ["TreatWarningsAsErrors"] = "true" };
        foreach (var p in expected) Equal(props.Descendants(p.Key).Single().Value, p.Value);
        foreach (string file in new[] { "core/LookLab.Core.csproj", "tests/LookLab.Tests.csproj" }) Check(!XDocument.Parse(Read(file)).Descendants("PackageReference").Any(), "offline project has package reference");
        var config = XDocument.Parse(Read("nuget.config"));
        foreach (string key in new[] { "packageSources", "fallbackPackageFolders" }) { Equal(config.Descendants(key).Single().Elements().Count(), 1); Equal(config.Descendants(key).Single().Elements().Single().Name.LocalName, "clear"); }
    }

    private static void ParameterContract()
    {
        Equal(Schema.Parameters.Count, 42); Schema.Validate(Schema.Defaults);
        Sequence(Schema.Parameters.Take(18).Select(p => p.Id), ParameterSchema.TasteIds);
        (string Id, double Min, double Max)[] ranges = {
            ("hueRotation",0,360), ("hueSpread",60,360), ("lightness",.45,.85), ("lightnessAlt",0,.2), ("chroma",.04,.2), ("classes",4,8),
            ("bgLightness",.05,.95), ("bgHue",0,360), ("bgTint",0,.05), ("gap",0,.3), ("edgeWeight",0,3), ("edgeBrightness",0,1),
            ("gloss",0,1), ("glow",0,1), ("fog",0,1), ("turnMs",150,900), ("easeA",0,1), ("easeB",0,1)
        };
        foreach (var (id, min, max) in ranges) { Near(Schema[id].Min!.Value, min); Near(Schema[id].Max!.Value, max); }
        Near(Schema.Defaults.Number("gap"), .24); Near(Schema.Defaults.Number("stickerShrink"), .82);
        Equal(Schema.Defaults.Integer("classes"), 6); Equal(Schema.Defaults.Choice("finish"), "matte"); Check(Schema.Defaults.Boolean("cellsVisible"), "cells hidden by default");
        Equal(Schema.Defaults.Colour("accentPrimary").H, 220.0); Near(Schema.Defaults.Curve("settle").A, 1.0 / 3);
        Check(Enum.GetValues<ParameterKind>().All(k => Schema.Parameters.Any(p => p.Kind == k)), "schema does not exercise all six kinds");
        Check(!Schema.Parameters.Any(p => p.Id.Contains("cvd", StringComparison.OrdinalIgnoreCase) || p.Id == "gridOverlay"), "app state leaked into preset schema");
    }

    private static void ParameterRefusals()
    {
        void Bad(Action<JsonNode> change, string id)
        {
            JsonNode node = JsonNode.Parse(Read("data/parameters.json"))!; change(node); Refuse(() => ParameterSchema.Parse(node.ToJsonString()), id);
        }
        Bad(n => n["version"] = 2, "version"); Bad(n => n["format"] = "unknown", "format");
        Bad(n => n["parameters"]![0]!["group"] = "unknown", "hueRotation");
        Bad(n => n["parameters"]![0]!["kind"] = "unknown", "hueRotation");
        Bad(n => n["parameters"]![1]!["id"] = "hueRotation", "hueRotation");
        Bad(n => n["parameters"]![0]!["min"] = 400, "hueRotation");
        Bad(n => n["parameters"]![0]!["step"] = 0, "hueRotation");
        Bad(n => n["parameters"]![0]!["default"] = 361, "hueRotation");
        Bad(n => n["parameters"]![5]!["step"] = .5, "classes");
        Bad(n => n["parameters"]![18]!["values"] = new JsonArray("perspective", "perspective"), "projection");
        Bad(n => n["parameters"]![0]!["description"] = "two\nlines", "hueRotation");
    }

    private static void DefaultRoundTrip()
    {
        byte[] bytes = File.ReadAllBytes(FileAt("presets/default.json"));
        Preset preset = Presets.Load(FileAt("presets/default.json"), Schema);
        byte[] actual = Presets.CanonicalBytes(preset, Schema);
        if (!actual.SequenceEqual(bytes))
        {
            string[] a = Encoding.UTF8.GetString(actual).Split('\n'), b = Encoding.UTF8.GetString(bytes).Split('\n');
            var first = a.Zip(b).FirstOrDefault(p => p.First != p.Second);
            throw new Exception("default canonical difference: " + first.First + " expected " + first.Second);
        }
        Sequence(Presets.CanonicalBytes(preset with { Params = Schema.Defaults }, Schema), bytes);
        Check(bytes[^1] == 10 && !bytes.Contains((byte)13) && !(bytes[0] == 239 && bytes[1] == 187), "BOM, CR or missing final newline");
    }

    private static Preset Generated(int seed)
    {
        var rng = new Random(seed); var values = new Dictionary<string, ParameterValue>();
        foreach (ParameterDefinition p in Schema.Parameters)
        {
            double unit = rng.NextDouble();
            values[p.Id] = p.Kind switch
            {
                ParameterKind.Number => new NumberValue(p.Min!.Value + unit * (p.Max!.Value - p.Min.Value)),
                ParameterKind.Integer => new IntegerValue((int)p.Min!.Value + rng.Next((int)(p.Max!.Value - p.Min.Value + 1))),
                ParameterKind.Enum => new EnumValue(p.Options![rng.Next(p.Options.Count)]),
                ParameterKind.Boolean => new BooleanValue(unit < .5),
                ParameterKind.Colour => new ColourValue(new OklchColour(unit, rng.NextDouble(), 360 * rng.NextDouble())),
                ParameterKind.Curve => new CurveValue(new BezierCurve(unit, rng.NextDouble())),
                _ => throw new Exception("kind")
            };
        }
        return new Preset("Generated \"look\" \\ " + seed + " café", seed % 2 == 0 ? "f1" : "f2", "solving", new ParameterSet(values));
    }

    private static void GeneratedRoundTrips()
    {
        using var temp = new TestFolder();
        for (int seed = 0; seed < 50; seed++)
        {
            Preset preset = Generated(seed); byte[] bytes = Presets.CanonicalBytes(preset, Schema);
            Check(bytes[^1] == 10 && !bytes.Contains((byte)13), "generated preset uses noncanonical line endings");
            Sequence(Presets.CanonicalBytes(Presets.Parse(Encoding.UTF8.GetString(bytes), Schema), Schema), bytes);
            string path = Path.Combine(temp.Path, "generated.json"); Presets.Save(path, preset, Schema);
            Sequence(File.ReadAllBytes(path), bytes); Equal(Presets.Load(path, Schema).Name, preset.Name);
        }
    }

    private static void PresetRefusals()
    {
        void Bad(Action<JsonNode> change, string id)
        {
            JsonNode node = JsonNode.Parse(Read("presets/default.json"))!; change(node); Refuse(() => Presets.Parse(node.ToJsonString(), Schema), id);
        }
        Bad(n => n["format"] = "unknown", "format"); Bad(n => n["version"] = 0, "version"); Bad(n => n["version"] = 2, "version");
        Bad(n => n["params"]!["mystery"] = 0, "mystery"); Bad(n => n["params"]!.AsObject().Remove("gap"), "gap");
        Bad(n => n["params"]!["gap"] = "0.24", "gap"); Bad(n => n["params"]!["gap"] = .31, "gap"); Bad(n => n["params"]!["classes"] = 4.5, "classes");
        Bad(n => n["params"]!["cellsVisible"] = 1, "cellsVisible"); Bad(n => n["params"]!["projection"] = "fish-eye", "projection");
        Bad(n => n["params"]!["accentPrimary"]!["L"] = 1.1, "accentPrimary");
        Bad(n => n["params"]!["settle"]!["a"] = -1, "settle"); Bad(n => n["params"]!["settle"]!["extra"] = 0, "settle");
        Bad(n => n["family"] = "f3", "family"); Bad(n => n["scene"] = "unknown", "scene"); Bad(n => n["name"] = " ", "name");
        Bad(n => n["extra"] = 1, "extra"); Bad(n => n.AsObject().Remove("name"), "name");
        Refuse(() => Presets.Parse(Read("presets/default.json").Replace("\"gap\": 0.24", "\"gap\": 1e999", StringComparison.Ordinal), Schema), "gap");
        Refuse(() => Presets.Parse(Read("presets/default.json").Replace("\"gap\": 0.24", "\"gap\": 0.24, \"gap\": 0.25", StringComparison.Ordinal), Schema), "gap");
        Refuse(() => Presets.Parse(Read("presets/default.json").Replace("\"gap\": 0.24", "\"gap\": NaN", StringComparison.Ordinal), Schema), "gap");
        Refuse(() => Presets.CanonicalBytes(Generated(0) with { Params = Schema.Defaults.With("gap", new NumberValue(double.PositiveInfinity)) }, Schema), "gap");
    }

    private static void PresetCultures()
    {
        CultureInfo previous = CultureInfo.CurrentCulture;
        try
        {
            byte[] expected = Presets.CanonicalBytes(Generated(123), Schema);
            foreach (string name in new[] { "de-DE", "fr-FR" })
            {
                var culture = new CultureInfo(name, false); Equal(culture.Name, name);
                Equal(culture.NumberFormat.NumberDecimalSeparator, ".", "named cultures must be invariant in this build");
                CultureInfo.CurrentCulture = culture;
                Sequence(Presets.CanonicalBytes(Generated(123), Schema), expected);
                var hostile = (CultureInfo)culture.Clone(); hostile.NumberFormat.NumberDecimalSeparator = ",";
                CultureInfo.CurrentCulture = hostile;
                Equal((.24).ToString("R", CultureInfo.CurrentCulture), "0,24");
                Sequence(Presets.CanonicalBytes(Generated(123), Schema), expected, "writer used CurrentCulture");
            }
        }
        finally { CultureInfo.CurrentCulture = previous; }
    }

    private static void PresetMigration()
    {
        Equal(PresetMigrations.Migrate("unchanged current text", 1), "unchanged current text");
        foreach (int version in new[] { -1, 0, 2, 99 }) Refuse(() => PresetMigrations.Migrate("{}", version), "version");
    }

    private static JsonNode Export()
    {
        var values = JsonNode.Parse(Read("presets/default.json"))!["params"]!;
        var look = new JsonObject(); foreach (string id in ParameterSchema.TasteIds) look[id] = values[id]!.DeepClone();
        var entries = new JsonArray(); int i = 0;
        foreach (string family in new[] { "f1", "f2" }) foreach (string scene in new[] { "solving", "inspecting", "celebrating" })
            entries.Add(new JsonObject { ["family"] = family, ["scene"] = scene, ["name"] = family + " " + scene, ["look"] = i++ == 0 ? look : null });
        return new JsonObject { ["kind"] = "tastelab-export", ["version"] = 2, ["presets"] = entries };
    }

    private static void ImportNulls()
    {
        string text = Export().ToJsonString(); ImportReport report = TasteImport.Parse(text, Schema);
        Equal(report.Presets.Count, 1); Equal(report.Skips.Count, 5); Equal(report.Presets[0].Family, "f1"); Equal(report.Presets[0].Scene, "solving");
        foreach (string id in ParameterSchema.TasteIds) Equal(report.Presets[0].Params.Values[id], Schema.Defaults.Values[id]);
        foreach (ParameterDefinition p in Schema.Parameters.Where(p => !ParameterSchema.TasteIds.Contains(p.Id))) Equal(report.Presets[0].Params.Values[p.Id], p.Default);
        foreach (ImportSkip skip in report.Skips) Check(skip.Family != null && skip.Scene != null && skip.Reason.Contains("null", StringComparison.Ordinal), "skip omitted reason or identity");
        using var temp = new TestFolder(); string output = Path.Combine(temp.Path, "presets");
        report = TasteImport.Write(text, Schema, output); Equal(Directory.GetFiles(output).Length, 1);
        Sequence(File.ReadAllBytes(Path.Combine(output, "preset-000.json")), Presets.CanonicalBytes(report.Presets[0], Schema));
        JsonNode empty = Export(); empty["presets"]![0]!["look"] = null;
        report = TasteImport.Parse(empty.ToJsonString(), Schema); Equal(report.Presets.Count, 0); Equal(report.Skips.Count, 6);
    }

    private static void ImportRefusals()
    {
        JsonNode node = Export();
        foreach (int version in new[] { 1, 4 }) { node["version"] = version; Refuse(() => TasteImport.Parse(node.ToJsonString(), Schema), "version"); }
        node = Export(); node["version"] = 3;
        node["images"] = new JsonObject { ["bundles"] = new JsonArray(), ["ratings"] = new JsonArray(), ["pairs"] = new JsonArray() };
        Sequence(TasteImport.Parse(node.ToJsonString(), Schema).Presets.Select(p => Presets.CanonicalBytes(p, Schema)).SelectMany(b => b).ToArray(),
            TasteImport.Parse(Export().ToJsonString(), Schema).Presets.Select(p => Presets.CanonicalBytes(p, Schema)).SelectMany(b => b).ToArray());
        node = Export();
        node = Export(); node["presets"]![0]!["look"]!["gap"] = .9;
        Refuse(() => TasteImport.Parse(node.ToJsonString(), Schema), "f1/solving"); Refuse(() => TasteImport.Parse(node.ToJsonString(), Schema), "gap");
        using var temp = new TestFolder(); string output = Path.Combine(temp.Path, "should-not-exist");
        Refuse(() => TasteImport.Write(node.ToJsonString(), Schema, output), "gap"); Check(!Directory.Exists(output), "invalid export wrote output");
        node = Export(); node["presets"]![0]!["look"]!.AsObject().Remove("classes"); Refuse(() => TasteImport.Parse(node.ToJsonString(), Schema), "classes");
        node = Export(); node["presets"]![0]!["look"]!["unknown"] = 1; Refuse(() => TasteImport.Parse(node.ToJsonString(), Schema), "unknown");
    }

    private static void ThemeSets()
    {
        Preset[] set = (from family in new[] { "f1", "f2" } from scene in new[] { "solving", "inspecting", "celebrating" }
                        select new Preset(family + " " + scene, family, scene, Schema.Defaults)).ToArray();
        Check(ThemeSet.Check(set, Schema).Matches, "consistent set rejected");
        set[5] = set[5] with { Params = set[5].Params.With("projection", new EnumValue("orthographic")) };
        ThemeSetReport report = ThemeSet.Check(set, Schema); Check(!report.Matches, "Structure mismatch missed"); Equal(report.Differences.Single().Id, "projection");
        set[5] = set[5] with { Params = Schema.Defaults.With("gap", new NumberValue(.1)) }; Equal(ThemeSet.Check(set, Schema).Differences.Single().Id, "gap");
        set[5] = set[5] with { Params = Schema.Defaults.With("gloss", new NumberValue(.8)) }; Check(ThemeSet.Check(set, Schema).Matches, "non-Structure group constrained");
        Equal(ThemeSet.Check(set.Take(5), Schema).Missing.Single(), "f2/celebrating");
        Refuse(() => ThemeSet.Check(set.Concat(new[] { set[0] }), Schema), "f1/solving");
    }

    private static void ReadmeContract()
    {
        string readme = Read("README.md");
        foreach (string format in new[] { "magic600-look-parameters", "magic600-look-preset", "magic600-look-layout", "magic600-look-flow", "magic600-look-cost", "tastelab-export" })
            Check(readme.Contains(format, StringComparison.Ordinal), "README missing format " + format);
        foreach (ParameterDefinition p in Schema.Parameters) Check(readme.Contains("`" + p.Id + "`", StringComparison.Ordinal), "README missing parameter " + p.Id);
        Check(readme.Contains("work/loop-memory/", StringComparison.Ordinal), "README missing privacy location");
        Check(readme.Contains("not measurements", StringComparison.Ordinal), "README missing estimate qualification");
    }
}
