using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using Godot;
using Magic600.Chrome;

namespace LookLab.App.Live;

// The lab's script interface (ADR-D01 section 6 and ADR-D01.57 to ADR-D01.59): the editor with its run queue and
// results, the Scripts dock, the console, the script panel slots, the six mounts, the canvas, packages by others
// and the solver's own key bindings. The entry points of the sample programs give sample results here; every
// other part (entry points as commands, queue, key binding, presets, slots, diagnostics, fixes) is the chrome library.
internal sealed class LabScripts
{
    public const string DefaultPreset = "default", CensusPanel = "census.orbits", TunerPanel = "mounts.tuner", FactsPanel = "mounts.facts", RingView = "scriptview";
    public static string SlotId(int slot) => "slot." + (slot + 1);
    public SixCsEditor Editor { get; }
    public ScriptList List { get; }
    public ScriptCanvas Canvas { get; }
    public ScriptCanvas View { get; }
    public ConsentSheet Consent { get; }
    public CommandBar Bar { get; }
    public FeedbackPanel EditorConsole { get; }
    public SlotTabs Slots { get; } = new();
    public RunQueue Queue { get; } = new();
    public ResultStore Results { get; } = new();
    public MountSet Mounts { get; } = new();
    public PackageStore Installed { get; } = new();
    public IReadOnlyList<PackageManifest> Packages => packages;
    public KeymapOverrides Overrides { get; private set; } = new();
    public LineHistory Lines { get; } = new();
    public string Preset { get; private set; } = DefaultPreset;
    public Keymap Keys { get; private set; }
    public IReadOnlyList<ScriptProgram> Programs => programs;
    public string? LastResult { get; private set; }
    public bool Solving { get; private set; } = true;
    public bool SafeMode { get; private set; }
    // The puzzle element a chosen problem row is about: both views mark it.
    public string? Marked { get; private set; }
    public IReadOnlyList<string> Presets => presets.Keys.Prepend(DefaultPreset).ToArray();
    private static readonly (string Id, string Label, string Help)[] BuiltIns =
    {
        ("script.restart", "Scripts: restart", "Load every program again and empty the queue."),
        ("script.safe", "Scripts: safe mode", "Scripts stay off, also at the next start."),
        ("script.abort", "Scripts: abort runs", "End every run that waits or runs."),
        ("script.faults", "Editor: sample faults", "One diagnostic of each severity in the open program."),
        ("script.unused", "Editor: select unused definitions", "Select every definition nothing uses."),
        ("script.reference", "Editor: insert the selection", "A typed reference to the selection at the caret."),
        ("script.bar", "Command bar", "One line about the selection. Enter never commits."),
    };
    private readonly List<ScriptProgram> programs = new();
    private readonly List<PackageManifest> packages = new();
    private readonly SortedDictionary<string, string> presets = new(StringComparer.Ordinal);
    private readonly Dictionary<int, int> tasks = new();
    private readonly HashSet<string> ran = new(StringComparer.Ordinal), failed = new(StringComparer.Ordinal);
    private readonly Dictionary<string, string> tuner = new(StringComparer.Ordinal) { ["ring"] = "4", ["mode"] = "0", ["mirror"] = "off", ["anchor"] = "cell 117", ["tint"] = "state.preview" };
    private static readonly string[] Tints = { "state.preview", "accent.strong", "severity.ok", "severity.warning" };
    private readonly ChromeFrame frame;
    private readonly LabDemo demo;
    private readonly LabInspect inspect;
    private readonly FeedbackModel feedback;
    private readonly CommandRegistry registry;
    private readonly Keymap defaults;
    private readonly Action<string, string?> run;
    private readonly Action<Keymap> keysChanged;
    private readonly string exportPath, root;
    private readonly Button presetButton, safeButton, solvingButton;
    private ScriptPanelHost? facts;
    private string? followed;
    private bool quiet;
    private string exported = "";

