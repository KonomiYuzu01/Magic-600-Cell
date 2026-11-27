using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Shell;

/// <summary>
/// The shell's root node. The window's background is transparent, so that the core's composition layer beneath the
/// window shows through where the UI draws nothing (route B, accepted for now by the owner on 18 Nov 2026, pending
/// three owner-machine checks).
/// </summary>
/// <remarks>
/// Step S4, a draft that was never launched: when the environment names the core library (M600_CORE_DLL), the host
/// library (M600_HOST_DLL) and the assets folder (M600_ASSETS), each an absolute path, the shell opens the core's
/// surface under the whole window, creates the host over it and submits one frame per process step. Without them it
/// shows the empty window as before. Step S5: when M600_EVENTS_DIR is set as well, <see cref="EventLog"/> records
/// every submitted frame and every confirmed presentation with its display time. Route B's check B1: when
/// M600_INPUT_ECHO is 1, a panel over the view echoes every key press and mouse click the window receives, and
/// <see cref="InputLog"/> records them; the echo works with or without the core. Charter E2's turns (a draft): when
/// M600_SESSION_DIR (a new folder) and M600_TWIST_MENU (the twist menu's record file) are set as well, the host opens
/// a session there, every key with a W3C code goes through its input router (<see cref="KeyChord"/>), a committed
/// twist is adopted and shown, <see cref="EventLog"/> writes a key row per measured press, and the close runs the
/// session's correctness control. C9 U3 (a draft): with the event log, the run row names the core's swap chain, and
/// every Godot frame that follows a submitted 3D frame gets a ui_frame row (<see cref="EventLog"/> says when its times
/// are read). C9 U8 (a draft): the run row names the window and the puzzle (M600_PUZZLE; Jumble on runs only with a
/// session, which the host opens in the jumbled puzzle), and when M600_LX4 is 1 the shell creates LX-4's module
/// (<see cref="IShellModule"/>), which receives
/// the window's messages through <see cref="WindowHook"/> before Godot does, and every submitted and presented frame.
/// Charter E3 (C9 U6, a draft): when M600_SESSION_FIXTURE names a session fixture's record, M600_SESSION_DIR is a
/// fresh copy of its session, which the host reopens at the revision that counts the fixture's presses; when
/// M600_RUNTIME names the c600-runtime binary, the host starts the language runtime's process; with the event log,
/// <see cref="StartupLog"/> records both and the first correct frame, and when M600_EXIT_AT_FIRST_FRAME is 1 the shell
/// quits once that frame is shown. LP4-3 (a draft): when M600_EDITOR is 1 the shell creates the editor panel
/// (<see cref="Editor.EditorPanel"/>), the write-check-run flow of a .6cs macro over the editor library
/// (M600_EDITOR_DLL); while its code field has the focus, the keys go on to it as well as to the session's router.
/// </remarks>
public partial class Shell : Control
{
    private CoreLibrary? _core;
    private HostLibrary? _hostLibrary;
    private CoreSurface? _surface;
    /// <summary>The host's model loads (charter E3's startup), from m6h_preload until m6h_create_preloaded takes them.
    /// </summary>
    private IntPtr _preload;
    private IntPtr _host;
    private EventLog? _log;
    private InputLog? _input;
    private Label? _echo;
    private long _presses;
    private long _clicks;
    private string _lastPress = "none";
    private string _lastClick = "none";
    private bool _hadFocus;
    private bool _focused;
    private bool _session;
    /// <summary>Brief SF2: posted keys waiting for their commit answer, by ticket.</summary>
    private readonly Dictionary<ulong, PostedKey> _posted = new();

    private sealed record PostedKey(string Chord, long Qpc, bool StaticMotion, long Arming, long Posted);
    /// <summary>The engine revision of the newest presentation the core confirmed.</summary>
    private ulong _shownRevision;
    /// <summary>The shown revision at which ADR-D02 last bound a sample state to the session (B5).</summary>
    private ulong _boundRevision;
    // The engine revision the last submitted frame drew: the adopted head, shown on screen or not.
    private ulong _adoptedRevision;
    private Vector2I _size;
    /// <summary>The 3D frame submitted in this process frame, for the next ui_frame row.</summary>
    private ulong? _submittedThisFrame;
    private long _preDrawQpc;
    private bool _uiFrames;
    private readonly List<IShellModule> _modules = new();
    /// <summary>LP4-3's editor panel, one of <see cref="_modules"/> when M600_EDITOR is 1.</summary>
    private Editor.EditorPanel? _editor;
    private StartupLog? _startup;
    /// <summary>The engine revision the session opened at (charter E3's R0).</summary>
    private ulong _openRevision;
    /// <summary>The first submitted frame that draws R0 over a publication of R0 or later.</summary>
    private ulong? _firstSubmitted;
    /// <summary>The first confirmed presentation of that frame or a later one: its sequence and display time.</summary>
    private (ulong FrameId, ulong DisplayQpc)? _firstShown;
    private bool _exitAtFirstFrame;
    private readonly HostLibrary.Presented[] _presented = new HostLibrary.Presented[16];
    /// <summary>The frame (A5): the chrome library over the window. Null unless M600_CHROME is 1.</summary>
    private ShellChrome? _chrome;
    private bool _contextMenuPress;
    /// <summary>True while the context menu holds the pointer: Turn and ViewAt then answer nothing.</summary>
    private bool _menuPointer;
    /// <summary>The window rectangle the core's surface has, the one the frame's opening asks for, and for how many
    /// process steps it has asked for it.</summary>
    private Rect2I _area;
    // ADR-D02 (owner, 26 November 2026): with --d02 a drag in the Global opening turns the camera as Look Lab's does.
    private static D02.ShellCamera _camera => D02.ShellJumble.Camera;
    private Rect2I _wanted;
    private int _steady;
    // ADR-D02 B18c: Local is open (with --d02), the cap it is focused on (-1 before the first step), and the two openings
    // the pane table was last set for (both empty while one-view frames are drawn).
    private bool _localOpen;
    // ADR-D02 B23: Global is drawn as the Hopf rings, so its camera is the linked 3D spin (the ring sheet ignores it).
    private bool _hopf;
    private int? _localCap = -1;
    private bool _localHidden;
    private const string HiddenLocalWords = "Local is hidden (view.local shows it again on its cap, linked to Global).";
    private (Rect2I Global, Rect2I Local) _paned;
    private bool _panedSplit; // ADR-D02 B20: whether the pane table holds the held split's two copies
    private (bool On, bool Antipode) _panedRaw; // ADR-D02 B21: the raw view the pane table holds
    private double? _localRaw; // ADR-D02 B21: the raw view's clip Local's focus was sent with (null: no raw view)
    private Magic600.Chrome.PaneLayout? _layout;

    public override void _Ready()
    {
        // ADR-D02's watched start (B4): when Godot activated the window, the foreground goes straight back.
        if (D02.ShellJumble.On)
        {
            D02.D02Window.Settle();
        }
        GetViewport().TransparentBg = true;
        if (InputLog.Requested())
        {
            CreateEcho();
            try
            {
                _input = InputLog.FromEnvironment();
            }
            catch (Exception error)
            {
                GD.PushError($"the input log did not open: {error.Message}");
            }
        }
        var core = System.Environment.GetEnvironmentVariable("M600_CORE_DLL");
        var host = System.Environment.GetEnvironmentVariable("M600_HOST_DLL");
        var assets = System.Environment.GetEnvironmentVariable("M600_ASSETS");
        if (string.IsNullOrEmpty(core) || string.IsNullOrEmpty(host) || string.IsNullOrEmpty(assets))
        {
            OpenChrome();
            return;
        }
        try
        {
            Open(core, host, assets);
        }
        catch (Exception error)
        {
            GD.PushError($"the core did not open: {error.Message}");
            CloseCore();
        }
        OpenChrome();
    }

