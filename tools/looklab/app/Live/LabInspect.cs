using System;
using System.Linq;
using Godot;
using LookLab.Core;
using Magic600.Chrome;

namespace LookLab.App.Live;

// The lab's inspector and bottom index. The View fields are the lab's own parameters and act on both views; the
// selection, the preview parameters, the problems and the task are sample data, because the lab has no engine
// and no runtime.
internal sealed class LabInspect
{
    public static readonly (string Id, string Label)[] ViewFields =
    {
        ("gap", "Cell gap"), ("stickerShrink", "Sticker size"), ("fieldOfView", "Field of view"), ("d4", "4D distance"), ("edgeWeight", "Edge weight"), ("fog", "Fog"),
    };
    public const string SampleTask = "orbit census (sample)";
    public InspectorModel Inspector { get; } = new();
    public FeedbackModel Feedback { get; } = new();
    public InspectorStrip Strip { get; }
    public InspectorPanel Panel { get; }
    // The inspector and, under it, the sections scripts add.
    public VBoxContainer InspectorColumn { get; } = new() { Name = "InspectorColumn" };
    public FeedbackPanel Problems { get; }
    public FeedbackPanel Console { get; }
    public FeedbackPanel Tasks { get; }
    private readonly ChromeFrame frame;
    private TaskItem? running;

    public LabInspect(LabApp app, ChromeFrame frame, LabDemo demo)
    {
        this.frame = frame;
        void Refuse(string why) => frame.Say(why, true);
        int cells = app.Linked.FocusCells;
        Inspector.Select(cells, "cells");
        Inspector.AddFact("selection.cells", "Cells", cells.ToString());
        Inspector.AddFact("selection.slots", "Sticker slots", (cells * Geometry.StickersPerCell).ToString("N0", System.Globalization.CultureInfo.InvariantCulture));
        Inspector.AddFact("selection.all", "Of all slots", "259,800");
        Inspector.AddFact("selection.source", "Chosen by", "the current twist");
        double Default(ParameterDefinition p) => ((NumberValue)p.Default).Value;
        foreach ((string id, string label) in ViewFields)
        {
            ParameterDefinition p = app.State.Schema[id]; double value = app.State.Preset.Params.Number(id);
            Inspector.AddNumber(InspectorModel.View, "view." + id, label, p.Min!.Value, p.Max!.Value, p.Step ?? .01, Default(p), value, false, value == Default(p) ? Provenance.Default : Provenance.Session);
        }
        Inspector.Edited += field =>
        {
            if (field.Binding != null) { frame.Say($"{field.Label} = {field.Binding}: kept as a live binding", true); return; }
            if (field.Section == InspectorModel.View) app.State.Set(field.Id.Substring(5), new NumberValue(field.Value));
            else if (demo.Current >= 0 && demo.Preview.Pending != null) demo.Stage(demo.Current);
        };
        app.State.Changed += (id, value) =>
        {
            if (GodotObject.IsInstanceValid(frame) && value is NumberValue number && ViewFields.Any(f => f.Id == id))
                Inspector.Sync("view." + id, number.Value, number.Value == Default(app.State.Schema[id]) ? Provenance.Default : Provenance.Session);
        };
        // The preview parameters follow what is staged: typed parameters of the staged program, sample values here.
        demo.Preview.Changed += () =>
        {
            bool shown = Inspector.Fields.Any(f => f.Section == InspectorModel.Preview);
            if (demo.Preview.Pending == null) Inspector.Clear(InspectorModel.Preview);
            else if (!shown)
            {
                Inspector.AddNumber(InspectorModel.Preview, "preview.cell", "cell", 0, 599, 1, 117, 117, true);
                Inspector.AddNumber(InspectorModel.Preview, "preview.axis", "axis", 0, 3, 1, 2, 2, true);
                Inspector.AddNumber(InspectorModel.Preview, "preview.turns", "turns", -2, 2, 1, 1, 1, true);
                Inspector.AddNumber(InspectorModel.Preview, "preview.repeat", "repeat", 1, 8, 1, 1, 1, true);
            }
        };
        Strip = new InspectorStrip(Inspector, () => frame.Tokens, Refuse);
        Panel = new InspectorPanel(Inspector, () => frame.Tokens, Refuse);
        Problems = new FeedbackPanel(Feedback, FeedbackPanel.Kind.Problems, () => frame.Tokens);
        Console = new FeedbackPanel(Feedback, FeedbackPanel.Kind.Console, () => frame.Tokens);
        Tasks = new FeedbackPanel(Feedback, FeedbackPanel.Kind.Tasks, () => frame.Tokens);
        Button run = ChromeWidgets.Tool("Run task", "Run a sample task"); run.Name = "RunSampleTask";
        run.Pressed += StartTask; Tasks.Tools.AddChild(run);
        Problems.Opened += p => frame.Say($"{p.Source}:{p.Line}  {p.Text}" + (p.Element != null ? $"  ({p.Element})" : ""), p.Severity != Severity.Info);
        for (int i = 0; i < 3; i++) Feedback.Report(Severity.Error, "unknown word `swep`; did you mean `sweep`?", "sweep.6cs", 12);
        Feedback.Report(Severity.Warning, "this macro moves a protected orbit", "my.6cs", 40, "cell 117");
        Feedback.Report(Severity.Info, "3 definitions are unused", "my.6cs", 1);
        Feedback.Report(Severity.Error, "expected `}` before the end of the file", "draft.6cs", 7);
        foreach (string line in new[] { "> preview my.corner_swap(cell 117)", "prepared 8 moves, 9 pieces move, protection holds", "> check sweep.6cs", "sweep.6cs:12 unknown word `swep`" }) Feedback.Log(line);
        Feedback.Changed += Badges; Feedback.Ended += task => frame.Say($"Task {task.State.ToString().ToLowerInvariant()}: {task.Name}", task.State != TaskState.Ok);
        demo.Moves.Changed += () => { if (Feedback.StateChanged() is > 0 and int n) Feedback.Log($"cancelled {n} task(s) that read the state the commit changed"); };
        Panel.SizeFlagsVertical = Control.SizeFlags.ExpandFill; InspectorColumn.AddChild(Panel); frame.SetContent("inspector", InspectorColumn); frame.SetContent("problems", Problems); frame.SetContent("console", Console); frame.SetContent("tasks", Tasks);
        frame.SetTopStrip(Strip);
        Strip.More += () => frame.Model.ToggleDock(DockSide.Right, "inspector");
        frame.Rebuilt += Rebuilt; frame.Recoloured += Rebuilt;
        var timer = new Timer { WaitTime = .2, Autostart = true, Name = "SampleTaskClock" }; timer.Timeout += () => Tick(.2); frame.AddChild(timer);
        StartTask();
    }

