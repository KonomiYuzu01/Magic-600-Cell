using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// One meeting cap's answer in one direction: yes, no, or undefined with its reason (a blocked or closed meeting cap).
public enum CoverAnswer { Yes, No, Undefined }

// A meeting cap d of the named cap c (JI.12): whether d covers c, and whether c covers d.
public sealed record CoverRow(int Cap, CoverAnswer CoveredBy, CoverAnswer Covers, string? Reason);

// Cap covers (ADR-D02.38; build step B11): a completed covers answer for the named cap, from J1's `covers` job. The
// slot shows its domain line, then "covered by", "covers" and "undefined", each in cap-id order; the expanded form is
// the table of the 80 meeting caps in both directions. Absence never reads "no": an answer at a blocked or closed
// meeting cap is undefined and listed. Nothing here ranks, filters or marks a cap.
public sealed class CapCovers
{
    public int Cap { get; }
    public int Threshold { get; }
    // The answer's domain: the 80 caps that meet the named cap, in cap-id order.
    public IReadOnlyList<int> Domain { get; }
    public IReadOnlyList<CoverRow> Rows { get; }
    public string Digest { get; }

    private CapCovers(int cap, int threshold, IReadOnlyList<int> domain, IReadOnlyList<CoverRow> rows, string digest)
    {
        Cap = cap; Threshold = threshold; Domain = domain; Rows = rows; Digest = digest;
    }

    public IReadOnlyList<int> CoveredBy => Rows.Where(r => r.CoveredBy == CoverAnswer.Yes).Select(r => r.Cap).ToArray();
    public IReadOnlyList<int> CoversCaps => Rows.Where(r => r.Covers == CoverAnswer.Yes).Select(r => r.Cap).ToArray();
    public IReadOnlyList<int> Undefined => Rows.Where(r => r.CoveredBy == CoverAnswer.Undefined || r.Covers == CoverAnswer.Undefined).Select(r => r.Cap).ToArray();
    private int Answered(Func<CoverRow, CoverAnswer> side) => Rows.Count(r => side(r) != CoverAnswer.Undefined);

    // A cap record's covers answer (`magic600-d02-sample/1` cap-<id>.json); null when the record holds no completed
    // answer, as at a blocked named cap, whose covers job is an error with no answers.
    public static CapCovers? FromRecord(string text)
    {
        using JsonDocument document = JsonDocument.Parse(text);
        if (!document.RootElement.TryGetProperty("covers", out JsonElement covers) || covers.ValueKind != JsonValueKind.Object) return null;
        if (covers.GetProperty("status").GetString() != "completed") return null;
        JsonElement parameters = covers.GetProperty("parameters");
        int cap = parameters.GetProperty("cap").GetInt32();
        int threshold = parameters.GetProperty("threshold").GetInt32();
        int[] domain = covers.GetProperty("domain").EnumerateArray().Select(d => d.GetInt32()).ToArray();
        var rows = new List<CoverRow>();
        foreach (JsonElement answer in covers.GetProperty("answers").EnumerateArray())
        {
            (CoverAnswer by, string? byReason) = Read(answer.GetProperty("d_covers_c"));
            (CoverAnswer covering, string? coveringReason) = Read(answer.GetProperty("c_covers_d"));
            rows.Add(new CoverRow(answer.GetProperty("cap").GetInt32(), by, covering, coveringReason ?? byReason));
        }
        if (domain.Length != MeetingCaps.Count || rows.Count != domain.Length || !rows.Select(r => r.Cap).SequenceEqual(domain))
            throw new FormatException("covers: a completed answer lists every one of the 80 meeting caps, in the domain's order");
        if (domain.Zip(domain.Skip(1)).Any(p => p.First >= p.Second)) throw new FormatException("covers: the domain is not in cap-id order");
        string digest = covers.TryGetProperty("input_digest", out JsonElement d) && d.ValueKind == JsonValueKind.String ? d.GetString()! : "";
        return new CapCovers(cap, threshold, domain, rows, digest);
    }

    private static (CoverAnswer, string?) Read(JsonElement side)
    {
        if (side.TryGetProperty("covers", out JsonElement covers) && covers.ValueKind is JsonValueKind.True or JsonValueKind.False)
            return (covers.GetBoolean() ? CoverAnswer.Yes : CoverAnswer.No, null);
        if (side.TryGetProperty("status", out JsonElement status) && status.GetString() == "not_applicable")
            return (CoverAnswer.Undefined, side.TryGetProperty("reason", out JsonElement reason) ? reason.GetString() : null);
        throw new FormatException("covers: an answer is neither a yes, a no nor not applicable");
    }

