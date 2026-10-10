using System;
using System.Buffers.Binary;
using Godot;

namespace LookLab.App;

internal static class FloatProbe
{
    public static void Check(bool injectOffset)
    {
        using var target = new FloatTarget();
        byte[] result = target.Draw(
            "#version 450\nvoid main() { vec2 p=vec2((gl_VertexIndex << 1) & 2,gl_VertexIndex & 2); gl_Position=vec4(p*2.0-1.0,0.0,1.0); }",
            "#version 450\nlayout(location=0) out vec4 value;\n" + FileAccess.GetFileAsString("res://shaders/float_probe.gdshaderinc") +
            "\nvoid main() { value=looklab_float_probe(" + (injectOffset ? "0.125" : "0.0") + "); }", 1, 3);
        float[] expected = { .25f, .5f, .75f, 1f };
        for (int i = 0; i < 4; i++)
            if (BinaryPrimitives.ReadSingleLittleEndian(result.AsSpan(i * 4, 4)) != expected[i])
                throw new Exception("float-offset: exact RGBA32F readback differs");
        GD.Print("LOOKLAB_STAGE0_GPU_PASS values=0.25,0.5,0.75,1 target=RGBA32F");
    }
}
