using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using Godot;

namespace Magic600.Shell.Lx4;

/// <summary>
/// LX-4's module in the product shell (C9 U8). The docstring of tools/perf/c9/lx4.py is its spec. The shell creates it
/// when M600_LX4 is 1 (launch.py --lx4). It loads LX-4's editor library (c600_shell_lx4.dll, built from
/// renderer/bridge/lx4) from M600_LX4_DLL, which reads and checks the fixture folder M600_LX4_FIXTURES. Without either
/// variable, or with a library or fixtures that do not check, the constructor throws, and the shell does not open the
/// core: a measurement run never goes on without its module.
///
/// Messages, before Godot sees them: the registered Magic600.LX4.Reset (wParam the edit's index, lParam the sample id)
/// and Magic600.LX4.Select (wParam start, lParam end, UTF-8 byte offsets), which, once the editor applied it, the module
/// acknowledges with {"t": "lx4-select", "sample", "select"} (the Select's ordinal since the Reset); WM_CHAR, except Godot's own character for a
/// turn key (scan code 0x24 to 0x26), which the shell keeps; and VK_BACK's key-down and key-up. The module consumes
/// them all; turn keys and every other message go on to Godot.
///
/// Frames: after each submitted 3D frame the library takes the newest answers of its worker thread, and the module
/// draws the echo, the status and the completion list in this process frame's Godot frame. Each endpoint the library
/// says this frame shows first becomes an lx4 row, {"t": "lx4", "sample", "endpoint", "frame_id", "buffer_sha256"},
/// whose frame_id is the 3D frame submitted in this process frame, as the ui_frame rows name it. The first frame after
/// a good Reset that draws the baseline writes the sample's ready row, which lx4.py waits for before it posts the
/// sample's inputs. When it starts, the module writes its module row, {"t": "module", "module": "lx4", "window",
/// "run_id", "build_identity", "puzzle", "qpc_frequency"}: the shell seals events.jsonl, with the run row, only at its
/// close, so lx4.py finds the window in this row of the live events.body.jsonl. The look is a placeholder; it stays
/// with the design line.
///
/// Re-run (rerun, RTE.50; C9, 20 Nov 2026): a Reset whose wParam carries the library's LX4_RESET_RERUN flag starts a
/// sample that times the re-run. When lx4_update asks for it, the module runs the buffer's exported function as the
/// host's view job (m6h_run_view, the language runtime with no host rows), polls the job's record each frame, and
/// hands the library the first confirmed frame that drew the run's publication with the run's source digest
/// (lx4_rerun_ended), or the refusal; the library then queues the sample's rerun row for that frame. The host does
/// not draw the run's value yet.
/// </summary>
public sealed unsafe class Lx4Module : IShellModule
{
    /// <summary>The C interface this module speaks; the library's lx4_abi_version must answer it.</summary>
    public const uint AbiVersion = 2;

    /// <summary>The reference view's entry, the library's REFERENCE_ENTRY.</summary>
    public const string ReferenceEntry = "exported";

    public const int Ok = 0;
    public const int InvalidArgument = 1;
    public const int FixturesRefused = 2;
    /// <summary>An input the sample cannot take; the sample has failed and the status says why.</summary>
    public const int StateRefused = 3;
    public const int Poisoned = 6;

    private const uint TextEcho = 0;
    private const uint TextStatus = 1;
    private const uint TextCompletion = 2;
    private const uint TextBuffer = 3;

    private const uint WmKeyDown = 0x0100;
    private const uint WmKeyUp = 0x0101;
    private const uint WmChar = 0x0102;
    private const nuint VkBack = 0x08;

    /// <summary>The endpoints by the library's index, with lx4.py's names; ready is the handshake.</summary>
    public static readonly IReadOnlyList<string> Endpoints = new[]
    {
        "echo", "syntax", "completion", "diagnostics", "stale", "rerun", "dependents", "ready",
    };

    /// <summary>The library's exports, in the order the Rust source declares them.</summary>
    public static readonly IReadOnlyList<string> Exports = new[]
    {
        "lx4_abi_version",
        "lx4_create",
        "lx4_destroy",
        "lx4_reset",
        "lx4_select",
        "lx4_char",
        "lx4_backspace",
        "lx4_update",
        "lx4_rerun_ended",
        "lx4_text",
        "lx4_next_shown",
        "lx4_last_error",
    };

