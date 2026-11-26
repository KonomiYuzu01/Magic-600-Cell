using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

// ADR-D02 build step B3: the tool slots, the release record and the numerals ADR-D02 rests on (revision 6, J-3,
// J-4, J-7; ADR-D02.7, .35, .36, .40).
internal static partial class Program
{
    private static SlotWords D02Words => SlotWords.Parse(Read("data/design/jumble-words.json"));
    private static Coverage D02Coverage(int tried = 9, int total = 9) => new("150", 3, "I_a/60", "menu order of x, then d", "jumble-contract-1", tried, total);
    private static ToolResult D02Done(long release, string? line = null, string digest = "rev-a", string parameters = "cap 150") =>
        new(release, digest, parameters, JobEnd.Completed, D02Coverage(), null, line == null ? null : new[] { line });

    // What the screen shows of a slot: its state, the result it shows, its reason and its word.
    private static string D02Screen(ToolSlot slot) => string.Join("|", slot.State, slot.Result?.Release.ToString() ?? "-", slot.Reason ?? "-", slot.Word(D02Words).Text);

    // J-4: only the latest release's result is taken, whatever order the results arrive in; a result whose digest or
    // parameters differ is discarded and the slot stays pending (J-7). Release and try numbers come from the record.
    private static void D02SlotLateResultIgnored()
    {
        var record = new ReleaseRecord(); var slot = new ToolSlot("slot.tool.jumble.release_check");
        long r1 = record.Next(), r2 = record.Next();
        Check(r2 > r1, "release numbers do not increase");
        slot.Request(r1, "rev-a", "cap 150"); slot.Request(r2, "rev-a", "cap 150");
        Check(!slot.Finish(D02Done(r1, "old")) && slot.State == SlotState.Pending, "an earlier release's result was taken");
        Check(!slot.Finish(D02Done(r2, digest: "rev-b")) && !slot.Finish(D02Done(r2, parameters: "cap 151")) && slot.State == SlotState.Pending, "a result for another input or other parameters was taken");
        Check(slot.Finish(D02Done(r2, "new")) && slot.State == SlotState.Ready && slot.Result!.Release == r2, "the latest result was not taken");
        Check(!slot.Finish(D02Done(r1, "late")) && slot.Result!.Lines![0] == "new", "a late result replaced the current one");
        RefuseArgument(() => slot.Request(r1, "rev-a", "cap 150"), "release numbers increase");
        // Try numbers count every release at a named cap and revision, superseded ones included, and restart at a commit.
        Sequence(new[] { record.TryNumber("150", 3), record.TryNumber("150", 3), record.TryNumber("151", 3), record.TryNumber("150", 3), record.TryNumber("150", 4) }, new[] { 1, 2, 1, 3, 1 }, "try numbers");
        Check(record.Superseded(r1) && !record.Superseded(r1) && record.IsSuperseded(r1) && !record.IsSuperseded(r2), "a superseded release is not recorded once");
        Check(record.Next() == r2 + 1, "the session release number jumped");
    }

    private static void RefuseArgument(Action action, string words)
    {
        try { action(); }
        catch (ArgumentException ex) { Check(ex.Message.Contains(words, StringComparison.Ordinal), "refusal did not say " + words + ": " + ex.Message); return; }
        throw new Exception("expected a refusal: " + words);
    }

    // J-7: cancelling the latest job returns the slot to its previous state and shows nothing of the cancelled job.
    private static void D02SlotCancelLatestRestores()
    {
        var slot = new ToolSlot("slot.tool.jumble.repair");
        slot.Request(1, "rev-a", "cap 150"); string empty = D02Screen(new ToolSlot("x"));
        Check(slot.Cancel(1) && slot.State == SlotState.Empty && D02Screen(slot) == empty, "a cancel from empty did not return to empty");
        Check(!slot.Finish(D02Done(1, "row")), "a cancelled job's result was taken");
        slot.Request(2, "rev-a", "cap 150"); slot.Finish(D02Done(2, "row 1")); string ready = D02Screen(slot);
        slot.Request(3, "rev-a", "cap 150"); Check(slot.State == SlotState.Pending, "a request is not pending");
        Check(slot.Cancel(3) && D02Screen(slot) == ready, "a cancel did not return to the ready result");
        // A job that ends cancelled does the same; superseded pending requests keep the state before them.
        slot.Request(4, "rev-a", "cap 150"); slot.Request(5, "rev-a", "cap 150");
        Check(slot.Finish(new ToolResult(5, "rev-a", "cap 150", JobEnd.Cancelled)) && D02Screen(slot) == ready, "a cancelled end did not return to the ready result");
    }

    // N-1, as the owner read it (ADR-D02.36): cancelling a job that is no longer the latest request changes nothing.
    private static void D02SlotCancelOlderChangesNothing()
    {
        var slot = new ToolSlot("slot.tool.jumble.release_check"); int changes = 0; slot.Changed += _ => changes++;
        slot.Request(1, "rev-a", "cap 150"); slot.Request(2, "rev-a", "cap 150");
        string before = D02Screen(slot); int seen = changes;
        Check(!slot.Cancel(1) && D02Screen(slot) == before && changes == seen, "cancelling an older job changed the screen");
        slot.Finish(D02Done(2, "check 2")); before = D02Screen(slot); seen = changes;
        foreach (long release in new long[] { 1, 2 }) Check(!slot.Cancel(release) && D02Screen(slot) == before && changes == seen, "a cancel after the result changed the screen");
        // The folded row is the newer release's to write, from the record, never the cancel's.
        var record = new ReleaseRecord(); long a = record.Next(), b = record.Next();
        Check(record.Superseded(a) && !record.IsSuperseded(b), "the newer release did not record the older as superseded");
    }

