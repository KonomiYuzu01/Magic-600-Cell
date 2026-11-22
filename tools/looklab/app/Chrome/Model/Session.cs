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
public sealed record Preview(int Id, string Statement, string Source, IReadOnlyList<string> Moves, PreviewStatus Status,
    string? Reason, IReadOnlyList<OrbitEffect> Effects, string Protection);

// A session holds one pending preview (ADR-D01.8 to ADR-D01.12). A new one replaces the shown one under a new
// id; after a replacement the commit is ignored for a short settle interval, so a key-down never commits a
// preview that has not been seen.
public sealed class PreviewModel
{
    public const double Settle = .4;
    public Preview? Pending { get; private set; }
    public int Scrub { get; private set; }
    public bool Replaced { get; private set; }
    public event Action? Changed;
    private int next = 1;
    private double shownAt = double.NegativeInfinity;

    public Preview Show(string statement, string source, IReadOnlyList<string> moves, PreviewStatus status, string? reason,
        IReadOnlyList<OrbitEffect> effects, string protection, double now)
    {
        if (status != PreviewStatus.Ready && string.IsNullOrWhiteSpace(reason)) throw J.Error("preview", "a preview that cannot commit states its reason");
        Replaced = Pending != null; if (Replaced) shownAt = now;
        Pending = new Preview(next++, statement, source, moves.ToArray(), status, status == PreviewStatus.Ready ? null : reason, effects.ToArray(), protection);
        Scrub = Pending.Moves.Count;
        Changed?.Invoke();
        return Pending;
    }

    public void SetScrub(int moves)
    {
        if (Pending == null) return;
        int to = Math.Clamp(moves, 0, Pending.Moves.Count);
        if (to != Scrub) { Scrub = to; Changed?.Invoke(); }
    }

    public void MarkStale(string reason)
    {
        if (Pending == null) return;
        Pending = Pending with { Status = PreviewStatus.Stale, Reason = reason }; Changed?.Invoke();
    }

    // Why the pending preview cannot be committed now, or null when it can.
    public string? CommitRefusal(double now)
    {
        if (Pending == null) return "there is no preview";
        if (Pending.Status != PreviewStatus.Ready) return Pending.Reason;
        if (Scrub == 0) return "the scrub is at the start; nothing would be committed";
        if (now - shownAt < Settle) return "the preview has just changed";
        return null;
    }

    public bool Discard()
    {
        if (Pending == null) return false;
        Pending = null; Scrub = 0; Replaced = false; Changed?.Invoke();
        return true;
    }

    // Takes the preview for commit. A scrub short of the end commits the prefix, which is its own preview with
    // its own id (ADR-D01.12).
    public Preview? Take(double now)
    {
        if (CommitRefusal(now) != null) return null;
        Preview taken = Pending!;
        if (Scrub < taken.Moves.Count) taken = taken with { Id = next++, Moves = taken.Moves.Take(Scrub).ToArray(), Statement = $"{taken.Statement} |> prefix({Scrub})" };
        Pending = null; Scrub = 0; Replaced = false; Changed?.Invoke();
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
    public event Action? Changed;
    private readonly List<HistoryEntry> entries = new();
    private readonly List<HistoryBranch> branches = new();
    private readonly List<string> actions = new();
    private int next = 1;

    private void Did(string action) { actions.Add(action); Changed?.Invoke(); }

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
        if (merge != null && merge == merged && Cursor == steps.Count - 1 && Cursor > 0) { steps[Cursor] = new LayoutStep(name, state); Changed?.Invoke(); return true; }
        merged = merge;
        steps.RemoveRange(Cursor + 1, steps.Count - Cursor - 1);
        steps.Add(new LayoutStep(name, state));
        if (steps.Count > Kept) steps.RemoveAt(0);
        Cursor = steps.Count - 1; Changed?.Invoke();
        return true;
    }

    public LayoutStep? Undo() => Move(-1);
    public LayoutStep? Redo() => Move(1);
    public LayoutStep? RollTo(int index) => index == Cursor ? null : Move(index - Cursor);

    private LayoutStep? Move(int by)
    {
        int to = Cursor + by;
        if (Cursor < 0 || to < 0 || to >= steps.Count) return null;
        Cursor = to; Changed?.Invoke();
        return steps[to];
    }
}
