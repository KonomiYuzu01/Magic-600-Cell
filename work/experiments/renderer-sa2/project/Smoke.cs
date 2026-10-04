using Godot;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;

// Main-thread scene state and render-thread resource state meet only in the
// locked result/mailbox. No lock is held across a Godot or native call: a
// synchronous TextureGetData can run outstanding callbacks reentrantly.
public partial class Smoke : Control
{
    private enum Stage { Init, Building, Wrap, Wrapping, Warmup, Run, Rebuilding, LossObserve, Teardown, Finished }
    private readonly object _gate = new();
    private readonly Queue<Action> _mailbox = new();
    private Arguments _args;
    private RunResult _result;
    private Level2 _level2;
    private bool _abort, _notDrawn, _failurePosted;

    // Main thread only.
    private Stage _stage = Stage.Init;
    private TextureRect _image;
    private readonly List<Texture2Drd> _views = new();
    private long _frame, _preDrawFrame, _transitionStart;
    private int _runFrames, _warmupLeft, _mainWidth, _mainHeight, _sizeIndex, _lossObserveLeft;
    private bool _expectDraw, _runThisIteration, _resizeNext;
    private static readonly Vector2I[] Sizes = { new(1280, 720), new(1024, 640), new(1440, 810), new(800, 600) };

    // Render thread only. Main receives copies of RIDs, never this mutable ring.
    private RenderingDevice _rd;
    private Native _native;
    private nint _context;
    private readonly Rid[] _slots = new Rid[3], _uniforms = new Rid[3];
    private readonly ulong[] _pointers = new ulong[3], _imported = new ulong[3];
    private readonly bool[] _registered = new bool[3];
    private readonly List<Rid> _ringRids = new();
    private Rid _display, _flushTexture, _shader, _pipeline;
    private int _width, _height;
    private uint _generation;
    private bool _deviceRemoved;
    private long _previousFrame;
    private bool _previousRun, _previousWarmup;
    private Rid _previousViewport;
    private uint _previousGeneration;
    private int _previousWidth, _previousHeight;

    public override void _Ready()
    {
        string[] userArgs = OS.GetCmdlineUserArgs();
        if (Level2Arguments.Requested(userArgs))
        {
            _level2 = new Level2();
            AddChild(_level2);
            return;
        }
        try { _args = Arguments.Parse(userArgs); }
        catch (ArgumentException e) { Console.Error.WriteLine("sa2: " + e.Message); GetTree().Quit(2); _stage = Stage.Finished; return; }
        // Measured on the main thread: the safe model renders here, so IsOnRenderThread()
        // is true; with a separate render thread it is false.
        _args.render_thread_separate_observed = !RenderingServer.IsOnRenderThread();
        _result = new RunResult(_args);
        var version = Engine.GetVersionInfo();
        string hash = version["hash"].AsString();
        string normalized = $"{version["major"]}.{version["minor"]}.{version["patch"]}.{version["status"]}.mono.{version["build"]}.{hash[..Math.Min(9, hash.Length)]}";
        _result.engine = new Dictionary<string, object> {
            ["version"] = normalized, ["version_info_string"] = version["string"].AsString(),
            ["editor_build"] = OS.HasFeature("editor"), ["debug_build"] = OS.IsDebugBuild(),
            ["driver"] = RenderingServer.GetCurrentRenderingDriverName(),
            ["rendering_method"] = RenderingServer.GetCurrentRenderingMethod(),
            ["adapter_name"] = RenderingServer.GetVideoAdapterName(),
            ["adapter_api_version"] = RenderingServer.GetVideoAdapterApiVersion()
        };
        _image = GetNode<TextureRect>("Image");
        RenderingServer.FramePreDraw += PreDraw;
        if (normalized != "4.7.2.stable.mono.official.ed1daf0bf") Fail("engine-version");
        if ((string)_result.engine["driver"] != "d3d12") Fail("driver-not-d3d12");
    }

    private void Update(Action<RunResult> action) { lock (_gate) action(_result); }
    private void Post(Action action) { lock (_gate) _mailbox.Enqueue(action); }
    private bool Aborted { get { lock (_gate) return _abort; } }

    private void Fail(string reason)
    {
        lock (_gate)
        {
            _result.Reason(reason);
            if (!_args.Probe) _result.status = "fail";
            _abort = true;
            if (_failurePosted) return;
            _failurePosted = true;
            _mailbox.Enqueue(FinishMain);
        }
    }

