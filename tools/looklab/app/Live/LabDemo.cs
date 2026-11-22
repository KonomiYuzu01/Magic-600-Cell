using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Godot;
using Magic600.Chrome;

namespace LookLab.App.Live;

// The lab has no engine, so the preview, the commit bar and the move history are driven by sample data that is
// marked as such on screen. The layout history is real: it records and restores the lab's own arrangement.
internal sealed class LabDemo
{
    private sealed record Sample(string Statement, string Source, string Origin, string Name, string[] Moves, PreviewStatus Status, string? Reason, int[] Moved, string Protection);

    public static readonly string[] Orbits = { "sample orbit 1", "sample orbit 2", "sample orbit 3", "sample orbit 4", "sample orbit 5" };
    private static readonly int[] Totals = { 120, 720, 1200, 600, 2400 };
    private static string[] Twists(params (int Cell, int Axis, int Turn)[] moves) =>
        moves.Select(m => $"twist(cell {m.Cell}, axis {m.Axis}, {(m.Turn > 0 ? "+" : "-")}{Math.Abs(m.Turn)})").ToArray();
    private static readonly Sample[] Samples =
    {
        new("preview my.corner_swap(cell 117)", "own macro", "macro", "my.corner_swap(cell 117)",
            Twists((117, 2, 1), (341, 0, 1), (117, 2, -1), (341, 0, -1), (117, 2, 1), (88, 3, 2), (117, 2, -1), (88, 3, -2)),
            PreviewStatus.Ready, null, new[] { 0, 6, 0, 0, 3 }, "Protected orbits untouched: 3 of 3."),
        new("preview run \"sweep.6cs\" on ring 4", "own program", "script", "sweep.6cs on ring 4",
            Twists((12, 1, 1), (13, 1, 1), (14, 1, 1), (15, 1, 1), (16, 1, 1), (12, 4, -1), (13, 4, -1), (14, 4, -1), (15, 4, -1), (16, 4, -1), (12, 1, -1), (13, 1, -1)),
            PreviewStatus.Ready, null, new[] { 0, 30, 48, 10, 96 }, "Protected orbits untouched: 3 of 3."),
        new("preview twist(cell 402, axis 1, +1)", "typed twist", "hand", "twist(cell 402, axis 1, +1)", Twists((402, 1, 1)),
            PreviewStatus.Ready, null, new[] { 1, 12, 20, 30, 20 }, "Protected orbits untouched: 3 of 3."),
        new("preview my.corner_swap(cell 9) |> conjugate(twist(cell 30, axis 0, +1))", "own macro", "macro", "conjugated corner swap",
            Twists((30, 0, 1), (9, 2, 1), (201, 0, 1), (9, 2, -1), (201, 0, -1), (30, 0, -1)),
            PreviewStatus.Impossible, "it moves a protected orbit: sample orbit 4 is locked", new[] { 0, 6, 0, 4, 3 }, "Would move protected sample orbit 4: 4 pieces."),
        new("preview shared.pack.solve_layer(2)", "package by another author", "script", "shared.pack.solve_layer(2)", Array.Empty<string>(),
            PreviewStatus.Refused, "in a solving session a word from a package written by someone else is refused", new[] { 0, 0, 0, 0, 0 }, "Nothing was prepared."),
    };

    public PreviewModel Preview { get; } = new();
    public MoveHistory Moves { get; } = new();
    public LayoutHistory Layout { get; } = new();
    public CommitBar Bar { get; }
    public PreviewPanel PreviewView { get; }
    public HistoryPanel History { get; }
    public Func<double> Clock { get; set; } = () => Time.GetTicksMsec() / 1000.0;
    public const string PreviewPane = "preview";
    private readonly ChromeFrame frame;
    private int nextSample;
    // The sample that is staged, or -1.
    public int Current { get; private set; } = -1;
    private Func<IReadOnlyList<string>>? layoutState;
    private Action<IReadOnlyList<string>>? layoutApply;
    private bool restoring;
    private const string Cut = "\n\u001e\n";

    public static bool Handles(string command) => command is "operation.preview" or "operation.preview.commit" or "operation.preview.cancel" or "view.preview.open" or
        "view.preview.scrub" or "session.undo" or "session.redo" or "session.undo.partial" or "session.rollback" or "layout.undo" or "layout.redo" or "history.dock.mode";

