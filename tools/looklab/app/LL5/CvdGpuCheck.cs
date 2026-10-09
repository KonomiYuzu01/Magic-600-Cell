using System;
using System.Buffers.Binary;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using Godot;
using LabColour = LookLab.Core.Colour;

namespace LookLab.App.LL5;

internal static class CvdGpuCheck
{
    public static async Task Run(Node host, LabApp app, CvdPreview preview, string fault)
    {
        Window window = host.GetWindow(); window.Mode = Window.ModeEnum.Windowed; window.Show();
        string display = DisplayServer.GetName(), driver = RenderingServer.GetCurrentRenderingDriverName().ToString(), adapter = RenderingServer.GetVideoAdapterName();
        GD.Print("LOOKLAB_GRAPHICS " + JsonSerializer.Serialize(new { display, driver, adapter }));
        if (display.ToLowerInvariant() != "windows" || driver is not ("vulkan" or "d3d12") || string.IsNullOrWhiteSpace(adapter))
            throw new Exception("ll5-graphics-environment: Windows display and real rendering driver required");
        await host.ToSignal(host.GetTree(), SceneTree.SignalName.ProcessFrame);
        if (!window.Visible || window.Mode == Window.ModeEnum.Minimized)
            throw new Exception("ll5-graphics-environment: the check window must remain visible");
        await host.ToSignal(RenderingServer.Singleton, RenderingServer.SignalName.FramePostDraw);

        ShaderMaterial material = app.View.Material;
        // Isolate the final colour path: unit diffuse, no specular, rim, edges,
        // fog or highlight. Presets and core results are left intact.
        material.SetShaderParameter("keyLight", 0f); material.SetShaderParameter("fillLight", 1f / .7f);
        foreach (string name in new[] { "gloss", "glow", "edgeWeight", "fog" }) material.SetShaderParameter(name, 0f);
        material.SetShaderParameter("finish_kind", 0); material.SetShaderParameter("highlight_kind", 0);
        material.SetShaderParameter("cellsVisible", true); material.SetShaderParameter("sliceVisible", false);
        material.SetShaderParameter("geometry_check", false);
        var source = new ShaderSource(Godot.FileAccess.GetFileAsString("res://shaders/cells.gdshader"));
        int samples = 0;
        foreach (string mode in LabColour.Modes)
        {
            preview.SelectMode(mode);
            if (fault == "ll5-wrong-matrix" && mode != "normal") material.SetShaderParameter("cvd_r0", Vector4.Zero);
            using var target = new FloatTarget();
            var bindings = Bindings(target, material, source);
            int width = CvdCheckData.LinearSamples.Length;
            byte[] result = target.Draw(CvdProbeSource.Vertex(source, width), source.Fragment(), width, (uint)width * 3, bindings);
            for (int i = 0; i < width; i++)
            {
                float[] actual = Enumerable.Range(0, 3).Select(component => BinaryPrimitives.ReadSingleLittleEndian(result.AsSpan(i * 16 + component * 4, 4))).ToArray();
                CvdCheckData.Require(actual, CvdCheckData.Reference(CvdCheckData.LinearSamples[i], mode),
                    fault == "ll5-wrong-matrix" ? fault : "ll5-cvd-gpu");
                if (BinaryPrimitives.ReadSingleLittleEndian(result.AsSpan(i * 16 + 12, 4)) != 1)
                    throw new Exception("ll5-cvd-gpu: colour sample was not drawn");
                samples++;
            }
        }
        GD.Print($"LOOKLAB_LL5_CVD_GPU_PASS modes={LabColour.Modes.Count} samples={samples} target=RGBA32F tolerance=2e-6-linear");
    }

    private static Godot.Collections.Array<RDUniform> Bindings(FloatTarget target, ShaderMaterial material, ShaderSource source)
    {
        RenderingDevice rd = target.Device;
        float[] samples = CvdCheckData.LinearSamples.SelectMany(rgb => new[] { (float)rgb[0], (float)rgb[1], (float)rgb[2], 1f }).ToArray();
        byte[] sampleBytes = Bytes(samples), parameterBytes = Bytes(source.PackParameters(name => Value(material.GetShaderParameter(name))));
        Rid inputs = target.Own(rd.StorageBufferCreate((uint)sampleBytes.Length, sampleBytes));
        Rid parameters = target.Own(rd.StorageBufferCreate((uint)parameterBytes.Length, parameterBytes));
        var bindings = new Godot.Collections.Array<RDUniform> { Buffer(0, inputs), Buffer(1, parameters) };
        using var state = new RDSamplerState { MinFilter = RenderingDevice.SamplerFilter.Nearest, MagFilter = RenderingDevice.SamplerFilter.Nearest,
            RepeatU = RenderingDevice.SamplerRepeatMode.ClampToEdge, RepeatV = RenderingDevice.SamplerRepeatMode.ClampToEdge };
        Rid sampler = target.Own(rd.SamplerCreate(state));
        foreach (ShaderUniform uniform in source.Uniforms.Where(u => u.Type == "sampler2D"))
        {
            using Image image = ((Texture2D)material.GetShaderParameter(uniform.Name).AsGodotObject()).GetImage();
            using var format = new RDTextureFormat { Width = (uint)image.GetWidth(), Height = (uint)image.GetHeight(), TextureType = RenderingDevice.TextureType.Type2D,
                Format = image.GetFormat() == Image.Format.Rf ? RenderingDevice.DataFormat.R32Sfloat : RenderingDevice.DataFormat.R32G32B32A32Sfloat,
                UsageBits = RenderingDevice.TextureUsageBits.SamplingBit };
            using var view = new RDTextureView();
            Rid texture = target.Own(rd.TextureCreate(format, view, new Godot.Collections.Array<byte[]> { image.GetData() }));
            var binding = new RDUniform { UniformType = RenderingDevice.UniformType.SamplerWithTexture, Binding = uniform.Binding };
            binding.AddId(sampler); binding.AddId(texture); bindings.Add(binding);
        }
        return bindings;
    }

    private static RDUniform Buffer(int binding, Rid buffer)
    {
        var uniform = new RDUniform { UniformType = RenderingDevice.UniformType.StorageBuffer, Binding = binding }; uniform.AddId(buffer); return uniform;
    }

    private static byte[] Bytes(float[] values)
    {
        var bytes = new byte[values.Length * 4];
        for (int i = 0; i < values.Length; i++) BinaryPrimitives.WriteSingleLittleEndian(bytes.AsSpan(i * 4, 4), values[i]);
        return bytes;
    }

    private static float[] Components(Vector4 v) => new[] { v.X, v.Y, v.Z, v.W };
    private static float[] Value(Variant value) => value.VariantType switch
    {
        Variant.Type.Float => new[] { (float)value.AsDouble() }, Variant.Type.Int => new[] { (float)value.AsInt32() },
        Variant.Type.Bool => new[] { value.AsBool() ? 1f : 0f }, Variant.Type.Vector4 => Components(value.AsVector4()),
        Variant.Type.Color => new[] { value.AsColor().R, value.AsColor().G, value.AsColor().B, value.AsColor().A },
        Variant.Type.Projection => Components(value.AsProjection().X).Concat(Components(value.AsProjection().Y))
            .Concat(Components(value.AsProjection().Z)).Concat(Components(value.AsProjection().W)).ToArray(),
        _ => throw new Exception("ll5-cvd-gpu: unbound or unsupported shader uniform " + value.VariantType)
    };
}
