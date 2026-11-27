using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

namespace Magic600.Chrome;

// A view as the product's core draws it (renderer/core/shaders/geometry.hlsl, projectWorld and focusVertex): the 4D
// camera Q (row-major, x' = Q x, as the shell's camera keeps it), the view's shape (cell shrink, sticker shrink, d4,
// zoom), the pane's aspect, the projection and, for a focused view (Local), the focus cell's unit centre, the cap's
// cosine and the spread. A view under the Hopf rings has no place here (the chart is not ported).
public sealed record CoreView(IReadOnlyList<double> Q, double CellShrink, double StickerShrink, double D4, double Zoom, double Aspect,
    bool Orthographic = false, IReadOnlyList<double>? FocusPole = null, double FocusCos = -2, double Spread = 1)
{
    // The core's shape until a host sets one (sa2_interop.h, SA2_SHAPE_DEFAULT_*).
    public const double DefaultCellShrink = .76, DefaultStickerShrink = .82, DefaultD4 = 1.18, DefaultZoom = 1.15;
    public static IReadOnlyList<double> Identity => new double[] { 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1 };

    // The view the core draws with no camera and no shape set: Global in the product without --d02.
    public static CoreView Default(double aspect) => new(Identity, DefaultCellShrink, DefaultStickerShrink, DefaultD4, DefaultZoom, aspect);

    // The cap's chart projection when Global uses a layout, and the sign of Local's raw chart when it draws one.
    public Func<int, double[], (double X, double Y, double Z)>? Chart { get; init; }
    public int RawSign { get; init; }
    public double FocusCosMax { get; init; } = double.PositiveInfinity;
}

// Where the solving surface's anchors sit in a view the core draws (ADR-D01.62): the core's own projection of a
// position's home point, so an anchor stays on its position when the camera turns. Which slot stands for a position
// of an orbit is sample data, as in Look Lab (ScenePlaces): position p of an orbit is the orbit's p-th piece in
// piece-id order, drawn at the shrink anchor of its first slot. Positions stay where they are, so no pose applies.
public sealed class CorePlaces
{
    public const int Cells = 600, StickersPerCell = 433, Slots = Cells * StickersPerCell, BaseVertices = 30480, Pieces = 177120;
    // S-B SPEC section 3 pin: SHA-256 of the 433 x 4 little-endian float32 shrink anchors (the core's `centers`).
    public const string AnchorSha256 = "0b6ead284d3f62719e3e6ec893d6c199d49698d246a59164bc362446e39faca6";
    public static readonly IReadOnlyList<string> Files = new[] { "mesh.json", "mesh_vertices.f32", "cell_frames.f32", "slot_piece.u32", "model.npz" };
    private readonly double[] anchors, frames, normal;
    private readonly double radius;
    private readonly int[][] orbitPieces;
    private readonly int[] firstSlot;

    private CorePlaces(double[] anchors, double[] frames, double[] normal, double radius, int[] slotPiece, int[] orbitIds)
    {
        this.anchors = anchors; this.frames = frames; this.normal = normal; this.radius = radius;
        firstSlot = Enumerable.Repeat(-1, orbitIds.Length).ToArray();
        for (int slot = 0; slot < slotPiece.Length; slot++) if (firstSlot[slotPiece[slot]] < 0) firstSlot[slotPiece[slot]] = slot;
        int count = orbitIds.Length == 0 ? 0 : orbitIds.Max() + 1;
        var byOrbit = Enumerable.Range(0, count).Select(_ => new List<int>()).ToArray();
        // Pieces outside the 35 orbits (orbit id -1) stand for no position.
        for (int piece = 0; piece < orbitIds.Length; piece++) if (orbitIds[piece] >= 0) byOrbit[orbitIds[piece]].Add(piece);
        orbitPieces = byOrbit.Select(l => l.ToArray()).ToArray();
    }

