using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Magic600.Chrome;

// Tests for the build findings of the A6 scoped re-check (B-3, B-10, Q5, N-2 to N-10). Each names the finding it pins.
internal static partial class Program
{
    // N-3: a twist of a cell or an axis that does not exist is no witness; it gives its reason and nothing commits.
    private static void A6NonexistentAddressesAreNoWitness()
    {
        Check(SampleCheck.Check(new[] { Twist(599, 5, 1) }).Status == PreviewStatus.Ready && SampleCheck.Parse(Twist(599, 5, -2)) != null, "the last cell and the last axis");
        foreach ((string twist, string named) in new[] { (Twist(600, 1, 1), "cell 600"), (Twist(5, 6, 1), "axis 6"), (Twist(999, 9, 1), "cell 999") })
        {
            PreviewCheck c = SampleCheck.Check(new[] { twist });
            Check(c.Status == PreviewStatus.Refused && c.Reason!.Contains(named + ", which does not exist", StringComparison.Ordinal) && c.Effects.Count == 0 && SampleCheck.Parse(twist) == null,
                twist + " was checked as a witness");
            var history = new MoveHistory(); var preview = new PreviewModel { Head = () => history.Revision };
            preview.Show("preview " + twist, "typed twist", new[] { twist }, c.Status, c.Reason, c.Effects, c.Protection, 0);
            Check(preview.CommitRefusal(5) == c.Reason && preview.Take(5) == null && history.Entries.Count == 0 && history.Revision == 0, twist + " was committed");
            var draft = new OperationDraft(); draft.Edit("core: " + twist);
            Check(draft.StagedCheck() is { Status: PreviewStatus.Refused } d && d.Reason == c.Reason, twist + " passed in a draft");
            var (_, edited) = SampleCheck.Rebuilt("preview " + twist);
            Check(edited.Status == PreviewStatus.Refused && edited.Reason == c.Reason, twist + " was rebuilt as a witness");
        }
        string demo = Read("app/Live/LabDemo.cs"), direct = Body(demo, "public void CommitDirect", "public void Execute");
        int checkAt = direct.IndexOf("SampleCheck.Check(new[] { twist })", StringComparison.Ordinal);
        Check(checkAt > 0 && checkAt < direct.IndexOf("Moves.Commit(\"hand\"", StringComparison.Ordinal), "a Direct twist commits without the check");
        Check(Body(demo, "public void StageTwist", "public void StageMacro").Contains("SampleCheck.Check(moves)", StringComparison.Ordinal), "a typed Direct twist is staged without the check");
    }

    // N-6: one witness shows one collateral however it was staged: as a sample, typed, or replayed from a branch.
    private static void A6OneWitnessOneCollateral()
    {
        PreviewSample typed = PreviewSamples.All.Single(s => s.Source == "typed twist");
        var history = new MoveHistory(); var preview = new PreviewModel { Head = () => history.Revision };
        Preview Staged(IReadOnlyList<string> moves, PreviewCheck c) => preview.Show("preview " + moves[0], "test", moves, c.Status, c.Reason, c.Effects, c.Protection, 0);
        Preview fromSample = Staged(typed.Moves, typed.Check()); AffectedRange? sampleRange = preview.Range();
        string[] same = { Twist(402, 1, 1) }; Preview fromTyped = Staged(same, SampleCheck.Check(same)); AffectedRange? typedRange = preview.Range();
        history.Commit("hand", same[0], "10:00", "commit " + same[0], same); history.Undo(); history.Commit("hand", "other", "10:01", "commit other", new[] { Twist(7, 1, 1) });
        string[] replayed = history.Branches[0].Entries.SelectMany(e => e.Moves).ToArray(); Preview fromReplay = Staged(replayed, SampleCheck.Check(replayed));
        Check(typed.Moves.SequenceEqual(same) && replayed.SequenceEqual(same), "the fixture");
        Check(fromSample.Effects.SequenceEqual(fromTyped.Effects) && fromTyped.Effects.SequenceEqual(fromReplay.Effects) && fromSample.Status == fromReplay.Status &&
            fromSample.Protection == fromReplay.Protection && sampleRange == typedRange && preview.Range() == typedRange, "one witness shows different collateral");
        // Every sample with a word is the sample rule's own result; the sample without a word keeps its refusal.
        foreach (PreviewSample s in PreviewSamples.All)
        {
            PreviewCheck c = s.Check();
            if (s.Moves.Count > 0)
            {
                PreviewCheck rule = SampleCheck.Check(s.Moves);
                Check(c.Status == rule.Status && c.Reason == rule.Reason && c.Effects.SequenceEqual(rule.Effects) && c.Protection == rule.Protection, s.Name + " shows stored counts");
            }
            else Check(c.Status == PreviewStatus.Refused && c.Reason == s.Refusal && c.Effects.Count == 0, s.Name + " lost its refusal");
        }
        Check(PreviewSamples.All.Count(s => s.Check().Status == PreviewStatus.Impossible) == 1 && PreviewSamples.All.Count(s => s.Check().Status == PreviewStatus.Ready) == 3, "the samples' results");
        string demo = Read("app/Live/LabDemo.cs");
        Check(Body(demo, "public void Stage(", "private (int Id").Contains("s.Check()", StringComparison.Ordinal) && !demo.Contains("int[] Moved", StringComparison.Ordinal), "the lab stages samples with stored counts");
    }

