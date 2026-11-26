using System;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

// ADR-D02 build step B25: word flows and signatures (ADR-D02.29, .31), on the seed 7001 restoration's nine words.
internal static partial class Program
{
    private static WordTree Seed7001() => WordTree.Read(Read("data/samples/jumble/seed7001-words.json"));

    private static (long N, bool OnLattice, int[]? Nu) SeedTotals(string state)
    {
        using JsonDocument d = JsonDocument.Parse(Read("data/samples/jumble/" + state + "/totals.json"));
        JsonElement nu = d.RootElement.GetProperty("nu");
        bool on = nu.GetProperty("result").GetString() != "undefined";
        return (d.RootElement.GetProperty("N").GetInt64(), on, on ? nu.GetProperty("nu").EnumerateArray().Select(v => v.GetInt32()).ToArray() : null);
    }

    // The stored word's length is its expanded length, the sum of its parts in word order; the tree folds them.
    private static void D02WordLengthIsExpanded()
    {
        WordTree tree = Seed7001();
        Check(tree.Parts.Count == 9 && tree.ExpandedLength == tree.Parts.Sum(p => p.Letters.Count), "the seed 7001 word has " + tree.Parts.Count + " parts and " + tree.ExpandedLength + " twists");
        string[] folded = tree.Folded().ToArray();
        Check(folded[0] == "seed 7001 restoration · " + JumbleWords.Twists(tree.ExpandedLength) && folded.Skip(1).Select(l => l.Trim().Split(" · ")[0]).SequenceEqual(Enumerable.Range(1, 9).Select(k => "word " + k)),
            "the folded tree reads " + string.Join(" | ", folded));
        Check(JumbleWords.LengthLine(tree.ExpandedLength, tree.Alphabet) == "9 twists · alphabet I_a", "the commit bar's line reads " + JumbleWords.LengthLine(tree.ExpandedLength, tree.Alphabet));
        // The parts chain: each starts where the one before ends, and the last ends solved.
        Check(tree.Parts[^1].OutputDigest == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "the restoration does not end solved");
        bool refused = false;
        var broken = tree with { Parts = tree.Parts.Reverse().ToArray() };
        try { WordTree.Read(JsonSerializer.Serialize(new { name = "x", alphabet = "I_a", parts = broken.Parts.Select(p => new { name = p.Name, letters = p.Letters.Select(l => new[] { l.Cap, l.Item }), input_digest = p.InputDigest, output_digest = p.OutputDigest }) })); }
        catch (FormatException) { refused = true; }
        Check(refused, "parts out of word order were read");
    }

    // Playback steps by part, and twist by twist inside an opened part; above the bound it shows numerals only.
    private static void D02PlaybackStepsByPartThenTwist()
    {
        WordTree tree = Seed7001();
        var play = new WordPlayback(tree);
        Check(play.Counter() == "part 0 of 9" && play.Digest == tree.Parts[0].InputDigest, "the playback starts at " + play.Counter());
        play.Forward(); play.Forward();
        Check(play.Counter() == "part 2 of 9" && play.Digest == tree.Parts[1].OutputDigest, "two parts forward read " + play.Counter());
        Check(play.Open() && play.Counter() == "part 3 of 9, twist 0 of 1", "the opened part reads " + play.Counter());
        play.Forward();
        Check(!play.Opened && play.Counter() == "part 3 of 9" && play.Digest == tree.Parts[2].OutputDigest, "the opened part's last twist did not close it: " + play.Counter());
        // A part of three twists steps twist by twist; back at its first twist closes it unplayed.
        var letters = new[] { new Letter(150, 1), new Letter(91, 50), new Letter(150, 1) };
        var three = new WordPlayback(new WordTree("repair", "I_a", new[] { new WordPart("row 1", letters, "a", "b"), new WordPart("row 2", letters, "b", "c") }));
        three.Open(); three.Forward(); three.Forward();
        Check(three.Counter() == "part 1 of 2, twist 2 of 3" && three.Shown()!.Count == 2, "twist stepping reads " + three.Counter());
        three.Back(); three.Back(); three.Back();
        Check(!three.Opened && three.Counter() == "part 0 of 2", "back at the first twist did not close the part: " + three.Counter());
        // Above X3's placeholder bound, numerals only.
        var longPart = new WordPart("long", Enumerable.Repeat(new Letter(1, 1), (int)JumbleWords.LongWordBound + 1).ToArray(), "a", "b");
        var big = new WordPlayback(new WordTree("long", "I_a", new[] { longPart }));
        big.Open(); big.Forward();
        Check(!big.ShowsLetters && big.Shown() == null && big.Counter() == "part 1 of 1, twist 1 of 10,001".Replace("10,001", (JumbleWords.LongWordBound + 1).ToString()), "a word above the bound shows letters: " + big.Counter());
        Check(JumbleWords.LongWordBoundLabel.Contains("placeholder") && JumbleWords.LongWordBoundLabel.Contains("X3"), "the bound is not labelled a placeholder");
    }

