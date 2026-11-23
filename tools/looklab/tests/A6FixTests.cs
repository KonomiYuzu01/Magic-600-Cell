using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using LookLab.Core;
using Magic600.Chrome;

// Tests for the A5 build defects that the A6 review found (B-1 to B-13, Q5). Each names the finding it pins.
internal static partial class Program
{
    // The registry Look Lab runs on: the command table, the record's draft rows and the owner's words.
    private static CommandRegistry LabRegistry()
    {
        CommandRegistry registry = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        registry.AddRows(Read("data/design/command-rows-draft.json"));
        registry.Relabel(Read("data/design/command-labels.json"));
        return registry;
    }

    private static string Twist(int cell, int axis, int turns) => new SampleCheck.Twist(cell, axis, turns).ToString();

    private static string Body(string text, string from, string to)
    {
        int at = text.IndexOf(from, StringComparison.Ordinal), end = at < 0 ? -1 : text.IndexOf(to, at + from.Length, StringComparison.Ordinal);
        if (at < 0 || end < 0) throw new Exception($"no text from `{from}` to `{to}`");
        return text.Substring(at, end - at);
    }

    // B-1: a draft whose protection check fails stops at the preview; the Goal never gates.
    private static void A6DraftFailedCheckStopsAtThePreview()
    {
        var draft = new OperationDraft();
        draft.Edit("prepare: " + Twist(5, 1, 1)); draft.Edit("core: " + Twist(40, 2, -1)); draft.SetGoal(null);
        Check(draft.GoalResult == GoalResult.Unmet, "the Goal is unmet in both cases");
        draft.SetMode(CheckMode.Strict); PreviewCheck failed = draft.StagedCheck();
        Check(draft.CanStage && !draft.CanCommit && failed.Status == PreviewStatus.Impossible && failed.Reason!.Contains("Strict", StringComparison.Ordinal) && failed.Effects.Count == 5,
            "a failed Strict check did not stop at the preview");
        var history = new MoveHistory(); var preview = new PreviewModel { Head = () => history.Revision };
        preview.Show(draft.Statement, "draft", draft.Word, failed.Status, failed.Reason, failed.Effects, failed.Protection, 0);
        Check(preview.CommitRefusal(10) == failed.Reason && preview.Take(10) == null && history.Entries.Count == 0, "a draft that failed Strict was committed");
        // A passing check with the Goal still unmet leaves the commit open.
        draft.SetMode(CheckMode.Net); PreviewCheck passed = draft.StagedCheck();
        Check(draft.CanCommit && draft.GoalResult == GoalResult.Unmet && passed.Status == PreviewStatus.Ready && passed.Protection.StartsWith("Net", StringComparison.Ordinal), "a passing check with the Goal unmet");
        preview.Show(draft.Statement, "draft", draft.Word, passed.Status, passed.Reason, passed.Effects, passed.Protection, 20);
        Check(preview.Take(21) is { Moves.Count: 2 }, "a passing draft did not commit");
        string solve = Read("app/Live/LabSolve.cs");
        Check(solve.Contains("if (Draft.CanStage)", StringComparison.Ordinal) && solve.Contains("Draft.StagedCheck()", StringComparison.Ordinal), "the lab stages the draft without its check");
        // N-2 (re-check): a word whose own effects move the locked orbit fails in both modes, exactly as the same
        // twist staged alone.
        string locking = Twist(30, 0, 1); PreviewCheck alone = SampleCheck.Check(new[] { locking });
        Check(alone.Status == PreviewStatus.Impossible && alone.Effects[SampleCheck.Locked].Moved > 0, "the locking fixture");
        foreach (CheckMode mode in new[] { CheckMode.Net, CheckMode.Strict })
        {
            var core = new OperationDraft(); core.Edit("core: " + locking); core.SetMode(mode); PreviewCheck c = core.StagedCheck();
            Check(c.Status == alone.Status && c.Reason == alone.Reason && c.Effects.SequenceEqual(alone.Effects) && core.Checked == CheckResult.Failed && !core.CanCommit,
                $"{mode} passed a draft whose effects move the locked orbit");
            preview.Show(core.Statement, "draft", core.Word, c.Status, c.Reason, c.Effects, c.Protection, 40, core.PrefixCheck());
            Check(preview.Take(41) == null && history.Entries.Count == 0, $"{mode} committed a draft that moves the locked orbit");
        }
    }

