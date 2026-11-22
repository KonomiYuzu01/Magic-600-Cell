using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using Godot;
using Magic600.Chrome;

namespace LookLab.App.Live;

// The lab's script interface (ADR-D01 section 6 and ADR-D01.57 to ADR-D01.59): the editor pane, the Scripts dock,
// the console line, the two script panel slots, the canvas and the solver's own key bindings. Look Lab has no
// interpreter, so the entry points of the sample programs are wired by hand here and say so; every other part
// (entry points as commands, key binding, keymap presets and export, slots, diagnostics) is the chrome library.
internal sealed class LabScripts
{
    public const string Stand = "Look Lab has no interpreter";
    public const string DefaultPreset = "default", CensusPanel = "census.orbits";
    public static string SlotId(int slot) => "slot." + (slot + 1);
    public SixCsEditor Editor { get; }
    public ScriptList List { get; }
    public ScriptCanvas Canvas { get; }
    public ScriptSlots Slots { get; } = new();
    public KeymapOverrides Overrides { get; private set; } = new();
    public LineHistory Lines { get; } = new();
    public string Preset { get; private set; } = DefaultPreset;
    public Keymap Keys { get; private set; }
    public IReadOnlyList<ScriptProgram> Programs => programs;
    public string? LastResult { get; private set; }
    public IReadOnlyList<string> Presets => presets.Keys.Prepend(DefaultPreset).ToArray();
    private readonly List<ScriptProgram> programs = new();
    private readonly SortedDictionary<string, string> presets = new(StringComparer.Ordinal);
    private readonly ChromeFrame frame;
    private readonly LabDemo demo;
    private readonly FeedbackModel feedback;
    private readonly CommandRegistry registry;
    private readonly Keymap defaults;
    private readonly Action<string, string?> run;
    private readonly Action<Keymap> keysChanged;
    private readonly string exportPath;
    private readonly Button presetButton;
    private bool quiet;

    public LabScripts(ChromeFrame frame, LabDemo demo, LabInspect inspect, CommandRegistry registry, Keymap defaults, string root, Action<string, string?> run, Action<Keymap> keysChanged)
    {
        this.frame = frame; this.demo = demo; feedback = inspect.Feedback; this.registry = registry; this.defaults = defaults; this.run = run; this.keysChanged = keysChanged; Keys = defaults;
        exportPath = Path.Combine(root, "work/loop-memory/looklab/keymap-export.json");
        foreach (string path in Directory.GetFiles(Path.Combine(root, "tools/looklab/data/samples"), "*.6cs").OrderBy(p => p, StringComparer.Ordinal))
            programs.Add(new ScriptProgram(Path.GetFileNameWithoutExtension(path), Path.GetFileName(path), File.ReadAllText(path).Replace("\r\n", "\n")));
        foreach (string path in Directory.GetFiles(Path.Combine(root, "tools/looklab/data/design/keymaps"), "*.json").OrderBy(p => p, StringComparer.Ordinal))
        {
            string text = File.ReadAllText(path); Keymap.Parse(text, registry, defaults); presets[Path.GetFileNameWithoutExtension(path)] = text;
        }
        Register();
        Editor = new SixCsEditor(() => frame.Tokens); List = new ScriptList(() => frame.Tokens) { KeysOf = e => Keys.KeysFor(e.Command, null) }; Canvas = new ScriptCanvas(() => frame.Tokens);
        Editor.SetPrograms(programs); List.SetPrograms(programs);
        Editor.Tools.AddChild(ChromeWidgets.Text("sample programs · no interpreter", "MicroLabel"));
        Editor.Run += (program, entry) =>
        {
            if (entry == null) Fail($"{program.File} has no entry point above the caret. An exported fn or command is one.");
            else run(entry.Command, null);
        };
        Editor.CommitKey += () => run("operation.preview.commit", null);
        Editor.Edited += program =>
        {
            programs[programs.FindIndex(p => p.Id == program.Id)] = program;
            Register(); List.SetPrograms(programs);
            feedback.ClearSource(program.File);
            foreach (ScriptDiagnostic d in SixCs.Diagnose(program.Source)) feedback.Report(d.Severity, d.Text, program.File, d.Line);
            ShowSlots();
        };
        List.Opened += (program, line) => Open(program.Id, line);
        List.RunEntry += entry => run(entry.Command, null);
        List.BindEntry += entry => Capture(entry.Command);
        presetButton = ChromeWidgets.Tool("", "Keymap preset: click for the next one. Your own bindings stay on top of it."); presetButton.Name = "KeymapPreset";
        presetButton.Pressed += () => { IReadOnlyList<string> all = Presets; SetPreset(all[(all.ToList().IndexOf(Preset) + 1) % all.Count]); };
        Button export = ChromeWidgets.Tool("export", "Export your own bindings as a versioned keymap file"); export.Name = "KeymapExport"; export.Pressed += () => run("file.keymap.export", null);
        Button back = ChromeWidgets.Tool("defaults", "Drop your own bindings and return to the default keymap"); back.Name = "KeymapDefaults"; back.Pressed += () => run("keymap.defaults", null);
        List.Tools.AddChild(presetButton); List.Tools.AddChild(export); List.Tools.AddChild(back);
        inspect.Console.Entered += RunLine;
        inspect.Problems.Opened += p => { if (programs.FirstOrDefault(x => x.File == p.Source) is { } program) Open(program.Id, p.Line); };
        // A read result computed before a commit dims and says so (ADR-D01.58).
        demo.Moves.Changed += () => { if (LastResult != null && GodotObject.IsInstanceValid(Editor)) Editor.ShowResult(LastResult + "   · computed before the last commit; run it again", true); };
        Slots.Changed += ShowSlots;
        frame.SetContent("editor", Editor); frame.SetContent("scripts", List); frame.SetContent("canvas", Canvas);
        frame.Rebuilt += Refresh; frame.Recoloured += Refresh;
        ShowSlots(); Apply();
    }

