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
