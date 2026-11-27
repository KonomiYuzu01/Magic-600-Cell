using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    // ADR-D01.77's role-less instruments are registered with revision 4's values and neutral placeholders: each of the
    // five groups has exactly revision 4's values, every value resolves to a placeholder, two values of one group never
    // draw the same, and every value's colour is a neutral step in every theme.
    private static void D02InstrumentPlaceholders()
    {
        var groups = new (string Group, string Instrument, string Values)[]
        {
            ("arrow", "arrows", "state operation"),
            ("orientation.glyph", "orientation-glyphs", "trivial c2 c5 d5 a5"),
            ("set.kind", "set-kind-glyphs", "query piece position"),
            ("block.mark", "block-marks", "attained protected selected"),
            ("block.outline", "block-marks", "home referenced"),
        };
        Equal(string.Join(" ", InstrumentPlaceholders.Groups), string.Join(" ", groups.Select(g => g.Group)), "revision 4's five groups, in order");
        FrameLayers layers = Design.Layers;
        ThemeFamily[] themes = Themes();
        Check(themes.Length >= 2, "both themes");
        foreach ((string group, string instrument, string values) in groups)
        {
            string[] expected = values.Split(' ');
            string Sorted(IEnumerable<string> names) => string.Join(" ", names.OrderBy(n => n, StringComparer.Ordinal));
            Equal(Sorted(Design.Roles.Keys.Where(r => r.StartsWith(group + ".", StringComparison.Ordinal))), Sorted(expected.Select(v => group + "." + v)),
                group + "'s roles are revision 4's values");
            Equal(string.Join(" ", InstrumentPlaceholders.Values(group)), values, group + "'s placeholders in revision 4's order");
            var drawings = new HashSet<(PlaceholderForm, string, int)>();
            var ticks = new HashSet<int>();
            var forms = new HashSet<PlaceholderForm>();
            foreach (string value in expected)
            {
                string role = group + "." + value;
                Equal(layers.Part(role)?.Id, instrument, role + "'s instrument");
                PlaceholderGlyph glyph = InstrumentPlaceholders.Resolve(role) ?? throw new Exception(role + " has no placeholder drawing");
                Equal(glyph.Instrument, instrument, role + "'s placeholder instrument");
                Check(glyph.Ticks >= 1 && glyph.Letter.Trim().Length > 0, role + "'s placeholder is blank or reads as zero");
                Check(drawings.Add((glyph.Form, glyph.Letter, glyph.Ticks)) && ticks.Add(glyph.Ticks), role + " draws like another value of " + group);
                forms.Add(glyph.Form);
                Check(Design.Roles[role].Cue?.StartsWith("placeholder", StringComparison.Ordinal) == true, role + "'s cue does not say it is a placeholder");
                foreach (ThemeFamily theme in themes)
                    Equal(theme.Roles[role].Ramp, "neutral", role + " in " + theme.Id + " is not a neutral step");
            }
            Equal(forms.Count, 1, group + " has one placeholder form");
        }
        Equal(InstrumentPlaceholders.All.Count(), 15, "the placeholders are the five groups' values and no more");
        Equal(string.Join(" ", layers.Instruments.Where(p => p.Roles.Count == 0).Select(p => p.Id)), "grip-marks", "the instruments still without roles");
        foreach (string role in new[] { "arrow.unknown", "set.kind", "rail.lock.whole", "numeral.undefined" })
            Check(InstrumentPlaceholders.Resolve(role) == null, role + " resolves to a default placeholder");
    }
}