    private void Refresh()
    {
        Editor.Refresh(); List.Refresh(); Canvas.Repaint();
        foreach (ScriptPanelHost host in frame.FindChildren("ScriptPanel_*", "", true, false).OfType<ScriptPanelHost>()) host.Refresh();
    }

    // Every exported fn or command is a command row of its own, so the palette lists it and a key can be bound to it.
    private void Register()
    {
        foreach (ScriptProgram p in programs)
            foreach (ScriptEntry e in p.Entries)
                if (!registry.Commands.ContainsKey(e.Command))
                    registry.Add(new ChromeCommand(e.Command, $"{p.File}: {e.Name}", e.Kind == ScriptKind.View ? "view" : "query", new[] { "any" },
                        $"Entry point of {p.File}; a run ends in {(e.Kind == ScriptKind.Preview ? "a preview" : e.Kind == ScriptKind.View ? "a view or layout change" : "a read result")}, never a commit. {Stand}: the sample entry points are wired by hand."));
    }

    public ScriptEntry? Find(string command) => programs.SelectMany(p => p.Entries).FirstOrDefault(e => e.Command == command);
    public string? Unavailable(string command) => Find(command) == null ? "its program no longer exports it" : null;

    public void Open(string program, int line = 0)
    {
        if (!frame.Model.IsOpen("editor")) frame.Model.Drop("editor", FrameModel.Pair, DropZone.Right); else frame.Model.Activate("editor");
        Editor.Show(program, line);
    }

    private void Fail(string text) { feedback.Log(text); frame.Say(text, true); }

    private void Result(ScriptEntry entry, string text, bool keep = true)
    {
        if (keep) LastResult = text;
        if (Editor.Open?.Id == entry.Program) Editor.ShowResult(text);
        feedback.Log(text); frame.Say(text);
    }

    // The .6cs line a command-layer action is echoed as in the console log (ADR-D01.57).
    public void Echo(string command, string? argument)
    {
        if (quiet || command is "command.find" or "help.open" or "transient.close" || command.StartsWith("radial.", StringComparison.Ordinal)) return;
        string word = Find(command) is { } entry ? entry.Program + "." + entry.Name : command;
        feedback.Log(argument == null ? word + "();" : $"{word}(\"{argument}\");");
    }

    public void RunEntry(string command)
    {
        if (Find(command) is not { } entry) { Fail(command + ": its program no longer exports it"); return; }
        switch (command)
        {
            case "script.sweep.ring":
                demo.Stage(1); Result(entry, "sweep.ring(4): a preview of 12 moves waits in the preview view. Nothing is committed.", false); break;
            case "script.sweep.reach":
                Result(entry, "sweep.reach(4) = record { cells: 30, orbits: 5, stickers: 12990 }   (sample value)"); break;
            case "script.census.orbits":
                int slot = Slots.Mount(CensusPanel);
                if (slot < 0) { Fail("census.orbits: both script panel slots are taken. Unmount one first."); break; }
                frame.Model.ShowDock(DockSide.Right, SlotId(slot));
                Result(entry, $"census.orbits() = record {{ cells: 600, faces: 1200, edges: 720, vertices: 120 }}   · its panel is in slot {slot + 1}"); break;
            case "script.census.fibres":
                Canvas.Show(Fibres(), "census.fibres");
                if (!frame.Model.IsOpen("canvas")) frame.Model.Drop("canvas", FrameModel.Pair, DropZone.Bottom); else frame.Model.Activate("canvas");
                Result(entry, "census.fibres(): drew 12 fibres of 10 vertices on the canvas", false); break;
            case "script.layout.side_by_side":
                frame.Model.SetPairArrangement("side", .5); Result(entry, "layout.side_by_side(): Global and Local sit side by side. A layout change, in the layout history.", false); break;
            case "script.layout.stacked":
                frame.Model.SetPairArrangement("stacked", .5); Result(entry, "layout.stacked(): Global sits over Local. A layout change, in the layout history.", false); break;
            default:
                Fail($"{entry.Program}.{entry.Name}: {Stand}, so only the sample entry points give a result. The entry point is registered, and a key can be bound to it."); break;
        }
    }

