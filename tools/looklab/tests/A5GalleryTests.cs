using System;
using System.IO;
using System.Linq;
using LookLab.Core;
using Magic600.Chrome;

internal static partial class Program
{
    private static ThemeFamily[] Families(DesignSystem system) => new[] { "graphite", "paper" }.Select(id => ThemeFamily.Parse(Read("data/design/themes/" + id + ".json"), system)).ToArray();

    // The motion table, reduced motion, and the two families as two characters with fixed accent roles.
    private static void A5GalleryMotionTableAndCharacters()
    {
        DesignSystem system = Design;
        string text = Read("data/design/motion.json"); MotionTable motions = MotionTable.Parse(text, system);
        foreach (string id in new[] { "twist.committed", "twist.queued", "twist.preview", "view.transition", "span.flash", "dock.open", "task.done", "marker.pending", "cursor.rollback" })
            Check(motions.Entries.Any(e => e.Id == id), "the motion table misses " + id);
        Check(motions.Entries.All(e => motions.Seconds(e.Id, system, true) == 0 && e.Static.Length > 0), "reduced motion leaves a motion running");
        Check(motions.Seconds("twist.queued", system, false) < motions.Seconds("twist.committed", system, false), "a queued twist is not faster");
        Check(motions["twist.preview"].Curve != motions["twist.committed"].Curve, "a preview twist moves like a committed one");
        Refuse(() => MotionTable.Parse(text.Replace("\"cue.task\"", "\"cue.success\""), system), "never tells a good result from a bad one");
        Refuse(() => MotionTable.Parse(text.Replace("\"duration\": \"flash\"", "\"duration\": \"blink\""), system), "unknown duration token");
        double[] curve = system.Curves["standard"]; double last = 0;
        for (int i = 0; i <= 20; i++) { double at = MotionTable.Ease(curve, i / 20.0); Check(at >= last - 1e-9, "a curve goes back"); last = at; }
        Check(Math.Abs(MotionTable.Ease(curve, 0)) < 1e-3 && Math.Abs(MotionTable.Ease(curve, 1) - 1) < 1e-3, "a curve does not run from 0 to 1");
        var settings = new MotionSettings { Reduced = true, Muted = false }; var kept = new MotionSettings(); kept.Restore(settings.Store());
        Check(kept.Reduced && !kept.Muted && kept.Rewards && new MotionSettings().Muted, "motion settings are not kept, or cues are not muted by default");

        ThemeFamily[] families = Families(system);
        Check(families[0].Character != families[1].Character, "the two families share one character");
        foreach (ThemeFamily family in families)
        {
            var look = new Appearance(system, families); look.SetTheme(family.Id); TokenSet t = look.Resolve();
            string[] roles = { "state.committed", "state.preview", "script.frame", "state.selected" };
            Check(roles.Select(r => t.Colour(r)).Distinct().Count() == roles.Length, family.Id + ": two accent roles share a colour");
            Check(t.ContrastIssues().Count == 0, family.Id + ": " + string.Join("; ", t.ContrastIssues()));
            look.HighContrast = true; Check(look.Resolve().ContrastIssues().Count == 0, family.Id + " high contrast: a pair fails");
            string[] keys = family.Scenes["solving"].Keys.OrderBy(k => k, StringComparer.Ordinal).ToArray();
            Check(keys.Length >= 6 && ThemeFamily.SceneLooks.All(l => family.Scenes[l].Keys.OrderBy(k => k, StringComparer.Ordinal).SequenceEqual(keys)), family.Id + ": the scene looks set different parameters");
            Check(ThemeFamily.SceneLooks.Select(l => string.Join(",", keys.Select(k => family.Scenes[l][k].ToString()))).Distinct().Count() == 3, family.Id + ": two scene looks are the same");
        }
        var plain = new Appearance(system, families);
        Check(!plain.Store().Contains("character", StringComparison.Ordinal) && !plain.Store().Contains("motion", StringComparison.Ordinal) && plain.Resolve().Lines == (plain.Family.Character == "lines"), "a default appearance stores a character or a motion line");
        plain.OtherCharacter = true; plain.ReducedMotion = true;
        Appearance again = Appearance.Parse(plain.Store(), system, families);
        Check(again.OtherCharacter && again.ReducedMotion && again.Resolve().ReducedMotion && again.Resolve().Lines != plain.Family.Character.Equals("lines", StringComparison.Ordinal), "the other character or reduced motion is not kept");
    }

