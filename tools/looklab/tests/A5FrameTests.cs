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

    private static void A5CommandsRegistryKeymapAndPalette()
    {
        CommandRegistry registry = CommandRegistry.FromTable(System.IO.File.ReadAllText(System.IO.Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        Equal(registry.Order.Count, 198); Equal(registry.Commands["session.new"].Label, "New solve"); Equal(CommandRegistry.Location("view.preview.open"), "view");
        string text = Read("data/design/keymap.json");
        Keymap keys = Keymap.Parse(text, registry);
        Check(registry.Commands.ContainsKey("radial.view") && keys.Radials["radial.view"].Entries.Count == 6, "a radial menu is not a command");
        // The hovered view's context wins, then the focused one, then global; an unbound key resolves to nothing.
        Equal(keys.Resolve("R", new[] { "global-view", "global" })!.Command, "view.reset");
        Equal(keys.Resolve("R", new[] { "local-view", "global-view", "global" })!.Command, "view.local.configure");
        Check(keys.Resolve("R", new[] { "global" }) == null && keys.Resolve("F1", new[] { "local-view", "global" })!.Command == "command.find", "context fallback");
        Equal(keys.KeysFor("command.find", null), "F1, Space"); Equal(keys.KeysFor("workspace.set", "layout:script"), "2"); Equal(Keymap.KeyName(true, true, false, "Z"), "Ctrl+Alt+Z");
        // User overrides are a second file: rebind, add and remove without touching the defaults.
        string overrides = "{\"format\":\"magic600-design-keymap\",\"version\":1,\"radials\":{},\"bindings\":[{\"key\":\"F1\",\"context\":\"global\",\"command\":\"help.open\"}," +
            "{\"key\":\"Space\",\"context\":\"global\",\"command\":\"\"},{\"key\":\"Ctrl+K\",\"context\":\"global\",\"command\":\"command.find\"}]}";
        Keymap mine = Keymap.Parse(overrides, registry, keys);
        Equal(mine.KeysFor("command.find", null), "Ctrl+K"); Equal(mine.KeysFor("help.open", null), "Shift+Slash, F1"); Equal(keys.KeysFor("command.find", null), "F1, Space");
        Check(mine.Resolve("Space", new[] { "global" }) == null && mine.Radials.Count == keys.Radials.Count, "override");
        foreach ((string from, string to, string name) in new[] {
            ("\"key\": \"Ctrl+Alt+Z\"", "\"key\": \"Alt+Ctrl+Z\"", "Alt+Ctrl+Z"), ("\"key\": \"Space\"", "\"key\": \"F1\"", "F1"),
            ("\"command\": \"help.open\"", "\"command\": \"help.shut\"", "Shift+Slash"), ("\"context\": \"editor\"", "\"context\": \"dialog\"", "Ctrl+Enter"),
            ("\"radial.workspace\": {", "\"wheel.workspace\": {", "wheel.workspace"), ("\"argument\": \"next\",\n      \"label\": \"Next theme family\"", "\"argument\": \"next\"", "T") })
        {
            Check(text.Contains(from, StringComparison.Ordinal), "fixture lost " + from);
            Refuse(() => Keymap.Parse(text.Replace(from, to), CommandRegistry.FromTable(System.IO.File.ReadAllText(System.IO.Path.Combine(Root, "docs/progress/1.0/command-table.json")))), name);
        }
        // The palette: a query filters, and only pins, recency and frequency order the rows.
        var palette = new PaletteModel(registry, keys, (command, argument) => command == "session.new" ? "no engine" : null);
        var all = palette.Rows("");
        Check(all.Count > registry.Order.Count && all[0].Command == registry.Order[0] && all.Single(r => r.Command == "session.new").Unavailable == "no engine", "palette rows");
        var theme = palette.Rows("theme");
        Check(theme.Count >= 3 && theme.All(r => (r.Label + r.Command).Contains("theme", StringComparison.OrdinalIgnoreCase)) && palette.Rows("THEME family").Count < theme.Count, "palette filter");
        PaletteRow tune = theme.Single(r => r.Command == "theme.tune"), next = theme.Single(r => r.Argument == "next");
        Equal(next.Label, "Next theme family"); Equal(next.Keys, "T"); Equal(next.Location, "theme");
        palette.Used(tune); palette.Used(next); palette.Used(tune); palette.Used(next);
        Sequence(palette.Rows("theme").Take(2).Select(r => r.Id), new[] { next.Id, tune.Id });
        palette.Pin(tune, true); Equal(palette.Rows("")[0].Id, tune.Id); Check(palette.Rows("")[0].Pinned, "pin not shown");
        var again = new PaletteModel(registry, keys, (_, _) => null); again.Restore(palette.Store());
        Equal(again.Store(), palette.Store()); Sequence(again.Rows("theme").Take(2).Select(r => r.Id), new[] { tune.Id, next.Id });
        again.Restore("use|gone.command||3|9\npin|gone.command|"); Equal(again.Store(), "");
        Refuse(() => again.Restore("use|theme.tune||many|1"), "palette");
    }

    private static void A5SessionPreviewAndTwoHistories()
    {
        var effects = new[] { new OrbitEffect("a", 2, 10) }; string[] three = { "m1", "m2", "m3" };
        var preview = new PreviewModel(); int changes = 0; preview.Changed += () => changes++;
        Equal(preview.CommitRefusal(0), "there is no preview"); Check(preview.Take(0) == null && !preview.Discard(), "nothing to take");
        Preview first = preview.Show("preview a", "own macro", three, PreviewStatus.Ready, null, effects, "ok", 10);
        Check(first.Id == 1 && preview.Scrub == 3 && !preview.Replaced && preview.CommitRefusal(10) == null, "a first preview commits at once");
        // A replacement takes a new id and cannot be committed inside the settle interval.
        Preview second = preview.Show("preview b", "own macro", three, PreviewStatus.Ready, null, effects, "ok", 20);
        Check(second.Id == 2 && preview.Replaced && preview.CommitRefusal(20 + PreviewModel.Settle / 2) != null && preview.Take(20.1) == null && preview.Pending == second, "settle");
        Check(preview.CommitRefusal(20 + PreviewModel.Settle * 1.01) == null, "settle ends");
        preview.SetScrub(9); Equal(preview.Scrub, 3); preview.SetScrub(0); Check(preview.CommitRefusal(30) != null, "an empty prefix commits nothing");
        preview.SetScrub(2); Preview prefix = preview.Take(30)!;
        Check(prefix.Id == 3 && prefix.Moves.Count == 2 && prefix.Statement.EndsWith("prefix(2)", StringComparison.Ordinal) && preview.Pending == null, "the prefix is its own preview");
        Refuse(() => preview.Show("preview c", "x", three, PreviewStatus.Refused, " ", effects, "ok", 40), "preview");
        preview.Show("preview c", "package", Array.Empty<string>(), PreviewStatus.Refused, "not yours", effects, "none", 40);
        Check(preview.CommitRefusal(99) == "not yours" && preview.Take(99) == null, "a refused preview never commits");
        preview.Show("preview d", "own macro", three, PreviewStatus.Ready, null, effects, "ok", 50); preview.MarkStale("state moved");
        Check(preview.Pending!.Status == PreviewStatus.Stale && preview.Take(99) == null && preview.Discard() && preview.Pending == null && changes > 8, "stale");

        var moves = new MoveHistory();
        Refuse(() => moves.Commit("robot", "x", "t", "s", three), "robot"); Refuse(() => moves.Commit("hand", "x", "t", "s", Array.Empty<string>()), "x");
        foreach (string name in new[] { "one", "two", "three" }) moves.Commit("hand", name, "10:00", "commit " + name, three);
        Sequence(moves.Entries.Select(e => e.Id), new[] { "t-0001", "t-0002", "t-0003" }); Equal(moves.Cursor, 3);
        Check(moves.Undo() && moves.Undo() && moves.Cursor == 1 && moves.Redo() && moves.Cursor == 2 && moves.Entries.Count == 3, "undo and redo move the cursor only");
        // A partial undo steps inside the entry and is recorded as its own action; a commit waits for it.
        Check(moves.StepBack() && moves.Inner == 1 && moves.Cursor == 2 && moves.Actions[^1] == "partial undo t-0002 move 3" && moves.CommitRefusal != null &&
            moves.Commit("hand", "x", "t", "s", three) == null, "partial undo");
        Check(moves.StepBack() && moves.StepBack() && moves.Inner == 0 && moves.Cursor == 1 && moves.Actions[^1] == "partial undo t-0002 move 1", "the last inner step finishes the undo");
        // A commit in mid-history keeps the abandoned steps as a branch.
        HistoryEntry four = moves.Commit("macro", "four", "10:05", "commit four", three)!;
        Check(four.Id == "t-0004" && moves.Entries.Count == 2 && moves.Branches.Count == 1 && moves.Branches[0].From == "t-0001" &&
            moves.Branches[0].Entries.Select(e => e.Name).SequenceEqual(new[] { "two", "three" }), "branch kept");
        Check(moves.RollTo(0) && moves.Actions[^1] == "roll to start (2 steps)" && !moves.RollTo(0) && !moves.RollTo(3) && !moves.Undo() && moves.RollTo(2) && !moves.Redo(), "roll");
        Equal(moves.Actions.Count, 3 + 3 + 3 + 1 + 2);

        var layout = new LayoutHistory();
        Check(layout.Undo() == null && layout.Record("Opened", "a") && !layout.Record("Same", "a") && layout.Record("Theme", "b") && layout.Record("Panes", "c"), "record");
        Check(layout.Undo()!.State == "b" && layout.Undo()!.State == "a" && layout.Undo() == null && layout.Redo()!.State == "b" && layout.Cursor == 1, "layout undo and redo");
        Check(layout.Record("Density", "d") && layout.Steps.Count == 3 && layout.Redo() == null && layout.RollTo(0)!.State == "a" && layout.RollTo(0) == null, "a new step drops the redo");
        layout.RollTo(2); layout.Record("Knob 1", "k1", "knob"); layout.Record("Knob 2", "k2", "knob"); layout.Record("Knob 3", "k3", "knob");
        Check(layout.Steps.Count == 4 && layout.Steps[^1].State == "k3" && layout.Undo()!.State == "d", "a dragged knob is one step");
        for (int i = 0; i < LayoutHistory.Kept + 20; i++) layout.Record("step", "s" + i);
        Check(layout.Steps.Count == LayoutHistory.Kept && layout.Cursor == LayoutHistory.Kept - 1, "the layout history is bounded");
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
