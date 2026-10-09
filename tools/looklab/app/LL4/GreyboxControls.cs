using System;
using System.IO;
using System.Linq;
using Godot;
using LookLab.Core;

namespace LookLab.App.LL4;

internal sealed partial class GreyboxControls : VBoxContainer
{
    private readonly LabApp app;
    private readonly SubViewportContainer stage;
    private readonly PanelContainer panel;
    private readonly VBoxContainer controls;
    private readonly FlowReportFiles files;
    public CommandCatalogue Catalogue { get; }
    public LayoutSpec[] Layouts { get; }
    public FlowScript[] Flows { get; }
    public GreyboxCanvas Canvas { get; }
    public FlowRunner Runner { get; } = new();
    public CheckButton Toggle { get; } = new() { Text = "Greybox and flows" };
    public OptionButton LayoutChoice { get; } = new() { ClipText = true, FitToLongestItem = false };
    public OptionButton ContextChoice { get; } = new() { ClipText = true, FitToLongestItem = false };
    public OptionButton FlowChoice { get; } = new() { ClipText = true, FitToLongestItem = false };
    public SpinBox MetricWidth { get; } = new() { MinValue = 1, MaxValue = 16384, Value = 1280 };
    public SpinBox MetricHeight { get; } = new() { MinValue = 1, MaxValue = 16384, Value = 720 };
    public Button RunButton { get; } = new() { Text = "Run flow" };
    public Button NextButton { get; } = new() { Text = "Next step", Disabled = true };
    public Button SaveButton { get; } = new() { Text = "Save report", Disabled = true };
    public Label Message { get; } = new() { AutowrapMode = TextServer.AutowrapMode.WordSmart };
    public Label ReportText { get; } = new();
    public LayoutSpec CurrentLayout => Layouts[LayoutChoice.Selected];

    public static void Attach(LabApp app, SubViewportContainer stage, PanelContainer panel, VBoxContainer column, string root)
    {
        var controls = new GreyboxControls(app, stage, panel, root) { Name = "LL4Controls" };
        column.AddChild(controls); column.MoveChild(controls, 0);
    }

    private GreyboxControls(LabApp app, SubViewportContainer stage, PanelContainer panel, string root)
    {
        this.app = app; this.stage = stage; this.panel = panel;
        files = new FlowReportFiles(root);
        Catalogue = CommandCatalogue.Load(Path.Combine(root, "docs/progress/1.0/command-table.json"));
        Layouts = Directory.GetFiles(Path.Combine(root, "tools/looklab/data/layouts"), "*.json")
            .OrderBy(p => p, StringComparer.Ordinal).Select(p => LayoutSpec.Load(p, Catalogue)).ToArray();
        Flows = Directory.GetFiles(Path.Combine(root, "tools/looklab/data/flows"), "*.json")
            .OrderBy(p => p, StringComparer.Ordinal).Select(p => FlowScript.Load(p, Catalogue)).ToArray();
        Canvas = new GreyboxCanvas(Catalogue) { Visible = false }; app.AddChild(Canvas);
        AddChild(Toggle);
        controls = new VBoxContainer { Visible = false }; AddChild(controls);
        foreach (LayoutSpec layout in Layouts) LayoutChoice.AddItem(layout.Id);
        LayoutChoice.Select(Array.FindIndex(Layouts, l => l.Id == app.State.Preset.Params.Choice("layoutId")));
        foreach (string context in Catalogue.Contexts) ContextChoice.AddItem(context);
        foreach (FlowScript flow in Flows) FlowChoice.AddItem(flow.Name + (flow.Draft ? " (draft)" : ""));
        controls.AddChild(LayoutChoice); controls.AddChild(ContextChoice); controls.AddChild(FlowChoice);
        controls.AddChild(new Label { Text = "Model window (pixels)" });
        var dimensions = new HBoxContainer(); controls.AddChild(dimensions);
        dimensions.AddChild(MetricWidth); dimensions.AddChild(new Label { Text = "x" }); dimensions.AddChild(MetricHeight);
        var buttons = new HBoxContainer(); controls.AddChild(buttons);
        buttons.AddChild(RunButton); buttons.AddChild(NextButton); buttons.AddChild(SaveButton);
        controls.AddChild(Message);
        var reportScroll = new ScrollContainer { CustomMinimumSize = new Vector2(0, 110) };
        controls.AddChild(reportScroll); reportScroll.AddChild(ReportText);
        Toggle.Toggled += active => { controls.Visible = active; Canvas.Visible = active; ResizeViews(); };
        LayoutChoice.ItemSelected += index => app.State.Set("layoutId", new EnumValue(Layouts[(int)index].Id));
        ContextChoice.ItemSelected += _ => { ClearReport(); RefreshCanvas(); };
        FlowChoice.ItemSelected += _ => ClearReport();
        MetricWidth.ValueChanged += _ => ClearReport(); MetricHeight.ValueChanged += _ => ClearReport();
        RunButton.Pressed += () => StartFlow(Flows[FlowChoice.Selected], CurrentLayout);
        NextButton.Pressed += () => { Runner.Advance(); RefreshReport(); };
        SaveButton.Pressed += SaveReport;
        app.State.Changed += Change;
        app.Resized += ResizeViews;
        RefreshCanvas();
    }

