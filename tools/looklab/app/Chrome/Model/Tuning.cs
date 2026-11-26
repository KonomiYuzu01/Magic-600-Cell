using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// One tuning: a theme family, its scene look, its five seed knobs (ADR-D01.50) and its advanced role overrides
// (B31). It is a value; Apply writes it into an Appearance and nothing else.
public sealed class TuningValue
{
    private static readonly IReadOnlyDictionary<string, Oklch> None = new SortedDictionary<string, Oklch>(StringComparer.Ordinal);
    public string Family { get; }
    public string Look { get; }
    public IReadOnlyDictionary<string, double> Knobs { get; }
    public IReadOnlyDictionary<string, Oklch> Overrides { get; }

    public TuningValue(string family, string look, IReadOnlyDictionary<string, double> knobs, IReadOnlyDictionary<string, Oklch>? overrides = null)
    {
        Family = family; Look = look; Knobs = new SortedDictionary<string, double>(knobs.ToDictionary(k => k.Key, k => k.Value), StringComparer.Ordinal);
        Overrides = new SortedDictionary<string, Oklch>((overrides ?? None).ToDictionary(k => k.Key, k => k.Value), StringComparer.Ordinal);
    }

    public static TuningValue Of(Appearance look) => new(look.Theme, look.SceneLook, ThemeFamily.KnobIds.ToDictionary(id => id, look.Knob), look.Overrides);

    // The family's other knobs and overrides go back to their shipped values first, so a value is drawn the same
    // wherever it comes from.
    public void Apply(Appearance look)
    {
        look.SetTheme(Family); look.SetSceneLook(Look); look.ResetKnobs(); look.ResetOverrides();
        foreach (var knob in Knobs) if (knob.Value != look.Family.Knobs[knob.Key].Default) look.SetKnob(knob.Key, knob.Value);
        foreach (var o in Overrides) look.SetOverride(o.Key, o.Value);
    }

    private static string N(double v) => v.ToString("0.####", CultureInfo.InvariantCulture);
    public string Text => $"{Family} · {Look} · " + string.Join(" ", Knobs.Select(k => k.Key + "=" + N(k.Value)))
        + (Overrides.Count == 0 ? "" : " · " + string.Join(" ", Overrides.Select(o => $"{o.Key}=({N(o.Value.L)},{N(o.Value.C)},{N(o.Value.H)})")));
    public bool Same(TuningValue other) => Text == other.Text;
}

// A tuning preset (B31): shipped ones are read-only and fork on edit; a user preset holds what was last saved in it.
public sealed class TuningPreset
{
    public string Id { get; }
    public string Name { get; }
    public bool Shipped { get; }
    public string? From { get; }
    public TuningValue Value { get; internal set; }

    public TuningPreset(string id, string name, bool shipped, TuningValue value, string? from = null) { Id = id; Name = name; Shipped = shipped; Value = value; From = from; }

    // tuning-presets.json: every preset is checked by applying it to a scratch appearance, so a bad family, look,
    // knob, range or role refuses the whole file.
    public static IReadOnlyList<TuningPreset> Parse(string text, Appearance look)
    {
        using JsonDocument d = J.Parse(text); JsonElement root = J.Root(d, "magic600-tuning-presets", "presets");
        var found = new List<TuningPreset>();
        var scratch = new Appearance(look.System, look.Themes);
        foreach (JsonElement p in root.GetProperty("presets").EnumerateArray())
        {
            J.Keys(p, "preset", "id", "name", "family", "look", "knobs", "overrides");
            string id = J.Text(p, "id");
            if (found.Any(f => f.Id == id)) throw J.Error(id, "repeated preset");
            var knobs = J.Map(p.GetProperty("knobs"), id).ToDictionary(k => k.Name, k => J.Number(k.Value, k.Name));
            var overrides = J.Map(p.GetProperty("overrides"), id).ToDictionary(k => k.Name, k => { double[] v = J.Numbers(k.Value, k.Name, 3); return new Oklch(v[0], v[1], v[2]); });
            var value = new TuningValue(J.Text(p, "family"), J.Text(p, "look"), knobs, overrides);
            value.Apply(scratch);
            found.Add(new TuningPreset(id, J.Text(p, "name"), true, TuningValue.Of(scratch)));
        }
        return found;
    }
}

