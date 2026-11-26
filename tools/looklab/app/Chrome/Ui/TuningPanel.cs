using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// ADR-D02.47, first half (B30): the dock panel `panel.tuning`. Family; Look; the five seed knobs, each over its live
// ramp strip; Compare with the two tuning slots, Swap and a held compare (the Hold button: held with the pointer, or
// with Space or Enter while it has the focus). It edits the Appearance through the Appearance panel, which stays
// beside it, so the frame recolours in place and the host's Recoloured follows (the shell sends the core the new
// palette). Its rows: tuning.open, tuning.slot.set, tuning.slot.swap, tuning.compare.hold and the table's theme.tune.
// It reuses the Appearance panel's knob slider and value, and the token specimen's swatches.
public sealed partial class TuningPanel : ScrollContainer
{
    public const string NodeName = "TuningPanel";
    public static readonly IReadOnlyList<string> Rows = Array.AsReadOnly(new[] { "tuning.open", "tuning.slot.set", "tuning.slot.swap", "tuning.compare.hold", "theme.tune",
        "tuning.preset.apply", "tuning.preset.save", "tuning.simulate.hold", "token.override.set", "token.override.reset" });
    public Tuning Model { get; }
    public Label Compare { get; } = new() { Name = "TuningCompareLine", ThemeTypeVariation = "DimLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    public Label Status { get; } = new() { Name = "TuningStatus", ThemeTypeVariation = "MicroLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    private readonly AppearancePanel appearance;
    private readonly ChromeFrame frame;
    private readonly VBoxContainer column = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill };
    private readonly Dictionary<string, (HSlider Slider, Label Value, HBoxContainer Strip)> knobs = new(StringComparer.Ordinal);
    private bool syncing;
    private string built = "";

