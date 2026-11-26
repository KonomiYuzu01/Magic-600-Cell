using System;
using System.Linq;
using Magic600.Chrome;

// ADR-D02 build step B13: staging a picked repair row as a sealed, whole-only span, the research origin refused at
// preview, and the boundary ledger (ADR-D02.5, .33, .40), on wj-ia-300's record of cap 150.
internal static partial class Program
{
    private static (RepairListModel List, RepairSteps Steps) RepairAt150(BoundaryLedger? ledger = null)
    {
        string record = Read("data/samples/jumble/wj-ia-300/cap-150.json");
        RepairListModel list = RepairListModel.From(record, 300)!;
        list.Ledger = ledger;
        return (list, RepairSteps.From(record));
    }

    // Only a pick, carrying its user event, stages a row; without one nothing is staged and the ledger counts it.
    private static void D02StageOnlyFromPick()
    {
        var ledger = new BoundaryLedger();
        var (list, steps) = RepairAt150(ledger);
        var stage = new RepairStage(ledger);
        Check(stage.Stage(list, list.Rows[1], steps, null) == null, "a row was staged without a user event");
        Check(stage.Word.Letters.Count == 0 && stage.Records.Count == 0 && !stage.WholeOnly, "an unasked stage left a word, a record or the bar");
        Check(ledger.Count == 1 && ledger.Unasked[0].Kind == "stage", "the unasked stage was not counted");
        RepairRow picked = list.Digit(2, new[] { RepairListModel.Context })!;
        SealedSpan span = stage.Stage(list, picked, steps, UserEvent.Key("2"))!;
        Check(span != null && span.Row == list.Rows[1] && span.Length == 3, "the picked row was not staged as one span of three letters");
        Check(stage.WholeOnly && stage.Bar == "Whole repair only", "the bar reads " + stage.Bar);
        RepairPairRecord written = stage.Records.Single();
        Check(written.Cap == 150 && written.Revision == 300 && written.Row == 2 && written.X == picked.X && written.D == picked.D && written.Y == picked.Y
            && written.Z == picked.Z && written.DeltaN == picked.DeltaN && written.Asked == "key 2", "the repair-pair record is " + written);
        Check(written.Pin.StartsWith("5a0429d2", StringComparison.Ordinal) && written.TieRule.Length > 0 && written.Menu.StartsWith("fddc8532", StringComparison.Ordinal), "the record does not carry what the list ran under");
        Check(string.Join(" ", stage.Word.Letters) == "150·1 215·21 150·1", "the staged letters are " + string.Join(" ", stage.Word.Letters));
        Check(ledger.Count == 1, "the asked stage was counted as unasked");
    }

    // A sealed span is removed whole and never edited; a prefix of it never commits.
    private static void D02SealedSpanRemovedWhole()
    {
        var ledger = new BoundaryLedger();
        var (list, steps) = RepairAt150(ledger);
        var stage = new RepairStage(ledger);
        stage.Word.Type(new Letter(91, 3));
        stage.Stage(list, list.Rows[0], steps, UserEvent.Key("1"));
        stage.Word.Type(new Letter(91, 4));
        Check(stage.Word.Text() == "91·3 [150·1 91·50 150·1] 91·4", "the word reads " + stage.Word.Text());
        Check(!stage.Word.Edit(2, new Letter(91, 7)), "a letter inside the sealed span was edited");
        Check(stage.Word.Edit(0, new Letter(91, 5)), "an ordinary letter could not be edited");
        Check(stage.PrefixRefusal(2) == "Whole repair only" && stage.PrefixRefusal(5) == null, "a prefix through the span was admitted");
        Check(stage.Remove(2) == 3, "removing inside the span did not remove three letters");
        Check(stage.Word.Text() == "91·5 91·4" && stage.Word.Spans.Count == 0 && !stage.WholeOnly && stage.Bar == "", "the word after removal reads " + stage.Word.Text());
        // Two spans: removing the first moves the second, which stays whole.
        stage.Stage(list, list.Rows[0], steps, UserEvent.Key("1"));
        stage.Stage(list, list.Rows[2], steps, UserEvent.Key("3"));
        Check(stage.Word.Spans.Count == 2 && stage.Remove(3) == 3 && stage.Word.Spans.Single().Start == 2 && stage.Word.Spans.Single().Row == list.Rows[2], "the second span did not move whole");
        Check(ledger.Count == 0, "the ledger counted an asked act");
    }

