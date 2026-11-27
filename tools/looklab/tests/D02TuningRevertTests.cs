using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Magic600.Chrome;

// CR-16's last two rows (ADR-D01.50, ADR-D02.47): the live selector's revert, which the shell's Esc runs, and the
// difference file with its parse-back.
internal static partial class Program
{
    // The panel as a test reads it back: the appearance, the slots, the preset in force and the presets.
    private static string TuningPanelState(Tuning tuning) => string.Join(" | ", tuning.Look.Store(), tuning.Shown, tuning.Other.Text, tuning.Preset ?? "-",
        string.Join(",", tuning.Presets.Select(p => p.Id + "=" + p.Value.Text)));

    private static string TuningColours(Appearance look) => string.Join(";", look.Resolve().Colours.Select(c => c.Key + "=" + c.Value.Value.Hex));

    // Revert puts back exactly the panel the first change found, after a run of changes across a family, a preset,
    // a slot, a knob and an override; then nothing is pending.
    private static void D02TuningRevertRestoresExactly()
    {
        var look = TuningLook(); var tuning = PresetTuning(look);
        Check(!tuning.Pending && tuning.Before == null, "a selection is pending at open");
        Equal(tuning.Revert(), Tuning.NothingPending, "a revert at open");
        string opening = TuningPanelState(tuning), live = tuning.Live.Text;
        Equal(tuning.ApplyPreset("graphite.inspecting"), null, "a preset");
        Check(tuning.Pending && tuning.Before!.Text == live, "the first change does not remember the tuning before it");
        Equal(tuning.SetKnob("tone", .03), null, "a knob");
        Equal(tuning.Tune("contrast=1.1"), null, "theme.tune");
        Equal(tuning.SetSlot("B"), null, "slot B");
        Equal(tuning.SetFamily(look.Themes.First(t => t.Id != look.Theme).Id), null, "another family");
        Equal(tuning.SetKnob("accentHue", 40), null, "a knob in the other family");
        Equal(tuning.Override("text.secondary=0.4 0.02 250"), null, "an override");
        Check(tuning.Pending && tuning.Before!.Text == live, "the later changes moved the tuning Revert puts back");
        Check(TuningPanelState(tuning) != opening, "the changes changed nothing");
        Equal(tuning.Revert(), null, "revert");
        Equal(TuningPanelState(tuning), opening, "the revert");
        Equal(tuning.Live.Text, live, "the live tuning after the revert");
        Check(!tuning.Pending && tuning.Before == null, "still pending after the revert");
        Equal(tuning.Revert(), Tuning.NothingPending, "a second revert");
        // After a save, a knob moved back to where the selection found it leaves nothing to revert.
        Equal(tuning.SetKnob("tone", .02), null, "tone");
        Equal(tuning.Save("Mine"), null, "save");
        string saved = TuningPanelState(tuning);
        double tone = look.Knob("tone");
        Equal(tuning.SetKnob("tone", .04), null, "tone again");
        Check(tuning.Pending, "a change after the save is not pending");
        Equal(tuning.SetKnob("tone", tone), null, "tone back");
        Check(!tuning.Pending, "a knob moved back still has a change to revert");
        Equal(TuningPanelState(tuning), saved, "the knob moved back");
        // A refused first change starts nothing.
        Check(tuning.Tune("tone=1") != null && !tuning.Pending, "a refused change started a selection");
    }

    // A selection ends, kept, on Save, on Keep (the host: the panel closed or out of view) and when the appearance is
    // changed outside the tuning, which Revert must not undo.
    private static void D02TuningSelectionEndsKept()
    {
        var look = TuningLook(); var tuning = PresetTuning(look);
        Equal(tuning.SetKnob("tone", .03), null, "tone");
        Equal(tuning.Save(), null, "save");
        Check(!tuning.Pending, "pending after a save");
        Equal(tuning.Revert(), Tuning.NothingPending, "a revert after a save");
        Equal(look.Knob("tone"), .03, "the save's tone");
        Equal(tuning.SetKnob("tone", .01), null, "tone again");
        tuning.Keep();
        Check(!tuning.Pending, "pending after Keep");
        Equal(tuning.Revert(), Tuning.NothingPending, "a revert after Keep");
        Equal(look.Knob("tone"), .01, "Keep changed the tone");
        Equal(tuning.SetKnob("tone", .02), null, "a third tone");
        Check(tuning.Pending, "not pending");
        look.SetKnob("contrast", 1.1);
        Check(!tuning.Pending, "pending after a change outside the tuning");
        Equal(tuning.Revert(), Tuning.NothingPending, "a revert after a change outside the tuning");
        Equal((look.Knob("tone"), look.Knob("contrast")), (.02, 1.1), "the change outside the tuning was undone");
    }

