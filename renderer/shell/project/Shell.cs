using System;
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
/// <see cref="InputLog"/> records them; the echo works with or without the core.
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
    private Vector2I _size;
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
        var hwnd = DisplayServer.WindowGetNativeHandle(DisplayServer.HandleType.WindowHandle);
        _size = DisplayServer.WindowGetSize();
        var surface = _surface = CoreSurface.Open(coreLibrary, (ulong)hwnd, 0, 0, (uint)_size.X, (uint)_size.Y);
        var code = hostLibrary.Create(coreLibrary, surface.Context, assets, out _host);
        if (code != HostLibrary.Ok)
        {
            throw new InvalidOperationException($"m6h_create answered {code}: {hostLibrary.LastError(IntPtr.Zero)}");
        }
        Require(hostLibrary, "m6h_set_viewport", hostLibrary.SetViewport(_host, (uint)_size.X, (uint)_size.Y));
        _log = EventLog.FromEnvironment();
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
        if (_echo is not { } echo)
        {
            return;
        }
        var qpc = EventLog.Now();
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
        if (DisplayServer.WindowIsFocused())
        {
            _hadFocus = true;
        }
        else if (_hadFocus)
        {
            _log?.LostForeground();
            _input?.LostForeground();
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
        var size = DisplayServer.WindowGetSize();
        if (size != _size && size.X > 0 && size.Y > 0)
        {
            surface.Resize((uint)size.X, (uint)size.Y);
            Require(library, "m6h_set_viewport", library.SetViewport(_host, (uint)size.X, (uint)size.Y));
            _size = size;
        }
        var submitted = EventLog.Now();
        var code = library.Frame(_host, out var frame);
        if (code == HostLibrary.Ok)
        {
            _log?.Frame(frame, submitted);
        }
        else if (code == HostLibrary.Unavailable)
        {
            code = library.Recover(_host);
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
            }
        } while (count == _presented.Length);
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
        CloseCore();
    }

    /// <summary>Closes the event log, the host, the surface and the libraries; the input echo stays.</summary>
    private void CloseCore()
    {
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
