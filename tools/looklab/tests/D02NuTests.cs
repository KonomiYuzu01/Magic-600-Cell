using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

// ADR-D02 build step B26: the ν band beside the rail at N = 0 (ADR-D02.28) and the ν-corrections rows of the Built-in
// branch (ADR-D02.39), on the samples' correction catalogue and the nu-c0, nu-c35 and on-zero states.
internal static partial class Program
{
    private static NuCorrections Catalogue() => NuCorrections.Read(Read("data/samples/jumble/corrections.json"));

    private static (long N, bool OnLattice, int[]? Nu) NuTotals(string state)
    {
        using JsonDocument d = JsonDocument.Parse(Read("data/samples/jumble/" + state + "/totals.json"));
        JsonElement nu = d.RootElement.GetProperty("nu");
        bool on = nu.GetProperty("result").GetString() != "undefined";
        return (d.RootElement.GetProperty("N").GetInt64(), on, on ? nu.GetProperty("nu").EnumerateArray().Select(v => v.GetInt32()).ToArray() : null);
    }

    // The band has one cell under each of the rail's 35 cells, in the rail's order in both settings; the 8 frame-group
    // marks sit on their orbits' cells and the nine always-zero coordinates are fixed dots.
    private static void D02NuBandAlignsWith35Cells()
    {
        NuCorrections c = Catalogue(); NuConstants k = c.Constants;
        OrbitAtlas atlas = OrbitAtlas.Parse(File.ReadAllText(Path.Combine(Root, "assets/seed_atlas.json")));
        var (n, on, nu) = NuTotals("nu-c0");
        foreach (bool byId in new[] { false, true })
        {
            var order = OrbitRailModel.Order(atlas, byId);
            NuBand band = NuBand.Of(k, order, nu, on, n);
            Check(band.Cells.Count == OrbitAtlas.Count && band.Cells.Select(x => x.Orbit).SequenceEqual(order), "the band's cells are not the rail's " + (byId ? "id" : "stage") + " order");
        }
        NuBand byOrbit = NuBand.Of(k, Enumerable.Range(0, 35).ToArray(), nu, on, n);
        int[] framed = byOrbit.Cells.Where(x => x.HasFrame).Select(x => x.Orbit).ToArray();
        Check(framed.SequenceEqual(new[] { 0, 3, 6, 8, 11, 22, 25, 32 }), "the frame-group marks sit on " + string.Join(" ", framed));
        int fixedDots = byOrbit.Cells.Count(x => x.BitFixed) + byOrbit.Cells.Count(x => x.FrameFixed);
        Check(fixedDots == 9 && byOrbit.Cells.Where(x => x.BitFixed).Select(x => x.Orbit).SequenceEqual(new[] { 0, 1, 2, 3, 5, 6 }), "the fixed dots number " + fixedDots);
        Check(byOrbit.Cells[8].Bit == 1 && byOrbit.Cells[25].Bit == 1 && byOrbit.Cells.Where(x => x.Orbit is not 8 and not 25).All(x => x.Bit is null or 0), "nu-c0's marks are not b8 and b25");
        Check(NuBand.Shape == "diamond", "the ν mark is a " + NuBand.Shape + ", in the ring family");
        Check(byOrbit.Line(k) == "ν band: b8 b25 · O17 dial 0 of 5 · O28 dial 0 of 5 · 9 fixed dots", "the band reads " + byOrbit.Line(k));
    }

    // O17 and O28 carry their C5 values as five-position dials; no other cell has one, and no bar carries a value.
    private static void D02C5ValuesAreDialsNotBars()
    {
        NuConstants k = Catalogue().Constants;
        int[] nu = new int[NuConstants.Count];
        nu[k.Index("C5_17")] = 3; nu[k.Index("C5_28")] = 4;
        NuBand band = NuBand.Of(k, Enumerable.Range(0, 35).ToArray(), nu, true, 0);
        Check(band.Cells.Where(x => x.HasDial).Select(x => (x.Orbit, x.Dial)).SequenceEqual(new[] { (17, (int?)3), (28, (int?)4) }), "the dials are not O17 at 3 and O28 at 4");
        Check(NuConstants.DialPositions == 5 && NuBand.DialKind == "dial", "a C5 value is not a five-position dial");
        Check(k.Moduli[k.Index("C5_17")] == 5 && k.Moduli[k.Index("C5_28")] == 5, "the C5 coordinates are not modulo 5");
        Check(band.Line(k).Contains("O17 dial 3 of 5") && band.Line(k).Contains("O28 dial 4 of 5"), "the band reads " + band.Line(k));
        // The ν strip's source draws a dial with five ticks and a hand, and no rectangle.
        string ui = Read("app/Chrome/Ui/NuBand.cs");
        string draw = ui[ui.IndexOf("public override void _Draw()", StringComparison.Ordinal)..ui.IndexOf("// A ν mark:", StringComparison.Ordinal)];
        Check(!draw.Contains("DrawRect", StringComparison.Ordinal), "the ν band draws a rectangle (a bar) among its marks");
    }

