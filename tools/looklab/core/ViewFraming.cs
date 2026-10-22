using System;
using System.Linq;

namespace LookLab.Core;

public readonly record struct ViewArrangement(RegionRect Global, RegionRect? Local, bool LocalOverlay);
public readonly record struct FocusFrame(double X, double Y, double M)
{
    public static FocusFrame Neutral => new(0, 0, 1);
}

public static class ViewFraming
{
    public static ViewArrangement Arrange(LayoutSpec layout, RegionRect stage)
    {
        LayoutRegion? local = layout.Regions.Values.SingleOrDefault(r => r.Kind == "local");
        if (local == null) return new ViewArrangement(stage, null, false);
        RegionRect global = layout.Regions.Values.Single(r => r.Kind == "global").Rect, inset = local.Rect;
        double x = Math.Min(global.X, inset.X), y = Math.Min(global.Y, inset.Y);
        double width = Math.Max(global.X + global.Width, inset.X + inset.Width) - x;
        double height = Math.Max(global.Y + global.Height, inset.Y + inset.Height) - y;
        RegionRect Map(RegionRect rect) => new(stage.X + (rect.X - x) / width * stage.Width,
            stage.Y + (rect.Y - y) / height * stage.Height, rect.Width / width * stage.Width, rect.Height / height * stage.Height);
        return new ViewArrangement(Map(global), Map(inset), local.Placement == "overlay");
    }

    // Theta zero: shrink about the verified sticker anchor, then apply its cell frame.
    // Sticker shrink cancels at the anchor. The caller caches these until gap changes.
    public static double[] MovingRestAnchors(Geometry mesh, TurnData turn, double gap)
    {
        var anchors = new double[Enumerable.Range(0, Geometry.Slots).Count(turn.IsMoving) * 4];
        int offset = 0;
        Span<double> point = stackalloc double[4];
        for (int slot = 0; slot < Geometry.Slots; slot++)
        {
            if (!turn.IsMoving(slot)) continue;
            int local = slot % Geometry.StickersPerCell;
            for (int i = 0; i < 4; i++) point[i] = (mesh.BaseNormal.Span[i] + (1 - gap) *
                (mesh.StickerAnchors.Span[local * 4 + i] - mesh.BaseNormal.Span[i])) / mesh.Radius;
            ReadOnlySpan<double> matrix = mesh.CellFrames.Span.Slice(slot / Geometry.StickersPerCell * 16, 16);
            for (int row = 0; row < 4; row++)
            {
                double value = 0;
                for (int col = 0; col < 4; col++) value += matrix[col * 4 + row] * point[col];
                anchors[offset++] = value;
            }
        }
        return anchors;
    }

    public static int[] MovingRestCells(TurnData turn) => Enumerable.Range(0, Geometry.Slots).Where(turn.IsMoving)
        .Select(slot => slot / Geometry.StickersPerCell).ToArray();

    public static Projection Camera(ReadOnlySpan<double> placed, StructureValues p)
    {
        double w = 5 - placed[2];
        return new Projection(placed[0] * p.Zoom / p.Aspect / w, placed[1] * p.Zoom / w, w);
    }

    public static Projection Project(ReadOnlySpan<double> world, StructureValues p)
    {
        double factor = p.Projection switch
        {
            ProjectionKind.Perspective => p.D4 / (p.D4 - world[3]),
            ProjectionKind.Stereographic => 1 / (1 - world[3]),
            ProjectionKind.Orthographic => 1,
            _ => throw Json.Error("projection", "unknown kind")
        };
        double w = 5 - world[2] * factor;
        return new Projection(world[0] * factor * p.Zoom / p.Aspect / w, world[1] * factor * p.Zoom / w, w);
    }

    public static Projection Apply(Projection projection, FocusFrame frame) =>
        new((projection.NdcX - frame.X) * frame.M, (projection.NdcY - frame.Y) * frame.M, projection.W);

    private static double Smooth(double value, double low, double high)
    {
        double t = Math.Clamp((value - low) / (high - low), 0, 1);
        return t * t * (3 - 2 * t);
    }

