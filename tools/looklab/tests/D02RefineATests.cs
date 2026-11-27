using System;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    private static string RefineCap(int cap) => Read($"data/samples/jumble/wj-ia-300/cap-{cap}.json");

    private static void D02AdmissibleCutKeepsTintAndFocus()
    {
        string record = RefineCap(150);
        Equal(JumbleOverlay.Of(null, record, OverlayPieces), OverlayView.Empty, "the default cap has no trace");
        var view = JumbleOverlay.Of(150, record, OverlayPieces);
        Equal(view.Layer, OverlayLayer.CutTrace);
        Equal(JumbleOverlay.Words(view, 0), "Cut of cap 150 · surface trace");
        Equal(view.CutCandidates!.Count(x => x), 1909);
        Check(view.Lines == null && JumbleOverlay.BreaksThrough(view) == null, "inside pieces are neither reference lines nor kept");

        var ink = JumbleStyle.Ink(line: false, cut: true, preview: false, markedPreview: false);
        foreach (byte tint in new byte[] { 3, 4, JumbleTint.OffLatticeEntry })
        {
            var style = JumbleStyle.Encode(tint, 0, false, ink, true, true);
            Equal(style.Outline, tint, "a cut candidate retains its tint");
            Equal(style.Fill, (byte)0x89, "both focus bits and the trace candidate");
            Check(!JumbleStyle.OutlineOnly(ink, InactiveMode.Hidden, true), "a hidden inside piece must not draw");
        }
        var height = JumbleStyle.Encode(0, JumbleTint.FirstHeight, true, ink, true, false);
        Equal(JumbleStyle.Decode(height.Outline, height.Fill, true), ((byte)0, JumbleTint.FirstHeight, OverlayInk.Cut));
    }

    private static void D02PreviewCollateralBreaksThroughHide()
    {
        var layers = new FocusLayers();
        layers.AddPredicate("test", "Empty", Numeral.Of(0));
        layers.Focus("test");
        layers.Cycle();
        layers.Cycle();
        var moved = new byte[] { 0, 1, 1, 0 };
        Equal(JumbleOverlay.Hidden(null, layers, _ => false, moved), 2L, "only the previewed pieces are references");
        foreach (bool marked in new[] { false, true })
        {
            var ink = JumbleStyle.Ink(false, false, moved[1] > 0, marked);
            var style = JumbleStyle.Encode(4, 0, false, ink, true, false);
            Equal(JumbleStyle.Decode(style.Outline, style.Fill, false), ((byte)4, (byte)0, ink));
            Check(JumbleStyle.OutlineOnly(ink, InactiveMode.Hidden, true), "excluded preview collateral draws a line");
            Check(!JumbleStyle.OutlineOnly(ink, InactiveMode.Dimmed, true), "Dim keeps the full reference");
        }
    }

    private static void D02MissingRepairStepShowsUnavailable()
    {
        string record = RefineCap(150);
        var row = RepairListModel.From(record, 300)!.Rows[9];
        Equal(RepairSteps.From(record).MovingWords(row, 2), "moving set unavailable");
        string label = RepairScrub.StepLabel(2, row.Steps);
        var view = JumbleOverlay.Of(150, record, OverlayPieces, row.Number, 2, label);
        Equal(JumbleOverlay.Words(view, 0), label + " · moving set unavailable");
        Check(view.Layer == OverlayLayer.None && view.CutCandidates == null && view.Lines == null,
            "an unavailable step leaves neither a moving overlay nor a cap substitute");
    }

    private static void D02HoveredBlockedCapKeepsItsStraddlers()
    {
        var naming = new CapNaming();
        naming.Name(150);
        OverlayView view = OverlayView.Empty;
        string blocked = RefineCap(0), open = RefineCap(150);
        naming.HoveredChanged += cap => view = JumbleOverlay.Of(naming.Named, open, OverlayPieces, hoveredRecord: cap == 0 ? blocked : null);
        naming.Hover(0);
        Equal(naming.Named, (int?)150, "hover preserves the named cap");
        Equal(view.NeverHiddenCount, 231L);
        Check(view.NeverHidden!.SequenceEqual(JumbleOverlay.Of(0, blocked, OverlayPieces).NeverHidden!), "hover keeps the certified straddlers");
        naming.Hover(null);
        Equal(view.NeverHiddenCount, 0L, "leaving the grip clears its references");
    }

    private static void D02FocusModesAreIndependent()
    {
        var global = new FocusLayers();
        var local = new FocusLayers();
        foreach (var pane in new[] { global, local })
        {
            pane.AddPredicate("test", "Domain", Numeral.Of(1));
            pane.Focus("test");
            pane.Cycle();
        }
        global.Cycle();
        Func<int, bool> globalMember = piece => piece == 0, localMember = piece => piece == 1;
        Equal(JumbleStyle.FocusMode(global, globalMember), InactiveMode.Hidden);
        Equal(JumbleStyle.FocusMode(local, localMember), InactiveMode.Dimmed);
        var style = JumbleStyle.Encode(0, JumbleTint.FirstHeight, true, OverlayInk.None, !globalMember(1), !localMember(1));
        Equal(style.Fill, (byte)0x81, "membership bits differ on a shared piece");
        local.Cycle();
        Equal(JumbleStyle.FocusMode(local, localMember), InactiveMode.Hidden);
        Equal(JumbleStyle.FocusMode(global, globalMember), InactiveMode.Hidden);
    }

    private static void D02ApplyingExpressionDimsFocusedPane()
    {
        var (filter, _, _, _) = FilterAt300();
        var global = new FocusLayers();
        var local = new FocusLayers();
        filter.AppliedChanged += () => filter.FocusApplied(local);
        filter.SetText("height >= 4");
        Check(filter.Run() != null, "expression preview");
        Equal(local.Inactive, InactiveMode.Normal, "preview leaves visibility alone");
        Equal(filter.Apply(UserEvent.Key("Enter")), (string?)null, "apply the expression");
        Equal(local.Focused, PieceFilterModel.LayerId);
        Equal(local.Inactive, InactiveMode.Dimmed);
        Equal(global.Inactive, InactiveMode.Normal, "the other pane keeps its mode");
        local.Cycle();
        filter.Run();
        Equal(filter.Apply(UserEvent.Key("Enter")), (string?)null);
        Equal(local.Inactive, InactiveMode.Hidden, "reapplying preserves Hide");
    }

    private static void D02FoldingRepairClearsFocusAndScrub()
    {
        var sections = new LocalAccordion();
        var repair = RepairListModel.From(RefineCap(150), 300)!;
        var scrub = new RepairScrub();
        sections.Changed += () => scrub.FollowSection(sections, repair);
        sections.Toggle("repair_list");
        repair.Focus(2, UserEvent.Key("Down"));
        scrub.Show(repair.Rows[1]);
        scrub.Scrub("forward");
        sections.SetCount("repair_list", repair.Rows.Count);
        Equal(scrub.Step, 2, "an update while open preserves the scrub");
        sections.Toggle("cap_covers");
        Check(repair.Focused == null && scrub.Row == null && scrub.Step == 0, "folding clears focus and scrub");
        sections.Toggle("repair_list");
        Check(repair.Focused == null && scrub.Label == "", "reopening starts without a row");
    }

    private static void D02StyleRequiresTheBoundRevision()
    {
        Check(JumbleStyle.BoundFor(300, 300), "matching state style is usable");
        Check(!JumbleStyle.BoundFor(300, 301), "the pending revision cannot use the previous tint and focus");
        Check(!JumbleStyle.BoundFor(301, 300), "a head ahead of the draw cannot supply its style");
        Check(!JumbleStyle.BoundFor(null, 301) && JumbleStyle.BoundFor(301, 301), "the style can draw after binding");
    }
}
