using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;

public sealed class RunResult
{
    public string format = "magic600-sa2-smoke-run-v1", status = "pass";
    public List<string> reasons = new();
    public Arguments config;
    public Dictionary<string, object> engine = new(), device = new(), dll = new();
    public FrameStats frames = new();
    public VerifyStats verify = new();
    public ResizeStats resize = new();
    public Dictionary<string, object> texture2drd_probe = new(), device_loss = new();
    public Dictionary<string, object> debug = new();
    public TeardownStats teardown = new();
    public List<Sa2Failure> sa2_failures = new();
    private readonly HashSet<long> _mismatchedFrames = new();

    public RunResult(Arguments args) { config = args; if (args.Probe) status = "recorded"; }
    public void Reason(string reason) { if (!reasons.Contains(reason)) reasons.Add(reason); }

    public void Mismatch(long frame, ulong expected, ulong? decoded, string reason)
    {
        if (_mismatchedFrames.Add(frame)) frames.mismatched++;
        if (frames.first_mismatch == null || frame < Convert.ToInt64(frames.first_mismatch["frame"]))
            frames.first_mismatch = new Dictionary<string, object> {
                ["frame"] = frame, ["expected_code"] = $"0x{expected:X16}",
                ["decoded_code"] = decoded.HasValue ? $"0x{decoded:X16}" : null
            };
        Reason(reason);
        if (!config.Probe) status = "fail";
    }

    public string Json() => JsonSerializer.Serialize(this, new JsonSerializerOptions { IncludeFields = true, WriteIndented = true });
    public static void WriteAtomic(string path, string json)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path));
        string temporary = path + ".tmp";
        File.WriteAllText(temporary, json);
        File.Move(temporary, path, true);
    }
}

public sealed class FrameStats
{
    public long run, produced, drawn, not_drawn, eligible, verified, mismatched;
    public long readbacks_requested, readbacks_completed;
    public Dictionary<string, long> skipped = new() { ["warmup"] = 0, ["transition"] = 0 };
    public Dictionary<string, object> first_mismatch;
}

public sealed class VerifyStats
{
    public long native_checks, rd_checks;
    public ulong native_mismatched_texels, rd_mismatched_texels, mismatched_texels;
}

public sealed class ResizeStats
{
    public int rebuilds;
    public List<int[]> sizes = new();
    public List<Dictionary<string, long>> transitions = new();
    public long longest_transition;
}

public sealed class TeardownStats
{
    public string phase = "not-started";
    public int? drain_result;
    public List<int> rebuild_drain_results = new();
    public List<Dictionary<string, object>> refcount_after = new();
    public int slots_unregistered;
    public bool detached;
}

public sealed class Sa2Failure
{
    public string function, last_error;
    public int status;
}
