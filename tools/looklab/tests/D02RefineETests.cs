using System;
using Magic600.Chrome;

internal static partial class Program
{
    private static void D02DockRebuildKeepsPanelFocus()
    {
        string source = Read("app/Chrome/Ui/ChromeFrame.cs");
        int capture = source.IndexOf("var focusOwner = IsInsideTree() ? GetViewport().GuiGetFocusOwner()", StringComparison.Ordinal);
        int detach = source.IndexOf("foreach (DockPanel panel in dockPanels) ChromeWidgets.Detach(panel.Content)", StringComparison.Ordinal);
        int attach = source.IndexOf("middle.AddChild(SideDock(DockSide.Right))", detach, StringComparison.Ordinal);
        int restore = source.IndexOf("focusOwner.GrabFocus()", StringComparison.Ordinal);
        Check(capture >= 0 && capture < detach && detach < attach && attach < restore,
            "the frame must restore a dock control's keyboard focus after reattaching it");
        Check(source.Contains("panel.Content.IsAncestorOf(focusOwner)")
            && source.Contains("focusOwner.IsVisibleInTree() && GetViewport().GuiGetFocusOwner() == null"),
            "only a visible dock control may retake focus, and only when no other control has it");
    }

    private static void D02NamingCapClearsRepairOverlay()
    {
        string record = RefineCap(150);
        var list = RepairListModel.From(record, 300)!;
        var scrub = new RepairScrub();
        // The old control still follows the list on both signals while the cap changes.
        void Refill() => scrub.Show(list.Focused is int row ? list.Rows[row - 1] : null);
        list.Changed += Refill;
        scrub.Changed += Refill;
        list.Down(UserEvent.Key("Down"));
        Check(scrub.Scrub("forward"), "row 1 can scrub to its second step");
        var moving = JumbleOverlay.Of(150, record, OverlayPieces, scrub.Row!.Number, scrub.Step, scrub.Label);
        Equal(moving.Layer, OverlayLayer.MovingSet);
        Equal(moving.Pieces, 2467L);

        scrub.ClearFocus(list);
        Check(list.Focused == null && scrub.Row == null && scrub.Step == 0, "the mounted old list cannot restore its row");
        var trace = JumbleOverlay.Of(0, RefineCap(0), OverlayPieces, scrub.Row?.Number ?? 0, scrub.Step, scrub.Label);
        Equal(trace.Layer, OverlayLayer.CutTrace);
        Equal(trace.Step, "");
        Equal(trace.NeverHiddenCount, 231L);
        string host = Read("app/Chrome/Ui/JumbleHost.cs");
        int clear = host.IndexOf("Scrub.ClearFocus(Repair)", StringComparison.Ordinal);
        Check(clear >= 0 && clear < host.IndexOf("Repair = record != null", StringComparison.Ordinal),
            "naming uses the old list to clear focus before replacing it");

        string preview = RefineShell("project/D02/ShellPreview.cs");
        int sameId = preview.IndexOf("if (pending.Status != PreviewStatus.Ready)", StringComparison.Ordinal);
        int retry = preview.IndexOf("if (moved == null) { SetOverlay(); Dress(); }", StringComparison.Ordinal);
        int load = preview.IndexOf("library.PreviewMoved(host, facts.Id, mask)", StringComparison.Ordinal);
        Check(sameId >= 0 && sameId < retry && retry < load
            && preview.Contains("moved == null && Model.Pending is { Status: PreviewStatus.Ready }"),
            "a ready preview can load the moving mask after its ID is already shown");
    }

    private static void D02DrawnPreviewKeepsMainViewOpenings()
    {
        // At the failing width, Local's wrapped footer occupied its entire height. The reserved slot feeds
        // Local's existing opening rule and keeps it in the same composite frame as Global and the preview.
        int slot = ViewPanes.MainSlotMinHeight;
        int localHeight = ViewPanes.OpeningHeight(337, slot, true);
        var layout = ViewPanes.Layout(new[]
        {
            new PaneSpec("Global", new PaneRect(51, 110, 391, slot)),
            new PaneSpec("Local", new PaneRect(453, 110, 337, localHeight)),
            new PaneSpec("Preview after", new PaneRect(798, 160, 436, 272)),
        });
        Equal(ViewPanes.Hit(layout, 454, 111)?.View, "Local");
        Equal(ViewPanes.OpeningHeight(337, slot, false), 0);
        string frame = Read("app/Chrome/Ui/ChromeFrame.cs");
        Check(frame.Contains("pane.Openings != null && pane.Content.IsInsideTree()")
            && frame.Contains("new Vector2(0, ViewPanes.MainSlotMinHeight)")
            && frame.Contains("Global.Slot.CustomMinimumSize = minimum")
            && frame.Contains("if (localDrawn) Local.Slot.CustomMinimumSize = minimum"),
            "the attached drawn pane must reserve space in the actual main slots");
    }
}
