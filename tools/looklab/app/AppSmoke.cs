using System;
using System.Collections.Generic;
using System.IO;
using LookLab.Core;

namespace LookLab.App;

internal static class AppSmoke
{
    public static void Run(LabApp app, string scratch, string fault)
    {
        var changes = new Dictionary<string, int>(StringComparer.Ordinal);
        app.State.Changed += (id, _) => changes[id] = changes.GetValueOrDefault(id) + 1;
        foreach (ParameterDefinition p in app.State.Schema.Parameters)
            if (fault != "skip-parameter" || p.Id != "gap") app.State.Set(p.Id, SmokeProof.NonDefault(p));
        if (app.Panels.ParameterCount != app.State.Schema.Parameters.Count) throw new Exception("generated panel count differs from schema");
        if (fault == "wrong-instance-count") app.View.Instances.InstanceCount = Geometry.Cells - 1;
        string path = Path.Combine(scratch, "smoke-preset.json");
        Presets.Save(path, app.State.Preset, app.State.Schema);
        byte[] saved = File.ReadAllBytes(path);
        Preset loaded = Presets.Load(path, app.State.Schema);
        byte[] reloaded = Presets.CanonicalBytes(loaded, app.State.Schema);
        if (fault == "changed-preset-byte") reloaded[0] ^= 1;
        SmokeProof.Require(app.State.Schema, app.State.Preset.Params, changes, app.View.Instances.InstanceCount, saved, reloaded);
        Godot.GD.Print($"LOOKLAB_SMOKE_PASS instances={app.View.Instances.InstanceCount} vertices={Geometry.BaseVertices} slots={Geometry.Slots} parameters={changes.Count} bytes={saved.Length}");
    }
}