    public LabScripts(ChromeFrame frame, LabDemo demo, LabInspect inspect, CommandRegistry registry, Keymap defaults, string root, Action<string, string?> run, Action<Keymap> keysChanged)
    {
        this.frame = frame; this.demo = demo; this.inspect = inspect; feedback = inspect.Feedback; this.registry = registry; this.defaults = defaults; this.run = run; this.keysChanged = keysChanged; Keys = defaults;
        this.root = root; exportPath = Path.Combine(root, "work/loop-memory/looklab/keymap-export.json");
        foreach (string path in Directory.GetFiles(Path.Combine(root, "tools/looklab/data/samples/packages"), "*.json").OrderBy(p => p, StringComparer.Ordinal))
            packages.Add(PackageManifest.Parse(File.ReadAllText(path)));
        foreach (string path in Directory.GetFiles(Path.Combine(root, "tools/looklab/data/design/keymaps"), "*.json").OrderBy(p => p, StringComparer.Ordinal))
        {
            string text = File.ReadAllText(path); Keymap.Parse(text, registry, defaults); presets[Path.GetFileNameWithoutExtension(path)] = text;
        }
        foreach ((string id, string label, string help) in BuiltIns)
            if (!registry.Commands.ContainsKey(id)) registry.Add(new ChromeCommand(id, label, "app", new[] { "any" }, help));
        Load(); Register(); exported = string.Join(" ", programs.SelectMany(x => x.Entries).Select(x => x.Command));
        Editor = new SixCsEditor(() => frame.Tokens, Queue, Results);
        List = new ScriptList(() => frame.Tokens)
        {
            KeysOf = e => Keys.KeysFor(e.Command, null), Mounts = Mounts, MarkOf = e => Results.Of(e.Command)?.Mark ?? -1,
            StateOf = e => Queue.Active.Any(i => i.Command == e.Command) ? "pending" : failed.Contains(e.Command) ? "failed" : Results.Of(e.Command) is { Since: not null } ? "stale" : ran.Contains(e.Command) ? "ok" : "",
            ConflictOf = e => Overrides.Bindings.Where(b => b.Command == e.Command).Select(b => KeyRules.Conflict(Base(), b.Key, b.Context, b.Command) is { } other ? $"{b.Key} also ran: {LabelOf(other)}" : null).FirstOrDefault(c => c != null),
        };
        Canvas = new ScriptCanvas(() => frame.Tokens); View = new ScriptCanvas(() => frame.Tokens, "ScriptView", "No script view yet");
        Consent = new ConsentSheet(() => frame.Tokens); Bar = new CommandBar(() => frame.Tokens); frame.Overlays.AddChild(Consent); frame.Overlays.AddChild(Bar);
        EditorConsole = new FeedbackPanel(feedback, FeedbackPanel.Kind.Console, () => frame.Tokens) { Name = "EditorConsole", Completer = Words };
        inspect.Console.Completer = Words;
        Editor.SetPrograms(programs); List.SetPrograms(programs);
        Editor.AddLower("console", "panel.console", "Console", EditorConsole);
        Editor.Run += (program, entry) =>
        {
            if (entry == null) Fail("no entry point here");
            else run(entry.Command, null);
        };
        Editor.CommitKey += () => run("operation.preview.commit", null);
        Editor.Abort += AbortRuns;
        Editor.Rerun += result => run(result.Command, null);
        // The caret is the focus source: resting in a preview entry point stages its preview.
        Editor.Focused += (program, entry) =>
        {
            if (entry.Kind != ScriptKind.Preview || SafeMode || !Editor.Code.HasFocus() || (followed == entry.Command && demo.Preview.Pending != null) || Queue.Active.Any()) return;
            followed = entry.Command; run(entry.Command, null);
        };
        Editor.Edited += program =>
        {
            programs[programs.FindIndex(p => p.Id == program.Id)] = program;
            Register(); List.SetPrograms(programs);
            feedback.ClearSource(program.File);
            foreach (ScriptDiagnostic d in SixCsService.Diagnose(program.Source)) feedback.Report(d.Severity, d.Text, program.File, d.Line);
            string now = string.Join(" ", programs.SelectMany(x => x.Entries).Select(x => x.Command));
            if (now != exported) { exported = now; ShowSlots(); ApplyMounts(); }
        };
        List.Opened += (program, line) => Open(program.Id, line);
        List.RunEntry += entry => run(entry.Command, null);
        List.BindEntry += entry => Capture(entry.Command);
        List.InstallAsked += Consent.Ask;
        List.Revoked += package => { Installed.Revoke(package.Id); Reload(); Say(ConsoleKind.Echo, $"revoked {package.Title}"); };
        Consent.Decided += (package, install) =>
        {
            if (!install) { Say(ConsoleKind.Echo, $"declined {package.Title}"); return; }
            if (Installed.Install(package)) { Reload(); Say(ConsoleKind.Echo, $"installed {package.Title}"); }
        };
        Bar.Entered += RunLine;
        presetButton = ChromeWidgets.Tool("", "Keymap preset: click for the next one. Your own bindings stay on top."); presetButton.Name = "KeymapPreset";
        presetButton.Pressed += () => { IReadOnlyList<string> all = Presets; SetPreset(all[(all.ToList().IndexOf(Preset) + 1) % all.Count]); };
        Button export = SessionWidgets.Tool("keymap.export", "Export your own bindings as a keymap file", frame.Tokens, () => run("file.keymap.export", null)); export.Name = "KeymapExport";
        Button back = SessionWidgets.Tool("keymap.defaults", "Drop your own bindings", frame.Tokens, () => run("keymap.defaults", null)); back.Name = "KeymapDefaults";
        Button restart = SessionWidgets.Tool("scripts.restart", "Restart scripts: load every program again", frame.Tokens, () => run("script.restart", null)); restart.Name = "ScriptsRestart";
        safeButton = SessionWidgets.Tool("scripts.safe", "Safe mode: scripts stay off, also at the next start", frame.Tokens, () => run("script.safe", null)); safeButton.Name = "SafeMode"; safeButton.ToggleMode = true;
        solvingButton = SessionWidgets.Tool("lock", "Solving session: a package prepares no preview", frame.Tokens, () => { Solving = !Solving; Apply(); }); solvingButton.Name = "SolvingSession"; solvingButton.ToggleMode = true;
        foreach (Button b in new[] { presetButton, export, back, restart, safeButton, solvingButton }) List.Tools.AddChild(b);
        inspect.Console.Entered += RunLine; EditorConsole.Entered += RunLine;
        inspect.Problems.Opened += p =>
        {
            Marked = p.Element; frame.Invalidate();
            if (programs.FirstOrDefault(x => x.File == p.Source) is { } program) Open(program.Id, p.Line);
        };
        // A commit or an undo dims the results computed before it and names what came between (ADR-D01.58).
        demo.Moves.Changed += () =>
        {
            string name = demo.Moves.Cursor > 0 && demo.Moves.Cursor <= demo.Moves.Entries.Count ? demo.Moves.Entries[demo.Moves.Cursor - 1].Name : "the first state";
            if (Results.StateChanged(name) > 0 && GodotObject.IsInstanceValid(List)) List.Refresh();
            Callable.From(Sources).CallDeferred();
        };
        Queue.Finished += item =>
        {
            if (tasks.Remove(item.Id, out int task)) feedback.Finish(task, true);
            RunEntry(item.Command);
        };
        Queue.Changed += () => { if (GodotObject.IsInstanceValid(List)) List.Refresh(); };
        // A cancelled task takes its run with it: Esc and a commit both cancel through the task list.
        feedback.Ended += task =>
        {
            if (task.State != TaskState.Cancelled) return;
            int[] runs = tasks.Where(t => t.Value == task.Id).Select(t => t.Key).ToArray();
            foreach (int id in runs) { tasks.Remove(id); Queue.Abort(i => i.Id == id); }
        };
        var clock = new Timer { WaitTime = .05, Autostart = true, Name = "ScriptRunClock" };
        clock.Timeout += () =>
        {
            Queue.Advance(clock.WaitTime);
            if (Queue.Items.FirstOrDefault(i => i.State == RunState.Running) is { } now && tasks.TryGetValue(now.Id, out int task)) feedback.Progress(task, now.Progress);
        };
        frame.AddChild(clock);
        Slots.Changed += ShowSlots; Mounts.Changed += () => { ApplyMounts(); if (GodotObject.IsInstanceValid(List)) List.Refresh(); };
        inspect.Inspector.Changed += () => { Targets(); Callable.From(Sources).CallDeferred(); };
        frame.SetContent("editor", Editor); frame.SetContent("scripts", List); frame.SetContent("canvas", Canvas); frame.SetContent(RingView, View);
        frame.Rebuilt += Refresh; frame.Recoloured += Refresh;
        ShowSlots(); Apply();
    }