    public LabDemo(ChromeFrame frame, Keymap keymap, Action<string, string?> run)
    {
        this.frame = frame;
        Bar = new CommitBar(Preview, () => frame.Tokens) { CommitKeys = keymap.KeysFor("operation.preview.commit", null) };
        PreviewView = new PreviewPanel(Preview, () => frame.Tokens);
        History = new HistoryPanel(Moves, Layout, () => frame.Tokens);
        Button stage = ChromeWidgets.Tool("Stage", "Stage the next sample preview"); stage.Name = "StageSample";
        stage.Pressed += () => run("operation.preview", null);
        Bar.Tools.AddChild(stage);
        Bar.Command += run; Bar.Parameter += SetParameter; History.Command += (command, argument) => { if (command == "layout.roll") Restore(Layout.RollTo(int.Parse(argument!, CultureInfo.InvariantCulture))); else run(command, argument); };
        foreach ((string origin, string name, string time, int sample) in new[] { ("hand", "twist(cell 5, axis 0, +1)", "10:02", 2), ("macro", "my.corner_swap(cell 40)", "10:05", 0),
            ("script", "sweep.6cs on ring 1", "10:11", 1), ("hand", "twist(cell 77, axis 3, -1)", "10:12", 2), ("macro", "my.corner_swap(cell 117)", "10:20", 0) })
            Moves.Commit(origin, name, time, "commit " + name, origin == "hand" ? new[] { name } : Samples[sample].Moves);
        frame.Rebuilt += () => { Bar.Refresh(); PreviewView.Refresh(); History.Refresh(); RecordLayout(); };
        frame.Recoloured += () => { Bar.Refresh(); PreviewView.Refresh(); History.Refresh(); RecordLayout(); };
    }

    // The layout history keeps the lab's stored arrangement parts: appearance, workspace and pane layout.
    public void TrackLayout(Func<IReadOnlyList<string>> state, Action<IReadOnlyList<string>> apply)
    {
        layoutState = state; layoutApply = apply; Layout.Record("Opened", string.Join(Cut, state()));
    }

    private void RecordLayout()
    {
        if (restoring || layoutState == null || Layout.Cursor < 0) return;
        string[] before = Layout.Steps[Layout.Cursor].State.Split(Cut); IReadOnlyList<string> now = layoutState();
        string name = "Arrange panes"; string? merge = null;
        if (now[0] != before[0])
        {
            string changed = now[0].Split('\n').Except(before[0].Split('\n')).FirstOrDefault() ?? "appearance";
            name = "Appearance: " + changed.Replace("=", " ");
            string key = changed.Split('=')[0]; if (key.StartsWith("knob.", StringComparison.Ordinal) || key == "scale") merge = key;
        }
        else if (now[1] != before[1]) name = "Workspace: " + frame.Workspaces.Names[frame.Workspace];
        else if (now.Count > 4 && before.Length > 4 && now[4] != before[4]) name = "Key bindings";
        Layout.Record(name, string.Join(Cut, now), merge);
    }

    private bool Restore(LayoutStep? step)
    {
        if (step == null || layoutApply == null) return false;
        restoring = true;
        try { layoutApply(step.State.Split(Cut)); frame.Flush(); } finally { restoring = false; }
        History.Refresh();
        return true;
    }

    // One named step of the layout history for everything the change does.
    public void LayoutRun(string name, Action change)
    {
        restoring = true;
        try { change(); frame.Flush(); } finally { restoring = false; }
        if (layoutState != null) Layout.Record(name, string.Join(Cut, layoutState()));
        History.Refresh();
    }

    // A preview that was refused before anything was prepared.
    public void Refuse(string statement, string source, string reason)
    {
        Preview.Show(statement, source, Array.Empty<string>(), PreviewStatus.Refused, reason, Orbits.Select((o, i) => new OrbitEffect(o, 0, Totals[i])).ToArray(), "Nothing was prepared.", Clock());
        if (!frame.Model.IsOpen(PreviewPane)) frame.Model.Drop(PreviewPane, FrameModel.Pair, DropZone.Right);
    }

    public void Stage(int? sample = null)
    {
        Current = (sample ?? nextSample++) % Samples.Length;
        Sample s = Samples[Current];
        Preview.Show(s.Statement, s.Source, s.Moves, s.Status, s.Reason, Orbits.Select((o, i) => new OrbitEffect(o, s.Moved[i], Totals[i])).ToArray(), s.Protection, Clock());
        if (!frame.Model.IsOpen(PreviewPane)) frame.Model.Drop(PreviewPane, FrameModel.Pair, DropZone.Right);
    }

