using System;
using System.Collections.Generic;
using System.Linq;

namespace Magic600.Chrome;

// ADR-D01.77 and revision 4's token list: arrows, orientation glyphs, set-kind glyphs and block marks keep a meaning
// fixed by name; the number of their values is framework and their drawing is the art line's. Until the art line draws
// them, each value has a neutral placeholder: one plain form per group, and inside a group the value's letter and a
// count of ticks in the list's order. Ticks start at one, so no value reads as zero; a role with no placeholder
// resolves to nothing, never to a default. None of these instruments shows a quantity, so none needs J-3's undefined
// form. The colour is the value's own role, a neutral step in every theme. Nothing in ADR-D02 places them; ADR-D03's
// canvas does (the solving surface and the Library).
public enum PlaceholderForm
{
    Glyph,      // a plain square with the letter, the ticks under it
    Arrow,      // a straight line from source to destination, the square at its head
    Outline,    // a plain rectangle round the region, the square at its top-left corner
}

public sealed record PlaceholderGlyph(string Role, string Group, string Instrument, PlaceholderForm Form, string Letter, int Ticks);

public static class InstrumentPlaceholders
{
    private static readonly (string Group, string Instrument, PlaceholderForm Form, (string Value, string Letter)[] Values)[] Table =
    {
        ("arrow", "arrows", PlaceholderForm.Arrow, new[] { ("state", "S"), ("operation", "O") }),
        ("orientation.glyph", "orientation-glyphs", PlaceholderForm.Glyph,
            new[] { ("trivial", "T"), ("c2", "C2"), ("c5", "C5"), ("d5", "D5"), ("a5", "A5") }),
        ("set.kind", "set-kind-glyphs", PlaceholderForm.Glyph, new[] { ("query", "Q"), ("piece", "P"), ("position", "O") }),
        ("block.mark", "block-marks", PlaceholderForm.Glyph, new[] { ("attained", "A"), ("protected", "P"), ("selected", "S") }),
        ("block.outline", "block-marks", PlaceholderForm.Outline, new[] { ("home", "H"), ("referenced", "R") }),
    };

    private static readonly Dictionary<string, PlaceholderGlyph> byRole = Table
        .SelectMany(g => g.Values.Select((v, i) => new PlaceholderGlyph(g.Group + "." + v.Value, g.Group, g.Instrument, g.Form, v.Letter, i + 1)))
        .ToDictionary(p => p.Role, StringComparer.Ordinal);

    // The groups in revision 4's order, and each group's values in its order.
    public static IReadOnlyList<string> Groups { get; } = Array.AsReadOnly(Table.Select(g => g.Group).ToArray());
    public static IReadOnlyList<string> Values(string group) =>
        Table.Where(g => g.Group == group).SelectMany(g => g.Values.Select(v => v.Value)).ToArray();
    public static IEnumerable<PlaceholderGlyph> All => byRole.Values;

    public static PlaceholderGlyph? Resolve(string role) => byRole.TryGetValue(role, out PlaceholderGlyph? glyph) ? glyph : null;
}
