using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using Magic600.Chrome;

namespace LookLab.App.Live;

// The whole frame, used: time runs past the end of the sample task, every command Look Lab can run goes through
// the entry the palette, the keys and the radial menus use, every dock, pane, workspace, theme, look and density
// is shown, and every visible button takes a real mouse press through the root viewport. Any logged error or
// caught listener fails the check, and so does a button whose own signal did not fire.
internal static class Sweep
{
    public const string Mode = "a5-sweep";
    // Buttons that write a file or open a native dialog are pressed by their commands' own checks, not here.
    private static readonly string[] Skipped = { "KeymapExport" };

    // A look at the real window from a fresh start: the keys given are pressed in turn, then the window is
    // pictured into the scratch folder and closed.
    public const string LookMode = "a5-look";
    public static async Task Look(Node host, string root, string scratch, string keys)
    {
        string folder = Path.Combine(scratch, LookMode);
        LabErrors.Install(Path.Combine(folder, "errors.log")); LabErrors.Reset();
        var session = new LiveSession(root, folder); host.AddChild(session);
        SceneTree tree = host.GetTree();
        for (int i = 0; i < 90; i++) await host.ToSignal(tree, SceneTree.SignalName.ProcessFrame);
        foreach (string key in keys.Split('+', StringSplitOptions.RemoveEmptyEntries))
        {
            session.Frame.Press(key);
            for (int i = 0; i < 40; i++) await host.ToSignal(tree, SceneTree.SignalName.ProcessFrame);
        }
        Image picture = session.GetViewport().GetTexture().GetImage();
        if (picture.GetWidth() > 1300) picture.Resize(1200, picture.GetHeight() * 1200 / picture.GetWidth(), Image.Interpolation.Lanczos);
        picture.SavePng(Path.Combine(folder, "look.png"));
        GD.Print($"LOOKLAB_A5_LOOK errors={LabErrors.Count}" + (LabErrors.Count > 0 ? " first=" + LabErrors.All[0].Split('\n')[0] : ""));
    }

