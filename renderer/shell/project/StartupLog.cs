using System;
using System.IO;

namespace Magic600.Shell;

/// <summary>
/// The shell's part of charter E3's startup log (C9 U6, a draft that was never launched): rows of tools/perf/c9's DRAFT
/// magic600-c9-startup-v1, which cold_start.py reads. The shell writes them to startup.body.jsonl in M600_EVENTS_DIR as
/// they happen, each flushed at once; after the shell exits, the launcher (launch.py) writes startup.jsonl from its own
/// run and process_start rows and this body, and removes the body.
/// </summary>
/// <remarks>
/// session_opened is read when m6h_open_session has answered, runtime_loaded when m6h_load_runtime has answered (the
/// runtime's hello arrived). first_correct_frame names the first confirmed presentation of a frame that draws the
/// revision the session opened at over a publication of that revision or later, with its display time; it is written at
/// the close, with check from the session's correctness control (pass or fail), whose shown revision is that frame's
/// when no key was pressed (draft).
/// </remarks>
public sealed class StartupLog : IDisposable
{
    public const string BodyName = "startup.body.jsonl";

    private FileStream? _body;

    private StartupLog(string directory)
    {
        _body = new FileStream(Path.Combine(directory, BodyName), FileMode.CreateNew, FileAccess.Write);
    }

    /// <summary>The log for M600_EVENTS_DIR, or null when it is not set; the folder holds no startup log yet.</summary>
    public static StartupLog? FromEnvironment()
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
        return new StartupLog(directory);
    }

    public void SessionOpened(long qpc) => Event("session_opened", qpc);

    public void RuntimeLoaded(long qpc) => Event("runtime_loaded", qpc);

    public void FirstCorrectFrame(ulong frameId, ulong displayQpc, bool pass) => Write(json =>
    {
        json.WriteString("t", "first_correct_frame");
        json.WriteNumber("frame_id", frameId);
        json.WriteNumber("display_qpc", displayQpc);
        json.WriteString("check", pass ? "pass" : "fail");
    });

    private void Event(string kind, long qpc) => Write(json =>
    {
        json.WriteString("t", kind);
        json.WriteNumber("qpc", qpc);
    });

    private void Write(Action<System.Text.Json.Utf8JsonWriter> fields)
    {
        if (_body is not { } body)
        {
            return;
        }
        body.Write(EventLog.Row(fields));
        body.Flush();
    }

    public void Dispose()
    {
        _body?.Dispose();
        _body = null;
    }
}
