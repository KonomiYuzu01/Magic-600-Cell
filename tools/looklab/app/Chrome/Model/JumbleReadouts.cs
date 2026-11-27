using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// Whether seam_facts(all) has covered the state (ADR-D02.22): until it has, N and its two parts are pending; after a
// cut-off they are incomplete. Neither shows a number.
public enum SeamCoverage { Pending, Complete, CutOff }

// One readout slot: its label, its numeral and the unit word after it, if any.
public readonly record struct ReadoutSlot(string Label, Numeral Value, string? Unit = null);

// Global's border totals (ADR-D02.22): N with its on-grip and off-grip parts, the alignment domains, the off-lattice
// pieces, n2, n4, n6, and "above 6" with its largest height, undefined at a count of 0. N is the state's own total
// from its seam facts, never a sum over caps.
public sealed record GlobalTotals(Numeral N, Numeral OnGrip, Numeral OffGrip, Numeral Domains, Numeral OffLattice,
    Numeral N2, Numeral N4, Numeral N6, Numeral Above6, Numeral Above6Max)
{
    // Before a state is bound: every slot pending.
    public static readonly GlobalTotals Pending = new(Numeral.Pending, Numeral.Pending, Numeral.Pending, Numeral.Pending,
        Numeral.Pending, Numeral.Pending, Numeral.Pending, Numeral.Pending, Numeral.Pending, Numeral.Pending);

    // A state's totals record (`magic600-d02-sample/1` totals.json), with the seam facts' coverage.
    public static GlobalTotals Read(string totals, SeamCoverage coverage)
    {
        using var document = JsonDocument.Parse(totals);
        JsonElement root = document.RootElement, heights = root.GetProperty("height_counts");
        Numeral Seam(string key) => coverage switch
        {
            SeamCoverage.Pending => Numeral.Pending,
            SeamCoverage.CutOff => Numeral.Incomplete,
            _ => Of(root, key),
        };
        Numeral above = Of(heights, "n_above_6");
        Numeral max = above.Number is 0 ? Numeral.Undefined : Of(heights, "max_above_6");
        return new GlobalTotals(Seam("N"), Seam("N_on_grip"), Seam("N_off_grip"), Of(root, "domains"), Of(root, "off_lattice"),
            Of(heights, "n_2"), Of(heights, "n_4"), Of(heights, "n_6"), above, max);
    }

    // A number in the record, or undefined where the record has none (null or absent).
    internal static Numeral Of(JsonElement element, string key) =>
        element.TryGetProperty(key, out JsonElement value) && value.ValueKind == JsonValueKind.Number ? Numeral.Of(value.GetInt64()) : Numeral.Undefined;

    // The fixed slots in their fixed order; the unit word follows N.
    public IReadOnlyList<ReadoutSlot> Slots(string unit, string above) => new ReadoutSlot[]
    {
        new("N", N, unit), new("on grip", OnGrip), new("off grip", OffGrip), new("domains", Domains),
        new("off lattice", OffLattice), new("n2", N2), new("n4", N4), new("n6", N6), new(above, Above6), new("max", Above6Max),
    };
}

// Local's border for the named cap (ADR-D02.22): In, Out, A, N_c, the floor and u, the straddler count. A blocked
// cap has no In, Out, A, N_c or floor: they are undefined, and u counts its straddlers. An admissible cap has none
// (u = 0 exactly; u > 0 exactly when the cap is blocked). A cap with no certified rim row is pending throughout.
public sealed record CapReadout(int Cap, Numeral In, Numeral Out, Numeral A, Numeral Nc, Numeral Floor, Numeral U)
{
    public DiagnosticState CertificateState => In.Kind switch
    {
        NumeralKind.Undefined => DiagnosticState.Failed,
        NumeralKind.Value => DiagnosticState.Ok,
        _ => DiagnosticState.Pending,
    };

    // A named cap's record (`magic600-d02-sample/1` cap-<id>.json): its rim row and, at a blocked cap, its straddlers.
    public static CapReadout Read(string capRecord)
    {
        using var document = JsonDocument.Parse(capRecord);
        JsonElement root = document.RootElement, rim = root.GetProperty("rim");
        int cap = root.GetProperty("cap").GetInt32();
        string status = rim.TryGetProperty("status", out JsonElement s) ? s.GetString() ?? "" : "";
        if (status == "blocked" || rim.TryGetProperty("blocked", out JsonElement b) && b.ValueKind == JsonValueKind.True)
        {
            Numeral u = root.TryGetProperty("straddlers", out JsonElement straddlers) ? GlobalTotals.Of(straddlers, "pieces") : Numeral.Pending;
            if (u.Kind == NumeralKind.Undefined) u = Numeral.Pending;
            return new CapReadout(cap, Numeral.Undefined, Numeral.Undefined, Numeral.Undefined, Numeral.Undefined, Numeral.Undefined, u);
        }
        if (status != "admissible")
            return new CapReadout(cap, Numeral.Pending, Numeral.Pending, Numeral.Pending, Numeral.Pending, Numeral.Pending, Numeral.Pending);
        return new CapReadout(cap, GlobalTotals.Of(rim, "In_c"), GlobalTotals.Of(rim, "Out_c"), GlobalTotals.Of(rim, "A_c"),
            GlobalTotals.Of(rim, "N_c"), GlobalTotals.Of(rim, "F_c"), Numeral.Of(0));
    }

    // The fixed slots in their fixed order; the unit word follows N_c.
    public IReadOnlyList<ReadoutSlot> Slots(string unit) => new ReadoutSlot[]
    {
        new("cap", Numeral.Of(Cap)), new("In", In), new("Out", Out), new("A", A), new("N_c", Nc, unit), new("floor", Floor), new("u", U),
    };
}

// One neutral change mark per slot (ADR-D02.22): a slot whose value differs from the previous revision's value is
// marked, with no colour and no direction. A slot that held no value before (pending at the first binding, undefined)
// is not marked: nothing changed that the researcher saw as a number. Slots are compared by position, which is fixed.
public static class ReadoutChanges
{
    public static IReadOnlySet<int> Marked(IReadOnlyList<ReadoutSlot>? before, IReadOnlyList<ReadoutSlot> after)
    {
        var marked = new HashSet<int>();
        if (before == null || before.Count != after.Count) return marked;
        for (int i = 0; i < after.Count; i++)
            if (before[i].Label == after[i].Label && before[i].Value.Kind == NumeralKind.Value && before[i].Value != after[i].Value) marked.Add(i);
        return marked;
    }
}
