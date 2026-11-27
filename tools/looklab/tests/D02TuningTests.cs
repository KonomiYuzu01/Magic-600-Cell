using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Magic600.Chrome;

// B30 (ADR-D02.47, first half): the tuning panel's model: where it opens, that a knob moves colours only, and the
// compare's Swap and held compare.
internal static partial class Program
{
    private static Appearance TuningLook() => new(Design, Themes());

    // A frame registered as the product shell registers its docks with D02 on.
    private static FrameModel TuningFrame()
    {
        var frame = new FrameModel();
        foreach (string id in new[] { "library", "scripts", "tokens" }) frame.AddDockPanel(DockSide.Left, id);
        foreach (string id in new[] { "history", "inspector", "slot.1", "slot.2", "tune", "appearance", Tuning.PanelId }) frame.AddDockPanel(DockSide.Right, id);
        foreach (string id in new[] { "problems", "console", "tasks" }) frame.AddDockPanel(DockSide.Bottom, id);
        return frame;
    }

    // In every shipped layout the palette's row opens the panel as a tab of the right dock, beside History, and
    // leaves the main pair and the other docks as the layout has them, so the views being judged stay visible.
    private static void D02TuningOpensInRightDockOfCurrentLayout()
    {
        WorkspaceSet shipped = WorkspaceSet.Load(FileAt("data/design/workspaces.json"));
        Check(shipped.Stored.Count >= 3, "too few shipped layouts");
        foreach (var (id, stored) in shipped.Stored)
        {
            FrameModel frame = TuningFrame();
            Check(frame.Apply(stored), id + ": the layout does not apply");
            string? left = frame.Docks[DockSide.Left].Open, bottom = frame.Docks[DockSide.Bottom].Open;
            string centre = string.Join(",", frame.Centre.Panes());
            Check(Tuning.Open(frame), id + ": the tuning panel does not open");
            Equal(frame.Docks[DockSide.Right].Open, Tuning.PanelId, id + ": the right dock's open panel");
            Check(frame.Docks[DockSide.Right].Panels.Contains("history"), id + ": History is not in the right dock beside it");
            Check(frame.Docks[DockSide.Left].Open == left && frame.Docks[DockSide.Bottom].Open == bottom, id + ": another dock changed");
            Equal(string.Join(",", frame.Centre.Panes()), centre, id + ": the centre changed");
            Check(frame.Centre.Panes().Contains(FrameModel.Pair) && !frame.FocusMode && !frame.PairMaximised || id != "solve", id + ": the main pair is hidden");
        }
        FrameModel solve = TuningFrame(); solve.Apply(shipped.Stored["solve"]);
        Equal(solve.Docks[DockSide.Right].Open, "history", "Solve's right dock at open");
        Tuning.Open(solve);
        Equal(solve.Docks[DockSide.Right].Open, Tuning.PanelId, "Solve's right dock after the row");
        // The rows: the panel's own draft row opens it; the compare rows and the table's theme.tune reach the panel.
        CommandRegistry registry = LabRegistry(); registry.AddRows(Read("data/design/command-rows-d02.json"));
        Check(new[] { "tuning.open", "tuning.slot.set", "tuning.slot.swap", "tuning.compare.hold", "theme.tune" }.All(registry.Commands.ContainsKey), "a tuning row is not registered");
        Check(IconSet.Parse(Read("data/design/icons.json")).Icons.ContainsKey(Tuning.Glyph), "the panel's glyph has no icon");
    }

