using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Godot;
using LookLab.Core;

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
        Build = r => new LabApp(r);
        SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        App = Build(root); AddChild(App);
        Notice = new Label { Name = "LiveNotice", Visible = false, MouseFilter = MouseFilterEnum.Ignore, Position = new Vector2(8, 6) };
        Notice.AddThemeColorOverride("font_outline_color", Colors.Black); Notice.AddThemeConstantOverride("outline_size", 6);
        AddChild(Notice);
        watch = new DataWatch(root, WatchedFolders, WatchedFiles);
        if (File.Exists(stateFile))
        {
            try
            {
                saved = File.ReadAllBytes(stateFile);
                App.RestoreView(ViewStates.Parse(System.Text.Encoding.UTF8.GetString(saved), App.State.Schema));
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
        LabApp? next = null;
        try
        {
            next = Build(root);
            AddChild(next); MoveChild(next, 0);
            next.RestoreView(kept);
        }
        catch (Exception ex)
        {
            if (next != null) { if (next.GetParent() == this) RemoveChild(next); next.QueueFree(); }
            Refusals++;
            Say($"Refused {what}: {ex.Message} (showing the last good data)", true);
            return false;
        }
        LabApp old = App; App = next;
        RemoveChild(old); old.QueueFree();
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
        GD.Print($"LOOKLAB_A5_LIVE_PASS reloads={session.Reloads} refusals={session.Refusals} restart=kept parts={before.Ui.Count} checks=1");
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