    // The revert is refused while the compare is held, as every edit is; after the release it goes through.
    private static void D02TuningRevertRefusedWhileHeld()
    {
        var look = TuningLook(); var tuning = PresetTuning(look);
        string opening = TuningPanelState(tuning);
        Equal(tuning.SetKnob("tone", .03), null, "tone");
        Equal(tuning.SetSlot("B"), null, "slot B");
        Equal(tuning.SetKnob("tone", .05), null, "tone in slot B");
        string store = look.Store();
        Equal(tuning.Hold(), null, "hold");
        Check(tuning.Pending, "the hold ended the selection");
        Equal(tuning.Revert(), "release the held compare first", "a revert while held");
        Equal(tuning.Release(), null, "release");
        Equal(look.Store(), store, "the release");
        Check(tuning.Pending, "the release ended the selection");
        Equal(tuning.Revert(), null, "the revert after the release");
        Equal(TuningPanelState(tuning), opening, "the revert after the release");
        // A held simulation is no selection: it changes no appearance.
        Equal(tuning.HoldSimulation("deutan"), null, "simulate");
        Check(!tuning.Pending, "a simulation started a selection");
        tuning.ReleaseSimulation();
    }

    // A revert moves colours only: the per-piece tint bytes and the legends stay; the tint palette comes back.
    private static void D02TuningRevertTouchesNoState()
    {
        var (style, contact) = TintAt("wj-ia-300");
        int[] classes = ContactClasses.Assign(contact, style.Domains, null).Classes;
        var look = TuningLook(); var tuning = PresetTuning(look);
        TintMode[] modes = { TintMode.ContactClass, TintMode.Height, TintMode.OffLattice };
        var bytes = modes.Select(m => JumbleTint.PerPose(m, style, classes)).ToArray();
        var legends = modes.Select(m => JumbleTint.Line(m, JumbleTint.Legend(m, style, classes))).ToArray();
        string palette = string.Join(";", JumbleTint.Palette(look.Resolve()).Select(e => e.Colour.Hex));
        Equal(tuning.Tune("tone=0.05,contrast=1.1"), null, "theme.tune");
        Equal(tuning.Override(Tuning.ClassSlots[0] + "=0.6 0.1 30"), null, "a class override");
        Check(string.Join(";", JumbleTint.Palette(look.Resolve()).Select(e => e.Colour.Hex)) != palette, "the changes do not move the tint palette");
        Equal(tuning.Revert(), null, "revert");
        Equal(string.Join(";", JumbleTint.Palette(look.Resolve()).Select(e => e.Colour.Hex)), palette, "the tint palette after the revert");
        for (int m = 0; m < modes.Length; m++)
        {
            var (outline, fill) = JumbleTint.PerPose(modes[m], style, classes);
            Check(outline.SequenceEqual(bytes[m].Outline) && fill.SequenceEqual(bytes[m].Fill), "the " + modes[m] + " bytes moved");
            Equal(JumbleTint.Line(modes[m], JumbleTint.Legend(modes[m], style, classes)), legends[m], "the " + modes[m] + " legend");
        }
    }

