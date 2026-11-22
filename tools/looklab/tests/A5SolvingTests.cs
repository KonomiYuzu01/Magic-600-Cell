using System;
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
}
