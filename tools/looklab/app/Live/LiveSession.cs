using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Godot;
using LookLab.Core;
using Magic600.Chrome;

namespace LookLab.App.Live;

// The interactive Look Lab: it rebuilds the lab when a data file changes and keeps the camera, the
// look and the panel state across the rebuild and across a restart. A file the lab refuses leaves
// the last good lab on screen with the reason in the notice line.
internal sealed partial class LiveSession : Control
{
    public static readonly string[] WatchedFolders = { "tools/looklab/data", "tools/looklab/presets" };
    public static readonly string[] WatchedFiles = { "docs/progress/1.0/command-table.json" };
    public const double PollSeconds = .5, SaveSeconds = 1;
    public LabApp App { get; private set; }
    public ChromeFrame Frame { get; private set; }
    public Label Notice { get; }
    public int Reloads { get; private set; }
    public int Refusals { get; private set; }
    internal Func<string, LabApp> Build { get; set; }
    private readonly string root, stateFile;
    private readonly DataWatch watch;
    private readonly List<string> pending = new();
    private double sincePoll, sinceSave, open;
    private int pictured;
    private readonly string pictureFile;
    public const double PictureSeconds = 4;
    private byte[] saved = Array.Empty<byte>();

    public LiveSession(string root, string stateFolder)
    {
        this.root = root; stateFile = Path.Combine(stateFolder, "view.json"); pictureFile = Path.Combine(stateFolder, "frame.png");
        Build = r => new LabApp(r, opening: true);
        SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        App = Build(root); AddChild(App);
        Frame = LabFrame.Attach(App, root); AddChild(Frame);
        Notice = new Label { Name = "LiveNotice", Visible = false, MouseFilter = MouseFilterEnum.Ignore, Position = new Vector2(8, 6) };
        Notice.AddThemeColorOverride("font_outline_color", Colors.Black); Notice.AddThemeConstantOverride("outline_size", 6);
        AddChild(Notice);
        watch = new DataWatch(root, WatchedFolders, WatchedFiles);
        if (File.Exists(stateFile))
        {
            try
            {
                saved = File.ReadAllBytes(stateFile);
                App.RestoreView(ViewStates.Parse(System.Text.Encoding.UTF8.GetString(saved), App.State.Schema)); Frame.Flush();
            }
            catch (Exception ex) when (ex is FormatException or IOException or UnauthorizedAccessException)
            {
                Say("Kept view not restored: " + ex.Message, false);
            }
        }
    }

    // The window opens at most of the screen, so the frame has room at the screen's own scale.
    public override void _Ready()
    {
        if (DisplayServer.GetName() == "headless") return;
        Window window = GetWindow(); Rect2I screen = DisplayServer.ScreenGetUsableRect(window.CurrentScreen);
        if (window.Mode != Window.ModeEnum.Windowed || window.Size.X >= screen.Size.X * .7) return;
        var size = new Vector2I((int)(screen.Size.X * .86), (int)(screen.Size.Y * .86));
        window.Size = size; window.Position = screen.Position + (screen.Size - size) / 2;
    }

    private int lamp;

    public override void _Process(double delta)
    {
        sincePoll += delta; sinceSave += delta; open += delta;
        // Failures show: the error lamp counts them and the Problems panel lists them.
        if (LabErrors.Count != lamp && IsInstanceValid(Frame))
        {
            IReadOnlyList<string> all = LabErrors.All;
            for (int i = lamp; i < all.Count; i++) LabFrame.InspectOf(Frame).Feedback.Report(Severity.Error, all[i].Split('\n')[0], "look-lab", 0);
            lamp = all.Count;
            Frame.SetAlert(lamp, lamp == 0 ? "" : all[0].Split('\n')[0]);
        }
        // A picture of the window, four and twelve seconds after it opened, beside the kept view: what the lab looked like.
        if (pictured < 2 && open >= PictureSeconds * (1 + 2 * pictured) && DisplayServer.GetName() != "headless")
        {
            pictured++;
            try { GetViewport().GetTexture().GetImage().SavePng(pictureFile); }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or InvalidOperationException) { }
        }
        if (sincePoll >= PollSeconds)
        {
            sincePoll = 0;
            IReadOnlyList<string> changed = watch.Changed();
            // A file still being written shows up again in the next poll; reload once it is quiet.
            if (changed.Count > 0) pending.AddRange(changed.Where(c => !pending.Contains(c)));
            else if (pending.Count > 0) { string[] files = pending.ToArray(); pending.Clear(); Reload(files); }
        }
        if (sinceSave >= SaveSeconds) { sinceSave = 0; Save(); }
    }

    public bool Reload(IReadOnlyList<string> files)
    {
        string what = files.Count == 1 ? files[0] : files.Count + " files";
        ViewState kept = App.CaptureView();
        LabApp? next = null; ChromeFrame? frame = null;
        try
        {
            next = Build(root);
            AddChild(next); MoveChild(next, 0);
            frame = LabFrame.Attach(next, root);
            next.RestoreView(kept); frame.Flush();
        }
        catch (Exception ex)
        {
            frame?.Free();
            if (next != null) { if (next.GetParent() == this) RemoveChild(next); next.QueueFree(); }
            Refusals++;
            Say($"Refused {what}: {ex.Message} (showing the last good data)", true);
            return false;
        }
        LabApp old = App; App = next;
        ChromeFrame oldFrame = Frame; Frame = frame;
        RemoveChild(oldFrame); oldFrame.Free(); RemoveChild(old); old.QueueFree();
        AddChild(Frame); MoveChild(Frame, 1);
        Reloads++;
        Say($"Reloaded {what}", false);
        return true;
    }

    private void Say(string text, bool refused)
    {
        Notice.Text = text; Notice.Visible = true;
        Notice.AddThemeColorOverride("font_color", refused ? new Color(1f, .55f, .35f) : new Color(.75f, .9f, 1f));
        GD.Print("LOOKLAB_LIVE " + text);
    }

    public void Save()
    {
        try
        {
            byte[] bytes = ViewStates.Bytes(App.CaptureView(), App.State.Schema);
            if (bytes.AsSpan().SequenceEqual(saved)) return;
            string folder = Path.GetDirectoryName(stateFile)!;
            Directory.CreateDirectory(folder);
            foreach (string path in new[] { folder, stateFile })
                if ((File.Exists(path) || Directory.Exists(path)) && (File.GetAttributes(path) & FileAttributes.ReparsePoint) != 0)
                    throw new IOException("the view file must not pass through links");
            string temporary = stateFile + ".tmp";
            File.WriteAllBytes(temporary, bytes);
            File.Move(temporary, stateFile, true);
            saved = bytes;
        }
        catch (Exception ex) when (ex is FormatException or IOException or UnauthorizedAccessException)
        {
            Say("View not kept: " + ex.Message, true);
        }
    }

    public override void _Notification(int what)
    {
        if (what == NotificationWMCloseRequest) Closing(true);
        else if (what == NotificationPredelete && IsInstanceValid(App)) Keep();
    }

    private void Keep() { try { Save(); } catch (Exception ex) { Safe.Report(ex); } }

    // The window's close button: keep the view, then leave. Nothing here may stop the window from closing.
    public void Closing(bool quit)
    {
        if (IsInstanceValid(App)) Keep();
        if (quit) { LabErrors.Remove(); GetTree().Quit(); }
    }
}

