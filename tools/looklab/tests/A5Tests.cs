using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using LookLab.Core;

internal static partial class Program
{
    private static ViewState SampleView()
    {
        double[] q = Geometry.Identity(), spin = Geometry.Identity();
        GlobalLayout.Drag(q, spin, .37, -.21);
        var ui = new Dictionary<string, string> { ["greybox.open"] = "1", ["greybox.context"] = "solve" };
        return new ViewState(q, spin, ViewStates.Digest(Encoding.UTF8.GetBytes(Read("presets/default.json"))),
            new Preset("Kept", null, null, Schema.Defaults.With("gap", SmokeProof.NonDefault(Schema["gap"]))), ui);
    }

    private static void A5ViewStateRoundTripAndRefusals()
    {
        ViewState view = SampleView();
        byte[] bytes = ViewStates.Bytes(view, Schema);
        string text = Encoding.UTF8.GetString(bytes);
        ViewState back = ViewStates.Parse(text, Schema);
        Sequence(back.Q, view.Q); Sequence(back.Spin, view.Spin); Equal(back.DefaultDigest, view.DefaultDigest);
        Check(ViewStates.Bytes(back, Schema).AsSpan().SequenceEqual(bytes), "view bytes differ after a round trip");
        Check(!text.Contains('\r') && text.EndsWith("}\n", StringComparison.Ordinal), "view file is not LF with a final newline");
        Refuse(() => ViewStates.Parse(text.Replace("\"version\": 1", "\"version\": 2"), Schema), "version");
        Refuse(() => ViewStates.Parse(text.Replace("\"ui\"", "\"extra\": 1, \"ui\""), Schema), "extra");
        Refuse(() => ViewStates.Parse(text.Replace(view.DefaultDigest, "abc"), Schema), "defaultPreset");
        Refuse(() => ViewStates.Parse(text.Replace("\"greybox.open\": \"1\"", "\"greybox.open\": 1"), Schema), "greybox.open");
        double[] skew = view.Q.ToArray(); skew[0] += .01;
        Refuse(() => ViewStates.Bytes(view with { Q = skew }, Schema), "q");
        Refuse(() => ViewStates.Bytes(view with { Spin = new double[15] }, Schema), "spin");
        Refuse(() => ViewStates.Bytes(view with { Preset = view.Preset with { Params = view.Preset.Params.With("gap", new NumberValue(2)) } }, Schema), "gap");
    }

    private static void A5KeptLookFollowsTheDefaultOnDisk()
    {
        ViewState view = SampleView();
        var current = new Preset("Default", null, null, Schema.Defaults);
        Check(ReferenceEquals(ViewStates.Resolve(view, current, view.DefaultDigest), view.Preset), "kept look dropped although the default is unchanged");
        Check(ReferenceEquals(ViewStates.Resolve(view, current, new string('0', 64)), current), "kept look hid a changed default");
    }

    private static void A5OpeningLookAppliesInRangeValuesOnly()
    {
        var preset = new Preset("Default", null, null, Schema.Defaults);
        string text = Read("data/opening.json");
        Preset opened = OpeningLook.Apply(preset, text, Schema);
        Check(opened.Params.Number("gap") > preset.Params.Number("gap") && opened.Params.Number("stickerShrink") < preset.Params.Number("stickerShrink"),
            "the opening look does not make cells and stickers smaller");
        Equal(opened.Params.Values.Count(v => !Equals(v.Value, preset.Params.Values[v.Key])), 4);
        Equal(opened.Params.Choice("projection"), "hopfRings");
        Refuse(() => OpeningLook.Apply(preset, text.Replace("0.3", "0.31"), Schema), "gap");
        Refuse(() => OpeningLook.Apply(preset, text.Replace("\"gap\"", "\"mystery\""), Schema), "mystery");
        Refuse(() => OpeningLook.Apply(preset, text.Replace("magic600-look-opening", "magic600-look-preset"), Schema), "format");
    }

