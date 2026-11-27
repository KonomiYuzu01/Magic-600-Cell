using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using Magic600.Chrome;

namespace Magic600.Shell;

/// <summary>
/// The shell's native host library (c600_shell_host.dll, built from renderer/bridge/host), step S3 of the shell: a
/// draft on route B, accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks. The library
/// runs the display host over the bridge. The shell hands it the core's function table
/// (<see cref="CoreLibrary.Functions"/>) and the context it created with sa2_create, then drives frames through it.
/// tests/test_renderer_shell.py checks <see cref="Exports"/>, the version, the status codes and the frame, presented
/// and view-result rows against the Rust source. Version 7 (LP4-3) adds the write-check-run flow's calls, which the
/// editor panel (<see cref="Editor.EditorPanel"/>) makes: <see cref="SetFocus"/>, <see cref="RegisterRow"/>,
/// <see cref="Palette"/>, <see cref="Invoke"/>, <see cref="TakeJob"/>, <see cref="FinishJob"/>,
/// <see cref="SetDraftWord"/> and <see cref="Answer"/>. Version 8 (ADR-D02 B5) adds <see cref="StateDigest"/>, the
/// head's state digests, by which a D02 sample binds to the session. Version 9 (ADR-D02, over the core's ABI 6) adds
/// <see cref="SetViewShape"/>, a view's cell gap, sticker shrink, d4 and zoom. Version 10 (ADR-D02 B14, over the core's
/// ABI 7) adds <see cref="SetPieceStyle"/> and <see cref="SetViewStyle"/>, the per-piece style and the style slots.
/// Version 11 (ADR-D02 B18b, over the core's ABI 8) adds Local and the composite frame: <see cref="OpenLocal"/>,
/// <see cref="SetViewCamera"/>, <see cref="SetViewVisible"/>, <see cref="SetPane"/>, <see cref="ClearPane"/>, and
/// <see cref="Submitted"/>'s pane count and held flag. Version 12 (ADR-D02 B18c, over the core's ABI 9) adds
/// <see cref="SetViewFocus"/>, Local's own projection, focus cap and spread. Version 13 (ADR-D02 B23, over the core's
/// ABI 10) adds <see cref="SetViewLayout"/>, the Hopf-ring opening. Version 14 (ADR-D02 B17) adds
/// <see cref="PickRequest"/> and <see cref="PickPoll"/>, picks in Jumble.
/// </summary>
public sealed unsafe class HostLibrary : IDisposable
{
    /// <summary>The C interface this shell speaks; the library's m6h_abi_version must answer it.</summary>
    public const uint AbiVersion = 14;

    /// <summary>The name of the correctness control that <see cref="CloseSession"/> runs (the run row's).</summary>
    public const string CorrectnessControl = "journal-reopen-and-kernel-replay";

    /// <summary><see cref="OpenSession"/>'s puzzle: 600-cell-Full with Jumble off, index 0 of
    /// <see cref="EventLog.Puzzles"/>.</summary>
    public const uint PuzzleJumbleOff = 0;
    /// <summary><see cref="OpenSession"/>'s puzzle: the jumbled puzzle, index 1 of <see cref="EventLog.Puzzles"/>.
    /// </summary>
    public const uint PuzzleJumbleOn = 1;

    /// <summary><see cref="SetFocus"/>: the puzzle has the keyboard focus; the router's global map gets every key.
    /// </summary>
    public const uint FocusPuzzle = 0;
    /// <summary><see cref="SetFocus"/>: a text field has it and keeps its typing, editing and office chords and a key
    /// bound to a commit row (DT-M1 C4).</summary>
    public const uint FocusText = 1;

    public const int Ok = 0;
    public const int InvalidArgument = 1;
    public const int ModelNotLoaded = 2;
    public const int CoreRefused = 3;
    public const int HostRefused = 4;
    /// <summary>The renderer is unavailable, for example after the device was lost; <see cref="Recover"/> may help.</summary>
    public const int Unavailable = 5;
    /// <summary>An earlier call panicked; only <see cref="LastError"/> and <see cref="Destroy"/> are left.</summary>
    public const int Poisoned = 6;
    /// <summary>The session did not open, is already open, or is not open.</summary>
    public const int SessionRefused = 7;
    /// <summary>The language runtime did not start, or the host has started one already.</summary>
    public const int RuntimeRefused = 8;

    /// <summary><see cref="KeyResult.Press"/>: no measured press (no twist key, or a repeat).</summary>
    public const uint PressNone = 0;
    /// <summary><see cref="KeyResult.Press"/>: a twist key committed twist.apply.</summary>
    public const uint PressCommitted = 1;
    /// <summary><see cref="KeyResult.Press"/>: a twist key that the router or the layer refused.</summary>
    public const uint PressRefused = 2;
    /// <summary><see cref="KeyResult.Press"/>: no measured press, but an undo or redo changed the state.</summary>
    public const uint PressOther = 3;

    /// <summary><see cref="ViewResult.Status"/>: the run has not ended yet.</summary>
    public const uint ViewStatusPending = 0;
    /// <summary><see cref="ViewResult.Status"/>: the run's publication was installed; a draft at once.</summary>
    public const uint ViewStatusPublished = 1;
    /// <summary><see cref="ViewResult.Status"/>: the run ended without a publication; <see cref="LastError"/> gives
    /// its {"code", "reason"} JSON after <see cref="GetViewResult"/>.</summary>
    public const uint ViewStatusRefused = 2;
    /// <summary><see cref="ViewResult.Kind"/>: a run of <see cref="RunView"/>.</summary>
    public const uint ViewKindRun = 0;
    /// <summary><see cref="ViewResult.Kind"/>: a draft of <see cref="ViewDraft"/>.</summary>
    public const uint ViewKindDraft = 1;

