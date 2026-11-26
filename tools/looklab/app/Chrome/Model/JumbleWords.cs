using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// ADR-D02 build step B25: word flows and signatures (ADR-D02.29, .31).

// One named part of a stored word: its letters and the digests of the states before and after it.
public sealed record WordPart(string Name, IReadOnlyList<Letter> Letters, string InputDigest, string OutputDigest);

// A stored word with named spans, shown folded in word order with expanded lengths (ADR-D02.31).
public sealed record WordTree(string Name, string Alphabet, IReadOnlyList<WordPart> Parts)
{
    public long ExpandedLength => Parts.Sum(p => (long)p.Letters.Count);

    // The folded tree: the word's line, then each part's in word order, each with its expanded length.
    public IReadOnlyList<string> Folded() =>
        new[] { Name + " · " + JumbleWords.Twists(ExpandedLength) }.Concat(Parts.Select(p => "  " + p.Name + " · " + JumbleWords.Twists(p.Letters.Count))).ToArray();

    // The sample's stored word (seed7001-words.json): its parts in word order, each checked to follow the one before.
    public static WordTree Read(string json)
    {
        using JsonDocument document = JsonDocument.Parse(json);
        JsonElement root = document.RootElement;
        var parts = root.GetProperty("parts").EnumerateArray().Select(p => new WordPart(
            p.GetProperty("name").GetString()!,
            p.GetProperty("letters").EnumerateArray().Select(l => new Letter(l[0].GetInt32(), l[1].GetInt32())).ToArray(),
            p.GetProperty("input_digest").GetString()!, p.GetProperty("output_digest").GetString()!)).ToArray();
        for (int i = 1; i < parts.Length; i++)
            if (parts[i].InputDigest != parts[i - 1].OutputDigest) throw new FormatException(parts[i].Name + " does not start where " + parts[i - 1].Name + " ends");
        return new WordTree(root.GetProperty("name").GetString()!, root.GetProperty("alphabet").GetString()!, parts);
    }
}

public static class JumbleWords
{
    // X3's bound on a long word's playback, a labelled placeholder until X3 measures it ([data: X3]).
    public const long LongWordBound = 10_000;
    public const string LongWordBoundLabel = "placeholder bound, until X3 measures it";
    public const string Crossing = "On the lattice, ν = 0: labels pass to 600-cell-Full";

    public static string Twists(long n) => n.ToString("N0", CultureInfo.InvariantCulture) + (n == 1 ? " twist" : " twists");

    // The commit bar's line: the word's exact expanded length with its alphabet.
    public static string LengthLine(long length, string alphabet) => Twists(length) + " · alphabet " + alphabet;

    // The crossing sentence, shown only with the reachability that makes it true: the word ends on the lattice with
    // ν = 0, and the view shows that reachability.
    public static string? CrossingSentence(bool endsOnLatticeAtNuZero, bool reachabilityShown) =>
        endsOnLatticeAtNuZero && reachabilityShown ? Crossing : null;
}

// Playback of a stored word (ADR-D02.31): by part, and twist by twist inside an opened part. Above the long-word bound
// it shows numerals only, never the letters.
public sealed class WordPlayback
{
    public WordTree Tree { get; }
    // How many parts are played, and inside the opened part how many of its twists.
    public int Done { get; private set; }
    public int Twist { get; private set; }
    public bool Opened { get; private set; }
    public event Action? Changed;

    public WordPlayback(WordTree tree) { Tree = tree; }

    public int Parts => Tree.Parts.Count;
    public bool ShowsLetters => Tree.ExpandedLength <= JumbleWords.LongWordBound;
    private int PartLength => Tree.Parts[Done].Letters.Count;

    // Opens the next part for twist-by-twist playback; a played-out word opens nothing.
    public bool Open() { if (Opened || Done >= Parts) return false; Opened = true; Twist = 0; Safe.Raise(Changed); return true; }

