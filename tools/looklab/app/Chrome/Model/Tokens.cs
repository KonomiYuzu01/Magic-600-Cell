using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Json;

// The design-system chrome library (ADR-D01). This folder has no Godot and no Look Lab dependency:
// Look Lab and the product shell both compile it as source.
namespace Magic600.Chrome;

public readonly record struct Rgb(double R, double G, double B)
{
    public string Hex => "#" + string.Concat(new[] { R, G, B }.Select(v => ((int)Math.Round(Math.Clamp(v, 0, 1) * 255)).ToString("x2", CultureInfo.InvariantCulture)));
}

public readonly record struct Oklch(double L, double C, double H)
{
    private static double Encode(double v) => v <= .0031308 ? 12.92 * v : 1.055 * Math.Pow(v, 1 / 2.4) - .055;
    private static double Decode(double v) => v <= .04045 ? v / 12.92 : Math.Pow((v + .055) / 1.055, 2.4);

    private double[] Linear(double chroma)
    {
        double a = chroma * Math.Cos(H * Math.PI / 180), b = chroma * Math.Sin(H * Math.PI / 180);
        double l = Math.Pow(L + .3963377774 * a + .2158037573 * b, 3), m = Math.Pow(L - .1055613458 * a - .0638541728 * b, 3),
            s = Math.Pow(L - .0894841775 * a - 1.2914855480 * b, 3);
        return new[] { 4.0767416621 * l - 3.3077115913 * m + .2309699292 * s, -1.2684380046 * l + 2.6097574011 * m - .3413193965 * s,
            -.0041960863 * l - .7034186147 * m + 1.7076147010 * s };
    }

    // sRGB with the chroma reduced until the colour is inside the gamut; lightness and hue are kept.
    public Rgb ToSrgb()
    {
        double low = 0, high = Math.Max(0, C);
        double[] rgb = Linear(high);
        if (rgb.Any(v => v < -1e-6 || v > 1 + 1e-6))
        {
            for (int i = 0; i < 24; i++)
            {
                double mid = (low + high) / 2;
                if (Linear(mid).Any(v => v < -1e-6 || v > 1 + 1e-6)) high = mid; else low = mid;
            }
            rgb = Linear(low);
        }
        return new Rgb(Encode(Math.Clamp(rgb[0], 0, 1)), Encode(Math.Clamp(rgb[1], 0, 1)), Encode(Math.Clamp(rgb[2], 0, 1)));
    }

    public static double Luminance(Rgb c) => .2126 * Decode(c.R) + .7152 * Decode(c.G) + .0722 * Decode(c.B);
    public static double Contrast(Rgb a, Rgb b)
    {
        double x = Luminance(a), y = Luminance(b);
        return (Math.Max(x, y) + .05) / (Math.Min(x, y) + .05);
    }
}

internal static class J
{
    public static FormatException Error(string id, string message) => new(id + ": " + message);
    public static JsonDocument Parse(string text) => JsonDocument.Parse(text);

    public static JsonElement Root(JsonDocument document, string format, params string[] keys)
    {
        JsonElement root = document.RootElement;
        Keys(root, "document", new[] { "format", "version" }.Concat(keys).ToArray());
        if (Text(root, "format") != format) throw Error("format", "expected " + format);
        if (!root.GetProperty("version").TryGetInt32(out int version) || version != 1) throw Error("version", "only version 1 is supported");
        return root;
    }

