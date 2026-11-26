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
    public static readonly IReadOnlyList<string> Rows = Array.AsReadOnly(new[] { "tuning.open", "tuning.slot.set", "tuning.slot.swap", "tuning.compare.hold", "theme.tune" });
    public Tuning Model { get; }
    public Label Compare { get; } = new() { Name = "TuningCompareLine", ThemeTypeVariation = "DimLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    public Label Status { get; } = new() { Name = "TuningStatus", ThemeTypeVariation = "MicroLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    private readonly AppearancePanel appearance;
    private readonly ChromeFrame frame;
    private readonly VBoxContainer column = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill };
    private readonly Dictionary<string, (HSlider Slider, Label Value, HBoxContainer Strip)> knobs = new(StringComparer.Ordinal);
    private bool syncing;
    private string built = "";

    public TuningPanel(AppearancePanel appearance, ChromeFrame frame)
    {
        this.appearance = appearance; this.frame = frame; Model = new Tuning(appearance.Look);
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
        _ => null,
    };

    public void Execute(string command, string? argument)
    {
        switch (command)
        {
            case "tuning.open": if (!Tuning.Open(frame.Model)) Say(frame.Model.Refusal ?? "the right dock has no tuning panel"); break;
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
        foreach (Node child in column.GetChildren()) { column.RemoveChild(child); if (child != Compare && child != Status) child.QueueFree(); }
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
        Sync();
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
    }
}
