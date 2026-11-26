using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

// ADR-D02 build step B9: the threshold (ADR-D02.15), naming a cap and its 80 meeting caps (.16), and the inspector
// tiers (.19).
internal static partial class Program
{
    private static CapMapModel Rim(string state) => CapMapModel.Read(Read("data/samples/jumble/" + state + "/rim.json"));

    // DT-J1's O1 (b): no threshold until the researcher writes one; then no members, no counts and no outline.
    private static void D02NoDefaultThreshold()
    {
        CapMapModel map = Rim("wj-ia-300");
        Check(!CapThreshold.None.IsSet && CapThreshold.None.Counts(map) == null, "the empty threshold counted");
        Check(CapThreshold.None.Members(map).Count == 0, "the empty threshold has members");
        Check(Enumerable.Range(0, 600).All(c => CapThreshold.None.Member(map, c) == null), "a cap is a member of no threshold");
        Check(CapThreshold.Parse("").IsSet == false && CapThreshold.Parse("   ").IsSet == false, "an empty text set a threshold");
        // The draft rows set and clear it; their notes keep "no default".
        using JsonDocument rows = JsonDocument.Parse(Read("data/design/command-rows-d02.json"));
        string note = rows.RootElement.GetProperty("commands").EnumerateArray().Single(r => r.GetProperty("id").GetString() == "jumble.capmap.threshold.set").GetProperty("note").GetString()!;
        Check(note.Contains("no default threshold", StringComparison.Ordinal), "the threshold row lost its no-default rule");
        // The three forms read, in either spelling, and a wrong formula is refused.
        Check(CapThreshold.Parse("N_c ≥ 10, N_c <= 50 and N_c = F_c").Formula == "N_c ≥ 10, N_c ≤ 50, N_c = F_c", "the formulas did not read back");
        bool refused = false;
        try { CapThreshold.Parse("N_c > 10"); } catch (FormatException) { refused = true; }
        Check(refused, "a formula outside the three forms was read");
    }

    // Members, non-members, blocked and pending are counted apart, and they add up to 600; a blocked cap is never a
    // member or a non-member, since it has no N_c.
    private static void D02ThresholdCountsBlockedApart()
    {
        foreach (string state in new[] { "wj-ia-300", "ia-40", "wj-ia-end" })
        {
            CapMapModel map = Rim(state);
            using JsonDocument rim = JsonDocument.Parse(Read("data/samples/jumble/" + state + "/rim.json"));
            var rows = rim.RootElement.GetProperty("rows").EnumerateArray().ToArray();
            foreach (string formula in new[] { "N_c ≥ 10", "N_c ≤ 40", "N_c = F_c", "N_c >= 1, N_c = F_c" })
            {
                CapThreshold threshold = CapThreshold.Parse(formula);
                ThresholdCounts c = threshold.Counts(map)!.Value;
                Check(c.Members + c.NonMembers + c.Blocked + c.Pending == 600, state + " " + formula + ": the counts do not add up to 600");
                int blocked = rows.Count(r => r.GetProperty("status").GetString() == "blocked");
                Check(c.Blocked == blocked, state + " " + formula + ": " + c.Blocked + " blocked, the rim has " + blocked);
                int members = rows.Count(r => r.GetProperty("status").GetString() == "admissible" && threshold.Predicates.All(p =>
                    p.Holds(r.GetProperty("N_c").GetInt64(), r.GetProperty("F_c").GetInt64())));
                Check(c.Members == members, state + " " + formula + ": " + c.Members + " members, by the rim " + members);
                Check(Enumerable.Range(0, 600).Where(cap => map.Class(cap) == CapClass.Blocked).All(cap => threshold.Member(map, cap) == null),
                    state + ": a blocked cap was tested");
            }
        }
        CapMapModel pending = CapMapModel.Pending();
        Check(CapThreshold.Parse("N_c ≥ 0").Counts(pending) == new ThresholdCounts(0, 0, 0, 600), "pending caps were counted as members");
    }

    // No sort: members come in cap-id order whatever their N_c, and the map's arrangement never moves.
    private static void D02ThresholdNeverReorders()
    {
        CapMapModel map = Rim("wj-ia-300");
        CapLayout layout = CapLayout.Read(Read("data/samples/jumble/cap-layout.json"));
        var before = Enumerable.Range(0, 600).Select(c => layout.Of(c)).ToArray();
        foreach (string formula in new[] { "N_c ≥ 10", "N_c ≤ 900", "N_c = F_c" })
        {
            var members = CapThreshold.Parse(formula).Members(map);
            Check(members.SequenceEqual(members.OrderBy(c => c)), formula + ": the members are not in cap-id order");
            Check(members.Count > 0, formula + ": the fixture has no member to order");
        }
        var byValue = CapThreshold.Parse("N_c ≥ 1").Members(map);
        Check(!byValue.SequenceEqual(byValue.OrderByDescending(c => map.Nc(c)).ThenBy(c => c)), "the fixture cannot tell cap-id order from value order");
        Check(Enumerable.Range(0, 600).Select(c => layout.Of(c)).SequenceEqual(before), "a threshold moved a cap on the sheet");
    }

