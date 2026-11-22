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
