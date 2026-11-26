using System;
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
            .All(f => f.FieldType == typeof(Appearance) || f.FieldType == typeof(TuningValue) || f.FieldType == typeof(string) || f.FieldType == typeof(bool)),
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
}