    // The model's files in `folder` (an assets/ folder with its manifest.json), each checked against the manifest.
    public static CorePlaces Load(string folder)
    {
        using var manifest = JsonDocument.Parse(File.ReadAllText(Path.Combine(folder, "manifest.json")));
        JsonElement listed = manifest.RootElement.GetProperty("files");
        byte[] Read(string name)
        {
            byte[] bytes = File.ReadAllBytes(Path.Combine(folder, name));
            string want = listed.TryGetProperty("assets/" + name, out JsonElement digest) ? digest.GetString() ?? "" : throw new InvalidDataException(name + ": no manifest entry");
            if (!string.Equals(Convert.ToHexString(SHA256.HashData(bytes)), want, StringComparison.OrdinalIgnoreCase)) throw new InvalidDataException(name + ": SHA-256 differs from the manifest");
            return bytes;
        }
        using var mesh = JsonDocument.Parse(Read("mesh.json"));
        JsonElement root = mesh.RootElement;
        int[] offsets = root.GetProperty("offsets").EnumerateArray().Select(x => x.GetInt32()).ToArray();
        double[] normal = root.GetProperty("normal").EnumerateArray().Select(x => x.GetDouble()).ToArray();
        double radius = root.GetProperty("normal_length").GetDouble();
        if (offsets.Length != StickersPerCell + 1 || offsets[^1] != BaseVertices || normal.Length != 4 || !(radius > 0)) throw new InvalidDataException("mesh.json: not the full model");
        double[] anchors = ShrinkAnchors(offsets, Floats(Read("mesh_vertices.f32"), BaseVertices * 4, "mesh_vertices.f32"));
        double[] frames = Floats(Read("cell_frames.f32"), Cells * 16, "cell_frames.f32");
        byte[] pieceBytes = Read("slot_piece.u32");
        if (pieceBytes.Length != Slots * 4) throw new InvalidDataException("slot_piece.u32: invalid length");
        int[] slotPiece = new int[Slots];
        for (int slot = 0; slot < Slots; slot++)
        {
            uint piece = BinaryPrimitives.ReadUInt32LittleEndian(pieceBytes.AsSpan(slot * 4, 4));
            slotPiece[slot] = piece < Pieces ? (int)piece : throw new InvalidDataException("slot_piece.u32: invalid piece");
        }
        return new CorePlaces(anchors, frames, normal, radius, slotPiece, OrbitIds(Read("model.npz")));
    }

    private static double[] Floats(byte[] bytes, int count, string name)
    {
        if (bytes.Length != count * 4) throw new InvalidDataException(name + ": invalid length");
        var result = new double[count];
        for (int i = 0; i < count; i++)
        {
            result[i] = BinaryPrimitives.ReadSingleLittleEndian(bytes.AsSpan(i * 4, 4));
            if (!double.IsFinite(result[i])) throw new InvalidDataException(name + ": non-finite float");
        }
        return result;
    }

    // The core's shrink anchors (S-B SPEC section 3): each sticker's area-weighted triangle centroid, in float64 in
    // file order and rounded once to float32, held to the pinned digest.
    public static double[] ShrinkAnchors(int[] offsets, double[] vertices)
    {
        var result = new double[(offsets.Length - 1) * 4];
        var bytes = new byte[result.Length * 4];
        for (int local = 0; local + 1 < offsets.Length; local++)
        {
            double total = 0; var weighted = new double[4];
            for (int t = offsets[local]; t < offsets[local + 1]; t += 3)
            {
                int a = t * 4, b = a + 4, c = b + 4;
                double d11 = 0, d22 = 0, d12 = 0;
                for (int i = 0; i < 4; i++)
                {
                    double e1 = vertices[b + i] - vertices[a + i], e2 = vertices[c + i] - vertices[a + i];
                    d11 += e1 * e1; d22 += e2 * e2; d12 += e1 * e2;
                }
                double area = .5 * Math.Sqrt(Math.Max(d11 * d22 - d12 * d12, 0));
                total += area;
                for (int i = 0; i < 4; i++) weighted[i] += area * ((vertices[a + i] + vertices[b + i] + vertices[c + i]) / 3);
            }
            if (!(total > 0)) throw new InvalidDataException($"sticker {local} has no positive area");
            for (int i = 0; i < 4; i++)
            {
                float rounded = (float)(weighted[i] / total);
                result[local * 4 + i] = rounded;
                BinaryPrimitives.WriteSingleLittleEndian(bytes.AsSpan((local * 4 + i) * 4, 4), rounded);
            }
        }
        if (offsets.Length == StickersPerCell + 1 && Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant() != AnchorSha256)
            throw new InvalidDataException("shrink anchors: SHA-256 differs from the S-B SPEC section 3 pin");
        return result;
    }

