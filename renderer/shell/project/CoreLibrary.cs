using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace Magic600.Shell;

/// <summary>
/// The S-A2 core library (sa2_interop.dll) as the shell loads it. The shell is the only place that loads it: the
/// bridge crates load no library (CT-RPI-20), so the shell resolves the exports named in <see cref="Symbols"/> and
/// hands their addresses, in that order, to the bridge's function table (c600-render-ffi's CoreFunctions).
/// </summary>
public sealed class CoreLibrary : IDisposable
{
    /// <summary>The ABI this shell speaks; the core's sa2_abi_version must answer it.</summary>
    public const uint AbiVersion = 10;

    /// <summary>
    /// The exports of c600-render-ffi's SYMBOLS, in its order. tests/test_renderer_shell.py checks the two lists
    /// against each other.
    /// </summary>
    public static readonly IReadOnlyList<string> Symbols = new[]
    {
        "sa2_abi_version",
        "sa2_last_error",
        "sa2_describe",
        "sa2_engine_adopt",
        "sa2_prepare",
        "sa2_poll",
        "sa2_activate",
        "sa2_cancel",
        "sa2_retire",
        "sa2_frame_submit",
        "sa2_frame_reports",
        "sa2_presented",
        "sa2_pick_request",
        "sa2_pick_poll",
        "sa2_recover",
        "sa2_counters",
        "sa2_device_status",
        "sa2_view_shape_set",
        "sa2_style_set",
        "sa2_view_style_set",
        "sa2_frame_submit_panes",
        "sa2_frame_panes",
        "sa2_view_focus_set",
        "sa2_view_layout_set",
    };

    /// <summary>
    /// The calls the shell makes itself on route B, accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks: it creates the context on the
    /// core's own device, loads the scene, attaches, moves and resizes the surface beneath its window, drains, and
    /// takes everything down again. They are not in the bridge's table. tests/test_renderer_shell.py checks that the
    /// header declares each one.
    /// </summary>
    public static readonly IReadOnlyList<string> ShellCalls = new[]
    {
        "sa2_create",
        "sa2_scene_load",
        "sa2_surface_attach",
        "sa2_surface_move",
        "sa2_resize",
        "sa2_drain",
        "sa2_surface_detach",
        "sa2_scene_unload",
        "sa2_detach",
    };

    /// <summary>
    /// C9's diagnostic call, added to ABI 5 without a version change: the surface's swap-chain address, which the event
    /// log records for the PresentMon cross-check. It is optional, since an ABI 5 core built before it lacks it.
    /// </summary>
    public const string SwapChainCall = "sa2_surface_swap_chain";

    private IntPtr _handle;

    private CoreLibrary(IntPtr handle, IntPtr[] functions, IntPtr[] shellFunctions, IntPtr swapChain)
    {
        _handle = handle;
        Functions = functions;
        ShellFunctions = shellFunctions;
        SwapChainFunction = swapChain;
    }

    /// <summary>The export addresses in <see cref="Symbols"/> order.</summary>
    public IReadOnlyList<IntPtr> Functions { get; }

    /// <summary>The export addresses in <see cref="ShellCalls"/> order.</summary>
    public IReadOnlyList<IntPtr> ShellFunctions { get; }

    /// <summary>The address of <see cref="SwapChainCall"/>, or zero when the core lacks it.</summary>
    public IntPtr SwapChainFunction { get; }

    /// <summary>
    /// Loads the core from an absolute path, resolves every export and checks the ABI version. A missing export or
    /// another ABI version unloads the library and throws, so a shell never runs over a partial table.
    /// </summary>
    public static unsafe CoreLibrary Load(string path)
    {
        if (!System.IO.Path.IsPathFullyQualified(path))
        {
            throw new ArgumentException("the core library needs an absolute path", nameof(path));
        }
        var handle = NativeLibrary.Load(path);
        try
        {
            var functions = Resolve(handle, Symbols);
            var version = ((delegate* unmanaged[Cdecl]<uint>)functions[0])();
            if (version != AbiVersion)
            {
                throw new NotSupportedException($"the core speaks ABI {version}, the shell ABI {AbiVersion}");
            }
            NativeLibrary.TryGetExport(handle, SwapChainCall, out var swapChain);
            return new CoreLibrary(handle, functions, Resolve(handle, ShellCalls), swapChain);
        }
        catch
        {
            NativeLibrary.Free(handle);
            throw;
        }
    }

    private static IntPtr[] Resolve(IntPtr handle, IReadOnlyList<string> names)
    {
        var functions = new IntPtr[names.Count];
        for (var i = 0; i < names.Count; i++)
        {
            if (!NativeLibrary.TryGetExport(handle, names[i], out functions[i]))
            {
                throw new EntryPointNotFoundException($"the core does not export {names[i]}");
            }
        }
        return functions;
    }

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            NativeLibrary.Free(_handle);
            _handle = IntPtr.Zero;
        }
    }
}
