using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Text.Json;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

/// <summary>
/// ADR-D02 B18, the part B20 needs: the certified transform of menu item 1's twist at the cap Local draws, read from the
/// host once per cap and revision and never committed. B20's turn preview sends it with
/// <see cref="HostLibrary.SetTransform"/>; a blocked cap has no row, so no ghost is drawn there.
/// </summary>
public static class ShellTurn
{
    private static (int Cap, ulong Revision)? _read;

    /// <summary>The cap Local draws (the named cap, else the default), or null with no state bound.</summary>
    public static int? Cap { get; private set; }

    /// <summary>The row of item 1's twist at <see cref="Cap"/>; null when that twist is not admissible.</summary>
    public static HostLibrary.TwistRow? Row { get; private set; }

    /// <summary>The host's code for the last read.</summary>
    public static int Code { get; private set; } = HostLibrary.Ok;

    /// <summary>The engine revision the host read the last row at (the head's), which the transform setter names.</summary>
    public static ulong RowRevision { get; private set; }

    /// <summary>
    /// Reads the row when the cap Local draws or the revision changed. The cap is the one the frame loop's framing
    /// call left (<see cref="ShellLocal.Plan"/>'s Framed, set by the shell's FollowLocalCap before the style is sent),
    /// never a second stateful framing call, so naming and selection precedence stay the renderer's (B18 review finding
    /// B18-05). A refusal, a session without Jumble included, is kept for its cap and revision like a row, so it is not
    /// asked again every frame (B18-06).
    /// </summary>
    public static void Note(HostLibrary library, IntPtr host, ulong revision)
    {
        if (ShellLocal.Plan.Framed is not int cap)
        {
            (Cap, Row, _read) = (null, null, null);
            return;
        }
        if (_read is { } read && read.Cap == cap && read.Revision == revision)
        {
            return;
        }
        Code = library.TwistTransform(host, (uint)cap, 1, out var row, out var at);
        (Cap, Row, RowRevision, _read) = (cap, row, at, (cap, revision));
    }

    /// <summary>
    /// B20: the cut slice's inside layer, the slice channel of the piece style: the row's moving set (at an admissible
    /// cap, exactly the cap's inside set, which B18's twist-row run checks by sha256), or cleared without a row.
    /// B18-04: the moving set is sent only while the row's revision is the adopted one: a row read at a head the host
    /// has not adopted yet (a commit in flight) answers <see cref="HostLibrary.Unavailable"/> and sends nothing, so the
    /// layer sent for the adopted revision stays and the caller sends again at the next step.
    /// </summary>
    public static int SendInside(HostLibrary library, IntPtr host, ulong adopted, out int count)
    {
        count = 0;
        if (Row is { } row && row.Revision != adopted)
        {
            return HostLibrary.Unavailable;
        }
        var moving = InsideFor(Row, adopted);
        if (moving is not null)
        {
            foreach (var bit in moving) count += bit;
        }
        return library.SetPieceStyle(host, adopted, HostLibrary.StyleSlice, moving, false);
    }

    /// <summary>B18-04: the row's moving set on the revision it was read at, else none.</summary>
    public static byte[]? InsideFor(HostLibrary.TwistRow? row, ulong adopted) => row is { } r && r.Revision == adopted ? r.Moving : null;

    /// <summary>
    /// B20: the turn preview, the one call site of the row's transform. On: the row's eight plane floats with the angle
    /// negated at phase 0, so the moving pieces draw the twist's result; off (or no row): cleared. Nothing is committed.
    /// </summary>
    public static int Preview(HostLibrary library, IntPtr host, bool on)
    {
        if (on && Row is { Plane.Length: 8 } row)
        {
            // B18-04: the row names the revision it was read at; the host refuses it on any other adopted revision,
            // and the slice sends it again at the next step.
            return library.SetTransform(host, row.Revision, row.Plane, -row.Angle, row.Moving, 0);
        }
        return library.SetTransform(host, 0, null, 0, null, 0);
    }

    // ---- B18 rest: the timed sweep and the half-turn flash of a committed key twist (ADR-D02.37, row J03). ----

    /// <summary>
    /// The router's default twist keys (c600-command's draft TWIST_KEYS: the key, its menu item and Shift's inverse item).
    /// test_d02_drawing.py pins this table to keymap.rs.
    /// </summary>
    public static readonly (string Key, ulong Item, ulong Inverse)[] TwistKeys = { ("KeyJ", 1, 1), ("KeyK", 2, 5), ("KeyL", 3, 9) };

