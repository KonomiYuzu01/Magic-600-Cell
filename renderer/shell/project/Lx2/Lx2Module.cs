using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace Magic600.Shell.Lx2;

/// <summary>
/// LX-2's module in the product shell (C9 U7; RTE.42 conditions 3 and 4: the renderer gate, LX-4's echo bar and charter
/// E2's commits during the runtime's fault windows). The docstring of tools/perf/c9/lx2.py is its spec. The shell
/// creates it when M600_LX2 is 1 (launch.py --lx2-injection), and the host's runtime is then c600-lx2's injection
/// child, M600_RUNTIME, which this module finds by its image path. Without M600_RUNTIME the constructor throws, and
/// the shell does not open the core.
///
/// Phases: the registered message Magic600.LX2.Phase (wParam a fault's number, 1 to 7 in <see cref="Faults"/>' order,
/// or 0 to stop) starts a phase, which runs fault windows of that fault back to back until another phase message; a
/// change is taken only between windows, after the runtime has recovered and the fault variables are clear. The module
/// consumes the message.
///
/// A window: the fault's view job (<see cref="HostLibrary.RunView"/>, under the host's view limits and deadline), one
/// turn this module posts to the shell's window as the shell's turn key J (WM_KEYDOWN and WM_KEYUP with scan code
/// 0x24, as lx4.py posts it; the native loop posts one per <see cref="PaceMs"/> while its armed child lives), then
/// control probes until one publishes. A twist ends a pending view job (KN7:cancelled.revision_replaced), so each
/// fault posts its turn where the fault has taken effect: the metered faults <see cref="LeadMs"/> after the start,
/// while the job runs; forced_kill right after it killed the child that runs the job; the native recursion and the
/// allocator ceiling after the job ended with the child's loss; the native loop from <see cref="LeadMs"/> after the
/// start while its child spins. A probe a turn cancels is sent again, so the window ends after the commit.
///
/// Native faults (the injection child's variables, which it reads when it starts; C9 product's two holes, 19 November
/// 2026): before the job the module ends every live injection child and waits until it has exited, so that the job's
/// start spawns a fresh one, and only then sets C600_LX2_CEILING or C600_LX2_FAULT in this process, between jobs, with
/// nothing queued. It clears them again after the job has ended (the native loop: once its armed child is seen), before
/// the probe spawns the next child.
///
/// Rows, {"t": "lx2", "event", "qpc", "window", "fault", ...}: phase and phase_end (windows); start (ticket, armed,
/// ended_before: the children ended before an armed start); turn (ok); kill (killed); fault_end (status, code);
/// child_exit (found, pid, exit_code); probe_refused (code); recovered (probes); error (reason), after which the module
/// does nothing more. A window runs from its start row to its recovered row. When it starts, the module writes its
/// module row, {"t": "module", "module": "lx2", "window", "run_id", "build_identity", "puzzle", "qpc_frequency",
/// "runtime" (the child's file name, never its path)}, which lx2.py reads in the live events.body.jsonl.
/// </summary>
public sealed class Lx2Module : IShellModule
{
    /// <summary>The faults by number (wParam 1 to 7); lx2.py's FAULTS lists the same names in the same order.</summary>
    public static readonly string[] Faults =
    {
        "alloc_metered", "recursion_metered", "loop_metered", "forced_kill", "alloc_ceiling", "recursion_native",
        "loop_native",
    };

    private const int AllocMetered = 1;
    private const int RecursionMetered = 2;
    private const int LoopMetered = 3;
    private const int ForcedKill = 4;
    private const int AllocCeiling = 5;
    private const int RecursionNative = 6;
    private const int LoopNative = 7;

    public const string CeilingVariable = "C600_LX2_CEILING";
    public const string FaultVariable = "C600_LX2_FAULT";
    /// <summary>c600_lx2::CEILING: the injection child's live heap ceiling for alloc_ceiling, 32 MiB.</summary>
    public const long CeilingBytes = 32L << 20;
    /// <summary>The metered faults' turn, forced_kill's kill and the native loop's first turn, after the
    /// start.</summary>
    public const int LeadMs = 300;
    /// <summary>The native loop's turns while its armed child lives.</summary>
    public const int PaceMs = 500;
    /// <summary>From a window's recovery to the next window's start.</summary>
    public const int GapMs = 200;
    /// <summary>Probes in one window before the module stops with an error.</summary>
    public const int ProbeAttempts = 100;
    /// <summary>How long the native loop's watcher looks for the armed child, and waits for it to exit.</summary>
    public const int FindMs = 5_000;
    public const int ExitMs = 120_000;
    /// <summary>Every view job's entry.</summary>
    public const string Entry = "work";

