using System;
using System.Collections.Generic;

namespace Magic600.Chrome;

// ADR-D01.20: a radial menu held on the key that opened it. While the key stays down, the entry whose direction from
// the hub is nearest the pointer's is the one release runs, and the hub says so; release in the hub runs nothing. A
// short tap leaves the menu open, to be chosen by a click or a number (ADR-D01.21). An entry runs as its palette row.
public enum RadialRelease { StayOpen, Run, Nothing }

public static class RadialHold
{
    public const double TapSeconds = .3;

    // The pointer at (dx, dy) from the hub: the entry whose direction is nearest, or -1 inside the hub's dead zone.
    public static int Pointed(double dx, double dy, IReadOnlyList<(double X, double Y)> directions, double dead)
    {
        double length = Math.Sqrt(dx * dx + dy * dy);
        if (length <= dead || directions.Count == 0) return -1;
        int best = -1; double most = double.MinValue;
        for (int i = 0; i < directions.Count; i++)
        {
            (double x, double y) = directions[i];
            double norm = Math.Sqrt(x * x + y * y);
            if (norm == 0) continue;
            double cosine = (x * dx + y * dy) / (norm * length);
            if (cosine > most) { most = cosine; best = i; }
        }
        return best;
    }

    // Entry i of n sits at this direction from the hub: the first straight up, then clockwise, the ring flattened to 0.8.
    public static (double X, double Y) Direction(int i, int n)
    {
        double angle = Math.PI * 2 * i / n;
        return (Math.Sin(angle), -Math.Cos(angle) * .8);
    }

    public static RadialRelease Release(double heldSeconds, int pointed) =>
        heldSeconds < TapSeconds ? RadialRelease.StayOpen : pointed >= 0 ? RadialRelease.Run : RadialRelease.Nothing;

    // The hub's second line while the key is held.
    public static string Hint(string? entry, string? unavailable) =>
        entry == null ? "Release here: nothing runs" : "Release runs " + entry + (unavailable == null ? "" : " (unavailable: " + unavailable + ")");

    // The status line after a release in the hub.
    public static string Let(string menu) => menu + ": released in the middle, nothing ran";
}
