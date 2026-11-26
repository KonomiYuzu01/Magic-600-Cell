using System;
using System.Collections.Generic;

namespace Magic600.Chrome;

// A mark a view's border carries: its glyph, its words, its colour role and its tooltip (ViewFrame.AddMark draws it).
public sealed record ViewMark(string Glyph, string Words, string Colour, string Tooltip);

// What the views say while Jumble is on and their data waits (build step B27). Global keeps 600-cell-Full's projection
// chain until the render contract's R5 admits a chart, and its border names that chain (ADR-D02.8). Until the
// exposed-face geometry exists, nothing is drawn in a hole, and every view's border and Tier 1 say so with the reason's
// code (ADR-D02.25).
public static class JumbleViews
{
    public const string Puzzle = "600-cell-Full";
    public const string ProjectionChain = "projection chain (" + Puzzle + ")";
    public const string ChainGlyph = "mark.link";
    public const string ExposedUnavailable = "exposed faces unavailable";
    public const string ExposedReason = "input.exposed_geometry_unavailable";
    public const string ExposedGlyph = "state.notdrawn";
    public const string ExposedRole = "state.exposed_face.unavailable";
    public const string GlobalView = "Global";

    // This build has no certified exposed-face geometry (ADR-D02.25's [data]); the build that ships it turns this on.
    public static bool ExposedGeometry => false;

    public static string? Chain(JumbleState state, string view) => state.Mode && view == GlobalView ? ProjectionChain : null;

    public static string? Exposed(JumbleState state) => state.Mode && !ExposedGeometry ? ExposedUnavailable : null;

    // Global's chain first, then the exposed faces, on each view while Jumble is on.
    public static IReadOnlyList<ViewMark> BorderMarks(JumbleState state, string view)
    {
        var marks = new List<ViewMark>();
        if (Chain(state, view) is { } chain)
            marks.Add(new ViewMark(ChainGlyph, chain, "text.dim", "Global keeps " + Puzzle + "'s projection chain until the render contract admits a chart"));
        if (Exposed(state) is { } exposed)
            marks.Add(new ViewMark(ExposedGlyph, exposed, ExposedRole, "Nothing is drawn in a hole: the exposed-face geometry does not exist yet (" + ExposedReason + ")"));
        return marks;
    }

    // A Tier 1 line with the unavailable state at its end, while Jumble is on; an empty line stays empty.
    public static string Tier1(string line, JumbleState state) => line.Length > 0 && Exposed(state) is { } words ? line + " · " + words : line;
}

// ADR-D02.17 (build step B20): the cut slice, Local's overlay. Local draws the outside layer filled and the inside
// layer (the cap's inside set) as outlines at a smaller shrink, clipped to a radius shell about the cap's pole; Global
// keeps its chain or chart. A held key splits Local into two linked copies of one view: the left draws the outside
// layer, the right the inside layer, with the same camera, scale and shell; release returns to one pane. A second
// held key previews the cap's twist as a ghost of the inside layer (pattern.preview) and names the twist on release
// only. A blocked cap has no ghost and no cut-cell state, only its straddlers (B16's trace) with their point pairs.
public sealed record SliceShell(double Min, double Max);

// One drawn pane of Local under the slice: its opening and what the core draws there.
public sealed record SlicePane(PaneSpec Pane, SliceLayer Layer, SliceShell Shell, double Shrink);

public enum SliceLayer { Both = 1, Outside = 2, Inside = 3 }

// The twist a released preview names: the cap, the menu item and the certified angle.
public sealed record NamedTwist(int Cap, int Item, double Angle);

public sealed class CutSlice
{
    // The inside layer's shrink factor (times the view's sticker shrink) and the shell's default and step, in radians
    // about the pole. The cut of a cap lies at about 0.25 rad (121/125); the default shell shows one layer beyond it.
    public const double Shrink = 0.6, ShellStep = 0.05, ShellLimit = Math.PI;
    public static readonly SliceShell DefaultShell = new(0, 0.6);

    // The held keys, placeholders free in the keymap (row requests: CR-14's held split and the turn preview; S is
    // seat 4 and G view.global there).
    public const string SplitKey = "Backslash", PreviewKey = "Down";

    public bool Split { get; private set; }
    public SliceShell Shell { get; private set; } = DefaultShell;
    public NamedTwist? Previewing { get; private set; }
    public NamedTwist? Named { get; private set; }
    public string? Refusal { get; private set; }

    public void HoldSplit() => Split = true;
    public void ReleaseSplit() => Split = false;

    // The shell control: widen or narrow its outer radius, never below its inner one nor past pi.
    public SliceShell Widen(int steps)
    {
        var max = Math.Clamp(Shell.Max + steps * ShellStep, Shell.Min + ShellStep, ShellLimit);
        Shell = Shell with { Max = Math.Round(max, 6) };
        return Shell;
    }

    // Pressing the preview key at the cap Local draws: a twist row (an admissible cap) starts the ghost; a blocked cap
    // (no row) has none and says why. Nothing is named while the key is held.
    public bool Press(int cap, double? angle)
    {
        Refusal = null;
        if (angle is not double a)
        {
            Previewing = null;
            Refusal = $"Cap {cap} is blocked: no turn preview, only its straddlers.";
            return false;
        }
        Previewing = new NamedTwist(cap, 1, a);
        return true;
    }

    // Releasing the preview key names the previewed twist, once; a release with no preview names nothing.
    public NamedTwist? Release()
    {
        var named = Previewing;
        Previewing = null;
        if (named is not null) Named = named;
        return named;
    }

    // The ghost is drawn only while a preview is held at an admissible cap.
    public bool Ghost => Previewing is not null;

    // Local's panes: one slice pane, or the held split's two linked copies of the same view.
    public IReadOnlyList<SlicePane> Panes(PaneSpec local)
    {
        if (!Split) return new[] { new SlicePane(local, SliceLayer.Both, Shell, Shrink) };
        var halves = ViewPanes.Split(local);
        return new[]
        {
            new SlicePane(halves[0], SliceLayer.Outside, Shell, Shrink),
            new SlicePane(halves[1], SliceLayer.Inside, Shell, Shrink),
        };
    }

    // Local's line: the layers, the shell and what the keys do now.
    public string Line(int? cap, bool blocked)
    {
        var shell = $"shell {Shell.Min:0.00}–{Shell.Max:0.00} rad";
        if (cap is not int c) return "";
        if (blocked) return $"Cut slice at cap {c} (blocked): straddlers only, no ghost · {shell}";
        var keys = Split ? "outside | inside (release " + SplitKey + " to join)" : "hold " + SplitKey + " to split, " + PreviewKey + " to preview the twist";
        var named = Named is { } n ? $" · named: cap {n.Cap} twist {n.Item}" : "";
        return $"Cut slice at cap {c}: outside filled, inside outlined · {shell} · {keys}{named}";
    }
}