    // A knob changes colours only: the per-piece tint bytes and the legend stay as they were, the tint palette and the
    // frame's colours move, and the model holds nothing but the appearance and its slots.
    private static void D02KnobChangeTouchesNoState()
    {
        var (style, contact) = TintAt("wj-ia-300");
        int[] classes = ContactClasses.Assign(contact, style.Domains, null).Classes;
        var look = TuningLook(); var tuning = new Tuning(look);
        TintMode[] modes = { TintMode.ContactClass, TintMode.Height, TintMode.OffLattice };
        var bytes = modes.Select(m => JumbleTint.PerPose(m, style, classes)).ToArray();
        var legends = modes.Select(m => JumbleTint.Line(m, JumbleTint.Legend(m, style, classes))).ToArray();
        string palette = string.Join(";", JumbleTint.Palette(look.Resolve()).Select(e => e.Colour.Hex));
        string window = look.Resolve().Colours["surface.window"].Value.Hex;
        foreach (string id in ThemeFamily.KnobIds)
        {
            Knob knob = look.Family.Knobs[id];
            foreach (double value in new[] { knob.Min, knob.Max, knob.Default })
            {
                Equal(tuning.SetKnob(id, value), null, id + " = " + value);
                for (int m = 0; m < modes.Length; m++)
                {
                    var (outline, fill) = JumbleTint.PerPose(modes[m], style, classes);
                    Check(outline.SequenceEqual(bytes[m].Outline) && fill.SequenceEqual(bytes[m].Fill), id + ": the " + modes[m] + " bytes moved");
                    Equal(JumbleTint.Line(modes[m], JumbleTint.Legend(modes[m], style, classes)), legends[m], id + ": the " + modes[m] + " legend");
                }
            }
        }
        // Tone moves the tint's ramps and the frame; accent hue and grid intensity move the frame's accent and grid only.
        Equal(tuning.Tune("tone=0.05"), null, "theme.tune tone");
        Check(string.Join(";", JumbleTint.Palette(look.Resolve()).Select(e => e.Colour.Hex)) != palette, "tone does not move the tint palette");
        Check(look.Resolve().Colours["surface.window"].Value.Hex != window, "tone does not move the frame");
        Check(tuning.Tune("tone=1") != null && tuning.Tune("hue=3") != null && tuning.Tune("") != null, "a bad theme.tune is accepted");
        Equal(look.Knob("tone"), 0.05, "a refused theme.tune changed a knob");
        // Each strip is live: twelve steps, moved by its knob.
        foreach (string id in ThemeFamily.KnobIds)
        {
            string before = string.Join(";", tuning.Strip(id).Steps.Select(c => c.Hex));
            Equal(tuning.Strip(id).Steps.Count, ThemeFamily.Steps, id + "'s strip");
            Knob knob = look.Family.Knobs[id];
            tuning.SetKnob(id, look.Knob(id) == knob.Max ? knob.Min : knob.Max);
            string after = string.Join(";", tuning.Strip(id).Steps.Select(c => c.Hex));
            Check(before != after || id == "gridIntensity", id + "'s strip does not follow its knob");
        }
        Check(tuning.Strip("gridIntensity").Mark is int mark && mark is >= 0 and < ThemeFamily.Steps, "grid intensity's strip marks no step");
        // No field of the model names a session, a puzzle state or the frame.
        Check(typeof(Tuning).GetFields(BindingFlags.Instance | BindingFlags.NonPublic | BindingFlags.Public)
            .All(f => f.FieldType == typeof(Appearance) || f.FieldType == typeof(TuningValue) || f.FieldType == typeof(string) || f.FieldType == typeof(bool)
                || f.FieldType == typeof(List<TuningPreset>) || f.FieldType == typeof(IReadOnlyList<string>)),
            "the tuning model holds more than the appearance and its slots");
    }

    // Swap shows the other slot; twice is no change, for one family and across families.
    private static void D02SwapIsAnInvolution()
    {
        foreach (bool across in new[] { false, true })
        {
            var look = TuningLook(); var tuning = new Tuning(look);
            Equal(tuning.SetKnob("tone", .03), null, "tone");
            Equal(tuning.SetSlot("B"), null, "set B");
            if (across) Equal(tuning.SetFamily(look.Themes.First(t => t.Id != look.Theme).Id), null, "another family");
            Equal(tuning.SetKnob("contrast", 1.1), null, "contrast");
            Equal(tuning.SetSlot("C"), "it needs the slot A or B", "slot C");
            string store = look.Store(), live = tuning.Live.Text, other = tuning.Other.Text, shown = tuning.Shown;
            Check(live != other, "the two slots are the same");
            Equal(tuning.Swap(), null, "swap");
            Equal(tuning.Live.Text, other, "swap shows the other slot");
            Equal(tuning.Other.Text, live, "swap keeps the live tuning in the slot it leaves");
            Check(tuning.Shown != shown, "swap shows the same slot");
            Equal(tuning.Swap(), null, "swap again");
            Equal((tuning.Shown, tuning.Live.Text, tuning.Other.Text), (shown, live, other), "two swaps");
            if (!across) Equal(look.Store(), store, "two swaps changed the appearance");
        }
        // At open both slots hold the tuning in force: a swap shows the opening tuning after a knob moved.
        var fresh = TuningLook(); var start = new Tuning(fresh); string opening = start.Live.Text;
        start.SetKnob("blackLevel", .1); start.Swap();
        Equal(start.Live.Text, opening, "a swap at open shows the opening tuning");
    }

