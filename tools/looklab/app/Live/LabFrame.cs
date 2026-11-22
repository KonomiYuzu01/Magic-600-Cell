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
    private static readonly System.Runtime.CompilerServices.ConditionalWeakTable<ChromeFrame, LabDemo> Demos = new();
    public static LabDemo DemoOf(ChromeFrame frame) => Demos.TryGetValue(frame, out LabDemo? demo) ? demo : throw new InvalidOperationException("the frame has no lab session");
    public const string Tune = "tune", Greybox = "greybox", Look = "appearance", Tokens = "tokens";
    private static readonly (DockSide Side, string Id, string Title, string Glyph, string Hint)[] Panels =
    {
        (DockSide.Left, "history", "History", "panel.history", "Puzzle history and layout history appear here."),
        (DockSide.Left, "scripts", "Scripts", "panel.scripts", "The session's .6cs programs and panels appear here."),
        (DockSide.Right, "inspector", "Inspector", "panel.inspector", "Select a cell, a sticker or a command to inspect it."),
        (DockSide.Bottom, "problems", "Problems", "panel.problems", "No problems."),
        (DockSide.Bottom, "console", "Console", "panel.console", "The 600 Cell Script console appears here."),
        (DockSide.Bottom, "tasks", "Tasks", "panel.tasks", "No task is running."),
    };
    private static readonly (string Id, string Title, string Hint)[] Panes =
    {
        ("editor", "Editor", "Open a .6cs program to edit it here."),
        ("preview", "Preview", "A pending preview is shown here, never in Global or Local."),
        ("canvas", "Canvas", "A script draws here, inside the script frame."),
        ("reference", "Reference", "Pinned references and nexus cards appear here."),
    };

    // Installed packages are looked for under tools/node of this checkout, of every folder above it (a copy of
    // the tree exported inside another checkout) and, from a linked worktree, of its main checkout.
    public static string[] PackageFolders(string root)
    {
        var folders = new System.Collections.Generic.List<string>();
        for (DirectoryInfo? at = new DirectoryInfo(Path.GetFullPath(root)); at != null; at = at.Parent) folders.Add(Path.Combine(at.FullName, "tools/node"));
        string link = Path.Combine(root, ".git");
        if (File.Exists(link))
        {
            string text = File.ReadAllText(link).Trim().Replace((char)92, '/'); int cut = text.IndexOf("/.git/worktrees/", StringComparison.Ordinal);
            if (text.StartsWith("gitdir: ", StringComparison.Ordinal) && cut > 8) folders.Add(Path.Combine(text.Substring(8, cut - 8), "tools/node"));
        }
        return folders.ToArray();
    }

    public static ChromeFrame Attach(LabApp app, string root)
    {
        ChromeFonts.SetRoots(PackageFolders(root));
        ChromeIcons.Use(IconSet.Load(Path.Combine(root, "tools/looklab/data/design/icons.json")));
        string folder = Path.Combine(root, "tools/looklab/data/design");
        DesignSystem design = DesignSystem.Load(Path.Combine(folder, "system.json"));
        ThemeFamily[] themes = Directory.GetFiles(Path.Combine(folder, "themes"), "*.json").OrderBy(p => p, StringComparer.Ordinal)
            .Select(p => ThemeFamily.Load(p, design)).ToArray();
        var look = new Appearance(design, themes);
        if (DisplayServer.IsDarkModeSupported()) look.FollowSystem(DisplayServer.IsDarkMode());
        // The chrome follows the scale of the screen it opens on (96 dpi is 1), in quarter steps.
        double screen = Math.Clamp(Math.Round(DisplayServer.ScreenGetDpi() / 96.0 * 4) / 4, 1, 3);
        look.SetSystemScale(screen);
        var frame = new ChromeFrame(look.Resolve(), WorkspaceSet.Load(Path.Combine(folder, "workspaces.json")));
        app.EnterFrame();
        foreach (var panel in Panels) frame.AddDockPanel(panel.Side, panel.Id, panel.Title, panel.Glyph, ChromeWidgets.Empty(panel.Title, panel.Hint));
        frame.AddDockPanel(DockSide.Right, Tune, "Tune", "panel.tune", app.ParameterPanel);
        foreach (var pane in Panes) frame.AddPane(pane.Id, pane.Title, ChromeWidgets.Empty(pane.Title, pane.Hint));
        frame.AddPane(Greybox, "Greybox", app.Greybox.Canvas);
        frame.SetMainViews(app.GlobalStage, app.Linked.LocalStage!);
        // The theme button in the top bar steps through the families; the scene behind the puzzle follows the
        // family's scene look, written through the lab's own parameters and inside their ranges.
        Button themeButton = ChromeWidgets.Tool(look.Family.Name, "Theme family: click for the next one"); themeButton.Name = "ThemeButton";
        frame.BarTools.AddChild(themeButton);
        var appearance = new AppearancePanel(look);
        frame.AddDockPanel(DockSide.Right, Look, "Appearance", "panel.appearance", appearance);
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
        // Commands: the registry is the command table, the keymap and the palette are generated from it. The
        // lab has no engine, so only the commands the frame itself carries out are available here.
        var registry = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(root, "docs/progress/1.0/command-table.json")));
        Keymap keymap = Keymap.Load(Path.Combine(folder, "keymap.json"), registry);
        string? Unavailable(string command, string? argument) => command switch
        {
            "command.find" or "transient.close" or "help.open" => null,
            _ when command.StartsWith("radial.", StringComparison.Ordinal) => null,
            "workspace.set" or "theme.select" or "density.preset.set" => argument != null ? null : "it needs an argument; choose one of its rows",
            _ when LabDemo.Handles(command) => null,
            _ => "Look Lab has no engine; this command runs in the product",
        };
        var commands = new PaletteModel(registry, keymap, Unavailable);
        var palette = new CommandPalette(commands, () => frame.Tokens);
        frame.Overlays.AddChild(palette);
        var sheet = new ShortcutSheet(registry, keymap, () => frame.Tokens); frame.Overlays.AddChild(sheet);
        var radial = new RadialOverlay(registry, Unavailable, () => frame.Tokens); frame.Overlays.AddChild(radial);
        frame.OverlayCommands.Add("transient.close"); frame.OverlayCommands.Add("help.open"); frame.OverlayCommands.Add("command.find");
        Button find = ChromeWidgets.Tool("", $"Find a command ({keymap.KeysFor("command.find", null)})"); find.Name = "FindButton";
        Button help = ChromeWidgets.Tool("", $"Keyboard shortcuts ({keymap.KeysFor("help.open", null)})"); help.Name = "ShortcutsButton";
        void Glyphs() { ChromeIcons.Apply(find, "search", frame.Tokens.Px("icon.small")); ChromeIcons.Apply(help, "help.keys", frame.Tokens.Px("icon.small")); }
        Glyphs(); frame.Rebuilt += Glyphs;
        find.Pressed += palette.Open; help.Pressed += sheet.Open;
        frame.BarTools.AddChild(find); frame.BarTools.AddChild(help);
        string Next(System.Collections.Generic.IReadOnlyList<string> ids, string current) => ids[(ids.ToList().IndexOf(current) + 1) % ids.Count];
        LabDemo demo = null!;
        void Execute(string command, string? argument)
        {
            if (Unavailable(command, argument) is { } why) { frame.Say($"{registry.Commands[command].Label}: {why}", true); return; }
            switch (command)
            {
                case "command.find": palette.Open(); break;
                case var menu when keymap.Radials.ContainsKey(menu): radial.Open(keymap.Radials[menu], frame.GetLocalMousePosition()); break;
                case "help.open": if (sheet.IsOpen) sheet.Close(); else sheet.Open(); break;
                case "transient.close":
                    if (frame.OpenOverlay is { } open) open.Close();
                    else if (frame.Model.FocusMode) frame.Model.SetFocusMode(false);
                    else if (frame.Model.PairMaximised) frame.Model.SetPairMaximised(false);
                    break;
                case "workspace.set" when argument == "layout:focus": frame.Model.SetFocusMode(!frame.Model.FocusMode); break;
                case "workspace.set" when argument == "layout:maximise": frame.Model.SetPairMaximised(!frame.Model.PairMaximised); break;
                case "workspace.set" when argument!.StartsWith("layout:", StringComparison.Ordinal):
                    if (!frame.SelectWorkspace(argument.Substring(7))) frame.Say("No workspace is named " + argument.Substring(7), true);
                    break;
                case "theme.select": appearance.Edit(a => a.SetTheme(Next(themes.Select(t => t.Id).ToArray(), a.Theme)), true, true); break;
                case "density.preset.set": appearance.Edit(a => a.SetDensity(Next(design.Densities, a.Density)), true, false); break;
                case var sample when LabDemo.Handles(sample): demo.Execute(command, argument); break;
                default: frame.Say(registry.Commands[command].Label + ": nothing to do for " + argument, true); break;
            }
        }
        palette.Run += row => Execute(row.Command, row.Argument);
        radial.Chosen += entry => Execute(entry.Command, entry.Argument);
        frame.Keys = keymap; frame.KeyCommand += binding => Execute(binding.Command, binding.Argument);
        // The sample session: commit bar under the views, the preview view and the History dock.
        demo = new LabDemo(frame, keymap, Execute); Demos.Add(frame, demo);
        frame.SetContent("history", demo.History); frame.SetContent(LabDemo.PreviewPane, demo.PreviewView); frame.SetStrip(demo.Bar);
        frame.Contexts = () => demo.Preview.Pending != null ? new[] { "preview" } : Array.Empty<string>();
        app.ViewParts["frame.palette"] = (() => commands.Store(), v => { try { commands.Restore(v); } catch (FormatException) { } });
        // The token specimen in the left dock follows every change of the appearance.
        var specimen = new TokenSpecimen();
        frame.AddDockPanel(DockSide.Left, Tokens, "Tokens", "panel.tokens", specimen);
        void ShowTokens() { if (GodotObject.IsInstanceValid(specimen)) specimen.Show(frame.Tokens); }
        frame.Rebuilt += ShowTokens; frame.Recoloured += ShowTokens;
        app.ViewParts["frame.appearance"] = (() => look.Store(), v =>
        {
            try { Appearance kept = Appearance.Parse(v, design, themes); kept.SetSystemScale(screen); look = kept; appearance.Show(look); ApplyLook(true, false); }
            catch (FormatException) { }
        });
        frame.Health.Text = "Look Lab · full detail · 259,800 slots";
        frame.SelectWorkspace(frame.Workspaces.Order[0]);
        // Every default comes from the workspace; the right dock starts collapsed (ADR-D01.22).
        frame.MarkBaseline();
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
                view.AddMark("mark.committed", "committed", "state.committed", "This view shows the committed state.");
                if (demo.Preview.Pending != null) view.AddMark("mark.pending", "preview pending", "state.pending", "A preview waits in the preview view; this view still shows the committed state.");
                view.AddMark("mark.link", "rotation", "accent.strong", "4D rotation linked: Global ⇄ Local");
            }
            frame.Local.AddMark("mark.autoframe", "auto-frame", "text.dim", app.Linked.Shape != null ? "Local stays on the moving cells and the cells around them. Its shape is the data file local-view.json." : "Local re-frames on the moving cells.");
            frame.Global.SetStatus(null, new[] { "drag: rotate 4D" }, $"gap {F("gap")}  shrink {F("stickerShrink")}");
            frame.Local.SetStatus(app.Linked.Shape != null ? $"{app.Linked.FocusCells} moving cells + neighbours" : "moving cells", new[] { app.Linked.Shape != null ? "drag: turn" : "drag: rotate 4D" }, app.Linked.Shape != null ? "own shape" : "follows Global");
        }
        frame.Rebuilt += Describe; frame.Recoloured += Describe;
        demo.Preview.Changed += () => { if (GodotObject.IsInstanceValid(frame)) Describe(); };
        app.State.Changed += (_, _) => { if (GodotObject.IsInstanceValid(frame)) Describe(); };
        app.ViewParts["frame.workspace"] = (() => frame.Workspace, v => { if (frame.Workspaces.Stored.ContainsKey(v)) frame.SelectWorkspace(v); });
        app.ViewParts["frame.zlayout"] = (() => frame.Model.Store(), v => frame.Model.Apply(v));
        frame.Flush();
        string[] kept = { "frame.appearance", "frame.workspace", "frame.zlayout" };
        demo.TrackLayout(() => kept.Select(k => app.ViewParts[k].Get()).ToArray(), parts => { for (int i = 0; i < kept.Length; i++) app.ViewParts[kept[i]].Set(parts[i]); });
        return frame;
    }
}