    /// <summary>
    /// Opens the frame (A5): the chrome library and the design data that build_shell.py staged, with the Global view as
    /// an opening over the core's picture when the core is open. Without M600_CHROME=1 the window stays bare, as every measured run has it.
    /// </summary>
    private void OpenChrome()
    {
        if (System.Environment.GetEnvironmentVariable("M600_CHROME") != "1" || ShellChrome.DesignFolder() is not { } folder)
        {
            return;
        }
        try
        {
            _chrome = new ShellChrome(this, folder, _host != IntPtr.Zero, () => RotationLinked);
            if (_chrome.Sample is { } sample && D02.ShellJumble.On)
            {
                WireSample(sample);
            }
            WireGrip();
            if (_echo is { } echo)
            {
                MoveChild(echo, -1);
            }
        }
        catch (Exception error)
        {
            GD.PushError($"the frame did not open: {error.Message}");
        }
    }

    /// <summary>ADR-D01.41: the rotation link the frames mark: none without Local, else whether Local is linked.</summary>
    internal bool? RotationLinked => _localOpen ? D02.ShellLocal.Linked : null;

    // ADR-D01.22 under --d02: Global's shape as last sent (cell shrink, sticker shrink, d4 and the opening's zoom, which
    // the wheel scales) and its projection, or null while the core is not open; and each view's frame field as last seen.
    private float[]? _globalShape;
    private uint _globalProjection = HostLibrary.ProjectionPerspective;
    private readonly Dictionary<string, double> _frameSeen = new(StringComparer.Ordinal);

    /// <summary>
    /// Sample mode under --d02: the hooks of the shared library that the host sets. ADR-D01.62: the strip's anchors sit
    /// where the core draws each view (its own projection, ported to Chrome/Model/CorePlaces.cs). ADR-D01.22: a View
    /// edit in the inspector is drawn on the view it is written for, and Global's section starts from what is drawn.
    /// </summary>
    private void WireSample(LookLab.App.Live.SampleMount sample)
    {
        var opening = D02.ShellShape.Read(D02.ShellShape.OpeningPath());
        if (sample.Places is { } places)
        {
            sample.GlobalView = () => _hopf || _chrome?.Opening() is not { Size.X: > 0, Size.Y: > 0 } g ? null
                : GlobalProjection(opening, (double)g.Size.X / g.Size.Y);
            sample.LocalView = () => _chrome?.LocalOpening() is { Size.X: > 0, Size.Y: > 0 } l
                ? LocalProjection(places, (double)l.Size.X / l.Size.Y) : null;
        }
        if (_host == IntPtr.Zero || _hostLibrary is null)
        {
            return;
        }
        // Global's shape was set from opening.json when the core opened (Open): the section shows what is drawn.
        _globalShape = opening;
        sample.Views.Sync("Global", Magic600.Chrome.ViewFields.CellShrink, opening[0]);
        sample.Views.Sync("Global", Magic600.Chrome.ViewFields.StickerShrink, opening[1]);
        foreach (var view in Magic600.Chrome.ViewFields.Views)
        {
            _frameSeen[view] = sample.Views.Value(view, Magic600.Chrome.ViewFields.Frame);
        }
        sample.ApplyView = ApplyView;
    }

    /// <summary>
    /// ADR-D01.22: draws a view's View fields on the core (cell shrink, sticker shrink and projection; 1 is
    /// orthographic) and answers why it did not, or null. Global's shape keeps the wheel's zoom; Local's new shape and
    /// focus are sent again by the follow code at the next step. The core draws no frame marks, so the frame field is
    /// answered with a reason.
    /// </summary>
    private string? ApplyView(string view, Magic600.Chrome.ViewFields fields)
    {
        if (_host == IntPtr.Zero || _hostLibrary is not { } library)
        {
            return "the core is not open";
        }
        var cell = (float)fields.Value(view, Magic600.Chrome.ViewFields.CellShrink);
        var sticker = (float)fields.Value(view, Magic600.Chrome.ViewFields.StickerShrink);
        var projection = fields.Value(view, Magic600.Chrome.ViewFields.Projection) == 1
            ? HostLibrary.ProjectionOrthographic : HostLibrary.ProjectionPerspective;
        if (view == "Global" && _globalShape is { } global)
        {
            var shape = new[] { cell, sticker, global[2], global[3] };
            var code = D02.ShellShape.Set(library, _host, HostLibrary.View, shape);
            if (code == HostLibrary.Ok)
            {
                code = D02.ShellShape.ApplyZoom(library, _host, HostLibrary.View, D02.ShellJumble.Zoom.Factor(HostLibrary.View));
            }
            if (code == HostLibrary.Ok)
            {
                code = library.SetViewFocus(_host, HostLibrary.View, projection, HostLibrary.FocusNone, -1, 1);
            }
            if (code != HostLibrary.Ok)
            {
                return $"the core refused it ({code}: {library.LastError(_host)})";
            }
            _globalShape = shape;
            _globalProjection = projection;
        }
        else if (view == "Local" && _localOpen && _localShape is { } local)
        {
            _localShape = local with { Projection = projection, Gap = 1 - cell, StickerShrink = sticker };
            _localCap = -1;
            _paned = default;
        }
        else
        {
            return $"{view} is not open";
        }
        var frame = fields.Value(view, Magic600.Chrome.ViewFields.Frame);
        if (_frameSeen.TryGetValue(view, out var seen) && seen != frame)
        {
            _frameSeen[view] = frame;
            return "the core draws no frame marks yet; the value is kept here";
        }
        return null;
    }

    /// <summary>A row-major 4×4 rotation from one sent column-major as it lies (see <see cref="SpinOf"/>).</summary>
    private static double[] Transposed(double[] m)
    {
        var t = new double[16];
        for (var i = 0; i < 16; i++)
        {
            t[i] = m[i % 4 * 4 + i / 4];
        }
        return t;
    }

