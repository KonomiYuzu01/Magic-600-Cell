using Godot;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Text.Json;

// Route-B probe host: Godot's transparent window draws an opaque frame and UI panel; c9pp's core
// layer sits beneath it in the window's topmost=FALSE DirectComposition target. Runs the same
// timeline as native/src/control.cpp. Arguments after "--": --out DIR --dll PATH [--topmost] [--scale S].
// Exit codes: 0 completed, 2 harness error, 3 suspected TDR (Godot's own device reports removal).
public partial class Probe : Control
{
    private const uint Base = 0x18408C, Ui = 0xE67814, FrameGrey = 0x3C3C3C, Cell = 24;
    private const int ResizeWidth = 1024, ResizeHeight = 640, Width = 1280, Height = 720;

    private readonly record struct Box(int X, int Y, int W, int H);
    private readonly record struct Layout(Box Core, Box Panel, Box UiSample, Box BgSample);

    private static Layout MakeLayout(int width, int height)
    {
        const int m = 40;
        Box core = new(m, m, width - 2 * m, height - 2 * m);
        Box panel = new(core.X + (core.W - 360) / 2, core.Y + (core.H - 200) / 2, 360, 200);
        return new Layout(core, panel, new Box(panel.X + 16, panel.Y + 16, 160, 80),
            new Box(core.X + 24, core.Y + core.H / 2 - 24, 48, 48));
    }

    private sealed class HarnessException : Exception
    {
        public int Code { get; }
        public HarnessException(int code, string message) : base(message) { Code = code; }
    }

    private string _out, _dll;
    private bool _topmost;
    private double _scale = 1.0;
    private Native _native;
    private nint _ctx;
    private ulong _hostDevice;
    private uint _nextFrame = 1;
    private long _coreFrames, _hostFrames, _samples;
    private readonly Stopwatch _clock = new();
    private double _lastSample = -1;
    private readonly List<string> _phases = new();
    private IEnumerator<int> _timeline;
    private bool _finished;
    private readonly ColorRect[] _frameRects = new ColorRect[4];
    private ColorRect _panel;
    private Vector2I _laidOut;
    private readonly Dictionary<string, object> _meta = new();

    private static Color Rgb(uint v) => Color.Color8((byte)(v >> 16), (byte)(v >> 8), (byte)v, 255);

    public override void _Ready()
    {
        try { ParseArguments(OS.GetCmdlineUserArgs()); }
        catch (ArgumentException e)
        {
            Console.Error.WriteLine("c9probe: " + e.Message);
            _finished = true;
            GetTree().Quit(2);
            return;
        }
        GetViewport().TransparentBg = true;
        RenderingServer.SetDefaultClearColor(new Color(0, 0, 0, 0));
        for (int i = 0; i < 4; i++)
        {
            _frameRects[i] = new ColorRect { Color = Rgb(FrameGrey), MouseFilter = MouseFilterEnum.Ignore };
            AddChild(_frameRects[i]);
        }
        _panel = new ColorRect { Color = Rgb(Ui), MouseFilter = MouseFilterEnum.Ignore };
        AddChild(_panel);
        if (_topmost) DisplayServer.WindowSetFlag(DisplayServer.WindowFlags.AlwaysOnTop, true);
        _clock.Start();
        _timeline = Timeline().GetEnumerator();
    }

    private void ParseArguments(string[] args)
    {
        HashSet<string> seen = new();
        for (int i = 0; i < args.Length; i++)
        {
            string a = args[i];
            if (!seen.Add(a)) throw new ArgumentException("duplicate option " + a);
            switch (a)
            {
                case "--out":
                case "--dll":
                case "--scale":
                    if (i + 1 >= args.Length || args[i + 1].Length == 0 || args[i + 1].StartsWith("--", StringComparison.Ordinal))
                        throw new ArgumentException(a + " needs a value");
                    string v = args[++i];
                    if (a == "--out") _out = Path.GetFullPath(v);
                    else if (a == "--dll") _dll = Path.GetFullPath(v);
                    else if (!double.TryParse(v, NumberStyles.Float, CultureInfo.InvariantCulture, out _scale)
                             || !double.IsFinite(_scale) || _scale < 0.25 || _scale > 10.0)
                        throw new ArgumentException("--scale must be a number in [0.25, 10]");
                    break;
                case "--topmost":
                    _topmost = true;
                    break;
                default:
                    throw new ArgumentException("unknown option " + a);
            }
        }
        if (_out == null || _dll == null) throw new ArgumentException("--out DIR and --dll PATH are required");
        if (!File.Exists(_dll)) throw new ArgumentException("--dll does not exist");
        Directory.CreateDirectory(_out);
    }

