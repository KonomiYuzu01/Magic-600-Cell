using System;

namespace Magic600.Shell.D02;

/// <summary>
/// The D02 piece style behind M600_D02=1 (ADR-D02 B17; ADR-D01.41): the selection and the hover drawn as outlines in
/// every view, through B14's style path (host version 10: m6h_set_piece_style's outline channel and the style slots).
/// The core drops a piece style at each adoption, so it is sent again for each revision a frame draws, and whenever the
/// selection or hover changes. Slot 1 styles a one-view frame and Global's pane, slot 2 Local's pane. Later layers
/// (B15's tints, B16's overlays, ADR-D01.46's focus layers) join here, so one place composes the per-piece bytes.
/// </summary>
public static class ShellStyle
{
    /// <summary>The palette entries: 1 the selection's outline, 2 the hover's.</summary>
    public const byte Selected = 1, Hovered = 2;

    /// <summary>The outline band, in barycentric units of each sticker triangle.</summary>
    public const float OutlineWidth = 0.18f;

    private static ulong _revision;
    private static bool _dirty = true;

    static ShellStyle() => ShellPick.Changed += Invalidate;

    /// <summary>Marks the style for sending again (the selection or hover changed).</summary>
    public static void Invalidate() => _dirty = true;

    /// <summary>
    /// Sends the piece style for the revision the next frame draws when it changed or the selection did; answers the
    /// first refusal's code, else <see cref="HostLibrary.Ok"/>.
    /// </summary>
    public static int Apply(HostLibrary library, IntPtr host, ulong revision)
    {
        if (!_dirty && revision == _revision)
        {
            return HostLibrary.Ok;
        }
        var outline = new byte[PieceCount];
        if (ShellPick.Hover is { } hover && hover.Piece < PieceCount)
        {
            outline[hover.Piece] = Hovered;
        }
        if (ShellPick.Selection is { } selection && selection.Piece < PieceCount)
        {
            outline[selection.Piece] = Selected;
        }
        var code = library.SetPieceStyle(host, HostLibrary.StyleOutline, outline, false);
        if (code != HostLibrary.Ok)
        {
            return code;
        }
        foreach (var slot in new uint[] { 1, 2 })
        {
            code = library.SetViewStyle(host, slot, View());
            if (code != HostLibrary.Ok)
            {
                return code;
            }
        }
        _revision = revision;
        _dirty = false;
        return HostLibrary.Ok;
    }

    /// <summary>The jumbled puzzle's pieces (the core's SA2_PIECES, the length of a pose index).</summary>
    public const int PieceCount = 177_120;

    /// <summary>The style slots' style: outlines drawn, no dim, the identity matrix.</summary>
    public static unsafe HostLibrary.ViewStyle View()
    {
        var style = HostLibrary.ViewStyle.Plain();
        style.TintMode = HostLibrary.TintOutline;
        style.OutlineWidth = OutlineWidth;
        // The selection's outline: the core's marked colour (draw.hlsl's straddler mark); the hover's: a light,
        // half-strong outline.
        ReadOnlySpan<float> entries = stackalloc float[] { 1.0f, 0.85f, 0.15f, 1.0f, 1.0f, 1.0f, 1.0f, 0.55f };
        for (var i = 0; i < 4; i++)
        {
            style.Palette[4 * Selected + i] = entries[i];
            style.Palette[4 * Hovered + i] = entries[4 + i];
        }
        return style;
    }
}