    // The held compare draws the other slot while held, refuses edits meanwhile, and puts the appearance back on release.
    private static void D02HeldCompareRestoresOnRelease()
    {
        foreach (bool across in new[] { false, true })
        {
            var look = TuningLook(); var tuning = new Tuning(look);
            if (across) tuning.SetFamily(look.Themes.First(t => t.Id != look.Theme).Id);
            tuning.SetKnob("accentHue", 40); tuning.SetSlot("B");
            tuning.SetKnob("accentHue", 300); tuning.SetKnob("gridIntensity", .9);
            look.HighContrast = true;
            string store = look.Store(), live = tuning.Live.Text, other = tuning.Other.Text;
            Equal(tuning.Hold(), null, "hold");
            Check(tuning.Holding, "not holding");
            Equal(tuning.Live.Text, other, "the hold draws the other slot");
            Equal(tuning.Slot(tuning.Shown).Text, live, "the shown slot while held");
            Check(tuning.SetKnob("tone", .02) != null && tuning.Swap() != null && tuning.SetSlot("B") != null && tuning.Tune("tone=0.02") != null, "an edit is accepted while held");
            Equal(tuning.Hold(), null, "a second hold");
            Equal(tuning.Release(), null, "release");
            Check(!tuning.Holding, "still holding");
            Equal(look.Store(), store, "the release does not put the appearance back");
            Equal((tuning.Live.Text, tuning.Other.Text), (live, other), "the slots after the release");
            Equal(tuning.Release(), null, "a second release");
            Equal(look.Store(), store, "a second release changed the appearance");
        }
    }

    // B31 (ADR-D02.47): presets, Advanced, the simulation and Save's checks.
    private static Tuning PresetTuning(Appearance look) => new(look, TuningPreset.Parse(Read("data/design/tuning-presets.json"), look));
    private static string L(double v) => v.ToString("R", System.Globalization.CultureInfo.InvariantCulture);

    // Both families pass Save as shipped, and so does every shipped preset.
    private static void D02BothThemesPassSaveAsShipped()
    {
        foreach (ThemeFamily family in TuningLook().Themes)
        {
            var look = TuningLook(); look.SetTheme(family.Id);
            var issues = Tuning.SaveIssues(look.Resolve());
            Check(issues.Count == 0, family.Id + " as shipped: " + string.Join("; ", issues));
            Equal(new Tuning(look).Save("Mine"), null, family.Id + ": Save as shipped");
        }
        var shipped = TuningLook(); var tuning = PresetTuning(shipped);
        Check(tuning.Presets.Count >= 2 && tuning.Presets.All(p => p.Shipped), "the shipped presets");
        Check(shipped.Themes.All(t => tuning.Presets.Any(p => p.Value.Family == t.Id)), "a family has no shipped preset");
        Equal(tuning.Preset, "graphite", "the preset in force at open");
        foreach (TuningPreset p in tuning.Presets.ToArray())
        {
            Equal(tuning.ApplyPreset(p.Id), null, p.Id + ": apply");
            Check(tuning.Live.Same(p.Value), p.Id + ": the live tuning is not the preset");
            var issues = Tuning.SaveIssues(shipped.Resolve());
            Check(issues.Count == 0, p.Id + ": " + string.Join("; ", issues));
        }
    }

