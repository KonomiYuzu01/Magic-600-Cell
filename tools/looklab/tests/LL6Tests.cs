using System;
using System.Linq;
using LookLab.App;
using LookLab.Core;

internal static partial class Program
{
    private static StructureValues FocusParameters(double gap, ProjectionKind kind, double d4 = 1.18, double fov = .2, double aspect = 1.6) =>
        new(1 - gap, .82, d4, 1 / Math.Tan(fov / 2), aspect, kind);

    private static FocusFrame Focus(double[] q, StructureValues p) =>
        ViewFraming.Target(ViewFraming.MovingRestAnchors(Mesh, Turn, 1 - p.Cs), q, p);

    private static void Bounded(FocusFrame frame)
    {
        Check(double.IsFinite(frame.X) && double.IsFinite(frame.Y) && double.IsFinite(frame.M), "non-finite frame");
        Check(frame.M >= 1 && frame.M <= 3, "magnification outside [1,3]");
        double bound = 1 - 1 / frame.M;
        Check(Math.Abs(frame.X) <= bound + 1e-12 && Math.Abs(frame.Y) <= bound + 1e-12, "centre outside frame bound");
    }

    private static void FrameNear(FocusFrame a, FocusFrame b, double tolerance)
    {
        Near(a.X, b.X, tolerance); Near(a.Y, b.Y, tolerance); Near(a.M, b.M, tolerance);
    }

    private static void LL6Arrangement()
    {
        var stage = new RegionRect(75, 25, 1000, 600);
        foreach (string id in Schema["layoutId"].Options!)
        {
            LayoutSpec layout = LayoutSpec.Load(FileAt("data/layouts/" + id + ".json"), Commands);
            ViewArrangement result = ViewFraming.Arrange(layout, stage);
            if (id is "central-stage" or "docked-workbench")
            {
                Equal(result.Global, stage); Check(result.Local == null, "single-view layout acquired Local");
            }
            else if (id == "split-views")
            {
                Equal(result.Global, stage with { Width = 500 });
                Equal(result.Local!.Value, stage with { X = 575, Width = 500 });
                Check(!result.LocalOverlay, "split Local is overlay");
            }
            else
            {
                Equal(result.Global, stage);
                Equal(result.Local!.Value, new RegionRect(735, 337, 320, 240));
                Check(result.LocalOverlay, "inset Local is not overlay");
            }
        }
    }

    private static void LL6ProjectionAndRestAnchors()
    {
        foreach (ProjectionKind kind in Enum.GetValues<ProjectionKind>())
            foreach (double[] world in new[] { new[] { .2, -.4, .3, .7 }, new[] { -.5, .3, -.8, -.2 } })
            {
                StructureValues p = FocusParameters(.24, kind);
                double factor = kind == ProjectionKind.Perspective ? p.D4 / (p.D4 - world[3]) :
                    kind == ProjectionKind.Stereographic ? 1 / (1 - world[3]) : 1;
                double w = 5 - world[2] * factor;
                Projection actual = ViewFraming.Project(world, p);
                Near(actual.W, w); Near(actual.NdcX, world[0] * factor * p.Zoom / p.Aspect / w);
                Near(actual.NdcY, world[1] * factor * p.Zoom / w);
                var frame = new FocusFrame(.125, -.25, 2.5);
                Projection framed = ViewFraming.Apply(actual, frame);
                Near(framed.NdcX, (actual.NdcX - frame.X) * frame.M);
                Near(framed.NdcY, (actual.NdcY - frame.Y) * frame.M); Near(framed.W, w);
            }
        double[] anchors = ViewFraming.MovingRestAnchors(Mesh, Turn, .24);
        Equal(anchors.Length, Enumerable.Range(0, Geometry.Slots).Count(Turn.IsMoving) * 4);
        int offset = 0;
        for (int slot = 0; slot < Geometry.Slots; slot++)
        {
            if (!Turn.IsMoving(slot)) continue;
            int local = slot % Geometry.StickersPerCell;
            var point = new double[4];
            for (int axis = 0; axis < 4; axis++)
                point[axis] = (Mesh.BaseNormal.Span[axis] + .76 * (Mesh.StickerAnchors.Span[local * 4 + axis] - Mesh.BaseNormal.Span[axis])) / Mesh.Radius;
            double[] expected = Geometry.Multiply(Mesh.CellFrames.Span.Slice(slot / Geometry.StickersPerCell * 16, 16), point);
            for (int axis = 0; axis < 4; axis++) Near(anchors[offset++], expected[axis]);
        }
        // The adapter must pack the new uniform using its existing vec4 path.
        var source = new ShaderSource(Read("app/shaders/cells.gdshader"));
        Equal(source.Uniforms.Single(u => u.Name == "view_frame").Type, "vec4");
        Check(source.VertexBody.Contains("clip.xy = (clip.xy - view_frame.xy * clip.w) * view_frame.z;", StringComparison.Ordinal) &&
            source.VertexBody.Contains("geometry_value = vec3(clip.x / clip.w, clip.y / clip.w, clip.w);", StringComparison.Ordinal) &&
            source.VertexBody.Contains("POSITION = clip;", StringComparison.Ordinal), "probe and production framing diverged");
    }