    private void Open(string core, string host, string assets)
    {
        var coreLibrary = _core = CoreLibrary.Load(core);
        var hostLibrary = _hostLibrary = HostLibrary.Load(host);
        var puzzle = EventLog.PuzzleFromEnvironment();
        var session = System.Environment.GetEnvironmentVariable("M600_SESSION_DIR");
        var menu = System.Environment.GetEnvironmentVariable("M600_TWIST_MENU");
        var withSession = !string.IsNullOrEmpty(session) && !string.IsNullOrEmpty(menu);
        if (puzzle != EventLog.Puzzles[0] && !withSession)
        {
            // The jumbled puzzle exists only in a session: the host enters it with jumble.on at the session's start.
            throw new NotSupportedException(
                $"the shell runs {puzzle} only with a session (M600_SESSION_DIR and M600_TWIST_MENU)");
        }
        // Charter E3's startup (a draft): the host loads the model on threads of its own while the core's surface
        // opens, and with a session the exact model the session's twist menu reads.
        Require(hostLibrary, "m6h_preload", hostLibrary.Preload(assets, withSession, out _preload));
        var hwnd = DisplayServer.WindowGetNativeHandle(DisplayServer.HandleType.WindowHandle);
        var window = (IntPtr)hwnd;
        _size = DisplayServer.WindowGetSize();
        _area = new Rect2I(Vector2I.Zero, _size);
        // The core opens Godot's own adapter, not DXGI's first hardware adapter (C9 U4).
        var device = RenderingServer.GetRenderingDevice()
            ?? throw new InvalidOperationException("the shell needs Godot's rendering device");
        var adapter = CoreSurface.AdapterLuid(device.GetDriverResource(RenderingDevice.DriverResource.PhysicalDevice, default, 0));
        var surface = _surface = CoreSurface.Open(coreLibrary, adapter, (ulong)hwnd, 0, 0, (uint)_size.X, (uint)_size.Y);
        var preload = _preload;
        _preload = IntPtr.Zero;
        var code = hostLibrary.CreatePreloaded(coreLibrary, surface.Context, preload, out _host);
        if (code != HostLibrary.Ok)
        {
            throw new InvalidOperationException(
                $"m6h_create_preloaded answered {code}: {hostLibrary.LastError(IntPtr.Zero)}");
        }
        if (D02.ShellJumble.On)
        {
            hostLibrary.BindPreviewCore(_host, core);
        }
        Require(hostLibrary, "m6h_set_viewport", hostLibrary.SetViewport(_host, (uint)_size.X, (uint)_size.Y));
        // ADR-D02 (owner, 26 November 2026: smaller stickers): with --d02 the view takes the owner's opening shape.
        Require(hostLibrary, "m6h_set_view_shape", D02.ShellShape.Apply(hostLibrary, _host));
        // ADR-D02 B23 and B22 (bare): with --d02 and an opening file that names the Hopf rings or the ring sheet, Global
        // opens on it; the rings turn with the linked 3D spin, the sheet lies flat.
        var layout = D02.ShellJumble.On ? D02.ShellHopf.Kind(D02.ShellShape.OpeningPath()) : D02.ShellHopf.Chain;
        if (layout != D02.ShellHopf.Chain && !D02.ShellHopf.Opens(layout))
        {
            GD.PushWarning("ADR-D02 B22: the ring sheet is held back (it hangs the GPU); Global opens on the chain");
            layout = D02.ShellHopf.Chain;
        }
        if (layout != D02.ShellHopf.Chain)
        {
            Require(hostLibrary, "m6h_set_view_layout", hostLibrary.SetViewLayout(_host, HostLibrary.View,
                D02.ShellHopf.Layout(D02.ShellHopf.RingsPath(), layout)));
            _hopf = layout == D02.ShellHopf.Rings;
            _camera.Dirty = true; // the first step sends the spin as Global's camera
            D02.ShellHopf.Took(layout);
        }
        // ADR-D02 B18c: with --d02 Local opens beside Global with its own shape, hidden until a cap is named.
        if (D02.ShellJumble.On)
        {
            OpenLocal(hostLibrary);
        }
        _log = EventLog.FromEnvironment();
        if (_log is { } log)
        {
            log.Describe(window, puzzle);
            log.SwapChain(surface.SwapChain());
            RenderingServer.FramePreDraw += OnFramePreDraw;
            RenderingServer.FramePostDraw += OnFramePostDraw;
            _uiFrames = true;
            _startup = StartupLog.FromEnvironment();
        }
        if (withSession)
        {
            var fixture = System.Environment.GetEnvironmentVariable("M600_SESSION_FIXTURE");
            var puzzleCode = (uint)Array.IndexOf(EventLog.Puzzles, puzzle);
            var opened = hostLibrary.OpenSession(_host, puzzleCode, session!, menu!,
                string.IsNullOrEmpty(fixture) ? null : fixture, out _openRevision);
            Require(hostLibrary, "m6h_open_session", opened);
            _startup?.SessionOpened(EventLog.Now());
            _session = true;
        }
        var runtime = System.Environment.GetEnvironmentVariable("M600_RUNTIME");
        if (!string.IsNullOrEmpty(runtime))
        {
            Require(hostLibrary, "m6h_load_runtime", hostLibrary.LoadRuntime(_host, runtime));
            _startup?.RuntimeLoaded(EventLog.Now());
        }
        _exitAtFirstFrame = System.Environment.GetEnvironmentVariable("M600_EXIT_AT_FIRST_FRAME") == "1";
        if (System.Environment.GetEnvironmentVariable("M600_LX4") == "1")
        {
            _modules.Add(new Lx4.Lx4Module(new ShellContext(window, this, _log, hostLibrary, _host)));
        }
        if (System.Environment.GetEnvironmentVariable("M600_LX2") == "1")
        {
            _modules.Add(new Lx2.Lx2Module(new ShellContext(window, this, _log, hostLibrary, _host)));
        }
        if (System.Environment.GetEnvironmentVariable("M600_EDITOR") == "1")
        {
            _editor = new Editor.EditorPanel(new ShellContext(window, this, _log, hostLibrary, _host));
            _modules.Add(_editor);
        }
        if (_modules.Count > 0)
        {
            WindowHook.Install(window, _modules);
        }
    }

    /// <summary>Calls every module; a module's exception is reported and never stops the shell's own work.</summary>
    private void EachModule(Action<IShellModule> call)
    {
        foreach (var module in _modules)
        {
            try
            {
                call(module);
            }
            catch (Exception error)
            {
                GD.PushError($"a shell module failed: {error.Message}");
            }
        }
    }

    /// <summary>Routes a key event to the session: a key-down's press goes to the log; a key-up releases it.</summary>
    private void Turn(InputEventKey key, long qpc)
    {
        if (!_session || KeyChord.Code(key.PhysicalKeycode) is not { } code)
        {
            return;
        }
        var input = new Magic600.Chrome.MotionKey(code, KeyChord.Chord(key, code), key.Pressed, key.Echo, qpc);
        if (!D02.ShellJumble.On || !D02.ShellMotion.TryQueue(input, SendTurn))
        {
            SendTurn(input, false);
        }
    }

    private void SendTurn(Magic600.Chrome.MotionKey key, bool staticMotion)
    {
        if (!_session || _hostLibrary is not { } library) return;
        if (!key.Pressed)
        {
            library.KeyUp(_host, key.Code);
            return;
        }
        var (chord, qpc) = (key.Chord, key.Qpc);
        if (D02.ShellExplore.State.On && Magic600.Chrome.MotionQueue.ChangesHead(chord))
        {
            if (!key.Echo)
            {
                _chrome?.Frame.Say(Magic600.Chrome.ExploreState.CommitRefusal, true);
            }
            return;
        }
        // Brief SF2: the key goes to the host's ordered key queue and the frame thread keeps drawing; its commit, the
        // journal write and the digests run on the host's key worker. A twist key arms its certified row (ADR-D02 B18
        // rest), which the worker reads just before the key, at the head the key will twist. Nothing of the key shows
        // until its answer arrives (PollKeys).
        var arming = EventLog.Now();
        (uint Cap, ulong Item)? arm = null;
        if (D02.ShellJumble.On && !key.Echo)
        {
            D02.ShellGrip.CancelGesture();
            arm = D02.ShellTurn.ArmFor(chord);
        }
        var code = library.KeyPost(_host, chord, key.Echo, arm is not null, arm?.Cap ?? 0, arm?.Item ?? 0, out var ticket);
        if (code != HostLibrary.Ok)
        {
            GD.PushWarning($"m6h_key_post {chord}: {code}: {library.LastError(_host)}");
            return;
        }
        // Under reduced motion no twist sweeps (its rows are 0 s), so the key is static and holds no queue.
        staticMotion |= !D02.ShellMotion.Sweeps;
        _posted[ticket] = new PostedKey(chord, qpc, staticMotion, arming, EventLog.Now());
        D02.ShellMotion.KeyPosted(staticMotion);
    }

    /// <summary>Brief SF2: applies every commit answer that is ready, in press order; then the host hands the keys still
    /// waiting to its worker.</summary>
    private void PollKeys(HostLibrary library)
    {
        while (_session)
        {
            if (library.KeyPoll(_host, out var answer) != HostLibrary.Ok)
            {
                GD.PushWarning($"m6h_key_poll: {library.LastError(_host)}");
                return;
            }
            if (answer.Ticket == 0) break;
            Answered(library, answer);
        }
        var waiting = false;
        foreach (var posted in _posted.Values) waiting |= !posted.StaticMotion;
        D02.ShellMotion.KeysAnswered(_posted.Count, waiting);
    }