    // Typing the same twists as a repair row makes an ordinary word: no span, no whole-only bar.
    private static void D02TypedTwistsAreOrdinaryWord()
    {
        var ledger = new BoundaryLedger();
        var (list, steps) = RepairAt150(ledger);
        var stage = new RepairStage(ledger);
        foreach (var (cap, item) in steps.Letters(list.Rows[0], 150)) stage.Word.Type(new Letter(cap, item));
        Check(stage.Word.Text() == "150·1 91·50 150·1", "the typed word reads " + stage.Word.Text());
        Check(stage.Word.Spans.Count == 0 && !stage.WholeOnly && stage.Bar == "" && stage.PrefixRefusal(1) == null, "typed twists became a sealed span");
        Check(stage.Word.Edit(1, new Letter(91, 49)) && stage.Remove(0) == 1, "a typed letter could not be edited or removed alone");
        Check(stage.Records.Count == 0, "typing wrote a repair-pair record");
    }

    // A research run's output is refused at preview in a solving session, with the word record's sentence.
    private static void D02ResearchOriginRefusedInSolvingSession()
    {
        string sentence = Words()["refusal.research"];
        Check(PreviewModel.Admit(Origin.Research, true) == sentence && PreviewAdmission.ResearchRefused == sentence, "the refusal reads " + PreviewModel.Admit(Origin.Research, true));
        Check(PreviewModel.Admit(Origin.Research, false) == null, "research output was refused outside a solving session");
        Check(PreviewModel.Admit(Origin.Solver, true) == null && PreviewModel.Admit(Origin.Library, true) == null, "the solver's or the Library's word was refused");
    }

    // Over a scripted session every staging and focusing carries its user event: the ledger ends at zero, and one act
    // without an event is counted.
    private static void D02BoundaryCountZeroOverScriptedSession()
    {
        var ledger = new BoundaryLedger();
        var (list, steps) = RepairAt150(ledger);
        var stage = new RepairStage(ledger);
        var scrub = new RepairScrub();
        string[] context = { RepairListModel.Context };
        list.Down(UserEvent.Key("Down")); list.Down(UserEvent.Key("Down")); list.Up(UserEvent.Key("Up")); list.Down(UserEvent.Key("Down"));
        scrub.Show(list.Rows[list.Focused!.Value - 1]); scrub.Scrub("forward");
        stage.Stage(list, list.Enter(context)!, steps, UserEvent.Key("Enter"));
        stage.Stage(list, list.Digit(5, context)!, steps, UserEvent.Key("5"));
        stage.Remove(0);
        Check(ledger.Count == 0 && ledger.Asked == 6, "the scripted session left " + ledger.Count + " unasked acts and " + ledger.Asked + " asked");
        Check(stage.Word.Spans.Count == 1 && stage.Records.Count == 2, "the scripted session staged " + stage.Word.Spans.Count + " spans");
        list.Down();
        stage.Stage(list, list.Rows[0], steps, null);
        Check(ledger.Count == 2 && ledger.Unasked.Select(u => u.Kind).SequenceEqual(new[] { "focus", "stage" }), "unasked acts were not counted");
        bool refused = false;
        try { ledger.Record("rank", "a row", UserEvent.Key("x")); } catch (ArgumentException) { refused = true; }
        Check(refused, "the ledger took an act it does not know");
    }
}
