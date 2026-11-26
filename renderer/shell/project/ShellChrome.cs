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
        ("editor", "Editor"), ("preview", "Preview"), ("canvas", "Canvas"), ("reference", "Reference"), ("scriptview", "Script view"), ("greybox", "Greybox"),
    };
    public const string NotHere = "not in this build yet";

    public ChromeFrame Frame { get; }
    public MoveHistory Moves { get; } = new();
    /// <summary>
    /// Sample mode (B4a): ADR-D01's whole frame on sample data, behind launch.py --sample, or null when it is off. The
    /// views, the History dock, the readouts and the session's undo and redo stay the real session's.
    /// </summary>
    internal SampleMount? Sample { get; }
    /// <summary>ADR-D02's Jumble host in Solve (B6), with D02 on.</summary>
    internal JumbleHost? Jumble { get; }
    /// <summary>Whether sample mode is on: launch.py --sample sets M600_SAMPLE=1.</summary>
    public static bool SampleOn => System.Environment.GetEnvironmentVariable("M600_SAMPLE") == "1";
    public const string SampleWords = "Sample mode: the strip, the rail, the draft line, the Library, the inspector, Problems, Console, Tasks, the scripts and History (sample) hold sample data. The views, History, the revision and the history position are this session's.";
    private readonly bool _live;
    private readonly Readout _revision, _position;
    private readonly Readout? _queue;
    private readonly HashSet<Key> _taken = new();
    private readonly Func<string, string?, string?> _unavailable;
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

    public ShellChrome(Control parent, string folder, bool live)
    {
        _live = live;
        ChromeFonts.SetRoots(PackageFolders(folder));
        var iconSet = IconSet.Load(Path.Combine(folder, "icons.json"));
        ChromeIcons.Use(iconSet);
        var design = DesignSystem.Load(Path.Combine(folder, "system.json"));
        var themes = Directory.GetFiles(Path.Combine(folder, "themes"), "*.json").OrderBy(p => p, StringComparer.Ordinal)
            .Select(p => ThemeFamily.Load(p, design)).ToArray();
        var motions = MotionTable.Parse(File.ReadAllText(Path.Combine(folder, "motion.json")), design);
        var palettes = StickerPalette.Parse(File.ReadAllText(Path.Combine(folder, "palettes.json")));
        var commandIcons = CommandIcons.Parse(File.ReadAllText(Path.Combine(folder, "command-icons.json")));
        ChromeBody.Commands = commandIcons;
        var look = new Appearance(design, themes);
        if (DisplayServer.IsDarkModeSupported())
        {
            look.FollowSystem(DisplayServer.IsDarkMode());
        }
        look.SetSystemScale(Math.Clamp(Math.Round(DisplayServer.ScreenGetDpi() / 96.0 * 4) / 4, 1, 3));
        var frame = Frame = new ChromeFrame(look.Resolve(), WorkspaceSet.Load(Path.Combine(folder, "workspaces.json"))) { OpenGround = live };
        frame.Name = "ShellChrome";
        parent.AddChild(frame);
        // A check's aid: M600_FRAME_PICTURE names a file that gets the window's own picture (the frame without the
        // core's picture, which lies beneath the window) a few seconds after the start.
        if (System.Environment.GetEnvironmentVariable("M600_FRAME_PICTURE") is { Length: > 0 } picture)
        {
            parent.GetTree().CreateTimer(5).Timeout += () => parent.GetViewport().GetTexture().GetImage().SavePng(picture);
        }
        frame.SetAnchorsAndOffsetsPreset(Control.LayoutPreset.FullRect);

        var history = new HistoryPanel(Moves, new LayoutHistory(), () => frame.Tokens);
        frame.AddDockPanel(DockSide.Right, "history", "History", "panel.history", history);
        foreach (var panel in Panels)
        {
            frame.AddDockPanel(panel.Side, panel.Id, panel.Title, panel.Glyph, ChromeBody.PaneState("notdrawn", panel.Title + ": " + NotHere));
        }
        var appearance = new AppearancePanel(look);
        frame.AddDockPanel(DockSide.Right, "appearance", "Appearance", "panel.appearance", appearance);
        var specimen = new TokenSpecimen();
        frame.AddDockPanel(DockSide.Left, "tokens", "Tokens", "panel.tokens", specimen);
        foreach (var pane in Panes)
        {
            frame.AddPane(pane.Id, pane.Title, ChromeBody.PaneState("notdrawn", pane.Title + ": " + NotHere));
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
        var keymap = Keymap.Load(Path.Combine(folder, "keymap.json"), registry);
        SampleMount? sample = null;
        var gallery = new ComponentGallery(() => frame.Tokens, motions, palettes, iconSet, commandIcons, registry.Order.ToArray());
        frame.AddPane("gallery", "Gallery", gallery);
        string? Unavailable(string command, string? argument) => command switch
        {
            "command.find" or "transient.close" or "help.open" or "motion.reduced.set" => null,
            _ when command.StartsWith("radial.", StringComparison.Ordinal) => null,
            "workspace.set" or "theme.select" or "density.preset.set" => argument != null ? null : "it needs an argument; choose one of its rows",
            _ when ShellJumble.On && ShellJumble.Drafts.Contains(command) => ShellJumble.DraftReason,
            _ when sample != null && sample.Handles(command) => sample.Unavailable(command, argument),
            _ => NotHere,
        };
        _unavailable = Unavailable;
        var commands = new PaletteModel(registry, keymap, Unavailable);
        var palette = new CommandPalette(commands, () => frame.Tokens);
        frame.Overlays.AddChild(palette);
        var sheet = new ShortcutSheet(registry, keymap, () => frame.Tokens);
        frame.Overlays.AddChild(sheet);
        var radial = new RadialOverlay(registry, Unavailable, () => frame.Tokens);
        frame.Overlays.AddChild(radial);
        frame.OverlayCommands.Add("transient.close");
        frame.OverlayCommands.Add("help.open");
        frame.OverlayCommands.Add("command.find");
        if (SampleOn)
        {
            sample = Sample = new SampleMount(frame, registry, keymap, ProjectSettings.GlobalizePath("res://sample-root"),
                id => motions.Seconds(id, design, look.ReducedMotion), Execute,
                k => { keymap = k; frame.Keys = k; commands.Keymap = k; sheet.Keymap = k; }, palette, sheet);
        }

        void ApplyLook(bool rebuild, bool scene) => frame.ApplyTokens(look.Resolve(), rebuild);
        appearance.Changed += ApplyLook;
        void NextPreset()
        {
            var all = themes.SelectMany(t => ThemeFamily.SceneLooks.Select(l => (t.Id, Look: l))).ToArray();
            var at = (Array.FindIndex(all, x => x.Id == look.Theme && x.Look == look.SceneLook) + 1) % all.Length;
            appearance.Edit(a => { a.SetTheme(all[at].Id); a.SetSceneLook(all[at].Look); }, true, true);
            frame.Say($"preset {at + 1}/{all.Length}: {look.Family.Name} · {look.SceneLook}");
        }
        void NextDensity() => appearance.Edit(a => a.SetDensity(design.Densities[(design.Densities.ToList().IndexOf(a.Density) + 1) % design.Densities.Count]), true, false);
        void Execute(string command, string? argument)
        {
            if (Unavailable(command, argument) is { } why)
            {
                frame.Say($"{registry.Commands[command].Label}: {why}", true);
                return;
            }
            sample?.Echo(command, argument);
            switch (command)
            {
                case "command.find": palette.Open(); break;
                case "help.open": if (sheet.IsOpen) sheet.Close(); else sheet.Open(); break;
                case var menu when keymap.Radials.ContainsKey(menu):
                    radial.Open(sample != null ? sample.WithContext(keymap.Radials[menu]) : keymap.Radials[menu], frame.GetLocalMousePosition());
                    break;
                case "transient.close":
                    sample?.Keys.Shown.Pin(false);
                    if (frame.OpenOverlay is { } open) open.Close();
                    else if (sample != null && sample.Inspect.Feedback.CancelLatest()) { }
                    else if (frame.Model.FocusMode) frame.Model.SetFocusMode(false);
                    else if (frame.Model.PairMaximised) frame.Model.SetPairMaximised(false);
                    break;
                case "workspace.set" when argument == "layout:focus": frame.Model.SetFocusMode(!frame.Model.FocusMode); break;
                case "workspace.set" when argument == "layout:maximise": frame.Model.SetPairMaximised(!frame.Model.PairMaximised); break;
                case "workspace.set":
                    if (!frame.SelectWorkspace(argument!.Substring(argument.IndexOf(':') + 1))) frame.Say("No workspace is named " + argument, true);
                    break;
                case "motion.reduced.set": appearance.Edit(a => a.ReducedMotion = argument == null ? !a.ReducedMotion : argument == "on", true, false); break;
                case "theme.select" when argument == "preset.next": NextPreset(); break;
                case "theme.select": appearance.Edit(a => a.SetTheme(themes[(Array.FindIndex(themes, t => t.Id == a.Theme) + 1) % themes.Length].Id), true, true); break;
                case "density.preset.set": NextDensity(); break;
                case var handled when sample != null && sample.Handles(handled): sample.Execute(command, argument); break;
            }
        }
        palette.Run += row => Execute(row.Command, row.Argument);
        radial.Chosen += entry => Execute(entry.Command, entry.Argument);
        frame.Keys = keymap;
        frame.KeyCommand += binding => Execute(binding.Command, binding.Argument);
        gallery.Asked += what =>
        {
            if (what == "preset") NextPreset();
            else if (what == "contrast") appearance.Edit(a => a.HighContrast = !a.HighContrast, true, false);
            else if (what == "character") appearance.Edit(a => a.OtherCharacter = !a.OtherCharacter, true, false);
            else NextDensity();
        };
        gallery.PaletteChosen += chosen => frame.Say(chosen.Name + ": the core does not take a sticker palette yet", true);
        gallery.MotionChanged += () => appearance.Edit(a => a.ReducedMotion = gallery.Motion.Reduced, true, false);

        // The commit bar is the frame's own; the host stages no preview yet, so it stays empty.
        var bar = new CommitBar(new PreviewModel(), () => frame.Tokens) { CommitKeys = keymap.KeysFor("operation.preview.commit", null) };
        bar.Command += Execute;
        if (sample == null)
        {
            frame.SetStrip(bar);
        }
        history.Command += (command, argument) => frame.Say(registry.Commands.TryGetValue(command, out var named) ? named.Label + ": " + NotHere : NotHere, true);

        var find = SessionWidgets.Tool("search", $"Find a command ({keymap.KeysFor("command.find", null)})", frame.Tokens, palette.Open);
        find.Name = "FindButton";
        var help = SessionWidgets.Tool("help.keys", $"Keyboard shortcuts ({keymap.KeysFor("help.open", null)})", frame.Tokens, sheet.Open);
        help.Name = "ShortcutsButton";
        var preset = SessionWidgets.Tool("preset.next", $"Next of the six presets ({keymap.KeysFor("theme.select", "preset.next")})", frame.Tokens, NextPreset);
        preset.Name = "PresetButton";
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
            ShellJumble.Attach(parent, frame, palette);
            // B6: the Jumble host in Solve, fed at each shown revision; Local shows its sections.
            var (names, empty, nothing) = ShellJumble.Words(folder);
            Jumble = new JumbleHost(frame, ShellJumble.Feed, names, empty, nothing);
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
        Moves.Changed += () => { if (GodotObject.IsInstanceValid(history)) history.Refresh(); Describe(); };
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
        Frame.Local.SetStatus("not drawn yet", Array.Empty<string>(), null);
        Jumble?.Marks(Frame.Global);
        Jumble?.Marks(Frame.Local);
        Sample?.Marks(Frame.Global);
        Sample?.Marks(Frame.Local);
    }

    /// <summary>The window pixels of the Global view's opening; empty while the view is not on screen.</summary>
    public Rect2I Opening()
    {
        var area = Frame.Opening;
        return new Rect2I((int)Math.Round(area.Position.X), (int)Math.Round(area.Position.Y), (int)Math.Floor(area.Size.X), (int)Math.Floor(area.Size.Y));
    }

    /// <summary>
    /// The one router's question (no key does both): whether the chrome takes this key event. A key-down the chrome
    /// takes keeps its key-up as well. A bound command the shell's chrome cannot carry out is not taken, so an undo
    /// or a redo still reaches the session.
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
        var takes = Frame.Claims(name, binding => _unavailable(binding.Command, binding.Argument) == null);
        if (takes)
        {
            _taken.Add(key.PhysicalKeycode);
        }
        return takes;
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