    private static void A5LocalShapeCentresTheFocusCells()
    {
        string text = Read("data/local-view.json");
        LocalShape shape = LocalShape.Parse(text, Schema);
        // Three cells near one another: the frame takes their middle to +w and draws a cap around them.
        float[] centres = { .6f, 0, 0, .8f, 0, .6f, 0, .8f, 0, 0, .6f, .8f, 0, 0, 0, -1 };
        LocalFrame frame = shape.Frame(centres, new[] { 0, 1, 2, 1 }, Geometry.Identity(), 2);
        double[] turned = new double[4];
        for (int i = 0; i < 4; i++) for (int j = 0; j < 4; j++) turned[i] += frame.Q[j * 4 + i] * frame.Axis[j];
        Check(Math.Abs(turned[3] - 1) < 1e-9 && Math.Abs(turned[0]) < 1e-9 && Math.Abs(turned[1]) < 1e-9 && Math.Abs(turned[2]) < 1e-9, "the focus is not turned to the middle of the view");
        for (int a = 0; a < 4; a++) for (int b = 0; b < 4; b++)
        {
            double dot = 0; for (int i = 0; i < 4; i++) dot += frame.Q[a * 4 + i] * frame.Q[b * 4 + i];
            Check(Math.Abs(dot - (a == b ? 1 : 0)) < 1e-9, "the Local frame is not a rotation");
        }
        double widest = Math.Acos(.6 * frame.Axis[0] + .8 * frame.Axis[3]);
        Check(Math.Abs(Math.Acos(frame.CapCos) - widest - shape.Margin) < 1e-6, "the cap is not the focus cells plus the margin");
        Check(frame.CapCos > -.99 * 1 && .6 * 0 + -1 * frame.Axis[3] < frame.CapCos, "the far cell is inside the cap");
        Check(frame.Zoom > 0 && shape.Frame(centres, new[] { 0 }, Geometry.Identity(), 2).Zoom > frame.Zoom, "a smaller focus is not drawn larger");
        Equal(shape.Frame(centres, new[] { 3 }, Geometry.Identity(), 1).Q[15], -1.0);
        Refuse(() => LocalShape.Parse(text.Replace("0.3,", "0.31,"), Schema), "gap");
        Refuse(() => LocalShape.Parse(text.Replace("orthographic", "hopfRings"), Schema), "projection");
        Refuse(() => LocalShape.Parse(text.Replace("0.86", "1.2"), Schema), "fill");
        Refuse(() => LocalShape.Parse(text.Replace("1.7", "3.5"), Schema), "spread");
        Check((shape with { Spread = 1 }).Frame(centres, new[] { 0, 1, 2 }, Geometry.Identity(), 2).Zoom > frame.Zoom, "spread cells do not take more room");
        Refuse(() => LocalShape.Parse(text.Replace("magic600-look-local-view", "magic600-look-opening"), Schema), "format");
    }

    private static void A5DataWatchReportsAddedChangedAndRemoved()
    {
        using var folder = new TestFolder();
        Directory.CreateDirectory(Path.Combine(folder.Path, "data/layouts"));
        string a = Path.Combine(folder.Path, "data/layouts/a.json"), table = Path.Combine(folder.Path, "table.json");
        File.WriteAllText(a, "{}"); File.WriteAllText(table, "{}"); File.WriteAllText(Path.Combine(folder.Path, "data/notes.txt"), "x");
        var watch = new DataWatch(folder.Path, new[] { "data", "missing" }, new[] { "table.json", "absent.json" });
        Equal(watch.Changed().Count, 0);
        File.WriteAllText(a, "{ }"); File.WriteAllText(Path.Combine(folder.Path, "data/b.json"), "{}");
        File.WriteAllText(Path.Combine(folder.Path, "data/notes.txt"), "changed");
        Sequence(watch.Changed(), new[] { "data/b.json", "data/layouts/a.json" });
        Equal(watch.Changed().Count, 0);
        File.SetLastWriteTimeUtc(table, File.GetLastWriteTimeUtc(table).AddSeconds(2)); File.Delete(a);
        Sequence(watch.Changed(), new[] { "data/layouts/a.json", "table.json" });
    }
}