    public override void _Process(double delta)
    {
        if (_finished) return;
        try
        {
            if (!_timeline.MoveNext()) Finish(0, "completed");
        }
        catch (HarnessException e) { Finish(e.Code, e.Message); }
        catch (Exception e) { Finish(2, e.GetType().Name + ": " + e.Message); }
    }

    private double Now => _clock.Elapsed.TotalSeconds;

    private void Core(int status, string what)
    {
        if (status == Native.OK) return;
        string text = what + ": " + _native.LastError(_ctx);
        throw new HarnessException(status == Native.E_DEVICE_REMOVED ? 3 : 2, text);
    }

    private void Phase(string name)
    {
        _phases.Add(name);
        Core(_native.SetPhase(_ctx, name), "c9pp_set_phase");
    }

    private unsafe void HostStatus()
    {
        int hr = 0;
        Core(_native.godot_device_status(_ctx, _hostDevice, &hr), "c9pp_godot_device_status");
        if (hr != 0) throw new HarnessException(3, $"suspected real TDR: Godot device removed reason 0x{hr:X8}");
    }

    private unsafe void Frame(uint sync)
    {
        C9FrameRecord r = new() { struct_size = 40 };
        Core(_native.frame(_ctx, _nextFrame++, sync, &r), "c9pp_frame");
        _coreFrames++;
    }

    private unsafe void Poll()
    {
        C9StatsRow r = new() { struct_size = 48 };
        Core(_native.poll(_ctx, &r), "c9pp_poll");
    }

    private unsafe void Sample(Layout l, bool force)
    {
        double t = Now;
        if (!force && _lastSample >= 0 && t - _lastSample < 0.1) return;
        _lastSample = t;
        C9SampleDesc d = new()
        {
            struct_size = 36, ui_x = l.UiSample.X, ui_y = l.UiSample.Y, ui_w = (uint)l.UiSample.W, ui_h = (uint)l.UiSample.H,
            bg_x = l.BgSample.X, bg_y = l.BgSample.Y, bg_w = (uint)l.BgSample.W, bg_h = (uint)l.BgSample.H
        };
        C9ScreenSample s = new() { struct_size = 88 };
        Core(_native.sample_screen(_ctx, &d, &s), "c9pp_sample_screen");
        _samples++;
    }

    private Layout Current()
    {
        Vector2I size = DisplayServer.WindowGetSize();
        return MakeLayout(size.X, size.Y);
    }

    // Godot's UI layer: an opaque frame around the core rectangle and an opaque panel inside it.
    private void LayOut()
    {
        Vector2I size = DisplayServer.WindowGetSize();
        if (size == _laidOut) return;
        _laidOut = size;
        Layout l = MakeLayout(size.X, size.Y);
        Box c = l.Core;
        Box[] around = {
            new(0, 0, size.X, c.Y), new(0, c.Y + c.H, size.X, size.Y - c.Y - c.H),
            new(0, c.Y, c.X, c.H), new(c.X + c.W, c.Y, size.X - c.X - c.W, c.H)
        };
        for (int i = 0; i < 4; i++)
        {
            _frameRects[i].Position = new Vector2(around[i].X, around[i].Y);
            _frameRects[i].Size = new Vector2(around[i].W, around[i].H);
        }
        _panel.Position = new Vector2(l.Panel.X, l.Panel.Y);
        _panel.Size = new Vector2(l.Panel.W, l.Panel.H);
    }

    private void Tick(Action coreWork)
    {
        HostStatus();
        LayOut();
        Layout l = Current();
        coreWork();
        Poll();
        Sample(l, false);
        Core(_native.Note(_ctx, "godot_frame",
            $"{{\"frames_drawn\":{Engine.GetFramesDrawn()},\"process_frames\":{Engine.GetProcessFrames()},\"core_frames\":{_coreFrames}}}"),
            "c9pp_note");
        _hostFrames++;
    }

    private IEnumerable<int> Timed(string name, double duration, uint sync, int perFrame)
    {
        Phase(name);
        double t0 = Now;
        while (Now - t0 < duration * _scale)
        {
            Tick(() => { for (int i = 0; i < perFrame; i++) Frame(sync); });
            yield return 0;
        }
    }