    // B-2: a typed line never writes the move history; in Direct a typed twist is staged, never committed.
    private static void A6TypedLineNeverCommits()
    {
        foreach (string line in new[] { "commit", "operation.preview.commit", "  commit now" }) Check(CommitPaths.TypedRefusal(line) == CommitPaths.TypedNeverCommits, line + " was not refused");
        Check(CommitPaths.TypedRefusal("operation.phase.append_twist") == null && CommitPaths.TypedRefusal("committed.view") == null, "a typed line that commits nothing was refused");
        Check(!CommitPaths.DirectTwistCommits(InputSurface.TypedLine) && CommitPaths.DirectTwistCommits(InputSurface.Key) && CommitPaths.DirectTwistCommits(InputSurface.Pointer), "which surfaces commit a Direct twist");
        string scripts = Read("app/Live/LabScripts.cs"), solve = Read("app/Live/LabSolve.cs"), demo = Read("app/Live/LabDemo.cs"), frame = Read("app/Live/LabFrame.cs");
        string run = Body(scripts, "public void RunLine", "public void Capture");
        Check(run.Contains("CommitPaths.TypedRefusal(line)", StringComparison.Ordinal) && run.Contains("demo.Surface = InputSurface.TypedLine;", StringComparison.Ordinal) &&
            run.Contains("finally { quiet = false; demo.Surface = InputSurface.Key; }", StringComparison.Ordinal), "a typed line does not run as one");
        Check(scripts.Contains("Bar.Entered += RunLine;", StringComparison.Ordinal) && frame.Contains("palette.TypedLine += scripts.RunLine;", StringComparison.Ordinal), "the command bar or the palette's typed mode bypasses the typed line");
        Check(solve.Contains("if (CommitPaths.DirectTwistCommits(session.Surface)) session.CommitDirect(twist); else session.StageTwist(twist);", StringComparison.Ordinal), "a typed Direct twist commits");
        Check(Body(demo, "public void StageTwist", "public void CommitDirect").Contains("ShowChecked(", StringComparison.Ordinal) && !Body(demo, "public void StageTwist", "public void CommitDirect").Contains("Moves.Commit", StringComparison.Ordinal) &&
            demo.Contains("if (Surface == InputSurface.TypedLine) {", StringComparison.Ordinal), "the lab writes history from a typed line");
    }

    // B-3: a prefix is prepared and rechecked as its own preview before it commits. A word that disturbs a
    // protected orbit and restores it passes; its first twist alone does not.
    private static void A6PrefixRecheckedAsItsOwnPreview()
    {
        string[] word = { Twist(30, 0, 1), Twist(30, 0, -1) };
        PreviewCheck whole = SampleCheck.Check(word), first = SampleCheck.Check(word.Take(1).ToArray());
        Check(whole.Status == PreviewStatus.Ready && whole.Effects.All(e => e.Moved == 0) && first.Status == PreviewStatus.Impossible && first.Effects[SampleCheck.Locked].Moved > 0, "the fixture");
        var history = new MoveHistory(); var preview = new PreviewModel { Head = () => history.Revision, Recheck = (_, m) => SampleCheck.Check(m) };
        preview.Show("preview disturb and restore", "own macro", word, whole.Status, whole.Reason, whole.Effects, whole.Protection, 0);
        preview.SetScrub(1);
        Check(preview.CommitRefusal(1) == null && preview.Take(1) == null && preview.Pending is { Id: 2, Status: PreviewStatus.Impossible } p && p.Reason == SampleCheck.LockedReason &&
            p.Moves.Count == 1 && p.Effects[SampleCheck.Locked].Moved > 0 && history.Entries.Count == 0, "a prefix took the whole word's check");
        Check(preview.Take(2) == null && preview.Replaced, "the failed prefix can be committed");
        // A prefix that passes its own recheck commits in the same action, with its own effects.
        string[] other = { Twist(5, 1, 1), Twist(7, 2, 1) }; PreviewCheck both = SampleCheck.Check(other);
        preview.Show("preview two twists", "own macro", other, both.Status, both.Reason, both.Effects, both.Protection, 10);
        preview.SetScrub(1); Preview? taken = preview.Take(11);
        Check(taken is { Id: 4, Status: PreviewStatus.Ready } && taken.Moves.Count == 1 && taken.Effects.Sum(e => e.Moved) < both.Effects.Sum(e => e.Moved) && preview.Pending == null, "a passing prefix did not commit with its own effects");
        Check(Read("app/Live/LabDemo.cs").Contains("Preview.Recheck = (_, moves) => SampleCheck.Check(moves);", StringComparison.Ordinal), "the lab gives the prefix no recheck");
        // A draft's prefix keeps the draft's protection policy (re-check). Under Strict, a Prepare twist and its
        // inverse in Cleanup pass whole; at the scrub after the first twist the prefix is the Prepare alone, which
        // fails Strict exactly as the same one-twist draft staged on its own, although the sample rule alone passes it.
        var strict = new OperationDraft(); strict.Edit("prepare: " + Twist(5, 1, 1)); strict.Edit("cleanup: " + Twist(5, 1, -1)); strict.SetMode(CheckMode.Strict);
        var single = new OperationDraft(); single.Edit("prepare: " + Twist(5, 1, 1)); single.SetMode(CheckMode.Strict);
        PreviewCheck whole2 = strict.StagedCheck(), alone = single.StagedCheck();
        Check(whole2.Status == PreviewStatus.Ready && alone.Status == PreviewStatus.Impossible && SampleCheck.Check(new[] { Twist(5, 1, 1) }).Status == PreviewStatus.Ready, "the Strict fixture");
        int head = history.Revision, kept = history.Entries.Count;
        preview.Show(strict.Statement, "draft", strict.Word, whole2.Status, whole2.Reason, whole2.Effects, whole2.Protection, 20, strict.PrefixCheck());
        // A later change of the draft never changes the check the staged preview took.
        strict.SetMode(CheckMode.Net); preview.SetScrub(1);
        Check(preview.CommitRefusal(21) == null && preview.Take(21) == null && preview.Pending is { Status: PreviewStatus.Impossible } q && q.Reason == alone.Reason && q.Moves.Count == 1 &&
            q.Effects.SequenceEqual(alone.Effects) && history.Revision == head && history.Entries.Count == kept, "a draft's prefix lost its Strict check");
        Check(strict.PrefixCheck()(new[] { Twist(7, 1, 1) }) is { Status: PreviewStatus.Refused, Reason: OperationDraft.NotAPrefix }, "moves that are not a prefix were checked as one");
        Check(Read("app/Live/LabSolve.cs").Contains("Draft.StagedCheck(), Draft.PrefixCheck())", StringComparison.Ordinal) && Read("app/Live/LabDemo.cs").Contains("Clock(), prefixCheck);", StringComparison.Ordinal),
            "the lab stages the draft without its prefix check");
    }

