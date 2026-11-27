using System;
using System.Collections.Generic;
using Magic600.Chrome;

internal static partial class Program
{
    // ADR-D01.42: a free view names its source on a binding chip (the committed state, a preview, a saved reference or a
    // history position) in a word or two, with the sentence as its tooltip. A verified log is bound to its saved state;
    // a log whose compile fails and a script carry no chip.
    private static void D01FreeViewsNameTheirSource()
    {
        Equal(ViewBinding.Committed.Words, "committed", "the committed state");
        Equal(ViewBinding.Preview.Words, "preview", "a preview");
        Equal(ViewBinding.Saved.Words, "saved", "a saved reference");
        Equal(ViewBinding.At(12, 40).Words, "history 12 of 40", "a history position");
        Equal(ViewBinding.Committed.Tip, "Shows the committed state", "the committed state's sentence");
        Equal(ViewBinding.Preview.Tip, "Shows a preview: nothing here is committed", "a preview's sentence");
        Equal(ViewBinding.Saved.Tip, "Shows a saved reference, not the session's state", "a saved reference's sentence");
        Equal(ViewBinding.At(12, 40).Tip, "Shows the state after move 12 of 40", "a history position's sentence");
        Equal(ViewBinding.At(0, 40).Tip, "Shows the start, before move 1 of 40", "the start of the history");
        Equal(ViewBinding.At(40, 40).Words, "history 40 of 40", "the end of the history");
        var sources = new HashSet<BindingSource>();
        foreach (ViewBinding binding in new[] { ViewBinding.Committed, ViewBinding.Preview, ViewBinding.Saved, ViewBinding.At(3, 9) })
        {
            sources.Add(binding.Source);
            Check(binding.Words.Split(' ').Length <= 4, binding.Words + ": a chip holds a word or two");
            Check(binding.Tip.StartsWith("Shows ", StringComparison.Ordinal), binding.Tip + ": the tooltip names what the view shows");
        }
        Equal(sources.Count, Enum.GetValues<BindingSource>().Length, "every source has words");
        Check(ViewBinding.At(3, 9) == ViewBinding.At(3, 9) && ViewBinding.At(3, 9) != ViewBinding.At(4, 9), "history positions compare by value");
        foreach ((int position, int count) in new[] { (10, 9), (-1, 9), (0, -1) })
        {
            bool refused = false;
            try { ViewBinding.At(position, count); } catch (ArgumentOutOfRangeException) { refused = true; }
            Check(refused, $"history position {position} of {count} is refused");
        }
        foreach (string words in new[] { "committed", "preview", "saved" }) Equal(ViewBinding.Of(words)!.Words, words, "the words parse back");
        Check(ViewBinding.Of(null) == null && ViewBinding.Of("") == null && ViewBinding.Of("history 1 of 2") == null, "no words, no chip");

        // The documents: a verified sample log is bound to its saved state; a bad hash and a script carry no chip.
        var demo = new SolvingDemo(Atlas);
        MemoryDocumentStore store = SampleDocuments.Store(DocumentTestModel, demo.States, new Dictionary<string, string> { ["example.6cs"] = "edition sample" });
        var set = new DocumentSet(store, new SampleDocumentCompiler(DocumentTestModel, demo.States));
        Equal(ViewBinding.Of(set.Open(SampleDocuments.MidLogPath)!.Binding), ViewBinding.Saved, "a verified log");
        Equal(ViewBinding.Of(set.Open(SampleDocuments.BadLogPath)!.Binding), null, "a log whose compile fails");
        Equal(ViewBinding.Of(set.Open("example.6cs")!.Binding), null, "a script");
    }
}