    private void PreDraw()
    {
        _preDrawFrame = _frame;
        if (_runThisIteration) Update(r => r.frames.drawn++);
        Update(r => {
            if (r.device_loss.TryGetValue("removed", out object removed) && removed is true)
                r.device_loss["frames_drawn_after"] = Convert.ToInt64(r.device_loss["frames_drawn_after"]) + 1;
        });
    }

    public override void _Process(double delta)
    {
        if (_level2 != null) return;
        if (_stage == Stage.Finished) return;
        _frame++;
        if (_expectDraw && _frame > 1 && _preDrawFrame != _frame - 1)
        {
            lock (_gate) _notDrawn = true;
            Update(r => { r.frames.not_drawn++; r.device_loss["not_drawn"] = true; });
            Fail("not-drawn");
        }
        _runThisIteration = false;
        _expectDraw = false;
        Action[] messages;
        lock (_gate) { messages = _mailbox.ToArray(); _mailbox.Clear(); }
        foreach (Action action in messages) action();
        if (_stage == Stage.Finished || _stage == Stage.Teardown) return;
        try
        {
            Vector2 size = GetViewport().GetVisibleRect().Size;
            int width = (int)size.X, height = (int)size.Y;
            if (_stage == Stage.Init)
            {
                _stage = Stage.Building;
                QueueStep((f, viewport) => Initialize(width, height));
            }
            else if (_stage == Stage.Wrap)
            {
                _stage = Stage.Wrapping;
                Rid[] serverTextures = _views.Select(v => v.GetRid()).ToArray();
                QueueStep((f, viewport) => CheckWrap(f, viewport, serverTextures));
            }
            else if (_stage == Stage.Warmup || _stage == Stage.Run)
            {
                if (_resizeNext)
                {
                    _resizeNext = false;
                    GetWindow().Size = Sizes[_sizeIndex = (_sizeIndex + 1) % Sizes.Length];
                    // Wait until the viewport reports its actual new extent.
                    size = GetViewport().GetVisibleRect().Size;
                    width = (int)size.X; height = (int)size.Y;
                }
                if (width != _mainWidth || height != _mainHeight)
                {
                    ClearViews();
                    _transitionStart = _frame;
                    _stage = Stage.Rebuilding;
                    QueueStep((f, viewport) => {
                        ObserveViewport(f, viewport, false, false);
                        if (!ReleaseRing(false)) { Post(FinishMain); return; }
                        _generation = (_generation + 1) & 4095;
                        BuildRing(width, height);
                    }, allowAbort: true);
                }
                else if (_stage == Stage.Warmup)
                {
                    ShowSlot(_frame);
                    QueueStep(Warmup);
                    if (--_warmupLeft == 0) _stage = Stage.Run;
                }
                else if (_runFrames >= _args.frames) FinishMain();
                else
                {
                    if (_transitionStart != 0)
                    {
                        long start = _transitionStart, end = _frame;
                        Update(r => {
                            r.resize.transitions.Add(new() { ["start_frame"] = start, ["end_frame"] = end, ["frames"] = end - start });
                            r.resize.longest_transition = Math.Max(r.resize.longest_transition, end - start);
                        });
                        _transitionStart = 0;
                    }
                    ShowSlot(_frame);
                    _runThisIteration = true;
                    int runIndex = ++_runFrames;
                    QueueStep((f, viewport) => Run(f, viewport, runIndex), isRun: true);
                    _resizeNext = _args.resize_every > 0 && runIndex % _args.resize_every == 0 && runIndex < _args.frames;
                }
            }
            else if (_stage == Stage.LossObserve)
            {
                if (--_lossObserveLeft <= 0) FinishMain();
                else QueueStep((f, viewport) => RecordRemovedReason(), allowAbort: true);
            }
            _expectDraw = _stage != Stage.Teardown && _stage != Stage.Finished;
        }
        catch (Exception e) { Update(r => r.engine["harness_error"] = e.Message); Fail("godot-call-failed"); FinishMain(); }
    }

    private void ShowSlot(long frame) => _image.Texture = _views[_args.route == "import-copy" ? 0 : (int)(frame % 3)];

    private void ClearViews()
    {
        _image.Texture = null;
        foreach (Texture2Drd view in _views) view.Dispose();
        _views.Clear();
    }