    public static void Keys(JsonElement value, string id, params string[] names)
    {
        if (value.ValueKind != JsonValueKind.Object) throw Error(id, "expected an object");
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonProperty p in value.EnumerateObject())
            if (!names.Contains(p.Name) || !seen.Add(p.Name)) throw Error(p.Name, "unknown or repeated key in " + id);
        foreach (string name in names) if (!seen.Contains(name)) throw Error(name, "missing in " + id);
    }

    public static List<JsonProperty> Map(JsonElement value, string id)
    {
        if (value.ValueKind != JsonValueKind.Object) throw Error(id, "expected an object");
        var seen = new HashSet<string>(StringComparer.Ordinal);
        var result = new List<JsonProperty>();
        foreach (JsonProperty p in value.EnumerateObject())
        {
            if (string.IsNullOrWhiteSpace(p.Name) || !seen.Add(p.Name)) throw Error(id, "empty or repeated key");
            result.Add(p);
        }
        return result;
    }

    public static string Text(JsonElement parent, string id) => String(parent.GetProperty(id), id);
    public static string String(JsonElement value, string id) =>
        value.ValueKind == JsonValueKind.String && value.GetString() is { Length: > 0 } s ? s : throw Error(id, "expected nonempty text");
    public static double Number(JsonElement value, string id) =>
        value.ValueKind == JsonValueKind.Number && value.TryGetDouble(out double v) && double.IsFinite(v) ? v : throw Error(id, "expected a finite number");
    public static double[] Numbers(JsonElement value, string id, int count)
    {
        if (value.ValueKind != JsonValueKind.Array || value.GetArrayLength() != count) throw Error(id, $"expected {count} numbers");
        return value.EnumerateArray().Select(v => Number(v, id)).ToArray();
    }
    public static string[] Strings(JsonElement value, string id)
    {
        if (value.ValueKind != JsonValueKind.Array || value.GetArrayLength() == 0) throw Error(id, "expected a nonempty list of text");
        string[] result = value.EnumerateArray().Select(v => String(v, id)).ToArray();
        return result.Distinct().Count() == result.Length ? result : throw Error(id, "repeated entry");
    }
}

// A semantic colour role shared by every theme. A state or encoding role also has a cue that is not colour
// (shape, label or position); a theme or an override may change the colour and never the cue (ADR-D01.4 item 7).
public sealed record ColourRole(string Id, string Group, string Meaning, string? Cue);
// Files: the face's font files by weight, relative to a folder of installed packages; the first file of a weight
// is the face and the others are its further Unicode subsets.
public sealed record TypeRole(string Id, IReadOnlyList<string> Faces, IReadOnlyDictionary<string, double> Ramp, bool Tabular,
    IReadOnlyDictionary<string, IReadOnlyList<string>> Files);
public sealed record ContrastRule(string Text, string Surface, double Minimum);

// The part of the design system that every theme shares: density table, type roles, motion, roles and component tokens.
public sealed class DesignSystem
{
    public static readonly IReadOnlyList<string> RoleGroups = Array.AsReadOnly(new[] { "chrome", "state", "encoding", "syntax" });
    public double Unit { get; }
    public IReadOnlyList<string> Densities { get; }
    public string ShippedDensity { get; }
    public IReadOnlyDictionary<string, double[]> DensityTable { get; }
    public IReadOnlyDictionary<string, TypeRole> Type { get; }
    public IReadOnlyDictionary<string, double> Durations { get; }
    public IReadOnlyDictionary<string, double[]> Curves { get; }
    public IReadOnlyDictionary<string, ColourRole> Roles { get; }
    public IReadOnlyDictionary<string, string> Components { get; }
    public IReadOnlyList<ContrastRule> Contrast { get; }

    public static DesignSystem Load(string path) => Parse(File.ReadAllText(path));
    public static DesignSystem Parse(string text)
    {
        using JsonDocument d = J.Parse(text);
        return new DesignSystem(J.Root(d, "magic600-design-system", "density", "type", "motion", "roles", "components", "contrast"));
    }

