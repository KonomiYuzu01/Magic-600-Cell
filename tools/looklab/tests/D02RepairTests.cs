using System;
using System.Linq;
using System.Reflection;
using System.Text.Json;
using Magic600.Chrome;

// ADR-D02 build step B12: the repair list exactly as the 17 October exception allows (ADR-D02.32, .34), on wj-ia-300's
// records of caps 150 and 91.
internal static partial class Program
{
    private const string RepairState = "data/samples/jumble/wj-ia-300/";

    // The rows keep the adapter's order, numbered from 1, with ΔN one numeral in its own column.
    private static void D02RepairOrderEqualsSample()
    {
        foreach (string cap in new[] { "cap-150.json", "cap-91.json" })
        {
            string record = Read(RepairState + cap);
            RepairListModel list = RepairListModel.From(record, 300)!;
            using JsonDocument document = JsonDocument.Parse(record);
            var rows = document.RootElement.GetProperty("repair_list").GetProperty("completed").GetProperty("rows").EnumerateArray().ToArray();
            Check(list.End == JobEnd.Completed && list.Error == null, cap + ": the completed result reads as " + list.End + " " + list.Error);
            Check(list.Rows.Count == rows.Length && rows.Length > 9, cap + ": " + list.Rows.Count + " rows, the record has " + rows.Length);
            for (int i = 0; i < rows.Length; i++)
            {
                RepairRow row = list.Rows[i];
                Check(row.Number == i + 1 && row.X == rows[i][0].GetInt32() && row.D == rows[i][1].GetInt32() && row.Y == rows[i][2].GetInt32()
                    && row.Z == rows[i][3].GetInt32() && row.DeltaN == rows[i][4].GetInt64(), cap + ": row " + (i + 1) + " is not the record's");
            }
            var cells = RepairListModel.Cells(list.Rows[0]);
            Check(cells.Count == RepairListModel.Columns.Count && cells[3] == rows[0][4].GetInt64().ToString(System.Globalization.CultureInfo.InvariantCulture),
                cap + ": the first row's cells are " + string.Join(" | ", cells));
        }
        var words = Words();
        Check(string.Join(" | ", RepairListModel.Columns) == words["repair.columns"], "the columns are not the word record's");
        Check(RepairListModel.WholeOnly == words["repair.whole"], "the whole-only word is not the record's");
    }

    // The list's model has no member that sorts, orders, filters, ranks or compares rows (reflection).
    private static void D02RepairModelHasNoSortMember()
    {
        string[] forbidden = { "sort", "order", "filter", "rank", "best", "compare", "score", "top" };
        foreach (Type type in new[] { typeof(RepairListModel), typeof(RepairRow), typeof(RepairSteps), typeof(RepairScrub) })
        {
            foreach (MemberInfo member in type.GetMembers(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly))
            {
                string name = member.Name.ToLowerInvariant();
                // A record's compiler-made EqualityContract and the like are not ours; every other member is checked.
                if (name.Contains("equalitycontract", StringComparison.Ordinal)) continue;
                Check(!forbidden.Any(f => name.Contains(f, StringComparison.Ordinal)), type.Name + " has a member " + member.Name);
            }
            Check(!type.GetInterfaces().Any(i => i.Name.StartsWith("IComparable", StringComparison.Ordinal) || i.Name.StartsWith("IComparer", StringComparison.Ordinal)), type.Name + " compares");
        }
    }

    // No row is focused when the list opens; arrow keys focus one.
    private static void D02RepairNoFocusOnOpen()
    {
        RepairListModel list = RepairListModel.From(Read(RepairState + "cap-150.json"), 300)!;
        Check(list.Focused == null, "a row is focused on open");
        Check(list.Enter(new[] { RepairListModel.Context }) == null, "Enter picked a row with none focused");
        list.Down();
        Check(list.Focused == 1, "the first arrow down focused row " + list.Focused);
        list.Down(); list.Up(); list.Up();
        Check(list.Focused == 1, "focus left the list at the top: " + list.Focused);
        RepairListModel fresh = RepairListModel.From(Read(RepairState + "cap-150.json"), 300)!;
        fresh.Up();
        Check(fresh.Focused == fresh.Rows.Count, "the first arrow up did not focus the last row");
    }

