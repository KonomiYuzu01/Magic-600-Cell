using System;
using System.Buffers.Binary;
using System.IO;
using System.Linq;
using System.Text.Json;

namespace LookLab.Core;

public static class Easing
{
    public static double Ease(double t, double a, double b)
    {
        if (!double.IsFinite(t) || !double.IsFinite(a) || !double.IsFinite(b) || a < 0 || a > 1 || b < 0 || b > 1)
            throw Json.Error("ease", "finite controls in [0,1] required");
        t = Math.Clamp(t, 0, 1);
        if (t == 0 || t == 1) return t;
        double lo = 0, hi = 1;
        for (int i = 0; i < 40; i++)
        {
            double s = (lo + hi) / 2;
            double x = 3 * (1 - s) * (1 - s) * s * a + 3 * (1 - s) * s * s * (1 - b) + s * s * s;
            if (x < t) lo = s; else hi = s;
        }
        double mid = (lo + hi) / 2;
        return 3 * (1 - mid) * mid * mid + mid * mid * mid;
    }

    public static double TurnAngle(double elapsedMs, double turnMs, double angle, double a, double b)
    {
        if (!double.IsFinite(turnMs) || turnMs <= 0 || !double.IsFinite(angle)) throw Json.Error("turnMs", "invalid duration or angle");
        return angle * Ease(elapsedMs / turnMs, a, b);
    }
}

public sealed class TurnData
{
    private readonly bool[] moving = new bool[Geometry.Slots];
    private readonly int[] source, destination, inverseSource, inverseDestination;
    private readonly uint[] even, odd;
    public string ModelId { get; }
    public double Angle { get; }
    public ReadOnlyMemory<double> PlaneU { get; }
    public ReadOnlyMemory<double> PlaneV { get; }
    public string EvenDigest { get; }
    public string OddDigest { get; }

    public TurnData(string path)
    {
        using var document = Json.Parse(File.ReadAllText(path));
        JsonElement root = document.RootElement;
        if (Json.Text(root, "format") != "magic600-sb-turn-v1") throw Json.Error("format", "expected magic600-sb-turn-v1");
        ModelId = Json.Text(root, "model_id");
        Angle = Json.Number(Json.Get(root, "angle"), "angle");
        double[] u = Plane(root, "plane_u"), v = Plane(root, "plane_v");
        if (Math.Abs(Geometry.Dot(u, u) - 1) > 1e-9 || Math.Abs(Geometry.Dot(v, v) - 1) > 1e-9 || Math.Abs(Geometry.Dot(u, v)) > 1e-9)
            throw Json.Error("plane", "orthonormal turn plane required");
        PlaneU = u;
        PlaneV = v;
        foreach (int slot in Slots(root, "moving_slots")) moving[slot] = true;
        source = Slots(root, "move_src"); destination = Slots(root, "move_dst");
        inverseSource = Slots(root, "inverse_src"); inverseDestination = Slots(root, "inverse_dst");
        CheckPermutation(source, destination, "move");
        CheckPermutation(inverseSource, inverseDestination, "inverse");
        if (source.Concat(destination).Any(s => !moving[s])) throw Json.Error("moving_slots", "must include all permuted slots");
        var labels = Json.Get(root, "labels");
        EvenDigest = Json.Text(labels, "revision_even_sha256");
        OddDigest = Json.Text(labels, "revision_odd_sha256");
        even = Enumerable.Range(0, Geometry.Slots).Select(s => (uint)s).ToArray();
        odd = Apply(even, false);
        ValidateBinding(0, even);
        ValidateBinding(1, odd);
        if (!Apply(odd, true).SequenceEqual(even)) throw Json.Error("inverse", "does not restore solved labels");
    }

    private static double[] Plane(JsonElement root, string id)
    {
        double[] result = Json.Array(Json.Get(root, id), id).Select(x => Json.Number(x, id)).ToArray();
        if (result.Length != 4) throw Json.Error(id, "four components required");
        return result;
    }

