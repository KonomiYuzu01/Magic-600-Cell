using System;
using System.Buffers.Binary;
using System.IO;
using System.Text.Json;
using Godot;

namespace Magic600.Shell.D02;

/// <summary>
/// The Hopf-ring opening behind M600_D02=1 (ADR-D02 build step B23, bare; host version 13 over the core's ABI 10):
/// when the owner's opening file names the "hopfRings" projection, Global opens on Look Lab's 20 Hopf rings of 30
/// cells. The ring data is Look Lab's tools/looklab/fixtures/hopf-rings.json, staged as look/hopf-rings.json; the core
/// checks it against its own cells and builds the ring charts (Look Lab's GlobalLayout). Nothing here is a constant of
/// the layout: the rings live in the file, the radii in the core, the field of view in the opening file.
/// </summary>
public static class ShellHopf
{
    /// <summary>The opening file's projection that asks for the Hopf rings.</summary>
    public const string Projection = "hopfRings";

    /// <summary>The opening file's projection that asks for the ring sheet (B22, bare): the same rings laid flat.</summary>
    public const string SheetProjection = "ringSheet";

    /// <summary>The layout kinds of sa2_view_layout: the projection chain, the Hopf rings and the ring sheet.</summary>
    public const uint Chain = 0, Rings = 1, Sheet = 2;

    /// <summary>The layout a projection name asks for; any other name keeps the chain.</summary>
    public static uint KindOf(string? projection) => projection switch
    {
        Projection => Rings,
        SheetProjection => Sheet,
        _ => Chain,
    };

    /// <summary>The ring file: under M600_D02_DATA when it holds one, else the build's staged copy.</summary>
    public static string RingsPath()
    {
        if (System.Environment.GetEnvironmentVariable("M600_D02_DATA") is { Length: > 0 } data)
        {
            var root = data.EndsWith("samples", StringComparison.Ordinal) ? Path.GetDirectoryName(data)! : data;
            var file = Path.Combine(root, "hopf-rings.json");
            if (File.Exists(file))
            {
                return file;
            }
        }
        return ProjectSettings.GlobalizePath("res://look/hopf-rings.json");
    }

    /// <summary>
    /// Whether the shell opens Global on a layout. The ring sheet is held back: on the owner's GPU its first frames
    /// removed the device (DXGI_ERROR_DEVICE_HUNG, 26 November 2026), though WARP draws it, so an opening file that
    /// names it keeps the chain until that is found.
    /// </summary>
    public static bool Opens(uint kind) => kind == Rings;

    /// <summary>The layout an opening file names: the Hopf rings, the ring sheet, or the chain.</summary>
    public static uint Kind(string openingPath)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(openingPath));
        return document.RootElement.GetProperty("params").TryGetProperty("projection", out var projection)
            ? KindOf(projection.GetString()) : Chain;
    }

    /// <summary>
    /// The sa2_view_layout row (<see cref="HostLibrary.LayoutBytes"/> bytes, little-endian) for a ring file: struct size,
    /// the kind (the Hopf rings or the ring sheet), J, K, the 20 origins, then each cell's ring and half-step.
    /// </summary>
    public static byte[] Layout(string path, uint kind)
    {
        if (kind != Rings && kind != Sheet)
        {
            throw new ArgumentOutOfRangeException(nameof(kind));
        }
        using var document = JsonDocument.Parse(File.ReadAllText(path));
        var root = document.RootElement;
        if (root.GetProperty("format").GetString() is not { } format || !format.Contains("hopf", StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidDataException($"{path} is not a Look Lab Hopf ring file");
        }
        var row = new byte[HostLibrary.LayoutBytes];
        BinaryPrimitives.WriteUInt32LittleEndian(row.AsSpan(0), (uint)HostLibrary.LayoutBytes);
        BinaryPrimitives.WriteUInt32LittleEndian(row.AsSpan(4), kind);
        void Floats(JsonElement values, int at, int count)
        {
            if (values.GetArrayLength() != count)
            {
                throw new InvalidDataException($"{path}: expected {count} numbers");
            }
            var i = 0;
            foreach (var value in values.EnumerateArray())
            {
                BinaryPrimitives.WriteSingleLittleEndian(row.AsSpan(at + 4 * i++), value.GetSingle());
            }
        }
        Floats(root.GetProperty("J"), 8, 4);
        Floats(root.GetProperty("K"), 24, 4);
        var rings = root.GetProperty("rings");
        if (rings.GetArrayLength() != 20)
        {
            throw new InvalidDataException($"{path}: expected 20 rings");
        }
        var r = 0;
        foreach (var ring in rings.EnumerateArray())
        {
            Floats(ring.GetProperty("origin"), 40 + 16 * r++, 4);
        }
        void Bytes(JsonElement values, int at)
        {
            if (values.GetArrayLength() != 600)
            {
                throw new InvalidDataException($"{path}: expected 600 cells");
            }
            var i = 0;
            foreach (var value in values.EnumerateArray())
            {
                row[at + i++] = checked((byte)value.GetInt32());
            }
        }
        Bytes(root.GetProperty("ringOf"), 360);
        Bytes(root.GetProperty("halfStep"), 960);
        return row;
    }
}
