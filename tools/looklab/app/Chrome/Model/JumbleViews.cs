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
