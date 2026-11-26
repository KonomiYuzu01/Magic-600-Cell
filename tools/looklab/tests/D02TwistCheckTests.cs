using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

// ADR-D02 build step B10: the kept twist checks and the Q3.b test (ADR-D02.35, .36), as the owner reads J-7 and N-1.
internal static partial class Program
{
    // On wj-I_a at a named cap (wj-ia-300, cap 150), a seat's twelve twists released faster than the checks complete.
    private static void D02Q3bTwelveTwists()
    {
        var checks = TwistCheck.FromRecord(Read("data/samples/jumble/wj-ia-300/cap-150.json"));
        Check(checks.Count == 12, "the cap record does not hold twelve release checks");
        var kept = new KeptChecks(new ReleaseRecord(), 150, 300);
        var jobs = new SampleJobs(3);
        // Releases at these ticks; a check takes three ticks, so most are superseded and a few complete.
        long[] at = { 0, 1, 2, 6, 7, 12, 13, 14, 15, 20, 21, 22 };
        var released = new List<long>();
        int item = 0, cancels = 0;
        for (long tick = 0; tick <= 30; tick++)
        {
            foreach (var (release, done) in jobs.Tick()) kept.Complete(release, checks[done]);
            while (item < at.Length && at[item] == tick)
            {
                long pendingBefore = kept.Pending ?? -1;
                var screenBefore = kept.Screen();
                long release = kept.Release(item + 1);
                jobs.Request(release, item + 1);
                released.Add(release);
                // (2) The folded row of a superseded release appears in the frame of the release that superseded it.
                if (pendingBefore > 0)
                {
                    var folded = kept.Entries.OfType<FoldedRow>().LastOrDefault();
                    Check(folded != null && folded.Releases.Contains(pendingBefore), "release " + pendingBefore + " was not folded when " + release + " superseded it");
                    Check(!kept.Screen().SequenceEqual(screenBefore), "the superseding release left the screen as it was");
                }
                // (4) Cancelling a job that is no longer the latest request leaves the screen identical (N-1).
                foreach (long older in released.Where(r => r != kept.Pending))
                {
                    var before = kept.Screen();
                    Check(!kept.Cancel(older), "cancelling release " + older + " was taken");
                    Check(kept.Screen().SequenceEqual(before), "cancelling release " + older + " changed the screen");
                    cancels++;
                }
                item++;
            }
        }
        Check(cancels > 30, "the run cancelled too few older releases: " + cancels);
        var rows = kept.Entries.OfType<CheckRow>().ToArray();
        var folds = kept.Entries.OfType<FoldedRow>().ToArray();
        Check(rows.Length >= 3 && folds.Length >= 3, "the run did not mix completed and folded rows: " + rows.Length + " and " + folds.Length);
        // (1) The completed rows appear in increasing release order, and the whole list is in release order.
        Check(rows.Select(r => r.Release).SequenceEqual(rows.Select(r => r.Release).OrderBy(r => r)), "the completed rows are not in release order");
        Check(kept.Entries.Select(e => e.Release).SequenceEqual(kept.Entries.Select(e => e.Release).OrderBy(r => r)), "the list is not in release order");
        Check(released.All(r => rows.Any(c => c.Release == r) ^ folds.Any(f => f.Releases.Contains(r))), "a release is neither completed nor folded, or both");
        // Try numbers count every release at the cap and revision, the superseded ones included.
        Check(rows.All(r => r.Try == released.IndexOf(r.Release) + 1), "a try number does not count the superseded releases");
        // (3) A folded row shows only "not checked:" and its twists' glyphs: no numeral of any field and no pending mark.
        foreach (FoldedRow f in folds)
        {
            string line = KeptChecks.Line(f);
            Check(line == "not checked: " + string.Join(" ", f.Items.Select(i => KeptChecks.Glyph(150, i))), "a folded row reads " + line);
            Check(!line.Contains("pending", StringComparison.Ordinal) && !line.Contains('|'), "a folded row carries a field or a pending mark");
        }
        var foldedFields = typeof(FoldedRow).GetProperties().Select(p => p.Name).Where(n => n != "EqualityContract").OrderBy(n => n).ToArray();
        Check(foldedFields.SequenceEqual(new[] { "Cap", "Items", "Release", "Releases" }), "a folded row has a field beyond its releases and twists: " + string.Join(", ", foldedFields));
        // J-7: cancelling the latest request withdraws it and returns the list to its state before that release.
        long last = kept.Release(1);
        var withPending = kept.Screen();
        Check(kept.Cancel(last) && kept.Pending == null && kept.Screen().SequenceEqual(withPending), "cancelling the latest did not return to the previous state");
        Check(!kept.Complete(last, checks[1]), "a cancelled request's late result was taken");
        // The completed row's cells, under try | twist | N_c | floor | ΔN | Δn_h | Δu.
        CheckRow first = rows[0];
        var cells = KeptChecks.Cells(first);
        Check(cells.Count == KeptChecks.Columns.Count && cells[1] == "150·" + first.Item && cells[3] == "44", "a completed row's cells moved: " + string.Join(" | ", cells));
        Check(KeptChecks.Cells(new CheckRow(1, 1, 150, 12, checks[12]))[5] == "n0 -1410 n2 +1410", "Δn_h does not show its changed heights");
    }

    // C2-1: the job layer withdraws a superseded request; a withdrawn job never completes, the others still do.
    private static void D02CancelledJobNeverCompletes()
    {
        var jobs = new SampleJobs(3);
        jobs.Request(1, 11);
        jobs.Request(2, 12);
        Check(jobs.Cancel(1) && !jobs.Cancel(1) && !jobs.Cancel(7), "a withdrawn or unknown job was cancelled twice");
        var done = new System.Collections.Generic.List<(long Release, int Item)>();
        for (int i = 0; i < 4; i++) done.AddRange(jobs.Tick());
        Check(done.Count == 1 && done[0] == (2, 12), "the jobs that completed are " + string.Join(", ", done));
    }
}