    /// <summary>What lx4_update changed (the Rust Lx4Update).</summary>
    [StructLayout(LayoutKind.Sequential)]
    private struct Update
    {
        public uint Changed;
        public uint Shown;
        public uint Rerun;
        public uint Reserved;
    }

    /// <summary>An endpoint a frame shows first (the Rust Lx4Shown).</summary>
    [StructLayout(LayoutKind.Sequential)]
    private struct Shown
    {
        public ulong Sample;
        public ulong FrameId;
        public uint Endpoint;
        public uint Reserved;
        public fixed byte BufferSha256[64];
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern uint RegisterWindowMessageW(string name);

    private IntPtr _library;
    private readonly Dictionary<string, IntPtr> _exports = new();
    private IntPtr _handle;
    private readonly uint _reset;
    private readonly uint _select;
    private readonly Label _echo;
    private readonly Label _status;
    private readonly Label _completion;
    private bool _poisonReported;
    // The last Reset's sample id and the Selects the editor has applied since (the lx4-select rows' ordinals).
    private ulong _sample;
    private long _selects;
    // The re-run's view job while its record is not final.
    private ulong _rerunTicket;
    private bool _rerunRunning;

    public Lx4Module(ShellContext context)
    {
        Context = context;
        var dll = Required("M600_LX4_DLL");
        var fixtures = Required("M600_LX4_FIXTURES");
        if (!System.IO.Path.IsPathFullyQualified(dll))
        {
            throw new ArgumentException("M600_LX4_DLL needs an absolute path");
        }
        _library = NativeLibrary.Load(dll);
        try
        {
            foreach (var name in Exports)
            {
                if (!NativeLibrary.TryGetExport(_library, name, out var address))
                {
                    throw new EntryPointNotFoundException($"LX-4's library does not export {name}");
                }
                _exports[name] = address;
            }
            var version = ((delegate* unmanaged[Cdecl]<uint>)_exports["lx4_abi_version"])();
            if (version != AbiVersion)
            {
                throw new NotSupportedException($"LX-4's library speaks version {version}, the module {AbiVersion}");
            }
            var path = Encoding.UTF8.GetBytes(fixtures);
            var created = IntPtr.Zero;
            int code;
            fixed (byte* p = path)
            {
                code = ((delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr*, int>)_exports["lx4_create"])(
                    p, (nuint)path.Length, &created);
            }
            if (code != Ok)
            {
                var reason = Text(IntPtr.Zero, "lx4_last_error", 0);
                throw new InvalidOperationException($"LX-4's fixtures do not load ({code}): {reason}");
            }
            _handle = created;
            _reset = Register("Magic600.LX4.Reset");
            _select = Register("Magic600.LX4.Select");
        }
        catch
        {
            Dispose();
            throw;
        }
        var window = (long)context.Window;
        var runId = System.Environment.GetEnvironmentVariable("M600_RUN_ID") ?? string.Empty;
        var build = System.Environment.GetEnvironmentVariable("M600_BUILD_IDENTITY") ?? string.Empty;
        var puzzle = EventLog.PuzzleFromEnvironment();
        Context.Log?.Append(json =>
        {
            json.WriteString("t", "module");
            json.WriteString("module", "lx4");
            json.WriteNumber("window", window);
            json.WriteString("run_id", runId);
            json.WriteString("build_identity", build);
            json.WriteString("puzzle", puzzle);
            json.WriteNumber("qpc_frequency", System.Diagnostics.Stopwatch.Frequency);
        });
        _echo = AddLabel(new Vector2(16, 16), new Vector2(900, 640));
        _status = AddLabel(new Vector2(940, 16), new Vector2(640, 200));
        _completion = AddLabel(new Vector2(940, 240), new Vector2(640, 400));
    }

    /// <summary>The window, the root control and the event log the shell gave this module.</summary>
    public ShellContext Context { get; }

    private static string Required(string name)
    {
        var value = System.Environment.GetEnvironmentVariable(name);
        if (string.IsNullOrEmpty(value))
        {
            throw new ArgumentException($"LX-4's module needs {name}");
        }
        return value;
    }