    public bool StartFlow(FlowScript flow, LayoutSpec layout)
    {
        bool accepted = Runner.Start(flow, layout, MetricWidth.Value, MetricHeight.Value);
        RefreshReport();
        return accepted;
    }

    private void RefreshReport()
    {
        ReportText.Text = Runner.Run == null ? "" : FlowReportText.Format(Runner.Run, Runner.StepIndex);
        Message.Text = Runner.Run == null ? Runner.Error : $"Step {Runner.StepIndex + 1}/{Runner.Run.Report.Steps.Count}";
        NextButton.Disabled = !Runner.CanAdvance; SaveButton.Disabled = Runner.Run == null;
        if (Runner.ActiveStep is { } step)
        {
            ContextChoice.Select(Catalogue.Contexts.ToList().IndexOf(step.Context));
            Canvas.ShowContext(CurrentLayout, step.Context); Canvas.Highlight(step.Command);
        }
        else Canvas.Highlight(null);
    }

    private void ClearReport() { Runner.Clear(); RefreshReport(); }
    private void RefreshCanvas() => Canvas.ShowContext(CurrentLayout, Catalogue.Contexts[ContextChoice.Selected]);

    private void Change(string id, ParameterValue value)
    {
        if (id == "layoutId")
        {
            LayoutChoice.Select(Array.FindIndex(Layouts, l => l.Id == ((EnumValue)value).Value));
            ClearReport(); RefreshCanvas();
        }
        ResizeViews();
    }

    private void ResizeViews()
    {
        if (app.Size.X <= 0 || app.Size.Y <= 0) return;
        if (Toggle.ButtonPressed)
        {
            float width = panel.Position.X / 2;
            Canvas.Position = Vector2.Zero; Canvas.Size = new Vector2(width, app.Size.Y);
            stage.Position = new Vector2(width, 0); stage.Size = new Vector2(width, app.Size.Y);
        }
        else
        {
            bool docked = app.State.Preset.Params.Choice("panelPlacement") == "docked" ||
                app.State.Preset.Params.Choice("layoutId") == "docked-workbench";
            stage.Position = Vector2.Zero;
            stage.Size = new Vector2(docked ? panel.Position.X : app.Size.X, app.Size.Y);
        }
    }

    private void SaveReport()
    {
        if (Runner.Run == null) return;
        try { files.Save(Runner.Run); Message.Text = "Report saved in work/loop-memory/looklab/flows/."; }
        catch (IOException) { Message.Text = "Could not save the report in the Look Lab flows folder."; }
        catch (UnauthorizedAccessException) { Message.Text = "Could not save the report in the Look Lab flows folder."; }
    }

    public override void _ExitTree()
    {
        app.State.Changed -= Change; app.Resized -= ResizeViews;
    }
}