    private DesignSystem(JsonElement root)
    {
        JsonElement density = root.GetProperty("density");
        J.Keys(density, "density", "unit", "presets", "shipped", "tokens");
        Unit = J.Number(density.GetProperty("unit"), "unit");
        if (Unit <= 0) throw J.Error("unit", "must be positive");
        Densities = Array.AsReadOnly(J.Strings(density.GetProperty("presets"), "presets"));
        ShippedDensity = J.Text(density, "shipped");
        if (!Densities.Contains(ShippedDensity)) throw J.Error("shipped", "not a density preset");
        var table = new SortedDictionary<string, double[]>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(density.GetProperty("tokens"), "tokens"))
        {
            double[] row = J.Numbers(p.Value, p.Name, Densities.Count);
            if (row.Any(v => v < 0) || row.Zip(row.Skip(1), (a, b) => b < a).Any(x => x)) throw J.Error(p.Name, "multiples must be nonnegative and never shrink with a roomier preset");
            table.Add(p.Name, row);
        }
        DensityTable = table;
        var type = new SortedDictionary<string, TypeRole>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(root.GetProperty("type"), "type"))
        {
            J.Keys(p.Value, p.Name, "faces", "files", "ramp", "tabular");
            var files = new SortedDictionary<string, IReadOnlyList<string>>(StringComparer.Ordinal);
            foreach (JsonProperty weight in J.Map(p.Value.GetProperty("files"), p.Name))
            {
                string[] names = J.Strings(weight.Value, p.Name);
                if (names.Length == 0 || names.Any(n => n.Contains("..", StringComparison.Ordinal) || n.Contains('\\') || n.StartsWith('/') || n.Contains(':') || !n.EndsWith(".woff2", StringComparison.Ordinal)))
                    throw J.Error(p.Name + ".files", "relative .woff2 paths with forward slashes required");
                files.Add(weight.Name, Array.AsReadOnly(names));
            }
            if (!files.ContainsKey("400") || files.Keys.Any(k => k != "400" && k != "600")) throw J.Error(p.Name + ".files", "weight 400 required, weight 600 optional");
            var ramp = new SortedDictionary<string, double>(StringComparer.Ordinal);
            foreach (JsonProperty step in J.Map(p.Value.GetProperty("ramp"), p.Name))
            {
                double size = J.Number(step.Value, step.Name);
                if (size < 6 || size > 72) throw J.Error(p.Name + "." + step.Name, "size outside 6..72");
                ramp.Add(step.Name, size);
            }
            if (!ramp.ContainsKey("body")) throw J.Error(p.Name, "a ramp with a body step required");
            JsonElement tabular = p.Value.GetProperty("tabular");
            if (tabular.ValueKind is not (JsonValueKind.True or JsonValueKind.False)) throw J.Error(p.Name, "tabular must be true or false");
            type.Add(p.Name, new TypeRole(p.Name, Array.AsReadOnly(J.Strings(p.Value.GetProperty("faces"), p.Name)), ramp, tabular.GetBoolean(), files));
        }
        if (!type.Keys.SequenceEqual(new[] { "code", "content", "ui" })) throw J.Error("type", "exactly the roles ui, content and code required");
        Type = type;
        JsonElement motion = root.GetProperty("motion");
        J.Keys(motion, "motion", "durations", "curves");
        var durations = new SortedDictionary<string, double>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(motion.GetProperty("durations"), "durations"))
        {
            double ms = J.Number(p.Value, p.Name);
            if (ms < 0 || ms > 2000) throw J.Error(p.Name, "duration outside 0..2000 ms");
            durations.Add(p.Name, ms);
        }
        Durations = durations;
        var curves = new SortedDictionary<string, double[]>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(motion.GetProperty("curves"), "curves"))
        {
            double[] c = J.Numbers(p.Value, p.Name, 4);
            if (c[0] < 0 || c[0] > 1 || c[2] < 0 || c[2] > 1) throw J.Error(p.Name, "bezier x values outside 0..1");
            curves.Add(p.Name, c);
        }
        Curves = curves;
        var roles = new SortedDictionary<string, ColourRole>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(root.GetProperty("roles"), "roles"))
        {
            bool cued = p.Value.ValueKind == JsonValueKind.Object && p.Value.TryGetProperty("cue", out _);
            J.Keys(p.Value, p.Name, cued ? new[] { "group", "meaning", "cue" } : new[] { "group", "meaning" });
            string group = J.Text(p.Value, "group");
            if (!RoleGroups.Contains(group)) throw J.Error(p.Name, "unknown role group");
            if ((group is "state" or "encoding") != cued) throw J.Error(p.Name, "a state or encoding role, and only such a role, needs a cue that is not colour");
            roles.Add(p.Name, new ColourRole(p.Name, group, J.Text(p.Value, "meaning"), cued ? J.Text(p.Value, "cue") : null));
        }
        Roles = roles;
        var components = new SortedDictionary<string, string>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(root.GetProperty("components"), "components"))
        {
            string role = J.String(p.Value, p.Name);
            if (!roles.ContainsKey(role)) throw J.Error(p.Name, "component token names an unknown role");
            components.Add(p.Name, role);
        }
        Components = components;
        var contrast = new List<ContrastRule>();
        JsonElement rules = root.GetProperty("contrast");
        if (rules.ValueKind != JsonValueKind.Array) throw J.Error("contrast", "expected a list");
        foreach (JsonElement rule in rules.EnumerateArray())
        {
            J.Keys(rule, "contrast", "text", "surface", "minimum");
            var item = new ContrastRule(J.Text(rule, "text"), J.Text(rule, "surface"), J.Number(rule.GetProperty("minimum"), "minimum"));
            if (!roles.ContainsKey(item.Text) || !roles.ContainsKey(item.Surface) || item.Minimum < 1 || item.Minimum > 21) throw J.Error("contrast", "unknown role or impossible minimum");
            contrast.Add(item);
        }
        Contrast = contrast.AsReadOnly();
    }

    // One density table from one unit times the display scale (ADR-D01.45). A nonzero size never falls below one pixel.
    public IReadOnlyDictionary<string, double> Density(string preset, double displayScale)
    {
        int column = Densities.ToList().IndexOf(preset);
        if (column < 0) throw J.Error(preset, "unknown density preset");
        if (!double.IsFinite(displayScale) || displayScale < .5 || displayScale > 4) throw J.Error("displayScale", "outside 0.5..4");
        var result = new SortedDictionary<string, double>(StringComparer.Ordinal);
        foreach (var token in DensityTable)
            result.Add(token.Key, token.Value[column] == 0 ? 0 : Math.Max(1, Math.Round(token.Value[column] * Unit * displayScale)));
        return result;
    }
}