    // Digits 1 to 9 pick rows 1 to 9 only in the list's focused context; later rows by focus and Enter.
    private static void D02RepairDigitsOnlyInContext()
    {
        RepairListModel list = RepairListModel.From(Read(RepairState + "cap-150.json"), 300)!;
        string[] inList = { RepairListModel.Context, "global" };
        foreach (string[] elsewhere in new[] { new[] { "global" }, new[] { "preview", "global" }, Array.Empty<string>() })
            for (int digit = 1; digit <= 9; digit++)
                Check(list.Digit(digit, elsewhere) == null, "digit " + digit + " picked a row in " + string.Join(",", elsewhere));
        for (int digit = 1; digit <= 9; digit++)
            Check(list.Digit(digit, inList) == list.Rows[digit - 1], "digit " + digit + " did not pick row " + digit + " in the list's context");
        Check(list.Digit(0, inList) == null && list.Digit(10, inList) == null, "a digit outside 1 to 9 picked a row");
        for (int i = 0; i < 12; i++) list.Down();
        Check(list.Enter(new[] { "global" }) == null, "Enter picked outside the list's context");
        Check(list.Enter(inList) == list.Rows[11], "Enter did not pick the focused row 12");
    }

    // An empty completed list names the cap, the revision and its coverage, and its second line always shows.
    private static void D02RepairEmptyNamesCapRevisionAndCoverage()
    {
        using JsonDocument document = JsonDocument.Parse(Read(RepairState + "cap-150.json"));
        string completed = document.RootElement.GetProperty("repair_list").GetProperty("completed").GetRawText();
        string empty = System.Text.RegularExpressions.Regex.Replace(completed, "\"rows\":\\s*\\[.*?\\]\\]", "\"rows\": []", System.Text.RegularExpressions.RegexOptions.Singleline);
        using JsonDocument emptied = JsonDocument.Parse(empty);
        RepairListModel list = RepairListModel.Read(emptied.RootElement, 300);
        Check(list.Rows.Count == 0 && list.End == JobEnd.Completed, "the emptied result still has rows");
        Check(list.EmptyLine() == "Cap 150, rev 300: no repair lowers N (177 of 177 pairs tried)", "the empty line reads " + list.EmptyLine());
        string expected = Words()["repair.empty.template"].Replace("{cap}", "150").Replace("{revision}", "300").Replace("{tried}", "177").Replace("{total}", "177");
        Check(list.EmptyLine() == expected, "the empty line is not the word record's template");
        string coverage = list.CoverageLine() ?? "";
        Check(coverage.Contains("pin 5a0429d2946f", StringComparison.Ordinal) && coverage.Contains("menu fddc8532b918", StringComparison.Ordinal)
            && coverage.Contains("tie rule: lowest value, then smaller pinned index", StringComparison.Ordinal), "the coverage line reads " + coverage);
        // A completed result without coverage, or one that tried less than everything, is an error.
        string bare = System.Text.RegularExpressions.Regex.Replace(completed, "\"coverage\":\\s*\\{.*?\\},\\s*\"input_digest\"", "\"input_digest\"", System.Text.RegularExpressions.RegexOptions.Singleline);
        using JsonDocument noCoverage = JsonDocument.Parse(bare);
        Check(RepairListModel.Read(noCoverage.RootElement, 300).Error == "a completed result without coverage", "a completed result without coverage is not an error");
    }