    // The difference file reads back to the live tuning: from a shipped preset, a fork, a user preset, another family
    // and no preset at all; an empty difference keeps its four lines.
    private static void D02TuningDiffRoundTrips()
    {
        void RoundTrip(Tuning tuning, string preset, string what)
        {
            string text = tuning.Difference();
            string[] lines = text.TrimEnd('\n').Split('\n');
            Check(text.EndsWith("\n") && !text.Contains('\r'), what + ": the file's line ends");
            Equal(lines[0], "format=" + TuningDiff.Format, what + ": the format line");
            Equal(lines[1], "preset=" + preset, what + ": the preset line");
            Equal((lines[2], lines[3]), ("theme=" + tuning.Look.Theme, "scene=" + tuning.Look.SceneLook), what + ": theme and scene");
            TuningDiff diff = TuningDiff.Parse(text);
            TuningValue back = diff.Over(tuning.BaseOf(diff.Preset, diff.Family));
            Check(back.Same(tuning.Live), what + ": read back " + back.Text + " for " + tuning.Live.Text);
            Check(back.Knobs.SequenceEqual(tuning.Live.Knobs) && back.Overrides.SequenceEqual(tuning.Live.Overrides), what + ": the read-back is not exact");
            var fresh = TuningLook(); back.Apply(fresh);
            Check(TuningValue.Of(fresh).Same(tuning.Live), what + ": the read-back applied to a fresh appearance");
            Equal(TuningColours(fresh), TuningColours(tuning.Look), what + ": the read-back's colours");
            Equal(TuningDiff.Write(back, diff.Preset, tuning.BaseOf(diff.Preset, diff.Family)), text, what + ": written again");
        }
        var look = TuningLook(); var tuning = PresetTuning(look);
        RoundTrip(tuning, "graphite", "graphite as shipped");
        Equal(tuning.Difference().TrimEnd('\n').Split('\n').Length, 4, "an empty difference's lines");
        Equal(tuning.ApplyPreset("graphite.inspecting"), null, "inspecting");
        RoundTrip(tuning, "graphite.inspecting", "inspecting as shipped");
        Equal(tuning.Difference().TrimEnd('\n').Split('\n').Length, 4, "the inspecting preset's difference from itself");
        Equal(tuning.ApplyPreset("graphite"), null, "graphite");
        Equal(tuning.SetKnob("tone", .03), null, "tone");
        Equal(tuning.Override("text.secondary=0.4 0.02 250"), null, "an override");
        string forked = tuning.Difference();
        Check(forked.Contains("\nknob.graphite.tone=0.03\n") && forked.Contains("\noverride.graphite.text.secondary=0.4 0.02 250\n"), "the fork's difference: " + forked);
        Check(!forked.Contains("knob.graphite.contrast="), "a knob equal to the preset's is in the difference");
        RoundTrip(tuning, "graphite", "a fork of graphite");
        Equal(tuning.Override("text.secondary=reset"), null, "the override reset");
        Equal(tuning.Save("Mine"), null, "save");
        Equal(tuning.SetKnob("blackLevel", .1), null, "black level");
        RoundTrip(tuning, "graphite", "a user preset from graphite");
        Equal(tuning.SetFamily("paper"), null, "paper");
        Equal(tuning.SetKnob("accentHue", 40), null, "accent hue in paper");
        RoundTrip(tuning, tuning.DifferenceBase?.Id ?? TuningDiff.NoPreset, "another family");
        var bare = new Tuning(TuningLook());
        Equal(bare.SetKnob("gridIntensity", .9), null, "grid intensity without presets");
        RoundTrip(bare, TuningDiff.NoPreset, "no preset");
        // The parse is whole or refused whole.
        string good = tuning.Difference();
        foreach (var (bad, why) in new[]
        {
            (good.Replace("format=" + TuningDiff.Format, "format=magic600-tuning-diff/0"), "another format"),
            (good.Replace("\npreset=", "\nthe-preset="), "no preset line"),
            ("format=" + TuningDiff.Format + "\npreset=\ntheme=graphite\nscene=solving\n", "an empty preset"),
            ("format=" + TuningDiff.Format + "\npreset=none\nknob.graphite.tone=0.01\ntheme=graphite\nscene=solving\n", "a knob before theme="),
            ("format=" + TuningDiff.Format + "\npreset=none\ntheme=graphite\nscene=solving\nknob.paper.tone=0.01\n", "a knob of another family"),
            ("format=" + TuningDiff.Format + "\npreset=none\ntheme=graphite\nscene=solving\nknob.graphite.tone=0.01\nknob.graphite.tone=0.02\n", "a repeated line"),
            ("format=" + TuningDiff.Format + "\npreset=none\ntheme=graphite\nscene=solving\nknob.graphite.tone=lots\n", "a bad number"),
            ("format=" + TuningDiff.Format + "\npreset=none\ntheme=graphite\nscene=solving\noverride.graphite.text.primary=0.4 0.02\n", "a short override"),
            ("format=" + TuningDiff.Format + "\npreset=none\ntheme=graphite\n", "no scene="),
        })
        {
            bool refused = false;
            try { TuningDiff.Parse(bad); } catch (FormatException) { refused = true; }
            Check(refused, why + " is accepted");
        }
        bool unknown = false;
        try { tuning.BaseOf("no.such.preset", "graphite"); } catch (FormatException) { unknown = true; }
        Check(unknown, "an unknown preset is read as a base");
    }

