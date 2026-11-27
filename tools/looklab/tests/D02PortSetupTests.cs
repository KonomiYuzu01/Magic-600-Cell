using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    // A demo state with a Target and the buffers as given (ADR-D01.68's tests build their own, so the sample rule's
    // rows are known: with both buffers set, a place p gives no row when p % 5 == 0 and rows otherwise).
    private static SolvingState SetupState(int target, bool buffers, string id = "setup-test") =>
        new(id, "setup test", Atlas, Atlas.StageOrder[0], 3) { Target = target, BufferA = buffers ? target + 1 : null, BufferB = buffers ? target + 2 : null };

    // ADR-D01.68: the tool ships only with its BoundaryClass and its PrincipleRecord; without either it is refused,
    // and the slot names what is missing.
    private static void D02PortSetupDeclaration()
    {
        ToolDeclaration shipped = SetupSearch.Declared;
        Check(shipped.Refusal() == null && shipped.Class == BoundaryClass.SetupSearchNamedTarget, "the setup search does not ship with its declaration");
        Equal(BoundaryClasses.Word(BoundaryClass.SetupSearchNamedTarget), "setup_search_named_target");
        Check(shipped.Line().Contains("setup_search_named_target", StringComparison.Ordinal) && shipped.Line().Contains("sample", StringComparison.Ordinal), shipped.Line());
        Check(SetupSearch.SamplePrinciple.Missing().Count == 0, "the sample PrincipleRecord lacks a part");
        string? noClass = new ToolDeclaration("Setup search", null, SetupSearch.SamplePrinciple).Refusal();
        string? noRecord = new ToolDeclaration("Setup search", BoundaryClass.SetupSearchNamedTarget, null).Refusal();
        string? neither = new ToolDeclaration("Setup search", null, null).Refusal();
        string? part = new ToolDeclaration("Setup search", BoundaryClass.SetupSearchNamedTarget, SetupSearch.SamplePrinciple with { Justification = " " }).Refusal();
        Check(noClass != null && noClass.Contains("BoundaryClass", StringComparison.Ordinal) && !noClass.Contains("PrincipleRecord", StringComparison.Ordinal), noClass ?? "no refusal without a class");
        Check(noRecord != null && noRecord.Contains("PrincipleRecord", StringComparison.Ordinal) && !noRecord.Contains("BoundaryClass", StringComparison.Ordinal), noRecord ?? "no refusal without a record");
        Check(neither != null && neither.Contains("BoundaryClass", StringComparison.Ordinal) && neither.Contains("PrincipleRecord", StringComparison.Ordinal), neither ?? "no refusal without either");
        Check(part != null && part.Contains("justification", StringComparison.Ordinal), part ?? "a record without its justification ships");
        var bare = new SetupSearch(new ToolDeclaration("Setup search", null, null));
        string? why = bare.Run(SetupState(1, true), null);
        Check(why != null && bare.Slot.State == SlotState.Refused && bare.Current == null && bare.Slot.Reason == why, "an undeclared setup search ran");
    }

    // ADR-D01.68: the search runs only for a target the solver names; an unset Target, an unknown argument and a
    // Wanted piece already home are refused, and a refused slot has no row to pick.
    private static void D02PortSetupRefusals()
    {
        var demo = new SolvingDemo(Atlas);
        Check(demo.Show("fresh") && demo.State.Target == null, "the fresh demo state names a Target");
        var search = new SetupSearch();
        string? why = search.Run(demo.State, null);
        Check(why != null && why.Contains("name a Target first", StringComparison.Ordinal) && search.Slot.State == SlotState.Refused, why ?? "ran with no Target");
        Check(search.Run(SetupState(1, true), "elsewhere") is { } bad && bad.Contains("wanted", StringComparison.Ordinal), "an unknown argument was taken");
        var draft = new OperationDraft(); draft.Append("twist(cell 3, axis 1, +1)");
        Check(search.Fill("1", draft, UserEvent.Click("Fill")) != null && draft.Twists(0).Count == 0, "a refused slot filled the draft");
        Check(new SetupSearch().Fill("1", draft, UserEvent.Click("Fill")) is { } notRun && notRun.Contains("run the setup search first", StringComparison.Ordinal), "a slot never run gave a row");
        Check(new SetupSearch().Word().Text == "not run", "the slot's word before a run");
    }

    // ADR-D01.68 with revision 6's J-7: a completed result carries its coverage (family, guarded alphabet, bound, all
    // tried); the alphabet excludes the caps that move the buffers; rows come in length order, then menu order, with
    // their lengths; a length is called minimal only within the guarded alphabet; an empty answer names the family.
    private static void D02PortSetupCoverageAndOrder()
    {
        foreach (bool buffers in new[] { true, false })
            for (int place = 0; place < 40; place++)
            {
                SolvingState s = SetupState(place, buffers);
                var search = new SetupSearch();
                Check(search.Run(s, null) == null && search.Current is { } r && search.Slot.State == SlotState.Ready, "the search did not complete at " + place);
                SetupResult result = search.Current!;
                Coverage? c = search.Slot.Result!.Coverage;
                int caps = buffers ? 3 : 6, letters = 2 * caps, total = letters + letters * (letters - 2) + letters * (letters - 2) * (letters - 2);
                Check(c != null && c.Tried == c.Total && c.Total == total && result.Coverage.Total == total, "the coverage at " + place + " is not every word of the family");
                Check(c!.Cap == OrbitAtlas.Position(place) && c.Revision == s.Revision && c.TieRule == SetupSearch.TieRule && c.Menu.Contains("length 1 to 3", StringComparison.Ordinal), "the coverage names another target or family");
                Check(result.Coverage.Alphabet.Count == caps && result.Coverage.Excluded.Count == (buffers ? 3 : 0), "the guarded alphabet at " + place);
                Check(buffers == result.Coverage.Excluded.Any(e => e.Moves == "Buffer A") && buffers == result.Coverage.Excluded.Any(e => e.Moves == "Buffer B"), "an excluded cap names no buffer");
                Check(result.Rows.All(row => row.Letters.All(l => result.Coverage.Alphabet.Contains(l.Cell) && !result.Coverage.Excluded.Any(e => e.Cell == l.Cell))), "a row turns a cap that moves a buffer");
                // The neutral order: the rows keep the enumeration's order, which is length and then menu order.
                List<string> order = SetupSearch.Words(result.Rows.SelectMany(row => row.Letters).Distinct().OrderBy(l => l.Cap).ThenBy(l => -l.Turns).ToArray(), 3).Select(w => string.Join(" ", w.Select(l => l.Short))).ToList();
                int[] at = result.Rows.Select(row => order.IndexOf(row.Short)).ToArray();
                Check(at.All(i => i >= 0) && at.Zip(at.Skip(1), (a, b) => a < b).All(x => x), "the rows are not in length and menu order at " + place);
                Check(result.Rows.Select((row, i) => row.Number == i + 1).All(x => x) && search.RowLines().Zip(result.Rows, (line, row) => line.Contains("len " + row.Length, StringComparison.Ordinal)).All(x => x), "a row does not show its length");
                if (result.Rows.Count > 0)
                {
                    Check(result.Minimal == result.Rows.Min(row => row.Length) && search.MinimalLine() is { } m && m.Contains("minimal within the guarded alphabet", StringComparison.Ordinal) && search.EmptyLine() == null, "the minimal line at " + place);
                    Check(search.Word().Text.EndsWith(result.Rows.Count == 1 ? " row" : " rows", StringComparison.Ordinal), search.Word().Text);
                }
                else
                {
                    Check(result.Minimal == null && search.MinimalLine() == null && search.EmptyLine() is { } e && e.Contains(result.Coverage.Family, StringComparison.Ordinal) && search.Word().Text == "no setup", "the empty line at " + place);
                    Check(search.Slot.Result!.IsEmpty, "an empty answer shows rows");
                }
                Check(search.CoverageLine() is { } line && line.Contains(total + " of " + total + " words tried", StringComparison.Ordinal) && line.Contains(SolvingDemo.Mark, StringComparison.Ordinal), "the coverage line at " + place);
            }
        // The sample rule (demo data) gives an empty answer at p0005 with both buffers set, and rows at p0001.
        var empty = new SetupSearch(); empty.Run(SetupState(5, true), null);
        var some = new SetupSearch(); some.Run(SetupState(1, true), null);
        Check(empty.Current is { Rows.Count: 0 } && some.Current is { Rows.Count: > 0 }, "the sample rule's empty and full places moved");
    }

    // ADR-D01.68: a row fills Prepare and Cleanup around the macro in Core as one change, never Core and never a whole
    // placement; the solver's own Prepare or Cleanup twists are never replaced; the row alone stages as the preview;
    // every act names the input that asked for it (the boundary ledger).
    private static void D02PortSetupFill()
    {
        var search = new SetupSearch(); search.Run(SetupState(1, true), null);
        SetupResult result = search.Current!;
        Check(result.Rows.Count >= 2, "the sample at p0001 gives fewer than two rows");
        var draft = new OperationDraft(); int changes = 0; draft.Changed += () => changes++;
        string? empty = search.Fill("1", draft, UserEvent.Click("Fill"));
        Check(empty != null && empty.Contains("place your macro in Core first", StringComparison.Ordinal) && empty.Contains("never a whole placement", StringComparison.Ordinal) && draft.Count == 0, "a row filled an empty Core");
        draft.SelectPhase("Core"); draft.Append("twist(cell 3, axis 1, +1)"); draft.Append("twist(cell 9, axis 2, -1)");
        string[] core = draft.Twists(1).ToArray(); changes = 0;
        Check(search.Fill("1", draft, UserEvent.Click("Fill")) == null, "row 1 did not fill");
        SetupRow first = result.Rows[0];
        Check(changes == 1 && draft.Twists(0).SequenceEqual(first.Word) && draft.Twists(2).SequenceEqual(first.Inverse) && draft.Twists(1).SequenceEqual(core), "the fill is not Prepare and Cleanup around Core, as one change");
        Check(first.Inverse.Count == first.Length && first.Inverse.First() == (first.Letters[^1] with { Turns = -first.Letters[^1].Turns }).Twist, "the Cleanup is not the setup undone");
        Check(search.Fill("2", draft, UserEvent.Click("Fill")) == null && draft.Twists(0).SequenceEqual(result.Rows[1].Word) && draft.Twists(1).SequenceEqual(core), "a second row did not replace the first row's fill");
        Check(search.Fill("99", draft, UserEvent.Click("Fill")) is { } none && none.Contains("no row 99", StringComparison.Ordinal), "a missing row was taken");
        draft.Edit("prepare: twist(cell 11, axis 0, +1)");
        Check(search.Fill("1", draft, UserEvent.Click("Fill")) is { } own && own.Contains("your own twists", StringComparison.Ordinal) && draft.Twists(0).Single() == "twist(cell 11, axis 0, +1)", "a row replaced the solver's own Prepare");
        draft.Edit("prepare: "); draft.Edit("cleanup: ");
        draft.SetRoute("direct");
        Check(search.Fill("1", draft, UserEvent.Click("Fill")) is { } direct && direct.Contains("Direct", StringComparison.Ordinal), "a row filled a draft in Direct");
        draft.SetRoute("draft");
        Check(search.Alone("1", UserEvent.Click("Alone"), out SetupRow? alone) == null && alone == first && draft.Twists(0).Count == 0, "the row alone touched the draft");
        Check(search.Ledger.Count == 0 && search.Ledger.Asked == 3, "a fill or a preview did not name its input");
        search.Fill("1", draft, null);
        Check(search.Ledger.Count == 1 && search.Ledger.Unasked[0].Kind == "apply", "an unasked fill was not counted");
    }

    // Revision 6, J-7: the rows answer one revision. A change of the Target or a buffer makes them stale, so no row can
    // be used; back at the revision they were made for, they show again. Naming Wanted's present place is one key.
    private static void D02PortSetupStale()
    {
        var demo = new SolvingDemo(Atlas); var search = new SetupSearch();
        demo.Changed += () => search.Follow(demo.State);
        int target = demo.State.Target!.Value;
        Check(search.Run(demo.State, null) == null && search.Slot.State == SlotState.Ready, "the mid state's search did not complete");
        Check(demo.SetTarget(null) == null && demo.State.Target != target, "the Target did not step");
        Check(search.Slot.State == SlotState.Stale && search.Current == null && search.RowLines().Count == 0, "the rows did not go stale with the Target");
        var draft = new OperationDraft(); draft.Append("twist(cell 3, axis 1, +1)");
        Check(search.Fill("1", draft, UserEvent.Click("Fill")) is { } stale && stale.Contains("stale", StringComparison.Ordinal), "a stale row was used");
        Check(demo.SetTarget(OrbitAtlas.Position(target)) == null && search.Slot.State == SlotState.Ready && search.Current != null, "back at its revision the result did not show again");
        Check(demo.AssignBuffer("B") == null && search.Slot.State == SlotState.Stale, "the rows did not go stale with a buffer");
        SolvingState s = demo.State;
        string? wanted = search.Run(s, SetupSearch.WantedArgument);
        if (s.WantedAt is int at && at != s.Target)
            Check(wanted == null && search.Current is { Naming: SetupNaming.Wanted } r && r.Place == at && search.Slot.Result!.Coverage!.Cap == OrbitAtlas.Position(at) && search.NamedLine()!.Contains("Wanted's present place", StringComparison.Ordinal), "Wanted's place was not named");
        else
            Check(wanted != null && search.Slot.State == SlotState.Refused, "a Wanted with no other place was searched");
    }

    // ADR-D01.68 mounted: the slot sits under the Target cell of the strip, the sample's solving surface carries the
    // rows and the row alone, and the shell registers the draft rows and lets them run with D02 on.
    private static void D02PortSetupMounted()
    {
        string strip = Read("app/Chrome/Ui/SolvingSurface.cs");
        int cell = strip.IndexOf("cells[cell.Slot] = view; box.AddChild(view);", StringComparison.Ordinal), slot = strip.IndexOf("if (cell.Slot == \"target\" && Demo.Setup is { } setup)", StringComparison.Ordinal);
        Check(cell >= 0 && slot > cell && strip.Contains("new SetupSlotView(setup, tokens)", StringComparison.Ordinal), "the setup slot is not under the Target cell");
        string solve = Read("app/Live/LabSolve.cs");
        Check(solve.Contains("Demo.Setup = Setup;", StringComparison.Ordinal) && solve.Contains("Demo.Changed += () => Setup.Follow(Demo.State);", StringComparison.Ordinal) &&
            solve.Contains("SetupSearch.Search or SetupSearch.Use", StringComparison.Ordinal), "the sample's solving surface does not carry the setup slot");
        Check(Read("app/Live/SampleMount.cs").Contains("Solve.PreviewSetup(argument.Substring(SetupSearch.PreviewArgument.Length))", StringComparison.Ordinal), "the shell's sample does not preview a row alone");
        string shell = System.IO.File.ReadAllText(System.IO.Path.Combine(Root, "renderer/shell/project/ShellChrome.cs"));
        int run = shell.IndexOf("_ when sample != null && SetupSearch.Ids.Contains(command)", StringComparison.Ordinal), drafts = shell.IndexOf("_ when ShellJumble.On && ShellJumble.Drafts.Contains(command)", StringComparison.Ordinal);
        Check(run >= 0 && run < drafts && shell.Contains("SetupRows.AddDrafts(registry,", StringComparison.Ordinal), "the shell does not register or run the setup rows");
        CommandRegistry registry = LabRegistry(); string rows = Read("data/design/command-rows-d02.json");
        IReadOnlyList<string> added = SetupRows.AddDrafts(registry, rows);
        Check(added.SequenceEqual(SetupSearch.Ids) && added.All(registry.Commands.ContainsKey) && registry.Commands[SetupSearch.Search].Kind == "query" && registry.Commands[SetupSearch.Use].Kind != "commit", "the setup draft rows");
        int before = registry.Order.Count; SetupRows.AddDrafts(registry, rows);
        Check(registry.Order.Count == before, "registering the setup rows twice repeats them");
    }
}
