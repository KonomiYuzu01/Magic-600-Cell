using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace Magic600.Shell;

/// <summary>
/// The shell's native host library (c600_shell_host.dll, built from renderer/bridge/host), step S3 of the shell: a
/// draft on route B, accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks. The library
/// runs the display host over the bridge. The shell hands it the core's function table
/// (<see cref="CoreLibrary.Functions"/>) and the context it created with sa2_create, then drives frames through it.
/// tests/test_renderer_shell.py checks <see cref="Exports"/>, the version, the status codes and the frame and presented
/// rows against the Rust source.
/// </summary>
public sealed unsafe class HostLibrary : IDisposable
{
    /// <summary>The C interface this shell speaks; the library's m6h_abi_version must answer it.</summary>
    public const uint AbiVersion = 3;

    /// <summary>The name of the correctness control that <see cref="CloseSession"/> runs (the run row's).</summary>
    public const string CorrectnessControl = "journal-reopen-and-kernel-replay";

    public const int Ok = 0;
    public const int InvalidArgument = 1;
    public const int ModelNotLoaded = 2;
    public const int CoreRefused = 3;
    public const int HostRefused = 4;
    /// <summary>The renderer is unavailable, for example after the device was lost; <see cref="Recover"/> may help.</summary>
    public const int Unavailable = 5;
    /// <summary>An earlier call panicked; only <see cref="LastError"/> and <see cref="Destroy"/> are left.</summary>
    public const int Poisoned = 6;
    /// <summary>The session did not open, is already open, or is not open.</summary>
    public const int SessionRefused = 7;
    /// <summary>The language runtime did not start, or the host has started one already.</summary>
    public const int RuntimeRefused = 8;

    /// <summary><see cref="KeyResult.Press"/>: no measured press (no twist key, or a repeat).</summary>
    public const uint PressNone = 0;
    /// <summary><see cref="KeyResult.Press"/>: a twist key committed twist.apply.</summary>
    public const uint PressCommitted = 1;
    /// <summary><see cref="KeyResult.Press"/>: a twist key that the router or the layer refused.</summary>
    public const uint PressRefused = 2;
    /// <summary><see cref="KeyResult.Press"/>: no measured press, but an undo or redo changed the state.</summary>
    public const uint PressOther = 3;

    /// <summary>The library's exports, in the order the Rust source declares them.</summary>
    public static readonly IReadOnlyList<string> Exports = new[]
    {
        "m6h_abi_version",
        "m6h_create",
        "m6h_set_camera",
        "m6h_set_viewport",
        "m6h_pump",
        "m6h_frame",
        "m6h_read_presented",
        "m6h_recover",
        "m6h_open_session",
        "m6h_load_runtime",
        "m6h_key_down",
        "m6h_key_up",
        "m6h_focus_lost",
        "m6h_close_session",
        "m6h_check",
        "m6h_last_error",
        "m6h_destroy",
    };

    /// <summary>A submitted frame, as m6h_frame writes it (the Rust M6hFrame).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct Submitted
    {
        public ulong Sequence;
        public ulong DeviceEpoch;
        public ulong Surface;
        /// <summary>The engine revision whose labels the frame draws.</summary>
        public ulong EngineRevision;
        /// <summary>The engine revision of the publication the frame draws, or 0 for none.</summary>
        public ulong PublicationRevision;
    }

    /// <summary>What m6h_key_down did with one key-down (the Rust M6hKey).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct KeyResult
    {
        public uint Press;
        public uint Reserved;
        /// <summary>The adopted engine revision after the key; for a committed press, the one that shows it.</summary>
        public ulong EngineRevision;
    }

    /// <summary>One confirmed presentation, as m6h_read_presented writes it (the Rust M6hPresented).</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct Presented
    {
        public ulong Sequence;
        public ulong DeviceEpoch;
        public ulong Surface;
        public ulong EngineRevision;
        public ulong Camera;
        /// <summary>The SyncQPCTime of the statistics row that confirmed the frame, in QPC ticks.</summary>
        public ulong DisplayQpc;
    }

    private IntPtr _library;
    private readonly Dictionary<string, IntPtr> _exports;

    private HostLibrary(IntPtr library, Dictionary<string, IntPtr> exports)
    {
        _library = library;
        _exports = exports;
    }

