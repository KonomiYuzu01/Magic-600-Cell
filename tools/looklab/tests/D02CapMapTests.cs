using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

// ADR-D02 build step B8: the cap map's classes from certified and completed rim rows only, no numeral at a blocked cap,
// and the ring-sheet arrangement (ADR-D02.13, .14, .27).
internal static partial class Program
{
    private static readonly string[] ClassKeys = { "pending", "blocked", "closed", "at_floor", "above_floor" };

    private static string ClassKey(CapClass k) => k switch
    {
        CapClass.Blocked => "blocked",
        CapClass.Closed => "closed",
        CapClass.AtFloor => "at_floor",
        CapClass.AboveFloor => "above_floor",
        _ => "pending",
    };

    // Every cap of every sample state with rim rows has exactly one class, the classes cover all 600 caps, and they
    // count as the sample builder counted them (cap_classes in totals.json).
    private static void D02CapClassesExclusiveAndExhaustive()
    {
        string folder = FileAt("data/samples/jumble");
        int states = 0;
        foreach (string dir in Directory.GetDirectories(folder).OrderBy(d => d, StringComparer.Ordinal))
        {
            string rim = Path.Combine(dir, "rim.json");
            if (!File.Exists(rim)) continue;
            string name = Path.GetFileName(dir);
            CapMapModel map = CapMapModel.Read(File.ReadAllText(rim));
            var counts = map.Counts();
            Check(counts.Values.Sum() == 600, name + ": the classes do not cover 600 caps");
            Check(Enumerable.Range(0, 600).All(c => Enum.IsDefined(map.Class(c))), name + ": a cap has no class");
            using JsonDocument totals = JsonDocument.Parse(File.ReadAllText(Path.Combine(dir, "totals.json")));
            JsonElement built = totals.RootElement.GetProperty("cap_classes");
            foreach (CapClass k in Enum.GetValues<CapClass>())
                Check(counts[k] == built.GetProperty(ClassKey(k)).GetInt32(), name + ": " + ClassKey(k) + " counts " + counts[k] + ", the builder " + built.GetProperty(ClassKey(k)).GetInt32());
            Check(ClassKeys.All(k => built.TryGetProperty(k, out _)), name + ": the builder's classes differ from the map's");
            states++;
        }
        Check(states >= 5, "fewer sample states with rim rows than written: " + states);
        Check(CapMapModel.Pending().Counts()[CapClass.Pending] == 600, "an unbound map is not all pending");
    }

    // ADR-D02.27: a blocked cap shows no N_c, nor does a pending one; an admissible cap with N_c > 0 shows its rim row's
    // N_c, and a closed cap is a plain cell (N_c = 0 by its class).
    private static void D02NoNumeralAtBlockedCap()
    {
        foreach (string state in new[] { "wj-ia-300", "ia-40", "wj-ia-end" })
        {
            string text = Read("data/samples/jumble/" + state + "/rim.json");
            CapMapModel map = CapMapModel.Read(text);
            using JsonDocument rim = JsonDocument.Parse(text);
            foreach (JsonElement row in rim.RootElement.GetProperty("rows").EnumerateArray())
            {
                int cap = row.GetProperty("cap").GetInt32();
                CapClass k = map.Class(cap);
                if (k is CapClass.Blocked or CapClass.Pending or CapClass.Closed)
                    Check(map.Numeral(cap) == null, state + ": cap " + cap + " (" + CapMapModel.Word(k) + ") shows a numeral");
                else
                    Check(map.Numeral(cap) == row.GetProperty("N_c").GetInt64() && map.Numeral(cap) > 0, state + ": cap " + cap + "'s numeral differs from its rim row");
                if (k == CapClass.Blocked) Check(map.Nc(cap) == null, state + ": a blocked cap has an N_c");
                if (k == CapClass.Closed) Check(map.Nc(cap) == 0, state + ": a closed cap's N_c is not 0");
            }
        }
    }

