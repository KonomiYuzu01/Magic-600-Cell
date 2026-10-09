using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.Json;

namespace LookLab.Core;

internal static class Json
{
    public static FormatException Error(string id, string message) => new(id + ": " + message);

    public static JsonDocument Parse(string text)
    {
        try
        {
            JsonDocument document = JsonDocument.Parse(text);
            try { Unique(document.RootElement); }
            catch { document.Dispose(); throw; }
            return document;
        }
        catch (JsonException ex) { throw Error(ex.Path ?? SyntaxPath(text), ex.Message); }
    }

    private static string SyntaxPath(string text)
    {
        // JsonDocument does not supply a property path on lexical failures (e.g. NaN).
        var reader = new Utf8JsonReader(Encoding.UTF8.GetBytes(text));
        var containers = new List<string>();
        string? pending = null;
        string path = "json";
        try
        {
            while (reader.Read())
            {
                switch (reader.TokenType)
                {
                    case JsonTokenType.PropertyName:
                        pending = reader.GetString()!;
                        path = string.Join("/", containers.Where(c => c.Length > 0).Append(pending));
                        break;
                    case JsonTokenType.StartObject:
                    case JsonTokenType.StartArray:
                        containers.Add(pending ?? ""); pending = null; break;
                    case JsonTokenType.EndObject:
                    case JsonTokenType.EndArray:
                        containers.RemoveAt(containers.Count - 1); pending = null; break;
                    default: pending = null; break;
                }
            }
        }
        catch (JsonException) { }
        return path;
    }

    private static void Unique(JsonElement value)
    {
        if (value.ValueKind == JsonValueKind.Object)
        {
            var seen = new HashSet<string>(StringComparer.Ordinal);
            foreach (JsonProperty property in value.EnumerateObject())
            {
                if (!seen.Add(property.Name)) throw Error(property.Name, "duplicate key");
                Unique(property.Value);
            }
        }
        else if (value.ValueKind == JsonValueKind.Array)
            foreach (JsonElement item in value.EnumerateArray()) Unique(item);
    }

    public static void Keys(JsonElement value, params string[] names)
    {
        if (value.ValueKind != JsonValueKind.Object) throw Error("object", "expected object");
        foreach (JsonProperty property in value.EnumerateObject())
            if (!names.Contains(property.Name, StringComparer.Ordinal))
                throw Error(property.Name, "unknown key");
        foreach (string name in names) Get(value, name);
    }

    public static JsonElement Get(JsonElement value, string id)
    {
        if (value.ValueKind != JsonValueKind.Object || !value.TryGetProperty(id, out JsonElement found))
            throw Error(id, "missing key");
        return found;
    }

    public static string String(JsonElement value, string id)
    {
        if (value.ValueKind != JsonValueKind.String) throw Error(id, "expected string");
        return value.GetString()!;
    }

    public static string Text(JsonElement value, string id) => String(Get(value, id), id);

    public static double Number(JsonElement value, string id)
    {
        if (value.ValueKind != JsonValueKind.Number || !value.TryGetDouble(out double number) || !double.IsFinite(number))
            throw Error(id, "expected finite number");
        return number;
    }

    public static int Integer(JsonElement value, string id)
    {
        double number = Number(value, id);
        if (number != Math.Truncate(number) || number < int.MinValue || number > int.MaxValue)
            throw Error(id, "expected integer");
        return (int)number;
    }

    public static bool Boolean(JsonElement value, string id)
    {
        if (value.ValueKind != JsonValueKind.True && value.ValueKind != JsonValueKind.False)
            throw Error(id, "expected boolean");
        return value.GetBoolean();
    }

    public static JsonElement.ArrayEnumerator Array(JsonElement value, string id)
    {
        if (value.ValueKind != JsonValueKind.Array) throw Error(id, "expected array");
        return value.EnumerateArray();
    }

    public static void Header(JsonElement root, string format)
    {
        if (Text(root, "format") != format) throw Error("format", "expected " + format);
        if (Integer(Get(root, "version"), "version") != 1) throw Error("version", "only version 1 is supported");
    }
}
