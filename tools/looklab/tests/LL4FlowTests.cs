using System;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using LookLab.Core;
using LookLab.App.LL4;

internal static partial class Program
{
    private static void LL4KnownFlowReports()
    {
        var runner = new FlowRunner();
        foreach (string layoutFile in Directory.GetFiles(FileAt("data/layouts"), "*.json"))
        {
            LayoutSpec layout = LayoutSpec.Load(layoutFile, Commands);
            foreach (string flowFile in Directory.GetFiles(FileAt("data/flows"), "*.json"))
            {
                FlowScript flow = FlowScript.Load(flowFile, Commands);
                foreach (var size in new[] { (640.0, 360.0), (1280.0, 720.0) })
                {
                    Check(runner.Start(flow, layout, size.Item1, size.Item2), "known flow refused");
                    FlowRun run = runner.Run!;
                    FlowReport expected = FlowMetrics.Report(flow, layout, size.Item1, size.Item2);
                    Sequence(run.Report.Steps, expected.Steps, "per-step metrics differ from core");
                    Equal(run.Report.TravelPixels, expected.TravelPixels); Equal(run.Report.EstimatedSeconds, expected.EstimatedSeconds);
                    Equal(run.Report.ASeconds, expected.ASeconds); Equal(run.Report.BSecondsPerBit, expected.BSecondsPerBit);
                    Equal(run.Layout, layout.Id); Equal(run.Flow, flow.Id); Equal(run.Draft, flow.Draft);
                    Equal(run.WindowWidthPixels, size.Item1); Equal(run.WindowHeightPixels, size.Item2);
                    for (int i = 0; i < flow.Steps.Count; i++)
                    {
                        Equal(runner.StepIndex, i); Equal(runner.ActiveStep, expected.Steps[i]);
                        Equal(runner.Advance(), i + 1 < flow.Steps.Count);
                    }
                    Check(!runner.CanAdvance && !runner.Advance(), "advanced beyond final step");
                }
            }
        }
    }

    private static void LL4RefusalAndReset()
    {
        LayoutSpec layout = LayoutSpec.Load(FileAt("data/layouts/docked-workbench.json"), Commands);
        FlowScript flow = FlowScript.Load(FileAt("data/flows/piece-operation.json"), Commands);
        FlowStep last = flow.Steps[^1];
        JsonNode node = JsonNode.Parse(Read("data/layouts/docked-workbench.json"))!;
        node["contexts"]![last.Context]!["placed"]!.AsObject().Remove(last.Command);
        node["contexts"]![last.Context]!["hidden"]!.AsArray().Add(last.Command);
        LayoutSpec hidden = LayoutSpec.Parse(node.ToJsonString(), Commands);
        var runner = new FlowRunner();
        Check(runner.Start(flow, layout, 1280, 720), "seed report missing");
        runner.Advance();
        Check(!runner.Start(flow, hidden, 1280, 720), "unreachable tail accepted");
        Check(runner.Run == null && runner.ActiveStep == null && !runner.CanAdvance, "partial or previous report retained");
        Equal(runner.StepIndex, -1);
        Check(runner.Error.Contains(last.Command, StringComparison.Ordinal) && runner.Error.Contains(last.Context, StringComparison.Ordinal), "refusal must name command and context");
        Check(runner.Start(flow, layout, 1280, 720) && runner.Error == "", "valid rerun retained refusal");
        runner.Clear(); Check(runner.Run == null && runner.Error == "" && runner.StepIndex == -1, "reset retained report");
        runner.RefusalEnabled = false;
        Refuse(() => runner.Start(flow, hidden, 1280, 720), last.Command);
        Check(runner.Run == null, "disabled handler retained a report");
    }

    private static void LL4ReportJsonAndPrivateWrites()
    {
        using var temporary = new TestFolder();
        var files = new FlowReportFiles(temporary.Path);
        Check(!Directory.Exists(files.Folder), "startup created outputs");
        LayoutSpec layout = LayoutSpec.Load(FileAt("data/layouts/docked-workbench.json"), Commands);
        FlowScript flow = FlowScript.Load(FileAt("data/flows/piece-operation.json"), Commands);
        var runner = new FlowRunner(); Check(runner.Start(flow, layout, 1280, 720), "report missing");
        byte[] bytes = FlowReportFiles.Bytes(runner.Run!);
        Sequence(bytes, FlowReportFiles.Bytes(runner.Run!), "JSON not deterministic");
        Check(bytes[^1] == 10 && !bytes.Contains((byte)13) && !bytes.Take(3).SequenceEqual(new byte[] { 239, 187, 191 }), "JSON encoding changed");
        string text = Encoding.UTF8.GetString(bytes);
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase };
        FlowRun saved = JsonSerializer.Deserialize<FlowRun>(text, options)!;
        Equal(saved.Format, "magic600-look-flow-report"); Equal(saved.Version, 1);
        Equal(saved.Layout, layout.Id); Equal(saved.Flow, flow.Id); Equal(saved.Name, flow.Name); Equal(saved.Draft, true);
        Equal(saved.WindowWidthPixels, 1280.0); Equal(saved.WindowHeightPixels, 720.0);
        Sequence(saved.Report.Steps, runner.Run!.Report.Steps, "serialized steps lost precision");
        Equal(saved.Report.TravelPixels, runner.Run.Report.TravelPixels); Equal(saved.Report.EstimatedSeconds, runner.Run.Report.EstimatedSeconds);
        Equal(saved.Report.ASeconds, runner.Run.Report.ASeconds); Equal(saved.Report.BSecondsPerBit, runner.Run.Report.BSecondsPerBit);
        string path = files.Save(runner.Run);
        Equal(Path.GetDirectoryName(path), files.Folder); Sequence(File.ReadAllBytes(path), bytes);
        Sequence(Directory.GetFiles(temporary.Path, "*", SearchOption.AllDirectories), new[] { path }, "save wrote outside flow outputs");
        // Untrusted metadata never becomes a path, and an existing report is retained.
        string second = files.Save(runner.Run with { Flow = "../../escape", Layout = "../escape" });
        Equal(Path.GetDirectoryName(second), files.Folder); Check(second != path && File.Exists(path), "save overwrote previous report");
        string report = FlowReportText.Format(runner.Run, 0);
        Check(report.Contains("(draft)", StringComparison.Ordinal) && report.Contains("not measured task times", StringComparison.Ordinal) &&
            report.Contains("a=0.1 s; b=0.15 s/bit", StringComparison.Ordinal), "report omitted draft, model constants or estimate status");
    }

    private static void LL4HeadlessHarness()
    {
        var run = Run("python", new[] { "-B", FileAt("tests/ll4_harness_test.py") });
        Check(run.Code == 0, run.Out + run.Error);
        Console.WriteLine(run.Error.Split('\n').First(line => line.StartsWith("Ran ", StringComparison.Ordinal)).Trim());
    }
}