    // A cut-off is never the empty list: it says how many of how many it tried.
    private static void D02RepairCutOffIsNotEmpty()
    {
        RepairListModel cut = RepairListModel.From(Read(RepairState + "cap-150.json"), 300, "cut_off")!;
        Check(cut.End == JobEnd.CutOff && cut.Rows.Count == 0, "the cut-off variant reads as " + cut.End);
        Check(cut.EmptyLine() == null, "a cut-off reads as the empty list");
        Check(cut.CutOffLine() == "Not found within budget (3 of 177 tried)", "the cut-off line reads " + cut.CutOffLine());
        string expected = Words()["repair.cut_off.template"].Replace("{tried}", "3").Replace("{total}", "177");
        Check(cut.CutOffLine() == expected, "the cut-off line is not the word record's template");
        RepairListModel cancelled = RepairListModel.From(Read(RepairState + "cap-150.json"), 300, "cancelled")!;
        Check(cancelled.End == JobEnd.Cancelled && cancelled.EmptyLine() == null && cancelled.CutOffLine() == null, "a cancelled job shows a line");
    }

    // A two-twist row (identity z) has two steps; its z is data, never a step. A three-twist row has three.
    private static void D02RepairTwoTwistRowHasTwoSteps()
    {
        string record = Read(RepairState + "cap-91.json");
        RepairListModel list = RepairListModel.From(record, 300)!;
        RepairRow two = list.Rows.First(r => r.Z == RepairRow.Identity);
        RepairRow three = list.Rows.First(r => r.Z != RepairRow.Identity);
        Check(two.TwoTwist && two.Steps == 2 && three.Steps == 3, "the steps are " + two.Steps + " and " + three.Steps);
        var steps = RepairSteps.From(record);
        Check(steps.Letters(two, 91).Count == 2 && steps.Letters(two, 91)[1] == (two.D, two.Y), "a two-twist row plays " + steps.Letters(two, 91).Count + " letters");
        var scrub = new RepairScrub();
        scrub.Show(two);
        Check(scrub.Label == "Repair 1 of 2", "the label reads " + scrub.Label);
        Check(scrub.Scrub("forward") && scrub.Label == "Repair 2 of 2" && !scrub.Scrub("forward"), "scrubbing a two-twist row passed step 2");
        Check(scrub.Scrub("back") && scrub.Label == "Repair 1 of 2" && !scrub.Scrub("back"), "scrubbing back passed step 1");
        scrub.Show(three);
        Check(scrub.Label == "Repair 1 of 3", "a three-twist row reads " + scrub.Label);
        Check(Words()["repair.step.template"].Replace("{step}", "1").Replace("{steps}", "3") == scrub.Label, "the step label is not the word record's template");
        // Step k's moving set is recorded for the first rows: cap 150's row 1 moves 1,909, 2,467 and 1,909 pieces.
        string r150 = Read(RepairState + "cap-150.json");
        RepairRow first = RepairListModel.From(r150, 300)!.Rows[0];
        var moving = RepairSteps.From(r150);
        Check(Enumerable.Range(1, 3).Select(k => moving.Moving(first, k).Value).SequenceEqual(new long[] { 1909, 2467, 1909 }), "row 1's moving sets are not the record's");
        Check(moving.Moving(list.Rows[^1], 1).Kind == NumeralKind.Pending || list.Rows.Count <= 9, "a row the sample has no steps for counts a moving set");
    }

    // The word record's sentences, by id, with templates under "<id>.template".
    private static System.Collections.Generic.Dictionary<string, string> Words()
    {
        using JsonDocument document = JsonDocument.Parse(Read("data/design/jumble-words.json"));
        var words = new System.Collections.Generic.Dictionary<string, string>(StringComparer.Ordinal);
        foreach (JsonElement s in document.RootElement.GetProperty("sentences").EnumerateArray())
        {
            words[s.GetProperty("id").GetString()!] = s.GetProperty("text").GetString()!;
            if (s.TryGetProperty("template", out JsonElement t)) words[s.GetProperty("id").GetString()! + ".template"] = t.GetString()!;
        }
        return words;
    }
}
