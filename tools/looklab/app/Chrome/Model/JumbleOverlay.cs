using System;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// B16 (ADR-D02.26, .34; jumbling-observation.md rule 5): the Jumble overlays as lines. They reuse the core's one line
// drawing, the outline channel of B14's style path, under the selection's and the hover's outlines and over the tint:
// - while a cap is named, its cut as its trace at piece resolution: at an admissible cap the inside set's outline,
//   whose border is the cut; at a blocked cap the straddlers, which lie across it;
// - while a repair row is focused, its step k's moving set in place of the trace (ADR-D02.34: one step at a time);
// - the never-hidden set (rule 5, items 1 and 2): a blocked named cap's straddlers and the focused step's moving set;
//   every overlay piece a focus layer would hide shows instead, outlined, with its exact count (item 4);
// - certificate-point glyphs stay off until their gate row and the owner's acceptance; their count shows in their place.
// The default cap Local draws while none is named is not a named cap and draws no overlay (ADR-D02.12).
public enum OverlayLayer { None, CutTrace, MovingSet }

public sealed record OverlayView(OverlayLayer Layer, int? Cap, bool Blocked, string Step, bool[]? Lines, bool[]? NeverHidden,
    long Pieces, long NeverHiddenCount, long? CertificatePoints)
{
    public static readonly OverlayView Empty = new(OverlayLayer.None, null, false, "", null, null, 0, 0, null);
}

public static class JumbleOverlay
{
    // The overlay's palette entry on the outline channel (the tint's plan leaves 15 free) and its colour's design slot.
    // A slot of its own, `line.jumble.overlay`, is requested; until then it takes the preview state's colour.
    public const byte Entry = 15;
    public const string Slot = "state.preview";
    public const string LineName = "JumbleOverlayLine";

    // The overlay a named cap's record gives: the focused step's moving set (row and step from 1), else the cut trace.
    // A record without masks, or a step the sample lacks, gives the trace or nothing.
    public static OverlayView Of(int? cap, string? record, int pieces, int row = 0, int step = 0, string stepLabel = "")
    {
        if (cap is not int c || record == null || pieces <= 0) return OverlayView.Empty;
        using JsonDocument document = JsonDocument.Parse(record);
        JsonElement root = document.RootElement;
        bool[]? straddlers = null, inside = null, moving = null;
        if (root.TryGetProperty("masks", out JsonElement masks))
        {
            if (masks.TryGetProperty("straddlers", out JsonElement s)) straddlers = JumbleTint.Mask(s, pieces);
            if (masks.TryGetProperty("inside", out JsonElement i)) inside = JumbleTint.Mask(i, pieces);
        }
        long? points = root.TryGetProperty("straddlers", out JsonElement st) && st.TryGetProperty("certificate_points", out JsonElement cp)
            ? cp.GetInt64() : null;
        if (row > 0 && step > 0 && root.TryGetProperty("repair_steps", out JsonElement steps) && steps.ValueKind == JsonValueKind.Array)
        {
            foreach (JsonElement s in steps.EnumerateArray())
            {
                if (s.GetProperty("row").GetInt32() != row || !s.TryGetProperty("moving_sets", out JsonElement sets)) continue;
                if (sets.ValueKind == JsonValueKind.Array && step <= sets.GetArrayLength()) moving = JumbleTint.Mask(sets[step - 1], pieces);
            }
        }
        bool[]? lines = moving ?? straddlers ?? inside;
        if (lines == null) return OverlayView.Empty with { Cap = c, Blocked = straddlers != null, CertificatePoints = points };
        bool[]? never = Union(straddlers, moving);
        return new OverlayView(moving != null ? OverlayLayer.MovingSet : OverlayLayer.CutTrace, c, straddlers != null,
            moving != null ? stepLabel : "", lines, never, Count(lines), Count(never), points);
    }

    // The outline channel's bytes: the overlay's entry on its pieces over the tint's (null for no tint); null when
    // neither draws anything. The selection's and the hover's outlines go on top afterwards.
    public static byte[]? Outline(byte[]? tint, bool[]? lines)
    {
        if (lines == null) return tint;
        byte[] outline = tint != null && tint.Length == lines.Length ? (byte[])tint.Clone() : new byte[lines.Length];
        for (int p = 0; p < lines.Length; p++) if (lines[p]) outline[p] = Entry;
        return outline;
    }

    // Rule 5: a piece no filter hides, the never-hidden set and the overlay's own pieces (which then show outlined);
    // null when there is none.
    public static Func<int, bool>? BreaksThrough(OverlayView view)
    {
        bool[]? lines = view.Lines, never = view.NeverHidden;
        if (lines == null && never == null) return null;
        return piece => (lines != null && piece < lines.Length && lines[piece]) || (never != null && piece < never.Length && never[piece]);
    }

    // The overlay pieces the focus layer would hide or dim without rule 5, counted exactly; 0 while it does not dim.
    public static long Hidden(bool[]? lines, FocusLayers? layers, Func<int, bool>? member)
    {
        if (lines == null || layers is not { Dimming: true } || member == null) return 0;
        long hidden = 0;
        for (int p = 0; p < lines.Length; p++) if (lines[p] && layers.ShownPiece(member(p)) < 1) hidden++;
        return hidden;
    }

    // The overlay's line on both view borders, empty with no overlay.
    public static string Words(OverlayView view, long hidden)
    {
        if (view.Cap is not int cap) return "";
        string glyphs = view.CertificatePoints is long n
            ? " · " + N(n) + " certificate points (glyphs wait for their gate row)" : "";
        string kept = hidden > 0 ? " · " + N(hidden) + " kept from the filter, outlined" : "";
        return view.Layer switch
        {
            OverlayLayer.MovingSet => view.Step + " · moving set outlined, never hidden · " + N(view.Pieces) + " pieces" + kept + glyphs,
            OverlayLayer.CutTrace when view.Blocked => "Cut of cap " + N(cap) + " · straddlers outlined, never hidden · " + N(view.Pieces) + " pieces" + kept + glyphs,
            OverlayLayer.CutTrace => "Cut of cap " + N(cap) + " · inside set outlined · " + N(view.Pieces) + " pieces" + kept + glyphs,
            _ => glyphs.Length > 0 ? "Cut of cap " + N(cap) + glyphs : "",
        };
    }

    private static bool[]? Union(bool[]? a, bool[]? b)
    {
        if (a == null) return b;
        if (b == null) return a;
        var union = new bool[Math.Max(a.Length, b.Length)];
        for (int p = 0; p < union.Length; p++) union[p] = (p < a.Length && a[p]) || (p < b.Length && b[p]);
        return union;
    }

    private static long Count(bool[]? set) => set?.LongCount(b => b) ?? 0;

    private static string N(long n) => n.ToString("N0", CultureInfo.InvariantCulture);
}
