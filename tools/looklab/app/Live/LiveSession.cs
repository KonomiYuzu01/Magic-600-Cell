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
    private double sincePoll, sinceSave;
    private byte[] saved = Array.Empty<byte>();

    public LiveSession(string root, string stateFolder)
    {
        this.root = root; stateFile = Path.Combine(stateFolder, "view.json");
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

    public override void _Process(double delta)
    {
        sincePoll += delta; sinceSave += delta;
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
        if (what == NotificationWMCloseRequest || what == NotificationPredelete) { if (IsInstanceValid(App)) Save(); }
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
        GD.Print($"LOOKLAB_A5_LIVE_PASS reloads={session.Reloads} refusals={session.Refusals} restart=kept parts={before.Ui.Count} checks=6");
    }

    // The window frame around the lab: both main views in the locked pair, the lab's panel in the right dock,
    // and a drop that would hide a main view refused with its reason on screen.
    private static void Framed(LiveSession session, string fault)
    {
        ChromeFrame frame = session.Frame; LabApp app = session.App;
        Require(frame.Global.Slot.IsAncestorOf(app.GlobalStage) && frame.Local.Slot.IsAncestorOf(app.Linked.LocalStage!) && app.Linked.LocalStage!.Visible,
            fault, "a main view is missing from the locked pair");
        Require(frame.Model.Docks[DockSide.Right].Open == LabFrame.Tune && frame.IsAncestorOf(app.ParameterPanel), fault, "the Tune panel is not in the right dock");
        Require(frame.Model.IsOpen(LabFrame.Greybox) && frame.IsAncestorOf(app.Greybox.Canvas), fault, "the open greybox is not a pane of the frame");
        Require(frame.FindChild("TopBar", true, false) != null && frame.Workspaces.Order.SequenceEqual(new[] { "solve", "script", "analyse", "reference" }), fault, "top bar or workspaces missing");
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
        // Icons: every rail button names an icon of the set; with the icon font installed it shows the glyph.
        Button[] rail = frame.FindChildren("*", "Button", true, false).Cast<Button>().Where(b => b.HasMeta("dock_panel") && b.HasMeta("icon")).ToArray();
        Require(rail.Length == 6 && rail.All(b => ChromeIcons.Set!.Icons.ContainsKey(b.GetMeta("icon").AsString()) &&
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
        Require(frame.Press("Ctrl+Z") && frame.Notice.Visible && frame.Notice.Text.Contains("no engine", StringComparison.Ordinal), fault, "an unavailable key gave no reason in place");
        Require(app.CaptureView().Ui["frame.palette"].Contains("use|workspace.set|layout:analyse|1|", StringComparison.Ordinal), fault, "palette use is not kept");
        string before = frame.Model.Store();
        bool refused = !frame.Model.Drop("editor", FrameModel.Pair, DropZone.Tab) && !frame.Model.Close(FrameModel.Pair);
        if (fault == "a5-pair-hidden") refused = false;
        frame.Flush();
        Require(refused && frame.Model.Store() == before && frame.Notice.Visible && frame.Notice.Text.Contains("Global and Local", StringComparison.Ordinal),
            fault, "a change that hides a main view was not refused on screen");
        Require(frame.SelectWorkspace("script") && frame.Model.IsOpen("editor") && frame.Model.PairArrangement == "stacked", fault, "the Script workspace was not applied");
        frame.Flush();
        Require(!frame.WorkspaceModified && frame.FindChild("WorkspaceReset", true, false) == null, fault, "a fresh workspace is marked as rearranged");
        PaneNode centre = frame.Model.Centre;
        Require(frame.Model.Resize(centre, .97) && centre.Ratio == .9 && frame.Model.SetPairArrangement("side", .3), fault, "a splitter drag was not kept in range");
        frame.Flush();
        Require(frame.WorkspaceModified && frame.FindChild("WorkspaceReset", true, false) != null, fault, "a rearranged workspace is not marked");
        Require(frame.SelectWorkspace("script") && !frame.WorkspaceModified && frame.Model.PairArrangement == "stacked", fault, "reset to preset failed");
        frame.Model.SetFocusMode(true); frame.Flush();
        Require(frame.FindChild("TopBar", true, false) == null && frame.Global.Slot.IsAncestorOf(app.GlobalStage) && !frame.IsAncestorOf(app.ParameterPanel),
            fault, "focus mode shows more than the pair");
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