    // N-5: Shift+Enter with nothing selected asks for the selection and previews nothing; the pending preview stays.
    private static void A6EmptyLibrarySelectionPreviewsNothing()
    {
        var library = new LibraryModel(); library.SetFilter("no such record");
        Check(library.Shown.Count == 0 && library.Current == null, "the fixture");
        string request = LibraryModel.PreviewRequest(library.Current?.Name);
        Check(request == LibraryModel.PreviewArgument && library.Macro(request.Substring(LibraryModel.PreviewArgument.Length)) == (null, LibraryModel.NoSelection), "an empty selection was previewed");
        // The dispatcher sends every macro: request to the Library, before the sample stager can take it.
        string lab = Read("app/Live/LabFrame.cs"), panel = Read("app/Chrome/Ui/LibraryPanel.cs"), solve = Read("app/Live/LabSolve.cs");
        int library2 = lab.IndexOf("case \"operation.preview\" when argument != null && argument.StartsWith(LabSolve.MacroPreview", StringComparison.Ordinal);
        Check(library2 > 0 && library2 < lab.IndexOf("case var sample when LabDemo.Handles(sample)", StringComparison.Ordinal), "a bare preview request reaches the sample stager");
        Check(panel.Contains("Send(\"operation.preview\", LibraryModel.PreviewRequest(name))", StringComparison.Ordinal) && !panel.Contains("name == null ? null", StringComparison.Ordinal), "the Library sends a bare preview");
        string preview = Body(solve, "public void PreviewMacro", "public void Execute");
        Check(preview.IndexOf("if (why != null)", StringComparison.Ordinal) is var refused && refused > 0 && refused < preview.IndexOf("session.StageMacro", StringComparison.Ordinal), "a refused preview request still stages");
        Check(Read("app/Live/LiveSession.cs").Contains("LibraryModel.NoSelection", StringComparison.Ordinal), "the live check does not press Shift+Enter with nothing selected");
    }

