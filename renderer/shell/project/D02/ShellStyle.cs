using System;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

/// <summary>
/// The D02 piece style behind M600_D02=1 (ADR-D02 B17; ADR-D01.41): the selection and the hover drawn as outlines in
/// every view, through B14's style path (host version 10: m6h_set_piece_style's outline channel and the style slots).
/// The core drops a piece style at each adoption, so it is sent again for each revision a frame draws, and whenever the
/// selection or hover changes. Slot 1 styles a one-view frame and Global's pane, slot 2 Local's pane. Later layers
/// (B15's tints, B16's overlays) join here, so one place composes the per-piece bytes. ADR-D01.46's focus layer is
/// drawn in the core: each pane's work set and inactive mode apply to its own slot.
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
    private static (InactiveMode Mode, bool[]? Outside) _global, _local;
    private static bool _fillSent;

    // B15 (ADR-D02.20): the tint channel's bytes per piece (the contact classes or the off-lattice mark on the outline,
    // the height ramp on the fill), its palette, and the line and candidate masks.
    private static byte[]? _tintOutline, _tintFill;
    private static float[]? _tintPalette;
    private static byte[]? _lines;
    private static bool[]? _cut;
    private static int? _cutCap;

    /// <summary>The bytes last sent on each channel (the fill null once cleared), for a D02 run's dump.</summary>
    public static byte[]? SentOutline { get; private set; }

    /// <summary>The fill channel's bytes last sent, or null when none are.</summary>
    public static byte[]? SentFill { get; private set; }
    public static byte[]? SentStyleOutline { get; private set; }
    public static byte[]? SentStyleFill { get; private set; }
    public static bool SentHeight { get; private set; }
    public static int? SentCutCap { get; private set; }
    public static ulong SentRevision => _revision;
    /// <summary>B30: the tint palette last sent with the view styles (64 floats), or null.</summary>
    public static float[]? SentPalette { get; private set; }

    /// <summary>The hovered and the selected piece the last sent outline marks.</summary>
    public static int? SentHover { get; private set; }

    /// <summary>The selected piece the last sent outline marks.</summary>
    public static int? SentSelection { get; private set; }

    private static (InactiveMode, bool[]?) Focus(FocusLayers? layers, Func<int, bool>? member, Func<int, bool> conflict)
    {
        var mode = JumbleStyle.FocusMode(layers, member);
        if (mode == InactiveMode.Normal) return (mode, null);
        var outside = new bool[PieceCount];
        for (var piece = 0; piece < PieceCount; piece++)
            outside[piece] = !member!(piece) && !conflict(piece);
        return (mode, outside);
    }

    /// <summary>ADR-D01.46: each pane's mode and membership, with protection conflicts fully visible.</summary>
    public static void SetFocusViews(FocusLayers? global, FocusLayers? local, Func<FocusLayers, Func<int, bool>?> member,
                                     Func<int, bool> conflict)
    {
        _global = Focus(global, global is { Dimming: true } ? member(global) : null, conflict);
        _local = _global.Outside != null && local is { Dimming: true } && global!.Focused == local.Focused
            ? (local.Inactive, _global.Outside)
            : Focus(local, local is { Dimming: true } ? member(local) : null, conflict);
        Invalidate();
    }

    public static InactiveMode SentGlobalMode { get; private set; }
    public static InactiveMode SentLocalMode { get; private set; }

    /// <summary>
    /// B15 (ADR-D02.20): the tint channel's bytes per piece on the outline and the fill (null where the mode draws
    /// nothing there; another length is dropped) and the palette, 64 floats of which entries 3 to 14 are read.
    /// </summary>
    /// <summary>
    /// B31 (ADR-D02.47): the held simulation's row-major 3x3 matrix, or null for none. It reaches the core's view style
    /// only when <see cref="CoreSimulation"/> is on: the look-cost row G8 measures it first (framework section 8).
    /// </summary>
    public static void SetSimulation(double[]? matrix)
    {
        Simulation = matrix?.Length == 9 ? matrix : null;
        Invalidate();
    }

    /// <summary>B31: the simulation matrix held now (null for none), whether or not the core is sent it.</summary>
    public static double[]? Simulation { get; private set; }

    /// <summary>B31: the G8 switch: M600_D02_CORE_SIMULATION=1 sends the simulation to the core; off by default.</summary>
    public static bool CoreSimulation => System.Environment.GetEnvironmentVariable("M600_D02_CORE_SIMULATION") == "1";

    /// <summary>B31: the matrix the last style sent carried (identity without a simulation or with the switch off).</summary>
    public static float[] SentMatrix { get; private set; } = { 1, 0, 0, 0, 1, 0, 0, 0, 1 };

    public static void SetTint(byte[]? outline, byte[]? fill, float[]? palette, OverlayView overlay)
    {
        _tintOutline = outline?.Length == PieceCount ? outline : null;
        _tintFill = fill?.Length == PieceCount ? fill : null;
        _tintPalette = palette?.Length == 64 ? palette : null;
        _lines = JumbleOverlay.Outline(null, overlay.Lines);
        _cut = overlay.CutCandidates;
        _cutCap = _cut == null ? null : overlay.Cap;
        Invalidate();
    }

    static ShellStyle() => ShellPick.Changed += Invalidate;

    private static ulong _pending;

    /// <summary>The revision handed out by a committed press, for the slice and status paths.</summary>
    public static void Committed(ulong revision) => _pending = Math.Max(_pending, revision);

    /// <summary>LF1: the revision a committed press handed out that a frame has not shown yet, else 0; the slice and
    /// status channels are sent for it too.</summary>
    public static ulong PendingAfter(ulong revision) => _pending > revision ? _pending : 0;

    /// <summary>Marks the style for sending again (the selection or hover changed).</summary>
    public static void Invalidate() => _dirty = true;

    /// <summary>B20: whether the slice's inside layer was sent, so Local's slots may draw the slice.</summary>
    private static bool _sliceSent;
    public static bool SliceSent => _sliceSent;

    /// <summary>
    /// Sends the piece style for the revision the next frame draws when it changed or the selection did; answers the
    /// first refusal's code, else <see cref="HostLibrary.Ok"/>.
    /// </summary>
    public static int Apply(HostLibrary library, IntPtr host, ulong revision, ulong? bound)
    {
        // B18 (for B20): the certified transform of the twist at the cap Local draws, read once per cap and revision.
        ShellTurn.Note(library, host, revision);
        // B18 rest: a committed twist's sweep advances its phase; a half-turn's flash starts or ends its outline.
        if (ShellTurn.Step(library, host, revision))
        {
            _dirty = true;
        }
        // B20: the slice's inside layer and the turn preview's transform. A failed slice never blocks the other styles:
        // they are sent as usual, Local's slots go without the slice while it fails, and its code is returned last.
        var sliced = ShellSlice.Apply(library, host, revision);
        ShellStatus.Prepare(library, host, revision, sliced == HostLibrary.Ok);
        if ((sliced == HostLibrary.Ok) != _sliceSent)
        {
            _sliceSent = sliced == HostLibrary.Ok;
            _dirty = true;
        }
        ShellGrip.StepGhost(revision);
        if (!_dirty && revision == _revision && !ShellSlice.Restyle)
        {
            return sliced;
        }
        // B15: the tint's outline bytes under the selection's and the hover's outlines.
        bool stateStyle = JumbleStyle.BoundFor(bound, revision);
        byte[]? tint = stateStyle ? _tintOutline : null;
        var outline = tint is null ? new byte[PieceCount] : (byte[])tint.Clone();
        // B18 rest: a half-turn's flash outlines the moved set in palette entry 15 (B16's overlay line) for
        // jumble.half_turn's fixed time, under the selection's and the hover's outlines.
        if (ShellTurn.Flash is { } flash)
        {
            for (var piece = 0; piece < PieceCount; piece++)
            {
                if (flash[piece] != 0) outline[piece] = JumbleOverlay.Entry;
            }
        }
        // ADR-D01.13: the grip's click ghost shares the hover palette entry in each view.
        if (ShellGrip.Moving is { } moving)
        {
            for (var piece = 0; piece < PieceCount; piece++)
                if (moving[piece] != 0) outline[piece] = Hovered;
        }
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
        foreach (int piece in ShellError.Pieces)
        {
            outline[piece] |= HostLibrary.StyleError;
        }
        var heights = stateStyle ? _tintFill : null;
        var lines = stateStyle ? _lines : null;
        var cut = stateStyle ? _cut : null;
        var preview = ShellPreview.MovingAt(revision);
        bool markedPreview = ShellPreview.Overlay != null;
        var global = stateStyle ? _global : default;
        var local = stateStyle ? _local : default;
        var styleOutline = outline;
        byte[]? fill = null, styleFill = null;
        if (heights != null || lines != null || cut != null || preview != null || global.Outside != null || local.Outside != null)
        {
            styleFill = new byte[PieceCount];
            if (heights != null) styleOutline = new byte[PieceCount];
            if (heights != null || global.Outside != null) fill = new byte[PieceCount];
            for (var piece = 0; piece < PieceCount; piece++)
            {
                var ink = JumbleStyle.Ink(lines?[piece] > 0, cut?[piece] == true, preview?[piece] > 0, markedPreview);
                var height = heights?[piece] ?? 0;
                bool outside = global.Outside?[piece] == true;
                (styleOutline[piece], styleFill[piece]) = JumbleStyle.Encode(outline[piece], height, heights != null, ink,
                    outside, local.Outside?[piece] == true);
                if (fill != null) fill[piece] = (byte)(height | (outside && !JumbleStyle.Reference(ink) ? HostLibrary.DimSet : 0));
            }
        }
        // Only the adopted revision gets these bytes. Core adoption clears both channels before a new state binds.
        var code = library.SetPieceStyle(host, revision, HostLibrary.StyleOutline, styleOutline, false);
        if (code != HostLibrary.Ok)
        {
            return code;
        }
        if (styleFill is not null || _fillSent)
        {
            code = library.SetPieceStyle(host, revision, HostLibrary.StyleFill, styleFill, false);
            if (code != HostLibrary.Ok)
            {
                return code;
            }
            _fillSent = styleFill is not null;
        }
        foreach (var slot in new uint[] { 1, 2, 3 })
        {
            var slotStyle = View(slot, slot == 1 ? global.Mode : local.Mode, heights != null, stateStyle ? _cutCap : null);
            code = library.SetViewStyle(host, slot, slotStyle);
            if (code != HostLibrary.Ok)
            {
                return code;
            }
            ShellError.Sent(slot, slotStyle);
        }
        if (ShellPreview.PreviewColour is { } previewInk)
        {
            code = library.SetViewStyle(host, 4, PreviewStyle(previewInk));
            if (code != HostLibrary.Ok) return code;
        }
        SentOutline = outline;
        SentFill = fill;
        SentStyleOutline = styleOutline;
        SentStyleFill = styleFill;
        SentHeight = heights != null;
        SentCutCap = stateStyle ? _cutCap : null;
        SentGlobalMode = global.Mode;
        SentLocalMode = local.Mode;
        SentHover = hovered;
        SentSelection = selected;
        SentPalette = _tintPalette;
        var view = View(1, global.Mode, heights != null, SentCutCap);
        var sent = new float[9];
        unsafe
        {
            for (var i = 0; i < 9; i++)
            {
                sent[i] = view.Matrix[i];
            }
        }
        SentMatrix = sent;
        ShellSlice.Restyle = false;
        _revision = revision;
        _dirty = false;
        return sliced;
    }

    /// <summary>The jumbled puzzle's pieces (the core's SA2_PIECES, the length of a pose index).</summary>
    public const int PieceCount = 177_120;

    /// <summary>
    /// The style slots' style: outlines and the fill tint drawn, the focus layer's pieces dimmed or hidden where the
    /// slot's view dims (none without a focus), the selection, hover and tint palette entries, the identity matrix.
    /// </summary>
    private static unsafe HostLibrary.ViewStyle View(uint slot, InactiveMode mode, bool height, int? cut)
    {
        var style = HostLibrary.ViewStyle.Plain();
        style.TintMode = HostLibrary.TintOutline | HostLibrary.TintFill;
        style.OutlineWidth = OutlineWidth;
        if (mode != InactiveMode.Normal)
        {
            style.DimMode = mode == InactiveMode.Hidden ? HostLibrary.HideMarked : HostLibrary.DimMarked;
            style.DimAmount = mode == InactiveMode.Hidden ? 0 : DimAmount;
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
        else
        {
            // B18 rest: without a tint palette, the flash's entry 15 still draws (state.preview's fallback colour).
            ReadOnlySpan<float> line = stackalloc float[] { 0.55f, 0.8f, 1.0f, 1.0f };
            for (var i = 0; i < 4; i++)
            {
                style.Palette[4 * JumbleOverlay.Entry + i] = line[i];
            }
        }
        // B22 rest (ADR-D02.10): the moved set's outline under a chart is Global's; Local's panes carry the motion.
        if (slot != 1 && ShellTurn.OutlineOnly)
        {
            style.Palette[4 * JumbleOverlay.Entry + 3] = 0;
        }
        // B31: the held simulation's matrix, with the G8 switch on.
        if (CoreSimulation && Simulation is { } matrix)
        {
            for (var i = 0; i < 9; i++)
            {
                style.Matrix[i] = (float)matrix[i];
            }
        }
        // B20: Local's slots (2, and 3 for the split's inside copy) draw the cut slice once its inside layer is sent.
        if (slot is 2 or 3 && _sliceSent)
        {
            ShellSlice.Style(ref style, slot, _tintPalette);
        }
        // B21: while the raw view is on, Local's slot draws no slice.
        if (slot is 2 or 3)
        {
            ShellRaw.Style(ref style);
        }
        if (slot is 1 or 2 or 3 && ShellError.Colour is { } error)
        {
            style.Palette[0] = (float)error.R;
            style.Palette[1] = (float)error.G;
            style.Palette[2] = (float)error.B;
            style.Palette[3] = ShellError.Alpha;
        }
        if (ShellPreview.Overlay != null && ShellPreview.OverlayColour is { } preview)
            SetPreviewInk(ref style, preview, .65f);
        // styled.hlsl's D02 options occupy an unused colour entry. Blue 0.5 and signed zero alpha tag it as data.
        int options = 4 * (height ? JumbleTint.FirstClass : JumbleTint.FirstHeight);
        style.Palette[options] = cut is int cap ? (cap + 1) / 1024f : 0;
        style.Palette[options + 1] = slot == 1 ? 0 : 1;
        style.Palette[options + 2] = 0.5f;
        style.Palette[options + 3] = BitConverter.Int32BitsToSingle(int.MinValue);
        return style;
    }

    private static HostLibrary.ViewStyle PreviewStyle(Rgb ink)
    {
        var style = HostLibrary.ViewStyle.Plain();
        style.TintMode = HostLibrary.TintOutline;
        style.OutlineWidth = OutlineWidth;
        SetPreviewInk(ref style, ink, 1);
        return style;
    }

    private static unsafe void SetPreviewInk(ref HostLibrary.ViewStyle style, Rgb ink, float alpha)
    {
        fixed (float* palette = style.Palette)
        {
            int at = 4 * JumbleOverlay.Entry;
            palette[at] = (float)ink.R;
            palette[at + 1] = (float)ink.G;
            palette[at + 2] = (float)ink.B;
            palette[at + 3] = alpha;
        }
    }
}