    // Off the lattice the band shows the undefined numeral as a whole; on the lattice above N = 0 it is not shown.
    private static void D02NuBandUndefinedOffLattice()
    {
        NuConstants k = Catalogue().Constants;
        var (n, on, nu) = NuTotals("wj-ia-300");
        NuBand off = NuBand.Of(k, Enumerable.Range(0, 35).ToArray(), nu, on, n);
        Check(!on && off.Undefined && !off.Shown && off.Cells.All(x => x.Bit == null && x.Frame == null && x.Dial == null), "wj-ia-300's band is not undefined");
        Check(off.Line(k) == "ν band: undefined (pieces off the lattice)", "the band off the lattice reads " + off.Line(k));
        NuBand above = NuBand.Of(k, Enumerable.Range(0, 35).ToArray(), new int[NuConstants.Count], true, 12);
        Check(!above.Undefined && !above.Shown && above.Line(k) == "ν band: read at N = 0 (N = 12)", "the band above N = 0 reads " + above.Line(k));
        var (zn, zon, znu) = NuTotals("on-zero");
        NuBand zero = NuBand.Of(k, Enumerable.Range(0, 35).ToArray(), znu, zon, zn);
        Check(zero.Shown && zero.Line(k).StartsWith("ν band: zero · ", StringComparison.Ordinal), "on-zero's band reads " + zero.Line(k));
    }

    // The ν-corrections are the catalogue's 36 rows in id order, each with its id, exact length and static ν row.
    private static void D02CorrectionsAre36InIdOrder()
    {
        NuCorrections c = Catalogue();
        Check(c.Rows.Count == 36 && c.Rows.Select(r => r.Id).SequenceEqual(Enumerable.Range(0, 36)), "the rows are not 36 in id order");
        using JsonDocument d = JsonDocument.Parse(Read("data/samples/jumble/corrections.json"));
        var stored = d.RootElement.GetProperty("catalogue").GetProperty("corrections").EnumerateArray().ToArray();
        for (int i = 0; i < 36; i++)
        {
            Check(c.Rows[i].Length == stored[i].GetProperty("letters").GetArrayLength() && c.Rows[i].Row.SequenceEqual(stored[i].GetProperty("row").EnumerateArray().Select(v => v.GetInt32())),
                "row " + i + " is not its catalogue row");
        }
        Check(c.RowLine(c.Rows[0]) == "c00 · w9 · 9 twists · ν b8 b25", "row 0 reads " + c.RowLine(c.Rows[0]));
        Check(NuCorrections.Path == "Library › Built-in › ν-corrections", "the rows' path reads " + NuCorrections.Path);
        // The selection only moves; nothing in the model orders or ranks the rows.
        c.Move(5); c.Move(-2); c.Move(100);
        Check(c.Selected == 35 && c.Rows.Select(r => r.Id).SequenceEqual(Enumerable.Range(0, 36)), "moving the selection reordered the rows");
        bool refused = false;
        var swapped = System.Text.Json.Nodes.JsonNode.Parse(Read("data/samples/jumble/corrections.json"))!;
        swapped["catalogue"]!["corrections"]![1]!["id"] = 99;
        try { NuCorrections.Read(swapped.ToJsonString()); }
        catch (FormatException) { refused = true; }
        Check(refused, "a catalogue out of id order was read");
    }

    // A checked set gives "ν after this set" from the state's ν row, and the samples bear it out: correction 0 on
    // on-zero is nu-c0's ν, correction 35 is nu-c35's, and on nu-c0 the set {0, 35} reaches nu-c35's; off the lattice
    // it is undefined, and the labels are never predicted.
    private static void D02CorrectionSetCheckMatchesSample()
    {
        NuCorrections c = Catalogue(); NuConstants k = c.Constants;
        int[] zero = NuTotals("on-zero").Nu!, c0 = NuTotals("nu-c0").Nu!, c35 = NuTotals("nu-c35").Nu!;
        c.Mark(0, 1);
        NuCorrectionCheck one = c.Check(zero);
        Check(one.After!.SequenceEqual(c0) && one.Length == 9, "correction 0 on on-zero is not nu-c0's ν");
        c.ClearSet(); c.Mark(35, 1);
        Check(c.Check(zero).After!.SequenceEqual(c35), "correction 35 on on-zero is not nu-c35's ν");
        c.ClearSet(); c.Mark(0, 1); c.Mark(35, 1);
        NuCorrectionCheck two = c.Check(c0);
        Check(two.After!.SequenceEqual(c35), "the set {0, 35} on nu-c0 is not nu-c35's ν");
        Check(two.Line(k) == "ν after this set: b25 · c00×+1 c35×+1, " + JumbleWords.Twists(9 + c.Rows[35].Length) + " · labels not predicted", "the check reads " + two.Line(k));
        // Signed multiplicities: -1 undoes +1, and a row at zero leaves the set.
        c.ClearSet(); c.Mark(0, -1);
        Check(c.Check(c0).After!.SequenceEqual(zero) && c.Set[0] == -1, "correction 0 at -1 on nu-c0 is not zero");
        c.Mark(0, 1);
        Check(c.Set.Count == 0, "a row at zero stayed in the set");
        c.Mark(17, 2);
        NuCorrectionCheck off = c.Check(null);
        Check(off.After == null && off.Line(k).StartsWith("ν after this set: undefined", StringComparison.Ordinal) && off.Line(k).EndsWith("labels not predicted", StringComparison.Ordinal), "off the lattice the check reads " + off.Line(k));
        c.Select(0); c.Preview();
        Check(c.PreviewLine(zero) == "Preview c00: 9 twists · ν after: b8 b25 · labels not predicted", "the preview reads " + c.PreviewLine(zero));
    }
}
