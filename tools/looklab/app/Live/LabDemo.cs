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
    public static IReadOnlyList<string> Orbits => SampleCheck.Orbits;
    // The samples are checked by the sample rule when they are staged, like any other word (PreviewSamples).
    private static IReadOnlyList<PreviewSample> Samples => PreviewSamples.All;
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
        "view.preview.scrub" or "session.undo" or "session.redo" or "session.undo.partial" or "session.branch.select" or "session.branch.replay" or "session.rollback" or
        "layout.undo" or "layout.redo" or "history.dock.mode" or "preview.side.both" or "preview.side.before" or "preview.side.after" or "preview.side.next";

    // Where the command being carried out came from: a typed line never writes the move history.
    public InputSurface Surface { get; set; } = InputSurface.Key;

    public LabDemo(ChromeFrame frame, Keymap keymap, Action<string, string?> run)
    {
        this.frame = frame;
        // A preview is built at a head; whatever moves the head makes it stale. A prefix is rechecked by the
        // sample rule as its own preview before it commits.
        Preview.Head = () => Moves.Revision; Moves.Changed += Preview.Follow;
        Preview.Recheck = (_, moves) => SampleCheck.Check(moves);
        Bar = new CommitBar(Preview, () => frame.Tokens) { CommitKeys = keymap.KeysFor("operation.preview.commit", null) };
        PreviewView = new PreviewPanel(Preview, () => frame.Tokens);
        History = new HistoryPanel(Moves, Layout, () => frame.Tokens);
        Button stage = ChromeWidgets.Tool("Stage", "Stage the next sample preview"); stage.Name = "StageSample";
        stage.Pressed += () => run("operation.preview", null);
        Bar.Tools.AddChild(stage);
        Bar.Command += run; Bar.Parameter += SetParameter; PreviewView.Command += run;
        History.Command += (command, argument) => { if (command == "layout.roll") Restore(Layout.RollTo(int.Parse(argument!, CultureInfo.InvariantCulture))); else run(command, argument); };
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

    private void OpenPane() { if (!frame.Model.IsOpen(PreviewPane)) frame.Model.Drop(PreviewPane, FrameModel.Pair, DropZone.Right); }

    // A preview that was refused before anything was prepared: nothing was evaluated, so no orbit reads as
    // untouched.
    public void Refuse(string statement, string source, string reason)
    {
        Preview.Show(statement, source, Array.Empty<string>(), PreviewStatus.Refused, reason, Array.Empty<OrbitEffect>(), "Nothing was prepared.", Clock());
        OpenPane();
    }

    public void Stage(int? sample = null)
    {
        Current = (sample ?? nextSample++) % Samples.Count;
        PreviewSample s = Samples[Current]; PreviewCheck check = s.Check();
        Preview.Show(s.Statement, s.Source, s.Moves, check.Status, check.Reason, check.Effects, check.Protection, Clock());
        OpenPane();
    }

    private (int Id, PreviewSample Sample)? rebuilt;
    // A preview the lab built from moves of its own, checked by the sample rule; it remembers how a commit of it
    // (or of its prefix) is named in the history.
    private Preview ShowChecked(string statement, string source, IReadOnlyList<string> moves, PreviewCheck check, string origin, string name)
    {
        Preview shown = Preview.Show(statement, source, moves, check.Status, check.Reason, check.Effects, check.Protection, Clock());
        rebuilt = (shown.Id, new PreviewSample(statement, source, origin, name, moves.ToArray()));
        OpenPane(); return shown;
    }

    // The later moves of an old line come back as a preview the solver commits (ADR-D01.16). The replay never
    // commits, and its effects are evaluated like any other preview's.
    public void ReplayBranch(int branch)
    {
        if (branch < 0 || branch >= Moves.Branches.Count) { frame.Say("Replay: there is no such kept branch", true); return; }
        HistoryBranch kept = Moves.Branches[branch]; string[] moves = kept.Entries.SelectMany(e => e.Moves).ToArray();
        string name = $"replay of the branch after {kept.From}";
        Preview shown = ShowChecked("preview " + name, "kept branch", moves, SampleCheck.Check(moves), "macro", name);
        frame.Say($"{moves.Length} moves of the old line are back as preview #{shown.Id}" + (shown.Status == PreviewStatus.Ready ? "" : ": " + shown.Reason), shown.Status != PreviewStatus.Ready);
    }

    // A host part that owns a preview's source rebuilds it itself and answers true.
    public Func<Preview, int, int, bool>? Reparam { get; set; }
    // An edited statement needs its own witness (ADR-D01.11): the old moves never stay ready under a new
    // statement (SampleCheck.Rebuilt).
    public void SetParameter(int index, int value)
    {
        if (Preview.Pending is not { } p) return;
        IReadOnlyList<StatementParameter> all = InlineParameters.Find(p.Statement); if (index < 0 || index >= all.Count) return;
        string to = InlineParameters.With(p.Statement, all[index], value); if (to == p.Statement) return;
        if (Reparam?.Invoke(p, index, value) == true) return;
        string said = $"{all[index].Name} {InlineParameters.Find(to)[index].Value}";
        (IReadOnlyList<string> moves, PreviewCheck check) = SampleCheck.Rebuilt(to);
        if (moves.Count > 0)
        {
            Preview shown = ShowChecked(to, p.Source, moves, check, "hand", moves[0]);
            frame.Say($"Preview rebuilt: {said}; its witness is the changed twist, checked again (sample rule)", shown.Status != PreviewStatus.Ready);
            return;
        }
        Preview.Show(to, p.Source, moves, check.Status, check.Reason, check.Effects, check.Protection, Clock());
        rebuilt = null; frame.Say($"Preview changed: {said}; no witness is built for it here, so it cannot be committed", true);
    }

    // How a commit is named: a sample by its own name, a preview the lab built by the name it was given, a
    // prefix like the preview it came from.
    private PreviewSample Of(Preview preview)
    {
        if (rebuilt is { } r && (r.Id == preview.Id || preview.Statement.StartsWith(r.Sample.Statement + " |> prefix(", StringComparison.Ordinal))) return r.Sample;
        return Samples.FirstOrDefault(s => preview.Statement.StartsWith(s.Statement, StringComparison.Ordinal))
            ?? new PreviewSample(preview.Statement, preview.Source, "hand", preview.Statement.Replace("preview ", ""), preview.Moves.ToArray());
    }

    // A preview that is not one of the samples names its own history entry.
    public Func<Preview, (string Origin, string Name)?>? Named { get; set; }

    // The draft is staged with its own check: a failed protection check stops at the preview (ADR-D01.4 items 1
    // and 2); the Goal never gates (OperationDraft.StagedCheck). A prefix of it is rechecked by the draft's own
    // prefix check, under the draft's protection policy (OperationDraft.PrefixCheck).
    public void StageDraft(string statement, IReadOnlyList<string> moves, PreviewCheck check, Func<IReadOnlyList<string>, PreviewCheck> prefixCheck) =>
        Preview.Show(statement, "draft", moves, check.Status, check.Reason, check.Effects, check.Protection, Clock(), prefixCheck);

    // A twist in Direct that came from a typed line: staged as the preview, never committed (ADR-D01.58).
    public void StageTwist(string twist)
    {
        string[] moves = { twist.Trim() };
        Preview shown = ShowChecked("preview " + moves[0], "typed twist", moves, SampleCheck.Check(moves), "hand", moves[0]);
        frame.Say($"Direct: the typed twist is staged as preview #{shown.Id}; a typed line never commits", shown.Status != PreviewStatus.Ready);
    }

    // A library macro alone, as the preview (Shift+Enter in the Library); its effects are evaluated like any other's.
    public void StageMacro(string name, IReadOnlyList<string> word)
    {
        Preview shown = ShowChecked("preview " + name, "library", word, SampleCheck.Check(word), "macro", name);
        frame.Say($"Library: {name} alone is preview #{shown.Id}", shown.Status != PreviewStatus.Ready);
    }

    // A twist key or a pointer in Direct commits its twist after the same check as any preview: a twist that names
    // no cell or axis of the puzzle, or moves a protected orbit, is not committed.
    public void CommitDirect(string twist)
    {
        if (Moves.CommitRefusal is { } why) { frame.Say("Not committed: " + why, true); return; }
        if (SampleCheck.Check(new[] { twist }) is { Status: not PreviewStatus.Ready } failed) { frame.Say("Not committed: " + failed.Reason, true); return; }
        HistoryEntry entry = Moves.Commit("hand", twist, DateTime.Now.ToString("HH:mm", CultureInfo.InvariantCulture), "commit " + twist, new[] { twist })!;
        frame.Say($"Direct: committed {entry.Id}: {entry.Name}");
    }

    public void Execute(string command, string? argument)
    {
        void Nothing(bool done, string text) { if (!done) frame.Say(text, true); }
        // A preview built at the old head was made stale by the history itself (PreviewModel.Follow).
        void Moved(bool done, string nothing) { if (!done) frame.Say(nothing, true); else frame.Say(Moves.Actions[^1]); }
        switch (command)
        {
            case "operation.preview":
                Stage(argument != null && argument.StartsWith("sample:", StringComparison.Ordinal) ? int.Parse(argument.Substring(7), CultureInfo.InvariantCulture) : null); break;
            case "operation.preview.cancel": Nothing(Preview.Discard(), "There is no preview to discard"); break;
            case "operation.preview.commit":
            {
                if (Surface == InputSurface.TypedLine) { frame.Say("Not committed: " + CommitPaths.TypedNeverCommits, true); break; }
                if ((Preview.CommitRefusal(Clock()) ?? Moves.CommitRefusal) is { } why) { frame.Say("Not committed: " + why, true); break; }
                bool prefix = Preview.AtPrefix;
                if (Preview.Take(Clock()) is not { } taken)
                {
                    // The prefix failed its own recheck: it stays as the preview, and nothing is written.
                    frame.Say($"Not committed: the prefix, rechecked as its own preview #{Preview.Pending?.Id}: {Preview.Pending?.Reason}", true); break;
                }
                PreviewSample from = Of(taken);
                (string Origin, string Name) named = Named?.Invoke(taken) ?? (from.Origin, from.Name);
                HistoryEntry entry = Moves.Commit(named.Origin, named.Name, DateTime.Now.ToString("HH:mm", CultureInfo.InvariantCulture), taken.Statement.Replace("preview ", "commit "), taken.Moves)!;
                frame.Say($"Committed {entry.Id}: {entry.Name} ({entry.Moves.Count} moves{(prefix ? ", a prefix rechecked as its own preview" : "")})");
                break;
            }
            case "view.preview.open":
                if (!frame.Model.IsOpen(PreviewPane)) frame.Model.Drop(PreviewPane, FrameModel.Pair, DropZone.Right); else frame.Model.Activate(PreviewPane);
                break;
            case "view.preview.scrub": Preview.SetScrub(Preview.Scrub + (argument == "back" ? -1 : 1)); break;
            case "session.undo": Moved(Moves.Undo(), "Nothing to undo"); break;
            case "session.redo": Moved(Moves.Redo(), "Nothing to redo"); break;
            case "session.undo.partial": Moved(Moves.StepBack(), "Nothing to step back in"); break;
            case "session.branch.select":
            {
                // A branch switch passes its confirm gate: the first request only asks, in the History dock, and
                // only the answer to the question that is open, at the head it was asked at, switches lines.
                string[] parts = (argument ?? "").Split(' ', StringSplitOptions.RemoveEmptyEntries); bool confirmed = parts.Length == 2 && parts[0] == "confirm";
                if (Moves.Branches.Count == 0) { frame.Say("There is no kept branch", true); break; }
                if (!int.TryParse(parts.Length > 0 ? parts[^1] : "0", NumberStyles.None, CultureInfo.InvariantCulture, out int branch)) { frame.Say("Switch branch: it needs the number of a branch", true); break; }
                if (confirmed)
                {
                    if (Moves.ConfirmRefusal(branch) is { } open) { frame.Say("Branch not switched: " + open, true); break; }
                    Preview.Discard(); Moves.Confirm(branch); frame.Say(Moves.Actions[^1]); break;
                }
                if (Moves.SelectRefusal(branch) is { } no) { frame.Say("Branch not switched: " + no, true); break; }
                History.Show(false); Moves.Ask(branch); frame.Say("Switch lines? Confirm in History");
                break;
            }
            case "session.branch.replay":
                if (!int.TryParse(argument ?? "0", NumberStyles.None, CultureInfo.InvariantCulture, out int replayed)) { frame.Say("Replay: it needs the number of a kept branch", true); break; }
                ReplayBranch(replayed); break;
            case "preview.side.both": Preview.SetSide(PreviewSide.Both); break;
            case "preview.side.before": Preview.SetSide(PreviewSide.Before); break;
            case "preview.side.after": Preview.SetSide(PreviewSide.After); break;
            case "preview.side.next": Preview.CycleSide(); break;
            case "session.rollback": Moved(argument != null && Moves.RollTo(int.Parse(argument, CultureInfo.InvariantCulture)), "The history is already there"); break;
            case "layout.undo": Nothing(Restore(Layout.Undo()), "No layout change to undo"); break;
            case "layout.redo": Nothing(Restore(Layout.Redo()), "No layout change to redo"); break;
            case "history.dock.mode": History.SetPlainList(!History.PlainList); break;
        }
    }
}
