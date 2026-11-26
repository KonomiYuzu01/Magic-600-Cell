using System;
using System.Collections.Generic;
using Godot;

namespace Magic600.Chrome;

// The kept twist checks under the named cap (ADR-D02.35, .36; build step B10): the columns
// `try | twist | N_c | floor | ΔN | Δn_h | Δu`, a completed row per check in release order with its session release on
// hover, and each run of superseded releases folded in its place as "not checked:" with the twists' glyphs only.
public sealed partial class TwistCheckList : VBoxContainer
{
    private static readonly int[] Widths = { 34, 64, 52, 46, 56, 132, 72 };

    public TwistCheckList(KeptChecks checks, Func<TokenSet> tokens)
    {
        Name = "TwistCheckList";
        TokenSet t = tokens();
        AddThemeConstantOverride("separation", 0);
        AddChild(Line(KeptChecks.Columns, "MicroLabel", t.C("text.dim"), "TwistCheckColumns"));
        int i = 0;
        foreach (KeptEntry entry in checks.Entries)
        {
            Control row = entry switch
            {
                CheckRow c => Line(KeptChecks.Cells(c), "ReadoutLabel", t.C("text"), "TwistCheck" + i),
                FoldedRow f => Folded(f, t),
                _ => new Control(),
            };
            row.TooltipText = entry is CheckRow ? "session release " + entry.Release : "not checked: superseded before its check completed";
            AddChild(row);
            i++;
        }
    }

    private static HBoxContainer Line(IReadOnlyList<string> cells, string variation, Color colour, string name)
    {
        var line = new HBoxContainer { Name = name, MouseFilter = MouseFilterEnum.Pass };
        for (int k = 0; k < cells.Count; k++)
        {
            Label cell = ChromeWidgets.Text(cells[k], variation);
            cell.AddThemeColorOverride("font_color", colour);
            cell.CustomMinimumSize = new Vector2(Widths[Math.Min(k, Widths.Length - 1)], 0);
            cell.HorizontalAlignment = k is 0 or 2 or 3 or 4 or 6 ? HorizontalAlignment.Right : HorizontalAlignment.Left;
            cell.MouseFilter = MouseFilterEnum.Pass;
            line.AddChild(cell);
        }
        return line;
    }

    private static Label Folded(FoldedRow f, TokenSet t)
    {
        Label label = ChromeWidgets.Text(KeptChecks.Line(f), "MicroLabel");
        label.Name = "TwistCheckFolded" + f.Release;
        label.AddThemeColorOverride("font_color", t.C("text.dim"));
        label.MouseFilter = MouseFilterEnum.Pass;
        return label;
    }
}

// Cap covers under the named-cap cell (ADR-D02.38; build step B11): the domain line, then "covered by", "covers" and
// "undefined" with their ids in cap-id order, and the inspect line. Hovering an id inspects that cap there; the cap map
// gets no mark. The expanded form is the table of the 80 meeting caps, both directions.
public sealed partial class CapCoversList : VBoxContainer
{
    private static readonly int[] Widths = { 46, 92, 92 };
    private readonly Label inspected;
    private readonly CoversInspect inspect;
    private readonly Func<CapMapModel> map;

    public CapCoversList(CapCovers covers, CoversInspect inspect, Func<CapMapModel> map, bool expanded, Action<bool> expand, Func<TokenSet> tokens)
    {
        Name = "CapCoversList"; this.inspect = inspect; this.map = map;
        TokenSet t = tokens();
        AddThemeConstantOverride("separation", t.Px("space.1"));
        Label domain = ChromeWidgets.Text(covers.DomainLine(), "MicroLabel"); domain.Name = "CoversDomain";
        domain.AddThemeColorOverride("font_color", t.C("text.dim")); domain.AutowrapMode = TextServer.AutowrapMode.WordSmart;
        AddChild(domain);
        AddChild(Ids("CoversCoveredBy", covers.CoveredByLine(), t));
        AddChild(Ids("CoversCovers", covers.CoversLine(), t));
        AddChild(Ids("CoversUndefined", covers.UndefinedLine(), t));
        inspected = ChromeWidgets.Text("", "ReadoutLabel"); inspected.Name = "CoversInspect";
        AddChild(inspected);
        Button table = ChromeWidgets.Tool((expanded ? "▾ " : "▸ ") + "The " + covers.Rows.Count + " meeting caps, both directions", expanded ? "Fold the table" : "Show the table of every meeting cap's answers", true);
        table.Name = "CoversTableToggle"; table.Alignment = HorizontalAlignment.Left; table.SetPressedNoSignal(expanded);
        table.Pressed += () => expand(!expanded);
        AddChild(table);
        if (expanded)
        {
            var grid = new GridContainer { Name = "CoversTable", Columns = 3 };
            foreach (string column in covers.Columns()) grid.AddChild(Cell(column, "MicroLabel", t.C("text.dim"), grid.GetChildCount()));
            foreach (var row in covers.Table())
                for (int k = 0; k < row.Count; k++)
                {
                    Label cell = Cell(row[k], "ReadoutLabel", t.C(row[k] == "undefined" ? "text.dim" : "text"), k);
                    if (k == 0) Hoverable(cell, int.Parse(row[0], System.Globalization.CultureInfo.InvariantCulture));
                    grid.AddChild(cell);
                }
            AddChild(grid);
        }
        inspect.Changed += Show; TreeExiting += () => inspect.Changed -= Show;
        Show();
    }

    private void Show()
    {
        if (!IsInstanceValid(this)) return;
        inspected.Text = inspect.Line(map());
    }

    private Control Ids(string name, (string Head, IReadOnlyList<int> Caps) line, TokenSet t)
    {
        var flow = new HFlowContainer { Name = name };
        flow.AddThemeConstantOverride("h_separation", t.Px("space.1"));
        Label head = ChromeWidgets.Text(line.Head + (line.Caps.Count > 0 ? ":" : ""), "MicroLabel");
        head.AddThemeColorOverride("font_color", t.C("text.dim"));
        flow.AddChild(head);
        foreach (int cap in line.Caps)
        {
            Label id = ChromeWidgets.Text(cap.ToString(System.Globalization.CultureInfo.InvariantCulture), "ReadoutLabel");
            id.Name = name + "_" + cap;
            Hoverable(id, cap);
            flow.AddChild(id);
        }
        return flow;
    }

    // Hovering an id inspects it; leaving it inspects nothing. Nothing else listens.
    private void Hoverable(Label id, int cap)
    {
        id.MouseFilter = MouseFilterEnum.Pass;
        id.TooltipText = "Hover to inspect cap " + cap + "; the cap map gets no mark";
        id.MouseEntered += () => inspect.Hover(cap);
        id.MouseExited += () => { if (inspect.Inspected == cap) inspect.Hover(null); };
    }

    private static Label Cell(string text, string variation, Color colour, int column)
    {
        Label cell = ChromeWidgets.Text(text, variation);
        cell.AddThemeColorOverride("font_color", colour);
        cell.CustomMinimumSize = new Vector2(Widths[Math.Min(column % 3, Widths.Length - 1)], 0);
        cell.HorizontalAlignment = column % 3 == 0 ? HorizontalAlignment.Right : HorizontalAlignment.Left;
        return cell;
    }
}
