using System;
using System.Linq;
using Magic600.Chrome;

// ADR-D02 build, track C, item 5 (ADR-D01.41): the rotation-link mark, one copy that Look Lab and the product draw
// from their own link state.
internal static partial class Program
{
    // No Local view: no mark. Linked (the default): the named link in the strong accent. Unlinked: the same glyph with
    // its own word in the dim colour, so the state never rests on colour alone. The glyph and both colours resolve.
    private static void D02PortViewLinkMarksEachState()
    {
        Check(ViewLink.Of(null) == null, "a host with no Local view shows a link mark");
        LinkMark linked = ViewLink.Of(true) ?? throw new Exception("the linked rotation has no mark");
        LinkMark unlinked = ViewLink.Of(false) ?? throw new Exception("the unlinked rotation has no mark");
        Equal(linked, new LinkMark("mark.link", "rotation", "accent.strong", "Rotation linked: one camera turns Global and Local together."));
        Equal(unlinked, new LinkMark("mark.link", "rotation unlinked", "text.dim", "Rotation unlinked: Global and Local each turn with their own camera."));
        // The words name what is linked, one camera's rotation, and not "4D": under the Hopf layout the link is its 3D spin.
        Check(new[] { linked, unlinked }.All(m => !m.Text.Contains("4D") && !m.Tip.Contains("4D") && m.Tip.Contains("Global") && m.Tip.Contains("Local")),
            "the link's words");
        Check(linked.Text != unlinked.Text && linked.Colour != unlinked.Colour, "the two states differ in colour alone");
        Check(IconSet.Parse(Read("data/design/icons.json")).Icons.ContainsKey(ViewLink.Glyph), "the link glyph is not in the icon set");
        foreach (ThemeFamily theme in Themes())
        {
            var tokens = new TokenSet(Design, theme, "compact", 1);
            Check(tokens.Has(linked.Colour) && tokens.Has(unlinked.Colour), "a link colour is missing in " + theme.Id);
        }
    }
}
