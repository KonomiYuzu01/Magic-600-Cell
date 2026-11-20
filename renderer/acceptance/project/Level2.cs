using Godot;
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

// A separate controller keeps the level 1 diagnostics and their frame/readback
// behavior intact. The mailbox is the only render-to-main state transition.
public partial class Level2 : Node
{
    private enum Stage { Settle, Building, Warmup, Run, Stopping, Finished }
    private Stage _stage;
    private readonly object _gate = new();
    private readonly Queue<Action> _mailbox = new();
    private Level2Arguments _args;
    private Level2Result _result;
    private Level2Windows _windows;
    private TextureRect _image;
    private readonly List<Texture2Drd> _views = new();
    private Rid[] _wrapped = Array.Empty<Rid>();
    private long _iteration, _preDrawIteration, _startup, _sceneFrame;
    private bool _expectDraw, _aborted;
    private int _warmupIndex, _exitCode;
    private string _reason;

    // Render-thread-owned state.
    private RenderingDevice _rd;
    private Native _native;
    private nint _context;
    private readonly Rid[] _slots = new Rid[3];
    private readonly ulong[] _pointers = new ulong[3], _imported = new ulong[3];
    private readonly bool[] _registered = new bool[3];
    private readonly List<Rid> _rids = new();
    private Rid _display, _flush;
    private int _width, _height, _settled;
    private bool _initializing, _ending, _sceneLoaded, _traceBegun, _traceEnded, _writeRun, _deviceRemoved;
    // C9 U4, route B (--sa2-route surface): the window's handle and whether the core's surface is attached.
    private nint _hwnd;
    private bool _surfaceAttached;
    private long _firstProduce, _traceStart, _nextSample, _foregroundWaitStart;
    private ulong _pendingShown;
    private uint _pendingWarmupSlot;
    private bool _pendingWarmup;
    // C9 U5c (--sa2-stages): the compiled handles (vertex, fragment), whether the pair is attached, the last reported
    // row's frame, the first and last traced scene frame, and every sa2_stage_report row as one JSON line.
    private readonly ulong[] _stageHandles = new ulong[2];
    private bool _stagesAttached;
    private ulong _lastStageRow, _traceFirst, _traceLast;
    private readonly List<string> _stageRows = new();
    private static readonly string[] StageKinds = { "vertex", "fragment" };
    private const string StageContract = "m600-stage-hlsl/1", StageArraysFormat = "magic600-stage-arrays/1";
    // The built-in projection's distance (renderer/core/src/scene.cpp constants()); the gate vertex stage's d4.
    private const float BuiltinD4 = 1.18f;