    private void RingBuilt(Rid[] slots, Rid display, int width, int height)
    {
        if (_stage == Stage.Teardown || Aborted) return;
        _mainWidth = width; _mainHeight = height;
        foreach (Rid rid in _args.route == "import-copy" ? new[] { display } : slots)
            _views.Add(new Texture2Drd { TextureRdRid = rid });
        _stage = Stage.Wrap;
    }

    private void FinishMain()
    {
        if (_stage == Stage.Teardown || _stage == Stage.Finished) return;
        _stage = Stage.Teardown;
        ClearViews();
        QueueStep(Teardown, allowAbort: true);
    }

    private void QueueStep(Action<long, Rid> action, bool allowAbort = false, bool isRun = false)
    {
        long frame = _frame;
        Rid viewport = GetViewport().GetTexture().GetRid();
        RenderingServer.CallOnRenderThread(Callable.From(() => {
            if (!allowAbort && Aborted) return;
            try
            {
                if (_context != 0 && !_deviceRemoved && !NotDrawn && (!isRun || _args.route != "rd-compute"))
                {
                    bool signalled = SignalFree(frame, !allowAbort);
                    if (!signalled && !allowAbort) return;
                }
                action(frame, viewport);
            }
            catch (Exception e)
            {
                Update(r => r.engine["harness_error"] = e.Message);
                Fail("godot-call-failed");
                if (allowAbort) { WriteResult(); Post(QuitMain); }
            }
        }));
    }

    private bool NotDrawn { get { lock (_gate) return _notDrawn; } }
    private unsafe bool SignalFree(long frame, bool fatal) =>
        Check("sa2_signal_godot_free", _native.sa2_signal_godot_free(_context, (ulong)Math.Max(0, frame - 1)), fatal);

    private unsafe bool Check(string function, int status, bool fatal = true)
    {
        if (status == Native.SA2_OK) return true;
        byte* buffer = stackalloc byte[2048];
        buffer[0] = 0;
        int errorStatus = _native.sa2_last_error(_context, buffer, 2048);
        int length = 0; while (length < 2047 && buffer[length] != 0) length++;
        string error = Encoding.UTF8.GetString(new ReadOnlySpan<byte>(buffer, length));
        Update(r => {
            r.sa2_failures.Add(new Sa2Failure { function = function, status = status, last_error = error });
            if (errorStatus != Native.SA2_OK)
                r.sa2_failures.Add(new Sa2Failure { function = "sa2_last_error", status = errorStatus, last_error = "could not retrieve last error" });
            r.Reason("sa2-call-failed");
            if (!_args.Probe) r.status = "fail";
        });
        if (status == Native.SA2_E_DEVICE_REMOVED) { _deviceRemoved = true; Update(r => r.Reason("device-removed")); }
        if (fatal) Fail("sa2-call-failed");
        return false;
    }