    private void Answered(HostLibrary library, HostLibrary.KeyAnswer answer)
    {
        if (!_posted.Remove(answer.Ticket, out var posted))
        {
            // A queued key-up or focus loss is answered only when it failed.
            if (answer.Code != HostLibrary.Ok) GD.PushWarning($"m6h_key_up {answer.Ticket}: {answer.Code}: {library.LastError(_host)}");
            return;
        }
        var (chord, qpc) = (posted.Chord, posted.Qpc);
        var handled = EventLog.Now();
        if (answer.Code != HostLibrary.Ok)
        {
            GD.PushWarning($"m6h_key_down {chord}: {answer.Code}: {library.LastError(_host)}");
            return;
        }
        if (answer.Press == HostLibrary.PressCommitted)
        {
            D02.ShellTurn.Armed(answer.RowStatus == 1 ? library.KeyRow(_host, answer.Ticket, answer.RowRevision) : null);
            var rowed = EventLog.Now();
            _log?.Press(qpc, handled, answer.EngineRevision);
            var chromeParts = _log != null ? new List<(string Name, long From, long To)>() : null;
            D02.ShellJumble.BindParts = chromeParts;
            try
            {
                _chrome?.Committed(chord, answer.EngineRevision);
            }
            finally
            {
                D02.ShellJumble.BindParts = null;
            }
            var chromed = EventLog.Now();
            D02.ShellTurn.Committed(answer.EngineRevision, posted.StaticMotion);
            var turned = EventLog.Now();
            D02.ShellStyle.Committed(answer.EngineRevision);
            // SF (27 Nov 2026), SF2: the steps of a committed key, only while the event log is on. "post" is the frame
            // thread's at the press; "queued" runs on the host's key worker (the commit) while frames go on; the rest
            // are the frame thread's at the answer.
            if (_log is { } keyLog)
            {
                var keySteps = new List<(string Name, long From, long To)> { ("post", posted.Arming, posted.Posted), ("queued", posted.Posted, handled),
                        ("row", handled, rowed), ("chrome", rowed, chromed), ("turn", chromed, turned), ("style", turned, EventLog.Now()) };
                foreach (var group in (chromeParts ?? new()).GroupBy(p => p.Name))
                {
                    keySteps.Add(("chrome." + group.Key, 0, group.Sum(p => p.To - p.From)));
                }
                keyLog.Steps("key", qpc, keySteps.ToArray());
            }
        }
        else if (answer.Press == HostLibrary.PressRefused)
        {
            _log?.Press(qpc, handled, 0);
            _chrome?.Refused(chord);
            D02.ShellTurn.Refused();
        }
        else if (answer.Press == HostLibrary.PressOther)
        {
            _chrome?.Stepped(chord);
        }
    }

    private void CreateEcho()
    {
        var style = new StyleBoxFlat { BgColor = new Color(0, 0, 0, 0.75f) };
        style.SetContentMarginAll(12);
        _echo = new Label { Position = new Vector2(16, 16), MouseFilter = MouseFilterEnum.Ignore, Text = EchoText() };
        _echo.AddThemeStyleboxOverride("normal", style);
        _echo.AddThemeFontSizeOverride("font_size", 28);
        _echo.AddThemeColorOverride("font_color", Colors.White);
        AddChild(_echo);
    }

    private string EchoText() =>
        $"Input echo (route B check B1)\nkeys: {_presses}   last: {_lastPress}\nclicks: {_clicks}   last: {_lastClick}";

    /// <summary>Echoes and logs each key press (not a repeat) and mouse click (not the wheel) the window receives.</summary>
    public override void _Input(InputEvent @event)
    {
        if (@event is InputEventKey previewKey && _editor?.TextFocused != true && _chrome?.TakePreviewKey(previewKey) == true)
        {
            GetViewport().SetInputAsHandled();
            return;
        }
        var qpc = EventLog.Now();
        bool contextMenuOpen = _chrome?.Frame.OpenOverlay is Magic600.Chrome.ContextMenuOverlay;
        bool menuPointer = _menuPointer = contextMenuOpen || _contextMenuPress;
        // A click that closes the menu on press belongs to the menu through release as well.
        if (@event is InputEventMouseButton { ButtonIndex: MouseButton.Left } left)
            _contextMenuPress = contextMenuOpen && left.Pressed;
        FollowGripInput(@event, menuPointer);
        var turned = D02.ShellJumble.On && Turn(@event);
        if (turned)
        {
            GetViewport().SetInputAsHandled();
        }
        // ADR-D02 B17: a click (a release that did not turn the camera) picks in the view under it; the pointer hovers.
        if (D02.ShellJumble.On && _session && !menuPointer)
        {
            switch (@event)
            {
                case InputEventMouseButton { ButtonIndex: MouseButton.Left, Pressed: false } up when !turned:
                    if (SurfacePoint(up.Position) is { } at)
                    {
                        if (GripClick(up, at)) GetViewport().SetInputAsHandled();
                        else D02.ShellPick.Click(at.X, at.Y, ViewAt(up.Position));
                    }
                    break;
                case InputEventMouseButton { ButtonIndex: MouseButton.Right, Pressed: false } right when !turned:
                    if (_chrome is { } menuChrome && menuChrome.Frame.OpenOverlay == null && SurfacePoint(right.Position) != null)
                    {
                        menuChrome.OpenContextMenu(right.Position);
                        GetViewport().SetInputAsHandled();
                    }
                    break;
                case InputEventMouseMotion move:
                    D02.ShellPick.Point(SurfacePoint(move.Position));
                    break;
            }
        }
        if (@event is InputEventMouseButton { ButtonIndex: MouseButton.Left, Pressed: false }) _gripPress = null;
        // The wheel zooms the view under the pointer (owner, 26 November 2026); it takes no focus.
        if (D02.ShellJumble.On && D02.ShellJumble.Zoom.Follow(@event, ViewAt))
        {
            GetViewport().SetInputAsHandled();
        }
        if (@event is InputEventKey turn)
        {
            // One router decides (A5): a key the chrome takes never reaches the session, and a key that goes to the
            // session's router never reaches the chrome.
            if (_chrome is not { } chrome)
            {
                if (Sliced(turn)) GetViewport().SetInputAsHandled(); else Turn(turn, qpc);
            }
            else if (!chrome.Takes(turn))
            {
                if (Sliced(turn)) GetViewport().SetInputAsHandled(); else Turn(turn, qpc);
                // The editor's code field keeps the keys its text focus takes (the router routes by that focus).
                if (_session && KeyChord.Code(turn.PhysicalKeycode) is not null && _editor?.TextFocused != true)
                {
                    GetViewport().SetInputAsHandled();
                }
            }
        }
        if (_echo is not { } echo)
        {
            return;
        }
        switch (@event)
        {
            case InputEventKey { Pressed: true, Echo: false } key:
                var label = OS.GetKeycodeString(key.Keycode);
                if (string.IsNullOrEmpty(label))
                {
                    label = OS.GetKeycodeString(key.PhysicalKeycode);
                }
                _input?.Press(qpc, (long)key.Keycode, label);
                _presses++;
                _lastPress = label;
                break;
            case InputEventMouseButton { Pressed: true } click when click.ButtonIndex is not (MouseButton.WheelUp
                or MouseButton.WheelDown or MouseButton.WheelLeft or MouseButton.WheelRight):
                var x = (long)click.Position.X;
                var y = (long)click.Position.Y;
                _input?.Click(qpc, (long)click.ButtonIndex, x, y);
                _clicks++;
                _lastClick = $"{click.ButtonIndex} at ({x}, {y})";
                break;
            default:
                return;
        }
        echo.Text = EchoText();
    }

    /// <summary>The Global opening, or the whole window without the frame, in the window's pixels.</summary>
    private bool InOpening(Vector2 point) =>
        (_chrome is { } framed ? framed.Opening() : new Rect2I(Vector2I.Zero, DisplayServer.WindowGetSize()))
            .HasPoint(new Vector2I((int)point.X, (int)point.Y));