    // An edit to a shipped preset forks it; Save fills the fork; the shipped preset is never written.
    private static void D02ShippedPresetForksOnEdit()
    {
        var look = TuningLook(); var tuning = PresetTuning(look);
        Equal(tuning.ApplyPreset("graphite"), null, "apply");
        string shipped = tuning.Current!.Value.Text;
        Equal(tuning.SetKnob("tone", .03), null, "a knob");
        TuningPreset fork = tuning.Current!;
        Check(!fork.Shipped && fork.From == "graphite" && fork.Id != "graphite", "the edit did not fork: " + fork.Id);
        Check(tuning.Edited && fork.Value.Text == shipped, "the fork's base is not the shipped tuning");
        Equal(tuning.Presets.First(p => p.Id == "graphite").Value.Text, shipped, "the shipped preset changed on edit");
        Check(tuning.Save("Graphite") != null, "a save under the shipped name was accepted");
        Equal(tuning.Save(), null, "save");
        Check(!tuning.Edited && fork.Value.Same(tuning.Live) && tuning.Preset == fork.Id, "the fork does not hold the saved tuning");
        Equal(tuning.Presets.First(p => p.Id == "graphite").Value.Text, shipped, "the shipped preset changed on save");
        Equal(tuning.SetKnob("tone", .01), null, "a second knob");
        Equal(tuning.Preset, fork.Id, "a user preset forked again");
        Equal(tuning.ApplyPreset("graphite"), null, "back to the shipped preset");
        Equal(tuning.Live.Text, shipped, "the shipped preset does not draw as shipped");
        Equal(tuning.ApplyPreset(fork.Id), null, "the fork");
        Equal(look.Knob("tone"), .03, "the fork's saved tone");
        // An advanced override forks a shipped preset too.
        Equal(tuning.ApplyPreset("paper"), null, "paper");
        Equal(tuning.Override("text.secondary=0.4 0.02 250"), null, "an override");
        Check(tuning.Current is { Shipped: false, From: "paper" } && tuning.Advanced, "the override did not fork paper or show Advanced");
        Check(tuning.AdvancedRoles("jumble.height").SequenceEqual(Tuning.HeightSlots), "Advanced's search: " + string.Join(",", tuning.AdvancedRoles("jumble.height")));
        Equal(tuning.Override("text.secondary=reset"), null, "reset");
        Check(look.Overrides.Count == 0, "the reset left an override");
    }

    // A height ramp that does not step one way is refused, naming the pair; nothing is stored.
    private static void D02SaveFailsOnNonMonotoneHeightRamp()
    {
        var look = TuningLook(); var tuning = PresetTuning(look);
        Rgb shipped = look.Resolve().Colour(Tuning.HeightSlots[2]);
        double first = ColourMetric.Oklab(look.Resolve().Colour(Tuning.HeightSlots[0])).L;
        Equal(tuning.Override(Tuning.HeightSlots[2] + "=" + L(first) + " 0 0"), null, "the override");
        Check(look.Resolve().Colour(Tuning.HeightSlots[2]) != shipped, "the frame's tokens ignore the override");
        int count = tuning.Presets.Count; string? before = tuning.Preset; string saved = tuning.Current!.Value.Text;
        string? refused = tuning.Save();
        Check(refused != null && refused.Contains(Tuning.HeightSlots[1] + " (L") && refused.Contains("then " + Tuning.HeightSlots[2]) && refused.Contains("not monotone"), "the refusal: " + refused);
        Check(tuning.Presets.Count == count && tuning.Preset == before && tuning.Current!.Value.Text == saved, "a refused save stored something");
        Equal(tuning.Override(Tuning.HeightSlots[2] + "=reset"), null, "reset");
        Equal(tuning.Save(), null, "the save after the reset");
    }

