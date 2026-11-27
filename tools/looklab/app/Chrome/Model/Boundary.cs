using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace Magic600.Chrome;

// Where a preview's word comes from (ADR-D02.33, .40): the solver's own keys, the Library, or a research run's output.
public enum Origin { Solver, Library, Research }

// A user event: the key press or click that a pick, a focus, an ordering or an apply answers. Only an input handler
// makes one; the model never does (ADR-D02.33: the program never picks, orders or stages a row by itself).
public sealed record UserEvent(string Kind, string Input)
{
    public static UserEvent Key(string key) => new("key", key);
    public static UserEvent Click(string target) => new("click", target);
}

// The boundary ledger (build step B13): every staging, focusing, ordering or applying names the user event that asked
// for it; one without counts here. A scripted session ends with a count of zero.
public sealed class BoundaryLedger
{
    public static readonly IReadOnlyList<string> Kinds = Array.AsReadOnly(new[] { "stage", "focus", "order", "apply", "scrub" });
    private readonly List<(string Kind, string What)> unasked = new();
    public int Count => unasked.Count;
    public IReadOnlyList<(string Kind, string What)> Unasked => unasked;
    public int Asked { get; private set; }

    // Records one act; returns whether a user event asked for it.
    public bool Record(string kind, string what, UserEvent? asked)
    {
        if (!Kinds.Contains(kind)) throw new ArgumentException("not a boundary act: " + kind);
        if (asked == null) { unasked.Add((kind, what)); return false; }
        Asked++;
        return true;
    }
}

// The preview's admission rule by origin (ADR-D02.40): in a solving session a research run's output is refused at
// preview, with its sentence; the solver's own words and the Library's are admitted.
public static class PreviewAdmission
{
    public const string ResearchRefused = "Refused at preview: uses a research run's output.";

    public static string? Admit(Origin origin, bool solvingSession) => solvingSession && origin == Origin.Research ? ResearchRefused : null;
}

// One letter of a word: a twist of a cap by an item of its menu, written "cap·item".
public readonly record struct Letter(int Cap, int Item)
{
    public override string ToString() => Cap.ToString(CultureInfo.InvariantCulture) + "·" + Item.ToString(CultureInfo.InvariantCulture);
}

// A copied repair row in a word (ADR-D02.32): its letters stay together, it is removed whole and never edited.
public sealed record SealedSpan(int Start, int Length, RepairRow Row);

// The staged word with its sealed spans. Typed twists are ordinary letters, even when they spell a repair row; only a
// pick makes a sealed span.
public sealed class StagedWord
{
    private readonly List<Letter> letters = new();
    private readonly List<SealedSpan> spans = new();
    public IReadOnlyList<Letter> Letters => letters;
    public IReadOnlyList<SealedSpan> Spans => spans;
    public event Action? Changed;

    // A typed twist: an ordinary letter at the end. Only RepairStage.Type calls it, after the word's origin is
    // admitted (C2-6).
    internal void Type(Letter letter) { letters.Add(letter); Safe.Raise(Changed); }

    internal SealedSpan Seal(RepairRow row, IReadOnlyList<Letter> word)
    {
        var span = new SealedSpan(letters.Count, word.Count, row);
        letters.AddRange(word); spans.Add(span); Safe.Raise(Changed);
        return span;
    }

    public void Clear()
    {
        if (letters.Count == 0) return;
        letters.Clear(); spans.Clear();
        Safe.Raise(Changed);
    }

    public SealedSpan? SpanAt(int index) => spans.FirstOrDefault(s => index >= s.Start && index < s.Start + s.Length);

    // Removes the letter at an index; inside a sealed span the whole span goes. Returns how many letters went.
    public int Remove(int index)
    {
        if (index < 0 || index >= letters.Count) return 0;
        SealedSpan? span = SpanAt(index);
        int start = span?.Start ?? index, length = span?.Length ?? 1;
        letters.RemoveRange(start, length);
        if (span != null) spans.Remove(span);
        for (int i = 0; i < spans.Count; i++)
            if (spans[i].Start > start) spans[i] = spans[i] with { Start = spans[i].Start - length };
        Safe.Raise(Changed);
        return length;
    }