    /// <summary>
    /// ADR-D01.40/.41: a drag that starts in Global's opening or in Local's drawn opening turns the rotation that view
    /// draws from. Linked (the default) both views draw from the one camera, so a drag in either turns both together;
    /// unlinked, a drag in Local turns only Local's own rotation and a drag in Global only Global's. Each camera keeps
    /// the three-pixel rule, so a click in either view still picks.
    /// </summary>
    private bool Turn(InputEvent input)
    {
        if (_menuPointer)
        {
            return false;
        }
        if (D02.ShellLocal.Linked)
        {
            return _camera.Follow(input, point => InOpening(point) || InLocal(point));
        }
        var global = _camera.Follow(input, InOpening);
        var local = D02.ShellLocal.Own.Follow(input, InLocal);
        return global || local;
    }

    /// <summary>A window point in Local's drawn opening (through the pane layout's hit), outside Global's.</summary>
    // SA2_MIN_TEXTURE_PX of renderer/core/include/sa2_interop.h.
    private const int CoreMinTexture = 128;

    private bool InLocal(Vector2 point) => _localOpen && ViewAt(point) == HostLibrary.Local;

    /// <summary>ADR-D02 B17: a window point in a drawn view as a pixel of the core's surface, or none.</summary>
    private (uint X, uint Y)? SurfacePoint(Vector2 point)
    {
        if (ViewAt(point) is null)
        {
            return null;
        }
        var x = (int)point.X - _area.Position.X;
        var y = (int)point.Y - _area.Position.Y;
        return x >= 0 && y >= 0 && x < _area.Size.X && y < _area.Size.Y ? ((uint)x, (uint)y) : null;
    }

    /// <summary>
    /// The drawn view under a window point: Global in its opening, Local in its drawn opening (B18c, through the pane
    /// layout's hit), else none.
    /// </summary>
    private ulong? ViewAt(Vector2 point)
    {
        if (_menuPointer)
        {
            return null;
        }
        if (InOpening(point))
        {
            return HostLibrary.View;
        }
        if (_layout is { } layout && Magic600.Chrome.ViewPanes.Hit(layout, (int)point.X, (int)point.Y) is { View: "Local" })
        {
            return HostLibrary.Local;
        }
        return null;
    }

    /// <summary>
    /// ADR-D02 B18c: opens Local with local-view.json's shape (through <see cref="D02.ShellShape.Set"/>, so the wheel
    /// scales Local's own zoom), orthographic and spread, hidden until a cap is named.
    /// </summary>
    private void OpenLocal(HostLibrary library)
    {
        var shape = D02.ShellLocal.Read(D02.ShellLocal.ShapePath());
        Require(library, "m6h_open_local", library.OpenLocal(_host));
        _localOpen = true;
        Require(library, "m6h_set_view_visible", library.SetViewVisible(_host, HostLibrary.Local, false));
        Require(library, "m6h_set_view_shape", D02.ShellShape.Set(library, _host, HostLibrary.Local, D02.ShellLocal.ViewShape(shape, 1)));
        Require(library, "m6h_set_view_focus", library.SetViewFocus(_host, HostLibrary.Local, shape.Projection, HostLibrary.FocusNone,
            -1, shape.Spread));
        _localShape = shape;
    }

    private D02.ShellLocal.Shape? _localShape;

    /// <summary>
    /// ADR-D02 B18c: follows the named cap. Local is focused on it (its pole centred, only its turning region drawn) and
    /// shown, and the chrome opens Local's slot. Until a cap is named Local draws the default cap 0 (owner, 26 November
    /// 2026: Local is never empty); only with no state bound is Local hidden behind its empty sentence.
    /// B17 (ADR-D01.41): a later selection re-frames Local on its sticker's cell while auto-framing is on.
    /// </summary>
    private void FollowLocalCap(HostLibrary library)
    {
        if (!_localOpen || _localShape is not { } shape)
        {
            return;
        }
        int? cap = D02.ShellLocal.Frame(_chrome?.Jumble?.Naming.Named);
        // ADR-D01's view.local: a hidden Local keeps its cap and the shared camera; the host stops drawing it.
        var hidden = _chrome?.Frame.LocalHide.Hidden == true;
        _chrome?.LocalLine(hidden ? HiddenLocalWords : D02.ShellRaw.Line(cap) ?? D02.ShellLocal.Plan.Words);
        if (cap == _localCap && _localRaw == D02.ShellRaw.Focus && hidden == _localHidden)
        {
            return;
        }
        _localCap = cap;
        _localRaw = D02.ShellRaw.Focus;
        _localHidden = hidden;
        var cell = cap is int c and >= 0 and < 600 ? (uint)c : HostLibrary.FocusNone;
        // ADR-D02 B21: the raw view draws Local's stereographic chart from the pole, clipped at its h.
        var (projection, focusCos) = D02.ShellRaw.Focus is double h && cell != HostLibrary.FocusNone
            ? (HostLibrary.ProjectionStereographic, (float)h)
            : (shape.Projection, cell == HostLibrary.FocusNone ? -1 : D02.ShellLocal.FocusCos);
        Require(library, "m6h_set_view_focus", library.SetViewFocus(_host, HostLibrary.Local, projection, cell, focusCos, shape.Spread));
        Require(library, "m6h_set_view_visible", library.SetViewVisible(_host, HostLibrary.Local, cell != HostLibrary.FocusNone && !hidden));
        if (_chrome is { } chrome)
        {
            chrome.Frame.LocalOpeningShown = cell != HostLibrary.FocusNone;
        }
    }

    /// <summary>
    /// ADR-D02 B18c: the surface's window rectangle and the pane layout for the openings: Global's alone, or the
    /// bounding box of Global's and Local's while Local is drawn.
    /// </summary>
    private (Rect2I Area, Magic600.Chrome.PaneLayout? Layout) Openings(Rect2I global)
    {
        if (!_localOpen || _chrome is not { } chrome || global.Size.X <= 0 || global.Size.Y <= 0)
        {
            return (global, null);
        }
        var local = chrome.LocalOpening();
        if (local.Size.X <= 0 || local.Size.Y <= 0)
        {
            return (global, null);
        }
        try
        {
            var layout = Magic600.Chrome.ViewPanes.Layout(new[]
            {
                new Magic600.Chrome.PaneSpec("Global", new Magic600.Chrome.PaneRect(global.Position.X, global.Position.Y, global.Size.X, global.Size.Y)),
                new Magic600.Chrome.PaneSpec("Local", new Magic600.Chrome.PaneRect(local.Position.X, local.Position.Y, local.Size.X, local.Size.Y)),
            });
            var s = layout.Surface;
            return (new Rect2I(s.X, s.Y, s.Width, s.Height), layout);
        }
        catch (ArgumentException)
        {
            return (global, null);
        }
    }

    /// <summary>
    /// ADR-D02 B18c: sets the host's pane table for the layout the surface now has: Global in pane 0 and Local in pane
    /// 1, or no pane (one-view frames) while Local is not drawn. Called once the surface has its new rectangle and
    /// viewport, so the panes lie inside it.
    /// </summary>
    // ADR-D02 B20: the cut slice's held keys (the split, the turn preview), after the router gave the key to the
    // session and never while the editor's text field has focus.
    private bool Sliced(InputEventKey key) => D02.ShellJumble.On && _session && _editor?.TextFocused != true && (D02.ShellSlice.Follow(key) || D02.ShellRaw.Follow(key));

