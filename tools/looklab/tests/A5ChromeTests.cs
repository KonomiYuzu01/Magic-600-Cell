using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using LookLab.Core;
using Magic600.Chrome;

internal static partial class Program
{
    private static DesignSystem? design;
    private static DesignSystem Design => design ??= DesignSystem.Load(FileAt("data/design/system.json"));
    private static ThemeFamily[] Themes() => Directory.GetFiles(FileAt("data/design/themes"), "*.json").OrderBy(p => p, StringComparer.Ordinal)
        .Select(p => ThemeFamily.Load(p, Design)).ToArray();

    private static void A5TokensEveryThemeAssignsEveryRoleAndPassesContrast()
    {
        ThemeFamily[] themes = Themes();
        Check(themes.Length >= 2 && themes.Any(t => t.Scheme == "dark") && themes.Any(t => t.Scheme == "light"), "two theme families, one dark and one light, required");
        foreach (ThemeFamily theme in themes) foreach (string density in Design.Densities)
        {
            var tokens = new TokenSet(Design, theme, density, 1);
            Equal(tokens.Colours.Count, Design.Roles.Count);
            Sequence(tokens.ContrastIssues(), Array.Empty<string>(), theme.Id + " contrast: " + string.Join("; ", tokens.ContrastIssues()));
            Check(!tokens.NonStandardColours && tokens.Colours.Values.All(c => !c.Overridden), "shipped colours flagged as overrides");
            foreach (string component in Design.Components.Keys) tokens.Colour(component);
            // The scene looks name Look Lab parameters and stay inside their ranges.
            foreach (var scene in theme.Scenes) foreach (var value in scene.Value)
                ParameterSchema.ValidateValue(Schema[value.Key], ParameterSchema.ReadValue(Schema[value.Key], value.Value));
        }
        // State and encoding roles keep distinct colours inside one theme, and every one has a cue that is not colour.
        foreach (ThemeFamily theme in themes)
        {
            var tokens = new TokenSet(Design, theme, Design.ShippedDensity, 1);
            string[] strip = Design.Roles.Values.Where(r => r.Group == "encoding").Select(r => tokens.Colour(r.Id).Hex).ToArray();
            Equal(strip.Distinct().Count(), strip.Length, "relationship strip classes share a colour in " + theme.Id);
        }
        Check(Design.Roles.Values.Where(r => r.Group is "state" or "encoding").All(r => !string.IsNullOrWhiteSpace(r.Cue)), "a state role has no cue");
    }

    private static void A5TokensDensityFromOneUnitAndDisplayScale()
    {
        Equal(Design.ShippedDensity, "compact");
        var compact = Design.Density("compact", 1); var comfortable = Design.Density("comfortable", 1); var scaled = Design.Density("compact", 1.5);
        Equal(compact["control.height"], Math.Round(Design.DensityTable["control.height"][0] * Design.Unit));
        Equal(compact["border.width"], 1.0); Equal(Design.Density("compact", .5)["border.width"], 1.0, "a border fell below one pixel");
        Check(Design.DensityTable.Keys.All(k => comfortable[k] >= compact[k]), "a comfortable size is smaller than its compact size");
        Equal(scaled["control.height"], Math.Round(Design.DensityTable["control.height"][0] * Design.Unit * 1.5));
        Refuse(() => Design.Density("roomy", 1), "roomy"); Refuse(() => Design.Density("compact", 9), "displayScale");
        Sequence(Design.Type.Keys, new[] { "code", "content", "ui" });
        Equal(Design.Type["ui"].Faces[0], "IBM Plex Sans"); Equal(Design.Type["content"].Faces[0], "IBM Plex Serif"); Equal(Design.Type["code"].Faces[0], "Lilex");
        Check(Design.Type["ui"].Tabular && Design.Type["code"].Tabular, "number fields need tabular figures");
    }