    public static async Task Run(Node host, string root, string scratch, string fault)
    {
        string folder = Path.Combine(scratch, Mode);
        string tracePath = OS.GetEnvironment("LOOKLAB_SWEEP_TRACE"); bool trace = tracePath.Length > 0;
        long began = System.Environment.TickCount64;
        void Trace(string text) { if (trace) File.AppendAllText(tracePath, (System.Environment.TickCount64 - began) + " " + text + System.Environment.NewLine); }
        Trace("start");
        if (Directory.Exists(folder)) Directory.Delete(folder, true);
        LabErrors.Install(Path.Combine(folder, "errors.log")); LabErrors.Reset(); Trace("errors installed");
        var session = new LiveSession(root, folder); host.AddChild(session);
        SceneTree tree = host.GetTree();
        async Task Frames(int count = 1) { for (int i = 0; i < count; i++) await host.ToSignal(tree, SceneTree.SignalName.ProcessFrame); }
        void Clean(string step)
        {
            Trace("step " + step);
            if (LabErrors.Count > 0) throw new Exception((fault.Length > 0 ? fault : Mode) + $": {LabErrors.Count} error(s) after {step}; the first: {LabErrors.All[0].Split('\n')[0]}");
        }
        tree.Root.Size = new Vector2I(1600, 900); await Frames(3);
        Trace("window " + session.GetViewport().GetVisibleRect() + " frame " + session.Frame.GetGlobalRect());
        ChromeFrame frame = session.Frame; LabKit kit = LabFrame.KitOf(frame); LabInspect inspect = LabFrame.InspectOf(frame);
        if (fault == "a5-missing-token") frame.Tokens.C("no.such.token");
        if (fault == "a5-listener-throws") inspect.Feedback.Changed += () => throw new InvalidOperationException("injected listener failure");
        // The ten-second sample task ends; a command must still run after it.
        inspect.Tick(11); await Frames(2);
        kit.Run("workspace.set", "layout:script"); await Frames(2);
        if (frame.Workspace != "script") throw new Exception((fault.Length > 0 ? fault : Mode) + ": a command did not run after the sample task ended");
        Clean("the sample task");
        void Calm()
        {
            frame.OpenOverlay?.Close(); frame.KeyCapture = null;
            (frame.Overlays.FindChild("KeyMap", false, false) as KeyMapPanel)?.Shown.Pin(false);
            if (frame.Model.FocusMode) frame.Model.SetFocusMode(false);
            if (frame.Model.PairMaximised) frame.Model.SetPairMaximised(false);
        }
        int commands = 0;
        foreach (PaletteRow row in kit.Commands.Rows("").Where(r => r.Unavailable == null && r.Command != "file.keymap.export").ToArray())
        {
            kit.Run(row.Command, row.Argument); commands++;
            await Frames(1); Calm(); await Frames(1);
            Clean("command " + row.Id);
        }
        foreach (string workspace in frame.Workspaces.Order)
        {
            frame.SelectWorkspace(workspace); await Frames(2);
            foreach (DockSide side in Enum.GetValues<DockSide>())
                foreach (string panel in frame.Model.Docks[side].Panels.ToArray())
                {
                    frame.Model.ShowDock(side, panel); await Frames(2); Clean($"dock {panel} in {workspace}");
                    frame.Model.ToggleDock(side, panel); await Frames(1);
                }
            foreach (string pane in frame.PaneIds)
            {
                if (!frame.Model.IsOpen(pane)) frame.Model.Drop(pane, FrameModel.Pair, DropZone.Right);
                await Frames(2); Clean($"pane {pane} in {workspace}");
                frame.Model.Close(pane); await Frames(1);
            }
            frame.SelectWorkspace(workspace);
        }
        foreach (string theme in kit.Themes)
            foreach (string look in ThemeFamily.SceneLooks)
                foreach (string density in kit.Densities)
                {
                    kit.Appearance.Edit(a => { a.SetTheme(theme); a.SetSceneLook(look); a.SetDensity(density); }, true, true);
                    await Frames(2); Clean($"{theme} {look} {density}");
                }
        kit.Appearance.Edit(a => { a.SetTheme(kit.Themes[0]); a.SetSceneLook(ThemeFamily.SceneLooks[0]); a.SetDensity(kit.Densities[0]); }, true, true);
        // Real presses: every plain button that is on screen, in every workspace and with each right-dock panel open.
        int clicks = 0;
        var settings = new List<(string Workspace, string? Right)>();
        foreach (string workspace in frame.Workspaces.Order)
        {
            settings.Add((workspace, null));
            if (workspace == frame.Workspaces.Order[0]) settings.AddRange(frame.Model.Docks[DockSide.Right].Panels.Where(p => p != LabFrame.Tune).Select(p => (workspace, (string?)p)));
        }
        var counted = new HashSet<string>(StringComparer.Ordinal); var wordLimits = new Dictionary<string, int>(StringComparer.Ordinal);
        using (System.Text.Json.JsonDocument limits = System.Text.Json.JsonDocument.Parse(File.ReadAllText(Path.Combine(root, "tools/looklab/data/design/screen-words.json"))))
            foreach (System.Text.Json.JsonProperty p in limits.RootElement.GetProperty("words").EnumerateObject()) wordLimits[p.Name] = p.Value.GetInt32();
        foreach ((string workspace, string? right) in settings)
        {
            var pressed = new HashSet<string>(StringComparer.Ordinal);
            for (int guard = 0; ; guard++)
            {
                if (guard == 1 && counted.Add(workspace))
                {
                    // Words per screen: what a person reads in labels and on buttons, against the screen's limit.
                    int words = ScreenWords(frame, session.App);
                    if (!wordLimits.TryGetValue(workspace, out int limit)) throw new Exception($"{Mode}: the {workspace} screen has no word limit in screen-words.json");
                    GD.Print($"LOOKLAB_A5_WORDS workspace={workspace} words={words} limit={limit}");
                    if (words > limit) throw new Exception($"{Mode}: the {workspace} screen shows {words} words, over its limit of {limit}");
                }
                if (guard > 400) throw new Exception(Mode + ": the buttons of " + workspace + " never ran out");
                Calm(); frame.SelectWorkspace(workspace); if (right != null) frame.Model.ShowDock(DockSide.Right, right);
                await Frames(2);
                Button? next = Visible(frame, session.App).FirstOrDefault(x => !pressed.Contains(Key(frame, x)));
                if (next == null) break;
                pressed.Add(Key(frame, next));
                Button button = next; string path = Key(frame, button);
                Trace("press " + path);
                bool fired = false; Action heard = () => fired = true; button.Pressed += heard;
                Control? under = null; Vector2 at = default;
                // A second try reads the button's place again: the first may have met a layout still settling.
                for (int attempt = 0; attempt < 2 && !fired && GodotObject.IsInstanceValid(button) && button.IsVisibleInTree(); attempt++)
                {
                    await Frames(1 + 3 * attempt);
                    at = button.GetGlobalRect().GetCenter();
                    Input.ParseInputEvent(new InputEventMouseMotion { Position = at, GlobalPosition = at }); await Frames(1);
                    under = frame.GetViewport().GuiGetHoveredControl();
                    foreach (bool down in new[] { true, false })
                    {
                        Input.ParseInputEvent(new InputEventMouseButton { ButtonIndex = MouseButton.Left, Pressed = down, Position = at, GlobalPosition = at, ButtonMask = down ? MouseButtonMask.Left : 0 });
                        await Frames(1);
                    }
                }
                if (GodotObject.IsInstanceValid(button)) button.Pressed -= heard;
                if (!fired) throw new Exception((fault.Length > 0 ? fault : Mode) + $": a press on {path} in {workspace} did not reach it; the pointer was over {(under != null && GodotObject.IsInstanceValid(under) ? under.Name + " " + under.GetClass() + " " + under.GetGlobalRect() + " under " + Key(frame, under) : "nothing")} (pressed at {at})");
                clicks++;
                await Frames(1); Clean($"a press on {path} in {workspace}");
            }
        }
        // A script frame never holds a commit control (ADR-D01.58): the commit bar is the frame's own.
        foreach (PanelContainer script in frame.FindChildren("*", "PanelContainer", true, false).OfType<PanelContainer>().Where(p => p.ThemeTypeVariation.ToString().StartsWith("ScriptFrame", StringComparison.Ordinal)))
            if (script.FindChildren("*", "Button", true, false).OfType<Button>().Any(b => b.Text == "Commit" || b.TooltipText.StartsWith("Commit", StringComparison.Ordinal)))
                throw new Exception(Mode + ": a script frame holds a commit control");
        Calm(); await Frames(1);
        // The close request keeps the view and logs nothing.
        session.Closing(false); await Frames(1); Clean("the close request");
        if (!File.Exists(Path.Combine(folder, "view.json"))) throw new Exception((fault.Length > 0 ? fault : Mode) + ": the close request did not keep the view");
        GD.Print($"LOOKLAB_A5_SWEEP_PASS commands={commands} clicks={clicks} errors=0");
    }

