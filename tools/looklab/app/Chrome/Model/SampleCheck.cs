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
    // The addresses a twist may name: the 600 cells and the six axes of a cell (the lab's parameter ranges).
    public const int Cells = 600, Axes = 6;
    private static readonly Regex Pattern = new(@"^twist\(cell (\d{1,6}), axis (\d{1,3}), ([+-][12])\)$", RegexOptions.CultureInvariant);

    public sealed record Twist(int Cell, int Axis, int Turns)
    {
        public override string ToString() => string.Create(CultureInfo.InvariantCulture, $"twist(cell {Cell}, axis {Axis}, {(Turns > 0 ? "+" : "-")}{Math.Abs(Turns)})");
    }

    // A typed twist of a cell and an axis that exist, or null.
    public static Twist? Parse(string text) => Read(text) is { } t && Outside(t) == null ? t : null;

    private static Twist? Read(string text)
    {
        Match m = Pattern.Match(text.Trim());
        return m.Success ? new Twist(int.Parse(m.Groups[1].Value, CultureInfo.InvariantCulture), int.Parse(m.Groups[2].Value, CultureInfo.InvariantCulture), int.Parse(m.Groups[3].Value, CultureInfo.InvariantCulture)) : null;
    }

    private static string? Outside(Twist t) =>
        t.Cell >= Cells ? $"{t} names cell {t.Cell}, which does not exist: cells are 0 to {Cells - 1}"
        : t.Axis >= Axes ? $"{t} names axis {t.Axis}, which does not exist: axes are 0 to {Axes - 1}" : null;

    // Why a typed twist names no part of the puzzle, or null. A move that is not a typed twist has its own reason.
    public static string? Unknown(string move) => Read(move) is { } t ? Outside(t) : null;

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
        // A twist of a cell or an axis that does not exist is no witness (ADR-D01.8).
        if (moves.Select(Unknown).FirstOrDefault(r => r != null) is { } outside) return new PreviewCheck(PreviewStatus.Refused, outside, Array.Empty<OrbitEffect>(), "Nothing was checked.");
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
        if (Read(body) is { } twist) { string[] moves = { twist.ToString() }; return (moves, Check(moves)); }
        return (Array.Empty<string>(), new PreviewCheck(PreviewStatus.Refused, NoWitness, Array.Empty<OrbitEffect>(), "Nothing was prepared."));
    }
}

// A sample preview the lab stages (demo data). A sample with a word is checked by the sample rule, exactly as
// the same word typed, edited or replayed, so one witness always shows one collateral; a sample without a word
// keeps its stated refusal.
public sealed record PreviewSample(string Statement, string Source, string Origin, string Name, IReadOnlyList<string> Moves, string? Refusal = null)
{
    public PreviewCheck Check() => Refusal is { } why ? new PreviewCheck(PreviewStatus.Refused, why, Array.Empty<OrbitEffect>(), "Nothing was prepared.") : SampleCheck.Check(Moves);
}

public static class PreviewSamples
{
    private static string[] Twists(params (int Cell, int Axis, int Turns)[] moves) => moves.Select(m => new SampleCheck.Twist(m.Cell, m.Axis, m.Turns).ToString()).ToArray();

    public static readonly IReadOnlyList<PreviewSample> All = Array.AsReadOnly(new PreviewSample[]
    {
        new("preview my.corner_swap(cell 117)", "own macro", "macro", "my.corner_swap(cell 117)",
            Twists((117, 2, 1), (341, 0, 1), (117, 2, -1), (341, 0, -1), (117, 2, 1), (88, 3, 2), (117, 2, -1), (88, 3, -2))),
        new("preview run \"sweep.6cs\" on ring 4", "own program", "script", "sweep.6cs on ring 4",
            Twists((12, 1, 1), (13, 1, 1), (14, 1, 1), (15, 1, 1), (16, 1, 1), (12, 4, -1), (13, 4, -1), (14, 4, -1), (15, 4, -1), (16, 4, -1), (12, 1, -1), (13, 1, -1))),
        new("preview twist(cell 402, axis 1, +1)", "typed twist", "hand", "twist(cell 402, axis 1, +1)", Twists((402, 1, 1))),
        new("preview my.corner_swap(cell 9) |> conjugate(twist(cell 30, axis 0, +1))", "own macro", "macro", "conjugated corner swap",
            Twists((30, 0, 1), (9, 2, 1), (201, 0, 1), (9, 2, -1), (201, 0, -1), (30, 0, -1))),
        new("preview shared.pack.solve_layer(2)", "package by another author", "script", "shared.pack.solve_layer(2)", Array.Empty<string>(),
            "in a solving session a word from a package written by someone else is refused"),
    });
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
