using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Magic600.Shell;

/// <summary>
/// The core's context and its surface beneath the shell's window, step S4 of the shell: a draft on route B, accepted
/// for now by the owner on 18 Nov 2026, pending three owner-machine checks. It drives the context's life through
/// <see cref="CoreLibrary.ShellCalls"/>: sa2_create, sa2_surface_attach and sa2_scene_load on open; sa2_surface_move
/// and sa2_resize while it lives; sa2_scene_unload, sa2_drain, sa2_surface_detach and sa2_detach on close. It was never
/// launched, so nothing here is Windows/DirectX, input or performance evidence.
/// </summary>
/// <remarks>
/// Draft choices: Godot's own adapter (its LUID, <see cref="AdapterLuid"/>; C9 U4); the config of the acceptance harness's defaults
/// (Godot's queue mode, matching barriers, COMMON handover states, 5000 ms waits, no debug callback); scene 1 with a
/// 1000 ms planned trace.
/// </remarks>
public sealed unsafe class CoreSurface : IDisposable
{
    // sa2_config, 28 bytes.
    [StructLayout(LayoutKind.Sequential)]
    private struct Config
    {
        public uint StructSize;
        public int QueueMode, BarrierApi, StateBeforeWrite, StateAfterWrite;
        public uint WaitTimeoutMs;
        public int DebugCallback;
    }

    // sa2_scene_config, 32 bytes on x64.
    [StructLayout(LayoutKind.Sequential)]
    private struct SceneConfig
    {
        public uint StructSize, Scene;
        public double TurnMs;
        public byte* Inject;
        public uint Flags, TraceMs;
    }

    public const uint WaitTimeoutMs = 5000;
    public const uint Scene = 1;
    public const uint TraceMs = 1000;
    public const uint DrainTimeoutMs = 5000;

    private readonly CoreLibrary _core;
    private bool _surface;
    private bool _scene;

    private CoreSurface(CoreLibrary core)
    {
        _core = core;
    }

    /// <summary>The core's context, for <see cref="HostLibrary.Create"/>.</summary>
    public IntPtr Context { get; private set; }

    private IntPtr Call(string name)
    {
        for (var i = 0; i < CoreLibrary.ShellCalls.Count; i++)
        {
            if (CoreLibrary.ShellCalls[i] == name)
            {
                return _core.ShellFunctions[i];
            }
        }
        throw new ArgumentException($"{name} is not a shell call", nameof(name));
    }

    private void Check(string name, int status)
    {
        if (status != 0)
        {
            throw new InvalidOperationException($"{name} answered {status}: {LastError()}");
        }
    }

    /// <summary>The core's last error for this context (or for a failed sa2_create).</summary>
    public string LastError()
    {
        var buffer = new byte[1024];
        fixed (byte* b = buffer)
        {
            // sa2_last_error is the second entry of the bridge's table (CoreLibrary.Symbols).
            ((delegate* unmanaged[Cdecl]<IntPtr, byte*, uint, int>)_core.Functions[1])(Context, b, (uint)buffer.Length);
        }
        var end = Array.IndexOf(buffer, (byte)0);
        return Encoding.UTF8.GetString(buffer, 0, end < 0 ? buffer.Length : end);
    }

    /// <summary>
    /// The LUID of the adapter Godot renders on, read from Godot's D3D12 physical device (an IDXGIAdapter) with
    /// IDXGIAdapter::GetDesc, vtable slot 8 (IUnknown 0-2, IDXGIObject 3-6, EnumOutputs 7). DXGI_ADAPTER_DESC is 304
    /// bytes on x64 with AdapterLuid (LowPart, HighPart) at offset 296. sa2_create opens that adapter, so the core
    /// draws on Godot's GPU even where DXGI lists another hardware adapter first, as on a hybrid laptop (C9 U4; the
    /// acceptance harness's Native.AdapterLuid reads it the same way).
    /// </summary>
    public static ulong AdapterLuid(ulong adapter)
    {
        if (adapter == 0)
        {
            throw new InvalidOperationException("Godot's adapter is unavailable");
        }
        var description = stackalloc byte[304];
        var table = *(IntPtr**)adapter;
        var getDesc = (delegate* unmanaged[Stdcall]<IntPtr, byte*, int>)table[8];
        var result = getDesc((IntPtr)adapter, description);
        if (result < 0)
        {
            throw new InvalidOperationException($"IDXGIAdapter::GetDesc answered 0x{result:X8}");
        }
        var luid = ((ulong)*(uint*)(description + 300) << 32) | *(uint*)(description + 296);
        if (luid == 0)
        {
            throw new InvalidOperationException("Godot's adapter has no LUID");
        }
        return luid;
    }

    /// <summary>
    /// Creates a context on the core's own device on the adapter <paramref name="adapterLuid"/> names, attaches its
    /// surface at (x, y) in the window's client pixels and loads the scene at the surface's size. A failure takes down
    /// what was made and throws.
    /// </summary>
    public static CoreSurface Open(CoreLibrary core, ulong adapterLuid, ulong hwnd, int x, int y, uint width, uint height)
    {
        var surface = new CoreSurface(core);
        try
        {
            var config = new Config { StructSize = (uint)sizeof(Config), WaitTimeoutMs = WaitTimeoutMs };
            var context = IntPtr.Zero;
            surface.Check("sa2_create",
                ((delegate* unmanaged[Cdecl]<ulong, Config*, IntPtr*, int>)surface.Call("sa2_create"))(adapterLuid, &config, &context));
            surface.Context = context;
            Live = surface;
            surface.Check("sa2_surface_attach",
                ((delegate* unmanaged[Cdecl]<IntPtr, ulong, int, int, uint, uint, int>)surface.Call("sa2_surface_attach"))(
                    context, hwnd, x, y, width, height));
            surface._surface = true;
            var scene = new SceneConfig { StructSize = (uint)sizeof(SceneConfig), Scene = Scene, TraceMs = TraceMs };
            surface.Check("sa2_scene_load",
                ((delegate* unmanaged[Cdecl]<IntPtr, SceneConfig*, int>)surface.Call("sa2_scene_load"))(context, &scene));
            surface._scene = true;
            return surface;
        }
        catch
        {
            surface.Dispose();
            throw;
        }
    }