    private IReadOnlyList<string> Words(string prefix) =>
        SixCsService.LineCompletions(prefix, programs.SelectMany(p => p.Entries).Select(e => e.Program + "." + e.Name).Concat(BuiltIns.Select(b => b.Id)));

    private string LabelOf(Binding b) => b.Label ?? registry.Commands[b.Command].Label;

    // The sample programs on file and the programs of the installed packages.
    private void Load()
    {
        programs.Clear();
        foreach (string path in Directory.GetFiles(Path.Combine(root, "tools/looklab/data/samples"), "*.6cs").OrderBy(p => p, StringComparer.Ordinal))
            programs.Add(new ScriptProgram(Path.GetFileNameWithoutExtension(path), Path.GetFileName(path), File.ReadAllText(path).Replace("\r\n", "\n")));
        foreach (PackageManifest package in packages.Where(p => Installed.Installed.Contains(p.Id)))
            programs.Add(new ScriptProgram(Path.GetFileNameWithoutExtension(package.File), package.File, package.Source));
    }

    private PackageManifest? PackageOf(string program) => packages.FirstOrDefault(p => Path.GetFileNameWithoutExtension(p.File) == program);

    // The author class the host records for a program (ADR-D01.4 item 4): a sample program on file is the
    // solver's own, an installed package's program is another author's, and anything else is of unknown origin.
    // No shipped built-in tool runs in this lab; the gallery shows that frame.
    public AuthorClass AuthorOf(string? program)
    {
        if (program == null) return AuthorClass.Other;
        string name = program.EndsWith(".6cs", StringComparison.Ordinal) ? Path.GetFileNameWithoutExtension(program) : program.Split('.')[0];
        return ScriptMarks.Of(shipped: false, digestMatches: false, own: programs.Any(p => p.Id == name) && PackageOf(name) == null);
    }

    private void Reload()
    {
        Load(); Register(); Editor.SetPrograms(programs); List.Packages = packages.Select(p => (p, Installed.Installed.Contains(p.Id))).ToArray(); List.SetPrograms(programs);
        ShowSlots(); ApplyMounts();
    }

    // Restart scripts: every program is loaded again, the queue and the results are emptied.
    public void Restart()
    {
        foreach (int task in tasks.Values.ToArray()) feedback.Cancel(task);
        tasks.Clear(); Queue.Clear(); Results.Clear(); ran.Clear(); failed.Clear(); followed = null;
        Reload(); Say(ConsoleKind.Echo, $"restarted {programs.Count} programs");
    }

    public void SetSafeMode(bool on)
    {
        SafeMode = on;
        if (on) { AbortRuns(); }
        Apply(); ShowSlots(); ApplyMounts();
        frame.Say(on ? "Safe mode: scripts are off" : "Scripts are on", on);
    }

    private void Refresh()
    {
        Editor.Refresh(); List.Refresh(); Canvas.Repaint(); View.Repaint(); EditorConsole.Refresh();
        foreach (ScriptPanelHost host in frame.FindChildren("ScriptPanel_*", "", true, false).OfType<ScriptPanelHost>()) host.Refresh();
        ShowOverlay(); Sources();
    }

