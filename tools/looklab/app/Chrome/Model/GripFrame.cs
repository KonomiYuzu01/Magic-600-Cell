using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

public sealed record GripMark(int Index, string Label, int Corner, IReadOnlyList<double> Point);
public sealed record GripKey(string Chord, ulong Item, string Meaning);
public sealed record GripOrder(int Frame, IReadOnlyList<GripMark> Marks, IReadOnlyList<GripKey> Keys);

// ADR-D01.74/.75: keycaps and the views read the same order and its exact host bindings.
public sealed class GripFrame
{
    public const string SelectRow = "grip.select", NextRow = "grip.frame.next";
    public const string Modifier = "Ctrl";

    private IReadOnlyList<GripOrder> orders = Array.Empty<GripOrder>();
    private GripOrder? current;

    public int Cap { get; private set; }
    public int Frame => current?.Frame ?? 0;
    public bool Ready => current != null;
    public IReadOnlyList<GripMark> Marks => current?.Marks ?? Array.Empty<GripMark>();
    public IReadOnlyList<GripKey> KeyMeanings => current?.Keys ?? Array.Empty<GripKey>();
    public IReadOnlyList<int> AdmissibleFrames { get; private set; } = Array.Empty<int>();
    public int? NextFrame => orders.FirstOrDefault(o => o.Frame > Frame)?.Frame ?? orders.FirstOrDefault()?.Frame;

    public event Action? Changed;

    public GripKey? Key(string chord) => KeyMeanings.FirstOrDefault(key => key.Chord == chord);

    public bool Next() => NextFrame is int next && Set(next);
    public bool Set(int frame) => Set(Cap, frame, orders);

    public bool Set(int cap, int frame, IReadOnlyList<GripOrder> available)
    {
        if (available.FirstOrDefault(order => order.Frame == frame) is not { } chosen) return false;
        if (Ready && Cap == cap && Frame == frame) return false;
        orders = Array.AsReadOnly(available.OrderBy(order => order.Frame).ToArray());
        AdmissibleFrames = Array.AsReadOnly(orders.Select(order => order.Frame).ToArray());
        (Cap, current) = (cap, chosen);
        Changed?.Invoke();
        return true;
    }

    public bool Set(JsonElement report)
    {
        var cap = report.GetProperty("cap").GetInt32();
        var frame = report.GetProperty("frame").GetInt32();
        if (Ready && cap == Cap) return Set(frame);
        var available = report.GetProperty("frames").EnumerateArray()
            .Where(row => !row.TryGetProperty("reason", out _)).Select(ReadOrder).ToArray();
        return Set(cap, frame, Array.AsReadOnly(available));
    }

    private static GripOrder ReadOrder(JsonElement row)
    {
        var marks = row.GetProperty("marks").EnumerateArray().Select(mark => new GripMark(
            mark.GetProperty("index").GetInt32(), mark.GetProperty("label").GetString()!, mark.GetProperty("corner").GetInt32(),
            Array.AsReadOnly(mark.GetProperty("point").EnumerateArray().Select(q =>
                (q[0].GetDouble() + Math.Sqrt(5) * q[1].GetDouble()) / q[2].GetDouble()).ToArray()))).ToArray();
        var keys = row.GetProperty("keys").EnumerateArray().Select(key => new GripKey(
            key.GetProperty("chord").GetString()!, key.GetProperty("item").GetUInt64(), key.GetProperty("meaning").GetString()!)).ToArray();
        return new GripOrder(row.GetProperty("frame").GetInt32(), Array.AsReadOnly(marks), Array.AsReadOnly(keys));
    }

    public void Dump(Utf8JsonWriter json)
    {
        json.WriteNumber("cap", Cap);
        json.WriteNumber("frame", Frame);
        json.WriteStartArray("admissible_frames");
        foreach (int frame in AdmissibleFrames) json.WriteNumberValue(frame);
        json.WriteEndArray();
        json.WriteStartArray("marks");
        foreach (var mark in Marks)
        {
            json.WriteStartObject();
            json.WriteNumber("index", mark.Index);
            json.WriteString("label", mark.Label);
            json.WriteNumber("corner", mark.Corner);
            json.WriteEndObject();
        }
        json.WriteEndArray();
        json.WriteStartArray("keys");
        foreach (var key in KeyMeanings)
        {
            json.WriteStartObject();
            json.WriteString("chord", key.Chord);
            json.WriteNumber("item", key.Item);
            json.WriteString("meaning", key.Meaning);
            json.WriteEndObject();
        }
        json.WriteEndArray();
    }

    // D02 carries the existing selection row and adds a label-only frame step. Both use the draft row shape.
    public static IReadOnlyList<string> AddDraftRows(CommandRegistry registry, string text)
    {
        using var doc = JsonDocument.Parse(text);
        if (!doc.RootElement.TryGetProperty("grip", out var rows)) return Array.Empty<string>();
        var draft = CommandRegistry.FromTable(JsonSerializer.Serialize(new { commands = rows }));
        foreach (string id in draft.Order)
            if (id != SelectRow || !registry.Commands.ContainsKey(id)) registry.Add(draft.Commands[id]);
        return draft.Order;
    }
}