    private unsafe void Initialize(int width, int height)
    {
        _rd = RenderingServer.GetRenderingDevice();
        if (_rd == null) { Fail("driver-not-d3d12"); return; }
        try { _native = new Native(_args.dll_path); }
        catch (Exception e) { Update(r => r.dll["load_error"] = e.Message); Fail("dll-load-failed"); return; }
        ulong device = _rd.GetDriverResource(RenderingDevice.DriverResource.LogicalDevice, default, 0);
        ulong queue = _rd.GetDriverResource(RenderingDevice.DriverResource.CommandQueue, default, 0);
        ulong adapter = _rd.GetDriverResource(RenderingDevice.DriverResource.PhysicalDevice, default, 0);
        Sa2DeviceInfo probed = new() { struct_size = Native.DeviceInfoSize };
        if (!Check("sa2_probe", _native.sa2_probe(device, queue, adapter, &probed))) return;
        Sa2DeviceInfo info = probed;
        Update(r => r.device = new() {
            ["struct_size"] = info.struct_size, ["queue_type"] = info.queue_type,
            ["queue_device_matches"] = info.queue_device_matches, ["adapter_matches_device"] = info.adapter_matches_device,
            ["enhanced_barriers"] = info.enhanced_barriers, ["debug_layer"] = info.debug_layer,
            ["max_feature_level"] = info.max_feature_level, ["node_count"] = info.node_count,
            ["vendor_id"] = info.vendor_id, ["device_id"] = info.device_id, ["umd_version"] = info.umd_version
        });
        if (info.queue_type != 0 || info.queue_device_matches != 1 || info.adapter_matches_device != 1)
        { Fail("device-ownership"); return; }
        int state = _args.handover == "render-target" ? Native.SA2_STATE_RENDER_TARGET
            : _args.route == "import-copy" ? Native.SA2_STATE_COPY_SOURCE : Native.SA2_STATE_ALL_SHADER_RESOURCE;
        int barrier = _args.barriers == "legacy" ? Native.SA2_BARRIERS_LEGACY
            : _args.barriers == "enhanced" ? Native.SA2_BARRIERS_ENHANCED : Native.SA2_BARRIERS_MATCH_GODOT;
        Sa2Config config = new() {
            struct_size = Native.ConfigSize, queue_mode = _args.queue == "own" ? Native.SA2_QUEUE_OWN : Native.SA2_QUEUE_SAME,
            barrier_api = barrier, state_before_write = state, state_after_write = state,
            wait_timeout_ms = (uint)_args.timeout_ms, debug_callback = 1
        };
        nint context = 0;
        if (!Check("sa2_attach", _native.sa2_attach(device, queue, &config, &context))) return;
        _context = context;
        Update(r => r.dll = new() {
            ["abi_version"] = Native.SA2_ABI_VERSION,
            ["resolved_barrier_api"] = barrier == 0 ? info.enhanced_barriers != 0 ? 2 : 1 : barrier,
            ["state_before_write"] = state, ["state_after_write"] = state,
            ["initial_state"] = _args.route.StartsWith("import-") ? Native.SA2_STATE_RENDER_TARGET : -1
        });
        byte[] flushData = new byte[4 * 4 * 4];
        for (int i = 3; i < flushData.Length; i += 4) flushData[i] = 255;
        _flushTexture = MakeTexture(4, 4, Sampling | CopyFrom, "SA2 flush", flushData, false);
        BuildRing(width, height);
    }

    private const RenderingDevice.TextureUsageBits Sampling = RenderingDevice.TextureUsageBits.SamplingBit;
    private const RenderingDevice.TextureUsageBits ColorAttachment = RenderingDevice.TextureUsageBits.ColorAttachmentBit;
    private const RenderingDevice.TextureUsageBits Storage = RenderingDevice.TextureUsageBits.StorageBit;
    private const RenderingDevice.TextureUsageBits CopyFrom = RenderingDevice.TextureUsageBits.CanCopyFromBit;
    private const RenderingDevice.TextureUsageBits CopyTo = RenderingDevice.TextureUsageBits.CanCopyToBit;

    private Rid MakeTexture(int width, int height, RenderingDevice.TextureUsageBits usage, string name, byte[] data = null, bool ring = true)
    {
        using RDTextureFormat format = new() {
            Width = (uint)width, Height = (uint)height, Depth = 1, ArrayLayers = 1, Mipmaps = 1,
            TextureType = RenderingDevice.TextureType.Type2D, Samples = RenderingDevice.TextureSamples.Samples1,
            Format = RenderingDevice.DataFormat.R8G8B8A8Unorm, UsageBits = usage
        };
        using RDTextureView view = new();
        var initial = new Godot.Collections.Array<byte[]>();
        if (data != null) initial.Add(data);
        Rid rid = _rd.TextureCreate(format, view, initial);
        RequireRid(rid);
        if (ring) _ringRids.Add(rid);
        _rd.SetResourceName(rid, name);
        return rid;
    }

    private static void RequireRid(Rid rid) { if (!rid.IsValid) throw new InvalidOperationException("Godot returned an invalid RID"); }

