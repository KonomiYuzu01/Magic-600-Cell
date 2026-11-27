using System.Linq;
using System.Text.Json;
using Godot;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

/// <summary>ADR-D01.74 on ADR-D01.75's grip: the key map reads the grip the views draw, and the dump says what its
/// twist keycaps carry and where the board stamped them.</summary>
public static class ShellGripKeys
{
    private static KeyMapPanel? _map;

    public static void Attach(KeyMapPanel? map)
    {
        _map = map;
        if (map != null) map.Grip = ShellGrip.Model;
    }

    public static void Dump(Utf8JsonWriter json)
    {
        if (_map is not { } map || !GodotObject.IsInstanceValid(map)) return;
        json.WriteStartObject("keycaps");
        json.WriteBoolean("shown", map.Visible);
        json.WriteString("header", map.Header.Trim());
        json.WriteString("legend", map.LegendText);
        json.WriteStartArray("grip");
        foreach (var cap in map.Caps.SelectMany(row => row))
        {
            if (cap.Grip is not { } turn) continue;
            json.WriteStartObject();
            json.WriteString("key", cap.Key);
            json.WriteString("chord", turn.Chord);
            json.WriteNumber("item", turn.Item);
            json.WriteString("meaning", turn.Meaning);
            json.WriteString("label", cap.Label);
            json.WriteString("context", cap.Context);
            json.WriteBoolean("changed", cap.Changed);
            json.WriteEndObject();
        }
        json.WriteEndArray();
        json.WriteStartArray("stamps");
        foreach (var (key, turn, box, at) in map.GripStamps)
        {
            json.WriteStartObject();
            json.WriteString("key", key);
            json.WriteString("label", turn.Meaning);
            json.WriteNumber("x", at.X);
            json.WriteNumber("y", at.Y);
            json.WriteStartArray("box");
            json.WriteNumberValue(box.Position.X);
            json.WriteNumberValue(box.Position.Y);
            json.WriteNumberValue(box.Size.X);
            json.WriteNumberValue(box.Size.Y);
            json.WriteEndArray();
            json.WriteEndObject();
        }
        json.WriteEndArray();
        json.WriteEndObject();
    }
}
