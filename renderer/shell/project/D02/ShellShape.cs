using System;
using System.IO;
using System.Text.Json;
using Godot;

namespace Magic600.Shell.D02;

/// <summary>
/// The view's shape behind M600_D02=1 (ADR-D02, host version 9 over the core's ABI 6): the owner's opening values
/// (26 Nov 2026: "the stickers must be smaller") from Look Lab's opening file, tools/looklab/data/opening.json, staged
/// as look/opening.json. Look Lab's gap is one minus the core's cell shrink; its sticker shrink is the core's. The
/// values live in that file, never in this code, so they stay tunable. Without M600_D02=1 nothing is called and every
/// frame keeps the core's default shape, byte for byte.
/// </summary>
public static class ShellShape
{
    /// <summary>The core's d4 (ABI 5's constant): the opening file has no d4, so the view keeps it.</summary>
    public const float D4 = 1.18f;

    /// <summary>
    /// The core's zoom for the chain projection (ABI 5's constant). The opening file's field of view (0.85) belongs to
    /// the Hopf-ring layout, which the core does not draw yet; Look Lab's zoom = 1/tan(fov/2) applies once it does.
    /// Until then the view keeps the chain projection's zoom.
    /// </summary>
    public const float Zoom = 1.15f;

    /// <summary>The opening file: under M600_D02_DATA when it holds one, else the build's staged copy.</summary>
    public static string OpeningPath()
    {
        if (System.Environment.GetEnvironmentVariable("M600_D02_DATA") is { Length: > 0 } data)
        {
            var root = data.EndsWith("samples", StringComparison.Ordinal) ? Path.GetDirectoryName(data)! : data;
            var file = Path.Combine(root, "opening.json");
            if (File.Exists(file))
            {
                return file;
            }
        }
        return ProjectSettings.GlobalizePath("res://look/opening.json");
    }

    /// <summary>The four floats the host takes (cell shrink, sticker shrink, d4, zoom) from an opening file.</summary>
    public static float[] Read(string path)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(path));
        var root = document.RootElement;
        if (root.GetProperty("format").GetString() != "magic600-look-opening")
        {
            throw new InvalidDataException($"{path} is not a Look Lab opening file");
        }
        var values = root.GetProperty("params");
        var gap = values.GetProperty("gap").GetSingle();
        var sticker = values.GetProperty("stickerShrink").GetSingle();
        return new[] { 1.0f - gap, sticker, D4, Zoom };
    }

    /// <summary>
    /// Sets the host's view to the opening shape when D02 is on; answers <see cref="HostLibrary.Ok"/> without a call
    /// when it is off. A value out of the core's range answers the host's refusal.
    /// </summary>
    public static int Apply(HostLibrary library, IntPtr host) =>
        ShellJumble.On ? Set(library, host, HostLibrary.View, Read(OpeningPath())) : HostLibrary.Ok;

    // Each view's opening shape, the one the wheel zoom scales, and the zoom last sent to the core.
    private static readonly System.Collections.Generic.Dictionary<ulong, float[]> Openings = new();
    private static readonly System.Collections.Generic.Dictionary<ulong, float> Sent = new();

    /// <summary>
    /// Sets a view's opening shape and remembers it as the shape the wheel zoom scales (Local's shape goes through
    /// here too, so its zoom scales Local's own). Answers the host's code.
    /// </summary>
    public static int Set(HostLibrary library, IntPtr host, ulong view, float[] shape)
    {
        var code = library.SetViewShape(host, view, shape);
        if (code == HostLibrary.Ok)
        {
            Openings[view] = (float[])shape.Clone();
            Sent[view] = shape[3];
        }
        return code;
    }

    /// <summary>
    /// Sends a view's opening shape with its zoom scaled by the wheel's factor (<see cref="ShellZoom"/>); a view with
    /// no opening shape keeps the core's and answers <see cref="HostLibrary.Ok"/> without a call.
    /// </summary>
    public static int ApplyZoom(HostLibrary library, IntPtr host, ulong view, double factor)
    {
        if (!Openings.TryGetValue(view, out var opening))
        {
            return HostLibrary.Ok;
        }
        var shape = (float[])opening.Clone();
        shape[3] = (float)(opening[3] * factor);
        var code = library.SetViewShape(host, view, shape);
        if (code == HostLibrary.Ok)
        {
            Sent[view] = shape[3];
        }
        return code;
    }

    /// <summary>The zoom last sent to the core for a view, or null before any.</summary>
    public static float? SentZoom(ulong view) => Sent.TryGetValue(view, out var zoom) ? zoom : null;
}
