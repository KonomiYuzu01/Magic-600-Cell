using System;
using Godot;

namespace Magic600.Chrome;

// The views and a keycap can stamp the same mark at their own anchor.
public static class GripMarks
{
    public static int Radius(TokenSet tokens) => Math.Max(6, tokens.Px("icon.small") / 2);

    public static void Draw(CanvasItem on, Vector2 at, GripMark mark, TokenSet tokens) =>
        Glyph(on, at, mark.Label, tokens);

    public static void Direction(CanvasItem on, Vector2 at, GripKey key, TokenSet tokens) =>
        Glyph(on, at, key.Meaning, tokens);

    private static void Glyph(CanvasItem on, Vector2 at, string label, TokenSet tokens)
    {
        int radius = Radius(tokens);
        int size = radius + 4;
        var font = ThemeDB.FallbackFont;
        var width = font.GetStringSize(label, HorizontalAlignment.Left, -1, size).X;
        var colour = tokens.C("window.background", .9f);
        if (label.Length == 1) on.DrawCircle(at, radius, colour);
        else on.DrawRect(new Rect2(at - new Vector2(width / 2 + 3, radius), new Vector2(width + 6, radius * 2)), colour);
        on.DrawString(font, at + new Vector2(-width / 2, (font.GetAscent(size) - font.GetDescent(size)) / 2),
            label, HorizontalAlignment.Left, -1, size, tokens.C("text.primary"));
    }
}