    public override void _Ready()
    {
        string[] values = OS.GetCmdlineUserArgs();
        try { _args = Level2Arguments.Parse(values); }
        catch (Exception e) when (e is ArgumentException || e is IOException || e is UnauthorizedAccessException)
        {
            Console.Error.WriteLine("sa2 l2: " + e.Message);
            try
            {
                string output = Level2Arguments.UsageOutput(values);
                if (output != null)
                {
                    Level2Result usage = new(new Level2Arguments());
                    foreach (string key in new List<string>(usage.options.Keys)) usage.options[key] = null;
                    foreach (string key in new List<string>(usage.configuration.Keys))
                        if (key != "framework") usage.configuration[key] = null;
                    usage.files["dll"] = null;
                    usage.exit_code = 1; usage.reason = "usage";
                    usage.Write(output);
                }
            }
            catch (Exception writeError) { Console.Error.WriteLine("sa2 l2 harness: " + writeError.Message); }
            _stage = Stage.Finished; GetTree().Quit(1); return;
        }
        _result = new(_args);
        try
        {
            string[] processArgs = Level2Windows.ProcessArguments();
            int separator = Array.IndexOf(processArgs, "--");
            int engineEnd = separator < 0 ? processArgs.Length : separator;
            bool validation = Array.IndexOf(processArgs, "--gpu-validation", 1, engineEnd - 1) >= 0;
            int thread = Array.IndexOf(processArgs, "--render-thread", 1, engineEnd - 1);
            if (validation != _args.gpu_validation || thread < 0 || thread + 1 >= engineEnd || processArgs[thread + 1] != _args.render_thread)
                throw new ArgumentException("engine and user validation/render-thread options disagree");
            _result.configuration["engine_arguments"] = Level2Windows.EngineArguments(processArgs);
            var version = Engine.GetVersionInfo();
            string hash = version["hash"].AsString();
            string normalized = $"{version["major"]}.{version["minor"]}.{version["patch"]}.{version["status"]}.mono.{version["build"]}.{hash[..Math.Min(9, hash.Length)]}";
            _result.configuration["framework_version"] = normalized;
            _result.configuration["rendering_driver"] = RenderingServer.GetCurrentRenderingDriverName();
            _result.configuration["rendering_method"] = RenderingServer.GetCurrentRenderingMethod();
            _result.configuration["render_thread_separate_observed"] = !RenderingServer.IsOnRenderThread();
            _result.environment["adapter"] = RenderingServer.GetVideoAdapterName();
            if (normalized != "4.7.2.stable.mono.official.ed1daf0bf") throw new InvalidOperationException("engine-version");
            if ((string)_result.configuration["rendering_driver"] != "d3d12") throw new InvalidOperationException("driver-not-d3d12");
            if (!RenderingServer.IsOnRenderThread() != (_args.render_thread == "separate")) throw new InvalidOperationException("render-thread-mismatch");
            if (System.Environment.GetEnvironmentVariable("M600_SA2_INJECT_UNCONFIRMED_DRAIN") == "1")
                throw new ArgumentException("drain injection is inapplicable in level 2");
            _result.files["godot:exe"] = Path.GetFullPath(System.Environment.ProcessPath);
            _result.files["godot:assembly"] = ProjectAssemblyPath(typeof(Smoke).Assembly);
            _result.assembly_mvid = typeof(Smoke).Assembly.ManifestModule.ModuleVersionId.ToString("D");
            foreach (string name in new[] { "project.godot", "Main.tscn", "Smoke.cs" })
                _result.files["godot:" + name] = ProjectSettings.GlobalizePath("res://" + name);
            Window window = GetWindow();
            GetTree().AutoAcceptQuit = false;
            window.CloseRequested += CloseRequested;
            window.ContentScaleMode = Window.ContentScaleModeEnum.Disabled;
            window.ContentScaleFactor = 1.0f;
            window.ContentScaleSize = Vector2I.Zero;
            GetViewport().Scaling3DMode = Viewport.Scaling3DModeEnum.Bilinear;
            GetViewport().Scaling3DScale = 1.0f;
            DisplayServer.WindowSetCurrentScreen(DisplayServer.GetPrimaryScreen());
            DisplayServer.WindowSetMode(DisplayServer.WindowMode.ExclusiveFullscreen);
            DisplayServer.WindowSetFlag(DisplayServer.WindowFlags.AlwaysOnTop, true);
            Input.MouseMode = Input.MouseModeEnum.Hidden;
            _image = GetParent().GetNode<TextureRect>("Image");
            _image.ExpandMode = TextureRect.ExpandModeEnum.IgnoreSize;
            _image.StretchMode = TextureRect.StretchModeEnum.Scale;
            _image.MouseFilter = Control.MouseFilterEnum.Ignore;
            _result.scaling["content_scale_mode"] = window.ContentScaleMode == Window.ContentScaleModeEnum.Disabled ? "disabled" : window.ContentScaleMode.ToString();
            _result.scaling["content_scale_factor"] = window.ContentScaleFactor;
            // Direct full-rectangle mapping; no fit, aspect preservation or crop.
            // Half-target injection changes only the registered target extent.
            _result.scaling["texture_stretch"] = _image.StretchMode == TextureRect.StretchModeEnum.Scale ? "none" : _image.StretchMode.ToString();
            _result.configuration["texture_expand_mode"] = _image.ExpandMode.ToString();
            _result.configuration["texture_stretch_mode"] = _image.StretchMode.ToString();
            _result.configuration["scaling_3d_mode"] = GetViewport().Scaling3DMode.ToString();
            _result.configuration["scaling_3d_scale"] = GetViewport().Scaling3DScale;
            _result.configuration["window_mode"] = DisplayServer.WindowGetMode() == DisplayServer.WindowMode.ExclusiveFullscreen ? "exclusive_fullscreen" : DisplayServer.WindowGetMode().ToString();
            bool disabled = DisplayServer.WindowGetVsyncMode() == DisplayServer.VSyncMode.Disabled;
            _result.configuration["vsync"] = disabled ? "disabled" : DisplayServer.WindowGetVsyncMode().ToString();
            _result.environment["vsync"] = !disabled;
            _result.environment["presentation_interval"] = disabled ? 0 : 1;
            // Route B presents as the product does: the core at SA2_SURFACE_SYNC_INTERVAL, Godot's window with vsync.
            bool surface = _args.route == "surface";
            if (!surface && !disabled) throw new InvalidOperationException("vsync-not-disabled");
            if (surface && disabled) throw new InvalidOperationException("vsync-disabled");
            if (surface) RequireSurfaceWindow();
            _hwnd = (nint)DisplayServer.WindowGetNativeHandle(DisplayServer.HandleType.WindowHandle);
            _windows = new(_hwnd);
            _result.qpc_frequency = _windows.Frequency;
            _result.window["display_required"] = _windows.DisplayRequired;
            _result.window["topmost"] = _windows.Topmost;
            if (!_windows.DisplayRequired || !_windows.Topmost) throw new InvalidOperationException("window-state");
            _startup = Level2Windows.Clock();
            RenderingServer.FramePreDraw += PreDraw;
            _stage = Stage.Settle;
        }
        catch (Exception e) { Fail(1, e is ArgumentException ? "usage" : e.Message); FinishMain(); }
    }

    // C9 U4, route B: the core's layer lies beneath the window's content, so Godot must draw a transparent window. The
    // build's override.cfg (renderer/acceptance/route-b) sets what the product shell's project.godot sets; a window that
    // is not transparent here fails before any device work. Whether the layer shows through in exclusive fullscreen
    // is for the owner's run to show.
    private void RequireSurfaceWindow()
    {
        GetViewport().TransparentBg = true;
        bool allowed = ProjectSettings.GetSetting("display/window/per_pixel_transparency/allowed").AsBool();
        bool transparent = DisplayServer.WindowGetFlag(DisplayServer.WindowFlags.Transparent);
        _result.configuration["per_pixel_transparency"] = allowed;
        _result.configuration["window_transparent"] = transparent;
        _result.configuration["transparent_background"] = GetViewport().TransparentBg;
        if (!allowed || !transparent || !GetViewport().TransparentBg) throw new InvalidOperationException("surface-transparency");
        string overrides = ProjectSettings.GlobalizePath("res://override.cfg");
        if (!File.Exists(overrides)) throw new InvalidOperationException("surface-override");
        _result.files["godot:override.cfg"] = overrides;
    }

    // Godot loads the project assembly from memory, so its Location is empty (FRAMEWORK-FACTS G9).
    // The load context keeps the path of the file it read; FullName checks its name.
    // The finalizer also compares the recorded MVID with that file's metadata.
    private static string ProjectAssemblyPath(System.Reflection.Assembly assembly)
    {
        try
        {
            var context = System.Runtime.Loader.AssemblyLoadContext.GetLoadContext(assembly);
            string path = context?.GetType().GetProperty("AssemblyLoadedPath")?.GetValue(context) as string;
            if (!string.IsNullOrEmpty(path) && File.Exists(path)
                && System.Reflection.AssemblyName.GetAssemblyName(path).FullName == assembly.FullName)
                return Path.GetFullPath(path);
        }
        catch (Exception) { }
        throw new InvalidOperationException("assembly-path");
    }

