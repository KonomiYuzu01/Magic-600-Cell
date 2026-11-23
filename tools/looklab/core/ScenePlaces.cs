using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.Linq;

namespace LookLab.Core;

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

    // The view's projection of a slot: the mean of its sticker's projected vertices, then the view frame.
    public Projection Project(int slot, StructureValues values, double[] q, double[] spin, TurnData turn, double theta, GlobalLayout layout, FocusFrame frame)
    {
        int cell = slot / Geometry.StickersPerCell; int[] own = stickerVertices[slot % Geometry.StickersPerCell];
        double x = 0, y = 0, w = 0;
        foreach (int v in own)
        {
            Projection p = geometry.Project(cell * Geometry.BaseVertices + v, values, q, turn, theta, layout, spin);
            x += p.NdcX; y += p.NdcY; w += p.W;
        }
        return ViewFraming.Apply(new Projection(x / own.Length, y / own.Length, w / own.Length), frame);
    }

    // A projected point as a place in the view from 0 to 1 (y down), or null when it is behind the camera or off
    // the view.
    public static (double X, double Y)? InView(Projection p)
    {
        if (p.W <= 0 || Math.Abs(p.NdcX) > 1 || Math.Abs(p.NdcY) > 1) return null;
        return ((p.NdcX + 1) / 2, (1 - p.NdcY) / 2);
    }
}
