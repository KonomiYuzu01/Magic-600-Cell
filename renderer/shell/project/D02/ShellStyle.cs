using System;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

/// <summary>
/// The D02 piece style behind M600_D02=1 (ADR-D02 B17; ADR-D01.41): the selection and the hover drawn as outlines in
/// every view, through B14's style path (host version 10: m6h_set_piece_style's outline channel and the style slots).
/// The core drops a piece style at each adoption, so it is sent again for each revision a frame draws, and whenever the
/// selection or hover changes. Slot 1 styles a one-view frame and Global's pane, slot 2 Local's pane. Later layers
/// (B15's tints, B16's overlays) join here, so one place composes the per-piece bytes. ADR-D01.46's focus layer is
/// drawn in the core (<see cref="SetFocus"/>): the pieces it leaves out are dimmed or hidden, and neither is picked.
/// </summary>
public static class ShellStyle
{
    /// <summary>The palette entries: 1 the selection's outline, 2 the hover's.</summary>
    public const byte Selected = 1, Hovered = 2;

    /// <summary>The outline band, in barycentric units of each sticker triangle.</summary>
    public const float OutlineWidth = 0.18f;

    /// <summary>A dimmed piece's colour is multiplied by 1 - DimAmount: FocusLayers.ShownPiece's quarter.</summary>
    public const float DimAmount = 0.75f;

    private static ulong _revision;
    private static bool _dirty = true;
    private static byte[]? _focus;
    private static bool _focusSent, _hide;

    // B15 (ADR-D02.20): the tint channel's bytes per piece (the contact classes or the off-lattice mark on the outline,
    // the height ramp on the fill), its palette (entries 3 to 14 read), and per style slot (1 Global, 2 Local) whether
    // that view's own focus layers dim.
    private static byte[]? _tintOutline, _tintFill;
    private static float[]? _tintPalette;
    private static readonly bool[] _slotDims = { true, true, true };

    /// <summary>The bytes last sent on each channel (the fill null once cleared), for a D02 run's dump.</summary>
    public static byte[]? SentOutline { get; private set; }

    /// <summary>The fill channel's bytes last sent, or null when none are.</summary>
    public static byte[]? SentFill { get; private set; }
    /// <summary>B30: the tint palette last sent with the view styles (64 floats), or null.</summary>
    public static float[]? SentPalette { get; private set; }

    /// <summary>The hovered and the selected piece the last sent outline marks.</summary>
    public static int? SentHover { get; private set; }

    /// <summary>The selected piece the last sent outline marks.</summary>
    public static int? SentSelection { get; private set; }

    /// <summary>
    /// ADR-D01.46's focus layer in both views, through Look Lab's <see cref="FocusLayers"/> model: while it dims (a
    /// focused predicate that applies, and a cycle that is not Normal), a piece shows fully when
    /// <see cref="FocusLayers.ShownPiece"/> says so (a member of the predicate, or a breakthrough: a protection conflict
    /// or a complete operation's effect, ADR-D01.4 item 8); every other piece takes the cycle's mode, dimmed to a quarter
    /// or hidden, and the core picks neither (ABI 10's styled ID pass). Null layers, or a model that does not dim, clear
    /// it. The chrome calls it from <see cref="FocusLayers.Changed"/> and whenever the predicate's members change.
    /// </summary>
    public static void SetFocus(FocusLayers? layers, Func<int, bool>? member, Func<int, bool>? breaksThrough = null)
    {
        _focus = null;
        _hide = false;
        _slotDims[1] = _slotDims[2] = true;
        if (layers is { Dimming: true } && member is not null)
        {
            _focus = new byte[PieceCount];
            for (var piece = 0; piece < PieceCount; piece++)
            {
                if (layers.ShownPiece(member(piece), breaksThrough?.Invoke(piece) ?? false) < 1)
                {
                    _focus[piece] = HostLibrary.DimSet;
                }
            }
            _hide = layers.Inactive == InactiveMode.Hidden;
        }
        Invalidate();
    }

    /// <summary>The focus layer's mode as the core draws it: none, dimmed or hidden (for a D02 run's dump).</summary>
    public static string FocusMode => _focus is null ? "none" : _hide ? "hidden" : "dimmed";

    /// <summary>
    /// ADR-D01.46 per view: the dim set from the view whose layers dim (Global's first), and each style slot dims only
    /// while its own view's layers dim with the same focused predicate. With both views dimming, Global's mode (dimmed or
    /// hidden) serves both. <paramref name="member"/> gives a view's focused predicate's members, piece by piece.
    /// B16: no focus layer dims or hides a piece <paramref name="breaksThrough"/> names (jumbling-observation.md rule 5).
    /// </summary>
    public static void SetFocusViews(FocusLayers? global, FocusLayers? local, Func<FocusLayers, Func<int, bool>?> member,
                                     Func<int, bool>? breaksThrough = null)
    {
        var source = global is { Dimming: true } ? global : local;
        SetFocus(source, source is null ? null : member(source), breaksThrough);
        _slotDims[1] = _focus is not null && global is { Dimming: true } && global.Focused == source!.Focused;
        _slotDims[2] = _focus is not null && local is { Dimming: true } && local.Focused == source!.Focused;
    }

