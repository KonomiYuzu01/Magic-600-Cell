using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

internal static partial class Program
{
    private static void D02GripFrameCyclesOrdersAndKeys()
    {
        var grip = new GripFrame();
        GripOrder Order(int frame, ulong item, params int[] corners) => new(frame,
            corners.Select((corner, i) => new GripMark(i + 1, (i + 1).ToString(), corner, new double[4])).ToArray(),
            new[] { new GripKey("KeyJ", item, "(14)(23)") });
        var orders = new[] { Order(0, 1, 7, 9, 11, 13), Order(2, 4, 9, 11, 7, 13), Order(9, 6, 13, 11, 9, 7) };
        int changed = 0;
        grip.Changed += () =>
        {
            changed++;
            var order = orders.Single(o => o.Frame == grip.Frame);
            Sequence(grip.Marks, order.Marks, "marks at Changed");
            Sequence(grip.KeyMeanings, order.Keys, "keys at Changed");
        };
        Check(grip.Set(150, 0, orders), "set the grip");
        Equal(grip.Cap, 150);
        Sequence(grip.AdmissibleFrames, new[] { 0, 2, 9 }, "admissible frames");
        Check(!grip.Set(0) && !grip.Set(1), "same and refused frames");
        Equal(changed, 1);
        Check(grip.Next(), "step over refused frame 1");
        Equal(grip.Frame, 2);
        Equal(grip.Key("KeyJ")!.Item, 4UL);
        Sequence(grip.Marks.Select(m => m.Corner), new[] { 9, 11, 7, 13 }, "reordered corners");
        Check(grip.Next(), "step to frame 9");
        Equal(grip.Frame, 9);
        Check(grip.Next(), "wrap to frame 0");
        Equal(grip.Frame, 0);
        Equal(grip.Key("KeyJ")!.Item, 1UL);
        Equal(changed, 4);
    }

    private static void D02GripRowsCarryTheExistingCommand()
    {
        string text = Read("data/design/command-rows-d02.json");
        using var doc = JsonDocument.Parse(text);
        var rows = doc.RootElement.GetProperty("grip").EnumerateArray().ToArray();
        Sequence(rows.Select(r => r.GetProperty("id").GetString()), new[] { GripFrame.SelectRow, GripFrame.NextRow }, "draft grip IDs");
        Check(rows.All(r => r.GetProperty("request").GetString() == "ADR-D01.75" &&
            r.GetProperty("kind").GetString() == "workspace" && r.GetProperty("label").GetString()!.Split(' ').Length <= 4),
            "the draft grip row format");
        var carried = rows[0];
        Check(carried.GetProperty("note").GetString()!.Contains("ADR-D01.13"), "the click modifier's request");
        var table = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        Equal(carried.GetProperty("label").GetString(), table.Commands[GripFrame.SelectRow].Label);
        table.AddRows(text);
        Sequence(GripFrame.AddDraftRows(table, text), new[] { GripFrame.SelectRow, GripFrame.NextRow }, "the marked draft rows");
        Equal(table.Order.Count(id => id == GripFrame.SelectRow), 1);
        Check(table.Commands.ContainsKey(GripFrame.NextRow), "next frame is registered");
    }

    private static void D02GripPointsFollowTheCoreClip()
    {
        // The raw view's cap centre appears at the middle, with the antipodal clip respecting the half-open boundary.
        var pole = CoreModel.CellCentre(150);
        var view = CoreView.Default(1) with { FocusPole = pole, FocusCos = 121.0 / 125, RawSign = 1 };
        var centre = CoreModel.PlaceCellPoint(150, null, view);
        Check(centre is { } at && Math.Abs(at.X - .5) < 1e-9 && Math.Abs(at.Y - .5) < 1e-9, "cap centre at the Local pole");
        Check(CoreModel.PlaceCellPoint(150, null, view with { RawSign = -1 }) == null, "antipodal clip");
        Check(CoreModel.PlaceCellPoint(150, null, view with { Chart = (_, _) => (0, 0, 5) }) == null, "near clip");
        Check(CoreModel.PlaceCellPoint(150, null, view with { Chart = (_, _) => (30, 0, 0) }) == null, "pane clip");
    }
}
