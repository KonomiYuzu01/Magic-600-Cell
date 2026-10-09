using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.RegularExpressions;

[assembly: InternalsVisibleTo("LookLab.Tests")]

namespace LookLab.Core;

public sealed record Preset(string Name, string? Family, string? Scene, ParameterSet Params);

public static class PresetMigrations
{
    public static string Migrate(string json, int sourceVersion)
    {
        if (sourceVersion == 1) return json;
        throw Json.Error("version", "no migration from version " + sourceVersion + " to version 1 is registered");
    }
}

public static class Presets
{
    public static Preset Load(string path, ParameterSchema schema) => Parse(File.ReadAllText(path), schema);

    public static Preset Parse(string text, ParameterSchema schema) => Parse(text, schema, PresetMigrations.Migrate);

    // Tests inject a migration per call without registering production migrations or changing global state.
    internal static Preset Parse(string text, ParameterSchema schema, Func<string, int, string> migrate)
    {
        using var source = Json.Parse(text);
        if (Json.Text(source.RootElement, "format") != "magic600-look-preset") throw Json.Error("format", "expected magic600-look-preset");
        int version = Json.Integer(Json.Get(source.RootElement, "version"), "version");
        using var document = Json.Parse(migrate(text, version));
        JsonElement root = document.RootElement;
        Json.Header(root, "magic600-look-preset");
        Json.Keys(root, "format", "version", "name", "family", "scene", "params");
        string name = Json.Text(root, "name");
        string? family = NullableText(Json.Get(root, "family"), "family"), scene = NullableText(Json.Get(root, "scene"), "scene");
        var values = new Dictionary<string, ParameterValue>(StringComparer.Ordinal);
        var parameters = Json.Get(root, "params");
        if (parameters.ValueKind != JsonValueKind.Object) throw Json.Error("params", "expected object");
        foreach (JsonProperty entry in parameters.EnumerateObject()) values.Add(entry.Name, ParameterSchema.ReadValue(schema[entry.Name], entry.Value));
        var preset = new Preset(name, family, scene, new ParameterSet(values));
        Validate(preset, schema);
        return preset;
    }

    private static string? NullableText(JsonElement value, string id) => value.ValueKind == JsonValueKind.Null ? null : Json.String(value, id);

    public static void Validate(Preset preset, ParameterSchema schema)
    {
        if (string.IsNullOrWhiteSpace(preset.Name)) throw Json.Error("name", "nonempty name required");
        if (preset.Family != null && preset.Family != "f1" && preset.Family != "f2") throw Json.Error("family", "expected f1, f2 or null");
        if (preset.Scene != null && preset.Scene != "solving" && preset.Scene != "inspecting" && preset.Scene != "celebrating")
            throw Json.Error("scene", "expected solving, inspecting, celebrating or null");
        schema.Validate(preset.Params);
    }

    public static byte[] CanonicalBytes(Preset preset, ParameterSchema schema)
    {
        Validate(preset, schema);
        using var memory = new MemoryStream();
        using (var writer = new Utf8JsonWriter(memory, new JsonWriterOptions { Indented = true, Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping }))
        {
            writer.WriteStartObject();
            writer.WriteString("format", "magic600-look-preset"); writer.WriteNumber("version", 1);
            writer.WriteString("name", preset.Name); writer.WriteString("family", preset.Family); writer.WriteString("scene", preset.Scene);
            writer.WriteStartObject("params");
            foreach (ParameterDefinition p in schema.Parameters)
            {
                writer.WritePropertyName(p.Id);
                WriteValue(writer, preset.Params.Values[p.Id]);
            }
            writer.WriteEndObject(); writer.WriteEndObject();
        }
        memory.WriteByte(10);
        // net8's indented writer uses the platform newline; canonical presets use LF.
        return Encoding.UTF8.GetBytes(Encoding.UTF8.GetString(memory.ToArray()).Replace("\r\n", "\n", StringComparison.Ordinal));
    }

    private static void Number(Utf8JsonWriter writer, double value) => writer.WriteRawValue(value.ToString("R", CultureInfo.InvariantCulture));

    private static void WriteValue(Utf8JsonWriter writer, ParameterValue value)
    {
        switch (value)
        {
            case NumberValue n: Number(writer, n.Value); break;
            case IntegerValue n: writer.WriteNumberValue(n.Value); break;
            case EnumValue e: writer.WriteStringValue(e.Value); break;
            case BooleanValue b: writer.WriteBooleanValue(b.Value); break;
            case ColourValue c:
                writer.WriteStartObject();
                writer.WritePropertyName("L"); Number(writer, c.Value.L);
                writer.WritePropertyName("C"); Number(writer, c.Value.C);
                writer.WritePropertyName("h"); Number(writer, c.Value.H);
                writer.WriteEndObject(); break;
            case CurveValue c:
                writer.WriteStartObject();
                writer.WritePropertyName("a"); Number(writer, c.Value.A);
                writer.WritePropertyName("b"); Number(writer, c.Value.B);
                writer.WriteEndObject(); break;
            default: throw Json.Error("params", "unsupported value type");
        }
    }