    // N-7: the Library's branch heads, History's layout steps, tabs, entries and switch answers, and the check-mode
    // chip are registry rows; a bound key dispatches them exactly as the pointer does.
    private static void A6SurfaceActionsResolveThroughTheRegistry()
    {
        CommandRegistry registry = LabRegistry();
        string[] rows = { "library.branch.toggle", "layout.roll", "history.show", "history.entry.toggle", "protection.policy.set", "session.branch.select" };
        foreach (string id in rows) Check(registry.Commands.ContainsKey(id), id + " is not a command");
        Check(CommandIcons.Parse(Read("data/design/command-icons.json")).Missing(rows).Count == 0, "a row has neither an icon nor the label-only mark");
        CommandRegistry table = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        Check(table.Commands.ContainsKey("protection.policy.set") && rows.Take(4).All(id => !table.Commands.ContainsKey(id)), "the command table was edited");
        Keymap keys = Keymap.Parse(Read("data/design/keymap.json"), registry);
        Keymap bound = Keymap.Parse("{\"format\":\"magic600-design-keymap\",\"version\":1,\"radials\":{},\"bindings\":[" +
            "{\"key\":\"Alt+T\",\"context\":\"global\",\"command\":\"library.branch.toggle\",\"argument\":\"Templates\",\"label\":\"Fold or open Templates\"}," +
            "{\"key\":\"Alt+S\",\"context\":\"global\",\"command\":\"protection.policy.set\",\"argument\":\"strict\",\"label\":\"Strict check\"}," +
            "{\"key\":\"Alt+L\",\"context\":\"global\",\"command\":\"history.show\",\"argument\":\"layout\",\"label\":\"Show the layout history\"}]}", registry, keys);
        // The pointer sends the head's title; the bound key sends the same command and argument; the model acts once.
        var byPointer = new LibraryModel(); var byKey = new LibraryModel();
        Binding toggle = bound.Resolve("Alt+T", new[] { "global" })!;
        Check(byPointer.Toggle("Templates") == null && toggle.Command == "library.branch.toggle" && byKey.Toggle(toggle.Argument) == null && byKey.IsOpen("Templates") &&
            byKey.Shown.SequenceEqual(byPointer.Shown) && byKey.Toggle("nowhere") != null, "a bound key folds the Library differently from its head");
        // The chip sends the other mode by name; a key bound to the row sets the same mode.
        var chip = new OperationDraft(); var key = new OperationDraft(); Binding policy = bound.Resolve("Alt+S", new[] { "global" })!;
        Check(chip.SetPolicy(chip.Mode == CheckMode.Net ? "strict" : "net") == null && key.SetPolicy(policy.Argument) == null && chip.Mode == CheckMode.Strict && key.Mode == chip.Mode &&
            key.SetPolicy(null) == null && key.Mode == CheckMode.Net && key.SetPolicy("safe") != null, "the protection policy row");
        Check(bound.Resolve("Alt+L", new[] { "global" }) is { Command: "history.show", Argument: "layout" }, "History's view is not bindable");
        // The surfaces send the rows, and the lab carries them out with no widget-only path left.
        string library = Read("app/Chrome/Ui/LibraryPanel.cs"), draft = Read("app/Chrome/Ui/DraftLine.cs"), history = Read("app/Chrome/Ui/SessionPanels.cs"), demo = Read("app/Live/LabDemo.cs"), solve = Read("app/Live/LabSolve.cs");
        Check(library.Contains("Send(\"library.branch.toggle\", branch)", StringComparison.Ordinal) && !library.Contains("Library.Fold(", StringComparison.Ordinal), "the Library's head folds by itself");
        Check(draft.Contains("Safe.Raise(Command, \"protection.policy.set\"", StringComparison.Ordinal) && !draft.Contains("Draft.SetMode(", StringComparison.Ordinal), "the check-mode chip sets the mode by itself");
        Check(history.Contains("Send(\"history.show\"", StringComparison.Ordinal) && history.Contains("Send(\"history.entry.toggle\"", StringComparison.Ordinal) &&
            history.Contains("Send(\"session.branch.select\", \"cancel\")", StringComparison.Ordinal) && history.Contains("Send(\"layout.roll\"", StringComparison.Ordinal), "a History action is widget-only");
        string handles = Body(demo, "public static bool Handles", ";");
        foreach (string id in new[] { "layout.roll", "history.show", "history.entry.toggle" }) Check(handles.Contains("\"" + id + "\"", StringComparison.Ordinal) && demo.Contains("case \"" + id + "\":", StringComparison.Ordinal), id + " is not handled by the lab");
        Check(demo.Contains("History.Command += run;", StringComparison.Ordinal) && !demo.Contains("command == \"layout.roll\"", StringComparison.Ordinal) && demo.Contains("parts is [\"cancel\"]", StringComparison.Ordinal), "History's commands are intercepted");
        Check(solve.Contains("\"library.branch.toggle\" or \"protection.policy.set\"", StringComparison.Ordinal) && solve.Contains("Library.Toggle(argument)", StringComparison.Ordinal) && solve.Contains("Draft.SetPolicy(argument)", StringComparison.Ordinal),
            "the Solve host does not carry out the rows");
    }

    // N-8: the effect signature has 35 slots in the rail's order, and an unknown effect is never a measured zero.
    private static void A6SignatureHas35SlotsInRailOrder()
    {
        foreach (bool byId in new[] { false, true })
        {
            IReadOnlyList<int> order = OrbitRailModel.Order(Atlas, byId);
            EffectSignature known = EffectSignature.Of(new Dictionary<int, int> { [34] = 3 }, order, false), zero = EffectSignature.Of(new Dictionary<int, int> { [34] = 0 }, order, false);
            Check(known.Slots.Count == OrbitAtlas.Count && known.Slots.Select(x => x.Orbit).SequenceEqual(order) && known.Slots[order.ToList().IndexOf(34)].Mark == SignatureMark.Moved &&
                known.Slots.Count(x => x.Mark == SignatureMark.Unknown) == OrbitAtlas.Count - 1, "O34's known effect is not in its rail slot, or another slot is not unknown");
            Check(zero.Mark(34) == SignatureMark.Untouched && zero.Slots.Where(x => x.Orbit != 34).All(x => x.Mark == SignatureMark.Unknown), "a measured zero reads as unknown, or unknown as zero");
            Check(LibraryModel.SignatureText(known).Contains("unverified: moves O34; untouched none; 34 orbits unknown", StringComparison.Ordinal) &&
                LibraryModel.SignatureText(zero).Contains("moves no orbit; untouched O34", StringComparison.Ordinal), LibraryModel.SignatureText(known));
            foreach (LibraryRecord m in new LibraryModel().Records.Where(r => r.IsMacro))
            {
                EffectSignature s = LibraryModel.Signature(m.Word, order); IReadOnlyList<OrbitEffect> effects = SampleCheck.Effects(m.Word)!;
                Check(s.Slots.Select(x => x.Orbit).SequenceEqual(order) && s.Slots.Count(x => x.Mark == SignatureMark.Unknown) == OrbitAtlas.Count - SampleCheck.Orbits.Count &&
                    SampleCheck.RailOrbits.Select((o, i) => s.Mark(o) == (effects[i].Moved > 0 ? SignatureMark.Moved : SignatureMark.Untouched)).All(x => x), m.Name + "'s signature");
            }
        }
        Check(SampleCheck.RailOrbits.Distinct().Count() == SampleCheck.Orbits.Count && SampleCheck.RailOrbits.All(o => o >= 0 && o < OrbitAtlas.Count), "the sample orbits' rail places");
        Refuse(() => EffectSignature.Of(new Dictionary<int, int>(), new[] { 1, 2, 3 }, false), "35 orbits once");
        string panel = Read("app/Chrome/Ui/LibraryPanel.cs");
        Check(panel.Contains("case SignatureMark.Moved:", StringComparison.Ordinal) && panel.Contains("case SignatureMark.Untouched:", StringComparison.Ordinal) && panel.Contains("RailOrder?.Invoke()", StringComparison.Ordinal) &&
            Read("app/Live/LabSolve.cs").Contains("LibraryView.RailOrder = () => OrbitRailModel.Order(Demo.Atlas, Rail.ByOrbitId);", StringComparison.Ordinal), "the Library does not draw the signature in the rail's order");
    }

