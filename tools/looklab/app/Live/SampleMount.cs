using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using Magic600.Chrome;

namespace LookLab.App.Live;

// ADR-D01's frame on sample data, for a host without Look Lab's engine: the product shell's sample mode. The shell's
// build stages Look Lab's sample wiring (LabDemo, LabSolve, LabScripts, SampleInspect) with this file
// (renderer/shell/build_shell.py), so there is one copy. It mounts the relationship strip on the seam and the orbit
// rail over the pair, the draft line with its Goal and Net/Strict chip over the sample commit bar, the Library dock with
// its signatures, the inspector's two tiers, the Problems, Console and Tasks panels, the editor, the Scripts dock and
// the script panel slots, the key map drawn as a keyboard, and the pick and focus layer keys. Every fact in them is
// sample data, and the sample history is a dock panel of its own. The host keeps what belongs to its real session:
// the views, its History dock, its readouts and the session's own undo and redo (Kept). The strip's anchors sit where
// the core draws their positions (ADR-D01.62, CorePlaces): in Global as the core draws it with no camera or shape set,
// until the host describes its views (GlobalView, LocalView).
internal sealed class SampleMount
{
    public const string HistoryPanel = "history.sample";
    public const int SampleCells = 20;
    // The source a failure names in Problems in the product.
    public const string ErrorSource = "shell";
    // Commands the host's real session carries out: sample mode never takes them.
    public static readonly IReadOnlySet<string> Kept = new HashSet<string>(StringComparer.Ordinal) { "session.undo", "session.redo" };
    public LabDemo Demo { get; }
    public SampleInspect Inspect { get; }
    public LabSolve Solve { get; }
    public LabScripts Scripts => scripts!;
    public KeyMapPanel Keys { get; }
    // The model's places for the anchors (the build's assets, M600_ASSETS), or null when they could not be read.
    public CorePlaces? Places { get; }
    // How the core draws each view, as the host describes it; null for Global: the core's view with no camera and no
    // shape set (the product without --d02), and no anchor under --d02 until the host describes Global. A view the
    // host describes as null draws no anchor; Local draws none until the host describes it.
    public Func<CoreView?>? GlobalView { get; set; }
    public Func<CoreView?>? LocalView { get; set; }
    public IReadOnlyDictionary<string, FocusLayers> Layers => layers;
    // A pick layer, the inactive mode or the staged preview changed: the host draws its view marks again (Marks).
    public event Action? Changed;
    private readonly Dictionary<string, FocusLayers> layers = new(StringComparer.Ordinal) { ["global"] = new FocusLayers(), ["local"] = new FocusLayers() };
    private readonly ChromeFrame frame;
    private readonly CommandRegistry registry;
    private readonly LabScripts? scripts;
    private int panelState = -1;

    private static readonly (PanelCondition Condition, string Text)[] DemoStates =
    {
        (PanelCondition.Empty, "Nothing is here yet (demo state)"), (PanelCondition.Loading, "Reading the session (demo state)"),
        (PanelCondition.Error, "The file could not be read (demo state)"), (PanelCondition.Stale, "Older than the committed state (demo state)"), (PanelCondition.Busy, "A run is working here (demo state)"),
    };