    private static int[] Slots(JsonElement root, string id)
    {
        int[] slots = Json.Array(Json.Get(root, id), id).Select(x => Json.Integer(x, id)).ToArray();
        if (slots.Any(s => s < 0 || s >= Geometry.Slots) || slots.Distinct().Count() != slots.Length)
            throw Json.Error(id, "distinct valid slots required");
        return slots;
    }

    private static void CheckPermutation(int[] src, int[] dst, string id)
    {
        if (src.Length != dst.Length || !src.OrderBy(s => s).SequenceEqual(dst.OrderBy(s => s)))
            throw Json.Error(id, "source and destination must permute the same slots");
    }

    public bool IsMoving(int slot) => moving[slot];
    public ReadOnlyMemory<uint> Snapshot(long revision)
    {
        if (revision < 0) throw Json.Error("revision", "must be nonnegative");
        return revision % 2 == 0 ? even : odd;
    }

    public uint[] Apply(ReadOnlySpan<uint> before, bool inverse)
    {
        if (before.Length != Geometry.Slots) throw Json.Error("labels", "259800 labels required");
        uint[] after = before.ToArray();
        int[] src = inverse ? inverseSource : source, dst = inverse ? inverseDestination : destination;
        for (int i = 0; i < src.Length; i++) after[dst[i]] = before[src[i]];
        return after;
    }

    public static string LabelsDigest(ReadOnlySpan<uint> labels)
    {
        var bytes = new byte[labels.Length * 4];
        for (int i = 0; i < labels.Length; i++) BinaryPrimitives.WriteUInt32LittleEndian(bytes.AsSpan(i * 4, 4), labels[i]);
        return AssetCatalogue.Digest(bytes);
    }

    public void ValidateBinding(long revision, ReadOnlySpan<uint> labels)
    {
        if (revision < 0 || labels.Length != Geometry.Slots || LabelsDigest(labels) != (revision % 2 == 0 ? EvenDigest : OddDigest))
            throw Json.Error("labels/revision_" + (revision % 2 == 0 ? "even" : "odd") + "_sha256", "bound snapshot mismatch at revision " + revision);
    }
}

public sealed record TurnFrame(long TurnIndex, double Phase, double Theta, long BoundRevision, ReadOnlyMemory<uint> Labels);

public sealed class TurnClock
{
    private readonly TurnData turn;
    private readonly double duration, a, b;
    private readonly Func<double> nowMs;
    private readonly double start;

    public TurnClock(TurnData turn, double turnMs, double easeA, double easeB, Func<double> nowMs)
    {
        if (!double.IsFinite(turnMs) || turnMs <= 0) throw Json.Error("turnMs", "positive duration required");
        Easing.Ease(0, easeA, easeB);
        this.turn = turn; duration = turnMs; a = easeA; b = easeB; this.nowMs = nowMs;
        start = nowMs();
        if (!double.IsFinite(start)) throw Json.Error("clock", "finite start required");
    }

    public TurnFrame Frame(Func<long, ReadOnlyMemory<uint>>? binding = null)
    {
        double elapsed = nowMs() - start;
        double turns = elapsed / duration;
        if (!double.IsFinite(elapsed) || elapsed < 0 || turns >= long.MaxValue)
            throw Json.Error("clock", "invalid elapsed time");
        long index = (long)Math.Floor(turns);
        // Use the same rounded quotient for both fields, keeping phase in the index's half-open interval.
        double phase = turns - index;
        double theta = (index % 2 == 0 ? 1 : -1) * turn.Angle * Easing.Ease(phase, a, b);
        ReadOnlyMemory<uint> labels = (binding ?? turn.Snapshot)(index);
        turn.ValidateBinding(index, labels.Span);
        return new TurnFrame(index, phase, theta, index, labels);
    }
}