// A live ramp strip under one knob: the steps the knob moves, drawn from the seeds in force; Mark is the step the
// knob's own value sits on where the strip spans the knob's range (grid intensity).
public sealed record TuningStrip(string Name, IReadOnlyList<Rgb> Steps, int? Mark);

// ADR-D02.47, first half (B30): the tuning panel's model over an Appearance. It sets the family, the scene look and
// the five seed knobs (ADR-D01.50), gives each knob its live ramp strip, and keeps the compare: two tuning slots, A and
// B, one of them shown (the live tuning is the shown slot), Swap, and a held compare that shows the other slot while
// held and puts the appearance back exactly on release. It holds no session and no puzzle state: a knob moves colours
// only. B31 adds the presets (shipped read-only, forked on edit), Advanced (role overrides behind a switch, with
// search), the held simulation (one 3x3 matrix for the core's view style and the chrome's filter) and Save's checks.
public sealed class Tuning
{
    public const string PanelId = "tuning", Glyph = "panel.tuning", Title = "Tuning";
    public static readonly IReadOnlyList<string> Slots = Array.AsReadOnly(new[] { "A", "B" });
    // The ramp each knob moves: tone, contrast and black level move every ramp's ladder (the neutral one shown),
    // accent hue the accent ramp, grid intensity the grid line's mix.
    public static readonly IReadOnlyDictionary<string, string> RampOf = new SortedDictionary<string, string>(StringComparer.Ordinal)
        { ["tone"] = "neutral", ["contrast"] = "neutral", ["blackLevel"] = "neutral", ["accentHue"] = "accent", ["gridIntensity"] = "grid.line" };

    public Appearance Look { get; }
    public string Shown { get; private set; } = "A";
    // The slot that is not shown; at the start both slots hold the tuning in force.
    public TuningValue Other { get; private set; }
    public bool Holding => held != null;
    private string? held;
    private TuningValue? heldLive;

    public Tuning(Appearance look, IReadOnlyList<TuningPreset>? shipped = null)
    {
        Look = look; Other = TuningValue.Of(look);
        presets.AddRange(shipped ?? Array.Empty<TuningPreset>());
        Preset = presets.FirstOrDefault(p => p.Value.Same(Live))?.Id;
    }

    // B31: the presets. The one in force is Preset; an edit to a shipped preset forks it into a user preset that the
    // next Save fills, and the shipped one is never written.
    private readonly List<TuningPreset> presets = new();
    public IReadOnlyList<TuningPreset> Presets => presets;
    public string? Preset { get; private set; }
    public TuningPreset? Current => presets.FirstOrDefault(p => p.Id == Preset);
    public bool Edited => Current is { } p && !p.Value.Same(Live);
    public string PresetLine => Current is not { } p ? "Preset: none" :
        $"Preset: {p.Name}" + (p.Shipped ? " (shipped, read-only)" : p.From != null ? $" (from {presets.First(s => s.Id == p.From).Name})" : "") + (Edited ? " · edited, not saved" : "");

    public string? ApplyPreset(string? id)
    {
        if (Busy is { } why) return why;
        if (presets.FirstOrDefault(p => p.Id == id || string.Equals(p.Name, id, StringComparison.OrdinalIgnoreCase)) is not { } preset)
            return "the presets are " + string.Join(", ", presets.Select(p => p.Id));
        string? refused = Try(() => preset.Value.Apply(Look));
        if (refused == null) Preset = preset.Id;
        return refused;
    }

    // An edit made: a shipped preset in force forks into a user preset (its base until saved).
    private string? Forked(string? refused)
    {
        if (refused == null && Current is { Shipped: true } shipped && !shipped.Value.Same(Live))
        {
            var fork = new TuningPreset(UniqueId(shipped.Id + ".copy"), shipped.Name + " copy", false, shipped.Value, shipped.Id);
            presets.Add(fork); Preset = fork.Id;
        }
        return refused;
    }

