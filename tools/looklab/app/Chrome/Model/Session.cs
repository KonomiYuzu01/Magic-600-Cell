using System;
using System.Collections.Generic;
using System.Linq;

namespace Magic600.Chrome;

// What the chrome shows of a session: the one pending preview and the two histories. The product fills these
// from the session store; the chrome only draws them and asks for commands. Nothing here computes a move.

public enum PreviewStatus { Ready, Impossible, Stale, Refused }

public sealed record OrbitEffect(string Orbit, int Moved, int Total);

// One built preview (ADR-D01.8): its staged .6cs statement, where it came from, the witness sequence, the
// effect on every orbit and on protection.
// What the stage of the preview view compares (ADR-D01.10): the committed state, the state after the whole
// sequence, or the two side by side.
public enum PreviewSide { Both, Before, After }

// The range a preview touches: how many orbits and how many pieces of them move over the whole sequence.
public sealed record AffectedRange(int Orbits, int OrbitsTotal, int Moved, int Total);

// An empty list of effects means the effects were not evaluated: only evaluated effects may say an orbit is
// untouched, and a preview that can commit always has them (ADR-D01.8).
public sealed record Preview(int Id, string Statement, string Source, IReadOnlyList<string> Moves, PreviewStatus Status,
    string? Reason, IReadOnlyList<OrbitEffect> Effects, string Protection)
{
    // The head of the move history the preview was built at (ADR-D01.9).
    public int Head { get; init; }
    // The preview's own check of a prefix, when its source has one: a draft's keeps the draft's protection policy.
    public Func<IReadOnlyList<string>, PreviewCheck>? PrefixCheck { get; init; }
    public bool Evaluated => Effects.Count > 0;
}

// What the host's check says of a sequence: its status, the reason when it cannot commit, its effects and
// protection.
public sealed record PreviewCheck(PreviewStatus Status, string? Reason, IReadOnlyList<OrbitEffect> Effects, string Protection);

// A session holds one pending preview (ADR-D01.8 to ADR-D01.12). A new one replaces the shown one under a new
// id; after a replacement the commit is ignored for a short settle interval, so a key-down never commits a
// preview that has not been seen.
public sealed class PreviewModel
{
    public const double Settle = .4;
    public Preview? Pending { get; private set; }
    public int Scrub { get; private set; }
    public bool Replaced { get; private set; }
    public PreviewSide Side { get; private set; }
    public event Action? Changed;
    private int next = 1;
    // The head the session is at now. A preview built at another head is stale and never commits (ADR-D01.9).
    public Func<int>? Head { get; set; }
    // The host's check of a prefix as its own preview (ADR-D01.12), for a preview without a prefix check of its
    // own. Without either, a prefix is refused.
    public Func<string, IReadOnlyList<string>, PreviewCheck>? Recheck { get; set; }
    public const string HeadMoved = "the committed state changed after this preview was built; stage it again";
    public const string NoPrefixCheck = "the prefix has no check of its own here, so it cannot be committed";
    private int Now => Head?.Invoke() ?? 0;

    public void SetSide(PreviewSide side) { if (side == Side) return; Side = side; Safe.Raise(Changed); }
    public void CycleSide() => SetSide((PreviewSide)(((int)Side + 1) % 3));
    public AffectedRange? Range() => Pending is not { Evaluated: true } p ? null
        : new AffectedRange(p.Effects.Count(e => e.Moved > 0), p.Effects.Count, p.Effects.Sum(e => e.Moved), p.Effects.Sum(e => e.Total));
    // How many of a row's cells stand for the moved share of an orbit: none only when nothing moves, all only
    // when everything does.
    public static int Lit(OrbitEffect effect, int cells) => effect.Moved <= 0 || effect.Total <= 0 ? 0 : effect.Moved >= effect.Total ? cells
        : Math.Clamp((int)Math.Round((double)cells * effect.Moved / effect.Total), 1, cells - 1);
    private double shownAt = double.NegativeInfinity;

    public Preview Show(string statement, string source, IReadOnlyList<string> moves, PreviewStatus status, string? reason,
        IReadOnlyList<OrbitEffect> effects, string protection, double now, Func<IReadOnlyList<string>, PreviewCheck>? prefixCheck = null)
    {
        if (status != PreviewStatus.Ready && string.IsNullOrWhiteSpace(reason)) throw J.Error("preview", "a preview that cannot commit states its reason");
        if (status == PreviewStatus.Ready && (effects.Count == 0 || moves.Count == 0)) throw J.Error("preview", "a preview that can commit has moves and evaluated effects");
        Replaced = Pending != null; if (Replaced) shownAt = now;
        Pending = new Preview(next++, statement, source, moves.ToArray(), status, status == PreviewStatus.Ready ? null : reason, effects.ToArray(), protection) { Head = Now, PrefixCheck = prefixCheck };
        Scrub = Pending.Moves.Count;
        Safe.Raise(Changed);
        return Pending;
    }

