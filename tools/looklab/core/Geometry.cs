using System;
using System.Buffers.Binary;
using System.IO;
using System.Linq;
using System.Text;

namespace LookLab.Core;

public enum ProjectionKind { Perspective, Stereographic, Orthographic }
public readonly record struct Projection(double NdcX, double NdcY, double W);
public sealed record StructureValues(double Cs, double Ss, double D4, double Zoom, double Aspect, ProjectionKind Projection = ProjectionKind.Perspective);

public sealed class Geometry
{
    public const int BaseVertices = 30480;
    public const int StickersPerCell = 433;
    public const int Cells = 600;
    public const int Slots = Cells * StickersPerCell;
    private readonly double[] vertices;
    private readonly int[] stickers;
    private readonly double[] centers;
    private readonly double[] frames;
    private readonly double[] normal;
    private readonly double radius;
    private readonly int[] slotOrbits;
    public string ModelId { get; }
    public CellStructure CellStructure { get; }
    // Verified immutable input views for renderer upload, in the original order.
    public ReadOnlyMemory<double> BaseVertexData => vertices;
    public ReadOnlyMemory<int> BaseStickerIds => stickers;
    public ReadOnlyMemory<double> StickerCenters => centers;
    public ReadOnlyMemory<double> CellFrames => frames;
    public ReadOnlyMemory<double> BaseNormal => normal;
    public double Radius => radius;
    public ReadOnlyMemory<int> SlotOrbits => slotOrbits;

    public Geometry(string repositoryRoot)
    {
        var assets = new AssetCatalogue(repositoryRoot);
        ModelId = assets.ModelId;
        using var mesh = Json.Parse(Encoding.UTF8.GetString(assets.ReadVerified("mesh.json")));
        var root = mesh.RootElement;
        foreach (var (key, count) in new[] { ("base_vertices", BaseVertices), ("base_stickers", StickersPerCell), ("cells", Cells), ("slots", Slots) })
            if (Json.Integer(Json.Get(root, key), key) != count) throw Json.Error("mesh.json/" + key, "full model count required");
        int[] offsets = Json.Array(Json.Get(root, "offsets"), "offsets").Select(x => Json.Integer(x, "offsets")).ToArray();
        if (offsets.Length != 434 || offsets[0] != 0 || offsets[^1] != BaseVertices ||
            offsets.Zip(offsets.Skip(1)).Any(p => p.Second <= p.First || (p.Second - p.First) % 3 != 0))
            throw Json.Error("mesh.json/offsets", "expected 433 complete triangle lists");
        normal = Json.Array(Json.Get(root, "normal"), "normal").Select(x => Json.Number(x, "normal")).ToArray();
        radius = Json.Number(Json.Get(root, "normal_length"), "normal_length");
        if (normal.Length != 4 || radius <= 0) throw Json.Error("mesh.json", "invalid normal or radius");
        vertices = Floats(assets.ReadVerified("mesh_vertices.f32"), BaseVertices * 4, "mesh_vertices.f32");
        centers = Floats(assets.ReadVerified("mesh_centers.f32"), StickersPerCell * 4, "mesh_centers.f32");
        frames = Floats(assets.ReadVerified("cell_frames.f32"), Cells * 16, "cell_frames.f32");
        byte[] stickerBytes = assets.ReadVerified("mesh_sticker.u32");
        if (stickerBytes.Length != BaseVertices * 4) throw Json.Error("mesh_sticker.u32", "invalid length");
        stickers = new int[BaseVertices];
        for (int local = 0; local < StickersPerCell; local++) for (int vi = offsets[local]; vi < offsets[local + 1]; vi++)
        {
            uint sticker = BinaryPrimitives.ReadUInt32LittleEndian(stickerBytes.AsSpan(vi * 4, 4));
            if (sticker != local) throw Json.Error("mesh_sticker.u32", "indices disagree with offsets");
            stickers[vi] = local;
        }
        CellStructure = CellStructure.Load(assets.ReadVerified("model.npz"));
        byte[] pieces = assets.ReadVerified("slot_piece.u32");
        if (pieces.Length != Slots * 4) throw Json.Error("slot_piece.u32", "invalid length");
        slotOrbits = new int[Slots];
        for (int slot = 0; slot < Slots; slot++)
        {
            uint piece = BinaryPrimitives.ReadUInt32LittleEndian(pieces.AsSpan(slot * 4, 4));
            if (piece >= CellStructure.OrbitIds.Length) throw Json.Error("slot_piece.u32", "invalid piece");
            slotOrbits[slot] = CellStructure.OrbitIds.Span[(int)piece];
        }
    }

