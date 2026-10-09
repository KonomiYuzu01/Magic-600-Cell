using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Json.Nodes;
using Godot;
using LookLab.Core;

namespace LookLab.App.LL4;

internal static class LL4Checks
{
    public static bool Handles(string mode) => mode is "ll4-metrics" or "ll4-refusal" or "ll4-greybox";

    public static void Run(LabApp app, string root, string mode, string fault)
    {
        var controls = (GreyboxControls)app.FindChild("LL4Controls", true, false);
        controls.Toggle.ButtonPressed = true;
        var stage = app.GetChildren().OfType<SubViewportContainer>().Single();
        Require(controls.Canvas.Size.X > 0 && stage.Size.X > 0 &&
            stage.Position.X == controls.Canvas.Position.X + controls.Canvas.Size.X, "ll4-greybox", "views are not beside one another");
        if (mode == "ll4-greybox") { Greybox(controls, fault); return; }
        int layoutIndex = Array.FindIndex(controls.Layouts, l => l.Id == "docked-workbench");
        int flowIndex = Array.FindIndex(controls.Flows, f => f.Id == "piece-operation");
        controls.LayoutChoice.Select(layoutIndex);
        controls.LayoutChoice.EmitSignal(OptionButton.SignalName.ItemSelected, layoutIndex);
        controls.FlowChoice.Select(flowIndex);
        controls.MetricWidth.Value = 1280; controls.MetricHeight.Value = 720;
        if (mode == "ll4-metrics") Metrics(controls);
        else Refusal(controls, root, fault);
    }

    private static void Require(bool condition, string check, string detail)
    {
        if (!condition) throw new Exception(check + ": " + detail);
    }

    private static void Metrics(GreyboxControls controls)
    {
        const string check = "ll4-metrics";
        FlowScript flow = controls.Flows[controls.FlowChoice.Selected];
        FlowReport expected = FlowMetrics.Report(flow, controls.CurrentLayout, 1280, 720);
        controls.RunButton.EmitSignal(Button.SignalName.Pressed);
        FlowReport? actual = controls.Runner.Run?.Report;
        Require(actual != null && actual.Steps.SequenceEqual(expected.Steps) &&
            actual.TravelPixels == expected.TravelPixels && actual.EstimatedSeconds == expected.EstimatedSeconds &&
            actual.ASeconds == expected.ASeconds && actual.BSecondsPerBit == expected.BSecondsPerBit,
            check, "report differs from the core");
        for (int i = 0; i < expected.Steps.Count; i++)
        {
            FlowStepReport step = expected.Steps[i];
            Require(controls.Runner.StepIndex == i && controls.Canvas.HighlightedCommand == step.Command &&
                controls.Canvas.Context == step.Context && controls.Canvas.CommandControls.ContainsKey(step.Command), check, "step highlight missing");
            Require(controls.Canvas.CommandControls[step.Command].GetMeta("ll4_region").AsString() == step.Region, check, "highlight in wrong region");
            foreach (double value in new[] { step.TravelPixels, step.TargetWidthPixels, step.EstimatedSeconds })
                Require(controls.ReportText.Text.Contains(value.ToString("R", CultureInfo.InvariantCulture), StringComparison.Ordinal), check, "step number missing from report");
            if (i + 1 < expected.Steps.Count) controls.NextButton.EmitSignal(Button.SignalName.Pressed);
        }
        Require(controls.NextButton.Disabled && !controls.SaveButton.Disabled &&
            controls.ReportText.Text.Contains("a=0.1 s; b=0.15 s/bit", StringComparison.Ordinal), check, "completion or constants missing");
        controls.MetricHeight.Value = 721;
        Require(controls.Runner.Run == null && controls.ReportText.Text == "" && controls.SaveButton.Disabled, check, "changed dimensions left a stale report");
        GD.Print($"LOOKLAB_LL4_METRICS_PASS steps={expected.Steps.Count} checks=1");
    }