public sealed record Knob(string Id, double Min, double Max, double Default);
public sealed record RoleSource(string? Ramp, int Step, string? MixFrom, string? MixTo, string? MixKnob, double MixScale);

// One theme family: OkLCH seeds, the lightness ladder of its ramps, ranged knobs that move only the seeds, the
// assignment of every shared role to a ramp step, and three scene looks (ADR-D01.49, ADR-D01.50).
public sealed class ThemeFamily
{
    public const int Steps = 12;
    public static readonly IReadOnlyList<string> KnobIds = Array.AsReadOnly(new[] { "tone", "contrast", "blackLevel", "accentHue", "gridIntensity" });
    public static readonly IReadOnlyList<string> SceneLooks = Array.AsReadOnly(new[] { "solving", "inspecting", "celebrating" });
    public string Id { get; }
    public string Name { get; }
    public string Scheme { get; }
    public IReadOnlyList<double> Ladder { get; }
    public IReadOnlyDictionary<string, (double Hue, double Chroma)> Ramps { get; }
    public IReadOnlyDictionary<string, Knob> Knobs { get; }
    public IReadOnlyDictionary<string, RoleSource> Roles { get; }
    public IReadOnlyDictionary<string, IReadOnlyDictionary<string, JsonElement>> Scenes { get; }

    public static ThemeFamily Load(string path, DesignSystem system) => Parse(File.ReadAllText(path), system);
    public static ThemeFamily Parse(string text, DesignSystem system)
    {
        using JsonDocument d = J.Parse(text);
        return new ThemeFamily(J.Root(d, "magic600-design-theme", "id", "name", "scheme", "ladder", "ramps", "knobs", "roles", "sceneLooks"), system);
    }

