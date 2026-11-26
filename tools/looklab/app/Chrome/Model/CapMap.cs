using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using Num = Magic600.Chrome.Numeral;

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
    // The certified rim row's In_c, Out_c, A_c, N_c and F_c, per cap; none at a blocked or pending cap.
    private readonly long?[] inC, outC, aC, nc, fc;

    private CapMapModel(CapClass[] classes)
    {
        this.classes = classes;
        inC = new long?[CapLayout.Caps]; outC = new long?[CapLayout.Caps]; aC = new long?[CapLayout.Caps];
        nc = new long?[CapLayout.Caps]; fc = new long?[CapLayout.Caps];
    }

    // Before a state is bound, or for a state with no rim record: every cap pending.
    public static CapMapModel Pending() => new(Enumerable.Repeat(CapClass.Pending, CapLayout.Caps).ToArray());

    // A state's rim record (`magic600-d02-sample/1` rim.json). A row counts only when the record is complete and the
    // row is certified (blocked or admissible); an uncertain or missing row leaves its cap pending.
    public static CapMapModel Read(string rim)
    {
        using JsonDocument document = JsonDocument.Parse(rim);
        JsonElement root = document.RootElement;
        bool complete = root.TryGetProperty("complete", out JsonElement c) && c.ValueKind == JsonValueKind.True;
        var map = Pending();
        long? Number(JsonElement row, string key) => row.TryGetProperty(key, out JsonElement v) && v.ValueKind == JsonValueKind.Number ? v.GetInt64() : null;
        foreach (JsonElement row in root.GetProperty("rows").EnumerateArray())
        {
            int cap = row.GetProperty("cap").GetInt32();
            if (cap < 0 || cap >= CapLayout.Caps) throw new FormatException("a rim row names no cap: " + cap);
            string status = row.TryGetProperty("status", out JsonElement s) && s.ValueKind == JsonValueKind.String ? s.GetString()! : "";
            if (!complete || status is not ("blocked" or "admissible")) continue;
            if (status == "blocked") { map.classes[cap] = CapClass.Blocked; continue; }
            if (Number(row, "N_c") is not long n) continue;
            bool atFloor = row.TryGetProperty("at_floor", out JsonElement f) && f.ValueKind == JsonValueKind.True;
            map.classes[cap] = n == 0 ? CapClass.Closed : atFloor ? CapClass.AtFloor : CapClass.AboveFloor;
            map.nc[cap] = n; map.fc[cap] = Number(row, "F_c");
            map.inC[cap] = Number(row, "In_c"); map.outC[cap] = Number(row, "Out_c"); map.aC[cap] = Number(row, "A_c");
        }
        return map;
    }

    public CapClass Class(int cap) => classes[cap];

    // The numeral a cap's cell shows (ADR-D02.14): N_c at an admissible cap with N_c > 0, at or above its floor; a
    // closed cap is a plain cell (N_c = 0 is its class), and a blocked or pending cap shows none.
    public long? Numeral(int cap) => classes[cap] is CapClass.AtFloor or CapClass.AboveFloor ? nc[cap] : null;

    // The N_c of an admissible cap, 0 at a closed one; null at a blocked or pending cap.
    public long? Nc(int cap) => classes[cap] == CapClass.Closed ? 0 : Numeral(cap);

    // The floor F_c of an admissible cap; null at a blocked or pending cap.
    public long? Floor(int cap) => classes[cap] is CapClass.Blocked or CapClass.Pending ? null : fc[cap];

    // The named cap's readout from its rim row (ADR-D02.22): a blocked cap's u needs its straddlers, which the rim
    // row does not carry, so it is pending until the cap's own record gives it.
    public CapReadout Readout(int cap) => classes[cap] switch
    {
        CapClass.Blocked => new CapReadout(cap, Num.Undefined, Num.Undefined, Num.Undefined, Num.Undefined, Num.Undefined, Num.Pending),
        CapClass.Pending => new CapReadout(cap, Num.Pending, Num.Pending, Num.Pending, Num.Pending, Num.Pending, Num.Pending),
        _ => new CapReadout(cap, Of(inC[cap]), Of(outC[cap]), Of(aC[cap]), Of(nc[cap]), Of(fc[cap]), Num.Of(0)),
    };

    private static Num Of(long? value) => value is long v ? Num.Of(v) : Num.Undefined;

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
