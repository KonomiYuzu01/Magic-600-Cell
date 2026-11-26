using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

// ADR-D02 build step B11: Cap covers (ADR-D02.38) and the predicate focus layers with their counts (ADR-D02.21, .23,
// .30), on wj-ia-300's sample records.
internal static partial class Program
{
    private const string CoversState = "data/samples/jumble/wj-ia-300/";

    // Cap 150's covers: every answer at a blocked meeting cap is undefined and listed, never read as "no".
    private static void D02CoversUndefinedIsListed()
    {
        string record = Read(CoversState + "cap-150.json");
        CapCovers covers = CapCovers.FromRecord(record) ?? throw new System.Exception("cap 150's record holds no completed covers answer");
        using JsonDocument document = JsonDocument.Parse(record);
        int[] blocked = document.RootElement.GetProperty("covers").GetProperty("answers").EnumerateArray()
            .Where(a => a.GetProperty("c_covers_d").TryGetProperty("status", out JsonElement s) && s.GetString() == "not_applicable")
            .Select(a => a.GetProperty("cap").GetInt32()).ToArray();
        Check(blocked.Length == 77, "the sample's cap 150 has " + blocked.Length + " not-applicable answers, not 77");
        Check(covers.Undefined.SequenceEqual(blocked), "the undefined list is not every not-applicable meeting cap in cap-id order");
        var lines = covers.Lines();
        Check(lines.Count == 4 && lines[0].StartsWith("The 80 caps that meet cap 150", System.StringComparison.Ordinal), "the first line is not the domain line: " + lines[0]);
        Check(lines[3].StartsWith("undefined · 77 (blocked): ", System.StringComparison.Ordinal), "the undefined line reads " + lines[3]);
        Check(blocked.All(c => lines[3].Split(": ")[1].Split(' ').Contains(c.ToString(System.Globalization.CultureInfo.InvariantCulture))), "an undefined meeting cap is missing from its line");
        // "Covers" answered only at the three unblocked meeting caps: its count says so, and nothing reads "no".
        Check(lines[2] == "covers · 0 of 3 answered", "the covers line reads " + lines[2]);
        Check(lines.All(l => !l.Contains(" no ", System.StringComparison.Ordinal) && !l.EndsWith(" no", System.StringComparison.Ordinal)), "a line reads no");
        foreach (var row in covers.Table().Where(r => blocked.Contains(int.Parse(r[0], System.Globalization.CultureInfo.InvariantCulture))))
            Check(row[2] == "undefined", "the table reads " + row[2] + " at blocked meeting cap " + row[0]);
        // A blocked named cap has no completed answer: its covers job is an error, and the slot shows none.
        Check(CapCovers.FromRecord(Read(CoversState + "cap-0.json")) == null, "the blocked cap 0's record gave a covers answer");
    }

    // The expanded form: the 80 meeting caps in cap-id order, both directions, each yes, no or undefined.
    private static void D02CoversTableHas80RowsBothDirections()
    {
        var meeting = MeetingCaps.Read(Read("data/samples/jumble/meeting-caps.json"));
        foreach (string cap in new[] { "cap-150.json", "cap-91.json" })
        {
            string record = Read(CoversState + cap);
            CapCovers covers = CapCovers.FromRecord(record)!;
            var table = covers.Table();
            Check(table.Count == MeetingCaps.Count, cap + ": the table has " + table.Count + " rows");
            Check(table.All(r => r.Count == 3), cap + ": a row has not three cells");
            Check(table.Select(r => int.Parse(r[0], System.Globalization.CultureInfo.InvariantCulture)).SequenceEqual(meeting.Of(covers.Cap).OrderBy(c => c)), cap + ": the rows are not the meeting caps in cap-id order");
            Check(covers.Columns().SequenceEqual(new[] { "cap", "covers " + covers.Cap, covers.Cap + " covers" }), cap + ": the columns are " + string.Join("|", covers.Columns()));
            Check(table.All(r => r.Skip(1).All(a => a is "yes" or "no" or "undefined")), cap + ": a cell is not yes, no or undefined");
            using JsonDocument document = JsonDocument.Parse(record);
            var answers = document.RootElement.GetProperty("covers").GetProperty("answers").EnumerateArray().ToArray();
            for (int i = 0; i < answers.Length; i++)
            {
                bool by = answers[i].GetProperty("d_covers_c").GetProperty("covers").GetBoolean();
                Check(table[i][1] == (by ? "yes" : "no"), cap + ": row " + table[i][0] + " reads " + table[i][1] + " for d covers c");
            }
            Check(covers.CoveredBy.Count == table.Count(r => r[1] == "yes") && covers.CoversCaps.Count == table.Count(r => r[2] == "yes"), cap + ": the lists and the table disagree");
        }
        Check(CapCovers.FromRecord(Read(CoversState + "cap-150.json"))!.CoveredBy.Count == 36, "cap 150 is not covered by 36 meeting caps");
    }

