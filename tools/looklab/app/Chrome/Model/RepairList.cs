using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// One repair pair of the 17 October exception (JI.12): an opening x of the named cap c, a meeting cap d, the twist y of
// d and the closing z of c that the pinned rule gives, and the change in N. A row whose z is the identity is a
// two-twist repair: its z is data, never a step.
public sealed record RepairRow(int Number, int X, int D, int Y, int Z, long DeltaN)
{
    public const int Identity = 0;
    public bool TwoTwist => Z == Identity;
    public int Steps => TwoTwist ? 2 : 3;
}

// The repair list (ADR-D02.32, .34; build step B12), exactly as the 17 October exception allows: the rows in the
// adapter's order (menu order of x, then d), no pair marked as best, ΔN one tabular numeral in a fixed column. No row
// is focused when the list opens; arrow keys focus a row; digits 1 to 9 pick rows 1 to 9 only in the list's focused
// context, later rows by focus and Enter. The solver picks; nothing here picks, orders, filters or stages a row.
public sealed class RepairListModel
{
    public const string Context = "repair.list.focused";
    public static readonly IReadOnlyList<string> Columns = Array.AsReadOnly(new[] { "#", "x d", "y z", "ΔN" });
    public const string WholeOnly = "Whole repair only";

    public int Cap { get; }
    public int Revision { get; }
    public JobEnd End { get; }
    public IReadOnlyList<RepairRow> Rows { get; }
    public Coverage? Coverage { get; }
    public string? Reason { get; }
    // The pin the result ran under: the menu's order digest; the menu identity and the tie rule are in Coverage.
    public string Pin { get; }
    public int? Focused { get; private set; }
    public event Action? Changed;
    // B13: every focus names the key that asked for it; one without counts on the boundary ledger.
    public BoundaryLedger? Ledger { get; set; }

    private RepairListModel(int cap, int revision, JobEnd end, IReadOnlyList<RepairRow> rows, Coverage? coverage, string? reason, string pin)
    {
        Cap = cap; Revision = revision; End = end; Rows = rows; Coverage = coverage; Reason = reason; Pin = pin;
    }

    // A repair-list result from a cap record (`magic600-d02-sample/1` cap-<id>.json), the named variant of the
    // sample's three (completed, cut_off, cancelled); null when the record has no such result. The rows keep the
    // record's order.
    public static RepairListModel? From(string record, int revision, string variant = "completed")
    {
        using JsonDocument document = JsonDocument.Parse(record);
        if (!document.RootElement.TryGetProperty("repair_list", out JsonElement list) || !list.TryGetProperty(variant, out JsonElement result)) return null;
        return Read(result, revision);
    }

    public static RepairListModel Read(JsonElement result, int revision)
    {
        JsonElement parameters = result.GetProperty("parameters");
        int cap = parameters.GetProperty("cap").GetInt32();
        string status = result.GetProperty("status").GetString() ?? "";
        JobEnd end = status switch
        {
            "completed" => JobEnd.Completed,
            "cut_off" => JobEnd.CutOff,
            "cancelled" => JobEnd.Cancelled,
            "error" => JobEnd.Error,
            _ => throw new FormatException("repair list: an unknown status " + status),
        };
        Coverage? coverage = null;
        if (result.TryGetProperty("coverage", out JsonElement c) && c.ValueKind == JsonValueKind.Object)
        {
            int total = Menu * c.GetProperty("meeting_caps").GetArrayLength();
            coverage = new Coverage(cap.ToString(CultureInfo.InvariantCulture), revision, parameters.GetProperty("menu_identity").GetString() ?? "",
                parameters.GetProperty("tie_rule").GetString() ?? "", parameters.GetProperty("contract_revision").GetString() ?? "",
                c.GetProperty("pairs_tried").GetInt32(), total);
        }
        var rows = new List<RepairRow>();
        if (result.TryGetProperty("rows", out JsonElement r) && r.ValueKind == JsonValueKind.Array)
            foreach (JsonElement row in r.EnumerateArray())
                rows.Add(new RepairRow(rows.Count + 1, row[0].GetInt32(), row[1].GetInt32(), row[2].GetInt32(), row[3].GetInt32(), row[4].GetInt64()));
        // C2-4: the 17 October exception's order is the adapter's, menu order of x and then d; a record in any other
        // order (by ΔN, say) is refused, never shown.
        for (int i = 1; i < rows.Count; i++)
            if ((rows[i].X, rows[i].D).CompareTo((rows[i - 1].X, rows[i - 1].D)) <= 0)
                throw new FormatException("repair list: row " + rows[i].Number + " is not after row " + rows[i - 1].Number + " in (x, d) order");
        string? reason = result.TryGetProperty("by", out JsonElement by) && by.ValueKind == JsonValueKind.String ? by.GetString()
            : result.TryGetProperty("message", out JsonElement m) && m.ValueKind == JsonValueKind.String ? m.GetString() : null;
        string pin = parameters.TryGetProperty("order_digest", out JsonElement o) ? o.GetString() ?? "" : "";
        return new RepairListModel(cap, revision, end, rows, coverage, reason, pin);
    }

