using System;
using System.Collections.Generic;
using System.Linq;
using System.Security.Cryptography;
using System.Text;

namespace Magic600.Chrome;

// ADR-D01.51: the frame marks changes to the colours that carry state or encoding.
public static class ColourMark
{
    public const string Slot = "mark.colours";

    public static IReadOnlyList<ViewMark> BorderMarks(TokenSet tokens, string view)
    {
        if (!tokens.NonStandardColours) return Array.Empty<ViewMark>();
        string[] roles = tokens.Colours.Where(c => c.Value.Overridden && tokens.System.Roles[c.Key].Group is "state" or "encoding")
            .Select(c => c.Key).OrderBy(name => name, StringComparer.Ordinal).ToArray();
        string names = string.Join(", ", roles.Take(6));
        if (roles.Length > 6) names += FormattableString.Invariant($" and {roles.Length - 6} more");
        return new[] { new ViewMark("panel.appearance", "non-standard colours", "severity.warning",
            $"Overridden roles: {names}\nReset colour token (palette) resets one\nToken set {Digest(tokens)}") };
    }

    public static string Digest(TokenSet tokens)
    {
        var canonical = new StringBuilder().Append(tokens.Theme.Id).Append('\n');
        foreach (var entry in tokens.Colours.OrderBy(c => c.Key, StringComparer.Ordinal))
        {
            Rgb value = entry.Value.Value;
            canonical.Append(FormattableString.Invariant($"{entry.Key}={value.R:R},{value.G:R},{value.B:R}\n"));
        }
        return Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(canonical.ToString())))[..12].ToLowerInvariant();
    }
}
