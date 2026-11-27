using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// ADR-D02.26 and .34: the certified inside mask selects candidate stickers for the cut stripe. Only straddlers
// and known moving sets are piece outlines and references. The certificate-point count stands in for the glyphs.
public enum OverlayLayer { None, CutTrace, MovingSet }

public sealed record OverlayView(OverlayLayer Layer, int? Cap, bool Blocked, string Step, bool[]? Lines, bool[]? NeverHidden,
    long Pieces, long NeverHiddenCount, long? CertificatePoints, bool[]? CutCandidates = null)
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

    public static OverlayView Of(int? cap, string? record, int pieces, int row = 0, int step = 0, string stepLabel = "",
                                 string? hoveredRecord = null)
    {
        if (pieces <= 0) return OverlayView.Empty;
        var named = ReadNamed(cap, record, pieces, row, step, stepLabel);
        if (hoveredRecord == null) return named;
        using var hovered = JsonDocument.Parse(hoveredRecord);
        var lines = Union(named.Lines, Mask(hovered.RootElement, "straddlers", pieces));
        return named with { Lines = lines, NeverHidden = lines, Pieces = Count(lines), NeverHiddenCount = Count(lines) };
    }

    private static OverlayView ReadNamed(int? cap, string? record, int pieces, int row, int step, string stepLabel)
    {
        if (cap is not int c || record == null) return OverlayView.Empty;
        using JsonDocument document = JsonDocument.Parse(record);
        JsonElement root = document.RootElement;
        bool[]? straddlers = Mask(root, "straddlers", pieces), moving = null;
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
        bool[]? cut = row == 0 && straddlers == null ? Mask(root, "inside", pieces) : null;
        bool[]? lines = Union(straddlers, moving);
        var layer = moving != null ? OverlayLayer.MovingSet
            : row == 0 && (straddlers != null || cut != null) ? OverlayLayer.CutTrace : OverlayLayer.None;
        return new OverlayView(layer, c, straddlers != null, row > 0 ? stepLabel : "", lines, lines,
            Count(lines), Count(lines), points, cut);
    }

    private static bool[]? Mask(JsonElement root, string set, int pieces) =>
        root.TryGetProperty("masks", out var masks) && masks.TryGetProperty(set, out var mask)
            ? JumbleTint.Mask(mask, pieces) : null;

    // Builds the line mask in a separate array from the piece tint.
    public static byte[]? Outline(byte[]? tint, bool[]? lines)
    {
        if (lines == null) return tint;
        byte[] outline = tint != null && tint.Length == lines.Length ? (byte[])tint.Clone() : new byte[lines.Length];
        for (int p = 0; p < lines.Length; p++) if (lines[p]) outline[p] = Entry;
        return outline;
    }

    // The bound cap record supplies the straddlers of a blocked cut, or the inside set of an admissible one.
    public static IEnumerable<int> PiecesOfCap(int cap, string? record, int pieceCount)
    {
        var view = Of(cap, record, pieceCount);
        if ((view.Lines ?? view.CutCandidates) is not { } mask) yield break;
        for (int piece = 0; piece < mask.Length; piece++) if (mask[piece]) yield return piece;
    }

    // Rule 5 references. Cut candidates keep the pane's ordinary focus treatment.
    public static Func<int, bool>? BreaksThrough(OverlayView view)
    {
        return view.NeverHidden is { } never ? piece => piece >= 0 && piece < never.Length && never[piece] : null;
    }

    public static long Hidden(bool[]? lines, FocusLayers? layers, Func<int, bool>? member,
                              byte[]? preview = null, Func<int, bool>? conflict = null)
    {
        if (layers is not { Dimming: true, Inactive: InactiveMode.Hidden } || member == null) return 0;
        long hidden = 0;
        for (int p = 0; p < (lines?.Length ?? preview?.Length ?? 0); p++)
            if ((lines?[p] == true || preview?[p] > 0) && !member(p) && conflict?.Invoke(p) != true) hidden++;
        return hidden;
    }

    // The overlay's line on both view borders, empty with no overlay.
    public static string Words(OverlayView view, long hidden)
    {
        string glyphs = view.CertificatePoints is long n
            ? " · " + N(n) + " certificate points (glyphs wait for their gate row)" : "";
        string kept = hidden > 0 ? N(hidden) + " kept from the filter, outlined" : "";
        string line = view.Layer switch
        {
            OverlayLayer.MovingSet => view.Step + " · moving set outlined, never hidden · " + N(view.Pieces) + " pieces",
            OverlayLayer.CutTrace when view.Blocked => "Cut of cap " + N(view.Cap!.Value) + " · straddlers outlined, never hidden · " + N(view.Pieces) + " pieces",
            OverlayLayer.CutTrace => "Cut of cap " + N(view.Cap!.Value) + " · surface trace",
            _ when view.Step.Length > 0 => view.Step + " · " + RepairSteps.MovingUnavailable,
            _ when view.Pieces > 0 => N(view.Pieces) + " straddlers outlined, never hidden",
            _ => "",
        };
        if (kept.Length > 0) line = line.Length > 0 ? line + " · " + kept : kept;
        return line + glyphs;
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
