using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Text.Json;

public sealed class Level2Size
{
    public int? width, height;
    public Level2Size() { }
    public Level2Size(int w, int h) { width = w; height = h; }
    public bool Same(Level2Size other) => width == other.width && height == other.height;
}

public sealed class Level2Sizes
{
    public Level2Size display = new(), window = new(), backbuffer = new(), displayed = new(), target = new();
    public long samples, samples_changed;
    public bool WindowSettled() => display.width > 0 && display.height > 0
        && display.Same(window) && display.Same(backbuffer) && display.Same(displayed);
    public bool Same(Level2Sizes other) => display.Same(other.display) && window.Same(other.window)
        && backbuffer.Same(other.backbuffer) && displayed.Same(other.displayed) && target.Same(other.target);
}

public sealed class Level2Result
{
    public string format = "magic600-l2-harness-v1", candidate = "sa2", mode, run_id, scene;
    public int process_id = Environment.ProcessId, exit_code = 1;
    public string reason = "not-finished";
    public Dictionary<string, object> options, configuration;
    public Dictionary<string, object> files = new() {
        ["dll"] = null, ["godot:exe"] = null, ["godot:assembly"] = null,
        ["godot:project.godot"] = null, ["godot:Main.tscn"] = null, ["godot:Smoke.cs"] = null
    };
    public object dll_identity;
    public Dictionary<string, object> dll_status = new() { ["abi_version"] = null, ["last_status"] = null, ["last_error"] = null };
    public long? qpc_frequency;
    public Level2Sizes sizes = new();
    public Dictionary<string, object> scaling = new() { ["content_scale_mode"] = null, ["content_scale_factor"] = null, ["texture_stretch"] = null };
    public string dpi_awareness;
    public Dictionary<string, object> environment = new() {
        ["power_source"] = null, ["power_mode"] = null, ["presenting_adapter"] = null, ["presentation_interval"] = null,
        ["declared"] = new Dictionary<string, object>(), ["display"] = new Dictionary<string, object> { ["width"] = null, ["height"] = null, ["refresh_hz"] = null },
        ["backbuffer"] = new Level2Size(), ["power_samples"] = new Dictionary<string, object> { ["samples"] = 0L, ["mains"] = 0L, ["battery"] = 0L },
        ["vsync"] = null, ["adapter"] = null, ["driver"] = null, ["msaa"] = null, ["warp"] = null
    };
    public Dictionary<string, object> window = new() {
        ["topmost"] = null, ["display_required"] = null, ["foreground_at_trace_start"] = null, ["sample_period_ms"] = 100,
        ["samples"] = 0L, ["samples_not_visible"] = 0L, ["samples_covered"] = 0L, ["samples_not_foreground"] = 0L,
        ["visible_throughout"] = null, ["foreground_throughout"] = null, ["presents"] = 0L
    };
    public Dictionary<string, object> debug = new() { ["enabled"] = null, ["debug_layer"] = null, ["counts"] = null, ["messages"] = null };

    public Level2Result(Level2Arguments a)
    {
        mode = a.mode; run_id = a.run_id; scene = a.scene;
        options = new() {
            ["trace_ms"] = a.trace_ms, ["preroll_ms"] = a.preroll_ms, ["turn_ms"] = a.turn_ms, ["inject"] = a.inject,
            ["declared"] = a.declared, ["gpu_validation"] = a.gpu_validation, ["conditions"] = a.conditions,
            ["no_vram"] = a.no_vram, ["debug_half_target"] = a.debug_half_target
        };
        configuration = new() {
            ["framework"] = "godot", ["framework_version"] = null, ["rendering_driver"] = null, ["window_mode"] = null, ["vsync"] = null,
            ["route"] = a.route, ["queue"] = a.queue, ["handover"] = a.handover, ["barriers"] = a.barriers,
            ["render_thread"] = a.render_thread, ["warmup_frames"] = a.warmup, ["engine_arguments"] = null,
            ["scaling_3d_mode"] = null, ["scaling_3d_scale"] = null, ["texture_expand_mode"] = null, ["texture_stretch_mode"] = null
        };
        environment["declared"] = a.declared;
        debug["enabled"] = a.gpu_validation;
        files["dll"] = a.dll_path;
    }

    public static string PowerSource(long samples, long mains, long battery) =>
        samples == 0 ? "unknown" : mains == samples ? "mains" : battery == samples ? "battery"
        : mains + battery == samples && mains > 0 && battery > 0 ? "changed" : "unknown";

    public void Write(string directory)
    {
        string temporary = Path.Combine(directory, "harness.json.tmp"), destination = Path.Combine(directory, "harness.json");
        string json = JsonSerializer.Serialize(this, new JsonSerializerOptions { IncludeFields = true, WriteIndented = true });
        bool created = false;
        try
        {
            using (FileStream stream = new(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None))
            {
                created = true;
                byte[] bytes = Encoding.UTF8.GetBytes(json + "\n");
                stream.Write(bytes);
                stream.Flush(true);
            }
            File.Move(temporary, destination, false);
        }
        finally { if (created && File.Exists(temporary)) File.Delete(temporary); }
    }
}
