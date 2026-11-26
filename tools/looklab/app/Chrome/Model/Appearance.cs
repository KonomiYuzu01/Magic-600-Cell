using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace Magic600.Chrome;

// What the user has chosen for the look of the chrome: theme family, density, display scale, high contrast,
// scene look and the tuning knobs of each family (ADR-D01.35, ADR-D01.45, ADR-D01.49, ADR-D01.50). It names
// tokens and knob values only; every colour and size is resolved from the design data.
public sealed class Appearance
{
    public string Theme { get; private set; }
    public string Density { get; private set; }
    public double DisplayScale { get; private set; } = 1;
    public bool HighContrast { get; set; }
    public bool OtherCharacter { get; set; }
    public bool ReducedMotion { get; set; }
    // The scale of the display the window is on, set by the host and not stored: the user's display scale
    // multiplies it, so the same stored appearance reads the same on a dense and on a plain screen.
    public double SystemScale { get; private set; } = 1;
    public double EffectiveScale => Math.Clamp(DisplayScale * SystemScale, .5, 4);
    public void SetSystemScale(double scale) { if (!(scale >= .5 && scale <= 4)) throw J.Error("systemScale", "outside 0.5..4"); SystemScale = scale; }
    public string SceneLook { get; private set; } = ThemeFamily.SceneLooks[0];
    private readonly DesignSystem system;
    private readonly IReadOnlyList<ThemeFamily> themes;
    private readonly SortedDictionary<string, SortedDictionary<string, double>> knobs = new(StringComparer.Ordinal);

    public Appearance(DesignSystem system, IReadOnlyList<ThemeFamily> themes)
    {
        if (themes.Count == 0) throw J.Error("themes", "at least one theme family required");
        this.system = system; this.themes = themes;
        Theme = themes[0].Id; Density = system.ShippedDensity;
    }

    public IReadOnlyList<ThemeFamily> Themes => themes;
    public ThemeFamily Family => themes.First(t => t.Id == Theme);
    public double Knob(string id) => knobs.TryGetValue(Theme, out var set) && set.TryGetValue(id, out double v) ? v : Family.Knobs[id].Default;
    public bool Tuned => knobs.TryGetValue(Theme, out var set) && set.Any(k => k.Value != Family.Knobs[k.Key].Default);

    // The theme follows the system's light or dark setting until a choice is stored (ADR-D01.50).
    public void FollowSystem(bool dark) => Theme = themes.FirstOrDefault(t => t.Scheme == (dark ? "dark" : "light"))?.Id ?? Theme;
    public void SetTheme(string id) { if (themes.All(t => t.Id != id)) throw J.Error(id, "unknown theme family"); Theme = id; }
    public void SetDensity(string preset) { system.Density(preset, DisplayScale); Density = preset; }
    public void SetDisplayScale(double scale) { system.Density(Density, scale); DisplayScale = scale; }
    public void SetSceneLook(string look) { if (!ThemeFamily.SceneLooks.Contains(look)) throw J.Error(look, "unknown scene look"); SceneLook = look; }

    public void SetKnob(string id, double value) => SetKnob(Family, id, value);
    private void SetKnob(ThemeFamily family, string id, double value)
    {
        if (!family.Knobs.TryGetValue(id, out Knob? knob)) throw J.Error(id, "unknown knob");
        if (!double.IsFinite(value) || value < knob.Min || value > knob.Max) throw J.Error(id, "outside its range");
        if (!knobs.TryGetValue(family.Id, out var set)) knobs[family.Id] = set = new SortedDictionary<string, double>(StringComparer.Ordinal);
        set[id] = value;
    }

    public void ResetKnobs() => knobs.Remove(Theme);

    // ADR-D01.50's advanced overrides (B31): semantic roles of the family in force, each an OkLCH colour.
    private readonly SortedDictionary<string, SortedDictionary<string, Oklch>> overrides = new(StringComparer.Ordinal);
    private static readonly IReadOnlyDictionary<string, Oklch> NoOverrides = new SortedDictionary<string, Oklch>(StringComparer.Ordinal);
    public DesignSystem System => system;
    public IReadOnlyDictionary<string, Oklch> Overrides => overrides.TryGetValue(Theme, out var set) ? set : NoOverrides;
    public void SetOverride(string role, Oklch c) => SetOverride(Family, role, c);
    private void SetOverride(ThemeFamily family, string role, Oklch c)
    {
        if (!system.Roles.ContainsKey(role)) throw J.Error(role, "not a semantic role");
        if (!(c.L >= 0 && c.L <= 1 && c.C >= 0 && c.C <= .4 && c.H >= 0 && c.H <= 360)) throw J.Error(role, "outside OkLCH ranges (L 0..1, C 0..0.4, h 0..360)");
        if (!overrides.TryGetValue(family.Id, out var set)) overrides[family.Id] = set = new SortedDictionary<string, Oklch>(StringComparer.Ordinal);
        set[role] = c;
    }
    public void ResetOverride(string role) { if (overrides.TryGetValue(Theme, out var set) && set.Remove(role) && set.Count == 0) overrides.Remove(Theme); }
    public void ResetOverrides() => overrides.Remove(Theme);

