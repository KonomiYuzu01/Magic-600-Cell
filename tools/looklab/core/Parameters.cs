using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using System.Text.Json;

namespace LookLab.Core;

public enum ParameterKind { Number, Integer, Enum, Boolean, Colour, Curve }
public readonly record struct OklchColour(double L, double C, double H);
public readonly record struct BezierCurve(double A, double B);
public abstract record ParameterValue;
public sealed record NumberValue(double Value) : ParameterValue;
public sealed record IntegerValue(int Value) : ParameterValue;
public sealed record EnumValue(string Value) : ParameterValue;
public sealed record BooleanValue(bool Value) : ParameterValue;
public sealed record ColourValue(OklchColour Value) : ParameterValue;
public sealed record CurveValue(BezierCurve Value) : ParameterValue;

public sealed class ParameterSet
{
    public IReadOnlyDictionary<string, ParameterValue> Values { get; }
    public ParameterSet(IEnumerable<KeyValuePair<string, ParameterValue>> values) =>
        Values = new ReadOnlyDictionary<string, ParameterValue>(values.ToDictionary(p => p.Key, p => p.Value, StringComparer.Ordinal));
    public double Number(string id) => Values[id] is NumberValue value ? value.Value : throw Json.Error(id, "not a number");
    public int Integer(string id) => Values[id] is IntegerValue value ? value.Value : throw Json.Error(id, "not an integer");
    public string Choice(string id) => Values[id] is EnumValue value ? value.Value : throw Json.Error(id, "not an enum");
    public bool Boolean(string id) => Values[id] is BooleanValue value ? value.Value : throw Json.Error(id, "not a boolean");
    public OklchColour Colour(string id) => Values[id] is ColourValue value ? value.Value : throw Json.Error(id, "not a colour");
    public BezierCurve Curve(string id) => Values[id] is CurveValue value ? value.Value : throw Json.Error(id, "not a curve");
    public ParameterSet With(string id, ParameterValue value)
    {
        var copy = Values.ToDictionary(p => p.Key, p => p.Value, StringComparer.Ordinal);
        copy[id] = value;
        return new ParameterSet(copy);
    }

    public StructureValues Structure(double aspect) => new(1 - Number("gap"), Number("stickerShrink"), Number("d4"),
        1 / Math.Tan(Number("fieldOfView") / 2), aspect, Choice("projection") switch
        {
            "perspective" => ProjectionKind.Perspective, "stereographic" => ProjectionKind.Stereographic,
            "orthographic" => ProjectionKind.Orthographic, _ => throw Json.Error("projection", "unknown projection")
        });
}

public sealed record ParameterDefinition(string Id, string Group, ParameterKind Kind, double? Min, double? Max,
    double? Step, IReadOnlyList<string>? Options, ParameterValue Default, string? CostFeature, string Description);

public sealed class ParameterSchema
{
    public static readonly IReadOnlyList<string> Groups = Array.AsReadOnly(new[] { "Structure", "Colour", "Material", "Light and depth", "Motion", "Frame", "Layout" });
    public static readonly IReadOnlyList<string> TasteIds = Array.AsReadOnly(new[] { "hueRotation", "hueSpread", "lightness", "lightnessAlt", "chroma", "classes", "bgLightness", "bgHue", "bgTint", "gap", "edgeWeight", "edgeBrightness", "gloss", "glow", "fog", "turnMs", "easeA", "easeB" });
    public IReadOnlyList<ParameterDefinition> Parameters { get; }
    private readonly Dictionary<string, ParameterDefinition> byId;
    public ParameterDefinition this[string id] => byId.TryGetValue(id, out var parameter) ? parameter : throw Json.Error(id, "unknown id");
    public ParameterSet Defaults => new(Parameters.Select(p => new KeyValuePair<string, ParameterValue>(p.Id, p.Default)));

    private ParameterSchema(List<ParameterDefinition> parameters)
    {
        Parameters = parameters.AsReadOnly();
        byId = parameters.ToDictionary(p => p.Id, StringComparer.Ordinal);
    }

    public static ParameterSchema Load(string path) => Parse(File.ReadAllText(path));