    // Every exported fn or command is a command row of its own, so the palette lists it and a key can be bound to it.
    private void Register()
    {
        foreach (ScriptProgram p in programs)
            foreach (ScriptEntry e in p.Entries)
                if (!registry.Commands.ContainsKey(e.Command))
                    registry.Add(new ChromeCommand(e.Command, $"{p.File}: {e.Name}", e.Kind == ScriptKind.View ? "view" : "query", new[] { "any" },
                        $"Entry point of {p.File}; a run ends in {(e.Kind == ScriptKind.Preview ? "a preview" : e.Kind == ScriptKind.View ? "a view or layout change" : "a read result")}, never a commit."));
    }

    public ScriptEntry? Find(string command) => programs.SelectMany(p => p.Entries).FirstOrDefault(e => e.Command == command);
    private static bool BuiltIn(string command) => BuiltIns.Any(b => b.Id == command);

    public string? Unavailable(string command)
    {
        if (BuiltIn(command)) return null;
        if (Find(command) is not { } entry) return "its program no longer exports it";
        if (SafeMode) return "safe mode is on";
        return Mounts.IsOn(entry.Program, MountKind.Palette) ? null : "its entries are switched off";
    }

    public void Open(string program, int line = 0)
    {
        if (!frame.Model.IsOpen("editor")) frame.Model.Drop("editor", FrameModel.Pair, DropZone.Right); else frame.Model.Activate("editor");
        Editor.Show(program, line);
    }

    private void Say(ConsoleKind kind, string text, string? notice = null)
    {
        feedback.Log(ConsoleRows.Write(kind, text));
        frame.Say(notice ?? text, kind is ConsoleKind.Error or ConsoleKind.Lock);
    }

    private void Fail(string text) => Say(ConsoleKind.Error, text);

    private void Result(ScriptEntry entry, string value, string? notice = null)
    {
        Results.Set(entry, value); LastResult = $"{entry.Program}.{entry.Name}({Argument(entry)}) = {value}";
        Say(ConsoleKind.Result, LastResult, notice ?? $"{entry.Name} = {value}");
    }

    private string Argument(ScriptEntry entry) => programs.First(p => p.Id == entry.Program).Source.Split('\n')[entry.Line - 1].Contains("ring: Nat") ? tuner["ring"] : "";

    // The .6cs line a command-layer action is echoed as in the console log (ADR-D01.57).
    public void Echo(string command, string? argument)
    {
        if (quiet || command is "command.find" or "help.open" or "transient.close" || command.StartsWith("radial.", StringComparison.Ordinal)) return;
        string word = Find(command) is { } entry ? entry.Program + "." + entry.Name : command;
        feedback.Log(argument == null ? word + "();" : $"{word}(\"{argument}\");");
    }

    // Every run goes through the visible queue; the lab's own script commands act at once.
    public void Start(string command)
    {
        switch (command)
        {
            case "script.restart": Restart(); return;
            case "script.safe": SetSafeMode(!SafeMode); return;
            case "script.abort": AbortRuns(); return;
            case "script.faults": Editor.ToggleFaults(); return;
            case "script.unused": int n = Editor.SelectUnused(); frame.Say(n == 0 ? "Every definition is used" : $"{n} unused selected"); return;
            case "script.reference": Editor.InsertReference(SixCsService.Reference("cell", "117")); return;
            case "script.bar": Bar.Ask($"{inspect.Inspector.SelectionCount} {inspect.Inspector.SelectionKind}"); return;
        }
        if (Find(command) is not { } entry) { Fail(command + ": its program no longer exports it"); return; }
        (int first, int last) = SixCsService.Region(programs.First(p => p.Id == entry.Program).Source, entry.Line);
        RunItem item = Queue.Enqueue(entry, first, last); failed.Remove(command);
        tasks[item.Id] = feedback.Start($"{entry.Program}.{entry.Name}", "script", entry.Kind != ScriptKind.View).Id;
    }

    // Runs what waits to its end at once: the checks use it instead of waiting.
    public void Drain() { for (int i = 0; i < 1000 && Queue.Active.Any(); i++) Queue.Advance(RunQueue.Cost); }

    public void AbortRuns()
    {
        foreach (RunItem item in Queue.Active.ToArray()) if (tasks.Remove(item.Id, out int task)) feedback.Cancel(task);
        int count = Queue.Abort();
        if (count > 0) Say(ConsoleKind.Echo, $"aborted {count} run(s)");
    }

