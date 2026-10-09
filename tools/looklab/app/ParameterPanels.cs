using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using LookLab.Core;

namespace LookLab.App;

internal sealed partial class ParameterPanels : VBoxContainer
{
    private readonly LookState state;
    private readonly Dictionary<string, Action<ParameterValue>> refresh = new(StringComparer.Ordinal);
    public int ParameterCount => refresh.Count;
    public IReadOnlyList<Label> Headers { get; }

    public ParameterPanels(LookState state)
    {
        this.state = state;
        var headers = new List<Label>();
        foreach (string group in ParameterSchema.Groups)
        {
            var header = new Label { Text = group }; headers.Add(header); AddChild(header);
            AddChild(new HSeparator());
            foreach (ParameterDefinition p in state.Schema.Parameters.Where(p => p.Group == group)) AddParameter(p);
        }
        Headers = headers.AsReadOnly();
        state.Changed += Refresh;
    }

    private void AddParameter(ParameterDefinition p)
    {
        var group = new VBoxContainer { TooltipText = p.Description };
        AddChild(group);
        group.AddChild(new Label { Text = p.Id, TooltipText = p.Description });
        ParameterValue initial = state.Preset.Params.Values[p.Id];
        switch (p.Kind)
        {
            case ParameterKind.Number:
            case ParameterKind.Integer:
            {
                var row = new HBoxContainer(); group.AddChild(row);
                var slider = new HSlider { MinValue = p.Min!.Value, MaxValue = p.Max!.Value, Step = p.Step!.Value,
                    SizeFlagsHorizontal = SizeFlags.ExpandFill, CustomMinimumSize = new Vector2(110, 0) };
                var field = new SpinBox { MinValue = p.Min.Value, MaxValue = p.Max.Value, Step = p.Step.Value,
                    CustomMinimumSize = new Vector2(92, 0) };
                row.AddChild(slider); row.AddChild(field);
                Action<ParameterValue> update = value =>
                {
                    double n = value is NumberValue number ? number.Value : ((IntegerValue)value).Value;
                    slider.SetValueNoSignal(n); field.SetValueNoSignal(n);
                };
                void Change(double value) => state.Set(p.Id, p.Kind == ParameterKind.Number
                    ? new NumberValue(value) : new IntegerValue((int)value));
                slider.ValueChanged += Change; field.ValueChanged += Change;
                refresh.Add(p.Id, update); update(initial);
                break;
            }
            case ParameterKind.Enum:
            {
                var choice = new OptionButton(); group.AddChild(choice);
                foreach (string option in p.Options!) choice.AddItem(option);
                choice.ItemSelected += index => state.Set(p.Id, new EnumValue(p.Options![(int)index]));
                refresh.Add(p.Id, value => choice.Select(p.Options!.ToList().IndexOf(((EnumValue)value).Value)));
                refresh[p.Id](initial);
                break;
            }
            case ParameterKind.Boolean:
            {
                var toggle = new CheckButton { Text = "Enabled" }; group.AddChild(toggle);
                toggle.Toggled += value => state.Set(p.Id, new BooleanValue(value));
                refresh.Add(p.Id, value => toggle.SetPressedNoSignal(((BooleanValue)value).Value));
                refresh[p.Id](initial);
                break;
            }
            case ParameterKind.Colour:
            {
                var row = new HBoxContainer(); group.AddChild(row);
                SpinBox l = Component(row, "L", 1, .01), c = Component(row, "C", 1, .01), h = Component(row, "h°", 360, 1);
                l.ValueChanged += value => state.Set(p.Id, new ColourValue(state.Preset.Params.Colour(p.Id) with { L = value }));
                c.ValueChanged += value => state.Set(p.Id, new ColourValue(state.Preset.Params.Colour(p.Id) with { C = value }));
                h.ValueChanged += value => state.Set(p.Id, new ColourValue(state.Preset.Params.Colour(p.Id) with { H = value }));
                refresh.Add(p.Id, value =>
                {
                    OklchColour colour = ((ColourValue)value).Value;
                    l.SetValueNoSignal(colour.L); c.SetValueNoSignal(colour.C); h.SetValueNoSignal(colour.H);
                });
                refresh[p.Id](initial);
                break;
            }
            case ParameterKind.Curve:
            {
                group.AddChild(new Label { Text = "(0,0) → (a,0) → (1−b,1) → (1,1)" });
                var row = new HBoxContainer(); group.AddChild(row);
                SpinBox a = Component(row, "a", 1, .01), b = Component(row, "b", 1, .01);
                a.ValueChanged += value => state.Set(p.Id, new CurveValue(state.Preset.Params.Curve(p.Id) with { A = value }));
                b.ValueChanged += value => state.Set(p.Id, new CurveValue(state.Preset.Params.Curve(p.Id) with { B = value }));
                refresh.Add(p.Id, value =>
                {
                    BezierCurve curve = ((CurveValue)value).Value;
                    a.SetValueNoSignal(curve.A); b.SetValueNoSignal(curve.B);
                });
                refresh[p.Id](initial);
                break;
            }
        }
    }

    private static SpinBox Component(HBoxContainer row, string label, double max, double step)
    {
        var column = new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill };
        row.AddChild(column); column.AddChild(new Label { Text = label });
        var control = new SpinBox { MinValue = 0, MaxValue = max, Step = step };
        column.AddChild(control);
        return control;
    }

    private void Refresh(string id, ParameterValue value) => refresh[id](value);
    public override void _ExitTree() => state.Changed -= Refresh;
}
