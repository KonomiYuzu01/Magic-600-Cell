using System;
using System.Buffers;
using System.Collections.Generic;
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
/// Key rows (charter E2, a draft): with the turns' session open, every measured press, a twist key's key-down that
/// committed or was refused, gets a key row in press order. A committed press that the host adopted at engine revision
/// R names the first submitted frame that draws revision R over a publication of revision R or later (the conservative
/// reading); a refused press, and a press no frame showed before the close, names no frame, which the timer counts as
/// unconfirmed. Frame rows carry the two revisions as extra fields. The run row's correctness is the session's
/// correctness control (<see cref="HostLibrary.CorrectnessControl"/>), with its record as correctness_detail; without
/// a session it is "fail" with correctness_control "none".
///
/// Swap chains (C9 U3, a draft): the run row's swap_chain is the core's composition swap chain as PresentMon prints its
/// address ("0x" and 16 upper-case hex digits), for key_to_frame.py's cross-check. The shell reads it after the
/// attach, each resize and each recovery; a run that saw more than one chain, or a core that did not give it, writes
/// null, so the cross-check is refused rather than half done. Godot exposes no address of its own swap chain, so
/// ui_swap_chain is null and ui_skew.py has to find the UI chain in the capture as the process's other chain.
///
/// UI frames (route B check B2, ui_skew.py's DRAFT ui_frame row): one row per Godot frame that follows a submitted 3D
/// frame in the same process frame. Its qpc is read at RenderingServer's frame_pre_draw, before Godot draws and
/// presents the frame, so that the UI present lies between the row's qpc and the next row's; drawn_qpc is read at
/// frame_post_draw, which on Godot's RenderingDevice path comes after the present.
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
    /// <summary>Presses without their frame yet, in press order: QPC and revision (0 for a refused press).</summary>
    private readonly Queue<(long Qpc, ulong Revision)> _pending = new();
    private long _keys;
    private long _uiFrames;
    /// <summary>The core's swap chains in the order the shell saw them; null where the core gave no address.</summary>
    private readonly List<ulong?> _swapChains = new();
    private bool _correct;
    private string _control = "none";
    private string? _detail;

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

    /// <summary>
    /// A measured press at <paramref name="qpc"/> (read when the key-down arrived): committed and adopted at
    /// <paramref name="revision"/>, or refused when it is 0. Its key row waits for the frame that shows it.
    /// </summary>
    public void Press(long qpc, ulong revision)
    {
        _pending.Enqueue((qpc, revision));
        Drain(null);
    }

    /// <summary>The session's correctness control: whether it passed and its record (canonical JSON).</summary>
    public void Correctness(bool pass, string control, string detail)
    {
        _correct = pass;
        _control = control;
        _detail = string.IsNullOrEmpty(detail) ? null : detail;
    }

    /// <summary>A frame handed to the core, at the QPC read just before the call.</summary>
    public void Frame(HostLibrary.Submitted frame, long qpc)
    {
        Write(json =>
        {
            json.WriteString("t", "frame");
            json.WriteNumber("frame_id", frame.Sequence);
            json.WriteNumber("qpc", qpc);
            json.WriteNumber("device_epoch", frame.DeviceEpoch);
            json.WriteNumber("surface", frame.Surface);
            json.WriteNumber("engine_revision", frame.EngineRevision);
            json.WriteNumber("publication_revision", frame.PublicationRevision);
        });
        Drain(frame);
    }

    /// <summary>
    /// Writes the key rows that are settled, in press order: refused presses, and committed presses that
    /// <paramref name="frame"/> shows. A press that waits for its frame holds back every later one.
    /// </summary>
    private void Drain(HostLibrary.Submitted? frame)
    {
        while (_pending.TryPeek(out var press))
        {
            ulong? frameId;
            if (press.Revision == 0)
            {
                frameId = null;
            }
            else if (frame is { } f && f.EngineRevision >= press.Revision && f.PublicationRevision >= press.Revision)
            {
                frameId = f.Sequence;
            }
            else
            {
                return;
            }
            _pending.Dequeue();
            WriteKey(press.Qpc, frameId);
        }
    }

    private void WriteKey(long qpc, ulong? frameId)
    {
        var seq = _keys++;
        Write(json =>
        {
            json.WriteString("t", "key");
            json.WriteNumber("seq", seq);
            json.WriteNumber("qpc", qpc);
            if (frameId is { } id)
            {
                json.WriteNumber("frame_id", id);
            }
            else
            {
                json.WriteNull("frame_id");
            }
        });
    }

    /// <summary>The core's swap chain after an attach, a resize or a recovery; null when the core gave no address.</summary>
    public void SwapChain(ulong? address)
    {
        if (_swapChains.Count == 0 || _swapChains[^1] != address)
        {
            _swapChains.Add(address);
        }
    }

    /// <summary>"0x" and 16 upper-case hex digits, as PresentMon prints a swap chain's address (DRAFT).</summary>
    internal static string Address(ulong address) => "0x" + address.ToString("X16", System.Globalization.CultureInfo.InvariantCulture);

    /// <summary>
    /// One Godot frame: <paramref name="qpc"/> read at frame_pre_draw, <paramref name="frameId"/> the 3D frame submitted
    /// in the same process frame, <paramref name="drawnQpc"/> read at frame_post_draw.
    /// </summary>
    public void UiFrame(long qpc, ulong frameId, long drawnQpc)
    {
        var seq = _uiFrames++;
        Write(json =>
        {
            json.WriteString("t", "ui_frame");
            json.WriteNumber("ui_seq", seq);
            json.WriteNumber("qpc", qpc);
            json.WriteNumber("frame_id", frameId);
            json.WriteNumber("drawn_qpc", drawnQpc);
        });
    }

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

    /// <summary>One JSON object of <paramref name="fields"/> and a line feed.</summary>
    internal static byte[] Row(Action<Utf8JsonWriter> fields)
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

    /// <summary>
    /// Writes events.jsonl: the run row, then every row recorded, and removes the body. Presses that no frame showed
    /// get their key rows without a frame first.
    /// </summary>
    public void Dispose()
    {
        if (_body is not { } body)
        {
            return;
        }
        while (_pending.TryDequeue(out var press))
        {
            WriteKey(press.Qpc, null);
        }
        _body = null;
        body.Dispose();
        Seal(_directory, FileName, BodyName, Row(json =>
        {
            json.WriteString("t", "run");
            json.WriteString("format", Format);
            json.WriteString("run_id", _runId);
            json.WriteString("build_identity", _buildIdentity);
            json.WriteNumber("qpc_frequency", Stopwatch.Frequency);
            json.WriteBoolean("synthetic", false);
            json.WriteBoolean("foreground_throughout", _foreground);
            json.WriteString("correctness", _correct ? "pass" : "fail");
            json.WriteString("correctness_control", _control);
            if (_detail is { } detail)
            {
                json.WritePropertyName("correctness_detail");
                json.WriteRawValue(detail);
            }
            json.WriteNumber("process_id", System.Environment.ProcessId);
            if (_swapChains is [{ } chain])
            {
                json.WriteString("swap_chain", Address(chain));
            }
            else
            {
                json.WriteNull("swap_chain");
            }
            json.WriteNull("ui_swap_chain");
        }));
    }

    /// <summary>
    /// Writes <paramref name="fileName"/> in <paramref name="directory"/>: the <paramref name="header"/> row, then the
    /// rows of the closed body file, through a partial file that is renamed only when complete; then removes the body.
    /// </summary>
    internal static void Seal(string directory, string fileName, string bodyName, byte[] header)
    {
        var bodyPath = Path.Combine(directory, bodyName);
        var partial = Path.Combine(directory, fileName + ".partial");
        using (var log = new FileStream(partial, FileMode.CreateNew, FileAccess.Write))
        {
            log.Write(header);
            using var rows = new FileStream(bodyPath, FileMode.Open, FileAccess.Read);
            rows.CopyTo(log);
        }
        File.Move(partial, Path.Combine(directory, fileName));
        File.Delete(bodyPath);
    }
}