    // N-9: a Template binds its roles to the strip, joins a phase and is previewed; another Target is another word,
    // checked again as a fresh preview.
    private static void A6TemplateBindsItsRoles()
    {
        var library = new LibraryModel(); var demo = new SolvingDemo(Atlas); SolvingState s = demo.State;
        LibraryRecord template = library.Records.Single(r => r.Kind == LibraryKind.Template);
        Check(template.IsTemplate && !template.IsMacro && template.Roles.SequenceEqual(LibraryModel.RoleNames) && s.Target != null && s.BufferA != null && s.BufferB != null && s.Problems().Count == 0, "the fixture");
        Check(library.Macro(template.Name).Why!.Contains("roles bind to the strip", StringComparison.Ordinal), "a Template bound with no strip");
        var (bound, why) = library.Macro(template.Name, s); LibraryRecord first = bound!;
        Check(why == null && first!.IsMacro && first.Word.Count == template.Word.Count && first.Name.Contains("Target " + OrbitAtlas.Position(s.Target!.Value), StringComparison.Ordinal) &&
            first.Word[0] == Twist(LibraryModel.SampleCell(s.Orbit, s.Target.Value), 1, 1) && first.Word[1] == Twist(LibraryModel.SampleCell(s.Orbit, s.BufferA!.Value), 2, -1) &&
            SampleCheck.Check(first.Word).Status == PreviewStatus.Ready, "the Template did not bind to the strip's roles");
        var draft = new OperationDraft(); Check(library.Insert(draft, template.Name, s) == null && draft.Twists(1).SequenceEqual(first.Word), "a bound Template did not join Core");
        var history = new MoveHistory(); var preview = new PreviewModel { Head = () => history.Revision };
        PreviewCheck c1 = SampleCheck.Check(first.Word); Preview p1 = preview.Show("preview " + first.Name, "library", first.Word, c1.Status, c1.Reason, c1.Effects, c1.Protection, 0);
        int other = s.Unfinished().First(p => p != s.Target && p != s.BufferA && p != s.BufferB);
        Check(demo.SetTarget(OrbitAtlas.Position(other)) == null && s.Target == other && s.Problems().Count == 0, "the second Target");
        var (second, why2) = library.Macro(template.Name, s);
        PreviewCheck c2 = SampleCheck.Check(second!.Word); Preview p2 = preview.Show("preview " + second.Name, "library", second.Word, c2.Status, c2.Reason, c2.Effects, c2.Protection, 1);
        Check(why2 == null && !second.Word.SequenceEqual(first.Word) && second.Name != first.Name && p2.Id != p1.Id && p2.Status == PreviewStatus.Ready && p2.Effects.SequenceEqual(c2.Effects) &&
            p2.Statement.Contains(OrbitAtlas.Position(other), StringComparison.Ordinal), "another Target reused the first binding");
        s.BufferB = null; Check(library.Macro(template.Name, s).Why!.Contains("Buffer B", StringComparison.Ordinal), "an unset role was bound");
        s.BufferB = s.Target; Check(library.Macro(template.Name, s).Why!.Contains("invalid tuple", StringComparison.Ordinal), "an invalid tuple was bound");
        string solve = Read("app/Live/LabSolve.cs");
        Check(solve.Contains("Library.Insert(Draft, argument, Demo.State)", StringComparison.Ordinal) && solve.Contains("Library.Macro(name, Demo.State)", StringComparison.Ordinal), "the lab binds no roles");
    }
}
