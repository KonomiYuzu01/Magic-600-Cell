using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    private static void D02PortSessionTimerAccumulates()
    {
        var timer = new SessionTimer();
        int changes = 0;
        timer.Changed += () => changes++;
        Check(!timer.Running, "the timer starts running");
        Equal(timer.Elapsed(10), 0.0);
        Equal(SessionTimer.Text(timer.Elapsed(10)), "0:00");
        Equal(timer.Row, "session.timer.start");
        Equal(timer.Stop(10), "the timer is already paused");

        Equal(timer.Start(10), null);
        Check(timer.Running, "the timer did not start");
        Equal(timer.Row, "session.timer.stop");
        Equal(timer.Start(12), "the timer is already running");
        Equal(timer.Elapsed(14), 4.0);
        Equal(timer.Stop(14), null);
        Check(!timer.Running, "the timer did not pause");
        Equal(timer.Row, "session.timer.start");
        Equal(timer.Elapsed(100), 4.0);
        Equal(timer.Start(100), null);
        Equal(timer.Stop(160.5), null);
        Equal(timer.Elapsed(200), 64.5);
        Equal(SessionTimer.Text(timer.Elapsed(200)), "1:04");
        Equal(SessionTimer.Text(3599.9), "59:59");
        Equal(SessionTimer.Text(3600), "1:00:00");
        Equal(SessionTimer.Text(3661), "1:01:01");
        Equal(changes, 4);
    }

    private static void D02PortSessionCellShowsFacts()
    {
        var model = new SessionCellModel();
        var facts = new SessionFacts(2, 4, 1, null, null, null, null, null, null);
        model.Update(facts, recording: true, practice: false);
        Check(!model.Expanded, "the Session cell starts expanded");
        Sequence(model.Facts().Select(fact => fact.Name),
            new[] { "entries", "moves", "completions", "log head", "revision", "state hash", "session", "model", "build" });
        Sequence(model.Facts().Take(3).Select(fact => fact.Value), new[] { "2", "4", "1" });
        Check(model.Facts().Take(3).All(fact => !fact.Waiting), "known counts are waiting");
        Check(model.Facts().Skip(3).All(fact => fact.Waiting && fact.Value == "waits for its row"), "missing facts need their waiting words");
        Sequence(model.Lamps.Select(lamp => lamp.Slot), new[] { "lamp.recording", "lamp.revision", "lamp.practice" });
        Sequence(model.Lamps.Select(lamp => lamp.Lit), new[] { true, false, false });

        facts = facts with { Revision = 7, LogHead = "head", StateHash = "hash", Session = "session", Model = "model", Build = "build" };
        model.Update(facts, recording: true, practice: true);
        Check(!model.Lamps[1].Lit, "the first known revision lit the lamp");
        Sequence(model.Facts().Skip(3).Select(fact => fact.Value), new[] { "head", "7", "hash", "session", "model", "build" });
        Check(model.Facts().All(fact => !fact.Waiting), "the supplied facts are waiting");
        model.Update(facts with { Revision = 8 }, recording: false, practice: true);
        Sequence(model.Lamps.Select(lamp => lamp.Lit), new[] { false, true, true });
        model.Toggle();
        Check(model.Expanded && !model.Lamps[1].Lit, "expanding did not clear the revision lamp");
        model.Update(facts with { Revision = 9 }, recording: false, practice: true);
        model.Toggle();
        Check(!model.Expanded && !model.Lamps[1].Lit, "the expanded cell missed a revision");
    }

    private static void D02PortPracticeMarkFollowsEachView()
    {
        foreach (string view in new[] { "Global", "Local", "Free" })
        {
            Equal(PracticeMark.BorderMarks(false, view).Count, 0);
            ViewMark mark = PracticeMark.BorderMarks(true, view).Single();
            Equal(mark.Glyph, "mark.practice");
            Equal(mark.Words, "practice copy");
            Check(mark.Tooltip.Contains("practice copy"), "the border mark does not explain practice");
        }
    }

    private static void D02PortCompletionLogChecksEveryLabel()
    {
        var demo = new SolvingDemo(Atlas);
        var log = new CompletionLog();
        int changes = 0;
        log.Changed += () => changes++;
        var order = demo.Atlas.StageOrder;
        var first = log.Check(demo.States[0], 42, null, sample: true);
        Sequence(first.Select(entry => entry.Orbit), order.Take(11));
        Sequence(first.Where(entry => entry.Moment), first.Take(1));
        Check(first.All(entry => entry.Sample && entry.AfterEntry == null && entry.Revision == 42), "the first check lost its provenance");
        Equal(log.Find(first[0].Name), first[0]);
        Equal(CompletionLog.Line(first[0]), first[0].Name + " complete · exact check of every label · revision 42");

        int unknown = order[20];
        OrbitResidual residual = demo.State.Residuals[unknown];
        (residual.NotHome, residual.WrongBuffer, residual.Orientation) = (0, 0, 0);
        Equal(log.Check(demo.State, 43, "entry-1", sample: true).Count, 0);
        Equal(log.Find(OrbitAtlas.Name(unknown)), null);
        Equal(changes, 1);

        demo.NextState();
        var second = log.Check(demo.State, demo.State.Revision, "entry-2", sample: false);
        Sequence(second.Select(entry => entry.Orbit), order.Skip(11).SkipLast(1));
        Check(second.All(entry => !entry.Moment && !entry.Sample && entry.AfterEntry == "entry-2" && entry.Revision == demo.State.Revision),
            "the endgame check lost its provenance");
        Equal(changes, 2);

        demo.NextState();
        Equal(log.Check(demo.State, demo.State.Revision, "entry-3", sample: false).Count, 0);
        Sequence(log.Entries, first.Concat(second), "reopening orbits altered recorded completions");
        Equal(changes, 2);
    }
}
