using System;
using System.Linq;
using Godot;
using LookLab.Core;
using LabColour = LookLab.Core.Colour;

namespace LookLab.App.LL5;

internal sealed partial class CvdPreview : VBoxContainer
{
    private readonly LookState state;
    private readonly LookViewport view;
    public OptionButton Choice { get; } = new();
    public string Mode { get; private set; } = "normal";
    internal bool TransformEnabled { get; set; } = true;

    public CvdPreview(LookState state, LookViewport view)
    {
        this.state = state; this.view = view;
        AddChild(new Label { Text = "Colour vision preview" }); AddChild(Choice);
        foreach (string mode in LabColour.Modes) Choice.AddItem(mode == "normal" ? "Normal vision" : mode);
        Choice.ItemSelected += index => SelectMode(LabColour.Modes[(int)index]);
        SelectMode("normal");
    }

    public void SelectMode(string mode)
    {
        int index = LabColour.Modes.ToList().IndexOf(mode);
        if (index < 0) throw new FormatException("CVD: unknown mode " + mode);
        Mode = mode; Choice.Select(index); Refresh();
    }

    public void Refresh()
    {
        float[][] rows = CvdTransform.Rows(Mode);
        view.Material.SetShaderParameter("cvd_enabled", Mode != "normal" && TransformEnabled);
        for (int i = 0; i < 3; i++)
            view.Material.SetShaderParameter("cvd_r" + i, new Vector4(rows[i][0], rows[i][1], rows[i][2], 0));
        // The shader simulates its final linear ALBEDO (including its fog).
        // The clear background is outside that shader, so use the same path.
        float[] background = ColourPath(LabColour.Background(state.Preset.Params).Linear);
        view.Environment.BackgroundColor = new Color(background[0], background[1], background[2]).LinearToSrgb();
    }

    public float[] ColourPath(double[] linear)
    {
        var rows = new float[3][];
        for (int i = 0; i < 3; i++)
        {
            Vector4 row = view.Material.GetShaderParameter("cvd_r" + i).AsVector4();
            rows[i] = new[] { row.X, row.Y, row.Z };
        }
        return CvdTransform.Apply(linear.Select(x => (float)x).ToArray(), rows,
            view.Material.GetShaderParameter("cvd_enabled").AsBool());
    }
}
