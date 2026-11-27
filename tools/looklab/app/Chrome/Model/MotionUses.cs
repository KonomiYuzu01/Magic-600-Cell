using System;
using System.Collections.Generic;

namespace Magic600.Chrome;

public enum MotionKind { Animated, Static, Held }
public sealed record MotionUse(string What, string RowId, string Trigger, MotionKind Kind);

// The D02 shell's uses of H-03. The Gallery's player takes the row being demonstrated directly from the table.
public static class MotionUses
{
    public const string JumbleSweep = "jumble.sweep", QueuedSweep = "twist.queued", HalfTurn = "jumble.half_turn";
    public const string ViewTransition = "view.transition", DockOpen = "dock.open", StripChange = "strip.change";
    public const string RewardMark = "reward.mark", OrbitComplete = "orbit.complete", Result = "slot.fill";

    public static IReadOnlyList<MotionUse> All { get; } = Array.AsReadOnly(new MotionUse[]
    {
        new("Committed twist sweep, including Local's slice at the turning cap", JumbleSweep, "state", MotionKind.Animated),
        new("Queued twist sweep", QueuedSweep, "state", MotionKind.Animated),
        new("Half-turn moved-set flash", HalfTurn, "state", MotionKind.Animated),
        new("B22 moved-set outline in Global's chart", HalfTurn, "state", MotionKind.Animated),
        new("Independent slice turn", "slice.turn", "view", MotionKind.Held),
        new("Held slice preview ghost", "twist.preview", "view", MotionKind.Static),
        new("Wheel zoom", ViewTransition, "view", MotionKind.Animated),
        new("Raw chart clip", ViewTransition, "view", MotionKind.Animated),
        new("Chart, split and focus settings", ViewTransition, "view", MotionKind.Static),
        new("Palette and in-window chrome overlays", DockOpen, "view", MotionKind.Animated),
        new("Dock panels and centre panes", DockOpen, "view", MotionKind.Static),
        new("View overlays and tint", ViewTransition, "view", MotionKind.Static),
        new("ADR-D01.25 puzzle diagnostic overlay", Result, "state", MotionKind.Static),
        new("ADR-D01.65 status patterns", Result, "state", MotionKind.Static),
        new("Rail lock appears", "lock.set", "state", MotionKind.Static),
        new("Rail lock clears", "lock.release", "state", MotionKind.Static),
        new("Jumble marks appear", "jumble.enter", "state", MotionKind.Static),
        new("Jumble marks clear", "jumble.leave", "state", MotionKind.Static),
        new("Repair step's moving set", "repair.step", "view", MotionKind.Static),
        new("Named cap and meeting caps", "cap.name", "view", MotionKind.Static),
        new("Pending glyph and settling lamp", "marker.pending", "state", MotionKind.Static),
        new("Milestone reward in the product", RewardMark, "state", MotionKind.Held),
        new("Sample first-orbit completion moment in History", OrbitComplete, "state", MotionKind.Static),
        new("Sample orbit rail solved mark", OrbitComplete, "state", MotionKind.Static),
        new("Check and repair results", Result, "state", MotionKind.Static),
        new("Sample relationship strip edge", StripChange, "state", MotionKind.Animated),
        new("Sample Next and Target marks", "next.activate", "state", MotionKind.Static),
        new("Sample rollback cursor", "cursor.rollback", "state", MotionKind.Static),
        new("Sample Draft and Direct marks", "route.switch", "state", MotionKind.Static),
    });
}
