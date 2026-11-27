using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using Godot;
using LookLab.App.Live;
using Magic600.Chrome;
using Magic600.Shell.D02;

namespace Magic600.Shell;

/// <summary>
/// The shell's frame: the chrome library and the design data that Look Lab uses (tools/looklab/app/Chrome and
/// tools/looklab/data/design), which build_shell.py stages into the build's project as Chrome/ and design/. There is
/// no second copy in the repository.
/// </summary>
/// <remarks>
/// On route B the core presents beneath the window, so the Global view is an opening in the frame's ground and every
/// other region is opaque chrome. The core draws one view; the Local view and every pane whose content the shell does
/// not have yet show the designed "not drawn yet" state. Real here: the History dock's entries (the twists this run
/// committed, and its undos and redos), and the readouts of the engine revision and the history position.
/// </remarks>
public sealed class ShellChrome
{
    // The docks follow the layouts of workspaces.json: Solve opens Library on the left and History on the right, so
    // each panel sits on the side its layouts name (a dock whose side lacks the named panel stays shut).
    private static readonly (DockSide Side, string Id, string Title, string Glyph)[] Panels =
    {
        (DockSide.Left, "library", "Library", "panel.library"),
        (DockSide.Left, "scripts", "Scripts", "panel.scripts"), (DockSide.Right, "inspector", "Inspector", "panel.inspector"),
        (DockSide.Right, "slot.1", "Script panel 1", "panel.slot1"), (DockSide.Right, "slot.2", "Script panel 2", "panel.slot2"),
        (DockSide.Right, "tune", "Tune", "panel.tune"),
        (DockSide.Bottom, "problems", "Problems", "panel.problems"), (DockSide.Bottom, "console", "Console", "panel.console"),
        (DockSide.Bottom, "tasks", "Tasks", "panel.tasks"),
    };
    private static readonly (string Id, string Title)[] Panes =
    {
        ("editor", "Editor"), ("preview", "Preview"), ("canvas", "Canvas"), ("scriptview", "Script view"), ("greybox", "Greybox"),
    };
    public const string NotHere = "not in this build yet";

    public ChromeFrame Frame { get; }
    /// <summary>Runs a row as the palette does (unavailable reason, echo, then the row): the D02 script player's command step.</summary>
    public static Action<string, string?>? Run { get; private set; }
    public MoveHistory Moves { get; } = new();
    /// <summary>
    /// Sample mode (B4a): ADR-D01's whole frame on sample data, behind launch.py --sample, or null when it is off. The
    /// views, the History dock, the readouts and the session's undo and redo stay the real session's.
    /// </summary>
    internal SampleMount? Sample { get; }
    /// <summary>ADR-D02's Jumble host in Solve (B6), with D02 on.</summary>
    internal JumbleHost? Jumble { get; }
    /// <summary>ADR-D02.47's tuning panel (B30), with D02 on: a tab of the right dock that the palette opens.</summary>
    internal TuningPanel? Tuning { get; }
    /// <summary>Whether sample mode is on: launch.py --sample sets M600_SAMPLE=1.</summary>
    public static bool SampleOn => System.Environment.GetEnvironmentVariable("M600_SAMPLE") == "1";
    public const string SampleWords = "Sample mode: the strip, the rail, the draft line, the Library, the inspector, Problems, Console, Tasks, the scripts, History (sample) and Research hold sample data. The views, History, the revision and the history position are this session's.";
    private readonly bool _live;
    // ADR-D01.41: the rotation link's state from the host: null with no Local view, true linked, false unlinked.
    private readonly Func<bool?> _rotationLink;
    private readonly Readout _revision, _position;
    private readonly Readout? _queue;
    private readonly HashSet<Key> _taken = new();
    private readonly Func<string, string?, string?> _unavailable;
    private readonly Action<Vector2>? _openContextMenu;
    private ulong _shown;

    /// <summary>The folders installed font and icon packages are looked for in: tools/node of every folder above the
    /// project and, from a linked worktree, of its main checkout.</summary>
    private static IEnumerable<string> PackageFolders(string start)
    {
        for (var at = new DirectoryInfo(Path.GetFullPath(start)); at != null; at = at.Parent)
        {
            yield return Path.Combine(at.FullName, "tools", "node");
            var link = Path.Combine(at.FullName, ".git");
            if (!File.Exists(link))
            {
                continue;
            }
            var text = File.ReadAllText(link).Trim().Replace((char)92, '/');
            var cut = text.IndexOf("/.git/worktrees/", StringComparison.Ordinal);
            if (text.StartsWith("gitdir: ", StringComparison.Ordinal) && cut > 8)
            {
                yield return Path.Combine(text.Substring(8, cut - 8), "tools", "node");
            }
        }
    }

    /// <summary>The design folder of this build, or null when the build was staged without it.</summary>
    public static string? DesignFolder()
    {
        var folder = ProjectSettings.GlobalizePath("res://design");
        return File.Exists(Path.Combine(folder, "system.json")) ? folder : null;
    }

