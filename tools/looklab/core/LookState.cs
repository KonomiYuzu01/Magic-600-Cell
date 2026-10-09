using System;
using System.Collections.Generic;
using System.Linq;

namespace LookLab.Core;

// Both generated controls and the app's smoke test use this validated setter.
public sealed class LookState
{
    public ParameterSchema Schema { get; }
    public Preset Preset { get; private set; }
    public event Action<string, ParameterValue>? Changed;

    public LookState(ParameterSchema schema, Preset preset)
    {
        Presets.Validate(preset, schema);
        Schema = schema; Preset = preset;
    }

    public void Set(string id, ParameterValue value)
    {
        ParameterSchema.ValidateValue(Schema[id], value);
        Preset = Preset with { Params = Preset.Params.With(id, value) };
        Changed?.Invoke(id, value);
    }

    public void Load(Preset preset)
    {
        Presets.Validate(preset, Schema);
        Preset = preset;
        foreach (ParameterDefinition p in Schema.Parameters) Changed?.Invoke(p.Id, preset.Params.Values[p.Id]);
    }

    public void Rename(string name)
    {
        Preset candidate = Preset with { Name = name };
        Presets.Validate(candidate, Schema);
        Preset = candidate;
    }
}

public static class SmokeProof
{
    public static ParameterValue NonDefault(ParameterDefinition p) => p.Default switch
    {
        NumberValue n => new NumberValue(n.Value == p.Max ? p.Min!.Value : Math.Min(p.Max!.Value, n.Value + p.Step!.Value)),
        IntegerValue n => new IntegerValue(n.Value == p.Max ? (int)p.Min!.Value : n.Value + (int)p.Step!.Value),
        EnumValue e => new EnumValue(p.Options!.First(option => option != e.Value)),
        BooleanValue b => new BooleanValue(!b.Value),
        ColourValue c => new ColourValue(c.Value with { H = c.Value.H == 360 ? 0 : c.Value.H + 1 }),
        CurveValue c => new CurveValue(c.Value with { A = c.Value.A == 1 ? 0 : Math.Min(1, c.Value.A + .1) }),
        _ => throw Json.Error(p.Id, "unsupported smoke value")
    };

    public static void Require(ParameterSchema schema, ParameterSet values, IReadOnlyDictionary<string, int> changes,
        int instances, ReadOnlySpan<byte> saved, ReadOnlySpan<byte> reloaded)
    {
        if (instances != Geometry.Cells) throw Json.Error("wrong-instance-count", "expected 600 instances");
        foreach (ParameterDefinition p in schema.Parameters)
            if (!changes.TryGetValue(p.Id, out int count) || count != 1 || values.Values[p.Id] == p.Default)
                throw Json.Error("skip-parameter", "expected one non-default change through the panel setter: " + p.Id);
        if (!saved.SequenceEqual(reloaded)) throw Json.Error("changed-preset-byte", "preset bytes differ after reload");
    }
}
