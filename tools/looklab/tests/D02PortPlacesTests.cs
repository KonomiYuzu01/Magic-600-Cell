using System;
using System.IO;
using System.Linq;
using LookLab.Core;
using Magic600.Chrome;

// ADR-D02 build, track C, item 2 (ADR-D01.62): the strip's anchors in a view the product's core draws.
internal static partial class Program
{
    private static CorePlaces? corePlaces;
    private static CorePlaces CoreModel => corePlaces ??= CorePlaces.Load(Path.Combine(Root, "assets"));

    // The sample mapping is Look Lab's own: the same slot stands for each position of every orbit.
    private static void D02PortPlacesMapPositionsAsLookLab()
    {
        var lab = new ScenePlaces(Root, Mesh);
        Check(CoreModel.Orbits == lab.Orbits && CoreModel.Orbits == OrbitAtlas.Count, "orbits: " + CoreModel.Orbits + " here, " + lab.Orbits + " in Look Lab");
        for (int orbit = 0; orbit < CoreModel.Orbits; orbit++)
        {
            Check(CoreModel.PiecesOf(orbit) == lab.Pieces(orbit), $"O{orbit:00}: {CoreModel.PiecesOf(orbit)} pieces here, {lab.Pieces(orbit)} in Look Lab");
            foreach (int position in new[] { 0, 1, lab.Pieces(orbit) / 2, lab.Pieces(orbit) - 1 })
                Check(CoreModel.Slot(orbit, position) == lab.Slot(orbit, position), $"O{orbit:00} position {position}: slot {CoreModel.Slot(orbit, position)} here, {lab.Slot(orbit, position)} in Look Lab");
        }
        Check(CoreModel.Slot(0, -1) == null && CoreModel.Slot(0, CoreModel.PiecesOf(0)) == null && CoreModel.Slot(99, 0) == null, "a position outside its orbit has a slot");
    }

    // The core's projection of a position's shrink anchor lands where Look Lab's projection of the sticker's vertices
    // lands (their mean), within a small part of the view, in perspective and orthographic, still and turned.
    private static void D02PortPlacesFollowTheCoresProjection()
    {
        var lab = new ScenePlaces(Root, Mesh); var layout = new GlobalLayout(Mesh.CellStructure); double[] spin = Geometry.Identity();
        double[] turned = Geometry.Identity(); Geometry.Rotate(turned, 0, 3, .3); Geometry.Rotate(turned, 1, 2, -.2);
        foreach (bool orthographic in new[] { false, true })
            foreach (double[] q in new[] { Geometry.Identity(), turned })
            {
                // Look Lab keeps its camera column-major; the core's view takes it row-major (x' = Q x).
                var view = new CoreView(Enumerable.Range(0, 16).Select(i => q[i % 4 * 4 + i / 4]).ToArray(), .76, .82, 1.18, 1.15, 1.5, orthographic);
                var values = new StructureValues(.76, .82, 1.18, 1.15, 1.5, orthographic ? ProjectionKind.Orthographic : ProjectionKind.Perspective);
                int compared = 0;
                for (int orbit = 0; orbit < CoreModel.Orbits; orbit += 3)
                    for (int position = 0; position < CoreModel.PiecesOf(orbit); position += Math.Max(1, CoreModel.PiecesOf(orbit) / 7))
                    {
                        int slot = CoreModel.Slot(orbit, position)!.Value;
                        (double X, double Y)? mine = CoreModel.Place(slot, view);
                        (double X, double Y)? theirs = ScenePlaces.InView(lab.Project(slot, values, q, spin, Turn, 0, layout, FocusFrame.Neutral));
                        if (mine == null || theirs == null)
                        {
                            // Only at the pane's edge may one of them be off it.
                            Check((mine ?? theirs) is not { } shown || Near(shown), $"slot {slot}: drawn here {mine != null}, in Look Lab {theirs != null}");
                            continue;
                        }
                        Check(Math.Abs(mine.Value.X - theirs.Value.X) < .01 && Math.Abs(mine.Value.Y - theirs.Value.Y) < .01,
                            $"slot {slot} ({(orthographic ? "orthographic" : "perspective")}): {mine} here, {theirs} in Look Lab");
                        compared++;
                    }
                Check(compared > 20, "only " + compared + " places compared");
            }
        // Off the pane, nothing: a zoom that pushes the slot out of the view.
        int any = CoreModel.Slot(0, 0)!.Value;
        Check(CoreModel.Place(any, new CoreView(CoreView.Identity, .76, .82, 1.18, 400, 1)) == null, "a slot off the pane has a place");
        static bool Near((double X, double Y) p) => p.X < .02 || p.X > .98 || p.Y < .02 || p.Y > .98;
    }

    // A focused view (Local) draws only the focus cell's cap: the camera takes the pole to +w, a slot of the focus
    // cell has a place and a slot of the antipodal side has none; the spread moves a place outwards from the middle.
    private static void D02PortPlacesFocusAsTheCore()
    {
        const int cell = 117;
        double[] pole = CoreModel.CellCentre(cell);
        Check(Math.Abs(pole.Sum(x => x * x) - 1) < 1e-9, "a cell centre is not a unit vector");
        double[] q = CorePlaces.FocusedCamera(CoreView.Identity, pole);
        double[] w = new double[4]; for (int r = 0; r < 4; r++) for (int k = 0; k < 4; k++) w[r] += q[r * 4 + k] * pole[k];
        Check(Math.Abs(w[3] - 1) < 1e-9 && Math.Abs(w[0]) + Math.Abs(w[1]) + Math.Abs(w[2]) < 1e-9, "the focused camera does not take the pole to +w");
        var local = new CoreView(CoreView.Identity, .7, .6, 2, 1, 1, true, pole, 121.0 / 125, 1);
        int mine = cell * CorePlaces.StickersPerCell + 200;
        (double X, double Y)? centre = CoreModel.Place(mine, local);
        Check(centre != null, "a slot of the focus cell has no place in Local");
        int far = Enumerable.Range(0, CorePlaces.Cells).OrderBy(c => CoreModel.CellCentre(c).Zip(pole, (a, b) => a * b).Sum()).First() * CorePlaces.StickersPerCell;
        Check(CoreModel.Place(far, local) == null, "a slot of the far side has a place in Local");
        (double X, double Y)? spread = CoreModel.Place(mine, local with { Spread = 1.7 });
        Check(spread != null && Math.Abs(spread.Value.X - .5) + Math.Abs(spread.Value.Y - .5) >= Math.Abs(centre!.Value.X - .5) + Math.Abs(centre.Value.Y - .5) - 1e-9,
            $"the spread moved {centre} to {spread}, not outwards");
    }
}
