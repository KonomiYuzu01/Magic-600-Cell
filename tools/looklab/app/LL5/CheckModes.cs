using System;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using Godot;
using LookLab.Core;
using LabColour = LookLab.Core.Colour;

namespace LookLab.App.LL5;

internal static class CheckModes
{
    public static bool Handles(string mode) => mode is "ll5-cvd" or "ll5-cost" or "ll5-palette" or "ll5-cvd-gpu";

    public static async Task Run(Node host, LabApp app, string mode, string fault)
    {
        LabControls controls = app.Panels.GetChildren().OfType<LabControls>().Single();
        if (mode == "ll5-cvd-gpu") await CvdGpuCheck.Run(host, app, controls.Preview, fault);
        else if (mode == "ll5-cvd") Cvd(app, controls.Preview, fault);
        else if (mode == "ll5-cost") Cost(app, controls, fault);
        else Palette(app, controls.Palette);
        host.GetTree().Quit();
    }

    private static void Cvd(LabApp app, CvdPreview preview, string fault)
    {
        const string disabled = "ll5-disable-cvd";
        preview.TransformEnabled = fault != disabled;
        if (preview.Mode != "normal" || preview.Choice.Selected != 0) throw new Exception("ll5-cvd: normal vision must be the default");
        int samples = 0;
        foreach (string mode in LabColour.Modes)
        {
            preview.Choice.EmitSignal(OptionButton.SignalName.ItemSelected, LabColour.Modes.ToList().IndexOf(mode));
            if (preview.Mode != mode) throw new Exception("ll5-cvd: preview switch did not select " + mode);
            foreach (double[] linear in CvdCheckData.LinearSamples)
            {
                CvdCheckData.Require(preview.ColourPath(linear), CvdCheckData.Reference(linear, mode), fault == disabled ? disabled : "ll5-cvd");
                samples++;
            }
            double[] background = LabColour.Background(app.State.Preset.Params).Linear;
            Color displayed = app.View.Environment.BackgroundColor.SrgbToLinear();
            CvdCheckData.Require(new[] { displayed.R, displayed.G, displayed.B }, CvdCheckData.Reference(background, mode), "ll5-cvd-background");
        }
        // A live preset change must retain the selected preview and clear colour.
        app.State.Set("bgHue", new NumberValue(130));
        Color changed = app.View.Environment.BackgroundColor.SrgbToLinear();
        CvdCheckData.Require(new[] { changed.R, changed.G, changed.B },
            CvdCheckData.Reference(LabColour.Background(app.State.Preset.Params).Linear, "tritan"), "ll5-cvd-live-background");
        preview.SelectMode("normal");
        GD.Print($"LOOKLAB_LL5_CVD_PASS modes={LabColour.Modes.Count} samples={samples} backgrounds=5 tolerance=2e-6-linear");
    }

    private static void Cost(LabApp app, LabControls controls, string fault)
    {
        const string disabled = "ll5-disable-null-cost";
        CostMeter meter = controls.Costs; meter.HandleNull = fault != disabled; meter.Refresh();
        int count = app.State.Schema.Parameters.Count(p => p.CostFeature != null);
        if (meter.Rows.Count != count || count == 0 || meter.Rows.Any(r => r.Cost != "not measured" || !meter.Text.Contains(r.Text, StringComparison.Ordinal)))
            throw new Exception((fault == disabled ? disabled : "ll5-cost") + ": every null entry must show not measured");
        if (!controls.StatusText.EndsWith(meter.Summary, StringComparison.Ordinal) || meter.Summary != "cost not measured")
            throw new Exception("ll5-cost: status line must use the meter");
        var features = new JsonObject();
        foreach (string feature in app.State.Schema.Parameters.Where(p => p.CostFeature != null).Select(p => p.CostFeature!).Distinct()) features.Add(feature, null);
        string measured = meter.Rows[0].Feature;
        features[measured] = new JsonObject { ["milliseconds"] = 1.25, ["source"] = "synthetic full-detail fixture" };
        var data = new JsonObject { ["format"] = "magic600-look-cost", ["version"] = 1, ["features"] = features };
        meter.SetTable(CostTable.Parse(data.ToJsonString(), app.State.Schema));
        app.State.Set(meter.Rows[0].Parameter, new NumberValue(.5));
        CostRow row = meter.Rows[0];
        if (row.Cost != "1.25 ms (source: synthetic full-detail fixture)" || row.Setting != "0.5" ||
            !meter.Text.Contains(row.Text, StringComparison.Ordinal) || !controls.StatusText.EndsWith(meter.Summary, StringComparison.Ordinal) ||
            !controls.StatusText.Contains(row.Cost, StringComparison.Ordinal) || meter.Rows.Skip(1).Any(r => r.Cost != "not measured"))
            throw new Exception("ll5-cost: measured value, source, current setting or live status missing");
        GD.Print($"LOOKLAB_LL5_COST_PASS nulls={count} measured=1 source=shown live=passed");
    }

    private static void Palette(LabApp app, PalettePanel panel)
    {
        if (panel.Thresholds != null || panel.Report.Modes.Count != LabColour.Modes.Count ||
            panel.Text.Contains("(pass)", StringComparison.Ordinal) || panel.Text.Contains("(fail)", StringComparison.Ordinal))
            throw new Exception("ll5-palette: default report must assign no threshold marks");
        panel.SetThresholds(new PaletteThresholds(2, 2));
        if (!panel.Text.Contains("(fail)", StringComparison.Ordinal) || panel.Report.Modes.Any(m => m.MeetsDistance != false || m.MeetsBackground != false))
            throw new Exception("ll5-palette: caller thresholds were not applied");
        panel.SetThresholds(null);
        app.State.Set("lightness", new NumberValue(.85)); app.State.Set("lightnessAlt", new NumberValue(.2));
        if (panel.Report.GamutFailures.Count == 0 || panel.Report.GamutFailures.Any(f =>
            !panel.Text.Contains("class " + f.ClassIndex + ": " + f.Reason, StringComparison.Ordinal)) ||
            panel.Text.Contains("(fail)", StringComparison.Ordinal) || !panel.Text.Contains("unavailable", StringComparison.Ordinal))
            throw new Exception("ll5-palette: live gamut failures must name the class and reason without threshold marks");
        GD.Print($"LOOKLAB_LL5_PALETTE_PASS modes={panel.Report.Modes.Count} pairs={panel.Report.ClassPairs.Count} same_ring={panel.Report.SameRingAdjacencies} gamut_failures={panel.Report.GamutFailures.Count} thresholds=caller");
    }
}