    private IEnumerable<int> Hold(double duration, double period)
    {
        Phase("hold");
        double t0 = Now, last = -1;
        while (Now - t0 < duration * _scale)
        {
            Tick(() => {
                if (last < 0 || Now - last >= period * _scale) { Frame(1); last = Now; }
            });
            yield return 0;
        }
    }

    private IEnumerable<int> Resize(string name, int width, int height)
    {
        Phase(name);
        DisplayServer.WindowSetSize(new Vector2I(width, height));
        double t0 = Now;
        while (true)
        {
            Layout l = Current();
            bool settled = l.Core.W == width - 80 && l.Core.H == height - 80;
            if (settled || Now - t0 >= 1.0 * _scale)
            {
                Vector2I size = DisplayServer.WindowGetSize();
                Core(_native.Note(_ctx, "host_resize",
                    $"{{\"requested\":[{width},{height}],\"client\":[{size.X},{size.Y}],\"settled\":{(settled ? "true" : "false")}}}"),
                    "c9pp_note");
                Core(_native.Resize(_ctx, l.Core.X, l.Core.Y, (uint)l.Core.W, (uint)l.Core.H), "c9pp_resize");
                break;
            }
            Tick(() => { });
            yield return 0;
        }
        double t1 = Now;
        while (Now - t1 < 2.0 * _scale)
        {
            Tick(() => Frame(1));
            yield return 0;
        }
    }

    private unsafe void AfterRemoval()
    {
        HostStatus(); // Godot's device must be untouched by the core's removal.
        C9FrameRecord r = new() { struct_size = 40 };
        int attempt = _native.frame(_ctx, _nextFrame++, 1, &r);
        Core(_native.Note(_ctx, "frame_after_removal", $"{{\"status\":{attempt}}}"), "c9pp_note");
        C9StatsRow sr = new() { struct_size = 48 };
        _native.poll(_ctx, &sr);
        Sample(Current(), true);
    }

    private unsafe void Recover()
    {
        HostStatus();
        uint epoch = 0;
        Core(_native.recover(_ctx, 0, &epoch), "c9pp_recover");
        HostStatus();
    }

    private IEnumerable<int> Loss()
    {
        foreach (int step in Timed("loss_pre", 0.5, 1, 1)) yield return step;
        Phase("loss");
        int removed = _native.RemoveDevice(_ctx);
        if (removed == Native.E_UNSUPPORTED)
        {
            Core(_native.Note(_ctx, "loss_skipped", JsonSerializer.Serialize(new Dictionary<string, string> {
                ["reason"] = _native.LastError(_ctx) })), "c9pp_note");
        }
        else
        {
            Core(removed, "c9pp_remove_device");
            AfterRemoval();
            yield return 0; // Let Godot draw one frame with the core removed.
            Recover();
        }
        foreach (int step in Timed("recovered", 5.0, 1, 1)) yield return step;
    }

