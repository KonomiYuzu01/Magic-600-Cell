using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    // ADR-D01.20: a radial menu held on its key. Release runs the entry the pointer points at, release in the hub runs
    // nothing, a short tap leaves the menu open for a click or a number, and the hub says what release would run.
    private static void D01RadialHeldReleaseRunsThePointedEntry()
    {
        // Four entries: up, right, down, left, the ring flattened to 0.8.
        var four = Enumerable.Range(0, 4).Select(i => RadialHold.Direction(i, 4)).ToArray();
        Check(Math.Abs(four[0].X) < 1e-9 && Math.Abs(four[0].Y + .8) < 1e-9, "the first entry sits straight up");
        Check(Math.Abs(four[1].X - 1) < 1e-9 && Math.Abs(four[1].Y) < 1e-9, "the second entry sits to the right");
        Equal(RadialHold.Pointed(0, -60, four, 40), 0, "pointing up");
        Equal(RadialHold.Pointed(60, 0, four, 40), 1, "pointing right");
        Equal(RadialHold.Pointed(0, 60, four, 40), 2, "pointing down");
        Equal(RadialHold.Pointed(-60, 0, four, 40), 3, "pointing left");
        Equal(RadialHold.Pointed(50, -20, four, 40), 1, "a pointer nearer right than up");
        Equal(RadialHold.Pointed(10, -60, four, 40), 0, "a pointer nearer up than right");
        Equal(RadialHold.Pointed(20, 20, four, 40), -1, "inside the hub's dead zone");
        Equal(RadialHold.Pointed(0, -40, four, 40), -1, "on the dead zone's edge");
        Equal(RadialHold.Pointed(0, -60, Array.Empty<(double, double)>(), 40), -1, "no entries");
        // Five entries, as the workspace menu: each entry's own direction points at it.
        var five = Enumerable.Range(0, 5).Select(i => RadialHold.Direction(i, 5)).ToArray();
        for (int i = 0; i < 5; i++) Equal(RadialHold.Pointed(five[i].X * 200, five[i].Y * 200, five, 42), i, "entry " + (i + 1) + " of five");
        // Release: a tap stays open; a hold runs the pointed entry, or nothing in the hub.
        Equal(RadialHold.Release(.1, 2), RadialRelease.StayOpen, "a tap over an entry");
        Equal(RadialHold.Release(.1, -1), RadialRelease.StayOpen, "a tap in the hub");
        Equal(RadialHold.Release(RadialHold.TapSeconds, 2), RadialRelease.Run, "a hold of the tap's length");
        Equal(RadialHold.Release(.8, 0), RadialRelease.Run, "a hold over the first entry");
        Equal(RadialHold.Release(.8, -1), RadialRelease.Nothing, "a hold released in the hub");
        // The hub's words while held, and the status line after a release in the hub.
        Equal(RadialHold.Hint(null, null), "Release here: nothing runs", "the hub's line in the hub");
        Equal(RadialHold.Hint("Workspace: Analyse", null), "Release runs Workspace: Analyse", "the hub's line over an entry");
        Equal(RadialHold.Hint("Reset view", "not here"), "Release runs Reset view (unavailable: not here)", "an unavailable entry says why");
        Equal(RadialHold.Let("Workspaces"), "Workspaces: released in the middle, nothing ran", "a release in the hub");
        // Every radial of the keymap is a command bound to a key, so each one can be held.
        CommandRegistry registry = LabRegistry();
        Keymap keys = Keymap.Parse(Read("data/design/keymap.json"), registry);
        foreach (string radial in keys.Radials.Keys)
            Check(keys.Bindings.Any(b => b.Command == radial), radial + " has no key to hold");
    }
}