    private ThemeFamily(JsonElement root, DesignSystem system)
    {
        Id = J.Text(root, "id"); Name = J.Text(root, "name"); Scheme = J.Text(root, "scheme");
        if (Scheme != "dark" && Scheme != "light") throw J.Error("scheme", "expected dark or light");
        double[] ladder = J.Numbers(root.GetProperty("ladder"), "ladder", Steps);
        bool rising = ladder[^1] > ladder[0];
        if (ladder.Any(v => v < 0 || v > 1) || ladder.Zip(ladder.Skip(1), (a, b) => rising ? b > a : b < a).Any(ok => !ok) || rising != (Scheme == "dark"))
            throw J.Error("ladder", "twelve lightness values in 0..1, rising for a dark scheme and falling for a light one");
        Ladder = Array.AsReadOnly(ladder);
        var ramps = new SortedDictionary<string, (double, double)>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(root.GetProperty("ramps"), "ramps"))
        {
            J.Keys(p.Value, p.Name, "hue", "chroma");
            double hue = J.Number(p.Value.GetProperty("hue"), p.Name), chroma = J.Number(p.Value.GetProperty("chroma"), p.Name);
            if (hue < 0 || hue > 360 || chroma < 0 || chroma > .4) throw J.Error(p.Name, "hue outside 0..360 or chroma outside 0..0.4");
            ramps.Add(p.Name, (hue, chroma));
        }
        if (!ramps.ContainsKey("neutral") || !ramps.ContainsKey("accent")) throw J.Error("ramps", "neutral and accent ramps required");
        Ramps = ramps;
        var knobs = new SortedDictionary<string, Knob>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(root.GetProperty("knobs"), "knobs"))
        {
            J.Keys(p.Value, p.Name, "min", "max", "value");
            var knob = new Knob(p.Name, J.Number(p.Value.GetProperty("min"), p.Name), J.Number(p.Value.GetProperty("max"), p.Name), J.Number(p.Value.GetProperty("value"), p.Name));
            if (knob.Min >= knob.Max || knob.Default < knob.Min || knob.Default > knob.Max) throw J.Error(p.Name, "default outside its range");
            knobs.Add(p.Name, knob);
        }
        if (!knobs.Keys.SequenceEqual(KnobIds.OrderBy(k => k, StringComparer.Ordinal))) throw J.Error("knobs", "exactly tone, contrast, blackLevel, accentHue and gridIntensity required");
        Knobs = knobs;
        var roles = new SortedDictionary<string, RoleSource>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(root.GetProperty("roles"), "roles"))
        {
            if (!system.Roles.ContainsKey(p.Name)) throw J.Error(p.Name, "not a shared role");
            if (p.Value.ValueKind == JsonValueKind.String)
            {
                string[] parts = p.Value.GetString()!.Split('.');
                if (parts.Length != 2 || !ramps.ContainsKey(parts[0]) || !int.TryParse(parts[1], NumberStyles.None, CultureInfo.InvariantCulture, out int step) || step >= Steps)
                    throw J.Error(p.Name, "expected <ramp>.<step 0..11>");
                roles.Add(p.Name, new RoleSource(parts[0], step, null, null, null, 0));
            }
            else
            {
                J.Keys(p.Value, p.Name, "mix", "knob", "scale");
                string[] mix = J.Strings(p.Value.GetProperty("mix"), p.Name);
                string knob = J.Text(p.Value, "knob");
                double scale = J.Number(p.Value.GetProperty("scale"), p.Name);
                if (mix.Length != 2 || !knobs.ContainsKey(knob) || scale <= 0 || scale > 1) throw J.Error(p.Name, "mix needs two roles, a knob and a scale in 0..1");
                roles.Add(p.Name, new RoleSource(null, 0, mix[0], mix[1], knob, scale));
            }
        }
        foreach (string role in system.Roles.Keys) if (!roles.ContainsKey(role)) throw J.Error(role, "role not assigned by theme " + Id);
        foreach (var mix in roles.Where(r => r.Value.MixFrom != null))
            if (!roles.TryGetValue(mix.Value.MixFrom!, out var a) || !roles.TryGetValue(mix.Value.MixTo!, out var b) || a.Ramp == null || b.Ramp == null)
                throw J.Error(mix.Key, "a mix must name two ramp roles");
        Roles = roles;
        var scenes = new SortedDictionary<string, IReadOnlyDictionary<string, JsonElement>>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(root.GetProperty("sceneLooks"), "sceneLooks"))
        {
            var values = new SortedDictionary<string, JsonElement>(StringComparer.Ordinal);
            foreach (JsonProperty v in J.Map(p.Value, p.Name)) values.Add(v.Name, v.Value.Clone());
            scenes.Add(p.Name, values);
        }
        if (!scenes.Keys.SequenceEqual(SceneLooks.OrderBy(k => k, StringComparer.Ordinal))) throw J.Error("sceneLooks", "exactly solving, inspecting and celebrating required");
        Scenes = scenes;
    }

    public Oklch Step(string ramp, int step, IReadOnlyDictionary<string, double> knobs)
    {
        (double hue, double chroma) = Ramps[ramp];
        if (ramp == "accent") hue = knobs["accentHue"];
        // Tone moves the whole ladder, contrast spreads it around its middle, black level lifts its dark end.
        double l = Math.Clamp(.5 + (Ladder[step] - .5) * knobs["contrast"] + knobs["tone"], 0, 1);
        return new Oklch(knobs["blackLevel"] + (1 - knobs["blackLevel"]) * l, chroma, hue);
    }
}