    private void PreDraw() => _preDrawIteration = _iteration;
    private void Post(Action action) { lock (_gate) _mailbox.Enqueue(action); }
    private bool Aborted { get { lock (_gate) return _aborted; } }
    private void Fail(int code, string reason)
    {
        lock (_gate)
        {
            if (!_aborted) { _exitCode = code; _reason = reason; }
            _aborted = true;
            _mailbox.Enqueue(FinishMain);
        }
    }

    private void CloseRequested() { Fail(1, "window-closed"); FinishMain(); }
    public override void _UnhandledKeyInput(InputEvent @event)
    {
        if (@event is InputEventKey key && key.Pressed && key.Keycode == Key.Escape) CloseRequested();
    }

    public override void _Process(double delta)
    {
        if (_stage == Stage.Finished || _stage == Stage.Stopping) { Dispatch(); return; }
        _iteration++;
        if (_expectDraw && _preDrawIteration != _iteration - 1) Fail(1, "not-drawn");
        _expectDraw = false;
        Dispatch();
        if (_stage == Stage.Stopping || _stage == Stage.Finished) return;
        try
        {
            Vector2 extent = _image.GetGlobalRect().Size;
            Level2Size displayed = new((int)Math.Round(extent.X), (int)Math.Round(extent.Y));
            if (_stage == Stage.Settle) Queue(() => Settle(displayed));
            else if (_stage == Stage.Warmup)
            {
                uint slot = (uint)(_warmupIndex++ % 3);
                Rid[] wrapped = _wrapped;
                _image.Texture = _views[_args.route == "import-copy" ? 0 : (int)slot];
                Queue(() => Warmup(slot, wrapped));
                _expectDraw = true;
                if (_warmupIndex >= _args.warmup) _stage = Stage.Run;
            }
            else if (_stage == Stage.Run)
            {
                long frame = ++_sceneFrame;
                Rid[] wrapped = _wrapped;
                if (_args.route != "surface") _image.Texture = _views[_args.route == "import-copy" ? 0 : (int)(frame % 3)];
                Queue(() => Frame((ulong)frame, displayed, wrapped));
                _expectDraw = true;
            }
        }
        catch (Exception e) { Fail(1, e.Message); FinishMain(); }
    }

    private void Dispatch()
    {
        Action[] messages;
        lock (_gate) { messages = _mailbox.ToArray(); _mailbox.Clear(); }
        foreach (Action action in messages) action();
    }

    private void Queue(Action action, bool finishing = false)
    {
        try
        {
            RenderingServer.CallOnRenderThread(Callable.From(() => {
                if ((Aborted || _ending) && !finishing) return;
                try { action(); }
                catch (Exception e)
                {
                    Console.Error.WriteLine("sa2 l2: " + e.Message);
                    Fail(1, e.Message);
                    if (finishing) Complete();
                }
            }));
        }
        catch (Exception e) { Fail(1, e.Message); Complete(); }
    }

    private Level2Sizes ReadSizes(Level2Size displayed)
    {
        Dictionary<string, object> display = _windows.Display(out string displayAdapter);
        string adapter = (string)_result.environment["adapter"], driver = (string)_result.environment["driver"];
        _result.environment["display"] = display;
        _result.environment["presenting_adapter"] = string.IsNullOrEmpty(adapter) || driver == null ? null : adapter + ", driver " + driver
            + (displayAdapter == adapter ? ", drives the window's display" : ", display driven by another adapter");
        Level2Size backbuffer = new(_rd.ScreenGetWidth(), _rd.ScreenGetHeight());
        _result.environment["backbuffer"] = backbuffer;
        return new() { display = new((int)display["width"], (int)display["height"]), window = _windows.ClientSize(),
            backbuffer = backbuffer, displayed = displayed, target = _width == 0 ? new() : new(_width, _height) };
    }

    private void Settle(Level2Size displayed)
    {
        if (_initializing) return;
        _rd ??= RenderingServer.GetRenderingDevice();
        if (_rd == null) throw new InvalidOperationException("rendering-device-unavailable");
        // System awareness is enough (FRAMEWORK-FACTS G10): EnumDisplaySettingsW reports physical pixels in every DPI
        // context, so a window that Windows scales never matches the display size and never settles.
        _result.dpi_awareness = Level2Windows.DpiAwareness;
        if (_result.dpi_awareness != "per-monitor-v2" && _result.dpi_awareness != "system") throw new InvalidOperationException("dpi-awareness");
        Level2Sizes current = ReadSizes(displayed);
        _result.sizes = current;
        _settled = current.WindowSettled() ? _settled + 1 : 0;
        if (_settled < 2)
        {
            if (Level2Windows.Clock() - _startup > _windows.Frequency * 5) throw new InvalidOperationException("fullscreen-size-unsettled");
            return;
        }
        _initializing = true;
        Post(() => _stage = Stage.Building);
        Initialize((int)displayed.width, (int)displayed.height);
    }

    private unsafe void NativeStatus(string function, int status)
    {
        if (status == Native.SA2_OK) return;
        byte* error = stackalloc byte[2048]; error[0] = 0;
        _native.sa2_last_error(function == "sa2_identity" ? 0 : _context, error, 2048);
        _result.dll_status["last_status"] = status;
        _result.dll_status["last_error"] = Utf8(error, 2048);
        if (status == Native.SA2_E_DEVICE_REMOVED) _deviceRemoved = true;
    }

    private void Require(string function, int status)
    {
        NativeStatus(function, status);
        if (status != Native.SA2_OK) throw new InvalidOperationException(function);
    }

    private static unsafe string Utf8(byte* buffer, int capacity)
    {
        int length = 0; while (length < capacity - 1 && buffer[length] != 0) length++;
        return Encoding.UTF8.GetString(new ReadOnlySpan<byte>(buffer, length));
    }

