using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using LookLab.Core;
using LookLab.App.Platform;

namespace LookLab.App.LL3;

internal static class ModeControls
{
    public static void Attach(LabApp app, string root, VBoxContainer column)
    {
        var row = new HBoxContainer(); column.AddChild(row);
        foreach (string mode in new[] { "Swipe", "Compare", "Capture" })
        {
            var button = new Button { Text = mode, SizeFlagsHorizontal = Control.SizeFlags.ExpandFill }; row.AddChild(button);
            button.Pressed += () => Open(app, root, mode);
        }
    }

    public static void Open(LabApp app, string root, string mode)
    {
        var window = new Window { Title = "Look Lab " + mode.ToLowerInvariant(), Size = new Vector2I(1000, 580), Transient = true };
        app.AddChild(window);
        var column = new VBoxContainer(); column.SetAnchorsAndOffsetsPreset(Control.LayoutPreset.FullRect); window.AddChild(column);
        var status = new Label { AutowrapMode = TextServer.AutowrapMode.WordSmart }; column.AddChild(status);
        try
        {
            if (mode == "Swipe") column.AddChild(new SwipeMode(root, root, app.State.Schema, app.State.Preset, app));
            else if (mode == "Compare") column.AddChild(new CompareMode(root, app.State.Schema, app));
            else { window.Size = new Vector2I(480, 180); column.AddChild(new CaptureControls(app, root, () => (SubViewport)app.View.GetViewport())); }
        }
        catch (Exception ex) { status.Text = ex.Message; }
        window.CloseRequested += () => window.QueueFree();
        window.PopupCentered();
    }
}

internal sealed partial class SwipeMode : VBoxContainer
{
    public SwipeSession Session { get; private set; }
    public CompareStage Stage { get; }
    public bool AnswerEnabled { get; set; } = true;
    private readonly string outputRoot;
    private readonly ParameterSchema schema;
    private readonly Label status;
    private readonly SpinBox seed;

    public SwipeMode(string root, string outputRoot, ParameterSchema schema, Preset best, LabApp? app = null)
    {
        this.outputRoot = outputRoot; this.schema = schema;
        SizeFlagsHorizontal = SizeFlags.ExpandFill; SizeFlagsVertical = SizeFlags.ExpandFill;
        Session = new SwipeSession(outputRoot, schema, best, 1);
        var row = new HBoxContainer(); AddChild(row);
        row.AddChild(new Label { Text = "Seed" });
        seed = new SpinBox { MinValue = 0, MaxValue = int.MaxValue, Step = 1, Value = 1 }; row.AddChild(seed);
        var restart = new Button { Text = "Restart from current best" }; row.AddChild(restart);
        var better = new Button { Text = "Candidate is better" }; row.AddChild(better);
        var worse = new Button { Text = "Candidate is worse" }; row.AddChild(worse);
        status = new Label { AutowrapMode = TextServer.AutowrapMode.WordSmart }; AddChild(status);
        Stage = new CompareStage(root, schema, new[] { Session.Best, Session.Candidate }, sharedTiming: false); AddChild(Stage);
        better.Pressed += () => Guard(() => Answer(true)); worse.Pressed += () => Guard(() => Answer(false));
        restart.Pressed += () => Guard(() => { Session = new SwipeSession(this.outputRoot, this.schema, Session.Best, (int)seed.Value); Refresh(); });
        if (app != null)
        {
            var useBest = new Button { Text = "Use best in editor" }; row.AddChild(useBest);
            useBest.Pressed += () => Guard(() => app.State.Load(Session.Best));
            AddChild(new CaptureControls(app, outputRoot, () => Stage.Viewports[0]));
        }
        Refresh();
    }

    public void Answer(bool better)
    {
        if (!AnswerEnabled) return;
        Session.Answer(better); Refresh();
    }

    private void Refresh()
    {
        Stage.UpdatePresets(new[] { Session.Best, Session.Candidate });
        status.Text = "Left: current best. Right: candidate. Changed: " + string.Join(", ", Session.ChangedIds) + ".";
    }

    private void Guard(Action action) { try { action(); } catch (Exception ex) { status.Text = ex.Message; } }
}

