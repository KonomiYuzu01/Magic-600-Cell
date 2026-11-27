using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Magic600.Chrome;

// The motion table (ADR-D01.55, ADR-D01.56): every motion of the frame with its meaning, a duration token, a
// curve token and the static picture that says the same without moving. A sound cue is a slot only: an id with
// its visual equivalent; no audio is played. No entry differs with whether a check result is good or bad.
// Version 2 (ADR-D02.37) adds the Jumble rows and a reserved row that is off: it is listed, never played.
public sealed record MotionEntry(string Id, string Meaning, string Duration, string Curve, string Static, string? Cue, bool Off = false);

public sealed class MotionTable
{
    public int Version { get; }
    public IReadOnlyList<MotionEntry> Entries { get; }
    public MotionEntry this[string id] => Entries.First(e => e.Id == id);

    private MotionTable(int version, IReadOnlyList<MotionEntry> entries) { Version = version; Entries = entries; }

    public static MotionTable Parse(string text, DesignSystem system)
    {
        using JsonDocument d = J.Parse(text); JsonElement root = d.RootElement;
        J.Keys(root, "document", "format", "version", "motions");
        if (J.Text(root, "format") != "magic600-design-motion") throw J.Error("format", "expected magic600-design-motion");
        if (!root.GetProperty("version").TryGetInt32(out int version) || version is not (1 or 2)) throw J.Error("version", "only versions 1 and 2 are supported");
        var entries = new List<MotionEntry>();
        foreach (JsonElement m in root.GetProperty("motions").EnumerateArray())
        {
            JsonElement o = default;
            bool off = m.ValueKind == JsonValueKind.Object && m.TryGetProperty("off", out o);
            if (off && (version < 2 || o.ValueKind != JsonValueKind.True)) throw J.Error("off", "a reserved row is a version 2 key and only true");
            J.Keys(m, "motion", off ? new[] { "id", "meaning", "duration", "curve", "static", "cue", "off" } : new[] { "id", "meaning", "duration", "curve", "static", "cue" });
            string id = J.Text(m, "id");
            var entry = new MotionEntry(id, J.Text(m, "meaning"), J.Text(m, "duration"), J.Text(m, "curve"), J.Text(m, "static"),
                m.GetProperty("cue").ValueKind == JsonValueKind.Null ? null : J.Text(m, "cue"), off);
            if (entries.Any(e => e.Id == id)) throw J.Error(id, "repeated motion");
            if (!system.Durations.ContainsKey(entry.Duration)) throw J.Error(id, "unknown duration token " + entry.Duration);
            if (!system.Curves.ContainsKey(entry.Curve)) throw J.Error(id, "unknown curve token " + entry.Curve);
            if (entry.Meaning.Length == 0 || entry.Static.Length == 0) throw J.Error(id, "a motion needs a meaning and a static equivalent");
            if (Regex.IsMatch(id + " " + (entry.Cue ?? ""), @"\b(good|bad|pass|fail|success|error|win|lose)\b")) throw J.Error(id, "a motion or cue never tells a good result from a bad one");
            entries.Add(entry);
        }
        if (entries.Count == 0) throw J.Error("motions", "empty");
        return new MotionTable(version, entries);
    }

    // Seconds a motion takes: none under reduced motion, where its static equivalent shows at once, and none for
    // a reserved row that is off.
    public double Seconds(string id, DesignSystem system, bool reduced) => reduced || this[id].Off ? 0 : system.Durations[this[id].Duration] / 1000;

    // The eased share of the way at a share of the time, for a curve token (a cubic Bezier through 0,0 and 1,1).
    public static double Ease(double[] curve, double time)
    {
        time = Math.Clamp(time, 0, 1);
        double x1 = curve[0], y1 = curve[1], x2 = curve[2], y2 = curve[3], lo = 0, hi = 1, u = time;
        double B(double a, double b, double t) => 3 * a * t * (1 - t) * (1 - t) + 3 * b * t * t * (1 - t) + t * t * t;
        for (int i = 0; i < 24; i++) { u = (lo + hi) / 2; if (B(x1, x2, u) < time) lo = u; else hi = u; }
        return B(y1, y2, u);
    }
}