    // Hovering an id in Cap covers inspects that cap and marks nothing on the cap map.
    private static void D02HoverInCoversMarksNoCapMapCell()
    {
        var map = CapMapModel.Read(Read(CoversState + "rim.json"));
        var meeting = MeetingCaps.Read(Read("data/samples/jumble/meeting-caps.json"));
        var naming = new CapNaming();
        naming.Name(150);
        CapThreshold threshold = CapThreshold.Parse("N_c >= 50");
        CapMarks before = CapMarks.Of(naming, threshold, meeting);
        var inspect = new CoversInspect();
        int changes = 0;
        inspect.Changed += () => changes++;
        CapCovers covers = CapCovers.FromRecord(Read(CoversState + "cap-150.json"))!;
        foreach (int cap in covers.CoveredBy.Concat(covers.Undefined))
        {
            inspect.Hover(cap);
            Check(inspect.Inspected == cap, "hovering " + cap + " does not inspect it");
            Check(CapMarks.Of(naming, threshold, meeting) == before, "hovering " + cap + " in Cap covers changed the cap map's marks");
            Check(naming.Named == 150 && naming.Hovered == null, "hovering " + cap + " in Cap covers named a cap or moved the map's hover");
        }
        inspect.Hover(8);
        Check(inspect.Line(map).StartsWith("cap 8 · blocked · N_c ", System.StringComparison.Ordinal), "the inspect line reads " + inspect.Line(map));
        inspect.Hover(null);
        Check(inspect.Inspected == null && inspect.Line(map) == "", "leaving the ids left a cap inspected");
        Check(changes > 0, "hovering raised no change");
    }

