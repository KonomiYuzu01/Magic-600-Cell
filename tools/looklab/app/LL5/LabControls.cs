using System;
using System.IO;
using Godot;
using LookLab.Core;

namespace LookLab.App.LL5;

internal sealed partial class LabControls : VBoxContainer
{
    private readonly LookState state;
    private readonly Label status;
    private string baseStatus = "Full detail: 259,800 slots";
    public CvdPreview Preview { get; }
    public PalettePanel Palette { get; }
    public CostMeter Costs { get; }
    public string StatusText => status.Text;

    private LabControls(string root, LookState state, LookViewport view, Label status)
    {
        this.state = state; this.status = status;
        Preview = new CvdPreview(state, view); AddChild(Preview);
        Palette = new PalettePanel(state, view.Geometry.CellStructure); AddChild(Palette);
        Costs = new CostMeter(state, CostTable.Load(Path.Combine(root, "tools/looklab/data/cost.json"), state.Schema)); AddChild(Costs);
        AddChild(new HSeparator());
        state.Changed += Refresh; UpdateStatus(baseStatus);
    }

    public static LabControls Attach(string root, LookState state, LookViewport view, VBoxContainer panels, Label status)
    {
        var controls = new LabControls(root, state, view, status) { SizeFlagsHorizontal = SizeFlags.ExpandFill };
        panels.AddChild(controls); panels.MoveChild(controls, 0); return controls;
    }

    public void UpdateStatus(string text)
    {
        baseStatus = text.Replace(" · cost not measured", "", StringComparison.Ordinal);
        status.Text = baseStatus + " · " + Costs.Summary;
    }

    private void Refresh(string id, ParameterValue value)
    {
        Preview.Refresh(); Palette.Refresh(); Costs.Refresh(); UpdateStatus(baseStatus);
    }

    public override void _ExitTree() => state.Changed -= Refresh;
}
