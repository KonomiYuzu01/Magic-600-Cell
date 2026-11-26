using System;
using Magic600.Chrome;

// Step C-.40 (ADR-D01.40/.41): Local for both puzzles. The base puzzle draws cell 0's turning region with no D02 state
// and no naming; a selection re-frames it while auto-framing is on. The jumbled puzzle keeps the product's cap rule.
internal static partial class Program
{
    private static void D02LocalPlanDrawsCellZeroOnTheBasePuzzle()
    {
        var plan = new LocalPlan(jumble: false);
        Check(!plan.Jumble && plan.AutoFrame, "the base plan is Jumble on or starts with auto-framing off");
        // Local is never empty: the opening state, with nothing named, bound or selected, draws cell 0.
        Equal(plan.Follow(null, null, null), (int?)LocalPlan.Default, "the cell Local frames on the opening state");
        Equal(LocalPlan.Default, 0, "the default is the lowest cell id");
        Equal(plan.Cap, (int?)0, "the cap Local follows");
        Check(!plan.BySelection, "the default counts as a selection");
        Equal(plan.Words, "Local shows cell 0's turning region by default. " + LocalPlan.Hint, "the default's readout");
        // A Jumble naming or host cap means nothing on the base puzzle.
        Equal(plan.Follow(150, 7, null), (int?)0, "the base puzzle reads a Jumble cap");
        // ADR-D01.41: a selection re-frames Local on its sticker's cell; clearing it returns Local to cell 0.
        Equal(plan.Follow(null, null, 412), (int?)412, "the selection's cell");
        Check(plan.BySelection && plan.Cap == 0, "the selection did not frame, or it moved the cap");
        Equal(plan.Words, "Local frames cell 412, the selected sticker's cell. Clear the selection to return to cell 0.", "the selection's readout");
        Equal(plan.Follow(null, null, 412), (int?)412, "the same selection at the next step");
        Equal(plan.Follow(null, null, 9), (int?)9, "another selection");
        Equal(plan.Follow(null, null, null), (int?)0, "the cleared selection");
        Check(!plan.BySelection, "the cleared selection still frames");
        // Auto-framing off: Local stays on cell 0 and says so.
        plan.AutoFrame = false;
        Equal(plan.Follow(null, null, 33), (int?)0, "a selection with auto-framing off");
        Equal(plan.Words, "Local shows cell 0's turning region by default. Auto-framing is off.", "the readout with auto-framing off");
        plan.AutoFrame = true;
        Equal(plan.Follow(null, null, 33), (int?)33, "the newer selection once auto-framing is on again");
        // The default is a number, never read from the state.
        string source = Read("app/Chrome/Model/LocalPlan.cs");
        foreach (string word in new[] { "CapMapModel", "N_c", "Class", "OrderBy", "Sort", "Min(", "Max(" })
            Check(!source.Contains(word, StringComparison.Ordinal), "LocalPlan reads " + word);
    }

    private static void D02LocalPlanKeepsTheJumbleRule()
    {
        var plan = new LocalPlan(jumble: true);
        // No state bound and no cap named: the host gives none, and Local draws nothing.
        Equal(plan.Follow(null, null, null), (int?)null, "Local with nothing to draw");
        Check(plan.Words == null, "the plan words a Jumble Local");
        // The host's default cap while a state is bound; the naming wins over it.
        Equal(plan.Follow(null, 0, null), (int?)0, "the host's default cap");
        Equal(plan.Follow(150, 0, null), (int?)150, "the named cap");
        // A selection re-frames; naming another cap returns Local to the cap even while the selection stays.
        Equal(plan.Follow(150, 150, 412), (int?)412, "the selection's cell");
        Equal(plan.Follow(8, 8, 412), (int?)8, "a new naming after the selection");
        Equal(plan.Follow(8, 8, 412), (int?)8, "the kept selection after the naming");
        Equal(plan.Follow(8, 8, 5), (int?)5, "a newer selection");
        Equal(plan.Follow(8, 8, null), (int?)8, "the cleared selection");
        Check(plan.Words == null, "the plan words a Jumble Local");
    }
}
