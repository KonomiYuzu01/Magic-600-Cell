using System;
using System.IO;
using System.Text.Json;
using Godot;

namespace Magic600.Shell.D02;

/// <summary>
/// Local's own shape behind M600_D02=1 (ADR-D02 build step B18c, host version 12 over the core's ABI 9): Look Lab's
/// tools/looklab/data/local-view.json, staged as look/local-view.json. Local draws only the named cap's turning region,
/// centred on the cap's pole, pulled apart and orthographic, as Look Lab's LocalShape draws its focus cells. The values
/// live in that file, never in this code; only the cap's geometry is fixed, because it is the jumbled puzzle's.
/// </summary>
public static class ShellLocal
{
    /// <summary>
    /// Cap c of the jumbled puzzle is the half-space n_c·x ≥ κ with κ = (121/125)|n|² (rust/c600-jumble locality.rs),
    /// and the pole n_c is cell c's centre, so in the core's world, whose unit is the cell inradius, the cap is
    /// dot(x, pole) ≥ 121/125. The core draws a sticker in Local when its posed centre lies in the cap.
    /// </summary>
    public const float FocusCos = 121f / 125f;

    /// <summary>The 600-cell's circumradius in units of its inradius: √((96 − 32√5) / (12 + 4√5)).</summary>
    public static readonly double Circumradius = Math.Sqrt((96 - 32 * Math.Sqrt(5)) / (12 + 4 * Math.Sqrt(5)));

    /// <summary>Local's shape as local-view.json gives it.</summary>
    public sealed record Shape(uint Projection, float D4, float Gap, float StickerShrink, float Margin, float Fill, float Spread);

    /// <summary>The shape file: under M600_D02_DATA when it holds one, else the build's staged copy.</summary>
    public static string ShapePath()
    {
        if (System.Environment.GetEnvironmentVariable("M600_D02_DATA") is { Length: > 0 } data)
        {
            var root = data.EndsWith("samples", StringComparison.Ordinal) ? Path.GetDirectoryName(data)! : data;
            var file = Path.Combine(root, "local-view.json");
            if (File.Exists(file))
            {
                return file;
            }
        }
        return ProjectSettings.GlobalizePath("res://look/local-view.json");
    }

    /// <summary>Reads a local-view.json; the projection is perspective or orthographic, as the core draws.</summary>
    public static Shape Read(string path)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(path));
        var root = document.RootElement;
        if (root.GetProperty("format").GetString() != "magic600-look-local-view")
        {
            throw new InvalidDataException($"{path} is not a Look Lab local view file");
        }
        var projection = root.GetProperty("projection").GetString() switch
        {
            "orthographic" => HostLibrary.ProjectionOrthographic,
            "perspective" => HostLibrary.ProjectionPerspective,
            var other => throw new InvalidDataException($"{path}: the core draws no {other} projection"),
        };
        float Number(string name) => root.GetProperty(name).GetSingle();
        return new Shape(projection, Number("d4"), Number("gap"), Number("stickerShrink"), Number("margin"), Number("fill"), Number("spread"));
    }

    /// <summary>
    /// Look Lab's LocalShape.Frame zoom for the cap: the cap's widest angle (plus the margin) gives its edge in the
    /// projection, the spread widens it, and the fill keeps it inside the pane.
    /// </summary>
    public static double Zoom(Shape shape, double aspect)
    {
        double cap = Math.Min(Math.Acos(FocusCos / Circumradius) + shape.Margin, 1.45);
        double r = Circumradius;
        bool orthographic = shape.Projection == HostLibrary.ProjectionOrthographic;
        double edge = orthographic ? r * Math.Sin(cap) : shape.D4 * r * Math.Sin(cap) / (shape.D4 - r * Math.Cos(cap));
        double centre = orthographic ? 1 : shape.D4 / (shape.D4 - 1);
        double radius = Math.Min(edge * shape.Spread + .12 * centre, 4);
        return shape.Fill * Math.Sqrt(25 - radius * radius) / radius * Math.Min(1, aspect);
    }

    /// <summary>The four floats of Local's view shape: cell shrink (1 − gap), sticker shrink, d4 and zoom.</summary>
    public static float[] ViewShape(Shape shape, double aspect) =>
        new[] { 1 - shape.Gap, shape.StickerShrink, shape.D4, (float)Zoom(shape, aspect) };

    /// <summary>
    /// ADR-D01.41: Local re-frames on a selection change by default. Auto-framing can be turned off for Local (a
    /// script turns it off through the same switch); then Local follows only the named cap.
    /// </summary>
    public static bool AutoFrame { get; set; } = true;

    private static int? _cap, _named, _selected;
    private static bool _selectionNewer;

    /// <summary>The cap Local follows: the named cap, else the Jumble host's default (the D02 script player's dump).</summary>
    public static int? Cap => _cap;

    /// <summary>
    /// ADR-D02 B17 (ADR-D01.41): the cell Local frames, the newer of two sources. Naming a cap frames that cap; a later
    /// selection, while auto-framing is on, frames the cell its sticker occupies (none off the lattice). Clearing the
    /// selection, or naming another cap, returns Local to the named cap. Local is never empty (owner, 26 November
    /// 2026): with no cap named it follows the host's LocalCap while a state is bound, the cap named last in the
    /// session or, before any naming, the lowest-numbered cap; the default is drawn, never named.
    /// </summary>
    public static int? Frame(int? namedCap)
    {
        if (namedCap != _named)
        {
            _named = namedCap;
            _selectionNewer = false;
        }
        _cap = namedCap ?? ShellJumble.Host?.LocalCap.Shown;
        var picked = ShellPick.Selection?.Cell;
        if (picked != _selected)
        {
            _selected = picked;
            _selectionNewer = picked is not null;
        }
        return AutoFrame && _selectionNewer && _selected is int cell ? cell : _cap;
    }
}