// What the solver set about motion and cues (stored with the appearance).
public sealed class MotionSettings
{
    public bool Reduced { get; set; }
    public bool Muted { get; set; } = true;
    public bool Rewards { get; set; } = true;
    public static bool IsReward(string id) => id is MotionUses.RewardMark or MotionUses.OrbitComplete;
    public static bool Shows(string id, bool rewards) => rewards || !IsReward(id);
    public string Store() => $"{(Reduced ? "reduced" : "full")},{(Muted ? "muted" : "cues")},{(Rewards ? "rewards" : "plain")}";
    public void Restore(string stored)
    {
        string[] parts = stored.Split(',');
        if (parts.Length != 3) return;
        Reduced = parts[0] == "reduced"; Muted = parts[1] != "cues"; Rewards = parts[2] != "plain";
    }
}

// OKLab distances and colour-vision simulation for the sticker palette (ADR-D01.52).
public static class ColourMetric
{
    public static readonly IReadOnlyList<string> Visions = Array.AsReadOnly(new[] { "normal", "protan", "deutan", "tritan" });
    // Machado, Oliveira and Fernandes (2009), severity 1, on linear RGB.
    private static readonly Dictionary<string, double[]> Matrices = new()
    {
        ["protan"] = new[] { .152286, 1.052583, -.204868, .114503, .786281, .099216, -.003882, -.048116, 1.051998 },
        ["deutan"] = new[] { .367322, .860646, -.227968, .280085, .672501, .047413, -.011820, .042940, .968881 },
        ["tritan"] = new[] { 1.255528, -.076749, -.178779, -.078411, .930809, .147602, .004733, .691367, .303900 },
    };

    private static double Decode(double v) => v <= .04045 ? v / 12.92 : Math.Pow((v + .055) / 1.055, 2.4);
    private static double Encode(double v) => v <= .0031308 ? v * 12.92 : 1.055 * Math.Pow(v, 1 / 2.4) - .055;

    public static Rgb Simulate(Rgb c, string vision) => vision == "normal" ? c : Apply(c, Matrices[vision]);

    // ADR-D02.47's held simulations: G4's three colour visions and a contrast view that keeps lightness only (the
    // contrast rules judge luminance, so a pair they pass stays apart here). Each is one row-major 3x3 matrix on linear
    // RGB, the same matrix the core's view style takes (B14) and the chrome's filter draws.
    public static readonly IReadOnlyList<string> Simulations = Array.AsReadOnly(new[] { "protan", "deutan", "tritan", "contrast" });
    private static readonly double[] Lightness = { .2126, .7152, .0722, .2126, .7152, .0722, .2126, .7152, .0722 };
    public static double[] SimulationMatrix(string? simulation) => simulation switch
    {
        null or "normal" => new double[] { 1, 0, 0, 0, 1, 0, 0, 0, 1 },
        "contrast" => (double[])Lightness.Clone(),
        _ when Matrices.TryGetValue(simulation, out double[]? m) => (double[])m.Clone(),
        _ => throw new FormatException(simulation + ": the simulations are " + string.Join(", ", Simulations)),
    };

    public static Rgb Apply(Rgb c, IReadOnlyList<double> m)
    {
        double r = Decode(c.R), g = Decode(c.G), b = Decode(c.B);
        double F(int row) => Encode(Math.Clamp(m[row * 3] * r + m[row * 3 + 1] * g + m[row * 3 + 2] * b, 0, 1));
        return new Rgb(F(0), F(1), F(2));
    }

    public static (double L, double A, double B) Oklab(Rgb c)
    {
        double r = Decode(c.R), g = Decode(c.G), b = Decode(c.B);
        double l = Math.Cbrt(.4122214708 * r + .5363325363 * g + .0514459929 * b), m = Math.Cbrt(.2119034982 * r + .6806995451 * g + .1073969566 * b),
            s = Math.Cbrt(.0883024619 * r + .2817188376 * g + .6299787005 * b);
        return (.2104542553 * l + .7936177850 * m - .0040720468 * s, 1.9779984951 * l - 2.4285922050 * m + .4505937099 * s, .0259040371 * l + .7827717662 * m - .8086757660 * s);
    }