    // Edits a letter in place; refused inside a sealed span, which is never edited.
    public bool Edit(int index, Letter letter)
    {
        if (index < 0 || index >= letters.Count || SpanAt(index) != null) return false;
        letters[index] = letter; Safe.Raise(Changed);
        return true;
    }

    // The word as text, a sealed span in brackets.
    public string Text()
    {
        var parts = new List<string>();
        for (int i = 0; i < letters.Count;)
        {
            if (SpanAt(i) is { } span && span.Start == i)
            {
                parts.Add("[" + string.Join(" ", letters.Skip(i).Take(span.Length)) + "]");
                i += span.Length;
            }
            else parts.Add(letters[i++].ToString());
        }
        return string.Join(" ", parts);
    }
}

// The repair-pair record a staged row writes to the sample preview (CR-15 writes it to the engine's preview with K2):
// the cap and revision, the row, and what the list ran under.
public sealed record RepairPairRecord(int Cap, int Revision, int Row, int X, int D, int Y, int Z, long DeltaN, string Pin, string Menu, string TieRule, string Asked);

// Staging a picked repair row (ADR-D02.33; build step B13): only a pick, carrying its user event, stages a row. The
// row becomes a sealed, whole-only fresh preview of the sample, which writes the repair-pair record; the bar reads
// "Whole repair only", and no prefix of it commits.
public sealed class RepairStage
{
    public StagedWord Word { get; } = new();
    public IReadOnlyList<RepairPairRecord> Records => records;
    private readonly List<RepairPairRecord> records = new();
    private readonly BoundaryLedger ledger;
    // Whether the session is a solving session, read from the session each time (ADR-D02.40, C2-6).
    private readonly Func<bool> solvingSession;
    public event Action? Changed;

    public RepairStage(BoundaryLedger ledger, Func<bool> solvingSession)
    {
        this.ledger = ledger; this.solvingSession = solvingSession; Word.Changed += () => Safe.Raise(Changed);
    }

    // The last refusal at admission, or null: a research run's output in a solving session.
    public string? Refusal { get; private set; }

    // Every staging entry admits its word's origin first (ADR-D02.40): a refused word stages nothing.
    private bool Admitted(Origin origin)
    {
        string? refusal = PreviewAdmission.Admit(origin, solvingSession());
        if (refusal != Refusal) { Refusal = refusal; Safe.Raise(Changed); }
        return refusal == null;
    }

    // A typed twist of the given origin, admitted first.
    public bool Type(Letter letter, Origin origin)
    {
        if (!Admitted(origin)) return false;
        Word.Type(letter);
        return true;
    }

    // The whole-only bar: on while the word holds a sealed span.
    public bool WholeOnly => Word.Spans.Count > 0;
    public string Bar => Refusal ?? (WholeOnly ? RepairListModel.WholeOnly : "");

    // A prefix of the word would commit part of a sealed span: refused while one is staged.
    public string? PrefixRefusal(int scrub) => WholeOnly && scrub < Word.Letters.Count ? RepairListModel.WholeOnly : null;

    // Stages a picked row: with no user event nothing is staged and the ledger counts it; a refused origin stages
    // nothing either.
    public SealedSpan? Stage(RepairListModel list, RepairRow row, RepairSteps steps, UserEvent? asked, Origin origin)
    {
        if (!ledger.Record("stage", "repair row " + row.Number + " at cap " + list.Cap, asked)) return null;
        // C2-3: only a completed list without an error stages a row.
        if (list.End != JobEnd.Completed || list.Error != null) return null;
        if (!Admitted(origin)) return null;
        if (!list.Rows.Contains(row)) throw new ArgumentException("the row is not in the list");
        var word = steps.Letters(row, list.Cap).Select(l => new Letter(l.Cap, l.Item)).ToArray();
        SealedSpan span = Word.Seal(row, word);
        Coverage? c = list.Coverage;
        records.Add(new RepairPairRecord(list.Cap, list.Revision, row.Number, row.X, row.D, row.Y, row.Z, row.DeltaN, list.Pin, c?.Menu ?? "", c?.TieRule ?? "", asked!.Kind + " " + asked.Input));
        Safe.Raise(Changed);
        return span;
    }

    // Removes the span under an index, whole.
    public int Remove(int index) => Word.Remove(index);
}
