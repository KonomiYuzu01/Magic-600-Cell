using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace Magic600.Shell.Editor;

/// <summary>
/// The shell's editor library (c600_shell_editor.dll, built from renderer/bridge/editor; LP4-3, a draft): the document
/// with EV1's editor services, the host's user modules with an explicit check and an explicit activation, and the runs
/// of the program jobs the host's command layer queues for a language row. It shares no object with the host library:
/// <see cref="EditorPanel"/> carries the JSON between them. tests/test_renderer_shell.py checks <see cref="Exports"/>,
/// the version and the codes against the Rust source.
/// </summary>
public sealed unsafe class EditorLibrary : IDisposable
{
    /// <summary>The C interface this shell speaks; the library's m6e_abi_version must answer it.</summary>
    public const uint AbiVersion = 1;

    public const int Ok = 0;
    public const int InvalidArgument = 1;
    /// <summary>A service, the module set or a binding refused; <see cref="LastError"/> gives {"code", "detail"}.
    /// </summary>
    public const int Refused = 2;
    /// <summary>The runtime was loaded already, or the worker thread did not start.</summary>
    public const int RuntimeRefused = 3;
    public const int Poisoned = 6;

    /// <summary><see cref="Create"/>'s units: Godot's columns count scalars (code points).</summary>
    public const uint UnitsUtf8 = 0;
    public const uint UnitsUtf16 = 1;
    public const uint UnitsScalar = 2;

    /// <summary><see cref="RunPoll"/>'s states.</summary>
    public const uint RunPending = 0;
    public const uint RunCompleted = 1;
    public const uint RunRefused = 2;

    /// <summary>The library's exports, in the order the Rust source declares them.</summary>
    public static readonly IReadOnlyList<string> Exports = new[]
    {
        "m6e_abi_version",
        "m6e_create",
        "m6e_load_runtime",
        "m6e_set_text",
        "m6e_edit",
        "m6e_diagnose",
        "m6e_complete",
        "m6e_check",
        "m6e_activate",
        "m6e_run",
        "m6e_run_poll",
        "m6e_answer",
        "m6e_last_error",
        "m6e_destroy",
    };

    private IntPtr _library;
    private readonly Dictionary<string, IntPtr> _exports;

    private EditorLibrary(IntPtr library, Dictionary<string, IntPtr> exports)
    {
        _library = library;
        _exports = exports;
    }

    /// <summary>
    /// Loads the library from an absolute path, resolves every export and checks the version. A missing export or
    /// another version unloads the library and throws.
    /// </summary>
    public static EditorLibrary Load(string path)
    {
        if (!System.IO.Path.IsPathFullyQualified(path))
        {
            throw new ArgumentException("the editor library needs an absolute path", nameof(path));
        }
        var library = NativeLibrary.Load(path);
        try
        {
            var exports = new Dictionary<string, IntPtr>();
            foreach (var name in Exports)
            {
                if (!NativeLibrary.TryGetExport(library, name, out var address))
                {
                    throw new EntryPointNotFoundException($"the editor library does not export {name}");
                }
                exports[name] = address;
            }
            var version = ((delegate* unmanaged[Cdecl]<uint>)exports["m6e_abi_version"])();
            if (version != AbiVersion)
            {
                throw new NotSupportedException($"the editor library speaks version {version}, the shell {AbiVersion}");
            }
            return new EditorLibrary(library, exports);
        }
        catch
        {
            NativeLibrary.Free(library);
            throw;
        }
    }

    /// <summary>Creates a handle whose document counts positions in <paramref name="units"/>.</summary>
    public int Create(uint units, out IntPtr editor)
    {
        var created = IntPtr.Zero;
        var code = ((delegate* unmanaged[Cdecl]<uint, IntPtr*, int>)_exports["m6e_create"])(units, &created);
        editor = created;
        return code;
    }

    /// <summary>Runs later runs in the language runtime's process that <paramref name="program"/> starts.</summary>
    public int LoadRuntime(IntPtr editor, string program) => WithText(editor, "m6e_load_runtime", program);

    /// <summary>Replaces the buffer; <see cref="Answer"/> gives {"digest"}.</summary>
    public int SetText(IntPtr editor, string text) => WithText(editor, "m6e_set_text", text);