    public static double Distance(Rgb a, Rgb b)
    {
        var x = Oklab(a); var y = Oklab(b);
        return Math.Sqrt((x.L - y.L) * (x.L - y.L) + (x.A - y.A) * (x.A - y.A) + (x.B - y.B) * (x.B - y.B));
    }
}

// One candidate sticker palette of a family: its colour classes, and for each kind of colour vision the smallest
// distance between two classes that can sit on adjacent cells (any two classes can).
public sealed record StickerPalette(string Id, string Name, string Family, IReadOnlyList<Rgb> Colours, IReadOnlyDictionary<string, double> Lab)
{
    public const double Needed = .09, NeededSimulated = .045;
    public double Smallest(string vision)
    {
        double least = double.MaxValue;
        for (int i = 0; i < Colours.Count; i++)
            for (int j = i + 1; j < Colours.Count; j++)
                least = Math.Min(least, ColourMetric.Distance(ColourMetric.Simulate(Colours[i], vision), ColourMetric.Simulate(Colours[j], vision)));
        return least;
    }
    public bool Passes => ColourMetric.Visions.All(v => Smallest(v) >= (v == "normal" ? Needed : NeededSimulated));

    public static IReadOnlyList<StickerPalette> Parse(string text)
    {
        using JsonDocument d = J.Parse(text); JsonElement root = J.Root(d, "magic600-design-palettes", "palettes");
        var found = new List<StickerPalette>();
        foreach (JsonElement p in root.GetProperty("palettes").EnumerateArray())
        {
            J.Keys(p, "palette", "id", "name", "family", "colours", "lab");
            var colours = new List<Rgb>();
            foreach (JsonElement c in p.GetProperty("colours").EnumerateArray())
            {
                double[] lch = J.Numbers(c, "colour", 3);
                if (!(lch[0] >= 0 && lch[0] <= 1 && lch[1] >= 0 && lch[1] <= .4 && lch[2] >= 0 && lch[2] <= 360)) throw J.Error(J.Text(p, "id"), "colour outside OkLCH ranges");
                colours.Add(new Oklch(lch[0], lch[1], lch[2]).ToSrgb());
            }
            if (colours.Count < 4) throw J.Error(J.Text(p, "id"), "a palette needs at least four classes");
            // The same palette as the parameters the core draws stickers from.
            var lab = new SortedDictionary<string, double>(StringComparer.Ordinal);
            foreach (JsonProperty value in J.Map(p.GetProperty("lab"), "lab")) lab.Add(value.Name, J.Number(value.Value, value.Name));
            found.Add(new StickerPalette(J.Text(p, "id"), J.Text(p, "name"), J.Text(p, "family"), colours, lab));
        }
        return found;
    }

    // Only passing palettes are offered.
    public static IReadOnlyList<StickerPalette> Offered(IEnumerable<StickerPalette> all, string family) => all.Where(p => p.Family == family && p.Passes).ToArray();
}

// Focus layers (ADR-D01.46): one pick layer at a time, one key that cycles how the inactive parts show, an
// opacity per element class. A dimmed or hidden element cannot be picked; a protection conflict and the effect
// of a complete operation break through every layer (ADR-D01.4 item 8).
public enum InactiveMode { Normal, Dimmed, Hidden }

// A predicate focus layer: its id, its label, its count, and whether it applies here.
public sealed record PredicateLayer(string Id, string Label, Numeral Count, bool Applies);

public sealed class FocusLayers
{
    public static readonly IReadOnlyList<(string Id, string Key, string Icon, string Name)> Layers = Array.AsReadOnly(new[]
    {
        ("cells", "1", "layer.cells", "Cells"), ("stickers", "2", "layer.stickers", "Stickers"), ("grips", "3", "layer.grips", "Grips"), ("axes", "4", "layer.axes", "Axes"), ("orbit", "5", "layer.orbit", "Orbit"),
    });
    public string Pick { get; private set; } = "cells";
    public InactiveMode Inactive { get; private set; } = InactiveMode.Normal;
    public string? Orbit { get; private set; }
    public event Action? Changed;
    private readonly SortedDictionary<string, double> opacity = new(StringComparer.Ordinal);

