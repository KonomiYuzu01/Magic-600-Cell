using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace Magic600.Chrome;

public sealed class SessionTimer
{
    public const string Certifies = "The timer certifies nothing and is not a record of the solve.";

    private double accumulated;
    private double? startedAt;
    public bool Running => startedAt.HasValue;
    public string Row => Running ? "session.timer.stop" : "session.timer.start";
    public event Action? Changed;

    public double Elapsed(double now) => accumulated + (startedAt is { } started ? now - started : 0);

    public string? Start(double now)
    {
        if (Running) return "the timer is already running";
        startedAt = now;
        Safe.Raise(Changed);
        return null;
    }

    public string? Stop(double now)
    {
        if (!Running) return "the timer is already paused";
        accumulated = Elapsed(now);
        startedAt = null;
        Safe.Raise(Changed);
        return null;
    }

    public static string Text(double seconds)
    {
        long whole = (long)seconds;
        long hours = whole / 3600;
        long minutes = whole / 60 % 60;
        long remainder = whole % 60;
        return hours == 0
            ? FormattableString.Invariant($"{minutes}:{remainder:00}")
            : FormattableString.Invariant($"{hours}:{minutes:00}:{remainder:00}");
    }
}

public sealed record SessionFacts(int Entries, int Moves, int Completions, string? LogHead, int? Revision,
    string? StateHash, string? Session, string? Model, string? Build);
public sealed record SessionFact(string Name, string Value, bool Waiting);
public sealed record SessionLamp(string Slot, string Word, bool Lit, string Tip);

public sealed class SessionCellModel
{
    private SessionFacts facts = new(0, 0, 0, null, null, null, null, null, null);
    private bool recording;
    private bool practice;
    private int? seenRevision;

    public bool Expanded { get; private set; }
    public event Action? Changed;

    public void Toggle()
    {
        Expanded = !Expanded;
        if (Expanded) seenRevision = facts.Revision;
        Safe.Raise(Changed);
    }

    public void Update(SessionFacts facts, bool recording, bool practice)
    {
        if (this.facts == facts && this.recording == recording && this.practice == practice) return;
        this.facts = facts;
        this.recording = recording;
        this.practice = practice;
        // The first known revision establishes the baseline, and an expanded cell shows each update.
        if (Expanded || seenRevision == null) seenRevision = facts.Revision;
        Safe.Raise(Changed);
    }

    public IReadOnlyList<SessionLamp> Lamps
    {
        get
        {
            bool changed = facts.Revision is { } revision && revision != seenRevision;
            return new SessionLamp[]
            {
                new("lamp.recording", "recording", recording, recording ? "Recording is on" : "Recording is off"),
                new("lamp.revision", "revision", changed, changed ? "Revision changed since this cell was opened" : "Revision lamp is idle"),
                new("lamp.practice", "practice", practice, practice ? "Practice copy" : "Practice is off"),
            };
        }
    }

    public IReadOnlyList<SessionFact> Facts() => new[]
    {
        Fact("entries", facts.Entries.ToString(CultureInfo.InvariantCulture)),
        Fact("moves", facts.Moves.ToString(CultureInfo.InvariantCulture)),
        Fact("completions", facts.Completions.ToString(CultureInfo.InvariantCulture)),
        Fact("log head", facts.LogHead),
        Fact("revision", facts.Revision?.ToString(CultureInfo.InvariantCulture)),
        Fact("state hash", facts.StateHash),
        Fact("session", facts.Session),
        Fact("model", facts.Model),
        Fact("build", facts.Build),
    };

    private static SessionFact Fact(string name, string? value) => new(name, value ?? "waits for its row", value == null);
}

public static class PracticeMark
{
    public const string Slot = "mark.practice";

    public static IReadOnlyList<ViewMark> BorderMarks(bool practice, string view) => practice
        ? new[] { new ViewMark(Slot, "practice copy", Slot, "This is a practice copy.") }
        : Array.Empty<ViewMark>();
}

public sealed record CompletionEntry(int Orbit, string Name, int Revision, string? AfterEntry, bool Moment, bool Sample);

public sealed class CompletionLog
{
    private readonly List<CompletionEntry> entries = new();
    public IReadOnlyList<CompletionEntry> Entries { get; }
    public event Action? Changed;

    public CompletionLog() => Entries = entries.AsReadOnly();

    public IReadOnlyList<CompletionEntry> Check(SolvingState state, int revision, string? afterEntry, bool sample)
    {
        var added = new List<CompletionEntry>();
        foreach (int orbit in state.Atlas.StageOrder)
        {
            if (!state.Residuals[orbit].ExactlySolved || entries.Any(entry => entry.Orbit == orbit)) continue;
            var entry = new CompletionEntry(orbit, OrbitAtlas.Name(orbit), revision, afterEntry, entries.Count == 0, sample);
            entries.Add(entry);
            added.Add(entry);
        }
        if (added.Count > 0) Safe.Raise(Changed);
        return added;
    }

    public CompletionEntry? Find(string name) => entries.Find(entry => entry.Name == name);

    public static string Line(CompletionEntry entry) =>
        FormattableString.Invariant($"{entry.Name} complete · exact check of every label · revision {entry.Revision}");
}
