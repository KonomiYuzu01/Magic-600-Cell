using System;
using System.Collections.Generic;
using System.Globalization;
using Godot;

namespace Magic600.Chrome;

// The cap map (ADR-D02.13, .14; build step B8): all 600 caps in the ring-sheet arrangement, 20 Hopf rings of 30 caps,
// drawn in one _Draw. Each cell shows its class by pattern, never by colour alone (ADR-D02.27):
//   blocked      a dark solid ground with a level bar, and no numeral;
//   closed       a plain cell;
//   at floor     a corner bracket and its N_c (N_c = F_c > 0);
//   above floor  its N_c numeral only;
//   pending      ADR-D01's pending ring, while the cap has no certified and completed rim row.
// The columns are cyclic: a dashed wrap mark runs down both sides. Hovering a cell only inspects it (its tooltip);
// naming a cap comes with B9.
public sealed partial class CapMapControl : Control
{
    // The widest numeral a cell keeps room for: N_c stays below 10,000 (it counts seams at one cap).
    public const string WidestNumeral = "0000";
    private readonly Func<TokenSet> tokens;
    private Font? font;
    private int size;
    private Vector2 cell;
    private float gutter;

    public CapLayout? Layout { get; private set; }
    public CapMapModel Map { get; private set; } = CapMapModel.Pending();

    public CapMapControl(Func<TokenSet> tokens)
    {
        this.tokens = tokens; Name = "CapMapCells"; MouseFilter = MouseFilterEnum.Pass;
        TooltipText = "Cap map"; SizeFlagsHorizontal = SizeFlags.ShrinkBegin;
        Restyle();
    }

    public void Show(CapLayout? layout, CapMapModel map)
    {
        Layout = layout; Map = map; Restyle();
    }

    // The cell size follows the theme's micro type: room for the widest numeral and a hairline gap.
    public void Restyle()
    {
        TokenSet t = tokens();
        font = ChromeTheme.Face(t, "ui");
        size = (int)t.TypeSize("ui", "micro");
        int pad = Math.Max(2, t.Px("space.1"));
        cell = new Vector2(Mathf.Ceil(font.GetStringSize(WidestNumeral, HorizontalAlignment.Left, -1, size).X) + 2 * pad, size + pad + 2);
        gutter = Math.Max(6, t.Px("space.2"));
        CustomMinimumSize = Layout == null ? Vector2.Zero : new Vector2(2 * gutter + CapLayout.Columns * cell.X, CapLayout.Rows * cell.Y);
        QueueRedraw();
    }

    public override void _Draw()
    {
        if (Layout == null || font == null) return;
        TokenSet t = tokens();
        for (int r = 0; r < CapLayout.Rows; r++)
            for (int c = 0; c < CapLayout.Columns; c++)
            {
                int cap = Layout.At(r, c);
                Cell(this, t, font, size, new Rect2(gutter + c * cell.X, r * cell.Y, cell.X, cell.Y), Map.Class(cap), Map.Numeral(cap)?.ToString(CultureInfo.InvariantCulture));
            }
        float height = CapLayout.Rows * cell.Y, line = Math.Max(1, t.Px("border.width"));
        Color mark = t.C("text.dim");
        Wrap(this, new Vector2(gutter / 2, 0), height, mark, line);
        Wrap(this, new Vector2(gutter * 1.5f + CapLayout.Columns * cell.X, 0), height, mark, line);
    }

    // The wrap mark: a dashed rule down a side of the sheet, where the last column meets the first.
    public static void Wrap(CanvasItem on, Vector2 top, float height, Color colour, float line) =>
        on.DrawDashedLine(top, top + new Vector2(0, height), colour, line, 3);

