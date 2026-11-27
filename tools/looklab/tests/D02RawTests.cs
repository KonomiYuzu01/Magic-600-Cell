using System;
using System.Linq;
using Magic600.Chrome;

// ADR-D02.18 (build step B21): the raw view's pure model.
internal static partial class Program
{
    // The caption is shown whenever the raw view is on, with or without the antipode chart, at any clip; it is the words
    // file's sentence raw.caption.
    private static void D02RawCaptionAlwaysShown()
    {
        var raw = new RawView();
        Equal(RawView.Caption, Sentence("raw.caption"), "the caption's words");
        Check(raw.Marks().Count == 0 && raw.Line(150) == "", "the raw view is off but marks or a line were shown");
        raw.Toggle();
        foreach (var h in new[] { RawView.DefaultH, RawView.MeetingTick, 0.0 })
        {
            raw.SetClip(h);
            foreach (var antipode in new[] { false, true })
            {
                if (raw.Antipode != antipode) raw.ToggleAntipode();
                Check(raw.Marks().Count(m => m.Words == RawView.Caption) == 1, $"no caption mark at h {h}, antipode {antipode}");
                Check(raw.Line(150).EndsWith(RawView.Caption, StringComparison.Ordinal), $"Local's line lacks the caption at h {h}: {raw.Line(150)}");
            }
        }
        Check(raw.Line(150).Contains("all 80 meeting poles inside"), "the meeting poles' tick is not read at h 0");
        raw.Toggle();
        Check(!raw.On && !raw.Antipode && raw.Marks().Count == 0, "off did not drop the antipode chart and the caption");
    }

    // The antipode chart is labelled with the built word raw.antipode, and nothing the raw view shows says "mirror".
    private static void D02AntipodeNeverSaysMirrored()
    {
        var raw = new RawView();
        Equal(RawView.AntipodeLabel, Sentence("raw.antipode"), "the antipode's label");
        Check(raw.ToggleAntipode() && raw.On, "the antipode's key did not turn the raw view on with the antipode chart");
        var panes = raw.Panes(new PaneSpec("Local", new PaneRect(0, 0, 400, 300)));
        Equal(panes[1].Label, "antipode", "the right chart's label");
        var shown = panes.Select(p => p.Label).Concat(raw.Marks().SelectMany(m => new[] { m.Words, m.Tooltip })).Append(raw.Line(150));
        Check(shown.All(s => !s.Contains("mirror", StringComparison.OrdinalIgnoreCase)), "the raw view says mirror");
        Check(raw.Marks().Any(m => m.Words == "antipode"), "the antipode chart's mark is missing");
    }

    // The antipode chart shares the clip and the handedness: both panes carry the one h, -I has determinant +1, and at
    // h = 0 each anchor goes to exactly one chart (half-open at 0, which the chart keeps).
    private static void D02AntipodeSharesClipAndHandedness()
    {
        var raw = new RawView();
        raw.ToggleAntipode();
        var local = new PaneSpec("Local", new PaneRect(10, 20, 301, 200));
        raw.SetClip(0.5);
        var panes = raw.Panes(local);
        Check(panes.Count == 2 && !panes[0].Antipode && panes[1].Antipode, "the antipode chart is not the right of two panes");
        Check(panes.All(p => p.H == 0.5 && p.Pane.View == "Local"), "the charts do not share the clip or Local's view");
        Equal(panes[0].Pane.Rect.Width + panes[1].Pane.Rect.Width, 301, "the charts fill Local's opening");
        Equal(RawView.AntipodeDeterminant, 1, "the determinant of -I in R4");
        foreach (var dot in new[] { -1.0, -0.3, -1e-9, 0.0, 1e-9, 0.3, 1.0 })
            Check(RawView.Drawn(dot, 0, false) != RawView.Drawn(dot, 0, true), $"at h = 0 the anchor dot {dot} is not drawn in exactly one chart");
        Check(RawView.Drawn(0, 0, false) && !RawView.Drawn(0, 0, true), "an anchor on the equator is not the chart's");
        Check(!RawView.Drawn(0.4, 0.5, false) && !RawView.Drawn(-0.4, 0.5, true), "a chart drew past its clip");
        Equal(raw.SetClip(-0.2), 0.0, "the clip went past 90 degrees");
        Equal(raw.SetClip(0.99), RawView.DefaultH, "the clip went inside the named cap's cut");
        Equal(raw.Widen(-1), RawView.DefaultH, "narrowing went inside the cut");
        Equal(RawView.ParseClip("tick"), RawView.MeetingTick, "the tick argument");
        Check(RawView.ParseClip("1.2") == null && RawView.ParseClip("x") == null && RawView.ParseClip("0.25") == 0.25, "the clip's argument");
        Check(Math.Abs(RawView.MeetingTick - Math.Cos(49.1176 * Math.PI / 180)) < 1e-5, "the meeting poles' tick is not at 49.12 degrees");
    }
}