    // B-13: whatever moves the head, a Direct twist included, makes an older ready preview stale.
    private static void A6HeadMoveMakesAReadyPreviewStale()
    {
        var history = new MoveHistory(); var preview = new PreviewModel { Head = () => history.Revision }; history.Changed += preview.Follow;
        history.Commit("hand", "a", "10:00", "commit a", new[] { Twist(5, 1, 1) });
        string[] staged = { Twist(402, 1, 1) }; PreviewCheck c = SampleCheck.Check(staged);
        preview.Show("preview " + staged[0], "typed twist", staged, c.Status, c.Reason, c.Effects, c.Protection, 0);
        Check(preview.CommitRefusal(1) == null, "a fresh preview");
        // A Direct twist is a transaction of its own.
        string direct = Twist(7, 1, 1); history.Commit("hand", direct, "10:01", "commit " + direct, new[] { direct });
        int head = history.Revision, entries = history.Entries.Count;
        Check(preview.Pending!.Status == PreviewStatus.Stale && preview.Pending.Reason == PreviewModel.HeadMoved && preview.Take(5) == null && history.Revision == head && history.Entries.Count == entries,
            "an older preview stayed ready after a Direct twist");
        // The commit gate compares heads even where nothing follows the history.
        var bare = new PreviewModel { Head = () => history.Revision };
        bare.Show("preview " + staged[0], "typed twist", staged, c.Status, c.Reason, c.Effects, c.Protection, 0); history.Undo();
        Check(bare.Pending!.Status == PreviewStatus.Ready && bare.CommitRefusal(5) == PreviewModel.HeadMoved && bare.Take(5) == null, "the commit gate does not compare heads");
        Check(Read("app/Live/LabDemo.cs").Contains("Preview.Head = () => Moves.Revision; Moves.Changed += Preview.Follow;", StringComparison.Ordinal), "the lab's preview does not follow the head");
    }

    // B-12: an edited statement never keeps its old witness ready.
    private static void A6EditedStatementGetsItsOwnWitness()
    {
        string s = "preview twist(cell 402, axis 1, +1)"; IReadOnlyList<StatementParameter> parameters = InlineParameters.Find(s);
        string axis3 = InlineParameters.With(s, parameters[1], 3);
        (IReadOnlyList<string> moves, PreviewCheck check) = SampleCheck.Rebuilt(axis3);
        Check(moves.Single() == Twist(402, 3, 1) && "preview " + moves[0] == axis3 && check.Status == PreviewStatus.Ready && check.Effects.Count == 5, "the witness does not name the new axis");
        (IReadOnlyList<string> zero, PreviewCheck locked) = SampleCheck.Rebuilt(InlineParameters.With(s, parameters[1], 0));
        Check(zero.Single() == Twist(402, 0, 1) && locked.Status == PreviewStatus.Impossible, "the changed twist was not checked again");
        string macro = "preview my.corner_swap(cell 117)";
        (IReadOnlyList<string> none, PreviewCheck refused) = SampleCheck.Rebuilt(InlineParameters.With(macro, InlineParameters.Find(macro)[0], 118));
        Check(none.Count == 0 && refused.Status == PreviewStatus.Refused && refused.Reason == SampleCheck.NoWitness && refused.Effects.Count == 0, "an edited macro statement kept a witness");
        string body = Body(Read("app/Live/LabDemo.cs"), "public void SetParameter", "private PreviewSample Of");
        Check(body.Contains("SampleCheck.Rebuilt(to)", StringComparison.Ordinal) && !body.Contains("p.Moves", StringComparison.Ordinal) && !body.Contains("p.Effects", StringComparison.Ordinal), "the lab keeps the old witness");
    }