internal static class LiveChecks
{
    public static bool Handles(string mode) => mode == "a5-live";

    // One rebuild that keeps the view, one refused rebuild that keeps the lab, and a restart from the kept file.
    public static void Run(Node host, string root, string scratch, string fault)
    {
        string folder = Path.Combine(scratch, "a5-live");
        // Each run starts from no kept view, so a fault is judged by its own check and not by an earlier run's file.
        if (Directory.Exists(folder)) Directory.Delete(folder, true);
        var session = new LiveSession(root, folder); host.AddChild(session);
        LabApp first = session.App;
        first.View.SetProcess(false);
        GlobalLayout.Drag(first.View.Q, first.View.Spin, .37, -.21);
        ParameterDefinition gap = first.State.Schema["gap"];
        first.State.Set("gap", SmokeProof.NonDefault(gap));
        first.ViewParts["greybox.open"].Set("1"); first.ViewParts["greybox.context"].Set("solve");
        ViewState before = first.CaptureView();
        if (fault == "a5-view-dropped") session.Build = r => { var app = new LabApp(r); app.ViewParts.Clear(); return app; };
        Require(session.Reload(new[] { "tools/looklab/data/layouts/split-views.json" }) && !ReferenceEquals(session.App, first), fault, "valid data was not reloaded");
        Same(before, session.App.CaptureView(), first.State.Schema, fault, "reload");
        LabApp good = session.App;
        session.Build = _ => throw new FormatException("bgTint: outside its range");
        bool refused = !session.Reload(new[] { "tools/looklab/data/parameters.json" });
        if (fault == "a5-bad-data-shown") refused = false;
        Require(refused && ReferenceEquals(session.App, good) && session.Notice.Text.Contains("bgTint", StringComparison.Ordinal) &&
            session.Refusals == 1 && session.Reloads == 1, fault, "refused data replaced the last good lab");
        session.Save();
        if (fault == "a5-view-not-written") File.Delete(Path.Combine(folder, "view.json"));
        var restarted = new LiveSession(root, folder); host.AddChild(restarted);
        Same(before, restarted.App.CaptureView(), first.State.Schema, fault, "restart");
        Framed(restarted, fault);
        GD.Print($"LOOKLAB_A5_LIVE_PASS reloads={session.Reloads} refusals={session.Refusals} restart=kept parts={before.Ui.Count} checks=7");
    }

