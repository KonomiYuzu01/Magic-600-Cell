using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

// ADR-D02 build step B7: the totals on Global's border and the named cap's readout on Local's (ADR-D02.22), with
// pending, incomplete and undefined kept apart, and N never derived from the caps.
internal static partial class Program
{
    private static string Totals(string state) => Read("data/samples/jumble/" + state + "/totals.json");

    // ADR-D02.22: N and its two parts are pending until seam_facts(all) covers the state; the other totals do not wait.
    private static void D02TotalNPendingUntilSeamFactsAll()
    {
        GlobalTotals pending = GlobalTotals.Read(Totals("wj-ia-300"), SeamCoverage.Pending);
        Check(pending.N == Numeral.Pending && pending.OnGrip == Numeral.Pending && pending.OffGrip == Numeral.Pending, "N or a part was not pending");
        Check(pending.N.Text == "" && pending.N.Word == "pending", "a pending N showed a number");
        Check(pending.Domains == Numeral.Of(37) && pending.OffLattice == Numeral.Of(120343), "a total that does not need the seam facts waited");
        Check(GlobalTotals.Pending.Slots("seams", "above 6").All(s => s.Value == Numeral.Pending), "an unbound state showed a value");
    }

    // ADR-D02.22: after a cut-off N and its parts are incomplete, never a number and never pending.
    private static void D02TotalNIncompleteAfterCutOff()
    {
        GlobalTotals cut = GlobalTotals.Read(Totals("wj-ia-300"), SeamCoverage.CutOff);
        Check(cut.N == Numeral.Incomplete && cut.OnGrip == Numeral.Incomplete && cut.OffGrip == Numeral.Incomplete, "N or a part was not incomplete");
        Check(cut.N.Text == "" && cut.N.Word == "incomplete" && cut.N != Numeral.Pending, "an incomplete N showed a number or read as pending");
    }

    // J-3: total N is the state's own count, never the sum of the caps' N_c.
    private static void D02TotalNIsNotSumOfCaps()
    {
        GlobalTotals totals = GlobalTotals.Read(Totals("wj-ia-300"), SeamCoverage.Complete);
        using JsonDocument rim = JsonDocument.Parse(Read("data/samples/jumble/wj-ia-300/rim.json"));
        long sum = rim.RootElement.GetProperty("rows").EnumerateArray()
            .Where(r => r.GetProperty("N_c").ValueKind == JsonValueKind.Number).Sum(r => r.GetProperty("N_c").GetInt64());
        Check(totals.N == Numeral.Of(98048), "N is not the state's total");
        Check(sum != 98048, "the fixture cannot tell N from the caps' sum");
        Check(totals.N.Number != sum, "N equals the caps' sum");
        Check(totals.N.Number == totals.OnGrip.Number + totals.OffGrip.Number, "N's two parts do not add up to N");
    }

    // ADR-D02.22: a blocked cap has no In, Out, A, N_c or floor; u counts its straddlers. An admissible cap shows its
    // rim row and u = 0; a cap with no certified row is pending throughout.
    private static void D02BlockedCapReadoutUndefined()
    {
        CapReadout blocked = CapReadout.Read(Read("data/samples/jumble/wj-ia-300/cap-0.json"));
        Check(new[] { blocked.In, blocked.Out, blocked.A, blocked.Nc, blocked.Floor }.All(n => n == Numeral.Undefined), "a blocked cap showed a rim numeral");
        Check(blocked.U == Numeral.Of(231), "u is not the blocked cap's straddler count");
        CapReadout open = CapReadout.Read(Read("data/samples/jumble/wj-ia-300/cap-150.json"));
        Check(open.In == Numeral.Of(472) && open.Out == Numeral.Of(428) && open.A == Numeral.Of(428) && open.Nc == Numeral.Of(44) && open.Floor == Numeral.Of(44),
            "an admissible cap's readout differs from its rim row");
        Check(open.U == Numeral.Of(0), "an admissible cap has straddlers");
        CapReadout pending = CapReadout.Read("{\"cap\":5,\"rim\":{\"status\":\"pending\",\"blocked\":null,\"In_c\":null,\"Out_c\":null,\"A_c\":null,\"N_c\":null,\"F_c\":null}}");
        Check(new[] { pending.In, pending.Out, pending.A, pending.Nc, pending.Floor, pending.U }.All(n => n == Numeral.Pending), "an uncertified cap was not pending");
        Check(blocked.Slots("seams").Select(s => s.Label).SequenceEqual(new[] { "cap", "In", "Out", "A", "N_c", "floor", "u" }), "the cap readout's slots moved");
    }

