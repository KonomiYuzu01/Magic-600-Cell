using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using Godot;
using LookLab.Core;
using FileAccess = Godot.FileAccess;

namespace LookLab.App;

internal static class ShaderGeometryCheck
{
    // Same tolerance as the S-B GPU/reference probe. Core calculations are
    // double precision; shader matrices, trig, division and output are f32.
    public const double AbsoluteTolerance = 1e-4;
    public const double RelativeTolerance = 1e-4;
    public const ulong ExpectedPrimitives = 6096000;

    public static async Task Run(Node host, string root, string fault)
    {
        var schema = ParameterSchema.Load(Path.Combine(root, "tools/looklab/data/parameters.json"));
        var state = new LookState(schema, new Preset("Geometry check", null, null, schema.Defaults));
        var view = new LookViewport(root, state) { Animate = false };
        host.AddChild(view);
        if (fault == "missing-instance") view.Instances.VisibleInstanceCount = Geometry.Cells - 1;
        // Nothing except the cells is in this scene. The normal drawing shader
        // remains bound here; the pixel probes below use its exact function bodies.
        for (int i = 0; i < 3; i++) await host.ToSignal(RenderingServer.Singleton, RenderingServer.SignalName.FramePostDraw);
        ulong primitives = RenderingServer.GetRenderingInfo(RenderingServer.RenderingInfo.TotalPrimitivesInFrame);
        if (view.Instances.InstanceCount != Geometry.Cells || view.Instances.VisibleInstanceCount != Geometry.Cells || primitives != ExpectedPrimitives)
            throw new Exception($"missing-instance: instances={view.Instances.InstanceCount} visible={view.Instances.VisibleInstanceCount} primitives={primitives}; expected 600,600,{ExpectedPrimitives}");
        GD.Print($"LOOKLAB_DRAW_COUNT_PASS instances=600 visible=600 primitives={primitives}");
        var source = new ShaderSource(FileAccess.GetFileAsString("res://shaders/cells.gdshader"));
        using var fixture = JsonDocument.Parse(File.ReadAllText(Path.Combine(root, "tools/looklab/fixtures/sb-reference.json")));
        JsonElement f = fixture.RootElement, parameters = f.GetProperty("parameters");
        var structure = new StructureValues(parameters.GetProperty("cs").GetDouble(), parameters.GetProperty("ss").GetDouble(),
            parameters.GetProperty("d4").GetDouble(), parameters.GetProperty("zoom").GetDouble(), f.GetProperty("aspect").GetDouble());
        int cases = 0, samples = 0;
        foreach (JsonElement data in f.GetProperty("cases").EnumerateArray())
        {
            double[] q = Geometry.Identity();
            foreach (JsonElement r in data.GetProperty("rotations").EnumerateArray()) Geometry.Rotate(q, r[0].GetInt32(), r[1].GetInt32(), r[2].GetDouble());
            double theta = data.GetProperty("theta").GetDouble();
            JsonElement[] points = data.GetProperty("samples").EnumerateArray().ToArray();
            if (points.Length != 300) throw new Exception("geometry: expected 300 sampled vertices per case");
            SetCase(view, structure, q, theta, points.Length, fault == "transpose-q");
            using var target = new FloatTarget();
            Godot.Collections.Array<RDUniform> uniforms = Bindings(target, view, source, points);
            byte[] result = target.Draw(source.Vertex(), source.Fragment(), points.Length, (uint)points.Length * 3, uniforms);
            for (int index = 0; index < points.Length; index++)
            {
                int global = points[index].GetProperty("vertex").GetInt32();
                LookLab.Core.Projection expected = view.Geometry.Project(global, structure, q, view.Turn, theta);
                double[] core = { expected.NdcX, expected.NdcY, expected.W };
                JsonElement reference = points[index].GetProperty("reference");
                for (int component = 0; component < 3; component++)
                {
                    float actual = BinaryPrimitives.ReadSingleLittleEndian(result.AsSpan(index * 16 + component * 4, 4));
                    if (!Near(actual, core[component]) || !Near(actual, reference[component].GetDouble()))
                        throw new Exception($"{(fault == "transpose-q" ? "transpose-q" : "geometry")}: {data.GetProperty("camera").GetString()}/{data.GetProperty("pose").GetString()} vertex={global} component={component} actual={actual:R} core={core[component]:R} fixture={reference[component].GetDouble():R}; tolerance=1e-4+1e-4*abs(expected)");
                }
            }
            samples += points.Length; cases++;
        }
        if (cases != 9 || samples != 2700) throw new Exception("geometry: expected nine camera/pose cases and 2700 vertices");
        GD.Print($"LOOKLAB_GEOMETRY_PASS cases={cases} samples={samples} target=RGBA32F tolerance=1e-4+1e-4*abs(expected)");
        host.GetTree().Quit();
    }

    private static bool Near(double actual, double expected) => double.IsFinite(actual) &&
        Math.Abs(actual - expected) <= AbsoluteTolerance + RelativeTolerance * Math.Abs(expected);

    private static void SetCase(LookViewport view, StructureValues p, double[] q, double theta, int width, bool transpose)
    {
        view.Material.SetShaderParameter("q", LookViewport.Matrix(q));
        foreach (var (name, value) in new[] { ("cs", p.Cs), ("ss", p.Ss), ("d4", p.D4), ("zoom", p.Zoom), ("aspect", p.Aspect), ("theta", theta) })
            view.Material.SetShaderParameter(name, (float)value);
        view.Material.SetShaderParameter("projection_kind", (int)p.Projection);
        view.Material.SetShaderParameter("geometry_check", true);
        view.Material.SetShaderParameter("transpose_q", transpose);
        view.Material.SetShaderParameter("geometry_width", width);
    }

