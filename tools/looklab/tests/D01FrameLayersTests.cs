using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    // ADR-D01.77: the frame is a canvas in two layers. The surfaces and the instruments are the record's, in its order;
    // every role sits in one layer; the number of an instrument's values is pinned here, so a change is a framework change.
    private static void D01FrameHasTwoLayers()
    {
        FrameLayers layers = Design.Layers;
        Equal(string.Join(" ", layers.Surfaces.Select(p => p.Id)), "shell strip rail top-bar docks view-border key-map help-plate learn empty-state completion",
            "the surfaces of ADR-D01.77, in its order");
        Equal(string.Join(" ", layers.Instruments.Select(p => p.Id)),
            "strip-cells rail-cells status-patterns locks lamps chips route-mark residual-level grip-marks commit-status numerals arrows orientation-glyphs set-kind-glyphs block-marks",
            "the instruments of ADR-D01.77, in its order");

        // A role with no cue is a surface; a role with a cue is an instrument unless a surface names it, and the completion
        // moment is the one surface that does.
        foreach (ColourRole role in Design.Roles.Values)
        {
            FrameLayer layer = layers.Layer(role.Id);
            if (role.Cue == null) Equal(layer, FrameLayer.Surface, role.Id + " has no cue");
            else if (layer == FrameLayer.Surface) Equal(layers.Part(role.Id)?.Id, "completion", role.Id + " is a cued role on a surface");
        }
        Equal(layers.Layer("rail.moment.complete"), FrameLayer.Surface, "the completion moment is for the art line");
        Equal(layers.Layer("surface.window"), FrameLayer.Surface, "the shell");
        Equal(layers.Layer("syntax.keyword"), FrameLayer.Surface, "a syntax colour has no cue");
        Equal(layers.Layer("state.committed"), FrameLayer.Instrument, "a cued state role outside the named instruments");

        var values = layers.Instruments.ToDictionary(p => p.Id, p => p.Roles.Count);
        foreach ((string id, int count) in new[] { ("locks", 3), ("lamps", 3), ("chips", 2), ("route-mark", 2), ("residual-level", 1), ("commit-status", 6), ("numerals", 1) })
            Equal(values[id], count, id + "'s values");
        Equal(string.Join(" ", layers.Instruments.Where(p => p.Roles.Count == 0).Select(p => p.Id)), "grip-marks arrows orientation-glyphs set-kind-glyphs block-marks",
            "the instruments not drawn in this build yet");
        Equal(layers.Part("numeral.undefined")?.Id, "numerals", "revision 6's undefined pattern (J-3)");
        Equal(layers.Part("rail.cell.residual_level")?.Id, "residual-level", "the residual level is its own instrument, not a rail cell");

        Equal(layers.Words("rail.lock.whole"), "Instrument: locks", "an instrument's words");
        Equal(layers.Words("surface.window"), "Surface: the shell", "a surface's words");
        Equal(layers.Words("state.committed"), "Instrument", "a cued role no part names");
        Equal(layers.Words("syntax.keyword"), "Surface", "a plain role no part names");
        foreach (FramePart part in layers.Surfaces)
        {
            Check(part.Roles.Count + part.Styles.Count > 0, part.Id + " has nothing to paint it");
            foreach (string style in part.Styles) Equal(layers.StylePart(style), part, style + " paints one surface");
        }
        foreach (string style in new[] { "KeyMapBody", "HelpPlate", "LearnGround", "EmptyState" }) Check(layers.StylePart(style) != null, style + " is a surface's own style");

        string system = Read("data/design/system.json");
        Refuse(() => DesignSystem.Parse(system.Replace("\"rail.lock.\"", "\"surface.panel\"")), "surface.panel is an instrument's role");
        Refuse(() => DesignSystem.Parse(system.Replace("\"rail.lock.\"", "\"rail.latch.\"")), "rail.latch. names no role");
        Refuse(() => DesignSystem.Parse(system.Replace("\"lamp.\"", "\"rail.lock.\"")), "in two parts of the frame");
        Refuse(() => DesignSystem.Parse(system.Replace("\"KeyMapBody\"", "\"BarTop\"")), "paints two parts of the frame");

        // Recolouring an instrument marks the views; recolouring a surface, the completion moment included, does not.
        ThemeFamily theme = Themes()[0];
        TokenSet Resolve(string? role = null) => new(Design, theme, Design.ShippedDensity, 1, overrides:
            role == null ? null : new Dictionary<string, Oklch> { [role] = new(.6, .15, 30) });
        foreach (string role in new[] { "rail.lock.whole", "state.committed", "numeral.undefined" })
        {
            TokenSet tokens = Resolve(role);
            Check(tokens.NonStandardColours, role + " recoloured leaves the views unmarked");
            Check(ColourMark.BorderMarks(tokens, "Global").Single().Tooltip.StartsWith("Overridden instruments: " + role, StringComparison.Ordinal), "the mark names " + role);
        }
        foreach (string role in new[] { "surface.window", "strip.ground", "rail.moment.complete", "syntax.keyword" })
        {
            TokenSet tokens = Resolve(role);
            Check(!tokens.NonStandardColours && ColourMark.BorderMarks(tokens, "Global").Count == 0, role + " recoloured marks the views");
        }

        // High contrast repaints surfaces only.
        TokenSet plain = Resolve(), contrast = new(Design, theme, Design.ShippedDensity, 1, highContrast: true);
        string[] changed = plain.Colours.Keys.Where(k => plain.Colours[k].Value != contrast.Colours[k].Value).ToArray();
        Check(changed.Length > 0, "high contrast changed nothing");
        Equal(string.Join(" ", changed.Where(k => layers.Layer(k) == FrameLayer.Instrument)), "", "high contrast changed an instrument");
    }
}