    public ShellChrome(Control parent, string folder, bool live, Func<bool?>? rotationLink = null)
    {
        _live = live;
        _rotationLink = rotationLink ?? (() => Frame?.LocalDrawn == true ? true : null);
        ChromeFonts.SetRoots(PackageFolders(folder));
        var iconSet = IconSet.Load(Path.Combine(folder, "icons.json"));
        ChromeIcons.Use(iconSet);
        var design = DesignSystem.Load(Path.Combine(folder, "system.json"));
        var themes = Directory.GetFiles(Path.Combine(folder, "themes"), "*.json").OrderBy(p => p, StringComparer.Ordinal)
            .Select(p => ThemeFamily.Load(p, design)).ToArray();
        var motions = MotionTable.Parse(File.ReadAllText(Path.Combine(folder, "motion.json")), design);
        var palettes = StickerPalette.Parse(File.ReadAllText(Path.Combine(folder, "palettes.json")));
        string iconRows = File.ReadAllText(Path.Combine(folder, "command-icons.json"));
        var commandIcons = CommandIcons.Parse(ShellJumble.On ? StatusPatterns.AddCommandLabels(iconRows) : iconRows);
        ChromeBody.Commands = commandIcons;
        var look = new Appearance(design, themes);
        // ADR-D02 B18 rest: the sweep and the half-turn read their rows' seconds (none under reduced motion) and curves.
        D02.ShellTurn.MotionRow = id => (motions.Seconds(id, design, look.ReducedMotion), design.Curves[motions[id].Curve]);
        if (ShellJumble.On && parent is Shell shell) ShellMotion.ApplyRawClip = shell.FollowRawClip;
        if (DisplayServer.IsDarkModeSupported())
        {
            look.FollowSystem(DisplayServer.IsDarkMode());
        }
        look.SetSystemScale(Math.Clamp(Math.Round(DisplayServer.ScreenGetDpi() / 96.0 * 4) / 4, 1, 3));
        var frame = Frame = new ChromeFrame(look.Resolve(), WorkspaceSet.Load(Path.Combine(folder, "workspaces.json"))) { OpenGround = live, LocalDrawn = live && ShellJumble.On };
        frame.Overlays.ChildEnteredTree += child => { if (child is ChromeOverlay overlay) overlay.Motions = motions; };
        frame.Name = "ShellChrome";
        parent.AddChild(frame);
        // A check's aid: M600_FRAME_PICTURE names a file that gets the window's own picture (the frame without the
        // core's picture, which lies beneath the window) a few seconds after the start.
        if (System.Environment.GetEnvironmentVariable("M600_FRAME_PICTURE") is { Length: > 0 } picture)
        {
            parent.GetTree().CreateTimer(5).Timeout += () => parent.GetViewport().GetTexture().GetImage().SavePng(picture);
        }
        frame.SetAnchorsAndOffsetsPreset(Control.LayoutPreset.FullRect);

        // ADR-D01.14: the layout history, shared by History's dock and the layout rows.
        var layouts = new LayoutHistory();
        var history = new HistoryPanel(Moves, layouts, () => frame.Tokens);
        frame.AddDockPanel(DockSide.Right, "history", "History", "panel.history", history);
        foreach (var panel in Panels)
        {
            frame.AddDockPanel(panel.Side, panel.Id, panel.Title, panel.Glyph, ChromeBody.PaneState("notdrawn", panel.Title + ": " + NotHere));
        }
        var appearance = new AppearancePanel(look);
        frame.AddDockPanel(DockSide.Right, "appearance", "Appearance", "panel.appearance", appearance);
        // B30: the tuning panel beside History and Appearance; its knobs edit the appearance through the Appearance panel.
        if (ShellJumble.On)
        {
            // B31: the shipped presets; the held simulation's matrix goes to the core's view style only with the G8
            // switch on (the look-cost row), and over the chrome always.
            var presetFile = Path.Combine(folder, "tuning-presets.json");
            var presets = File.Exists(presetFile) ? TuningPreset.Parse(File.ReadAllText(presetFile), look) : null;
            // CR-16's tuning.diff.export writes into the run folder (M600_EVENTS_DIR), beside the run's other files; a build
            // started without one has no place to write, and the row says so.
            var runFolder = System.Environment.GetEnvironmentVariable("M600_EVENTS_DIR");
            Tuning = ShellJumble.TuningHost = new TuningPanel(appearance, frame, presets)
            {
                CoreSimulation = ShellStyle.CoreSimulation,
                ExportFolder = !string.IsNullOrEmpty(runFolder) && Directory.Exists(runFolder) ? runFolder : null,
            };
            var tuning = Tuning;
            tuning.Simulated += () => ShellStyle.SetSimulation(tuning.Model.Simulating == null ? null : tuning.Model.Matrix);
            frame.AddDockPanel(DockSide.Right, Magic600.Chrome.Tuning.PanelId, Magic600.Chrome.Tuning.Title, Magic600.Chrome.Tuning.Glyph, Tuning);
        }
        var specimen = new TokenSpecimen();
        frame.AddDockPanel(DockSide.Left, "tokens", "Tokens", "panel.tokens", specimen);
        foreach (var pane in Panes)
        {
            // The preview view is a free view bound to the preview (ADR-D01.10, ADR-D01.42), drawn or not.
            frame.AddPane(pane.Id, pane.Title, ChromeBody.PaneState("notdrawn", pane.Title + ": " + NotHere),
                pane.Id == "preview" ? ViewBinding.Preview : null);
        }
        // The core draws one view, beneath the window: the Global view is an opening. The Local view is not drawn yet.
        Control global = live
            ? new Control { Name = "CoreOpening", MouseFilter = Control.MouseFilterEnum.Pass }
            : ChromeBody.PaneState("notdrawn", "Global view: the core is not loaded");
        frame.SetMainViews(global, ChromeBody.PaneState("notdrawn", "Local view: not drawn yet"));

        var registry = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(folder, "command-table.json")));
        // ADR-D02's build (B4): with D02 on, its draft rows follow the table's and the Jumble rows show their display names.
        if (ShellJumble.On)
        {
            ShellJumble.AddRows(registry, folder);
        }
        // Sample mode registers the draft rows Look Lab registers (Library branches, replay, preview sides, History and the
        // layout roll), so the sample's controls reach a row; the palette marks each one "draft row" (owner's bug list,
        // 26 November 2026: the Library branches would not open).
        string[] sampleDrafts = [];
        if (SampleOn)
        {
            var draftRows = File.ReadAllText(Path.Combine(folder, "command-rows-draft.json"));
            var drafts = new CommandRegistry();
            drafts.AddRows(draftRows);
            registry.AddRows(draftRows);
            sampleDrafts = drafts.Order.ToArray();
            // ADR-D01.68: the setup slot under the Target, on the sample's solving surface; its rows are ADR-D02 draft rows.
            sampleDrafts = [.. sampleDrafts, .. SetupRows.AddDrafts(registry, File.ReadAllText(Path.Combine(folder, "command-rows-d02.json")))];
        }
        // B28: the research pane, a centre peer of the pair in Analyse; it registers its draft rows in either mode (C5-5),
        // and in sample mode reads the synthetic sample.
        ShellResearch.Load(frame, registry, folder, SampleOn, () => Moves.Entries.Count);
        var keymap = Keymap.Load(Path.Combine(folder, "keymap.json"), registry);
        var contextMenus = ShellJumble.On ? ContextMenuSet.Parse(File.ReadAllText(Path.Combine(folder, "context-menus.json")), registry) : null;
        var helpCatalog = HelpCatalog.Load(Path.Combine(folder, "glossary-terms.json"), Path.Combine(folder, "help-figures.json"), registry);
        var learnPane = new LearnPane(new LearnModel(helpCatalog), () => frame.Tokens);
        frame.AddPane("reference", "Learn", learnPane);
        SampleMount? sample = null;
        // ADR-D01.74: the key map in either mode; outside sample mode the keymaps folder's key sets.
        KeyMapPanel? keyMap = null;
        KeySets? keySets = null;
        Action<Keymap>? keysTaken = null;
        LayoutTracker? layoutTracker = null;
        ShellCards? cards = null;
        Action? pickedTarget = null;
        var gallery = new ComponentGallery(() => frame.Tokens, motions, palettes, iconSet, commandIcons, registry.Order.ToArray());
        frame.AddPane("gallery", "Gallery", gallery);
        string? Unavailable(string command, string? argument) => command switch
        {
            // B28: a research row naming a research object (or a tab row) is the research pane's. B29: the pane says
            // why one is unavailable (a start before its budget, the two CR-17 rows).
            _ when ShellResearch.Model is { } research && ResearchModel.Handles(command, argument) => research.Unavailable(command, argument),
            _ when ShellDocuments.Handles(command) => ShellDocuments.Unavailable(command, argument),
            _ when ShellSession.Handles(command) => ShellSession.Unavailable(command),
            _ when ShellExplore.Handles(command) => ShellExplore.Unavailable(argument),
            _ when ShellScene.Handles(command) => null,
            _ when ShellStatus.Handles(command) => null,
            "command.find" or "transient.close" or "help.open" or "motion.reduced.set" or "workspace.reset" => null,
            "menu.context.open" when contextMenus != null => null,
            "target.current.set" when pickedTarget != null && argument == null => null,
            _ when layoutTracker != null && LayoutTracker.Handles(command) => null,
            _ when command.StartsWith("radial.", StringComparison.Ordinal) => null,
            "workspace.set" or "theme.select" or "density.preset.set" => argument != null ? null : "it needs an argument; choose one of its rows",
            // B9: the naming and threshold draft rows work; the other draft rows wait for their steps.
            _ when Tuning != null && Tuning.Handles(command) => Tuning.Unavailable(command, argument),
            _ when Jumble != null && Jumble.Handles(command) => Jumble.Unavailable(command, argument),
            _ when cards != null && ShellCards.Handles(command) => cards.Unavailable(command, argument),
            // ADR-D01.68: the setup rows run on the sample's solving surface, D02 on or off.
            _ when sample != null && SetupSearch.Ids.Contains(command) => sample.Unavailable(command, argument),
            // ADR-D01.74: the key set row runs where key sets are loaded: the sample's presets, or the keymaps folder's.
            "keymap.preset.set" => KeySets.Unavailable(sample != null ? sample.Scripts.Presets : keySets?.Names, argument),
            _ when ShellJumble.On && ShellJumble.Drafts.Contains(command) => ShellJumble.DraftReason,
            _ when sample != null && sample.Handles(command) => sample.Unavailable(command, argument),
            // ADR-D01's Local row: the Local hide, where the frame draws Local (the D02 product).
            ViewPanes.LocalRow => frame.LocalDrawn ? null : "this build does not draw Local",
            _ => NotHere,
        };
        // ADR-D01.39: the explore state's gate runs first for every caller; it adds a refusal, never removes one.
        _unavailable = Unavailable;
        var gated = ShellExplore.State.Compose(Unavailable, registry);
        var commands = new PaletteModel(registry, keymap, gated);
        var palette = new CommandPalette(commands, () => frame.Tokens);
        palette.DraftRows.UnionWith(sampleDrafts);
        palette.DraftRows.UnionWith(ShellResearch.DraftIds);
        frame.Overlays.AddChild(palette);
        var sheet = new ShortcutSheet(registry, keymap, () => frame.Tokens);
        frame.Overlays.AddChild(sheet);
        var helpCard = new HelpCard(() => frame.Tokens);
        frame.Overlays.AddChild(helpCard);
        var radial = new RadialOverlay(registry, gated, () => frame.Tokens);
        frame.Overlays.AddChild(radial);
        var contextMenu = contextMenus == null ? null : new ContextMenuOverlay(registry, gated, () => frame.Tokens);
        if (contextMenu != null) frame.Overlays.AddChild(contextMenu);
        frame.OverlayCommands.Add("transient.close");
        frame.OverlayCommands.Add("help.open");
        frame.OverlayCommands.Add("command.find");
        // Every keymap change reaches each reader of the keys; keysTaken adds the readers made later.
        void TakeKeys(Keymap k) { keymap = k; frame.Keys = k; commands.Keymap = k; sheet.Keymap = k; keysTaken?.Invoke(k); }
        if (SampleOn)
        {
            sample = Sample = new SampleMount(frame, registry, keymap, ProjectSettings.GlobalizePath("res://sample-root"),
                id => motions.Seconds(id, design, look.ReducedMotion), Execute,
                TakeKeys, palette, sheet, layouts);
            sample.Look = appearance;
            sample.Solve.Strip.Motions = motions;
            // ADR-D01.70: the shell's History draws its entries' effect signatures in the sample rail's order.
            history.RailOrder = sample.Solve.LibraryView.RailOrder;
            sample.Solve.Rail.OrderChanged += history.Refresh;
            history.Refresh();
        }
        // ADR-D01.74: outside sample mode the key map's key is the key list's (Shift+/), so F1 keeps ADR-D02.12's
        // help route; its List button opens the list, and the key sets are the keymaps folder's presets.
        if (sample == null)
        {
            keyMap = new KeyMapPanel(frame, registry) { ShowArgument = HelpCatalog.Keys };
            keyMap.List += sheet.Open;
            frame.Overlays.AddChild(keyMap);
            keySets = KeySets.Load(Path.Combine(folder, "keymaps"), registry, keymap);
        }
        else
        {
            keyMap = sample.Keys;
        }
        ShellDocuments.Load(frame, registry, sample);
        ShellSession.Load(frame, registry, sample, history, Moves, Execute);
        ShellExplore.Load(frame);
        // Failures show in Problems with the error lamp, in every mode, and go to the run's errors.log.
        ChromeErrors.Mount(frame, sample?.Inspect.Feedback, "shell",
            System.Environment.GetEnvironmentVariable("M600_EVENTS_DIR") is { Length: > 0 } events ? Path.Combine(events, "errors.log") : null);

        void ApplyLook(bool rebuild, bool scene)
        {
            if (look.ReducedMotion && !frame.Tokens.ReducedMotion)
            {
                (parent as Shell)?.FlushMotion(reduced: true);
                foreach (var overlay in frame.Overlays.GetChildren().OfType<ChromeOverlay>()) overlay.FinishOpening();
            }
            gallery.Motion.Rewards = look.Rewards;
            frame.ApplyTokens(look.Resolve(), rebuild);
            if (history.Rewards != look.Rewards)
            {
                history.Rewards = look.Rewards;
                history.Refresh();
            }
        }
        appearance.Changed += ApplyLook;
        ShellScene.Load(frame, palettes, () => (look.Family, look.SceneLook),
            name => appearance.Edit(a => a.SetSceneLook(name), true, true));
        appearance.Changed += (rebuild, scene) => ShellScene.Refresh();
        if (ShellJumble.On) ShellStatus.Load(frame, folder);
        void NextPreset()
        {
            var all = themes.SelectMany(t => ThemeFamily.SceneLooks.Select(l => (t.Id, Look: l))).ToArray();
            var at = (Array.FindIndex(all, x => x.Id == look.Theme && x.Look == look.SceneLook) + 1) % all.Length;
            appearance.Edit(a => { a.SetTheme(all[at].Id); a.SetSceneLook(all[at].Look); }, true, true);
            frame.Say($"preset {at + 1}/{all.Length}: {look.Family.Name} · {look.SceneLook}");
        }
        void NextDensity() => appearance.Edit(a => a.SetDensity(design.Densities[(design.Densities.ToList().IndexOf(a.Density) + 1) % design.Densities.Count]), true, false);
        void Execute(string command, string? argument) => ExecuteAt(command, argument, null);
        // ADR-D01.74: a key set change never changes a key's meaning silently: the status line names each key it gave
        // another meaning, and the key map marks them until it has been shown with them.
        void UseKeySet(string? name)
        {
            if (sample != null)
            {
                IReadOnlyList<string> all = sample.Scripts.Presets;
                sample.Scripts.SetPreset(name ?? all[(all.ToList().IndexOf(sample.Scripts.Preset) + 1) % all.Count]);
            }
            else if (keySets != null)
            {
                Keymap before = keymap;
                TakeKeys(keySets.Choose(name));
                if (keyMap != null) keyMap.KeySet = keySets.Name;
                frame.Say(KeyMapModel.Said(keySets.Name, KeyMapModel.Changes(before, keymap, registry)));
            }
        }
        void ExecuteAt(string command, string? argument, Vector2? pointer)
        {
            if (registry.Commands[command].Kind == "commit" || command is "session.resume" or "explore.set" or "operation.phase.append_twist")
                (parent as Shell)?.FlushMotion();
            if (ShellJumble.On) ShellGrip.CancelGesture();
            if (gated(command, argument) is { } why)
            {
                frame.Say($"{registry.Commands[command].Label}: {why}", true);
                return;
            }
            sample?.Echo(command, argument);
            switch (command)
            {
                // B28: the research rows (a rename is workspace.set research.names:<ref>=<name>), before workspace.set.
                case var research when ShellResearch.Model != null && ResearchModel.Handles(research, argument):
                    ShellResearch.Execute(frame, command, argument);
                    // B29: a research row run from another layout (the palette, a script) brings its pane into view.
                    if (ShellResearch.Pane is { } shown && GodotObject.IsInstanceValid(shown) && !shown.IsVisibleInTree())
                    {
                        Execute("workspace.set", "layout:analyse");
                    }
                    break;
                case var document when ShellDocuments.Handles(document):
                    ShellDocuments.Execute(frame, command, argument);
                    break;
                case var session when ShellSession.Handles(session):
                    ShellSession.Execute(frame, command, argument);
                    break;
                case var explore when ShellExplore.Handles(explore): ShellExplore.Execute(frame, argument); break;
                case var scene when ShellScene.Handles(scene): ShellScene.Execute(frame, command, argument); break;
                case var status when ShellStatus.Handles(status): ShellStatus.Execute(command, argument); break;
                case "command.find": palette.Open(); break;
                case ViewPanes.LocalRow: frame.LocalHide.Toggle(); frame.Say(frame.LocalHide.Hidden ? "Local hidden" : "Local shown"); break;
                case "keymap.preset.set": UseKeySet(argument); break;
                case "menu.context.open" when contextMenu != null && contextMenus != null:
                    Vector2 at = argument switch
                    {
                        "global" => frame.Global.GetGlobalRect().GetCenter() - frame.GlobalPosition,
                        "local" => frame.Local.GetGlobalRect().GetCenter() - frame.GlobalPosition,
                        _ => pointer is { } point ? point - frame.GlobalPosition : frame.GetLocalMousePosition(),
                    };
                    contextMenu.Open(ShellPick.Selection == null ? contextMenus.View : contextMenus.Pick, keymap, at);
                    break;
                case "target.current.set" when pickedTarget != null && argument == null: pickedTarget(); break;
                case "help.open":
                    var topic = argument ?? HelpTopics.At(frame.GetViewport());
                    // ADR-D01.74: the key list's topic pins the key map, or lets it go; its List button opens the list.
                    if (topic == HelpCatalog.Keys) { if (keyMap != null) keyMap.Shown.Pin(!keyMap.Shown.Pinned); else if (sheet.IsOpen) sheet.Close(); else sheet.Open(); }
                    else if (topic == null) palette.Open(); // ADR-D02.12's F1 route when the pointer has no help topic.
                    else if (helpCatalog.Resolve(topic) is { } resolved) helpCard.Open(helpCatalog.Card(resolved), frame.GetLocalMousePosition());
                    else frame.Say("No help topic named " + topic, true);
                    break;
                case var menu when keymap.Radials.ContainsKey(menu):
                    radial.Open(sample != null ? sample.WithContext(keymap.Radials[menu]) : keymap.Radials[menu], frame.LocalPointer);
                    break;
                case "transient.close":
                    keyMap?.Shown.Pin(false);
                    if (frame.OpenOverlay is { } open) open.Close();
                    // ADR-D01.50: Esc reverts the tuning panel's live change and is spent on it; the other transients
                    // close only when no change is pending. An open overlay closes first: it holds the keys.
                    else if (Tuning != null && Tuning.Escape()) { }
                    else if (sample != null && sample.Inspect.Feedback.CancelLatest()) { }
                    else if (frame.Model.FocusMode) frame.Model.SetFocusMode(false);
                    else if (frame.Model.PairMaximised) frame.Model.SetPairMaximised(false);
                    break;
                case "workspace.set" when argument == "layout:focus": frame.Model.SetFocusMode(!frame.Model.FocusMode); break;
                case "workspace.set" when argument == "layout:maximise": frame.Model.SetPairMaximised(!frame.Model.PairMaximised); break;
                // The layout reset button and the row are one path (owner's bug list, 26 November 2026).
                case "workspace.reset": frame.SelectWorkspace(frame.Workspace); break;
                case var layout when layoutTracker != null && LayoutTracker.Handles(layout):
                    if (layoutTracker.Execute(command, argument) is { } still) frame.Say(still, true);
                    break;
                case "workspace.set":
                    if (!frame.SelectWorkspace(argument!.Substring(argument.IndexOf(':') + 1))) frame.Say("No workspace is named " + argument, true);
                    break;
                case "motion.reduced.set": appearance.Edit(a => a.ReducedMotion = argument == null ? !a.ReducedMotion : argument == "on", true, false); break;
                case "theme.select" when argument == "preset.next": NextPreset(); break;
                case "theme.select": appearance.Edit(a => a.SetTheme(themes[(Array.FindIndex(themes, t => t.Id == a.Theme) + 1) % themes.Length].Id), true, true); break;
                case "density.preset.set": NextDensity(); break;
                // The reset row also takes role=reset. The tuning panel appends =reset to the role it receives.
                case "token.override.reset" when Tuning != null && argument?.Trim() is { } reset && reset.EndsWith("=reset", StringComparison.Ordinal):
                    Tuning.Execute(command, reset[..^"=reset".Length]); break;
                case var tuned when Tuning != null && Tuning.Handles(tuned): Tuning.Execute(command, argument); break;
                case var jumble when Jumble != null && Jumble.Handles(jumble): Jumble.Execute(command, argument); break;
                case var card when cards != null && ShellCards.Handles(card): cards.Execute(command, argument); break;
                case var handled when sample != null && sample.Handles(handled): sample.Execute(command, argument); break;
            }
        }
        Run = Execute;
        _openContextMenu = contextMenu == null ? null : at => ExecuteAt("menu.context.open", null, at);
        if (ShellResearch.Pane is { } researchPane)
        {
            researchPane.Command += Execute;
        }
        palette.Run += row => Execute(row.Command, row.Argument);
        helpCard.Learn += link => { Execute(link.Command, link.Argument); learnPane.Select(link.TopicId); };
        radial.Chosen += entry => Execute(entry.Command, entry.Argument);
        radial.Let += menu => frame.Say(RadialHold.Let(menu.Label));
        if (contextMenu != null) contextMenu.Chosen += entry => Execute(entry.Command, entry.Argument);
        frame.Keys = keymap;
        // ADR-D01.20: a radial menu opened by its key follows the pointer while that key stays down.
        frame.KeyCommand += binding =>
        {
            Execute(binding.Command, binding.Argument);
            if (radial.IsOpen && radial.Menu?.Id == binding.Command) radial.Hold(binding.Key);
        };
        frame.RowCommand += Execute;
        // Without the sample mount the shell tracks the layout itself (with it, SampleMount does).
        if (sample == null)
        {
            var tracker = layoutTracker = FrameLayout.Tracker(frame, layouts);
            tracker.Restored += history.Refresh;
            history.Command += Execute;
            Callable.From(() => tracker.Track(FrameLayout.State(frame, () => look.Store(), () => ""),
                FrameLayout.Apply(frame, v => { if (v.Length > 0) appearance.Edit(a => a.Restore(v), true, true); }, _ => { }))).CallDeferred();
        }
        gallery.Asked += what =>
        {
            if (what == "preset") NextPreset();
            else if (what == "contrast") appearance.Edit(a => a.HighContrast = !a.HighContrast, true, false);
            else if (what == "character") appearance.Edit(a => a.OtherCharacter = !a.OtherCharacter, true, false);
            else NextDensity();
        };
        gallery.PaletteChosen += ShellScene.ChoosePalette;
        gallery.MotionChanged += () => appearance.Edit(a =>
        {
            a.ReducedMotion = gallery.Motion.Reduced;
            a.Rewards = gallery.Motion.Rewards;
        }, true, false);

        // The commit bar is the frame's own; the host stages no preview yet, so it stays empty.
        var bar = new CommitBar(new PreviewModel(), () => frame.Tokens) { CommitKeys = keymap.KeysFor("operation.preview.commit", null) };
        bar.Command += Execute;
        keysTaken += k => bar.CommitKeys = k.KeysFor("operation.preview.commit", null);
        if (sample == null)
        {
            frame.SetStrip(bar);
        }
        history.Command += (command, argument) =>
        {
            if (command == "session.completion")
            {
                Execute(command, argument);
            }
            else
            {
                frame.Say(registry.Commands.TryGetValue(command, out var named) ? named.Label + ": " + NotHere : NotHere, true);
            }
        };

        var find = SessionWidgets.Tool("search", $"Find a command ({keymap.KeysFor("command.find", null)})", frame.Tokens, palette.Open);
        find.Name = "FindButton";
        HelpTopics.Mark(find, "command.find");
        // ADR-D01.74: the shortcuts button pins the key map, or lets it go, as a short press of its key does.
        string KeysTip() => $"Key map: hold {keyMap?.ShowKey ?? keymap.KeysFor("help.open", HelpCatalog.Keys)} to see it, press it shortly to pin it";
        var help = SessionWidgets.Tool("help.keys", KeysTip(), frame.Tokens, () => Execute("help.open", HelpCatalog.Keys));
        help.Name = "ShortcutsButton";
        HelpTopics.Mark(help, "help.open");
        keysTaken += k => { find.TooltipText = $"Find a command ({k.KeysFor("command.find", null)})"; help.TooltipText = KeysTip(); };
        var preset = SessionWidgets.Tool("preset.next", $"Next of the six presets ({keymap.KeysFor("theme.select", "preset.next")})", frame.Tokens, NextPreset);
        preset.Name = "PresetButton";
        HelpTopics.Mark(preset, "theme.select");
        _revision = new Readout("RevisionReadout", "readout.revision", "Engine revision the core last showed", () => frame.Tokens);
        _position = new Readout("PositionReadout", "readout.position", "History position: the entry in view, of the entries of this run", () => frame.Tokens);
        foreach (Control tool in new Control[] { _revision, _position, preset, find, help })
        {
            frame.BarTools.AddChild(tool);
        }
        frame.BarTools.AddChild(SessionWidgets.Chip(live ? "core" : "no core", frame.Tokens.C(live ? "severity.ok" : "severity.warning"),
            live ? "The core draws the Global view beneath the window" : "The shell was started without the core: nothing is drawn"));
        if (sample != null)
        {
            var chip = SessionWidgets.Chip("sample", frame.Tokens.C("text.dim"), SampleWords);
            chip.Name = "SampleChip";
            frame.BarTools.AddChild(chip);
            _queue = new Readout("QueueReadout", "readout.queue", "Script runs waiting or running (sample)", () => frame.Tokens);
            frame.BarTools.AddChild(_queue);
            frame.BarTools.MoveChild(_queue, 0);
            var clock = new Godot.Timer { WaitTime = .25, Autostart = true, Name = "SampleQueueClock" };
            clock.Timeout += () => { if (GodotObject.IsInstanceValid(_queue)) _queue.Show(HistoryReadouts.Queue(sample.Scripts.Queue.Active.Count())); };
            frame.AddChild(clock);
            sample.Changed += Describe;
        }
        if (ShellJumble.On)
        {
            ShellJumble.Attach(parent, frame, palette, () => cards?.ObservePicks());
            // The live Target key and the menu share the shell's existing picked-cell action.
            if (frame.HostKeys.TryGetValue("E", out pickedTarget))
                frame.HostKeys["E"] = () => Execute("target.current.set", null);
            // B6: the Jumble host in Solve, fed at each shown revision; Local shows its sections.
            var (names, empty, nothing) = ShellJumble.Words(folder);
            // B32 findings: in sample mode the key map holds the help key (ADR-D01.74), so Local's hint names the key
            // that opens the palette there; outside sample mode F1 with no help topic opens the palette (PR 446's reading).
            // The cap map's refusal ("F1 › jumble.cap.unname") is routed the same way.
            Func<string, string>? paletteWords = null;
            if (sample != null)
            {
                string? taken = sample.Keys.ShowKey ?? keymap.Bindings.FirstOrDefault(b => b.Command == "help.open")?.Key;
                string paletteKeys = keymap.KeysFor("command.find", null);
                paletteWords = words => ShellJumble.PaletteRoute(words, taken, paletteKeys);
                empty = paletteWords(empty);
            }
            var sentences = ShellJumble.Sentences(folder);
            Jumble = new JumbleHost(frame, ShellJumble.Feed, names, empty, nothing, sentences["totals.unit"], sentences["totals.above"]);
            Jumble.PaletteWords = paletteWords;
            // B7: Global's totals follow the bound state, pending while none is bound; B8: so does the cap map.
            var host = Jumble;
            var layout = ShellJumble.Layout();
            // B11: the focus layers of Global and Local take the three predicates; the pose rows count them.
            if (sample != null)
            {
                host.Layers = sample.Layers.Values.ToArray();
            }
            else
            {
                host.OwnLayers();
                // ADR-D01.74: the key map names the shell's own keys, as the sample's names its own.
                if (keyMap != null && frame.HostKeys.ContainsKey("E") && registry.Commands.TryGetValue("target.current.set", out var targetRow)) keyMap.Host["E"] = targetRow.Label;
                if (keyMap != null && frame.HostKeys.ContainsKey("F10")) keyMap.Host["F10"] = "Inactive cells";
            }
            // B27: the key map's four held seat entries (ADR-D02.42), from this build's keymap, in either mode.
            if (keyMap != null) keyMap.Seats = SeatKeys.Read(File.ReadAllText(Path.Combine(folder, "keymap.json")));
            host.CoversSource = ShellJumble.Covers;
            host.RecordSource = ShellJumble.Record;
            ShellJumble.BindingChanged += () =>
            {
                host.SetTotals(ShellJumble.Totals());
                host.SetStyle(ShellJumble.Style());
                host.SetCapMap(layout, ShellJumble.CapMap());
                host.SetNu(ShellJumble.Corrections(), ShellJumble.Bound is { } nuBound ? ShellJumble.StateAfter(nuBound) : null);
            };
            host.SetTotals(ShellJumble.Totals());
            host.SetStyle(ShellJumble.Style());
            host.SetCapMap(layout, ShellJumble.CapMap());
            // B26: the ν band and the ν-corrections, from the samples' catalogue and the bound state's totals.
            host.SetNu(ShellJumble.Corrections(), ShellJumble.Bound is { } nuBound ? ShellJumble.StateAfter(nuBound) : null);
            // B9: the 80 meeting caps of a named cap, and its own record where the bound sample has one.
            host.CapRecord = ShellJumble.CapRecord;
            host.SetMeeting(ShellJumble.Meeting());
            // B10: the kept twist checks, from the cap record's release checks, on the sample jobs' tick clock.
            host.CheckSource = ShellJumble.TwistChecks;
            // B22 rest: the palette's Global chart rows switch Global's layout; its border names the chart it draws.
            host.ChartRow = ShellHopf.Request;
            host.Chart = ShellHopf.ChartName;
            ShellHopf.Applied += Describe;
            // B21: the palette's raw view rows and Local's raw view marks.
            host.RawRow = ShellRaw.Request;
            // B32 findings: the palette's cut slice rows, beside the held split key.
            host.SliceRow = ShellSlice.Request;
            host.Raw = () => ShellRaw.Shown;
            ShellRaw.Applied += Describe;
            var jumbleClock = new Godot.Timer { Name = "JumbleClock", WaitTime = 0.15, Autostart = true };
            jumbleClock.Timeout += host.Tick;
            frame.AddChild(jumbleClock);
            // Step C-.40: on the base puzzle Local draws cell 0 without a Jumble cap, so the Jumble empty sentence stays hidden.
            host.SetDrawnWithoutCap(live && !ShellLocal.Plan.Jumble);
            ShellJumble.Host = host;
            // B15: the tint channel and the focus layers, drawn by the core.
            ShellTint.Attach(host, frame);
            ShellError.Attach(host, frame);
            ShellGrip.Attach(host, frame);
            // ADR-D01.74: the key map's twist keycaps read the same grip.
            ShellGripKeys.Attach(keyMap);
            cards = new ShellCards(frame, host, sentences["totals.unit"]);
            frame.SetMainViews(global, Jumble.Surface);
            Jumble.Solving(sample?.Solve.Strip, sample?.Solve.Rail);
            Jumble.Changed += Describe;
        }

        void Dress()
        {
            if (GodotObject.IsInstanceValid(specimen)) specimen.Show(frame.Tokens);
            if (GodotObject.IsInstanceValid(gallery)) gallery.Rebuild();
            if (GodotObject.IsInstanceValid(history)) history.Refresh();
            if (GodotObject.IsInstanceValid(bar)) bar.Refresh();
            Describe();
        }
        void Icons()
        {
            _revision.Dress(); _position.Dress(); _queue?.Dress();
            ChromeIcons.Apply(find, "search", frame.Tokens.Px("icon.small")); ChromeIcons.Apply(help, "help.keys", frame.Tokens.Px("icon.small"));
            ChromeIcons.Apply(preset, "preset.next", frame.Tokens.Px("icon.small"));
        }
        frame.Rebuilt += () => { Icons(); Dress(); };
        frame.Recoloured += Dress;
        ShellExplore.Changed += Describe;
        frame.TreeExiting += () => ShellExplore.Changed -= Describe;
        Moves.Changed += () => { if (GodotObject.IsInstanceValid(history)) history.Refresh(); Describe(); ShellResearch.Model?.Committed(); };
        frame.SelectWorkspace(frame.Workspaces.Order[0]);
        frame.MarkBaseline();
        frame.Flush();
        Describe();
    }

    private void Describe()
    {
        if (!GodotObject.IsInstanceValid(Frame))
        {
            return;
        }
        _revision.Set("r " + _shown.ToString("0000", CultureInfo.InvariantCulture));
        _position.Show(HistoryReadouts.Position(Moves));
        ShellSession.Describe(_live && _shown <= int.MaxValue ? (int)_shown : null);
        Frame.Global.ClearMarks();
        if (_live)
        {
            Frame.Global.AddMark("mark.committed", "committed", "state.committed", "This view shows the committed state.");
            Frame.Global.SetStatus(null, new[] { "J K L: twist", "Shift: inverse" }, "revision " + _shown.ToString(CultureInfo.InvariantCulture));
        }
        else
        {
            Frame.Global.SetStatus("not drawn", Array.Empty<string>(), null);
        }
        Frame.Local.ClearMarks();
        Frame.Local.SetStatus(Frame.LocalDrawn ? null : "not drawn yet", Array.Empty<string>(), null);
        ViewLink.Mark(Frame, _rotationLink);
        Jumble?.Marks(Frame.Global);
        Jumble?.Marks(Frame.Local);
        Sample?.Marks(Frame.Global);
        Sample?.Marks(Frame.Local);
        ShellSession.Marks(Frame.Global);
        ShellSession.Marks(Frame.Local);
        foreach (ViewFrame view in new[] { Frame.Global, Frame.Local })
        {
            ShellExplore.Marks(view);
            foreach (ViewMark mark in ColourMark.BorderMarks(Frame.Tokens, view.ViewName))
                view.AddMark(mark.Glyph, mark.Words, mark.Colour, mark.Tooltip).SetMeta("slot", ColourMark.Slot);
        }
    }

    private string? _localWords;

    /// <summary>Step C-.40: Local's readout from its plan (the base puzzle's), shown only while Local is drawn.</summary>
    internal void LocalLine(string? words)
    {
        words = Frame.LocalDrawn ? words : null;
        if (words == _localWords)
        {
            return;
        }
        _localWords = words;
        ViewLine.Set(Frame.Local, "LocalPlanLine", words, LocalPlan.Hint, Frame.Tokens.C("text.dim"));
    }

    /// <summary>The window pixels of the Global view's opening; empty while the view is not on screen.</summary>
    public Rect2I Opening() => Pixels(Frame.Opening);

    /// <summary>
    /// The window pixels of Local's drawn opening (ADR-D02 B18c): the top of Local's slot while a cap is named; empty
    /// without --d02, while no cap is named, or while Local is not on screen.
    /// </summary>
    public Rect2I LocalOpening() => Pixels(Frame.LocalOpening);

    internal void OpenContextMenu(Vector2 at) => _openContextMenu?.Invoke(at);

    private static Rect2I Pixels(Rect2 area) =>
        new((int)Math.Round(area.Position.X), (int)Math.Round(area.Position.Y), (int)Math.Floor(area.Size.X), (int)Math.Floor(area.Size.Y));

    /// <summary>
    /// The one router's question (no key does both): whether the chrome takes this key event. A key-down the chrome
    /// takes keeps its key-up as well. A bound command the shell's chrome cannot carry out is not taken, so an undo
    /// or a redo still reaches the session. With a live session a twist key is the session's too: sample mode's
    /// draft phase carries out the twist row, and taking J for it would keep the router from twisting. The frame's
    /// own keys (a key capture, an open overlay, the host keys and text focus) are still taken first.
    /// </summary>
    public bool Takes(InputEventKey key)
    {
        if (!key.Pressed)
        {
            return _taken.Remove(key.PhysicalKeycode);
        }
        if (key.Keycode is Key.Shift or Key.Ctrl or Key.Alt or Key.Meta or Key.None)
        {
            return false;
        }
        var name = Keymap.KeyName(key.CtrlPressed, key.AltPressed, key.ShiftPressed, OS.GetKeycodeString(key.Keycode));
        // The Local row's key (L in Global) is also the router's twist key KeyL: in a live session the twist keeps it,
        // and the row runs from the palette or a script.
        var takes = Frame.Claims(name, binding => _unavailable(binding.Command, binding.Argument) == null &&
            !(_live && KeyMapModel.IsTwist(binding.Command)) &&
            !(_live && binding.Command == ViewPanes.LocalRow && ShellTurn.ItemOf(PhysicalChord(name)) != null));
        if (takes)
        {
            _taken.Add(key.PhysicalKeycode);
        }
        return takes;
    }

    /// <summary>A key name of the key map ("L", "Shift+L") as the router's physical chord ("KeyL", "Shift+KeyL").</summary>
    internal static string PhysicalChord(string name)
    {
        var at = name.LastIndexOf('+') + 1;
        return name.Substring(0, at) + "Key" + name.Substring(at);
    }

    /// <summary>A twist the session committed: a History entry at the engine revision that shows it.</summary>
    public void Committed(string chord, ulong revision)
    {
        var inverse = chord.Contains("Shift+", StringComparison.Ordinal);
        var key = chord.Substring(chord.LastIndexOf('+') + 1).Replace("Key", "", StringComparison.Ordinal);
        var name = $"twist {key}{(inverse ? "′" : "")}";
        Moves.Commit("hand", name, DateTime.Now.ToString("HH:mm", CultureInfo.InvariantCulture), $"{name} at revision {revision.ToString(CultureInfo.InvariantCulture)}", new[] { name });
    }

    /// <summary>An undo or a redo the session carried out: the History cursor follows.</summary>
    public void Stepped(string chord)
    {
        if (chord.EndsWith("KeyZ", StringComparison.Ordinal) && !chord.Contains("Shift+", StringComparison.Ordinal))
        {
            Moves.Undo();
        }
        else
        {
            Moves.Redo();
        }
    }

    public void Refused(string chord) => Frame.Say($"{chord}: the session refused this twist", true);

    /// <summary>The engine revision of the newest presentation the core confirmed.</summary>
    public void Shown(ulong revision)
    {
        if (revision != _shown)
        {
            _shown = revision;
            Describe();
        }
    }
}
