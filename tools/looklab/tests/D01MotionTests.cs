using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    private static MotionTable D01Motions() => MotionTable.Parse(Read("data/design/motion.json"), Design);

    private static void D01MotionUsesMatchTable()
    {
        MotionTable motions = D01Motions();
        foreach (var use in MotionUses.All)
        {
            Check(use.Trigger is "state" or "view", use.What + " has no state or view trigger");
            var row = motions[use.RowId];
            if (use.Kind == MotionKind.Animated)
                Check(!row.Off && motions.Seconds(use.RowId, Design, false) > 0, use.What + " animates an off row");
            Equal(motions.Seconds(use.RowId, Design, true), 0d, use.What + " under reduced motion");
        }
        var result = MotionUses.All.Single(use => use.What == "Check and repair results");
        Equal(result.RowId, "slot.fill");
        Equal(result.Kind, MotionKind.Static);
        Equal(motions.Seconds(result.RowId, Design, false), 0d);
        Refuse(() => MotionTable.Parse(Read("data/design/motion.json").Replace("slot.fill", "slot.success"), Design),
            "never tells a good result from a bad one");
    }

    private static void D01MotionQueueKeepsEventsAndSpeedsUp()
    {
        MotionTable motions = D01Motions();
        double regular = motions.Seconds(MotionUses.JumbleSweep, Design, false);
        double queued = motions.Seconds(MotionUses.QueuedSweep, Design, false);
        var standardCurve = Design.Curves[motions[MotionUses.JumbleSweep].Curve];
        var linearCurve = Design.Curves[motions[MotionUses.QueuedSweep].Curve];
        var queue = new MotionQueue();
        var live = new MotionKey("KeyK", "KeyK", true, false, 1);
        // A single tap: its own release and a repeat wait in order but never shorten its sweep.
        var tap = new MotionQueue();
        Equal(tap.StartSweep(regular, queued).RowId, MotionUses.JumbleSweep);
        Check(tap.TryQueue(new MotionKey("KeyK", "KeyK", true, true, 2), queued, .05, .3f)
            && tap.TryQueue(new MotionKey("KeyK", "KeyK", false, false, 3), queued, .06, .4f), "a release left the queue");
        Check(tap.Active is { RowId: MotionUses.JumbleSweep } a && Math.Abs(a.Seconds - .220) < 1e-9, "a release shortened its own sweep");
        Equal(tap.PendingPresses, 0);
        tap.EndSweep(); tap.Drain(_ => { });
        Equal(tap.StartSweep(regular, queued).RowId, MotionUses.JumbleSweep, "a drained release kept the burst");
        Check(!queue.TryQueue(live, queued), "an idle key waited");
        var first = queue.StartSweep(regular, queued);
        Equal(first.RowId, MotionUses.JumbleSweep);
        Near(first.Seconds, .220);
        float phase = first.Phase(.040, standardCurve);
        MotionKey[] events =
        {
            new("KeyK", "KeyK", false, false, 2),
            new("KeyL", "Shift+KeyL", true, false, 3),
            new("KeyL", "KeyL", true, true, 4),
            new("KeyL", "KeyL", false, false, 5),
            new("KeyZ", "Ctrl+KeyZ", true, false, 6),
            new("KeyZ", "KeyZ", true, true, 7),
            new("KeyZ", "KeyZ", false, false, 8),
            new("Enter", "Ctrl+Shift+Enter", true, false, 9),
            new("Enter", "Enter", false, false, 10),
            new("KeyY", "Ctrl+KeyY", true, false, 11),
            new("KeyY", "KeyY", false, false, 12),
            live with { Qpc = 13 },
        };
        Check(events.All(key => queue.TryQueue(key, queued, .045, phase)), "a key, release or echo bypassed the queue");
        Check(!queue.TryQueue(new MotionKey("Enter", "Enter", true, false, 14), queued), "an unrelated key queued");
        Equal(queue.Count, events.Length);
        var shortened = queue.Active!.Value;
        Equal(shortened.RowId, MotionUses.QueuedSweep);
        Near(shortened.Seconds, .110);
        Equal(shortened.Phase(.045, linearCurve), phase);
        Near(shortened.Phase((.045 + .110) / 2, linearCurve), (1 + phase) / 2, 1e-6);
        Equal(shortened.Phase(.110, linearCurve), 1f);

        var seen = new List<MotionKey>();
        var timings = new List<SweepTiming> { shortened };
        void Send(MotionKey key)
        {
            seen.Add(key);
            if (key.Pressed && !key.Echo && key.Code is "KeyK" or "KeyL")
                timings.Add(queue.StartSweep(regular, queued));
        }
        queue.Drain(Send);
        Equal(seen.Count, 0, "an active sweep let an event through");
        queue.EndSweep(); queue.Drain(Send);
        Equal(seen.Count, 2, "the release and next press did not run together");
        queue.EndSweep(); queue.Drain(Send);
        Sequence(seen, events, "the FIFO dropped or reordered events");
        Near(timings[1].Seconds, MotionQueue.FrameSeconds + (.110 - MotionQueue.FrameSeconds) * .8);
        Check(timings.Zip(timings.Skip(1)).All(p => p.Second.RowId == MotionUses.QueuedSweep
            && p.Second.Seconds < p.First.Seconds && p.Second.Seconds >= MotionQueue.FrameSeconds), "the burst did not speed up");
        queue.EndSweep();
        Equal(queue.StartSweep(regular, queued), first, "an empty burst did not reset");

        // A later command flushes releases and repeats as well as presses, even with a sweep in progress.
        Check(events.All(key => queue.TryQueue(key, queued)), "a flush input bypassed the queue");
        var flushed = new List<MotionKey>();
        queue.Flush(key => { Check(queue.Active == null, "a sweep held up a flush"); flushed.Add(key); });
        Sequence(flushed, events);
        Equal(queue.Count, 0);
        Check(!queue.TryQueue(live, queued), "a key queued with no running sweep");
        Equal(queue.StartSweep(regular, queued), first, "a flush kept the burst's speed");
    }

    private static void D01MotionViewRetargetsFromTheDrawnValue()
    {
        MotionTable motions = D01Motions();
        double seconds = motions.Seconds(MotionUses.ViewTransition, Design, false);
        double[] curve = Design.Curves[motions[MotionUses.ViewTransition].Curve];
        const double step = 1.1;
        var zoom = new ViewEase(1);
        zoom.Retarget(zoom.Target * step, 1, seconds, curve);
        Equal(zoom.Drawn, 1d);
        Check(zoom.Step(1.060, false) && 1 < zoom.Drawn && zoom.Drawn < step, "the first notch jumped");
        Near(zoom.Drawn, 1 + (step - 1) * MotionTable.Ease(curve, .060 / .260));
        double midway = zoom.Drawn;
        zoom.Retarget(zoom.Target * step, 1.060, seconds, curve);
        Equal(zoom.Target, step * step);
        Equal(zoom.Drawn, midway, "the second notch jumped");
        Check(!zoom.Step(1.060, false), "retargeting jumped at time zero");
        Check(zoom.Step(1.061, false) && midway < zoom.Drawn && zoom.Drawn < step, "the zoom restarted from an old target");
        Check(zoom.Step(1.321, false), "the zoom never settled");
        Equal(zoom.Drawn, step * step);
        zoom.Retarget(zoom.Target * step, 2, seconds, curve);
        zoom.Step(2.01, true);
        Equal(zoom.Drawn, zoom.Target, "reduced motion left an ease running");
        zoom.Retarget(zoom.Target / step, 3, motions.Seconds(MotionUses.ViewTransition, Design, true), curve);
        Equal(zoom.Drawn, zoom.Target, "a reduced-motion notch waited");
    }

    private static void D01MotionAppearanceKeepsTheSwitches()
    {
        var themes = Themes();
        var look = new Appearance(Design, themes);
        string defaults = look.Store();
        Check(!look.ReducedMotion && look.Rewards, "the default motion settings changed");
        look.ReducedMotion = true;
        look.Rewards = false;
        string stored = look.Store();
        Check(stored.Contains("\nmotion=reduced", StringComparison.Ordinal) && stored.Contains("\nrewards=off", StringComparison.Ordinal), "a switch was not stored");
        var restored = Appearance.Parse(stored, Design, themes);
        Check(restored.Resolve().ReducedMotion && !restored.Rewards, "an appearance lost a switch");
        Check(!MotionSettings.Shows(MotionUses.RewardMark, restored.Rewards) && !MotionSettings.Shows(MotionUses.OrbitComplete, restored.Rewards), "a disabled reward still shows");
        Check(MotionSettings.Shows(MotionUses.Result, restored.Rewards), "disabling rewards hid an ordinary result");
        restored.Restore(defaults);
        Check(!restored.ReducedMotion && MotionSettings.Shows(MotionUses.RewardMark, restored.Rewards), "an older appearance lost the defaults");
    }

    private static void D01MotionDragValuesAndRelease()
    {
        FrameModel frame = Frame();
        Check(frame.Drop("editor", FrameModel.Pair, DropZone.Left), "no splitter to drag");
        double ratio = .5 + .0043 - .0016;
        Check(frame.Resize(frame.Centre, ratio) && frame.SetPairArrangement("side", ratio), "a drag ratio was refused");
        Equal(frame.Centre.Ratio, ratio, "the pane splitter rounded toward a preset");
        Equal(frame.PairRatio, ratio, "the pair splitter rounded toward a preset");
        var look = new Appearance(Design, Themes());
        var knob = look.Family.Knobs["tone"];
        double sample = knob.Default + (knob.Max - knob.Min) / 240;
        look.SetKnob("tone", sample);
        Equal(look.Knob("tone"), sample, "the numeric knob snapped to its default");

        var slice = new CutSlice();
        Check(slice.Press(17, .625) && slice.Named == null, "holding a slice preview named an exact twist");
        var named = slice.Release();
        Check(named is { Cap: 17, Item: 1, Angle: .625 } && slice.Named == named, "release did not use the certified cap and item");
        Check(!slice.Press(18, null) && slice.Release() == null && slice.Named == named, "a blocked preview named an exact twist");
    }
}