    // ADR-D02.16: no job starts before a cap is named; every slot stays Empty until then.
    private static void D02NoJobBeforeNaming()
    {
        var naming = new CapNaming();
        var jobs = new CapJobs(naming);
        foreach (string slot in JumbleSlots.Ids)
            Check(!jobs.Request(slot, 1, "d"), slot + " started before a cap was named");
        Check(jobs.Slots.Values.All(s => s.State == SlotState.Empty), "a slot left Empty before a cap was named");
        Check(naming.Name(150) && naming.Named == 150, "the cap was not named");
        Check(jobs.Request(JumbleSlots.Ids[2], 2, "d") && jobs.Slots[JumbleSlots.Ids[2]].State == SlotState.Pending, "a job did not start at the named cap");
        Check(!naming.Name(600) && !naming.Name(-1) && naming.Named == 150, "a cap outside 0 to 599 was named");
        Check(CapNaming.Typed("150") == 150 && CapNaming.Typed("cap 7") == 7 && CapNaming.Typed("600") == null && CapNaming.Typed("x") == null,
            "a typed number read wrongly");
        naming.Unname();
        Check(naming.Named == null, "unnaming left a cap named");
    }

    // Hovering only inspects: it never names a cap, and it never starts a job.
    private static void D02HoverNamesNothing()
    {
        var naming = new CapNaming();
        var jobs = new CapJobs(naming);
        int changes = 0;
        naming.Changed += _ => changes++;
        foreach (int cap in new[] { 0, 150, 599 }) naming.Hover(cap);
        Check(naming.Hovered == 599 && naming.Named == null && changes == 0, "hovering named a cap");
        Check(!jobs.Request(JumbleSlots.Ids[0], 1, "d"), "hovering let a job start");
        naming.Hover(null);
        Check(naming.Hovered == null, "the hover did not clear");
        naming.Name(5); naming.Hover(9);
        Check(naming.Named == 5 && changes == 1, "hovering moved the named cap");
    }

    // Every cap meets 80 caps, symmetrically, as the named caps' covers records list them; and the inspector tiers.
    private static void D02MeetingCapsAre80()
    {
        MeetingCaps meeting = MeetingCaps.Read(Read("data/samples/jumble/meeting-caps.json"));
        Check(Enumerable.Range(0, 600).All(c => meeting.Of(c).Count == MeetingCaps.Count && !meeting.Of(c).Contains(c)), "a cap does not meet 80 others");
        int records = 0;
        // The named caps' records are in the state folders; cap-layout.json beside them is not one.
        foreach (string file in Directory.GetDirectories(FileAt("data/samples/jumble")).SelectMany(d => Directory.GetFiles(d, "cap-*.json")))
        {
            using JsonDocument doc = JsonDocument.Parse(File.ReadAllText(file));
            JsonElement covers = doc.RootElement.GetProperty("covers");
            if (!covers.TryGetProperty("domain", out JsonElement domain) || domain.ValueKind != JsonValueKind.Array) continue;
            int cap = doc.RootElement.GetProperty("cap").GetInt32();
            Check(domain.EnumerateArray().Select(d => d.GetInt32()).SequenceEqual(meeting.Of(cap)), Path.GetFileName(file) + ": the covers domain is not the meeting caps");
            records++;
        }
        Check(records >= 5, "fewer named-cap records than written: " + records);
        // Tier 1: the kind, the id and two to four numerals. Tier 2: the facts in one fixed order, blocked or not.
        CapMapModel map = Rim("wj-ia-300");
        CapLayout layout = CapLayout.Read(Read("data/samples/jumble/cap-layout.json"));
        var (kind, id, numerals) = CapFacts.Tier1(map, 150);
        Check(kind == "cap" && id == "150" && numerals.Count is >= 2 and <= 4, "Tier 1 is not the kind, the id and two to four numerals");
        var open = CapFacts.Tier2(map, layout, map.Readout(150));
        var blocked = CapFacts.Tier2(map, layout, CapReadout.Read(Read("data/samples/jumble/wj-ia-300/cap-0.json")));
        Check(open.Select(f => f.Label).SequenceEqual(CapFacts.Tier2Order) && blocked.Select(f => f.Label).SequenceEqual(CapFacts.Tier2Order), "Tier 2's order moved");
        Check(open.Single(f => f.Label == "N_c").Fact == "44" && blocked.Single(f => f.Label == "N_c").Fact == "undefined" && blocked.Single(f => f.Label == "u").Fact == "231",
            "Tier 2's facts differ from the records");
    }
}