    // A cap with an uncertain row, a missing row or a row from an incomplete record is pending, never guessed.
    private static void D02UncertainRimRowIsPending()
    {
        string Row(int cap, string status, string nc, string atFloor) =>
            "{\"cap\":" + cap + ",\"status\":\"" + status + "\",\"blocked\":" + (status == "blocked" ? "true" : "false") + ",\"N_c\":" + nc + ",\"F_c\":" + nc + ",\"at_floor\":" + atFloor + "}";
        string rows = string.Join(",", Row(0, "blocked", "null", "null"), Row(1, "admissible", "0", "true"), Row(2, "admissible", "7", "true"),
            Row(3, "admissible", "9", "false"), Row(4, "uncertain", "5", "false"), Row(5, "admissible", "null", "null"));
        CapMapModel complete = CapMapModel.Read("{\"complete\":true,\"rows\":[" + rows + "]}");
        Check(complete.Class(0) == CapClass.Blocked && complete.Class(1) == CapClass.Closed && complete.Class(2) == CapClass.AtFloor
            && complete.Class(3) == CapClass.AboveFloor, "a certified row's class is wrong");
        Check(complete.Class(4) == CapClass.Pending, "an uncertain row was not pending");
        Check(complete.Class(5) == CapClass.Pending, "an admissible row without N_c was not pending");
        Check(complete.Class(6) == CapClass.Pending && complete.Class(599) == CapClass.Pending, "a cap without a row was not pending");
        Check(complete.Numeral(4) == null && complete.Numeral(5) == null, "a pending cap shows a numeral");
        CapMapModel cut = CapMapModel.Read("{\"complete\":false,\"rows\":[" + rows + "]}");
        Check(Enumerable.Range(0, 600).All(c => cut.Class(c) == CapClass.Pending), "a row of an incomplete record was used");
    }

    // At the journal's end (wj-ia-end, all 841 records): 27 admissible caps, all above their floor, and 573 blocked.
    private static void D02CapMapMatchesRim()
    {
        CapMapModel end = CapMapModel.Read(Read("data/samples/jumble/wj-ia-end/rim.json"));
        var counts = end.Counts();
        Check(counts[CapClass.Blocked] == 573, "wj-ia-end: " + counts[CapClass.Blocked] + " blocked caps, not 573");
        Check(counts[CapClass.Closed] + counts[CapClass.AtFloor] + counts[CapClass.AboveFloor] == 27, "wj-ia-end: the admissible caps are not 27");
        Check(counts[CapClass.Pending] == 0, "wj-ia-end: a cap is pending in a complete record");
        // The named caps' own records agree with the map.
        foreach (string file in Directory.GetFiles(FileAt("data/samples/jumble/wj-ia-300"), "cap-*.json"))
        {
            using JsonDocument doc = JsonDocument.Parse(File.ReadAllText(file));
            int cap = doc.RootElement.GetProperty("cap").GetInt32();
            string status = doc.RootElement.GetProperty("rim").GetProperty("status").GetString()!;
            CapMapModel map = CapMapModel.Read(Read("data/samples/jumble/wj-ia-300/rim.json"));
            Check((status == "blocked") == (map.Class(cap) == CapClass.Blocked), "wj-ia-300: cap " + cap + "'s record and the map disagree");
        }
    }

    // The ring sheet: 20 rows of 30, a bijection onto the 600 caps, cyclic columns, built from the Hopf rings of the
    // model this build ships (its digest). The face-neighbour property is checked against the model's cell normals in
    // tests/test_d02_samples.py.
    private static void D02CapLayoutIsRingSheet()
    {
        string text = Read("data/samples/jumble/cap-layout.json");
        CapLayout layout = CapLayout.Read(text);
        var seen = new bool[600];
        for (int r = 0; r < CapLayout.Rows; r++)
            for (int c = 0; c < CapLayout.Columns; c++)
            {
                int cap = layout.At(r, c);
                Check(!seen[cap], "cap " + cap + " sits twice on the sheet"); seen[cap] = true;
                Check(layout.Of(cap) == (r, c), "cap " + cap + "'s place does not read back");
            }
        Check(seen.All(s => s), "a cap is missing from the sheet");
        Check(layout.At(3, CapLayout.Columns) == layout.At(3, 0) && layout.At(3, -1) == layout.At(3, CapLayout.Columns - 1), "the columns do not wrap");
        using JsonDocument document = JsonDocument.Parse(text);
        using JsonDocument rings = JsonDocument.Parse(Read("fixtures/hopf-rings.json"));
        Check(document.RootElement.GetProperty("columns_cyclic").GetBoolean(), "the layout does not say its columns wrap");
        Check(document.RootElement.GetProperty("model").GetString() == rings.RootElement.GetProperty("modelDigest").GetString(), "the layout is not of the Hopf rings' model");
        bool refused = false;
        try { CapLayout.Read("{\"rows\":[[0]]}"); } catch (FormatException) { refused = true; }
        Check(refused, "a layout that is not 20 rows of 30 was read");
    }
}