    private string UniqueId(string stem)
    {
        string id = stem;
        for (int n = 2; presets.Any(p => p.Id == id); n++) id = stem + n.ToString(CultureInfo.InvariantCulture);
        return id;
    }

    // tuning.preset.save: the Save checks first; a refusal names its failing pairs and stores nothing. A name saves
    // under that user preset (new or replaced); no name saves into the user preset in force, forking a shipped one.
    public IReadOnlyList<string> Issues { get; private set; } = Array.Empty<string>();
    public string? Save(string? name = null)
    {
        if (Busy is { } why) return why;
        Issues = SaveIssues(Look.Resolve());
        if (Issues.Count > 0) return "not saved: " + Issues[0] + (Issues.Count > 1 ? $" (and {Issues.Count - 1} more)" : "");
        TuningPreset? target;
        if (!string.IsNullOrWhiteSpace(name))
        {
            name = name.Trim();
            target = presets.FirstOrDefault(p => string.Equals(p.Name, name, StringComparison.OrdinalIgnoreCase));
            if (target is { Shipped: true }) return $"{target.Name} is a shipped preset and read-only; save under another name";
            if (target == null) presets.Add(target = new TuningPreset(UniqueId("user." + new string(name.ToLowerInvariant().Select(c => char.IsLetterOrDigit(c) ? c : '-').ToArray())), name, false, Live, Current?.Shipped == true ? Current.Id : Current?.From));
        }
        else if (Current is { Shipped: false } user) target = user;
        else
        {
            TuningPreset? shipped = Current;
            presets.Add(target = new TuningPreset(UniqueId((shipped?.Id ?? "user.tuning") + ".copy"), (shipped?.Name ?? "Tuning") + " copy", false, Live, shipped?.Id));
        }
        target.Value = Live; Preset = target.Id;
        return null;
    }

    // The slots Save checks (JumbleTint's palette plan): the contact classes with their overflow, and the height ramp.
    public static readonly IReadOnlyList<string> ClassSlots = JumbleTint.Slots.Where(s => s.Entry < JumbleTint.FirstHeight).Select(s => s.Slot).ToArray();
    public static readonly IReadOnlyList<string> HeightSlots = JumbleTint.Slots.Where(s => s.Entry >= JumbleTint.FirstHeight).Select(s => s.Slot).ToArray();
    private static string Short(string slot) => slot.Substring(slot.LastIndexOf('.') + 1);
    private static string F(double v) => v.ToString("0.000", CultureInfo.InvariantCulture);

    // ADR-D02.47's Save checks, each naming its failing pair: the system's contrast rules; a height ramp whose OkLab
    // lightness steps one way only; contact-class colours pairwise apart under normal vision and each of G4's
    // simulations (the sticker palette's bars, ADR-D01.52).
    public static IReadOnlyList<string> SaveIssues(TokenSet tokens)
    {
        var issues = new List<string>(tokens.ContrastIssues().Select(c => "contrast " + c));
        double[] light = HeightSlots.Select(s => ColourMetric.Oklab(tokens.Colour(s)).L).ToArray();
        int way = Math.Sign(light[1] - light[0]);
        for (int i = 1; i < light.Length; i++)
            if (way == 0 || Math.Sign(light[i] - light[i - 1]) != way)
                issues.Add($"height ramp {HeightSlots[i - 1]} (L {F(light[i - 1])}) then {HeightSlots[i]} (L {F(light[i])}) is not monotone");
        Rgb[] classes = ClassSlots.Select(tokens.Colour).ToArray();
        foreach (string vision in ColourMetric.Visions)
        {
            double bar = vision == "normal" ? StickerPalette.Needed : StickerPalette.NeededSimulated;
            for (int i = 0; i < classes.Length; i++)
                for (int j = i + 1; j < classes.Length; j++)
                {
                    double d = ColourMetric.Distance(ColourMetric.Simulate(classes[i], vision), ColourMetric.Simulate(classes[j], vision));
                    if (d < bar) issues.Add($"contact classes {Short(ClassSlots[i])} and {Short(ClassSlots[j])} under {vision}: {F(d)} < {F(bar)}");
                }
        }
        return issues;
    }