    /// <summary>The router's default grip cap.</summary>
    public const uint GripCap = 0;

    private static uint ActiveGripCap => ShellGrip.Model.Ready ? (uint)ShellGrip.Model.Cap : GripCap;

    /// <summary>A motion row's seconds (0 under reduced motion: its static equivalent at once) and its curve, from the
    /// shell's motion table (ShellChrome sets it).</summary>
    public static Func<string, (double Seconds, double[] Curve)>? MotionRow { get; set; }

    /// <summary>A sweep, a flash or a static cut of one committed twist, as the dump reports it.</summary>
    private sealed class Motion
    {
        public Motion(string kind, ulong revision, HostLibrary.TwistRow row, SweepTiming timing, double[] curve, Outline? chartOutline)
        {
            (Kind, Revision, Row, Timing, Curve, ChartOutline) = (kind, revision, row, timing, curve, chartOutline);
            Committed = Stopwatch.GetTimestamp();
        }

        public string Kind { get; }
        public ulong Revision { get; }
        public HostLibrary.TwistRow Row { get; }
        public SweepTiming Timing { get; set; }
        public double Seconds => Timing.Seconds;
        public double[] Curve { get; set; }
        public Outline? ChartOutline { get; }
        public long Committed { get; }
        public long? Started { get; set; }
        public bool Popped { get; set; }
        public bool Held { get; set; }
        public ulong? FirstFrameRevision { get; set; }
        public string Ended { get; set; } = "";
        public double ShownMs { get; set; }
        public List<(double Ms, float Phase)> Phases { get; } = new();
    }

    private static HostLibrary.TwistRow? _armed;
    private static Motion? _motion;
    private static readonly List<Motion> _done = new();
    private static int _refused;

    /// <summary>The menu item a chord twists at the grip cap, or null for a key that is not a twist key.</summary>
    public static ulong? ItemOf(string chord)
    {
        if (ShellGrip.Model.Ready) return ShellGrip.Model.Key(chord)?.Item;
        var shift = chord.StartsWith("Shift+", StringComparison.Ordinal);
        var key = shift ? chord.Substring(6) : chord;
        foreach (var (name, item, inverse) in TwistKeys)
        {
            if (name == key) return shift ? inverse : item;
        }
        return null;
    }

    /// <summary>Before a key-down reaches the session: a twist key's certified row at the grip cap, read at the head the
    /// press would twist (none when the twist is not admissible there, so a refusal never sweeps).</summary>
    public static void Arm(HostLibrary library, IntPtr host, string chord)
    {
        _armed = null;
        if (ItemOf(chord) is ulong item && library.TwistTransform(host, ActiveGripCap, item, out var row, out _) == HostLibrary.Ok)
        {
            _armed = row;
        }
    }

    /// <summary>Brief SF2: the grip cap and menu item a posted twist key arms (its row is read by the host just before
    /// the key, at the head the key will twist), or null for a key that is not a twist key.</summary>
    public static (uint Cap, ulong Item)? ArmFor(string chord) => ItemOf(chord) is ulong item ? (ActiveGripCap, item) : null;

    /// <summary>Brief SF2: the row the host read for the answered key (null: none, or not admissible).</summary>
    public static void Armed(HostLibrary.TwistRow? row) => _armed = row;