    private unsafe void Attach()
    {
        RenderingDevice rd = RenderingServer.GetRenderingDevice();
        if (rd == null || RenderingServer.GetCurrentRenderingDriverName() != "d3d12")
            throw new HarnessException(2, "Godot is not running on D3D12");
        _hostDevice = rd.GetDriverResource(RenderingDevice.DriverResource.LogicalDevice, default, 0);
        ulong adapter = rd.GetDriverResource(RenderingDevice.DriverResource.PhysicalDevice, default, 0);
        ulong hwnd = (ulong)DisplayServer.WindowGetNativeHandle(DisplayServer.HandleType.WindowHandle);
        if (_hostDevice == 0 || adapter == 0 || hwnd == 0) throw new HarnessException(2, "Godot device, adapter or window handle unavailable");
        try { _native = new Native(_dll); }
        catch (Exception e) { throw new HarnessException(2, "c9pp.dll load failed: " + e.Message); }
        Vector2I size = DisplayServer.WindowGetSize();
        Layout l = MakeLayout(size.X, size.Y);
        C9AttachDesc d = new()
        {
            struct_size = 72, hwnd = hwnd, adapter = adapter, host_device = _hostDevice,
            x = l.Core.X, y = l.Core.Y, width = (uint)l.Core.W, height = (uint)l.Core.H,
            cell_px = Cell, max_latency = 2, wait_timeout_ms = 100, base_rgb = Base
        };
        C9AttachInfo info = new() { struct_size = 48 };
        nint ctx = 0;
        int attached;
        fixed (byte* log = Native.Utf8Z(Path.Combine(_out, "core.jsonl")))
            attached = _native.attach(&d, log, &info, &ctx);
        if (attached != Native.OK) throw new HarnessException(2, "c9pp_attach: " + _native.LastError(0));
        _ctx = ctx;
        var version = Engine.GetVersionInfo();
        _meta["harness"] = "godot";
        _meta["abi_version"] = Native.ABI_VERSION;
        _meta["engine_version"] = version["string"].AsString();
        _meta["driver"] = RenderingServer.GetCurrentRenderingDriverName();
        _meta["rendering_method"] = RenderingServer.GetCurrentRenderingMethod();
        _meta["adapter"] = RenderingServer.GetVideoAdapterName();
        _meta["render_on_main_thread"] = RenderingServer.IsOnRenderThread();
        _meta["window_transparent_flag"] = DisplayServer.WindowGetFlag(DisplayServer.WindowFlags.Transparent);
        _meta["viewport_transparent_bg"] = GetViewport().TransparentBg;
        _meta["vsync_mode"] = DisplayServer.WindowGetVsyncMode().ToString();
        _meta["topmost"] = _topmost;
        _meta["scale"] = _scale;
        _meta["client_size"] = new[] { size.X, size.Y };
        _meta["device_source"] = info.device_source;
        _meta["device_independent"] = info.device_independent;
        _meta["factory_hr"] = info.factory_hr;
        _meta["target_hr"] = info.target_hr;
        _meta["ui_rgb"] = new[] { 230, 120, 20 };
        _meta["frame_rgb"] = new[] { 60, 60, 60 };
        _meta["base_rgb"] = new[] { 24, 64, 140 };
        File.WriteAllText(Path.Combine(_out, "meta.json"), JsonSerializer.Serialize(_meta, new JsonSerializerOptions { WriteIndented = true }) + "\n");
        Core(_native.Note(_ctx, "host_colours", "{\"ui_rgb\":[230,120,20],\"frame_rgb\":[60,60,60],\"base_rgb\":[24,64,140]}"), "c9pp_note");
    }

    private IEnumerable<int> Timeline()
    {
        // Let the window appear and Godot's swap chain settle before attaching beneath it.
        for (int i = 0; i < 10; i++) { LayOut(); yield return 0; }
        Attach();
        foreach (int s in Timed("warmup", 2.0, 1, 1)) yield return s;
        foreach (int s in Timed("steady", 10.0, 1, 1)) yield return s;
        foreach (int s in Hold(3.0, 0.3)) yield return s;
        foreach (int s in Timed("burst", 5.0, 0, 3)) yield return s;
        foreach (int s in Resize("resize_a", ResizeWidth, ResizeHeight)) yield return s;
        foreach (int s in Resize("resize_b", Width, Height)) yield return s;
        foreach (int s in Loss()) yield return s;
        Phase("finish");
    }

    private void Finish(int code, string reason)
    {
        if (_finished) return;
        _finished = true;
        if (_ctx != 0)
        {
            if (code != 0)
                _native.Note(_ctx, "harness_stop", JsonSerializer.Serialize(new Dictionary<string, object> { ["exit_code"] = code, ["reason"] = reason }));
            _native.Detach(_ctx);
            _ctx = 0;
        }
        Dictionary<string, object> result = new()
        {
            ["harness"] = "godot", ["exit_code"] = code, ["reason"] = reason, ["phases"] = _phases,
            ["core_frames"] = _coreFrames, ["host_frames"] = _hostFrames, ["samples"] = _samples,
            ["frames_drawn"] = Engine.GetFramesDrawn()
        };
        try { File.WriteAllText(Path.Combine(_out, "result.json"), JsonSerializer.Serialize(result, new JsonSerializerOptions { WriteIndented = true }) + "\n"); }
        catch (Exception e) { Console.Error.WriteLine("c9probe: cannot write result.json: " + e.Message); }
        Console.WriteLine($"c9probe: exit {code} ({reason})");
        GetTree().Quit(code);
    }

    public override void _ExitTree()
    {
        if (_ctx != 0) { _native.Detach(_ctx); _ctx = 0; }
    }
}