    // The window frame around the lab: both main views in the locked pair, the lab's panel in the right dock,
    // and a drop that would hide a main view refused with its reason on screen.
    private static void Framed(LiveSession session, string fault)
    {
        ChromeFrame frame = session.Frame; LabApp app = session.App; LabDemo demo = LabFrame.DemoOf(frame);
        Require(frame.Global.Slot.IsAncestorOf(app.GlobalStage) && frame.Local.Slot.IsAncestorOf(app.Linked.LocalStage!) && app.Linked.LocalStage!.Visible,
            fault, "a main view is missing from the locked pair");
        if (frame.Model.Docks[DockSide.Right].Open != LabFrame.Tune) { frame.Model.ShowDock(DockSide.Right, LabFrame.Tune); frame.Flush(); }
        Require(frame.Model.Docks[DockSide.Right].Open == LabFrame.Tune && frame.IsAncestorOf(app.ParameterPanel), fault, "the Tune panel is not in the right dock");
        // Local has its own shape: a cap around the moving cells, the one on file that is chosen; Global draws every cell.
        app.View._Process(0);
        Require(app.Linked.Shape != null && app.Linked.LocalChoices == 2 && app.Linked.Local!.Material.GetShaderParameter("focus_cos").AsDouble() > -1 &&
            app.Linked.Local.Material.GetShaderParameter("projection_kind").AsInt32() == (int)app.Linked.Shape.Projection &&
            Math.Abs(app.Linked.Local.Material.GetShaderParameter("ghost").AsDouble() - app.Linked.Shape.Ghost) < 1e-6 &&
            app.View.Material.GetShaderParameter("focus_cos").AsDouble() < -1.5 && app.State.Preset.Params.Choice("projection") != "",
            fault, "Local does not draw its own shape of the moving cells");
        // The two inspector tiers read one model; a View field acts on the lab, stays in range and resets.
        SampleInspect inspect = LabFrame.InspectOf(frame);
        double gapWas = app.State.Preset.Params.Number("gap"), gapDefault = inspect.Inspector["view.gap"].Default;
        frame.Model.ShowDock(DockSide.Right, "inspector"); frame.Flush();
        ValueField gapField = inspect.Panel.Fields.Single(f => f.Field.Id == "view.gap");
        gapField.DragBy(-ValueField.DragPixels * 2);
        bool dragged = Math.Abs(app.State.Preset.Params.Number("gap") - (gapWas - .02)) < 1e-9 && gapField.Value.Text == (gapWas - .02).ToString("0.00", System.Globalization.CultureInfo.InvariantCulture);
        bool refusedValue = gapField.Type("0.9") != null && frame.Notice.Text.Contains("outside", StringComparison.Ordinal) && Math.Abs(app.State.Preset.Params.Number("gap") - (gapWas - .02)) < 1e-9;
        bool bound = gapField.Type("= 0.1 + 0.1") == null && inspect.Inspector["view.gap"].Binding == "0.1 + 0.1" && app.CaptureView().Ui["frame.bindings"] == "view.gap=0.1 + 0.1";
        gapField.Caption.EmitSignal(BaseButton.SignalName.Pressed);
        bool reset = app.State.Preset.Params.Number("gap") == gapDefault && inspect.Inspector["view.gap"].Provenance == Provenance.Default;
        inspect.Inspector.Set("view.gap", gapWas);
        Require(frame.IsAncestorOf(inspect.Strip) && frame.IsAncestorOf(inspect.Panel) && inspect.Strip.Selection.Text.EndsWith("cells", StringComparison.Ordinal) &&
            dragged && refusedValue && bound && reset && app.State.Preset.Params.Number("gap") == gapWas, fault, "an inspector field did not drag, refuse, bind and reset");
        demo.Execute("operation.preview", "sample:0"); frame.Flush();
        int previewId = demo.Preview.Pending!.Id;
        Require(inspect.Strip.Fields.Count == 4 && inspect.Inspector.Nudge("preview.repeat", 1) && demo.Preview.Pending!.Id == previewId + 1 && demo.Preview.Replaced, fault, "a preview parameter did not prepare a new preview");
        demo.Execute("operation.preview.cancel", null);
        Require(inspect.Strip.Fields.Count == 0 && !inspect.Inspector.Fields.Any(f => f.Section == InspectorModel.Preview), fault, "preview parameters stayed after the preview was discarded");
        // The bottom index: merged repeats, count badges on the collapsed tabs, Find Last Error, Esc cancels a task.
        inspect.StartTask(); frame.Flush();
        Require(inspect.Feedback.Problems.Count == 4 && inspect.Feedback.Count(Severity.Error) == 4 && frame.Badge("problems") == "4" && frame.Badge("tasks") == "1" &&
            frame.FindChildren("*", "Label", true, false).Any(l => l.HasMeta("badge")) && inspect.Problems.FindLastError() && inspect.Problems.Found == inspect.Feedback.LastError!.Id,
            fault, "the bottom index does not count, badge and find its problems");
        inspect.Tick(1);
        Require(inspect.Feedback.Tasks[^1].Progress > 0 && frame.Press("Escape") && inspect.Feedback.Running == 0 && frame.Badge("tasks") == null &&
            inspect.Feedback.Tasks[^1].State == TaskState.Cancelled, fault, "Esc did not cancel the running task");
        // Labels are off by default: workspace tabs are icons and marks are glyphs; the toggle brings the words back.
        bool iconsOnly = !frame.Labels && frame.Global.Marks.GetChild(0).GetChild(0).GetChildCount() == 1;
        frame.Labels = true; frame.Flush();
        Require(iconsOnly && frame.Global.Marks.GetChild(0).GetChild(0).GetChildCount() == 2 && app.CaptureView().Ui["frame.labels"] == "on", fault, "the labels toggle did not show and hide the words");
        frame.Labels = false; frame.Flush();
        frame.Model.ShowDock(DockSide.Right, LabFrame.Tune); frame.Flush();
        Require(frame.Model.IsOpen(LabFrame.Greybox) && frame.IsAncestorOf(app.Greybox.Canvas), fault, "the open greybox is not a pane of the frame");
        Require(frame.FindChild("TopBar", true, false) != null && frame.Workspaces.Order.SequenceEqual(new[] { "solve", "script", "analyse", "reference", "gallery" }), fault, "top bar or workspaces missing");
        // The theme button: the other family, the scene behind the puzzle follows, and the choice is kept.
        var themeButton = (Button)frame.FindChild("ThemeButton", true, false);
        string family = frame.Tokens.Theme.Id; Color window = frame.Tokens.C("window.background");
        double ground = app.State.Preset.Params.Number("bgLightness");
        themeButton.EmitSignal(BaseButton.SignalName.Pressed); frame.Flush();
        Require(frame.Tokens.Theme.Id != family && frame.Tokens.C("window.background") != window && themeButton.Text == frame.Tokens.Theme.Name &&
            app.State.Preset.Params.Number("bgLightness") != ground && app.CaptureView().Ui["frame.appearance"].Contains("theme=" + frame.Tokens.Theme.Id, StringComparison.Ordinal),
            fault, "the theme button did not switch the family and the scene");
        // The Appearance panel: density and display scale resize the frame, high contrast and a knob recolour it
        // in place, and a value outside a knob's range changes nothing.
        Require(frame.Model.ToggleDock(DockSide.Right, LabFrame.Look), fault, "the Appearance panel is not in the right dock");
        frame.Flush();
        var panel = (AppearancePanel)frame.FindChild("AppearancePanel", true, false) ?? throw new Exception("a5-live: no Appearance panel");
        int bar = frame.Tokens.Px("bar.height"); ViewFrame global = frame.Global;
        Require(panel.Edit(a => a.SetDensity("comfortable"), true, false) && frame.Tokens.Px("bar.height") > bar && !ReferenceEquals(frame.Global, global), fault, "density did not resize the frame");
        bar = frame.Tokens.Px("bar.height");
        Require(panel.Edit(a => a.SetDisplayScale(1.5), true, false) && frame.Tokens.Px("bar.height") > bar, fault, "display scale did not resize the frame");
        global = frame.Global; Color accent = frame.Tokens.C("accent.base"), text = frame.Tokens.C("text");
        double hue = (panel.Look.Knob("accentHue") + 120) % 360;
        bool recoloured = panel.Edit(a => a.SetKnob("accentHue", hue), false, false) && panel.Edit(a => a.HighContrast = true, false, false);
        if (fault == "a5-knob-ignored") recoloured = recoloured && frame.Tokens.C("accent.base") == accent;
        Require(recoloured && frame.Tokens.C("accent.base") != accent && frame.Tokens.C("text") != text && ReferenceEquals(frame.Global, global) &&
            frame.Tokens.ContrastIssues().Count == 0, fault, "a knob or high contrast did not recolour the frame in place");
        string kept = panel.Look.Store();
        Require(!panel.Edit(a => a.SetKnob("tone", 4), false, false) && panel.Look.Store() == kept && panel.Issues.Text.Contains("tone", StringComparison.Ordinal),
            fault, "a knob left its range");
        Require(panel.Edit(a => a.SetSceneLook("celebrating"), false, true) &&
            app.State.Preset.Params.Number("bgHue") == frame.Tokens.Theme.Scenes["celebrating"]["bgHue"].GetDouble(), fault, "the scene look was not applied");
        Require(panel.Edit(a => { a.SetDensity("compact"); a.SetDisplayScale(1); a.HighContrast = false; a.ResetKnobs(); }, true, false) && !panel.Look.Tuned, fault, "the appearance was not reset");
        // The token specimen: three type roles with the face in use, every ramp step and every colour role.
        Require(frame.Model.ToggleDock(DockSide.Left, LabFrame.Tokens), fault, "the Tokens panel is not in the left dock");
        frame.Flush();
        var specimen = (TokenSpecimen)frame.FindChild("TokenSpecimen", true, false) ?? throw new Exception("a5-live: no token specimen");
        int swatches = frame.Tokens.Theme.Ramps.Count * ThemeFamily.Steps + frame.Tokens.System.Roles.Count;
        if (ChromeFonts.Roots.Any(r => File.Exists(Path.Combine(r, frame.Tokens.System.Type["ui"].Files["400"][0]))))
            Require(specimen.Faces["ui"] == "IBM Plex Sans" && frame.Theme.DefaultFont is FontVariation { BaseFont: FontFile }, fault, "the installed UI face is not in use");
        // Icons: every dock button, the bottom tabs included, names an icon of the set; with the icon font installed it shows the glyph.
        Button[] rail = frame.FindChildren("*", "Button", true, false).Cast<Button>().Where(b => b.HasMeta("dock_panel") && b.HasMeta("icon")).ToArray();
        Require(rail.Length == 12 && rail.All(b => ChromeIcons.Set!.Icons.ContainsKey(b.GetMeta("icon").AsString()) &&
            b.Text == (ChromeIcons.Glyph(b.GetMeta("icon").AsString()) ?? ChromeIcons.Fallback(b.GetMeta("icon").AsString()))), fault, "a rail button has no icon");
        GD.Print("LOOKLAB_A5_ICONS installed=" + ChromeIcons.Installed + " set=" + ChromeIcons.Set!.Icons.Count);
        GD.Print("LOOKLAB_A5_SCALE system=" + panel.Look.SystemScale.ToString(System.Globalization.CultureInfo.InvariantCulture) + " body=" + frame.Theme.DefaultFontSize);
        GD.Print("LOOKLAB_A5_FACES " + string.Join(", ", specimen.Faces.Select(f => f.Key + "=" + f.Value)));
        Require(specimen.Swatches == swatches && specimen.Faces.Count == 3 && specimen.Faces.All(f => frame.Tokens.System.Type[f.Key].Faces.Contains(f.Value)),
            fault, "the token specimen does not show every ramp step, role and type role");
        Require(frame.Model.ToggleDock(DockSide.Left, LabFrame.Tokens), fault, "the Tokens panel did not close");
        Require(frame.Model.ToggleDock(DockSide.Right, LabFrame.Tune), fault, "the Tune panel did not come back");
        frame.Flush();
        // Commands: a key opens the palette; it filters by what is typed, orders by pins and use only, refuses an
        // unavailable row with its reason, runs a typed command, and single keys act without it.
        var palette = (CommandPalette)frame.Overlays.FindChild("CommandPalette", true, false);
        Require(frame.Press("F1") && palette.IsOpen && palette.Rows.Count >= 198 && !frame.Press("R"), fault, "the palette did not open with every command");
        palette.Type("workspace"); int found = palette.Rows.Count;
        Require(found > 3 && found < 30 && palette.Rows.All(r => (r.Label + r.Command).Contains("workspace", StringComparison.OrdinalIgnoreCase)), fault, "the palette did not filter");
        PaletteRow last = palette.Rows[^1]; palette.TogglePin(found - 1);
        Require(palette.Rows[0].Id == last.Id && palette.Rows[0].Pinned, fault, "a pinned row is not first");
        palette.TogglePin(0);
        palette.Type("New solve");
        bool held = !palette.Accept() && palette.IsOpen && palette.Help.Text.Contains("Unavailable", StringComparison.Ordinal);
        if (fault == "a5-unavailable-ran") held = false;
        Require(held, fault, "an unavailable command ran from the palette");
        palette.Type("> workspace.set layout:analyse");
        Require(palette.Rows.Count == 1 && palette.Accept() && !palette.IsOpen && frame.Workspace == "analyse", fault, "the typed command did not run");
        frame.Press("F1"); palette.Type("");
        Require(palette.Rows[0].Command == "workspace.set" && palette.Rows[0].Argument == "layout:analyse" && palette.Rows[0].Keys == "3", fault, "the used row is not first with its key");
        Require(frame.Press("Escape") && !palette.IsOpen && frame.Press("1") && frame.Workspace == "solve", fault, "single keys did not act");
        string theme = frame.Tokens.Theme.Id;
        Require(frame.Press("T") && frame.Tokens.Theme.Id != theme && frame.Press("T") && frame.Tokens.Theme.Id == theme, fault, "the theme key did not step the families");
        Require(frame.Press("Ctrl+Z") && frame.Notice.Visible && frame.Notice.Text.StartsWith("undo t-", StringComparison.Ordinal) && frame.Press("Ctrl+Y"), fault, "a key gave no feedback in place");
        // The shortcut sheet lists every binding of the keymap; while it is open only closing keys act.
        var sheet = (ShortcutSheet)frame.Overlays.FindChild("ShortcutSheet", true, false);
        Require(frame.Press("Shift+Slash") && sheet.IsOpen && sheet.Shown == frame.Keys!.Bindings.Count && sheet.Shown > 15, fault, "the shortcut sheet does not list the keymap");
        Require(!frame.Press("2") && frame.Workspace == "solve" && frame.Press("F1") && palette.IsOpen && !sheet.IsOpen, fault, "two overlays were open, or a key acted under one");
        Require(frame.Press("Shift+Slash") && sheet.IsOpen && !palette.IsOpen && frame.Press("Escape") && frame.OpenOverlay == null, fault, "Esc did not close the sheet");
        frame.Flush();
        Button second = frame.FindChildren("*", "Button", true, false).Cast<Button>().Single(b => b.HasMeta("workspace") && b.GetMeta("workspace").AsString() == "script");
        Require(second.TooltipText.EndsWith("(2)", StringComparison.Ordinal), fault, "a button's tooltip does not name its key");
        // A radial menu opens on its key around the pointer; a number picks an entry and closes it.
        var radial = (RadialOverlay)frame.Overlays.FindChild("RadialOverlay", true, false);
        Require(frame.Press("W") && radial.IsOpen && radial.Menu!.Id == "radial.workspace" && radial.Entries.Count == radial.Menu.Entries.Count && radial.Entries.Count >= 4, fault, "the radial menu did not open");
        Require(radial.Entries.Select(b => b.Position).Distinct().Count() == radial.Entries.Count && frame.Press("2") && !radial.IsOpen && frame.Workspace == "script", fault, "a radial entry was not chosen by its number");
        Require(frame.Press("W") && frame.Press("Escape") && !radial.IsOpen && frame.Workspace == "script" && frame.Press("1") && frame.Workspace == "solve", fault, "Esc did not close the radial menu");
        // Preview and commit: one pending preview in its own view, the pair keeps the committed state, a
        // replaced preview cannot be committed by the same key-down, and a refused one never commits.
         double now = 100; demo.Clock = () => now;
        var commitBar = (CommitBar)frame.FindChild("CommitBar", true, false);
        int marks = frame.Global.Marks.GetChildCount(), entries = demo.Moves.Entries.Count;
        Require(commitBar.Commit.Disabled && frame.Press("Ctrl+Enter") && demo.Moves.Entries.Count == entries && frame.IsAncestorOf(demo.History), fault, "a commit ran without a preview");
        ((Button)commitBar.Tools.FindChild("StageSample", true, false)).EmitSignal(BaseButton.SignalName.Pressed); frame.Flush();
        Require(demo.Preview.Pending is { Id: 3, Status: PreviewStatus.Ready } && frame.Model.IsOpen(LabDemo.PreviewPane) && frame.IsAncestorOf(demo.PreviewView) &&
            frame.Global.Marks.GetChildCount() == marks + 1 && frame.Global.Slot.IsAncestorOf(app.GlobalStage) && !commitBar.Commit.Disabled, fault, "the preview is not in its own view with the pair marked pending");
        demo.Execute("operation.preview", "sample:1");
        if (fault == "a5-settle-ignored") now += 1;
        Require(demo.Preview.Replaced && demo.Preview.Pending!.Id == 4 && frame.Press("Ctrl+Enter") && demo.Preview.Pending != null && demo.Moves.Entries.Count == entries, fault, "a preview was committed as it replaced another");
        now += 1; int moves = demo.Preview.Pending!.Moves.Count;
        Require(frame.Press("Left") && demo.Preview.Scrub == moves - 1 && frame.Press("Ctrl+Enter") && demo.Preview.Pending == null &&
            demo.Moves.Entries.Count == entries + 1 && demo.Moves.Entries[^1].Moves.Count == moves - 1 && demo.Moves.Entries[^1].Origin == "script", fault, "the prefix up to the scrub was not committed as one entry");
        demo.Execute("operation.preview", "sample:3"); now += 1;
        Require(commitBar.Commit.Disabled && commitBar.Reason.Text.Contains("protected", StringComparison.Ordinal) && frame.Press("Ctrl+Enter") && demo.Moves.Entries.Count == entries + 1 &&
            frame.Press("Escape") && demo.Preview.Pending == null, fault, "an impossible preview was committed, or Esc did not discard it");
        // Two histories: an undo in one never moves the other; an abandoned redo is kept as a branch.
        int layoutAt = demo.Layout.Cursor, cursor = demo.Moves.Cursor;
        Require(frame.Press("Ctrl+Z") && demo.Moves.Cursor == cursor - 1 && frame.Press("Ctrl+Y") && demo.Moves.Cursor == cursor && demo.Layout.Cursor == layoutAt, fault, "the move history keys did not act alone");
        string themeWas = frame.Tokens.Theme.Id;
        Require(frame.Press("T") && demo.Layout.Cursor == layoutAt + 1 && demo.Layout.Steps[^1].Name.StartsWith("Appearance: theme", StringComparison.Ordinal) &&
            frame.Press("Ctrl+Alt+Z") && frame.Tokens.Theme.Id == themeWas && demo.Layout.Cursor == layoutAt && demo.Moves.Cursor == cursor &&
            frame.Press("Ctrl+Alt+Y") && frame.Tokens.Theme.Id != themeWas && frame.Press("Ctrl+Alt+Z") && frame.Tokens.Theme.Id == themeWas, fault, "the layout history did not undo and redo the theme alone");
        demo.Execute("session.undo.partial", null); bool inside = demo.Moves.Inner == 1;
        demo.Execute("session.rollback", "2"); demo.Execute("operation.preview", "sample:2"); now += 1;
        Require(inside && demo.Moves.Cursor == 2 && frame.Press("Ctrl+Enter") && demo.Moves.Entries.Count == 3 && demo.Moves.Branches.Count == 1 &&
            demo.Moves.Branches[0].Entries.Count == entries - 1 && demo.History.List.GetChildren().Count(c => c.HasMeta("entry")) == 3, fault, "the abandoned redo steps were not kept as a branch");
        Require(app.CaptureView().Ui["frame.palette"].Contains("use|workspace.set|layout:analyse|1|", StringComparison.Ordinal), fault, "palette use is not kept");
        Library(frame, demo, fault);
        Scripts(frame, fault);
        string before = frame.Model.Store();
        bool refused = !frame.Model.Drop("editor", FrameModel.Pair, DropZone.Tab) && !frame.Model.Close(FrameModel.Pair);
        if (fault == "a5-pair-hidden") refused = false;
        frame.Flush();
        Require(refused && frame.Model.Store() == before && frame.Notice.Visible && frame.Notice.Text.Contains("Global and Local", StringComparison.Ordinal),
            fault, "a change that hides a main view was not refused on screen");
        Require(frame.SelectWorkspace("script") && frame.Model.IsOpen("editor") && frame.Model.PairArrangement == "side", fault, "the Script workspace was not applied");
        frame.Flush();
        Require(!frame.WorkspaceModified && frame.FindChild("WorkspaceReset", true, false) == null, fault, "a fresh workspace is marked as rearranged");
        PaneNode centre = frame.Model.Centre;
        Require(frame.Model.Resize(centre, .97) && centre.Ratio == .9 && frame.Model.SetPairArrangement("stacked", .3), fault, "a splitter drag was not kept in range");
        frame.Flush();
        Require(frame.WorkspaceModified && frame.FindChild("WorkspaceReset", true, false) != null, fault, "a rearranged workspace is not marked");
        Require(frame.SelectWorkspace("script") && !frame.WorkspaceModified && frame.Model.PairArrangement == "side", fault, "reset to preset failed");
        frame.Model.SetFocusMode(true); frame.Flush();
        Require(frame.FindChild("TopBar", true, false) == null && frame.Global.Slot.IsAncestorOf(app.GlobalStage) && !frame.IsAncestorOf(app.ParameterPanel),
            fault, "focus mode shows more than the pair");
    }

