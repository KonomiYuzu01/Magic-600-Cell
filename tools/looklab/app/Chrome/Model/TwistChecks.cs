using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// One twist check's result (ADR-D02.35): the twisted cap's N_c and floor after the twist, ΔN, Δn_h and the named cap's
// Δu, from a release-check record (`magic600-d02-sample/1` cap-<id>.json, release_checks.rows).
public sealed record TwistCheck(int Item, int TwistedCap, long? Nc, long? Floor, long? DeltaN, IReadOnlyList<(string Height, long Delta)>? DeltaNh, long? DeltaU)
{
    public static readonly IReadOnlyList<string> Heights = Array.AsReadOnly(new[] { "n_0", "n_2", "n_4", "n_6", "n_above_6" });

    public static TwistCheck Read(JsonElement row, int item)
    {
        long? Number(JsonElement e, string key) => e.ValueKind == JsonValueKind.Object && e.TryGetProperty(key, out JsonElement v) && v.ValueKind == JsonValueKind.Number ? v.GetInt64() : null;
        JsonElement after = row.TryGetProperty("rim_after", out JsonElement a) ? a : default;
        JsonElement nh = row.TryGetProperty("delta_n_h", out JsonElement h) ? h : default;
        bool refused = row.TryGetProperty("status", out JsonElement status) && status.GetString() != "ok";
        var deltas = !refused && nh.ValueKind == JsonValueKind.Object ? Heights.Select(k => (k, Number(nh, k) ?? 0)).ToArray() : null;
        int cap = row.TryGetProperty("letter_cap", out JsonElement c) && c.ValueKind == JsonValueKind.Number ? c.GetInt32() : -1;
        return new TwistCheck(item, cap, Number(after, "N_c"), Number(row, "floor"), Number(row, "delta_N"), deltas, Number(row, "delta_u"));
    }

    // A cap record's twelve kept checks, by menu item.
    public static IReadOnlyDictionary<int, TwistCheck> FromRecord(string capRecord)
    {
        using JsonDocument document = JsonDocument.Parse(capRecord);
        if (!document.RootElement.TryGetProperty("release_checks", out JsonElement checks)) return new Dictionary<int, TwistCheck>();
        int[] items = checks.GetProperty("items").EnumerateArray().Select(i => i.GetInt32()).ToArray();
        JsonElement[] rows = checks.GetProperty("rows").EnumerateArray().ToArray();
        return items.Zip(rows, (item, row) => (item, row)).ToDictionary(p => p.item, p => Read(p.row, p.item));
    }
}

// The kept twist checks' entries, in release order (ADR-D02.35).
public abstract record KeptEntry(long Release);

// A completed check: its try number at the named cap and revision, its twist and its result.
public sealed record CheckRow(long Release, int Try, int Cap, int Item, TwistCheck Check) : KeptEntry(Release);

// A run of consecutive superseded releases, folded in its place (ADR-D02.36): "not checked:" and the twists' glyphs.
// It holds the releases and the twists from the release record and nothing else: no numeral, no pending mark, and no
// field of any cancelled job.
public sealed record FoldedRow(long Release, IReadOnlyList<long> Releases, int Cap, IReadOnlyList<int> Items) : KeptEntry(Release);

// The kept twist checks at the named cap (ADR-D02.35, .36, as the owner reads J-7 and N-1, 26 November 2026). A release
// supersedes the pending one, and the folded row is written in that same call, from the release record alone. Only
// the latest release's result is taken; cancelling the latest returns the list to its state before that release was
// pending (J-7), and cancelling any other release changes nothing (N-1).
public sealed class KeptChecks
{
    private readonly ReleaseRecord record;
    private readonly List<KeptEntry> entries = new();
    private (long Release, int Try, int Item)? pending;
    public int Cap { get; }
    public int Revision { get; }
    public event Action? Changed;

    public KeptChecks(ReleaseRecord record, int cap, int revision) { this.record = record; Cap = cap; Revision = revision; }

    public IReadOnlyList<KeptEntry> Entries => entries;
    public long? Pending => pending?.Release;
    // Every release made at this cap and revision, in order.
    public IReadOnlyList<long> Released => released;
    private readonly List<long> released = new();

