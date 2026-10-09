using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using LookLab.Core;
using LookLab.App.LL3;

internal static partial class Program
{
    private static void LL3Candidates()
    {
        using var temp = new TestFolder();
        var initial = new Preset("Replay", null, null, Schema.Defaults);
        var left = new SwipeSession(temp.Path, Schema, initial, 731);
        var right = new SwipeSession(temp.Path, Schema, initial, 731);
        var seen = new HashSet<ParameterKind>();
        Check(!Directory.Exists(Path.GetDirectoryName(left.RecordPath)), "constructing a swipe wrote outputs");
        for (int i = 0; i < 160; i++)
        {
            Equal(SwipeSession.Digest(left.Candidate, Schema), SwipeSession.Digest(right.Candidate, Schema), "seed replay differs");
            Schema.Validate(left.Candidate.Params);
            string[] actual = Schema.Parameters.Where(p => left.Best.Params.Values[p.Id] != left.Candidate.Params.Values[p.Id]).Select(p => p.Id).ToArray();
            Check(actual.Length >= 1 && actual.Length <= 3, "candidate must change one to three ids");
            Sequence(actual, left.ChangedIds);
            foreach (string id in actual)
            {
                seen.Add(Schema[id].Kind);
                if (left.Best.Params.Values[id] is NumberValue n && left.Candidate.Params.Values[id] is NumberValue c)
                    Check(Math.Abs(n.Value - c.Value) <= Schema[id].Step!.Value + 1e-12, "numeric candidate exceeded one schema step");
            }
            left.Answer(i % 3 == 0); right.Answer(i % 3 == 0);
        }
        Sequence(seen.OrderBy(k => k), Enum.GetValues<ParameterKind>().OrderBy(k => k), "generator did not cover all schema kinds");
        Equal(Directory.GetFiles(Path.GetDirectoryName(left.RecordPath)!).Length, 2, "swipe wrote extra files");
    }

    private static void LL3SwipeRecords()
    {
        using var temp = new TestFolder();
        var session = new SwipeSession(temp.Path, Schema, new Preset("Best", null, null, Schema.Defaults), 0);
        Preset best = session.Best, candidate = session.Candidate;
        string[] changed = session.ChangedIds.ToArray();
        session.Answer(true); Check(ReferenceEquals(session.Best, candidate), "better did not promote candidate");
        Verify(1, best, candidate, "better", changed);
        best = session.Best; candidate = session.Candidate; changed = session.ChangedIds.ToArray();
        session.Answer(false); Check(ReferenceEquals(session.Best, best), "worse promoted candidate");
        Verify(2, best, candidate, "worse", changed);
        Check(!File.ReadAllBytes(session.RecordPath).Take(3).SequenceEqual(new byte[] { 239, 187, 191 }), "record has a UTF-8 BOM");

        void Verify(int count, Preset before, Preset compared, string verdict, string[] ids)
        {
            string[] lines = File.ReadAllLines(session.RecordPath); Equal(lines.Length, count);
            using var document = JsonDocument.Parse(lines[^1]); var record = document.RootElement;
            Equal(record.EnumerateObject().Count(), 5);
            Check(DateTimeOffset.TryParse(record.GetProperty("time").GetString(), out _), "invalid record timestamp");
            Equal(record.GetProperty("best_sha256").GetString(), SwipeSession.Digest(before, Schema));
            Equal(record.GetProperty("candidate_sha256").GetString(), SwipeSession.Digest(compared, Schema));
            Equal(record.GetProperty("verdict").GetString(), verdict);
            Sequence(record.GetProperty("changed_parameter_ids").EnumerateArray().Select(e => e.GetString()), ids);
        }
    }

    private static void LL3Compare()
    {
        var preset = new Preset("Compare", null, null, Schema.Defaults);
        foreach (int count in new[] { 2, 3, 4 })
        {
            double now = 0;
            var session = new CompareSession(Schema, Enumerable.Repeat(preset, count).ToArray(), Turn, () => now);
            session.Camera.Drag(30, -10);
            now = preset.Params.Number("turnMs") * 1.5; session.Advance();
            foreach (ComparePane pane in session.Panes)
            {
                Check(ReferenceEquals(pane.Camera, session.Camera) && ReferenceEquals(pane.Clock, session.Clock), "pane has a separate camera or clock");
                Near(pane.Frame.Phase, .5); Equal(pane.Frame.BoundRevision, 1L);
                Check(ReferenceEquals(pane.Frame, session.Panes[0].Frame), "panes sampled separately");
            }
            TurnFrame previous = session.Panes[0].Frame;
            session.ClockEnabled = false; now *= 2; session.Advance();
            Check(ReferenceEquals(previous, session.Panes[0].Frame) && session.Clock.Frame().BoundRevision != previous.BoundRevision,
                "disabled-clock fault did not stop the handler");
        }
        foreach (int count in new[] { 1, 5 }) Refuse(() => new CompareSession(Schema, Enumerable.Repeat(preset, count).ToArray(), Turn, () => 0), "compare");
    }

    private static void LL3CaptureRules()
    {
        Equal(CaptureEncoding.Frames(1), 24); Equal(CaptureEncoding.Frames(10), 240);
        foreach (double seconds in new[] { 0.0, -1, 10.01, double.NaN, double.PositiveInfinity }) Refuse(() => CaptureEncoding.Frames(seconds), "capture");
        string[] args = CaptureEncoding.Arguments(321, 241, 10, "clip.mp4").ToArray();
        Equal(args[Array.IndexOf(args, "-frames:v") + 1], "240");
        Equal(args[Array.IndexOf(args, "-framerate") + 1], "24");
        Equal(args[Array.IndexOf(args, "-t") + 1], "10");
        Equal(args[Array.IndexOf(args, "-video_size") + 1], "321x241");
        Check(args.Contains("libx264") && args.Contains("pipe:0"), "clip must use local FFmpeg and native raw frames");
        using var temp = new TestFolder();
        string source = OutputPaths.FileAt(temp.Path, CaptureEncoding.Folder, "prepared.png");
        Directory.CreateDirectory(Path.GetDirectoryName(source)!); File.WriteAllBytes(source, new byte[] { 1, 2, 3 });
        string gallery = CaptureEncoding.Publish(temp.Path, source);
        Equal(Path.GetRelativePath(temp.Path, gallery).Replace('\\', '/'), "work/gallery/looklab/prepared.png");
        Sequence(File.ReadAllBytes(gallery), File.ReadAllBytes(source));
        bool refused = false;
        try { OutputPaths.FileAt(temp.Path, "../outside", "bad.png"); } catch (IOException) { refused = true; }
        Check(refused, "capture output escaped its root");
        string? previous = Environment.GetEnvironmentVariable("LOOKLAB_FFMPEG");
        try { Environment.SetEnvironmentVariable("LOOKLAB_FFMPEG", Path.Combine(temp.Path, "absent.exe")); Check(CaptureEncoding.FindFfmpeg() == null, "missing FFmpeg must disable clips"); }
        finally { Environment.SetEnvironmentVariable("LOOKLAB_FFMPEG", previous); }
    }

    private static void LL3PythonChecks()
    {
        var result = Run("python", new[] { "-B", FileAt("tests/ll3_capture_test.py") });
        Check(result.Code == 0, result.Out + result.Error);
        Console.WriteLine(result.Error.Trim());
    }
}