    private static void A5TokensKnobsMoveSeedsInsideTheirRanges()
    {
        ThemeFamily theme = Themes().First(t => t.Scheme == "dark");
        var shipped = new TokenSet(Design, theme, "compact", 1);
        var tuned = new TokenSet(Design, theme, "compact", 1, new Dictionary<string, double> { ["accentHue"] = 20, ["blackLevel"] = .1, ["gridIntensity"] = 1 });
        Check(tuned.Colour("accent.base") != shipped.Colour("accent.base"), "accent hue knob did not move the accent");
        Check(Oklch.Luminance(tuned.Colour("surface.window")) > Oklch.Luminance(shipped.Colour("surface.window")), "black level knob did not lift the dark end");
        Check(tuned.Colour("grid.line") != shipped.Colour("grid.line"), "grid intensity knob did not move the grid line");
        var hued = new TokenSet(Design, theme, "compact", 1, new Dictionary<string, double> { ["accentHue"] = 20, ["gridIntensity"] = 1 });
        Equal(hued.Colour("state.invalid"), shipped.Colour("state.invalid"), "the accent knob moved a state colour that has its own ramp");
        Check(!tuned.NonStandardColours, "a knob counted as a colour override");
        Refuse(() => new TokenSet(Design, theme, "compact", 1, new Dictionary<string, double> { ["tone"] = .5 }), "tone");
        Refuse(() => new TokenSet(Design, theme, "compact", 1, new Dictionary<string, double> { ["gamma"] = 1 }), "gamma");
        foreach (Rgb c in shipped.Colours.Values.Select(v => v.Value)) Check(new[] { c.R, c.G, c.B }.All(v => v >= 0 && v <= 1), "colour outside sRGB");
        Equal(new Oklch(1, 0, 0).ToSrgb().Hex, "#ffffff"); Equal(new Oklch(0, 0, 0).ToSrgb().Hex, "#000000");
        Near(Oklch.Contrast(new Rgb(1, 1, 1), new Rgb(0, 0, 0)), 21);
    }

    private static void A5TokensOverridesFlagNonStandardStateColours()
    {
        ThemeFamily theme = Themes().First(t => t.Scheme == "dark");
        string file = "{\"format\":\"magic600-design-overrides\",\"version\":1,\"theme\":\"" + theme.Id + "\",\"roles\":{\"ROLE\":{\"L\":0.7,\"C\":0.1,\"h\":120}}}";
        var chrome = new TokenSet(Design, theme, "compact", 1, null, TokenOverrides.Parse(file.Replace("ROLE", "surface.panel"), theme));
        Check(chrome.Colours["surface.panel"].Overridden && !chrome.NonStandardColours, "a chrome override was flagged as non-standard state colours");
        Check(chrome.ContrastIssues().Count > 0, "the contrast check passed a light panel under light text");
        var state = new TokenSet(Design, theme, "compact", 1, null, TokenOverrides.Parse(file.Replace("ROLE", "state.preview"), theme));
        Check(state.NonStandardColours && state.Colours["state.preview"].Overridden, "an overridden state colour did not raise the non-standard mark");
        Equal(Design.Roles["state.preview"].Cue, "dashed outline and the label \"preview\"");
        Refuse(() => TokenOverrides.Parse(file.Replace("ROLE", "state.preview").Replace(theme.Id, "other"), theme), "theme");
        Refuse(() => new TokenSet(Design, theme, "compact", 1, null, TokenOverrides.Parse(file.Replace("ROLE", "mystery.role"), theme)), "mystery.role");
        Refuse(() => new TokenSet(Design, theme, "compact", 1, null, TokenOverrides.Parse(file.Replace("0.7", "1.7").Replace("ROLE", "state.preview"), theme)), "state.preview");
    }