    private static uint Register(string name)
    {
        var id = RegisterWindowMessageW(name);
        if (id == 0)
        {
            throw new System.ComponentModel.Win32Exception(
                Marshal.GetLastPInvokeError(), $"RegisterWindowMessageW({name}) failed");
        }
        return id;
    }

    private Label AddLabel(Vector2 position, Vector2 size)
    {
        var font = new SystemFont { FontNames = new[] { "Consolas", "Cascadia Mono", "monospace" } };
        var label = new Label { Position = position, Size = size, ClipText = true };
        label.AddThemeFontOverride("font", font);
        label.AddThemeFontSizeOverride("font_size", 13);
        Context.Root.AddChild(label);
        return label;
    }

    public bool OnWindowMessage(uint message, nuint wParam, nint lParam, long qpc)
    {
        if (_handle == IntPtr.Zero)
        {
            return false;
        }
        if (message == _reset)
        {
            _sample = (ulong)lParam;
            _selects = 0;
            Report(((delegate* unmanaged[Cdecl]<IntPtr, uint, ulong, int>)_exports["lx4_reset"])(
                _handle, (uint)wParam, (ulong)lParam));
            return true;
        }
        if (message == _select)
        {
            // An applied Select is acknowledged, so that the harness posts the replacement's keys only after it
            // (C9 U8, 20 Nov 2026: Selects were applied after keys posted later).
            if (Report(((delegate* unmanaged[Cdecl]<IntPtr, ulong, ulong, int>)_exports["lx4_select"])(
                    _handle, (ulong)wParam, (ulong)lParam)) == Ok)
            {
                var sample = _sample;
                var ordinal = ++_selects;
                Context.Log?.Append(json =>
                {
                    json.WriteString("t", "lx4-select");
                    json.WriteNumber("sample", sample);
                    json.WriteNumber("select", ordinal);
                });
            }
            return true;
        }
        if (message == WmChar)
        {
            uint taken = 1;
            var scan = (uint)((lParam >> 16) & 0xFF);
            Report(((delegate* unmanaged[Cdecl]<IntPtr, ushort, uint, uint*, int>)_exports["lx4_char"])(
                _handle, (ushort)wParam, scan, &taken));
            return taken != 0;
        }
        if ((message == WmKeyDown || message == WmKeyUp) && wParam == VkBack)
        {
            if (message == WmKeyDown)
            {
                Report(((delegate* unmanaged[Cdecl]<IntPtr, int>)_exports["lx4_backspace"])(_handle));
            }
            return true;
        }
        return false;
    }

    public void OnFrame(HostLibrary.Submitted frame, long qpc)
    {
        if (_handle == IntPtr.Zero)
        {
            return;
        }
        Update update;
        if (Report(((delegate* unmanaged[Cdecl]<IntPtr, ulong, Update*, int>)_exports["lx4_update"])(
                _handle, frame.Sequence, &update)) != Ok)
        {
            return;
        }
        if ((update.Changed & (1u << (int)TextEcho)) != 0)
        {
            _echo.Text = Text(_handle, "lx4_text", TextEcho);
        }
        if ((update.Changed & (1u << (int)TextStatus)) != 0)
        {
            _status.Text = Text(_handle, "lx4_text", TextStatus);
        }
        if ((update.Changed & (1u << (int)TextCompletion)) != 0)
        {
            _completion.Text = Text(_handle, "lx4_text", TextCompletion);
        }
        if (update.Rerun != 0)
        {
            StartRerun();
        }
        PollRerun();
        // A rerun row the poll queued is written in this process frame.
        var next = (delegate* unmanaged[Cdecl]<IntPtr, Shown*, uint*, int>)_exports["lx4_next_shown"];
        while (true)
        {
            Shown shown;
            uint present = 0;
            if (Report(next(_handle, &shown, &present)) != Ok || present == 0)
            {
                return;
            }
            var sample = shown.Sample;
            var frameId = shown.FrameId;
            var endpoint = shown.Endpoint < Endpoints.Count ? Endpoints[(int)shown.Endpoint] : "unknown";
            var sha = Encoding.ASCII.GetString(shown.BufferSha256, 64);
            Context.Log?.Append(json =>
            {
                json.WriteString("t", "lx4");
                json.WriteNumber("sample", sample);
                json.WriteString("endpoint", endpoint);
                json.WriteNumber("frame_id", frameId);
                json.WriteString("buffer_sha256", sha);
            });
        }
    }

