using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Magic600.Chrome;

public enum CapPredicateKind { AtLeast, AtMost, AtFloor }

// One predicate of the cap map's threshold, written as its formula (ADR-D02.15): N_c ≥ k, N_c ≤ k or N_c = F_c.
public sealed record CapPredicate(CapPredicateKind Kind, int K = 0)
{
    public string Formula => Kind switch
    {
        CapPredicateKind.AtLeast => "N_c ≥ " + K.ToString(CultureInfo.InvariantCulture),
        CapPredicateKind.AtMost => "N_c ≤ " + K.ToString(CultureInfo.InvariantCulture),
        _ => "N_c = F_c",
    };

    public bool Holds(long nc, long floor) => Kind switch
    {
        CapPredicateKind.AtLeast => nc >= K,
        CapPredicateKind.AtMost => nc <= K,
        _ => nc == floor,
    };

    private static readonly Regex Bound = new(@"^(?:N_c\s*)?(≥|>=|≤|<=)\s*(\d{1,6})$", RegexOptions.CultureInvariant);
    private static readonly Regex Floor = new(@"^(?:N_c\s*)?(?:=|==)\s*F_c$", RegexOptions.CultureInvariant);

    // Reads one formula: "N_c ≥ 10", "N_c >= 10", ">= 10", "N_c ≤ 4", "<= 4", "N_c = F_c" or "= F_c".
    public static CapPredicate Parse(string formula)
    {
        string text = formula.Trim();
        if (Floor.IsMatch(text)) return new CapPredicate(CapPredicateKind.AtFloor);
        Match m = Bound.Match(text);
        if (!m.Success) throw new FormatException("not a threshold formula: " + text + " (write N_c ≥ k, N_c ≤ k or N_c = F_c)");
        int k = int.Parse(m.Groups[2].Value, CultureInfo.InvariantCulture);
        return new CapPredicate(m.Groups[1].Value is "≥" or ">=" ? CapPredicateKind.AtLeast : CapPredicateKind.AtMost, k);
    }
}

// What a threshold counts, each apart (ADR-D02.15): admissible caps that meet every predicate, admissible caps that do
// not, blocked caps (which have no N_c to test) and pending caps (no certified row yet).
public readonly record struct ThresholdCounts(int Members, int NonMembers, int Blocked, int Pending);

// The cap map's threshold: a set of predicates that a member meets together. There is no default threshold (DT-J1's
// O1 (b)): until the researcher writes one there are no members and no counts. Members come in cap-id order, never
// sorted by a value.
public sealed class CapThreshold
{
    public static readonly CapThreshold None = new(Array.Empty<CapPredicate>());
    public IReadOnlyList<CapPredicate> Predicates { get; }
    public bool IsSet => Predicates.Count > 0;
    public string Formula => string.Join(", ", Predicates.Select(p => p.Formula));

    private CapThreshold(IReadOnlyList<CapPredicate> predicates) { Predicates = predicates; }

    public static CapThreshold Of(params CapPredicate[] predicates) => predicates.Length == 0 ? None : new(Array.AsReadOnly(predicates.Distinct().ToArray()));

    // Formulas separated by commas, semicolons or "and"; an empty text is no threshold.
    public static CapThreshold Parse(string text)
    {
        string[] parts = Regex.Split(text.Trim(), @"\s*(?:,|;|\band\b)\s*").Where(s => s.Length > 0).ToArray();
        return Of(parts.Select(CapPredicate.Parse).ToArray());
    }

    // Whether a cap is a member: null at a blocked or pending cap, and for every cap while no threshold is set.
    public bool? Member(CapMapModel map, int cap)
    {
        if (!IsSet || map.Nc(cap) is not long nc || map.Floor(cap) is not long floor) return null;
        return Predicates.All(p => p.Holds(nc, floor));
    }

    // The four counts, or none while no threshold is set.
    public ThresholdCounts? Counts(CapMapModel map)
    {
        if (!IsSet) return null;
        int members = 0, non = 0, blocked = 0, pending = 0;
        for (int cap = 0; cap < CapLayout.Caps; cap++)
        {
            switch (map.Class(cap))
            {
                case CapClass.Blocked: blocked++; break;
                case CapClass.Pending: pending++; break;
                default: if (Member(map, cap) == true) members++; else non++; break;
            }
        }
        return new ThresholdCounts(members, non, blocked, pending);
    }

    public IReadOnlyList<int> Members(CapMapModel map) =>
        Enumerable.Range(0, CapLayout.Caps).Where(cap => Member(map, cap) == true).ToArray();

    // The count line the cap map's panel and, while the dock is closed, Global's border show.
    public static string Line(ThresholdCounts c) =>
        $"members {c.Members} · non-members {c.NonMembers} · blocked {c.Blocked} · pending {c.Pending}";

    // ADR-D02.15: while the dock is closed (or holds another panel), the threshold field and its count stay on
    // Global's border; with the cap map open they are in its panel.
    public static bool OnGlobalBorder(string? bottomDockOpen) => bottomDockOpen != EntrySlot.CapMapPanel;
}

// The named cap (ADR-D02.16, .19): one cap or none. A cap's cell names it, and so does its typed number; hovering a
// cell only inspects it and names nothing.
public sealed class CapNaming
{
    public int? Named { get; private set; }
    public int? Hovered { get; private set; }
    public event Action<int?>? Changed;

    public bool Name(int cap)
    {
        if (cap < 0 || cap >= CapLayout.Caps) return false;
        if (Named == cap) return true;
        Named = cap; Safe.Raise(Changed, Named);
        return true;
    }

