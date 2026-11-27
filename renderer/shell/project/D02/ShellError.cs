using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using Godot;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

/// <summary>ADR-D01.25's shared Global and Local error overlay, drawn through the D02 outline channel.</summary>
public static class ShellError
{
    public const float Alpha = 0.8f;
    private static ChromeFrame? _frame;
    private static ErrorOverlay? _model;
    private static readonly float[]?[] _sent = new float[3][];

    public static IReadOnlyList<int> Pieces => _model?.Pieces ?? Array.Empty<int>();
    public static Rgb? Colour => _frame is { } frame && GodotObject.IsInstanceValid(frame) ? frame.Tokens.Colour("severity.error") : null;

    public static void Attach(JumbleHost host, ChromeFrame frame)
    {
        _frame = frame;
        var model = _model = new ErrorOverlay(ShellStyle.PieceCount);
        Array.Clear(_sent);
        model.Changed += ShellStyle.Invalidate;
        frame.Rebuilt += ShellStyle.Invalidate;
        frame.Recoloured += ShellStyle.Invalidate;
        host.BindErrors(model, ShellResearch.Model);
        ShellStyle.Invalidate();
    }

    public static unsafe void Sent(uint slot, HostLibrary.ViewStyle view)
    {
        _sent[slot - 1] = new[] { view.Palette[0], view.Palette[1], view.Palette[2], view.Palette[3] };
    }

    public static void Dump(Utf8JsonWriter json)
    {
        if (_model is not { } model) return;
        json.WriteStartObject("error_overlay");
        model.Dump(json);
        json.WriteNumber("outline_bits", ShellStyle.SentOutline?.Count(b => (b & HostLibrary.StyleError) != 0) ?? 0);
        json.WriteStartObject("palette");
        for (int i = 0; i < _sent.Length; i++)
        {
            json.WriteStartArray((i + 1).ToString(System.Globalization.CultureInfo.InvariantCulture));
            foreach (float channel in _sent[i] ?? Array.Empty<float>()) json.WriteNumberValue(channel);
            json.WriteEndArray();
        }
        json.WriteEndObject();
        json.WriteEndObject();
    }
}
