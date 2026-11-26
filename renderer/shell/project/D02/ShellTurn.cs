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

    /// <summary>Reads the row when the cap Local draws or the revision changed; a refusal is read again next frame.</summary>
    public static void Note(HostLibrary library, IntPtr host, ulong revision)
    {
        if (ShellLocal.Frame(ShellJumble.Host?.Overlay.Cap) is not int cap)
        {
            (Cap, Row, _read) = (null, null, null);
            return;
        }
        if (_read is { } read && read.Cap == cap && read.Revision == revision)
        {
            return;
        }
        Code = library.TwistTransform(host, (uint)cap, 1, out var row);
        (Cap, Row) = (cap, row);
        _read = Code == HostLibrary.Ok ? (cap, revision) : null;
    }

    /// <summary>The dump's turn row: the cap, the host's code, the status and, for a row, its moving count, the sha256
    /// of its moving set packed one bit per piece (least significant bit first), its angle and its plane.</summary>
    public static void Dump(Utf8JsonWriter json)
    {
        json.WriteStartObject("turn");
        if (Cap is int cap) json.WriteNumber("cap", cap); else json.WriteNull("cap");
        json.WriteNumber("code", Code);
        json.WriteNumber("status", Row is null ? 0 : 1);
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