    private unsafe void Initialize(int width, int height)
    {
        _width = _args.debug_half_target ? width / 2 : width;
        _height = _args.debug_half_target ? height / 2 : height;
        if (_width < 128 || _height < 128) throw new InvalidOperationException("target-too-small");
        _native = new Native(_args.dll_path);
        _result.files["dll"] = _native.ModulePath;
        _result.dll_status["abi_version"] = _native.sa2_abi_version();
        _result.dll_status["last_status"] = 0; _result.dll_status["last_error"] = "";
        ulong device = _rd.GetDriverResource(RenderingDevice.DriverResource.LogicalDevice, default, 0);
        ulong queue = _rd.GetDriverResource(RenderingDevice.DriverResource.CommandQueue, default, 0);
        ulong adapter = _rd.GetDriverResource(RenderingDevice.DriverResource.PhysicalDevice, default, 0);
        Sa2DeviceInfo info = new() { struct_size = Native.DeviceInfoSize };
        Require("sa2_probe", _native.sa2_probe(device, queue, adapter, &info));
        _result.debug["debug_layer"] = info.debug_layer;
        _result.environment["driver"] = info.umd_version == 0 ? null
            : $"{info.umd_version >> 48}.{(info.umd_version >> 32) & 65535}.{(info.umd_version >> 16) & 65535}.{info.umd_version & 65535}";
        _result.environment["warp"] = info.vendor_id == 0x1414 && info.device_id == 0x8c;
        _result.environment["msaa"] = 1;
        if (info.queue_type != 0 || info.queue_device_matches != 1 || info.adapter_matches_device != 1)
            throw new InvalidOperationException("device-ownership");
        int state = _args.handover == "render-target" ? Native.SA2_STATE_RENDER_TARGET
            : _args.route == "import-copy" ? Native.SA2_STATE_COPY_SOURCE : Native.SA2_STATE_ALL_SHADER_RESOURCE;
        int barrier = _args.barriers == "legacy" ? Native.SA2_BARRIERS_LEGACY : _args.barriers == "enhanced" ? Native.SA2_BARRIERS_ENHANCED : Native.SA2_BARRIERS_MATCH_GODOT;
        Sa2Config config = new() { struct_size = Native.ConfigSize, queue_mode = _args.queue == "own" ? Native.SA2_QUEUE_OWN : Native.SA2_QUEUE_SAME,
            barrier_api = barrier, state_before_write = state, state_after_write = state, wait_timeout_ms = 5000, debug_callback = _args.gpu_validation ? 1 : 0 };
        if (_args.route == "surface") { InitializeSurface(config, adapter); return; }
        nint context = 0;
        Require("sa2_attach", _native.sa2_attach(device, queue, &config, &context));
        _context = context;
        _flush = MakeTexture(4, 4, RenderingDevice.TextureUsageBits.SamplingBit | RenderingDevice.TextureUsageBits.CanCopyFromBit, false, new byte[64]);
        BuildRing();
        _result.sizes.target = new(_width, _height);
        Rid[] slots = (Rid[])_slots.Clone(); Rid display = _display;
        Post(() => {
            if (Aborted) return;
            foreach (Rid rid in _args.route == "import-copy" ? new[] { display } : slots)
                _views.Add(new Texture2Drd { TextureRdRid = rid });
            _wrapped = _views.ConvertAll(texture => texture.GetRid()).ToArray();
            _stage = _args.warmup == 0 ? Stage.Run : Stage.Warmup;
        });
    }

    // C9 U4, route B: the core's own device on Godot's adapter and its surface over the window's client area at the
    // displayed size; no ring and no Godot texture. Each scene frame is drawn and presented by
    // sa2_scene_produce(SA2_TARGET_SURFACE). The swap chain's address names the core's presents in the PresentMon
    // capture (sa2_surface_swap_chain); the finalizer measures that chain and lists Godot's own as ignored.
    private unsafe void InitializeSurface(Sa2Config config, ulong adapter)
    {
        nint context = 0;
        Require("sa2_create", _native.sa2_create(Native.AdapterLuid(adapter), &config, &context));
        _context = context;
        Require("sa2_surface_attach", _native.sa2_surface_attach(_context, (ulong)_hwnd, 0, 0, (uint)_width, (uint)_height));
        _surfaceAttached = true;
        ulong chain = 0;
        Require("sa2_surface_swap_chain", _native.sa2_surface_swap_chain(_context, &chain));
        _result.presentation = new() { ["route"] = "surface", ["swap_chain"] = "0x" + chain.ToString("X16"),
            ["sync_interval"] = Native.SA2_SURFACE_SYNC_INTERVAL };
        _result.sizes.target = new(_width, _height);
        Post(() => { if (!Aborted) _stage = Stage.Run; });
    }

    private Rid MakeTexture(int width, int height, RenderingDevice.TextureUsageBits usage, bool ring = true, byte[] data = null)
    {
        using RDTextureFormat format = new() { Width = (uint)width, Height = (uint)height, Depth = 1, ArrayLayers = 1, Mipmaps = 1,
            TextureType = RenderingDevice.TextureType.Type2D, Samples = RenderingDevice.TextureSamples.Samples1,
            Format = RenderingDevice.DataFormat.R8G8B8A8Unorm, UsageBits = usage };
        using RDTextureView view = new();
        var initial = new Godot.Collections.Array<byte[]>();
        if (data != null) initial.Add(data);
        Rid rid = _rd.TextureCreate(format, view, initial);
        if (!rid.IsValid) throw new InvalidOperationException("texture-create");
        _rd.SetResourceName(rid, "SA2 level 2 texture");
        if (ring) _rids.Add(rid);
        return rid;
    }