    // orbit_id.npy of the model archive: each piece's orbit (int16), -1 for none.
    private static int[] OrbitIds(byte[] archive)
    {
        using var zip = new ZipArchive(new MemoryStream(archive, false), ZipArchiveMode.Read);
        ZipArchiveEntry entry = zip.GetEntry("orbit_id.npy") ?? throw new InvalidDataException("model.npz: no orbit_id.npy");
        using var stream = entry.Open(); using var memory = new MemoryStream(); stream.CopyTo(memory);
        byte[] npy = memory.ToArray();
        if (npy.Length < 10 || npy[0] != 0x93 || Encoding.ASCII.GetString(npy, 1, 5) != "NUMPY") throw new InvalidDataException("orbit_id.npy: not an .npy array");
        int headerLength = npy[6] == 1 ? BinaryPrimitives.ReadUInt16LittleEndian(npy.AsSpan(8, 2)) : (int)BinaryPrimitives.ReadUInt32LittleEndian(npy.AsSpan(8, 4));
        int start = (npy[6] == 1 ? 10 : 12) + headerLength;
        string header = Encoding.ASCII.GetString(npy, npy[6] == 1 ? 10 : 12, headerLength);
        if (!header.Contains("'descr': '<i2'", StringComparison.Ordinal) || !header.Contains("'fortran_order': False", StringComparison.Ordinal) ||
            !header.Contains($"'shape': ({Pieces},)", StringComparison.Ordinal) || npy.Length - start != Pieces * 2)
            throw new InvalidDataException("orbit_id.npy: expected " + Pieces + " little-endian int16 values");
        var ids = new int[Pieces];
        for (int i = 0; i < Pieces; i++) ids[i] = BinaryPrimitives.ReadInt16LittleEndian(npy.AsSpan(start + i * 2, 2));
        return ids;
    }

    public int Orbits => orbitPieces.Length;
    public int PiecesOf(int orbit) => orbit >= 0 && orbit < orbitPieces.Length ? orbitPieces[orbit].Length : 0;

    // The slot that stands for a position of an orbit (sample data), or null.
    public int? Slot(int orbit, int position) => position >= 0 && position < PiecesOf(orbit) ? firstSlot[orbitPieces[orbit][position]] : null;

    // A point of a cell's frame (already divided by the radius) in the world: geometry.hlsl's framePoint.
    private double[] Framed(int cell, double[] v)
    {
        var world = new double[4];
        for (int column = 0; column < 4; column++)
            for (int row = 0; row < 4; row++) world[row] += frames[cell * 16 + column * 4 + row] * v[column];
        return world;
    }

    // The home point of a slot's sticker anchor in the world, before any pose: geometry.hlsl's homePoint with the
    // vertex at the anchor, so the sticker shrink drops out.
    public double[] Home(int slot, double cellShrink)
    {
        int cell = slot / StickersPerCell, local = slot % StickersPerCell;
        var v = new double[4];
        for (int i = 0; i < 4; i++) v[i] = (normal[i] + cellShrink * (anchors[local * 4 + i] - normal[i])) / radius;
        return Framed(cell, v);
    }

    // A cell's unit centre in the world, as the core takes a focus cell's pole (scene.cpp, cell_centre).
    public double[] CellCentre(int cell)
    {
        double[] centre = Framed(cell, normal.ToArray());
        double length = Math.Sqrt(centre.Sum(x => x * x));
        return centre.Select(x => x / length).ToArray();
    }

    private static double[] Times(IReadOnlyList<double> q, double[] x)
    {
        var y = new double[4];
        for (int row = 0; row < 4; row++) for (int k = 0; k < 4; k++) y[row] += q[row * 4 + k] * x[k];
        return y;
    }