    /// <summary>
    /// A committed press at engine revision <paramref name="revision"/>. The armed row is that twist's when the press
    /// entered the revision after the row's (the head the row was read at); then a half-turn (angle pi) flashes for
    /// jumble.half_turn's fixed time and any other twist sweeps for jumble.sweep's; under reduced motion (0 seconds) both
    /// are static: the cut to the new poses, nothing drawn turning or outlined. A motion still running ends where it is.
    /// </summary>
    public static void Committed(ulong revision, bool staticMotion)
    {
        var row = _armed;
        _armed = null;
        End("next twist");
        _outline = null;
        if (row is null || revision != row.Revision + 1 || MotionRow is null)
        {
            return;
        }
        var half = Math.Abs(row.Angle - MathF.PI) < 1e-4f;
        var id = half ? MotionUses.HalfTurn : MotionUses.JumbleSweep;
        var (seconds, curve) = MotionRow(id);
        if (staticMotion) seconds = 0;
        var kind = seconds <= 0 ? "static" : half ? "flash" : "sweep";
        // B22 rest (ADR-D02.10): with a chart in Global (whose layout draws no turn, the core's viewFlags bit 3) a sweep
        // outlines the moved set for the half-turn's fixed time, in Global only; Local carries the motion. With the
        // slice in Local at another cap than the twisted one, no view sweeps it.
        if (kind == "sweep" && ShellHopf.Drawn != ShellHopf.Chain)
        {
            var (outlineSeconds, _) = MotionRow(MotionUses.HalfTurn);
            _outline = outlineSeconds > 0 ? new Outline(revision, row.Moving, outlineSeconds) : null;
            if (Cap != (int)ActiveGripCap)
            {
                kind = "static";
            }
        }
        var timing = new SweepTiming(id, kind == "static" ? 0 : seconds);
        if (kind == "sweep")
        {
            var queued = MotionRow(MotionUses.QueuedSweep);
            timing = ShellMotion.Keys.StartSweep(seconds, queued.Seconds);
            if (timing.RowId == MotionUses.QueuedSweep) curve = queued.Curve;
        }
        _motion = new Motion(kind, revision, row, timing, curve, _outline);
        if (_motion.Kind == "static")
        {
            End("static");
        }
    }

    internal static bool Queue(MotionKey key)
    {
        var motion = _motion;
        var queued = MotionRow?.Invoke(MotionUses.QueuedSweep);
        double elapsed = motion?.Started is long since ? Stopwatch.GetElapsedTime(since, key.Qpc).TotalSeconds : 0;
        float phase = motion is { Phases.Count: > 0 } ? motion.Phases[^1].Phase : 0;
        if (!ShellMotion.Keys.TryQueue(key, queued?.Seconds ?? 0, elapsed, phase)) return false;
        if (motion is { Kind: "sweep" } && ShellMotion.Keys.Active is { } timing && timing != motion.Timing)
        {
            motion.Timing = timing;
            motion.Curve = queued!.Value.Curve;
        }
        return true;
    }

    internal static void CutMotion(HostLibrary library, IntPtr host, bool reduced)
    {
        if (_motion is { Kind: "sweep" } motion)
        {
            if (motion.Held)
                library.SetTransform(host, motion.Revision, motion.Row.Plane, motion.Row.Angle, motion.Row.Moving, 1);
            End(reduced ? "reduced" : "flushed");
        }
        else if (reduced && End("reduced")) ShellStyle.Invalidate();
        if (reduced)
        {
            if (_outline?.Started is not null) ShellStyle.Invalidate();
            _outline = null;
        }
    }

    /// <summary>A refused press: no sweep, no flash.</summary>
    public static void Refused()
    {
        _armed = null;
        _refused++;
    }

    /// <summary>The flash's moving set while it shows (the outline channel draws it in palette entry 15), else null.</summary>
    public static byte[]? Flash => _motion is { Kind: "flash", Started: not null } motion ? motion.Row.Moving
        : _outline is { Started: not null } outline ? outline.Moving : null;

    /// <summary>B22 rest: the chart's moved-set outline shows and no flash does, so Local's panes leave entry 15 out.</summary>
    public static bool OutlineOnly => _outline is { Started: not null } && _motion is not { Kind: "flash", Started: not null };

    // B22 rest: the moved set's outline under a chart (ADR-D02.10), its count shown and ended for the dump.
    private sealed class Outline
    {
        public Outline(ulong revision, byte[] moving, double seconds) => (Revision, Moving, Seconds) = (revision, moving, seconds);
        public ulong Revision { get; }
        public byte[] Moving { get; }
        public double Seconds { get; }
        public long? Started { get; set; }
    }

    private static Outline? _outline;
    private static int _outlinesShown, _outlinesEnded;
    private static double _outlineMs;

    // Starts the outline on its adopted revision and ends it after its time or at a newer revision; true when the
    // outline bytes must be sent again.
    private static bool StepOutline(ulong adopted)
    {
        if (_outline is not { } outline || adopted < outline.Revision)
        {
            return false;
        }
        var now = Stopwatch.GetTimestamp();
        if (adopted > outline.Revision || (outline.Started is long started
            && (_outlineMs = Stopwatch.GetElapsedTime(started, now).TotalMilliseconds) >= outline.Seconds * 1000))
        {
            _outline = null;
            _outlinesEnded++;
            return outline.Started is not null;
        }
        if (outline.Started is null)
        {
            outline.Started = now;
            _outlinesShown++;
            return true;
        }
        return false;
    }

