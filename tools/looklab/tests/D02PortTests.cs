using System;
using System.IO;
using System.Linq;
using Magic600.Chrome;

// ADR-D02 build, track C: Look Lab's own wiring moved into the shared chrome library, which the product shell stages
// (owner, 26 November 2026: the product is the only place where UI is built, shown and accepted).
internal static partial class Program
{
    // The error lamp's log (item 1): failures in order, a lamp shown only the new ones, a cleared log shown again from
    // its start, the cap at Safe.Kept, one line per failure, and a log file that takes each failure.
    private static void D02PortErrorLogShowsEachFailureOnce()
    {
        var log = new ErrorLog();
        log.Add("first\n  at a stack line"); log.Add("second");
        ErrorsSince seen = log.Since(0, 0);
        Check(seen.Fresh.Count == 2 && seen.Shown == 2 && !seen.Restarted && ErrorLog.Line(seen.Fresh[0]) == "first", "a fresh lamp saw " + string.Join("|", seen.Fresh));
        log.Add("third");
        ErrorsSince next = log.Since(seen.Cleared, seen.Shown);
        Check(next.Fresh.SequenceEqual(new[] { "third" }) && next.Shown == 3, "the lamp saw again " + string.Join("|", next.Fresh));
        Check(log.Since(next.Cleared, next.Shown).Fresh.Count == 0, "a lamp that showed every failure saw one again");
        log.Clear(); log.Add("after the clear");
        ErrorsSince again = log.Since(next.Cleared, next.Shown);
        Check(again.Restarted && again.Fresh.SequenceEqual(new[] { "after the clear" }) && again.Shown == 1, "a cleared log was not shown from its start");
        for (int i = 0; i < Safe.Kept + 5; i++) log.Add("flood " + i);
        Check(log.Count == Safe.Kept, "the log kept " + log.Count + " failures");
        using var folder = new TestFolder();
        string file = Path.Combine(folder.Path, "deeper", "errors.log");
        var written = new ErrorLog(); written.Open(file);
        Check(written.Writing && File.Exists(file) && File.ReadAllText(file).Length == 0, "the log file was not started empty");
        written.Add("one"); written.Add("two");
        string[] lines = File.ReadAllLines(file);
        Check(lines.Length == 2 && lines[0].EndsWith(" one", StringComparison.Ordinal) && lines[1].EndsWith(" two", StringComparison.Ordinal), "the log file holds " + string.Join("|", lines));
        // A path that cannot be a file is dropped; the failures still show.
        var refused = new ErrorLog(); refused.Open(folder.Path); refused.Add("still shown");
        Check(!refused.Writing && refused.Count == 1, "a folder was taken as the log file");
    }

    // A listener the chrome caught reaches the log through Safe.Failed, as ChromeErrors.Install subscribes it, and
    // the other listeners still run.
    private static void D02PortCaughtListenerReachesTheLog()
    {
        var log = new ErrorLog();
        Safe.Failed += log.Add;
        try
        {
            bool second = false;
            Action handlers = () => throw new InvalidOperationException("a panel could not repaint");
            handlers += () => second = true;
            Safe.Raise(handlers);
            Check(second, "a failing listener stopped the next one");
            Check(log.Count == 1 && ErrorLog.Line(log.All[0]) == "InvalidOperationException: a panel could not repaint", "the log holds " + string.Join("|", log.All));
        }
        finally { Safe.Failed -= log.Add; Safe.Reset(); }
    }
}
