using System;

namespace Magic600.Chrome;

// ADR-D01.41's link mark drawn on the frames (the words are in Model/ViewLink.cs). The host passes its link state:
// null while it draws no Local view, true while the rotation is linked, false while it is unlinked. A host calls it
// once per describe, after it cleared its marks, so the mark shows once.
public static partial class ViewLink
{
    // The link's mark on one view's frame.
    public static void Mark(ViewFrame view, Func<bool?> linked)
    {
        if (Of(linked()) is { } mark) view.AddMark(mark.Glyph, mark.Text, mark.Colour, mark.Tip);
    }

    // The same mark on both main frames: a link is named on each view it joins.
    public static void Mark(ChromeFrame frame, Func<bool?> linked)
    {
        Mark(frame.Global, linked);
        Mark(frame.Local, linked);
    }
}
