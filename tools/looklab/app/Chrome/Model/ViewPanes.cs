using System;
using System.Collections.Generic;
using System.Linq;

namespace Magic600.Chrome;

// A rectangle in whole pixels: a view's opening in the window, or a pane in the core's surface.
public readonly record struct PaneRect(int X, int Y, int Width, int Height)
{
    public int Right => X + Width;
    public int Bottom => Y + Height;
    public bool IsEmpty => Width <= 0 || Height <= 0;
    public bool Contains(int x, int y) => x >= X && x < Right && y >= Y && y < Bottom;
    public bool Overlaps(PaneRect other) => X < other.Right && other.X < Right && Y < other.Bottom && other.Y < Bottom;
    public PaneRect Offset(int dx, int dy) => this with { X = X + dx, Y = Y + dy };
}

// One pane of a composite frame: the view it draws and its rectangle in the surface.
public sealed record PaneSpec(string View, PaneRect Rect);

// The surface over the drawn openings (its rectangle in the window) and each pane within it.
public sealed record PaneLayout(PaneRect Surface, IReadOnlyList<PaneSpec> Panes);

// Two drawn views on one surface (ADR-D02 build step B18c): the core's surface covers the bounding box of the drawn
// openings, and each opening is a pane of the composite frame (core ABI 8, host 11). At most four panes; they never
// overlap. A point in the window resolves to the pane it lands in, or to nothing.
public static class ViewPanes
{
    public const int MaxPanes = 4;

    // The surface and the panes for openings given in window pixels. Refused: no opening, a fifth, an empty one, or
    // two that overlap.
    public static PaneLayout Layout(IReadOnlyList<PaneSpec> openings)
    {
        if (openings.Count == 0) throw new ArgumentException("a composite frame needs at least one pane");
        if (openings.Count > MaxPanes) throw new ArgumentException("a composite frame has at most " + MaxPanes + " panes; a fifth is refused");
        if (openings.Any(o => o.Rect.IsEmpty)) throw new ArgumentException("a pane's opening is empty");
        for (int i = 0; i < openings.Count; i++)
            for (int j = i + 1; j < openings.Count; j++)
                if (openings[i].Rect.Overlaps(openings[j].Rect))
                    throw new ArgumentException("the openings of " + openings[i].View + " and " + openings[j].View + " overlap");
        int left = openings.Min(o => o.Rect.X), top = openings.Min(o => o.Rect.Y);
        int right = openings.Max(o => o.Rect.Right), bottom = openings.Max(o => o.Rect.Bottom);
        var surface = new PaneRect(left, top, right - left, bottom - top);
        return new PaneLayout(surface, openings.Select(o => o with { Rect = o.Rect.Offset(-left, -top) }).ToArray());
    }

    // Two side-by-side panes of one view's opening (B20's held split, B21's antipode chart): the left takes the floor
    // of half the width, the right the rest.
    public static IReadOnlyList<PaneSpec> Split(PaneSpec opening)
    {
        if (opening.Rect.Width < 2 || opening.Rect.Height <= 0) throw new ArgumentException("an opening too narrow to split");
        int half = opening.Rect.Width / 2;
        return new[]
        {
            opening with { Rect = opening.Rect with { Width = half } },
            opening with { Rect = opening.Rect with { X = opening.Rect.X + half, Width = opening.Rect.Width - half } },
        };
    }

    // The pane a window point lands in: its view and the point in surface pixels; nothing between or outside the panes.
    public static (string View, int X, int Y)? Hit(PaneLayout layout, int x, int y)
    {
        int sx = x - layout.Surface.X, sy = y - layout.Surface.Y;
        foreach (PaneSpec pane in layout.Panes)
            if (pane.Rect.Contains(sx, sy)) return (pane.View, sx, sy);
        return null;
    }

    // Whether a pick reply may be taken: only from the view the click resolved to (B18c drops any other).
    public static bool Takes(string clicked, string replied) => string.Equals(clicked, replied, StringComparison.Ordinal);
}