    // The camera a focused view draws with: Q' = Q T, where T takes the pole to +w (scene.cpp, product_constants).
    public static double[] FocusedCamera(IReadOnlyList<double> q, IReadOnlyList<double> pole)
    {
        var t = new double[16]; for (int i = 0; i < 4; i++) t[i * 4 + i] = 1;
        double dot = pole[3];
        if (dot > -1 + 1e-9)
        {
            double[] sum = { pole[0], pole[1], pole[2], pole[3] + 1 };
            for (int i = 0; i < 4; i++) for (int j = 0; j < 4; j++) t[i * 4 + j] += -sum[i] * sum[j] / (1 + dot) + (i == 3 ? 2 * pole[j] : 0);
        }
        else { t[10] = -1; t[15] = -1; }
        var turned = new double[16];
        for (int i = 0; i < 4; i++) for (int j = 0; j < 4; j++) for (int k = 0; k < 4; k++) turned[i * 4 + j] += q[i * 4 + k] * t[k * 4 + j];
        return turned;
    }

    // A slot's place in a view from 0 to 1 (y down), or null when the view does not draw it: outside the focus cap,
    // behind the camera's near plane or off the pane.
    public (double X, double Y)? Place(int slot, CoreView view)
    {
        int cell = slot / StickersPerCell, local = slot % StickersPerCell;
        var anchor = new double[4];
        for (int i = 0; i < 4; i++) anchor[i] = anchors[local * 4 + i] / radius;
        return Project(cell, Home(slot, view.CellShrink), Framed(cell, anchor), view);
    }

    // The anchors' projection for a view the host describes (null: the view is not drawn).
    public Func<int, int, (double X, double Y)?> Projector(Func<CoreView?> view) =>
        (orbit, position) => Slot(orbit, position) is { } slot && view() is { } shown ? Place(slot, shown) : null;

    // A retained corner in model units, or the cap centre when the point is null. The core's cell shrink and
    // clip apply to these fixed positions, with no sticker shrink or piece pose.
    public (double X, double Y)? PlaceCellPoint(int cell, IReadOnlyList<double>? point, CoreView view)
    {
        var centre = CellCentre(cell);
        if (point == null) return Project(cell, centre, centre, view);
        var anchor = point.Select(x => x / radius).ToArray();
        var world = anchor.Select((x, i) => centre[i] + view.CellShrink * (x - centre[i])).ToArray();
        return Project(cell, world, anchor, view);
    }

    private (double X, double Y)? Project(int cell, double[] world, double[] anchor, CoreView view)
    {
        if (view.Chart is { } chart)
        {
            var (x, y, z) = chart(cell, world);
            return Clip(x, y, z, view);
        }
        IReadOnlyList<double> q = view.Q;
        if (view.FocusPole is { } pole)
        {
            double dot = 0; for (int i = 0; i < 4; i++) dot += anchor[i] * pole[i];
            if (view.RawSign < 0 ? -dot <= view.FocusCos : dot < view.FocusCos || dot > view.FocusCosMax) return null;
            q = FocusedCamera(view.Q, pole);
        }
        double[] w = Times(q, world);
        if (view.RawSign != 0)
        {
            double length = Math.Sqrt(w.Sum(x => x * x));
            double factor = 2 * view.RawSign / Math.Max(length + view.RawSign * w[3], .0001 * length);
            return Clip(w[0] * factor, w[1] * factor, w[2] * factor, view);
        }
        double scale = view.Orthographic ? 1 : view.D4 / (view.D4 - w[3]);
        double px = w[0] * scale, py = w[1] * scale, pz = w[2] * scale;
        if (view.Spread > 1)
        {
            var n = new double[4]; for (int i = 0; i < 4; i++) n[i] = normal[i] / radius;
            double[] c = Times(q, Framed(cell, n));
            double factor = (view.Orthographic ? 1 : view.D4 / (view.D4 - c[3])) * (view.Spread - 1);
            px += c[0] * factor; py += c[1] * factor; pz += c[2] * factor;
        }
        return Clip(px, py, pz, view);
    }

    private static (double X, double Y)? Clip(double px, double py, double pz, CoreView view)
    {
        double z = 5 - pz;
        if (z < .05 || z > 100 || !(view.Aspect > 0)) return null;
        double x = px * view.Zoom / view.Aspect / z, y = py * view.Zoom / z;
        if (Math.Abs(x) > 1 || Math.Abs(y) > 1 || !double.IsFinite(x) || !double.IsFinite(y)) return null;
        return ((x + 1) / 2, (1 - y) / 2);
    }

}
