using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// The ν coordinates' constants (nucoords-r0), from the correction catalogue's nu_constants: 45 names in row order
// (b0 to b34, one per orbit; f and an orbit id for the 8 frame groups; C5_17 and C5_28), their moduli, and the nine
// coordinates that are always zero on the menu I_a.
public sealed record NuConstants(IReadOnlyList<string> Names, IReadOnlyList<int> Moduli, IReadOnlyList<int> FrameOrbits, IReadOnlyList<int> DialOrbits, IReadOnlySet<string> Fixed)
{
    public const int Count = 45;
    public const int DialPositions = 5;

    public static NuConstants Read(JsonElement c)
    {
        string[] names = c.GetProperty("names").EnumerateArray().Select(n => n.GetString()!).ToArray();
        int[] moduli = c.GetProperty("moduli").EnumerateArray().Select(m => m.GetInt32()).ToArray();
        int[] frames = c.GetProperty("frame_bit_orbits").EnumerateArray().Select(m => m.GetInt32()).ToArray();
        int[] dials = c.GetProperty("c5_orbits").EnumerateArray().Select(m => m.GetInt32()).ToArray();
        var fixedNames = c.GetProperty("i_a_fixed").EnumerateArray().Select(n => n.GetString()!).ToHashSet(StringComparer.Ordinal);
        string[] wanted = Enumerable.Range(0, OrbitAtlas.Count).Select(o => "b" + o.ToString(CultureInfo.InvariantCulture))
            .Concat(frames.Select(o => "f" + o.ToString(CultureInfo.InvariantCulture))).Concat(dials.Select(o => "C5_" + o.ToString(CultureInfo.InvariantCulture))).ToArray();
        if (names.Length != Count || moduli.Length != Count || !names.SequenceEqual(wanted))
            throw new FormatException("the ν constants name " + names.Length + " coordinates, not b0 to b34, the frame groups and the C5 dials in row order");
        if (frames.Length != 8 || dials.Length != 2 || fixedNames.Count != 9 || !fixedNames.All(names.Contains))
            throw new FormatException("the ν constants have " + frames.Length + " frame groups, " + dials.Length + " dials and " + fixedNames.Count + " fixed coordinates");
        for (int i = 0; i < Count; i++)
        {
            int want = names[i].StartsWith("C5_", StringComparison.Ordinal) ? DialPositions : 2;
            if (moduli[i] != want) throw new FormatException("the ν coordinate " + names[i] + " has modulus " + moduli[i] + ", not " + want);
        }
        return new NuConstants(names, moduli, frames, dials, fixedNames);
    }

    public int Index(string name) => Names.ToList().IndexOf(name);
    public int? Frame(int orbit) => FrameOrbits.Contains(orbit) ? Index("f" + orbit.ToString(CultureInfo.InvariantCulture)) : null;
    public int? Dial(int orbit) => DialOrbits.Contains(orbit) ? Index("C5_" + orbit.ToString(CultureInfo.InvariantCulture)) : null;

    // A ν row's nonzero coordinates by name, a C5 value written with it: "b8 b25", "C5_17=2"; "zero" for none.
    public string Nonzero(IReadOnlyList<int> row)
    {
        string[] parts = Enumerable.Range(0, Count).Where(i => row[i] != 0)
            .Select(i => Moduli[i] == 2 ? Names[i] : Names[i] + "=" + row[i].ToString(CultureInfo.InvariantCulture)).ToArray();
        return parts.Length == 0 ? "zero" : string.Join(" ", parts);
    }

    // The sum of rows with signed multiplicities, each coordinate reduced by its modulus into 0 to modulus - 1.
    public int[] Add(IReadOnlyList<int> start, IEnumerable<(IReadOnlyList<int> Row, int Times)> rows)
    {
        long[] sum = start.Select(v => (long)v).ToArray();
        foreach ((IReadOnlyList<int> row, int times) in rows)
            for (int i = 0; i < Count; i++) sum[i] += (long)times * row[i];
        return sum.Select((v, i) => (int)(((v % Moduli[i]) + Moduli[i]) % Moduli[i])).ToArray();
    }
}

// One cell of the ν band, under one cell of the rail: the orbit's 0/1 mark, its frame group's mark where it has one,
// and its C5 value as a five-position dial where it has one. A fixed coordinate (always zero) is a fixed dot and
// carries no value.
public sealed record NuCell(int Orbit, int? Bit, bool BitFixed, int? Frame, bool FrameFixed, bool HasFrame, int? Dial, bool HasDial);

