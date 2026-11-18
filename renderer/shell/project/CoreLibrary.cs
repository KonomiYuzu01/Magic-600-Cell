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
    public const uint AbiVersion = 4;

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
    };

    private IntPtr _handle;

    private CoreLibrary(IntPtr handle, IntPtr[] functions)
    {
        _handle = handle;
        Functions = functions;
    }

    /// <summary>The export addresses in <see cref="Symbols"/> order.</summary>
    public IReadOnlyList<IntPtr> Functions { get; }

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
            var functions = new IntPtr[Symbols.Count];
            for (var i = 0; i < Symbols.Count; i++)
            {
                if (!NativeLibrary.TryGetExport(handle, Symbols[i], out functions[i]))
                {
                    throw new EntryPointNotFoundException($"the core does not export {Symbols[i]}");
                }
            }
            var version = ((delegate* unmanaged[Cdecl]<uint>)functions[0])();
            if (version != AbiVersion)
            {
                throw new NotSupportedException($"the core speaks ABI {version}, the shell ABI {AbiVersion}");
            }
            return new CoreLibrary(handle, functions);
        }
        catch
        {
            NativeLibrary.Free(handle);
            throw;
        }
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