    public void SetScrub(int moves)
    {
        if (Pending == null) return;
        int to = Math.Clamp(moves, 0, Pending.Moves.Count);
        if (to != Scrub) { Scrub = to; Safe.Raise(Changed); }
    }

    public void MarkStale(string reason)
    {
        if (Pending == null) return;
        Pending = Pending with { Status = PreviewStatus.Stale, Reason = reason }; Safe.Raise(Changed);
    }

    // Called when the move history changes: a ready preview built at another head becomes stale, whatever moved
    // the head (a commit, a Direct twist, an undo or a switch of lines).
    public void Follow()
    {
        if (Pending is { Status: PreviewStatus.Ready } p && Head != null && p.Head != Now) MarkStale(HeadMoved);
    }

    // Why the pending preview cannot be committed now, or null when it can.
    public string? CommitRefusal(double now)
    {
        if (Pending == null) return "there is no preview";
        if (Pending.Status != PreviewStatus.Ready) return Pending.Reason;
        if (Head != null && Pending.Head != Now) return HeadMoved;
        if (Scrub == 0) return "the scrub is at the start; nothing would be committed";
        if (now - shownAt < Settle) return "the preview has just changed";
        return null;
    }

    public bool Discard()
    {
        if (Pending == null) return false;
        Pending = null; Scrub = 0; Replaced = false; Safe.Raise(Changed);
        return true;
    }

    // A scrub short of the end commits the prefix: in the same action it is prepared and rechecked as its own
    // preview (ADR-D01.12), and only a passing recheck commits.
    public bool AtPrefix => Pending != null && Scrub < Pending.Moves.Count;

    // Takes the preview for commit, or null. At a prefix the recheck decides: a prefix that fails it stays as the
    // pending preview, under its own id and with its own result, and nothing is taken.
    public Preview? Take(double now)
    {
        if (CommitRefusal(now) != null) return null;
        Preview taken = Pending!;
        if (Scrub < taken.Moves.Count)
        {
            IReadOnlyList<string> moves = taken.Moves.Take(Scrub).ToArray(); string statement = $"{taken.Statement} |> prefix({Scrub})";
            PreviewCheck check = taken.PrefixCheck?.Invoke(moves) ?? Recheck?.Invoke(statement, moves) ?? new PreviewCheck(PreviewStatus.Refused, NoPrefixCheck, Array.Empty<OrbitEffect>(), "Nothing was checked.");
            if (check.Status == PreviewStatus.Ready && check.Effects.Count == 0) check = check with { Status = PreviewStatus.Refused, Reason = "the recheck of the prefix gave no effects" };
            taken = new Preview(next++, statement, taken.Source, moves, check.Status, check.Status == PreviewStatus.Ready ? null : check.Reason ?? "the prefix failed its recheck",
                check.Effects.ToArray(), check.Protection) { Head = Now };
            if (taken.Status != PreviewStatus.Ready) { Pending = taken; Scrub = moves.Count; Replaced = true; shownAt = now; Safe.Raise(Changed); return null; }
        }
        Pending = null; Scrub = 0; Replaced = false; Safe.Raise(Changed);
        return taken;
    }
}

public sealed record HistoryEntry(string Id, string Origin, string Name, string Time, string Statement, IReadOnlyList<string> Moves);
public sealed record HistoryBranch(string From, IReadOnlyList<HistoryEntry> Entries);

// The move history (ADR-D01.14 to ADR-D01.16): committed transactions in order with a rollback cursor. Nothing
// is rewritten: abandoned redo steps stay as a recoverable branch, and every undo, redo, roll and partial undo
// is one recorded action.
public sealed class MoveHistory
{
    public static readonly IReadOnlyList<string> Origins = Array.AsReadOnly(new[] { "hand", "macro", "script" });
    public IReadOnlyList<HistoryEntry> Entries => entries;
    public IReadOnlyList<HistoryBranch> Branches => branches;
    public IReadOnlyList<string> Actions => actions;
    // Entries up to the cursor are applied. Inside the entry just before the cursor, Inner moves are undone.
    public int Cursor { get; private set; }
    public int Inner { get; private set; }
    // The head's identity: it changes with every recorded action, so a preview or a question asked at one head
    // is known to be older than the next.
    public int Revision { get; private set; }
    // The confirm gate of a branch switch (ADR-D01.16): the branch the solver was asked about, or -1. Any recorded
    // action closes the question.
    public int Asking { get; private set; } = -1;
    private int askedAt;
    public event Action? Changed;
    private readonly List<HistoryEntry> entries = new();
    private readonly List<HistoryBranch> branches = new();
    private readonly List<string> actions = new();
    private int next = 1;