    /// <summary>
    /// Each step before the frame. A sweep names phase 0 for its revision at once: the host holds it while the revision
    /// is handed out but not adopted and sets it with the adoption, so the revision's first frame draws the old pose.
    /// Its time starts at the first step after that frame (<see cref="Framed"/>), past the shell's work at a newly
    /// adopted revision (about 60 ms in the product runs of 27 November 2026), so no part of the sweep is skipped;
    /// then it sets the eased phase of its time, and 1 at its end. The next adoption drops it. A flash starts on the adopted revision and ends after its time. A newer
    /// adopted revision ends either. Answers true when the flash's outline must be sent again.
    /// </summary>
    public static bool Step(HostLibrary library, IntPtr host, ulong adopted)
    {
        ShellMotion.ApplyRawClip?.Invoke();
        return StepOutline(adopted) | StepMotion(library, host, adopted);
    }

    private static bool StepMotion(HostLibrary library, IntPtr host, ulong adopted)
    {
        if (_motion is not { } motion)
        {
            return false;
        }
        var now = Stopwatch.GetTimestamp();
        if (adopted > motion.Revision)
        {
            return End("superseded");
        }
        if (motion.Kind == "flash")
        {
            if (adopted < motion.Revision)
            {
                return false;
            }
            if (motion.Started is not long started)
            {
                motion.Started = now;
                return true;
            }
            motion.ShownMs = Stopwatch.GetElapsedTime(started, now).TotalMilliseconds;
            return motion.ShownMs >= motion.Seconds * 1000 && End("done");
        }
        var row = motion.Row;
        if (motion.Started is null && motion.FirstFrameRevision is not null)
        {
            motion.Started = now;
            motion.Phases.Add((0, motion.Popped ? 1f : 0f));
            return false;
        }
        if (motion.Started is not long since)
        {
            if (!motion.Held)
            {
                motion.Held = library.SetTransform(host, motion.Revision, row.Plane, row.Angle, row.Moving, 0) == HostLibrary.Ok;
            }
            // Never drawn: ended after two seconds.
            return Stopwatch.GetElapsedTime(motion.Committed, now).TotalSeconds > 2 && End(motion.Held ? "never drawn" : "refused by the host");
        }
        var elapsed = Stopwatch.GetElapsedTime(since, now).TotalSeconds;
        var phase = motion.Timing.Phase(elapsed, motion.Curve);
        if (library.SetTransform(host, motion.Revision, row.Plane, row.Angle, row.Moving, phase) != HostLibrary.Ok)
        {
            return End("refused by the host");
        }
        motion.Phases.Add((elapsed * 1000, phase));
        return elapsed >= motion.Seconds && End("done");
    }

    /// <summary>After a submitted frame: the first frame that drew the sweep's revision drew its phase 0; when no phase 0
    /// was held for it, that frame drew the end pose first (popped, the dump reports it).</summary>
    public static void Framed(ulong engineRevision)
    {
        if (_motion is { Kind: "sweep", FirstFrameRevision: null } motion && engineRevision == motion.Revision)
        {
            motion.Popped = !motion.Held;
            motion.FirstFrameRevision = engineRevision;
        }
    }

    // Ends the running motion; true when it was a shown flash, whose outline must be cleared.
    private static bool End(string why)
    {
        if (_motion is not { } motion)
        {
            return false;
        }
        motion.Ended = why;
        _done.Add(motion);
        if (_done.Count > 16) _done.RemoveAt(0);
        _motion = null;
        if (motion.Kind == "sweep") ShellMotion.SweepEnded();
        return motion.Kind == "flash" && motion.Started is not null;
    }

