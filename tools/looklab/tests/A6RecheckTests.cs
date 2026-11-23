using System;
using System.Collections.Generic;
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
}
