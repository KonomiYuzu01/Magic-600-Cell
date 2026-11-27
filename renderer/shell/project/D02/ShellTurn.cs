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
        return library.SetPieceStyle(host, HostLibrary.StyleSlice, moving, false);
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
    /// The router's twist keys (c600-command's draft TWIST_KEYS: the key, its menu item and Shift's inverse item) and its
    /// one grip cap (slot 0, cap 0, while grip.select is not built). A key twist commits that item at that cap, so the
    /// shell reads the twist's certified row before the press. test_d02_drawing.py pins this table to keymap.rs.
    /// </summary>
    public static readonly (string Key, ulong Item, ulong Inverse)[] TwistKeys = { ("KeyJ", 1, 1), ("KeyK", 2, 5), ("KeyL", 3, 9) };

    /// <summary>The router's grip cap, the cap every twist key turns.</summary>
    public const uint GripCap = 0;

    /// <summary>A motion row's seconds (0 under reduced motion: its static equivalent at once) and its curve, from the
    /// shell's motion table (ShellChrome sets it).</summary>
    public static Func<string, (double Seconds, double[] Curve)>? MotionRow { get; set; }

    /// <summary>A sweep, a flash or a static cut of one committed twist, as the dump reports it.</summary>
    private sealed class Motion
    {
        public Motion(string kind, ulong revision, HostLibrary.TwistRow row, double seconds, double[] curve)
        {
            (Kind, Revision, Row, Seconds, Curve, Committed) = (kind, revision, row, seconds, curve, Stopwatch.GetTimestamp());
        }

        public string Kind { get; }
        public ulong Revision { get; }
        public HostLibrary.TwistRow Row { get; }
        public double Seconds { get; }
        public double[] Curve { get; }
        public long Committed { get; }
        public long? Started { get; set; }
        public bool Popped { get; set; }
        public bool Held { get; set; }
        public bool Drawn { get; set; }
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
        if (ItemOf(chord) is ulong item && library.TwistTransform(host, GripCap, item, out var row, out _) == HostLibrary.Ok)
        {
            _armed = row;
        }
    }

    /// <summary>
    /// A committed press at engine revision <paramref name="revision"/>. The armed row is that twist's when the press
    /// entered the revision after the row's (the head the row was read at); then a half-turn (angle pi) flashes for
    /// jumble.half_turn's fixed time and any other twist sweeps for jumble.sweep's; under reduced motion (0 seconds) both
    /// are static: the cut to the new poses, nothing drawn turning or outlined. A motion still running ends where it is.
    /// </summary>
    public static void Committed(ulong revision)
    {
        var row = _armed;
        _armed = null;
        End("next twist");
        if (row is null || revision != row.Revision + 1 || MotionRow is null)
        {
            return;
        }
        var half = Math.Abs(row.Angle - MathF.PI) < 1e-4f;
        var (seconds, curve) = MotionRow(half ? "jumble.half_turn" : "jumble.sweep");
        _motion = new Motion(seconds <= 0 ? "static" : half ? "flash" : "sweep", revision, row, seconds, curve);
        if (_motion.Kind == "static")
        {
            End("static");
        }
    }

    /// <summary>A refused press: no sweep, no flash.</summary>
    public static void Refused()
    {
        _armed = null;
        _refused++;
    }

    /// <summary>The flash's moving set while it shows (the outline channel draws it in palette entry 15), else null.</summary>
    public static byte[]? Flash => _motion is { Kind: "flash", Started: not null } motion ? motion.Row.Moving : null;

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
        if (motion.Started is null && motion.Drawn)
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
        var ms = Stopwatch.GetElapsedTime(since, now).TotalMilliseconds;
        var t = Math.Min(1, ms / (motion.Seconds * 1000));
        var phase = t >= 1 ? 1f : (float)MotionTable.Ease(motion.Curve, t);
        if (library.SetTransform(host, motion.Revision, row.Plane, row.Angle, row.Moving, phase) != HostLibrary.Ok)
        {
            return End("refused by the host");
        }
        motion.Phases.Add((ms, phase));
        return t >= 1 && End("done");
    }

    /// <summary>After a submitted frame: the first frame that drew the sweep's revision drew its phase 0; when no phase 0
    /// was held for it, that frame drew the end pose first (popped, the dump reports it).</summary>
    public static void Framed(ulong engineRevision)
    {
        if (_motion is { Kind: "sweep", Started: null, Drawn: false } motion && engineRevision == motion.Revision)
        {
            motion.Popped = !motion.Held;
            motion.Drawn = true;
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
        return motion.Kind == "flash" && motion.Started is not null;
    }

    /// <summary>The dump's sweep row: refused presses, the running motion's kind, and each ended motion with its kind,
    /// revision, angle, moving count, duration, why it ended, whether it popped, and its phases with their times.</summary>
    private static void DumpSweep(Utf8JsonWriter json)
    {
        json.WriteStartObject("sweep");
        json.WriteNumber("refused", _refused);
        json.WriteBoolean("reduced", MotionRow?.Invoke("jumble.sweep").Seconds is 0);
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
}
