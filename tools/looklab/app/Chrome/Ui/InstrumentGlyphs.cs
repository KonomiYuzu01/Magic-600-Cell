using Godot;

namespace Magic600.Chrome;

// ADR-D01.77's arrows, orientation glyphs, set-kind glyphs and block marks, drawn as their neutral placeholders
// (InstrumentPlaceholders) the way a Grip frame mark draws its label: the value's letter in a plain square with its
// ticks under it, all in the value's own role colour. The art line replaces these drawings. Nothing in ADR-D02 calls
// them; ADR-D03's canvas places them. Each call returns false and draws nothing for a role of another form or none.
public static class InstrumentGlyphs
{
    // An orientation glyph, a set-kind glyph or a block mark, centred at a point.
    public static bool Draw(CanvasItem on, Vector2 at, string role, TokenSet tokens) =>
        InstrumentPlaceholders.Resolve(role) is { Form: PlaceholderForm.Glyph } glyph && Square(on, at, glyph, tokens);

    // An arrow: its tail is the source and its head the destination; the square sits at the head.
    public static bool Arrow(CanvasItem on, Vector2 source, Vector2 destination, string role, TokenSet tokens)
    {
        if (InstrumentPlaceholders.Resolve(role) is not { Form: PlaceholderForm.Arrow } glyph) return false;
        on.DrawLine(source, destination, tokens.C(role), 1, true);
        return Square(on, destination, glyph, tokens);
    }

    // A block outline: a plain rectangle round the region, the square at its top-left corner.
    public static bool Outline(CanvasItem on, Rect2 region, string role, TokenSet tokens)
    {
        if (InstrumentPlaceholders.Resolve(role) is not { Form: PlaceholderForm.Outline } glyph) return false;
        on.DrawRect(region, tokens.C(role), false, 1);
        return Square(on, region.Position, glyph, tokens);
    }

    private static bool Square(CanvasItem on, Vector2 at, PlaceholderGlyph glyph, TokenSet tokens)
    {
        int radius = GripMarks.Radius(tokens), size = radius + 2;
        Color colour = tokens.C(glyph.Role);
        Font font = ThemeDB.FallbackFont;
        on.DrawRect(new Rect2(at - new Vector2(radius, radius), new Vector2(radius * 2, radius * 2)), colour, false, 1);
        float width = font.GetStringSize(glyph.Letter, HorizontalAlignment.Left, -1, size).X;
        on.DrawString(font, at + new Vector2(-width / 2, (font.GetAscent(size) - font.GetDescent(size)) / 2),
            glyph.Letter, HorizontalAlignment.Left, -1, size, colour);
        float step = radius * 2f / (glyph.Ticks + 1);
        for (int k = 1; k <= glyph.Ticks; k++)
            on.DrawLine(new Vector2(at.X - radius + step * k, at.Y + radius + 1), new Vector2(at.X - radius + step * k, at.Y + radius + 4), colour, 1);
        return true;
    }
}