    private unsafe void BuildRing(int width, int height)
    {
        if (width < 128 || height < 128) { Fail("viewport-too-small"); return; }
        _width = width; _height = height;
        Update(r => { if (r.resize.sizes.Count != 0) r.resize.rebuilds++; r.resize.sizes.Add(new[] { width, height }); });
        if (_args.route == "rd-compute") BuildShader();
        for (uint k = 0; k < 3; k++)
        {
            if (_args.route.StartsWith("import-"))
            {
                ulong pointer = 0;
                if (!Check("sa2_create_texture", _native.sa2_create_texture(_context, (uint)width, (uint)height, Native.SA2_STATE_RENDER_TARGET, &pointer))) return;
                _imported[k] = _pointers[k] = pointer;
                _slots[k] = _rd.TextureCreateFromExtension(RenderingDevice.TextureType.Type2D,
                    RenderingDevice.DataFormat.R8G8B8A8Unorm, RenderingDevice.TextureSamples.Samples1,
                    Sampling | ColorAttachment | CopyFrom, pointer, (ulong)width, (ulong)height, 1, 1, 1);
                RequireRid(_slots[k]); _ringRids.Add(_slots[k]);
                _rd.SetResourceName(_slots[k], $"SA2 imported slot {k}");
            }
            else
            {
                _slots[k] = MakeTexture(width, height, Sampling | CopyFrom | (_args.route == "rd-compute" ? Storage : ColorAttachment), $"SA2 {_args.route} slot {k}");
                if (_args.route == "export") _pointers[k] = _rd.GetDriverResource(RenderingDevice.DriverResource.Texture, _slots[k], 0);
            }
            if (_args.route != "rd-compute")
            {
                if (!Check("sa2_register_slot", _native.sa2_register_slot(_context, k, _pointers[k], (uint)width, (uint)height, _args.route == "export" ? 1 : 0))) return;
                _registered[k] = true;
            }
            else
            {
                using RDUniform uniform = new() { UniformType = RenderingDevice.UniformType.Image, Binding = 0 };
                uniform.AddId(_slots[k]);
                _uniforms[k] = _rd.UniformSetCreate(new Godot.Collections.Array<RDUniform> { uniform }, _shader, 0);
                RequireRid(_uniforms[k]); _ringRids.Add(_uniforms[k]);
            }
        }
        if (_args.route == "import-copy") _display = MakeTexture(width, height, Sampling | CopyTo | CopyFrom, "SA2 display");
        Rid[] slots = (Rid[])_slots.Clone(); Rid display = _display;
        Post(() => RingBuilt(slots, display, width, height));
    }

    private const string ComputeShader = """
        #version 450
        layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;
        layout(rgba8, set = 0, binding = 0) uniform writeonly image2D target;
        layout(push_constant, std430) uniform Params {
            uint lo; uint hi; uint fill; uint width; uint height;
            uint pad0; uint pad1; uint pad2;
        } p;
        void main() {
            uvec2 xy = gl_GlobalInvocationID.xy;
            if (xy.x >= p.width || xy.y >= p.height) return;
            ivec2 corner = ivec2(-1);
            if (xy.x < 64u && xy.y < 64u) corner = ivec2(xy);
            else if (xy.x >= p.width - 64u && xy.y >= p.height - 64u)
                corner = ivec2(xy - uvec2(p.width - 64u, p.height - 64u));
            uint rgba = p.fill;
            if (corner.x >= 0) {
                uint bit = uint(corner.y / 8 * 8 + corner.x / 8);
                uint word = bit < 32u ? p.lo : p.hi;
                rgba = ((word >> (bit % 32u)) & 1u) != 0u ? 0xffffffffu : 0xff000000u;
            }
            imageStore(target, ivec2(xy), vec4(float(rgba & 255u), float((rgba >> 8) & 255u),
                float((rgba >> 16) & 255u), float(rgba >> 24)) / 255.0);
        }
        """;

    private void BuildShader()
    {
        using RDShaderSource source = new() { SourceCompute = ComputeShader };
        using RDShaderSpirV spirv = _rd.ShaderCompileSpirVFromSource(source, false);
        if (!string.IsNullOrEmpty(spirv.GetStageCompileError(RenderingDevice.ShaderStage.Compute)))
            throw new InvalidOperationException("compute shader compile failed");
        _shader = _rd.ShaderCreateFromSpirV(spirv, "SA2 code compute");
        RequireRid(_shader); _ringRids.Add(_shader);
        _pipeline = _rd.ComputePipelineCreate(_shader);
        RequireRid(_pipeline); _ringRids.Add(_pipeline);
    }

    private void CheckWrap(long frame, Rid viewport, Rid[] serverTextures)
    {
        if (_args.route == "export")
            for (int k = 0; k < 3; k++)
                if (_rd.GetDriverResource(RenderingDevice.DriverResource.Texture, _slots[k], 0) != _pointers[k])
                { Fail("resource-changed"); return; }
        if (_args.route == "import-texture2drd")
        {
            bool accepted = serverTextures.All(rid => RenderingServer.TextureGetRdTexture(rid).IsValid);
            Update(r => r.texture2drd_probe = new() { ["checked"] = true, ["accepted"] = accepted });
            if (!accepted)
            {
                Update(r => { r.status = "unsupported"; r.Reason("texture2drd-refused"); });
                lock (_gate) _abort = true;
                Post(FinishMain);
                return;
            }
        }
        ObserveViewport(frame, viewport, false, false);
        Post(() => {
            if (_stage != Stage.Wrapping || Aborted) return;
            _warmupLeft = _args.warmup; _stage = _warmupLeft == 0 ? Stage.Run : Stage.Warmup;
        });
    }