    private static double[] LL6SingularCamera()
    {
        double[] q = Geometry.Identity();
        foreach (var (a, b, angle) in new[] { (0, 3, -.0006970494917220394), (1, 3, .00042836660660398846),
            (0, 1, -.9028905621136518), (1, 3, -.38311103899196103), (0, 3, -1.1950874362436468) })
            Geometry.Rotate(q, a, b, angle);
        return q;
    }

    private static void LL6FocusRegressions()
    {
        double[] q = Geometry.Identity();
        Geometry.Rotate(q, 0, 3, -1.1563928655169962); Geometry.Rotate(q, 1, 3, -.7853981627125591);
        Bounded(Focus(q, FocusParameters(0, ProjectionKind.Stereographic)));
        double[] left = Geometry.Identity(), right = Geometry.Identity();
        Geometry.Rotate(left, 0, 3, -2.70447); Geometry.Rotate(right, 0, 3, -2.70446);
        StructureValues p = FocusParameters(.24, ProjectionKind.Perspective);
        FrameNear(Focus(left, p), Focus(right, p), .01);
        p = FocusParameters(0, ProjectionKind.Perspective, 1.023802562641747);
        foreach (double separation in new[] { 1e-5, 1e-7 })
        {
            left = LL6SingularCamera(); right = LL6SingularCamera();
            Geometry.Rotate(left, 0, 3, -separation / 2); Geometry.Rotate(right, 0, 3, separation / 2);
            FocusFrame a = Focus(left, p), b = Focus(right, p);
            Bounded(a); Bounded(b); FrameNear(a, b, .01);
        }
        Equal(ViewFraming.Target(Array.Empty<double>(), q, p), FocusFrame.Neutral);
        Equal(ViewFraming.Target(new[] { double.NaN, 0, 0, 0, 0, 0, 0, double.PositiveInfinity }, q, p), FocusFrame.Neutral);
        // At a singular 4D denominator the fade is already zero.
        Equal(ViewFraming.Target(new[] { 0.0, 0, 0, p.D4 }, Geometry.Identity(), p), FocusFrame.Neutral);
    }

    private static double[] RandomCamera(Random random)
    {
        double[] q = Geometry.Identity();
        for (int a = 0; a < 4; a++) for (int b = a + 1; b < 4; b++) Geometry.Rotate(q, a, b, random.NextDouble() * 2 * Math.PI);
        return q;
    }

    private static void LL6FocusSweep()
    {
        var random = new Random(60006);
        foreach (ProjectionKind kind in Enum.GetValues<ProjectionKind>())
            for (int i = 0; i < 120; i++)
            {
                StructureValues p = FocusParameters(random.NextDouble() * .3, kind, 1.01 + random.NextDouble() * 2.99,
                    i % 3 == 0 ? .2 : i % 3 == 1 ? 2.8 : .2 + random.NextDouble() * 2.6);
                double[] anchors = ViewFraming.MovingRestAnchors(Mesh, Turn, 1 - p.Cs), q = RandomCamera(random);
                FocusFrame before = ViewFraming.Target(anchors, q, p); Bounded(before);
                int a = random.Next(4), b = (a + 1 + random.Next(3)) % 4;
                Geometry.Rotate(q, a, b, 1e-7);
                FocusFrame after = ViewFraming.Target(anchors, q, p); Bounded(after); FrameNear(before, after, 1e-3);
            }
        // One anchor crosses the fade into the empty set and out again.
        double[] point = { .8, .1, 0, .4 };
        StructureValues parameters = FocusParameters(0, ProjectionKind.Orthographic);
        FocusFrame previous = FocusFrame.Neutral; bool empty = false, nonempty = false;
        for (int i = 0; i <= 40000; i++)
        {
            double[] q = Geometry.Identity(); Geometry.Rotate(q, 0, 1, i * Math.PI / 20000);
            FocusFrame current = ViewFraming.Target(point, q, parameters); Bounded(current);
            if (i > 0) FrameNear(previous, current, .01);
            empty |= current.M == 1; nonempty |= current.M > 1;
            previous = current;
        }
        Check(empty && nonempty, "sweep did not enter and leave the empty set");
    }