    /// <summary>Replaces [start, end) in the declared units with <paramref name="text"/>.</summary>
    public int Edit(IntPtr editor, ulong start, ulong end, string text)
    {
        var t = Encoding.UTF8.GetBytes(text);
        fixed (byte* tb = t)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, ulong, ulong, byte*, nuint, int>)_exports["m6e_edit"])(
                editor, start, end, t.Length == 0 ? null : tb, (nuint)t.Length);
        }
    }

    /// <summary>Diagnoses the buffer; <see cref="Answer"/> gives the verdict, the exports and the diagnostics.</summary>
    public int Diagnose(IntPtr editor) => ((delegate* unmanaged[Cdecl]<IntPtr, int>)_exports["m6e_diagnose"])(editor);

    /// <summary>Completes at <paramref name="position"/>; <see cref="Answer"/> gives the candidates.</summary>
    public int Complete(IntPtr editor, ulong position) =>
        ((delegate* unmanaged[Cdecl]<IntPtr, ulong, int>)_exports["m6e_complete"])(editor, position);

    /// <summary>
    /// Checks the buffer with its function <paramref name="entry"/> as the entry; <see cref="Answer"/> gives the
    /// candidate and its check reference. Nothing activates.
    /// </summary>
    public int Check(IntPtr editor, string entry) => WithText(editor, "m6e_check", entry);

    /// <summary>
    /// Activates the checked candidate of <paramref name="module"/> when its active version is
    /// <paramref name="expected"/> (32 bytes, or empty for none): the explicit switch.
    /// </summary>
    public int Activate(IntPtr editor, string module, ReadOnlySpan<byte> expected, ReadOnlySpan<byte> candidate,
        ReadOnlySpan<byte> reference)
    {
        if ((expected.Length is not (0 or 32)) || candidate.Length != 32 || reference.Length != 32)
        {
            throw new ArgumentException("a digest is 32 bytes");
        }
        var m = Encoding.UTF8.GetBytes(module);
        fixed (byte* mb = m)
        fixed (byte* eb = expected)
        fixed (byte* cb = candidate)
        fixed (byte* rb = reference)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, byte*, byte*, byte*, int>)_exports["m6e_activate"])(
                editor, mb, (nuint)m.Length, expected.Length == 0 ? null : eb, cb, rb);
        }
    }

    /// <summary>Runs the program job whose JSON the host's <see cref="HostLibrary.TakeJob"/> answered.</summary>
    public int Run(IntPtr editor, string job, out ulong ticket)
    {
        var j = Encoding.UTF8.GetBytes(job);
        ulong t = 0;
        int code;
        fixed (byte* jb = j)
        {
            code = ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, ulong*, int>)_exports["m6e_run"])(
                editor, jb, (nuint)j.Length, &t);
        }
        ticket = t;
        return code;
    }

    /// <summary>Reads a run's state; <see cref="Answer"/> gives its record once it ended.</summary>
    public int RunPoll(IntPtr editor, ulong ticket, out uint state)
    {
        uint s = 0;
        var code = ((delegate* unmanaged[Cdecl]<IntPtr, ulong, uint*, int>)_exports["m6e_run_poll"])(editor, ticket, &s);
        state = s;
        return code;
    }

    /// <summary>The JSON answer of the last call that gives one.</summary>
    public string Answer(IntPtr editor) => Text(editor, "m6e_answer");

    /// <summary>The reason for the last failed call: {"code", "detail"} after <see cref="Refused"/>.</summary>
    public string LastError(IntPtr editor) => Text(editor, "m6e_last_error");

    /// <summary>Frees a handle; the worker thread ends after the run in flight.</summary>
    public void Destroy(IntPtr editor) => ((delegate* unmanaged[Cdecl]<IntPtr, void>)_exports["m6e_destroy"])(editor);

    private int WithText(IntPtr editor, string export, string text)
    {
        var t = Encoding.UTF8.GetBytes(text);
        fixed (byte* tb = t)
        {
            return ((delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, int>)_exports[export])(
                editor, t.Length == 0 ? null : tb, (nuint)t.Length);
        }
    }

    private string Text(IntPtr editor, string export)
    {
        var call = (delegate* unmanaged[Cdecl]<IntPtr, byte*, uint, uint*, int>)_exports[export];
        uint size = 0;
        call(editor, null, 0, &size);
        if (size == 0)
        {
            return string.Empty;
        }
        var buffer = new byte[size];
        fixed (byte* b = buffer)
        {
            if (call(editor, b, size, &size) != Ok)
            {
                return string.Empty;
            }
        }
        return Encoding.UTF8.GetString(buffer, 0, (int)size - 1);
    }

    public void Dispose()
    {
        if (_library != IntPtr.Zero)
        {
            NativeLibrary.Free(_library);
            _library = IntPtr.Zero;
        }
    }
}
