using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using Godot;
using LookLab.App.Live;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

internal static class ShellSession
{
    private static readonly HashSet<string> Rows = new(StringComparer.Ordinal);
    private static SessionTimer timer = new();
    private static SessionCellModel cell = new();
    private static CompletionLog completions = new();
    private static ChromeFrame? frame;
    private static HistoryPanel? history;
    private static TimerCell? timerControl;
    private static SessionCell? cellControl;
    private static bool sampleOn, practice;
    private static int? revision;
    private static string? build;

    public static void Load(ChromeFrame frame, CommandRegistry registry, SampleMount? sample,
        HistoryPanel history, MoveHistory moves, Action<string, string?> execute)
    {
        Rows.Clear();
        Rows.UnionWith(registry.Order.Where(id => id is "session.timer.start" or "session.timer.stop" or
            "session.practice" or "session.completion"));
        ShellSession.frame = frame;
        ShellSession.history = history;
        sampleOn = sample != null;
        practice = false;
        revision = null;
        string? identity = System.Environment.GetEnvironmentVariable("M600_BUILD_IDENTITY");
        build = string.IsNullOrWhiteSpace(identity) ? null : identity;

        timer = new SessionTimer();
        cell = new SessionCellModel();
        var timerView = timerControl = new TimerCell(timer, () => frame.Tokens, Now);
        var sessionView = cellControl = new SessionCell(cell, () => frame.Tokens);
        timerView.Command += row => execute(row, null);
        frame.BarTools.AddChild(timerView);
        frame.BarTools.AddChild(sessionView);
        // Set the order after the shell adds its queue readout and the rest of its tools.
        Callable.From(() =>
        {
            frame.BarTools.MoveChild(timerView, 0);
            frame.BarTools.MoveChild(sessionView, 1);
        }).CallDeferred();
        frame.Recoloured += timerView.Refresh;
        frame.Recoloured += sessionView.Refresh;

        var log = completions = new CompletionLog();
        history.Completions = log;
        moves.Changed += Refresh;
        log.Changed += Refresh;
        if (sample?.Solve.Demo is { } demo)
        {
            void CheckCompletion()
            {
                SolvingState state = demo.State;
                log.Check(state, state.Revision, moves.Entries.LastOrDefault()?.Id, sample: true);
            }
            demo.Changed += CheckCompletion;
            CheckCompletion();
        }
        Refresh();
    }

    public static bool Handles(string command) => Rows.Contains(command);

    public static string? Unavailable(string command) => sampleOn ? null : command switch
    {
        "session.practice" => "waits for K2's practice-session service",
        "session.completion" => "waits for K2's completion records",
        _ => null,
    };

    public static void Execute(ChromeFrame frame, string command, string? argument)
    {
        switch (command)
        {
            case "session.timer.start":
            case "session.timer.stop":
                string? refusal = command == "session.timer.start" ? timer.Start(Now()) : timer.Stop(Now());
                if (refusal != null) frame.Say(refusal, true);
                break;
            case "session.practice":
                if (practice)
                {
                    frame.Say("This session is already a practice copy", true);
                    break;
                }
                practice = true;
                Refresh();
                Marks(frame.Global);
                Marks(frame.Local);
                frame.Say("Practice copy (sample): every view border carries the practice mark");
                break;
            case "session.completion":
                if (argument == null || completions.Find(argument) is not { } entry)
                {
                    frame.Say($"{argument ?? "That orbit"} has no completion entry", true);
                    break;
                }
                if (history is { ShowLayout: true } panel) panel.Show(false);
                frame.Model.RevealDock(DockSide.Right, "history", frame.Folds(DockSide.Right));
                frame.Say(CompletionLog.Line(entry));
                break;
        }
    }

    public static void Describe(int? revision)
    {
        ShellSession.revision = revision;
        Refresh();
    }

    public static void Marks(ViewFrame view)
    {
        foreach (ViewMark mark in PracticeMark.BorderMarks(practice, view.ViewName))
            view.AddMark(mark.Glyph, mark.Words, mark.Colour, mark.Tooltip).SetMeta("slot", PracticeMark.Slot);
    }

    private static double Now() => Time.GetTicksMsec() / 1000.0;

    private static void Refresh()
    {
        if (history == null) return;
        MoveHistory moves = history.Moves;
        string head = moves.Cursor == 0 ? "none" : moves.Entries[moves.Cursor - 1].Id;
        var facts = new SessionFacts(moves.Entries.Count, moves.Entries.Sum(entry => entry.Moves.Count),
            completions.Entries.Count, head, revision, null, sampleOn ? "sample session" : null,
            sampleOn ? "600-cell-Full sample" : "600-cell-Full", build);
        cell.Update(facts, ShellDocuments.Recording, practice);
    }

    public static void Dump(Utf8JsonWriter json)
    {
        if (frame is not { } host || history is not { } panel) return;
        json.WriteStartObject("session");
        json.WriteStartObject("timer");
        json.WriteBoolean("cell", InTree(timerControl));
        json.WriteBoolean("running", timer.Running);
        json.WriteString("text", SessionTimer.Text(timer.Elapsed(Now())));
        json.WriteString("row", timer.Row);
        json.WriteEndObject();

        json.WriteStartObject("cell");
        json.WriteBoolean("present", InTree(cellControl));
        json.WriteBoolean("expanded", cell.Expanded);
        json.WriteStartArray("lamps");
        foreach (SessionLamp lamp in cell.Lamps)
        {
            json.WriteStartObject();
            json.WriteString("slot", lamp.Slot);
            json.WriteBoolean("lit", lamp.Lit);
            json.WriteEndObject();
        }
        json.WriteEndArray();
        json.WriteStartArray("facts");
        foreach (SessionFact fact in cell.Facts())
        {
            json.WriteStartObject();
            json.WriteString("name", fact.Name);
            json.WriteString("value", fact.Value);
            json.WriteBoolean("waiting", fact.Waiting);
            json.WriteEndObject();
        }
        json.WriteEndArray();
        json.WriteEndObject();

        json.WriteStartObject("practice");
        json.WriteBoolean("on", practice);
        json.WriteNumber("global", PracticeMarks(host.Global));
        json.WriteNumber("local", PracticeMarks(host.Local));
        json.WriteEndObject();

        json.WriteStartArray("completions");
        foreach (CompletionEntry entry in completions.Entries)
        {
            json.WriteStartObject();
            json.WriteString("name", entry.Name);
            json.WriteNumber("revision", entry.Revision);
            json.WriteBoolean("moment", entry.Moment);
            json.WriteBoolean("sample", entry.Sample);
            json.WriteEndObject();
        }
        json.WriteEndArray();
        json.WriteNumber("historyRows", panel.List.GetChildren().Count(row => row.HasMeta("completion")));
        json.WriteBoolean("momentSlot", panel.List.FindChild("MomentSlot", true, false) is Control);
        json.WriteEndObject();
        ShellExplore.Dump(json);
        ShellScene.Dump(json);
    }

    private static bool InTree(Control? control) => control != null && GodotObject.IsInstanceValid(control) && control.IsInsideTree();

    private static int PracticeMarks(ViewFrame? view) => view?.Marks.GetChildren().Count(mark =>
        mark.HasMeta("slot") && mark.GetMeta("slot").AsString() == PracticeMark.Slot) ?? 0;
}
