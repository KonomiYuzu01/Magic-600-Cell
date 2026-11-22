using System;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    private static FrameModel Frame()
    {
        var frame = new FrameModel();
        frame.AddDockPanel(DockSide.Left, "history"); frame.AddDockPanel(DockSide.Left, "scripts");
        frame.AddDockPanel(DockSide.Right, "inspector"); frame.AddDockPanel(DockSide.Bottom, "problems"); frame.AddDockPanel(DockSide.Bottom, "console");
        return frame;
    }

    private static void A5FrameLockedPairRefusesHidingAMainView()
    {
        FrameModel frame = Frame(); int changes = 0; frame.Changed += () => changes++;
        string start = frame.Store();
        Check(!frame.Close(FrameModel.Pair) && frame.Refusal!.Contains("cannot be closed", StringComparison.Ordinal), "the pair was closed");
        Check(!frame.Drop("editor", FrameModel.Pair, DropZone.Tab) && frame.Refusal!.Contains("hide the Global and Local views", StringComparison.Ordinal), "a tab was dropped over the pair");
        Check(!frame.Drop(FrameModel.Pair, FrameModel.Pair, DropZone.Left) && !frame.Drop("editor", "nowhere", DropZone.Left), "an impossible drop was accepted");
        Check(!frame.SetPairArrangement("tabbed", .5) && !frame.SetPairArrangement("side", .9), "a pair arrangement that hides a view was accepted");
        Equal(frame.Store(), start, "a refused change moved the arrangement"); Equal(changes, 6, "a refusal was not announced");
        // Free tiling around the pair: beside it, tabbed with each other, moved, closed.
        Check(frame.Drop("editor", FrameModel.Pair, DropZone.Left) && frame.Refusal == null, "editor not opened beside the pair");
        Check(frame.Drop("preview", "editor", DropZone.Tab) && frame.Drop("canvas", FrameModel.Pair, DropZone.Bottom), "panes not tiled");
        Sequence(frame.Centre.Panes(), new[] { "editor", "preview", FrameModel.Pair, "canvas" });
        Check(frame.Drop("canvas", "preview", DropZone.Tab) && frame.Activate("editor") && frame.Close("preview"), "panes not moved or closed");
        Sequence(frame.Centre.Panes(), new[] { "editor", "canvas", FrameModel.Pair });
        Check(frame.Close("editor") && frame.Close("canvas") && !frame.Centre.IsSplit && frame.IsOpen(FrameModel.Pair), "the pair is not what remains");
        Check(frame.Drop("editor", FrameModel.Pair, DropZone.Left) && frame.Resize(frame.Centre, .02) && frame.Centre.Ratio == .1, "a splitter left its range");
        Check(!frame.Resize(PaneNode.Split(true, .5, PaneNode.Group("a"), PaneNode.Group("b")), .5) && !frame.Resize(frame.Centre.A!, .5) && frame.Close("editor"), "a foreign splitter was resized");
        // One panel per dock; the open panel's button collapses the dock.
        Check(frame.ToggleDock(DockSide.Left, "history") && frame.Docks[DockSide.Left].Open == "history", "dock not opened");
        Check(frame.ToggleDock(DockSide.Left, "scripts") && frame.Docks[DockSide.Left].Open == "scripts", "dock shows two panels");
        Check(frame.ToggleDock(DockSide.Left, "scripts") && frame.Docks[DockSide.Left].Open == null, "dock not collapsed");
        Check(!frame.ToggleDock(DockSide.Left, "inspector"), "a panel opened in another dock");
        Refuse(() => frame.AddDockPanel(DockSide.Bottom, "history"), "history");
        Check(frame.SetPairMaximised(true) && frame.OnlyPairShown && frame.SetPairMaximised(false) && frame.SetFocusMode(true) && frame.OnlyPairShown, "maximise or focus mode");
    }

    private static void A5IconSetNamesGlyphsAndFallbacks()
    {
        string text = Read("data/design/icons.json");
        IconSet icons = IconSet.Parse(text);
        Check(icons.Font.EndsWith("lucide.woff2", StringComparison.Ordinal) && icons.Icons.Count >= 20, "icon set");
        Equal(icons.Icons["panel.history"], new IconEntry("history", "H"));
        Check(icons.Icons.Values.Select(i => i.Glyph).Distinct().Count() == icons.Icons.Count, "two meanings share a glyph");
        Refuse(() => IconSet.Parse(text.Replace("\"fallback\": \"H\"", "\"fallback\": \"History\"")), "panel.history");
        Refuse(() => IconSet.Parse(text.Replace("\"font\": \"lucide-static", "\"font\": \"../lucide-static")), "icons");
        Refuse(() => IconSet.Parse(text.Replace("lucide.woff2", "lucide.ttf")), "icons");
        Refuse(() => IconSet.Parse(text.Replace("\"glyph\": \"history\",", "")), "panel.history");
    }

    private static void A5FrameStoredArrangementRoundTripAndWorkspaces()
    {
        FrameModel frame = Frame();
        frame.Drop("editor", FrameModel.Pair, DropZone.Left); frame.Drop("preview", FrameModel.Pair, DropZone.Bottom);
        frame.ToggleDock(DockSide.Right, "inspector"); frame.SetPairArrangement("stacked", .6);
        string stored = frame.Store();
        FrameModel other = Frame();
        Check(other.Apply(stored), "stored arrangement refused: " + other.Refusal); Equal(other.Store(), stored);
        // A panel this build does not have stays closed; a centre that hides a main view is refused whole.
        Check(new FrameModel().Apply(stored), "an unknown dock panel refused the arrangement");
        foreach ((string from, string to, string reason) in new[]
        {
            ("\"tabs\":[\"pair\"]", "\"tabs\":[\"pair\",\"canvas\"]", "tabs a pane over"),
            ("\"tabs\":[\"pair\"]", "\"tabs\":[\"reference\"]", "leaves out"),
            ("\"tabs\":[\"preview\"]", "\"tabs\":[\"editor\"]", "twice"),
            ("\"ratio\":0.6}", "\"ratio\":0.95}", "less than a fifth"),
            ("\"focus\":false", "\"focus\":0", "cannot be read"),
            ("\"focus\":false", "\"focus\":false,\"floating\":[]", "cannot be read"),
        })
        {
            Check(stored.Contains(from, StringComparison.Ordinal), "fixture lost " + from);
            Check(!other.Apply(stored.Replace(from, to)) && other.Refusal!.Contains(reason, StringComparison.Ordinal), "accepted: " + to + " / " + other.Refusal);
            Equal(other.Store(), stored, "a refused arrangement was applied in part");
        }
        // Status line: hovered element first, gestures dropped from the end to fit, never longer than the cap.
        Equal(StatusLine.Compose("cell 17", new[] { "drag: rotate", "wheel: zoom" }, "gap 0.30"), "cell 17  ·  drag: rotate  ·  wheel: zoom  ·  gap 0.30");
        Equal(StatusLine.Compose("cell 17", new[] { "drag: rotate", "wheel: zoom" }, "gap 0.30", 40), "cell 17  ·  drag: rotate  ·  gap 0.30");
        Equal(StatusLine.Compose(null, Array.Empty<string>(), null), "nothing hovered");
        Equal(StatusLine.Compose(new string('x', 200), new[] { "drag" }, "v").Length, StatusLine.Cap);
        Refuse(() => StatusLine.Compose("a", Array.Empty<string>(), null, 3), "cap");
        WorkspaceSet shipped = WorkspaceSet.Load(FileAt("data/design/workspaces.json"));
        Sequence(shipped.Order, new[] { "solve", "script", "analyse", "reference" });
        Sequence(shipped.Order.Select(id => shipped.Names[id]), new[] { "Solve", "Script", "Analyse", "Reference" });
        foreach (string id in shipped.Order) { FrameModel f = Frame(); Check(f.Apply(shipped.Stored[id]) && f.IsOpen(FrameModel.Pair) && !f.FocusMode, id + " not applied"); }
        string text = Read("data/design/workspaces.json");
        Refuse(() => WorkspaceSet.Parse(text.Replace("\"id\": \"script\"", "\"id\": \"solve\"")), "solve");
        Refuse(() => WorkspaceSet.Parse(text.Replace("\"reference\"\n", "\"pair\"\n")), "reference");
    }
}