    public static ParameterSchema Parse(string text)
    {
        using var document = Json.Parse(text);
        var root = document.RootElement;
        Json.Keys(root, "format", "version", "parameters");
        Json.Header(root, "magic600-look-parameters");
        var parameters = new List<ParameterDefinition>();
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonElement p in Json.Array(Json.Get(root, "parameters"), "parameters"))
        {
            string id = Json.Text(p, "id");
            if (string.IsNullOrWhiteSpace(id) || !seen.Add(id)) throw Json.Error(id, "empty or duplicate parameter id");
            string group = Json.Text(p, "group");
            if (!Groups.Contains(group)) throw Json.Error(id, "unknown group " + group);
            ParameterKind kind = Json.Text(p, "kind") switch
            {
                "number" => ParameterKind.Number, "integer" => ParameterKind.Integer, "enum" => ParameterKind.Enum,
                "boolean" => ParameterKind.Boolean, "colour" => ParameterKind.Colour, "curve" => ParameterKind.Curve,
                _ => throw Json.Error(id, "unknown kind")
            };
            string[] required = { "id", "group", "kind", "default", "costFeature", "description" };
            string[] extras = kind is ParameterKind.Number or ParameterKind.Integer ? new[] { "min", "max", "step" }
                : kind == ParameterKind.Enum ? new[] { "values" } : Array.Empty<string>();
            try { Json.Keys(p, required.Concat(extras).ToArray()); }
            catch (FormatException ex) { throw Json.Error(id, ex.Message); }
            double? min = null, max = null, step = null;
            IReadOnlyList<string>? options = null;
            if (kind is ParameterKind.Number or ParameterKind.Integer)
            {
                min = Json.Number(Json.Get(p, "min"), id); max = Json.Number(Json.Get(p, "max"), id); step = Json.Number(Json.Get(p, "step"), id);
                if (min >= max || step <= 0 || step > max - min || (kind == ParameterKind.Integer &&
                    (min != Math.Truncate(min.Value) || max != Math.Truncate(max.Value) || step != Math.Truncate(step.Value) ||
                     min < int.MinValue || max > int.MaxValue))) throw Json.Error(id, "invalid range or step");
            }
            if (kind == ParameterKind.Enum)
            {
                string[] values = Json.Array(Json.Get(p, "values"), id).Select(v => Json.String(v, id)).ToArray();
                if (values.Length == 0 || values.Any(string.IsNullOrWhiteSpace) || values.Distinct().Count() != values.Length)
                    throw Json.Error(id, "nonempty unique enum values required");
                options = Array.AsReadOnly(values);
            }
            JsonElement cost = Json.Get(p, "costFeature");
            string? feature = cost.ValueKind == JsonValueKind.Null ? null : Json.String(cost, id);
            if (feature != null && string.IsNullOrWhiteSpace(feature)) throw Json.Error(id, "empty costFeature");
            string description = Json.Text(p, "description");
            if (string.IsNullOrWhiteSpace(description) || description.Contains('\n') || description.Contains('\r')) throw Json.Error(id, "one-line description required");
            var definition = new ParameterDefinition(id, group, kind, min, max, step, options, new BooleanValue(false), feature, description);
            definition = definition with { Default = ReadValue(definition, Json.Get(p, "default")) };
            parameters.Add(definition);
        }
        if (parameters.Count == 0) throw Json.Error("parameters", "at least one parameter required");
        return new ParameterSchema(parameters);
    }

    public static ParameterValue ReadValue(ParameterDefinition p, JsonElement element)
    {
        try
        {
            ParameterValue value = p.Kind switch
            {
                ParameterKind.Number => new NumberValue(Json.Number(element, p.Id)),
                ParameterKind.Integer => new IntegerValue(Json.Integer(element, p.Id)),
                ParameterKind.Enum => new EnumValue(Json.String(element, p.Id)),
                ParameterKind.Boolean => new BooleanValue(Json.Boolean(element, p.Id)),
                ParameterKind.Colour => ReadColour(element, p.Id),
                ParameterKind.Curve => ReadCurve(element, p.Id),
                _ => throw Json.Error(p.Id, "unsupported kind")
            };
            ValidateValue(p, value);
            return value;
        }
        catch (FormatException ex) { throw Json.Error(p.Id, ex.Message); }
    }

    private static ColourValue ReadColour(JsonElement value, string id)
    {
        Json.Keys(value, "L", "C", "h");
        return new ColourValue(new OklchColour(Json.Number(Json.Get(value, "L"), id), Json.Number(Json.Get(value, "C"), id), Json.Number(Json.Get(value, "h"), id)));
    }

    private static CurveValue ReadCurve(JsonElement value, string id)
    {
        Json.Keys(value, "a", "b");
        return new CurveValue(new BezierCurve(Json.Number(Json.Get(value, "a"), id), Json.Number(Json.Get(value, "b"), id)));
    }

    public static void ValidateValue(ParameterDefinition p, ParameterValue value)
    {
        bool valid = (p.Kind, value) switch
        {
            (ParameterKind.Number, NumberValue n) => double.IsFinite(n.Value) && n.Value >= p.Min && n.Value <= p.Max,
            (ParameterKind.Integer, IntegerValue n) => n.Value >= p.Min && n.Value <= p.Max,
            (ParameterKind.Enum, EnumValue e) => p.Options!.Contains(e.Value, StringComparer.Ordinal),
            (ParameterKind.Boolean, BooleanValue) => true,
            (ParameterKind.Colour, ColourValue c) => double.IsFinite(c.Value.L) && double.IsFinite(c.Value.C) && double.IsFinite(c.Value.H) &&
                c.Value.L >= 0 && c.Value.L <= 1 && c.Value.C >= 0 && c.Value.C <= 1 && c.Value.H >= 0 && c.Value.H <= 360,
            (ParameterKind.Curve, CurveValue c) => double.IsFinite(c.Value.A) && double.IsFinite(c.Value.B) &&
                c.Value.A >= 0 && c.Value.A <= 1 && c.Value.B >= 0 && c.Value.B <= 1,
            _ => false
        };
        if (!valid) throw Json.Error(p.Id, "wrong type, non-finite or out-of-range value");
    }

    public void Validate(ParameterSet values)
    {
        foreach (string id in values.Values.Keys) if (!byId.ContainsKey(id)) throw Json.Error(id, "unknown id");
        foreach (ParameterDefinition p in Parameters)
        {
            if (!values.Values.TryGetValue(p.Id, out ParameterValue? value)) throw Json.Error(p.Id, "missing id");
            ValidateValue(p, value);
        }
    }
}