    private static double[] Floats(byte[] bytes, int count, string name)
    {
        if (bytes.Length != count * 4) throw Json.Error(name, "invalid length");
        var result = new double[count];
        for (int i = 0; i < count; i++)
        {
            result[i] = BinaryPrimitives.ReadSingleLittleEndian(bytes.AsSpan(i * 4, 4));
            if (!double.IsFinite(result[i])) throw Json.Error(name, "non-finite float");
        }
        return result;
    }

    public static int CellOfSlot(int slot)
    {
        if (slot < 0 || slot >= Slots) throw new ArgumentOutOfRangeException(nameof(slot));
        return slot / StickersPerCell;
    }

    public Projection Project(int globalVertex, StructureValues values, double[] q, TurnData turn, double theta)
    {
        if (globalVertex < 0 || globalVertex >= Cells * BaseVertices) throw new ArgumentOutOfRangeException(nameof(globalVertex));
        if (turn.ModelId != ModelId) throw Json.Error("turn/model_id", "does not match assets");
        if (q.Length != 16 || q.Any(x => !double.IsFinite(x))) throw Json.Error("Q", "expected 16 finite column-major values");
        if (!double.IsFinite(theta) || !double.IsFinite(values.Cs) || !double.IsFinite(values.Ss) ||
            !double.IsFinite(values.D4) || values.D4 <= 0 || !double.IsFinite(values.Zoom) || values.Zoom <= 0 ||
            !double.IsFinite(values.Aspect) || values.Aspect <= 0) throw Json.Error("structure", "invalid projection values");
        int cell = globalVertex / BaseVertices, vi = globalVertex % BaseVertices, local = stickers[vi];
        var vertex = new double[4];
        for (int i = 0; i < 4; i++) vertex[i] = (normal[i] + values.Cs * (centers[local * 4 + i] - normal[i])
            + values.Cs * values.Ss * (vertices[vi * 4 + i] - centers[local * 4 + i])) / radius;
        double[] world = Multiply(frames.AsSpan(cell * 16, 16), vertex);
        if (theta != 0 && turn.IsMoving(cell * StickersPerCell + local))
        {
            double x = Dot(world, turn.PlaneU.Span), y = Dot(world, turn.PlaneV.Span);
            double c = Math.Cos(theta), s = Math.Sin(theta);
            double du = (c - 1) * x - s * y, dv = s * x + (c - 1) * y;
            for (int i = 0; i < 4; i++) world[i] += du * turn.PlaneU.Span[i] + dv * turn.PlaneV.Span[i];
        }
        world = Multiply(q, world);
        double factor = values.Projection switch
        {
            ProjectionKind.Perspective => values.D4 / (values.D4 - world[3]),
            ProjectionKind.Stereographic => 1 / (1 - world[3]),
            ProjectionKind.Orthographic => 1,
            _ => throw Json.Error("projection", "unknown kind")
        };
        double px = factor * world[0], py = factor * world[1], pz = factor * world[2];
        double w = 5 - pz;
        var projection = new Projection(px * values.Zoom / values.Aspect / w, py * values.Zoom / w, w);
        if (!double.IsFinite(projection.NdcX) || !double.IsFinite(projection.NdcY) || !double.IsFinite(w))
            throw Json.Error("projection", "singular projection");
        return projection;
    }

    public static double[] Identity() => Enumerable.Range(0, 16).Select(i => i % 5 == 0 ? 1.0 : 0.0).ToArray();

    public static void Rotate(double[] q, int a, int b, double angle)
    {
        if (q.Length != 16 || a < 0 || a > 3 || b < 0 || b > 3 || a == b || !double.IsFinite(angle))
            throw Json.Error("rotate", "invalid camera rotation");
        double c = Math.Cos(angle), s = Math.Sin(angle);
        for (int j = 0; j < 4; j++)
        {
            double x = q[a * 4 + j], y = q[b * 4 + j];
            q[a * 4 + j] = c * x - s * y;
            q[b * 4 + j] = s * x + c * y;
        }
    }

    internal static double[] Multiply(ReadOnlySpan<double> matrix, ReadOnlySpan<double> vector)
    {
        var result = new double[4];
        for (int row = 0; row < 4; row++) for (int col = 0; col < 4; col++) result[row] += matrix[col * 4 + row] * vector[col];
        return result;
    }

    internal static double Dot(ReadOnlySpan<double> a, ReadOnlySpan<double> b)
    {
        double sum = 0;
        for (int i = 0; i < a.Length; i++) sum += a[i] * b[i];
        return sum;
    }
}
