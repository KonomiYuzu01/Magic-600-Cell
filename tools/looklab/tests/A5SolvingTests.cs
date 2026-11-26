using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    private static OrbitAtlas Atlas => OrbitAtlas.Parse(File.ReadAllText(Path.Combine(Root, "assets/seed_atlas.json")));

    // The relationship strip (ADR-D01 revision 4, items 62 and 63) on the three demo states.
    private static void A5SolvingStripFactsAndInvalidTuple()
    {
        OrbitAtlas atlas = Atlas; var demo = new SolvingDemo(atlas);
        Check(atlas.Orbits.Count == 35 && atlas.StageOrder.OrderBy(x => x).SequenceEqual(Enumerable.Range(0, 35)) && OrbitAtlas.Name(7) == "O07", "the atlas does not give 35 orbits in one stage order");
        Check(atlas.Orbits[3].Touches.SequenceEqual(new[] { 4, 5 }) && atlas.TouchedBy(4).Contains(3) && atlas.Orbits[34].Orientation == "A5", "the seed dependencies or the groups are not the atlas's");
        Sequence(demo.States.Select(s => s.Id), new[] { "mid", "endgame", "fresh" });
        string[] words = { "Target", "Occupant", "Wanted", "Buffer A", "Buffer B", "Target frame", "Residual", "Next" };
        foreach (SolvingState state in demo.States)
        {
            Sequence(state.Cells().Select(c => c.Word), words);
            Check(state.Problems().Count == 0, state.Id + ": a demo state starts ill-formed");
            Check(state.Unfinished().Count == state.Residual.NotHome && state.Unfinished().SequenceEqual(state.Unfinished().OrderBy(p => p)), state.Id + ": the unfinished positions are not the residual's, in position-id order");
        }
        // Mid-solve: the facts, and the Wanted piece drawn as derived under Home.
        SolvingState mid = demo.State; StripCell Cell(string slot) => demo.State.Cells().First(c => c.Slot == slot);
        Check(mid.WantedPlace() == "in A" && Cell("wanted").State == "derived" && Cell("occupant").State == "mismatch" && Cell("buffer_a").Detail == OrbitAtlas.Piece(mid.Wanted!.Value), "mid-solve does not say that Wanted is in A");
        Check(Cell("next").State == "other_orbit" && Cell("residual").Value == "14", "Next in another orbit or the residual numeral");
        // Captured names its revision and keeps the Wanted piece when the Target moves on.
        int wanted = mid.Wanted!.Value; Check(demo.SetFrame("captured") == null && Cell("wanted").State == "set" && Cell("target_frame").Detail == "r 1205", "Captured does not name its revision");
        Check(demo.SetTarget(null) == null && mid.Wanted == wanted && mid.Target != wanted, "a capture follows the Target");
        Check(demo.SetFrame("home") == null && mid.Wanted == mid.Target && demo.SetFrame("sideways") != null, "Home does not derive the Wanted piece, or a wrong frame is taken");
        // An ill-formed tuple is shown as invalid and never corrected.
        while (mid.Problems().Count > 0) demo.SetTarget(null);
        int target = mid.Target!.Value, before = mid.BufferA!.Value;
        Check(demo.AssignBuffer("A:" + OrbitAtlas.Position(target)) == null && mid.BufferA == target && mid.Target == target, "an ill-formed tuple was corrected");
        Check(mid.Problems().Single().Text == "Target and Buffer A are one position" && Cell("target").State == "invalid" && Cell("buffer_a").State == "invalid" && Cell("buffer_b").State == "set", "an ill-formed tuple does not show as invalid");
        Check(demo.AssignBuffer("A:" + OrbitAtlas.Position(before)) == null && mid.Problems().Count == 0 && demo.AssignBuffer("C") != null, "the tuple does not become well-formed again");
        Check(demo.AssignBuffer("B:9999999") == null && Cell("buffer_b").State == "invalid" && demo.AssignBuffer("B") == null && mid.Problems().Count == 0, "a place outside the orbit is not invalid");
        // Next: one action each sets, clears and activates; activating keeps the orbit's tuple.
        int orbit = mid.Orbit; (int nextOrbit, int nextPiece) = mid.Next!.Value;
        Check(demo.ActivateNext() == null && mid.Orbit == nextOrbit && mid.Target == nextPiece && mid.Next == null && mid.BufferA == null, "activating Next");
        Check(demo.ClearNext() != null && demo.SetNext(null) == null && Cell("next").State == "set" && demo.ClearNext() == null && Cell("next").State == "unset", "setting and clearing Next");
        Check(demo.SwitchOrbit(OrbitAtlas.Name(orbit)) == null && mid.Target == target && mid.BufferA == before, "the orbit that was left lost its tuple");
        // Unknown is never zero, and a fresh scramble sets nothing.
        int unknown = atlas.StageOrder[20];
        Check(mid.Classes(unknown).All(c => c.Numeral == "unknown") && !mid.Residuals[unknown].ExactlySolved && mid.Residuals[unknown].Level(7200) == null && mid.Residuals[atlas.StageOrder[0]].ExactlySolved, "unknown reads as zero");
        demo.NextState(); Check(demo.State.Id == "endgame" && demo.State.WantedPlace() == "in A" && demo.State.Classes(demo.State.Orbit).Last() == ("Last buffer", "permitted"), "the endgame at the last buffer");
        demo.NextState(); Check(demo.State.Cells().Take(5).All(c => c.State == "unset") && demo.SetNext(null) != null && demo.SetTarget(null) == null && Cell("target").State == "set", "a fresh scramble");
        // The strip states facts: no word of it names a macro or a star.
        foreach (SolvingState state in demo.States)
            Check(!state.Cells().Any(c => (c.Value + " " + c.Detail).Contains("macro", StringComparison.OrdinalIgnoreCase) || (c.Value + c.Detail).Contains("star", StringComparison.OrdinalIgnoreCase)), "the strip names a macro or a star");
        // The lab shows the owner's words without editing the command table.
        CommandRegistry registry = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        registry.Relabel(Read("data/design/command-labels.json"));
        Check(registry.Commands["target.current.set"].Label == "Set Target" && registry.Commands["input.destination.set"].Label == "Draft or Direct" && registry.Commands["operation.phase.select"].Label == "Select phase Prepare / Core / Cleanup", "the owner's words for the solving commands");
        Refuse(() => registry.Relabel("{\"labels\":{\"no.such\":\"x\"}}"), "unknown command");
    }

    // The orbit rail (ADR-D01.64): the order never depends on the state, and the marks are separate.
    private static void A5SolvingRailOrderMarksAndJump()
    {
        OrbitAtlas atlas = Atlas; var demo = new SolvingDemo(atlas);
        foreach (bool byId in new[] { false, true })
        {
            int[][] orders = demo.States.Select(s => OrbitRailModel.Cells(s, byId).Select(c => c.Orbit).ToArray()).ToArray();
            Check(orders.All(o => o.SequenceEqual(orders[0])) && orders[0].Length == 35 && orders[0].SequenceEqual(OrbitRailModel.Order(atlas, byId)), "the rail's order depends on the state");
        }
        Check(OrbitRailModel.Order(atlas, false).SequenceEqual(atlas.StageOrder) && OrbitRailModel.Order(atlas, true).SequenceEqual(Enumerable.Range(0, 35)) && OrbitRailModel.OrderLabel == "one admissible order", "the default order is not the seed stage order");
        // The order is the same after the working orbit, the Target and Next change.
        int[] before = OrbitRailModel.Cells(demo.State, false).Select(c => c.Orbit).ToArray();
        demo.SwitchOrbit("O05"); demo.SetTarget(null); demo.SetNext(null);
        Sequence(OrbitRailModel.Cells(demo.State, false).Select(c => c.Orbit), before);
        var cells = OrbitRailModel.Cells(demo.State, false);
        Check(cells.Count(c => c.Working) == 1 && cells.Single(c => c.Working).Orbit == 5 && cells.Count(c => c.Next) == 1, "the working mark or the Next glyph");
        // Solved, lock and working are three separate marks: each combination that the demo holds is kept apart.
        Check(cells.Any(c => c.Solved && c.Lock == OrbitLock.None) && cells.Any(c => c.Solved && c.Lock == OrbitLock.Partial) && cells.Any(c => c.Solved && c.Lock == OrbitLock.Whole) && cells.Any(c => !c.Solved && c.Lock == OrbitLock.Partial), "solved and lock are one mark");
        RailCell unknown = cells.Single(c => c.Level == null);
        Check(unknown.Status == "unknown_frame" && !unknown.Solved && cells.Where(c => c.Solved).All(c => c.Level == 0 && c.Status == "solved"), "unknown is drawn as zero");
        Check(cells.All(c => c.Id == OrbitAtlas.Name(c.Orbit) && OrbitAtlas.Groups.Contains(c.Group)) && cells.Select(c => c.Group).Distinct().Count() == 5, "ids or groups");
        Check(OrbitRailModel.Cells(demo.States[1], false).Single(c => c.Working).Status == "permitted_residual", "the endgame at the last buffer");
        // A held orbit key followed by two digits; digits without the key do nothing.
        var jump = new OrbitJump();
        Check(jump.Digit(1) == null && !jump.Held, "a digit without the orbit key");
        jump.Hold(); Check(jump.Digit(0) == null && jump.Typed == "0" && jump.Digit(7) == 7 && jump.Typed == "", "O then 07");
        Check(jump.Digit(9) == null && jump.Digit(9) == null && jump.Digit(3) == null && jump.Digit(4) == 34, "O then 99 names no orbit");
        jump.Digit(1); jump.Release(); Check(!jump.Held && jump.Typed == "" && jump.Digit(2) == null, "releasing the key");
    }

    // The draft line (ADR-D01.66): three phases, the route, a Goal that never gates, Net and Strict.
    private static void A5SolvingDraftLineGoalAndRoute()
    {
        var draft = new OperationDraft(); int changes = 0, cleared = 0; draft.Changed += () => changes++; draft.HeldCleared += () => cleared++;
        Sequence(OperationDraft.Phases, new[] { "Prepare", "Core", "Cleanup" });
        Check(draft.Route == InputRoute.Draft && OperationDraft.Phases[draft.Active] == "Core" && !draft.CanCommit && draft.GoalResult == GoalResult.Unset && draft.Checked == CheckResult.Unchecked, "an empty draft");
        // Twists join the active phase; the statement is the draft's projection.
        Check(draft.Append("a") && draft.SelectPhase("prepare") == null && draft.Append("s") && draft.SelectPhase(null) == null && draft.Active == 1 && draft.SelectPhase("middle") != null, "phases");
        Check(draft.Statement == "preview operation { prepare { s } core { a } cleanup {  } }" && draft.Word.SequenceEqual(new[] { "s", "a" }), draft.Statement);
        // An unmet Goal leaves the commit open: the Goal has its own result.
        draft.SetGoal(null);
        Check(draft.Goal == OperationDraft.SampleGoal && draft.GoalResult == GoalResult.Unmet && draft.CanCommit, "an unmet Goal closed the commit");
        draft.Append("b"); Check(draft.GoalResult == GoalResult.Met && draft.CanCommit, "a met Goal");
        draft.SetGoal(null); Check(draft.GoalResult == GoalResult.Unset && draft.CanCommit, "no Goal");
        // A word can pass Net and fail Strict; the chip never says one "safe".
        Check(draft.Result(CheckMode.Net) == CheckResult.Passed && draft.Result(CheckMode.Strict) == CheckResult.Failed && draft.Checked == CheckResult.Passed, "Net and Strict are one result");
        draft.SetMode(CheckMode.Strict); Check(draft.Checked == CheckResult.Failed && draft.CanStage && !draft.CanCommit, "a failed Strict check left the commit open");
        Check(draft.Edit("cleanup: s'") == null && draft.Checked == CheckResult.Passed && draft.Edit("nowhere: x") != null && draft.Edit(null) == null && draft.Twists(1).Count == 1, "editing a phase");
        // The Draft/Direct switch clears held keys, and in Direct no twist joins a phase.
        draft.Hold("O"); draft.Hold("Shift");
        Check(draft.SetRoute(null) == null && draft.Route == InputRoute.Direct && draft.Held.Count == 0 && cleared == 1, "the switch kept a held key");
        int count = draft.Count; Check(!draft.Append("c") && draft.Count == count && !draft.CanCommit, "a Direct twist joined the draft");
        draft.Hold("O"); Check(draft.SetRoute("draft") == null && draft.Held.Count == 0 && cleared == 2 && draft.SetRoute("sideways") != null && draft.CanCommit && changes > 8, "back to Draft");
        // The twelve commands of the solving surface have the owner's words and are handled by the lab.
        CommandRegistry registry = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        string labels = Read("data/design/command-labels.json"); registry.Relabel(labels);
        string[] twelve = { "target.current.set", "target.next.set", "target.next.activate", "target.next.clear", "buffer.assign", "target.frame.set", "orbit.switch", "input.destination.set", "intent.set",
            "operation.phase.select", "operation.phase.edit", "operation.phase.append_twist" };
        string lab = File.ReadAllText(Path.Combine(Lab, "app/Live/LabSolve.cs")); string handles = lab.Substring(lab.IndexOf("public static bool Handles", StringComparison.Ordinal)); handles = handles.Substring(0, handles.IndexOf(';'));
        foreach (string id in twelve) Check(registry.Commands.ContainsKey(id) && labels.Contains("\"" + id + "\"", StringComparison.Ordinal) && handles.Contains("\"" + id + "\"", StringComparison.Ordinal), id + " still reads as not in this sample");
    }

    // The key map (ADR-D01.74): a keyboard of the keymap in force, shown while its key is held, pinnable.
    private static void A5SolvingKeyMapHeldPinnedAndNamed()
    {
        CommandRegistry registry = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        registry.Relabel(Read("data/design/command-labels.json"));
        Keymap keys = Keymap.Parse(Read("data/design/keymap.json"), registry);
        // It shows while its key is held; a short press pins it and the next short press lets it go.
        var show = new KeyMapShow(); int changes = 0; show.Changed += () => changes++;
        Check(!show.Visible, "shown at rest");
        show.Press(10); show.Press(10.1); Check(show.Visible && show.Held && !show.Pinned, "not shown while its key is held");
        show.Release(11); Check(!show.Visible && !show.Pinned, "a long hold pinned it");
        show.Press(20); show.Release(20.1); Check(show.Visible && show.Pinned && !show.Held, "a short press did not pin it");
        show.Press(30); show.Release(31); Check(show.Visible && show.Pinned, "a long hold let the pin go");
        show.Press(40); show.Release(40.1); Check(!show.Visible && changes == 8, "a short press did not let it go");
        // The header names the active key set, the place and the held modifiers.
        string[] local = { "local-view", "global" }, global = { "global-view", "global" }, none = { "global" };
        Check(KeyMapModel.Header("default", local, false, false, false) == "Key set: default \u00b7 pointer over Local \u00b7 no modifier held", KeyMapModel.Header("default", local, false, false, false));
        Check(KeyMapModel.Header("left-hand with your bindings", none, true, false, true) == "Key set: left-hand with your bindings \u00b7 everywhere \u00b7 with Ctrl+Shift", "the header");
        // Every key of the keymap that has a cap is on the board once.
        string[] caps = KeyMapModel.Rows.SelectMany(r => r).Select(k => k.Key).ToArray();
        Check(caps.Length == caps.Distinct().Count() && caps.Length > 55, "a key is drawn twice");
        foreach (Binding b in keys.Bindings) Check(caps.Contains(b.Key.Split('+')[^1]), b.Key + " has no cap");
        KeyCap Cap(IReadOnlyList<string> contexts, string key, bool ctrl = false, bool alt = false, bool shift = false, params string[] down) =>
            KeyMapModel.Caps(keys, registry, contexts, ctrl, alt, shift, down, new Dictionary<string, string> { ["F9"] = "Tour" }).SelectMany(r => r).Single(c => c.Key == key);
        // What a key does follows the place: the same key, two meanings, each with its place named.
        KeyCap r1 = Cap(local, "R"), r2 = Cap(global, "R"), r3 = Cap(none, "R");
        Check(r1.State == KeyCapState.Bound && r2.State == KeyCapState.Bound && r1.Label != r2.Label && r1.Context == "local-view" && r2.Context == "global-view", "R over the two views");
        Check(r3.State == KeyCapState.OutOfContext && r3.Label != null && r3.Context != null && KeyMapModel.Where.ContainsKey(r3.Context), "a key bound elsewhere reads as unbound or as bound here");
        Check(Cap(local, "J").State == KeyCapState.Twist && Cap(local, "K").State == KeyCapState.Unbound && Cap(local, "K").Label == null && Cap(local, "F9").Label == "Tour" && Cap(local, "F9").Context == KeyMapModel.Host, "twist, unbound and host keys");
        // Modifiers pick the layer; a held key is marked and keeps its meaning.
        Check(Cap(none, "N").Label != Cap(none, "N", shift: true).Label && Cap(none, "N", alt: true).State == KeyCapState.Bound && Cap(none, "Z").State == KeyCapState.Unbound && Cap(none, "Z", ctrl: true).State == KeyCapState.Bound, "modifier layers");
        KeyCap held = Cap(none, "E", false, false, false, "E");
        Check(held.State == KeyCapState.Held && held.Down && held.Label == Cap(none, "E").Label, "a held key");
        // Same key set, place and modifiers: the same meaning for every key.
        var one = KeyMapModel.Caps(keys, registry, local, false, false, false, Array.Empty<string>()); var two = KeyMapModel.Caps(keys, registry, local, false, false, false, Array.Empty<string>());
        Check(one.SelectMany(r => r).SequenceEqual(two.SelectMany(r => r)), "a key changed meaning with nothing named");
        foreach (string role in new[] { "bound", "unbound", "held", "out_of_context", "twist" }) Check(Read("data/design/system.json").Contains("\"keymap.key." + role + "\"", StringComparison.Ordinal), "keymap.key." + role);
    }

    // Inline parameters of the commit bar (ADR-D01.11): the numbers of the staged statement, edited in place.
    private static void A5CommitBarInlineParameters()
    {
        var twist = InlineParameters.Find("preview twist(cell 402, axis 1, +1)");
        Sequence(twist.Select(p => p.Name + "=" + p.Value), new[] { "cell=402", "axis=1", "turns=1" });
        Check(twist[2].Signed && !twist[0].Signed && twist.All(p => p.Value >= p.Min && p.Value <= p.Max), "ranges");
        // Quoted text is never a parameter, and a number inside a name is not one.
        Sequence(InlineParameters.Find("preview run \"sweep 6.6cs\" on ring 4").Select(p => p.Name + "=" + p.Value), new[] { "ring=4" });
        Check(InlineParameters.Find("preview my.swap2(o7)").Count == 0 && InlineParameters.Find("").Count == 0, "a number inside a name");
        string s = "preview twist(cell 402, axis 1, +1)";
        Check(InlineParameters.With(s, twist[0], 9) == "preview twist(cell 9, axis 1, +1)" && InlineParameters.With(s, twist[0], 5000) == "preview twist(cell 599, axis 1, +1)" && InlineParameters.With(s, twist[1], -3) == "preview twist(cell 402, axis 0, +1)", "a value left its range");
        // A count of turns keeps its sign form and is never zero.
        Check(InlineParameters.With(s, twist[2], 2) == "preview twist(cell 402, axis 1, +2)" && InlineParameters.With(s, twist[2], 0) == "preview twist(cell 402, axis 1, -1)" && InlineParameters.With(s, twist[2], -9) == "preview twist(cell 402, axis 1, -2)", "turns");
        string back = InlineParameters.With(s, twist[2], 0); Check(InlineParameters.With(back, InlineParameters.Find(back)[2], 0).EndsWith("+1)", StringComparison.Ordinal), "turns from the other side");
        // The staged draft's parameters come in the order of its twists.
        var draft = new OperationDraft(); draft.Edit("prepare: twist(cell 5, axis 0, +1)"); draft.Edit("core: twist(cell 40, axis 2, -1); twist(cell 77, axis 3, +1)");
        var all = InlineParameters.Find(draft.Statement);
        Check(all.Count == 9 && all.Count == draft.Word.Sum(t => InlineParameters.Find(t).Count) && all[3].Value == 40 && all[8].Value == 1, "the draft's parameters");
        // The bar offers them and asks the host; it rebuilds nothing itself.
        string bar = File.ReadAllText(Path.Combine(Lab, "app/Chrome/Ui/SessionPanels.cs"));
        Check(bar.Contains("Safe.Raise(Parameter, index", StringComparison.Ordinal) && bar.Contains("InlineParameters.Find(p.Statement)", StringComparison.Ordinal), "the commit bar has no inline parameters");
    }

    // Two automatic screen checks: every action has a path from the keyboard, and each screen has a word limit.
    private static void A5ScreenChecksKeyboardAndWords()
    {
        CommandRegistry registry = LabRegistry();
        string text = Read("data/design/keymap.json"); Keymap keys = Keymap.Parse(text, registry);
        using System.Text.Json.JsonDocument checks = System.Text.Json.JsonDocument.Parse(Read("data/design/screen-words.json"));
        // The palette lists every command and opens from the keyboard in every key set, so a command is a path.
        var sets = new List<Keymap> { keys };
        foreach (string preset in Directory.GetFiles(Path.Combine(Lab, "data/design/keymaps"), "*.json")) sets.Add(Keymap.Parse(File.ReadAllText(preset), registry, keys));
        foreach (Keymap set in sets)
        {
            Check(set.Bindings.Any(b => b.Command == "command.find" && b.Context == Keymap.Global), "a key set cannot open the palette");
            Check(set.Bindings.Any(b => b.Command == "transient.close" && b.Context == Keymap.Global) && set.Bindings.Any(b => b.Command == "help.open"), "a key set cannot close an overlay or show the key map");
        }
        var palette = new PaletteModel(registry, keys, (_, _) => null);
        Check(registry.Commands.Keys.All(id => palette.Rows(registry.Commands[id].Label).Any(r => r.Command == id)), "a command is missing from the palette");
        // Every action an element raises or sends is a command of the registry, or is listed as pointer-only.
        var raised = new SortedSet<string>(StringComparer.Ordinal);
        foreach (string folder in new[] { "app/Chrome/Ui", "app/Live" })
            foreach (string file in Directory.GetFiles(Path.Combine(Lab, folder), "*.cs"))
                foreach (System.Text.RegularExpressions.Match m in System.Text.RegularExpressions.Regex.Matches(File.ReadAllText(file), "(?:Raise\\(Command, |Send\\()\"([a-z0-9_.]+)\""))
                    raised.Add(m.Groups[1].Value);
        string[] listed = checks.RootElement.GetProperty("pointer_only").EnumerateArray().Select(e => e.GetString()!).ToArray();
        string[] pointerOnly = raised.Where(id => !registry.Commands.ContainsKey(id)).ToArray();
        Check(raised.Count >= 10, "the scan found too few actions: " + raised.Count);
        Sequence(pointerOnly, listed, "actions with no path from the keyboard: " + string.Join(", ", pointerOnly));
        Check(listed.Length == 0, "an action is still pointer-only: " + string.Join(", ", listed));
        // A6 re-check N-7: on the solving surfaces (the Library, the History dock and the draft line) a button, a chip
        // or a row never acts by itself: each action sends a registry command, so a bound key reaches it too.
        foreach ((string file, string from, string? to) in new[] { ("app/Chrome/Ui/LibraryPanel.cs", "public sealed partial class LibraryPanel", "public sealed partial class SignatureMarks"),
            ("app/Chrome/Ui/DraftLine.cs", "public sealed partial class DraftLine", null), ("app/Chrome/Ui/SessionPanels.cs", "public sealed partial class HistoryPanel", "private static Control CursorLine") })
        {
            string all = File.ReadAllText(Path.Combine(Lab, file)); int start = all.IndexOf(from, StringComparison.Ordinal), end = to == null ? all.Length : all.IndexOf(to, start, StringComparison.Ordinal);
            Check(start >= 0 && end > start, file + ": the surface was not found");
            int actions = 0;
            foreach (System.Text.RegularExpressions.Match m in System.Text.RegularExpressions.Regex.Matches(all.Substring(start, end - start), @"(?<before>[^\n]{0,24})\(\) => (?<call>[A-Za-z_][A-Za-z_.]*)\((?<first>[^,)]*)"))
            {
                if (m.Groups["before"].Value.EndsWith("Changed += ", StringComparison.Ordinal) || m.Groups["before"].Value.EndsWith("Draw += ", StringComparison.Ordinal)) continue;
                actions++;
                string call = m.Groups["call"].Value;
                Check(call == "Send" || (call == "Safe.Raise" && m.Groups["first"].Value == "Command"), $"{file}: a widget-only action calls {call}({m.Groups["first"].Value}");
            }
            Check(actions >= 2, file + ": the scan found too few actions");
        }
        // The solving surface from the keyboard alone: eleven commands have a key, and the orbit jump is a held key.
        string[] keyed = { "target.current.set", "target.next.set", "target.next.activate", "target.next.clear", "buffer.assign", "target.frame.set", "input.destination.set", "intent.set",
            "operation.phase.select", "operation.phase.edit", "operation.phase.append_twist" };
        foreach (string id in keyed) Check(keys.Bindings.Any(b => b.Command == id), id + " has no key");
        Check(File.ReadAllText(Path.Combine(Lab, "app/Chrome/Ui/OrbitRail.cs")).Contains("Key.O ", StringComparison.Ordinal), "the orbit jump lost its key");
        // Each workspace has a word limit, and the running sweep holds each screen to it.
        var words = checks.RootElement.GetProperty("words").EnumerateObject().ToDictionary(p => p.Name, p => p.Value.GetInt32());
        Sequence(words.Keys.OrderBy(k => k, StringComparer.Ordinal), new[] { "analyse", "gallery", "reference", "script", "solve" });
        Check(words.Values.All(v => v > 0 && v < 400), "a word limit that limits nothing");
        Check(File.ReadAllText(Path.Combine(Lab, "app/Live/Sweep.cs")).Contains("over its limit of", StringComparison.Ordinal), "the sweep no longer counts words");
    }

    // History in full (ADR-D01.15, ADR-D01.16): branches switch without loss, behind a confirm gate.
    private static void A5HistoryBranchesInFull()
    {
        var h = new MoveHistory();
        HistoryEntry Add(string name, int moves) => h.Commit("hand", name, "10:00", "commit " + name, Enumerable.Range(0, moves).Select(i => name + i).ToArray())!;
        HistoryEntry a = Add("a", 2), b = Add("b", 3), c = Add("c", 1);
        string[] All() => h.Entries.Concat(h.Branches.SelectMany(x => x.Entries)).Select(e => e.Id).OrderBy(x => x, StringComparer.Ordinal).ToArray();
        // A commit in the middle records a branch from that point: the later entries of the old line are kept.
        h.RollTo(1); HistoryEntry d = Add("d", 2);
        Check(h.Branches.Count == 1 && h.Branches[0].From == a.Id && h.Branches[0].Entries.Select(e => e.Id).SequenceEqual(new[] { b.Id, c.Id }) && h.Entries.Select(e => e.Id).SequenceEqual(new[] { a.Id, d.Id }), "the branch of a commit in the middle");
        string[] every = All(); Check(every.Length == 4, "an entry was lost");
        // A partly undone entry refuses the switch, and so does a branch that does not exist.
        h.StepBack(); Check(h.Inner == 1 && h.SelectRefusal(0) != null && !h.SelectBranch(0) && h.Actions[^1].StartsWith("partial undo " + d.Id, StringComparison.Ordinal), "a switch inside a partial undo");
        h.Redo(); Check(h.SelectRefusal(0) == null && h.SelectRefusal(1) != null && h.SelectRefusal(-1) != null && !h.SelectBranch(3), "refusals");
        // The switch swaps the lines: the branch is applied to its end, the old later entries become the branch.
        int actions = h.Actions.Count;
        Check(h.SelectBranch(0) && h.Entries.Select(e => e.Id).SequenceEqual(new[] { a.Id, b.Id, c.Id }) && h.Cursor == 3 && h.Inner == 0, "the branch did not become the line");
        Check(h.Branches.Count == 1 && h.Branches[0].From == a.Id && h.Branches[0].Entries.Single().Id == d.Id && All().SequenceEqual(every) && h.Actions.Count == actions + 1 && h.Actions[^1].StartsWith("switch to the branch after " + a.Id, StringComparison.Ordinal), "the old line was rewritten or lost");
        // Switching back gives the earlier line again, and a branch whose starting entry left the line is refused.
        Check(h.SelectBranch(0) && h.Entries.Select(e => e.Id).SequenceEqual(new[] { a.Id, d.Id }) && All().SequenceEqual(every), "switching back");
        h.RollTo(0); Add("e", 1);
        Check(h.Branches.Count == 2 && h.Branches.Any(x => x.From == "start") && h.SelectRefusal(h.Branches.ToList().FindIndex(x => x.From == a.Id)) != null && h.SelectRefusal(h.Branches.ToList().FindIndex(x => x.From == "start")) == null, "a branch off the line");
        Check(All().Length == 5, "an entry was lost at the start");
        // The command exists, the lab handles it, and the panel sends it only from its confirm control.
        CommandRegistry registry = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        string panel = File.ReadAllText(Path.Combine(Lab, "app/Chrome/Ui/SessionPanels.cs")), lab = File.ReadAllText(Path.Combine(Lab, "app/Live/LabDemo.cs"));
        Check(registry.Commands.ContainsKey("session.branch.select") && lab.Contains("Moves.ConfirmRefusal(branch)", StringComparison.Ordinal), "the branch switch has no command or no gate");
        // The panel sends three requests: Switch (which only asks), the answer to the open question, and Cancel.
        Check(System.Text.RegularExpressions.Regex.Matches(panel, "Send\\(\"session.branch.select\"").Count == 3 && panel.Contains("\"confirm \" + at", StringComparison.Ordinal) &&
            panel.Contains("Send(\"session.branch.select\", \"cancel\")", StringComparison.Ordinal) && lab.Contains("History.Show(false); Moves.Ask(branch);", StringComparison.Ordinal) &&
            !lab.Contains("Moves.SelectBranch(", StringComparison.Ordinal), "the panel switches without the confirm gate");
    }

    // Stage 10 points 9 to 11: palette aliases and typed arguments, readout facts with lamps, the five panel states.
    private static void A5PaletteReadoutsAndPanelStates()
    {
        CommandRegistry registry = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        Keymap keys = Keymap.Parse(Read("data/design/keymap.json"), registry);
        var palette = new PaletteModel(registry, keys, (_, _) => null);
        string[] order = palette.Rows("").Select(r => r.Id).ToArray();
        PaletteRow target = palette.Rows("").First(r => r.Command == "target.current.set"), undo = palette.Rows("").First(r => r.Command == "session.undo");
        // An alias is found by a query, shows on its row and never reorders the rows.
        palette.Alias(target, "  zqaim | here\n"); Check(palette.Rows("zqaim").Single().Id == target.Id && palette.Rows("zqaim")[0].Alias == "zqaim   here" && palette.Rows("").Select(r => r.Id).SequenceEqual(order), "the alias");
        palette.Alias(undo, "zqback"); palette.Pin(undo, true);
        // Pins and aliases are kept and restored; an empty alias takes it away.
        var again = new PaletteModel(registry, keys, (_, _) => null); again.Restore(palette.Store());
        Check(again.Rows("zqback").Single().Pinned && again.Rows("zqaim").Single().Id == target.Id && again.Rows("")[0].Id == undo.Id && again.Store() == palette.Store(), "pins and aliases were not kept");
        again.Alias(target, " "); Check(again.Rows("zqaim").Count == 0 && !again.Store().Contains("zqaim", StringComparison.Ordinal), "an empty alias stayed");
        Refuse(() => again.Restore("alias|target.current.set||"), "unreadable");
        // Typed mode: a command with an argument of the solver's own is a row; an unknown command is none.
        PaletteRow? typed = palette.Typed("orbit.switch", "O05");
        Check(typed != null && typed.Command == "orbit.switch" && typed.Argument == "O05" && typed.Label.EndsWith(": O05", StringComparison.Ordinal) && palette.Typed("no.such.command", "x") == null, "typed mode");
        string overlay = File.ReadAllText(Path.Combine(Lab, "app/Chrome/Ui/CommandPalette.cs"));
        foreach (string part in new[] { "MenuPin", "MenuAlias", "MenuBind", "Godot.Key.Menu", "MouseButton.Right" }) Check(overlay.Contains(part, StringComparison.Ordinal), "the row menu lost " + part);

        // Readouts: the value, the expanded line and the lamp follow the history.
        var h = new MoveHistory(); h.Commit("hand", "a", "10:00", "commit a", new[] { "x", "y", "z" }); h.Commit("hand", "b", "10:01", "commit b", new[] { "x" });
        ReadoutFact revision = HistoryReadouts.Revision(h, 1200), position = HistoryReadouts.Position(h);
        Check(revision.Value == "r 1202" && revision.Lamp == LampState.Idle && revision.Detail == "head t-0002, whole" && position.Value == "2/2" && position.Lamp == LampState.Idle && position.Detail == "2 applied of 2 kept, 0 branches", revision.Detail + " / " + position.Detail);
        h.Undo(); h.StepBack(); revision = HistoryReadouts.Revision(h, 1200); position = HistoryReadouts.Position(h);
        Check(revision.Value == "r 1201" && revision.Lamp == LampState.Attention && revision.Detail == "head t-0001, 1 of its 3 moves undone" && position.Value == "1/2" && position.Lamp == LampState.Active, revision.Detail);
        Check(HistoryReadouts.Queue(0).Lamp == LampState.Idle && HistoryReadouts.Queue(2).Lamp == LampState.Active && HistoryReadouts.Queue(2).Value == "2", "the queue readout");
        foreach (ReadoutFact fact in new[] { revision, position, HistoryReadouts.Queue(0), HistoryReadouts.Queue(1) }) Check(fact.LampSays.Length > 8 && fact.Detail.Length > 8, "a lamp without words");

        // Panel states: five besides ready, each with a reason; three take the content's place, two keep it.
        var states = new PanelStates(); int changes = 0; states.Changed += () => changes++; states.Register("history"); states.Register("look");
        Check(PanelStates.Five.Count == 5 && PanelStates.Five.Distinct().Count() == 5 && !PanelStates.Five.Contains(PanelCondition.Ready) && states.Of("history").Condition == PanelCondition.Ready, "the five states");
        Sequence(PanelStates.Five.Where(PanelStates.HidesContent), new[] { PanelCondition.Empty, PanelCondition.Loading, PanelCondition.Error });
        Check(PanelStates.Action(PanelCondition.Error) == "retry" && PanelStates.Action(PanelCondition.Stale) == "refresh" && PanelStates.Action(PanelCondition.Busy) == null, "what a panel offers");
        Refuse(() => states.Set("history", PanelCondition.Error, " "), "says why");
        states.Set("history", PanelCondition.Error, "the file is missing"); states.Set("look", PanelCondition.Stale, "older than r 1202");
        Check(states.Of("history") == new PanelState(PanelCondition.Error, "the file is missing") && states.Of("look").Condition == PanelCondition.Stale && states.Of("other").Condition == PanelCondition.Ready && changes == 2, "set");
        string? asked = null; states.Asked += (id, action) => asked = id + ":" + action; states.Ask("history", "retry");
        states.Set("history", PanelCondition.Ready); states.Clear(); states.Clear();
        Check(asked == "history:retry" && states.Of("look").Condition == PanelCondition.Ready && changes == 4 && states.Known.SequenceEqual(new[] { "history", "look" }), "ask and clear");
        string box = File.ReadAllText(Path.Combine(Lab, "app/Chrome/Ui/PanelStateBox.cs"));
        foreach (PanelCondition c in PanelStates.Five) Check(box.Contains("case PanelCondition." + c + ":", StringComparison.Ordinal), c + " has no glyph");
    }

    // Stage 10 points 8 and 6: one ladder for every field of both tiers, what is away from default, no commit
    // control in the inspector; the preview view's sides and the range a preview touches.
    private static void A5InspectorLadderAndPreviewRange()
    {
        var m = new InspectorModel(); int edits = 0; m.Edited += _ => edits++;
        m.AddFact("sel.kind", "kind", "edge piece");
        m.AddNumber(InspectorModel.View, "v.gap", "gap", 0, 1, .01, .2, .2);
        m.AddNumber(InspectorModel.Preview, "p.cell", "cell", 0, 599, 1, 117, 117, true);
        Check(InspectorModel.Rungs.SequenceEqual(new[] { 100d, 10d, 1d }), "the ladder has three rungs, never finer than the step");
        Check(m.Rung("p.cell") == 2 && m.Beyond == 2, "a field starts on its own step; the dock holds two fields beyond the strip");
        Check(m.Step("p.cell", 1) && m["p.cell"].Value == 118 && m["p.cell"].Provenance == Provenance.Session, "one notch is one step");
        Check(m.Climb("p.cell", -1) && m.Rung("p.cell") == 1 && edits == 1, "changing the rung is not an edit");
        Check(m.Step("p.cell", 2) && m["p.cell"].Value == 138, "a notch is ten steps on the middle rung");
        Check(m.Climb("p.cell", -1) && !m.Climb("p.cell", -1) && m.Step("p.cell", 9) && m["p.cell"].Value == 599, "the top rung stays inside the range");
        Check(m.Climb("p.cell", 5) && m.Rung("p.cell") == 2 && !m.Climb("p.cell", 1), "the ladder ends at the field's step");
        Check(!m.Climb("sel.kind", 1) && !m.Step("sel.kind", 1), "a fact has no ladder");
        Check(m.Enter("v.gap", "= 0.1 + 0.1") == null && m.Away(InspectorModel.View) == 1 && m.Away(InspectorModel.Preview) == 1 && m.Away(InspectorModel.Selection) == 0, "a section counts what is away from default, a binding too");
        Check(m.ResetSection(InspectorModel.View) == 1 && m.Away(InspectorModel.View) == 0 && m["v.gap"].Binding == null && m["p.cell"].Value == 599, "one reset for a section leaves the others");
        string ui = Read("app/Chrome/Ui/InspectorPanels.cs"); int from = ui.IndexOf("class ValueField", StringComparison.Ordinal), to = ui.IndexOf("class FeedbackPanel", StringComparison.Ordinal);
        Check(from > 0 && to > from && !ui.Substring(from, to - from).Contains("preview.commit", StringComparison.Ordinal), "neither inspector tier has a commit control");

        var p = new PreviewModel();
        Check(p.Range() == null && p.Side == PreviewSide.Both, "no preview has no range; the stage starts with both sides");
        var effects = new[] { new OrbitEffect("a", 0, 120), new OrbitEffect("b", 30, 720), new OrbitEffect("c", 1, 2400), new OrbitEffect("d", 600, 600) };
        p.Show(Origin.Solver, "preview x", "own macro", new[] { "m1", "m2" }, PreviewStatus.Ready, null, effects, "Protected orbits untouched.", 0);
        Check(p.Range() == new AffectedRange(3, 4, 631, 3840), "the range counts the orbits and pieces that move");
        int changes = 0; p.Changed += () => changes++;
        p.CycleSide(); Check(p.Side == PreviewSide.Before, "the next side is before"); p.CycleSide(); p.CycleSide();
        Check(p.Side == PreviewSide.Both && changes == 3, "the sides come round: both, before, after");
        p.SetSide(PreviewSide.Both); Check(changes == 3, "the same side changes nothing");
        Check(effects.Select(e => PreviewModel.Lit(e, 24)).SequenceEqual(new[] { 0, 1, 1, 24 }), "an untouched orbit lights nothing, a touched one at least one cell, a whole one all");
        Check(PreviewModel.Lit(new OrbitEffect("e", 719, 720), 24) == 23, "all cells only when every piece moves");
        string panel = Read("app/Chrome/Ui/PreviewCompare.cs");
        Check(!panel.Contains("Safe.Raise(Command", StringComparison.Ordinal) && !panel.Contains("Button", StringComparison.Ordinal), "the comparison is a picture: it has no control and raises no command");
    }
}