    /// <summary>The dump's sweep row: refused presses, the running motion's kind, and each ended motion with its kind,
    /// revision, angle, moving count, duration, why it ended, whether it popped, and its phases with their times.</summary>
    private static void DumpSweep(Utf8JsonWriter json)
    {
        json.WriteStartObject("sweep");
        json.WriteNumber("refused", _refused);
        json.WriteBoolean("reduced", MotionRow?.Invoke("jumble.sweep").Seconds is 0);
        // B22 rest: Global's chart, and its moved-set outlines shown and ended.
        json.WriteBoolean("chart", ShellHopf.Drawn != ShellHopf.Chain);
        json.WriteNumber("outlines_shown", _outlinesShown);
        json.WriteNumber("outlines_ended", _outlinesEnded);
        json.WriteNumber("outline_ms", Math.Round(_outlineMs, 1));
        if (_motion is { } running) json.WriteString("running", running.Kind); else json.WriteNull("running");
        json.WriteStartArray("motions");
        foreach (var motion in _done)
        {
            json.WriteStartObject();
            json.WriteString("kind", motion.Kind);
            json.WriteNumber("revision", motion.Revision);
            json.WriteNumber("angle", motion.Row.Angle);
            var moving = 0;
            foreach (var bit in motion.Row.Moving) moving += bit;
            json.WriteNumber("moving", moving);
            json.WriteNumber("duration_ms", motion.Seconds * 1000);
            json.WriteString("ended", motion.Ended);
            json.WriteBoolean("popped", motion.Popped);
            json.WriteNumber("shown_ms", Math.Round(motion.ShownMs, 1));
            json.WriteStartArray("ms");
            foreach (var (ms, _) in motion.Phases) json.WriteNumberValue(Math.Round(ms, 1));
            json.WriteEndArray();
            json.WriteStartArray("phases");
            foreach (var (_, phase) in motion.Phases) json.WriteNumberValue(phase);
            json.WriteEndArray();
            json.WriteEndObject();
        }
        json.WriteEndArray();
        json.WriteEndObject();
    }

    /// <summary>The dump's turn row: the cap, the host's code, the status and, for a row, its moving count, the sha256
    /// of its moving set packed one bit per piece (least significant bit first), its angle and its plane.</summary>
    public static void Dump(Utf8JsonWriter json)
    {
        json.WriteStartObject("turn");
        if (Cap is int cap) json.WriteNumber("cap", cap); else json.WriteNull("cap");
        json.WriteNumber("code", Code);
        json.WriteNumber("status", Row is null ? 0 : 1);
        json.WriteNumber("read_at", RowRevision);
        if (Row is { } row)
        {
            long moving = 0;
            var bits = new byte[(row.Moving.Length + 7) / 8];
            for (var piece = 0; piece < row.Moving.Length; piece++)
            {
                if (row.Moving[piece] == 0) continue;
                moving++;
                bits[piece >> 3] |= (byte)(1 << (piece & 7));
            }
            json.WriteNumber("moving", moving);
            json.WriteString("moving_sha256", Convert.ToHexString(SHA256.HashData(bits)).ToLowerInvariant());
            // B18-08: the row's revision and the sha256 of its eight plane floats (IEEE single, little-endian).
            json.WriteNumber("revision", row.Revision);
            var plane = new byte[row.Plane.Length * 4];
            Buffer.BlockCopy(row.Plane, 0, plane, 0, plane.Length);
            json.WriteString("plane_sha256", Convert.ToHexString(SHA256.HashData(plane)).ToLowerInvariant());
            json.WriteNumber("angle", row.Angle);
            json.WriteStartArray("plane");
            foreach (var value in row.Plane)
            {
                json.WriteNumberValue(value);
            }
            json.WriteEndArray();
        }
        json.WriteEndObject();
        DumpSweep(json);
    }

    internal static void DumpMotion(Utf8JsonWriter json)
    {
        json.WriteNumber("queue_length", ShellMotion.Keys.Count);
        json.WriteNumber("queued_events", ShellMotion.Keys.Enqueued);
        json.WriteStartArray("turns");
        foreach (var motion in _done) Write(motion);
        if (_motion is { } running) Write(running);
        json.WriteEndArray();

        void Write(Motion motion)
        {
            json.WriteStartObject();
            json.WriteString("kind", motion.Kind);
            json.WriteString("row_id", motion.Timing.RowId);
            json.WriteNumber("duration_ms", motion.Seconds * 1000);
            json.WriteNumber("revision", motion.Revision);
            json.WriteNumber("armed_revision", motion.Row.Revision);
            if (motion.FirstFrameRevision is ulong revision) json.WriteNumber("first_frame_revision", revision);
            else json.WriteNull("first_frame_revision");
            json.WriteBoolean("popped", motion.Popped);
            json.WriteString("ended", motion.Ended);
            if (motion.ChartOutline is { } outline)
            {
                json.WriteStartObject("chart_outline");
                json.WriteString("row_id", MotionUses.HalfTurn);
                json.WriteNumber("duration_ms", outline.Seconds * 1000);
                json.WriteNumber("revision", outline.Revision);
                json.WriteEndObject();
            }
            json.WriteEndObject();
        }
    }
}
