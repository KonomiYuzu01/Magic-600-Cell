using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// The tool slots ADR-D02 rests on: ADR-D01.61 as revision 6 amends it (J-4, J-7), and ADR-D02.7, .35, .36 and .40.
// A slot takes a typed job result only when it answers the latest request: the same release, the same input digest
// and the same parameters. The model is pure C#; Ui/ToolSlotBox.cs draws one slot.
public enum SlotState { Empty, Pending, Ready, Stale, Refused, BudgetStopped, Error }

public enum JobEnd { Completed, Cancelled, CutOff, Error }

// What a completed result covered (revision 6, J-7): for the jumbled puzzle the cap, the order-pinned menu identity,
// the tie rule and the contract revision it ran under, and how many were tried of how many.
public sealed record Coverage(string Cap, int Revision, string Menu, string TieRule, string Contract, int Tried, int Total);

// One job result: the release and the input it answers, how it ended, its coverage, an error's or a cut-off's
// reason, and the result's own lines for the slot to show.
public sealed record ToolResult(long Release, string Digest, string Parameters, JobEnd End, Coverage? Coverage = null,
    string? Reason = null, IReadOnlyList<string>? Lines = null)
{
    public bool IsEmpty => Lines == null || Lines.Count == 0;
}

public sealed class ToolSlot
{
    public string Id { get; }
    public SlotState State { get; private set; } = SlotState.Empty;
    // The result the slot shows: ready, budget-stopped, or stale since it was shown.
    public ToolResult? Result { get; private set; }
    // Why the slot is refused or in error.
    public string? Reason { get; private set; }
    // The latest request's release, or none.
    public long? Latest { get; private set; }
    // The digest of the revision on screen.
    public string? Shown { get; private set; }

    private string? digest, parameters;
    private bool open;
    private (SlotState State, ToolResult? Result, string? Reason) settled = (SlotState.Empty, null, null);

    public ToolSlot(string id) { Id = id; }

    public event Action<ToolSlot>? Changed;

    // A request for the revision on screen. Release numbers increase within the session (J-4). The state before
    // the request is kept, so cancelling the latest job returns to it; a superseded pending request keeps the state
    // before it, since its result is never taken.
    public void Request(long release, string digest, string parameters)
    {
        if (Latest is long latest && release <= latest) throw new ArgumentException("release numbers increase: " + release + " after " + latest);
        if (State != SlotState.Pending) settled = (State, Result, Reason);
        Latest = release; this.digest = digest; this.parameters = parameters; Shown = digest; open = true;
        Set(SlotState.Pending, Result, null);
    }

    // A request refused before any job starts, naming the failing fact (ADR-D02.40).
    public void Refuse(string reason)
    {
        if (string.IsNullOrWhiteSpace(reason)) throw new ArgumentException("a refusal names the failing fact");
        open = false;
        Set(SlotState.Refused, null, reason);
    }

    // Takes a result only when it answers the latest open request; returns whether the slot took it. A result for an
    // earlier release never replaces the current one, whatever order the results arrive in (J-4); a result whose
    // digest or parameters do not match is discarded, and the slot stays pending (J-7).
    public bool Finish(ToolResult result)
    {
        if (!open || result.Release != Latest || result.Digest != digest || result.Parameters != parameters || result.Digest != Shown) return false;
        open = false;
        switch (result.End)
        {
            case JobEnd.Completed when result.Coverage == null:
                Set(SlotState.Error, null, "a completed result without coverage");
                break;
            case JobEnd.Completed:
                Set(SlotState.Ready, result, null);
                break;
            case JobEnd.CutOff:
                Set(SlotState.BudgetStopped, result, result.Reason);
                break;
            case JobEnd.Error:
                Set(SlotState.Error, null, string.IsNullOrWhiteSpace(result.Reason) ? "the job failed" : result.Reason);
                break;
            default:
                Set(settled.State, settled.Result, settled.Reason);
                break;
        }
        return true;
    }

    // Cancels a job. Cancelling the latest open request returns the slot to its previous state and shows nothing of
    // the cancelled job (J-7); cancelling a job that is no longer the latest request changes nothing (N-1).
    public bool Cancel(long release)
    {
        if (!open || release != Latest) return false;
        open = false;
        Set(settled.State, settled.Result, settled.Reason);
        return true;
    }

    // The revision on screen changed (a twist, commit, undo, redo or a branch switch). A shown result, a refusal and
    // an open request go stale; back at the revision a result was made for, the result shows again.
    public void Invalidate(string digest)
    {
        Shown = digest;
        if (Result != null && Result.Digest == digest && State == SlotState.Stale)
        {
            Set(Result.End == JobEnd.CutOff ? SlotState.BudgetStopped : SlotState.Ready, Result, Result.End == JobEnd.CutOff ? Result.Reason : null);
            return;
        }
        if (open && this.digest == digest) return;
        open = false;
        if (State is SlotState.Pending or SlotState.Ready or SlotState.BudgetStopped or SlotState.Error or SlotState.Refused)
            Set(SlotState.Stale, Result, null);
    }

