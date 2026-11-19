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
/// are read). C9 U8 (a draft): the run row names the window and the puzzle (M600_PUZZLE; the host opens only Jumble
/// off for now), and when M600_LX4 is 1 the shell creates LX-4's module (<see cref="IShellModule"/>), which receives
/// the window's messages through <see cref="WindowHook"/> before Godot does, and every submitted and presented frame.
/// </remarks>
public partial class Shell : Control
{
    private CoreLibrary? _core;
    private HostLibrary? _hostLibrary;
    private CoreSurface? _surface;
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
        if (puzzle != EventLog.Puzzles[0])
        {
            // The host opens a session of 600-cell-Full only; Jumble on needs the jumbled puzzle's session and view.
            throw new NotSupportedException($"the shell does not run {puzzle} yet");
        }
        var hwnd = DisplayServer.WindowGetNativeHandle(DisplayServer.HandleType.WindowHandle);
        var window = (IntPtr)hwnd;
        _size = DisplayServer.WindowGetSize();
        var surface = _surface = CoreSurface.Open(coreLibrary, (ulong)hwnd, 0, 0, (uint)_size.X, (uint)_size.Y);
        var code = hostLibrary.Create(coreLibrary, surface.Context, assets, out _host);
        if (code != HostLibrary.Ok)
        {
            throw new InvalidOperationException($"m6h_create answered {code}: {hostLibrary.LastError(IntPtr.Zero)}");
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
        }
        var session = System.Environment.GetEnvironmentVariable("M600_SESSION_DIR");
        var menu = System.Environment.GetEnvironmentVariable("M600_TWIST_MENU");
        if (!string.IsNullOrEmpty(session) && !string.IsNullOrEmpty(menu))
        {
            Require(hostLibrary, "m6h_open_session", hostLibrary.OpenSession(_host, session, menu));
            _session = true;
        }
        if (System.Environment.GetEnvironmentVariable("M600_LX4") == "1")
        {
            _modules.Add(new Lx4.Lx4Module(new ShellContext(window, this, _log)));
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
        if (answer != HostLibrary.Ok)
        {
            GD.PushWarning($"m6h_key_down {chord}: {answer}: {library.LastError(_host)}");
            return;
        }
        if (result.Press == HostLibrary.PressCommitted)
        {
            _log?.Press(qpc, result.EngineRevision);
        }
        else if (result.Press == HostLibrary.PressRefused)
        {
            _log?.Press(qpc, 0);
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
        if (code == HostLibrary.Ok)
        {
            _log?.Frame(frame, submitted);
            _submittedThisFrame = frame.Sequence;
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

    /// <summary>Closes the session: its correctness control runs once, and the run row carries its outcome.</summary>
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
            return;
        }
        _log?.Correctness(pass, HostLibrary.CorrectnessControl, library.Check(_host));
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
        _log?.Dispose();
        _log = null;
        if (_host != IntPtr.Zero && _hostLibrary is { } library)
        {
            library.Destroy(_host);
            _host = IntPtr.Zero;
        }
        _surface?.Dispose();
        _surface = null;
        _hostLibrary?.Dispose();
        _hostLibrary = null;
        _core?.Dispose();
        _core = null;
    }
}