    // B-5: only the answer to the open question, for its branch and at its head, switches lines.
    private static void A6BranchConfirmOnlyForTheOpenQuestion()
    {
        var h = new MoveHistory();
        foreach (string n in new[] { "a", "b", "c" }) h.Commit("hand", n, "10:00", "commit " + n, new[] { n + "1", n + "2" });
        h.RollTo(1); h.Commit("hand", "d", "10:01", "commit d", new[] { "d1", "d2" });
        string[] line = h.Entries.Select(e => e.Id).ToArray(); bool Kept() => h.Entries.Select(e => e.Id).SequenceEqual(line);
        Check(h.Branches.Count == 1 && h.Asking == -1, "the fixture");
        Check(h.ConfirmRefusal(0) != null && !h.Confirm(0) && Kept(), "a confirmation with no question switched lines");
        Check(h.Ask(0) && h.Asking == 0 && h.Ask(-1) && h.Asking == -1 && !h.Confirm(0) && Kept(), "a confirmation after Cancel switched lines");
        h.Ask(0); h.Undo(); Check(h.Asking == -1 && !h.Confirm(0), "a confirmation after a head change switched lines"); h.Redo(); Check(Kept(), "redo");
        h.Ask(0); Check(h.ConfirmRefusal(1) != null && !h.Confirm(1) && h.Asking == 0, "a confirmation for another branch");
        Check(h.Confirm(0) && h.Asking == -1 && !Kept() && !h.Confirm(0), "the open question did not switch lines once");
        h.StepBack(); Check(!h.Ask(0) && h.Asking == -1, "a question while an entry is partly undone"); h.Redo();
        string demo = Read("app/Live/LabDemo.cs"), panel = Read("app/Chrome/Ui/SessionPanels.cs");
        Check(demo.Contains("Moves.ConfirmRefusal(branch)", StringComparison.Ordinal) && demo.Contains("Moves.Confirm(branch)", StringComparison.Ordinal) && !demo.Contains("Moves.SelectBranch(", StringComparison.Ordinal) &&
            panel.Contains("public int Asking => Moves.Asking;", StringComparison.Ordinal), "the lab trusts the word confirm");
    }

    // B-4: only evaluated effects may say an orbit is untouched.
    private static void A6OnlyEvaluatedEffectsSayUntouched()
    {
        var p = new PreviewModel();
        Refuse(() => p.Show("preview r", "kept branch", new[] { "m" }, PreviewStatus.Ready, null, Array.Empty<OrbitEffect>(), "not checked", 0), "preview");
        p.Show("preview pack", "package", Array.Empty<string>(), PreviewStatus.Refused, "not yours", Array.Empty<OrbitEffect>(), "Nothing was prepared.", 0);
        Check(!p.Pending!.Evaluated && p.Range() == null, "an unprepared package reads as evaluated");
        string[] kept = { Twist(5, 0, 1), Twist(117, 2, 1), Twist(341, 0, 1) };
        PreviewCheck replay = SampleCheck.Check(kept), identity = SampleCheck.Check(new[] { Twist(9, 1, 1), Twist(9, 1, -1) });
        PreviewCheck unprepared = SampleCheck.Check(Array.Empty<string>()), unknown = SampleCheck.Check(new[] { "R" });
        Check(replay.Effects.Count == 5 && replay.Effects.Any(e => e.Moved > 0), "a replay without effects");
        Check(identity.Status == PreviewStatus.Ready && identity.Effects.Count == 5 && identity.Effects.All(e => e.Moved == 0), "an evaluated identity");
        Check(unprepared.Effects.Count == 0 && unknown.Effects.Count == 0 && unknown.Status == PreviewStatus.Refused && unknown.Reason == SampleCheck.NotTwists, "an unevaluated word reports effects");
        string panel = Read("app/Chrome/Ui/SessionPanels.cs"), demo = Read("app/Live/LabDemo.cs");
        Check(panel.Contains("if (!p.Evaluated)", StringComparison.Ordinal) && panel.Contains("EffectsNotEvaluated", StringComparison.Ordinal), "the preview view says untouched without evaluated effects");
        string replayBody = Body(demo, "public void ReplayBranch", "public Func<Preview, int, int, bool>?");
        Check(replayBody.Contains("SampleCheck.Check(moves)", StringComparison.Ordinal) && !replayBody.Contains("Array.Empty<OrbitEffect>()", StringComparison.Ordinal) &&
            Body(demo, "public void Refuse", "public void Stage").Contains("PreviewStatus.Refused, reason, Array.Empty<OrbitEffect>()", StringComparison.Ordinal), "the lab's refusal or replay");
    }

    // B-7: each orbit owns its draft; activating Next in another orbit saves the present one's.
    private static void A6NextAcrossOrbitsCarriesEachDraft()
    {
        var draft = new OperationDraft(); var demo = new SolvingDemo(Atlas) { Draft = draft };
        SolvingState mid = demo.State; int a = mid.Orbit; int b = mid.Next!.Value.Orbit;
        Check(a != b, "Next lies in another orbit");
        draft.Edit("core: " + Twist(40, 2, -1)); draft.Edit("prepare: " + Twist(5, 1, 1)); draft.SetGoal(null); draft.SetMode(CheckMode.Strict); draft.SelectPhase("cleanup");
        string wordA = draft.Statement;
        Check(demo.ActivateNext() == null && mid.Orbit == b && draft.Count == 0 && draft.Goal == null && draft.Mode == CheckMode.Net && draft.Active == 1, "the draft of A stayed active in B");
        draft.Append(Twist(77, 3, 1)); draft.SetGoal("B goal"); string wordB = draft.Statement;
        Check(demo.SwitchOrbit(OrbitAtlas.Name(a)) == null && draft.Statement == wordA && draft.Goal == OperationDraft.SampleGoal && draft.Mode == CheckMode.Strict && draft.Active == 2, "A's draft was not restored");
        Check(demo.SwitchOrbit(OrbitAtlas.Name(b)) == null && draft.Statement == wordB && draft.Goal == "B goal" && draft.Mode == CheckMode.Net, "B's draft was not restored");
        draft.SetRoute("direct"); demo.SwitchOrbit(OrbitAtlas.Name(a)); Check(draft.Route == InputRoute.Draft, "A's route"); demo.SwitchOrbit(OrbitAtlas.Name(b)); Check(draft.Route == InputRoute.Direct, "B's route");
        Check(Read("app/Live/LabSolve.cs").Contains("{ Draft = Draft }", StringComparison.Ordinal), "the lab's draft belongs to no orbit");
    }

