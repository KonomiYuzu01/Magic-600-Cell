using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.Linq;

namespace LookLab.Core;

// The Local view's focus, as the shader reads it (cells.gdshader: focus_axis, focus_cos, focus_spread): the cap of
// cells the view draws around its axis, and how far it spreads the cells apart. A cap cosine below -1.5 is no cap,
// as in the Global view.
public sealed record LocalFocus(IReadOnlyList<double> Axis, double CapCos, double Spread)
{
    public static readonly LocalFocus None = new(new double[] { 0, 0, 0, 1 }, -2, 1);
}

// Where the solving surface's anchors sit in a view (ADR-D01.62): the view's own projection of a point of the
// model, so an anchor stays on its geometry when the camera turns. Which slot stands for a position of an orbit
// is sample data, because the lab has no engine: position p of an orbit is the orbit's p-th piece in piece-id
// order, and the piece is drawn at its first sticker. The projection itself is the core's, the one the shader
// geometry check holds the GPU to.
public sealed class ScenePlaces
{
    private readonly Geometry geometry;
    private readonly int[][] orbitPieces;
    private readonly int[] firstSlot;
    private readonly int[][] stickerVertices;

    public ScenePlaces(string repositoryRoot, Geometry geometry)
    {
        this.geometry = geometry;
        byte[] bytes = new AssetCatalogue(repositoryRoot).ReadVerified("slot_piece.u32");
        if (bytes.Length != Geometry.Slots * 4) throw Json.Error("slot_piece.u32", "invalid length");
        int pieces = geometry.CellStructure.OrbitIds.Length;
        firstSlot = Enumerable.Repeat(-1, pieces).ToArray();
        for (int slot = 0; slot < Geometry.Slots; slot++)
        {
            int piece = (int)BinaryPrimitives.ReadUInt32LittleEndian(bytes.AsSpan(slot * 4, 4));
            if (piece < 0 || piece >= pieces) throw Json.Error("slot_piece.u32", "invalid piece");
            if (firstSlot[piece] < 0) firstSlot[piece] = slot;
        }
        ReadOnlySpan<int> orbits = geometry.CellStructure.OrbitIds.Span;
        int count = 0; for (int p = 0; p < pieces; p++) count = Math.Max(count, orbits[p] + 1);
        var byOrbit = Enumerable.Range(0, count).Select(_ => new List<int>()).ToArray();
        // Pieces outside the 35 orbits (orbit id -1) stand for no position.
        for (int p = 0; p < pieces; p++) if (orbits[p] >= 0) byOrbit[orbits[p]].Add(p);
        orbitPieces = byOrbit.Select(l => l.ToArray()).ToArray();
        var vertices = Enumerable.Range(0, Geometry.StickersPerCell).Select(_ => new List<int>()).ToArray();
        ReadOnlySpan<int> stickers = geometry.BaseStickerIds.Span;
        for (int v = 0; v < Geometry.BaseVertices; v++) vertices[stickers[v]].Add(v);
        stickerVertices = vertices.Select(l => l.ToArray()).ToArray();
    }

    public int Orbits => orbitPieces.Length;
    public int Pieces(int orbit) => orbit >= 0 && orbit < orbitPieces.Length ? orbitPieces[orbit].Length : 0;

    // The slot that stands for a position of an orbit (sample data), or null.
    public int? Slot(int orbit, int position) => position >= 0 && position < Pieces(orbit) ? firstSlot[orbitPieces[orbit][position]] : null;

    // The view's projection of a slot: the mean of its sticker's projected vertices, then the view frame. With a
    // Local focus it is the shader's own: a cell outside the cap is not drawn, so it has no place (null), and a cell
    // inside is moved by the spread before the camera.
    public Projection? Project(int slot, StructureValues values, double[] q, double[] spin, TurnData turn, double theta, GlobalLayout layout, FocusFrame frame, LocalFocus? focus = null)
    {
        int cell = slot / Geometry.StickersPerCell; int[] own = stickerVertices[slot % Geometry.StickersPerCell];
        IReadOnlyList<double> centre = layout.Centre(cell);
        bool capped = focus != null && focus.CapCos > -1.5;
        if (capped && Cosine(centre, focus!.Axis) < focus.CapCos) return null;
        double[]? offset = capped && focus!.Spread != 1 && !GlobalLayout.IsLayout(values.Projection) ? SpreadOffset(centre, values, q, turn, theta, turn.IsMoving(slot), focus.Spread) : null;
        double x = 0, y = 0, w = 0;
        foreach (int v in own)
        {
            int vertex = cell * Geometry.BaseVertices + v;
            Projection p;
            if (offset == null) p = geometry.Project(vertex, values, q, turn, theta, layout, spin);
            else
            {
                double[] at = geometry.Projected(vertex, values, q, turn, theta);
                for (int i = 0; i < 3; i++) at[i] += offset[i];
                p = ViewFraming.Camera(at, values);
            }
            x += p.NdcX; y += p.NdcY; w += p.W;
        }
        return ViewFraming.Apply(new Projection(x / own.Length, y / own.Length, w / own.Length), frame);
    }

    private static double Cosine(IReadOnlyList<double> centre, IReadOnlyList<double> axis)
    {
        double length = Math.Sqrt(centre.Sum(c => c * c)), dot = 0;
        for (int i = 0; i < 4; i++) dot += centre[i] * axis[i];
        return length > 0 ? dot / length : -1;
    }

    // The shader's spread: the cell's centre, turned with the twist when the sticker moves, taken through the
    // camera rotation and scaled like a point (perspective only), moves the cell by (spread - 1) times itself.
    private static double[] SpreadOffset(IReadOnlyList<double> centre, StructureValues values, double[] q, TurnData turn, double theta, bool moving, double spread)
    {
        double[] c = centre.ToArray();
        if (moving && theta != 0)
        {
            ReadOnlySpan<double> u = turn.PlaneU.Span, v = turn.PlaneV.Span;
            double cx = Geometry.Dot(c, u), cy = Geometry.Dot(c, v), cos = Math.Cos(theta), sin = Math.Sin(theta);
            double du = (cos - 1) * cx - sin * cy, dv = sin * cx + (cos - 1) * cy;
            for (int i = 0; i < 4; i++) c[i] += du * u[i] + dv * v[i];
        }
        double[] turned = Geometry.Multiply(q, c);
        double factor = values.Projection == ProjectionKind.Perspective ? values.D4 / (values.D4 - turned[3]) : 1;
        return new[] { turned[0] * factor * (spread - 1), turned[1] * factor * (spread - 1), turned[2] * factor * (spread - 1) };
    }

    // A projected point as a place in the view from 0 to 1 (y down), or null when it is not drawn, behind the camera
    // or off the view.
    public static (double X, double Y)? InView(Projection? projection)
    {
        if (projection is not { } p || p.W <= 0 || Math.Abs(p.NdcX) > 1 || Math.Abs(p.NdcY) > 1) return null;
        return ((p.NdcX + 1) / 2, (1 - p.NdcY) / 2);
    }
}