    // The 59 non-identity items of a cap's menu: a pair is (x, d) for each of them and each misaligned meeting cap.
    public const int Menu = 59;

    private static string Signed(long value) => value > 0 ? "+" + value.ToString(CultureInfo.InvariantCulture) : value.ToString(CultureInfo.InvariantCulture);

    public static IReadOnlyList<string> Cells(RepairRow row) => new[]
    {
        row.Number.ToString(CultureInfo.InvariantCulture),
        row.X.ToString(CultureInfo.InvariantCulture) + " " + row.D.ToString(CultureInfo.InvariantCulture),
        row.Y.ToString(CultureInfo.InvariantCulture) + " " + row.Z.ToString(CultureInfo.InvariantCulture),
        Signed(row.DeltaN),
    };

    // The empty line of a completed result with no rows: the cap, the revision and its coverage.
    public string? EmptyLine() => End == JobEnd.Completed && Rows.Count == 0 && Coverage is { } c
        ? "Cap " + Cap.ToString(CultureInfo.InvariantCulture) + ", rev " + Revision.ToString(CultureInfo.InvariantCulture) + ": no repair lowers N ("
          + c.Tried.ToString(CultureInfo.InvariantCulture) + " of " + c.Total.ToString(CultureInfo.InvariantCulture) + " pairs tried)"
        : null;

    // The second line of a completed result: the pin, the menu and the tie rule it ran under. Always shown when the
    // list is empty, on expand otherwise (revision 6, J-7).
    public string? CoverageLine() => Coverage is { } c
        ? "pin " + Short(Pin) + " · menu " + Short(c.Menu) + " · tie rule: " + c.TieRule + " · " + c.Contract
        : null;

    public string? CutOffLine() => End == JobEnd.CutOff && Coverage is { } c
        ? "Not found within budget (" + c.Tried.ToString(CultureInfo.InvariantCulture) + " of " + c.Total.ToString(CultureInfo.InvariantCulture) + " tried)"
        : null;

    // A completed result without coverage is an error (ADR-D02.32).
    public string? Error => End == JobEnd.Completed && Coverage == null ? "a completed result without coverage"
        : End == JobEnd.Completed && Coverage!.Tried != Coverage.Total ? "a completed result that tried " + Coverage.Tried + " of " + Coverage.Total
        : End == JobEnd.Error ? Reason ?? "the job failed" : null;

    private static string Short(string digest) => digest.Length > 12 ? digest[..12] : digest;

    // Arrow keys: the first press focuses the first (or the last) row; focus stays inside the list.
    public void Down(UserEvent? asked = null) => Move(+1, asked);
    public void Up(UserEvent? asked = null) => Move(-1, asked);

    private void Move(int by, UserEvent? asked)
    {
        if (Rows.Count == 0) return;
        Ledger?.Record("focus", "a repair row at cap " + Cap, asked);
        int next = Focused is int f ? Math.Clamp(f + by, 1, Rows.Count) : by > 0 ? 1 : Rows.Count;
        if (next == Focused) return;
        Focused = next; Safe.Raise(Changed);
    }

    public void Unfocus() { if (Focused == null) return; Focused = null; Safe.Raise(Changed); }

    // `jumble.repair.focus k` (B32 findings): the row the solver names, 1 to m, as arrows to it would focus it.
    public bool Focus(int row, UserEvent? asked)
    {
        if (row < 1 || row > Rows.Count) return false;
        Ledger?.Record("focus", "a repair row at cap " + Cap, asked);
        if (row != Focused) { Focused = row; Safe.Raise(Changed); }
        return true;
    }

