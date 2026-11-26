using System;
using System.Linq;
using Magic600.Chrome;

// ADR-D02 build step B18c: two drawn views on one surface; the panes' layout and the hit of a window point.
internal static partial class Program
{
    // Global left of Local at 1280×720, with chrome between them (window pixels).
    private static readonly PaneSpec GlobalOpening = new("Global", new PaneRect(212, 64, 560, 470));
    private static readonly PaneSpec LocalOpening = new("Local", new PaneRect(790, 64, 300, 470));

    // The surface is the openings' bounding box, and each pane keeps its opening's size at its offset in it.
    private static void D02PaneLayoutIsBoundingBox()
    {
        PaneLayout layout = ViewPanes.Layout(new[] { GlobalOpening, LocalOpening });
        Check(layout.Surface == new PaneRect(212, 64, 878, 470), "the surface is " + layout.Surface);
        Check(layout.Panes[0] == new PaneSpec("Global", new PaneRect(0, 0, 560, 470)) && layout.Panes[1] == new PaneSpec("Local", new PaneRect(578, 0, 300, 470)),
            "the panes are " + string.Join(", ", layout.Panes));
        // One opening alone: the surface is the opening, as on the one-view path.
        PaneLayout one = ViewPanes.Layout(new[] { GlobalOpening });
        Check(one.Surface == GlobalOpening.Rect && one.Panes.Single().Rect == new PaneRect(0, 0, 560, 470), "one opening's layout is " + one.Surface);
        // Openings at different heights: the box covers both.
        PaneLayout staggered = ViewPanes.Layout(new[] { GlobalOpening, LocalOpening with { Rect = new PaneRect(790, 100, 300, 500) } });
        Check(staggered.Surface == new PaneRect(212, 64, 878, 536) && staggered.Panes[1].Rect == new PaneRect(578, 36, 300, 500), "the staggered layout is " + staggered.Surface);
    }

    // No two panes overlap: overlapping or empty openings are refused, and a split's halves touch without overlapping.
    private static void D02PanesNeverOverlap()
    {
        bool Refused(params PaneSpec[] openings)
        {
            try { ViewPanes.Layout(openings); return false; } catch (ArgumentException) { return true; }
        }
        Check(Refused(GlobalOpening, LocalOpening with { Rect = new PaneRect(700, 64, 300, 470) }), "overlapping openings were laid out");
        Check(Refused(GlobalOpening, LocalOpening with { Rect = new PaneRect(790, 64, 0, 470) }), "an empty opening was laid out");
        Check(!Refused(GlobalOpening, LocalOpening with { Rect = new PaneRect(772, 64, 300, 470) }), "touching openings were refused");
        var halves = ViewPanes.Split(LocalOpening);
        Check(halves.Count == 2 && halves.All(h => h.View == "Local") && !halves[0].Rect.Overlaps(halves[1].Rect)
            && halves[0].Rect.Width + halves[1].Rect.Width == LocalOpening.Rect.Width && halves[1].Rect.X == halves[0].Rect.Right, "the split is " + string.Join(", ", halves));
        PaneLayout split = ViewPanes.Layout(new[] { GlobalOpening }.Concat(halves).ToArray());
        Check(split.Panes.Count == 3, "the split layout has " + split.Panes.Count + " panes");
        for (int i = 0; i < split.Panes.Count; i++)
            for (int j = i + 1; j < split.Panes.Count; j++)
                Check(!split.Panes[i].Rect.Overlaps(split.Panes[j].Rect), "panes " + i + " and " + j + " overlap");
    }

    // A fifth pane is refused; four fit.
    private static void D02FifthPaneRefused()
    {
        PaneSpec[] four = Enumerable.Range(0, 4).Select(i => new PaneSpec(i % 2 == 0 ? "Global" : "Local", new PaneRect(i * 100, 0, 90, 90))).ToArray();
        Check(ViewPanes.Layout(four).Panes.Count == ViewPanes.MaxPanes, "four panes were refused");
        bool refused = false;
        try { ViewPanes.Layout(four.Append(new PaneSpec("Local", new PaneRect(400, 0, 90, 90))).ToArray()); } catch (ArgumentException) { refused = true; }
        Check(refused, "a fifth pane was laid out");
        refused = false;
        try { ViewPanes.Layout(Array.Empty<PaneSpec>()); } catch (ArgumentException) { refused = true; }
        Check(refused, "a layout with no pane was made");
    }

    // A window point resolves to the view it lands in, in surface pixels; chrome between the panes resolves to nothing,
    // and a reply naming another view is not taken.
    private static void D02HitResolvesItsView()
    {
        PaneLayout layout = ViewPanes.Layout(new[] { GlobalOpening, LocalOpening });
        Check(ViewPanes.Hit(layout, 212, 64) == ("Global", 0, 0), "the opening's corner hits " + ViewPanes.Hit(layout, 212, 64));
        Check(ViewPanes.Hit(layout, 771, 533) == ("Global", 559, 469), "Global's last pixel hits " + ViewPanes.Hit(layout, 771, 533));
        Check(ViewPanes.Hit(layout, 800, 100) == ("Local", 588, 36), "a point in Local hits " + ViewPanes.Hit(layout, 800, 100));
        Check(ViewPanes.Hit(layout, 780, 100) == null, "the chrome between the openings hit a pane");
        Check(ViewPanes.Hit(layout, 772, 100) == null && ViewPanes.Hit(layout, 1090, 100) == null && ViewPanes.Hit(layout, 500, 534) == null, "a point outside the panes hit one");
        Check(ViewPanes.Takes("Local", "Local") && !ViewPanes.Takes("Local", "Global"), "a reply naming another view was taken");
    }
}
