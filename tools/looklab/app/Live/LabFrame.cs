using System;
using System.IO;
using System.Linq;
using Godot;
using LookLab.Core;
using Magic600.Chrome;

namespace LookLab.App.Live;

// Look Lab inside the ADR-D01 window frame: the lab's Global and Local views fill the locked pair and its
// parameter panel is the "Tune" panel of the right dock. The frame, its tokens and its workspaces are the
// chrome library; only this file knows about the lab.
internal static class LabFrame
{
    public const string Tune = "tune", Greybox = "greybox", Look = "appearance", Tokens = "tokens";
    private static readonly (DockSide Side, string Id, string Title, string Glyph, string Hint)[] Panels =
    {
        (DockSide.Left, "history", "History", "H", "Puzzle history and layout history appear here."),
        (DockSide.Left, "scripts", "Scripts", "S", "The session's .6cs programs and panels appear here."),
        (DockSide.Right, "inspector", "Inspector", "I", "Select a cell, a sticker or a command to inspect it."),
        (DockSide.Bottom, "problems", "Problems", "P", "No problems."),
        (DockSide.Bottom, "console", "Console", "C", "The 600 Cell Script console appears here."),
        (DockSide.Bottom, "tasks", "Tasks", "T", "No task is running."),
    };
    private static readonly (string Id, string Title, string Hint)[] Panes =
    {
        ("editor", "Editor", "Open a .6cs program to edit it here."),
        ("preview", "Preview", "A pending preview is shown here, never in Global or Local."),
        ("canvas", "Canvas", "A script draws here, inside the script frame."),
        ("reference", "Reference", "Pinned references and nexus cards appear here."),
    };

    public static ChromeFrame Attach(LabApp app, string root)
    {
        string folder = Path.Combine(root, "tools/looklab/data/design");
        DesignSystem design = DesignSystem.Load(Path.Combine(folder, "system.json"));
        ThemeFamily[] themes = Directory.GetFiles(Path.Combine(folder, "themes"), "*.json").OrderBy(p => p, StringComparer.Ordinal)
            .Select(p => ThemeFamily.Load(p, design)).ToArray();
        var look = new Appearance(design, themes);
        var frame = new ChromeFrame(look.Resolve(), WorkspaceSet.Load(Path.Combine(folder, "workspaces.json")));
        app.EnterFrame();
        foreach (var panel in Panels) frame.AddDockPanel(panel.Side, panel.Id, panel.Title, panel.Glyph, ChromeWidgets.Empty(panel.Title, panel.Hint));
        frame.AddDockPanel(DockSide.Right, Tune, "Tune", "~", app.ParameterPanel);
        foreach (var pane in Panes) frame.AddPane(pane.Id, pane.Title, ChromeWidgets.Empty(pane.Title, pane.Hint));
        frame.AddPane(Greybox, "Greybox", app.Greybox.Canvas);
        frame.SetMainViews(app.GlobalStage, app.Linked.LocalStage!);
        // The theme button in the top bar steps through the families; the scene behind the puzzle follows the
        // family's scene look, written through the lab's own parameters and inside their ranges.
        Button themeButton = ChromeWidgets.Tool(look.Family.Name, "Theme family: click for the next one"); themeButton.Name = "ThemeButton";
        frame.BarTools.AddChild(themeButton);
        var appearance = new AppearancePanel(look);
        frame.AddDockPanel(DockSide.Right, Look, "Appearance", "A", appearance);
        void ApplyScene()
        {
            foreach (var value in look.Family.Scenes[look.SceneLook])
                app.State.Set(value.Key, ParameterSchema.ReadValue(app.State.Schema[value.Key], value.Value));
        }
        void ApplyLook(bool rebuild, bool scene)
        {
            themeButton.Text = look.Family.Name;
            frame.ApplyTokens(look.Resolve(), rebuild);
            if (scene) ApplyScene();
        }
        themeButton.Pressed += () =>
        {
            int next = (Array.FindIndex(themes, t => t.Id == look.Theme) + 1) % themes.Length;
            appearance.Edit(a => a.SetTheme(themes[next].Id), true, true);
        };
        appearance.Changed += ApplyLook;
        // The token specimen in the left dock follows every change of the appearance.
        var specimen = new TokenSpecimen();
        frame.AddDockPanel(DockSide.Left, Tokens, "Tokens", "D", specimen);
        void ShowTokens() { if (GodotObject.IsInstanceValid(specimen)) specimen.Show(frame.Tokens); }
        frame.Rebuilt += ShowTokens; frame.Recoloured += ShowTokens;
        app.ViewParts["frame.appearance"] = (() => look.Store(), v =>
        {
            try { look = Appearance.Parse(v, design, themes); appearance.Show(look); ApplyLook(true, false); }
            catch (FormatException) { }
        });
        frame.Health.Text = "Look Lab · full detail · 259,800 slots";
        frame.SelectWorkspace(frame.Workspaces.Order[0]);
        // The lab opens on its own panel; every other default comes from the workspace.
        frame.Model.ShowDock(DockSide.Right, Tune); frame.MarkBaseline();
        // The greybox canvas is a pane beside the pair while its switch is on.
        void ShowGreybox(bool on)
        {
            if (on && !frame.Model.IsOpen(Greybox)) frame.Model.Drop(Greybox, FrameModel.Pair, DropZone.Left);
            else if (!on && frame.Model.IsOpen(Greybox)) frame.Model.Close(Greybox);
        }
        app.Greybox.Toggle.Toggled += ShowGreybox;
        frame.Rebuilt += () => { if (app.Greybox.Toggle.ButtonPressed != frame.Model.IsOpen(Greybox)) app.Greybox.Toggle.ButtonPressed = frame.Model.IsOpen(Greybox); };
        // Status lines and frame marks. Both main views show the committed state; the 4D rotation link is named on both.
        void Describe()
        {
            var p = app.State.Preset.Params;
            string F(string id) => p.Number(id).ToString("0.00", System.Globalization.CultureInfo.InvariantCulture);
            foreach (ViewFrame view in new[] { frame.Global, frame.Local })
            {
                view.ClearMarks();
                view.AddMark("■", "committed", "state.committed", "This view shows the committed state.");
                view.AddMark("⇄", "rotation", "accent.strong", "4D rotation linked: Global ⇄ Local");
            }
            frame.Local.AddMark("◎", "auto-frame", "text.dim", "Local re-frames on the moving cells.");
            frame.Global.SetStatus(null, new[] { "drag: rotate 4D" }, $"gap {F("gap")}  shrink {F("stickerShrink")}");
            frame.Local.SetStatus("moving cells", new[] { "drag: rotate 4D" }, "follows Global");
        }
        frame.Rebuilt += Describe; frame.Recoloured += Describe;
        app.State.Changed += (_, _) => { if (GodotObject.IsInstanceValid(frame)) Describe(); };
        app.ViewParts["frame.workspace"] = (() => frame.Workspace, v => { if (frame.Workspaces.Stored.ContainsKey(v)) frame.SelectWorkspace(v); });
        app.ViewParts["frame.zlayout"] = (() => frame.Model.Store(), v => frame.Model.Apply(v));
        frame.Flush();
        return frame;
    }
}