    // ADR-D02.22: "above 6" shows its count, and its largest height is undefined at a count of 0, never 0 or 6.
    private static void D02Above6MaxUndefinedAtZero()
    {
        GlobalTotals none = GlobalTotals.Read(Totals("wj-ia-300"), SeamCoverage.Complete);
        Check(none.Above6 == Numeral.Of(0) && none.Above6Max == Numeral.Undefined, "the largest height above 6 was not undefined at a count of 0");
        string some = "{\"N\":1,\"N_on_grip\":1,\"N_off_grip\":0,\"domains\":2,\"off_lattice\":5,\"height_counts\":{\"n_2\":1,\"n_4\":1,\"n_6\":1,\"n_above_6\":3,\"max_above_6\":10}}";
        GlobalTotals deep = GlobalTotals.Read(some, SeamCoverage.Complete);
        Check(deep.Above6 == Numeral.Of(3) && deep.Above6Max == Numeral.Of(10), "the count or the largest height above 6 was lost");
        // The slots are fixed: their labels and order never depend on the values.
        Check(none.Slots("seams", "above 6").Select(s => s.Label).SequenceEqual(deep.Slots("seams", "above 6").Select(s => s.Label)), "the totals' slots moved with their values");
        Check(none.Slots("seams", "above 6")[0].Unit == "seams", "N lacks its unit word");
    }

    // Every sample state's totals read back exactly, and one neutral mark marks exactly the slots that changed.
    private static void D02TotalsMatchSample()
    {
        string folder = FileAt("data/samples/jumble");
        int states = 0;
        foreach (string dir in Directory.GetDirectories(folder).OrderBy(d => d, StringComparer.Ordinal))
        {
            string file = Path.Combine(dir, "totals.json");
            if (!File.Exists(file)) continue;
            string text = File.ReadAllText(file);
            using JsonDocument document = JsonDocument.Parse(text);
            JsonElement root = document.RootElement, heights = root.GetProperty("height_counts");
            GlobalTotals totals = GlobalTotals.Read(text, SeamCoverage.Complete);
            string name = Path.GetFileName(dir);
            Check(totals.N.Number == root.GetProperty("N").GetInt64() && totals.OnGrip.Number == root.GetProperty("N_on_grip").GetInt64()
                && totals.OffGrip.Number == root.GetProperty("N_off_grip").GetInt64(), name + ": N or a part differs");
            Check(totals.Domains.Number == root.GetProperty("domains").GetInt64() && totals.OffLattice.Number == root.GetProperty("off_lattice").GetInt64(), name + ": domains or off-lattice differ");
            Check(totals.N2.Number == heights.GetProperty("n_2").GetInt64() && totals.N4.Number == heights.GetProperty("n_4").GetInt64()
                && totals.N6.Number == heights.GetProperty("n_6").GetInt64() && totals.Above6.Number == heights.GetProperty("n_above_6").GetInt64(), name + ": a height count differs");
            states++;
        }
        Check(states >= 16, "fewer sample states than written: " + states);
        var before = GlobalTotals.Read(Totals("wj-ia-300"), SeamCoverage.Complete).Slots("seams", "above 6");
        var after = GlobalTotals.Read(Totals("wj-ia-420"), SeamCoverage.Complete).Slots("seams", "above 6");
        var marked = ReadoutChanges.Marked(before, after);
        Check(marked.SetEquals(Enumerable.Range(0, after.Count).Where(i => before[i].Value != after[i].Value)), "the change marks differ from the changed slots");
        Check(ReadoutChanges.Marked(null, after).Count == 0 && ReadoutChanges.Marked(after, after).Count == 0, "a first or an unchanged readout was marked");
        Check(ReadoutChanges.Marked(GlobalTotals.Pending.Slots("seams", "above 6"), after).Count == 0, "the first binding marked slots that held no value");
        var undefinedBefore = after.Select((s, i) => i == 0 ? s with { Value = Numeral.Undefined } : s).ToList();
        Check(after[0].Value.Kind == NumeralKind.Value && !ReadoutChanges.Marked(undefinedBefore, after).Contains(0), "a slot that was undefined was marked");
    }
}
