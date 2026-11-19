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
/// every submitted frame and every confirmed presentation with its display time.
/// </remarks>
public partial class Shell : Control
{
    private CoreLibrary? _core;
    private HostLibrary? _hostLibrary;
    private CoreSurface? _surface;
    private IntPtr _host;
    private EventLog? _log;
    private Vector2I _size;
    private readonly HostLibrary.Presented[] _presented = new HostLibrary.Presented[16];

    public override void _Ready()
    {
        GetViewport().TransparentBg = true;
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
            Close();
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

    private void Require(HostLibrary library, string name, int code)
    {
        if (code != HostLibrary.Ok)
        {
            throw new InvalidOperationException($"{name} answered {code}: {library.LastError(_host)}");
        }
    }

    public override void _Process(double delta)
    {
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
        if (_log is { } log && !DisplayServer.WindowIsFocused())
        {
            log.LostForeground();
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
