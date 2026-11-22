using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace Magic600.Chrome;

// The five states a panel can be in besides ready (ADR-D01.33): each has its own word and glyph, and says why.
public enum PanelCondition { Ready, Empty, Loading, Error, Stale, Busy }

public sealed record PanelState(PanelCondition Condition, string Text);

public sealed class PanelStates
{
    public static readonly IReadOnlyList<PanelCondition> Five = Array.AsReadOnly(new[] { PanelCondition.Empty, PanelCondition.Loading, PanelCondition.Error, PanelCondition.Stale, PanelCondition.Busy });
    private static readonly PanelState ReadyState = new(PanelCondition.Ready, "");
    private readonly Dictionary<string, PanelState> states = new(StringComparer.Ordinal);
    private readonly SortedSet<string> known = new(StringComparer.Ordinal);
    public IReadOnlyCollection<string> Known => known;
    public event Action? Changed;
    // A panel's own control asks: "retry" after an error, "refresh" when stale.
    public event Action<string, string>? Asked;

    public void Register(string id) => known.Add(id);
    public PanelState Of(string id) => states.TryGetValue(id, out PanelState? state) ? state : ReadyState;

    // Empty, loading and error take the content's place; stale and busy keep it in view under a banner.
    public static bool HidesContent(PanelCondition condition) => condition is PanelCondition.Empty or PanelCondition.Loading or PanelCondition.Error;
    public static string Word(PanelCondition condition) => condition.ToString();
    // What the solver can do about it from the panel itself, or null.
    public static string? Action(PanelCondition condition) => condition switch { PanelCondition.Error => "retry", PanelCondition.Stale => "refresh", _ => null };

    public void Set(string id, PanelCondition condition, string text = "")
    {
        if (condition == PanelCondition.Ready) { if (states.Remove(id)) Safe.Raise(Changed); return; }
        // A state always says what it is about: an error its reason, a stale panel what it is older than.
        if (string.IsNullOrWhiteSpace(text)) throw J.Error(id, "a panel state says why");
        states[id] = new PanelState(condition, text.Trim()); Safe.Raise(Changed);
    }

    public void Clear() { if (states.Count == 0) return; states.Clear(); Safe.Raise(Changed); }
    public void Ask(string id, string action) => Safe.Raise(Asked, id, action);
}

public enum LampState { Idle, Active, Attention }

// One readout cell of the top bar: its value, its expanded line and its lamp, each with words.
public sealed record ReadoutFact(string Value, string Detail, LampState Lamp, string LampSays);

public static class HistoryReadouts
{
    public static ReadoutFact Revision(MoveHistory h, int first)
    {
        string head = h.Cursor == 0 ? "start" : h.Entries[h.Cursor - 1].Id; bool partly = h.Inner > 0;
        return new ReadoutFact("r " + (first + h.Cursor).ToString("0000", CultureInfo.InvariantCulture),
            partly ? $"head {head}, {h.Inner} of its {h.Entries[h.Cursor - 1].Moves.Count} moves undone" : $"head {head}, whole",
            partly ? LampState.Attention : LampState.Idle, partly ? "an entry is partly undone" : "the committed state is whole");
    }

    public static ReadoutFact Position(MoveHistory h)
    {
        bool behind = h.Cursor < h.Entries.Count;
        return new ReadoutFact($"{h.Cursor}/{h.Entries.Count}", $"{h.Cursor} applied of {h.Entries.Count} kept, {h.Branches.Count} {(h.Branches.Count == 1 ? "branch" : "branches")}",
            behind ? LampState.Active : LampState.Idle, behind ? "behind the newest entry" : "at the newest entry");
    }

    public static ReadoutFact Queue(int active) => new(active.ToString(CultureInfo.InvariantCulture), active == 0 ? "no script run waits" : $"{active} waiting or running",
        active > 0 ? LampState.Active : LampState.Idle, active > 0 ? "script runs are waiting or running" : "the run queue is empty");
}