    private unsafe void BuildRing()
    {
        var sampled = RenderingDevice.TextureUsageBits.SamplingBit | RenderingDevice.TextureUsageBits.ColorAttachmentBit | RenderingDevice.TextureUsageBits.CanCopyFromBit;
        for (uint k = 0; k < 3; k++)
        {
            if (_args.route == "export")
            {
                _slots[k] = MakeTexture(_width, _height, sampled);
                _pointers[k] = _rd.GetDriverResource(RenderingDevice.DriverResource.Texture, _slots[k], 0);
            }
            else
            {
                ulong resource = 0;
                Require("sa2_create_texture", _native.sa2_create_texture(_context, (uint)_width, (uint)_height, Native.SA2_STATE_RENDER_TARGET, &resource));
                _imported[k] = _pointers[k] = resource;
                _slots[k] = _rd.TextureCreateFromExtension(RenderingDevice.TextureType.Type2D, RenderingDevice.DataFormat.R8G8B8A8Unorm,
                    RenderingDevice.TextureSamples.Samples1, sampled, resource, (ulong)_width, (ulong)_height, 1, 1, 1);
                if (!_slots[k].IsValid) throw new InvalidOperationException("texture-import");
                _rids.Add(_slots[k]);
            }
            Require("sa2_register_slot", _native.sa2_register_slot(_context, k, _pointers[k], (uint)_width, (uint)_height, _args.route == "export" ? 1 : 0));
            _registered[k] = true;
        }
        if (_args.route == "import-copy") _display = MakeTexture(_width, _height,
            RenderingDevice.TextureUsageBits.SamplingBit | RenderingDevice.TextureUsageBits.CanCopyFromBit | RenderingDevice.TextureUsageBits.CanCopyToBit);
    }

    private void CopyDisplay(uint slot)
    {
        if (_args.route == "import-copy" && _rd.TextureCopy(_slots[slot], _display, Vector3.Zero, Vector3.Zero, new Vector3(_width, _height, 1), 0, 0, 0, 0) != Error.Ok)
            throw new InvalidOperationException("texture-copy");
    }

    private unsafe void MarkPrevious()
    {
        // The preceding draw/present is already queued before this render callback.
        // Warm-up uses fence value zero; scene production starts monotonically at 1.
        if (_pendingWarmup)
        {
            Require("sa2_mark_shown", _native.sa2_mark_shown(_context, _pendingWarmupSlot, 0));
            _pendingWarmup = false;
        }
        if (_pendingShown != 0)
        {
            Require("sa2_mark_shown", _native.sa2_mark_shown(_context, (uint)(_pendingShown % 3), _pendingShown));
            _pendingShown = 0;
        }
    }

    private void CheckWrap(Rid[] wrapped)
    {
        if (_args.route == "import-texture2drd")
        {
            // The level 1 editor-binary refusal remains a failure in level 2.
            foreach (Rid texture in wrapped)
                if (!RenderingServer.TextureGetRdTexture(texture).IsValid) throw new InvalidOperationException("texture2drd-refused");
        }
    }

    private void Warmup(uint slot, Rid[] wrapped)
    {
        MarkPrevious();
        CheckWrap(wrapped);
        CopyDisplay(slot);
        _pendingWarmupSlot = slot; _pendingWarmup = true;
    }

    private unsafe void LoadScene()
    {
        if (_args.route == "export")
            for (int k = 0; k < 3; k++)
                if (_rd.GetDriverResource(RenderingDevice.DriverResource.Texture, _slots[k], 0) != _pointers[k])
                    throw new InvalidOperationException("resource-changed");
        byte[] inject = _args.inject == null ? null : Encoding.UTF8.GetBytes(_args.inject + "\0");
        if (_args.scene == "wj")
        {
            // ABI 3: W-J's pose path. The DLL copies the menu, data directory and injection strings.
            byte[] menu = Encoding.UTF8.GetBytes(_args.menu + "\0"), data = Encoding.UTF8.GetBytes(_args.data_path + "\0");
            fixed (byte* injection = inject)
            fixed (byte* menuText = menu)
            fixed (byte* dataText = data)
            {
                Sa2WjConfig config = new() { struct_size = Native.WjConfigSize, flags = _args.no_vram ? Native.SA2_SCENE_NO_VRAM : 0,
                    menu = menuText, data_utf8 = dataText, turn_ms = _args.turn_ms, inject = injection, trace_ms = (uint)_args.trace_ms, reserved = 0 };
                Require("sa2_scene_load_wj", _native.sa2_scene_load_wj(_context, &config));
            }
        }
        else
        {
            fixed (byte* injection = inject)
            {
                Sa2SceneConfig config = new() { struct_size = Native.SceneConfigSize, scene = (uint)(_args.scene == null ? 1 : _args.scene[1] - '0'),
                    turn_ms = _args.turn_ms, inject = injection, flags = _args.no_vram ? Native.SA2_SCENE_NO_VRAM : 0, trace_ms = (uint)_args.trace_ms };
                Require("sa2_scene_load", _native.sa2_scene_load(_context, &config));
            }
        }
        _sceneLoaded = true;
        byte* identity = stackalloc byte[16384]; identity[0] = 0;
        Require("sa2_identity", _native.sa2_identity(identity, 16384));
        _result.dll_identity = JsonSerializer.Deserialize<JsonElement>(Utf8(identity, 16384));
    }