public sealed record ResolvedColour(Rgb Value, bool Overridden);

// One resolved token set: what the Godot theme and the scene constants are built from (ADR-D01.5).
public sealed class TokenSet
{
    public DesignSystem System { get; }
    public ThemeFamily Theme { get; }
    public string Density { get; }
    public double DisplayScale { get; }
    public IReadOnlyDictionary<string, ResolvedColour> Colours { get; }
    public IReadOnlyDictionary<string, double> Sizes { get; }
    public IReadOnlyDictionary<string, double> Knobs { get; }
    // True when a state or encoding role differs from its shipped colour: the view frame then shows the
    // "non-standard colours" mark (ADR-D01.51).
    public bool NonStandardColours { get; }
    public bool HighContrast { get; }

    public TokenSet(DesignSystem system, ThemeFamily theme, string density, double displayScale,
        IReadOnlyDictionary<string, double>? knobs = null, IReadOnlyDictionary<string, Oklch>? overrides = null, bool highContrast = false)
    {
        HighContrast = highContrast; System = system; Theme = theme; Density = density; DisplayScale = displayScale;
        Sizes = system.Density(density, displayScale);
        var values = new SortedDictionary<string, double>(StringComparer.Ordinal);
        foreach (Knob knob in theme.Knobs.Values)
        {
            double value = knobs != null && knobs.TryGetValue(knob.Id, out double v) ? v : knob.Default;
            if (!double.IsFinite(value) || value < knob.Min || value > knob.Max)
                throw J.Error(knob.Id, $"outside {knob.Min.ToString(CultureInfo.InvariantCulture)}..{knob.Max.ToString(CultureInfo.InvariantCulture)}");
            values.Add(knob.Id, value);
        }
        if (knobs != null) foreach (string id in knobs.Keys) if (!theme.Knobs.ContainsKey(id)) throw J.Error(id, "unknown knob");
        Knobs = values;
        var colours = new SortedDictionary<string, ResolvedColour>(StringComparer.Ordinal);
        foreach (var role in theme.Roles.Where(r => r.Value.Ramp != null))
            colours.Add(role.Key, new ResolvedColour(theme.Step(role.Value.Ramp!, role.Value.Step, values).ToSrgb(), false));
        foreach (var role in theme.Roles.Where(r => r.Value.Ramp == null))
        {
            Rgb a = colours[role.Value.MixFrom!].Value, b = colours[role.Value.MixTo!].Value;
            double t = Math.Clamp(values[role.Value.MixKnob!] * role.Value.MixScale, 0, 1);
            colours.Add(role.Key, new ResolvedColour(new Rgb(a.R + (b.R - a.R) * t, a.G + (b.G - a.G) * t, a.B + (b.B - a.B) * t), false));
        }
        if (highContrast)
        {
            // High contrast (ADR-D01.35): text and lines at the end of the scale on a ground at the other end;
            // each weaker text step takes the next stronger one. State and encoding colours are not changed.
            bool dark = theme.Scheme == "dark";
            Rgb end = dark ? new Rgb(1, 1, 1) : new Rgb(0, 0, 0), ground = dark ? new Rgb(0, 0, 0) : new Rgb(1, 1, 1);
            Rgb primary = colours["text.primary"].Value, secondary = colours["text.secondary"].Value;
            foreach ((string role, Rgb value) in new[] { ("text.primary", end), ("text.secondary", primary), ("text.disabled", secondary),
                ("border.subtle", secondary), ("border.strong", end), ("surface.window", ground), ("surface.sunken", ground) })
                colours[role] = new ResolvedColour(value, false);
        }
        if (overrides != null) foreach (var entry in overrides)
        {
            if (!system.Roles.TryGetValue(entry.Key, out ColourRole? role)) throw J.Error(entry.Key, "override names an unknown role");
            Oklch c = entry.Value;
            if (!(c.L >= 0 && c.L <= 1 && c.C >= 0 && c.C <= .4 && c.H >= 0 && c.H <= 360)) throw J.Error(entry.Key, "override colour outside OkLCH ranges");
            Rgb value = c.ToSrgb();
            if (value != colours[entry.Key].Value && role.Group is "state" or "encoding") NonStandardColours = true;
            colours[entry.Key] = new ResolvedColour(value, true);
        }
        Colours = colours;
    }