    private void SetPanes(HostLibrary library, Rect2I global, Magic600.Chrome.PaneLayout? layout)
    {
        var local = layout is null ? default : _chrome!.LocalOpening();
        var globalPane = layout is null && previewPanes.Length == 0 ? default : global;
        if (_paned == (globalPane, local) && _panedSplit == D02.ShellSlice.Split && _panedRaw == D02.ShellRaw.PaneKey
            && panedPreview.SequenceEqual(previewPanes))
        {
            return;
        }
        Require(library, "m6h_clear_pane", library.ClearPane(_host, 0));
        Require(library, "m6h_clear_pane", library.ClearPane(_host, 1));
        Require(library, "m6h_clear_pane", library.ClearPane(_host, 2));
        if (previewPanes.Length > 0 || panedPreview.Length > 0)
            Require(library, "m6h_clear_pane", library.ClearPane(_host, 3));
        uint nextPane = 1;
        if (layout is not null)
        {
            // B17: Global's pane draws with style slot 1. ADR-D02 B20: Local draws its cut slice with slot 2, or, while
            // the split key is held, as two linked copies of the one view, the outside layer (slot 2) and the inside
            // layer (slot 3) side by side (D02.ShellStyle sets the three slots).
            var g = layout.Panes[0].Rect;
            Require(library, "m6h_set_pane", library.SetPane(_host, 0, HostLibrary.View, g.X, g.Y, (uint)g.Width, (uint)g.Height, 1));
            // ADR-D02 B21: the raw view draws Local's chart with slot 2 and, with the antipode, the antipode chart beside it.
            var raw = D02.ShellRaw.On ? D02.ShellRaw.Panes(layout.Panes[1]) : null;
            var copies = D02.ShellSlice.Panes(layout.Panes[1]);
            if (raw is not null)
            {
                copies = Array.Empty<Magic600.Chrome.SlicePane>();
            }
            for (var i = 0; i < copies.Count; i++)
            {
                var r = copies[i].Pane.Rect;
                var slot = copies[i].Layer == Magic600.Chrome.SliceLayer.Inside ? 3u : 2u;
                Require(library, "m6h_set_pane", library.SetPane(_host, (uint)i + 1, HostLibrary.Local, r.X, r.Y, (uint)r.Width, (uint)r.Height, slot));
            }
            for (var i = 0; raw is not null && i < raw.Count; i++)
            {
                var r = raw[i].Pane.Rect;
                Require(library, "m6h_set_pane", library.SetPane(_host, (uint)i + 1, HostLibrary.Local, r.X, r.Y, (uint)r.Width, (uint)r.Height, 2));
                if (raw[i].Antipode)
                {
                    Require(library, "m6h_set_pane_flags", library.SetPaneFlags(_host, (uint)i + 1, HostLibrary.PaneAntipode));
                }
            }
            var first = raw is null ? copies[0].Pane.Rect : raw[0].Pane.Rect;
            nextPane += (uint)(raw?.Count ?? copies.Count);
            var aspect = first.Height > 0 ? (double)first.Width / first.Height : 1;
            if (_localShape is { } shape)
            {
                Require(library, "m6h_set_view_shape", D02.ShellShape.Set(library, _host, HostLibrary.Local, D02.ShellLocal.ViewShape(shape, aspect)));
                Require(library, "m6h_set_view_shape", D02.ShellShape.ApplyZoom(library, _host, HostLibrary.Local, D02.ShellJumble.Zoom.Factor(HostLibrary.Local)));
            }
        }
        else if (previewPanes.Length > 0)
        {
            var g = global.Position - _area.Position;
            Require(library, "m6h_set_pane", library.SetPane(_host, 0, HostLibrary.View, g.X, g.Y, (uint)global.Size.X, (uint)global.Size.Y, 1));
        }
        SetPreviewPanes(library, nextPane);
        _paned = (globalPane, local);
        _panedSplit = D02.ShellSlice.Split;
        _panedRaw = D02.ShellRaw.PaneKey;
        _layout = layout;
    }

    /// <summary>
    /// A rotation's layout spin as a view camera, as Look Lab's LocalShape.Frame takes it (column-major as it lies):
    /// Local's camera, and Global's on a Hopf layout.
    /// </summary>
    private static float[] SpinOf(D02.ShellCamera camera)
    {
        var m = new float[16];
        for (var i = 0; i < 16; i++)
        {
            m[i] = (float)camera.Spin[i];
        }
        return m;
    }

    /// <summary>
    /// Follows the foreground: the logs count it lost once the window had it and lost it, or if it never had it.
    /// </summary>
    private void FollowFocus()
    {
        var focused = DisplayServer.WindowIsFocused();
        if (focused)
        {
            _hadFocus = true;
        }
        else if (_hadFocus)
        {
            _log?.LostForeground();
            _input?.LostForeground();
        }
        if (_focused && !focused && _session && _hostLibrary is { } library)
        {
            // The router releases its held keys; their key-ups never arrive.
            library.FocusLost(_host);
            D02.ShellGrip.FocusLost();
        }
        _focused = focused;
    }

    private int? _gripPress;

    public override void _Notification(int what)
    {
        // ADR-D01.55: focus loss or closing settles the motion and releases held keys; then the grip's own handling.
        if ((what == (int)NotificationWMWindowFocusOut || what == (int)NotificationWMCloseRequest) && D02.ShellJumble.On)
        {
            FlushMotion();
            D02.ShellMotion.Keys.ReleaseHeldKeys();
        }
        if (!D02.ShellJumble.On || !_session) return;
        if (what == NotificationWMWindowFocusOut) D02.ShellGrip.FocusLost();
        if (what == NotificationWMMouseExit)
        {
            D02.ShellPick.Point(null);
            D02.ShellGrip.CancelGesture();
        }
    }

    private bool CanClickGrip() => _session && _chrome?.Frame.OpenOverlay == null && !_menuPointer &&
        _editor?.TextFocused != true && !D02.ShellExplore.State.On;

    private void FollowGripInput(InputEvent input, bool menuPointer)
    {
        if (!D02.ShellJumble.On || !D02.ShellGrip.Model.Ready) return;
        if (input is InputEventKey key) D02.ShellGrip.Follow(key);
        if (input is InputEventMouseButton { ButtonIndex: MouseButton.Left, Pressed: true } down)
        {
            D02.ShellGrip.CancelGesture();
            _gripPress = down.CtrlPressed && D02.ShellGrip.Held && !menuPointer && CanClickGrip() &&
                SurfacePoint(down.Position) != null ? D02.ShellGrip.Gesture : null;
        }
        if (input is InputEventMouseMotion && menuPointer) D02.ShellPick.Point(null);
    }

    private bool GripClick(InputEventMouseButton up, (uint X, uint Y) at)
    {
        if (_gripPress is not int press || press != D02.ShellGrip.Gesture || !up.CtrlPressed || !CanClickGrip()) return false;
        var revision = _adoptedRevision;
        var qpc = EventLog.Now();
        var view = ViewAt(up.Position);
        D02.ShellPick.GripClick(at.X, at.Y, pick =>
        {
            if (revision != _adoptedRevision || press != D02.ShellGrip.Gesture || !CanClickGrip()) return true;
            if (!D02.ShellGrip.ClickHits(pick, revision)) return false;
            Turn(new InputEventKey { Keycode = Key.J, PhysicalKeycode = Key.J, Pressed = true }, qpc);
            _hostLibrary?.KeyUp(_host, "KeyJ");
            return true;
        }, view);
        return true;
    }

    private Magic600.Chrome.CoreView GlobalProjection(float[] opening, double aspect)
    {
        var shape = _globalShape ?? opening;
        return new(_camera.Q, shape[0], shape[1], shape[2], D02.ShellShape.SentZoom(HostLibrary.View) ?? shape[3],
            aspect, _globalProjection == HostLibrary.ProjectionOrthographic);
    }

    private Magic600.Chrome.CoreView? LocalProjection(Magic600.Chrome.CorePlaces places, double aspect)
    {
        if (_localShape is not { } shape || _localCap is not int cap || cap is < 0 or >= 600) return null;
        // Local's column-major spin is transposed for the row-major anchor path.
        return new(Transposed(D02.ShellLocal.Camera(_camera).Spin), 1 - shape.Gap, shape.StickerShrink, shape.D4,
            D02.ShellShape.SentZoom(HostLibrary.Local) ?? 1, aspect, shape.Projection == HostLibrary.ProjectionOrthographic,
            places.CellCentre(cap), D02.ShellLocal.FocusCos, shape.Spread);
    }