    // The slot's word and its mark (ADR-D02.40): one word per state, from the typed result, each with the role whose
    // cue is its non-colour mark.
    public (string Text, string Mark) Word(SlotWords words)
    {
        string mark = SlotWords.Mark(State);
        return State switch
        {
            SlotState.Empty => (words.NotRun, mark),
            SlotState.Pending => (words.Pending, mark),
            SlotState.Ready when Result!.IsEmpty => (words.None(Result.Coverage!), mark),
            SlotState.Ready => (string.Join(" ", Result!.Lines!), mark),
            SlotState.Stale => (words.Stale, mark),
            SlotState.Refused => (Reason!, mark),
            SlotState.BudgetStopped => (Result!.Coverage is Coverage c && string.IsNullOrEmpty(Result.Reason) ? words.CutOff(c) : words.Incomplete(Result.Reason ?? "no coverage"), mark),
            _ => (words.Error(Reason ?? ""), mark),
        };
    }

    private void Set(SlotState state, ToolResult? result, string? reason)
    {
        State = state; Result = result; Reason = reason;
        Changed?.Invoke(this);
    }
}

// The slot words of ADR-D02.40, read from jumble-words.json by sentence id, with the placeholders filled.
public sealed class SlotWords
{
    private readonly Dictionary<string, string> templates;
    private SlotWords(Dictionary<string, string> templates) { this.templates = templates; }

    public string NotRun => templates["result.not_run"];
    public string Pending => "pending";
    public string Stale => "stale";
    public string None(Coverage c) => Fill("result.none", ("tried", c.Tried), ("total", c.Total));
    public string CutOff(Coverage c) => Fill("result.cut_off", ("tried", c.Tried), ("total", c.Total));
    public string Incomplete(string reason) => templates["result.incomplete"].Replace("{reason}", reason, StringComparison.Ordinal);
    public string Error(string reason) => "error: " + reason;

    public static string Mark(SlotState state) => "slot.tool." + state switch
    {
        SlotState.BudgetStopped => "budget_stopped",
        _ => state.ToString().ToLowerInvariant(),
    };

    public static SlotWords Parse(string text)
    {
        using JsonDocument d = J.Parse(text);
        if (J.Text(d.RootElement, "format") != "magic600-design-jumble-words") throw J.Error("format", "expected magic600-design-jumble-words");
        var templates = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (JsonElement s in d.RootElement.GetProperty("sentences").EnumerateArray())
            templates[J.Text(s, "id")] = s.TryGetProperty("template", out JsonElement t) ? t.GetString()! : J.Text(s, "text");
        foreach (string id in new[] { "result.not_run", "result.none", "result.cut_off", "result.incomplete" })
            if (!templates.ContainsKey(id)) throw J.Error(id, "a slot word is missing");
        return new SlotWords(templates);
    }

    private string Fill(string id, params (string Name, int Value)[] values) =>
        values.Aggregate(templates[id], (w, v) => w.Replace("{" + v.Name + "}", v.Value.ToString(CultureInfo.InvariantCulture), StringComparison.Ordinal));
}

// The host's release record (J-4; ADR-D02.35, .36): session release numbers, and try numbers at a named cap and
// revision, superseded releases counted, restarting at each commit (a new revision).
public sealed class ReleaseRecord
{
    private long next;
    private readonly Dictionary<(string Cap, int Revision), int> tries = new();
    private readonly HashSet<long> superseded = new();

    public long Next() => ++next;

    public int TryNumber(string cap, int revision)
    {
        tries.TryGetValue((cap, revision), out int last);
        tries[(cap, revision)] = last + 1;
        return last + 1;
    }

    // A release whose check never completes because a newer release was made: written when the newer one is made,
    // from the solver's own record, never from a cancelled job (ADR-D02.36).
    public bool Superseded(long release)
    {
        if (release <= 0 || release > next) throw new ArgumentOutOfRangeException(nameof(release), "not a release of this session");
        return superseded.Add(release);
    }

    public bool IsSuperseded(long release) => superseded.Contains(release);
}

// An exact numeral and the states that are not one (revision 6, J-3; ADR-D02.22): undefined is distinct from zero,
// from pending and from incomplete, and none of them shows a number.
public enum NumeralKind { Value, Undefined, Pending, Incomplete }

public readonly record struct Numeral(NumeralKind Kind, long Value)
{
    public static Numeral Of(long value) => new(NumeralKind.Value, value);
    public static readonly Numeral Undefined = new(NumeralKind.Undefined, 0);
    public static readonly Numeral Pending = new(NumeralKind.Pending, 0);
    public static readonly Numeral Incomplete = new(NumeralKind.Incomplete, 0);

    public long? Number => Kind == NumeralKind.Value ? Value : null;
    // The tabular numeral, or nothing: a state that is not a value shows its pattern, never a number.
    public string Text => Kind == NumeralKind.Value ? Value.ToString(CultureInfo.InvariantCulture) : "";
    // The role that draws it.
    public string Role => Kind switch
    {
        NumeralKind.Value => "text.primary",
        NumeralKind.Undefined => "numeral.undefined",
        NumeralKind.Pending => "state.pending",
        _ => "slot.tool.budget_stopped",
    };
    // The word read out in Tier 1 and by a screen reader.
    public string Word => Kind switch
    {
        NumeralKind.Value => Text,
        NumeralKind.Undefined => "undefined",
        NumeralKind.Pending => "pending",
        _ => "incomplete",
    };
}

// ADR-D02.7: the eight Jumble tool slots, by name.
public static class JumbleSlots
{
    public static readonly IReadOnlyList<string> Ids = Array.AsReadOnly(new[] { "survey", "straddlers", "covers", "release_check", "repair", "nu", "corrections_check", "checkpoint" }
        .Select(s => "slot.tool.jumble." + s).ToArray());

    public static IReadOnlyDictionary<string, ToolSlot> Create() => Ids.ToDictionary(id => id, id => new ToolSlot(id), StringComparer.Ordinal);
}
