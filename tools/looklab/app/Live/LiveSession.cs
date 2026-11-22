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
        GD.Print($"LOOKLAB_A5_LIVE_PASS reloads={session.Reloads} refusals={session.Refusals} restart=kept parts={before.Ui.Count} checks=3");
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