    /// <summary>The library's exports, in the order the Rust source declares them.</summary>
    public static readonly IReadOnlyList<string> Exports = new[]
    {
        "m6h_abi_version",
        "m6h_create",
        "m6h_preload",
        "m6h_create_preloaded",
        "m6h_preload_free",
        "m6h_set_camera",
        "m6h_set_viewport",
        "m6h_set_view_shape",
        "m6h_set_view_focus",
        "m6h_pick_request",
        "m6h_pick_poll",
        "m6h_set_view_layout",
        "m6h_set_piece_style",
        "m6h_set_transform",
        "m6h_twist_transform",
        "m6h_set_view_style",
        "m6h_open_local",
        "m6h_set_view_camera",
        "m6h_set_view_visible",
        "m6h_set_pane",
        "m6h_clear_pane",
        "m6h_pump",
        "m6h_frame",
        "m6h_read_presented",
        "m6h_recover",
        "m6h_open_session",
        "m6h_load_runtime",
        "m6h_run_view",
        "m6h_view_result",
        "m6h_view_draft",
        "m6h_key_down",
        "m6h_key_up",
        "m6h_focus_lost",
        "m6h_set_focus",
        "m6h_register_row",
        "m6h_palette",
        "m6h_invoke",
        "m6h_take_job",
        "m6h_finish_job",
        "m6h_set_draft_word",
        "m6h_state_digest",
        "m6h_answer",
        "m6h_close_session",
        "m6h_check",
        "m6h_last_error",
        "m6h_destroy",
        // ADR-D02 B21: the raw view's antipode pane.
        "m6h_set_pane_flags",
        "m6h_set_sticker_colours",
        "m6h_set_scene_look",
    };

    /// <summary>A submitted frame, as m6h_frame writes it (the Rust M6hFrame).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct Submitted
    {
        public ulong Sequence;
        public ulong DeviceEpoch;
        public ulong Surface;
        /// <summary>The engine revision whose labels the frame draws.</summary>
        public ulong EngineRevision;
        /// <summary>The engine revision of the publication the frame draws, or 0 for none.</summary>
        public ulong PublicationRevision;
        /// <summary>The panes of a composite frame, or 0 for a one-view frame (version 11).</summary>
        public uint Panes;
        /// <summary>1 when nothing was submitted because a drawn pane's view did not show the adopted revision yet; the
        /// other fields then name the last frame (version 11).</summary>
        public uint Held;
    }

    /// <summary>What m6h_key_down did with one key-down (the Rust M6hKey).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct KeyResult
    {
        public uint Press;
        public uint Reserved;
        /// <summary>The adopted engine revision after the key; for a committed press, the one that shows it.</summary>
        public ulong EngineRevision;
    }

    /// <summary>One confirmed presentation, as m6h_read_presented writes it (the Rust M6hPresented).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct Presented
    {
        public ulong Sequence;
        public ulong DeviceEpoch;
        public ulong Surface;
        public ulong EngineRevision;
        public ulong Camera;
        /// <summary>The SyncQPCTime of the statistics row that confirmed the frame, in QPC ticks.</summary>
        public ulong DisplayQpc;
    }

    /// <summary>A view job's or draft's record, as m6h_view_result writes it (the Rust M6hViewResult).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct ViewResult
    {
        /// <summary>
        /// <see cref="ViewStatusPending"/>, <see cref="ViewStatusPublished"/> or <see cref="ViewStatusRefused"/>.
        /// </summary>
        public uint Status;
        /// <summary><see cref="ViewKindRun"/> or <see cref="ViewKindDraft"/>.</summary>
        public uint Kind;
        /// <summary>The engine revision adopted at the call: the revision a run's publication shows.</summary>
        public ulong Revision;
        /// <summary>The SHA-256 of the run's source text, or the draft's digest.</summary>
        public fixed byte SourceDigest[32];
        /// <summary>A run's first confirmed frame that drew its publication; a draft's first stale one.</summary>
        public ulong Frame;
        /// <summary>A draft's first confirmed frame after <see cref="Frame"/> that drew current output.</summary>
        public ulong ClearedFrame;
        /// <summary>1 when the run completed with a value.</summary>
        public uint HasValue;
        public uint Reserved;
        /// <summary>The SHA-256 of the completed run's value bytes.</summary>
        public fixed byte ValueSha256[32];
    }

    private IntPtr _library;
    private readonly Dictionary<string, IntPtr> _exports;

    private HostLibrary(IntPtr library, Dictionary<string, IntPtr> exports)
    {
        _library = library;
        _exports = exports;
    }

    /// <summary>
    /// Loads the library from an absolute path, resolves every export and checks the version. A missing export or
    /// another version unloads the library and throws.
    /// </summary>
    public static HostLibrary Load(string path)
    {
        if (!System.IO.Path.IsPathFullyQualified(path))
        {
            throw new ArgumentException("the host library needs an absolute path", nameof(path));
        }
        if (sizeof(ViewResult) != 104)
        {
            throw new NotSupportedException($"the view result row is {sizeof(ViewResult)} bytes, the library's 104");
        }
        var library = NativeLibrary.Load(path);
        try
        {
            var exports = new Dictionary<string, IntPtr>();
            foreach (var name in Exports)
            {
                if (!NativeLibrary.TryGetExport(library, name, out var address))
                {
                    throw new EntryPointNotFoundException($"the host library does not export {name}");
                }
                exports[name] = address;
            }
            var version = ((delegate* unmanaged[Cdecl]<uint>)exports["m6h_abi_version"])();
            if (version != AbiVersion)
            {
                throw new NotSupportedException($"the host library speaks version {version}, the shell {AbiVersion}");
            }
            return new HostLibrary(library, exports);
        }
        catch
        {
            NativeLibrary.Free(library);
            throw;
        }
    }