    private static void A5AppearanceResolvesStoresAndRefuses()
    {
        ThemeFamily[] themes = Themes();
        var look = new Appearance(Design, themes);
        Equal(look.Theme, themes[0].Id); Equal(look.Density, "compact"); Check(!look.Tuned && !look.HighContrast, "a fresh appearance is tuned");
        Equal(look.Resolve().Colour("surface.panel"), new TokenSet(Design, themes[0], "compact", 1).Colour("surface.panel"));
        look.SetKnob("accentHue", 20); look.SetTheme(themes[1].Id);
        Check(!look.Tuned && look.Knob("accentHue") == themes[1].Knobs["accentHue"].Default, "a knob of one family moved another");
        look.SetTheme(themes[0].Id); Check(look.Tuned && look.Knob("accentHue") == 20, "a family lost its knob");
        look.SetDensity("comfortable"); look.SetDisplayScale(1.25); look.SetSceneLook("inspecting"); look.HighContrast = true;
        Appearance again = Appearance.Parse(look.Store(), Design, themes);
        Equal(again.Store(), look.Store()); Equal(again.Resolve().Size("control.height"), look.Resolve().Size("control.height"));
        // High contrast: text and lines at the end of the scale, in both schemes, with every contrast rule still met.
        foreach (ThemeFamily theme in themes)
        {
            var plain = new TokenSet(Design, theme, "compact", 1); var high = new TokenSet(Design, theme, "compact", 1, null, null, true);
            Near(Oklch.Contrast(high.Colour("text.primary"), high.Colour("surface.window")), 21);
            Check(Oklch.Contrast(high.Colour("border.subtle"), high.Colour("surface.panel")) > Oklch.Contrast(plain.Colour("border.subtle"), plain.Colour("surface.panel")) * 2, "high contrast borders are not stronger");
            Sequence(high.ContrastIssues(), Array.Empty<string>()); Check(!high.NonStandardColours, "high contrast counted as a colour override");
            Equal(high.Colour("state.preview"), plain.Colour("state.preview"));
        }
        look.ResetKnobs(); Check(!look.Tuned, "knobs not reset");
        Refuse(() => look.SetTheme("neon"), "neon"); Refuse(() => look.SetDensity("roomy"), "roomy"); Refuse(() => look.SetDisplayScale(9), "displayScale");
        Refuse(() => look.SetKnob("tone", 5), "tone"); Refuse(() => look.SetSceneLook("resting"), "resting");
        string stored = again.Store();
        foreach ((string from, string to, string name) in new[] { ("scale=1.25", "scale=40", "displayScale"), ("contrast=high", "contrast=max", "contrast"),
            ("density=comfortable", "density=roomy", "roomy"), ("scene=inspecting", "scene=inspecting\nscene=solving", "scene"), ("accentHue=20", "accentHue=720", "accentHue"),
            ("theme=", "family=", "family") })
        {
            Check(stored.Contains(from, StringComparison.Ordinal), "fixture lost " + from);
            Refuse(() => Appearance.Parse(stored.Replace(from, to), Design, themes), name);
        }
    }

    private static void A5TokensFileRefusals()
    {
        string system = Read("data/design/system.json"), theme = File.ReadAllText(Directory.GetFiles(FileAt("data/design/themes"), "*.json").OrderBy(p => p, StringComparer.Ordinal).First());
        Refuse(() => DesignSystem.Parse(system.Replace("\"shipped\": \"compact\"", "\"shipped\": \"dense\"")), "shipped");
        Refuse(() => DesignSystem.Parse(system.Replace("\"unit\": 4", "\"unit\": 0")), "unit");
        Refuse(() => DesignSystem.Parse(system.Replace("\"ui\": {", "\"display\": {")), "type");
        Refuse(() => DesignSystem.Parse(system.Replace("\"cue\": \"lock glyph\"", "\"meaning2\": \"x\"")), "meaning2");
        Refuse(() => DesignSystem.Parse(system.Replace("\"window.background\": \"surface.window\"", "\"window.background\": \"surface.nowhere\"")), "window.background");
        Refuse(() => ThemeFamily.Parse(theme.Replace("\"scheme\": \"dark\"", "\"scheme\": \"light\""), Design), "ladder");
        Refuse(() => ThemeFamily.Parse(theme.Replace("\"surface.panel\": \"neutral.1\"", "\"surface.panel\": \"neutral.12\""), Design), "surface.panel");
        Refuse(() => ThemeFamily.Parse(theme.Replace("\"surface.panel\": \"neutral.1\",", ""), Design), "surface.panel");
        Refuse(() => ThemeFamily.Parse(theme.Replace("\"celebrating\"", "\"resting\""), Design), "sceneLooks");
        Refuse(() => ThemeFamily.Parse(theme.Replace("\"tone\"", "\"warmth\""), Design), "knobs");
    }
}