    private const string Header = "edition \"6cs-draft-1\";\nmodule lx2 requires [];\n\n";
    private const uint WmKeyDown = 0x0100;
    private const uint WmKeyUp = 0x0101;
    /// <summary>The shell's turn key J: virtual key and scan code (lx4.py's TURN_KEYS["KeyJ"]).</summary>
    private const uint VkJ = 0x4A;
    private const uint ScanJ = 0x24;
    private const uint ProcessQueryLimitedInformation = 0x1000;

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern uint RegisterWindowMessageW(string name);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool PostMessageW(IntPtr window, uint message, nuint wParam, nint lParam);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr OpenProcess(uint access, [MarshalAs(UnmanagedType.Bool)] bool inherit, int id);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool QueryFullProcessImageNameW(
        IntPtr process, uint flags, StringBuilder name, ref uint size);

    [DllImport("kernel32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseHandle(IntPtr handle);

    private enum Stage { Idle, Preparing, Running, Recovering, Gap, Stopped }

    /// <summary>What the native loop's watcher saw of the armed child.</summary>
    private sealed record ChildExit(bool Found, int Pid, int? ExitCode, long Qpc);

    private readonly uint _phaseMessage;
    private readonly string _runtime;
    private readonly long _frequency = Stopwatch.Frequency;
    private int _requested;
    private int _phase;
    private Stage _stage = Stage.Idle;
    private long _window;
    private long _windows;
    private ulong _faultTicket;
    private long _start;
    private long _lastTurn;
    private long _gapUntil;
    private bool _faultFinal;
    private bool _turned;
    private bool _killRequested;
    private bool _exitWritten;
    private int _probes;
    private string? _armed;
    private int _endedBefore;
    private Task<int>? _prepare;
    private Task<int>? _kill;
    private Task<ChildExit>? _watch;
    private volatile int _childPid;
    private ulong _probeTicket;

    public Lx2Module(ShellContext context)
    {
        Context = context;
        var runtime = System.Environment.GetEnvironmentVariable("M600_RUNTIME");
        if (string.IsNullOrEmpty(runtime) || !Path.IsPathFullyQualified(runtime))
        {
            throw new ArgumentException("LX-2's module needs M600_RUNTIME, the injection child's absolute path");
        }
        _runtime = Path.GetFullPath(runtime);
        _phaseMessage = Register("Magic600.LX2.Phase");
        var window = (long)context.Window;
        var runId = System.Environment.GetEnvironmentVariable("M600_RUN_ID") ?? string.Empty;
        var build = System.Environment.GetEnvironmentVariable("M600_BUILD_IDENTITY") ?? string.Empty;
        var puzzle = EventLog.PuzzleFromEnvironment();
        var child = Path.GetFileName(_runtime);
        Context.Log?.Append(json =>
        {
            json.WriteString("t", "module");
            json.WriteString("module", "lx2");
            json.WriteNumber("window", window);
            json.WriteString("run_id", runId);
            json.WriteString("build_identity", build);
            json.WriteString("puzzle", puzzle);
            json.WriteNumber("qpc_frequency", Stopwatch.Frequency);
            json.WriteString("runtime", child);
        });
    }

    /// <summary>The window, the root control, the event log and the host the shell gave this module.</summary>
    public ShellContext Context { get; }

    /// <summary>The view job's text for a fault number, or the control probe's for 0: module lx2, entry
    /// <see cref="Entry"/>, no kernel call (the view's jobs run with no host row). The bodies are c600-lx2's
    /// trials'.</summary>
    public static string Source(int fault)
    {
        var body = fault switch
        {
            AllocMetered =>
                "    let grown = repeat(1000000000000, list<Nat>(), fn(done: List<Nat>) -> List<Nat> {\n" +
                "        return push(done, length(done));\n    });\n    return length(grown);\n",
            RecursionMetered => "    return down(0);\n",
            LoopMetered or ForcedKill =>
                "    return repeat(1000000000000, 0, fn(acc: Nat) -> Nat {\n" +
                "        return exact_add(acc, 1);\n    });\n",
            AllocCeiling =>
                "    let seed = push(list<Nat>(), 0);\n" +
                "    let grown = repeat(1000000000000, seed, fn(done: List<Nat>) -> List<Nat> {\n" +
                "        return concat(done, done);\n    });\n    return length(grown);\n",
            _ => "    return 1;\n",
        };
        var down = fault == RecursionMetered
            ? "\nfn down(n: Nat) -> Nat effects [] {\n    return down(exact_add(n, 1));\n}\n"
            : string.Empty;
        return $"{Header}export fn {Entry}() -> Nat effects [] {{\n{body}}}\n{down}";
    }

    public bool OnWindowMessage(uint message, nuint wParam, nint lParam, long qpc)
    {
        if (message != _phaseMessage)
        {
            return false;
        }
        if (wParam > (nuint)Faults.Length)
        {
            Error($"Magic600.LX2.Phase names no fault: {wParam}");
        }
        else
        {
            _requested = (int)wParam;
        }
        return true;
    }

    public void OnFrame(HostLibrary.Submitted frame, long qpc)
    {
        try
        {
            Step(EventLog.Now());
        }
        catch (Exception error)
        {
            Error($"{error.GetType().Name}: {error.Message}");
        }
    }

    public void OnPresented(HostLibrary.Presented row)
    {
    }

    public void Dispose()
    {
        Disarm();
        _stage = Stage.Stopped;
    }

    private static uint Register(string name)
    {
        var id = RegisterWindowMessageW(name);
        if (id == 0)
        {
            throw new Win32Exception(Marshal.GetLastPInvokeError(), $"RegisterWindowMessageW({name}) failed");
        }
        return id;
    }

    private static bool Native(int fault) => fault is AllocCeiling or RecursionNative or LoopNative;

    private double Ms(long ticks) => ticks * 1000.0 / _frequency;

    private void Step(long now)
    {
        switch (_stage)
        {
            case Stage.Idle:
                if (_requested != _phase)
                {
                    if (_phase != 0)
                    {
                        var windows = _windows;
                        Row("phase_end", json => json.WriteNumber("windows", windows));
                    }
                    _phase = _requested;
                    _windows = 0;
                    if (_phase != 0)
                    {
                        Row("phase");
                    }
                }
                if (_phase != 0)
                {
                    Begin(now);
                }
                break;
            case Stage.Preparing:
                if (_prepare is { IsCompleted: true } prepare)
                {
                    _endedBefore = prepare.Result;
                    Arm();
                    Submit(now);
                }
                break;
            case Stage.Running:
                Run(now);
                break;
            case Stage.Recovering:
                if (Final(_probeTicket, out var status, out var code))
                {
                    if (status == "published")
                    {
                        var probes = _probes;
                        Row("recovered", json => json.WriteNumber("probes", probes));
                        _stage = Stage.Gap;
                        _gapUntil = now + GapMs * _frequency / 1000;
                    }
                    else
                    {
                        Row("probe_refused", json => json.WriteString("code", code));
                        Probe();
                    }
                }
                break;
            case Stage.Gap:
                if (now >= _gapUntil)
                {
                    _stage = Stage.Idle;
                }
                break;
            case Stage.Stopped:
                break;
        }
    }

    private void Begin(long now)
    {
        _window++;
        _windows++;
        _faultFinal = _turned = _killRequested = _exitWritten = false;
        _probes = _endedBefore = 0;
        _kill = null;
        _watch = null;
        _childPid = 0;
        if (Native(_phase))
        {
            var runtime = _runtime;
            _prepare = Task.Run(() => EndChildren(runtime));
            _stage = Stage.Preparing;
        }
        else
        {
            Submit(now);
        }
    }

    private void Submit(long now)
    {
        var code = Context.Host.RunView(Context.HostHandle, Source(_phase), Entry, out var ticket);
        if (code != HostLibrary.Ok)
        {
            Error($"RunView answered {code}: {Context.Host.LastError(Context.HostHandle)}");
            return;
        }
        _faultTicket = ticket;
        _start = EventLog.Now();
        _stage = Stage.Running;
        var armed = _armed;
        var ended = _endedBefore;
        Row("start", json =>
        {
            json.WriteNumber("ticket", ticket);
            if (armed is null)
            {
                json.WriteNull("armed");
            }
            else
            {
                json.WriteString("armed", armed);
            }
            json.WriteNumber("ended_before", ended);
        });
        if (_phase == LoopNative)
        {
            var runtime = _runtime;
            _watch = Task.Run(() => Watch(runtime));
        }
    }

    private void Run(long now)
    {
        if (!_faultFinal && Final(_faultTicket, out var status, out var code))
        {
            _faultFinal = true;
            Row("fault_end", json =>
            {
                json.WriteString("status", status);
                json.WriteString("code", code);
            });
        }
        var elapsed = Ms(now - _start);
        switch (_phase)
        {
            case AllocMetered or RecursionMetered or LoopMetered:
                if (!_turned && !_faultFinal && elapsed >= LeadMs)
                {
                    Turn();
                }
                if (_faultFinal)
                {
                    Probe();
                }
                break;
            case ForcedKill:
                if (!_killRequested && !_faultFinal && elapsed >= LeadMs)
                {
                    var runtime = _runtime;
                    _kill = Task.Run(() => EndChildren(runtime));
                    _killRequested = true;
                }
                if (_kill is { IsCompleted: true } kill && !_turned)
                {
                    var killed = kill.Result;
                    Row("kill", json => json.WriteNumber("killed", killed));
                    Turn();
                }
                if (_faultFinal && (!_killRequested || _turned))
                {
                    Probe();
                }
                break;
            case AllocCeiling or RecursionNative:
                if (_faultFinal)
                {
                    Disarm();
                    Turn();
                    Probe();
                }
                break;
            case LoopNative:
                var watch = _watch!;
                var seen = _childPid != 0;
                if (seen && !watch.IsCompleted && elapsed >= LeadMs && (!_turned || Ms(now - _lastTurn) >= PaceMs))
                {
                    Turn();
                }
                if (_faultFinal && seen)
                {
                    // The armed child has started, so its spawn has read the variables.
                    Disarm();
                }
                if (watch.IsCompleted)
                {
                    if (!_exitWritten)
                    {
                        _exitWritten = true;
                        var exit = watch.Result;
                        Row("child_exit", json =>
                        {
                            json.WriteBoolean("found", exit.Found);
                            json.WriteNumber("pid", exit.Pid);
                            if (exit.ExitCode is { } exitCode)
                            {
                                json.WriteNumber("exit_code", exitCode);
                            }
                            else
                            {
                                json.WriteNull("exit_code");
                            }
                            json.WriteNumber("exit_qpc", exit.Qpc);
                        });
                    }
                    if (_faultFinal)
                    {
                        Disarm();
                        Probe();
                    }
                }
                break;
        }
    }

    /// <summary>Whether the job's record is final, with its status and, for a refused one, its code.</summary>
    private bool Final(ulong ticket, out string status, out string code)
    {
        status = code = string.Empty;
        var answer = Context.Host.GetViewResult(Context.HostHandle, ticket, out var result);
        if (answer != HostLibrary.Ok)
        {
            throw new InvalidOperationException(
                $"m6h_view_result answered {answer}: {Context.Host.LastError(Context.HostHandle)}");
        }
        if (result.Status == HostLibrary.ViewStatusPending)
        {
            return false;
        }
        if (result.Status == HostLibrary.ViewStatusPublished)
        {
            status = "published";
            return true;
        }
        status = "refused";
        code = RefusalCode(Context.Host.LastError(Context.HostHandle));
        return true;
    }

    private static string RefusalCode(string error)
    {
        try
        {
            using var document = JsonDocument.Parse(error);
            return document.RootElement.TryGetProperty("code", out var code) && code.ValueKind == JsonValueKind.String
                ? code.GetString() ?? string.Empty
                : string.Empty;
        }
        catch (JsonException)
        {
            return string.Empty;
        }
    }

    private void Probe()
    {
        if (_stage == Stage.Stopped)
        {
            return;
        }
        if (++_probes > ProbeAttempts)
        {
            Error($"no probe published in {ProbeAttempts} attempts");
            return;
        }
        var code = Context.Host.RunView(Context.HostHandle, Source(0), Entry, out var ticket);
        if (code != HostLibrary.Ok)
        {
            Error($"RunView answered {code}: {Context.Host.LastError(Context.HostHandle)}");
            return;
        }
        _probeTicket = ticket;
        _stage = Stage.Recovering;
    }

    /// <summary>One press of the turn key J, posted to the shell's window; its turn row's qpc is read just
    /// before.</summary>
    private void Turn()
    {
        var qpc = EventLog.Now();
        var down = PostMessageW(Context.Window, WmKeyDown, VkJ, (nint)(1 | ScanJ << 16));
        var up = PostMessageW(Context.Window, WmKeyUp, VkJ, (nint)(0xC0000001L | (long)ScanJ << 16));
        _turned = true;
        _lastTurn = qpc;
        var ok = down && up;
        Row("turn", json => json.WriteBoolean("ok", ok), qpc);
    }

    private void Arm()
    {
        (string Name, string Value)? variable = _phase switch
        {
            AllocCeiling => (CeilingVariable, CeilingBytes.ToString(System.Globalization.CultureInfo.InvariantCulture)),
            RecursionNative => (FaultVariable, "recursion@0"),
            LoopNative => (FaultVariable, "loop@0"),
            _ => null,
        };
        if (variable is { } set)
        {
            System.Environment.SetEnvironmentVariable(set.Name, set.Value);
            _armed = $"{set.Name}={set.Value}";
        }
    }

    private void Disarm()
    {
        if (_armed is null)
        {
            return;
        }
        System.Environment.SetEnvironmentVariable(CeilingVariable, null);
        System.Environment.SetEnvironmentVariable(FaultVariable, null);
        _armed = null;
    }

    private void Error(string reason)
    {
        Disarm();
        _stage = Stage.Stopped;
        Row("error", json => json.WriteString("reason", reason));
    }

    private void Row(string name, Action<Utf8JsonWriter>? fields = null, long? at = null)
    {
        var qpc = at ?? EventLog.Now();
        var window = _window;
        var fault = _phase == 0 ? null : Faults[_phase - 1];
        Context.Log?.Append(json =>
        {
            json.WriteString("t", "lx2");
            json.WriteString("event", name);
            json.WriteNumber("qpc", qpc);
            json.WriteNumber("window", window);
            if (fault is null)
            {
                json.WriteNull("fault");
            }
            else
            {
                json.WriteString("fault", fault);
            }
            fields?.Invoke(json);
        });
    }

    // ---- the injection child's processes (worker threads only: never the main thread) ----

    /// <summary>The full image path of process <paramref name="id"/>, or null.</summary>
    private static string? ImagePath(int id)
    {
        var handle = OpenProcess(ProcessQueryLimitedInformation, false, id);
        if (handle == IntPtr.Zero)
        {
            return null;
        }
        try
        {
            var size = 32_768u;
            var name = new StringBuilder((int)size);
            return QueryFullProcessImageNameW(handle, 0, name, ref size) ? name.ToString(0, (int)size) : null;
        }
        finally
        {
            CloseHandle(handle);
        }
    }

    /// <summary>The live processes whose image is <paramref name="runtime"/>.</summary>
    private static List<Process> Find(string runtime)
    {
        var found = new List<Process>();
        foreach (var process in Process.GetProcessesByName(Path.GetFileNameWithoutExtension(runtime)))
        {
            if (string.Equals(ImagePath(process.Id), runtime, StringComparison.OrdinalIgnoreCase))
            {
                found.Add(process);
            }
            else
            {
                process.Dispose();
            }
        }
        return found;
    }

    /// <summary>Ends every live injection child and waits until each has exited; the number ended.</summary>
    private static int EndChildren(string runtime)
    {
        var ended = 0;
        foreach (var process in Find(runtime))
        {
            using (process)
            {
                try
                {
                    process.Kill();
                    ended++;
                }
                catch (InvalidOperationException)
                {
                    // It exited on its own meanwhile.
                }
                if (!process.WaitForExit(FindMs))
                {
                    throw new TimeoutException($"injection child {process.Id} did not exit within {FindMs} ms");
                }
            }
        }
        return ended;
    }

    /// <summary>The native loop's watcher: finds the armed child, then waits until it exits (the host's
    /// deadline).</summary>
    private ChildExit Watch(string runtime)
    {
        var until = Stopwatch.GetTimestamp() + FindMs * _frequency / 1000;
        Process? child = null;
        while (child is null && Stopwatch.GetTimestamp() < until)
        {
            var found = Find(runtime);
            child = found.Count > 0 ? found[0] : null;
            for (var i = 1; i < found.Count; i++)
            {
                found[i].Dispose();
            }
            if (child is null)
            {
                Thread.Sleep(5);
            }
        }
        if (child is null)
        {
            return new ChildExit(false, 0, null, EventLog.Now());
        }
        using (child)
        {
            // Opens and keeps the process handle, so that its exit code stays readable after it exits.
            _ = child.SafeHandle;
            _childPid = child.Id;
            child.WaitForExit(ExitMs);
            var qpc = EventLog.Now();
            int? code = null;
            try
            {
                if (child.HasExited)
                {
                    code = child.ExitCode;
                }
            }
            catch (InvalidOperationException)
            {
            }
            return new ChildExit(true, child.Id, code, qpc);
        }
    }
}
