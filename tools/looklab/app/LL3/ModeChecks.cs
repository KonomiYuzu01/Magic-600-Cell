using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using Godot;
using LookLab.Core;

namespace LookLab.App.LL3;

internal static class ModeChecks
{
    public static async Task<bool> Dispatch(Entry host, LabApp app, string root, string scratch, string mode, string fault)
    {
        if (mode is "swipe" or "compare") { ModeControls.Open(app, root, mode == "swipe" ? "Swipe" : "Compare"); return true; }
        if (mode is not ("swipe-check" or "compare-check" or "capture-check")) return false;
        string? temp = System.Environment.GetEnvironmentVariable("LOOKLAB_TEMP_ROOT");
        if (temp == null || Path.GetFullPath(temp) != Path.GetFullPath(scratch)) throw new Exception("LL3 check requires the harness temporary root");
        if (mode == "swipe-check") Swipe(host, app, root, Path.Combine(scratch, "ll3-swipe"), fault);
        else if (mode == "compare-check") Compare(host, app, root, fault);
        else await Capture(host, app, Path.Combine(scratch, "ll3-capture"), fault);
        host.GetTree().Quit();
        return true;
    }

    private static void Require(bool condition, string check, string reason)
    {
        if (!condition) throw new Exception(check + ": " + reason);
    }

    private static void Swipe(Node host, LabApp app, string root, string output, string fault)
    {
        const string check = "swipe-answer-disabled";
        var mode = new SwipeMode(root, output, app.State.Schema, app.State.Preset) { AnswerEnabled = fault != check };
        host.AddChild(mode);
        Preset best = mode.Session.Best, candidate = mode.Session.Candidate;
        string[] ids = mode.Session.ChangedIds.ToArray();
        mode.Answer(true);
        Require(ReferenceEquals(mode.Session.Best, candidate), check, "better did not promote the candidate");
        Verify(1, best, candidate, "better", ids);
        best = mode.Session.Best; candidate = mode.Session.Candidate; ids = mode.Session.ChangedIds.ToArray();
        mode.Answer(false);
        Require(ReferenceEquals(mode.Session.Best, best), check, "worse changed the best");
        Verify(2, best, candidate, "worse", ids);
        host.RemoveChild(mode); mode.QueueFree();
        GD.Print("LOOKLAB_SWIPE_PASS answers=2 records=2");

        void Verify(int count, Preset previous, Preset compared, string verdict, string[] changed)
        {
            string[] lines = File.ReadAllLines(mode.Session.RecordPath);
            Require(lines.Length == count && Directory.GetFiles(Path.GetDirectoryName(mode.Session.RecordPath)!).Length == 1, check, "answer must append exactly one record and nothing else");
            using var document = JsonDocument.Parse(lines[^1]); var record = document.RootElement;
            Require(record.EnumerateObject().Count() == 5 && DateTimeOffset.TryParse(record.GetProperty("time").GetString(), out _), check, "invalid record fields or time");
            Require(record.GetProperty("best_sha256").GetString() == SwipeSession.Digest(previous, app.State.Schema) &&
                record.GetProperty("candidate_sha256").GetString() == SwipeSession.Digest(compared, app.State.Schema) &&
                record.GetProperty("verdict").GetString() == verdict &&
                record.GetProperty("changed_parameter_ids").EnumerateArray().Select(e => e.GetString()).SequenceEqual(changed), check, "record does not describe the compared pair");
        }
    }

    private static void Compare(Node host, LabApp app, string root, string fault)
    {
        const string check = "shared-clock-disabled";
        int samples = 0;
        foreach (int count in new[] { 2, 3, 4 })
        {
            double now = 0;
            Preset first = app.State.Preset;
            Preset[] presets = Enumerable.Range(0, count).Select(i => first with { Name = "Pane " + (i + 1),
                Params = first.Params.With("turnMs", new NumberValue(190 + 20 * i)) }).ToArray();
            var stage = new CompareStage(root, app.State.Schema, presets, () => now);
            stage.Session.ClockEnabled = fault != check;
            host.AddChild(stage); stage.SetProcess(false);
            stage.Session.Camera.Drag(11, -7);
            foreach (double time in new[] { 95.0, 189.999, 190.0, 285.0, 380.0 })
            {
                now = time; stage.Step(); TurnFrame expected = stage.Session.Clock.Frame();
                Require(stage.Viewports.Distinct().Count() == count, check, "each pane needs its own viewport");
                for (int i = 0; i < count; i++)
                {
                    ComparePane pane = stage.Session.Panes[i];
                    Require(ReferenceEquals(pane.Camera, stage.Session.Camera) && ReferenceEquals(pane.Clock, stage.Session.Clock), check, "camera and clock must be shared objects");
                    Require(pane.Frame.Phase == expected.Phase && pane.Frame.BoundRevision == expected.BoundRevision && pane.Frame.Theta == expected.Theta, check, "scripted turn did not reach the shared phase");
                    Require(stage.Views[i].Q.SequenceEqual(stage.Session.Camera.Q) &&
                        Math.Abs(stage.Views[i].Material.GetShaderParameter("theta").AsDouble() - expected.Theta) < 1e-6 &&
                        stage.Views[i].Material.GetShaderParameter("q").AsProjection() == LookViewport.Matrix(stage.Session.Camera.Q), check, "view did not bind the shared camera and turn");
                    samples++;
                }
            }
            host.RemoveChild(stage); stage.QueueFree();
        }
        GD.Print($"LOOKLAB_COMPARE_PASS counts=2,3,4 samples={samples} camera=shared clock=shared");
    }

    private static async Task Capture(Node host, LabApp app, string output, string fault)
    {
        host.GetWindow().Mode = Window.ModeEnum.Windowed; host.GetWindow().Show();
        string driver = RenderingServer.GetCurrentRenderingDriverName().ToString();
        string display = DisplayServer.GetName(), adapter = RenderingServer.GetVideoAdapterName();
        GD.Print("LOOKLAB_GRAPHICS " + JsonSerializer.Serialize(new { driver, display, adapter }));
        Require(display.ToLowerInvariant() == "windows" && (driver is "vulkan" or "d3d12") && adapter.Trim().Length > 0,
            "graphics-environment", "Windows display and real driver required");
        await host.ToSignal(host.GetTree(), SceneTree.SignalName.ProcessFrame);
        Require(host.GetWindow().Visible && host.GetWindow().Mode != Window.ModeEnum.Minimized, "graphics-environment", "capture window must remain visible");
        var service = new CaptureService(host, output) { Enabled = fault != "capture-handler-disabled" };
        var viewport = (SubViewport)app.View.GetViewport();
        string? still = await service.Still(viewport);
        Require(still != null && File.Exists(still), "capture-handler-disabled", "capture handler produced no still");
        using Image image = Image.LoadFromFile(still!);
        Require(!image.IsEmpty() && image.GetWidth() == viewport.Size.X && image.GetHeight() == viewport.Size.Y,
            "capture-handler-disabled", "still is not a PNG of the viewport size");
        string? clip = service.Ffmpeg == null ? null : await service.Clip(viewport, 1);
        Require(service.Ffmpeg == null || (clip != null && File.Exists(clip)), "capture-handler-disabled", "clip handler produced no clip");
        GD.Print("LOOKLAB_CAPTURE_PASS " + JsonSerializer.Serialize(new
        {
            width = viewport.Size.X, height = viewport.Size.Y, still = Path.GetFileName(still),
            clip = clip == null ? null : Path.GetFileName(clip), frames = clip == null ? 0 : CaptureEncoding.FrameRate
        }));
    }
}