    // B31: the preset line, Save's issues in place, the simulation line, and the host's hook for the core's half.
    public Label PresetInfo { get; } = new() { Name = "TuningPresetLine", ThemeTypeVariation = "DimLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    public Label Issues { get; } = new() { Name = "TuningIssues", ThemeTypeVariation = "MicroLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    public Label SimulationInfo { get; } = new() { Name = "TuningSimulationLine", ThemeTypeVariation = "MicroLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    public event Action? Simulated;
    // Whether the host sends the matrix to the core's view style too (B14); off until the look-cost row G8 passes.
    public bool CoreSimulation { get; set; }
    public SimulationFilter Filter { get; } = new();
    private readonly VBoxContainer roles = new() { Name = "TuningRoles" };
    private string search = "";

    public TuningPanel(AppearancePanel appearance, ChromeFrame frame, IReadOnlyList<TuningPreset>? presets = null)
    {
        this.appearance = appearance; this.frame = frame; Model = new Tuning(appearance.Look, presets);
        frame.Overlays.AddChild(Filter);
        Name = NodeName; HorizontalScrollMode = ScrollMode.Disabled;
        var margin = new MarginContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill }; AddChild(margin);
        foreach (string side in new[] { "margin_left", "margin_right", "margin_top", "margin_bottom" }) margin.AddThemeConstantOverride(side, 8);
        margin.AddChild(column);
        // Knob ranges are the family's: the panel is built again only for another family, never under a held button.
        appearance.Changed += (_, _) => { if (Model.Look.Theme != built && !Model.Holding) Build(); else Sync(); };
        Build();
    }

    public bool Handles(string command) => Rows.Contains(command);

    public string? Unavailable(string command, string? argument) => command switch
    {
        "tuning.slot.set" when argument is not ("A" or "B") => "it needs the slot A or B",
        "tuning.compare.hold" when argument is not (null or "on" or "off") => "it takes on or off, or nothing to switch",
        "theme.tune" when string.IsNullOrWhiteSpace(argument) => "it needs knob=value, such as tone=0.03",
        "tuning.preset.apply" when string.IsNullOrWhiteSpace(argument) => "it needs a preset: " + string.Join(", ", Model.Presets.Select(p => p.Id)),
        "tuning.simulate.hold" when argument != null && argument != "off" && !ColourMetric.Simulations.Contains(argument) => "it takes " + string.Join(", ", ColourMetric.Simulations) + ", off, or nothing to switch",
        "token.override.set" when string.IsNullOrWhiteSpace(argument) => "it needs role=L C h, such as tint.jumble.height.4=0.5 0.02 250",
        "token.override.reset" when string.IsNullOrWhiteSpace(argument) => "it needs the role to reset",
        _ => null,
    };

    public void Execute(string command, string? argument)
    {
        switch (command)
        {
            // An explicit open shows what it opens: in a folded right dock the panel is revealed over the fold.
            case "tuning.open": if (!Tuning.Open(frame.Model, frame.Folds(DockSide.Right))) Say(frame.Model.Refusal ?? "the right dock has no tuning panel"); break;
            case "tuning.preset.apply":
                bool across = Model.Presets.FirstOrDefault(p => p.Id == argument || string.Equals(p.Name, argument, StringComparison.OrdinalIgnoreCase)) is { } preset && preset.Value.Family != Model.Look.Theme;
                Edit(() => Model.ApplyPreset(argument), across, true); break;
            case "tuning.preset.save": Save(argument); break;
            case "tuning.simulate.hold":
                if (argument == "off" || argument == null && Model.Simulating != null) Simulate(null); else Simulate(argument ?? Model.Simulation);
                break;
            // ADR-D01.50's table rows: an override of a role of the family in force, and its reset.
            case "token.override.set": Edit(() => Model.Override(argument), false, false); break;
            case "token.override.reset": Edit(() => Model.Override(argument?.Trim() + "=reset"), false, false); break;
            case "tuning.slot.set": Edit(() => Model.SetSlot(argument), false, false); break;
            case "tuning.slot.swap": Edit(Model.Swap, Crosses, true); break;
            case "tuning.compare.hold": Edit(argument == "on" || argument == null && !Model.Holding ? Model.Hold : Model.Release, Crosses, true); break;
            case "theme.tune": Edit(() => Model.Tune(argument), false, false); break;
        }
    }

    private void Say(string? text) => Status.Text = text ?? "";

    // A swap or a hold rebuilds the frame only when the other slot is another family (sizes and character change).
    private bool Crosses => (Model.Holding ? Model.Slot(Model.Shown).Family : Model.Other.Family) != Model.Look.Theme;

    // Every change goes through the Appearance panel's Edit, so the frame and the panel beside it follow; a refusal
    // leaves the appearance as it was and shows its reason here.
    private bool Edit(Func<string?> change, bool rebuild, bool scene)
    {
        string? refused = null;
        bool done = appearance.Edit(_ => { refused = change(); if (refused != null) throw new FormatException(refused); }, rebuild, scene);
        Say(done ? null : refused);
        if (done && !rebuild) appearance.Show(appearance.Look);
        Sync();
        return done;
    }

    private Control Row(string label, Control control, Control? after = null)
    {
        var row = new HBoxContainer();
        Label name = ChromeWidgets.Text(label, "DimLabel"); name.CustomMinimumSize = new Vector2(88, 0); row.AddChild(name);
        control.SizeFlagsHorizontal = SizeFlags.ExpandFill; row.AddChild(control);
        if (after != null) row.AddChild(after);
        column.AddChild(row);
        return control;
    }

    private void Choice(string label, string name, string[] ids, string[] titles, string current, Func<string, string?> chosen)
    {
        var choice = new OptionButton { Name = name, FitToLongestItem = false, ClipText = true };
        foreach (string title in titles) choice.AddItem(title);
        choice.Select(Array.IndexOf(ids, current));
        choice.ItemSelected += index => Edit(() => chosen(ids[(int)index]), true, true);
        Row(label, choice);
    }

    private Button Tool(string name, string text, string tooltip, Action pressed)
    {
        Button button = ChromeWidgets.Tool(text, tooltip); button.Name = name;
        button.Pressed += pressed;
        return button;
    }

    private void Build()
    {
        foreach (Node child in column.GetChildren())
        {
            column.RemoveChild(child);
            if (child != Compare && child != Status && child != PresetInfo && child != Issues && child != SimulationInfo && child != roles) child.QueueFree();
        }
        knobs.Clear();
        Appearance look = Model.Look; built = look.Theme;
        column.AddChild(ChromeWidgets.Text("TUNING", "MicroLabel"));
        Choice("Family", "TuningFamily", look.Themes.Select(t => t.Id).ToArray(), look.Themes.Select(t => t.Name).ToArray(), look.Theme, Model.SetFamily);
        string[] looks = ThemeFamily.SceneLooks.ToArray();
        Choice("Look", "TuningLook", looks, looks.Select(AppearancePanel.Title).ToArray(), look.SceneLook, Model.SetLook);
        column.AddChild(new HSeparator());
        column.AddChild(ChromeWidgets.Text("SEED KNOBS", "MicroLabel"));
        foreach (string id in ThemeFamily.KnobIds)
        {
            Knob knob = look.Family.Knobs[id];
            HSlider slider = AppearancePanel.KnobSlider(id, knob, look.Knob(id)); slider.Name = "TuningKnob_" + id;
            Label value = AppearancePanel.KnobValue(knob, look.Knob(id));
            // A knob moves colours only: the frame is recoloured in place while it is dragged.
            slider.ValueChanged += v => { if (!syncing) Edit(() => Model.SetKnob(id, v), false, false); };
            Row(AppearancePanel.Title(id), slider, value);
            var strip = new HBoxContainer { Name = "TuningStrip_" + id, MouseFilter = MouseFilterEnum.Ignore };
            strip.AddThemeConstantOverride("separation", 1);
            for (int step = 0; step < ThemeFamily.Steps; step++)
                strip.AddChild(new ColorRect { SizeFlagsHorizontal = SizeFlags.ExpandFill, MouseFilter = MouseFilterEnum.Pass });
            column.AddChild(strip);
            knobs[id] = (slider, value, strip);
        }
        column.AddChild(new HSeparator());
        column.AddChild(ChromeWidgets.Text("COMPARE", "MicroLabel"));
        var tools = new HFlowContainer(); column.AddChild(tools);
        tools.AddChild(Tool("TuningSetA", "Set A", "Store the live tuning in slot A", () => Edit(() => Model.SetSlot("A"), false, false)));
        tools.AddChild(Tool("TuningSetB", "Set B", "Store the live tuning in slot B", () => Edit(() => Model.SetSlot("B"), false, false)));
        tools.AddChild(Tool("TuningSwap", "Swap", "Show the other slot; the live tuning goes into the slot it leaves", () => Edit(Model.Swap, Crosses, true)));
        Button hold = ChromeWidgets.Tool("Hold", "Hold to draw the other slot; release to return (pointer, or Space or Enter while focused)"); hold.Name = "TuningHold";
        hold.ButtonDown += () => Edit(Model.Hold, Crosses, true);
        hold.ButtonUp += () => Edit(Model.Release, Crosses, true);
        tools.AddChild(hold);
        column.AddChild(Compare);
        column.AddChild(Status);
        // B31: Presets (shipped read-only, forked on edit) and Save, which fails in place with its pairs.
        column.AddChild(new HSeparator());
        column.AddChild(ChromeWidgets.Text("PRESETS", "MicroLabel"));
        var presets = new OptionButton { Name = "TuningPreset", FitToLongestItem = false, ClipText = true };
        presets.ItemSelected += index =>
        {
            if (syncing || index < 0 || index >= Model.Presets.Count) return;
            Execute("tuning.preset.apply", Model.Presets[(int)index].Id);
        };
        Row("Preset", presets);
        var saving = new HFlowContainer(); column.AddChild(saving);
        saving.AddChild(Tool("TuningSave", "Save", "Save the tuning into its preset after the checks (contrast, the height ramp, the contact classes under each simulation)", () => Save(null)));
        column.AddChild(PresetInfo);
        column.AddChild(Issues);
        // B31: the held simulation over the real frame: the chrome's filter now; the pieces when the host sends the core.
        column.AddChild(new HSeparator());
        column.AddChild(ChromeWidgets.Text("SIMULATION", "MicroLabel"));
        string[] simulations = ColourMetric.Simulations.ToArray();
        var simulation = new OptionButton { Name = "TuningSimulation", FitToLongestItem = false, ClipText = true };
        foreach (string id in simulations) simulation.AddItem(id == "contrast" ? "Contrast (lightness only)" : AppearancePanel.Title(id));
        simulation.Select(Array.IndexOf(simulations, Model.Simulation));
        simulation.ItemSelected += index => { if (Model.Simulating != null) Simulate(simulations[(int)index]); else { Model.HoldSimulation(simulations[(int)index]); Model.ReleaseSimulation(); Sync(); } };
        Row("Vision", simulation);
        Button held = ChromeWidgets.Tool("Hold to simulate", "Hold to draw the frame as the chosen vision sees it (pointer, or Space or Enter while focused)"); held.Name = "TuningSimulate";
        held.ButtonDown += () => Simulate(Model.Simulation);
        held.ButtonUp += () => Simulate(null);
        column.AddChild(held);
        column.AddChild(SimulationInfo);
        // B31: Advanced, behind a switch: semantic role overrides of this family, with search.
        column.AddChild(new HSeparator());
        var advanced = new CheckButton { Name = "TuningAdvanced", Text = "Advanced", ButtonPressed = Model.Advanced };
        advanced.Toggled += on => { Model.SetAdvanced(on); Sync(); };
        column.AddChild(advanced);
        var find = new LineEdit { Name = "TuningAdvancedSearch", PlaceholderText = "Search roles", Text = search, ClearButtonEnabled = true };
        find.TextChanged += text => { search = text; Sync(); };
        column.AddChild(find);
        column.AddChild(roles);
        Sync();
    }

    // Save: the checks first; a refusal stays in place, every failing pair listed under the Save button.
    private void Save(string? name)
    {
        string? refused = Model.Save(name);
        Say(refused);
        Sync();
    }

    // Holds (an id) or releases (null) the simulation; the filter draws it over the chrome, the host sends the core.
    private void Simulate(string? id)
    {
        if (id == null) Model.ReleaseSimulation();
        else if (Model.HoldSimulation(id) is { } refused) { Say(refused); return; }
        Filter.Show(Model.Simulating == null ? null : Model.Matrix);
        Safe.Raise(Simulated);
        Sync();
    }

    // The words under the hold: what is simulated, and what the simulation covers.
    public string SimulationText => Model.Simulating is not { } held ? $"Released. Hold to simulate {Model.Simulation}."
        : $"Holding {held}: " + (CoreSimulation ? "the pieces and the chrome." : "the chrome only; the pieces wait for the core stage's look-cost row (G8).");

    private void SyncAdvanced()
    {
        foreach (Node child in roles.GetChildren()) { roles.RemoveChild(child); child.QueueFree(); }
        if (column.FindChild("TuningAdvancedSearch", false, false) is LineEdit find) find.Visible = Model.Advanced;
        if (column.FindChild("TuningAdvanced", false, false) is CheckButton toggle) toggle.SetPressedNoSignal(Model.Advanced);
        roles.Visible = Model.Advanced;
        if (!Model.Advanced) return;
        TokenSet tokens = Model.Look.Resolve();
        IReadOnlyList<string> found = Model.AdvancedRoles(search);
        foreach (string role in found.Take(12))
        {
            var row = new HBoxContainer { Name = "TuningRole_" + role.Replace('.', '_') };
            var swatch = new ColorRect { CustomMinimumSize = new Vector2(frame.Tokens.Px("icon.medium") / 2f, 0), Color = ChromeBody.Engine(tokens.Colour(role)), MouseFilter = MouseFilterEnum.Ignore };
            row.AddChild(swatch);
            bool overridden = Model.Look.Overrides.TryGetValue(role, out Oklch o);
            row.AddChild(ChromeWidgets.Text(role + (overridden ? " *" : ""), "MicroLabel"));
            var value = new LineEdit { SizeFlagsHorizontal = SizeFlags.ExpandFill, PlaceholderText = "L C h",
                Text = overridden ? string.Join(" ", new[] { o.L, o.C, o.H }.Select(v => v.ToString("0.###", System.Globalization.CultureInfo.InvariantCulture))) : "" };
            value.TextSubmitted += text => { if (string.IsNullOrWhiteSpace(text)) Execute("token.override.reset", role); else Execute("token.override.set", role + "=" + text); };
            row.AddChild(value);
            if (overridden) row.AddChild(Tool("TuningReset_" + role.Replace('.', '_'), "Reset", "Back to the family's colour", () => Execute("token.override.reset", role)));
            roles.AddChild(row);
        }
        if (found.Count > 12) roles.AddChild(ChromeWidgets.Text($"{found.Count - 12} more; search to narrow", "MicroLabel"));
    }

    // The knob values and the live ramp strips, from the appearance in force; no control is replaced.
    private void Sync()
    {
        if (knobs.Count == 0 || knobs.Values.Any(k => !IsInstanceValid(k.Slider))) return;
        Appearance look = Model.Look;
        int height = Math.Max(6, frame.Tokens.Px("icon.medium") / 3);
        syncing = true;
        try
        {
            foreach (var (id, (slider, value, strip)) in knobs)
            {
                Knob knob = look.Family.Knobs[id];
                slider.SetValueNoSignal(look.Knob(id)); value.Text = AppearancePanel.KnobText(knob, look.Knob(id));
                TuningStrip ramp = Model.Strip(id);
                strip.TooltipText = ramp.Name;
                for (int step = 0; step < strip.GetChildCount() && step < ramp.Steps.Count; step++)
                {
                    var swatch = strip.GetChild<ColorRect>(step);
                    swatch.Color = ChromeBody.Engine(ramp.Steps[step]);
                    swatch.CustomMinimumSize = new Vector2(0, ramp.Mark == step ? height * 2 : height);
                    swatch.TooltipText = $"{ramp.Name}.{step}" + (ramp.Mark == step ? " (now)" : "");
                }
            }
        }
        finally { syncing = false; }
        Compare.Text = Model.Line;
        // B31: the presets list (a fork or a save adds one), the preset line, Save's issues, the simulation, Advanced.
        if (column.FindChild("TuningPreset", true, false) is OptionButton presets)
        {
            syncing = true;
            try
            {
                if (presets.ItemCount != Model.Presets.Count)
                {
                    presets.Clear();
                    foreach (TuningPreset p in Model.Presets) presets.AddItem(p.Name + (p.Shipped ? " (shipped)" : ""));
                }
                presets.Select(Model.Presets.ToList().FindIndex(p => p.Id == Model.Preset));
            }
            finally { syncing = false; }
        }
        PresetInfo.Text = Model.PresetLine;
        Issues.Text = Model.Issues.Count == 0 ? "" : "Save refused:\n" + string.Join("\n", Model.Issues.Select(i => "· " + i));
        SimulationInfo.Text = SimulationText;
        SyncAdvanced();
    }
}

// B31: the chrome's half of the held simulation: one full-window rectangle over the chrome that maps every drawn
// pixel through the simulation's 3x3 matrix on linear RGB (the core's view style takes the same matrix). It keeps
// each pixel's alpha, so where the window shows the core's frame beneath, nothing is drawn.
public sealed partial class SimulationFilter : ColorRect
{
    private const string Code = @"shader_type canvas_item;
uniform sampler2D screen : hint_screen_texture, filter_nearest;
uniform vec3 row0 = vec3(1.0, 0.0, 0.0);
uniform vec3 row1 = vec3(0.0, 1.0, 0.0);
uniform vec3 row2 = vec3(0.0, 0.0, 1.0);
vec3 decode(vec3 c) { return mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(vec3(0.04045), c)); }
vec3 encode(vec3 c) { return mix(c * 12.92, 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, step(vec3(0.0031308), c)); }
void fragment() {
    vec4 seen = texture(screen, SCREEN_UV);
    vec3 l = decode(seen.rgb);
    COLOR = vec4(encode(clamp(vec3(dot(row0, l), dot(row1, l), dot(row2, l)), 0.0, 1.0)), seen.a);
}";
    private readonly ShaderMaterial material = new() { Shader = new Shader { Code = Code } };
    public double[]? Matrix { get; private set; }

    public SimulationFilter()
    {
        Name = "SimulationFilter"; Visible = false; MouseFilter = MouseFilterEnum.Ignore; Material = material;
        SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
    }

    // A matrix shows the filter on top of the chrome; null hides it.
    public void Show(double[]? matrix)
    {
        Matrix = matrix;
        Visible = matrix != null;
        if (matrix == null) return;
        for (int row = 0; row < 3; row++)
            material.SetShaderParameter("row" + row, new Vector3((float)matrix[row * 3], (float)matrix[row * 3 + 1], (float)matrix[row * 3 + 2]));
        GetParent()?.MoveChild(this, -1);
    }
}