    // B31: Advanced, behind a switch: per-role overrides of the family in force (ADR-D01.50), found by search.
    public bool Advanced { get; private set; }
    public void SetAdvanced(bool on) => Advanced = on;
    public IReadOnlyList<string> AdvancedRoles(string? query) => Look.Family.Roles.Keys
        .Where(r => string.IsNullOrWhiteSpace(query) || r.Contains(query.Trim(), StringComparison.OrdinalIgnoreCase)).OrderBy(r => r, StringComparer.Ordinal).ToArray();

    // The table's token.override.set (role=L C h) and token.override.reset (role=reset) for the family in force. It
    // switches Advanced on, so the panel shows what it changed.
    public string? Override(string? argument)
    {
        if (Busy is { } why) return why;
        string[] kv = (argument ?? "").Split('=', 2, StringSplitOptions.TrimEntries);
        if (kv.Length != 2 || !Look.Family.Roles.ContainsKey(kv[0])) return "it needs role=L C h or role=reset, such as tint.jumble.height.4=0.5 0.02 250";
        Advanced = true;
        if (kv[1] == "reset") { Look.ResetOverride(kv[0]); return Forked(null); }
        string[] lch = kv[1].Split(new[] { ' ', ',' }, StringSplitOptions.RemoveEmptyEntries);
        var v = new double[3];
        if (lch.Length != 3 || Enumerable.Range(0, 3).Any(i => !double.TryParse(lch[i], NumberStyles.Float, CultureInfo.InvariantCulture, out v[i])))
            return "the colour is L C h, such as 0.5 0.02 250";
        return Forked(Try(() => Look.SetOverride(kv[0], new Oklch(v[0], v[1], v[2]))));
    }

    // B31: the held simulation. It changes no appearance: the host sends Matrix to the core's view style and draws it
    // over the chrome while held.
    public string Simulation { get; private set; } = "deutan";
    public string? Simulating { get; private set; }
    public string? HoldSimulation(string? id)
    {
        id ??= Simulation;
        if (!ColourMetric.Simulations.Contains(id)) return "the simulations are " + string.Join(", ", ColourMetric.Simulations);
        Simulation = id; Simulating = id;
        return null;
    }
    public void ReleaseSimulation() => Simulating = null;
    public double[] Matrix => ColourMetric.SimulationMatrix(Simulating);

    public TuningValue Live => TuningValue.Of(Look);
    public string Hidden => Shown == "A" ? "B" : "A";
    public TuningValue Slot(string slot) => slot == Shown ? heldLive ?? Live : Other;

    // The panel opens as a tab in the current layout's right dock (in Solve beside History), so the main pair and
    // the other docks stay as they are.
    // An explicit open shows what it opens: in a folded dock the panel is revealed over the fold (B31).
    public static bool Open(FrameModel frame, bool folded = false) => frame.RevealDock(DockSide.Right, PanelId, folded);

    private string? Busy => Holding ? "release the held compare first" : null;

    public string? SetFamily(string id) => Busy ?? Forked(Try(() => Look.SetTheme(id)));
    public string? SetLook(string id) => Busy ?? Forked(Try(() => Look.SetSceneLook(id)));
    public string? SetKnob(string id, double value)
    {
        if (Busy is { } why) return why;
        if (!Look.Family.Knobs.TryGetValue(id, out Knob? knob)) return id + " is not a seed knob";
        return Forked(Try(() => Look.SetKnob(id, Math.Clamp(value, knob.Min, knob.Max))));
    }

