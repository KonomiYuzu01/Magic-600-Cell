using System;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

// B16 (ADR-D02.26, .34; jumbling-observation.md rule 5): the overlays as lines on the outline channel, from the sample
// records of wj-ia-300: the named cap's trace, a focused step's moving set in its place, a blocked cap's straddlers
// never hidden with the certificate points counted in place of their glyphs, and the exact count of overlay pieces a
// focus layer would hide, which show outlined instead.
internal static partial class Program
{
    private const int OverlayPieces = 177_120;

    private static bool[] OverlayMask(string record, Func<JsonElement, JsonElement> pick)
    {
        using var document = JsonDocument.Parse(record);
        return JumbleTint.Mask(pick(document.RootElement), OverlayPieces);
    }

    private static void D02OverlayTraceAtANamedCapOnly()
    {
        string open = Read("data/samples/jumble/wj-ia-300/cap-150.json");
        Equal(JumbleOverlay.Of(null, open, OverlayPieces), OverlayView.Empty, "an overlay with no cap named");
        Equal(JumbleOverlay.Words(OverlayView.Empty, 0), "", "the line with no cap named");
        OverlayView trace = JumbleOverlay.Of(150, open, OverlayPieces);
        bool[] inside = OverlayMask(open, r => r.GetProperty("masks").GetProperty("inside"));
        Check(trace.Layer == OverlayLayer.CutTrace && !trace.Blocked && trace.NeverHidden == null, "cap 150's trace layer");
        Check(trace.Lines!.SequenceEqual(inside), "the trace is not the inside set's outline");
        Equal(trace.Pieces, inside.LongCount(b => b), "the trace's pieces");
        Equal(JumbleOverlay.Words(trace, 0), "Cut of cap 150 · inside set outlined · " + trace.Pieces.ToString("N0", System.Globalization.CultureInfo.InvariantCulture) + " pieces", "the trace's line");
        // The lines go over the tint on the overlay's entry; the rest of the tint stays.
        var tint = Enumerable.Repeat((byte)3, OverlayPieces).ToArray();
        byte[] outline = JumbleOverlay.Outline(tint, trace.Lines)!;
        Check(Enumerable.Range(0, OverlayPieces).All(p => outline[p] == (inside[p] ? JumbleOverlay.Entry : 3)), "the outline bytes");
        Check(tint.All(b => b == 3), "the outline changed the tint's bytes");
        Equal(JumbleOverlay.Outline(null, null), (byte[]?)null, "the outline with neither");
    }

    private static void D02OverlayStepMovingSetReplacesTheTrace()
    {
        string open = Read("data/samples/jumble/wj-ia-300/cap-150.json");
        for (int k = 1; k <= 3; k++)
        {
            OverlayView step = JumbleOverlay.Of(150, open, OverlayPieces, 1, k, RepairScrub.StepLabel(k, 3));
            using var document = JsonDocument.Parse(open);
            var sets = document.RootElement.GetProperty("repair_steps")[0].GetProperty("moving_sets");
            if (k > sets.GetArrayLength())
            {
                Check(step.Layer == OverlayLayer.CutTrace, "a step the sample lacks does not fall back to the trace");
                continue;
            }
            bool[] moving = JumbleTint.Mask(sets[k - 1], OverlayPieces);
            Check(step.Layer == OverlayLayer.MovingSet && step.Lines!.SequenceEqual(moving), "step " + k + "'s moving set");
            Check(step.NeverHidden!.SequenceEqual(moving), "step " + k + "'s moving set is not never-hidden");
            Check(JumbleOverlay.Words(step, 0).StartsWith("Repair " + k + " of 3 · moving set outlined, never hidden · ", StringComparison.Ordinal), "step " + k + "'s line");
        }
    }

    private static void D02OverlayBlockedCapKeepsStraddlersAndCountsPoints()
    {
        string blocked = Read("data/samples/jumble/wj-ia-300/cap-0.json");
        OverlayView view = JumbleOverlay.Of(0, blocked, OverlayPieces);
        bool[] straddlers = OverlayMask(blocked, r => r.GetProperty("masks").GetProperty("straddlers"));
        Check(view.Layer == OverlayLayer.CutTrace && view.Blocked, "cap 0's trace");
        Check(view.Lines!.SequenceEqual(straddlers) && view.NeverHidden!.SequenceEqual(straddlers), "the straddlers are not the trace and never hidden");
        Equal(view.Pieces, 231L, "the straddlers");
        Equal(view.CertificatePoints, (long?)462, "the certificate points");
        Equal(JumbleOverlay.Words(view, 0), "Cut of cap 0 · straddlers outlined, never hidden · 231 pieces · 462 certificate points (glyphs wait for their gate row)", "cap 0's line");
    }

    private static void D02OverlayHiddenCountIsExactAndNothingHides()
    {
        OverlayView trace = JumbleOverlay.Of(150, Read("data/samples/jumble/wj-ia-300/cap-150.json"), OverlayPieces);
        var layers = new FocusLayers();
        layers.AddPredicate(new PredicateLayer("test.even", "even", Numeral.Of(OverlayPieces / 2), true));
        layers.Focus("test.even");
        Func<int, bool> member = p => p % 2 == 0;
        Equal(JumbleOverlay.Hidden(trace.Lines, layers, member), 0L, "a hidden count while the layer does not dim");
        foreach (InactiveMode mode in new[] { InactiveMode.Dimmed, InactiveMode.Hidden })
        {
            layers.Cycle();
            Equal(layers.Inactive, mode, "the cycle");
            long odd = Enumerable.Range(0, OverlayPieces).LongCount(p => trace.Lines![p] && p % 2 == 1);
            Equal(JumbleOverlay.Hidden(trace.Lines, layers, member), odd, "the hidden count under " + mode);
            Func<int, bool> keep = JumbleOverlay.BreaksThrough(trace)!;
            Check(Enumerable.Range(0, OverlayPieces).Where(p => trace.Lines![p]).All(p => layers.ShownPiece(member(p), keep(p)) == 1), "an overlay piece hides under " + mode);
            Check(JumbleOverlay.Words(trace, odd).EndsWith(" · " + odd.ToString("N0", System.Globalization.CultureInfo.InvariantCulture) + " kept from the filter, outlined", StringComparison.Ordinal), "the line's hidden count");
        }
        Equal(JumbleOverlay.BreaksThrough(OverlayView.Empty), (Func<int, bool>?)null, "a breakthrough with no overlay");
    }
}