    private static byte[] Floats(ReadOnlySpan<float> values)
    {
        var bytes = new byte[values.Length * 4];
        for (int i = 0; i < values.Length; i++) BinaryPrimitives.WriteSingleLittleEndian(bytes.AsSpan(i * 4, 4), values[i]);
        return bytes;
    }

    private static Godot.Collections.Array<RDUniform> Bindings(FloatTarget target, LookViewport view, ShaderSource source, JsonElement[] samples)
    {
        RenderingDevice rd = target.Device;
        // Fetch actual MultiMesh transforms/custom data and actual mesh attributes,
        // rather than recreating a parallel geometry upload from the core.
        var arrays = view.Instances.Mesh.SurfaceGetArrays(0);
        Vector3[] positions = arrays[(int)Mesh.ArrayType.Vertex].AsVector3Array();
        Vector2[] uv = arrays[(int)Mesh.ArrayType.TexUV].AsVector2Array();
        var inputs = new float[samples.Length * 28];
        for (int i = 0; i < samples.Length; i++)
        {
            int g = samples[i].GetProperty("vertex").GetInt32(), cell = g / Geometry.BaseVertices, vi = g % Geometry.BaseVertices;
            Transform3D model = view.Instances.GetInstanceTransform(cell); Color custom = view.Instances.GetInstanceCustomData(cell);
            Vector3 position = positions[vi];
            float[] item = { position.X, position.Y, position.Z, uv[vi].X, uv[vi].Y, cell, 0, 0,
                model.Basis.X.X, model.Basis.X.Y, model.Basis.X.Z, 0,
                model.Basis.Y.X, model.Basis.Y.Y, model.Basis.Y.Z, 0,
                model.Basis.Z.X, model.Basis.Z.Y, model.Basis.Z.Z, 0,
                model.Origin.X, model.Origin.Y, model.Origin.Z, 1,
                custom.R, custom.G, custom.B, custom.A };
            Array.Copy(item, 0, inputs, i * 28, 28);
        }
        byte[] inputBytes = Floats(inputs);
        Rid input = target.Own(rd.StorageBufferCreate((uint)inputBytes.Length, inputBytes));
        float[] parameters = source.PackParameters(name => Value(view.Material.GetShaderParameter(name)));
        byte[] parameterBytes = Floats(parameters);
        Rid parameter = target.Own(rd.StorageBufferCreate((uint)parameterBytes.Length, parameterBytes));
        var bindings = new Godot.Collections.Array<RDUniform>
        {
            BufferUniform(0, input), BufferUniform(1, parameter)
        };
        using var samplerState = new RDSamplerState { MinFilter = RenderingDevice.SamplerFilter.Nearest, MagFilter = RenderingDevice.SamplerFilter.Nearest,
            RepeatU = RenderingDevice.SamplerRepeatMode.ClampToEdge, RepeatV = RenderingDevice.SamplerRepeatMode.ClampToEdge };
        Rid sampler = target.Own(rd.SamplerCreate(samplerState));
        foreach (ShaderUniform uniform in source.Uniforms.Where(u => u.Type == "sampler2D"))
        {
            var texture = (Texture2D)view.Material.GetShaderParameter(uniform.Name).AsGodotObject();
            using Image image = texture.GetImage();
            using var format = new RDTextureFormat { Width = (uint)image.GetWidth(), Height = (uint)image.GetHeight(),
                TextureType = RenderingDevice.TextureType.Type2D,
                Format = image.GetFormat() == Image.Format.Rf ? RenderingDevice.DataFormat.R32Sfloat : RenderingDevice.DataFormat.R32G32B32A32Sfloat,
                UsageBits = RenderingDevice.TextureUsageBits.SamplingBit };
            using var textureView = new RDTextureView();
            Rid uploaded = target.Own(rd.TextureCreate(format, textureView, new Godot.Collections.Array<byte[]> { image.GetData() }));
            var binding = new RDUniform { UniformType = RenderingDevice.UniformType.SamplerWithTexture, Binding = uniform.Binding };
            binding.AddId(sampler); binding.AddId(uploaded); bindings.Add(binding);
        }
        return bindings;
    }

    private static RDUniform BufferUniform(int binding, Rid buffer)
    {
        var uniform = new RDUniform { UniformType = RenderingDevice.UniformType.StorageBuffer, Binding = binding };
        uniform.AddId(buffer); return uniform;
    }

    private static float[] Value(Variant value) => value.VariantType switch
    {
        Variant.Type.Float => new[] { (float)value.AsDouble() },
        Variant.Type.Int => new[] { (float)value.AsInt32() },
        Variant.Type.Bool => new[] { value.AsBool() ? 1f : 0f },
        Variant.Type.Vector4 => Components(value.AsVector4()),
        Variant.Type.Color => new[] { value.AsColor().R, value.AsColor().G, value.AsColor().B, value.AsColor().A },
        Variant.Type.Projection => Components(value.AsProjection()),
        _ => throw new Exception("geometry: unbound or unsupported drawing uniform " + value.VariantType)
    };
    private static float[] Components(Vector4 v) => new[] { v.X, v.Y, v.Z, v.W };
    private static float[] Components(Godot.Projection p) => Components(p.X).Concat(Components(p.Y)).Concat(Components(p.Z)).Concat(Components(p.W)).ToArray();
}