    private static string Ids(IEnumerable<int> caps) => string.Join(" ", caps.Select(c => c.ToString(CultureInfo.InvariantCulture)));

    // The domain line: what the answer covers and on what terms (JI.12, ENG-0J's M4 definition). The three heads are
    // the word record's (jumble-words.json: covers.covered_by, covers.covers, covers.undefined).
    public string DomainLine() =>
        "The " + Domain.Count.ToString(CultureInfo.InvariantCulture) + " caps that meet cap " + Cap.ToString(CultureInfo.InvariantCulture)
        + " · at least " + Threshold.ToString(CultureInfo.InvariantCulture) + " aligned face pieces a side";

    // The three lists in cap-id order, each with how many of the 80 it answered, so an empty list never reads "no"
    // where answers are undefined.
    public (string Head, IReadOnlyList<int> Caps) CoveredByLine() =>
        ("covered by · " + CoveredBy.Count.ToString(CultureInfo.InvariantCulture) + " of " + Answered(r => r.CoveredBy).ToString(CultureInfo.InvariantCulture) + " answered", CoveredBy);

    public (string Head, IReadOnlyList<int> Caps) CoversLine() =>
        ("covers · " + CoversCaps.Count.ToString(CultureInfo.InvariantCulture) + " of " + Answered(r => r.Covers).ToString(CultureInfo.InvariantCulture) + " answered", CoversCaps);

    public (string Head, IReadOnlyList<int> Caps) UndefinedLine()
    {
        string[] reasons = Rows.Where(r => Undefined.Contains(r.Cap)).Select(r => r.Reason ?? "undefined").Distinct().OrderBy(r => r, StringComparer.Ordinal).ToArray();
        return ("undefined · " + Undefined.Count.ToString(CultureInfo.InvariantCulture) + (reasons.Length > 0 ? " (" + string.Join(", ", reasons) + ")" : ""), Undefined);
    }

    // The four lines as text, as the slot reads them.
    public IReadOnlyList<string> Lines()
    {
        string Line((string Head, IReadOnlyList<int> Caps) line) => line.Head + (line.Caps.Count > 0 ? ": " + Ids(line.Caps) : "");
        return new[] { DomainLine(), Line(CoveredByLine()), Line(CoversLine()), Line(UndefinedLine()) };
    }

    // The expanded form: the 80 meeting caps in cap-id order, both directions, each yes, no or undefined.
    public IReadOnlyList<string> Columns() => new[] { "cap", "covers " + Cap.ToString(CultureInfo.InvariantCulture), Cap.ToString(CultureInfo.InvariantCulture) + " covers" };

    public static string Word(CoverAnswer answer) => answer switch { CoverAnswer.Yes => "yes", CoverAnswer.No => "no", _ => "undefined" };

    public IReadOnlyList<IReadOnlyList<string>> Table() =>
        Rows.Select(r => (IReadOnlyList<string>)new[] { r.Cap.ToString(CultureInfo.InvariantCulture), Word(r.CoveredBy), Word(r.Covers) }).ToArray();
}

// Hovering an id in Cap covers inspects that cap (ADR-D02.38): it changes only this line, never the named cap, the
// cap map's hover or its marks.
public sealed class CoversInspect
{
    public int? Inspected { get; private set; }
    // The adopted revision's Jumble facts: Tier 1 says what is unavailable while Jumble is on (ADR-D02.25).
    public JumbleState State { get; set; } = JumbleState.Off();
    public event Action? Changed;

    public void Hover(int? cap)
    {
        int? next = cap is >= 0 and < CapLayout.Caps ? cap : null;
        if (next == Inspected) return;
        Inspected = next; Safe.Raise(Changed);
    }

    // The inspected cap's Tier 1 (ADR-D02.19): kind, id, N_c, floor and u, with the class's word.
    public string Line(CapMapModel map)
    {
        if (Inspected is not int cap) return "";
        var (kind, id, numerals) = CapFacts.Tier1(map, cap);
        string Word(Numeral n) => n.Kind == NumeralKind.Value ? n.Text : n.Word;
        return JumbleViews.Tier1(kind + " " + id + " · " + CapMapModel.Word(map.Class(cap)) + " · " + string.Join(" · ", numerals.Select(n => n.Label + " " + Word(n.Value))), State);
    }
}

// What the cap map marks (ADR-D02.15, .16): the threshold, the named cap, its meeting caps and the cap the map's own
// hover inspects. Cap covers' hover is not among them.
public sealed record CapMarks(string Threshold, int? Named, string Meeting, int? Hovered)
{
    public static CapMarks Of(CapNaming naming, CapThreshold threshold, MeetingCaps? meeting) =>
        new(threshold.Formula, naming.Named, naming.Named is int c && meeting != null ? string.Join(",", meeting.Of(c)) : "", naming.Hovered);
}

