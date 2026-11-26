using System;
using Magic600.Chrome;

// Local is never empty (owner, 26 November 2026): the default cap while none is named, a named cap kept until another
// is named, and the empty state only while no state is bound.
internal static partial class Program
{
    private static void D02LocalCapDrawsTheLowestCapUntilOneIsNamed()
    {
        const string hint = "Name a cap: click a numbered cap on the cap map, or F1 › jumble.cap.name <id>";
        var local = new LocalCap();
        Check(local.Follow(null, false) == null && local.Words(hint) == null, "Local draws a cap with no state bound");
        Equal(local.Follow(null, true), (int?)LocalCap.Default, "the cap Local draws before any naming");
        Equal(LocalCap.Default, 0, "the default is the lowest cap id");
        Equal(local.Words(hint), "Local shows cap 0 by default; no cap is named. " + hint, "the default's readout");
        Check(local.Named == null && local.Last == null, "the default counts as named");
        Equal(local.Follow(150, true), (int?)150, "the named cap");
        Check(local.Words(hint) == null, "the named cap's Local says it is a default");
        // An undo or a twist keeps the naming; the same cap follows.
        Equal(local.Follow(150, true), (int?)150, "the named cap after an undo");
        // Un-named, Local keeps the cap named last and says that no cap is named now.
        Equal(local.Follow(null, true), (int?)150, "the cap Local draws after an un-naming");
        Equal(local.Words(hint), "Local shows cap 150, the cap named last; no cap is named now. " + hint, "the kept cap's readout");
        Equal(local.Follow(7, true), (int?)7, "another named cap");
        Equal(local.Follow(null, false), (int?)null, "Local with the state unbound");
        Check(local.Words(hint) == null, "words with nothing drawn");
        // The default is a number, never read from the state: the model names no class, count or order.
        string source = Read("app/Chrome/Model/LocalCap.cs");
        foreach (string word in new[] { "CapMapModel", "N_c", "Class", "OrderBy", "Sort", "Min(", "Max(" })
            Check(!source.Contains(word, StringComparison.Ordinal), "LocalCap reads " + word);
    }
}