    public bool SetPick(string layer) { if (Layers.All(l => l.Id != layer) || Pick == layer) return false; Pick = layer; Safe.Raise(Changed); return true; }
    public InactiveMode Cycle() { Inactive = (InactiveMode)(((int)Inactive + 1) % 3); Safe.Raise(Changed); return Inactive; }
    public void Highlight(string? orbit) { Orbit = orbit; Safe.Raise(Changed); }
    public double ClassOpacity(string layer) => opacity.TryGetValue(layer, out double v) ? v : 1;
    public bool SetOpacity(string layer, double value)
    {
        if (Layers.All(l => l.Id != layer) || !(value >= 0 && value <= 1)) return false;
        opacity[layer] = value; Safe.Raise(Changed); return true;
    }

    // Predicate focus layers (ADR-D01.46 as revision 4 reads it; ADR-D02's B11): a predicate with its count beside it,
    // on the same cycling key. One predicate is focused at a time, and none by default; while one is focused and the
    // cycle is not Normal, its members show and the rest take the cycle's mode. A predicate that does not apply here
    // (the straddlers at an unblocked cap) dims nothing.
    private readonly List<PredicateLayer> predicates = new();
    public IReadOnlyList<PredicateLayer> Predicates => predicates;
    public string? Focused { get; private set; }

    // Adds a predicate, or updates its label and count in place; the focus stays where it was.
    public void AddPredicate(string id, string label, Numeral count, bool applies = true)
    {
        if (string.IsNullOrEmpty(id) || Layers.Any(l => l.Id == id)) throw new ArgumentException("a predicate layer needs its own id: " + id);
        var layer = new PredicateLayer(id, label, count, applies);
        int at = predicates.FindIndex(p => p.Id == id);
        if (at >= 0 && predicates[at] == layer) return;
        if (at >= 0) predicates[at] = layer; else predicates.Add(layer);
        Safe.Raise(Changed);
    }

    public void AddPredicate(PredicateLayer layer) => AddPredicate(layer.Id, layer.Label, layer.Count, layer.Applies);

    // Takes a predicate away, and its focus with it.
    public bool RemovePredicate(string id)
    {
        if (predicates.RemoveAll(p => p.Id == id) == 0) return false;
        if (Focused == id) Focused = null;
        Safe.Raise(Changed); return true;
    }

    // Focuses a predicate, or none; the focused predicate again un-focuses it.
    public bool Focus(string? id)
    {
        if (id != null && predicates.All(p => p.Id != id)) return false;
        Focused = Focused == id ? null : id; Safe.Raise(Changed); return true;
    }

    // Whether the focused predicate dims or hides its non-members now.
    public bool Dimming => Inactive != InactiveMode.Normal && Focused is string f && predicates.First(p => p.Id == f).Applies;

    // How strongly a piece shows under the focused predicate: a member or a breakthrough fully, the rest by the mode.
    public double ShownPiece(bool member, bool breaksThrough = false) =>
        breaksThrough || member || !Dimming ? 1 : Inactive == InactiveMode.Dimmed ? .25 : 0;

    // A predicate's key text: its label and its count.
    public static string KeyText(PredicateLayer layer) => layer.Label + " · " + (layer.Count.Kind == NumeralKind.Value ? layer.Count.Value.ToString("N0", CultureInfo.InvariantCulture) : layer.Count.Word);

    // How strongly an element shows: the active layer fully, the others by the inactive mode, a breakthrough always.
    public double Shown(string layer, bool breaksThrough = false) =>
        breaksThrough ? 1 : layer == Pick ? ClassOpacity(layer) : Inactive switch { InactiveMode.Normal => ClassOpacity(layer), InactiveMode.Dimmed => .25 * ClassOpacity(layer), _ => 0 };
    public bool CanPick(string layer) => layer == Pick && ClassOpacity(layer) > .25;

    public string Store() => string.Join(";", new[] { Pick, Inactive.ToString() }.Concat(opacity.Select(o => o.Key + "=" + o.Value.ToString("R", CultureInfo.InvariantCulture))));
    public void Restore(string stored)
    {
        string[] parts = stored.Split(';');
        if (parts.Length < 2 || Layers.All(l => l.Id != parts[0]) || !Enum.TryParse(parts[1], out InactiveMode mode)) return;
        Pick = parts[0]; Inactive = mode; opacity.Clear();
        foreach (string part in parts.Skip(2))
        {
            string[] pair = part.Split('=');
            if (pair.Length == 2 && Layers.Any(l => l.Id == pair[0]) && double.TryParse(pair[1], NumberStyles.Float, CultureInfo.InvariantCulture, out double v) && v >= 0 && v <= 1) opacity[pair[0]] = v;
        }
        Safe.Raise(Changed);
    }
}