    /// <summary>
    /// Loads the library from an absolute path, resolves every export and checks the version. A missing export or
    /// another version unloads the library and throws.
    /// </summary>
    public static HostLibrary Load(string path)
    {
        if (!System.IO.Path.IsPathFullyQualified(path))
        {
            throw new ArgumentException("the host library needs an absolute path", nameof(path));
        }
        var library = NativeLibrary.Load(path);
        try
        {
            var exports = new Dictionary<string, IntPtr>();
            foreach (var name in Exports)
            {
                if (!NativeLibrary.TryGetExport(library, name, out var address))
                {
                    throw new EntryPointNotFoundException($"the host library does not export {name}");
                }
                exports[name] = address;
            }
            var version = ((delegate* unmanaged[Cdecl]<uint>)exports["m6h_abi_version"])();
            if (version != AbiVersion)
            {
                throw new NotSupportedException($"the host library speaks version {version}, the shell {AbiVersion}");
            }
            return new HostLibrary(library, exports);
        }
        catch
        {
            NativeLibrary.Free(library);
            throw;
        }
    }

    /// <summary>
    /// Creates a host over the core's table and a context with a surface attached; it loads the shipped model from
    /// <paramref name="assets"/>. On failure <paramref name="host"/> is zero and <see cref="LastError"/> with a zero
    /// handle gives the reason.
    /// </summary>
    public int Create(CoreLibrary core, IntPtr context, string assets, out IntPtr host)
    {
        var table = new IntPtr[core.Functions.Count];
        for (var i = 0; i < table.Length; i++)
        {
            table[i] = core.Functions[i];
        }
        var path = Encoding.UTF8.GetBytes(assets);
        var created = IntPtr.Zero;
        int code;
        fixed (IntPtr* functions = table)
        fixed (byte* bytes = path)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr*, IntPtr, byte*, nuint, IntPtr*, int>)_exports["m6h_create"])(
                functions, context, bytes, (nuint)path.Length, &created);
        }
        host = created;
        return code;
    }

    /// <summary>Sets the view's 4D rotation: 16 floats, column-major.</summary>
    public int SetCamera(IntPtr host, ReadOnlySpan<float> matrix)
    {
        if (matrix.Length != 16)
        {
            throw new ArgumentException("a camera is 16 floats", nameof(matrix));
        }
        fixed (float* m = matrix)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, float*, int>)_exports["m6h_set_camera"])(host, m);
        }
    }

    public int SetViewport(IntPtr host, uint width, uint height) =>
        ((delegate* unmanaged[Cdecl]<IntPtr, uint, uint, int>)_exports["m6h_set_viewport"])(host, width, height);

    public int Pump(IntPtr host) => ((delegate* unmanaged[Cdecl]<IntPtr, int>)_exports["m6h_pump"])(host);

    /// <summary>Submits a frame of the view to the surface; <paramref name="frame"/> names it.</summary>
    public int Frame(IntPtr host, out Submitted frame)
    {
        Submitted f = default;
        var code = ((delegate* unmanaged[Cdecl]<IntPtr, Submitted*, int>)_exports["m6h_frame"])(host, &f);
        frame = f;
        return code;
    }

    /// <summary>Reads up to rows.Length confirmed presentations; read again while the count fills the span.</summary>
    public int ReadPresented(IntPtr host, Span<Presented> rows, out int count)
    {
        uint n = 0;
        int code;
        fixed (Presented* r = rows)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr, Presented*, uint, uint*, int>)_exports["m6h_read_presented"])(
                host, r, (uint)rows.Length, &n);
        }
        count = (int)n;
        return code;
    }

    public int Recover(IntPtr host) => ((delegate* unmanaged[Cdecl]<IntPtr, int>)_exports["m6h_recover"])(host);

    /// <summary>
    /// Opens the turns' session with the twist menu of the record file <paramref name="menu"/> pinned. Without a
    /// <paramref name="fixture"/> it is a new journal in <paramref name="folder"/> (created if missing; it must hold
    /// nothing) at the solved state. With one (the path of a session fixture's fixture.json), <paramref name="folder"/>
    /// is a fresh copy of the fixture's session, which the host reopens and adopts at the revision that counts the
    /// fixture's presses. <paramref name="revision"/> is the engine revision the session opened at.
    /// </summary>
    public int OpenSession(IntPtr host, string folder, string menu, string? fixture, out ulong revision)
    {
        var f = Encoding.UTF8.GetBytes(folder);
        var m = Encoding.UTF8.GetBytes(menu);
        var x = fixture is null ? null : Encoding.UTF8.GetBytes(fixture);
        ulong opened = 0;
        int code;
        fixed (byte* fb = f)
        fixed (byte* mb = m)
        fixed (byte* xb = x)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, byte*, nuint, byte*, nuint, ulong*, int>)
                _exports["m6h_open_session"])(host, fb, (nuint)f.Length, mb, (nuint)m.Length, xb,
                (nuint)(x?.Length ?? 0), &opened);
        }
        revision = opened;
        return code;
    }

    /// <summary>
    /// Starts the language runtime's process from <paramref name="program"/> (the c600-runtime binary) and waits for
    /// its hello; the host keeps it until <see cref="Destroy"/>.
    /// </summary>
    public int LoadRuntime(IntPtr host, string program)
    {
        var p = Encoding.UTF8.GetBytes(program);
        fixed (byte* pb = p)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, int>)_exports["m6h_load_runtime"])(
                host, pb, (nuint)p.Length);
        }
    }

    /// <summary>Routes one key-down, <paramref name="chord"/> in c600-input's form (Shift+KeyJ).</summary>
    public int KeyDown(IntPtr host, string chord, bool repeat, out KeyResult key)
    {
        var c = Encoding.UTF8.GetBytes(chord);
        KeyResult k = default;
        int code;
        fixed (byte* cb = c)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, uint, KeyResult*, int>)_exports["m6h_key_down"])(
                host, cb, (nuint)c.Length, repeat ? 1u : 0u, &k);
        }
        key = k;
        return code;
    }

    /// <summary>Routes a key-up of <paramref name="chord"/>; its modifiers are ignored.</summary>
    public int KeyUp(IntPtr host, string chord)
    {
        var c = Encoding.UTF8.GetBytes(chord);
        fixed (byte* cb = c)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, int>)_exports["m6h_key_up"])(
                host, cb, (nuint)c.Length);
        }
    }

    /// <summary>The window lost the keyboard focus: the router releases its held keys.</summary>
    public int FocusLost(IntPtr host) => ((delegate* unmanaged[Cdecl]<IntPtr, int>)_exports["m6h_focus_lost"])(host);

    /// <summary>
    /// Closes the session and runs its correctness control once. <paramref name="shownRevision"/> is the engine
    /// revision of the newest presented frame (0 for none); <see cref="Check"/> gives the control's record.
    /// </summary>
    public int CloseSession(IntPtr host, ulong shownRevision, out bool pass)
    {
        uint p = 0;
        var code = ((delegate* unmanaged[Cdecl]<IntPtr, ulong, uint*, int>)_exports["m6h_close_session"])(
            host, shownRevision, &p);
        pass = code == Ok && p == 1;
        return code;
    }

    /// <summary>The correctness control's record as canonical JSON, empty before the close.</summary>
    public string Check(IntPtr host) => Text(host, "m6h_check");

    /// <summary>The reason for the handle's last failed call, or with a zero handle for the last failed create.</summary>
    public string LastError(IntPtr host) => Text(host, "m6h_last_error");

    private string Text(IntPtr host, string export)
    {
        var call = (delegate* unmanaged[Cdecl]<IntPtr, byte*, uint, uint*, int>)_exports[export];
        uint size = 0;
        call(host, null, 0, &size);
        if (size == 0)
        {
            return string.Empty;
        }
        var buffer = new byte[size];
        fixed (byte* b = buffer)
        {
            if (call(host, b, size, &size) != Ok)
            {
                return string.Empty;
            }
        }
        return Encoding.UTF8.GetString(buffer, 0, (int)size - 1);
    }

    /// <summary>Frees a host. The core's context stays the shell's to detach afterwards.</summary>
    public void Destroy(IntPtr host) => ((delegate* unmanaged[Cdecl]<IntPtr, void>)_exports["m6h_destroy"])(host);

    public void Dispose()
    {
        if (_library != IntPtr.Zero)
        {
            NativeLibrary.Free(_library);
            _library = IntPtr.Zero;
        }
    }
}