    // The file goes to the host's place (the product shell's run folder); with none, or one that is missing, the row
    // refuses with a clear reason and writes nothing; while the compare is held it is refused.
    private static void D02TuningExportPlace()
    {
        var look = TuningLook(); var tuning = PresetTuning(look);
        Equal(tuning.SetKnob("tone", .05), null, "tone");
        Equal(tuning.Export(null), Tuning.NoPlace, "an export without a place");
        Equal(tuning.Export(""), Tuning.NoPlace, "an export to an empty place");
        using var folder = new TestFolder();
        string missing = Path.Combine(folder.Path, "missing");
        Equal(tuning.Export(missing), Tuning.NoPlace, "an export to a missing folder");
        Check(!Directory.Exists(missing), "the export made the missing folder");
        Equal(tuning.ExportLine, "", "a refused export has a line");
        Equal(tuning.Hold(), null, "hold");
        Equal(tuning.Export(folder.Path), "release the held compare first", "an export while held");
        Equal(tuning.Release(), null, "release");
        Check(!File.Exists(Path.Combine(folder.Path, Tuning.DiffFile)), "a refused export wrote the file");
        Equal(tuning.Export(folder.Path), null, "the export");
        byte[] written = File.ReadAllBytes(Path.Combine(folder.Path, Tuning.DiffFile));
        Check(written.Length > 3 && !(written[0] == 0xEF && written[1] == 0xBB) && !written.Contains((byte)'\r'), "the file has a byte order mark or a CR");
        Equal(System.Text.Encoding.UTF8.GetString(written), tuning.Difference(), "the file's text");
        Check(tuning.ExportLine.StartsWith("Exported " + Tuning.DiffFile + " to the run folder: the difference from Graphite") && tuning.ExportLine.Contains("1 knob, no override"),
            "the export line: " + tuning.ExportLine);
        Check(tuning.Pending, "the export ended the selection");
    }

    // The two rows are wired: registered as draft rows, run by the panel, Esc through the shell's transient.close
    // (an open overlay first, then the revert, then the other transients), and no key of their own.
    private static void D02TuningRevertRowsWired()
    {
        CommandRegistry registry = LabRegistry(); registry.AddRows(Read("data/design/command-rows-d02.json"));
        Check(new[] { "tuning.selector.revert", "tuning.diff.export" }.All(registry.Commands.ContainsKey), "a CR-16 row is not registered");
        string panel = Read("app/Chrome/Ui/TuningPanel.cs");
        Check(panel.Contains("\"tuning.selector.revert\", \"tuning.diff.export\" });"), "the panel's rows do not name the two rows");
        Check(panel.Contains("case \"tuning.selector.revert\": Edit(Model.Revert,") && panel.Contains("case \"tuning.diff.export\": Export();"), "the panel does not run the two rows");
        Check(panel.Contains("frame.Model.Changed += () => { if (Model.Pending && !InView) Model.Keep();"), "the panel does not keep a pending change when it leaves view");
        string shell = File.ReadAllText(Path.Combine(Root, "renderer/shell/project/ShellChrome.cs"));
        int close = shell.IndexOf("case \"transient.close\":", StringComparison.Ordinal);
        Check(close >= 0, "the shell has no transient.close");
        int end = shell.IndexOf("break;", close, StringComparison.Ordinal);
        string block = shell.Substring(close, end - close);
        int overlay = block.IndexOf("open.Close()", StringComparison.Ordinal), revert = block.IndexOf("Tuning.Escape()", StringComparison.Ordinal),
            feedback = block.IndexOf("CancelLatest()", StringComparison.Ordinal), focus = block.IndexOf("SetFocusMode(false)", StringComparison.Ordinal);
        Check(overlay >= 0 && overlay < revert && revert < feedback && feedback < focus, "Esc's order in transient.close: " + string.Join(",", overlay, revert, feedback, focus));
        Check(shell.Contains("ExportFolder = !string.IsNullOrEmpty(runFolder) && Directory.Exists(runFolder) ? runFolder : null"), "the shell gives the panel no run folder");
        string keymap = Read("data/design/keymap.json");
        Check(!keymap.Contains("tuning.selector.revert") && !keymap.Contains("tuning.diff.export"), "a CR-16 row has a key of its own");
    }
}