    // Q5: replaying a kept branch and choosing the preview's side are commands of the registry.
    private static void A6ReplayAndSidesResolveThroughTheRegistry()
    {
        CommandRegistry registry = LabRegistry();
        string[] rows = { "session.branch.replay", "preview.side.both", "preview.side.before", "preview.side.after", "preview.side.next" };
        foreach (string id in rows) Check(registry.Commands.ContainsKey(id), id + " is not a command");
        Check(registry.Commands["session.branch.replay"].Kind == "query" && rows.Skip(1).All(id => registry.Commands[id].Kind == "view"), "a replay that commits, or a side that is not a view");
        // The command table is not edited: the rows join it at its freeze, and a repeated id is refused.
        CommandRegistry table = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        Check(rows.All(id => !table.Commands.ContainsKey(id)), "a draft row is in the command table");
        Refuse(() => table.AddRows(Read("data/design/command-rows-draft.json").Replace("\"session.branch.replay\"", "\"session.undo\"")), "session.undo");
        Check(CommandIcons.Parse(Read("data/design/command-icons.json")).Missing(rows).Count == 0, "a draft row has neither an icon nor the label-only mark");
        // The palette lists each one, and a key can be bound to each one.
        Keymap keys = Keymap.Parse(Read("data/design/keymap.json"), registry);
        var palette = new PaletteModel(registry, keys, (_, _) => null);
        foreach (string id in rows) Check(palette.Rows(registry.Commands[id].Label).Any(r => r.Command == id), id + " is not in the palette");
        Keymap bound = Keymap.Parse("{\"format\":\"magic600-design-keymap\",\"version\":1,\"radials\":{},\"bindings\":[{\"key\":\"S\",\"context\":\"preview\",\"command\":\"preview.side.next\"},{\"key\":\"Shift+R\",\"context\":\"global\",\"command\":\"session.branch.replay\",\"argument\":\"0\",\"label\":\"Replay the first kept branch\"}]}", registry, keys);
        Check(bound.Resolve("S", new[] { "preview", "global" })?.Command == "preview.side.next" && bound.Resolve("Shift+R", new[] { "global" }) is { Command: "session.branch.replay", Argument: "0" }, "a draft row cannot be bound");
        // The surfaces send them and the lab carries them out.
        string panel = Read("app/Chrome/Ui/SessionPanels.cs"), demo = Read("app/Live/LabDemo.cs");
        Check(panel.Contains("Send(\"session.branch.replay\"", StringComparison.Ordinal) && panel.Contains("Safe.Raise(Command, \"preview.side.next\"", StringComparison.Ordinal) &&
            !panel.Contains("Model.CycleSide", StringComparison.Ordinal) && !panel.Contains("Safe.Raise(Replay", StringComparison.Ordinal), "a surface calls a local handler");
        string handles = Body(demo, "public static bool Handles", ";");
        foreach (string id in rows) Check(handles.Contains("\"" + id + "\"", StringComparison.Ordinal) && demo.Contains("case \"" + id + "\":", StringComparison.Ordinal), id + " is not handled by the lab");
        Check(Read("app/Live/LabFrame.cs").Contains("registry.AddRows(File.ReadAllText(Path.Combine(folder, \"command-rows-draft.json\")));", StringComparison.Ordinal), "the lab does not register the draft rows");
    }

    // B-8: the gallery's pane states are the docks' five, Busy among them, drawn by the box the docks use.
    private static void A6GalleryPaneStatesAreTheDocks()
    {
        Sequence(PanelStates.Five.Select(PanelStates.Id), new[] { "empty", "loading", "error", "stale", "busy" });
        Check(PanelStates.Five.All(c => PanelStates.Example(c).Length > 8), "a specimen state without its reason");
        string gallery = Read("app/Chrome/Ui/Gallery.cs"), page = gallery.Substring(gallery.IndexOf("private void PanePage", StringComparison.Ordinal));
        Check(page.Contains("foreach (PanelCondition condition in PanelStates.Five)", StringComparison.Ordinal) && page.Contains("new PanelStateBox(", StringComparison.Ordinal) &&
            !page.Contains("Placeholders", StringComparison.Ordinal) && !gallery.Contains("ChromeBody.PaneStates", StringComparison.Ordinal), "the gallery draws its own pane states");
        string frame = Read("app/Live/LabFrame.cs");
        foreach (PanelCondition c in PanelStates.Five) Check(frame.Contains("PanelCondition." + c + ",", StringComparison.Ordinal), c + " is not among the docks' demo states");
    }