    private void Rebuilt() { Strip.Refresh(); Panel.Refresh(); Problems.Refresh(); Console.Refresh(); Tasks.Refresh(); }

    public void StartTask() { if (running == null || Feedback.Tasks.All(t => t.Id != running.Id || t.State != TaskState.Running)) running = Feedback.Start(SampleTask, "tasks", true); }

    // The sample task takes ten seconds.
    public void Tick(double seconds)
    {
        if (running == null || Feedback.Tasks.FirstOrDefault(t => t.Id == running.Id) is not { State: TaskState.Running } now) return;
        if (now.Progress + seconds / 10 >= 1) Feedback.Finish(now.Id, true); else Feedback.Progress(now.Id, now.Progress + seconds / 10);
    }

    // Count badges on the collapsed bottom tabs, and the quiet status slot (ADR-D01.26, ADR-D01.27).
    private void Badges()
    {
        if (!GodotObject.IsInstanceValid(frame)) return;
        int errors = Feedback.Count(Severity.Error), warnings = Feedback.Count(Severity.Warning);
        frame.SetBadge("problems", errors > 0 ? errors.ToString() : warnings > 0 ? warnings.ToString() : null, errors > 0 ? "severity.error" : "severity.warning",
            $"{errors} errors, {warnings} warnings, {Feedback.Count(Severity.Info)} notes");
        frame.SetBadge("tasks", Feedback.Running > 0 ? Feedback.Running.ToString() : null, "state.pending", Feedback.Running + " running");
        string health = Feedback.Running == 0 ? "" : Feedback.Running == 1 ? "1 task" : Feedback.Running + " tasks";
        if (frame.Health.Text != health) { frame.Health.Text = health; frame.Invalidate(); }
    }
}
