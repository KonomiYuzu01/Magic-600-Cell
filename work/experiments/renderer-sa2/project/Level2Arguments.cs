using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text.RegularExpressions;

public sealed class Level2Arguments
{
    public string mode, scene, out_path, run_id, dll_path, inject;
    public int trace_ms = 192000, preroll_ms = 4000;
    public double turn_ms = 190;
    public bool gpu_validation, no_vram, debug_half_target;
    public string conditions = "enforce";
    public string route = "export", queue = "same", handover = "tracked", barriers = "match", render_thread = "safe";
    public int warmup = 3, timeout_ms = 5000;
    public Dictionary<string, object> declared = new();
    public static readonly string[] Outputs = { "harness.json", "harness.json.tmp", "native.json", "trace.jsonl", "geometry.json" };

    public static bool Requested(string[] values) => Array.Exists(values, v => v.StartsWith("--l2-", StringComparison.Ordinal));

    public static bool OutputUsable(string path) => !string.IsNullOrEmpty(path) && Directory.Exists(path)
        && Array.TrueForAll(Outputs, name => !File.Exists(Path.Combine(path, name)) && !Directory.Exists(Path.Combine(path, name)));

    // Recover only an unambiguous output directory for a usage result. Never select
    // the first of two --l2-out values or overwrite another invocation's evidence.
    public static string UsageOutput(string[] values)
    {
        int index = Array.IndexOf(values, "--l2-out");
        if (index < 0 || index + 1 >= values.Length || Array.LastIndexOf(values, "--l2-out") != index) return null;
        string path = values[index + 1];
        return OutputUsable(path) ? Path.GetFullPath(path) : null;
    }

    public static Level2Arguments Parse(string[] values)
    {
        Level2Arguments a = new();
        HashSet<string> seen = new();
        for (int i = 0; i < values.Length; i++)
        {
            string key = values[i];
            if (key != "--l2-declare" && !seen.Add(key)) throw new ArgumentException("repeated argument");
            if (key == "--l2-no-vram") { a.no_vram = true; continue; }
            if (key == "--l2-debug-half-target") { a.debug_half_target = true; continue; }
            if (++i >= values.Length) throw new ArgumentException("missing argument value");
            string value = values[i];
            switch (key)
            {
                case "--l2-mode": a.mode = Choice(value, "run", "geometry"); break;
                case "--l2-scene": a.scene = Choice(value, "w1", "w2", "w3", "w4"); break;
                case "--l2-out": a.out_path = value; break;
                case "--l2-run-id": a.run_id = value; break;
                case "--l2-dll": a.dll_path = value; break;
                case "--l2-trace-ms": a.trace_ms = Number(value, 1000, 3600000); break;
                case "--l2-preroll-ms": a.preroll_ms = Number(value, 0, 60000); break;
                case "--l2-turn-ms":
                    if (!double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out a.turn_ms)
                        || !double.IsFinite(a.turn_ms) || a.turn_ms <= 0 || a.turn_ms > 10000)
                        throw new ArgumentException("invalid turn length");
                    break;
                case "--l2-inject": a.inject = Choice(value, "corrupt-label", "swap-same-colour", "delay-adoption", "stale-binding"); break;
                case "--l2-declare":
                    int equal = value.IndexOf('=');
                    if (equal <= 0 || value[..equal] == "overlays" || a.declared.ContainsKey(value[..equal]))
                        throw new ArgumentException("invalid or repeated declaration");
                    string declaredValue = value[(equal + 1)..];
                    a.declared.Add(value[..equal], declaredValue == "true" ? true : declaredValue == "false" ? false : (object)declaredValue);
                    break;
                case "--l2-gpu-validation": a.gpu_validation = Choice(value, "0", "1") == "1"; break;
                case "--l2-conditions": a.conditions = Choice(value, "enforce", "record"); break;
                case "--sa2-route": a.route = Choice(value, "export", "import-copy", "import-texture2drd"); break;
                case "--sa2-queue": a.queue = Choice(value, "same", "own"); break;
                case "--sa2-handover": a.handover = Choice(value, "tracked", "render-target"); break;
                case "--sa2-barriers": a.barriers = Choice(value, "match", "legacy", "enhanced"); break;
                case "--sa2-render-thread": a.render_thread = Choice(value, "safe", "separate"); break;
                case "--sa2-warmup": a.warmup = Number(value, 0, int.MaxValue); break;
                case "--sa2-timeout-ms": a.timeout_ms = Number(value, 5000, 5000); break;
                case "--sa2-gpu-validation":
                    Choice(value, "0", "1"); // Checked against the level 2 option below.
                    break;
                default: throw new ArgumentException("unknown or inapplicable level 2 argument");
            }
        }
        if (a.mode == null || a.out_path == null || a.run_id == null || a.dll_path == null
            || (a.mode == "run" && a.scene == null)) throw new ArgumentException("missing required level 2 argument");
        if (!Regex.IsMatch(a.run_id, @"\A[A-Za-z0-9][A-Za-z0-9._-]{0,127}\z")) throw new ArgumentException("invalid run ID");
        if (!Path.IsPathFullyQualified(a.dll_path) || !File.Exists(a.dll_path)
            || !string.Equals(Path.GetFileName(a.dll_path), "sa2_interop.dll", StringComparison.OrdinalIgnoreCase))
            throw new ArgumentException("--l2-dll requires the absolute path of sa2_interop.dll");
        if (!OutputUsable(a.out_path)) throw new ArgumentException("--l2-out requires an existing directory without app/DLL outputs");
        a.out_path = Path.GetFullPath(a.out_path);
        if (seen.Contains("--l2-turn-ms") && a.scene != "w3") throw new ArgumentException("turn length is W3 only");
        if (a.inject != null && (a.mode != "run" || (a.scene != "w3" && a.scene != "w4")))
            throw new ArgumentException("injection is W3/W4 run only");
        if (seen.Contains("--sa2-gpu-validation"))
        {
            int index = Array.IndexOf(values, "--sa2-gpu-validation");
            if ((values[index + 1] == "1") != a.gpu_validation) throw new ArgumentException("validation options disagree");
        }
        bool noWarmup = a.handover == "render-target";
        if (noWarmup && !seen.Contains("--sa2-warmup")) a.warmup = 0;
        if ((noWarmup && a.warmup != 0) || (!noWarmup && a.warmup < 3))
            throw new ArgumentException("warmup must be zero for render-target and at least three for tracked routes");
        return a;
    }

    private static string Choice(string value, params string[] allowed) =>
        Array.IndexOf(allowed, value) >= 0 ? value : throw new ArgumentException("invalid argument value");

    private static int Number(string value, int minimum, int maximum)
    {
        if (!int.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out int n) || n < minimum || n > maximum)
            throw new ArgumentException("out-of-range numeric argument");
        return n;
    }
}