    public static FocusFrame Target(ReadOnlySpan<double> anchors, ReadOnlySpan<double> q, StructureValues p, double magnification = 3)
    {
        double weight = 0, x = 0, y = 0;
        Span<double> world = stackalloc double[4];
        for (int offset = 0; offset < anchors.Length; offset += 4)
        {
            for (int row = 0; row < 4; row++)
            {
                world[row] = 0;
                for (int col = 0; col < 4; col++) world[row] += q[col * 4 + row] * anchors[offset + col];
            }
            double denominator = p.Projection == ProjectionKind.Perspective ? p.D4 - world[3] : 1 - world[3];
            double g = p.Projection == ProjectionKind.Orthographic ? 1 : Smooth(denominator, .02, .1);
            if (!double.IsFinite(g) || g == 0) continue; // The denominator fade reaches zero before division is singular.
            Projection projected = Project(world, p);
            if (!double.IsFinite(projected.W) || !double.IsFinite(projected.NdcX) || !double.IsFinite(projected.NdcY)) continue;
            double a = Smooth(projected.W, .5, 1) * (1 - Smooth(Math.Abs(projected.NdcX), 1, 1.25)) *
                (1 - Smooth(Math.Abs(projected.NdcY), 1, 1.25)) * g;
            weight += a; x += a * projected.NdcX; y += a * projected.NdcY;
        }
        double m = 1 + (magnification - 1) * Math.Min(1, weight), bound = 1 - 1 / m;
        return new FocusFrame(Math.Clamp(x / Math.Max(weight, 1), -bound, bound),
            Math.Clamp(y / Math.Max(weight, 1), -bound, bound), m);
    }

    public static FocusFrame Target(ReadOnlySpan<double> anchors, ReadOnlySpan<int> cells, ReadOnlySpan<double> q, ReadOnlySpan<double> spin,
        StructureValues p, GlobalLayout layout, double magnification = 3)
    {
        if (!GlobalLayout.IsLayout(p.Projection)) return Target(anchors, q, p, magnification);
        if (anchors.Length % 4 != 0 || cells.Length != anchors.Length / 4) throw Json.Error("anchors", "one chart cell per 4D anchor required");
        double weight = 0, x = 0, y = 0;
        for (int i = 0; i < cells.Length; i++)
        {
            double[] placed = layout.Place(p.Projection, cells[i], anchors.Slice(i * 4, 4), q, spin);
            Projection projected = Camera(placed, p);
            if (!double.IsFinite(projected.W) || !double.IsFinite(projected.NdcX) || !double.IsFinite(projected.NdcY)) continue;
            double a = Smooth(projected.W, .5, 1) * (1 - Smooth(Math.Abs(projected.NdcX), 1, 1.25)) *
                (1 - Smooth(Math.Abs(projected.NdcY), 1, 1.25));
            weight += a; x += a * projected.NdcX; y += a * projected.NdcY;
        }
        double m = 1 + (magnification - 1) * Math.Min(1, weight), bound = 1 - 1 / m;
        return new FocusFrame(Math.Clamp(x / Math.Max(weight, 1), -bound, bound),
            Math.Clamp(y / Math.Max(weight, 1), -bound, bound), m);
    }
}

public sealed class FrameFollower
{
    public FocusFrame Frame { get; private set; } = FocusFrame.Neutral;
    private bool first = true;
    public void Reset() => first = true;

    public FocusFrame Step(FocusFrame target, double dt)
    {
        if (!double.IsFinite(dt) || dt <= 0) return Frame;
        if (first) { first = false; return Frame = target; }
        double response = 1 - Math.Exp(-dt / .12);
        double Follow(double value, double goal, double speed) => value + Math.Clamp((goal - value) * response, -speed * dt, speed * dt);
        return Frame = new FocusFrame(Follow(Frame.X, target.X, 2), Follow(Frame.Y, target.Y, 2), Follow(Frame.M, target.M, 4));
    }
}
