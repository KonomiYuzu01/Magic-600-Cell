using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using LookLab.Core;
using LookLab.App.Platform;
using LookLab.App;

internal static partial class Program
{
    private static void UploadInputs()
    {
        Equal(Mesh.BaseVertexData.Length, Geometry.BaseVertices * 4);
        Equal(Mesh.BaseStickerIds.Length, Geometry.BaseVertices);
        Equal(Mesh.StickerCenters.Length, Geometry.StickersPerCell * 4);
        Equal(Mesh.CellFrames.Length, Geometry.Cells * 16);
        Equal(Mesh.SlotOrbits.Length, Geometry.Slots);
        using var document = JsonDocument.Parse(Read("fixtures/sb-reference.json"));
        foreach (var data in document.RootElement.GetProperty("cases").EnumerateArray())
        {
            double[] q = Camera(data); double theta = data.GetProperty("theta").GetDouble();
            StructureValues p = ReferenceValues(document.RootElement);
            foreach (var sample in data.GetProperty("samples").EnumerateArray())
            {
                int global = sample.GetProperty("vertex").GetInt32(), cell = global / Geometry.BaseVertices, vi = global % Geometry.BaseVertices;
                int local = Mesh.BaseStickerIds.Span[vi]; var basePoint = new double[4];
                for (int i = 0; i < 4; i++) basePoint[i] = (Mesh.BaseNormal.Span[i] + p.Cs * (Mesh.StickerCenters.Span[local * 4 + i] - Mesh.BaseNormal.Span[i])
                    + p.Cs * p.Ss * (Mesh.BaseVertexData.Span[vi * 4 + i] - Mesh.StickerCenters.Span[local * 4 + i])) / Mesh.Radius;
                double[] world = Geometry.Multiply(Mesh.CellFrames.Span.Slice(cell * 16, 16), basePoint);
                if (Turn.IsMoving(cell * Geometry.StickersPerCell + local))
                {
                    double x = Geometry.Dot(world, Turn.PlaneU.Span), y = Geometry.Dot(world, Turn.PlaneV.Span);
                    double du = (Math.Cos(theta) - 1) * x - Math.Sin(theta) * y, dv = Math.Sin(theta) * x + (Math.Cos(theta) - 1) * y;
                    for (int i = 0; i < 4; i++) world[i] += du * Turn.PlaneU.Span[i] + dv * Turn.PlaneV.Span[i];
                }
                world = Geometry.Multiply(q, world);
                double f = p.D4 / (p.D4 - world[3]), w = 5 - world[2] * f;
                Projection expected = Mesh.Project(global, p, q, Turn, theta);
                Near(world[0] * f * p.Zoom / p.Aspect / w, expected.NdcX);
                Near(world[1] * f * p.Zoom / w, expected.NdcY); Near(w, expected.W);
            }
        }
        // The actual vertex orbit is 34; its 120 pieces meet 20 cells each.
        Equal(Mesh.SlotOrbits.Span.ToArray().Count(orbit => orbit == 34), 2400);
    }

    private static void LookStateValidation()
    {
        var state = new LookState(Schema, new Preset("Smoke", null, null, Schema.Defaults));
        int changed = 0; state.Changed += (_, _) => changed++;
        Refuse(() => state.Set("gap", new NumberValue(2)), "gap"); Equal(changed, 0);
        Near(state.Preset.Params.Number("gap"), Schema.Defaults.Number("gap"));
        foreach (ParameterDefinition p in Schema.Parameters) state.Set(p.Id, SmokeProof.NonDefault(p));
        Equal(changed, Schema.Parameters.Count); Schema.Validate(state.Preset.Params);
        Preset candidate = state.Preset;
        Refuse(() => state.Load(candidate with { Params = candidate.Params.With("unknown", new NumberValue(1)) }), "unknown");
        Check(ReferenceEquals(state.Preset, candidate), "failed load changed the preset");
        state.Load(new Preset("Loaded", null, null, Schema.Defaults));
        Equal(changed, Schema.Parameters.Count * 2);
    }