    public void RunEntry(string command)
    {
        if (Find(command) is not { } entry) { Fail(command + ": its program no longer exports it"); return; }
        ran.Add(command);
        if (PackageOf(entry.Program) is { } package && PackageStore.PreviewRefusal(package, entry.Kind, Solving) is { } refusal)
        {
            failed.Add(command); demo.Refuse($"{entry.Program}.{entry.Name}(ring {tuner["ring"]})", package.File, refusal);
            Say(ConsoleKind.Lock, refusal); List.Refresh(); return;
        }
        switch (command)
        {
            case "script.sweep.ring":
                demo.Stage(1); Say(ConsoleKind.Echo, "sweep.ring: preview of 12 moves staged", "Preview staged · 12 moves"); break;
            case "script.sweep.reach":
                Result(entry, "record { cells: 30, orbits: 5, stickers: 12990 }"); break;
            case "script.census.orbits":
                frame.Model.ShowDock(DockSide.Right, SlotId(Slots.Mount(CensusPanel)));
                Result(entry, "record { cells: 600, faces: 1200, edges: 720, vertices: 120 }", "Census panel mounted"); break;
            case "script.census.fibres":
                Canvas.Show(Fibres(), "census.fibres", AuthorOf("census"));
                if (!frame.Model.IsOpen("canvas")) frame.Model.Drop("canvas", FrameModel.Pair, DropZone.Bottom); else frame.Model.Activate("canvas");
                Result(entry, "12", "12 fibres drawn"); break;
            case "script.layout.side_by_side":
                demo.LayoutRun("Script: side by side", () => frame.Model.SetPairArrangement("side", .5)); frame.Say("Side by side"); break;
            case "script.layout.stacked":
                demo.LayoutRun("Script: stacked", () => frame.Model.SetPairArrangement("stacked", .5)); frame.Say("Stacked"); break;
            case "script.layout.studio":
                // Layout, density and a key in one run: one step of the layout history.
                demo.LayoutRun("Script: studio", () =>
                {
                    quiet = true;
                    try { frame.Model.SetPairArrangement("stacked", .6); run("density.preset.set", null); Bind("F10", "script.layout.side_by_side"); }
                    finally { quiet = false; }
                });
                frame.Say("Studio layout · F10 returns"); break;
            case "script.mounts.neighbours":
                Result(entry, "set of 12 cells"); break;
            case "script.mounts.facts":
                Result(entry, $"record {{ cells: {inspect.Inspector.SelectionCount}, rings: 1 }}", "Inspector section added"); ApplyMounts();
                frame.Model.ShowDock(DockSide.Right, "inspector"); break;
            case "script.mounts.tuner":
                if (Mounts.IsOn("mounts", MountKind.Panel)) frame.Model.ShowDock(DockSide.Right, SlotId(Slots.Mount(TunerPanel)));
                Result(entry, tuner["ring"], "Tuner panel mounted"); break;
            case "script.mounts.ring_view":
                ApplyMounts(); frame.Say("Script view opened"); break;
            case "script.mounts.annotate_ring":
                Result(entry, "30", "Ring marked in both views"); ApplyMounts(); break;
            default:
                // An entry point without a sample of its own: a read gives a plain result, a preview the next sample.
                if (entry.Kind == ScriptKind.Read) Result(entry, "ok");
                else if (entry.Kind == ScriptKind.Preview) { demo.Stage(); Say(ConsoleKind.Echo, $"{entry.Program}.{entry.Name}: preview staged", "Preview staged"); }
                else frame.Say(entry.Name + ": nothing to change");
                break;
        }
        List.Refresh();
    }

    // The mounts follow their switches: a part shows when its entry point has run and its mount is on.
    private void ApplyMounts()
    {
        if (!GodotObject.IsInstanceValid(frame)) return;
        bool Shown(string entry, MountKind kind) => !SafeMode && ran.Contains("script.mounts." + entry) && Find("script.mounts." + entry) != null && Mounts.IsOn("mounts", kind);
        // Inspector section.
        if (facts != null && GodotObject.IsInstanceValid(facts)) { ChromeWidgets.Detach(facts); facts.QueueFree(); }
        facts = null;
        if (Shown("facts", MountKind.Inspector))
        {
            facts = new ScriptPanelHost(FactsPanel, Spec(FactsPanel), () => frame.Tokens, false, AuthorOf(Spec(FactsPanel)?.Program)); inspect.InspectorColumn.AddChild(facts); Targets();
        }
        // Docked panel.
        if (!Mounts.IsOn("mounts", MountKind.Panel) && Slots.SlotOf(TunerPanel) >= 0) Slots.Unmount(TunerPanel);
        // Free view.
        bool view = Shown("ring_view", MountKind.View);
        if (view)
        {
            View.Show(RingCensus(), "mounts.ring_view", AuthorOf("mounts"));
            if (!frame.Model.IsOpen(RingView)) frame.Model.Drop(RingView, FrameModel.Pair, DropZone.Bottom); else frame.Model.Activate(RingView);
        }
        else if (frame.Model.IsOpen(RingView)) frame.Model.Close(RingView);
        frame.Invalidate();
    }

    public bool Annotated => !SafeMode && ran.Contains("script.mounts.annotate_ring") && Find("script.mounts.annotate_ring") != null && Mounts.IsOn("mounts", MountKind.Overlay);

    // The marks the frame of each main view shows for the scripts: the annotation and the element of a problem row.
    public IEnumerable<(string Glyph, string Text, string Colour, string Tip)> ViewMarks()
    {
        if (Annotated) yield return ("script.mark", "ring " + tuner["ring"], tuner["tint"], "Marked by mounts.6cs");
        if (Marked != null) yield return ("severity.warning", Marked, "severity.warning", "The element of the chosen problem");
    }

