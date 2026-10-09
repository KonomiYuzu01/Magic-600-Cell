using System.Linq;

namespace LookLab.App.LL5;

internal static class CvdProbeSource
{
    // Known linear colours enter the actual drawing fragment's sticker input.
    // No parallel CVD shader is authored for the probe.
    public static string Vertex(ShaderSource source, int width) => "#version 450\n" +
        "layout(set=0,binding=0,std430) readonly buffer Samples { vec4 sample_colours[]; };\n" +
        string.Join("\n", source.Varyings.Select(v => $"layout(location={v.Location}) out {v.Type} {v.Name};")) + "\n" +
        "void main() { int i=gl_VertexIndex/3; int corner=gl_VertexIndex%3; " +
        "vec2 offset=vec2(0.0,0.4); if(corner==0) offset=vec2(-0.4,-0.4); if(corner==1) offset=vec2(0.4,-0.4); " +
        $"vec2 ndc=vec2(2.0*(float(i)+0.5+offset.x)/{width}.0-1.0,offset.y*2.0); " +
        "gl_Position=vec4(ndc,0.0,1.0); projected=vec3(ndc,0.0); visible_w=0.0; barycentric=vec3(1.0); " +
        "sticker_colour=sample_colours[i].rgb; moving=0.0; geometry_value=vec3(0.0); }\n";
}