    // The solver releases a twist key: a new release; a still-pending one is superseded and folded now.
    public long Release(int item)
    {
        long release = record.Next();
        int tried = record.TryNumber(Cap.ToString(CultureInfo.InvariantCulture), Revision);
        if (pending is { } old)
        {
            record.Superseded(old.Release);
            // A run is consecutive releases: a completed row or a cancelled release between them ends it.
            if (entries.Count > 0 && entries[^1] is FoldedRow run && run.Releases[^1] + 1 == old.Release)
                entries[^1] = run with { Releases = run.Releases.Append(old.Release).ToArray(), Items = run.Items.Append(old.Item).ToArray() };
            else
                entries.Add(new FoldedRow(old.Release, new[] { old.Release }, Cap, new[] { old.Item }));
        }
        pending = (release, tried, item);
        released.Add(release);
        Safe.Raise(Changed);
        return release;
    }

    // A check finished: only the latest release's result is taken; a late one changes nothing.
    public bool Complete(long release, TwistCheck check)
    {
        if (pending is not { } p || p.Release != release) return false;
        entries.Add(new CheckRow(release, p.Try, Cap, p.Item, check));
        pending = null;
        Safe.Raise(Changed);
        return true;
    }

    // J-7: cancelling the latest pending check withdraws it and the list stays as before. N-1: any other release is a
    // no-op, so the screen is unchanged.
    public bool Cancel(long release)
    {
        if (pending is not { } p || p.Release != release) return false;
        pending = null;
        Safe.Raise(Changed);
        return true;
    }

    // The list as the screen shows it, one line per entry, for the screen-identity test.
    public IReadOnlyList<string> Screen() => entries.Select(Line).ToArray();

    public static string Glyph(int cap, int item) => cap.ToString(CultureInfo.InvariantCulture) + "·" + item.ToString(CultureInfo.InvariantCulture);

    public static string Line(KeptEntry entry) => entry switch
    {
        FoldedRow f => "not checked: " + string.Join(" ", f.Items.Select(i => Glyph(f.Cap, i))),
        CheckRow c => string.Join(" | ", Cells(c)),
        _ => "",
    };

    public static readonly IReadOnlyList<string> Columns = Array.AsReadOnly(new[] { "try", "twist", "N_c", "floor", "ΔN", "Δn_h", "Δu" });

    // A completed row's cells under the columns `try | twist | N_c | floor | ΔN | Δn_h | Δu`.
    public static IReadOnlyList<string> Cells(CheckRow c)
    {
        string Value(long? v) => v is long n ? n.ToString(CultureInfo.InvariantCulture) : "undefined";
        string Signed(long? v) => v is long n ? (n > 0 ? "+" : "") + n.ToString(CultureInfo.InvariantCulture) : "undefined";
        var nh = c.Check.DeltaNh?.Where(d => d.Delta != 0).Select(d => d.Height.Replace("n_above_6", "above 6").Replace("_", "") + " " + Signed(d.Delta)).ToArray();
        return new[]
        {
            c.Try.ToString(CultureInfo.InvariantCulture), Glyph(c.Check.TwistedCap >= 0 ? c.Check.TwistedCap : c.Cap, c.Item),
            Value(c.Check.Nc), Value(c.Check.Floor), Signed(c.Check.DeltaN), nh == null ? "undefined" : nh.Length == 0 ? "0" : string.Join(" ", nh), Signed(c.Check.DeltaU),
        };
    }
}

// The sample stand-in for the check jobs (B10): a tick clock that completes each request a fixed number of ticks
// after it was made, with the cap record's result. A request that is no longer the latest still "completes", and
// KeptChecks ignores it, as the host does.
public sealed class SampleJobs
{
    private readonly List<(long Release, int Item, long Due)> queue = new();
    public long Now { get; private set; }
    public int Ticks { get; }

    public SampleJobs(int ticks) { Ticks = ticks; }

    public void Request(long release, int item) => queue.Add((release, item, Now + Ticks));

    // A newer request supersedes this one: the job is withdrawn and never completes. Answers whether it was queued.
    public bool Cancel(long release) => queue.RemoveAll(q => q.Release == release) > 0;

    // One tick: the requests now due, oldest first.
    public IReadOnlyList<(long Release, int Item)> Tick()
    {
        Now++;
        var due = queue.Where(q => q.Due <= Now).OrderBy(q => q.Release).Select(q => (q.Release, q.Item)).ToArray();
        queue.RemoveAll(q => q.Due <= Now);
        return due;
    }
}
