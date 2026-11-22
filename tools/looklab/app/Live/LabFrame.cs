using System;
using System.IO;
using System.Linq;
using Godot;
using Magic600.Chrome;

namespace LookLab.App.Live;

// Look Lab inside the ADR-D01 window frame: the lab's Global and Local views fill the locked pair and its
// parameter panel is the "Tune" panel of the right dock. The frame, its tokens and its workspaces are the
// chrome library; only this file knows about the lab.
internal static class LabFrame
{
    public const string Tune = "tune", Greybox = "greybox";
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
        ThemeFamily theme = Directory.GetFiles(Path.Combine(folder, "themes"), "*.json").OrderBy(p => p, StringComparer.Ordinal)
            .Select(p => ThemeFamily.Load(p, design)).First();
        var tokens = new TokenSet(design, theme, design.ShippedDensity, 1);
        var frame = new ChromeFrame(tokens, WorkspaceSet.Load(Path.Combine(folder, "workspaces.json")));
        app.EnterFrame();
        foreach (var panel in Panels) frame.AddDockPanel(panel.Side, panel.Id, panel.Title, panel.Glyph, ChromeWidgets.Empty(panel.Title, panel.Hint));
        frame.AddDockPanel(DockSide.Right, Tune, "Tune", "~", app.ParameterPanel);
        foreach (var pane in Panes) frame.AddPane(pane.Id, pane.Title, ChromeWidgets.Empty(pane.Title, pane.Hint));
        frame.AddPane(Greybox, "Greybox", app.Greybox.Canvas);
        frame.SetMainViews(app.GlobalStage, app.Linked.LocalStage!);
        frame.Health.Text = "Look Lab · full detail · 259,800 slots";
        frame.SelectWorkspace(frame.Workspaces.Order[0]);
        // The lab opens on its own panel; every other default comes from the workspace.
        frame.Model.ShowDock(DockSide.Right, Tune);
        // The greybox canvas is a pane beside the pair while its switch is on.
        void ShowGreybox(bool on)
        {
            if (on && !frame.Model.IsOpen(Greybox)) frame.Model.Drop(Greybox, FrameModel.Pair, DropZone.Left);
            else if (!on && frame.Model.IsOpen(Greybox)) frame.Model.Close(Greybox);
        }
        app.Greybox.Toggle.Toggled += ShowGreybox;
        frame.Rebuilt += () => { if (app.Greybox.Toggle.ButtonPressed != frame.Model.IsOpen(Greybox)) app.Greybox.Toggle.ButtonPressed = frame.Model.IsOpen(Greybox); };
        app.ViewParts["frame.workspace"] = (() => frame.Workspace, v => { if (frame.Workspaces.Stored.ContainsKey(v)) frame.SelectWorkspace(v); });
        app.ViewParts["frame.zlayout"] = (() => frame.Model.Store(), v => frame.Model.Apply(v));
        frame.Flush();
        return frame;
    }
}