    public void Unname()
    {
        if (Named == null) return;
        Named = null; Safe.Raise(Changed, Named);
    }

    // Hovering inspects: it changes what the tooltip reads, never the named cap.
    public void Hover(int? cap) => Hovered = cap is >= 0 and < CapLayout.Caps ? cap : null;

    // What hovering a cell says first (owner's bug report, 26 November 2026: nothing said that a click names a cap).
    public const string Hint = "click to name this cap";

    // Why a click on a cap, or the prompt's Enter, names nothing now; null when it names the cap. A blocked cap is named
    // like any other (ADR-D02.30 speaks of the named blocked cap); while Jumble is off no cap is named, as the row is
    // unavailable then.
    public static string? Refusal(int cap, bool jumbleOn, int? named) =>
        cap is < 0 or >= CapLayout.Caps ? "no cap is numbered " + cap.ToString(CultureInfo.InvariantCulture) + "; caps are 0 to 599"
        : !jumbleOn ? "Jumble is off: a cap is named only while Jumble is on"
        : named == cap ? "cap " + cap.ToString(CultureInfo.InvariantCulture) + " is already named; F1 › jumble.cap.unname un-names it"
        : null;

    // A cell's tooltip: the hint, or why a click names nothing, over what the cell shows.
    public static string Tip(string? refusal, string describe) => (refusal ?? Hint) + "\n" + describe;

    // A typed cap number, "150" or "cap 150"; null when the text names no cap.
    public static int? Typed(string? text)
    {
        if (text == null) return null;
        Match m = Regex.Match(text.Trim(), @"^(?:cap\s*)?(\d{1,3})$", RegexOptions.IgnoreCase | RegexOptions.CultureInvariant);
        return m.Success && int.Parse(m.Groups[1].Value, CultureInfo.InvariantCulture) is int cap && cap < CapLayout.Caps ? cap : null;
    }
}

// The 80 caps meeting each cap (ADR-D02.16; J1's meeting_caps, from meeting-caps.json): drawn in a static form when a
// cap is named.
public sealed class MeetingCaps
{
    public const int Count = 80;
    private readonly int[][] caps;

    private MeetingCaps(int[][] caps) { this.caps = caps; }

    public IReadOnlyList<int> Of(int cap) => caps[cap];

    public static MeetingCaps Read(string text)
    {
        using JsonDocument document = JsonDocument.Parse(text);
        int[][] rows = document.RootElement.GetProperty("caps").EnumerateArray().Select(r => r.EnumerateArray().Select(c => c.GetInt32()).ToArray()).ToArray();
        if (rows.Length != CapLayout.Caps || rows.Any(r => r.Length != Count)) throw new FormatException("meeting caps: not 600 rows of 80");
        for (int c = 0; c < rows.Length; c++)
            foreach (int d in rows[c])
                if (d < 0 || d >= CapLayout.Caps || d == c || Array.IndexOf(rows[d], c) < 0) throw new FormatException("meeting caps: not a symmetric relation at cap " + c);
        return new MeetingCaps(rows);
    }
}

// ADR-D02.16: no job starts before a cap is named. Every Jumble slot stays Empty until then; a request names the cap.
public sealed class CapJobs
{
    public IReadOnlyDictionary<string, ToolSlot> Slots { get; } = JumbleSlots.Create();
    private readonly CapNaming naming;

    public CapJobs(CapNaming naming) { this.naming = naming; }

    public bool Request(string slot, long release, string digest)
    {
        if (naming.Named is not int cap || !Slots.TryGetValue(slot, out ToolSlot? target)) return false;
        target.Request(release, digest, "{\"cap\":" + cap.ToString(CultureInfo.InvariantCulture) + "}");
        return true;
    }
}

// The two inspector tiers for a cap (ADR-D02.19): Tier 1 is the kind, the id and two to four numerals; Tier 2 is the
// cap's facts in one fixed order, whatever their values.
public static class CapFacts
{
    public static readonly IReadOnlyList<string> Tier2Order = Array.AsReadOnly(new[] { "cap", "ring", "place", "class", "In", "Out", "A", "N_c", "floor", "u", "meeting caps" });

    public static (string Kind, string Id, IReadOnlyList<ReadoutSlot> Numerals) Tier1(CapMapModel map, int cap)
    {
        CapReadout r = map.Readout(cap);
        return ("cap", cap.ToString(CultureInfo.InvariantCulture), new ReadoutSlot[] { new("N_c", r.Nc), new("floor", r.Floor), new("u", r.U) });
    }

    public static IReadOnlyList<(string Label, string Fact)> Tier2(CapMapModel map, CapLayout layout, CapReadout readout, int meeting = MeetingCaps.Count)
    {
        (int row, int column) = layout.Of(readout.Cap);
        string Word(Numeral n) => n.Kind == NumeralKind.Value ? n.Text : n.Word;
        var facts = new (string, string)[]
        {
            ("cap", readout.Cap.ToString(CultureInfo.InvariantCulture)), ("ring", (row + 1).ToString(CultureInfo.InvariantCulture)),
            ("place", (column + 1).ToString(CultureInfo.InvariantCulture)), ("class", CapMapModel.Word(map.Class(readout.Cap))),
            ("In", Word(readout.In)), ("Out", Word(readout.Out)), ("A", Word(readout.A)), ("N_c", Word(readout.Nc)),
            ("floor", Word(readout.Floor)), ("u", Word(readout.U)), ("meeting caps", meeting.ToString(CultureInfo.InvariantCulture)),
        };
        return facts;
    }
}
