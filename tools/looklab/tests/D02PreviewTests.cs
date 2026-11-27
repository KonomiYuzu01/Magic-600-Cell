using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

internal static partial class Program
{
    private static RepairPreviewFacts PreviewFacts(ulong id, int letters) =>
        new(id, "after", Enumerable.Repeat(12, letters).ToArray(), new[] { new OrbitEffect("Orbit 0", 12, 24) },
            "No protected orbits", 12);

    private static void D02PreviewFollowsOnlyTheStagedWord()
    {
        var ledger = new BoundaryLedger();
        var (list, steps) = RepairAt150(ledger);
        var stage = new RepairStage(ledger, () => true);
        var main = (Revision: 300, Digest: "committed", Pieces: new[] { 10, 11, 12 });
        var model = new PreviewModel { Head = () => main.Revision };
        ulong builds = 0;
        var preview = new RepairPreview(stage, ledger, model, () => 0, word => PreviewFacts(++builds, word.Count));
        Check(!preview.Overlay && model.Pending == null && builds == 0, "opening the list staged a preview or an overlay");
        list.Down(UserEvent.Key("Down"));
        Check(model.Pending == null, "focusing a row staged a preview");
        stage.Stage(list, list.Rows[1], steps, UserEvent.Key("2"), Origin.Solver);
        Check(model.Pending is { Id: 1, WholeOnly: true } p && p.Moves.SequenceEqual(new[] { "150·1", "215·21", "150·1" })
            && model.Scrub == 3 && preview.Facts?.Id == 1, "the pick did not build the staged word");
        stage.Stage(list, list.Rows[0], steps, UserEvent.Key("1"), Origin.Solver);
        Check(model.Pending is { Id: 2, WholeOnly: true } && model.Replaced && preview.Facts?.Id == 2
            && model.Scrub == stage.Word.Letters.Count, "the word change did not replace the preview");
        Check(main.Revision == 300 && main.Digest == "committed" && main.Pieces.SequenceEqual(new[] { 10, 11, 12 }),
            "the main pair changed while staging");
        Check(ledger.Count == 0, "the preview counted an unasked act");
    }

    private static void D02PreviewScrubCancelAndCommitRefusals()
    {
        double clock = 0;
        int head = 300;
        bool refused = false;
        var ledger = new BoundaryLedger();
        var (list, steps) = RepairAt150(ledger);
        var stage = new RepairStage(ledger, () => true);
        var model = new PreviewModel { Head = () => head };
        var preview = new RepairPreview(stage, ledger, model, () => clock, word => refused
            ? PreviewFacts(0, word.Count) with { Refusal = "Twist 2 is blocked" }
            : PreviewFacts(1, word.Count));
        var asked = UserEvent.Key("Ctrl+Enter");
        void Stage() => stage.Stage(list, list.Rows[1], steps, UserEvent.Key("2"), Origin.Solver);
        Stage();
        Equal(preview.CommitRefusal(asked), "the preview has just changed");
        Check(model.Settling(clock) && preview.CommitRefusal(asked) == PreviewModel.JustChanged,
            "a fresh whole-only preview did not start the settle interval");
        preview.Scrub("forward", UserEvent.Key("Right"));
        Equal(model.Scrub, 3);
        preview.Scrub("back", UserEvent.Key("Left"));
        Equal(model.Scrub, 2);
        preview.Scrub("start", UserEvent.Click("scrub"));
        preview.Scrub("back", UserEvent.Key("Left"));
        Equal(model.Scrub, 0);
        clock = PreviewModel.Settle + 0.1;
        Check(!model.Settling(clock), "the settle interval did not end");
        Check(preview.CommitRefusal(asked) == null && !model.AtPrefix, "whole-only was refused at the start");
        Check(model.Take(clock)?.Moves.Count == 3, "a scrub at the start did not take the full repair");
        preview.Cancel();
        Check(stage.Word.Letters.Count == 0 && stage.Word.Spans.Count == 0 && preview.Facts == null, "cancel left a staged word");
        Stage();
        head++;
        model.Follow();
        Check(model.Pending?.Status == PreviewStatus.Stale && preview.CommitRefusal(asked) == PreviewModel.HeadMoved,
            "a different head did not refuse the preview");
        preview.Cancel();
        Check(model.Pending == null && model.Scrub == 0, "cancel left a preview");
        refused = true;
        Stage();
        Check(model.Pending?.Status == PreviewStatus.Refused && preview.CommitRefusal(asked) == "Twist 2 is blocked",
            "the refusal did not reach the commit gate");
        Check(ledger.Count == 0, "the preview counted an unasked act");
    }

    private static void D02PreviewComparisonUsesFourPanes()
    {
        Equal(ViewPanes.Comparison(PreviewSide.Both, 2), PreviewSide.Both);
        Equal(ViewPanes.Comparison(PreviewSide.Both, 3), PreviewSide.After);
        Equal(ViewPanes.Comparison(PreviewSide.Before, 3), PreviewSide.Before);
    }

    private static void D02PreviewOverlayRowIsDraft()
    {
        using var design = JsonDocument.Parse(Read("data/design/command-rows-d02.json"));
        var row = design.RootElement.GetProperty("preview").EnumerateArray().Single();
        const string id = "preview.overlay.script.toggle";
        Equal(row.GetProperty("id").GetString(), id);
        Equal(row.GetProperty("kind").GetString(), "view");
        Equal(row.GetProperty("request").GetString(), "ADR-D01.10");
        var table = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        Check(!table.Commands.ContainsKey(id), "the preview overlay draft is in the command table");
    }
}