    /// <summary>Moves the surface to (x, y) in the window's client pixels; no GPU work.</summary>
    public void Move(int x, int y) =>
        Check("sa2_surface_move", ((delegate* unmanaged[Cdecl]<IntPtr, int, int, int>)Call("sa2_surface_move"))(Context, x, y));

    /// <summary>Resizes the surface after a confirmed drain; the host's viewport must follow.</summary>
    public void Resize(uint width, uint height)
    {
        Check("sa2_drain", Drain());
        Check("sa2_resize", ((delegate* unmanaged[Cdecl]<IntPtr, uint, uint, int>)Call("sa2_resize"))(Context, width, height));
    }

    /// <summary>
    /// The address of the surface's current swap chain (a resize or a recovery makes a new one), or null when the core
    /// lacks <see cref="CoreLibrary.SwapChainCall"/> or does not answer it.
    /// </summary>
    public ulong? SwapChain()
    {
        if (_core.SwapChainFunction == IntPtr.Zero || Context == IntPtr.Zero)
        {
            return null;
        }
        ulong address = 0;
        var status = ((delegate* unmanaged[Cdecl]<IntPtr, ulong*, int>)_core.SwapChainFunction)(Context, &address);
        return status == 0 && address != 0 ? address : null;
    }

    private int Drain() => ((delegate* unmanaged[Cdecl]<IntPtr, uint, int>)Call("sa2_drain"))(Context, DrainTimeoutMs);

    /// <summary>
    /// Unloads the scene, drains, detaches the surface and ends the context. Each step runs once; a failed step is
    /// left to the next process start, because a context the core refuses to end cannot be ended here.
    /// </summary>
    public void Dispose()
    {
        if (Context == IntPtr.Zero)
        {
            return;
        }
        if (_scene)
        {
            ((delegate* unmanaged[Cdecl]<IntPtr, int>)Call("sa2_scene_unload"))(Context);
            _scene = false;
        }
        Drain();
        if (_surface)
        {
            ((delegate* unmanaged[Cdecl]<IntPtr, int>)Call("sa2_surface_detach"))(Context);
            _surface = false;
        }
        ((delegate* unmanaged[Cdecl]<IntPtr, int>)Call("sa2_detach"))(Context);
        Context = IntPtr.Zero;
        if (Live == this)
        {
            Live = null;
        }
    }

    // ADR-D02 B19 and B24 (the gate rows): with M600_D02_GATE=1 (renderer/shell/gate_d02.py sets it) every frame row of
    // the event log carries the core's counters (sa2_counters), so tools/perf/d02/gate_row.py gives a row's
    // view_vertex_evals from the core's own pipeline statistics. Off by default: a normal run's rows are unchanged.
    private static readonly bool GateOn = System.Environment.GetEnvironmentVariable("M600_D02_GATE") == "1";

    /// <summary>The open surface the gate fields read: set by <see cref="Open"/>, cleared by <see cref="Dispose"/>.</summary>
    internal static CoreSurface? Live { get; private set; }

    /// <summary>sa2_counter_values (64 bytes on x64).</summary>
    [StructLayout(LayoutKind.Sequential)]
    private struct CounterValues
    {
        public uint StructSize;
        public uint Reserved;
        public ulong ViewVertexEvalsLast;
        public ulong ViewVertexEvalsTotal;
        public ulong BytesUploaded;
        public ulong FramesSubmitted;
        public ulong FramesReported;
        public ulong IdPasses;
        public ulong ReportsDropped;
    }

    /// <summary>
    /// Writes the gate fields into a frame row: the core's reported frames, its view vertex evaluations (the newest
    /// reported frame's and the total) and its dropped reports. Nothing unless M600_D02_GATE=1 and a surface is open.
    /// </summary>
    internal static void GateFields(System.Text.Json.Utf8JsonWriter json)
    {
        if (!GateOn || Live is not { } surface || surface.Context == IntPtr.Zero)
        {
            return;
        }
        var index = -1;
        for (var i = 0; i < CoreLibrary.Symbols.Count; i++)
        {
            if (CoreLibrary.Symbols[i] == "sa2_counters")
            {
                index = i;
                break;
            }
        }
        if (index < 0)
        {
            return;
        }
        var values = new CounterValues { StructSize = (uint)sizeof(CounterValues) };
        var status = ((delegate* unmanaged[Cdecl]<IntPtr, CounterValues*, int>)surface._core.Functions[index])(surface.Context, &values);
        if (status != 0)
        {
            json.WriteNumber("core_counters_status", status);
            return;
        }
        json.WriteNumber("core_frames_reported", values.FramesReported);
        json.WriteNumber("core_view_vertex_evals_total", values.ViewVertexEvalsTotal);
        json.WriteNumber("core_view_vertex_evals_last", values.ViewVertexEvalsLast);
        json.WriteNumber("core_reports_dropped", values.ReportsDropped);
    }
}
