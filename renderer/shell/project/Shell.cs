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
/// quits once that frame is shown.
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
    private Vector2I _size;
    /// <summary>The 3D frame submitted in this process frame, for the next ui_frame row.</summary>
    private ulong? _submittedThisFrame;
    private long _preDrawQpc;
    private bool _uiFrames;
    private readonly List<IShellModule> _modules = new();
    private StartupLog? _startup;
    /// <summary>The engine revision the session opened at (charter E3's R0).</summary>
    private ulong _openRevision;
    /// <summary>The first submitted frame that draws R0 over a publication of R0 or later.</summary>
    private ulong? _firstSubmitted;
    /// <summary>The first confirmed presentation of that frame or a later one: its sequence and display time.</summary>
    private (ulong FrameId, ulong DisplayQpc)? _firstShown;
    private bool _exitAtFirstFrame;
    private readonly HostLibrary.Presented[] _presented = new HostLibrary.Presented[16];

    public override void _Ready()
    {
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
        }
        else if (result.Press == HostLibrary.PressRefused)
        {
            _log?.Press(qpc, handled, 0);
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
        if (@event is InputEventKey turn)
        {
            Turn(turn, qpc);
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
        var size = DisplayServer.WindowGetSize();
        if (size != _size && size.X > 0 && size.Y > 0)
        {
            surface.Resize((uint)size.X, (uint)size.Y);
            _log?.SwapChain(surface.SwapChain());
            Require(library, "m6h_set_viewport", library.SetViewport(_host, (uint)size.X, (uint)size.Y));
            _size = size;
        }
        _submittedThisFrame = null;
        var submitted = EventLog.Now();
        var code = library.Frame(_host, out var frame);
        var returned = EventLog.Now();
        if (code == HostLibrary.Ok)
        {
            _log?.Frame(frame, submitted, returned);
            _submittedThisFrame = frame.Sequence;
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