    // Two contact classes apart in normal vision but not under one of G4's simulations are refused, naming the pair.
    private static void D02SaveFailsOnCollidingClassesUnderSimulation()
    {
        var a = new Oklch(.65, .14, 40); Rgb ra = a.ToSrgb();
        (Oklch B, string Vision)? found = null;
        for (int hue = 0; hue < 360 && found == null; hue += 5)
        {
            var b = new Oklch(.65, .14, hue); Rgb rb = b.ToSrgb();
            if (ColourMetric.Distance(ra, rb) < StickerPalette.Needed) continue;
            foreach (string vision in ColourMetric.Visions.Skip(1))
                if (ColourMetric.Distance(ColourMetric.Simulate(ra, vision), ColourMetric.Simulate(rb, vision)) < StickerPalette.NeededSimulated) { found = (b, vision); break; }
        }
        Check(found != null, "no pair collides under a simulation only");
        var look = TuningLook(); var tuning = PresetTuning(look);
        var (bb, seen) = found!.Value;
        Equal(tuning.Override(Tuning.ClassSlots[0] + $"={L(a.L)} {L(a.C)} {L(a.H)}"), null, "class 1");
        Equal(tuning.Override(Tuning.ClassSlots[1] + $"={L(bb.L)} {L(bb.C)} {L(bb.H)}"), null, "class 2");
        string? refused = tuning.Save();
        Check(refused != null && tuning.Issues.Any(i => i.StartsWith($"contact classes 1 and 2 under {seen}:")), "the refusal: " + refused + " | " + string.Join("; ", tuning.Issues));
        Check(!tuning.Issues.Any(i => i.StartsWith("contact classes 1 and 2 under normal")), "the pair collides in normal vision too");
    }

    // A pair below its contrast rule is refused, naming the pair.
    private static void D02SaveFailsOnContrastRule()
    {
        var look = TuningLook(); var tuning = PresetTuning(look);
        var (l, a, b) = ColourMetric.Oklab(look.Resolve().Colour("surface.window"));
        double c = Math.Sqrt(a * a + b * b), h = (Math.Atan2(b, a) * 180 / Math.PI + 360) % 360;
        Equal(tuning.Override($"text.primary={L(l)} {L(Math.Min(c, .4))} {L(h)}"), null, "the override");
        string? refused = tuning.Save("Dark text");
        Check(refused != null && refused.StartsWith("not saved: contrast text.primary on surface.window:"), "the refusal: " + refused);
        Check(tuning.Presets.All(p => p.Name != "Dark text"), "a refused save made a preset");
    }

    // The held simulation is one matrix (identity when released) and changes no appearance; an explicit open reveals
    // the folded right dock, and a rail press does too.
    private static void D02SimulationAndRevealedDock()
    {
        var look = TuningLook(); var tuning = new Tuning(look); string store = look.Store();
        Check(tuning.Matrix.SequenceEqual(new double[] { 1, 0, 0, 0, 1, 0, 0, 0, 1 }), "the matrix at rest");
        Equal(tuning.HoldSimulation("protan"), null, "hold protan");
        Check(tuning.Matrix.SequenceEqual(ColourMetric.SimulationMatrix("protan")) && tuning.Simulating == "protan", "the held matrix");
        Rgb red = new(1, 0, 0);
        Equal(ColourMetric.Apply(red, tuning.Matrix).Hex, ColourMetric.Simulate(red, "protan").Hex, "the chrome's matrix is not the simulation's");
        Check(tuning.HoldSimulation("sepia") != null, "an unknown simulation was held");
        tuning.ReleaseSimulation();
        Check(tuning.Simulating == null && tuning.Matrix[0] == 1 && tuning.Matrix[1] == 0, "the release");
        Equal(tuning.HoldSimulation(null), null, "hold the chosen one");
        Equal(tuning.Simulating, "protan", "the chosen simulation");
        Equal(look.Store(), store, "a simulation changed the appearance");
        FrameModel frame = TuningFrame(); frame.Apply(WorkspaceSet.Load(FileAt("data/design/workspaces.json")).Stored["solve"]);
        Check(Tuning.Open(frame, true) && frame.Docks[DockSide.Right].Revealed == Tuning.PanelId, "the folded open is not revealed");
        Check(frame.PressRail(DockSide.Right, "history", true) && frame.Docks[DockSide.Right].Open == "history" && frame.Docks[DockSide.Right].Revealed == "history", "a folded rail press");
        Check(frame.PressRail(DockSide.Right, "history", true) && frame.Docks[DockSide.Right].Open == null && frame.Docks[DockSide.Right].Revealed == null, "a second press closes");
        Check(Tuning.Open(frame) && frame.Docks[DockSide.Right].Revealed == null, "a wide open marks a reveal");
    }
}