    // A typed line of the console or of the palette's typed mode: a sample entry point or a command id. It never commits.
    public void RunLine(string text)
    {
        string line = text.Trim().TrimStart('>').Trim().TrimEnd(';').Trim();
        if (line.Length == 0) return;
        Lines.Add(line); feedback.Log("> " + line);
        if (Regex.IsMatch(line, @"^(commit\b|operation\.preview\.commit\b)")) { Fail("A typed line never commits. Stage a preview, read it, then press the commit key."); return; }
        quiet = true;
        try
        {
            Match call = Regex.Match(line, @"^(?:preview\s+|run\s+)?([a-z_][a-z0-9_]*)\.([a-z_][a-z0-9_]*)\s*(?:\(.*\))?$");
            string[] parts = line.Split(' ', 2);
            if (call.Success && Find($"script.{call.Groups[1].Value}.{call.Groups[2].Value}") is { } entry) run(entry.Command, null);
            else if (registry.Commands.ContainsKey(parts[0])) run(parts[0], parts.Length > 1 ? parts[1].Trim() : null);
            else Fail($"unknown word `{parts[0]}`. {Stand}: a line names a sample entry point, such as sweep.ring(4), or a command id.");
        }
        finally { quiet = false; }
    }

    // The next key pressed becomes the binding of the command; Esc leaves it.
    public void Capture(string command)
    {
        List.SetBinding(command);
        frame.KeyCapture = key =>
        {
            List.SetBinding(null);
            if (key != "Escape") Bind(key, command);
        };
    }

    public bool Bind(string key, string command)
    {
        Binding? old = Keys.Resolve(key, new[] { Keymap.Global });
        // A run key and the commit key never share a key in one context (ADR-D01.58).
        if (old?.Command == "operation.preview.commit") { Fail($"{key} is the commit key. A run never shares a key with it."); return false; }
        Overrides.Bind(key, Keymap.Global, command); Apply();
        frame.Say($"{key} runs {registry.Commands[command].Label}" + (old != null ? $". It ran {old.Label ?? registry.Commands[old.Command].Label} before." : ""));
        return true;
    }

    public void SetPreset(string preset)
    {
        if (preset != DefaultPreset && !presets.ContainsKey(preset)) { Fail("No keymap preset is named " + preset); return; }
        Preset = preset; Apply(); frame.Say("Keymap preset: " + preset);
    }

    public void Defaults() { Overrides.Clear(); Preset = DefaultPreset; Apply(); frame.Say("The default keymap is in force; your own bindings were dropped."); }

    public void Export()
    {
        Directory.CreateDirectory(Path.GetDirectoryName(exportPath)!); File.WriteAllText(exportPath, Overrides.ToJson());
        frame.Say($"Exported {Overrides.Bindings.Count} of your own bindings to work/loop-memory/looklab/keymap-export.json");
    }

    private Keymap Base() => Preset == DefaultPreset ? defaults : Keymap.Parse(presets[Preset], registry, defaults);

    private void Apply()
    {
        Keys = Overrides.Over(Base(), registry); keysChanged(Keys);
        presetButton.Text = "keys: " + Preset; List.Refresh();
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

    public bool Mount(string panel)
    {
        int slot = Slots.Mount(panel);
        if (slot < 0) { Fail("Both script panel slots are taken. Unmount one first."); return false; }
        frame.Model.ShowDock(DockSide.Right, SlotId(slot));
        return true;
    }

    private ScriptPanelSpec? Spec(string id) => id == CensusPanel && Find("script.census.orbits") != null
        ? new ScriptPanelSpec(id, "Element census (sample panel)", "census.6cs",
            new (string, string)[] { ("Cells", "600"), ("Sticker slots per cell", "433"), ("Sticker slots", "259,800"), ("Legal generators", "1,200") },
            new (string, double)[] { ("vertices", 120), ("cells", 600), ("edges", 720), ("faces", 1200) })
        : null;

    private void ShowSlots()
    {
        if (!GodotObject.IsInstanceValid(frame)) return;
        for (int i = 0; i < ScriptSlots.Count; i++)
        {
            string? id = Slots.Mounted[i];
            if (id == null) { frame.SetContent(SlotId(i), ChromeWidgets.Empty("Script panel slot " + (i + 1), "A panel a script declares mounts here. Run census.orbits from the Scripts dock.")); continue; }
            var host = new ScriptPanelHost(id, Spec(id), () => frame.Tokens); host.Unmount += panel => run("panel.unmount", panel);
            frame.SetContent(SlotId(i), host);
        }
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
