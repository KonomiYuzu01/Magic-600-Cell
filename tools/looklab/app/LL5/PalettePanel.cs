using Godot;
using LookLab.Core;
using LabColour = LookLab.Core.Colour;

namespace LookLab.App.LL5;

internal sealed partial class PalettePanel : VBoxContainer
{
    private readonly LookState state;
    private readonly CellStructure structure;
    private readonly Label values = new() { AutowrapMode = TextServer.AutowrapMode.WordSmart };
    public PaletteThresholds? Thresholds { get; private set; }
    public PaletteReport Report { get; private set; } = null!;
    public string Text => values.Text;

    public PalettePanel(LookState state, CellStructure structure, PaletteThresholds? thresholds = null)
    {
        this.state = state; this.structure = structure; Thresholds = thresholds;
        AddChild(new Label { Text = "Palette check" }); AddChild(values); Refresh();
    }

    public void SetThresholds(PaletteThresholds? thresholds)
    {
        // Validate the caller's marks before replacing the displayed report.
        PaletteReport report = LabColour.Report(state.Preset.Params, structure, thresholds);
        Thresholds = thresholds; Report = report; values.Text = PaletteDisplay.Text(report, thresholds);
    }

    public void Refresh() => SetThresholds(Thresholds);
}
