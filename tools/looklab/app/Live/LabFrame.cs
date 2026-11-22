using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Godot;
using LookLab.Core;
using Magic600.Chrome;

namespace LookLab.App.Live;

// Look Lab inside the ADR-D01 window frame: the lab's Global and Local views fill the locked pair and its
// parameter panel is the "Tune" panel of the right dock. The frame, its tokens and its workspaces are the
// chrome library; only this file knows about the lab.
internal sealed record LabKit(Action<string, string?> Run, PaletteModel Commands, AppearancePanel Appearance, IReadOnlyList<string> Themes, IReadOnlyList<string> Densities);

internal static class LabFrame
{
    private static readonly System.Runtime.CompilerServices.ConditionalWeakTable<ChromeFrame, LabKit> Kits = new();
    public static LabKit KitOf(ChromeFrame frame) => Kits.TryGetValue(frame, out LabKit? kit) ? kit : throw new InvalidOperationException("the frame has no lab kit");
    private static readonly System.Runtime.CompilerServices.ConditionalWeakTable<ChromeFrame, LabDemo> Demos = new();
    private static readonly System.Runtime.CompilerServices.ConditionalWeakTable<ChromeFrame, LabInspect> Inspects = new();
    public static LabInspect InspectOf(ChromeFrame frame) => Inspects.TryGetValue(frame, out LabInspect? inspect) ? inspect : throw new InvalidOperationException("the frame has no lab inspector");
    private static readonly System.Runtime.CompilerServices.ConditionalWeakTable<ChromeFrame, LabScripts> AllScripts = new();
    public static LabScripts ScriptsOf(ChromeFrame frame) => AllScripts.TryGetValue(frame, out LabScripts? scripts) ? scripts : throw new InvalidOperationException("the frame has no lab scripts");
    private static readonly System.Runtime.CompilerServices.ConditionalWeakTable<ChromeFrame, LabSolve> Solves = new();
    private static readonly System.Runtime.CompilerServices.ConditionalWeakTable<ChromeFrame, KeyMapPanel> KeyMaps = new();
    public static LabSolve SolveOf(ChromeFrame frame) => Solves.TryGetValue(frame, out LabSolve? solve) ? solve : throw new InvalidOperationException("the frame has no solving surface");
    public static LabDemo DemoOf(ChromeFrame frame) => Demos.TryGetValue(frame, out LabDemo? demo) ? demo : throw new InvalidOperationException("the frame has no lab session");
    public const string Tune = "tune", Greybox = "greybox", Look = "appearance", Tokens = "tokens";
    private static readonly (DockSide Side, string Id, string Title, string Glyph, string Hint)[] Panels =
    {
        (DockSide.Left, "history", "History", "panel.history", "No moves yet"),
        (DockSide.Left, "scripts", "Scripts", "panel.scripts", "No programs yet"),
        (DockSide.Right, "inspector", "Inspector", "panel.inspector", "Nothing selected"),
        (DockSide.Right, "slot.1", "Script panel 1", "panel.slot1", "Empty panel slot"),
        (DockSide.Right, "slot.2", "Script panel 2", "panel.slot2", "Empty panel slot"),
        (DockSide.Bottom, "problems", "Problems", "panel.problems", "No problems"),
        (DockSide.Bottom, "console", "Console", "panel.console", "Console is empty"),
        (DockSide.Bottom, "tasks", "Tasks", "panel.tasks", "No task running"),
    };
    private static readonly (string Id, string Title, string Glyph, string Hint)[] Panes =
    {
        ("editor", "Editor", "workspace.script", "No program open"),
        ("preview", "Preview", "preview.moves", "No preview staged"),
        ("canvas", "Canvas", "workspace.analyse", "Nothing drawn"),
        ("reference", "Reference", "workspace.reference", "Nothing pinned yet"),
        (LabScripts.RingView, "Script view", "mount.view", "No script view yet"),
    };
    // The showing this build opens on. A kept arrangement from an earlier showing is not restored.
    public const string Showing = "stage8";
    public const string GalleryPane = "gallery";

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