    // B-6 of A6: Solve opens the Library on the left and keeps History in the right dock. Real key events in the
    // Library's filter: a typed name filters in place, Up and Down move, Enter adds the macro to Core as one change
    // and Shift+Enter previews it alone; nothing is committed.
    private static void Library(ChromeFrame frame, LabDemo demo, string fault)
    {
        frame.Flush();
        LabSolve solve = LabFrame.SolveOf(frame); LibraryPanel library = solve.LibraryView;
        Require(frame.Workspace == "solve" && frame.Model.Docks[DockSide.Left].Open == "library" && frame.Model.Docks[DockSide.Right].Open == "history" &&
            frame.IsAncestorOf(library) && frame.IsAncestorOf(demo.History), fault, "Solve does not open the Library on the left with History on the right");
        demo.Preview.Discard(); solve.Draft.Clear();
        int entries = demo.Moves.Entries.Count, changes = 0;
        Viewport port = frame.GetViewport();
        void Type(Godot.Key key, bool shift = false, char text = '\0')
        {
            foreach (bool down in new[] { true, false }) port.PushInput(new InputEventKey { Keycode = key, Pressed = down, ShiftPressed = shift, Unicode = down ? text : 0 });
        }
        library.Focus();
        foreach (char c in "swap") Type(Godot.Key.A + (c - 'a'), false, c);
        Type(Godot.Key.Down); Type(Godot.Key.Up);
        void Count() => changes++;
        solve.Draft.Changed += Count; Type(Godot.Key.Enter); solve.Draft.Changed -= Count;
        LibraryRecord swap = solve.Library.Records.Single(r => r.Name == "Swap pair B");
        string seen = $"filter '{library.Filter.Text}' (editing {library.Filter.IsEditing()}, focus {library.Filter.HasFocus()}), {solve.Library.Shown.Count} shown, {changes} draft changes, route {solve.Draft.Route}, Core {solve.Draft.Twists(1).Count} twists, " +
            $"preview {demo.Preview.Pending?.Source ?? "none"} {demo.Preview.Pending?.Status}, notice '{frame.Notice.Text}'";
        Require(library.Filter.Text == "swap" && solve.Library.Shown.Count == 1 && changes == 1 && solve.Draft.Active == 1 && solve.Draft.Twists(1).SequenceEqual(swap.Word) &&
            demo.Preview.Pending is { Source: "draft", Status: PreviewStatus.Ready } && demo.Moves.Entries.Count == entries, fault, "a filtered Library macro did not join Core from the keyboard: " + seen);
        Type(Godot.Key.Enter, true);
        Require(demo.Preview.Pending is { Source: "library" } alone && alone.Moves.SequenceEqual(swap.Word) && solve.Draft.Twists(1).SequenceEqual(swap.Word) && demo.Moves.Entries.Count == entries,
            fault, "Shift+Enter did not preview the macro alone, or the draft lost its twists");
        // A6 re-check N-5: with nothing matching the filter, Shift+Enter previews nothing; the pending preview stays.
        Preview kept = demo.Preview.Pending!;
        foreach (char c in "zz") Type(Godot.Key.A + (c - 'a'), false, c);
        Type(Godot.Key.Enter, true);
        Require(solve.Library.Shown.Count == 0 && demo.Preview.Pending is { } still && still.Id == kept.Id && still.Moves.SequenceEqual(kept.Moves) && demo.Moves.Entries.Count == entries &&
            frame.Notice.Text.Contains(LibraryModel.NoSelection, StringComparison.Ordinal), fault, $"Shift+Enter with nothing selected changed the preview: notice '{frame.Notice.Text}'");
        demo.Preview.Discard(); solve.Draft.Clear(); solve.Library.SetFilter(""); library.Filter.ReleaseFocus(); library.Filter.Text = ""; frame.Flush();
        Require(demo.Preview.Pending == null && solve.Draft.Count == 0 && frame.IsAncestorOf(demo.History), fault, "the Library step left a preview or a draft behind");
        GD.Print($"LOOKLAB_A6_LIBRARY filter=swap core={swap.Word.Count} changes={changes} history=right");
    }