    public TokenSet Resolve() => new(system, Family, Density, EffectiveScale,
        ThemeFamily.KnobIds.ToDictionary(id => id, Knob), Overrides.Count > 0 ? Overrides : null, HighContrast) { OtherCharacter = OtherCharacter, ReducedMotion = ReducedMotion };

    private static string N(double v) => v.ToString("R", CultureInfo.InvariantCulture);

    // One line per setting, in a fixed order.
    public string Store()
    {
        var lines = new List<string> { "theme=" + Theme, "density=" + Density, "scale=" + N(DisplayScale), "contrast=" + (HighContrast ? "high" : "standard"), "scene=" + SceneLook };
        if (OtherCharacter) lines.Add("character=other");
        if (ReducedMotion) lines.Add("motion=reduced");
        foreach (var family in knobs) foreach (var knob in family.Value) lines.Add($"knob.{family.Key}.{knob.Key}={N(knob.Value)}");
        foreach (var family in overrides) foreach (var o in family.Value) lines.Add($"override.{family.Key}.{o.Key}={N(o.Value.L)} {N(o.Value.C)} {N(o.Value.H)}");
        return string.Join("\n", lines);
    }

    // A stored appearance put back in place (the layout history's undo), whole or refused whole: hosts that hold this
    // instance keep it.
    public void Restore(string stored)
    {
        Appearance kept = Parse(stored, system, themes);
        Theme = kept.Theme; Density = kept.Density; DisplayScale = kept.DisplayScale; HighContrast = kept.HighContrast; SceneLook = kept.SceneLook;
        OtherCharacter = kept.OtherCharacter; ReducedMotion = kept.ReducedMotion;
        knobs.Clear();
        foreach (var family in kept.knobs) knobs[family.Key] = new SortedDictionary<string, double>(family.Value, StringComparer.Ordinal);
        overrides.Clear();
        foreach (var family in kept.overrides) overrides[family.Key] = new SortedDictionary<string, Oklch>(family.Value, StringComparer.Ordinal);
    }

    // A stored appearance is applied whole or refused whole.
    public static Appearance Parse(string stored, DesignSystem system, IReadOnlyList<ThemeFamily> themes)
    {
        var result = new Appearance(system, themes);
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (string line in stored.Split('\n'))
        {
            int cut = line.IndexOf('=');
            if (cut <= 0) throw J.Error("appearance", "expected name=value lines");
            string name = line.Substring(0, cut), value = line.Substring(cut + 1);
            if (!seen.Add(name)) throw J.Error(name, "repeated setting");
            double Number() => double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out double v) ? v : throw J.Error(name, "expected a number");
            string[] parts = name.Split('.');
            if (name == "theme") result.SetTheme(value);
            else if (name == "density") result.Density = system.Densities.Contains(value) ? value : throw J.Error(value, "unknown density preset");
            else if (name == "scale") result.DisplayScale = Number();
            else if (name == "contrast") result.HighContrast = value == "high" ? true : value == "standard" ? false : throw J.Error(name, "expected high or standard");
            else if (name == "scene") result.SetSceneLook(value);
            else if (name == "character" && value == "other") result.OtherCharacter = true;
            else if (name == "motion" && value == "reduced") result.ReducedMotion = true;
            else if (parts.Length == 3 && parts[0] == "knob" && themes.FirstOrDefault(t => t.Id == parts[1]) is { } family) result.SetKnob(family, parts[2], Number());
            // A role name has dots of its own: override.<family>.<role>=L C h.
            else if (parts.Length >= 3 && parts[0] == "override" && themes.FirstOrDefault(t => t.Id == parts[1]) is { } tuned)
            {
                string[] lch = value.Split(' ', StringSplitOptions.RemoveEmptyEntries);
                double At(int i) => lch.Length == 3 && double.TryParse(lch[i], NumberStyles.Float, CultureInfo.InvariantCulture, out double v) ? v : throw J.Error(name, "expected L C h");
                result.SetOverride(tuned, name.Substring(("override." + parts[1] + ".").Length), new Oklch(At(0), At(1), At(2)));
            }
            else throw J.Error(name, "unknown setting");
        }
        system.Density(result.Density, result.DisplayScale);
        return result;
    }
}