    public Rgb Colour(string token) => Colours[System.Components.TryGetValue(token, out string? role) ? role : token].Value;
    public double Size(string token) => Sizes.TryGetValue(token, out double v) ? v : throw J.Error(token, "unknown size token");
    public double TypeSize(string role, string step) => Math.Round(System.Type[role].Ramp[step] * DisplayScale);
    public double Duration(string token) => System.Durations[token];

    // The contrast pairs that fall below their minimum; an override file is saved only when this is empty (ADR-D01.50).
    public IReadOnlyList<string> ContrastIssues() => System.Contrast
        .Select(rule => (rule, ratio: Oklch.Contrast(Colours[rule.Text].Value, Colours[rule.Surface].Value)))
        .Where(x => x.ratio < x.rule.Minimum)
        .Select(x => $"{x.rule.Text} on {x.rule.Surface}: {x.ratio.ToString("0.00", CultureInfo.InvariantCulture)} < {x.rule.Minimum.ToString(CultureInfo.InvariantCulture)}").ToArray();
}

// A per-theme difference file of semantic colour overrides (ADR-D01.50, ADR-D01.51).
public static class TokenOverrides
{
    public static IReadOnlyDictionary<string, Oklch> Parse(string text, ThemeFamily theme)
    {
        using JsonDocument d = J.Parse(text);
        JsonElement root = J.Root(d, "magic600-design-overrides", "theme", "roles");
        if (J.Text(root, "theme") != theme.Id) throw J.Error("theme", "overrides belong to another theme");
        var result = new SortedDictionary<string, Oklch>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(root.GetProperty("roles"), "roles"))
        {
            J.Keys(p.Value, p.Name, "L", "C", "h");
            result.Add(p.Name, new Oklch(J.Number(p.Value.GetProperty("L"), p.Name), J.Number(p.Value.GetProperty("C"), p.Name), J.Number(p.Value.GetProperty("h"), p.Name)));
        }
        return result;
    }
}