    private unsafe void Warmup(long frame, Rid viewport)
    {
        uint slot = (uint)(frame % 3);
        if (!Check("sa2_mark_shown", _native.sa2_mark_shown(_context, slot, (ulong)frame))) return;
        if (_args.route == "import-copy") CopyDisplay(slot);
        ObserveViewport(frame, viewport, false, true);
    }

    private void CopyDisplay(uint slot)
    {
        if (_rd.TextureCopy(_slots[slot], _display, Vector3.Zero, Vector3.Zero, new Vector3(_width, _height, 1), 0, 0, 0, 0) != Error.Ok)
            throw new InvalidOperationException("texture_copy failed");
    }

    private unsafe void Run(long frame, Rid viewport, int runIndex)
    {
        uint slot = (uint)(frame % 3), sequence = unchecked((uint)frame);
        Update(r => r.frames.run++);
        if (_args.route == "rd-compute")
        {
            long list = _rd.ComputeListBegin();
            _rd.ComputeListBindComputePipeline(list, _pipeline);
            _rd.ComputeListBindUniformSet(list, _uniforms[slot], 0);
            _rd.ComputeListSetPushConstant(list, CodeLayout.PushConstants(sequence, slot, _generation, _width, _height), 32);
            _rd.ComputeListDispatch(list, (uint)(_width + 7) / 8, (uint)(_height + 7) / 8, 1);
            _rd.ComputeListEnd();
        }
        else
        {
            if (!Check("sa2_produce", _native.sa2_produce(_context, slot, (ulong)frame, sequence, _generation))) return;
            if (!Check("sa2_godot_wait_ready", _native.sa2_godot_wait_ready(_context, (ulong)frame))) return;
            if (!Check("sa2_mark_shown", _native.sa2_mark_shown(_context, slot, (ulong)frame))) return;
            if (_args.route == "import-copy") CopyDisplay(slot);
        }
        Update(r => r.frames.produced++);
        if (runIndex % _args.verify_every == 0) Verify(frame, slot, sequence);
        if (NotDrawn) return;
        ObserveViewport(frame, viewport, true, false);
        if (runIndex == _args.device_loss_at)
        {
            Update(r => r.device_loss = new() { ["requested"] = true, ["run_frame"] = runIndex,
                ["iteration"] = frame, ["removed"] = false, ["frames_drawn_after"] = 0L });
            WriteResult(); // An engine abort may leave this pre-removal snapshot.
            int removal = _native.sa2_remove_device(_context);
            Check("sa2_remove_device", removal, false);
            Update(r => { r.status = "recorded"; r.device_loss["remove_status"] = removal;
                r.device_loss["removed"] = removal == Native.SA2_OK; });
            RecordRemovedReason();
            lock (_gate) _abort = true;
            Post(() => {
                if (_stage == Stage.Teardown || _stage == Stage.Finished) return;
                _stage = Stage.LossObserve; _lossObserveLeft = 2;
            });
        }
    }

    private unsafe void Verify(long frame, uint slot, uint sequence)
    {
        if (_args.route != "rd-compute")
        {
            ulong nativeBad = 0;
            int status = _native.sa2_verify_slot(_context, slot, sequence, _generation, &nativeBad);
            ulong mismatches = nativeBad;
            Update(r => { r.verify.native_checks++; r.verify.native_mismatched_texels += mismatches; r.verify.mismatched_texels += mismatches; });
            Check("sa2_verify_slot", status, status != Native.SA2_E_VERIFY);
            if (mismatches != 0) Update(r => r.Mismatch(frame, CodeLayout.Encode(sequence, slot, _generation), null, "verify-mismatch"));
        }
        Rid shown = _args.route == "import-copy" ? _display : _slots[slot];
        byte[] data = _rd.TextureGetData(shown, 0);
        ulong bad = CodeLayout.MismatchedTexels(data, _width, _height, sequence, slot, _generation);
        Update(r => {
            r.verify.rd_checks++; r.verify.rd_mismatched_texels += bad; r.verify.mismatched_texels += bad;
            if (bad != 0) r.Mismatch(frame, CodeLayout.Encode(sequence, slot, _generation), CodeLayout.Decode(data, _width, _height, _width, _height), "verify-mismatch");
        });
    }