    private unsafe void Frame(ulong frame, Level2Size displayed, Rid[] wrapped)
    {
        MarkPrevious();
        if (!_sceneLoaded)
        {
            CheckWrap(wrapped);
            // Warm-up fence value zero cannot wait for an in-flight Godot read
            // on the own queue. Complete every warm-up before scene frame 1.
            Flush();
            Require("sa2_drain", _native.sa2_drain(_context, 5000));
            LoadScene();
            if (_args.stages_path != null) AttachStages();
        }
        if (_args.mode == "geometry" || _args.mode == "lattice")
        {
            // W-J's geometry and lattice checks run in their own invocations, outside any trace (ABI 3).
            byte[] directory = Encoding.UTF8.GetBytes(_args.out_path + "\0");
            fixed (byte* path = directory)
            {
                if (_args.mode == "geometry")
                    OutputStatus("sa2_scene_geometry_check", _native.sa2_scene_geometry_check(_context, path), "geometry-check");
                else
                    OutputStatus("sa2_scene_lattice_check", _native.sa2_scene_lattice_check(_context, path), "lattice-check");
            }
            _ending = true;
            Post(FinishMain); return;
        }
        long now = Level2Windows.Clock();
        if (_traceBegun && now - _traceStart >= _args.trace_ms * (_windows.Frequency / 1000.0))
        {
            EndTrace(); _writeRun = true; _ending = true; Post(FinishMain); return;
        }
        if (!_traceBegun && _firstProduce != 0 && now - _firstProduce >= _args.preroll_ms * (_windows.Frequency / 1000.0))
        {
            if (_foregroundWaitStart == 0) _foregroundWaitStart = now;
            bool foreground = _windows.Foreground;
            if (foreground || now - _foregroundWaitStart >= _windows.Frequency * 5)
            {
                _result.window["foreground_at_trace_start"] = foreground;
                if (!foreground && _args.conditions == "enforce") { Fail(3, "foreground"); return; }
                _result.sizes = ReadSizes(displayed);
                Require("sa2_scene_trace_begin", _native.sa2_scene_trace_begin(_context));
                _traceBegun = true; _traceStart = Level2Windows.Clock(); _nextSample = _traceStart;
                now = _traceStart;
            }
        }
        if (_traceBegun && now >= _nextSample)
        {
            bool acceptable = _windows.Sample(_result);
            _result.sizes.samples++;
            if (!_result.sizes.Same(ReadSizes(displayed))) _result.sizes.samples_changed++;
            _nextSample = now + _windows.Frequency / 10;
            if (!acceptable && _args.conditions == "enforce") { EndTrace(); Fail(3, "conditions"); return; }
        }
        if (_args.route == "surface")
        {
            // Route B: the core draws and presents the frame itself; Godot shows only its transparent window.
            if (_firstProduce == 0) _firstProduce = Level2Windows.Clock();
            Require("sa2_scene_produce", _native.sa2_scene_produce(_context, Native.SA2_TARGET_SURFACE, frame));
            Produced(frame);
            return;
        }
        Require("sa2_signal_godot_free", _native.sa2_signal_godot_free(_context, frame - 1));
        if (_firstProduce == 0) _firstProduce = Level2Windows.Clock();
        Require("sa2_scene_produce", _native.sa2_scene_produce(_context, (uint)(frame % 3), frame));
        Require("sa2_godot_wait_ready", _native.sa2_godot_wait_ready(_context, frame));
        CopyDisplay((uint)(frame % 3));
        _pendingShown = frame;
        Produced(frame);
    }

    private void Produced(ulong frame)
    {
        if (_traceBegun)
        {
            _result.window["presents"] = Convert.ToInt64(_result.window["presents"]) + 1;
            // The trace's frames are the scene frames from its first produce to its last (trace.jsonl counts them).
            if (_traceFirst == 0) _traceFirst = frame;
            _traceLast = frame;
        }
        CollectStageRows();
    }