    private (int Id, Sample Sample)? rebuilt;
    // A host part that owns a preview's source rebuilds it itself and answers true.
    public Func<Preview, int, int, bool>? Reparam { get; set; }
    public const string NotEvaluated = " Parameters changed: the moves and effects are still the sample's (demo data).";
    public void SetParameter(int index, int value)
    {
        if (Preview.Pending is not { } p) return;
        IReadOnlyList<StatementParameter> all = InlineParameters.Find(p.Statement); if (index < 0 || index >= all.Count) return;
        string to = InlineParameters.With(p.Statement, all[index], value); if (to == p.Statement) return;
        if (Reparam?.Invoke(p, index, value) == true) return;
        Sample from = Of(p);
        Preview shown = Preview.Show(to, p.Source, p.Moves, p.Status, p.Reason, p.Effects, p.Protection.EndsWith(NotEvaluated, StringComparison.Ordinal) ? p.Protection : p.Protection + NotEvaluated, Clock());
        rebuilt = (shown.Id, from); frame.Say($"Preview rebuilt: {all[index].Name} {InlineParameters.Find(to)[index].Value} (demo data: the moves are the sample's)");
    }

    private Sample Of(Preview preview) => rebuilt is { } r && r.Id == preview.Id ? r.Sample : Samples.FirstOrDefault(s => preview.Statement.StartsWith(s.Statement, StringComparison.Ordinal)) ?? Samples[^1];

    // A preview that is not one of the samples names its own history entry.
    public Func<Preview, (string Origin, string Name)?>? Named { get; set; }
    public void StageDraft(string statement, IReadOnlyList<string> moves) =>
        Preview.Show(statement, "draft", moves, PreviewStatus.Ready, null, Array.Empty<OrbitEffect>(), "not checked (demo)", Clock());
    public void CommitDirect(string twist)
    {
        if (Moves.CommitRefusal is { } why) { frame.Say("Not committed: " + why, true); return; }
        HistoryEntry entry = Moves.Commit("hand", twist, DateTime.Now.ToString("HH:mm", CultureInfo.InvariantCulture), "commit " + twist, new[] { twist })!;
        frame.Say($"Direct: committed {entry.Id}: {entry.Name}");
    }

    public void Execute(string command, string? argument)
    {
        void Nothing(bool done, string text) { if (!done) frame.Say(text, true); }
        void Moved(bool done, string nothing)
        {
            if (!done) { frame.Say(nothing, true); return; }
            frame.Say(Moves.Actions[^1]);
            if (Preview.Pending is { Status: PreviewStatus.Ready }) Preview.MarkStale("the committed state changed after this preview was built; stage it again");
        }
        switch (command)
        {
            case "operation.preview":
                Stage(argument != null && argument.StartsWith("sample:", StringComparison.Ordinal) ? int.Parse(argument.Substring(7), CultureInfo.InvariantCulture) : null); break;
            case "operation.preview.cancel": Nothing(Preview.Discard(), "There is no preview to discard"); break;
            case "operation.preview.commit":
                if ((Preview.CommitRefusal(Clock()) ?? Moves.CommitRefusal) is { } why) { frame.Say("Not committed: " + why, true); break; }
                Preview taken = Preview.Take(Clock())!; Sample from = Of(taken);
                (string Origin, string Name) named = Named?.Invoke(taken) ?? (from.Origin, from.Name);
                HistoryEntry entry = Moves.Commit(named.Origin, named.Name, DateTime.Now.ToString("HH:mm", CultureInfo.InvariantCulture), taken.Statement.Replace("preview ", "commit "), taken.Moves)!;
                frame.Say($"Committed {entry.Id}: {entry.Name} ({entry.Moves.Count} moves)");
                break;
            case "view.preview.open":
                if (!frame.Model.IsOpen(PreviewPane)) frame.Model.Drop(PreviewPane, FrameModel.Pair, DropZone.Right); else frame.Model.Activate(PreviewPane);
                break;
            case "view.preview.scrub": Preview.SetScrub(Preview.Scrub + (argument == "back" ? -1 : 1)); break;
            case "session.undo": Moved(Moves.Undo(), "Nothing to undo"); break;
            case "session.redo": Moved(Moves.Redo(), "Nothing to redo"); break;
            case "session.undo.partial": Moved(Moves.StepBack(), "Nothing to step back in"); break;
            case "session.rollback": Moved(argument != null && Moves.RollTo(int.Parse(argument, CultureInfo.InvariantCulture)), "The history is already there"); break;
            case "layout.undo": Nothing(Restore(Layout.Undo()), "No layout change to undo"); break;
            case "layout.redo": Nothing(Restore(Layout.Redo()), "No layout change to redo"); break;
            case "history.dock.mode": History.SetPlainList(!History.PlainList); break;
        }
    }
}
