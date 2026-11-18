using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;

public sealed class Arguments
{
    public string out_path = "", dll_path = "";
    public string route = "export", queue = "same", handover = "tracked", barriers = "match";
    public int frames = 1200, warmup = 3, resize_every = 0, verify_every = 50;
    public int device_loss_at = 0, timeout_ms = 5000;
    // Godot consumes --gpu-validation and --render-thread itself; they never reach
    // OS.GetCmdlineArgs(). The runner declares both again as required user arguments.
    public bool gpu_validation;
    public string render_thread = "";
    // Measured by Smoke._Ready on the main thread, never inferred from arguments.
    public bool render_thread_separate_observed;
    public bool Probe => handover == "render-target" || barriers != "match"
        || route == "import-texture2drd" || device_loss_at != 0;

    public static Arguments Parse(string[] values)
    {
        Arguments a = new();
        HashSet<string> seen = new();
        for (int i = 0; i < values.Length; i += 2)
        {
            string key = values[i];
            if (i + 1 >= values.Length || !seen.Add(key)) throw new ArgumentException("missing or repeated argument");
            string value = values[i + 1];
            switch (key)
            {
                case "--sa2-out": a.out_path = value; break;
                case "--sa2-dll": a.dll_path = value; break;
                case "--sa2-route": a.route = Choice(value, "rd-compute", "export", "import-copy", "import-texture2drd"); break;
                case "--sa2-queue": a.queue = Choice(value, "same", "own"); break;
                case "--sa2-handover": a.handover = Choice(value, "tracked", "render-target"); break;
                case "--sa2-barriers": a.barriers = Choice(value, "match", "legacy", "enhanced"); break;
                case "--sa2-frames": a.frames = Number(value, false); break;
                case "--sa2-warmup": a.warmup = Number(value, true); break;
                case "--sa2-resize-every": a.resize_every = Number(value, true); break;
                case "--sa2-verify-every": a.verify_every = Number(value, false); break;
                case "--sa2-device-loss-at": a.device_loss_at = Number(value, true); break;
                case "--sa2-timeout-ms": a.timeout_ms = Number(value, false); break;
                case "--sa2-gpu-validation": a.gpu_validation = Choice(value, "0", "1") == "1"; break;
                case "--sa2-render-thread": a.render_thread = Choice(value, "safe", "separate"); break;
                default: throw new ArgumentException("unknown argument");
            }
        }
        if (!Path.IsPathFullyQualified(a.out_path) || !Path.IsPathFullyQualified(a.dll_path))
            throw new ArgumentException("--sa2-out and --sa2-dll require absolute paths");
        if (!seen.Contains("--sa2-gpu-validation") || !seen.Contains("--sa2-render-thread"))
            throw new ArgumentException("--sa2-gpu-validation and --sa2-render-thread are required");
        bool noWarmup = a.route == "rd-compute" || a.handover == "render-target";
        if (noWarmup && !seen.Contains("--sa2-warmup")) a.warmup = 0;
        if ((noWarmup && a.warmup != 0) || (!noWarmup && a.warmup < 3))
            throw new ArgumentException("warmup must be zero for compute/render-target and at least three for tracked native routes");
        if (a.route == "rd-compute" && a.queue != "same")
            throw new ArgumentException("rd-compute requires the same queue");
        if (a.device_loss_at > a.frames) throw new ArgumentException("device-loss frame exceeds run length");
        return a;
    }

    private static string Choice(string value, params string[] allowed) =>
        Array.IndexOf(allowed, value) >= 0 ? value : throw new ArgumentException("invalid argument value");

    private static int Number(string value, bool zero)
    {
        if (!int.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out int n) || n < (zero ? 0 : 1))
            throw new ArgumentException("invalid numeric argument");
        return n;
    }
}