// The pose rows of a bound state (`magic600-d02-sample/1` style.json): each pose class's alignment domain, its pieces'
// height and its piece count. The focus layers count from here (ADR-D02.21, .23).
public sealed class PoseStyle
{
    public IReadOnlyList<string> Domains { get; }
    public int LatticeDomain { get; }
    // The rows' digest: the state a saved focus layer was made for.
    public string Digest { get; private init; } = "";
    private readonly (int Domain, int Height, long Count)[] rows;

    private PoseStyle(IReadOnlyList<string> domains, int lattice, (int, int, long)[] rows) { Domains = domains; LatticeDomain = lattice; this.rows = rows; }

    public static PoseStyle Read(string text)
    {
        using JsonDocument document = JsonDocument.Parse(text);
        JsonElement root = document.RootElement;
        string[] domains = root.GetProperty("domains").EnumerateArray().Select(d => d.GetString()!).ToArray();
        // The piece's height is its left height (heights[0]), as the totals' n_h count it.
        var rows = root.GetProperty("rows").EnumerateArray()
            .Select(r => (r.GetProperty("domain").GetInt32(), r.GetProperty("heights")[0].GetInt32(), r.GetProperty("count").GetInt64())).ToArray();
        if (rows.Any(r => r.Item1 < 0 || r.Item1 >= domains.Length || r.Item2 < 0 || r.Item3 < 0)) throw new FormatException("style: a row outside its domains or with a negative height or count");
        return new PoseStyle(domains, root.GetProperty("lattice_domain").GetInt32(), rows)
        {
            Digest = root.TryGetProperty("digest", out JsonElement digest) && digest.ValueKind == JsonValueKind.String ? digest.GetString()! : "",
        };
    }

    public long Pieces => rows.Sum(r => r.Count);
    // The pose rows as the focus layer's `=` expressions read them (PieceFilter.cs).
    public IReadOnlyList<PoseRow> Rows => rows.Select(r => new PoseRow(r.Domain, r.Height, r.Count)).ToArray();
    public long InDomain(int domain) => rows.Where(r => r.Domain == domain).Sum(r => r.Count);
    public long HeightAtLeast(int k) => rows.Where(r => r.Height >= k).Sum(r => r.Count);
}

// The three predicate focus layers ADR-D02 adds (ADR-D02.21, .23, .30; build step B11), with their counts. Their
// drawing comes in B15.
public static class JumbleFocus
{
    public const string Straddlers = "jumble.straddlers", Domain = "jumble.domain", Height = "jumble.height";

    // The named blocked cap's straddlers: the count at a blocked named cap with a record; at an unblocked cap the
    // layer dims nothing (ADR-D02.30); pending while the cap or its record is missing.
    public static PredicateLayer StraddlersOf(int? named, string? record)
    {
        if (named == null || record == null) return new PredicateLayer(Straddlers, "Straddlers", Numeral.Pending, false);
        using JsonDocument document = JsonDocument.Parse(record);
        JsonElement root = document.RootElement;
        // C2-10: an uncertain cap's blocked is null: its straddlers are pending.
        JsonElement flag = root.GetProperty("rim").GetProperty("blocked");
        if (flag.ValueKind is not (JsonValueKind.True or JsonValueKind.False)) return new PredicateLayer(Straddlers, "Straddlers", Numeral.Pending, false);
        bool blocked = flag.GetBoolean();
        if (!blocked) return new PredicateLayer(Straddlers, "Straddlers", Numeral.Of(0), false);
        return root.GetProperty("masks").TryGetProperty("straddlers", out JsonElement mask)
            ? new PredicateLayer(Straddlers, "Straddlers", Numeral.Of(mask.GetProperty("count").GetInt64()), true)
            : new PredicateLayer(Straddlers, "Straddlers", Numeral.Pending, false);
    }

    public static PredicateLayer DomainOf(PoseStyle style, int domain)
    {
        if (domain < 0 || domain >= style.Domains.Count) throw new ArgumentOutOfRangeException(nameof(domain));
        string label = "Domain " + domain.ToString(CultureInfo.InvariantCulture) + (domain == style.LatticeDomain ? " (lattice)" : "");
        return new PredicateLayer(Domain, label, Numeral.Of(style.InDomain(domain)), true);
    }

    public static PredicateLayer HeightOf(PoseStyle style, int k)
    {
        if (k < 0) throw new ArgumentOutOfRangeException(nameof(k));
        return new PredicateLayer(Height, "Height ≥ " + k.ToString(CultureInfo.InvariantCulture), Numeral.Of(style.HeightAtLeast(k)), true);
    }
}