    // Sticker palettes pass their distances, every command has an icon or is a label on purpose, no colour is written in the code.
    private static void A5GalleryPalettesIconsAndTokenLint()
    {
        var palettes = StickerPalette.Parse(Read("data/design/palettes.json"));
        foreach (string family in new[] { "graphite", "paper" })
        {
            Check(StickerPalette.Offered(palettes, family).Count >= 3, family + ": fewer than three palettes pass");
            Check(palettes.Any(p => p.Family == family && !p.Passes), family + ": no candidate shows what is not offered");
        }
        Check(palettes.Where(p => p.Passes).All(p => p.Smallest("normal") >= StickerPalette.Needed && ColourMetric.Visions.Skip(1).All(v => p.Smallest(v) >= StickerPalette.NeededSimulated)), "an offered palette is too close");
        // A palette is also the lab parameters the core draws stickers from, inside their ranges.
        foreach (StickerPalette palette in palettes)
        {
            Check(palette.Lab.Count > 0, palette.Id + " names no lab parameter");
            foreach (var value in palette.Lab)
                ParameterSchema.ValidateValue(Schema[value.Key], ParameterSchema.ReadValue(Schema[value.Key], System.Text.Json.JsonDocument.Parse(value.Value.ToString("R", System.Globalization.CultureInfo.InvariantCulture)).RootElement));
        }
        Rgb red = new(1, 0, 0), green = new(0, .6, 0);
        Check(ColourMetric.Distance(ColourMetric.Simulate(red, "deutan"), ColourMetric.Simulate(green, "deutan")) < ColourMetric.Distance(red, green), "the simulation does not bring red and green together");

        CommandRegistry registry = LabRegistry();
        CommandIcons commands = CommandIcons.Parse(Read("data/design/command-icons.json")); IconSet icons = IconSet.Parse(Read("data/design/icons.json"));
        Check(commands.Missing(registry.Order).Count == 0, "commands with neither an icon nor the label-only mark: " + string.Join(", ", commands.Missing(registry.Order).Take(5)));
        // ADR-D02's draft rows are registered by the shell only with D02 on; their marks name known commands too.
        var d02 = new CommandRegistry(); d02.AddRows(Read("data/design/command-rows-d02.json"));
        Check(commands.Icons.Values.Concat(commands.Arguments.Values).All(icons.Icons.ContainsKey) && commands.Icons.Keys.Concat(commands.LabelOnly).All(id => registry.Commands.ContainsKey(id) || d02.Commands.ContainsKey(id)), "the command icons name an unknown icon or command");
        Keymap keys = Keymap.Parse(Read("data/design/keymap.json"), registry);
        Check(keys.Radials.Values.SelectMany(r => r.Entries).All(e => commands.Of(e.Command, e.Argument) != null), "a radial entry has no icon");
        Check(FocusLayers.Layers.All(l => icons.Icons.ContainsKey(l.Icon)) && icons.Icons.Keys.Count(k => k.StartsWith("glyph.", StringComparison.Ordinal)) >= 6, "a layer or content glyph is missing");

        // Token lint (G-4): the chrome library and the lab's wiring of it name tokens. The lab's own reload notice is not chrome.
        foreach (string folder in new[] { "app/Chrome/Ui", "app/Live" })
            foreach (string file in Directory.GetFiles(Path.Combine(Lab, folder), "*.cs").Where(f => Path.GetFileName(f) != "LiveSession.cs"))
            {
                var found = TokenLint.Scan(File.ReadAllText(file));
                Check(found.Count == 0, $"{Path.GetFileName(file)}:{(found.Count > 0 ? found[0].Line : 0)} writes a colour: {(found.Count > 0 ? found[0].Text : "")}");
            }
        Check(TokenLint.Scan("var c = new Color(1, 0, 0);\n// new Color(0, 0, 0)\nvar d = Colors.Red;").Count == 2, "the lint misses a colour literal");
        Check(TokenLint.Words(new[] { "Ring 3 turns by 72°", "r 0142", "F9" }) == 3, "the word count");
    }

    // Focus layers: one pick layer, a cycle for the rest, nothing dimmed or hidden can be picked; a long list builds only what shows.
    private static void A5GalleryFocusLayersAndVirtualRows()
    {
        var layers = new FocusLayers(); int changes = 0; layers.Changed += () => changes++;
        Check(layers.Pick == "cells" && layers.CanPick("cells") && !layers.CanPick("stickers") && layers.Shown("stickers") == 1, "the default layers");
        Check(layers.SetPick("stickers") && !layers.SetPick("stickers") && !layers.SetPick("edges") && changes == 1, "the pick layer");
        Check(layers.Cycle() == InactiveMode.Dimmed && layers.Shown("cells") == .25 && layers.Shown("stickers") == 1 && !layers.CanPick("cells"), "a dimmed layer");
        Check(layers.Cycle() == InactiveMode.Hidden && layers.Shown("cells") == 0 && layers.Shown("cells", true) == 1, "a protection conflict does not break through a hidden layer");
        Check(layers.SetOpacity("stickers", .2) && !layers.CanPick("stickers") && !layers.SetOpacity("stickers", 2), "a faint class can be picked");
        var kept = new FocusLayers(); kept.Restore(layers.Store()); Equal(kept.Store(), layers.Store());
        Check(layers.Cycle() == InactiveMode.Normal, "the cycle does not come back");
        kept.Restore("nonsense"); Equal(kept.Store(), layers.Store().Replace("Normal", "Hidden"));

        (int first, int count) = VirtualRows.Range(10000, 20, 50000, 400);
        Check(first == 2498 && count == 25 && VirtualRows.Range(10000, 20, 0, 400).First == 0 && VirtualRows.Range(0, 20, 0, 400).Count == 0, "the rows in view");
        (int tail, int left) = VirtualRows.Range(10000, 20, 10000 * 20, 400);
        Check(tail + left <= 10000 && left >= 1, "the last rows");
    }
}