    // One step forward: a whole part, or one twist of the opened part; its last twist closes it as played.
    public bool Forward()
    {
        if (Done >= Parts) return false;
        if (!Opened) { Done++; Safe.Raise(Changed); return true; }
        Twist++;
        if (Twist >= PartLength) { Opened = false; Twist = 0; Done++; }
        Safe.Raise(Changed);
        return true;
    }

    // One step back: a whole part, or one twist of the opened part; at its first twist it closes unplayed.
    public bool Back()
    {
        if (Opened) { if (Twist > 0) Twist--; else Opened = false; Safe.Raise(Changed); return true; }
        if (Done == 0) return false;
        Done--; Safe.Raise(Changed); return true;
    }

    // "part j of p", and inside an opened part "part j of p, twist k of n".
    public string Counter()
    {
        string part = "part " + (Opened ? Done + 1 : Done).ToString(CultureInfo.InvariantCulture) + " of " + Parts.ToString(CultureInfo.InvariantCulture);
        return Opened ? part + ", twist " + Twist.ToString(CultureInfo.InvariantCulture) + " of " + PartLength.ToString(CultureInfo.InvariantCulture) : part;
    }

    // The letters of the opened part as far as played, or null above the bound (numerals only).
    public IReadOnlyList<Letter>? Shown() => !ShowsLetters ? null : Opened ? Tree.Parts[Done].Letters.Take(Twist).ToArray() : Array.Empty<Letter>();

    // The state digest the playback stands at: the last played part's output, or the word's start.
    public string Digest => Done == 0 ? Tree.Parts[0].InputDigest : Tree.Parts[Done - 1].OutputDigest;
}

// A word's signature (ADR-D02.29): a lattice-to-lattice word shows its static 35-slot signature and its ν row, from
// the build's data; a word that leaves the lattice shows the undefined numeral in its slots, with its exact length and
// alphabet.
public sealed record WordSignature(IReadOnlyList<Numeral> Slots, IReadOnlyList<int>? NuRow, long Length, string Alphabet)
{
    public const int SlotCount = 35;

    public static WordSignature Of(bool startsOnLattice, bool endsOnLattice, IReadOnlyList<long>? slots, IReadOnlyList<int>? nuRow, long length, string alphabet)
    {
        if (!startsOnLattice || !endsOnLattice)
            return new WordSignature(Enumerable.Repeat(Numeral.Undefined, SlotCount).ToArray(), null, length, alphabet);
        if (slots != null && slots.Count != SlotCount) throw new ArgumentException("a signature has 35 slots");
        // Data the build has not shipped yet is pending, never zero.
        var shown = slots?.Select(Numeral.Of).ToArray() ?? Enumerable.Repeat(Numeral.Pending, SlotCount).ToArray();
        return new WordSignature(shown, nuRow, length, alphabet);
    }

    public bool Defined => Slots.All(s => s.Kind != NumeralKind.Undefined);

    public string Line() => (Defined ? "35 slots" + (NuRow == null ? "" : " · ν row of " + NuRow.Count.ToString(CultureInfo.InvariantCulture)) : "35 slots undefined (the word leaves the lattice)")
        + " · " + JumbleWords.LengthLine(Length, Alphabet);
}

// A Jumble-on log tab (ADR-D02.29): residual ticks when saved at N = 0, otherwise the undefined band with N.
public sealed record LogTabMark(IReadOnlyList<int>? Ticks, long N)
{
    public static LogTabMark Of(long n, IReadOnlyList<int>? nuRow) =>
        n == 0 && nuRow != null ? new LogTabMark(nuRow.Select((v, i) => (v, i)).Where(x => x.v != 0).Select(x => x.i).ToArray(), 0) : new LogTabMark(null, n);

    public string Line() => Ticks is { } t
        ? (t.Count == 0 ? "N = 0 · no residual ticks" : "N = 0 · residual ticks at " + string.Join(" ", t))
        : "undefined band · N " + N.ToString("N0", CultureInfo.InvariantCulture);
}
