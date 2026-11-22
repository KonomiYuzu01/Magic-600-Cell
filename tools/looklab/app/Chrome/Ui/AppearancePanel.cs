using System;
using System.Globalization;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The appearance settings as a dock panel: theme family, density, display scale, high contrast, scene look and
// the family's tuning knobs (ADR-D01.35, ADR-D01.45, ADR-D01.49, ADR-D01.50). It edits an Appearance and
// reports what the host has to redo; it holds no colour or size of its own.
public sealed partial class AppearancePanel : ScrollContainer
{
    public Appearance Look { get; private set; }
    // rebuild: sizes, faces or the family changed; scene: the scene look to apply changed.
    public event Action<bool, bool>? Changed;
    public Label Issues { get; } = new() { ThemeTypeVariation = "MicroLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    private readonly VBoxContainer column = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill };

    public AppearancePanel(Appearance look)
    {
        Look = look; Name = "AppearancePanel"; HorizontalScrollMode = ScrollMode.Disabled;
        var margin = new MarginContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill }; AddChild(margin);
        foreach (string side in new[] { "margin_left", "margin_right", "margin_top", "margin_bottom" }) margin.AddThemeConstantOverride(side, 8);
        margin.AddChild(column);
        Refresh();
    }

    public void Show(Appearance look) { Look = look; Refresh(); }

    // Every control of the panel goes through here; a refused value leaves the appearance as it was.
    public bool Edit(Action<Appearance> change, bool rebuild, bool scene)
    {
        try { change(Look); }
        catch (FormatException ex) { Issues.Text = ex.Message; return false; }
        if (rebuild) Refresh(); else Report();
        Safe.Raise(Changed, rebuild, scene);
        return true;
    }

    private void Report()
    {
        var issues = Look.Resolve().ContrastIssues();
        Issues.Text = issues.Count == 0 ? (Look.Tuned ? "Tuned. Every contrast pair passes." : "Shipped values. Every contrast pair passes.") : "Low contrast: " + string.Join("; ", issues);
    }

    private static string Title(string id) => char.ToUpperInvariant(id[0]) + string.Concat(id.Skip(1).Select(c => char.IsUpper(c) ? " " + char.ToLowerInvariant(c) : c.ToString()));

    private Control Row(string label, Control control)
    {
        var row = new HBoxContainer();
        Label name = ChromeWidgets.Text(label, "DimLabel"); name.CustomMinimumSize = new Vector2(96, 0); row.AddChild(name);
        control.SizeFlagsHorizontal = SizeFlags.ExpandFill; row.AddChild(control);
        column.AddChild(row);
        return control;
    }

    private OptionButton Choice(string label, string[] ids, string[] names, string current, Action<string> chosen)
    {
        var choice = new OptionButton { Name = label.Replace(" ", "") + "Choice", FitToLongestItem = false, ClipText = true };
        foreach (string name in names) choice.AddItem(name);
        choice.Select(Array.IndexOf(ids, current));
        choice.ItemSelected += index => chosen(ids[(int)index]);
        Row(label, choice);
        return choice;
    }

    private void Refresh()
    {
        foreach (Node child in column.GetChildren()) { column.RemoveChild(child); if (child != Issues) child.QueueFree(); }
        string[] families = Look.Themes.Select(t => t.Id).ToArray();
        Choice("Theme", families, Look.Themes.Select(t => t.Name).ToArray(), Look.Theme, id => Edit(a => a.SetTheme(id), true, true));
        string[] densities = Look.Resolve().System.Densities.ToArray();
        Choice("Density", densities, densities.Select(Title).ToArray(), Look.Density, id => Edit(a => a.SetDensity(id), true, false));
        var scale = new HSlider { Name = "ScaleSlider", MinValue = .75, MaxValue = 2, Step = .25, Value = Look.DisplayScale, TickCount = 6, TicksOnBorders = true };
        Label scaleValue = ChromeWidgets.Text(Look.DisplayScale.ToString("0.00", CultureInfo.InvariantCulture) + "x", "DimLabel");
        scale.ValueChanged += v => scaleValue.Text = v.ToString("0.00", CultureInfo.InvariantCulture) + "x";
        // The frame is rebuilt at the new scale when the drag ends, not under the pointer.
        scale.DragEnded += changed => { if (changed) Edit(a => a.SetDisplayScale(scale.Value), true, false); };
        Row("Display scale", scale); scale.GetParent().AddChild(scaleValue);
        var contrast = new CheckButton { Name = "HighContrast", Text = "High contrast", ButtonPressed = Look.HighContrast };
        contrast.Toggled += on => Edit(a => a.HighContrast = on, false, false);
        column.AddChild(contrast);
        var character = new CheckButton { Name = "OtherCharacter", Text = Look.Family.Character == "lines" ? "Draw with tone steps" : "Draw with lines", ButtonPressed = Look.OtherCharacter,
            TooltipText = "The comparison character: this family drawn the other family's way" };
        character.Toggled += on => Edit(a => a.OtherCharacter = on, true, false);
        column.AddChild(character);
        var reduced = new CheckButton { Name = "ReducedMotion", Text = "Reduced motion", ButtonPressed = Look.ReducedMotion, TooltipText = "Every motion shows its static equivalent at once" };
        reduced.Toggled += on => Edit(a => a.ReducedMotion = on, false, false);
        column.AddChild(reduced);
        string[] looks = ThemeFamily.SceneLooks.ToArray();
        Choice("Scene look", looks, looks.Select(Title).ToArray(), Look.SceneLook, id => Edit(a => a.SetSceneLook(id), false, true));
        column.AddChild(new HSeparator());
        var head = new HBoxContainer(); column.AddChild(head);
        Label title = ChromeWidgets.Text("TUNE " + Look.Family.Name.ToUpperInvariant(), "MicroLabel"); title.SizeFlagsHorizontal = SizeFlags.ExpandFill; head.AddChild(title);
        Button reset = ChromeWidgets.Tool("Reset", "Return this family's knobs to their shipped values"); reset.Name = "KnobReset";
        reset.Pressed += () => Edit(a => a.ResetKnobs(), true, false); head.AddChild(reset);
        foreach (string id in ThemeFamily.KnobIds)
        {
            Knob knob = Look.Family.Knobs[id];
            string format = knob.Max - knob.Min > 10 ? "0" : "0.000";
            var slider = new HSlider { Name = id + "Knob", MinValue = knob.Min, MaxValue = knob.Max, Step = (knob.Max - knob.Min) / 240, Value = Look.Knob(id),
                TooltipText = $"{Title(id)}: {knob.Min.ToString(CultureInfo.InvariantCulture)} to {knob.Max.ToString(CultureInfo.InvariantCulture)}, shipped {knob.Default.ToString(CultureInfo.InvariantCulture)}" };
            Label value = ChromeWidgets.Text(Look.Knob(id).ToString(format, CultureInfo.InvariantCulture), "DimLabel"); value.CustomMinimumSize = new Vector2(44, 0);
            value.HorizontalAlignment = HorizontalAlignment.Right;
            // A knob moves colours only, so the frame is recoloured in place while it is dragged.
            slider.ValueChanged += v => { if (Edit(a => a.SetKnob(id, Math.Clamp(v, knob.Min, knob.Max)), false, false)) value.Text = Look.Knob(id).ToString(format, CultureInfo.InvariantCulture); };
            Row(Title(id), slider); slider.GetParent().AddChild(value);
        }
        column.AddChild(Issues);
        Report();
    }
}
