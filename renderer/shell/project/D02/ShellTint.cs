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
/// each pane's focus membership and the overlay ink share the style bytes.
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
        if (System.Environment.GetEnvironmentVariable("M600_SESSION_DIR") is { Length: > 0 } session)
        {
            var path = Path.Combine(session, "jumble-contact-classes.json");
            try
            {
                if (File.Exists(path)) host.LoadClasses(File.ReadAllText(path));
            }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException or JsonException)
            {
                Safe.Report(error);
            }
            host.Settings.Changed += key =>
            {
                if (key != JumbleTint.MemoryKey || host.Settings.Get(key) is not { } saved) return;
                try
                {
                    File.WriteAllText(path + ".part", saved);
                    File.Move(path + ".part", path, overwrite: true);
                }
                catch (Exception error) when (error is IOException or UnauthorizedAccessException)
                {
                    Safe.Report(error);
                }
            };
        }
        ShellJumble.BindingChanged += () => ShellJumble.Timed("tint", Bind);
        host.TintChanged += Draw;
        host.OverlayChanged += Draw;
        host.PreviewMoving = () => ShellPreview.MovingAt((ulong)host.Source.State.Revision);
        host.FocusConflict = ShellStatus.Conflict;
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
        ShellStyle.SetTint(outline, fill,
                           _frame is { } frame && GodotObject.IsInstanceValid(frame) ? Palette(frame.Tokens) : null, host.Overlay);
        RefreshFocus();
    }

    public static void RefreshFocus()
    {
        if (_host is not { } host) return;
        var global = host.Layers.Count > 0 ? host.Layers[0] : null;
        var local = host.Layers.Count > 1 ? host.Layers[1] : null;
        host.OverlayWords();
        ShellStyle.SetFocusViews(global, local, host.FocusMember, ShellStatus.Conflict);
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
        var globalMode = ShellStyle.SentGlobalMode;
        var localMode = ShellStyle.SentLocalMode;
        json.WriteString("focus", Mode(globalMode != InactiveMode.Normal ? globalMode : localMode));
        json.WriteNumber("revision", ShellStyle.SentRevision);
        json.WriteStartArray("modes");
        json.WriteStringValue(Mode(globalMode));
        json.WriteStringValue(Mode(localMode));
        json.WriteEndArray();
        json.WriteStartArray("dims");
        json.WriteBooleanValue(globalMode != InactiveMode.Normal);
        json.WriteBooleanValue(localMode != InactiveMode.Normal);
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
    /// B16's dump: the overlay readouts and the reference and focus entries actually sent to the core.
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
        if (ShellStyle.SentCutCap is int trace) json.WriteNumber("trace_cap", trace); else json.WriteNull("trace_cap");
        var outline = ShellStyle.SentStyleOutline;
        var fill = ShellStyle.SentStyleFill;
        json.WriteString("fill_bytes_sha256", Sha(fill));
        var preview = ShellPreview.MovingAt(ShellStyle.SentRevision);
        long entries = 0, kept = 0, previewKept = 0, globalOnly = 0, localOnly = 0, candidates = 0;
        if (outline != null && fill != null)
        {
            for (var piece = 0; piece < fill.Length; piece++)
            {
                var (_, _, ink) = JumbleStyle.Decode(outline[piece], fill[piece], ShellStyle.SentHeight);
                if (ink == OverlayInk.Cut) candidates++;
                if (ink == OverlayInk.Outline) entries++;
                if (!JumbleStyle.Reference(ink)) continue;
                kept++;
                if (preview?[piece] > 0) previewKept++;
                if (JumbleStyle.OutlineOnly(ink, ShellStyle.SentGlobalMode, (fill[piece] & JumbleStyle.GlobalInactive) != 0)) globalOnly++;
                if (JumbleStyle.OutlineOnly(ink, ShellStyle.SentLocalMode, (fill[piece] & JumbleStyle.LocalInactive) != 0)) localOnly++;
            }
        }
        json.WriteNumber("trace_candidates", candidates);
        json.WriteNumber("outline_entries", entries);
        json.WriteNumber("kept", kept);
        json.WriteNumber("preview_kept", previewKept);
        json.WriteStartArray("outline_only");
        json.WriteNumberValue(globalOnly);
        json.WriteNumberValue(localOnly);
        json.WriteEndArray();
        foreach (var (name, v) in new[] { ("global_line", frame?.Global), ("local_line", frame?.Local) })
        {
            var label = v is not null && GodotObject.IsInstanceValid(v) ? v.Readouts.GetNodeOrNull<Label>(JumbleOverlay.LineName) : null;
            json.WriteString(name, label is not null && label.IsVisibleInTree() ? label.Text : "");
        }
        json.WriteEndObject();
    }

    private static string Mode(InactiveMode mode) => mode == InactiveMode.Normal ? "none" : mode.ToString().ToLowerInvariant();

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