    /// <summary>Whether a style slot dims the focus set now (for a D02 run's dump).</summary>
    public static bool SlotDims(uint slot) => _focus is not null && _slotDims[slot is 1 or 2 ? slot : 1];

    /// <summary>
    /// B15 (ADR-D02.20): the tint channel's bytes per piece on the outline and the fill (null where the mode draws
    /// nothing there; another length is dropped) and the palette, 64 floats of which entries 3 to 14 are read.
    /// </summary>
    public static void SetTint(byte[]? outline, byte[]? fill, float[]? palette)
    {
        _tintOutline = outline?.Length == PieceCount ? outline : null;
        _tintFill = fill?.Length == PieceCount ? fill : null;
        _tintPalette = palette?.Length == 64 ? palette : null;
        Invalidate();
    }

    // The fill channel: the height ramp's palette index with the focus layer's dim bit, or null when neither is set.
    private static byte[]? Fill()
    {
        if (_tintFill is null)
        {
            return _focus;
        }
        if (_focus is null)
        {
            return _tintFill;
        }
        var fill = new byte[PieceCount];
        for (var piece = 0; piece < PieceCount; piece++)
        {
            fill[piece] = (byte)(_tintFill[piece] | _focus[piece]);
        }
        return fill;
    }

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
        // B15: the tint's outline bytes under the selection's and the hover's outlines.
        var outline = _tintOutline is null ? new byte[PieceCount] : (byte[])_tintOutline.Clone();
        int? hovered = null, selected = null;
        if (ShellPick.Hover is { } hover && hover.Piece < PieceCount)
        {
            outline[hover.Piece] = Hovered;
            hovered = (int)hover.Piece;
        }
        if (ShellPick.Selection is { } selection && selection.Piece < PieceCount)
        {
            outline[selection.Piece] = Selected;
            selected = (int)selection.Piece;
        }
        var code = library.SetPieceStyle(host, HostLibrary.StyleOutline, outline, false);
        if (code != HostLibrary.Ok)
        {
            return code;
        }
        SentOutline = outline;
        SentHover = hovered;
        SentSelection = selected;
        // ADR-D01.46 and B15: the fill bytes, the height ramp with the focus layer's dim bit; once sent, a cleared fill
        // clears the channel.
        var fill = Fill();
        if (fill is not null || _focusSent)
        {
            code = library.SetPieceStyle(host, HostLibrary.StyleFill, fill, false);
            if (code != HostLibrary.Ok)
            {
                return code;
            }
            _focusSent = fill is not null;
        }
        SentFill = fill;
        foreach (var slot in new uint[] { 1, 2 })
        {
            code = library.SetViewStyle(host, slot, View(slot));
            if (code != HostLibrary.Ok)
            {
                return code;
            }
        }
        SentPalette = _tintPalette;
        _revision = revision;
        _dirty = false;
        return HostLibrary.Ok;
    }

    /// <summary>The jumbled puzzle's pieces (the core's SA2_PIECES, the length of a pose index).</summary>
    public const int PieceCount = 177_120;

    /// <summary>
    /// The style slots' style: outlines and the fill tint drawn, the focus layer's pieces dimmed or hidden where the
    /// slot's view dims (none without a focus), the selection, hover and tint palette entries, the identity matrix.
    /// </summary>
    public static unsafe HostLibrary.ViewStyle View(uint slot = 1)
    {
        var style = HostLibrary.ViewStyle.Plain();
        style.TintMode = HostLibrary.TintOutline | HostLibrary.TintFill;
        style.OutlineWidth = OutlineWidth;
        if (_focus is not null && _slotDims[slot is 1 or 2 ? slot : 1])
        {
            style.DimMode = _hide ? HostLibrary.HideMarked : HostLibrary.DimMarked;
            style.DimAmount = _hide ? 0 : DimAmount;
        }
        // The selection's outline: the core's marked colour (draw.hlsl's straddler mark); the hover's: a light,
        // half-strong outline.
        ReadOnlySpan<float> entries = stackalloc float[] { 1.0f, 0.85f, 0.15f, 1.0f, 1.0f, 1.0f, 1.0f, 0.55f };
        for (var i = 0; i < 4; i++)
        {
            style.Palette[4 * Selected + i] = entries[i];
            style.Palette[4 * Hovered + i] = entries[4 + i];
        }
        // B15: the tint's entries, 3 to 14 (JumbleTint's palette plan).
        if (_tintPalette is not null)
        {
            for (var i = 4 * JumbleTint.FirstClass; i < 64; i++)
            {
                style.Palette[i] = _tintPalette[i];
            }
        }
        return style;
    }
}
