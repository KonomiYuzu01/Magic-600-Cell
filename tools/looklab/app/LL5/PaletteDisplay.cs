using System;
using System.Globalization;
using System.Linq;
using LookLab.Core;

namespace LookLab.App.LL5;

internal static class PaletteDisplay
{
    private static string Value(double? value) => value?.ToString("G6", CultureInfo.InvariantCulture) ?? "unavailable";
    private static string Mark(bool? mark) => mark == null ? "" : mark.Value ? " (pass)" : " (fail)";

    public static string Text(PaletteReport report, PaletteThresholds? thresholds)
    {
        string header = thresholds == null ? "Values only; no thresholds supplied."
            : $"Caller thresholds: distance {Value(thresholds.Distance)}, background lightness {Value(thresholds.BackgroundLightness)}.";
        string modes = string.Join("\n", report.Modes.Select(m =>
            $"{m.Mode}: minimum OKLab distance {Value(m.MinDistance)}{(thresholds == null ? "" : Mark(m.MeetsDistance))}; " +
            $"minimum background lightness distance {Value(m.MinBackgroundLightness)}{(thresholds == null ? "" : Mark(m.MeetsBackground))}"));
        string pairs = string.Join(", ", report.ClassPairs.Select(p => $"{p.A}/{p.B}"));
        string gamut = report.GamutFailures.Count == 0 ? "Gamut failures: none."
            : string.Join("\n", report.GamutFailures.Select(f => $"Gamut failure: {(f.ClassIndex == null ? "background" : "class " + f.ClassIndex)}: {f.Reason}"));
        return header + "\n" + modes + "\nClass pairs (zero-based): " + pairs +
            $"\nSame-ring face adjacencies: {report.SameRingAdjacencies} (no colour-distance minimum).\n" + gamut;
    }
}
