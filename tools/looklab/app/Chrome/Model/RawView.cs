using System;
using System.Collections.Generic;
using System.Globalization;

namespace Magic600.Chrome;

// One drawn chart of the raw view: Local's pane, whether it is the antipode chart, the shared clip and its label.
public sealed record RawPane(PaneSpec Pane, bool Antipode, double H, string Label);

// ADR-D02.18 (build step B21): the raw view, Local's cap-centred stereographic mode (view.mode.raw). Its clip is the
// half-space x·p̂ ≥ h about the named cap's pole p̂; the default h is exactly the named cap's cut (Local's cap test,
// 121/125), and h = 0 is 90°, the furthest the clip goes. One key adds the antipode chart beside it: the pole chart after
// -I, a proper rotation of R4, so it shares the clip and the handedness; at h = 0 each sticker goes to one chart by its
// anchor, half-open. The core draws both charts (SA2_PROJECTION_STEREOGRAPHIC, SA2_PANE_ANTIPODE); this is the model.
public sealed class RawView
{
    public static (float Cell, float Sticker) Shrinks(bool raw, float gap, float sticker) =>
        raw ? (1, 1) : (1 - gap, sticker);

    public const double DefaultH = 121.0 / 125.0, ClipStep = 0.02;
    // The static tick (no detent): at h ≤ φ²/4 = cos 49.12° all 80 meeting poles of the named cap lie inside the clip.
    // The 80 meeting caps are the first six shells of cell centres about a pole (4, 12, 24, 12, 4 and 24 cells), and the
    // sixth lies at cos (3 + √5)/8; the next shell (cos 0.559) holds no meeting cap.
    public static readonly double MeetingTick = (3 + Math.Sqrt(5)) / 8;
    public const int MeetingPoles = 80;
    // jumble-words.json: the built word raw.antipode and the sentence raw.caption.
    public const string AntipodeLabel = "antipode";
    public const string Caption = "No raw view certifies no holes; N = 0 does.";
    public const string CaptionGlyph = "view.names", AntipodeGlyph = "mark.link";
    // CR-14's draft rows. The keys are placeholders free in the keymap (row requests).
    public const string Row = "view.mode.raw", AntipodeRow = "view.mode.raw.antipode", ClipRow = "view.mode.raw.clip";
    public static readonly IReadOnlySet<string> Rows = new HashSet<string>(StringComparer.Ordinal) { Row, AntipodeRow, ClipRow };
    public const string RawKey = "Home", AntipodeKey = "End", NarrowKey = "PageUp", WidenKey = "PageDown";

    public bool On { get; private set; }
    public bool Antipode { get; private set; }
    public double H { get; private set; } = DefaultH;

    // The raw view's key or row: on or off; off drops the antipode chart.
    public bool Toggle()
    {
        On = !On;
        if (!On) Antipode = false;
        return On;
    }

    // The antipode's key or row: adds or removes the antipode chart, and turns the raw view on with it.
    public bool ToggleAntipode()
    {
        Antipode = !On || !Antipode;
        On = true;
        return Antipode;
    }

    // The clip control: h from 0 (90°) up to the named cap's cut, never past 90°, never inside the cut (Local is never
    // empty: the named cap's region is always drawn).
    public double SetClip(double h)
    {
        H = Math.Round(Math.Clamp(h, 0, DefaultH), 6);
        return H;
    }

    // Widen (h down) or narrow (h up) by steps.
    public double Widen(int steps) => SetClip(H - steps * ClipStep);

    // The clip row's argument: a number h, or "cut" (the default) or "tick" (the meeting poles' tick).
    public static double? ParseClip(string? argument) => argument?.Trim() switch
    {
        "cut" => DefaultH,
        "tick" => MeetingTick,
        { } text when double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out var h) && h >= 0 && h <= DefaultH => h,
        _ => null,
    };

    // Whether a sticker whose anchor has dot product anchorDot with the pole is drawn: the chart keeps anchorDot ≥ h, the
    // antipode chart -anchorDot > h (half-open, so at h = 0 each sticker is in exactly one chart). The core's test.
    public static bool Drawn(double anchorDot, double h, bool antipode) => antipode ? -anchorDot > h : anchorDot >= h;

    // The determinant of the map the antipode chart applies before the chart: -I in R4, a proper rotation (+1), so the
    // two charts share the handedness. The core checks the charts' Jacobian signs at build.
    public static int AntipodeDeterminant
    {
        get
        {
            var d = 1;
            for (var i = 0; i < 4; i++) d *= -1;
            return d;
        }
    }

    // Local's charts: one, or with the antipode two side by side (ViewPanes.Split): the left the chart, the right the
    // antipode chart, both with the one clip.
    public IReadOnlyList<RawPane> Panes(PaneSpec local)
    {
        if (!Antipode) return new[] { new RawPane(local, false, H, "") };
        var halves = ViewPanes.Split(local);
        return new[] { new RawPane(halves[0], false, H, ""), new RawPane(halves[1], true, H, AntipodeLabel) };
    }

    // Local's border marks while the raw view is on: the caption always, and the antipode chart's label.
    public IReadOnlyList<ViewMark> Marks()
    {
        if (!On) return Array.Empty<ViewMark>();
        var caption = new ViewMark(CaptionGlyph, Caption, "text.dim", "A chart can hide a hole at its clip or its rim; only N = 0 certifies none");
        return Antipode
            ? new[] { caption, new ViewMark(AntipodeGlyph, AntipodeLabel, "text.dim", "The right chart is the left chart's pole chart after -I, a rotation: same clip, same handedness") }
            : new[] { caption };
    }

    // The clip in degrees about the pole.
    public double Degrees => Math.Acos(H) * 180 / Math.PI;

    // Local's line: the chart, the clip, the tick, the keys and the caption.
    public string Line(int? cap)
    {
        if (!On || cap is not int c) return "";
        var tick = H <= MeetingTick + 1e-9 ? "all 80 meeting poles inside"
            : string.Create(CultureInfo.InvariantCulture, $"tick at h {MeetingTick:0.000}: all 80 meeting poles inside");
        var keys = (Antipode ? AntipodeKey + " removes the antipode chart" : AntipodeKey + " adds the antipode chart") + ", " + NarrowKey + "/" + WidenKey + " the clip";
        return string.Create(CultureInfo.InvariantCulture, $"Raw view at cap {c}: stereographic from its pole · clip h {H:0.000} ({Degrees:0.0}°) · {tick} · {keys} · {Caption}");
    }
}