    public SampleMount(ChromeFrame frame, CommandRegistry registry, Keymap keymap, string root, Func<string, double> seconds,
        Action<string, string?> run, Action<Keymap> keysChanged, CommandPalette palette, ShortcutSheet sheet)
    {
        this.frame = frame; this.registry = registry;
        Demo = new LabDemo(frame, keymap, run);
        frame.AddDockPanel(DockSide.Right, HistoryPanel, "History (sample)", "panel.history", Demo.History);
        frame.SetContent(LabDemo.PreviewPane, Demo.PreviewView);
        Inspect = new SampleInspect(frame, Demo, SampleCells);
        // Failures show: the shared error lamp lists them in Problems beside the sample rows (ChromeErrors).
        ChromeErrors.Mount(frame, Inspect.Feedback, ErrorSource);
        Places = LoadPlaces(System.Environment.GetEnvironmentVariable("M600_ASSETS") is { Length: > 0 } assets ? assets : System.IO.Path.Combine(root, "assets"));
        Solve = new LabSolve(frame, root, seconds, run, Demo, Places?.Projector(Global), Places is { } local ? LocalPlace(local) : null);
        frame.SetContent("library", Solve.LibraryView);
        frame.SetStrip(Solve.Bars(Demo.Bar));
        // The key map: shown while the help key is held, pinned by a short press.
        Keys = new KeyMapPanel(frame, registry); frame.Overlays.AddChild(Keys); Keys.List += sheet.Open;
        foreach ((string key, string label) in new[] { ("O", "Orbit jump: hold, then two digits"), ("F4", "Command bar"), ("F10", "Inactive cells"), ("F11", "Next demo state"),
            ("F12", "Panel states (demo)"), ("6", "Pick layer"), ("7", "Pick layer"), ("8", "Pick layer"), ("9", "Pick layer"), ("0", "Pick layer") }) Keys.Host[key] = label;
        scripts = new LabScripts(frame, Demo, Inspect, registry, keymap, root, run, k =>
        {
            Demo.Bar.CommitKeys = k.KeysFor("operation.preview.commit", null); keysChanged(k);
            if (scripts != null) { string set = scripts.Preset + (scripts.Overrides.Bindings.Count > 0 ? " with your bindings" : ""); if (Keys.KeySet != set) { Keys.KeySet = set; frame.Say("Key set: " + set); } }
        });
        palette.Bind += row => Scripts.Capture(row.Command);
        palette.TypedLine += Scripts.RunLine;
        frame.HostKeys["F4"] = () => run("script.bar", null);
        // The five panel states, on every dock panel at once: demo states, said so in their words.
        frame.HostKeys["F12"] = () => ShowPanelState(panelState + 1);
        frame.PanelStates.Asked += (id, action) => { frame.PanelStates.Set(id, PanelCondition.Ready); frame.Say($"{id}: {action} asked (demo state cleared)"); };
        // Pick-layer keys beside each view: 6 to 0 pick a layer in the view under the pointer, F10 cycles how the others show.
        DressViews(); frame.Rebuilt += DressViews;
        foreach (var layer in FocusLayers.Layers) { string id = layer.Id; frame.HostKeys[LayerKey(id)] = () => Aimed().SetPick(id); }
        frame.HostKeys["F10"] = () => Aimed().Cycle();
        foreach (FocusLayers changed in layers.Values) changed.Changed += () => { if (GodotObject.IsInstanceValid(frame)) Safe.Raise(Changed); };
        Demo.Preview.Changed += () => { if (GodotObject.IsInstanceValid(frame)) Safe.Raise(Changed); };
        frame.Contexts = () => Demo.Preview.Pending != null ? new[] { "preview" } : Array.Empty<string>();
    }

    private static CorePlaces? LoadPlaces(string folder)
    {
        try { return CorePlaces.Load(folder); }
        catch (Exception ex) when (ex is System.IO.IOException or UnauthorizedAccessException or System.IO.InvalidDataException or System.Text.Json.JsonException or InvalidOperationException or KeyNotFoundException)
        {
            Safe.Report("Anchors: the model's places could not be read from " + folder + ": " + ex.Message);
            return null;
        }
    }

    // Global: the host's description, else the core's own view at the opening's aspect (without --d02 the core draws
    // Global with no camera and no shape set).
    private CoreView? Global()
    {
        if (GlobalView != null) return GlobalView();
        if (System.Environment.GetEnvironmentVariable("M600_D02") == "1") return null;
        Vector2 size = frame.Global.Slot.Size;
        return size.X > 0 && size.Y > 0 ? CoreView.Default(size.X / size.Y) : null;
    }

    // Local's anchor layer covers Local's whole slot, and the core draws Local in the slot's opening (its top): a place
    // in the opening is moved to its place in the slot.
    private Func<int, int, (double X, double Y)?> LocalPlace(CorePlaces places)
    {
        Func<int, int, (double X, double Y)?> inOpening = places.Projector(() => LocalView?.Invoke());
        return (orbit, position) =>
        {
            if (LocalView == null || inOpening(orbit, position) is not { } at) return null;
            Rect2 slot = frame.Local.Slot.GetGlobalRect(), opening = frame.LocalOpening;
            if (slot.Size.X <= 0 || slot.Size.Y <= 0 || opening.Size.X <= 0 || opening.Size.Y <= 0) return null;
            return ((opening.Position.X - slot.Position.X + at.X * opening.Size.X) / slot.Size.X, (opening.Position.Y - slot.Position.Y + at.Y * opening.Size.Y) / slot.Size.Y);
        };
    }