    // Digits 1 to 9 pick rows 1 to 9, and only in the list's focused context; elsewhere a digit is not the list's.
    // C2-3: a list that shows its error instead of its rows picks nothing.
    public RepairRow? Digit(int digit, IEnumerable<string> contexts) =>
        Error == null && contexts.Contains(Context) && digit is >= 1 and <= 9 && digit <= Rows.Count ? Rows[digit - 1] : null;

    // Enter picks the focused row, in the same context.
    public RepairRow? Enter(IEnumerable<string> contexts) => Error == null && contexts.Contains(Context) && Focused is int f ? Rows[f - 1] : null;

    public DiagnosticState CheckState => Error != null ? DiagnosticState.Failed
        : End == JobEnd.Completed ? DiagnosticState.Ok : DiagnosticState.Pending;
}

// A repair row's steps (ADR-D02.34): step k's moving set is the inside set at its prefix state, after steps 1 to k−1.
// The sample records the letters and moving sets of its first rows. The steps follow the preview's step keys.
public sealed class RepairSteps
{
    public const string MovingUnavailable = "moving set unavailable";
    private readonly Dictionary<int, (IReadOnlyList<(int Cap, int Item)> Letters, IReadOnlyList<long> Moving)> steps = new();

    public static RepairSteps From(string record)
    {
        var result = new RepairSteps();
        using JsonDocument document = JsonDocument.Parse(record);
        if (!document.RootElement.TryGetProperty("repair_steps", out JsonElement all) || all.ValueKind != JsonValueKind.Array) return result;
        foreach (JsonElement step in all.EnumerateArray())
        {
            var letters = step.GetProperty("letters").EnumerateArray().Select(l => (l[0].GetInt32(), l[1].GetInt32())).ToArray();
            var moving = step.GetProperty("moving_sets").EnumerateArray().Select(s => s.GetProperty("count").GetInt64()).ToArray();
            result.steps[step.GetProperty("row").GetInt32()] = (letters, moving);
        }
        return result;
    }

    // The moving-set count of step k of a row; pending in the sample data means no set is available to draw.
    public Numeral Moving(RepairRow row, int k)
    {
        if (k < 1 || k > row.Steps) throw new ArgumentOutOfRangeException(nameof(k));
        return steps.TryGetValue(row.Number, out var s) && k <= s.Moving.Count ? Numeral.Of(s.Moving[k - 1]) : Numeral.Pending;
    }

    public string MovingWords(RepairRow row, int k) => Moving(row, k) is { Kind: NumeralKind.Value } n
        ? "moving " + n.Value.ToString("N0", CultureInfo.InvariantCulture) : MovingUnavailable;

    // The letters a row's steps play, as recorded; a two-twist row's identity z is not a step.
    public IReadOnlyList<(int Cap, int Item)> Letters(RepairRow row, int cap) =>
        steps.TryGetValue(row.Number, out var s) ? s.Letters.Take(row.Steps).ToArray()
            : row.TwoTwist ? new[] { (cap, row.X), (row.D, row.Y) } : new[] { (cap, row.X), (row.D, row.Y), (cap, row.Z) };
}

// The step focus of a focused repair row: "Repair k of m", moved by the preview's scrub keys.
public sealed class RepairScrub
{
    public RepairRow? Row { get; private set; }
    public int Step { get; private set; }
    public event Action? Changed;

    public void Show(RepairRow? row)
    {
        if (row == Row) return;
        Row = row; Step = row == null ? 0 : 1; Safe.Raise(Changed);
    }

    // `view.preview.scrub` forward or back: one step, inside 1 to m.
    public bool Scrub(string direction)
    {
        if (Row == null) return false;
        int next = Math.Clamp(Step + (direction == "back" ? -1 : 1), 1, Row.Steps);
        if (next == Step) return false;
        Step = next; Safe.Raise(Changed); return true;
    }

    public string Label => Row == null ? "" : StepLabel(Step, Row.Steps);

    public void FollowSection(LocalAccordion accordion, RepairListModel? list)
    {
        if (accordion.Open == "repair_list") return;
        list?.Unfocus();
        Show(null);
    }

    public static string StepLabel(int k, int m) => "Repair " + k.ToString(CultureInfo.InvariantCulture) + " of " + m.ToString(CultureInfo.InvariantCulture);
}