    private void WireGrip()
    {
        if (!D02.ShellJumble.On || !D02.ShellLocal.Plan.Jumble || !_session || _hostLibrary is not { } library || _chrome == null) return;
        var places = _chrome.Sample?.Places ?? Magic600.Chrome.CorePlaces.Load(System.Environment.GetEnvironmentVariable("M600_ASSETS")!);
        _globalShape ??= D02.ShellShape.Read(D02.ShellShape.OpeningPath());
        D02.ShellGrip.Connect(library, _host, places, () => _adoptedRevision, CanClickGrip, () => GripPanes(places));
    }

    private IEnumerable<D02.ShellGrip.GripPane> GripPanes(Magic600.Chrome.CorePlaces places)
    {
        Rect2 Rect(Magic600.Chrome.PaneRect pane) => new(pane.X + _area.Position.X, pane.Y + _area.Position.Y, pane.Width, pane.Height);
        if (_chrome?.Opening() is { Size.X: > 0, Size.Y: > 0 } opening && _globalShape is { } shape)
        {
            var rect = _layout is { } panes ? Rect(panes.Panes[0].Rect) : new Rect2(opening.Position, opening.Size);
            var view = GlobalProjection(shape, (double)rect.Size.X / rect.Size.Y);
            if (D02.ShellHopf.Drawn != D02.ShellHopf.Chain)
                view = view with { Chart = D02.ShellGrip.Chart(D02.ShellHopf.Drawn, places, Transposed(_camera.Spin)) };
            yield return new("Global", rect, view);
        }
        if (_layout is not { } layout || !_localOpen || _chrome?.LocalOpening() is not { Size.X: > 0, Size.Y: > 0 }) yield break;
        var local = layout.Panes[1];
        if (D02.ShellRaw.On)
        {
            foreach (var raw in D02.ShellRaw.Panes(local))
            {
                var rect = Rect(raw.Pane.Rect);
                if (LocalProjection(places, (double)rect.Size.X / rect.Size.Y) is { } view)
                    yield return new(raw.Antipode ? "Local antipode" : "Local", rect,
                        view with { RawSign = raw.Antipode ? -1 : 1, FocusCos = raw.H });
            }
        }
        else
        {
            foreach (var slice in D02.ShellSlice.Panes(local))
            {
                var rect = Rect(slice.Pane.Rect);
                if (LocalProjection(places, (double)rect.Size.X / rect.Size.Y) is not { } view) continue;
                if (D02.ShellStyle.SliceSent)
                    view = view with { FocusCos = Math.Cos(slice.Shell.Max) - 1e-5, FocusCosMax = Math.Cos(slice.Shell.Min) + 1e-5 };
                yield return new(slice.Layer == Magic600.Chrome.SliceLayer.Inside ? "Local inside" : "Local", rect, view);
            }
        }
    }

    private void Require(HostLibrary library, string name, int code)
    {
        if (code != HostLibrary.Ok)
        {
            throw new InvalidOperationException($"{name} answered {code}: {library.LastError(_host)}");
        }
    }

    public override void _Process(double delta)
    {
        FollowFocus();
        if (_host == IntPtr.Zero || _hostLibrary is not { } library || _surface is not { } surface)
        {
            return;
        }
        // With the frame (A5) the core's surface lies under the Global view's opening and follows it once the opening
        // has kept one place for a few steps; without the frame it lies under the whole window. ADR-D02 B18c: while
        // Local is drawn, the surface covers the bounding box of both openings, and each is a pane of the frame.
        PollKeys(library);
        FollowLocalCap(library);
        var global = _chrome is { } framed ? framed.Opening() : new Rect2I(Vector2I.Zero, DisplayServer.WindowGetSize());
        repairPreview?.Tick();
        var (area, layout) = Openings(global);
        (area, layout) = PreviewOpenings(global, area, layout);
        _steady = area == _wanted ? _steady + 1 : 0;
        _wanted = area;
        // The core refuses a surface under SA2_MIN_TEXTURE_PX (sa2_interop.h) on a side: a smaller opening (a frame
        // still laying out) keeps the last surface instead of failing every frame.
        if (area != _area && area.Size.X >= CoreMinTexture && area.Size.Y >= CoreMinTexture && (_chrome is null || _steady >= 6))
        {
            if (area.Position != _area.Position)
            {
                surface.Move(area.Position.X, area.Position.Y);
            }
            var size = area.Size;
            if (size != _size)
            {
                surface.Resize((uint)size.X, (uint)size.Y);
                _log?.SwapChain(surface.SwapChain());
                Require(library, "m6h_set_viewport", library.SetViewport(_host, (uint)size.X, (uint)size.Y));
                _size = size;
            }
            _area = area;
        }
        if (_localOpen && area == _area)
        {
            SetPanes(library, global, layout);
        }
        // ADR-D02 B22 rest: a palette chart row switches Global's layout at the next step (ADR-D02.9).
        if (D02.ShellJumble.On && D02.ShellHopf.TakeRequest() is uint wanted && wanted != D02.ShellHopf.Drawn)
        {
            var took = library.SetViewLayout(_host, HostLibrary.View,
                wanted == D02.ShellHopf.Chain ? null : D02.ShellHopf.Layout(D02.ShellHopf.RingsPath(), wanted));
            if (took == HostLibrary.Ok)
            {
                _hopf = wanted == D02.ShellHopf.Rings;
                _camera.Dirty = true;
                D02.ShellHopf.Took(wanted);
            }
            else
            {
                GD.PushWarning($"ADR-D02 B22: m6h_set_view_layout answered {took}: {library.LastError(_host)}");
            }
        }
        var turning = D02.ShellLocal.Camera(_camera);
        if (D02.ShellJumble.On && (_camera.Dirty || turning.Dirty))
        {
            if (_camera.Dirty)
            {
                var camera = _hopf ? SpinOf(_camera) : _camera.ColumnMajor();
                Require(library, "m6h_set_camera", library.SetCamera(_host, camera));
                D02.ShellLocal.Sent(HostLibrary.View, camera);
            }
            // B18c: Local turns with its 3D spin, as Look Lab's Local does, the linked one unless Local is unlinked
            // (ADR-D01.41); the core turns the cap's pole to +w.
            if (_localOpen)
            {
                var spin = SpinOf(turning);
                Require(library, "m6h_set_view_camera", library.SetViewCamera(_host, HostLibrary.Local, spin));
                D02.ShellLocal.Sent(HostLibrary.Local, spin);
            }
            _camera.Dirty = false;
            turning.Dirty = false;
        }
        if (D02.ShellJumble.On)
        {
            foreach (var view in D02.ShellJumble.Zoom.TakeDirty())
            {
                Require(library, "m6h_set_view_shape", D02.ShellShape.ApplyZoom(library, _host, view, D02.ShellJumble.Zoom.Factor(view)));
            }
        }
        // ADR-D02 B17: the selection and hover outlines, sent again for each revision a frame draws; a refusal (before
        // the first posed adoption) is retried at the next step.
        if (D02.ShellJumble.On && _session)
        {
            D02.ShellStyle.Apply(library, _host, _adoptedRevision);
            D02.ShellStatus.Apply(library, _host, _adoptedRevision);
        }
        // ADR-D01.52 and .49/.5: the sticker colours and the scene look, sent again until the host takes them.
        if (D02.ShellJumble.On)
        {
            D02.ShellScene.Apply(library, _host);
        }
        _submittedThisFrame = null;
        var submitted = EventLog.Now();
        var code = library.Frame(_host, out var frame);
        var returned = EventLog.Now();
        if (code == HostLibrary.Ok)
        {
            _log?.Frame(frame, submitted, returned);
            _submittedThisFrame = frame.Sequence;
            _adoptedRevision = frame.EngineRevision;
            repairPreview?.Follow();
            D02.ShellTurn.Framed(frame.EngineRevision);
            D02.ShellStatus.Framed(frame);
            // ADR-D02 B17: the clicks and hovers of this step ask the ID pass on the frame just submitted.
            if (D02.ShellJumble.On && _session && D02.ShellPick.Step(library, _host))
            {
                D02.ShellStyle.Invalidate();
            }
            if (_session && _firstSubmitted is null && frame.EngineRevision >= _openRevision
                && frame.PublicationRevision >= _openRevision)
            {
                _firstSubmitted = frame.Sequence;
            }
            if (_modules.Count > 0)
            {
                EachModule(module => module.OnFrame(frame, submitted));
            }
        }
        else if (code == HostLibrary.Unavailable)
        {
            code = library.Recover(_host);
            if (code == HostLibrary.Ok)
            {
                _log?.SwapChain(surface.SwapChain());
            }
        }
        if (code != HostLibrary.Ok)
        {
            GD.PushWarning($"frame: {code}: {library.LastError(_host)}");
        }
        int count;
        do
        {
            if (library.ReadPresented(_host, _presented, out count) != HostLibrary.Ok)
            {
                break;
            }
            for (var i = 0; i < count; i++)
            {
                _log?.Presented(_presented[i]);
                if (_modules.Count > 0)
                {
                    var row = _presented[i];
                    EachModule(module => module.OnPresented(row));
                }
                _shownRevision = Math.Max(_shownRevision, _presented[i].EngineRevision);
                if (_firstShown is null && _firstSubmitted is { } first && _presented[i].Sequence >= first)
                {
                    _firstShown = (_presented[i].Sequence, _presented[i].DisplayQpc);
                    if (_exitAtFirstFrame)
                    {
                        GetTree().Quit();
                    }
                }
            }
        } while (count == _presented.Length);
        // SF3 (27 Nov 2026): the chrome's Describe at a newly shown revision, as a "steps" row when it took over 1 ms.
        var describing = EventLog.Now();
        _chrome?.Shown(_shownRevision);
        if (_log is { } shownLog && EventLog.Now() - describing > System.Diagnostics.Stopwatch.Frequency / 1000)
        {
            shownLog.Steps("shown", describing, ("describe", describing, EventLog.Now()));
        }
        // ADR-D02 B5: at each newly adopted revision, the sample state whose digests equal the head's binds. It follows
        // the revision a submitted frame drew, not the one confirmed on screen: a window behind the owner's presents
        // nothing, and the facts must still follow the session (checkpoint C1).
        if (D02.ShellJumble.On && _session && _adoptedRevision != _boundRevision)
        {
            _boundRevision = _adoptedRevision;
            var asking = EventLog.Now();
            var read = library.StateDigest(_host) == HostLibrary.Ok;
            var digested = EventLog.Now();
            var parts = _log != null ? new List<(string Name, long From, long To)>() : null;
            D02.ShellJumble.BindParts = parts;
            try
            {
                if (read)
                {
                    D02.ShellJumble.Bind(library.Answer(_host), (long)_adoptedRevision);
                }
                else
                {
                    D02.ShellJumble.Unknown((long)_adoptedRevision);
                }
            }
            finally
            {
                D02.ShellJumble.BindParts = null;
            }
            // SF (27 Nov 2026): the bind's steps at a newly adopted revision, only while the event log is on; SF3: with
            // its parts (feed, chip, totals, style, capmap, nu, tint, cards, inspect) between the two totals.
            if (_log is { } log && parts != null)
            {
                var bound = EventLog.Now();
                // A part that ran more than once in the bind is written once, with its summed time.
                var summed = new List<(string Name, long From, long To)> { ("state_digest", asking, digested) };
                foreach (var group in parts.GroupBy(p => p.Name))
                {
                    summed.Add((group.Key, 0, group.Sum(p => p.To - p.From)));
                }
                summed.Add(("bind", digested, bound));
                log.Steps("bind", asking, summed.ToArray());
            }
        }
    }