    private static void Refusal(GreyboxControls controls, string root, string fault)
    {
        string check = fault == "skip-flow-refusal" ? fault : "ll4-refusal";
        FlowScript flow = controls.Flows[controls.FlowChoice.Selected];
        FlowStep unreachable = flow.Steps[^1];
        JsonNode node = JsonNode.Parse(File.ReadAllText(Path.Combine(root, "tools/looklab/data/layouts/docked-workbench.json")))!;
        var context = node["contexts"]![unreachable.Context]!;
        context["placed"]!.AsObject().Remove(unreachable.Command);
        context["hidden"]!.AsArray().Add(unreachable.Command);
        LayoutSpec hidden = LayoutSpec.Parse(node.ToJsonString(), controls.Catalogue);
        Require(controls.StartFlow(flow, controls.CurrentLayout), check, "valid seed flow failed");
        controls.Runner.RefusalEnabled = fault != "skip-flow-refusal";
        bool handled = false;
        try { handled = !controls.StartFlow(flow, hidden); }
        catch (FormatException) { } // An unhandled core error is not a UI refusal.
        Require(handled && controls.Runner.Error.Contains(unreachable.Command, StringComparison.Ordinal) &&
            controls.Runner.Error.Contains(unreachable.Context, StringComparison.Ordinal) && controls.Runner.Run == null &&
            controls.Runner.ActiveStep == null && controls.Runner.StepIndex == -1 && controls.ReportText.Text == "" &&
            controls.Message.Text == controls.Runner.Error && controls.NextButton.Disabled && controls.SaveButton.Disabled,
            check, "unreachable flow was not refused without a partial or stale report");
        GD.Print($"LOOKLAB_LL4_REFUSAL_PASS command={unreachable.Command} context={unreachable.Context} checks=1");
    }

    private static void Greybox(GreyboxControls controls, string fault)
    {
        string check = fault == "skip-catalogue-filter" ? fault : "ll4-greybox";
        controls.Canvas.CatalogueFilterEnabled = fault != "skip-catalogue-filter";
        int contexts = 0, placements = 0;
        for (int i = 0; i < controls.Layouts.Length; i++)
        {
            controls.LayoutChoice.Select(i); controls.LayoutChoice.EmitSignal(OptionButton.SignalName.ItemSelected, i);
            foreach (string context in controls.Catalogue.Contexts)
            {
                controls.ContextChoice.Select(controls.Catalogue.Contexts.ToList().IndexOf(context));
                controls.ContextChoice.EmitSignal(OptionButton.SignalName.ItemSelected, controls.ContextChoice.Selected);
                Button[] shown = Descendants(controls.Canvas).OfType<Button>().Where(b => b.HasMeta("ll4_command")).ToArray();
                string[] expected = controls.Catalogue.Commands.Values.Where(c => c.Contexts.Contains(context)).Select(c => c.Id).OrderBy(c => c, StringComparer.Ordinal).ToArray();
                Require(shown.All(b => controls.Catalogue.Commands.ContainsKey(b.Text) && b.IsVisibleInTree()) &&
                    shown.Select(b => b.Text).OrderBy(c => c, StringComparer.Ordinal).SequenceEqual(expected), check, "shown set differs in " + context);
                foreach (Button button in shown)
                    Require(button.GetMeta("ll4_region").AsString() == controls.CurrentLayout.Resolve(button.Text, context).Id, check, "command in wrong region");
                foreach (var region in controls.Canvas.RegionControls)
                {
                    RegionRect rect = controls.CurrentLayout.Regions[region.Key].Rect;
                    Require(region.Value.Position.IsEqualApprox(new Vector2((float)rect.X * controls.Canvas.Size.X, (float)rect.Y * controls.Canvas.Size.Y)), check, "region position differs");
                    Require(region.Value.Size.IsEqualApprox(new Vector2((float)rect.Width * controls.Canvas.Size.X, (float)rect.Height * controls.Canvas.Size.Y)), check, "region dimensions differ");
                }
                contexts++; placements += shown.Length;
            }
        }
        GD.Print($"LOOKLAB_LL4_GREYBOX_PASS layouts={controls.Layouts.Length} contexts={contexts} commands={controls.Catalogue.Commands.Count} placements={placements} checks=1");
    }

    private static IEnumerable<Node> Descendants(Node node)
    {
        foreach (Node child in node.GetChildren())
        {
            yield return child;
            foreach (Node nested in Descendants(child)) yield return nested;
        }
    }
}
