using System;
using System.Collections.Generic;
using System.Linq;

namespace Magic600.Chrome;

public readonly record struct MotionKey(string Code, string Chord, bool Pressed, bool Echo, long Qpc);

public readonly record struct SweepTiming(string RowId, double Seconds, double FromTime = 0, float FromPhase = 0)
{
    public float Phase(double elapsed, double[] curve)
    {
        if (elapsed >= Seconds) return 1;
        if (elapsed <= FromTime) return FromPhase;
        double share = MotionTable.Ease(curve, (elapsed - FromTime) / (Seconds - FromTime));
        return (float)(FromPhase + (1 - FromPhase) * share);
    }
}

// Only the input waits. The caller delivers each event and starts a sweep only for a committed twist it can draw.
public sealed class MotionQueue
{
    public const double FrameSeconds = 1.0 / 60;
    private readonly Queue<MotionKey> pending = new();
    private readonly HashSet<string> held = new(StringComparer.Ordinal);
    private double? lastQueuedSeconds;

    public int Count => pending.Count;
    public int PendingPresses => pending.Count(key => key.Pressed && !key.Echo);
    public long Enqueued { get; private set; }
    public SweepTiming? Active { get; private set; }

    public static bool ChangesHead(string chord) => chord is
        "KeyJ" or "KeyK" or "KeyL" or "Shift+KeyJ" or "Shift+KeyK" or "Shift+KeyL" or
        "Ctrl+Shift+Enter" or "Ctrl+KeyZ" or "Ctrl+KeyY";

    public bool TryQueue(MotionKey key, double queuedSeconds, double elapsed = 0, float drawnPhase = 0)
    {
        // A release can arrive after its modifiers were released, and repeats follow the held physical key.
        bool headKey = ChangesHead(key.Chord) || held.Contains(key.Code);
        if (!key.Pressed) held.Remove(key.Code);
        else if (headKey) held.Add(key.Code);
        if (!headKey || (Active is null && Count == 0)) return false;

        pending.Enqueue(key);
        Enqueued++;
        // Only a new press speeds the running sweep up: a release or a repeat keeps its place in the queue, so a
        // single tap's own release never shortens its sweep.
        if (key.Pressed && !key.Echo && Active is { RowId: MotionUses.JumbleSweep } current)
        {
            double seconds = Math.Min(current.Seconds, queuedSeconds);
            lastQueuedSeconds = seconds;
            Active = new SweepTiming(MotionUses.QueuedSweep, seconds, elapsed, drawnPhase);
        }
        return true;
    }

    public SweepTiming StartSweep(double seconds, double queuedSeconds)
    {
        var timing = new SweepTiming(MotionUses.JumbleSweep, seconds);
        if (lastQueuedSeconds.HasValue || PendingPresses > 0)
        {
            // Reduce the time above one frame on each sweep, including the last press taken from the queue.
            double faster = lastQueuedSeconds is double prior ? FrameSeconds + (prior - FrameSeconds) * .8 : queuedSeconds;
            lastQueuedSeconds = faster;
            timing = new SweepTiming(MotionUses.QueuedSweep, faster);
        }
        Active = timing;
        return timing;
    }

    public void EndSweep()
    {
        Active = null;
        if (Count == 0) lastQueuedSeconds = null;
    }

    public void Drain(Action<MotionKey> send)
    {
        while (Active is null && pending.TryDequeue(out var key)) send(key);
        if (Active is null) lastQueuedSeconds = null;
    }

    public void Flush(Action<MotionKey> sendStatic)
    {
        Active = null;
        while (pending.TryDequeue(out var key)) sendStatic(key);
        lastQueuedSeconds = null;
    }

    public void ReleaseHeldKeys() => held.Clear();
}