    private void ObserveViewport(long frame, Rid serverViewport, bool currentRun, bool currentWarmup)
    {
        if (NotDrawn || _rd == null || _deviceRemoved) return;
        Rid viewport = RenderingServer.TextureGetRdTexture(serverViewport);
        int viewportWidth = 0, viewportHeight = 0;
        if (viewport.IsValid)
        {
            using RDTextureFormat format = _rd.TextureGetFormat(viewport);
            viewportWidth = (int)format.Width; viewportHeight = (int)format.Height;
        }
        bool eligible = _previousRun && _previousFrame == frame - 1 && _previousGeneration == _generation
            && viewport.IsValid && viewport == _previousViewport
            && viewportWidth >= _previousWidth && viewportHeight >= _previousHeight;
        if (eligible)
        {
            long capturedFrame = _previousFrame;
            int ringWidth = _previousWidth, ringHeight = _previousHeight;
            ulong expected = CodeLayout.Encode(unchecked((uint)capturedFrame), (uint)(capturedFrame % 3), _previousGeneration);
            Update(r => { r.frames.eligible++; r.frames.readbacks_requested++; });
            Error status = _rd.TextureGetDataAsync(viewport, 0, Callable.From<byte[]>(data => {
                ulong? decoded = CodeLayout.Decode(data, viewportWidth, viewportHeight, ringWidth, ringHeight);
                Update(r => {
                    r.frames.readbacks_completed++;
                    if (decoded == expected) r.frames.verified++;
                    else r.Mismatch(capturedFrame, expected, decoded, "decode-mismatch");
                });
            }));
            if (status != Error.Ok) Fail("readback-missing");
        }
        else Update(r => r.frames.skipped[_previousWarmup ? "warmup" : "transition"]++);
        _previousFrame = frame; _previousRun = currentRun; _previousWarmup = currentWarmup;
        _previousViewport = viewport; _previousGeneration = _generation;
        _previousWidth = _width; _previousHeight = _height;
    }

    private unsafe void RecordRemovedReason()
    {
        int nativeReason = 0;
        int status = _native.sa2_device_removed_reason(_context, &nativeReason);
        int reason = nativeReason;
        Check("sa2_device_removed_reason", status, false);
        if (reason < 0) { _deviceRemoved = true; Update(r => r.Reason("device-removed")); }
        Update(r => { r.device_loss["reason_status"] = status; r.device_loss["hresult"] = reason; });
    }

    private bool Flush()
    {
        if (!_flushTexture.IsValid || _rd == null) return true;
        try
        {
            byte[] data = _rd.TextureGetData(_flushTexture, 0);
            if (data.Length == 64) return true;
        }
        catch (Exception e) { Update(r => r.engine["flush_error"] = e.Message); }
        Update(r => { r.Reason("godot-call-failed"); if (!_args.Probe) r.status = "fail"; });
        return false;
    }

    private void FreeRingRids()
    {
        for (int i = _ringRids.Count - 1; i >= 0; i--) _rd.FreeRid(_ringRids[i]);
        _ringRids.Clear();
        Array.Clear(_slots); Array.Clear(_uniforms);
        _display = _shader = _pipeline = default;
    }