    /// <summary>
    /// Creates a host over the core's table and a context with a surface attached; it loads the shipped model from
    /// <paramref name="assets"/>. On failure <paramref name="host"/> is zero and <see cref="LastError"/> with a zero
    /// handle gives the reason.
    /// </summary>
    public int Create(CoreLibrary core, IntPtr context, string assets, out IntPtr host)
    {
        var table = new IntPtr[core.Functions.Count];
        for (var i = 0; i < table.Length; i++)
        {
            table[i] = core.Functions[i];
        }
        var path = Encoding.UTF8.GetBytes(assets);
        var created = IntPtr.Zero;
        int code;
        fixed (IntPtr* functions = table)
        fixed (byte* bytes = path)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr*, IntPtr, byte*, nuint, IntPtr*, int>)_exports["m6h_create"])(
                functions, context, bytes, (nuint)path.Length, &created);
        }
        host = created;
        return code;
    }

    /// <summary>
    /// Charter E3's startup (a draft): starts the loads that <see cref="Create"/> makes from
    /// <paramref name="assets"/>, and with <paramref name="session"/> the exact model's that
    /// <see cref="OpenSession"/> reads, each on a thread of its own, and returns at once. <see cref="CreatePreloaded"/>
    /// takes <paramref name="preload"/>; <see cref="FreePreload"/> frees one that no call took. On failure
    /// <paramref name="preload"/> is zero and <see cref="LastError"/> with a zero handle gives the reason.
    /// </summary>
    public int Preload(string assets, bool session, out IntPtr preload)
    {
        var path = Encoding.UTF8.GetBytes(assets);
        var started = IntPtr.Zero;
        int code;
        fixed (byte* bytes = path)
        {
            code = ((delegate* unmanaged[Cdecl]<byte*, nuint, uint, IntPtr*, int>)_exports["m6h_preload"])(
                bytes, (nuint)path.Length, session ? 1u : 0u, &started);
        }
        preload = started;
        return code;
    }

    /// <summary>
    /// <see cref="Create"/> with the loads that <see cref="Preload"/> started, from its assets folder. The call takes
    /// <paramref name="preload"/> whatever it answers. On failure <paramref name="host"/> is zero and
    /// <see cref="LastError"/> with a zero handle gives the reason.
    /// </summary>
    public int CreatePreloaded(CoreLibrary core, IntPtr context, IntPtr preload, out IntPtr host)
    {
        var table = new IntPtr[core.Functions.Count];
        for (var i = 0; i < table.Length; i++)
        {
            table[i] = core.Functions[i];
        }
        var created = IntPtr.Zero;
        int code;
        fixed (IntPtr* functions = table)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr*, IntPtr, IntPtr, IntPtr*, int>)_exports["m6h_create_preloaded"])(
                functions, context, preload, &created);
        }
        host = created;
        return code;
    }

    /// <summary>Frees a preload that no call took; it waits for the loads still running.</summary>
    public void FreePreload(IntPtr preload) =>
        ((delegate* unmanaged[Cdecl]<IntPtr, void>)_exports["m6h_preload_free"])(preload);

    /// <summary>Sets the view's 4D rotation: 16 floats, column-major.</summary>
    public int SetCamera(IntPtr host, ReadOnlySpan<float> matrix)
    {
        if (matrix.Length != 16)
        {
            throw new ArgumentException("a camera is 16 floats", nameof(matrix));
        }
        fixed (float* m = matrix)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, float*, int>)_exports["m6h_set_camera"])(host, m);
        }
    }

    public int SetViewport(IntPtr host, uint width, uint height) =>
        ((delegate* unmanaged[Cdecl]<IntPtr, uint, uint, int>)_exports["m6h_set_viewport"])(host, width, height);

    /// <summary>The view number of the host's one view (the Rust source's VIEW).</summary>
    public const ulong View = 1;
    /// <summary>Local's view number (the Rust source's LOCAL; version 11).</summary>
    public const ulong Local = 2;
    /// <summary>The panes of a composite frame (the Rust source's M6H_PANES_MAX).</summary>
    public const uint PanesMax = 4;

    /// <summary>Opens Local beside the view (version 11); each adoption then publishes both. Local is never closed.
    /// </summary>
    public int OpenLocal(IntPtr host) => ((delegate* unmanaged[Cdecl]<IntPtr, int>)_exports["m6h_open_local"])(host);

    /// <summary>Sets a view's 4D rotation, 16 floats column-major (version 11; <see cref="SetCamera"/> for any view).
    /// </summary>
    public int SetViewCamera(IntPtr host, ulong view, ReadOnlySpan<float> matrix)
    {
        if (matrix.Length != 16)
        {
            throw new ArgumentException("a camera is 16 floats", nameof(matrix));
        }
        fixed (float* m = matrix)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, ulong, float*, int>)_exports["m6h_set_view_camera"])(host, view, m);
        }
    }

    /// <summary>Shows or hides an open Local (version 11): a hidden Local gets no pane and no draw.</summary>
    public int SetViewVisible(IntPtr host, ulong view, bool visible) =>
        ((delegate* unmanaged[Cdecl]<IntPtr, ulong, uint, int>)_exports["m6h_set_view_visible"])(host, view, visible ? 1u : 0u);

    /// <summary>
    /// Sets pane 0 to 3 (version 11): a view drawn into a rectangle of the surface, in surface pixels, with style slot
    /// 0 (none) to 4. While any pane is set, frames are composite frames; panes lie inside the viewport and never
    /// overlap.
    /// </summary>
    public int SetPane(IntPtr host, uint pane, ulong view, int x, int y, uint width, uint height, uint style) =>
        ((delegate* unmanaged[Cdecl]<IntPtr, uint, ulong, int, int, uint, uint, uint, int>)_exports["m6h_set_pane"])(
            host, pane, view, x, y, width, height, style);

    /// <summary>Clears pane 0 to 3 (version 11); with every pane clear, frames are one-view frames again.</summary>
    public int ClearPane(IntPtr host, uint pane) =>
        ((delegate* unmanaged[Cdecl]<IntPtr, uint, int>)_exports["m6h_clear_pane"])(host, pane);

    /// <summary>ADR-D02 B21: a set pane's core flags, 0 or <see cref="PaneAntipode"/>; a later SetPane resets them.</summary>
    public int SetPaneFlags(IntPtr host, uint pane, uint flags) =>
        ((delegate* unmanaged[Cdecl]<IntPtr, uint, uint, int>)_exports["m6h_set_pane_flags"])(host, pane, flags);

    /// <summary>ADR-D02 B21: the raw view's pane flag, its stereographic chart after -I (SA2_PANE_ANTIPODE).</summary>
    public const uint PaneAntipode = 1;

    /// <summary>The focus's perspective projection (the shape's d4).</summary>
    public const uint ProjectionPerspective = 0;
    /// <summary>The focus's orthographic projection.</summary>
    public const uint ProjectionOrthographic = 1;
    /// <summary>No focus cell: every sticker is drawn.</summary>
    public const uint FocusNone = uint.MaxValue;
    /// <summary>ADR-D02 B21: the raw view's stereographic chart from the focus pole; it needs a focus cell and 0 &lt;= focus_cos.</summary>
    public const uint ProjectionStereographic = 2;

    /// <summary>
    /// Sets the focus a view's frames draw with (version 12): the projection, the cell whose centre is the pole (turned
    /// to +w before the view's camera) or <see cref="FocusNone"/>, the least dot product with the pole of a drawn
    /// sticker's posed centre, and the spread (1 to 3) that pulls the cells apart. (perspective, none, -1, 1) is the
    /// default. A view that is not open or a value out of range answers <see cref="InvalidArgument"/>.
    /// </summary>
    public int SetViewFocus(IntPtr host, ulong view, uint projection, uint focusCell, float focusCos, float spread) =>
        ((delegate* unmanaged[Cdecl]<IntPtr, ulong, uint, uint, float, float, int>)_exports["m6h_set_view_focus"])(
            host, view, projection, focusCell, focusCos, spread);

    /// <summary>A pick's states (the Rust M6H_PICK_*) and the host's "none" (M6H_NONE).</summary>
    public const uint PickPending = 0, PickReady = 1, PickStale = 2, None = uint.MaxValue;

    /// <summary>What m6h_pick_poll answers (the Rust M6hPick, version 14).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct Pick
    {
        public uint State;
        /// <summary>1 when a sticker of the view was under the point.</summary>
        public uint Hit;
        /// <summary>The view whose pane holds the point.</summary>
        public ulong View;
        /// <summary>The engine revision the picked frame drew.</summary>
        public ulong EngineRevision;
        /// <summary>The sticker hit, as its home slot; <see cref="None"/> without a hit.</summary>
        public uint Sticker;
        public uint Piece;
        /// <summary>The slot the sticker occupies; <see cref="None"/> for a piece off the lattice.</summary>
        public uint Slot;
        /// <summary>1 when the piece's pose is on the lattice.</summary>
        public uint Lattice;
    }

    /// <summary>
    /// Asks the core's ID pass at surface pixel (x, y) of the newest frame (version 14, ADR-D02 B17); writes the ticket
    /// <see cref="PickPoll"/> answers.
    /// </summary>
    public int PickRequest(IntPtr host, uint x, uint y, out ulong ticket)
    {
        ulong t = 0;
        var code = ((delegate* unmanaged[Cdecl]<IntPtr, uint, uint, ulong*, int>)_exports["m6h_pick_request"])(host, x, y, &t);
        ticket = t;
        return code;
    }

    /// <summary>Answers a pick without waiting (version 14): pending, ready (with or without a hit) or stale.</summary>
    public int PickPoll(IntPtr host, ulong ticket, out Pick pick)
    {
        Pick p = default;
        var code = ((delegate* unmanaged[Cdecl]<IntPtr, ulong, Pick*, int>)_exports["m6h_pick_poll"])(host, ticket, &p);
        pick = p;
        return code;
    }

    /// <summary>The size of a view layout row (sa2_view_layout).</summary>
    public const int LayoutBytes = 1560;

    /// <summary>
    /// Sets the layout a view's frames draw with (version 13): null for the projection chain, or a sa2_view_layout row of
    /// <see cref="LayoutBytes"/> bytes (D02.ShellHopf builds it from Look Lab's hopf-rings.json). A view that is not
    /// open, a row of another size or rings that do not fit the core's cells answer a refusal.
    /// </summary>
    public int SetViewLayout(IntPtr host, ulong view, byte[]? layout)
    {
        var call = (delegate* unmanaged[Cdecl]<IntPtr, ulong, byte*, int>)_exports["m6h_set_view_layout"];
        if (layout is null)
        {
            return call(host, view, null);
        }
        if (layout.Length != LayoutBytes)
        {
            return InvalidArgument;
        }
        fixed (byte* row = layout)
        {
            return call(host, view, row);
        }
    }

    /// <summary>
    /// Sets the shape that a view's frames draw with (version 9): the cell shrink, the sticker shrink, d4 and the zoom,
    /// or null for the core's defaults (0.76, 0.82, 1.18 and 1.15, the shape every view has until this call). A view
    /// that is not open or a value out of the core's range answers <see cref="InvalidArgument"/> and changes nothing.
    /// </summary>
    public int SetViewShape(IntPtr host, ulong view, float[]? shape)
    {
        if (shape is not null && shape.Length != 4)
        {
            throw new ArgumentException("a view shape is 4 floats", nameof(shape));
        }
        fixed (float* s = shape)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, ulong, float*, int>)_exports["m6h_set_view_shape"])(host, view, s);
        }
    }

    /// <summary>A style slot as the core's sa2_view_style lays it out (348 bytes; version 10, with B20's slice).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct ViewStyle
    {
        public uint StructSize, TintMode, DimMode;
        public float DimAmount, OutlineWidth;
        public fixed float Palette[64];
        public fixed float Matrix[9];
        /// <summary>ADR-D02 B20: the cut slice (<see cref="SliceOff"/> to <see cref="SliceInside"/>), its inside layer's
        /// shrink factor, its radius shell about the focus pole (angles) and the turn preview's ghost colour.</summary>
        public uint SliceMode;
        public float SliceShrink, ShellMin, ShellMax;
        public fixed float Ghost[4];
        public uint Reserved;

        /// <summary>A slot that changes nothing: no tint, no dim, an empty palette, the identity matrix.</summary>
        public static ViewStyle Plain()
        {
            var style = new ViewStyle { StructSize = (uint)sizeof(ViewStyle) };
            style.Matrix[0] = style.Matrix[4] = style.Matrix[8] = 1;
            return style;
        }
    }

    /// <summary>The piece style's channels and the tint and dim modes (the core's SA2_STYLE_* values).</summary>
    public const uint StyleFill = 0, StyleOutline = 1, TintFill = 1, TintOutline = 2, DimMarked = 1, DimUnmarked = 2;

    /// <summary>ADR-D02 B17 (the core's ABI 10): the dim modes that hide instead; a hidden piece is neither drawn nor picked.</summary>
    public const uint HideMarked = 3, HideUnmarked = 4;

    /// <summary>ADR-D02 B20: the piece style's slice channel (one byte per piece, 1 = the inside layer) and the slice
    /// modes: off, both layers, the outside layer only (the split's left copy), the inside layer only (its right).</summary>
    public const uint StyleSlice = 2, SliceOff = 0, SliceBoth = 1, SliceOutside = 2, SliceInside = 3;

    /// <summary>A fill byte's dim-set bit (the core's SA2_STYLE_DIM): the pieces a dim or hide mode acts on.</summary>
    public const byte DimSet = 0x80;
    /// <summary>An outline byte's view-only error overlay bit (the core's SA2_STYLE_ERROR).</summary>
    public const byte StyleError = 0x10;

    /// <summary>
    /// Sets one channel of the piece style of engine revision <paramref name="revision"/>, the one it was computed for
    /// (version 10; checkpoint C3.2's revision binding): one byte per piece, or with <paramref name="byPose"/> one byte
    /// per pose of the session's head, which the host expands with the head's own pose index; null clears the channel.
    /// The adopted revision's style is set at once; one for a revision handed out but not adopted yet is held and set
    /// with its adoption; an older one answers <see cref="Unavailable"/>. The core drops it at the next adoption, so it is
    /// set again for each shown revision.
    /// </summary>
    public int SetPieceStyle(IntPtr host, ulong revision, uint channel, byte[]? styles, bool byPose)
    {
        fixed (byte* s = styles)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, ulong, uint, byte*, uint, uint, int>)_exports["m6h_set_piece_style"])(
                host, revision, channel, s, (uint)(styles?.Length ?? 0), byPose ? 1u : 0u);
        }
    }

    /// <summary>
    /// ADR-D02 B18 (for B20): sets the adopted revision's twist transform: the plane (u then v, 8 floats), the angle, one
    /// byte per piece (1 moves) and the phase in 0..1; the moving pieces draw turned by R((phase - 1) angle) after their
    /// pose, so phase 1 draws the revision as it is. Null clears it. The core drops it at the next adoption.
    /// <paramref name="revision"/> is the row's (<see cref="TwistRow.Revision"/>): the host sets it only while that
    /// revision is the adopted one (B18 review finding B18-04). A plane that is not 8 floats, or a moving set without
    /// one, answers <see cref="InvalidArgument"/> before the host is called, so it never reads past a short array
    /// (B18-01).
    /// </summary>
    public int SetTransform(IntPtr host, ulong revision, float[]? plane, float angle, byte[]? moving, float phase)
    {
        if (plane is { Length: not 8 } || (moving is not null && plane is null))
        {
            return InvalidArgument;
        }
        fixed (float* p = plane)
        fixed (byte* m = moving)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, ulong, float*, float, byte*, uint, float, int>)_exports["m6h_set_transform"])(
                host, revision, p, angle, m, (uint)(moving?.Length ?? 0), phase);
        }
    }

    /// <summary>
    /// A twist's certified transform: its plane (u then v), its angle in (0, pi], its moving pieces and the engine
    /// revision of the head it was read at, which <see cref="SetTransform"/> names.
    /// </summary>
    public sealed record TwistRow(float[] Plane, float Angle, byte[] Moving, ulong Revision);

    /// <summary>
    /// ADR-D02 B18: the certified transform of the session's twist at <paramref name="cap"/> with menu item
    /// <paramref name="item"/>, not committed; null when the twist is not admissible or has no plane (the identity).
    /// </summary>
    public int TwistTransform(IntPtr host, uint cap, ulong item, out TwistRow? row, out ulong revision)
    {
        var plane = new float[8];
        var moving = new byte[177_120];
        float angle = 0;
        uint status = 0;
        ulong at = 0;
        int code;
        fixed (float* p = plane)
        fixed (byte* m = moving)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr, uint, ulong, float*, float*, byte*, uint*, ulong*, int>)_exports["m6h_twist_transform"])(
                host, cap, item, p, &angle, m, &status, &at);
        }
        revision = at;
        row = code == Ok && status == 1 ? new TwistRow(plane, angle, moving, at) : null;
        return code;
    }

    /// <summary>Sets style slot 1 to 4 (a one-view frame uses slot 1), or clears it with null (version 10).</summary>
    public int SetViewStyle(IntPtr host, uint slot, ViewStyle? style)
    {
        var call = (delegate* unmanaged[Cdecl]<IntPtr, uint, ViewStyle*, int>)_exports["m6h_set_view_style"];
        if (style is not { } value)
        {
            return call(host, slot, null);
        }
        return call(host, slot, &value);
    }

    public int Pump(IntPtr host) => ((delegate* unmanaged[Cdecl]<IntPtr, int>)_exports["m6h_pump"])(host);

    /// <summary>Submits a frame of the view to the surface; <paramref name="frame"/> names it.</summary>
    public int Frame(IntPtr host, out Submitted frame)
    {
        Submitted f = default;
        var code = ((delegate* unmanaged[Cdecl]<IntPtr, Submitted*, int>)_exports["m6h_frame"])(host, &f);
        frame = f;
        return code;
    }

    /// <summary>Reads up to rows.Length confirmed presentations; read again while the count fills the span.</summary>
    public int ReadPresented(IntPtr host, Span<Presented> rows, out int count)
    {
        uint n = 0;
        int code;
        fixed (Presented* r = rows)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr, Presented*, uint, uint*, int>)_exports["m6h_read_presented"])(
                host, r, (uint)rows.Length, &n);
        }
        count = (int)n;
        return code;
    }

    public int Recover(IntPtr host) => ((delegate* unmanaged[Cdecl]<IntPtr, int>)_exports["m6h_recover"])(host);

    /// <summary>
    /// Opens the turns' session of <paramref name="puzzle"/> (<see cref="PuzzleJumbleOff"/> or
    /// <see cref="PuzzleJumbleOn"/>) with the twist menu of the record file <paramref name="menu"/> pinned. Without a
    /// <paramref name="fixture"/> it is a new journal in <paramref name="folder"/> (created if missing; it must hold
    /// nothing) at the solved state; with Jumble on it commits jumble.on at once. With one (the path of a session
    /// fixture's fixture.json, of the same puzzle), <paramref name="folder"/> is a fresh copy of the fixture's
    /// session, which the host reopens and adopts at the revision its record names. <paramref name="revision"/> is
    /// the engine revision the session opened at.
    /// </summary>
    public int OpenSession(IntPtr host, uint puzzle, string folder, string menu, string? fixture, out ulong revision)
    {
        var f = Encoding.UTF8.GetBytes(folder);
        var m = Encoding.UTF8.GetBytes(menu);
        var x = fixture is null ? null : Encoding.UTF8.GetBytes(fixture);
        ulong opened = 0;
        int code;
        fixed (byte* fb = f)
        fixed (byte* mb = m)
        fixed (byte* xb = x)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr, uint, byte*, nuint, byte*, nuint, byte*, nuint, ulong*, int>)
                _exports["m6h_open_session"])(host, puzzle, fb, (nuint)f.Length, mb, (nuint)m.Length, xb,
                (nuint)(x?.Length ?? 0), &opened);
        }
        revision = opened;
        return code;
    }

    /// <summary>
    /// Starts the language runtime's process from <paramref name="program"/> (the c600-runtime binary) and waits for
    /// its hello; the host keeps it until <see cref="Destroy"/>.
    /// </summary>
    public int LoadRuntime(IntPtr host, string program)
    {
        var p = Encoding.UTF8.GetBytes(program);
        fixed (byte* pb = p)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, int>)_exports["m6h_load_runtime"])(
                host, pb, (nuint)p.Length);
        }
    }

    /// <summary>
    /// Schedules the program text <paramref name="source"/> as the view's job, run from its exported function
    /// <paramref name="entry"/> in the loaded runtime; returns at once. <see cref="Pump"/> or <see cref="Frame"/>
    /// delivers the run's publication when it ends; <see cref="GetViewResult"/> reads the record of
    /// <paramref name="ticket"/>.
    /// </summary>
    public int RunView(IntPtr host, string source, string entry, out ulong ticket)
    {
        var s = Encoding.UTF8.GetBytes(source);
        var e = Encoding.UTF8.GetBytes(entry);
        ulong t = 0;
        int code;
        fixed (byte* sb = s)
        fixed (byte* eb = e)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, byte*, nuint, ulong*, int>)
                _exports["m6h_run_view"])(host, sb, (nuint)s.Length, eb, (nuint)e.Length, &t);
        }
        ticket = t;
        return code;
    }

    /// <summary>
    /// Reads a view job's or draft's record; a refused one's code and reason are then in <see cref="LastError"/>.
    /// </summary>
    public int GetViewResult(IntPtr host, ulong ticket, out ViewResult result)
    {
        ViewResult r = default;
        var code = ((delegate* unmanaged[Cdecl]<IntPtr, ulong, ViewResult*, int>)_exports["m6h_view_result"])(
            host, ticket, &r);
        result = r;
        return code;
    }

    /// <summary>
    /// Names the editor's text by its SHA-256 <paramref name="digest"/> without running it, so that the view reads
    /// stale while its output came from another text. A draft cancels no run.
    /// </summary>
    public int ViewDraft(IntPtr host, ReadOnlySpan<byte> digest, out ulong ticket)
    {
        if (digest.Length != 32)
        {
            throw new ArgumentException("a digest is 32 bytes", nameof(digest));
        }
        ulong t = 0;
        int code;
        fixed (byte* d = digest)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr, byte*, ulong*, int>)_exports["m6h_view_draft"])(host, d, &t);
        }
        ticket = t;
        return code;
    }

    /// <summary>Routes one key-down, <paramref name="chord"/> in c600-input's form (Shift+KeyJ).</summary>
    public int KeyDown(IntPtr host, string chord, bool repeat, out KeyResult key)
    {
        var c = Encoding.UTF8.GetBytes(chord);
        KeyResult k = default;
        int code;
        fixed (byte* cb = c)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, uint, KeyResult*, int>)_exports["m6h_key_down"])(
                host, cb, (nuint)c.Length, repeat ? 1u : 0u, &k);
        }
        key = k;
        return code;
    }

    /// <summary>Routes a key-up of <paramref name="chord"/>; its modifiers are ignored.</summary>
    public int KeyUp(IntPtr host, string chord)
    {
        var c = Encoding.UTF8.GetBytes(chord);
        fixed (byte* cb = c)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, int>)_exports["m6h_key_up"])(
                host, cb, (nuint)c.Length);
        }
    }

    /// <summary>The window lost the keyboard focus: the router releases its held keys.</summary>
    public int FocusLost(IntPtr host) => ((delegate* unmanaged[Cdecl]<IntPtr, int>)_exports["m6h_focus_lost"])(host);

    /// <summary>
    /// Tells the session's router what has the keyboard focus inside the window: <see cref="FocusText"/> while a text
    /// field has it, <see cref="FocusPuzzle"/> otherwise. Held keys are released.
    /// </summary>
    public int SetFocus(IntPtr host, uint focus) =>
        ((delegate* unmanaged[Cdecl]<IntPtr, uint, int>)_exports["m6h_set_focus"])(host, focus);

    /// <summary>
    /// Registers the entry <paramref name="entry"/> of the user module <paramref name="module"/> as a language
    /// overlay row (tool.module.entry, a query row) labelled <paramref name="label"/>, with the key
    /// <paramref name="chord"/> in c600-input's form, or none when it is empty. <see cref="Answer"/> gives {"row"}.
    /// </summary>
    public int RegisterRow(IntPtr host, string module, string entry, string label, string chord)
    {
        var m = Encoding.UTF8.GetBytes(module);
        var e = Encoding.UTF8.GetBytes(entry);
        var l = Encoding.UTF8.GetBytes(label);
        var c = Encoding.UTF8.GetBytes(chord);
        fixed (byte* mb = m)
        fixed (byte* eb = e)
        fixed (byte* lb = l)
        fixed (byte* cb = c)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, byte*, nuint, byte*, nuint, byte*, nuint, int>)
                _exports["m6h_register_row"])(host, mb, (nuint)m.Length, eb, (nuint)e.Length, lb, (nuint)l.Length,
                cb, (nuint)c.Length);
        }
    }

    /// <summary>Reads the session's palette; <see cref="Answer"/> gives {"revision", "generation", "entries"}.</summary>
    public int Palette(IntPtr host) => ((delegate* unmanaged[Cdecl]<IntPtr, int>)_exports["m6h_palette"])(host);

    /// <summary>
    /// Invokes <paramref name="row"/> from the palette with the absolute arguments <paramref name="args"/> (a JSON
    /// object, or empty for none), through the shell. <paramref name="job"/> is the program job of a running
    /// receipt, or 0; <see cref="Answer"/> gives the receipt.
    /// </summary>
    public int Invoke(IntPtr host, string row, string args, out ulong job)
    {
        var r = Encoding.UTF8.GetBytes(row);
        var a = Encoding.UTF8.GetBytes(args);
        ulong j = 0;
        int code;
        fixed (byte* rb = r)
        fixed (byte* ab = a)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, byte*, nuint, ulong*, int>)_exports["m6h_invoke"])(
                host, rb, (nuint)r.Length, a.Length == 0 ? null : ab, (nuint)a.Length, &j);
        }
        job = j;
        return code;
    }

    /// <summary>
    /// Takes the next program job of the session's layer, a language row's: <paramref name="job"/> is its ID, or 0
    /// for none, and <see cref="Answer"/> gives the run it names, which the editor library runs.
    /// </summary>
    public int TakeJob(IntPtr host, out ulong job)
    {
        ulong j = 0;
        var code = ((delegate* unmanaged[Cdecl]<IntPtr, ulong*, int>)_exports["m6h_take_job"])(host, &j);
        job = j;
        return code;
    }

    /// <summary>
    /// Reports the program job <paramref name="job"/>: with an empty <paramref name="errorCode"/> the run's output
    /// <paramref name="value"/> as its result, otherwise the error that ended its run. <see cref="Answer"/> gives the
    /// receipt of the command that invoked the job, or null.
    /// </summary>
    public int FinishJob(IntPtr host, ulong job, string value, string errorCode, string detail)
    {
        var v = Encoding.UTF8.GetBytes(value);
        var c = Encoding.UTF8.GetBytes(errorCode);
        var d = Encoding.UTF8.GetBytes(detail);
        fixed (byte* vb = v)
        fixed (byte* cb = c)
        fixed (byte* db = d)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, ulong, byte*, nuint, byte*, nuint, byte*, nuint, int>)
                _exports["m6h_finish_job"])(host, job, v.Length == 0 ? null : vb, (nuint)v.Length,
                c.Length == 0 ? null : cb, (nuint)c.Length, d.Length == 0 ? null : db, (nuint)d.Length);
        }
    }

    /// <summary>
    /// Sets the editor's draft, the word <paramref name="letters"/>, which operation.preview's key previews; an empty
    /// word clears it. Setting it previews nothing.
    /// </summary>
    public int SetDraftWord(IntPtr host, ReadOnlySpan<int> letters)
    {
        fixed (int* l = letters)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, int*, nuint, int>)_exports["m6h_set_draft_word"])(
                host, letters.Length == 0 ? null : l, (nuint)letters.Length);
        }
    }

    /// <summary>The JSON answer of the last call of the write-check-run flow.</summary>
    public string Answer(IntPtr host) => Text(host, "m6h_answer");

    /// <summary>
    /// Reads the head's state digests (ADR-D02 B5); <see cref="Answer"/> gives {"digest", "pose_ids_sha256",
    /// "pose_lattice_sha256"}, the last two null with Jumble off.
    /// </summary>
    public int StateDigest(IntPtr host) => ((delegate* unmanaged[Cdecl]<IntPtr, int>)_exports["m6h_state_digest"])(host);

    /// <summary>
    /// Closes the session and runs its correctness control once. <paramref name="shownRevision"/> is the engine
    /// revision of the newest presented frame (0 for none); <see cref="Check"/> gives the control's record.
    /// </summary>
    public int CloseSession(IntPtr host, ulong shownRevision, out bool pass)
    {
        uint p = 0;
        var code = ((delegate* unmanaged[Cdecl]<IntPtr, ulong, uint*, int>)_exports["m6h_close_session"])(
            host, shownRevision, &p);
        pass = code == Ok && p == 1;
        return code;
    }

    /// <summary>The correctness control's record as canonical JSON, empty before the close.</summary>
    public string Check(IntPtr host) => Text(host, "m6h_check");

    /// <summary>The reason for the handle's last failed call, or with a zero handle for the last failed create.</summary>
    public string LastError(IntPtr host) => Text(host, "m6h_last_error");

    private string Text(IntPtr host, string export)
    {
        var call = (delegate* unmanaged[Cdecl]<IntPtr, byte*, uint, uint*, int>)_exports[export];
        uint size = 0;
        call(host, null, 0, &size);
        if (size == 0)
        {
            return string.Empty;
        }
        var buffer = new byte[size];
        fixed (byte* b = buffer)
        {
            if (call(host, b, size, &size) != Ok)
            {
                return string.Empty;
            }
        }
        return Encoding.UTF8.GetString(buffer, 0, (int)size - 1);
    }

    /// <summary>Frees a host. The core's context stays the shell's to detach afterwards.</summary>
    public void Destroy(IntPtr host) => ((delegate* unmanaged[Cdecl]<IntPtr, void>)_exports["m6h_destroy"])(host);

    /// <summary>Sets one encoded RGB triple per home cell, or clears the table with null or an empty array.</summary>
    public int SetStickerColours(IntPtr host, float[]? rgb)
    {
        const int count = StickerColouring.CellCount;
        if (rgb is { Length: > 0 } && rgb.Length != 3 * count)
        {
            return InvalidArgument;
        }
        fixed (float* p = rgb)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, float*, uint, int>)_exports["m6h_set_sticker_colours"])(
                host, p, p == null ? 0u : count);
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct SceneLook
    {
        public uint StructSize, Reserved;
        public float Red, Green, Blue, Alpha;
        public float Fog, Glow, EdgeBrightness, EdgeWeight;
    }

    /// <summary>Sets the clear colour and the four carried scene values. Null restores the core's defaults.</summary>
    public int SetSceneLook(IntPtr host, SceneLookValues? look)
    {
        var call = (delegate* unmanaged[Cdecl]<IntPtr, SceneLook*, int>)_exports["m6h_set_scene_look"];
        if (look is null)
        {
            return call(host, null);
        }
        var row = new SceneLook
        {
            StructSize = (uint)sizeof(SceneLook), Reserved = 0,
            Red = (float)look.Background.R, Green = (float)look.Background.G, Blue = (float)look.Background.B, Alpha = 1,
            Fog = (float)look.Fog, Glow = (float)look.Glow,
            EdgeBrightness = (float)look.EdgeBrightness, EdgeWeight = (float)look.EdgeWeight,
        };
        return call(host, &row);
    }

    public void Dispose()
    {
        if (_library != IntPtr.Zero)
        {
            NativeLibrary.Free(_library);
            _library = IntPtr.Zero;
        }
    }
}
