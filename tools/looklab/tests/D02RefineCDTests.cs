using System;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

internal static partial class Program
{
    private static void D02ChosenSliceAndRepairStatus()
    {
        var slice = new CutSlice();
        slice.StepItem(-1);
        slice.StepItem(-1);
        Equal(slice.Item, 59, "the selector wraps through the identity to the last item");
        Check(slice.Press(150, .4) && slice.Named == null, "the held turn must not be named");
        slice.StepItem(1);
        var named = slice.Release()!;
        Equal(named, new NamedTwist(150, 59, .4), "the held item is the one named on release");
        var checks = new KeptChecks(new ReleaseRecord(), named.Cap, 300);
        var check = new TwistCheck(named.Item, named.Cap, 2, 0, -4, null, 0);
        Check(checks.Complete(checks.Release(named.Item), check) && checks.Entries.Single() is CheckRow row
            && row.Item == named.Item && row.Check.Nc == 2, "the selected turn's check belongs in the kept list");

        var model = new PreviewModel { Head = () => 300 };
        var preview = model.Show(Origin.Solver, "repair", "Repair list", new[] { "150·2", "50·3" }, PreviewStatus.Ready,
            null, new[] { new OrbitEffect("Orbit 1", 2, 4) }, "Checked", 0, wholeOnly: true);
        byte[] moved = { 1, 1, 1, 0 }, prefix = { 1, 1, 0, 0 };
        var ready = StatusPatterns.PreviewStatuses(4, new[] { 0, 3 }, null, StatusPatterns.RepairFeed(preview, 300, moved, null));
        var partial = StatusPatterns.PreviewStatuses(4, new[] { 0, 3 }, null, StatusPatterns.RepairFeed(preview, 300, moved, prefix));
        var stale = StatusPatterns.PreviewStatuses(4, new[] { 0, 3 }, null, StatusPatterns.RepairFeed(preview, 301, moved, prefix));
        Check(ready[StatusPatterns.Conflict - 1]!.SequenceEqual(new[] { 0 })
            && ready[StatusPatterns.Preview - 1]!.SequenceEqual(new[] { 0, 1, 2 }), "the live repair must supply preview and protection conflict");
        Check(partial[StatusPatterns.UncheckedPrefix - 1]!.SequenceEqual(new[] { 0, 1 })
            && partial[StatusPatterns.Preview - 1] == null, "only the selected prefix should get the unchecked pattern");
        Check(stale[StatusPatterns.Stale - 1]!.SequenceEqual(new[] { 0, 1, 2 }) && stale[StatusPatterns.Conflict - 1] == null,
            "a revision change must show the stale effect");
    }

    private static void D02CutCellsUnavailableOnSlice()
    {
        using var sample = JsonDocument.Parse(Read("data/samples/jumble/wj-ia-300/cap-150.json"));
        Check(sample.RootElement.GetProperty("masks").TryGetProperty("inside", out _)
            && !sample.RootElement.TryGetProperty("cut_cells", out _), "the sample carries an inside set without cut-cell geometry");
        var slice = new CutSlice();
        foreach (var (cap, blocked) in new[] { (150, false), (0, true) })
            Check(slice.Line(cap, blocked).Contains(CutSlice.CutCellsUnavailable), "the cut slice must show unavailable cut cells");
        Check(slice.Line(null, false) == "", "no slice line appears without a cap");
    }

    private static void D02RawSubmitsUnitShrinks()
    {
        using var local = JsonDocument.Parse(Read("data/local-view.json"));
        var gap = local.RootElement.GetProperty("gap").GetSingle();
        var sticker = local.RootElement.GetProperty("stickerShrink").GetSingle();
        var raw = new RawView();
        var before = RawView.Shrinks(raw.On, gap, sticker);
        raw.Toggle();
        Equal(RawView.Shrinks(raw.On, gap, sticker), (1f, 1f), "raw must draw both shrinks at one");
        raw.Toggle();
        Equal(RawView.Shrinks(raw.On, gap, sticker), before, "exit must restore Local's shrinks");
        Equal(before, (1 - gap, sticker), "Local's configured shrinks");
    }

    private static void D02RingSheetFitsAtOpening()
    {
        using var opening = JsonDocument.Parse(Read("data/opening.json"));
        var fov = opening.RootElement.GetProperty("params").GetProperty("fieldOfView").GetDouble();
        var hopf = 1 / Math.Tan(fov / 2);
        Equal(JumbleViews.OpeningZoom(hopf, null), hopf, "the Hopf rings keep their opening field of view");
        double chart = .6 * Math.Sqrt((96 - 32 * Math.Sqrt(5)) / (12 + 4 * Math.Sqrt(5)));
        foreach (double aspect in new[] { 582.0 / 196, 1.0, .6 })
        {
            double zoom = JumbleViews.OpeningZoom(hopf, aspect);
            for (int row = 0; row < 20; row++)
                for (int column = 0; column < 30; column++)
                {
                    var x = Math.Abs((column - 14.5 + .25) * .6 * .68) + chart;
                    var y = Math.Abs((row - 9.5) * .6 * .57) + chart;
                    Check(x * zoom <= aspect * (5 - chart) && y * zoom <= 5 - chart, "a sheet chart clips at opening");
                }
        }
    }

    private static void D02PieceFieldsUseMasksAndSavedSets()
    {
        var model = new PieceFilterModel(new[] { new PoseRow(0, 0, 2), new PoseRow(1, 4, 2) }, 0, 2, () => 300, "sample");
        bool[] inside = { true, false, true, false }, straddlers = { false, true, false, true };
        bool[]? moved = { false, false, true, true };
        model.BindPieces(new[] { 0, 0, 1, 1 }, field => field.Name switch
        {
            "inside-a-cap" when field.Cap == 150 => p => inside[p],
            "straddler" => p => straddlers[p],
            "moved" when moved != null => p => moved[p],
            "class" => p => field.Comparison!.Matches(p < 2 ? 1 : 2),
            _ => null,
        });
        FilterPreview? Run(string expr) { model.SetText(expr); return model.Run(); }
        Equal(Run("inside-a-cap 150 and height >= 4")?.Count, 1L, "inside and height must intersect by piece");
        Check(model.Apply(UserEvent.Key("apply")) == null && model.Member() is { } member && member(2) && !member(3),
            "focus and preview must use the same subset of a pose row");
        Check(model.Save("Cap set") == null, "save the shown members");
        Equal(Run("set \"Cap set\" and class = 2")?.Count, 1L, "the saved set can be composed with a class");
        Equal(Run("straddler and moved")?.Count, 1L, "movement reads the current mask");
        Check(model.Apply(UserEvent.Key("apply")) == null, "apply the current moved set");
        moved = null;
        model.RefreshFields();
        Check(model.Member() == null && model.PreviewLine().Contains("unavailable"), "dropped movement data must not hide every piece");
        foreach (string expr in new[] { "moved", "not rim", "lattice or seam", "inside-a-cap 12", "set unknown" })
            Check(Run(expr) == null && model.Error!.Contains("unavailable"), expr + " must report unavailable data");

        string[] fields = FilterParser.Fields.Split(", ");
        foreach (var (file, skip) in new[] { ("screen-words.json", 0), ("jumble-words.json", 3) })
        {
            using var words = JsonDocument.Parse(Read("data/design/" + file));
            Check(words.RootElement.GetProperty("filter_fields").EnumerateArray().Select(x => x.GetString()).SequenceEqual(fields.Skip(skip)),
                "the recorded filter fields must match the parser");
        }
    }
}
