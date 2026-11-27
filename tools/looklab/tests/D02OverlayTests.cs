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

}