    public void OnPresented(HostLibrary.Presented row)
    {
    }

    /// <summary>Runs the buffer's reference view as the host's view job, or ends the re-run refused.</summary>
    private void StartRerun()
    {
        var source = Text(_handle, "lx4_text", TextBuffer);
        var code = Context.Host.RunView(Context.HostHandle, source, ReferenceEntry, out var ticket);
        if (code != HostLibrary.Ok)
        {
            _rerunRunning = false;
            RerunEnded(0, null, $"m6h_run_view answered {code}: {Context.Host.LastError(Context.HostHandle)}");
            return;
        }
        // A newer run supersedes the host's pending one, so only the newest ticket is polled.
        _rerunTicket = ticket;
        _rerunRunning = true;
    }

    /// <summary>
    /// Hands the library the re-run's end once its record is final: published with the first confirmed frame that drew
    /// its publication, or refused.
    /// </summary>
    private void PollRerun()
    {
        if (!_rerunRunning)
        {
            return;
        }
        var code = Context.Host.GetViewResult(Context.HostHandle, _rerunTicket, out var result);
        if (code != HostLibrary.Ok)
        {
            _rerunRunning = false;
            RerunEnded(0, null, $"m6h_view_result answered {code}: {Context.Host.LastError(Context.HostHandle)}");
            return;
        }
        if (result.Status == HostLibrary.ViewStatusPending
            || (result.Status == HostLibrary.ViewStatusPublished && result.Frame == 0))
        {
            return;
        }
        _rerunRunning = false;
        if (result.Status == HostLibrary.ViewStatusPublished)
        {
            RerunEnded(result.Frame, result.SourceDigest, string.Empty);
        }
        else
        {
            RerunEnded(0, null, Context.Host.LastError(Context.HostHandle));
        }
    }

    private void RerunEnded(ulong frame, byte* digest, string reason)
    {
        var bytes = Encoding.UTF8.GetBytes(reason);
        fixed (byte* r = bytes)
        {
            Report(((delegate* unmanaged[Cdecl]<IntPtr, ulong, byte*, byte*, nuint, int>)_exports["lx4_rerun_ended"])(
                _handle, frame, digest, r, (nuint)bytes.Length));
        }
    }

    /// <summary>
    /// A poisoned library is reported once; a refused input shows in the status as the sample's failure.
    /// </summary>
    private int Report(int code)
    {
        if (code == Poisoned && !_poisonReported)
        {
            _poisonReported = true;
            GD.PushError($"LX-4's library is poisoned: {Text(_handle, "lx4_last_error", 0)}");
        }
        return code;
    }

    /// <summary>
    /// A NUL-terminated text of the library: lx4_text's (with <paramref name="which"/>) or lx4_last_error's.
    /// </summary>
    private string Text(IntPtr handle, string export, uint which)
    {
        uint size = 0;
        int code;
        if (export == "lx4_text")
        {
            var call = (delegate* unmanaged[Cdecl]<IntPtr, uint, byte*, uint, uint*, int>)_exports[export];
            call(handle, which, null, 0, &size);
            if (size == 0)
            {
                return string.Empty;
            }
            var buffer = new byte[size];
            fixed (byte* b = buffer)
            {
                code = call(handle, which, b, size, &size);
            }
            return code == Ok ? Encoding.UTF8.GetString(buffer, 0, (int)size - 1) : string.Empty;
        }
        else
        {
            var call = (delegate* unmanaged[Cdecl]<IntPtr, byte*, uint, uint*, int>)_exports[export];
            call(handle, null, 0, &size);
            if (size == 0)
            {
                return string.Empty;
            }
            var buffer = new byte[size];
            fixed (byte* b = buffer)
            {
                code = call(handle, b, size, &size);
            }
            return code == Ok ? Encoding.UTF8.GetString(buffer, 0, (int)size - 1) : string.Empty;
        }
    }

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            ((delegate* unmanaged[Cdecl]<IntPtr, void>)_exports["lx4_destroy"])(_handle);
            _handle = IntPtr.Zero;
        }
        if (_library != IntPtr.Zero)
        {
            NativeLibrary.Free(_library);
            _library = IntPtr.Zero;
        }
    }
}
