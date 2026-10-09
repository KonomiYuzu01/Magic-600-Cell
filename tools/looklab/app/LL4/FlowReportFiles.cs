using System;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;

namespace LookLab.App.LL4;

internal sealed class FlowReportFiles
{
    private readonly string root;
    public string Folder { get; }
    private static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase, WriteIndented = true
    };

    public FlowReportFiles(string checkoutRoot)
    {
        root = Path.GetFullPath(checkoutRoot);
        Folder = Path.GetFullPath(Path.Combine(root, "work/loop-memory/looklab/flows"));
    }

    public static byte[] Bytes(FlowRun run) => Encoding.UTF8.GetBytes(
        JsonSerializer.Serialize(run, Options).Replace("\r\n", "\n", StringComparison.Ordinal) + "\n");

    public string Save(FlowRun run)
    {
        // Names are generated here, never taken from a flow or a layout id.
        for (string? path = Folder; path != null; path = Path.GetDirectoryName(path))
        {
            if ((Directory.Exists(path) || File.Exists(path)) &&
                (File.GetAttributes(path) & FileAttributes.ReparsePoint) != 0)
                throw new IOException("Flow reports must not pass through links.");
            if (path == root) break;
        }
        byte[] bytes = Bytes(run);
        Directory.CreateDirectory(Folder);
        string target = Path.Combine(Folder, "flow-" + Guid.NewGuid().ToString("N") + ".json");
        using var output = new FileStream(target, FileMode.CreateNew, FileAccess.Write, FileShare.None);
        output.Write(bytes);
        return target;
    }
}

internal static class FlowReportText
{
    public static string Format(FlowRun run, int activeStep)
    {
        static string N(double value) => value.ToString("R", CultureInfo.InvariantCulture);
        var report = run.Report;
        string[] rows = report.Steps.Select((step, i) =>
            $"{(i == activeStep ? ">" : " ")} {i + 1}. {step.Command} [{step.Context}] @ {step.Region}\n" +
            $"   travel {N(step.TravelPixels)} px; target {N(step.TargetWidthPixels)} px; estimated {N(step.EstimatedSeconds)} s").ToArray();
        return $"{run.Name}{(run.Draft ? " (draft)" : "")}\n" +
            $"{run.Layout}; model window {N(run.WindowWidthPixels)} x {N(run.WindowHeightPixels)} px\n" +
            $"Fitts estimate: a={N(report.ASeconds)} s; b={N(report.BSecondsPerBit)} s/bit\n" +
            "t = a + b * log2(1 + D/W); W = smaller region dimension\n" +
            "Estimates of region acquisition; not measured task times.\n" +
            string.Join("\n", rows) +
            $"\nTotal travel {N(report.TravelPixels)} px; estimated {N(report.EstimatedSeconds)} s";
    }
}
