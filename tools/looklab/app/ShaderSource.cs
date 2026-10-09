using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;

namespace LookLab.App;

// A narrow adapter for cells.gdshader, not a second projection implementation.
// Both authored function bodies are copied byte for byte. Only their Godot
// builtins, uniform bindings and stage IO are supplied for RenderingDevice.
internal sealed record ShaderUniform(string Type, string Name, int Offset, int Binding);
internal sealed record ShaderVarying(string Type, string Name, int Location);

internal sealed class ShaderSource
{
    public IReadOnlyList<ShaderUniform> Uniforms { get; }
    public IReadOnlyList<ShaderVarying> Varyings { get; }
    public string VertexBody { get; }
    public string FragmentBody { get; }
    public int ParameterVectors { get; }

    public ShaderSource(string drawingShader)
    {
        VertexBody = Function(drawingShader, "vertex");
        FragmentBody = Function(drawingShader, "fragment");
        var uniforms = new List<ShaderUniform>(); int offset = 0, binding = 2;
        foreach (Match match in Regex.Matches(drawingShader, @"(?m)^uniform (\w+) (\w+)[^;]*;"))
        {
            string type = match.Groups[1].Value, name = match.Groups[2].Value;
            if (!new[] { "sampler2D", "float", "int", "bool", "vec4", "mat4" }.Contains(type))
                throw new FormatException("unsupported drawing uniform: " + type);
            uniforms.Add(new ShaderUniform(type, name, type == "sampler2D" ? -1 : offset, type == "sampler2D" ? binding++ : -1));
            if (type != "sampler2D") offset += type == "mat4" ? 4 : 1;
        }
        Uniforms = uniforms.AsReadOnly(); ParameterVectors = offset;
        Varyings = Regex.Matches(drawingShader, @"(?m)^varying (\w+) (\w+);")
            .Select((match, index) => new ShaderVarying(match.Groups[1].Value, match.Groups[2].Value, index)).ToArray();
        if (!Uniforms.Any(u => u.Name == "geometry_check") || !Uniforms.Any(u => u.Name == "transpose_q") ||
            !Varyings.Any(v => v.Name == "geometry_value")) throw new FormatException("drawing shader geometry mode is missing");
    }

    private static string Function(string source, string name)
    {
        Match start = Regex.Match(source, @"\bvoid " + name + @"\(\)\s*\{");
        if (!start.Success) throw new FormatException("drawing shader has no " + name + " function");
        int depth = 1, i = start.Index + start.Length;
        for (; i < source.Length && depth != 0; i++)
        {
            if (source[i] == '{') depth++;
            if (source[i] == '}') depth--;
        }
        if (depth != 0) throw new FormatException("unbalanced drawing shader " + name);
        return source[start.Index..i];
    }

    private string Bindings()
    {
        var text = new StringBuilder("layout(set=0,binding=1,std430) readonly buffer Parameters { vec4 parameter_data[]; };\n");
        foreach (ShaderUniform u in Uniforms)
        {
            string value = $"parameter_data[{u.Offset}]";
            if (u.Type == "sampler2D")
                text.Append($"layout(set=0,binding={u.Binding}) uniform sampler2D {u.Name};\n");
            else
            {
                value = u.Type switch
                {
                    "float" => value + ".x", "int" => "int(" + value + ".x)", "bool" => "(" + value + ".x != 0.0)",
                    "mat4" => $"mat4(parameter_data[{u.Offset}],parameter_data[{u.Offset + 1}],parameter_data[{u.Offset + 2}],parameter_data[{u.Offset + 3}])",
                    _ => value
                };
                text.Append($"#define {u.Name} {value}\n");
            }
        }
        return text.ToString();
    }

    private string StageVaryings(string direction) => string.Join("\n", Varyings.Select(v =>
        $"layout(location={v.Location}) {direction} {v.Type} {v.Name};")) + "\n";

    public string Vertex() => "#version 450\n" + Bindings() + StageVaryings("out") +
        "struct Sample { vec4 position; vec4 meta; mat4 model; vec4 custom_data; };\n" +
        "layout(set=0,binding=0,std430) readonly buffer Samples { Sample sample_data[]; };\n" +
        "vec3 VERTEX; vec2 UV; vec2 UV2; int INSTANCE_ID; vec4 INSTANCE_CUSTOM; mat4 MODEL_MATRIX; vec4 POSITION;\n" +
        VertexBody + "\nvoid main() { int i=gl_VertexIndex/3; Sample sample_value=sample_data[i]; " +
        "VERTEX=sample_value.position.xyz; UV=vec2(sample_value.position.w,sample_value.meta.x); " +
        "UV2=vec2(float(i),float(gl_VertexIndex%3)); INSTANCE_ID=int(sample_value.meta.y); " +
        "MODEL_MATRIX=sample_value.model; INSTANCE_CUSTOM=sample_value.custom_data; vertex(); gl_Position=POSITION; }\n";

    public string Fragment() => "#version 450\n" + Bindings() + StageVaryings("in") +
        "vec3 ALBEDO; layout(location=0) out vec4 output_value;\n" + FragmentBody +
        "\nvoid main() { fragment(); output_value=vec4(ALBEDO,1.0); }\n";

    public float[] PackParameters(Func<string, float[]> value)
    {
        var data = new float[ParameterVectors * 4];
        foreach (ShaderUniform u in Uniforms.Where(u => u.Type != "sampler2D"))
        {
            float[] input = value(u.Name);
            int count = u.Type == "mat4" ? 16 : u.Type == "vec4" ? 4 : 1;
            if (input.Length != count || input.Any(v => !float.IsFinite(v))) throw new FormatException("wrong shader uniform: " + u.Name);
            Array.Copy(input, 0, data, u.Offset * 4, count);
        }
        return data;
    }
}
