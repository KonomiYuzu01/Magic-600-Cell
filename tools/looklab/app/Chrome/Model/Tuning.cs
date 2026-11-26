using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace Magic600.Chrome;

// One tuning: a theme family, its scene look and its five seed knobs (ADR-D01.50). It is a value; Apply writes it
// into an Appearance and nothing else.
public sealed class TuningValue
{
    public string Family { get; }
    public string Look { get; }
    public IReadOnlyDictionary<string, double> Knobs { get; }

    public TuningValue(string family, string look, IReadOnlyDictionary<string, double> knobs)
    {
        Family = family; Look = look; Knobs = new SortedDictionary<string, double>(knobs.ToDictionary(k => k.Key, k => k.Value), StringComparer.Ordinal);
    }

    public static TuningValue Of(Appearance look) => new(look.Theme, look.SceneLook, ThemeFamily.KnobIds.ToDictionary(id => id, look.Knob));

    // The family's other knobs go back to their shipped values first, so a value is drawn the same wherever it comes from.
    public void Apply(Appearance look)
    {
        look.SetTheme(Family); look.SetSceneLook(Look); look.ResetKnobs();
        foreach (var knob in Knobs) if (knob.Value != look.Family.Knobs[knob.Key].Default) look.SetKnob(knob.Key, knob.Value);
    }

    public string Text => $"{Family} · {Look} · " + string.Join(" ", Knobs.Select(k => k.Key + "=" + k.Value.ToString("0.####", CultureInfo.InvariantCulture)));
    public bool Same(TuningValue other) => Text == other.Text;
}

// A live ramp strip under one knob: the steps the knob moves, drawn from the seeds in force; Mark is the step the
// knob's own value sits on where the strip spans the knob's range (grid intensity).
public sealed record TuningStrip(string Name, IReadOnlyList<Rgb> Steps, int? Mark);

// ADR-D02.47, first half (B30): the tuning panel's model over an Appearance. It sets the family, the scene look and
// the five seed knobs (ADR-D01.50), gives each knob its live ramp strip, and keeps the compare: two tuning slots, A and
// B, one of them shown (the live tuning is the shown slot), Swap, and a held compare that shows the other slot while
// held and puts the appearance back exactly on release. It holds no session and no puzzle state: a knob moves colours
// only. The presets, Advanced, the simulation and the Save checks are B31's.
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

    public Tuning(Appearance look) { Look = look; Other = TuningValue.Of(look); }

    public TuningValue Live => TuningValue.Of(Look);
    public string Hidden => Shown == "A" ? "B" : "A";
    public TuningValue Slot(string slot) => slot == Shown ? heldLive ?? Live : Other;

    // The panel opens as a tab in the current layout's right dock (in Solve beside History), so the main pair and
    // the other docks stay as they are.
    public static bool Open(FrameModel frame) => frame.ShowDock(DockSide.Right, PanelId);

    private string? Busy => Holding ? "release the held compare first" : null;

    public string? SetFamily(string id) => Busy ?? Try(() => Look.SetTheme(id));
    public string? SetLook(string id) => Busy ?? Try(() => Look.SetSceneLook(id));
    public string? SetKnob(string id, double value)
    {
        if (Busy is { } why) return why;
        if (!Look.Family.Knobs.TryGetValue(id, out Knob? knob)) return id + " is not a seed knob";
        return Try(() => Look.SetKnob(id, Math.Clamp(value, knob.Min, knob.Max)));
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
        return null;
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