    private static string LayerKey(string id) => id == "cycle" ? "F10" : ((FocusLayers.Layers.ToList().FindIndex(l => l.Id == id) + 6) % 10).ToString(System.Globalization.CultureInfo.InvariantCulture);

    private FocusLayers Aimed() => layers[frame.Local.Focused ? "local" : "global"];

    private void DressViews()
    {
        foreach ((ViewFrame view, string id) in new[] { (frame.Global, "global"), (frame.Local, "local") })
            if (view.Tools.GetNodeOrNull("LayerKeys") == null) { var keys = new LayerKeys(layers[id], () => frame.Tokens, LayerKey); view.Tools.AddChild(keys); view.Tools.MoveChild(keys, 0); }
    }

    private void ShowPanelState(int index)
    {
        panelState = index;
        if (index < 0 || index >= DemoStates.Length) { panelState = -1; frame.PanelStates.Clear(); frame.Say("Panels: ready"); return; }
        foreach (string id in frame.PanelStates.Known.ToArray()) frame.PanelStates.Set(id, DemoStates[index].Condition, DemoStates[index].Text);
        frame.Say($"Panels: {PanelStates.Word(DemoStates[index].Condition).ToLowerInvariant()} (demo state, F12 for the next)");
    }

    // Whether sample mode carries out the command: the sample session's, the solving surface's and the scripts'.
    public bool Handles(string command) => !Kept.Contains(command) &&
        (LabDemo.Handles(command) || LabSolve.Handles(command) || command.StartsWith("script.", StringComparison.Ordinal) ||
         command is "keymap.defaults" or "file.keymap.export" or "panel.mount" or "panel.unmount");

    // Why a command sample mode handles cannot run now, or null.
    public string? Unavailable(string command, string? argument) => command switch
    {
        "panel.mount" or "panel.unmount" => argument != null ? null : "it needs the id of a script panel",
        _ when command.StartsWith("script.", StringComparison.Ordinal) => Scripts.Unavailable(command),
        _ => null,
    };

    // Every command the host carries out is echoed in the sample console, as Look Lab does.
    public void Echo(string command, string? argument) => Scripts.Echo(command, argument);

    public RadialMenu WithContext(RadialMenu menu) => Scripts.WithContext(menu);

    // Esc in sample mode: the pinned key map closes, then the latest sample task is cancelled. True when it did that.
    public bool Close() { Keys.Shown.Pin(false); return Inspect.Feedback.CancelLatest(); }

    public void Execute(string command, string? argument)
    {
        switch (command)
        {
            case "keymap.defaults": Scripts.Defaults(); break;
            case "file.keymap.export": Scripts.Export(); break;
            case "panel.mount": Scripts.Mount(argument!); break;
            case "panel.unmount": if (!Scripts.Slots.Unmount(argument!)) frame.Say("No script panel is named " + argument, true); break;
            case var entry when entry.StartsWith("script.", StringComparison.Ordinal): Scripts.Start(entry); break;
            case "operation.preview" when argument != null && argument.StartsWith(LabSolve.MacroPreview, StringComparison.Ordinal): Solve.PreviewMacro(argument.Substring(LabSolve.MacroPreview.Length)); break;
            case var sample when LabDemo.Handles(sample): Demo.Execute(command, argument); break;
            case var solving when LabSolve.Handles(solving): Solve.Execute(command, argument, registry.Commands[command].Label); break;
        }
    }

    // The sample marks of a view: a staged preview, the scripts' marks, the picked layer and how the others show.
    public void Marks(ViewFrame view)
    {
        if (Demo.Preview.Pending != null) view.AddMark("mark.pending", "preview pending (sample)", "state.pending", "A sample preview waits in the preview view; this view still shows the committed state.");
        foreach (var mark in Scripts.ViewMarks()) view.AddMark(mark.Glyph, mark.Text, mark.Colour, mark.Tip);
        FocusLayers own = layers[view == frame.Global ? "global" : "local"]; var picked = FocusLayers.Layers.First(l => l.Id == own.Pick);
        view.AddMark(picked.Icon, picked.Name.ToLowerInvariant(), "text.dim", $"A click picks {picked.Name.ToLowerInvariant()} in this view.");
        if (own.Inactive != InactiveMode.Normal)
            view.AddMark("mark.protected", "shows through", "state.protected", $"The other layers are {own.Inactive.ToString().ToLowerInvariant()} and cannot be picked. A protection conflict still shows through every layer (sample).");
    }
}
