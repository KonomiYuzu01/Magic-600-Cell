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
// The columns are cyclic: a dashed wrap mark runs down both sides. Over the patterns (B9): the threshold's members
// carry the Query set outline, the named cap ADR-D01's selection brackets, and its 80 meeting caps a small square in
// the cell's corner, which is neither an outline nor a dim. A click on a cell, or a typed number and Enter while the
// map has focus, names a cap; hovering a cell only inspects it (its tooltip).
public sealed partial class CapMapControl : Control, IOwnKeys
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
    public CapThreshold Threshold { get; private set; } = CapThreshold.None;
    public int? Named { get; private set; }
    public IReadOnlyList<int> Meeting { get; private set; } = Array.Empty<int>();
    // Where hovering goes: it inspects and names nothing.
    public CapNaming? Naming { get; set; }
    // The digits typed while the map has focus.
    public string Typed { get; private set; } = "";
    // A cell was clicked, or a typed number entered: the host names that cap.
    public event Action<int>? Picked;
    public event Action? TypedChanged;
    private HashSet<int> members = new();
    private HashSet<int> meeting = new();

    public CapMapControl(Func<TokenSet> tokens)
    {
        this.tokens = tokens; Name = "CapMapCells"; MouseFilter = MouseFilterEnum.Stop; FocusMode = FocusModeEnum.All;
        TooltipText = "Cap map"; SizeFlagsHorizontal = SizeFlags.ShrinkBegin;
        Restyle();
    }

    // The threshold's members, the named cap and its meeting caps, over the classes.
    public void Mark(CapThreshold threshold, int? named, IReadOnlyList<int> meetingCaps)
    {
        Threshold = threshold; Named = named; Meeting = meetingCaps;
        members = new HashSet<int>(threshold.Members(Map)); meeting = new HashSet<int>(meetingCaps);
        QueueRedraw();
    }

    public override void _GuiInput(InputEvent e)
    {
        switch (e)
        {
            case InputEventMouseMotion motion:
                Naming?.Hover(CapAt(motion.Position));
                break;
            case InputEventMouseButton { ButtonIndex: MouseButton.Left, Pressed: true } press when CapAt(press.Position) is int cap:
                GrabFocus(); Picked?.Invoke(cap); AcceptEvent();
                break;
            case InputEventKey { Pressed: true } key:
                int digit = key.Keycode is >= Key.Key0 and <= Key.Key9 ? (int)(key.Keycode - Key.Key0) : key.Keycode is >= Key.Kp0 and <= Key.Kp9 ? (int)(key.Keycode - Key.Kp0) : -1;
                if (digit >= 0 && Typed.Length < 3) Type(Typed + digit.ToString(CultureInfo.InvariantCulture));
                else if (key.Keycode == Key.Backspace && Typed.Length > 0) Type(Typed[..^1]);
                else if (key.Keycode == Key.Escape && Typed.Length > 0) Type("");
                else if (key.Keycode is Key.Enter or Key.KpEnter && CapNaming.Typed(Typed) is int typed) { Type(""); Picked?.Invoke(typed); }
                else break;
                AcceptEvent();
                break;
        }
    }

    private void Type(string text) { Typed = text; Safe.Raise(TypedChanged); }

    public void Show(CapLayout? layout, CapMapModel map)
    {
        Layout = layout; Map = map; members = new HashSet<int>(Threshold.Members(map)); Restyle();
    }

    // The cell size follows the theme's micro type: room for the widest numeral and a hairline gap.
    public void Restyle()
    {
        TokenSet t = tokens();
        font = ChromeTheme.Face(t, "ui");
        size = (int)t.TypeSize("ui", "micro");
        int pad = Math.Max(2, t.Px("space.1"));
        cell = new Vector2(Mathf.Ceil(font.GetStringSize(WidestNumeral, HorizontalAlignment.Left, -1, size).X) + 2 * pad, size + 2);
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
                var box = new Rect2(gutter + c * cell.X, r * cell.Y, cell.X, cell.Y);
                Cell(this, t, font, size, box, Map.Class(cap), Map.Numeral(cap)?.ToString(CultureInfo.InvariantCulture));
                Over(this, t, box, members.Contains(cap), meeting.Contains(cap), Named == cap);
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

    // The marks over a cell's pattern: the Query set outline at a threshold member, the meeting square, the selection
    // brackets at the named cap.
    public static void Over(CanvasItem on, TokenSet t, Rect2 box, bool member, bool meets, bool named)
    {
        float line = Math.Max(1, t.Px("border.width"));
        if (member) on.DrawRect(box.Grow(-line / 2), t.C("accent.strong"), false, Math.Max(1.5f, 1.5f * line));
        if (meets)
        {
            float side = Math.Max(3, Mathf.Floor(box.Size.Y * .3f));
            on.DrawRect(new Rect2(box.Position.X + line, box.End.Y - side - line, side, side), t.C("text"));
        }
        if (!named) return;
        float w = Math.Max(2, t.Px("focus.width")), arm = Math.Min(box.Size.X, box.Size.Y) * .45f;
        Color ink = t.C("state.selected");
        Rect2 outer = box.Grow(w / 2);
        foreach ((Vector2 corner, Vector2 dx, Vector2 dy) in new[]
        {
            (outer.Position, Vector2.Right, Vector2.Down), (new Vector2(outer.End.X, outer.Position.Y), Vector2.Left, Vector2.Down),
            (new Vector2(outer.Position.X, outer.End.Y), Vector2.Right, Vector2.Up), (outer.End, Vector2.Left, Vector2.Up),
        })
        {
            on.DrawLine(corner, corner + dx * arm, ink, w);
            on.DrawLine(corner, corner + dy * arm, ink, w);
        }
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
        string marks = (Threshold.Member(Map, cap) == true ? " · threshold member" : "") + (meeting.Contains(cap) ? " · meets the named cap" : "")
            + (Named == cap ? " · named" : "");
        return $"cap {cap} · ring {row + 1}, place {column + 1} · {CapMapModel.Word(k)} · {nc}{marks}";
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
