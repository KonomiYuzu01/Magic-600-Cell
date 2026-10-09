using System;
using System.Collections.Generic;
using Godot;

namespace LookLab.App;

// Shared RGBA32F draw/readback path for the minimal probe and drawing shader.
internal sealed class FloatTarget : IDisposable
{
    public RenderingDevice Device { get; }
    private readonly List<Rid> resources = new();

    public FloatTarget() => Device = RenderingServer.CreateLocalRenderingDevice()
        ?? throw new Exception("real rendering device unavailable");

    public Rid Own(Rid rid)
    {
        if (!rid.IsValid) throw new Exception("RGBA32F draw resource could not be created");
        resources.Add(rid); return rid;
    }

    public byte[] Draw(string vertex, string fragment, int width, uint vertexCount, Godot.Collections.Array<RDUniform>? uniforms = null)
    {
        using var format = new RDTextureFormat
        {
            Width = (uint)width, Height = 1, Format = RenderingDevice.DataFormat.R32G32B32A32Sfloat,
            TextureType = RenderingDevice.TextureType.Type2D,
            UsageBits = RenderingDevice.TextureUsageBits.ColorAttachmentBit | RenderingDevice.TextureUsageBits.CanCopyFromBit
        };
        using var view = new RDTextureView();
        Rid target = Own(Device.TextureCreate(format, view));
        Rid framebuffer = Own(Device.FramebufferCreate(new Godot.Collections.Array<Rid> { target }));
        using var source = new RDShaderSource { SourceVertex = vertex, SourceFragment = fragment };
        using RDShaderSpirV spirv = Device.ShaderCompileSpirVFromSource(source, false);
        foreach (RenderingDevice.ShaderStage stage in new[] { RenderingDevice.ShaderStage.Vertex, RenderingDevice.ShaderStage.Fragment })
            if (spirv.GetStageCompileError(stage).Length != 0) throw new Exception(spirv.GetStageCompileError(stage));
        Rid shader = Own(Device.ShaderCreateFromSpirV(spirv));
        using var blend = new RDPipelineColorBlendState { Attachments = new Godot.Collections.Array<RDPipelineColorBlendStateAttachment> { new() } };
        using var raster = new RDPipelineRasterizationState { CullMode = RenderingDevice.PolygonCullMode.Disabled };
        using var multisample = new RDPipelineMultisampleState();
        using var depth = new RDPipelineDepthStencilState();
        Rid pipeline = Own(Device.RenderPipelineCreate(shader, Device.FramebufferGetFormat(framebuffer), -1,
            RenderingDevice.RenderPrimitive.Triangles, raster, multisample, depth, blend));
        Rid bindings = uniforms == null ? default : Own(Device.UniformSetCreate(uniforms, shader, 0));
        long draw = Device.DrawListBegin(framebuffer, RenderingDevice.DrawFlags.ClearColorAll, new[] { new Color(-999, -999, -999, -999) });
        Device.DrawListBindRenderPipeline(draw, pipeline);
        if (uniforms != null) Device.DrawListBindUniformSet(draw, bindings, 0);
        Device.DrawListDraw(draw, false, 1, vertexCount);
        Device.DrawListEnd(); Device.Submit(); Device.Sync();
        byte[] result = Device.TextureGetData(target, 0);
        if (result.Length != width * 16) throw new Exception("RGBA32F readback has wrong length");
        return result;
    }

    public void Dispose()
    {
        for (int i = resources.Count - 1; i >= 0; i--) Device.FreeRid(resources[i]);
        Device.Dispose();
    }
}
