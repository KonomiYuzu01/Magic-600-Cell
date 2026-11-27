using System;
using System.Text.Json;
using Magic600.Chrome;

internal static partial class Program
{
    private static void D02SliceDumpKeepsHeldAngle()
    {
        const float angle = 2.0943952f;
        var slice = new CutSlice();
        slice.StepItem(1);
        Check(slice.Press(150, angle), "item 2 can be held as a ghost");
        var named = slice.Release()!;
        Equal(named, new NamedTwist(150, 2, angle));
        Equal(JsonSerializer.Serialize((double)angle), JsonSerializer.Serialize(named.Angle),
            "the ghost and the release must record the same native angle");
        var shell = RefineShell("project/D02/ShellSlice.cs");
        Check(shell.Contains("json.WriteNumber(\"ghost_angle\", (double)row.Angle)")
            && shell.Contains("json.WriteNumber(\"angle\", named.Angle)"), "both dumps use double precision");
    }

    private static void D02StatusScriptScrubsWithUserEvent()
    {
        using var script = JsonDocument.Parse(RefineShell("d02_scripts/status-patterns.json"));
        var steps = script.RootElement.GetProperty("steps");
        int at = steps.GetArrayLength() - 6;
        Equal(steps[at].GetProperty("command").GetString(), "command.find");
        Equal(steps[at + 1].GetProperty("text").GetString(), "> view.preview.scrub back");
        Equal(steps[at + 2].GetProperty("chord").GetString(), "Enter");

        int head = 302;
        var ledger = new BoundaryLedger();
        var (list, repairSteps) = RepairAt150(ledger);
        var stage = new RepairStage(ledger, () => true);
        var model = new PreviewModel { Head = () => head };
        var repair = new RepairPreview(stage, ledger, model, () => 0, letters => PreviewFacts(1, letters.Count));
        stage.Stage(list, list.Rows[1], repairSteps, UserEvent.Key("2"), Origin.Solver);
        Equal(model.Scrub, 3);
        repair.Scrub("back", UserEvent.Click("command palette"));
        Equal(model.Scrub, 2);
        Equal(ledger.Count, 0);
        byte[] moved = { 1, 1, 1 }, prefix = { 1, 1, 0 };
        Equal(StatusPatterns.RepairFeed(model.Pending, head, moved, prefix).Unchecked, prefix);
        head++;
        model.Follow();
        Equal(StatusPatterns.RepairFeed(model.Pending, head, moved, prefix), new StatusPatterns.PreviewFeed(null, null, moved));
    }
}
