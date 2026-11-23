using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.RegularExpressions;

namespace Magic600.Chrome;

// A sample check for a build without an engine. Demo data, marked as such wherever it shows: the rule below is
// not the 600-cell's mechanics. It gives a word of typed twists its effects on five sample orbits and a
// protection result, so that the chrome's gates can be shown and tested on evaluated effects: a prefix rechecked
// as its own preview, a replayed branch, an edited statement and a staged draft.
public static class SampleCheck
{
    public static readonly IReadOnlyList<string> Orbits = Array.AsReadOnly(new[] { "sample orbit 1", "sample orbit 2", "sample orbit 3", "sample orbit 4", "sample orbit 5" });
    public static readonly IReadOnlyList<int> Totals = Array.AsReadOnly(new[] { 120, 720, 1200, 600, 2400 });
    // The protected orbit of the samples: sample orbit 4 is locked.
    public const int Locked = 3;
    public const string LockedReason = "it moves a protected orbit: sample orbit 4 is locked";
    public const string NotTwists = "a move is not a typed twist, and Look Lab has no engine to check it";
    public const string Mark = " (sample rule, demo data)";
    private static readonly Regex Pattern = new(@"^twist\(cell (\d{1,3}), axis (\d), ([+-][12])\)$", RegexOptions.CultureInvariant);

    public sealed record Twist(int Cell, int Axis, int Turns)
    {
        public override string ToString() => string.Create(CultureInfo.InvariantCulture, $"twist(cell {Cell}, axis {Axis}, {(Turns > 0 ? "+" : "-")}{Math.Abs(Turns)})");
    }

    // A typed twist, or null.
    public static Twist? Parse(string text)
    {
        Match m = Pattern.Match(text.Trim());
        return m.Success ? new Twist(int.Parse(m.Groups[1].Value, CultureInfo.InvariantCulture), int.Parse(m.Groups[2].Value, CultureInfo.InvariantCulture), int.Parse(m.Groups[3].Value, CultureInfo.InvariantCulture)) : null;
    }

    // The word reduced: adjacent twists of one cell on one axis merge, and cancel when their turns sum to a
    // multiple of five. Null when a move is not a typed twist.
    public static IReadOnlyList<Twist>? Reduce(IReadOnlyList<string> moves)
    {
        var word = new List<Twist>();
        foreach (string move in moves)
        {
            if (Parse(move) is not { } t) return null;
            if (word.Count > 0 && word[^1].Cell == t.Cell && word[^1].Axis == t.Axis)
            {
                int turns = ((word[^1].Turns + t.Turns) % 5 + 5) % 5; if (turns > 2) turns -= 5;
                word.RemoveAt(word.Count - 1); if (turns != 0) word.Add(t with { Turns = turns });
            }
            else word.Add(t);
        }
        return word;
    }

    // How many pieces of a sample orbit one turn of a twist moves (the rule): the locked orbit only by twists on
    // axis 0 of a cell that is a multiple of three; every other orbit by one to four pieces.
    public static int Weight(Twist t, int orbit) => orbit == Locked ? (t.Axis == 0 && t.Cell % 3 == 0 ? 2 : 0) : 1 + (t.Cell + 7 * t.Axis + 3 * orbit) % 4;

    // The effect on every sample orbit, or null when a move is not a typed twist.
    public static IReadOnlyList<OrbitEffect>? Effects(IReadOnlyList<string> moves)
    {
        if (Reduce(moves) is not { } word) return null;
        return Orbits.Select((name, i) => new OrbitEffect(name, Math.Min(Totals[i], word.Sum(t => Math.Abs(t.Turns) * Weight(t, i))), Totals[i])).ToArray();
    }

    // The check of a word: refused when it cannot be evaluated, impossible when it moves the locked orbit (unless
    // the caller has its own protection check), ready otherwise.
    public static PreviewCheck Check(IReadOnlyList<string> moves, bool locked = true)
    {
        if (moves.Count == 0) return new PreviewCheck(PreviewStatus.Refused, "there is no move to check", Array.Empty<OrbitEffect>(), "Nothing was checked.");
        if (Effects(moves) is not { } effects) return new PreviewCheck(PreviewStatus.Refused, NotTwists, Array.Empty<OrbitEffect>(), "Nothing was checked.");
        int hit = effects[Locked].Moved;
        if (locked && hit > 0) return new PreviewCheck(PreviewStatus.Impossible, LockedReason, effects, $"Would move protected sample orbit 4: {hit} pieces." + Mark);
        return new PreviewCheck(PreviewStatus.Ready, null, effects, (locked ? "Protected orbits untouched." : "Effects evaluated; protection is the draft's own check.") + Mark);
    }

    public const string NoWitness = "no witness is built for the changed statement: Look Lab has no engine (demo)";

    // An edited statement needs its own witness (ADR-D01.11): a typed twist is its own witness and is checked
    // again; any other statement gets none here, so its preview is refused and cannot commit.
    public static (IReadOnlyList<string> Moves, PreviewCheck Check) Rebuilt(string statement)
    {
        string body = statement.StartsWith("preview ", StringComparison.Ordinal) ? statement.Substring(8) : statement;
        if (Parse(body) is { } twist) { string[] moves = { twist.ToString() }; return (moves, Check(moves)); }
        return (Array.Empty<string>(), new PreviewCheck(PreviewStatus.Refused, NoWitness, Array.Empty<OrbitEffect>(), "Nothing was prepared."));
    }
}

// Where a command came from. A typed line (the command bar, the console and the palette's typed mode) never
// writes the move history (ADR-D01.58): it cannot ask for a commit, and in Direct a twist typed there is staged
// as the preview instead of being committed. A twist key or a pointer in Direct commits its twist (ADR-D01.13).
public enum InputSurface { Key, Pointer, TypedLine }

public static class CommitPaths
{
    public const string TypedNeverCommits = "A typed line never commits.";
    public static string? TypedRefusal(string line) => Regex.IsMatch(line.Trim(), @"^(commit\b|operation\.preview\.commit\b)", RegexOptions.CultureInvariant) ? TypedNeverCommits : null;
    public static bool DirectTwistCommits(InputSurface surface) => surface != InputSurface.TypedLine;
}
