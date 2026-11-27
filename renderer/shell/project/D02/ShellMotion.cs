using System;
using System.Diagnostics;
using System.Text.Json;
using Godot;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

internal static class ShellMotion
{
    internal static MotionQueue Keys { get; } = new();
    internal static double Now => Stopwatch.GetTimestamp() / (double)Stopwatch.Frequency;
    internal static Action? ApplyRawClip { get; set; }
    private static Action<MotionKey, bool>? _send;
    /// <summary>Brief SF2: the keys posted to the host that have no answer yet (the script driver's dumps wait for
    /// them).</summary>
    internal static int Posted { get; private set; }
    internal static bool Sweeps => (ShellTurn.MotionRow?.Invoke(MotionUses.JumbleSweep).Seconds ?? 0) > 0;
    private static (int Cap, ViewEase Ease)? _clip;

    internal static bool TryQueue(MotionKey key, Action<MotionKey, bool> send)
    {
        if (!ShellTurn.Queue(key)) return false;
        _send ??= send;
        return true;
    }

    /// <summary>Brief SF2: after the frame's answers, a key still waiting for its answer holds the queue; with none, a
    /// queue that no sweep runs (a refused, static or other answer) sends its next keys, as a synchronous key did.</summary>
    internal static void KeysAnswered(int posted, bool waiting)
    {
        Posted = posted;
        Keys.Waiting = waiting;
        if (!waiting && Keys.Active is null && Keys.Count > 0) Callable.From(Drain).CallDeferred();
    }

    internal static void KeyPosted(bool staticMotion)
    {
        Posted++;
        if (!staticMotion) Keys.Waiting = true;
    }

    internal static void SweepEnded()
    {
        Keys.EndSweep();
        // The shell still has to submit and account for this frame at the old head.
        if (Keys.Count > 0) Callable.From(Drain).CallDeferred();
    }

    private static void Drain()
    {
        if (_send is { } send) Keys.Drain(key => send(key, false));
        if (Keys.Count == 0) _send = null;
    }

    internal static void Flush()
    {
        if (_send is { } send) Keys.Flush(key => send(key, true));
        _send = null;
    }

    internal static float? StepRawClip(int? cap, double? target)
    {
        if (cap is not int cell || target is not double h)
        {
            _clip = null;
            return null;
        }
        if (_clip is not { } clip || clip.Cap != cell)
        {
            _clip = (cell, new ViewEase(h));
            return null;
        }
        var (seconds, curve) = ShellTurn.MotionRow?.Invoke(MotionUses.ViewTransition) ?? (0, Array.Empty<double>());
        var now = Now;
        bool retargeted = clip.Ease.Target != h;
        if (retargeted) clip.Ease.Retarget(h, now, seconds, curve);
        bool moved = clip.Ease.Step(now, seconds <= 0);
        // FollowLocalCap sent the requested clip on a change. Replace it with the eased clip before the frame.
        return moved || (retargeted && seconds > 0) ? (float)clip.Ease.Drawn : null;
    }

    internal static void DumpRawClip(Utf8JsonWriter json)
    {
        if (_clip is not { } clip) return;
        json.WriteStartObject("raw_clip");
        json.WriteString("row_id", MotionUses.ViewTransition);
        json.WriteNumber("drawn", clip.Ease.Drawn);
        json.WriteNumber("target", clip.Ease.Target);
        json.WriteNumber("duration_ms", clip.Ease.Seconds * 1000);
        json.WriteEndObject();
    }
}
