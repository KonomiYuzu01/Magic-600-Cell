using System;
using System.Diagnostics;
using System.IO;

namespace Magic600.Shell;

/// <summary>
/// The input echo's log for route B's check B1 (owner decision, 18 Nov 2026): every key press and mouse click that
/// reaches the shell's window while the core's layer shows beneath it, so that the check's result is a record and not
/// only the owner's eye. Format magic600-c9-shell-input-v1 (a DRAFT of C9), one JSON object per line, QPC values in
/// integer ticks, in input.jsonl beside the E2 event log. It is a file of its own because the E2 timer
/// (tools/perf/c9/key_to_frame.py) refuses any row type it does not know.
/// </summary>
/// <remarks>
/// Rows: <c>{"t": "press", "seq", "qpc", "keycode", "label"}</c> for each key press (not a repeat) and
/// <c>{"t": "click", "seq", "qpc", "button", "x", "y"}</c> for each mouse-button press, in window pixels; seq counts
/// each kind from 0. The run row comes first and is written on close, because it carries the totals and whether the
/// window kept the foreground: <c>{"t": "run", "format", "run_id", "build_identity", "qpc_frequency", "synthetic",
/// "foreground_throughout", "process_id", "presses", "clicks"}</c>. The QPC is the shell's receipt of the event in
/// <c>_Input</c>, the earliest point the shell sees. The log is written only when M600_INPUT_ECHO is 1 and
/// M600_EVENTS_DIR and M600_BUILD_IDENTITY are set as for <see cref="EventLog"/>; a shell that never closes leaves only
/// input.body.jsonl.
/// </remarks>
public sealed class InputLog : IDisposable
{
    public const string Format = "magic600-c9-shell-input-v1";
    public const string FileName = "input.jsonl";
    public const string BodyName = "input.body.jsonl";

    private readonly string _directory;
    private readonly string _runId;
    private readonly string _buildIdentity;
    private FileStream? _body;
    private bool _foreground = true;
    private long _presses;
    private long _clicks;

    private InputLog(string directory, string runId, string buildIdentity)
    {
        _directory = directory;
        _runId = runId;
        _buildIdentity = buildIdentity;
        _body = new FileStream(Path.Combine(directory, BodyName), FileMode.CreateNew, FileAccess.Write);
    }

    /// <summary>Whether the environment asks for the input echo (M600_INPUT_ECHO=1).</summary>
    public static bool Requested() => System.Environment.GetEnvironmentVariable("M600_INPUT_ECHO") == "1";

    /// <summary>The log the environment asks for, or null without the echo or without M600_EVENTS_DIR.</summary>
    public static InputLog? FromEnvironment()
    {
        var directory = System.Environment.GetEnvironmentVariable("M600_EVENTS_DIR");
        if (!Requested() || string.IsNullOrEmpty(directory))
        {
            return null;
        }
        if (!Path.IsPathFullyQualified(directory) || !Directory.Exists(directory))
        {
            throw new ArgumentException("M600_EVENTS_DIR must name an existing absolute folder");
        }
        if (File.Exists(Path.Combine(directory, FileName)) || File.Exists(Path.Combine(directory, BodyName)))
        {
            throw new IOException("the events folder already holds an input log; a log is never replaced");
        }
        var build = System.Environment.GetEnvironmentVariable("M600_BUILD_IDENTITY");
        if (string.IsNullOrWhiteSpace(build))
        {
            throw new ArgumentException("M600_BUILD_IDENTITY must name the build the log comes from");
        }
        var run = System.Environment.GetEnvironmentVariable("M600_RUN_ID");
        if (string.IsNullOrWhiteSpace(run))
        {
            run = DateTime.UtcNow.ToString("yyyyMMdd'T'HHmmss'Z'", System.Globalization.CultureInfo.InvariantCulture);
        }
        return new InputLog(directory, run, build);
    }

    /// <summary>Records that the window was not the foreground window at some point of the run.</summary>
    public void LostForeground() => _foreground = false;

    /// <summary>A key press (not a repeat), at the QPC of its receipt.</summary>
    public void Press(long qpc, long keycode, string label)
    {
        var seq = _presses++;
        Write(EventLog.Row(json =>
        {
            json.WriteString("t", "press");
            json.WriteNumber("seq", seq);
            json.WriteNumber("qpc", qpc);
            json.WriteNumber("keycode", keycode);
            json.WriteString("label", label);
        }));
    }

    /// <summary>A mouse-button press at window pixel (x, y), at the QPC of its receipt.</summary>
    public void Click(long qpc, long button, long x, long y)
    {
        var seq = _clicks++;
        Write(EventLog.Row(json =>
        {
            json.WriteString("t", "click");
            json.WriteNumber("seq", seq);
            json.WriteNumber("qpc", qpc);
            json.WriteNumber("button", button);
            json.WriteNumber("x", x);
            json.WriteNumber("y", y);
        }));
    }

    private void Write(byte[] row) => _body?.Write(row);

    /// <summary>Writes input.jsonl: the run row, then every row recorded, and removes the body.</summary>
    public void Dispose()
    {
        if (_body is not { } body)
        {
            return;
        }
        _body = null;
        body.Dispose();
        EventLog.Seal(_directory, FileName, BodyName, EventLog.Row(json =>
        {
            json.WriteString("t", "run");
            json.WriteString("format", Format);
            json.WriteString("run_id", _runId);
            json.WriteString("build_identity", _buildIdentity);
            json.WriteNumber("qpc_frequency", Stopwatch.Frequency);
            json.WriteBoolean("synthetic", false);
            json.WriteBoolean("foreground_throughout", _foreground);
            json.WriteNumber("process_id", System.Environment.ProcessId);
            json.WriteNumber("presses", _presses);
            json.WriteNumber("clicks", _clicks);
        }));
    }
}
