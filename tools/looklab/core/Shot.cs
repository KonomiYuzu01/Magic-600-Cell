using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace LookLab.Core;

public sealed record ShotSegment(int Frames, double DragX, double DragY);
public sealed record ShotTurn(int Start, int Frames);
public sealed record ShotState(int Frame, double[] Q, double[] Spin, TurnFrame Turn);

public sealed class Shot
{
    public const string Format = "magic600-look-shot";
    public const int FrameRate = 24;
    public const double MaxSeconds = 10;
    public string Id { get; }
    public string Name { get; }
    public int Width { get; }
    public int Height { get; }
    public double Seconds { get; }
    public int Frames { get; }
    public int HeroFrame { get; }
    public double EaseA { get; }
    public double EaseB { get; }
    public IReadOnlyList<ShotSegment> Camera { get; }
    public IReadOnlyList<ShotTurn> Turns { get; }
    public IReadOnlyList<int> SpotFrames { get; }

    private Shot(string id, string name, int[] size, double seconds, int frames, int hero, double[] ease,
        ShotSegment[] camera, ShotTurn[] turns)
    {
        Id = id; Name = name; Width = size[0]; Height = size[1]; Seconds = seconds; Frames = frames;
        HeroFrame = hero; EaseA = ease[0]; EaseB = ease[1];
        Camera = Array.AsReadOnly(camera); Turns = Array.AsReadOnly(turns);
        SpotFrames = Array.AsReadOnly(new[] { 0, hero, frames - 1 });
    }

    public static Shot Load(string path) => Parse(File.ReadAllText(path));

    private static int FrameCount(double seconds, string id, bool positive)
    {
        double count = seconds * FrameRate, rounded = Math.Round(count);
        if (seconds < 0 || count > MaxSeconds * FrameRate + 1e-9 || Math.Abs(count - rounded) > 1e-9 ||
            (positive && (seconds <= 0 || rounded < 1)))
            throw Json.Error(id, "expected frame-aligned time within ten seconds" + (positive ? " with at least one frame" : ""));
        return (int)rounded;
    }

    public static Shot Parse(string text)
    {
        using var document = Json.Parse(text);
        JsonElement root = document.RootElement;
        Json.Header(root, Format);
        Json.Keys(root, "format", "version", "id", "name", "size", "seconds", "heroFrame", "ease", "camera", "turns");
        string id = Json.Text(root, "id"), name = Json.Text(root, "name");
        if (!Regex.IsMatch(id, @"\A[a-z0-9][a-z0-9-]*\z", RegexOptions.CultureInvariant)) throw Json.Error("id", "expected a lowercase shot id");
        if (string.IsNullOrWhiteSpace(name)) throw Json.Error("name", "nonempty name required");
        double seconds = Json.Number(Json.Get(root, "seconds"), "seconds");
        int frames = FrameCount(seconds, "seconds", true);
        int[] size = Json.Array(Json.Get(root, "size"), "size").Select(x => Json.Integer(x, "size")).ToArray();
        if (size.Length != 2 || size[0] < 64 || size[0] > 3840 || size[1] < 64 || size[1] > 2160 || size.Any(x => x % 2 != 0))
            throw Json.Error("size", "expected even width 64 to 3840 and height 64 to 2160");
        int hero = Json.Integer(Json.Get(root, "heroFrame"), "heroFrame");
        if (hero < 1 || hero > frames - 2) throw Json.Error("heroFrame", "must be between the first and last frames");
        double[] ease = Json.Array(Json.Get(root, "ease"), "ease").Select(x => Json.Number(x, "ease")).ToArray();
        if (ease.Length != 2) throw Json.Error("ease", "two controls required");
        Easing.Ease(0, ease[0], ease[1]);
        ShotSegment[] camera = Json.Array(Json.Get(root, "camera"), "camera").Select(segment =>
        {
            Json.Keys(segment, "seconds", "dragX", "dragY");
            int n = FrameCount(Json.Number(Json.Get(segment, "seconds"), "seconds"), "seconds", true);
            double Drag(string field)
            {
                double value = Json.Number(Json.Get(segment, field), field);
                if (Math.Abs(value) > 2 * Math.PI) throw Json.Error(field, "absolute drag must be at most two pi");
                return value;
            }
            return new ShotSegment(n, Drag("dragX"), Drag("dragY"));
        }).ToArray();
        if (camera.Length == 0 || camera.Sum(s => (long)s.Frames) != frames) throw Json.Error("camera", "segments must sum to the shot's frame count");
        ShotTurn[] turns = Json.Array(Json.Get(root, "turns"), "turns").Select(t =>
        {
            Json.Keys(t, "start", "seconds");
            return new ShotTurn(FrameCount(Json.Number(Json.Get(t, "start"), "start"), "start", false),
                FrameCount(Json.Number(Json.Get(t, "seconds"), "seconds"), "seconds", true));
        }).ToArray();
        int end = 0;
        foreach (ShotTurn t in turns)
        {
            if (t.Start < end || t.Start + t.Frames > frames) throw Json.Error("turns", "turns must be ordered, disjoint and inside the shot");
            end = t.Start + t.Frames;
        }
        return new Shot(id, name, size, seconds, frames, hero, ease, camera, turns);
    }