// Token lint (ADR-D01.33): chrome and host sources name tokens; a colour written in the code is a finding.
public static class TokenLint
{
    private static readonly Regex Literal = new(@"new\s+Color\s*\(|\bColors\.[A-Z]\w+|Color\.FromHtml|""#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?""", RegexOptions.Compiled);
    public static IReadOnlyList<(int Line, string Text)> Scan(string source)
    {
        var found = new List<(int, string)>(); string[] lines = source.Split('\n');
        for (int i = 0; i < lines.Length; i++)
        {
            string line = lines[i]; int note = line.IndexOf("//", StringComparison.Ordinal);
            string code = note >= 0 ? line.Substring(0, note) : line;
            // The one place that turns a resolved token into an engine colour.
            if (Literal.IsMatch(code) && !code.Contains("(float)c.R", StringComparison.Ordinal)) found.Add((i + 1, code.Trim()));
        }
        return found;
    }

    // Words a person reads on a screen: runs of letters, two or more; numbers, symbols and key names do not count.
    public static int Words(IEnumerable<string> texts) => texts.Sum(t => Regex.Matches(t, @"[A-Za-z][A-Za-z'’-]+").Count);
}

// A long list builds only the rows in view (ADR-D01.33): which rows a scroll offset shows.
public static class VirtualRows
{
    public static (int First, int Count) Range(int rows, double rowHeight, double offset, double viewHeight, int spare = 2)
    {
        if (rows <= 0 || rowHeight <= 0) return (0, 0);
        int first = Math.Clamp((int)Math.Floor(offset / rowHeight) - spare, 0, rows - 1);
        int count = Math.Min(rows - first, (int)Math.Ceiling(viewHeight / rowHeight) + 2 * spare + 1);
        return (first, Math.Max(0, count));
    }
}

// Which icon a registered command shows, or that it is a label on purpose (ADR-D01.53).
public sealed class CommandIcons
{
    public IReadOnlyDictionary<string, string> Icons { get; }
    public IReadOnlyCollection<string> LabelOnly { get; }
    // Icons of a command with one argument, keyed "command argument".
    public IReadOnlyDictionary<string, string> Arguments { get; private init; } = new Dictionary<string, string>();
    private CommandIcons(IReadOnlyDictionary<string, string> icons, IReadOnlyCollection<string> labels) { Icons = icons; LabelOnly = labels; }

    public static CommandIcons Parse(string text)
    {
        using JsonDocument d = J.Parse(text); JsonElement root = J.Root(d, "magic600-design-command-icons", "icons", "arguments", "labelOnly");
        var icons = new SortedDictionary<string, string>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(root.GetProperty("icons"), "icons")) icons.Add(p.Name, J.String(p.Value, p.Name));
        var labels = new SortedSet<string>(J.Strings(root.GetProperty("labelOnly"), "labelOnly"), StringComparer.Ordinal);
        if (labels.Overlaps(icons.Keys)) throw J.Error("labelOnly", "a command has an icon and is label only");
        var arguments = new SortedDictionary<string, string>(StringComparer.Ordinal);
        foreach (JsonProperty p in J.Map(root.GetProperty("arguments"), "arguments")) arguments.Add(p.Name, J.String(p.Value, p.Name));
        return new CommandIcons(icons, labels) { Arguments = arguments };
    }

    public string? Of(string command, string? argument = null) =>
        argument != null && Arguments.TryGetValue(command + " " + argument, out string? own) ? own : Icons.TryGetValue(command, out string? icon) ? icon : null;
    // The commands that have neither an icon nor the label-only mark.
    public IReadOnlyList<string> Missing(IEnumerable<string> commands) => commands.Where(c => !Icons.ContainsKey(c) && !LabelOnly.Contains(c)).OrderBy(c => c, StringComparer.Ordinal).ToArray();
}