    // B-9: the host draws a script's frame from the recorded author class: the mark alone, a thin border, a
    // patterned border.
    private static void A6ScriptFramesByAuthorClass()
    {
        Check(ScriptMarks.Of(true, true, false) == AuthorClass.BuiltIn && ScriptMarks.Of(true, true, true) == AuthorClass.BuiltIn, "a verified built-in tool");
        Check(ScriptMarks.Of(true, false, true) == AuthorClass.Own && ScriptMarks.Of(false, false, true) == AuthorClass.Own, "the solver's own script, a fork of a built-in tool included");
        Check(ScriptMarks.Of(true, false, false) == AuthorClass.Other && ScriptMarks.Of(false, false, false) == AuthorClass.Other && ScriptMarks.Of(false, true, false) == AuthorClass.Other,
            "a modified built-in nobody forked, a package by another author, a module of unknown origin");
        string[] frames = new[] { AuthorClass.BuiltIn, AuthorClass.Own, AuthorClass.Other }.Select(ScriptMarks.Frame).ToArray();
        Check(frames.Distinct().Count() == 3 && frames.All(f => f.StartsWith("ScriptFrame", StringComparison.Ordinal)) && !ScriptMarks.Bordered(AuthorClass.BuiltIn) && ScriptMarks.Bordered(AuthorClass.Own) &&
            !ScriptMarks.Patterned(AuthorClass.Own) && ScriptMarks.Patterned(AuthorClass.Other), "one frame for every class");
        string theme = Read("app/Chrome/Ui/ChromeTheme.cs"), panels = Read("app/Chrome/Ui/ScriptPanels.cs"), scripts = Read("app/Live/LabScripts.cs");
        foreach (string f in frames) Check(theme.Contains("theme.SetTypeVariation(\"" + f + "\", \"PanelContainer\")", StringComparison.Ordinal), f + " has no style");
        Check(Body(theme, "SetStylebox(\"panel\", \"ScriptFrameBuiltIn\"", ";").Contains("null", StringComparison.Ordinal) && Body(theme, "SetStylebox(\"panel\", \"ScriptFrame\",", ";").Contains("\"border.width\"", StringComparison.Ordinal),
            "the built-in frame has a border, or the own frame is not thin");
        Check(panels.Contains("AuthorClass author = AuthorClass.Other)", StringComparison.Ordinal) && panels.Contains("public void Show(IReadOnlyList<CanvasMark> marks, string source, AuthorClass author)", StringComparison.Ordinal) &&
            panels.Contains("on.DrawDashedLine(", StringComparison.Ordinal) && !panels.Contains("ThemeTypeVariation = \"ScriptFrame\"", StringComparison.Ordinal), "a host constructor makes one frame decision for every author");
        Check(System.Text.RegularExpressions.Regex.Matches(scripts, @"AuthorOf\(").Count >= 6 && !scripts.Contains("ThemeTypeVariation = \"ScriptFrame\"", StringComparison.Ordinal), "the lab does not pass the author class");
        Check(Read("app/Live/Sweep.cs").Contains("StartsWith(\"ScriptFrame\"", StringComparison.Ordinal), "the sweep misses a frame of another class");
        Check(Read("app/Chrome/Ui/Gallery.cs").Contains("foreach (AuthorClass author in new[] { AuthorClass.BuiltIn, AuthorClass.Own, AuthorClass.Other })", StringComparison.Ordinal), "the gallery shows one frame");
    }