    // The overlay annotation: a display-only drawing over both views with a legend that leads to its line.
    private void ShowOverlay()
    {
        foreach (ViewFrame view in new[] { frame.Global, frame.Local })
        {
            if (view.Slot.FindChild("ScriptOverlay", false, false) is { } old) { view.Slot.RemoveChild(old); old.QueueFree(); }
            if (!Annotated) continue;
            var overlay = new Control { Name = "ScriptOverlay", MouseFilter = Control.MouseFilterEnum.Ignore }; view.Slot.AddChild(overlay); overlay.SetAnchorsAndOffsetsPreset(Control.LayoutPreset.FullRect);
            var surface = new CanvasSurface(() => frame.Tokens, true); overlay.AddChild(surface); surface.SetAnchorsAndOffsetsPreset(Control.LayoutPreset.FullRect);
            var marks = new List<CanvasMark> { new("circle", .5, .5, .31, 0, tuner["tint"]) };
            for (int i = 0; i < 30; i++) marks.Add(new CanvasMark("dot", .5 + .31 * Math.Cos(i * Math.Tau / 30), .5 + .31 * Math.Sin(i * Math.Tau / 30), .006, 0, tuner["tint"]));
            surface.Show(marks);
            var legend = new PanelContainer { Name = "ScriptLegend" }; ScriptFrames.Apply(legend, AuthorOf("mounts")); overlay.AddChild(legend);
            legend.Position = new Vector2(frame.Tokens.Px("space.2"), frame.Tokens.Px("control.height") + 3 * frame.Tokens.Px("space.2"));
            Button reveal = SessionWidgets.Tool("script.mark", "Drawn by mounts.6cs: show its line. Drag it into the editor for a reference.", frame.Tokens,
                () => { if (Find("script.mounts.annotate_ring") is { } entry) { Open(entry.Program, entry.Line); Editor.Reveal(entry); } }, "ring " + tuner["ring"]);
            ScriptDrag.Source(reveal, () => SixCsService.Reference("cell", "117")); legend.AddChild(reveal);
        }
    }

    // History rows and inspector facts can be dragged into the editor as typed references.
    private void Sources()
    {
        if (!GodotObject.IsInstanceValid(frame)) return;
        foreach (Control row in demo.History.List.GetChildren().OfType<Control>().Where(r => r.HasMeta("entry")))
        {
            string id = row.GetMeta("entry").AsString(); ScriptDrag.Source(row, () => SixCsService.Reference("history", id));
        }
        foreach (ValueField field in inspect.Panel.Fields.Where(f => GodotObject.IsInstanceValid(f) && f.Field.IsFact))
        {
            string id = field.Field.Id; ScriptDrag.Source(field.Value, () => SixCsService.Reference("fact", id));
        }
    }

    // A script panel's target is the selection, unless the panel is pinned.
    private void Targets()
    {
        if (!GodotObject.IsInstanceValid(frame)) return;
        string target = $"{inspect.Inspector.SelectionCount} {inspect.Inspector.SelectionKind}";
        foreach (ScriptPanelHost host in frame.FindChildren("ScriptPanel_*", "", true, false).OfType<ScriptPanelHost>()) host.SetTarget(target);
        if (facts != null && GodotObject.IsInstanceValid(facts)) facts.SetTarget(target);
    }

    // The radial menu of a view with the context action of the sample program, when its mount is on.
    public RadialMenu WithContext(RadialMenu menu) =>
        !SafeMode && Find("script.mounts.neighbours") != null && Mounts.IsOn("mounts", MountKind.Context) && menu.Entries.Count < 8
            ? new RadialMenu(menu.Id, menu.Label, menu.Entries.Append(new RadialEntry("script.mounts.neighbours", null, "Neighbours")).ToArray()) : menu;

    // A typed line of the console, the command bar or the palette's typed mode: an entry point or a command id. It never commits.
    public void RunLine(string text)
    {
        string line = text.Trim().TrimStart('>').Trim().TrimEnd(';').Trim();
        if (line.Length == 0) return;
        Lines.Add(line); feedback.Log(ConsoleRows.Write(ConsoleKind.Typed, line));
        if (CommitPaths.TypedRefusal(line) is { } never) { Say(ConsoleKind.Lock, never); return; }
        // Whatever the line runs knows it came from a typed line, so nothing it runs writes the move history:
        // in Direct a typed twist is staged, not committed (ADR-D01.58).
        quiet = true; demo.Surface = InputSurface.TypedLine;
        try
        {
            Match call = Regex.Match(line, @"^(?:preview\s+|run\s+)?([a-z_][a-z0-9_]*)\.([a-z_][a-z0-9_]*)\s*(?:\(.*\))?$");
            string[] parts = line.Split(' ', 2);
            if (call.Success && Find($"script.{call.Groups[1].Value}.{call.Groups[2].Value}") is { } entry) run(entry.Command, null);
            else if (registry.Commands.ContainsKey(parts[0])) run(parts[0], parts.Length > 1 ? parts[1].Trim() : null);
            else Fail($"unknown word `{parts[0]}`");
        }
        finally { quiet = false; demo.Surface = InputSurface.Key; }
    }

    // The next key pressed becomes the binding of the command; Esc leaves it.
    public void Capture(string command)
    {
        List.SetBinding(command); frame.Say("Press the key to bind · Esc leaves");
        frame.KeyCapture = key =>
        {
            List.SetBinding(null);
            if (key != "Escape") Bind(key, command);
        };
    }

    public bool Bind(string key, string command)
    {
        // A run key and the commit key never share a key in one context (ADR-D01.58).
        if (KeyRules.Runs(command) && Keys.Bindings.Any(b => b.Key == key && b.Command == KeyRules.Commit)) { Say(ConsoleKind.Lock, $"{key} is the commit key. A run never shares it."); return false; }
        Binding? old = KeyRules.Conflict(Keys, key, Keymap.Global, command);
        Overrides.Bind(key, Keymap.Global, command); Apply();
        frame.Say($"{key} runs {registry.Commands[command].Label}" + (old != null ? $" · it ran {LabelOf(old)}" : ""), old != null);
        return true;
    }