    private void Did(string action) { actions.Add(action); Revision++; Asking = -1; Safe.Raise(Changed); }

    // Why a commit is refused now, or null.
    public string? CommitRefusal => Inner > 0 ? $"{entries[Cursor - 1].Id} is partly undone; redo it or finish the undo first" : null;

    public HistoryEntry? Commit(string origin, string name, string time, string statement, IReadOnlyList<string> moves)
    {
        if (!Origins.Contains(origin)) throw J.Error(origin, "unknown origin");
        if (moves.Count == 0) throw J.Error(name, "a transaction holds at least one move");
        if (CommitRefusal != null) return null;
        if (Cursor < entries.Count)
        {
            branches.Add(new HistoryBranch(Cursor == 0 ? "start" : entries[Cursor - 1].Id, entries.Skip(Cursor).ToArray()));
            entries.RemoveRange(Cursor, entries.Count - Cursor);
        }
        var entry = new HistoryEntry($"t-{next++:0000}", origin, name, time, statement, moves.ToArray());
        entries.Add(entry); Cursor = entries.Count;
        Did("commit " + entry.Id);
        return entry;
    }

    public bool Undo()
    {
        if (Cursor == 0) return false;
        string id = entries[Cursor - 1].Id; Cursor--; Inner = 0;
        Did("undo " + id);
        return true;
    }

    public bool Redo()
    {
        if (Inner > 0) { Inner = 0; Did("redo " + entries[Cursor - 1].Id); return true; }
        if (Cursor == entries.Count) return false;
        Cursor++; Did("redo " + entries[Cursor - 1].Id);
        return true;
    }

    // One move back inside the last applied entry; the action names the entry and the move (ADR-D01.15).
    public bool StepBack()
    {
        if (Cursor == 0) return false;
        HistoryEntry entry = entries[Cursor - 1];
        int move = entry.Moves.Count - Inner; Inner++;
        if (Inner == entry.Moves.Count) { Inner = 0; Cursor--; }
        Did($"partial undo {entry.Id} move {move}");
        return true;
    }

    private int At(string from) => from == "start" ? 0 : entries.FindIndex(e => e.Id == from) + 1 is var at && at > 0 ? at : -1;

    // Why a branch cannot become the line now, or null (ADR-D01.16).
    public string? SelectRefusal(int branch) =>
        branch < 0 || branch >= branches.Count ? "there is no such branch"
        : Inner > 0 ? $"{entries[Cursor - 1].Id} is partly undone; redo it or finish the undo first"
        : At(branches[branch].From) < 0 ? $"{branches[branch].From} is not on this line" : null;

    // The first request of a switch only asks; -1 closes the question. A branch that cannot be switched to now
    // is not asked about.
    public bool Ask(int branch)
    {
        if (branch >= 0 && SelectRefusal(branch) != null) return false;
        int to = branch < 0 ? -1 : branch;
        if (to == Asking && askedAt == Revision) return true;
        Asking = to; askedAt = Revision; Safe.Raise(Changed); return true;
    }

    // Why a confirmation of a switch to this branch is refused now, or null: only an answer to the question that
    // is open, for that branch and at the head it was asked at, switches lines.
    public string? ConfirmRefusal(int branch) =>
        Asking < 0 ? "no switch is waiting for its confirmation; ask first"
        : Asking != branch ? $"the open question is about branch {Asking}, not {branch}"
        : askedAt != Revision ? "the history changed after the question was asked; ask again"
        : SelectRefusal(branch);

    public bool Confirm(int branch) => ConfirmRefusal(branch) == null && SelectBranch(branch);

    // The branch becomes the line and is applied to its end. The later entries of the old line are kept as a
    // branch in its place: nothing is rewritten and nothing is lost.
    public bool SelectBranch(int branch)
    {
        if (SelectRefusal(branch) != null) return false;
        HistoryBranch taken = branches[branch]; int at = At(taken.From);
        HistoryEntry[] tail = entries.Skip(at).ToArray(); entries.RemoveRange(at, entries.Count - at); entries.AddRange(taken.Entries);
        branches.RemoveAt(branch); if (tail.Length > 0) branches.Insert(branch, new HistoryBranch(taken.From, tail));
        Cursor = entries.Count; Inner = 0;
        Did($"switch to the branch after {taken.From} ({taken.Entries.Count} {(taken.Entries.Count == 1 ? "entry" : "entries")})");
        return true;
    }