    // B-10: an anchor sits where the view's own projection puts its point, so it follows the camera.
    private static void A6AnchorsFollowTheViewsProjection()
    {
        var places = new ScenePlaces(Root, Mesh); OrbitAtlas atlas = Atlas;
        Check(places.Orbits == OrbitAtlas.Count, "orbits in the model: " + places.Orbits);
        for (int o = 0; o < OrbitAtlas.Count; o++) Check(places.Pieces(o) == atlas.Orbits[o].Pieces, $"O{o:00}: {places.Pieces(o)} pieces in the model, {atlas.Orbits[o].Pieces} in the atlas");
        var values = new StructureValues(.9, .8, 3, 1, 1, ProjectionKind.Orthographic); var layout = new GlobalLayout(Mesh.CellStructure); double[] spin = Geometry.Identity();
        Func<int, int, (double X, double Y)?> View(double[] q) => (o, p) => places.Slot(o, p) is { } slot ? ScenePlaces.InView(places.Project(slot, values, q, spin, Turn, 0, layout, FocusFrame.Neutral)) : null;
        double[] still = Geometry.Identity(), turned = Geometry.Identity(); Geometry.Rotate(turned, 0, 2, .6); Geometry.Rotate(turned, 1, 2, -.4);
        SolvingState s = new SolvingDemo(atlas).State;
        var before = StripAnchors.Of(s, View(still)); var after = StripAnchors.Of(s, View(turned));
        foreach (string slot in new[] { "target", "buffer_a", "buffer_b", "wanted" }) Check(before.ContainsKey(slot) && after.ContainsKey(slot), slot + " has no anchor");
        // The Target stays put and the view turns: its anchor follows the projection of its position.
        (double X, double Y) target = View(turned)(s.Orbit, s.Target!.Value)!.Value;
        Check(Math.Abs(after["target"].X - before["target"].X) + Math.Abs(after["target"].Y - before["target"].Y) > 1e-3 && after["target"].X == target.X && after["target"].Y == target.Y && !after["target"].OnPiece,
            "the Target anchor did not follow the camera");
        // A displaced piece: the Wanted anchor sits on the position the piece is at now.
        int at = s.WantedAt!.Value; (double X, double Y) wanted = View(turned)(s.Orbit, at)!.Value;
        Check(at != s.Wanted && after["wanted"].X == wanted.X && after["wanted"].Y == wanted.Y && after["wanted"].OnPiece, "the Wanted anchor is not on its piece's place");
        // No projection, no anchor; a position outside the orbit has no stand-in.
        Check(StripAnchors.Of(s, null).Count == 0 && places.Slot(s.Orbit, -1) == null && places.Slot(s.Orbit, places.Pieces(s.Orbit)) == null, "an anchor was placed from an id");
        string ui = Read("app/Chrome/Ui/SolvingSurface.cs"), frame = Read("app/Live/LabFrame.cs");
        Check(!ui.Contains("2654435761", StringComparison.Ordinal) && ui.Contains("StripAnchors.Of(strip.Demo.State, Project)", StringComparison.Ordinal) &&
            frame.Contains("m.GetShaderParameter(\"q\").AsProjection()", StringComparison.Ordinal) && Read("app/Live/LabSolve.cs").Contains("GlobalAnchors.Project = globalPlace; LocalAnchors.Project = localPlace;", StringComparison.Ordinal),
            "the lab places anchors from ids");
    }

    // B-6: Solve's left dock is the Library and History stays in the right dock; a filtered macro joins Core from
    // the keyboard as one change, and Shift+Enter previews it alone.
    private static void A6LibraryInSolve()
    {
        var library = new LibraryModel();
        Check(library.Current?.Name == "Three-cycle A" && library.Shown.All(r => LibraryModel.BranchOf(r.Kind) == "Macros"), "a fresh Library does not open on the macros");
        foreach (LibraryRecord m in library.Records.Where(r => r.IsMacro))
        {
            Check(SampleCheck.Check(m.Word).Status == PreviewStatus.Ready && LibraryModel.Signature(m.Word).Count == SampleCheck.Orbits.Count && !LibraryModel.Signature(m.Word)[SampleCheck.Locked] &&
                LibraryModel.SignatureText(m.Word).Contains("unverified", StringComparison.Ordinal), m.Name + " is not a sample macro the check passes, or its signature claims a certificate");
        }
        Check(LibraryModel.Branches.Select(b => b.Title).SequenceEqual(new[] { "Macros", "Templates", "Sets", "Blocks", "Saved states", "Built-in" }) &&
            library.Records.Count(r => r.Kind == LibraryKind.BuiltInTool) == 7 && Enum.GetValues<LibraryKind>().Select(LibraryModel.Glyph).Distinct().Count() == 8, "the tree's branches, the shipped tools or one glyph per kind");
        // Filter in place, move by key, add to the active phase (Core by default) as one change.
        library.SetFilter("swap"); library.Move(1); library.Move(-1);
        var draft = new OperationDraft(); int changes = 0; draft.Changed += () => changes++;
        LibraryRecord swap = library.Current!;
        Check(swap.Name == "Swap pair B" && library.Shown.Count == 1 && draft.Active == 1 && library.Insert(draft) == null && changes == 1 &&
            draft.Twists(1).SequenceEqual(swap.Word) && draft.Twists(0).Count == 0 && draft.Twists(2).Count == 0, "the filtered macro did not join Core as one change");
        Check(draft.CanStage && draft.StagedCheck().Status == PreviewStatus.Ready, "the draft with the macro does not stage");
        // Only a macro joins a phase; Direct has no phase; a filter reaches folded branches; a name selects anywhere.
        library.SetFilter("built"); Check(library.IsOpen("Built-in") && library.Shown.Count == 7 && library.Insert(draft) != null && changes == 1, "a built-in tool joined a phase, or a filter missed a folded branch");
        Check(library.Select("Three-cycle on roles") == null && library.Filter == "" && library.Insert(draft)!.Contains("only a macro", StringComparison.Ordinal), "a Template joined a phase unbound");
        Check(library.Select("nothing") != null && library.Macro("Orient twin").Macro != null, "a missing record was selected");
        draft.SetRoute("direct"); Check(library.Insert(draft, "Orient twin")!.Contains("Direct", StringComparison.Ordinal) && draft.Twists(1).SequenceEqual(swap.Word), "a macro joined a phase in Direct");
        library.Fold("Macros"); library.SetFilter(""); Check(!library.IsOpen("Macros") && !library.Shown.Any(r => r.IsMacro) && library.Shown.All(r => r.Kind == LibraryKind.Template) && library.Current?.Kind == LibraryKind.Template, "a folded branch still shows its rows");
        Refuse(() => new LibraryModel(new[] { library.Records[0], library.Records[0] }), "share a name");
        // The shipped Solve layout: Library on the left, History on the right; Analyse keeps History one click away.
        WorkspaceSet shipped = WorkspaceSet.Load(FileAt("data/design/workspaces.json"));
        var frame = new FrameModel();
        foreach (string id in new[] { "library", "scripts", "tokens" }) frame.AddDockPanel(DockSide.Left, id);
        foreach (string id in new[] { "history", "inspector", "slot.1", "slot.2", "tune", "appearance" }) frame.AddDockPanel(DockSide.Right, id);
        foreach (string id in new[] { "problems", "console", "tasks" }) frame.AddDockPanel(DockSide.Bottom, id);
        Check(frame.Apply(shipped.Stored["solve"]) && frame.Docks[DockSide.Left].Open == "library" && frame.Docks[DockSide.Right].Open == "history" && frame.Docks[DockSide.Bottom].Open == null,
            "Solve does not open the Library on the left and History on the right");
        Check(frame.Apply(shipped.Stored["analyse"]) && frame.Docks[DockSide.Right].Panels.Contains("history"), "History is not reachable in Analyse");
        // The lab registers the docks so, and the panel's keys send the registry's commands.
        string lab = Read("app/Live/LabFrame.cs"), panel = Read("app/Chrome/Ui/LibraryPanel.cs"), solve = Read("app/Live/LabSolve.cs");
        Check(lab.Contains("(DockSide.Left, \"library\", \"Library\", \"panel.library\"", StringComparison.Ordinal) && lab.Contains("(DockSide.Right, \"history\", \"History\", \"panel.history\"", StringComparison.Ordinal) &&
            lab.Contains("frame.SetContent(\"library\", solve.LibraryView);", StringComparison.Ordinal), "the lab does not register the Library on the left and History on the right");
        CommandRegistry registry = LabRegistry();
        Check(new[] { "macro.add", "macro.select", "macro.filter", "macro.base.open", "operation.preview" }.All(registry.Commands.ContainsKey), "a Library command is not in the registry");
        Check(panel.Contains("Filter.GuiInput += Key;", StringComparison.Ordinal) && panel.Contains("when key.ShiftPressed: Send(\"operation.preview\", name == null ? null : \"macro:\" + name)", StringComparison.Ordinal) &&
            panel.Contains("Godot.Key.KpEnter: Send(\"macro.add\", name)", StringComparison.Ordinal) && solve.Contains("why = Library.Insert(Draft, argument);", StringComparison.Ordinal) &&
            solve.Contains("public const string MacroPreview = \"macro:\";", StringComparison.Ordinal) && lab.Contains("solve.PreviewMacro(", StringComparison.Ordinal), "the Library's keys do not reach the draft and the preview");
        IconSet icons = IconSet.Parse(Read("data/design/icons.json"));
        Check(icons.Icons.ContainsKey("panel.library") && Enum.GetValues<LibraryKind>().All(k => icons.Icons.ContainsKey(LibraryModel.Glyph(k))), "a Library glyph has no icon");
    }