    public void SetPreset(string preset)
    {
        if (preset != DefaultPreset && !presets.ContainsKey(preset)) { Fail("No keymap preset is named " + preset); return; }
        Preset = preset; Apply(); frame.Say("Keymap preset: " + preset);
    }

    public void Defaults() { Overrides.Clear(); Preset = DefaultPreset; Apply(); frame.Say("Default keymap · own bindings dropped"); }

    public void Export()
    {
        Directory.CreateDirectory(Path.GetDirectoryName(exportPath)!); File.WriteAllText(exportPath, Overrides.ToJson());
        frame.Say($"Exported {Overrides.Bindings.Count} own bindings");
    }

    private Keymap Base() => Preset == DefaultPreset ? defaults : Keymap.Parse(presets[Preset], registry, defaults);

    private void Apply()
    {
        Keys = Overrides.Over(Base(), registry); keysChanged(Keys);
        presetButton.Text = Preset; safeButton.SetPressedNoSignal(SafeMode); solvingButton.SetPressedNoSignal(Solving);
        List.Packages = packages.Select(p => (p, Installed.Installed.Contains(p.Id))).ToArray(); List.Refresh();
    }

    public string Store() => Preset + "\n" + Overrides.ToJson();

    public void Restore(string stored)
    {
        int cut = stored.IndexOf('\n');
        if (cut < 0) return;
        string preset = stored.Substring(0, cut);
        if (preset != DefaultPreset && !presets.ContainsKey(preset)) return;
        try { Preset = preset; Overrides = KeymapOverrides.Parse(stored.Substring(cut + 1), registry, Base()); }
        catch (FormatException) { Preset = DefaultPreset; Overrides = new KeymapOverrides(); }
        Apply();
    }

    // What a restart keeps of the scripts: safe mode, the mount switches, the installed packages, the session kind.
    public string StoreState() => string.Join("|", SafeMode ? "safe" : "on", Mounts.Store(), Installed.Store(), Solving ? "solving" : "practice");

    public void RestoreState(string stored)
    {
        string[] parts = stored.Split('|');
        if (parts.Length != 4) return;
        SafeMode = parts[0] == "safe"; Mounts.Restore(parts[1]); Installed.Restore(parts[2]); Solving = parts[3] != "practice";
        Reload(); Apply();
    }

    public bool Mount(string panel)
    {
        frame.Model.ShowDock(DockSide.Right, SlotId(Slots.Mount(panel)));
        return true;
    }

    private double Ring => double.Parse(tuner["ring"], CultureInfo.InvariantCulture);

    private ScriptPanelSpec? Spec(string id)
    {
        if (id == CensusPanel && Find("script.census.orbits") != null)
            return new ScriptPanelSpec(id, "Element census", "census.6cs",
                new (string, string)[] { ("Cells", "600"), ("Sticker slots per cell", "433"), ("Sticker slots", "259,800"), ("Legal generators", "1,200") },
                new (string, double)[] { ("vertices", 120), ("cells", 600), ("edges", 720), ("faces", 1200) });
        if (id == TunerPanel && Find("script.mounts.tuner") != null)
        {
            int ring = (int)Ring;
            var heat = Enumerable.Range(0, 30).Select(i => ((i + 1).ToString(), 3.0 + (i * 7 + ring * 3) % 5)).ToArray();
            return new ScriptPanelSpec(id, "Ring tuner", "mounts.6cs", Array.Empty<(string, string)>(), Array.Empty<(string, double)>())
            {
                Widgets = new PanelWidget[]
                {
                    new NumberWidget("ring", "Ring", 0, 19, 1, ring),
                    new ChoiceWidget("mode", "Sweep", new[] { "one axis", "two axes", "all axes" }, int.Parse(tuner["mode"], CultureInfo.InvariantCulture)),
                    new SwitchWidget("mirror", "Mirror", tuner["mirror"] == "on"),
                    new ReferenceWidget("anchor", "Anchor", "cell", tuner["anchor"]),
                    new TokenWidget("tint", "Mark", Tints, tuner["tint"]),
                    new TableWidget("reach", "", new[] { "Reach", "count" }, new IReadOnlyList<string>[] { new[] { "cells", "30" }, new[] { "orbits", "5" }, new[] { "sticker slots", "12,990" } }),
                    new TreeWidget("orbits", "", new (int, string, string)[] { (0, "ring " + ring, "30"), (1, "sample orbit 1", "6"), (1, "sample orbit 2", "12"), (1, "sample orbit 3", "12") }),
                    new ChartWidget("by-orbit", "Cells by orbit", "bars", new (string, double)[] { ("1", 6), ("2", 12), ("3", 12), ("4", 0), ("5", 0) }, tuner["tint"]),
                    new ChartWidget("by-depth", "Reach by depth", "line", Enumerable.Range(1, 6).Select(d => (d.ToString(), (double)Math.Min(600, 30 * d * d - ring))).ToArray(), tuner["tint"]),
                    new ChartWidget("around", "Stickers moved around the ring", "heat", heat, tuner["tint"]),
                    new CanvasWidget("diagram", "", RingCensus()),
                },
            };
        }
        if (id == FactsPanel && Find("script.mounts.facts") != null)
            return new ScriptPanelSpec(id, "Selection rings", "mounts.6cs", new (string, string)[] { ("Cells", inspect.Inspector.SelectionCount.ToString()), ("Rings", "1"), ("Orbits", "5") }, Array.Empty<(string, double)>())
            {
                Widgets = new PanelWidget[] { new ChartWidget("around", "", "heat", Enumerable.Range(0, 30).Select(i => ((i + 1).ToString(), 1.0 + (i * 5) % 7)).ToArray()) },
            };
        return null;
    }