internal sealed partial class CompareMode : VBoxContainer
{
    private CompareStage? stage;
    private readonly List<OptionButton> choices = new();

    public CompareMode(string root, ParameterSchema schema, LabApp app)
    {
        SizeFlagsVertical = SizeFlags.ExpandFill;
        var presets = new List<Preset>();
        foreach (string file in Directory.EnumerateFiles(Path.Combine(root, "tools/looklab/presets"), "*.json").OrderBy(p => p, StringComparer.Ordinal))
            presets.Add(Presets.Load(file, schema));
        var owner = new PresetFiles(root, schema);
        if (owner.Exists) foreach (string file in Directory.EnumerateFiles(owner.Folder, "*.json").OrderBy(p => p, StringComparer.Ordinal)) presets.Add(owner.Load(file));
        if (presets.Count == 0) throw new FormatException("compare: no presets available");
        var row = new HBoxContainer(); AddChild(row); row.AddChild(new Label { Text = "Panes" });
        var count = new SpinBox { MinValue = 2, MaxValue = 4, Step = 1, Value = 2 }; row.AddChild(count);
        for (int i = 0; i < 4; i++)
        {
            var choice = new OptionButton { SizeFlagsHorizontal = SizeFlags.ExpandFill };
            foreach (Preset preset in presets) choice.AddItem(preset.Name);
            choice.Select(Math.Min(i, presets.Count - 1)); row.AddChild(choice); choices.Add(choice);
            choice.ItemSelected += _ => Rebuild();
        }
        AddChild(new Label { Text = "Drag any pane to rotate all. Shared turn timing and easing use the first selected preset." });
        count.ValueChanged += _ => Rebuild();
        Rebuild();
        var pane = new OptionButton();
        for (int i = 0; i < 4; i++) pane.AddItem("Capture pane " + (i + 1));
        AddChild(pane);
        AddChild(new CaptureControls(app, root, () => stage!.Viewports[Math.Min(pane.Selected, stage.Viewports.Count - 1)]));

        void Rebuild()
        {
            int n = (int)count.Value;
            for (int i = 0; i < choices.Count; i++) choices[i].Visible = i < n;
            double[]? q = stage?.Session.Camera.Q.ToArray();
            if (stage != null) { RemoveChild(stage); stage.QueueFree(); }
            stage = new CompareStage(root, schema, choices.Take(n).Select(c => presets[c.Selected]).ToArray());
            if (q != null) Array.Copy(q, stage.Session.Camera.Q, 16);
            AddChild(stage); MoveChild(stage, 2);
        }
    }
}

internal sealed partial class CaptureControls : HBoxContainer
{
    public CaptureControls(Node host, string root, Func<SubViewport> viewport)
    {
        var service = new CaptureService(host, root);
        var still = new Button { Text = "Save PNG" }; AddChild(still);
        var seconds = new SpinBox { MinValue = 1, MaxValue = 10, Step = 1, Value = 1, Suffix = "s" }; AddChild(seconds);
        var clip = new Button { Text = "Save clip", Disabled = service.Ffmpeg == null }; AddChild(clip);
        var status = new Label { Text = service.Ffmpeg == null ? "Clip unavailable: FFmpeg is not installed locally." : "Clips: 24 fps, at most 10 seconds.",
            AutowrapMode = TextServer.AutowrapMode.WordSmart, SizeFlagsHorizontal = SizeFlags.ExpandFill }; AddChild(status);
        still.Pressed += async () => await Capture(false); clip.Pressed += async () => await Capture(true);

        async Task Capture(bool video)
        {
            still.Disabled = true; clip.Disabled = true;
            status.Text = video ? "Capturing clip..." : "Capturing still...";
            try
            {
                string? path = video ? await service.Clip(viewport(), seconds.Value) : await service.Still(viewport());
                if (GodotObject.IsInstanceValid(status)) status.Text = "Saved to captures and the workbench gallery: " + Path.GetFileName(path);
            }
            catch (Exception ex) { if (GodotObject.IsInstanceValid(status)) status.Text = ex.Message; }
            finally
            {
                if (GodotObject.IsInstanceValid(still)) still.Disabled = false;
                if (GodotObject.IsInstanceValid(clip)) clip.Disabled = service.Ffmpeg == null;
            }
        }
    }
}