    /// <summary>Before Godot draws and presents its frame (RenderingServer's frame_pre_draw).</summary>
    private void OnFramePreDraw() => _preDrawQpc = EventLog.Now();

    /// <summary>
    /// After Godot drew its frame (frame_post_draw): one ui_frame row naming the 3D frame submitted in the same process
    /// frame. A Godot frame without one gets no row.
    /// </summary>
    private void OnFramePostDraw()
    {
        var drawn = EventLog.Now();
        if (_submittedThisFrame is { } frameId && _preDrawQpc != 0)
        {
            _log?.UiFrame(_preDrawQpc, frameId, drawn);
        }
        _submittedThisFrame = null;
        _preDrawQpc = 0;
    }

    public override void _ExitTree()
    {
        Close();
    }

    private void Close()
    {
        if (!_hadFocus)
        {
            _log?.LostForeground();
            _input?.LostForeground();
        }
        _input?.Dispose();
        _input = null;
        CloseSession();
        CloseCore();
    }

    /// <summary>
    /// Closes the session: its correctness control runs once, and the run row carries its outcome; the startup log's
    /// first correct frame carries it too (a control that did not run counts as a failure).
    /// </summary>
    private void CloseSession()
    {
        if (!_session || _hostLibrary is not { } library)
        {
            return;
        }
        // Brief SF2: every posted key is committed and answered before the session closes.
        if (library.KeyDrain(_host) != HostLibrary.Ok) GD.PushWarning($"m6h_key_drain: {library.LastError(_host)}");
        PollKeys(library);
        _session = false;
        var code = library.CloseSession(_host, _shownRevision, out var pass);
        if (code != HostLibrary.Ok)
        {
            GD.PushError($"m6h_close_session: {code}: {library.LastError(_host)}");
            pass = false;
        }
        else
        {
            _log?.Correctness(pass, HostLibrary.CorrectnessControl, library.Check(_host));
        }
        if (_firstShown is { } shown)
        {
            _startup?.FirstCorrectFrame(shown.FrameId, shown.DisplayQpc, pass);
        }
    }

    /// <summary>Closes the event log, the host, the surface and the libraries; the input echo stays.</summary>
    private void CloseCore()
    {
        if (D02.ShellJumble.On) D02.ShellGrip.Disconnect();
        if (_uiFrames)
        {
            RenderingServer.FramePreDraw -= OnFramePreDraw;
            RenderingServer.FramePostDraw -= OnFramePostDraw;
            _uiFrames = false;
        }
        // The modules go before the log closes, so that their last rows reach it.
        WindowHook.Remove();
        EachModule(module => module.Dispose());
        _modules.Clear();
        _editor = null;
        _startup?.Dispose();
        _startup = null;
        _log?.Dispose();
        _log = null;
        if (_host != IntPtr.Zero && _hostLibrary is { } library)
        {
            library.Destroy(_host);
            _host = IntPtr.Zero;
        }
        // A preload that no host took: freeing it waits for its loads, so none runs after the library is freed.
        if (_preload != IntPtr.Zero && _hostLibrary is { } preloads)
        {
            preloads.FreePreload(_preload);
            _preload = IntPtr.Zero;
        }
        _surface?.Dispose();
        _surface = null;
        _hostLibrary?.Dispose();
        _hostLibrary = null;
        _core?.Dispose();
        _core = null;
    }

    internal void FlushMotion(bool reduced = false)
    {
        if (!_session || _hostLibrary is not { } library || (!reduced && D02.ShellMotion.Keys.Count == 0)) return;
        D02.ShellTurn.CutMotion(library, _host, reduced);
        D02.ShellMotion.Flush();
    }

    internal void FollowRawClip()
    {
        if (!_localOpen || _localShape is not { } shape || _hostLibrary is not { } library) return;
        if (D02.ShellMotion.StepRawClip(_localCap, _localRaw) is float h && _localCap is int cell)
            Require(library, "m6h_set_view_focus", library.SetViewFocus(_host, HostLibrary.Local,
                HostLibrary.ProjectionStereographic, (uint)cell, h, shape.Spread));
    }
}
