using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

// ADR-D02 build, track C, item 3 (ADR-D01.14): the layout history, shared by Look Lab and the product.
internal static partial class Program
{
    // A stand-in arrangement: the five FrameLayout parts, which the tracker reads and puts back.
    private sealed class Arrangement
    {
        public string[] Parts = { "theme=graphite\ndensity=compact", "solve", "panes:a", "on", "keys:1" };
        public int Flushed;
        public LayoutTracker Tracker(LayoutHistory? layout = null)
        {
            var tracker = new LayoutTracker(id => id == "solve" ? "Solve" : id == "script" ? "Script" : id, () => Flushed++, layout);
            tracker.Track(() => Parts.ToArray(), parts => Parts = parts.ToArray());
            return tracker;
        }
    }

    // Each change is one step named by the first part that changed; a knob dragged is one step; undo, redo and roll put
    // the arrangement back and record nothing; a named run is one step; the refusals say why.
    private static void D02PortLayoutTrackerNamesAndRestoresSteps()
    {
        var room = new Arrangement(); var shared = new LayoutHistory();
        LayoutTracker tracker = room.Tracker(shared);
        Check(ReferenceEquals(tracker.Layout, shared) && tracker.Tracking && shared.Steps.Select(s => s.Name).SequenceEqual(new[] { "Opened" }), "the history does not start Opened on the host's history");
        room.Parts[1] = "script"; tracker.Record();
        room.Parts[0] = "theme=paper\ndensity=compact"; tracker.Record();
        room.Parts[0] = "theme=paper\ndensity=compact\nknob.paper.tone=0.1"; tracker.Record();
        room.Parts[0] = "theme=paper\ndensity=compact\nknob.paper.tone=0.2"; tracker.Record();
        room.Parts[2] = "panes:b"; tracker.Record();
        room.Parts[4] = "keys:2"; tracker.Record();
        tracker.Record();
        Sequence(shared.Steps.Select(s => s.Name), new[] { "Opened", "Workspace: Script", "Appearance: theme paper", "Appearance: knob.paper.tone 0.2", "Arrange panes", "Key bindings" }, "the steps are " + string.Join(" | ", shared.Steps.Select(s => s.Name)));
        int restored = 0; tracker.Restored += () => restored++;
        Check(tracker.Execute("layout.undo", null) == null && room.Parts[4] == "keys:1" && room.Parts[2] == "panes:b" && room.Flushed == 1, "undo did not put the key bindings back");
        // The rebuild after a restore records nothing.
        tracker.Record();
        Check(tracker.Execute("layout.roll", "1") == null && room.Parts[1] == "script" && room.Parts[0] == "theme=graphite\ndensity=compact" && shared.Cursor == 1, "roll did not go to step 1");
        Check(tracker.Execute("layout.redo", null) == null && room.Parts[0] == "theme=paper\ndensity=compact" && shared.Steps.Count == 6 && restored == 3, "redo did not go forward");
        Check(tracker.Execute("layout.roll", "2") == "The layout history is already there" && tracker.Execute("layout.roll", null) == "Roll the layout: it needs the number of a layout step", "a roll in place was not refused");
        tracker.Execute("layout.roll", "0");
        Check(tracker.Execute("layout.undo", null) == "No layout change to undo", "an undo past the start was not refused");
        // A named run, made by a script, is one step and drops the redo steps.
        tracker.Run("Script: stacked", () => { room.Parts[2] = "panes:stacked"; room.Parts[1] = "solve"; });
        Check(shared.Steps.Count == 2 && shared.Steps[^1].Name == "Script: stacked" && tracker.Execute("layout.redo", null) == "No layout change to redo", "the named run is not one step");
        Check(LayoutTracker.Handles("layout.undo") && LayoutTracker.Handles("layout.redo") && LayoutTracker.Handles("layout.roll") && !LayoutTracker.Handles("session.undo"), "the tracker's commands are wrong");
        // A move never moves it: a host that is not tracking records nothing.
        var idle = new LayoutTracker(id => id, () => { });
        idle.Record(); Check(!idle.Tracking && idle.Layout.Steps.Count == 0 && idle.Execute("layout.undo", null) != null, "an idle tracker recorded");
    }

    // The appearance is put back in place, so a host that holds its Appearance keeps it; a bad stored text changes nothing.
    private static void D02PortAppearanceRestoresInPlace()
    {
        ThemeFamily[] themes = Themes();
        var look = new Appearance(Design, themes);
        string before = look.Store();
        look.SetTheme(themes[^1].Id); look.SetDensity("comfortable"); look.SetKnob("accentHue", 20); look.HighContrast = true; look.ReducedMotion = true;
        string after = look.Store();
        look.Restore(before);
        Equal(look.Store(), before, "the appearance was not put back");
        look.Restore(after);
        Equal(look.Store(), after, "the tuned appearance was not put back");
        Refuse(() => look.Restore("theme=nowhere"), "nowhere");
        Equal(look.Store(), after, "a refused text changed the appearance");
    }
}