    private static void PresetPathBoundary()
    {
        using var temp = new TestFolder();
        var files = new PresetFiles(temp.Path, Schema);
        Check(!files.Exists, "preset folder must be created on first save only");
        files.PrepareSave(); Check(files.Exists, "save did not create folder");
        string path = Path.Combine(files.Folder, "test.json");
        var preset = new Preset("Test", null, null, Schema.Defaults);
        files.Save(path, preset); Sequence(Presets.CanonicalBytes(files.Load(path), Schema), File.ReadAllBytes(path));
        foreach (string bad in new[] { Path.Combine(temp.Path, "outside.json"), Path.Combine(files.Folder, "../outside.json"), Path.Combine(files.Folder, "bad.txt") })
        {
            bool refused = false;
            try { files.Save(bad, preset); } catch (IOException) { refused = true; }
            Check(refused && !File.Exists(bad), "preset boundary did not refuse " + bad);
        }
    }

    private static void ShaderAdapter()
    {
        string text = Read("app/shaders/cells.gdshader");
        var source = new ShaderSource(text);
        Check(source.Vertex().Contains(source.VertexBody, StringComparison.Ordinal), "drawing vertex body changed in adapter");
        Check(source.Fragment().Contains(source.FragmentBody, StringComparison.Ordinal), "drawing fragment body changed in adapter");
        Check(text.Contains(source.VertexBody, StringComparison.Ordinal) && text.Contains(source.FragmentBody, StringComparison.Ordinal), "adapter does not use the drawing shader itself");
        Equal(source.Uniforms.Select(u => u.Name).Distinct().Count(), source.Uniforms.Count);
        Equal(source.Varyings.Count, 6);
        Sequence(source.Uniforms.Where(u => u.Type == "sampler2D").Select(u => u.Binding), Enumerable.Range(2, 7));
        var values = new Dictionary<string, float[]>();
        foreach (ShaderUniform u in source.Uniforms.Where(u => u.Type != "sampler2D"))
            values[u.Name] = Enumerable.Range(1, u.Type == "mat4" ? 16 : u.Type == "vec4" ? 4 : 1).Select(i => (float)i).ToArray();
        values["transpose_q"][0] = 0;
        float[] packed = source.PackParameters(name => values[name]);
        Equal(packed.Length, source.ParameterVectors * 4);
        foreach (ShaderUniform u in source.Uniforms.Where(u => u.Type != "sampler2D"))
            Sequence(packed.Skip(u.Offset * 4).Take(values[u.Name].Length), values[u.Name], "uniform packing " + u.Name);
        ShaderUniform q = source.Uniforms.Single(u => u.Name == "q");
        Check(source.Vertex().Contains($"#define q mat4(parameter_data[{q.Offset}],parameter_data[{q.Offset + 1}],parameter_data[{q.Offset + 2}],parameter_data[{q.Offset + 3}])", StringComparison.Ordinal), "Q binding changed column order");
        values["transpose_q"][0] = 1;
        float[] fault = source.PackParameters(name => values[name]);
        Equal(packed.Zip(fault).Count(p => p.First != p.Second), 1);
        Equal(fault[source.Uniforms.Single(u => u.Name == "transpose_q").Offset * 4], 1f);
        Check(source.VertexBody.Contains("transpose(q) * world", StringComparison.Ordinal), "transpose fault is not in the drawing shader");
    }

    private static void ShaderAdapterRefusals()
    {
        string text = Read("app/shaders/cells.gdshader");
        Refuse(() => new ShaderSource(text.Replace("void vertex()", "void removed()", StringComparison.Ordinal)), "vertex");
        Refuse(() => new ShaderSource(text.Replace("uniform float radius;", "uniform unknown radius;", StringComparison.Ordinal)), "uniform");
        Refuse(() => new ShaderSource(text[..text.LastIndexOf('}')]), "unbalanced");
        var source = new ShaderSource(text);
        Refuse(() => source.PackParameters(_ => Array.Empty<float>()), "uniform");
        Refuse(() => source.PackParameters(_ => new[] { float.NaN }), "uniform");
    }
}