// ADR-D02.28: the ν band beside the rail at N = 0, aligned with its 35 cells in the rail's order. Its marks use a
// shape outside the ring family (a diamond); a C5 value is a five-position dial, never a bar. Off the lattice the band
// shows numeral.undefined; on the lattice above N = 0 it is not shown, since ν is read at N = 0.
public sealed record NuBand(IReadOnlyList<NuCell> Cells, bool Undefined, bool Shown, long? N, IReadOnlyList<int>? Row)
{
    public const string Shape = "diamond";
    public const string DialKind = "dial";

    public static NuBand? ForState(NuConstants? constants, IReadOnlyList<int> order, JumbleState adopted, long? n, IReadOnlyList<int>? nu)
    {
        if (!adopted.Mode) return null;
        if (constants != null && n is long total)
            return Of(constants, order, nu, adopted.LatticeFlag, total);
        return JumbleNumerals.Undefined(adopted) ? new NuBand(Array.Empty<NuCell>(), true, false, null, null) : null;
    }

    public static NuBand Of(NuConstants k, IReadOnlyList<int> railOrder, IReadOnlyList<int>? nu, bool onLattice, long n)
    {
        if (railOrder.Count != OrbitAtlas.Count || railOrder.Distinct().Count() != OrbitAtlas.Count)
            throw new ArgumentException("a rail order names each of the 35 orbits once");
        bool shown = onLattice && n == 0 && nu is { Count: NuConstants.Count };
        int? Value(int index) => shown ? nu![index] : null;
        var cells = railOrder.Select(o =>
        {
            int bit = k.Index("b" + o.ToString(CultureInfo.InvariantCulture));
            int? frame = k.Frame(o), dial = k.Dial(o);
            bool bitFixed = k.Fixed.Contains(k.Names[bit]), frameFixed = frame is int f && k.Fixed.Contains(k.Names[f]);
            return new NuCell(o, bitFixed ? null : Value(bit), bitFixed, frame is int fi && !frameFixed ? Value(fi) : null, frameFixed, frame != null,
                dial is int d ? Value(d) : null, dial != null);
        }).ToArray();
        return new NuBand(cells, !onLattice, shown, n, shown ? nu : null);
    }

    // The band as one line, for Tier 1 and a screen reader.
    public string Line(NuConstants? k)
    {
        if (Undefined) return "ν band: " + Numeral.Undefined.Word + " (pieces off the lattice)";
        if (!Shown) return "ν band: read at N = 0 (N = " + N?.ToString("N0", CultureInfo.InvariantCulture) + ")";
        string dials = string.Join(" · ", Cells.Where(c => c.HasDial).Select(c => OrbitAtlas.Name(c.Orbit) + " dial " + c.Dial!.Value.ToString(CultureInfo.InvariantCulture) + " of " + NuConstants.DialPositions));
        return "ν band: " + k!.Nonzero(Row!) + " · " + dials + " · " + k.Fixed.Count + " fixed dots";
    }
}

// One ν-correction of the catalogue: its id, family, exact length and static ν row.
public sealed record NuCorrection(int Id, string Family, int Length, IReadOnlyList<int> Row)
{
    public string Label => "c" + Id.ToString("00", CultureInfo.InvariantCulture);
}

// The check of a marked set: "ν after this set" from the state's ν row, or undefined off the lattice; the labels are
// not predicted.
public sealed record NuCorrectionCheck(IReadOnlyList<int>? After, IReadOnlyDictionary<int, int> Set, long Length)
{
    public const string AfterWords = "ν after this set";
    public const string LabelWords = "labels not predicted";

    public string Line(NuConstants k)
    {
        string set = Set.Count == 0 ? "an empty set" : string.Join(" ", Set.Select(p => "c" + p.Key.ToString("00", CultureInfo.InvariantCulture) + (p.Value > 0 ? "×+" : "×") + p.Value.ToString(CultureInfo.InvariantCulture)));
        string after = After == null ? Numeral.Undefined.Word + " (the state is off the lattice)" : k.Nonzero(After);
        return AfterWords + ": " + after + " · " + set + ", " + JumbleWords.Twists(Length) + " · " + LabelWords;
    }
}

