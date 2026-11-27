using System;
using System.Security.Cryptography;
using System.Text.Json;

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
    /// </summary>
    public static int SendInside(HostLibrary library, IntPtr host, out int count)
    {
        count = 0;
        if (Row is { } row)
        {
            foreach (var bit in row.Moving) count += bit;
        }
        return library.SetPieceStyle(host, HostLibrary.StyleSlice, Row?.Moving, false);
    }

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
    }
}
