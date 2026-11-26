using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;
using System.Text.Json;
using Godot;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

/// <summary>
/// B15 (ADR-D02.20, .21, .23, .24, .30): the Jumble host's tint channel and focus layers, drawn by the core through
/// <see cref="ShellStyle"/>. At each binding it gives the host the bound sample state's contact graph and pose ids;
/// whenever the mode, the classes, the named cap, the theme or a view's focus layers change it sends the per-piece
/// bytes again: the contact classes and the off-lattice mark on the outline channel, the height ramp on the fill with
/// the focus layer's dim bit. Both views draw the same bytes (Global in style slot 1, Local in slot 2).
/// </summary>
public static class ShellTint
{
    private static JumbleHost? _host;
    private static ChromeFrame? _frame;

    /// <summary>Follows the host from now on; the host's focus layers are set before this.</summary>
    public static void Attach(JumbleHost host, ChromeFrame frame)
    {
        _host = host;
        _frame = frame;
        ShellJumble.BindingChanged += Bind;
        host.TintChanged += Draw;
        foreach (var layers in host.Layers)
        {
            layers.Changed += Draw;
        }
        frame.Rebuilt += Draw;
        Bind();
    }

    // The bound state's contact graph and pose ids (the host has its pose rows already); a file that does not read
    // binds none and is reported.
    private static void Bind()
    {
        if (_host is not { } host)
        {
            return;
        }
        ContactGraph? contact = null;
        (int[] Poses, string Digest)? poses = null;
        try
        {
            contact = ShellJumble.Contact();
            poses = host.Style is { } style ? ShellJumble.Poses(style.RowCount) : null;
        }
        catch (Exception error) when (error is FormatException or JsonException or IOException or KeyNotFoundException
                                          or InvalidOperationException or InvalidDataException)
        {
            Safe.Report(error);
        }
        host.SetContact(contact, poses?.Poses, poses?.Digest ?? "");
    }

    private static void Draw()
    {
        if (_host is not { } host)
        {
            return;
        }
        var (outline, fill) = host.TintBytes();
        ShellStyle.SetTint(outline, fill, _frame is { } frame && GodotObject.IsInstanceValid(frame) ? Palette(frame.Tokens) : null);
        var global = host.Layers.Count > 0 ? host.Layers[0] : null;
        var local = host.Layers.Count > 1 ? host.Layers[1] : null;
        ShellStyle.SetFocusViews(global, local, host.FocusMember);
    }

    /// <summary>The core's 16-entry palette with the tint's entries 3 to 14 in the theme's colours.</summary>
    public static float[] Palette(TokenSet tokens)
    {
        var palette = new float[64];
        foreach (var (entry, colour, alpha) in JumbleTint.Palette(tokens))
        {
            palette[4 * entry] = (float)colour.R;
            palette[4 * entry + 1] = (float)colour.G;
            palette[4 * entry + 2] = (float)colour.B;
            palette[4 * entry + 3] = (float)alpha;
        }
        return palette;
    }

    /// <summary>
    /// The dump's tint context: the mode, the legend's line on each border, the classes and the moved domains, the
    /// digests of the bytes last sent on each channel with the hover and selection they carry, and the focus dim.
    /// </summary>
    public static void Dump(Utf8JsonWriter json, ChromeFrame? frame)
    {
        if (_host is not { } host)
        {
            return;
        }
        json.WriteStartObject("tint");
        json.WriteString("mode", JumbleTint.Word(host.Tint));
        json.WriteString("line", host.TintLine);
        json.WriteString("bound", ShellJumble.Bound ?? "");
        json.WriteStartArray("classes");
        foreach (var c in host.Classes ?? Array.Empty<int>())
        {
            json.WriteNumberValue(c);
        }
        json.WriteEndArray();
        json.WriteStartArray("moved");
        foreach (var d in host.ClassMemory.Moved)
        {
            json.WriteNumberValue(d);
        }
        json.WriteEndArray();
        json.WriteNumber("remembered", host.ClassMemory.Memory.Count);
        json.WriteString("outline_sha256", Sha(ShellStyle.SentOutline));
        json.WriteString("fill_sha256", Sha(ShellStyle.SentFill));
        Piece(json, "hover", ShellStyle.SentHover);
        Piece(json, "selection", ShellStyle.SentSelection);
        json.WriteString("focus", ShellStyle.FocusMode);
        json.WriteStartArray("dims");
        json.WriteBooleanValue(ShellStyle.SlotDims(1));
        json.WriteBooleanValue(ShellStyle.SlotDims(2));
        json.WriteEndArray();
        foreach (var (name, view) in new[] { ("global_legend", frame?.Global), ("local_legend", frame?.Local) })
        {
            var row = view is not null && GodotObject.IsInstanceValid(view) ? view.Readouts.GetNodeOrNull<TintLegendRow>(TintLegendRow.NodeName) : null;
            json.WriteString(name, row is not null && row.IsVisibleInTree() ? row.Line : "");
        }
        json.WriteEndObject();
    }

    private static void Piece(Utf8JsonWriter json, string name, int? piece)
    {
        if (piece is int value)
        {
            json.WriteNumber(name, value);
        }
        else
        {
            json.WriteNull(name);
        }
    }

    private static string Sha(byte[]? bytes) => bytes is null ? "" : Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
}