    private static void LL6GuardContinuity()
    {
        var random = new Random(60601);
        double[] anchors = ViewFraming.MovingRestAnchors(Mesh, Turn, 0);
        Check(anchors.Chunk(4).All(a => Math.Sqrt(a.Sum(x => x * x)) < 4), "orthographic w guard unexpectedly reachable");
        foreach (ProjectionKind kind in Enum.GetValues<ProjectionKind>())
            foreach (var (guard, edge) in new[] { ("w", .5), ("w", 1.0), ("x", 1.0), ("x", 1.25),
                ("y", 1.0), ("y", 1.25), ("den", .02), ("den", .1) })
            {
                if (guard == "den" && kind == ProjectionKind.Orthographic) continue;
                if (guard == "w" && kind == ProjectionKind.Orthographic) continue; // w = 5-z stays above 1 for these rest anchors.
                StructureValues p = FocusParameters(0, kind, 1.01);
                bool found = false;
                for (int attempt = 0; attempt < 150 && !found; attempt++)
                {
                    double[] basis = RandomCamera(random);
                    int offset = random.Next(anchors.Length / 4) * 4, a = random.Next(3), b = 3;
                    double Measure(double angle)
                    {
                        double[] q = (double[])basis.Clone(); Geometry.Rotate(q, a, b, angle);
                        double[] world = Geometry.Multiply(q, anchors.AsSpan(offset, 4));
                        Projection projection = ViewFraming.Project(world, p);
                        return guard switch { "w" => projection.W, "x" => Math.Abs(projection.NdcX),
                            "y" => Math.Abs(projection.NdcY), _ => (kind == ProjectionKind.Perspective ? p.D4 : 1) - world[3] };
                    }
                    for (int segment = 0; segment < 48 && !found; segment++)
                    {
                        double lo = -Math.PI + segment * Math.PI / 24, hi = lo + Math.PI / 24;
                        double lowValue = Measure(lo) - edge, highValue = Measure(hi) - edge;
                        if (!double.IsFinite(lowValue) || !double.IsFinite(highValue) || lowValue * highValue >= 0) continue;
                        for (int step = 0; step < 48; step++)
                        {
                            double mid = (lo + hi) / 2, value = Measure(mid) - edge;
                            if (value * lowValue > 0) { lo = mid; lowValue = value; } else hi = mid;
                        }
                        double crossing = (lo + hi) / 2;
                        if (Math.Abs(Measure(crossing) - edge) > 1e-5) continue; // Skip singular discontinuities of the raw projection.
                        double[] left = (double[])basis.Clone(), right = (double[])basis.Clone();
                        Geometry.Rotate(left, a, b, crossing - 5e-8); Geometry.Rotate(right, a, b, crossing + 5e-8);
                        FocusFrame before = ViewFraming.Target(anchors, left, p), after = ViewFraming.Target(anchors, right, p);
                        Bounded(before); Bounded(after); FrameNear(before, after, 1e-3); found = true;
                    }
                }
                Check(found, $"seeded search did not cross {kind}/{guard}/{edge}");
            }
    }

    private static void LL6Follower()
    {
        var follower = new FrameFollower();
        FocusFrame target = Focus(LL6SingularCamera(), FocusParameters(0, ProjectionKind.Perspective, 1.023802562641747));
        Equal(follower.Step(FocusFrame.Neutral, .01), FocusFrame.Neutral);
        var random = new Random(60602);
        for (int i = 0; i < 1000; i++)
        {
            double dt = .001 + random.NextDouble() * .04;
            FocusFrame before = follower.Frame;
            FocusFrame next = follower.Step(i == 0 ? target : new FocusFrame(random.NextDouble() * 4 - 2, random.NextDouble() * 4 - 2,
                1 + random.NextDouble() * 2), dt);
            Check(Math.Abs(next.X - before.X) <= 2 * dt + 1e-12 && Math.Abs(next.Y - before.Y) <= 2 * dt + 1e-12 &&
                Math.Abs(next.M - before.M) <= 4 * dt + 1e-12, "follower exceeded per-step cap");
        }
        follower.Reset(); Equal(follower.Step(target, .01), target, "first visible step did not set target directly");
        foreach (double dt in new[] { 0, -.1, double.NaN, double.PositiveInfinity, double.NegativeInfinity })
            Equal(follower.Step(FocusFrame.Neutral, dt), target, "bad dt changed frame");
        follower.Reset(); follower.Step(FocusFrame.Neutral, .01);
        for (int i = 0; i < 100; i++) follower.Step(target, .01);
        FrameNear(follower.Frame, target, .002);
        for (int i = 0; i < 200; i++) follower.Step(target, .01);
        FrameNear(follower.Frame, target, 1e-6);
    }

}
