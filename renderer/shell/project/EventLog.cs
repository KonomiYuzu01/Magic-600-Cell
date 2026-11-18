using System;
using System.Buffers;
using System.Diagnostics;
using System.IO;
using System.Text.Json;

namespace Magic600.Shell;

/// <summary>
/// The shell's event log for charter E2, step S5 of the shell (a draft on route B, accepted for now by the owner on
/// 18 Nov 2026, pending three owner-machine checks). The format is tools/perf/c9's DRAFT magic600-c9-shell-events-v1,
/// which key_to_frame.py reads: one JSON object per line, QPC values in integer ticks.
/// </summary>
/// <remarks>
/// The log is written only when M600_EVENTS_DIR names an absolute folder that holds no log yet, and M600_BUILD_IDENTITY
/// names the build. M600_RUN_ID names the run (default: the UTC start time). The shell writes a frame row for every
/// frame it submits and a presented row for every presentation the core confirms. On close it writes events.jsonl,
/// with the run row first, because the run row's foreground_throughout is known only at the end. A shell that never
/// closes leaves only events.body.jsonl, which the timer does not read.
///
/// What the log cannot yet say: there is no turn without the command layer, so there are no key rows; there is no
/// correctness control, so correctness is "fail" with correctness_control "none", and the timer marks the run invalid;
/// the core does not report its swap chain's address, so there is no swap_chain and no PresentMon cross-check.
/// </remarks>
public sealed class EventLog : IDisposable
{
    public const string Format = "magic600-c9-shell-events-v1";
    public const string FileName = "events.jsonl";
    public const string BodyName = "events.body.jsonl";

    private readonly string _directory;
    private readonly string _runId;
    private readonly string _buildIdentity;
    private FileStream? _body;
    private bool _foreground = true;

    private EventLog(string directory, string runId, string buildIdentity)
    {
        _directory = directory;
        _runId = runId;
        _buildIdentity = buildIdentity;
        _body = new FileStream(Path.Combine(directory, BodyName), FileMode.CreateNew, FileAccess.Write);
    }

    /// <summary>The log the environment asks for, or null when M600_EVENTS_DIR is not set.</summary>
    public static EventLog? FromEnvironment()
    {
        var directory = System.Environment.GetEnvironmentVariable("M600_EVENTS_DIR");
        if (string.IsNullOrEmpty(directory))
        {
            return null;
        }
        if (!Path.IsPathFullyQualified(directory) || !Directory.Exists(directory))
        {
            throw new ArgumentException("M600_EVENTS_DIR must name an existing absolute folder");
        }
        if (File.Exists(Path.Combine(directory, FileName)) || File.Exists(Path.Combine(directory, BodyName)))
        {
            throw new IOException("the events folder already holds a log; a log is never replaced");
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
        return new EventLog(directory, run, build);
    }

    /// <summary>The machine's QueryPerformanceCounter, which Stopwatch reads on Windows.</summary>
    public static long Now() => Stopwatch.GetTimestamp();

    /// <summary>Records that the window was not the foreground window at some point of the run.</summary>
    public void LostForeground() => _foreground = false;

    /// <summary>A frame handed to the core, at the QPC read just before the call.</summary>
    public void Frame(HostLibrary.Submitted frame, long qpc) => Write(json =>
    {
        json.WriteString("t", "frame");
        json.WriteNumber("frame_id", frame.Sequence);
        json.WriteNumber("qpc", qpc);
        json.WriteNumber("device_epoch", frame.DeviceEpoch);
        json.WriteNumber("surface", frame.Surface);
    });

    /// <summary>A presentation the core confirmed. The sequence is the frame's, as the core's presented row carries it.</summary>
    public void Presented(HostLibrary.Presented row) => Write(json =>
    {
        json.WriteString("t", "presented");
        json.WriteNumber("frame_id", row.Sequence);
        json.WriteNumber("device_epoch", row.DeviceEpoch);
        json.WriteNumber("surface", row.Surface);
        json.WriteNumber("sequence", row.Sequence);
        json.WriteNumber("display_qpc", row.DisplayQpc);
    });

    private void Write(Action<Utf8JsonWriter> fields)
    {
        if (_body is not { } body)
        {
            return;
        }
        body.Write(Row(fields));
    }

    private static byte[] Row(Action<Utf8JsonWriter> fields)
    {
        var buffer = new ArrayBufferWriter<byte>();
        using (var json = new Utf8JsonWriter(buffer))
        {
            json.WriteStartObject();
            fields(json);
            json.WriteEndObject();
        }
        var row = new byte[buffer.WrittenCount + 1];
        buffer.WrittenSpan.CopyTo(row);
        row[^1] = (byte)'\n';
        return row;
    }

    /// <summary>Writes events.jsonl: the run row, then every row recorded, and removes the body.</summary>
    public void Dispose()
    {
        if (_body is not { } body)
        {
            return;
        }
        _body = null;
        body.Dispose();
        var bodyPath = Path.Combine(_directory, BodyName);
        var partial = Path.Combine(_directory, FileName + ".partial");
        using (var log = new FileStream(partial, FileMode.CreateNew, FileAccess.Write))
        {
            log.Write(Row(json =>
            {
                json.WriteString("t", "run");
                json.WriteString("format", Format);
                json.WriteString("run_id", _runId);
                json.WriteString("build_identity", _buildIdentity);
                json.WriteNumber("qpc_frequency", Stopwatch.Frequency);
                json.WriteBoolean("synthetic", false);
                json.WriteBoolean("foreground_throughout", _foreground);
                json.WriteString("correctness", "fail");
                json.WriteString("correctness_control", "none");
                json.WriteNumber("process_id", System.Environment.ProcessId);
            }));
            using var rows = new FileStream(bodyPath, FileMode.Open, FileAccess.Read);
            rows.CopyTo(log);
        }
        File.Move(partial, Path.Combine(_directory, FileName));
        File.Delete(bodyPath);
    }
}