    // Q2: no numeric gesture goes finer than the step. Every value a field takes lies on its grid, and the display
    // has the decimals the grid needs, so keyboard, ladder, typed input and drag agree on what is stored and shown.
    private static void A6NumericGesturesStayOnTheGrid()
    {
        var model = new InspectorModel();
        InspectField axis = model.AddNumber(InspectorModel.Preview, "p.axis", "axis", 0, 3, 1, 1, 1);
        InspectField gap = model.AddNumber(InspectorModel.View, "v.gap", "gap", .05, .9, .005, .3, .3);
        bool Shown(InspectField f) => double.Parse(f.Display, System.Globalization.CultureInfo.InvariantCulture) == Math.Round(f.Value, f.Decimals);
        Check(!model.Nudge("p.axis", 1, .1) && axis.Value == 1 && axis.Display == "1", "a fine nudge changed a unit-step value");
        Check(model.Set("p.axis", 2.4) && axis.Value == 2 && axis.Display == "2" && model.Step("p.axis", 1) && axis.Value == 3 && !model.Nudge("p.axis", 1, 10), "typed or laddered unit values left the grid");
        Check(axis.Decimals == 0 && gap.Decimals == 3 && gap.Display == "0.300", "the display has not the grid's decimals");
        Check(model.Nudge("v.gap", 1) && Math.Abs(gap.Value - .305) < 1e-12 && gap.Display == "0.305" && Shown(gap), "one step is not shown");
        Check(model.Set("v.gap", .3124) && Math.Abs(gap.Value - .31) < 1e-12 && gap.Display == "0.310" && Shown(gap), "a typed value is not snapped to the grid");
        Check(model.Nudge("v.gap", -3, 10) && Math.Abs(gap.Value - .16) < 1e-12 && Shown(gap), "a coarse nudge left the grid");
        string ui = Read("app/Chrome/Ui/InspectorPanels.cs");
        Check(!ui.Contains("? .1", StringComparison.Ordinal) && ui.Contains("motion.AltPressed ? 1 :", StringComparison.Ordinal), "a gesture still passes a tenth of a step");
    }
}
