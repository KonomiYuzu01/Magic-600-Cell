using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
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
        ShellJumble.BindingChanged += () => ShellJumble.Timed("tint", Bind);
        host.TintChanged += Draw;
        host.OverlayChanged += Draw;
        foreach (var layers in host.Layers)
        {
            layers.Changed += Draw;
        }
        frame.Rebuilt += Draw;
        // B30: a knob recolours the frame in place; the core gets the new palette, so the tinted pieces follow.
        frame.Recoloured += Draw;
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
        // B16 (ADR-D02.26): the overlay's lines on the outline channel over the tint, its hidden count with the focus.
        host.OverlayWords();
        ShellStyle.SetTint(JumbleOverlay.Outline(outline, host.Overlay.Lines), fill,
                           _frame is { } frame && GodotObject.IsInstanceValid(frame) ? Palette(frame.Tokens) : null);
        RefreshFocus();
    }

    public static void RefreshFocus()
    {
        if (_host is not { } host) return;
        var global = host.Layers.Count > 0 ? host.Layers[0] : null;
        var local = host.Layers.Count > 1 ? host.Layers[1] : null;
        var overlay = JumbleOverlay.BreaksThrough(host.Overlay);
        ShellStyle.SetFocusViews(global, local, host.FocusMember, piece => (overlay?.Invoke(piece) ?? false) || ShellStatus.Conflict(piece));
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
        // B16: the overlay's lines.
        var line = tokens.Colour(JumbleOverlay.Slot);
        palette[4 * JumbleOverlay.Entry] = (float)line.R;
        palette[4 * JumbleOverlay.Entry + 1] = (float)line.G;
        palette[4 * JumbleOverlay.Entry + 2] = (float)line.B;
        palette[4 * JumbleOverlay.Entry + 3] = 1;
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
        json.WriteString("outline_sha256", Sha(ShellStyle.SentOutline?.Select(value => (byte)(value & 0x0F)).ToArray()));
        json.WriteString("fill_sha256", Sha(ShellStyle.SentFill));
        json.WriteString("palette_sha256", ShellStyle.SentPalette is { } sent ? Sha(System.Runtime.InteropServices.MemoryMarshal.AsBytes(sent.AsSpan()).ToArray()) : "");
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
        Overlay(json, host, frame);
        ShellTurn.Dump(json);
        ShellSlice.Dump(json);
        ShellRaw.Dump(json);
    }

    /// <summary>
    /// B16's dump: the overlay's layer, cap and step, its pieces, never-hidden set and hidden count, the certificate
    /// points, its line on each border, and what the bytes last sent carry: the pieces with the overlay's entry on the
    /// outline channel, and the never-hidden or overlay pieces that a focus layer still dims or hides on the fill.
    /// </summary>
    private static void Overlay(Utf8JsonWriter json, JumbleHost host, ChromeFrame? frame)
    {
        var view = host.Overlay;
        json.WriteStartObject("overlay");
        json.WriteString("layer", view.Layer.ToString());
        if (view.Cap is int cap) json.WriteNumber("cap", cap); else json.WriteNull("cap");
        json.WriteString("step", view.Step);
        json.WriteNumber("pieces", view.Pieces);
        json.WriteNumber("never_hidden", view.NeverHiddenCount);
        json.WriteNumber("hidden", host.OverlayHidden);
        if (view.CertificatePoints is long points) json.WriteNumber("certificate_points", points); else json.WriteNull("certificate_points");
        json.WriteString("line", host.OverlayLine);
        var outline = ShellStyle.SentOutline;
        var fill = ShellStyle.SentFill;
        var keep = JumbleOverlay.BreaksThrough(view);
        long entries = 0, kept = 0, dimmed = 0;
        for (var piece = 0; piece < ShellStyle.PieceCount; piece++)
        {
            if (outline is not null && (outline[piece] & 0x0F) == JumbleOverlay.Entry) entries++;
            if (keep is not null && keep(piece))
            {
                kept++;
                if (fill is not null && (fill[piece] & HostLibrary.DimSet) != 0) dimmed++;
            }
        }
        json.WriteNumber("outline_entries", entries);
        json.WriteNumber("kept", kept);
        json.WriteNumber("kept_dimmed", dimmed);
        foreach (var (name, v) in new[] { ("global_line", frame?.Global), ("local_line", frame?.Local) })
        {
            var label = v is not null && GodotObject.IsInstanceValid(v) ? v.Readouts.GetNodeOrNull<Label>(JumbleOverlay.LineName) : null;
            json.WriteString(name, label is not null && label.IsVisibleInTree() ? label.Text : "");
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
