using System;
using System.Collections.Generic;
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
    // ADR-D02 B23: Global is drawn as the Hopf rings, so its camera is the linked 3D spin.
    private bool _hopf;
    private int? _localCap = -1;
    private (Rect2I Global, Rect2I Local) _paned;
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
            _chrome = new ShellChrome(this, folder, _host != IntPtr.Zero);
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
        Require(hostLibrary, "m6h_set_viewport", hostLibrary.SetViewport(_host, (uint)_size.X, (uint)_size.Y));
        // ADR-D02 (owner, 26 November 2026: smaller stickers): with --d02 the view takes the owner's opening shape.
        Require(hostLibrary, "m6h_set_view_shape", D02.ShellShape.Apply(hostLibrary, _host));
        // ADR-D02 B23 (bare): with --d02 and an opening file that names the Hopf rings, Global opens on them, turned
        // by the linked 3D spin.
        if (D02.ShellJumble.On && D02.ShellHopf.Named(D02.ShellShape.OpeningPath()))
        {
            Require(hostLibrary, "m6h_set_view_layout", hostLibrary.SetViewLayout(_host, HostLibrary.View,
                D02.ShellHopf.Layout(D02.ShellHopf.RingsPath())));
            _hopf = true;
            _camera.Dirty = true; // the first step sends the spin as Global's camera
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
        if (!_session || _hostLibrary is not { } library || KeyChord.Code(key.PhysicalKeycode) is not { } code)
        {
            return;
        }
        if (!key.Pressed)
        {
            library.KeyUp(_host, code);
            return;
        }
        var chord = KeyChord.Chord(key, code);
        var answer = library.KeyDown(_host, chord, key.Echo, out var result);
        var handled = EventLog.Now();
        if (answer != HostLibrary.Ok)
        {
            GD.PushWarning($"m6h_key_down {chord}: {answer}: {library.LastError(_host)}");
            return;
        }
        if (result.Press == HostLibrary.PressCommitted)
        {
            _log?.Press(qpc, handled, result.EngineRevision);
            _chrome?.Committed(chord, result.EngineRevision);
        }
        else if (result.Press == HostLibrary.PressRefused)
        {
            _log?.Press(qpc, handled, 0);
            _chrome?.Refused(chord);
        }
        else if (result.Press == HostLibrary.PressOther)
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
        var qpc = EventLog.Now();
        if (D02.ShellJumble.On && _camera.Follow(@event, InOpening))
        {
            GetViewport().SetInputAsHandled();
        }
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
                Turn(turn, qpc);
            }
            else if (!chrome.Takes(turn))
            {
                Turn(turn, qpc);
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
    /// The drawn view under a window point: Global in its opening, Local in its drawn opening (B18c, through the pane
    /// layout's hit), else none.
    /// </summary>
    private ulong? ViewAt(Vector2 point)
    {
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
    /// shown, and the chrome opens Local's slot; with no cap Local is hidden and its empty sentence takes the place.
    /// </summary>
    private void FollowLocalCap(HostLibrary library)
    {
        if (!_localOpen || _localShape is not { } shape)
        {
            return;
        }
        int? cap = _chrome?.Jumble?.Naming.Named;
        if (cap == _localCap)
        {
            return;
        }
        _localCap = cap;
        var cell = cap is int c and >= 0 and < 600 ? (uint)c : HostLibrary.FocusNone;
        Require(library, "m6h_set_view_focus", library.SetViewFocus(_host, HostLibrary.Local, shape.Projection, cell,
            cell == HostLibrary.FocusNone ? -1 : D02.ShellLocal.FocusCos, shape.Spread));
        Require(library, "m6h_set_view_visible", library.SetViewVisible(_host, HostLibrary.Local, cell != HostLibrary.FocusNone));
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
    private void SetPanes(HostLibrary library, Rect2I global, Magic600.Chrome.PaneLayout? layout)
    {
        var local = layout is null ? default : _chrome!.LocalOpening();
        if (_paned == (layout is null ? default : global, local))
        {
            return;
        }
        Require(library, "m6h_clear_pane", library.ClearPane(_host, 0));
        Require(library, "m6h_clear_pane", library.ClearPane(_host, 1));
        if (layout is not null)
        {
            var views = new[] { HostLibrary.View, HostLibrary.Local };
            for (var i = 0; i < 2; i++)
            {
                var r = layout.Panes[i].Rect;
                Require(library, "m6h_set_pane", library.SetPane(_host, (uint)i, views[i], r.X, r.Y, (uint)r.Width, (uint)r.Height, 0));
            }
            var aspect = layout.Panes[1].Rect.Height > 0 ? (double)layout.Panes[1].Rect.Width / layout.Panes[1].Rect.Height : 1;
            if (_localShape is { } shape)
            {
                Require(library, "m6h_set_view_shape", D02.ShellShape.Set(library, _host, HostLibrary.Local, D02.ShellLocal.ViewShape(shape, aspect)));
                Require(library, "m6h_set_view_shape", D02.ShellShape.ApplyZoom(library, _host, HostLibrary.Local, D02.ShellJumble.Zoom.Factor(HostLibrary.Local)));
            }
        }
        _paned = (layout is null ? default : global, local);
        _layout = layout;
    }

    /// <summary>Local's camera: the layout spin as Look Lab's LocalShape.Frame takes it (column-major as it lies).</summary>
    private static float[] LocalCamera()
    {
        var m = new float[16];
        for (var i = 0; i < 16; i++)
        {
            m[i] = (float)_camera.Spin[i];
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
        }
        _focused = focused;
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
        FollowLocalCap(library);
        var global = _chrome is { } framed ? framed.Opening() : new Rect2I(Vector2I.Zero, DisplayServer.WindowGetSize());
        var (area, layout) = Openings(global);
        _steady = area == _wanted ? _steady + 1 : 0;
        _wanted = area;
        if (area != _area && area.Size.X > 0 && area.Size.Y > 0 && (_chrome is null || _steady >= 6))
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
        if (D02.ShellJumble.On && _camera.Dirty)
        {
            Require(library, "m6h_set_camera", library.SetCamera(_host, _hopf ? LocalCamera() : _camera.ColumnMajor()));
            // B18c: Local turns with the linked 3D spin, as Look Lab's Local does; the core turns the cap's pole to +w.
            if (_localOpen)
            {
                Require(library, "m6h_set_view_camera", library.SetViewCamera(_host, HostLibrary.Local, LocalCamera()));
            }
            _camera.Dirty = false;
        }
        if (D02.ShellJumble.On)
        {
            foreach (var view in D02.ShellJumble.Zoom.TakeDirty())
            {
                Require(library, "m6h_set_view_shape", D02.ShellShape.ApplyZoom(library, _host, view, D02.ShellJumble.Zoom.Factor(view)));
            }
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
        _chrome?.Shown(_shownRevision);
        // ADR-D02 B5: at each newly adopted revision, the sample state whose digests equal the head's binds. It follows
        // the revision a submitted frame drew, not the one confirmed on screen: a window behind the owner's presents
        // nothing, and the facts must still follow the session (checkpoint C1).
        if (D02.ShellJumble.On && _session && _adoptedRevision != _boundRevision)
        {
            _boundRevision = _adoptedRevision;
            if (library.StateDigest(_host) == HostLibrary.Ok)
            {
                D02.ShellJumble.Bind(library.Answer(_host), (long)_adoptedRevision);
            }
            else
            {
                D02.ShellJumble.Unknown((long)_adoptedRevision);
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
}