    // A word that leaves the lattice shows the undefined numeral in its slots, with its exact length and alphabet; a
    // lattice-to-lattice word shows 35 slots and its ν row.
    private static void D02OffLatticeWordSignatureUndefined()
    {
        WordTree tree = Seed7001();
        var start = SeedTotals("s7001-0");
        var end = SeedTotals("s7001-9");
        Check(!start.OnLattice && end.OnLattice && end.N == 0, "the seed 7001 states are not off and then on the lattice");
        var off = WordSignature.Of(start.OnLattice, end.OnLattice, null, null, tree.ExpandedLength, tree.Alphabet);
        Check(off.Slots.Count == 35 && off.Slots.All(s => s.Kind == NumeralKind.Undefined) && off.NuRow == null, "an off-lattice word has defined slots");
        Check(off.Line() == "35 slots undefined (the word leaves the lattice) · 9 twists · alphabet I_a", "the signature reads " + off.Line());
        using JsonDocument c = JsonDocument.Parse(Read("data/samples/jumble/corrections.json"));
        JsonElement correction = c.RootElement.GetProperty("catalogue").GetProperty("corrections")[0];
        int[] row = correction.GetProperty("row").EnumerateArray().Select(v => v.GetInt32()).ToArray();
        var lattice = WordSignature.Of(true, true, null, row, correction.GetProperty("letters").GetArrayLength(), "I_a");
        Check(lattice.Defined && lattice.NuRow!.Count == 45 && lattice.Slots.All(s => s.Kind == NumeralKind.Pending), "a lattice-to-lattice word without shipped slots is not pending: " + lattice.Line());
        // The log tab: residual ticks at N = 0, the undefined band with N otherwise.
        Check(LogTabMark.Of(start.N, start.Nu).Line() == "undefined band · N 13,270", "the off-lattice log tab reads " + LogTabMark.Of(start.N, start.Nu).Line());
        Check(LogTabMark.Of(end.N, end.Nu).Line() == "N = 0 · no residual ticks", "the solved log tab reads " + LogTabMark.Of(end.N, end.Nu).Line());
        Check(LogTabMark.Of(0, row).Ticks!.Count == row.Count(v => v != 0), "a ν row's ticks are not its nonzero entries");
    }

    // The crossing sentence shows only with the reachability that makes it true.
    private static void D02CrossingSentenceNeedsReachability()
    {
        var end = SeedTotals("s7001-9");
        bool crosses = end.OnLattice && end.Nu!.All(v => v == 0);
        Check(JumbleWords.CrossingSentence(crosses, true) == "On the lattice, ν = 0: labels pass to 600-cell-Full", "the crossing sentence is missing at solved");
        Check(JumbleWords.CrossingSentence(crosses, false) == null, "the crossing sentence shows without reachability");
        var off = SeedTotals("s7001-8");
        Check(JumbleWords.CrossingSentence(off.OnLattice, true) == null, "the crossing sentence shows off the lattice");
    }
}
