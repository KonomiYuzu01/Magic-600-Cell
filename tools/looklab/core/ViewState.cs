using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;

namespace LookLab.Core;

// What Look Lab keeps across a data reload and a restart: the camera, the look being edited and the
// state of its panels. DefaultDigest names the default preset the look was edited from; when the
// default changes on disk, the new default is shown instead of the kept look.
public sealed record ViewState(IReadOnlyList<double> Q, IReadOnlyList<double> Spin, string DefaultDigest, Preset Preset,
    IReadOnlyDictionary<string, string> Ui);

public static class ViewStates
{
    public const string Format = "magic600-look-view";

    public static string Digest(ReadOnlySpan<byte> bytes) => Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();

    public static ViewState Parse(string text, ParameterSchema schema)
    {
        using var document = Json.Parse(text);
        JsonElement root = document.RootElement;
        Json.Keys(root, "format", "version", "q", "spin", "defaultPreset", "preset", "ui");
        Json.Header(root, Format);
        double[] q = Rotation(Json.Get(root, "q"), "q"), spin = Rotation(Json.Get(root, "spin"), "spin");
        string digest = Json.Text(root, "defaultPreset");
        if (digest.Length != 64 || digest.Any(c => !Uri.IsHexDigit(c) || char.IsUpper(c))) throw Json.Error("defaultPreset", "lowercase sha256 required");
        Preset preset = Presets.Parse(Json.Get(root, "preset").GetRawText(), schema);
        JsonElement ui = Json.Get(root, "ui");
        if (ui.ValueKind != JsonValueKind.Object) throw Json.Error("ui", "expected an object of strings");
        var values = new SortedDictionary<string, string>(StringComparer.Ordinal);
        foreach (JsonProperty entry in ui.EnumerateObject())
        {
            if (string.IsNullOrWhiteSpace(entry.Name) || !values.TryAdd(entry.Name, Json.String(entry.Value, entry.Name)))
                throw Json.Error("ui", "empty or repeated key");
        }
        return new ViewState(Array.AsReadOnly(q), Array.AsReadOnly(spin), digest, preset, new ReadOnlyDictionary<string, string>(values));
    }

    private static double[] Rotation(JsonElement value, string id)
    {
        double[] m = Json.Array(value, id).Select(v => Json.Number(v, id)).ToArray();
        Validate(m, id);
        return m;
    }

    private static void Validate(IReadOnlyList<double> m, string id)
    {
        if (m.Count != 16 || m.Any(v => !double.IsFinite(v))) throw Json.Error(id, "16 finite numbers required");
        for (int i = 0; i < 4; i++) for (int j = 0; j < 4; j++)
        {
            double dot = 0;
            for (int k = 0; k < 4; k++) dot += m[i * 4 + k] * m[j * 4 + k];
            if (Math.Abs(dot - (i == j ? 1 : 0)) > 1e-6) throw Json.Error(id, "rows must be orthonormal");
        }
    }

    public static byte[] Bytes(ViewState state, ParameterSchema schema)
    {
        Validate(state.Q, "q"); Validate(state.Spin, "spin");
        using var memory = new MemoryStream();
        using (var writer = new Utf8JsonWriter(memory, new JsonWriterOptions { Indented = true, Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping }))
        {
            writer.WriteStartObject();
            writer.WriteString("format", Format); writer.WriteNumber("version", 1);
            foreach ((string name, IReadOnlyList<double> m) in new[] { ("q", state.Q), ("spin", state.Spin) })
            {
                writer.WriteStartArray(name);
                foreach (double v in m) writer.WriteRawValue(v.ToString("R", CultureInfo.InvariantCulture));
                writer.WriteEndArray();
            }
            writer.WriteString("defaultPreset", state.DefaultDigest);
            writer.WritePropertyName("preset");
            writer.WriteRawValue(Presets.CanonicalBytes(state.Preset, schema));
            writer.WriteStartObject("ui");
            foreach (var entry in state.Ui.OrderBy(e => e.Key, StringComparer.Ordinal)) writer.WriteString(entry.Key, entry.Value);
            writer.WriteEndObject(); writer.WriteEndObject();
        }
        memory.WriteByte(10);
        return Encoding.UTF8.GetBytes(Encoding.UTF8.GetString(memory.ToArray()).Replace("\r\n", "\n", StringComparison.Ordinal));
    }

    // The kept look applies only while the default it was edited from is still the default on disk.
    public static Preset Resolve(ViewState kept, Preset currentDefault, string currentDefaultDigest) =>
        kept.DefaultDigest == currentDefaultDigest ? kept.Preset : currentDefault;
}

// The values the interactive lab opens with, over the default preset. Checks, stills and shots do not read it.
public static class OpeningLook
{
    public static Preset Apply(Preset preset, string text, ParameterSchema schema)
    {
        using var document = Json.Parse(text);
        JsonElement root = document.RootElement;
        Json.Keys(root, "format", "version", "params");
        Json.Header(root, "magic600-look-opening");
        JsonElement values = Json.Get(root, "params");
        if (values.ValueKind != JsonValueKind.Object) throw Json.Error("params", "expected an object");
        ParameterSet result = preset.Params;
        foreach (JsonProperty entry in values.EnumerateObject())
        {
            ParameterDefinition p = schema[entry.Name];
            ParameterValue value = ParameterSchema.ReadValue(p, entry.Value);
            ParameterSchema.ValidateValue(p, value);
            result = result.With(entry.Name, value);
        }
        return preset with { Params = result };
    }
}

// One poll of the data files Look Lab reloads: path to length and write time.
public sealed class DataWatch
{
    private readonly string root;
    private readonly string[] folders;
    private readonly string[] files;
    private Dictionary<string, (long Length, DateTime Written)> last;

    public DataWatch(string root, IEnumerable<string> folders, IEnumerable<string> files)
    {
        this.root = root; this.folders = folders.ToArray(); this.files = files.ToArray();
        last = Snapshot();
    }

    private Dictionary<string, (long, DateTime)> Snapshot()
    {
        var result = new Dictionary<string, (long, DateTime)>(StringComparer.Ordinal);
        IEnumerable<string> found = files.Select(f => Path.Combine(root, f)).Where(File.Exists);
        foreach (string folder in folders.Select(f => Path.Combine(root, f)).Where(Directory.Exists))
            found = found.Concat(Directory.EnumerateFiles(folder, "*.json", SearchOption.AllDirectories));
        foreach (string path in found)
        {
            var info = new FileInfo(path);
            result[Path.GetRelativePath(root, path).Replace('\\', '/')] = (info.Length, info.LastWriteTimeUtc);
        }
        return result;
    }

    // The files that were added, removed or rewritten since the last call, in ordinal order.
    public IReadOnlyList<string> Changed()
    {
        var now = Snapshot();
        string[] changed = now.Keys.Union(last.Keys).Where(k => !now.TryGetValue(k, out var a) || !last.TryGetValue(k, out var b) || a != b)
            .OrderBy(k => k, StringComparer.Ordinal).ToArray();
        last = now;
        return changed;
    }
}