    // The three predicate layers count as the sample does, none is focused by default, and the straddlers dim only at a
    // blocked cap.
    private static void D02FocusLayerCountsMatchSample()
    {
        var style = PoseStyle.Read(Read(CoversState + "style.json"));
        using JsonDocument totals = JsonDocument.Parse(Read(CoversState + "totals.json"));
        JsonElement heights = totals.RootElement.GetProperty("height_counts");
        long n0 = heights.GetProperty("n_0").GetInt64(), n2 = heights.GetProperty("n_2").GetInt64(), n4 = heights.GetProperty("n_4").GetInt64(),
            n6 = heights.GetProperty("n_6").GetInt64(), above = heights.GetProperty("n_above_6").GetInt64();
        Check(style.HeightAtLeast(0) == style.Pieces && style.Pieces == n0 + n2 + n4 + n6 + above, "height at least 0 is not every piece");
        Check(style.HeightAtLeast(2) == n2 + n4 + n6 + above, "height at least 2 is " + style.HeightAtLeast(2));
        Check(style.HeightAtLeast(2) == totals.RootElement.GetProperty("off_lattice").GetInt64(), "height at least 2 is not the off-lattice count");
        Check(style.HeightAtLeast(4) == n4 + n6 + above && style.HeightAtLeast(7) == above, "height at least 4 or 7 disagrees with n_h");
        Check(Enumerable.Range(0, style.Domains.Count).Sum(d => style.InDomain(d)) == style.Pieces, "the domains do not add up to the pieces");
        Check(style.Domains.Count == totals.RootElement.GetProperty("domains").GetInt32(), "the style's domains are not the totals' count");
        Check(style.InDomain(style.LatticeDomain) == n0, "the lattice domain holds " + style.InDomain(style.LatticeDomain) + " pieces, not n_0");

        string blockedRecord = Read(CoversState + "cap-0.json");
        using JsonDocument blocked = JsonDocument.Parse(blockedRecord);
        long straddlers = blocked.RootElement.GetProperty("masks").GetProperty("straddlers").GetProperty("count").GetInt64();
        PredicateLayer atBlocked = JumbleFocus.StraddlersOf(0, blockedRecord);
        Check(atBlocked.Count == Numeral.Of(straddlers) && atBlocked.Applies && straddlers > 0, "the straddlers at blocked cap 0 count " + atBlocked.Count.Value);
        PredicateLayer atOpen = JumbleFocus.StraddlersOf(150, Read(CoversState + "cap-150.json"));
        Check(!atOpen.Applies && atOpen.Count == Numeral.Of(0), "the straddlers apply at the unblocked cap 150");
        Check(!JumbleFocus.StraddlersOf(null, null).Applies && JumbleFocus.StraddlersOf(null, null).Count.Kind == NumeralKind.Pending, "the straddlers count without a named cap");

        var layers = new FocusLayers();
        layers.AddPredicate(atOpen);
        layers.AddPredicate(JumbleFocus.DomainOf(style, 5));
        layers.AddPredicate(JumbleFocus.HeightOf(style, 4));
        Check(layers.Predicates.Count == 3 && layers.Focused == null && !layers.Dimming, "a predicate is focused or dims by default");
        Check(FocusLayers.KeyText(layers.Predicates[1]) == "Domain 5 · " + style.InDomain(5).ToString("N0", System.Globalization.CultureInfo.InvariantCulture), "the domain key reads " + FocusLayers.KeyText(layers.Predicates[1]));
        Check(FocusLayers.KeyText(layers.Predicates[2]) == "Height ≥ 4 · " + n4.ToString("N0", System.Globalization.CultureInfo.InvariantCulture), "the height key reads " + FocusLayers.KeyText(layers.Predicates[2]));
        // The cycle key turns the focused predicate on; the straddlers dim nothing at an unblocked cap.
        Check(layers.Focus(JumbleFocus.Straddlers), "the straddlers could not be focused");
        layers.Cycle();
        Check(layers.Inactive == InactiveMode.Dimmed && !layers.Dimming && layers.ShownPiece(false) == 1, "the straddlers dim at an unblocked cap");
        layers.AddPredicate(atBlocked);
        Check(layers.Focused == JumbleFocus.Straddlers && layers.Dimming && layers.ShownPiece(false) == .25 && layers.ShownPiece(true) == 1, "the straddlers do not dim at the blocked cap 0, or the focus moved");
        Check(layers.ShownPiece(false, breaksThrough: true) == 1, "a breakthrough was dimmed");
        layers.Focus(JumbleFocus.Height);
        Check(layers.Focused == JumbleFocus.Height && layers.Dimming, "focusing the height layer did not move the focus");
        layers.Focus(JumbleFocus.Height);
        Check(layers.Focused == null && !layers.Dimming, "focusing the focused layer again did not un-focus it");
        Check(!layers.Focus("jumble.nothing"), "an unknown predicate was focused");
        bool refused = false;
        try { layers.AddPredicate("cells", "Cells", Numeral.Of(1)); } catch (System.ArgumentException) { refused = true; }
        Check(refused, "a predicate took a pick layer's id");
    }
}