    // "Roll to here": one action that may take several steps.
    public bool RollTo(int cursor)
    {
        if (cursor < 0 || cursor > entries.Count || (cursor == Cursor && Inner == 0)) return false;
        int steps = Math.Abs(cursor - Cursor) + (Inner > 0 ? 1 : 0);
        Cursor = cursor; Inner = 0;
        Did($"roll to {(cursor == 0 ? "start" : entries[cursor - 1].Id)} ({steps} {(steps == 1 ? "step" : "steps")})");
        return true;
    }
}

public sealed record LayoutStep(string Name, string State);

// The layout history (ADR-D01.14): camera, layout, workspace and design-profile changes, apart from the move
// history and with its own keys. Each step holds the whole arrangement as the host stores it.
public sealed class LayoutHistory
{
    public const int Kept = 100;
    public IReadOnlyList<LayoutStep> Steps => steps;
    public int Cursor { get; private set; } = -1;
    public event Action? Changed;
    private readonly List<LayoutStep> steps = new();
    private string? merged;

    // A run of changes to one setting (a dragged knob) is one step: pass the same merge key for each.
    public bool Record(string name, string state, string? merge = null)
    {
        if (Cursor >= 0 && steps[Cursor].State == state) return false;
        if (merge != null && merge == merged && Cursor == steps.Count - 1 && Cursor > 0) { steps[Cursor] = new LayoutStep(name, state); Safe.Raise(Changed); return true; }
        merged = merge;
        steps.RemoveRange(Cursor + 1, steps.Count - Cursor - 1);
        steps.Add(new LayoutStep(name, state));
        if (steps.Count > Kept) steps.RemoveAt(0);
        Cursor = steps.Count - 1; Safe.Raise(Changed);
        return true;
    }

    public LayoutStep? Undo() => Move(-1);
    public LayoutStep? Redo() => Move(1);
    public LayoutStep? RollTo(int index) => index == Cursor ? null : Move(index - Cursor);

    private LayoutStep? Move(int by)
    {
        int to = Cursor + by;
        if (Cursor < 0 || to < 0 || to >= steps.Count) return null;
        Cursor = to; Safe.Raise(Changed);
        return steps[to];
    }
}

// One number of a staged statement that the commit bar offers for editing in place (ADR-D01.11).
public sealed record StatementParameter(string Name, int Value, int Start, int Length, int Min, int Max, bool Signed);

// Inline parameters: the numbers of the staged statement, each named by the word before it; a bare signed
// number is a count of turns. Text inside quotes is never a parameter. The ranges here are sample ranges.
public static class InlineParameters
{
    public const int Shown = 4;
    public const string Turns = "turns";
    private static readonly System.Text.RegularExpressions.Regex Pattern = new(@"(?<![\w.])(?:(?<name>[a-z_]+) (?<value>\d+)|(?<signed>[+-]\d+))(?![\w.])", System.Text.RegularExpressions.RegexOptions.CultureInvariant);

    private static (int Min, int Max) Range(string name) => name switch { "cell" => (0, 599), "axis" => (0, 5), "ring" => (0, 11), Turns => (-2, 2), _ => (0, 999) };

    public static IReadOnlyList<StatementParameter> Find(string statement)
    {
        // Quoted text keeps its place and loses its content, so a file name never reads as a number.
        char[] open = statement.ToCharArray(); bool quoted = false;
        for (int i = 0; i < open.Length; i++) { if (open[i] == '"') { quoted = !quoted; continue; } if (quoted) open[i] = '_'; }
        var found = new List<StatementParameter>();
        foreach (System.Text.RegularExpressions.Match m in Pattern.Matches(new string(open)))
        {
            bool signed = m.Groups["signed"].Success; System.Text.RegularExpressions.Group value = signed ? m.Groups["signed"] : m.Groups["value"];
            if (value.Length > 6) continue;
            string name = signed ? Turns : m.Groups["name"].Value; (int min, int max) = Range(name);
            found.Add(new StatementParameter(name, int.Parse(value.Value, System.Globalization.CultureInfo.InvariantCulture), value.Index, value.Length, min, max, signed));
        }
        return found;
    }

    // The statement with one parameter changed. The value stays inside the range; a count of turns is never zero.
    public static string With(string statement, StatementParameter parameter, int value)
    {
        int to = Math.Clamp(value, parameter.Min, parameter.Max);
        if (parameter.Signed && to == 0) to = parameter.Value > 0 ? -1 : 1;
        string text = parameter.Signed && to > 0 ? "+" + to.ToString(System.Globalization.CultureInfo.InvariantCulture) : to.ToString(System.Globalization.CultureInfo.InvariantCulture);
        return statement.Substring(0, parameter.Start) + text + statement.Substring(parameter.Start + parameter.Length);
    }
}