    // J-7: a completed result without coverage is an error; a cut-off is budget-stopped and makes no negative claim.
    private static void D02SlotCompletedWithoutCoverageIsError()
    {
        SlotWords words = D02Words; var slot = new ToolSlot("slot.tool.jumble.covers");
        slot.Request(1, "rev-a", "cap 150");
        Check(slot.Finish(new ToolResult(1, "rev-a", "cap 150", JobEnd.Completed)) && slot.State == SlotState.Error && slot.Result == null, "a completed result without coverage was taken");
        Check(slot.Word(words).Text.Contains("without coverage", StringComparison.Ordinal), "the error does not name the missing coverage");
        slot.Request(2, "rev-a", "cap 150"); slot.Finish(new ToolResult(2, "rev-a", "cap 150", JobEnd.Completed, D02Coverage(9, 9)));
        Check(slot.State == SlotState.Ready && slot.Word(words).Text == "none (9 of 9 tried)", "an empty completed list does not name its coverage: " + slot.Word(words).Text);
        slot.Request(3, "rev-a", "cap 150"); slot.Finish(new ToolResult(3, "rev-a", "cap 150", JobEnd.CutOff, D02Coverage(3, 9)));
        Check(slot.State == SlotState.BudgetStopped && slot.Word(words).Text == "Not found within budget (3 of 9)", "a cut-off: " + slot.Word(words).Text);
        slot.Request(4, "rev-a", "cap 150"); slot.Finish(new ToolResult(4, "rev-a", "cap 150", JobEnd.CutOff, null, "seam facts cut off"));
        Check(slot.Word(words).Text == "Incomplete: seam facts cut off", "an incomplete result: " + slot.Word(words).Text);
        slot.Request(5, "rev-a", "cap 150"); slot.Finish(new ToolResult(5, "rev-a", "cap 150", JobEnd.Error, null, "adapter stopped"));
        Check(slot.State == SlotState.Error && slot.Word(words).Text.Contains("adapter stopped", StringComparison.Ordinal), "an error does not show its reason");
    }

    // ADR-D02.40: each of the seven states has its word and one mark, the role whose cue is not colour.
    private static void D02SlotWordsCoverSevenStates()
    {
        SlotWords words = D02Words; DesignSystem system = Design; var seen = new Dictionary<SlotState, (string Text, string Mark)>();
        var slot = new ToolSlot("slot.tool.jumble.survey"); void Take() => seen[slot.State] = slot.Word(words);
        Take();
        Equal(slot.Word(words).Text, "Not run", "the empty slot's word");
        slot.Request(1, "rev-a", "all"); Take();
        slot.Finish(D02Done(1, "N 126100", "rev-a", "all")); Take();
        slot.Invalidate("rev-b"); Take();
        Check(slot.State == SlotState.Stale, "a new revision did not make the result stale");
        slot.Invalidate("rev-a");
        Check(slot.State == SlotState.Ready && slot.Result!.Release == 1, "back at its revision the result did not show again");
        slot.Refuse("No cap is named."); Take();
        slot.Request(2, "rev-a", "all"); slot.Finish(new ToolResult(2, "rev-a", "all", JobEnd.CutOff, D02Coverage(4, 9))); Take();
        slot.Request(3, "rev-a", "all"); slot.Finish(new ToolResult(3, "rev-a", "all", JobEnd.Error, null, "adapter stopped")); Take();
        Sequence(seen.Keys.OrderBy(s => s), Enum.GetValues<SlotState>(), "not every state was reached");
        Check(seen.Values.All(w => w.Text.Length > 0), "a state has no word");
        Equal(seen.Values.Select(w => w.Mark).Distinct().Count(), 7, "two states share a mark");
        foreach (var (state, w) in seen) Check(system.Roles.TryGetValue(w.Mark, out ColourRole? role) && !string.IsNullOrWhiteSpace(role!.Cue), state + "'s mark is not a cued role");
        Equal(seen[SlotState.Refused].Text, "No cap is named.", "a refusal does not name the failing fact");
        Sequence(JumbleSlots.Ids, new[] { "survey", "straddlers", "covers", "release_check", "repair", "nu", "corrections_check", "checkpoint" }.Select(s => "slot.tool.jumble." + s), "the eight slot ids");
        Check(JumbleSlots.Create().Values.All(s => s.State == SlotState.Empty), "a new slot is not empty");
    }

    // J-3: undefined is distinct from zero, pending and incomplete, and none of them shows a number.
    private static void D02NumeralKindsStayApart()
    {
        Numeral[] all = { Numeral.Of(0), Numeral.Undefined, Numeral.Pending, Numeral.Incomplete };
        Equal(all.Distinct().Count(), 4, "two numeral kinds are equal");
        Equal(all.Select(n => n.Role).Distinct().Count(), 4, "two numeral kinds share a role");
        Equal(all.Select(n => n.Word).Distinct().Count(), 4, "two numeral kinds share a word");
        Check(all.Skip(1).All(n => n.Text.Length == 0 && n.Number == null), "a state that is not a value shows a number");
        Check(Numeral.Of(0).Text == "0" && Numeral.Of(126100).Number == 126100, "a value does not show its numeral");
        Check(Design.Roles.ContainsKey(Numeral.Undefined.Role) && Numeral.Undefined.Role == "numeral.undefined", "undefined is not drawn by numeral.undefined");
        Check(all.All(n => Design.Roles.ContainsKey(n.Role)), "a numeral role is not a role");
    }
}