    // The script interface: entry points are commands, a run ends in a preview or a read result and never in a
    // commit, a key binds to an entry point over the default keymap, a panel mounts in a slot and keeps it.
    private static bool Drained(LabScripts scripts) { scripts.Drain(); return true; }

    private static void Scripts(ChromeFrame frame, string fault)
    {
        LabScripts scripts = LabFrame.ScriptsOf(frame); LabDemo demo = LabFrame.DemoOf(frame); SampleInspect inspect = LabFrame.InspectOf(frame);
        Require(scripts.Programs.Count == 4 && scripts.Find("script.sweep.ring") is { Kind: ScriptKind.Preview } && scripts.Find("script.census.orbits") is { Kind: ScriptKind.Read } &&
            scripts.Find("script.layout.stacked") is { Kind: ScriptKind.View } && scripts.Programs.All(p => SixCs.Diagnose(p.Source).Count == 0), fault, "the sample programs or their entry points are missing");
        demo.Preview.Discard(); int entries = demo.Moves.Entries.Count;
        scripts.Open("sweep", scripts.Find("script.sweep.ring")!.Line + 1); frame.Flush();
        Require(frame.Model.IsOpen("editor") && frame.IsAncestorOf(scripts.Editor) && scripts.Editor.EntryAtCaret()?.Name == "ring", fault, "the editor does not show the program at its entry point");
        scripts.Editor.RunAtCaret(); scripts.Drain();
        Require(demo.Preview.Pending != null && inspect.Feedback.Console.Contains("sweep.ring();") && demo.Moves.Entries.Count == entries, fault, "a run from the editor did not end in a preview");
        scripts.RunLine("commit sweep.ring(4)");
        Require(demo.Moves.Entries.Count == entries && frame.Notice.Text.Contains("never commits", StringComparison.Ordinal), fault, "a typed line committed");
        demo.Preview.Discard();
        inspect.Console.Enter("sweep.reach(4)"); scripts.Drain();
        Require(scripts.LastResult != null && scripts.LastResult.StartsWith("sweep.reach(4) = record", StringComparison.Ordinal) && scripts.Results.Of("script.sweep.reach") is { Since: null } &&
            inspect.Console.History.Lines.Count == 1 && demo.Preview.Pending == null, fault, "a read entry point gave no inline result");
        scripts.Editor.Replace(scripts.Editor.Open!.Source + "export fn bad() -> Nat effects [commit] {\n");
        Require(scripts.Editor.Diagnostics.Any(d => d.Severity == Severity.Error && d.Text.Contains("never commits", StringComparison.Ordinal)) &&
            inspect.Feedback.Problems.Any(p => p.Source == "sweep.6cs" && p.Text.Contains("never closed", StringComparison.Ordinal)) && scripts.Find("script.sweep.bad") != null,
            fault, "an edited program was not checked beside the code");
        scripts.Editor.Replace(scripts.Programs.Single(p => p.Id == "sweep").Source.Replace("export fn bad() -> Nat effects [commit] {\n", ""));
        Require(scripts.Editor.Diagnostics.Count == 0 && scripts.Find("script.sweep.bad") == null && inspect.Feedback.Problems.All(p => !p.Text.Contains("never closed", StringComparison.Ordinal)), fault, "a mended program keeps its errors");
        // A key bound to an entry point runs it; the commit key is never given to a run.
        scripts.Capture("script.layout.stacked");
        Require(scripts.List.Binding == "script.layout.stacked" && frame.Press("F6") && scripts.List.Binding == null && frame.Keys!.KeysFor("script.layout.stacked", null) == "F6", fault, "a key was not bound to an entry point");
        frame.Model.SetPairArrangement("side", .5);
        Require(frame.Press("F6") && Drained(scripts) && frame.Model.PairArrangement == "stacked" && !scripts.Bind("Ctrl+Enter", "script.sweep.ring") && demo.Preview.Pending == null, fault, "the bound key did not run its entry point");
        Require(scripts.Store().Contains("\"key\": \"F6\"", StringComparison.Ordinal) && !scripts.Store().Contains("command.find", StringComparison.Ordinal), fault, "own bindings are not kept apart from the defaults");
        scripts.SetPreset("left-hand");
        Require(frame.Keys!.KeysFor("command.find", null).Contains("Q", StringComparison.Ordinal) && frame.Keys.KeysFor("script.layout.stacked", null) == "F6", fault, "the keymap preset was not applied under the own bindings");
        string kept = scripts.Store(); scripts.Defaults();
        Require(frame.Keys!.KeysFor("script.layout.stacked", null) == "" && !frame.Keys.KeysFor("command.find", null).Contains("Q", StringComparison.Ordinal), fault, "the default keymap did not return");
        scripts.Restore(kept);
        Require(scripts.Store() == kept && frame.Keys!.KeysFor("script.layout.stacked", null) == "F6", fault, "the stored keymap did not come back");
        scripts.Defaults();
        // A declared panel mounts in a slot; a panel whose program is missing keeps its slot and says so.
        scripts.RunEntry("script.census.orbits"); frame.Flush();
        Require(scripts.Slots.Shown(0) == LabScripts.CensusPanel && frame.Model.Docks[DockSide.Right].Open == "slot.1" &&
            frame.FindChild("ScriptPanel_census_orbits", true, false) is ScriptPanelHost { Spec: not null }, fault, "the declared panel is not in a slot");
        scripts.Slots.Restore("ghost.panel:0|" + LabScripts.CensusPanel + ":0"); frame.Model.ShowDock(DockSide.Right, "slot.1"); frame.Flush();
        Require(frame.FindChild("ScriptPanel_ghost_panel", true, false) is ScriptPanelHost { Spec: null } && scripts.Slots.Mount("third") >= 0 && scripts.Slots.Tabs(scripts.Slots.SlotOf("third")).Count == 2, fault, "a panel without its program lost its slot");
        scripts.Slots.Restore("|");
        scripts.RunEntry("script.census.fibres"); frame.Flush();
        Require(scripts.Canvas.Marks.Count == 24 && frame.Model.IsOpen("canvas") && frame.IsAncestorOf(scripts.Canvas), fault, "the canvas shows no drawing");
        frame.Model.Close("canvas"); frame.Flush();
    }

    private static void Same(ViewState a, ViewState b, ParameterSchema schema, string fault, string step)
    {
        Require(a.Q.SequenceEqual(b.Q) && a.Spin.SequenceEqual(b.Spin), fault, "camera not kept across " + step);
        Require(Presets.CanonicalBytes(a.Preset, schema).AsSpan().SequenceEqual(Presets.CanonicalBytes(b.Preset, schema)), fault, "look not kept across " + step);
        Require(a.Ui.Count == b.Ui.Count && a.Ui.All(e => b.Ui.TryGetValue(e.Key, out string? v) && v == e.Value), fault, "panel state not kept across " + step);
    }

    private static void Require(bool condition, string fault, string message)
    {
        if (!condition) throw new Exception((fault.Length > 0 ? fault : "a5-live") + ": " + message);
    }
}