    private void Edit(string panel, string widget, string value)
    {
        if (panel != TunerPanel || !tuner.ContainsKey(widget)) return;
        tuner[widget] = widget == "anchor" ? "cell " + (117 + 13 * (int.Parse(tuner["anchor"].Substring(5), CultureInfo.InvariantCulture) - 104) % 600 % 483 + 0) : value;
        ShowSlots(); if (ran.Contains("script.mounts.ring_view")) ApplyMounts(); else frame.Invalidate();
    }

    private void ShowSlots()
    {
        if (!GodotObject.IsInstanceValid(frame)) return;
        for (int i = 0; i < SlotTabs.Count; i++)
        {
            string? id = Slots.Shown(i);
            if (id == null || SafeMode) { frame.SetContent(SlotId(i), ChromeWidgets.Empty(SafeMode ? "scripts.safe" : i == 0 ? "panel.slot1" : "panel.slot2", SafeMode ? "Safe mode" : "Empty panel slot")); continue; }
            var column = new VBoxContainer(); column.AddThemeConstantOverride("separation", 0);
            if (Slots.Tabs(i).Count > 1)
            {
                // Two slots, then tabs inside them.
                var tabs = new HBoxContainer { Name = "SlotTabs" }; column.AddChild(tabs);
                foreach (string panel in Slots.Tabs(i))
                {
                    string name = panel; Button tab = ChromeWidgets.Tool(Spec(name)?.Title ?? name, name, true, "TabButton"); tab.SetPressedNoSignal(name == id); tab.ClipText = true; tab.SizeFlagsHorizontal = Control.SizeFlags.ExpandFill;
                    tab.Pressed += () => Slots.Mount(name); tabs.AddChild(tab);
                }
            }
            var host = new ScriptPanelHost(id, Spec(id), () => frame.Tokens, true, AuthorOf(Spec(id)?.Program)) { SizeFlagsVertical = Control.SizeFlags.ExpandFill }; host.Unmount += panel => run("panel.unmount", panel);
            host.Edited += (widget, value) => Edit(id, widget, value);
            var scroll = new ScrollContainer { SizeFlagsVertical = Control.SizeFlags.ExpandFill, HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled }; scroll.AddChild(host); host.SizeFlagsHorizontal = Control.SizeFlags.ExpandFill;
            column.AddChild(scroll); frame.SetContent(SlotId(i), column);
        }
        Targets();
    }

    // The census of one ring: its thirty cells in a ring, each with a bar and every fifth with its count.
    private IReadOnlyList<CanvasMark> RingCensus()
    {
        int ring = (int)Ring; string tint = tuner["tint"];
        var marks = new List<CanvasMark> { new("circle", .5, .5, .3, 0, "border.strong"), new("text", .41, .5, 0, 0, "text", $"ring {ring}"), new("text", .41, .56, 0, 0, "text.dim", "30 cells") };
        for (int i = 0; i < 30; i++)
        {
            double a = i * Math.Tau / 30 - Math.PI / 2; int count = 3 + (i / 5 + i * 2 + ring) % 5; double far = .3 + .022 * count;
            marks.Add(new CanvasMark("line", .5 + .3 * Math.Cos(a), .5 + .3 * Math.Sin(a), .5 + far * Math.Cos(a), .5 + far * Math.Sin(a), tint));
            marks.Add(new CanvasMark("dot", .5 + .3 * Math.Cos(a), .5 + .3 * Math.Sin(a), .011, 0, i % 5 == 0 ? "accent.strong" : tint));
            if (i % 5 == 0) marks.Add(new CanvasMark("text", .5 + .47 * Math.Cos(a) - .012, .5 + .47 * Math.Sin(a) + .012, 0, 0, "text", count.ToString()));
        }
        return marks;
    }

    // The twelve Hopf fibres of the 120 vertices sit over the corners of an icosahedron: seen along a fivefold
    // axis, two pentagons and the two poles at the centre.
    private static IReadOnlyList<CanvasMark> Fibres()
    {
        var marks = new List<CanvasMark> { new("circle", .5, .5, .42, 0, "border.strong"), new("dot", .5, .5, .022, 0, "accent.strong"), new("text", .53, .49, 0, 0, "text.dim", "2 poles") };
        for (int ring = 0; ring < 2; ring++)
        {
            double radius = ring == 0 ? .2 : .36, turn = ring * Math.PI / 5;
            for (int i = 0; i < 5; i++)
            {
                double a = turn + i * Math.Tau / 5 - Math.PI / 2, b = turn + (i + 1) * Math.Tau / 5 - Math.PI / 2;
                marks.Add(new CanvasMark("line", .5 + radius * Math.Cos(a), .5 + radius * Math.Sin(a), .5 + radius * Math.Cos(b), .5 + radius * Math.Sin(b), ring == 0 ? "state.preview" : "script.frame"));
                marks.Add(new CanvasMark("dot", .5 + radius * Math.Cos(a), .5 + radius * Math.Sin(a), .018, 0, "accent.strong"));
            }
        }
        marks.Add(new CanvasMark("text", .08, .97, 0, 0, "text", "12 fibres × 10 vertices = 120"));
        return marks;
    }
}