    public static void Save(string path, Preset preset, ParameterSchema schema) => File.WriteAllBytes(path, CanonicalBytes(preset, schema));
}

public sealed record ImportSkip(string? Family, string? Scene, string Reason);
public sealed record ImportReport(IReadOnlyList<Preset> Presets, IReadOnlyList<ImportSkip> Skips);

public static class TasteImport
{
    public static ImportReport Parse(string text, ParameterSchema schema)
    {
        using var document = Json.Parse(text);
        JsonElement root = document.RootElement;
        if (Json.Text(root, "kind") != "tastelab-export") throw Json.Error("kind", "expected tastelab-export");
        // Version 3 adds `images` (the Images tab's ratings), which the Look Lab does not read; `presets` is unchanged.
        int version = Json.Integer(Json.Get(root, "version"), "version");
        if (version is not (2 or 3)) throw Json.Error("version", "Taste Lab import supports versions 2 and 3");
        var presets = new List<Preset>(); var skips = new List<ImportSkip>();
        int index = 0;
        foreach (JsonElement entry in Json.Array(Json.Get(root, "presets"), "presets"))
        {
            string label = "presets[" + index++ + "]";
            try
            {
                string family = Json.Text(entry, "family"), scene = Json.Text(entry, "scene"), name = Json.Text(entry, "name");
                label += " " + family + "/" + scene;
                Presets.Validate(new Preset(name, family, scene, schema.Defaults), schema);
                JsonElement look = Json.Get(entry, "look");
                if (look.ValueKind == JsonValueKind.Null)
                {
                    skips.Add(new ImportSkip(family, scene, "look is null; scene was not rated"));
                    continue;
                }
                if (look.ValueKind != JsonValueKind.Object) throw Json.Error("look", "expected object or null");
                foreach (JsonProperty p in look.EnumerateObject())
                    if (!ParameterSchema.TasteIds.Contains(p.Name, StringComparer.Ordinal)) throw Json.Error(p.Name, "unknown Taste Lab id");
                ParameterSet values = schema.Defaults;
                foreach (string id in ParameterSchema.TasteIds) values = values.With(id, ParameterSchema.ReadValue(schema[id], Json.Get(look, id)));
                var preset = new Preset(name, family, scene, values);
                Presets.Validate(preset, schema);
                presets.Add(preset);
            }
            catch (FormatException ex) { throw Json.Error(label, ex.Message); }
        }
        return new ImportReport(presets.AsReadOnly(), skips.AsReadOnly());
    }

    // Validate and serialize the entire export before creating a directory, removing old imports or writing.
    public static ImportReport Write(string text, ParameterSchema schema, string outputFolder)
    {
        ImportReport report = Parse(text, schema);
        byte[][] bytes = report.Presets.Select(p => Presets.CanonicalBytes(p, schema)).ToArray();
        Directory.CreateDirectory(outputFolder);
        foreach (string path in Directory.EnumerateFiles(outputFolder))
            if (Regex.IsMatch(Path.GetFileName(path), @"\Apreset-[0-9]{3}\.json\z", RegexOptions.CultureInvariant)) File.Delete(path);
        for (int i = 0; i < bytes.Length; i++) File.WriteAllBytes(Path.Combine(outputFolder, "preset-" + i.ToString("D3", CultureInfo.InvariantCulture) + ".json"), bytes[i]);
        return report;
    }
}

public sealed record StructureDifference(string Id, string ReferencePreset, string OtherPreset);
public sealed record ThemeSetReport(IReadOnlyList<string> Missing, IReadOnlyList<StructureDifference> Differences)
{
    public bool Matches => Missing.Count == 0 && Differences.Count == 0;
}

public static class ThemeSet
{
    public static ThemeSetReport Check(IEnumerable<Preset> presets, ParameterSchema schema)
    {
        Preset[] set = presets.ToArray();
        foreach (Preset p in set) Presets.Validate(p, schema);
        var missing = new List<string>(); var differences = new List<StructureDifference>();
        var themes = new List<Preset>();
        foreach (string family in new[] { "f1", "f2" }) foreach (string scene in new[] { "solving", "inspecting", "celebrating" })
        {
            Preset[] entries = set.Where(p => p.Family == family && p.Scene == scene).ToArray();
            if (entries.Length == 0) missing.Add(family + "/" + scene);
            else if (entries.Length > 1) throw Json.Error(family + "/" + scene, "duplicate theme preset");
            else themes.Add(entries[0]);
        }
        if (themes.Count > 0) foreach (ParameterDefinition p in schema.Parameters.Where(p => p.Group == "Structure")) foreach (Preset theme in themes.Skip(1))
            if (themes[0].Params.Values[p.Id] != theme.Params.Values[p.Id]) differences.Add(new StructureDifference(p.Id, themes[0].Name, theme.Name));
        return new ThemeSetReport(missing.AsReadOnly(), differences.AsReadOnly());
    }
}