    // C9 U5c: the build's stage pair (prepare_host.py --stages) is compiled with the core's DXC and attached to the
    // loaded W1..W4 scene before its first frame, outside any trace. Each compile record goes to the run folder; a
    // failed compile ends the run as "stage-compile" with its record kept. The overdraw allowance is the smaller of
    // the two declared values; the arrays come from arrays.json, whose vertex d4 must be the built-in 1.18f.
    private unsafe void AttachStages()
    {
        string folder = _args.stages_path;
        byte[][] texts = new byte[2][];
        uint[][] arrays = new uint[2][];
        uint[] overdraws = new uint[2];
        string[] digests = new string[2], hashes = new string[2];
        uint d4;
        try
        {
            using JsonDocument given = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(folder, "arrays.json")));
            if (given.RootElement.GetProperty("format").GetString() != StageArraysFormat) throw new InvalidDataException();
            for (int k = 0; k < 2; k++)
            {
                string kind = StageKinds[k], hlsl = Path.Combine(folder, kind + ".hlsl"), json = Path.Combine(folder, kind + ".json");
                texts[k] = File.ReadAllBytes(hlsl);
                using JsonDocument record = JsonDocument.Parse(File.ReadAllBytes(json));
                JsonElement r = record.RootElement;
                hashes[k] = Convert.ToHexString(SHA256.HashData(texts[k])).ToLowerInvariant();
                if (r.GetProperty("kind").GetString() != kind || r.GetProperty("contract").GetString() != StageContract
                    || r.GetProperty("hlsl_sha256").GetString() != hashes[k] || texts[k].Length > Native.SA2_STAGE_HLSL_MAX_BYTES)
                    throw new InvalidDataException();
                overdraws[k] = r.GetProperty("overdraw").GetUInt32();
                digests[k] = r.GetProperty("digest").GetString();
                arrays[k] = StageArrays(r.GetProperty("arrays"), given.RootElement.GetProperty(kind));
                _result.files["stage:" + kind + ".hlsl"] = Path.GetFullPath(hlsl);
                _result.files["stage:" + kind + ".json"] = Path.GetFullPath(json);
            }
            JsonElement distance = given.RootElement.GetProperty("vertex").GetProperty("d4");
            if (distance.GetArrayLength() != 1) throw new InvalidDataException();
            d4 = uint.Parse(distance[0].GetString().AsSpan(2), NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture);
        }
        // Any unreadable, malformed or mismatched stage file: the folder is not the build's stage pair.
        catch (Exception) { throw new InvalidOperationException("stage-files"); }
        uint builtin = BitConverter.SingleToUInt32Bits(BuiltinD4);
        _result.files["stage:arrays.json"] = Path.GetFullPath(Path.Combine(folder, "arrays.json"));
        string native = Path.GetDirectoryName(_native.ModulePath);
        foreach (string library in new[] { "dxcompiler.dll", "dxil.dll" })
        {
            string path = Path.Combine(native, library);
            if (!File.Exists(path)) throw new InvalidOperationException("stage-compiler-missing");
            _result.files["dxc:" + library] = path;
        }
        uint overdraw = Math.Min(overdraws[0], overdraws[1]);
        _result.stages = new() {
            ["directory"] = folder, ["contract"] = StageContract, ["overdraw"] = overdraw,
            ["d4"] = new Dictionary<string, object> { ["bits"] = "0x" + d4.ToString("x8"), ["builtin_bits"] = "0x" + builtin.ToString("x8"),
                ["equal"] = d4 == builtin },
            ["attached"] = false, ["rows"] = 0, ["trace_frames"] = null
        };
        if (d4 != builtin) throw new InvalidOperationException("stage-d4");
        for (int k = 0; k < 2; k++)
        {
            string kind = StageKinds[k];
            byte[] record = new byte[Native.SA2_STAGE_RECORD_BYTES];
            ulong handle = 0; uint size = 0; int status;
            fixed (byte* text = texts[k])
            fixed (byte* output = record)
                status = _native.sa2_stage_compile(_context, k == 0 ? Native.SA2_STAGE_VERTEX : Native.SA2_STAGE_FRAGMENT, text,
                    (uint)texts[k].Length, &handle, output, (uint)record.Length, &size);
            NativeStatus("sa2_stage_compile", status);
            _stageHandles[k] = handle;
            string compiled = null;
            if (size > 1 && size <= record.Length)
            {
                WriteNew(Path.Combine(_args.out_path, "stage-compile-" + kind + ".json"), record.AsSpan(0, (int)size - 1));
                try { using JsonDocument written = JsonDocument.Parse(record.AsMemory(0, (int)size - 1)); compiled = written.RootElement.GetProperty("status").GetString(); }
                catch (Exception e) when (e is JsonException || e is KeyNotFoundException || e is InvalidOperationException) { }
            }
            _result.stages[kind] = new Dictionary<string, object> { ["digest"] = digests[k], ["hlsl_sha256"] = hashes[k],
                ["overdraw"] = overdraws[k], ["arrays"] = arrays[k].Length, ["handle"] = handle, ["compile"] = compiled ?? "unrecorded" };
            if (status != Native.SA2_OK || handle == 0) throw new InvalidOperationException("stage-compile");
        }
        fixed (uint* vertex = arrays[0])
        fixed (uint* fragment = arrays[1])
            Require("sa2_scene_stages", _native.sa2_scene_stages(_context, _stageHandles[0], _stageHandles[1], overdraw,
                vertex, (uint)arrays[0].Length, fragment, (uint)arrays[1].Length));
        _stagesAttached = true;
        _result.stages["attached"] = true;
    }

    // One stage's arrays in its declared layout ({name, offset, length}), from arrays.json's bit patterns for that kind.
    private static uint[] StageArrays(JsonElement layout, JsonElement given)
    {
        int total = 0, names = 0;
        foreach (JsonElement entry in layout.EnumerateArray())
            total = Math.Max(total, checked(entry.GetProperty("offset").GetInt32() + entry.GetProperty("length").GetInt32()));
        uint[] values = new uint[total];
        bool[] filled = new bool[total];
        foreach (JsonElement entry in layout.EnumerateArray())
        {
            int offset = entry.GetProperty("offset").GetInt32(), length = entry.GetProperty("length").GetInt32();
            JsonElement bits = given.GetProperty(entry.GetProperty("name").GetString());
            if (offset < 0 || length <= 0 || bits.GetArrayLength() != length) throw new InvalidDataException();
            for (int i = 0; i < length; i++)
            {
                string text = bits[i].GetString();
                if (text.Length != 10 || !text.StartsWith("0x", StringComparison.Ordinal) || filled[offset + i]) throw new InvalidDataException();
                values[offset + i] = uint.Parse(text.AsSpan(2), NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture);
                filled[offset + i] = true;
            }
            names++;
        }
        int declared = 0;
        foreach (JsonProperty _ in given.EnumerateObject()) declared++;
        if (declared != names || Array.IndexOf(filled, false) >= 0) throw new InvalidDataException();
        return values;
    }

    // Every completed chain frame once, in frame order: report until the frame number stops rising (sa2_interop.h).
    private unsafe void CollectStageRows()
    {
        if (!_stagesAttached) return;
        while (true)
        {
            Sa2StageFrame row = new() { struct_size = Native.StageFrameSize };
            Require("sa2_stage_report", _native.sa2_stage_report(_context, &row));
            if (row.struct_size != Native.StageFrameSize) throw new InvalidOperationException("stage-report");
            if (row.frame <= _lastStageRow) return;
            _lastStageRow = row.frame;
            _stageRows.Add(JsonSerializer.Serialize(new Dictionary<string, object> {
                ["frame"] = row.frame, ["stopped"] = row.stopped, ["bound"] = row.bound, ["allowance"] = row.allowance,
                ["vertex_stage"] = row.vertex_stage, ["fragment_stage"] = row.fragment_stage,
                ["user_pixel_invocations"] = row.user_pixel_invocations, ["builtin_pixel_invocations"] = row.builtin_pixel_invocations }));
        }
    }

    // After a successful drain every chain frame has completed: take the last rows, detach, release both handles and
    // write stage-frames.jsonl.
    private unsafe void FinishStages()
    {
        CollectStageRows();
        Require("sa2_scene_stages", _native.sa2_scene_stages(_context, 0, 0, 0, null, 0, null, 0));
        _stagesAttached = false;
        ReleaseStages();
        // WriteNew ends the file with the one newline after the last row; a newline per row here left a blank last
        // line, which the finalizer refused as stage-rows (C9 U5, 20 Nov 2026).
        WriteNew(Path.Combine(_args.out_path, "stage-frames.jsonl"), Encoding.UTF8.GetBytes(string.Join('\n', _stageRows)));
        _result.stages["rows"] = _stageRows.Count;
        _result.stages["trace_frames"] = _traceFirst == 0 ? null
            : new Dictionary<string, object> { ["first"] = _traceFirst, ["last"] = _traceLast };
    }

    private unsafe void ReleaseStages()
    {
        for (int k = 0; k < 2; k++)
            if (_stageHandles[k] != 0) { Require("sa2_stage_release", _native.sa2_stage_release(_context, _stageHandles[k])); _stageHandles[k] = 0; }
    }

    private static void WriteNew(string path, ReadOnlySpan<byte> contents)
    {
        using FileStream stream = new(path, FileMode.CreateNew, System.IO.FileAccess.Write, FileShare.None);
        stream.Write(contents);
        stream.WriteByte((byte)'\n');
        stream.Flush(true);
    }

    private unsafe void EndTrace()
    {
        if (_traceBegun && !_traceEnded)
        {
            Require("sa2_scene_trace_end", _native.sa2_scene_trace_end(_context));
            _traceEnded = true;
        }
    }

    private void OutputStatus(string function, int status, string reason)
    {
        NativeStatus(function, status);
        if (status == Native.SA2_E_CHECK_FAILED) { _exitCode = 2; _reason = reason; }
        else if (status != Native.SA2_OK) throw new InvalidOperationException(function);
    }

    private void FinishMain()
    {
        if (_stage == Stage.Stopping || _stage == Stage.Finished) return;
        _stage = Stage.Stopping;
        _expectDraw = false;
        if (_image != null) _image.Texture = null;
        foreach (Texture2Drd texture in _views) texture.Dispose();
        _views.Clear();
        Queue(Teardown, finishing: true);
    }

    private void Flush()
    {
        if (_flush.IsValid && _rd.TextureGetData(_flush, 0).Length != 64) throw new InvalidOperationException("godot-flush");
    }

    private unsafe void Teardown()
    {
        if (_context != 0)
        {
            if (!_deviceRemoved) { MarkPrevious(); EndTrace(); Flush(); }
            int drain = _native.sa2_drain(_context, 5000);
            NativeStatus("sa2_drain", drain);
            if (drain != Native.SA2_OK && drain != Native.SA2_E_DEVICE_REMOVED) throw new InvalidOperationException("sa2_drain");
            // A removal reported here is a device failure (exit 1), even after a complete trace; teardown goes on.
            if (drain == Native.SA2_E_DEVICE_REMOVED) Fail(1, "sa2_drain");
            // C9 U5c: the chain's rows, detach and release; after a failed compile, release what was compiled.
            if (drain == Native.SA2_OK)
            {
                if (_stagesAttached) FinishStages();
                else ReleaseStages();
            }
            if (_writeRun && !Aborted && drain == Native.SA2_OK)
            {
                byte[] directory = Encoding.UTF8.GetBytes(_args.out_path + "\0");
                fixed (byte* path = directory) OutputStatus("sa2_scene_write_run", _native.sa2_scene_write_run(_context, path),
                    _args.scene == "wj" ? "label-or-pose-check" : "label-check");
            }
            if (_sceneLoaded) { Require("sa2_scene_unload", _native.sa2_scene_unload(_context)); _sceneLoaded = false; }
            for (uint k = 0; k < 3; k++)
                if (_registered[k]) { Require("sa2_unregister_slot", _native.sa2_unregister_slot(_context, k)); _registered[k] = false; }
        }
        for (int i = _rids.Count - 1; i >= 0; i--) _rd.FreeRid(_rids[i]);
        _rids.Clear();
        if (_context != 0)
        {
            if (!_deviceRemoved)
            {
                Flush(); // Complete deferred RID destruction before native Release.
                Require("sa2_drain", _native.sa2_drain(_context, 5000));
            }
            if (_args.gpu_validation) ReadDebug();
            for (int k = 0; k < 3; k++)
                if (_imported[k] != 0)
                {
                    uint count = 0;
                    Require("sa2_release_texture", _native.sa2_release_texture(_context, _imported[k], &count));
                    if (count != 0) throw new InvalidOperationException("refcount-nonzero");
                    _imported[k] = 0;
                }
            if (_surfaceAttached)
            {
                Require("sa2_surface_detach", _native.sa2_surface_detach(_context));
                _surfaceAttached = false;
            }
            Require("sa2_detach", _native.sa2_detach(_context));
            _context = 0;
        }
        if (_flush.IsValid) { _rd.FreeRid(_flush); _flush = default; }
        if (_native != null && _context == 0) { _native.Dispose(); _native = null; }
        Complete();
    }

    private unsafe void ReadDebug()
    {
        Sa2DebugCounts counts = new() { struct_size = Native.DebugCountsSize };
        Require("sa2_debug_counts", _native.sa2_debug_counts(_context, &counts));
        int[] ids = new int[16]; for (int i = 0; i < ids.Length; i++) ids[i] = counts.Ids[i];
        _result.debug["counts"] = new Dictionary<string, object> {
            ["struct_size"] = counts.struct_size, ["distinct_id_count"] = counts.distinct_id_count, ["corruption"] = counts.corruption,
            ["error"] = counts.error, ["warning"] = counts.warning, ["info"] = counts.info, ["message"] = counts.message,
            ["mismatching_clear_value"] = counts.mismatching_clear_value, ["mentioning_sa2"] = counts.mentioning_sa2, ["ids"] = ids
        };
        byte* messages = stackalloc byte[16384]; messages[0] = 0;
        Require("sa2_debug_messages", _native.sa2_debug_messages(_context, messages, 16384));
        _result.debug["messages"] = Utf8(messages, 16384);
    }

    private void Complete()
    {
        lock (_gate) { _result.exit_code = _exitCode; _result.reason = _reason; }
        try { _result.Write(_args.out_path); }
        catch (Exception e) { Console.Error.WriteLine("sa2 l2 harness: " + e.Message); _exitCode = 1; }
        Post(() => {
            _windows?.Dispose();
            RenderingServer.FramePreDraw -= PreDraw;
            GetWindow().CloseRequested -= CloseRequested;
            _stage = Stage.Finished;
            GetTree().Quit(_exitCode);
        });
    }
}
