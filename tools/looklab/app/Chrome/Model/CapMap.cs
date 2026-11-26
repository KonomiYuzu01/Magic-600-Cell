using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// The cap map's classes (ADR-D02.14), exclusive and exhaustive, from certified and completed rim rows only: a cap
// without one is pending, never guessed.
public enum CapClass { Pending, Blocked, Closed, AtFloor, AboveFloor }

// The ring-sheet arrangement (B8, cap-layout.json): 20 rows of 30 caps, each row a Hopf ring, cap c being cell c of
// the model; row neighbours are face neighbours and the columns are cyclic.
public sealed class CapLayout
{
    public const int Rows = 20, Columns = 30, Caps = Rows * Columns;
    private readonly int[,] grid = new int[Rows, Columns];
    private readonly (int Row, int Column)[] place = new (int, int)[Caps];

    public int At(int row, int column) => grid[row, ((column % Columns) + Columns) % Columns];
    public (int Row, int Column) Of(int cap) => place[cap];

    public static CapLayout Read(string text)
    {
        using JsonDocument document = JsonDocument.Parse(text);
        JsonElement rows = document.RootElement.GetProperty("rows");
        if (rows.GetArrayLength() != Rows) throw new FormatException("a cap layout has 20 rows");
        var layout = new CapLayout();
        var seen = new bool[Caps];
        int r = 0;
        foreach (JsonElement row in rows.EnumerateArray())
        {
            if (row.GetArrayLength() != Columns) throw new FormatException("a cap layout row has 30 caps");
            int j = 0;
            foreach (JsonElement cell in row.EnumerateArray())
            {
                int cap = cell.GetInt32();
                if (cap < 0 || cap >= Caps || seen[cap]) throw new FormatException("a cap layout is a bijection onto the 600 caps");
                seen[cap] = true; layout.grid[r, j] = cap; layout.place[cap] = (r, j); j++;
            }
            r++;
        }
        return layout;
    }
}

// The 600 caps' classes and the one numeral the map shows: N_c at an admissible cap, and nothing else (a blocked cap
// shows no N_c, ADR-D02.27).
public sealed class CapMapModel
{
    private readonly CapClass[] classes;
    private readonly long?[] nc;

    private CapMapModel(CapClass[] classes, long?[] nc) { this.classes = classes; this.nc = nc; }

    // Before a state is bound, or for a state with no rim record: every cap pending.
    public static CapMapModel Pending() => new(Enumerable.Repeat(CapClass.Pending, CapLayout.Caps).ToArray(), new long?[CapLayout.Caps]);

    // A state's rim record (`magic600-d02-sample/1` rim.json). A row counts only when the record is complete and the
    // row is certified (blocked or admissible); an uncertain or missing row leaves its cap pending.
    public static CapMapModel Read(string rim)
    {
        using JsonDocument document = JsonDocument.Parse(rim);
        JsonElement root = document.RootElement;
        bool complete = root.TryGetProperty("complete", out JsonElement c) && c.ValueKind == JsonValueKind.True;
        var classes = Enumerable.Repeat(CapClass.Pending, CapLayout.Caps).ToArray();
        var nc = new long?[CapLayout.Caps];
        foreach (JsonElement row in root.GetProperty("rows").EnumerateArray())
        {
            int cap = row.GetProperty("cap").GetInt32();
            if (cap < 0 || cap >= CapLayout.Caps) throw new FormatException("a rim row names no cap: " + cap);
            string status = row.TryGetProperty("status", out JsonElement s) && s.ValueKind == JsonValueKind.String ? s.GetString()! : "";
            if (!complete || status is not ("blocked" or "admissible")) continue;
            if (status == "blocked") { classes[cap] = CapClass.Blocked; continue; }
            if (row.GetProperty("N_c").ValueKind != JsonValueKind.Number) continue;
            long n = row.GetProperty("N_c").GetInt64();
            bool atFloor = row.TryGetProperty("at_floor", out JsonElement f) && f.ValueKind == JsonValueKind.True;
            classes[cap] = n == 0 ? CapClass.Closed : atFloor ? CapClass.AtFloor : CapClass.AboveFloor;
            if (n > 0) nc[cap] = n;
        }
        return new CapMapModel(classes, nc);
    }

    public CapClass Class(int cap) => classes[cap];

    // The numeral a cap's cell shows (ADR-D02.14): N_c at an admissible cap with N_c > 0, at or above its floor; a
    // closed cap is a plain cell (N_c = 0 is its class), and a blocked or pending cap shows none.
    public long? Numeral(int cap) => classes[cap] is CapClass.AtFloor or CapClass.AboveFloor ? nc[cap] : null;

    // The N_c of an admissible cap, 0 at a closed one; null at a blocked or pending cap.
    public long? Nc(int cap) => classes[cap] == CapClass.Closed ? 0 : Numeral(cap);

    public IReadOnlyDictionary<CapClass, int> Counts() =>
        Enum.GetValues<CapClass>().ToDictionary(k => k, k => classes.Count(x => x == k));

    // The pattern slot that draws a class (ADR-D02.14): its cue is never colour alone.
    public static string Pattern(CapClass k) => k switch
    {
        CapClass.Blocked => "pattern.cap.blocked",
        CapClass.Closed => "pattern.cap.closed",
        CapClass.AtFloor => "pattern.cap.at_floor",
        CapClass.AboveFloor => "pattern.cap.above_floor",
        _ => "state.pending",
    };

    public static string Word(CapClass k) => k switch
    {
        CapClass.Blocked => "blocked",
        CapClass.Closed => "closed",
        CapClass.AtFloor => "at floor",
        CapClass.AboveFloor => "above floor",
        _ => "pending",
    };
}