    public ShotState[] Track(TurnData turn)
    {
        var track = new ShotState[Frames];
        int segment = 0, segmentEnd = Camera[0].Frames, revision = 0;
        for (int k = 0; k < Frames; k++)
        {
            double[] q = k == 0 ? Geometry.Identity() : track[k - 1].Q.ToArray();
            double[] spin = k == 0 ? Geometry.Identity() : track[k - 1].Spin.ToArray();
            if (k > 0)
            {
                while (k > segmentEnd) segmentEnd += Camera[++segment].Frames;
                ShotSegment s = Camera[segment];
                GlobalLayout.Drag(q, spin, s.DragX / s.Frames, s.DragY / s.Frames);
            }
            while (revision < Turns.Count && Turns[revision].Start + Turns[revision].Frames <= k) revision++;
            bool active = revision < Turns.Count && k >= Turns[revision].Start;
            double phase = active ? (k - Turns[revision].Start) / (double)Turns[revision].Frames : 0;
            double theta = active ? (revision % 2 == 0 ? 1 : -1) * turn.Angle * Easing.Ease(phase, EaseA, EaseB) : 0;
            ReadOnlyMemory<uint> labels = turn.Snapshot(revision);
            turn.ValidateBinding(revision, labels.Span);
            track[k] = new ShotState(k, q, spin, new TurnFrame(revision, phase, theta, revision, labels));
        }
        return track;
    }

    // The ring sheet ignores the camera, so only a turn angle is sure to show in every look; the first and last frames may render alike.
    public bool HeroShowsTurn(IReadOnlyList<ShotState> track) =>
        track[HeroFrame].Turn.Theta != track[0].Turn.Theta && track[HeroFrame].Turn.Theta != track[Frames - 1].Turn.Theta;

    public static string TrackDigest(IReadOnlyList<ShotState> track)
    {
        if (track.Count == 0) throw Json.Error("track", "nonempty track required");
        var bytes = new byte[track.Count * 272];
        int offset = 0;
        void Double(double value) { BinaryPrimitives.WriteDoubleLittleEndian(bytes.AsSpan(offset, 8), value); offset += 8; }
        foreach (ShotState state in track)
        {
            foreach (double value in state.Q) Double(value);
            foreach (double value in state.Spin) Double(value);
            BinaryPrimitives.WriteInt64LittleEndian(bytes.AsSpan(offset, 8), state.Turn.BoundRevision); offset += 8;
            Double(state.Turn.Theta);
        }
        return AssetCatalogue.Digest(bytes);
    }
}