    public const string NotInSample = "not in this sample";

    public static ChromeFrame Attach(LabApp app, string root)
    {
        ChromeFonts.SetRoots(PackageFolders(root));
        IconSet iconSet = IconSet.Load(Path.Combine(root, "tools/looklab/data/design/icons.json")); ChromeIcons.Use(iconSet);
        string folder = Path.Combine(root, "tools/looklab/data/design");
        DesignSystem design = DesignSystem.Load(Path.Combine(folder, "system.json"));
        MotionTable motions = MotionTable.Parse(File.ReadAllText(Path.Combine(folder, "motion.json")), design);
        var palettes = StickerPalette.Parse(File.ReadAllText(Path.Combine(folder, "palettes.json")));
        CommandIcons commandIcons = CommandIcons.Parse(File.ReadAllText(Path.Combine(folder, "command-icons.json"))); ChromeBody.Commands = commandIcons;
        ThemeFamily[] themes = Directory.GetFiles(Path.Combine(folder, "themes"), "*.json").OrderBy(p => p, StringComparer.Ordinal)
            .Select(p => ThemeFamily.Load(p, design)).ToArray();
        var look = new Appearance(design, themes);
        if (DisplayServer.IsDarkModeSupported()) look.FollowSystem(DisplayServer.IsDarkMode());
        // The chrome follows the scale of the screen it opens on (96 dpi is 1), in quarter steps.
        double screen = Math.Clamp(Math.Round(DisplayServer.ScreenGetDpi() / 96.0 * 4) / 4, 1, 3);
        look.SetSystemScale(screen);
        var frame = new ChromeFrame(look.Resolve(), WorkspaceSet.Load(Path.Combine(folder, "workspaces.json")));
        app.EnterFrame();
        foreach (var panel in Panels) frame.AddDockPanel(panel.Side, panel.Id, panel.Title, panel.Glyph, ChromeWidgets.Empty(panel.Glyph, panel.Hint));
        frame.AddDockPanel(DockSide.Right, Tune, "Tune", "panel.tune", app.ParameterPanel);
        foreach (var pane in Panes) frame.AddPane(pane.Id, pane.Title, ChromeWidgets.Empty(pane.Glyph, pane.Hint));
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
        // The lab shows the solving commands in the owner's words; the table's ids and labels are not edited.
        registry.Relabel(File.ReadAllText(Path.Combine(folder, "command-labels.json")));
        Keymap keymap = Keymap.Load(Path.Combine(folder, "keymap.json"), registry);
        LabScripts scripts = null!;
        // The component gallery: a pane of its own workspace, drawn from the tokens in force.
        var gallery = new ComponentGallery(() => frame.Tokens, motions, palettes, iconSet, commandIcons, registry.Order.ToArray());
        frame.AddPane(GalleryPane, "Gallery", gallery);
        void NextPreset()
        {
            var all = themes.SelectMany(t => ThemeFamily.SceneLooks.Select(l => (t.Id, Look: l))).ToArray();
            int at = (Array.FindIndex(all, x => x.Id == look.Theme && x.Look == look.SceneLook) + 1) % all.Length;
            appearance.Edit(a => { a.SetTheme(all[at].Id); a.SetSceneLook(all[at].Look); }, true, true);
            frame.Say($"preset {at + 1}/{all.Length}: {look.Family.Name} · {look.SceneLook}");
        }
        gallery.Asked += what =>
        {
            if (what == "preset") NextPreset();
            else if (what == "contrast") appearance.Edit(a => a.HighContrast = !a.HighContrast, true, false);
            else if (what == "character") appearance.Edit(a => a.OtherCharacter = !a.OtherCharacter, true, false);
            else appearance.Edit(a => a.SetDensity(design.Densities[(design.Densities.ToList().IndexOf(a.Density) + 1) % design.Densities.Count]), true, false);
        };
        // A chosen sticker palette is written through the lab's own palette parameters.
        gallery.PaletteChosen += chosen =>
        {
            foreach (var value in chosen.Lab)
                app.State.Set(value.Key, ParameterSchema.ReadValue(app.State.Schema[value.Key], System.Text.Json.JsonDocument.Parse(value.Value.ToString("R", System.Globalization.CultureInfo.InvariantCulture)).RootElement));
            frame.Say("Sticker palette: " + chosen.Name);
        };
        gallery.MotionChanged += () => appearance.Edit(a => a.ReducedMotion = gallery.Motion.Reduced, true, false);
        void ShowGallery() { if (GodotObject.IsInstanceValid(gallery)) gallery.Rebuild(); }
        frame.Rebuilt += ShowGallery; frame.Recoloured += ShowGallery;
        var layers = new System.Collections.Generic.Dictionary<string, FocusLayers> { ["global"] = new FocusLayers(), ["local"] = new FocusLayers() };
        string? Unavailable(string command, string? argument) => command switch
        {
            "keymap.defaults" or "file.keymap.export" => null,
            "panel.mount" or "panel.unmount" => argument != null ? null : "it needs the id of a script panel",
            _ when command.StartsWith("script.", StringComparison.Ordinal) => scripts?.Unavailable(command),
            "command.find" or "transient.close" or "help.open" or "motion.reduced.set" => null,
            _ when command.StartsWith("radial.", StringComparison.Ordinal) => null,
            "workspace.set" or "theme.select" or "density.preset.set" => argument != null ? null : "it needs an argument; choose one of its rows",
            _ when LabDemo.Handles(command) || LabSolve.Handles(command) => null,
            _ => NotInSample,
        };
        var commands = new PaletteModel(registry, keymap, Unavailable);
        var palette = new CommandPalette(commands, () => frame.Tokens);
        frame.Overlays.AddChild(palette);
        var sheet = new ShortcutSheet(registry, keymap, () => frame.Tokens); frame.Overlays.AddChild(sheet);
        var radial = new RadialOverlay(registry, Unavailable, () => frame.Tokens); frame.Overlays.AddChild(radial);
        frame.OverlayCommands.Add("transient.close"); frame.OverlayCommands.Add("help.open"); frame.OverlayCommands.Add("command.find");
        Button find = ChromeWidgets.Tool("", $"Find a command ({keymap.KeysFor("command.find", null)})"); find.Name = "FindButton";
        Button help = ChromeWidgets.Tool("", $"Key map: hold {keymap.KeysFor("help.open", null)}, or press it shortly to pin"); help.Name = "ShortcutsButton";
        void Glyphs() { ChromeIcons.Apply(find, "search", frame.Tokens.Px("icon.small")); ChromeIcons.Apply(help, "help.keys", frame.Tokens.Px("icon.small")); }
        Glyphs(); frame.Rebuilt += Glyphs;
        // The key map: shown while the help key is held, pinned by a short press or by this button.
        var keys = new KeyMapPanel(frame, registry); frame.Overlays.AddChild(keys); keys.List += sheet.Open; KeyMaps.Add(frame, keys);
        foreach ((string key, string label) in new[] { ("O", "Orbit jump: hold, then two digits"), ("F4", "Command bar"), ("F7", "Compare"), ("F8", "Compare"), ("F9", "Tour: next stop"), ("F10", "Inactive cells"),
            ("F11", "Next demo state"), ("6", "Pick layer"), ("7", "Pick layer"), ("8", "Pick layer"), ("9", "Pick layer"), ("0", "Pick layer") }) keys.Host[key] = label;
        find.Pressed += palette.Open; help.Pressed += () => keys.Shown.Pin(!keys.Shown.Pinned);
        frame.BarTools.AddChild(find); frame.BarTools.AddChild(help);
        string Next(System.Collections.Generic.IReadOnlyList<string> ids, string current) => ids[(ids.ToList().IndexOf(current) + 1) % ids.Count];
        LabDemo demo = null!; LabInspect inspect = null!; LabSolve solve = null!;
        void Execute(string command, string? argument)
        {
            if (Unavailable(command, argument) is { } why) { frame.Say($"{registry.Commands[command].Label}: {why}", true); return; }
            scripts.Echo(command, argument);
            switch (command)
            {
                case "keymap.defaults": scripts.Defaults(); break;
                case "file.keymap.export": scripts.Export(); break;
                case "panel.mount": scripts.Mount(argument!); break;
                case "panel.unmount": if (!scripts.Slots.Unmount(argument!)) frame.Say("No script panel is named " + argument, true); break;
                case var entry when entry.StartsWith("script.", StringComparison.Ordinal): scripts.Start(entry); break;
                case "command.find": palette.Open(); break;
                case var menu when keymap.Radials.ContainsKey(menu): radial.Open(scripts.WithContext(keymap.Radials[menu]), frame.GetLocalMousePosition()); break;
                case "help.open": if (sheet.IsOpen) sheet.Close(); else sheet.Open(); break;
                case "transient.close": keys.Shown.Pin(false);
                    if (frame.OpenOverlay is { } open) open.Close();
                    else if (inspect.Feedback.CancelLatest()) { }
                    else if (frame.Model.FocusMode) frame.Model.SetFocusMode(false);
                    else if (frame.Model.PairMaximised) frame.Model.SetPairMaximised(false);
                    break;
                case "workspace.set" when argument == "layout:focus": frame.Model.SetFocusMode(!frame.Model.FocusMode); break;
                case "workspace.set" when argument == "layout:maximise": frame.Model.SetPairMaximised(!frame.Model.PairMaximised); break;
                case "workspace.set" when argument!.StartsWith("layout:", StringComparison.Ordinal):
                    if (!frame.SelectWorkspace(argument.Substring(7))) frame.Say("No workspace is named " + argument.Substring(7), true);
                    break;
                case "motion.reduced.set":
                    appearance.Edit(a => a.ReducedMotion = argument == null ? !a.ReducedMotion : argument == "on", true, false);
                    frame.Say(look.ReducedMotion ? "Reduced motion: every motion shows its still picture at once" : "Motion on");
                    break;
                case "theme.select" when argument == "preset.next": NextPreset(); break;
                case "theme.select": appearance.Edit(a => a.SetTheme(Next(themes.Select(t => t.Id).ToArray(), a.Theme)), true, true); break;
                case "density.preset.set": appearance.Edit(a => a.SetDensity(Next(design.Densities, a.Density)), true, false); break;
                case var sample when LabDemo.Handles(sample): demo.Execute(command, argument); break;
                case var solving when LabSolve.Handles(solving): solve.Execute(command, argument, registry.Commands[command].Label); break;
                default: frame.Say(registry.Commands[command].Label + ": nothing to do for " + argument, true); break;
            }
        }
        palette.Run += row => Execute(row.Command, row.Argument);
        radial.Chosen += entry => Execute(entry.Command, entry.Argument);
        frame.Keys = keymap; frame.KeyCommand += binding => Execute(binding.Command, binding.Argument);
        // The sample session: commit bar under the views, the preview view and the History dock.
        demo = new LabDemo(frame, keymap, Execute); Demos.Add(frame, demo);
        frame.SetContent("history", demo.History); frame.SetContent(LabDemo.PreviewPane, demo.PreviewView); frame.SetStrip(demo.Bar);
        inspect = new LabInspect(app, frame, demo); Inspects.Add(frame, inspect);
        // The solving surface on the pair's body: the relationship strip on the seam, anchors in both views.
        solve = new LabSolve(frame, root, id => motions.Seconds(id, design, look.ReducedMotion), Execute, demo); Solves.Add(frame, solve);
        frame.SetStrip(solve.Bars(demo.Bar));
        app.ViewParts["frame.bindings"] = (() => inspect.Inspector.Store(), v => inspect.Inspector.Restore(v));
        // Scripts: editor, Scripts dock, console line, panel slots, canvas and the solver's own key bindings.
        scripts = new LabScripts(frame, demo, inspect, registry, keymap, root, Execute, k =>
        {
            keymap = k; frame.Keys = k; commands.Keymap = k; sheet.Keymap = k;
            if (scripts != null) { string set = scripts.Preset + (scripts.Overrides.Bindings.Count > 0 ? " with your bindings" : ""); if (keys.KeySet != set) { keys.KeySet = set; frame.Say("Key set: " + set); } }
 demo.Bar.CommitKeys = k.KeysFor("operation.preview.commit", null);
        });
        AllScripts.Add(frame, scripts);
        Kits.Add(frame, new LabKit(Execute, commands, appearance, themes.Select(t => t.Id).ToArray(), design.Densities.ToArray()));
        palette.TypedLine += scripts.RunLine;
        app.ViewParts["frame.keymap"] = (() => scripts.Store(), v => scripts.Restore(v));
        app.ViewParts["frame.scripts"] = (() => scripts.StoreState(), v => scripts.RestoreState(v));
        app.ViewParts["frame.slots"] = (() => scripts.Slots.Store(), v => scripts.Slots.Restore(v));
        app.ViewParts["frame.labels"] = (() => frame.Labels ? "on" : "off", v => frame.Labels = v == "on");
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
        // One chip says that what the docks hold is sample data.
        PanelContainer sample = SessionWidgets.Chip("sample", look.Resolve().C("text.dim"), "History, problems, tasks and scripts hold sample data"); sample.Name = "SampleChip";
        frame.BarTools.AddChild(sample);
        // The top bar is an instrument strip: readout cells for the revision, the history position and the run queue.
        var revision = new Readout("RevisionReadout", "readout.revision", "Revision of the committed state", () => frame.Tokens);
        var position = new Readout("PositionReadout", "readout.position", "History position: the entry in view, of the entries kept", () => frame.Tokens);
        var waiting = new Readout("QueueReadout", "readout.queue", "Script runs waiting or running", () => frame.Tokens);
        foreach (Readout cell in new[] { waiting, position, revision }) { frame.BarTools.AddChild(cell); frame.BarTools.MoveChild(cell, 0); }
        void Readouts()
        {
            if (!GodotObject.IsInstanceValid(revision)) return;
            var plain = System.Globalization.CultureInfo.InvariantCulture;
            revision.Set("r " + (1200 + demo.Moves.Cursor).ToString("0000", plain)); position.Set($"{demo.Moves.Cursor}/{demo.Moves.Entries.Count}"); waiting.Set(scripts.Queue.Active.Count().ToString(plain));
        }
        var clock = new Godot.Timer { WaitTime = .25, Autostart = true }; clock.Timeout += Readouts; frame.AddChild(clock);
        frame.Rebuilt += () => { foreach (Readout cell in new[] { revision, position, waiting }) if (GodotObject.IsInstanceValid(cell)) cell.Dress(); };
        // Pick-layer keys beside each view: 6 to 0 pick a layer in the view under the pointer, F10 cycles how the others show.
        string LayerKey(string id) => id == "cycle" ? "F10" : ((FocusLayers.Layers.ToList().FindIndex(l => l.Id == id) + 6) % 10).ToString(System.Globalization.CultureInfo.InvariantCulture);
        void DressViews()
        {
            foreach ((ViewFrame view, string id) in new[] { (frame.Global, "global"), (frame.Local, "local") })
                if (view.Tools.GetNodeOrNull("LayerKeys") == null) { var keys = new LayerKeys(layers[id], () => frame.Tokens, LayerKey); view.Tools.AddChild(keys); view.Tools.MoveChild(keys, 0); }
        }
        DressViews(); frame.Rebuilt += DressViews;
        FocusLayers Aimed() => layers[frame.Local.Focused ? "local" : "global"];
        foreach (var layer in FocusLayers.Layers) { string id = layer.Id; frame.HostKeys[LayerKey(id)] = () => Aimed().SetPick(id); }
        frame.HostKeys["F10"] = () => Aimed().Cycle();
        app.ViewParts["frame.layers"] = (() => layers["global"].Store() + "|" + layers["local"].Store(), v => { string[] kept = v.Split('|'); if (kept.Length == 2) { layers["global"].Restore(kept[0]); layers["local"].Restore(kept[1]); } });
        frame.AlertPressed += () => frame.Model.ShowDock(DockSide.Bottom, "problems");
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
                foreach (var mark in scripts.ViewMarks()) view.AddMark(mark.Glyph, mark.Text, mark.Colour, mark.Tip);
                FocusLayers own = layers[view == frame.Global ? "global" : "local"]; var picked = FocusLayers.Layers.First(l => l.Id == own.Pick);
                view.AddMark(picked.Icon, picked.Name.ToLowerInvariant(), "text.dim", $"A click picks {picked.Name.ToLowerInvariant()} in this view.");
                if (own.Inactive != InactiveMode.Normal)
                    view.AddMark("mark.protected", "shows through", "state.protected", $"The other layers are {own.Inactive.ToString().ToLowerInvariant()} and cannot be picked. A protection conflict still shows through every layer (sample).");
            }
            bool cap = app.Linked.Shape is { Ghost: > 0 }, rings = p.Choice("projection") == "hopfRings";
            frame.Local.AddMark("mark.autoframe", cap ? "A · cap" : "B · spread", "text.dim", cap ? "Local A: centred on the moving cells, their neighbours faint. F7 shows B." : "Local B: the moving cells spread apart. F7 shows A.");
            frame.Global.AddMark("mark.autoframe", rings ? "A · rings" : "B · ball", "text.dim", rings ? "Global A: the cells laid out as rings. F8 shows B." : "Global B: the whole puzzle as one ball. F8 shows A.");
            frame.Global.SetStatus(null, new[] { "drag: rotate 4D" }, $"gap {F("gap")}  shrink {F("stickerShrink")}");
            frame.Local.SetStatus($"{app.Linked.FocusCells} moving cells", new[] { "drag: turn" }, null);
        }
        // Compare keys of the lab: the other Local, the other Global.
        frame.HostKeys["F7"] = () => { app.Linked.LocalChoice = (app.Linked.LocalChoice + 1) % Math.Max(1, app.Linked.LocalChoices); Describe(); };
        frame.HostKeys["F8"] = () => app.State.Set("projection", new EnumValue(app.State.Preset.Params.Choice("projection") == "hopfRings" ? "perspective" : "hopfRings"));
        frame.HostKeys["F4"] = () => Execute("script.bar", null);
        // The tour key steps through the screens this showing changed.
        int tour = 0;
        void Home() { frame.SelectWorkspace(frame.Workspaces.Order[0]); }
        void Preset(string theme, string scene, bool contrast = false) => appearance.Edit(x => { x.SetTheme(theme); x.SetSceneLook(scene); x.HighContrast = contrast; }, true, true);
        void Page(string page) { frame.SelectWorkspace(GalleryPane); gallery.ShowPage(page); }
        string dark = themes[0].Id, light = themes[^1].Id;
        (string Name, Action Go)[] stops =
        {
            ("Graphite, the machined instrument: lines build depth", () => { Preset(dark, "solving"); Home(); }),
            ("Paper, the drafting sheet: tone steps build depth", () => { Preset(light, "solving"); Home(); }),
            ("every widget in every state", () => { Preset(dark, "solving"); Page("widgets"); }),
            ("the same under Paper, inspecting", () => { Preset(light, "inspecting"); Page("widgets"); }),
            ("high contrast", () => { Preset(dark, "solving", true); Page("widgets"); }),
            ("the celebrating look", () => { Preset(dark, "celebrating"); Home(); }),
            ("the motion table: each entry plays", () => { Preset(dark, "solving"); Page("motion"); gallery.Play(motions["twist.preview"]); }),
            ("sticker palettes with their distances", () => Page("palette")),
            ("icons, both tiers", () => Page("icons")),
            ("type and notation", () => Page("type")),
            ("a list of 10,000 rows", () => { Page("list"); gallery.List?.ScrollTo(5000); }),
            ("the five states of a pane", () => Page("panes")),
            ("pick layers; the other layers dimmed", () => { Home(); layers["global"].SetPick("stickers"); if (layers["global"].Inactive == InactiveMode.Normal) layers["global"].Cycle(); }),
            ("a radial menu", () => { Home(); Execute("radial.view", null); }),
            ("the relationship strip on the seam, mid-solve: Wanted is in A", () => { Home(); solve.Demo.Show("mid"); if (!solve.Strip.Expanded) solve.Strip.ToggleResidual(); }),
            ("the orbit rail: 35 cells in one admissible order, the working cell opened in place", () => { Home(); solve.Demo.Show("mid"); solve.Rail.Expand(solve.Demo.State.Orbit); }),
            ("the draft line: Prepare, Core and Cleanup, the Goal unmet and the commit still open; Strict fails where Net passes", () => { Home(); solve.Demo.Show("mid"); solve.Rail.Expand(null); solve.Draft.Clear(); solve.Draft.SetRoute("draft");
                solve.Draft.Edit("prepare: twist(cell 5, axis 0, +1)"); solve.Draft.Edit("core: twist(cell 40, axis 2, -1)"); solve.Draft.SetGoal(OperationDraft.SampleGoal); solve.Draft.SetMode(CheckMode.Strict); }),
            ("inline parameters on the commit bar: the numbers of the staged statement, edited in place", () => { Home(); solve.Draft.Clear(); demo.Stage(2); demo.SetParameter(1, 3); }),
            ("the key map, pinned: what each key does with the pointer over Local, and the active key set", () => { Home(); solve.Demo.Show("mid"); solve.Rail.Expand(null); solve.Draft.Clear(); keys.ContextsOverride = new[] { "local-view", "global" }; keys.Shown.Pin(true); }),
            ("the strip and the rail at the last buffer of the endgame", () => { Home(); keys.ContextsOverride = null; keys.Shown.Pin(false); solve.Demo.Show("endgame"); solve.Rail.Expand(null); }),
            ("the strip after a fresh scramble: nothing is set", () => { Home(); solve.Demo.Show("fresh"); }),
            ("the script workspace in the new body", () => { frame.SelectWorkspace("script"); scripts.Editor.SetWrap(true); scripts.Open("sweep", scripts.Find("script.sweep.reach")!.Line); }),
        };
        frame.HostKeysOverOverlays.Add("F9");
        frame.HostKeys["F9"] = () =>
        {
            frame.OpenOverlay?.Close();
            (string name, Action go) = stops[tour]; go(); Describe();
            frame.Say($"tour {tour + 1}/{stops.Length}: {name}");
            tour = (tour + 1) % stops.Length;
        };
        frame.Rebuilt += Describe; frame.Recoloured += Describe;
        foreach (FocusLayers changed in layers.Values) changed.Changed += () => { if (GodotObject.IsInstanceValid(frame)) Describe(); };
        demo.Preview.Changed += () => { if (GodotObject.IsInstanceValid(frame)) Describe(); };
        app.State.Changed += (_, _) => { if (GodotObject.IsInstanceValid(frame)) Describe(); };
        // A kept view from an earlier showing keeps its values but not its arrangement: this build opens on its
        // own first screen, with the scene of the theme family.
        string? seen = null;
        app.ViewParts["frame.showing"] = (() => Showing, v => seen = v);
        app.ViewParts["frame.workspace"] = (() => frame.Workspace, v => { if (seen == Showing && frame.Workspaces.Stored.ContainsKey(v)) frame.SelectWorkspace(v); });
        app.ViewParts["frame.zlayout"] = (() => frame.Model.Store(), v => { if (seen == Showing) frame.Model.Apply(v); });
        Callable.From(() =>
        {
            if (seen == Showing || !GodotObject.IsInstanceValid(frame)) { seen = Showing; return; }
            seen = Showing; ApplyScene(); Home(); Describe();
        }).CallDeferred();
        frame.Flush();
        string[] kept = { "frame.appearance", "frame.workspace", "frame.zlayout", "frame.labels", "frame.keymap" };
        demo.TrackLayout(() => kept.Select(k => app.ViewParts[k].Get()).ToArray(), parts => { for (int i = 0; i < kept.Length; i++) app.ViewParts[kept[i]].Set(parts[i]); });
        return frame;
    }
}
