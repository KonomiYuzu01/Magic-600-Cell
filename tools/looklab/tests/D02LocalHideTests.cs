using System;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    // ADR-D01's view.local (the gate's G1, B19): the Local hide closes Local's opening and drops its pane, Global's pane
    // is the same rectangle alone and with Local back, and showing Local again restores the two-pane layout.
    private static void D02LocalHideDropsAndRestoresLocalPane()
    {
        var hide = new LocalHide();
        int changed = 0;
        hide.Changed += () => changed++;
        Check(!hide.Hidden, "Local starts shown");
        Check(ViewPanes.LocalOpen(true, hide.Hidden) && !ViewPanes.LocalOpen(false, hide.Hidden), "open follows the framed cap");
        hide.Toggle();
        Check(hide.Hidden && changed == 1, "the row hides Local");
        Check(!ViewPanes.LocalOpen(true, hide.Hidden), "a hidden Local's opening is closed on a framed cap");
        Equal(ViewPanes.OpeningHeight(400, 900, ViewPanes.LocalOpen(true, hide.Hidden)), 0);
        Check(!hide.Set(true) && changed == 1, "hiding twice changes nothing");
        var openings = new[]
        {
            new PaneSpec("Global", new PaneRect(100, 80, 1200, 900)),
            new PaneSpec("Local", new PaneRect(1320, 80, 400, 400)),
        };
        var both = ViewPanes.Layout(ViewPanes.Shown(openings, false));
        var alone = ViewPanes.Layout(ViewPanes.Shown(openings, true));
        Equal(both.Panes.Count, 2);
        Equal(alone.Panes.Count, 1);
        Equal(alone.Panes[0].View, "Global");
        Equal(alone.Surface, openings[0].Rect);
        hide.Toggle();
        Check(!hide.Hidden && changed == 2, "the row shows Local again");
        var back = ViewPanes.Layout(ViewPanes.Shown(openings, hide.Hidden));
        Equal(back.Surface, both.Surface);
        Check(back.Panes.SequenceEqual(both.Panes), "Local returns to the same two panes");
        Equal(ViewPanes.LocalRow, "view.local");
    }
}