    // theme.tune's argument: knob=value pairs, comma separated, for the family in force.
    public string? Tune(string? argument)
    {
        if (Busy is { } busy) return busy;
        var pairs = new List<(string, double)>();
        foreach (string part in (argument ?? "").Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            string[] kv = part.Split('=', 2, StringSplitOptions.TrimEntries);
            if (kv.Length != 2 || !Look.Family.Knobs.TryGetValue(kv[0], out Knob? knob)) return $"'{part}' is not knob=value; the knobs are {string.Join(", ", ThemeFamily.KnobIds)}";
            if (!double.TryParse(kv[1], NumberStyles.Float, CultureInfo.InvariantCulture, out double v) || !double.IsFinite(v) || v < knob.Min || v > knob.Max)
                return $"{kv[0]} takes {knob.Min.ToString(CultureInfo.InvariantCulture)} to {knob.Max.ToString(CultureInfo.InvariantCulture)}";
            pairs.Add((kv[0], v));
        }
        if (pairs.Count == 0) return "it needs knob=value, such as tone=0.03";
        foreach (var (id, v) in pairs) Look.SetKnob(id, v);
        return Forked(null);
    }

    // Stores the live tuning in a slot; the shown slot is the live tuning already.
    public string? SetSlot(string? slot)
    {
        if (Busy is { } why) return why;
        if (slot == null || !Slots.Contains(slot)) return "it needs the slot A or B";
        if (slot != Shown) Other = Live;
        return null;
    }

    // Shows the other slot; the live tuning goes into the slot it leaves. Twice is no change.
    public string? Swap()
    {
        if (Busy is { } why) return why;
        TuningValue next = Other, now = Live;
        string? refused = Try(() => next.Apply(Look));
        if (refused != null) return refused;
        Other = now; Shown = Hidden;
        return null;
    }

    // While held, the other slot is drawn; the release puts the appearance back as it was when the hold began.
    public string? Hold()
    {
        if (Holding) return null;
        string stored = Look.Store(); TuningValue live = Live;
        string? refused = Try(() => Other.Apply(Look));
        if (refused != null) { Look.Restore(stored); return refused; }
        held = stored; heldLive = live;
        return null;
    }

    public string? Release()
    {
        if (held is not { } stored) return null;
        held = null; heldLive = null;
        return Try(() => Look.Restore(stored));
    }

    // The words of the compare line: which slot is shown, and what the other holds.
    public string Line => Holding
        ? $"Holding: slot {Hidden} drawn; release to return to slot {Shown}"
        : $"Showing slot {Shown}. Slot {Hidden}: " + (Other.Same(Live) ? "the same tuning" : Other.Text);

    public TuningStrip Strip(string knob)
    {
        ThemeFamily family = Look.Family;
        var knobs = ThemeFamily.KnobIds.ToDictionary(id => id, Look.Knob);
        string ramp = RampOf.TryGetValue(knob, out string? r) ? r : "neutral";
        var mix = family.Roles.Where(x => x.Value.MixKnob == knob && x.Value.MixFrom != null && x.Value.MixTo != null)
            .OrderBy(x => x.Key, StringComparer.Ordinal).FirstOrDefault();
        if (mix.Key == null)
        {
            if (!family.Ramps.ContainsKey(ramp)) ramp = "neutral";
            return new TuningStrip(ramp, Enumerable.Range(0, ThemeFamily.Steps).Select(s => family.Step(ramp, s, knobs).ToSrgb()).ToArray(), null);
        }
        // A mixed role across the knob's whole range, its ends from the seeds in force.
        RoleSource source = mix.Value;
        Rgb At(string role) { RoleSource end = family.Roles[role]; return family.Step(end.Ramp!, end.Step, knobs).ToSrgb(); }
        Rgb a = At(source.MixFrom!), b = At(source.MixTo!);
        Knob range = family.Knobs[knob];
        var steps = new Rgb[ThemeFamily.Steps];
        for (int i = 0; i < steps.Length; i++)
        {
            double v = range.Min + (range.Max - range.Min) * i / (steps.Length - 1), t = Math.Clamp(v * source.MixScale, 0, 1);
            steps[i] = new Rgb(a.R + (b.R - a.R) * t, a.G + (b.G - a.G) * t, a.B + (b.B - a.B) * t);
        }
        int mark = (int)Math.Round((Look.Knob(knob) - range.Min) / (range.Max - range.Min) * (steps.Length - 1));
        return new TuningStrip(mix.Key, steps, Math.Clamp(mark, 0, steps.Length - 1));
    }

    private static string? Try(Action change)
    {
        try { change(); return null; }
        catch (FormatException ex) { return ex.Message; }
    }
}
