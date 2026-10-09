using System;
using LookLab.Core;

namespace LookLab.App.LL4;

// Framework-neutral state shared by the controls and the core test runner.
internal sealed class FlowRunner
{
    public FlowRun? Run { get; private set; }
    public string Error { get; private set; } = "";
    public int StepIndex { get; private set; } = -1;
    public FlowStepReport? ActiveStep => Run == null || StepIndex < 0 ? null : Run.Report.Steps[StepIndex];
    public bool CanAdvance => Run != null && StepIndex + 1 < Run.Report.Steps.Count;
    internal bool RefusalEnabled { get; set; } = true;

    public void Clear()
    {
        Run = null; Error = ""; StepIndex = -1;
    }

    public bool Start(FlowScript flow, LayoutSpec layout, double width, double height)
    {
        Clear();
        try
        {
            // Resolve every step before publishing anything, including a highlight.
            FlowReport report = FlowMetrics.Report(flow, layout, width, height);
            Run = new FlowRun("magic600-look-flow-report", 1, layout.Id,
                flow.Id, flow.Name, flow.Draft, width, height, report);
            StepIndex = 0;
            return true;
        }
        catch (FormatException ex) when (RefusalEnabled)
        {
            Error = ex.Message;
            return false;
        }
    }

    public bool Advance()
    {
        if (!CanAdvance) return false;
        StepIndex++;
        return true;
    }
}

internal sealed record FlowRun(string Format, int Version, string Layout, string Flow,
    string Name, bool Draft, double WindowWidthPixels, double WindowHeightPixels, FlowReport Report);