// ADR-D02.39: Library › Built-in › ν-corrections. The 36 rows in id order, each with its id, exact length and static ν
// row; the selection moves over them; Shift+Enter previews one; Space marks a row into the set with a signed
// multiplicity; one chord checks the set. Nothing here ranks or suggests a row.
public sealed class NuCorrections
{
    public const string Path = "Library › Built-in › ν-corrections";
    public const int Expected = 36;
    public NuConstants Constants { get; }
    public IReadOnlyList<NuCorrection> Rows { get; }
    public int Selected { get; private set; }
    public NuCorrection Current => Rows[Selected];
    // The marked set: row id to its signed multiplicity, in id order; a row at zero leaves the set.
    public IReadOnlyDictionary<int, int> Set => marks;
    private readonly SortedDictionary<int, int> marks = new();
    public NuCorrection? Previewed { get; private set; }
    public NuCorrectionCheck? Checked { get; private set; }
    public event Action? Changed;

    private NuCorrections(NuConstants constants, IReadOnlyList<NuCorrection> rows) { Constants = constants; Rows = rows; }

    public static NuCorrections Read(string json)
    {
        using JsonDocument d = JsonDocument.Parse(json);
        NuConstants k = NuConstants.Read(d.RootElement.GetProperty("nu_constants"));
        JsonElement catalogue = d.RootElement.GetProperty("catalogue");
        if (!catalogue.GetProperty("names").EnumerateArray().Select(n => n.GetString()).SequenceEqual(k.Names))
            throw new FormatException("the catalogue's ν names differ from the constants'");
        NuCorrection[] rows = catalogue.GetProperty("corrections").EnumerateArray().Select(c => new NuCorrection(c.GetProperty("id").GetInt32(), c.GetProperty("family").GetString()!,
            c.GetProperty("letters").GetArrayLength(), c.GetProperty("row").EnumerateArray().Select(v => v.GetInt32()).ToArray())).ToArray();
        if (rows.Length != Expected || !rows.Select(r => r.Id).SequenceEqual(Enumerable.Range(0, Expected)))
            throw new FormatException("the catalogue holds " + rows.Length + " corrections, not 36 in id order");
        if (rows.Any(r => r.Row.Count != NuConstants.Count || r.Length == 0))
            throw new FormatException("a correction has no letters or a ν row that is not 45 long");
        return new NuCorrections(k, rows);
    }

    // A row's line: its id, family, exact length and static ν row.
    public string RowLine(NuCorrection r) => r.Label + " · " + r.Family + " · " + JumbleWords.Twists(r.Length) + " · ν " + Constants.Nonzero(r.Row)
        + (marks.TryGetValue(r.Id, out int m) ? " · ×" + (m > 0 ? "+" : "") + m.ToString(CultureInfo.InvariantCulture) : "");

    public void Move(int by) { Selected = Math.Clamp(Selected + by, 0, Rows.Count - 1); Safe.Raise(Changed); }

    public void Select(int id) { if (id >= 0 && id < Rows.Count) { Selected = id; Safe.Raise(Changed); } }

    // Space: the row's multiplicity moves by one, up or (with Shift) down; at zero the row leaves the set.
    public void Mark(int id, int by)
    {
        if (id < 0 || id >= Rows.Count || by == 0) return;
        int now = (marks.TryGetValue(id, out int m) ? m : 0) + by;
        if (now == 0) marks.Remove(id); else marks[id] = now;
        Checked = null; Safe.Raise(Changed);
    }

    public void Unmark(int id) { if (marks.Remove(id)) { Checked = null; Safe.Raise(Changed); } }
    public void ClearSet() { marks.Clear(); Checked = null; Safe.Raise(Changed); }

    public void ClearCheck()
    {
        if (Checked == null) return;
        Checked = null;
        Safe.Raise(Changed);
    }

    // Shift+Enter: the selected row alone, as its ν effect on the state.
    public void Preview() { Previewed = Current; Safe.Raise(Changed); }

    // The chord: the marked set checked against the state's ν row (null off the lattice).
    public NuCorrectionCheck Check(IReadOnlyList<int>? start)
    {
        IReadOnlyList<int>? after = start == null ? null : Constants.Add(start, marks.Select(p => (Rows[p.Key].Row, p.Value)));
        long length = marks.Sum(p => (long)Math.Abs(p.Value) * Rows[p.Key].Length);
        Checked = new NuCorrectionCheck(after, new SortedDictionary<int, int>(marks), length);
        Safe.Raise(Changed);
        return Checked;
    }

    // The preview line: the row's ν effect on the state's ν row.
    public string? PreviewLine(IReadOnlyList<int>? start) => Previewed is not { } r ? null
        : "Preview " + r.Label + ": " + JumbleWords.Twists(r.Length) + " · ν after: " + (start == null ? Numeral.Undefined.Word : Constants.Nonzero(Constants.Add(start, new[] { (r.Row, 1) })))
          + " · " + NuCorrectionCheck.LabelWords;
}