    // A button's place in the frame: node names the frame gave, and the position among siblings where it gave none,
    // so the same control has the same key after a rebuild.
    private static string Key(Node frame, Node node)
    {
        var parts = new List<string>();
        for (Node? at = node; at != null && at != frame; at = at.GetParent())
        {
            string name = at.Name.ToString();
            parts.Add(name.StartsWith('@') ? at.GetClass() + "#" + at.GetIndex() : name);
        }
        parts.Reverse();
        return string.Join("/", parts);
    }

    // The words on screen: the text of every label, button and empty field that is shown inside the window. Text
    // drawn by an instrument itself (rail ids, keycaps) and tooltips are not counted.
    private static int ScreenWords(ChromeFrame frame, LabApp app)
    {
        Rect2 window = frame.GetViewport().GetVisibleRect();
        return TokenLint.Words(frame.FindChildren("*", "Control", true, false).OfType<Control>()
            .Where(c => c.IsVisibleInTree() && c.Size.X >= 2 && c.Size.Y >= 2 && window.HasPoint(c.GetGlobalRect().GetCenter()) && !app.ParameterPanel.IsAncestorOf(c) && !app.Greybox.Canvas.IsAncestorOf(c))
            .Select(c => c switch { Label l => l.Text, Button b => b.Text, LineEdit e => e.Text.Length == 0 ? e.PlaceholderText : "", _ => "" }));
    }

    private static Button[] Visible(ChromeFrame frame, LabApp app)
    {
        Rect2 window = frame.GetViewport().GetVisibleRect();
        bool Shown(Button b)
        {
            if (b.GetType() != typeof(Button) || b.Disabled || !b.IsVisibleInTree() || Skipped.Contains(b.Name.ToString()) || b.Size.X < 2 || b.Size.Y < 2) return false;
            Vector2 at = b.GetGlobalRect().GetCenter();
            if (!window.HasPoint(at) || app.ParameterPanel.IsAncestorOf(b) || app.Greybox.Canvas.IsAncestorOf(b)) return false;
            for (Node? up = b.GetParent(); up != null && up != frame; up = up.GetParent())
                if (up is Control c && (c.ClipContents || c is ScrollContainer) && !c.GetGlobalRect().HasPoint(at)) return false;
            return true;
        }
        return frame.FindChildren("*", "Button", true, false).OfType<Button>().Where(Shown).ToArray();
    }
}