    // One cell's pattern. The legend draws its swatches with the same painter.
    public static void Cell(CanvasItem on, TokenSet t, Font font, int size, Rect2 box, CapClass k, string? numeral)
    {
        float line = Math.Max(1, t.Px("border.width"));
        Rect2 inner = box.Grow(-line);
        Vector2 centre = inner.GetCenter();
        Color ink = t.C(CapMapModel.Pattern(k));
        switch (k)
        {
            case CapClass.Blocked:
                on.DrawRect(inner, ink);
                float half = inner.Size.X * .28f;
                on.DrawLine(centre - new Vector2(half, 0), centre + new Vector2(half, 0), t.C("text.dim"), Math.Max(2, 2 * line));
                break;
            case CapClass.Closed:
                on.DrawRect(inner, ink, false, line);
                break;
            case CapClass.AtFloor:
                float arm = Math.Min(inner.Size.X, inner.Size.Y) * .5f, w = Math.Max(1.5f, 1.5f * line);
                on.DrawLine(inner.Position + new Vector2(0, arm), inner.Position, ink, w);
                on.DrawLine(inner.Position, inner.Position + new Vector2(arm, 0), ink, w);
                break;
            case CapClass.Pending:
                on.DrawArc(centre, Math.Min(inner.Size.X, inner.Size.Y) * .3f, 0, Mathf.Tau, 16, ink, line, true);
                break;
        }
        if (numeral == null || k is not (CapClass.AtFloor or CapClass.AboveFloor)) return;
        float width = font.GetStringSize(numeral, HorizontalAlignment.Left, -1, size).X;
        float baseline = inner.Position.Y + (inner.Size.Y + font.GetAscent(size) - font.GetDescent(size)) / 2;
        on.DrawString(font, new Vector2(inner.End.X - width - line, baseline), numeral, HorizontalAlignment.Left, -1, size, ink);
    }

    // The cap under a point of the sheet, if any.
    public int? CapAt(Vector2 point)
    {
        if (Layout == null || cell.X <= 0 || cell.Y <= 0) return null;
        int c = (int)Math.Floor((point.X - gutter) / cell.X), r = (int)Math.Floor(point.Y / cell.Y);
        return r < 0 || r >= CapLayout.Rows || c < 0 || c >= CapLayout.Columns ? null : Layout.At(r, c);
    }

    public override string _GetTooltip(Vector2 atPosition) => CapAt(atPosition) is int cap ? Describe(cap) : "";

    // What hovering a cell says: the cap, its ring and place, its class and its N_c where it has one.
    public string Describe(int cap)
    {
        if (Layout == null) return "";
        (int row, int column) = Layout.Of(cap);
        CapClass k = Map.Class(cap);
        string nc = k switch
        {
            CapClass.Blocked => "N_c undefined: the cap is blocked",
            CapClass.Pending => "no certified rim row yet",
            _ => "N_c " + Map.Nc(cap)?.ToString(CultureInfo.InvariantCulture),
        };
        return $"cap {cap} · ring {row + 1}, place {column + 1} · {CapMapModel.Word(k)} · {nc}";
    }
}

// One legend swatch: a cell drawn with the map's own painter, or the wrap mark.
public sealed partial class CapSwatch : Control
{
    private readonly Func<TokenSet> tokens;
    private readonly CapClass? kind;

    public CapSwatch(Func<TokenSet> tokens, CapClass? kind)
    {
        this.tokens = tokens; this.kind = kind; MouseFilter = MouseFilterEnum.Ignore;
        TokenSet t = tokens();
        int size = (int)t.TypeSize("ui", "micro");
        CustomMinimumSize = new Vector2(size * 2.4f, size + 4);
        SizeFlagsVertical = SizeFlags.ShrinkCenter;
    }

    public override void _Draw()
    {
        TokenSet t = tokens();
        if (kind is not CapClass k) { CapMapControl.Wrap(this, new Vector2(Size.X / 2, 0), Size.Y, t.C("text.dim"), Math.Max(1, t.Px("border.width"))); return; }
        int size = (int)t.TypeSize("ui", "micro");
        CapMapControl.Cell(this, t, ChromeTheme.Face(t, "ui"), size, new Rect2(Vector2.Zero, Size), k, k is CapClass.AtFloor or CapClass.AboveFloor ? "N" : null);
    }
}
