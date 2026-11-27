using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

public static class StickerColouring
{
    public const int CellCount = 600;
    private const int RingCount = 20;

    // The table is in home-cell order, the same as Look Lab's cell_classes[label / 433].
    public static float[] Table(string rings, StickerPalette palette)
    {
        using JsonDocument document = J.Parse(rings);
        JsonElement root = document.RootElement;
        if (root.GetProperty("format").GetString() != "magic600-look-hopf-rings" || root.GetProperty("version").GetInt32() != 1)
            throw J.Error("Hopf rings", "expected Look Lab's version 1 ring file");
        int[] ringOf = root.GetProperty("ringOf").EnumerateArray().Select(r => r.GetInt32()).ToArray();
        if (ringOf.Length != CellCount || ringOf.Any(r => r < 0 || r >= RingCount))
            throw J.Error("ringOf", "expected 600 rings in 0..19, one per cell");
        var graph = root.GetProperty("graph").EnumerateArray().Select(pair => (pair[0].GetInt32(), pair[1].GetInt32()));
        int[] ringClass = EquitableColouring(RingCount, graph, palette.Colours.Count)
            ?? throw J.Error("classes", "no proper ring colouring");

        var table = new float[3 * CellCount];
        for (int cell = 0; cell < CellCount; cell++)
        {
            Rgb colour = palette.Colours[ringClass[ringOf[cell]]]; // CellColouring.ClassOf
            table[3 * cell] = (float)colour.R;
            table[3 * cell + 1] = (float)colour.G;
            table[3 * cell + 2] = (float)colour.B;
        }
        return table;
    }

    // Ported from Look Lab's GlobalLayout.EquitableColouring, including its search order.
    private static int[]? EquitableColouring(int count, IEnumerable<(int A, int B)> pairs, int classes)
    {
        if (count < 1 || classes < 1) throw J.Error("classes", "positive counts required");
        var neighbors = Enumerable.Range(0, count).Select(_ => new List<int>()).ToArray();
        foreach (var (a, b) in pairs)
        {
            if (a < 0 || b < 0 || a >= count || b >= count) throw J.Error("ring graph", "invalid vertex");
            if (a == b) return null;
            neighbors[a].Add(b); neighbors[b].Add(a);
        }
        int low = count / classes, high = low + (count % classes == 0 ? 0 : 1);
        int[] colours = Enumerable.Repeat(-1, count).ToArray(), used = new int[classes];
        bool Search(int vertex)
        {
            if (used.Sum(n => Math.Max(0, low - n)) > count - vertex) return false;
            if (vertex == count) return true;
            for (int colour = 0; colour < classes; colour++)
            {
                if (used[colour] == high || neighbors[vertex].Any(n => colours[n] == colour)) continue;
                colours[vertex] = colour; used[colour]++;
                if (Search(vertex + 1)) return true;
                used[colour]--;
            }
            colours[vertex] = -1;
            return false;
        }
        return Search(0) ? colours : null;
    }
}
