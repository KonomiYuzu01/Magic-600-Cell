namespace Magic600.Chrome;

// A mark on a view's frame: its glyph, its word, its colour token and its tooltip.
public sealed record LinkMark(string Glyph, string Text, string Colour, string Tip);

// ADR-D01.41: the rotation of Global and Local is linked by default and can be unlinked; a camera link is named and
// marked on both frames. One copy for Look Lab and the product (ADR-D02 build, track C, item 5); the mark was Look
// Lab's own before, so the product's frames did not name the link.
// What the link joins is one camera: a turn moves both views. Global shows the camera's 4D rotation, or its 3D spin
// while it is laid out as the Hopf rings; Local keeps its cap centred and turns with the spin. So the words name the
// rotation and the camera, not "4D".
public static partial class ViewLink
{
    public const string Glyph = "mark.link";
    public const string Linked = "rotation", Unlinked = "rotation unlinked";

    // The mark for a host's link state: none while there is no Local view (nothing is linked), the named link while
    // linked (the default), and the same glyph with its word in the dim colour while unlinked, never colour alone.
    public static LinkMark? Of(bool? linked) => linked switch
    {
        null => null,
        true => new LinkMark(Glyph, Linked, "accent.strong", "Rotation linked: one camera turns Global and Local together."),
        false => new LinkMark(Glyph, Unlinked, "text.dim", "Rotation unlinked: Global and Local each turn with their own camera."),
    };
}