    private unsafe bool ReleaseRing(bool final)
    {
        bool firstFlush = Flush();
        FreeRingRids();
        bool disposed = Flush() && firstFlush;
        if (final && _flushTexture.IsValid) { _rd.FreeRid(_flushTexture); _flushTexture = default; }
        if (final) { Update(r => r.teardown.phase = "pending"); WriteResult(); }
        if (_context == 0) return disposed;
        int drain = _native.sa2_drain(_context, (uint)_args.timeout_ms);
        Check("sa2_drain", drain, false);
        Update(r => { if (final) r.teardown.drain_result = drain; else r.teardown.rebuild_drain_results.Add(drain); });
        bool safe = drain == Native.SA2_OK || drain == Native.SA2_E_DEVICE_REMOVED;
        if (!safe || (!disposed && drain != Native.SA2_E_DEVICE_REMOVED)) return false;
        bool released = true;
        for (uint k = 0; k < 3; k++)
        {
            if (_registered[k])
            {
                bool ok = Check("sa2_unregister_slot", _native.sa2_unregister_slot(_context, k), false);
                released &= ok;
                if (ok) { _registered[k] = false; Update(r => r.teardown.slots_unregistered++); }
            }
            if (_imported[k] != 0 && !_registered[k])
            {
                uint nativeCount = 0;
                bool ok = Check("sa2_release_texture", _native.sa2_release_texture(_context, _imported[k], &nativeCount), false);
                uint count = nativeCount;
                released &= ok;
                uint generation = _generation, slot = k;
                Update(r => {
                    r.teardown.refcount_after.Add(new() { ["generation"] = generation, ["slot"] = slot,
                        ["status"] = ok ? Native.SA2_OK : -1, ["refcount_after"] = count });
                    if (ok && count != 0) { r.Reason("refcount-nonzero"); if (!_args.Probe) r.status = "fail"; }
                });
                if (ok) _imported[k] = 0;
            }
            if (!_registered[k] && _imported[k] == 0) _pointers[k] = 0;
        }
        return released;
    }

    private unsafe void ReadDebug()
    {
        Sa2DebugCounts nativeCounts = new() { struct_size = Native.DebugCountsSize };
        Check("sa2_debug_counts", _native.sa2_debug_counts(_context, &nativeCounts), false);
        int[] ids = new int[Math.Min(nativeCounts.distinct_id_count, 16)];
        for (int i = 0; i < ids.Length; i++) ids[i] = nativeCounts.Ids[i];
        Sa2DebugCounts counts = nativeCounts;
        byte* buffer = stackalloc byte[16384]; buffer[0] = 0;
        Check("sa2_debug_messages", _native.sa2_debug_messages(_context, buffer, 16384), false);
        int length = 0; while (length < 16383 && buffer[length] != 0) length++;
        string messages = Encoding.UTF8.GetString(new ReadOnlySpan<byte>(buffer, length));
        Update(r => {
            r.debug = new() {
                ["struct_size"] = counts.struct_size, ["distinct_id_count"] = counts.distinct_id_count,
                ["corruption"] = counts.corruption, ["error"] = counts.error, ["warning"] = counts.warning,
                ["info"] = counts.info, ["message"] = counts.message,
                ["mismatching_clear_value"] = counts.mismatching_clear_value, ["mentioning_sa2"] = counts.mentioning_sa2,
                ["ids"] = ids, ["messages"] = messages
            };
            if (_args.gpu_validation && (counts.error != 0 || counts.corruption != 0))
            { r.Reason("debug-errors"); if (!_args.Probe) r.status = "fail"; }
        });
    }

    private unsafe void Teardown(long frame, Rid viewport)
    {
        if (!NotDrawn && !_deviceRemoved) ObserveViewport(frame, viewport, false, false);
        bool released = ReleaseRing(true);
        if (_context != 0)
        {
            ReadDebug();
            if (released)
            {
                int status = _native.sa2_detach(_context);
                Check("sa2_detach", status, false);
                if (status == Native.SA2_OK) { _context = 0; Update(r => r.teardown.detached = true); }
            }
        }
        if (_native != null && _context == 0) { _native.Dispose(); _native = null; }
        Update(r => {
            r.teardown.phase = released && r.teardown.detached ? "complete" : "incomplete";
            if (r.teardown.phase != "complete") { r.Reason("teardown-incomplete"); if (!_args.Probe) r.status = "fail"; }
            if (r.frames.readbacks_requested != r.frames.readbacks_completed) { r.Reason("readback-missing"); if (!_args.Probe) r.status = "fail"; }
            if (!_args.Probe && (r.frames.verified != r.frames.eligible || r.frames.eligible * 10 < r.frames.run * 9
                || r.frames.run != _args.frames || r.frames.run == 0)) { r.Reason("readback-coverage"); r.status = "fail"; }
        });
        WriteResult();
        Post(QuitMain);
    }

    private void WriteResult()
    {
        string json;
        lock (_gate) json = _result.Json();
        RunResult.WriteAtomic(_args.out_path, json);
    }

    private void QuitMain()
    {
        string status;
        lock (_gate) status = _result.status;
        _stage = Stage.Finished;
        RenderingServer.FramePreDraw -= PreDraw;
        GetTree().Quit(status == "fail" ? 1 : 0);
    }
}
