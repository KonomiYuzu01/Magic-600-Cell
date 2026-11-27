using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The orbit rail on the body of the locked pair (ADR-D01.64): one cell for each of the 35 orbits, in an order that
// never depends on the state. Nothing on it moves at idle and no cell is emphasised for being unfinished.
public sealed partial class OrbitRail : PanelContainer
{
    public SolvingDemo Demo { get; }
    public OrbitJump Jump { get; } = new();
    public bool ByOrbitId { get; private set; }
    public int? Expanded { get; private set; }
    public int? Hovered => cells.Hover;
    // ADR-D02.6: with Jumble on and pieces off the lattice the rail shows numeral.undefined as a whole.
    public bool Undefined { get; private set; }
    public void SetUndefined(bool on) { if (Undefined == on) return; Undefined = on; Refresh(); }
    private readonly Func<TokenSet> tokens; private readonly RailCells cells; private readonly Label head, typed; private readonly Button order; private readonly VBoxContainer column; private Control? expansion;
    public event Action<string, string?>? Command;

    public OrbitRail(SolvingDemo demo, Func<TokenSet> tokens)
    {
        Demo = demo; this.tokens = tokens; Name = "OrbitRail"; ThemeTypeVariation = "RailGround";
        HelpTopics.Mark(this, "orbit");
        column = new VBoxContainer(); column.AddThemeConstantOverride("separation", 0); AddChild(column);
        var top = new HBoxContainer(); column.AddChild(top);
        head = ChromeWidgets.Text("", "MicroLabel", true); head.SizeFlagsHorizontal = SizeFlags.ExpandFill; top.AddChild(head);
        typed = ChromeWidgets.Text("", "ReadoutLabel"); top.AddChild(typed);
        order = new Button { Name = "RailOrder", ThemeTypeVariation = "ToolButton", TooltipText = "A setting: the seed stage order, or orbit-id order. The state never changes the order." };
        order.Pressed += () => SetOrder(!ByOrbitId); top.AddChild(order);
        cells = new RailCells(this, tokens); column.AddChild(cells);
        demo.Changed += () => { if (IsInstanceValid(this)) Refresh(); };
        Refresh();
    }

    // Raised when the order setting changes, so the signatures drawn elsewhere follow the rail.
    public event Action? OrderChanged;
    public void SetOrder(bool byId) { ByOrbitId = byId; Refresh(); Safe.Raise(OrderChanged); }
    public IReadOnlyList<RailCell> Cells() => OrbitRailModel.Cells(Demo.State, ByOrbitId);
    public void Recolour() => Refresh();

    // A click selects the working orbit; a click on the working orbit opens and closes its cell in place.
    public void Choose(int orbit)
    {
        if (orbit == Demo.State.Orbit) { Expanded = Expanded == orbit ? null : orbit; Refresh(); }
        else Safe.Raise(Command, "orbit.switch", OrbitAtlas.Name(orbit));
    }

    public void Expand(int? orbit) { Expanded = orbit; Refresh(); }

    // B26 (ADR-D02.28): a band drawn under the cells, in the same column, so its 35 columns align with theirs.
    public void AddBand(Control band)
    {
        band.GetParent()?.RemoveChild(band);
        column.AddChild(band); column.MoveChild(band, cells.GetIndex() + 1);
    }

    public void Refresh()
    {
        TokenSet t = tokens();
        head.Text = $" Orbits · {(ByOrbitId ? "orbit-id order" : "seed stage order")}: {OrbitRailModel.OrderLabel} · {SolvingDemo.Mark}";
        head.AddThemeColorOverride("font_color", t.C("text.dim"));
        order.Text = ByOrbitId ? "by id" : "by stage";
        typed.Text = Jump.Held ? "O" + Jump.Typed.PadRight(2, '_') : Undefined ? Numeral.Undefined.Word : "";
        typed.AddThemeColorOverride("font_color", t.C(Jump.Held || !Undefined ? "rail.cell.working" : Numeral.Undefined.Role));
        cells.CustomMinimumSize = new Vector2(0, t.Px("rail.height")); cells.QueueRedraw();
        if (expansion != null) { column.RemoveChild(expansion); expansion.QueueFree(); expansion = null; }
        if (Expanded is { } open) { expansion = Expansion(open, t); column.AddChild(expansion); }
    }

    private static string Names(IEnumerable<int> ids) { string[] all = ids.Select(OrbitAtlas.Name).ToArray(); return all.Length == 0 ? "none" : string.Join(" ", all); }

    // The cell opened in place: the exact residual classes with exact numerals, then the unfinished positions.
    private Control Expansion(int orbit, TokenSet t)
    {
        SolvingState s = Demo.State; OrbitFacts facts = s.Atlas.Orbits[orbit];
        var inset = new MarginContainer { Name = "RailExpansion" };
        foreach (string side in new[] { "left", "right", "top", "bottom" }) inset.AddThemeConstantOverride("margin_" + side, t.Px("space.2"));
        var row = new HBoxContainer(); row.AddThemeConstantOverride("separation", t.Px("space.4")); inset.AddChild(row);
        var left = new VBoxContainer { CustomMinimumSize = new Vector2(t.Px("relation.width") * 1.5f, 0) }; left.AddThemeConstantOverride("separation", 0); row.AddChild(left);
        left.AddChild(ChromeWidgets.Text($"{OrbitAtlas.Name(orbit)} · {facts.Orientation} · {facts.Pieces} pieces · stage {facts.Stage} of {OrbitAtlas.Count}", "AccentLabel"));
        foreach ((string name, string numeral) in s.Classes(orbit))
        {
            var line = new HBoxContainer(); Label words = ChromeWidgets.Text(name, "MicroLabel"); words.SizeFlagsHorizontal = SizeFlags.ExpandFill; line.AddChild(words);
            Label number = ChromeWidgets.Text(Undefined ? Numeral.Undefined.Word : numeral, "ReadoutLabel");
            number.AddThemeColorOverride("font_color", t.C(Undefined ? Numeral.Undefined.Role : numeral == "unknown" ? "text.dim" : "text.primary")); line.AddChild(number);
            left.AddChild(line);
        }
        var right = new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill }; right.AddThemeConstantOverride("separation", 0); row.AddChild(right);
        IReadOnlyList<int> open = s.Displaced(orbit).Keys.OrderBy(p => p).ToArray();
        const int shown = 120;
        right.AddChild(ChromeWidgets.Text(s.Residuals[orbit].NotHome == null ? "Unfinished positions: unknown" : $"Unfinished positions, in position-id order: {open.Count}", "MicroLabel"));
        Label list = ChromeWidgets.Text(string.Join("  ", open.Take(shown).Select(OrbitAtlas.Position)) + (open.Count > shown ? $"  … and {open.Count - shown} more" : ""), "ReadoutLabel");
        list.Name = "UnfinishedPositions"; list.AutowrapMode = TextServer.AutowrapMode.WordSmart; list.MaxLinesVisible = 4; list.SizeFlagsHorizontal = SizeFlags.ExpandFill; right.AddChild(list);
        Label seeds = ChromeWidgets.Text($"Its seed touches: {Names(facts.Touches)}\nTouched by the seeds of: {Names(s.Atlas.TouchedBy(orbit))}", "MicroLabel");
        seeds.AutowrapMode = TextServer.AutowrapMode.WordSmart; seeds.AddThemeColorOverride("font_color", t.C("text.dim")); right.AddChild(seeds);
        return inset;
    }

    // A held orbit key followed by two digits selects the working orbit. While the key is held the digits are the
    // rail's, and the rail says so beside its label: no key changes meaning silently.
    public override void _Input(InputEvent @event)
    {
        if (@event is not InputEventKey key || !IsVisibleInTree()) return;
        if (key.PhysicalKeycode == Key.O && !key.CtrlPressed && !key.AltPressed && !key.ShiftPressed && !key.MetaPressed)
        {
            if (key.Pressed && GetViewport().GuiGetFocusOwner() is LineEdit or TextEdit) return;
            if (key.Pressed && !key.Echo) Jump.Hold(); else if (!key.Pressed) Jump.Release();
            if (!key.Echo) Refresh();
            GetViewport().SetInputAsHandled(); return;
        }
        if (!Jump.Held || !key.Pressed) return;
        int digit = key.PhysicalKeycode >= Key.Key0 && key.PhysicalKeycode <= Key.Key9 ? (int)(key.PhysicalKeycode - Key.Key0) : key.PhysicalKeycode >= Key.Kp0 && key.PhysicalKeycode <= Key.Kp9 ? (int)(key.PhysicalKeycode - Key.Kp0) : -1;
        if (digit < 0) return;
        GetViewport().SetInputAsHandled();
        if (!key.Echo) Type(digit);
    }

    // The same jump for a host that feeds the digits itself (a test, a key map).
    public void Type(int digit)
    {
        if (Jump.Digit(digit) is { } orbit) Safe.Raise(Command, "orbit.switch", OrbitAtlas.Name(orbit));
        Refresh();
    }

    public string Tip(int orbit)
    {
        SolvingState s = Demo.State; OrbitFacts f = s.Atlas.Orbits[orbit]; OrbitResidual r = s.Residuals[orbit];
        return $"{OrbitAtlas.Name(orbit)} · {f.Orientation} · {f.Pieces} pieces\nNot at Home: {OrbitResidual.Numeral(r.NotHome)} ({SolvingDemo.Mark})\nLock: {Protections.Words(r.Protection)}\nIts seed touches: {Names(f.Touches)}\nTouched by the seeds of: {Names(s.Atlas.TouchedBy(orbit))}";
    }
}

// The 35 cells, drawn as one control.
internal sealed partial class RailCells : Control
{
    private readonly OrbitRail rail; private readonly Func<TokenSet> tokens;
    public int? Hover { get; private set; }

    public RailCells(OrbitRail rail, Func<TokenSet> tokens)
    {
        this.rail = rail; this.tokens = tokens; Name = "RailCells"; MouseFilter = MouseFilterEnum.Stop; SizeFlagsHorizontal = SizeFlags.ExpandFill;
        MouseExited += () => { Hover = null; TooltipText = ""; QueueRedraw(); };
    }

    private float Step => Size.X / OrbitAtlas.Count;
    private int? At(Vector2 at) { var all = rail.Cells(); int i = (int)(at.X / Math.Max(1, Step)); return i >= 0 && i < all.Count && at.Y >= 0 && at.Y <= Size.Y ? all[i].Orbit : null; }

    public override void _GuiInput(InputEvent @event)
    {
        if (@event is InputEventMouseMotion motion)
        {
            int? now = At(motion.Position);
            if (now != Hover) { Hover = now; TooltipText = now is { } o ? rail.Tip(o) : ""; QueueRedraw(); }
        }
        else if (@event is InputEventMouseButton { Pressed: true, ButtonIndex: MouseButton.Left } click && At(click.Position) is { } orbit) { rail.Choose(orbit); AcceptEvent(); }
    }

    public override void _Draw()
    {
        TokenSet t = tokens(); var all = rail.Cells(); float step = Step, h = Size.Y; SolvingState s = rail.Demo.State;
        Font font = GetThemeFont("font", "MicroLabel"); int fontSize = Math.Max(8, GetThemeFontSize("font_size", "MicroLabel") - 1);
        // Static facts of the hovered cell only: never lit from the state.
        IReadOnlyList<int> touches = Hover is { } hv ? s.Atlas.Orbits[hv].Touches : Array.Empty<int>(), touchedBy = Hover is { } hw ? s.Atlas.TouchedBy(hw) : Array.Empty<int>();
        for (int i = 0; i < all.Count; i++)
        {
            RailCell cell = all[i]; float x = i * step; var box = new Rect2(x + 1, 1, step - 2, h - 2); float mid = x + step / 2;
            float line = fontSize + 2;
            DrawString(font, new Vector2(x, line - 1), cell.Id, HorizontalAlignment.Center, step, fontSize, t.C("rail.cell.id"));
            Group(cell.Group, new Vector2(mid, line + 7), 5, t.C("rail.cell.group"));
            // The residual level: length carries quantity. Unknown is hatched, apart from an empty track.
            var track = new Rect2(x + 3, line + 15, step - 6, 4);
            DrawRect(track, t.C("panel.border"), false, 1);
            if (cell.Level is { } level && !rail.Undefined) { if (level > 0) DrawRect(new Rect2(track.Position, new Vector2(Math.Max(2, track.Size.X * (float)level), track.Size.Y)), t.C("rail.status." + (cell.Status == "permitted_residual" ? "permitted_residual" : "open"))); }
            else for (float hx = track.Position.X; hx < track.End.X - 1; hx += 4) DrawLine(new Vector2(hx, track.End.Y), new Vector2(Math.Min(hx + 4, track.End.X), track.Position.Y), t.C(rail.Undefined ? Numeral.Undefined.Role : "rail.status.unknown_frame"), 1);
            // Three separate marks in three fixed places: solved, lock, Next.
            float my = line + 27, third = (step - 4) / 3;
            if (cell.Solved) { Vector2 c = new(x + 2 + third * .5f, my); DrawPolyline(new[] { c + new Vector2(-3, 0), c + new Vector2(-1, 2.5f), c + new Vector2(3.5f, -3) }, t.C("rail.cell.solved"), 1.6f, true); }
            if (cell.Lock != OrbitLock.None) Lock(new Vector2(x + 2 + third * 1.5f, my), cell.Protection, t);
            if (cell.Next) StripGlyphs.Draw(this, "next", new Vector2(x + 2 + third * 2.5f, my), 7, t.C("rail.cell.next"), 1.2f);
            if (cell.Working) { DrawRect(box, t.C("rail.cell.working"), false, 1); DrawRect(new Rect2(x + 1, h - 3, step - 2, 2), t.C("rail.cell.working")); }
            if (rail.Expanded == cell.Orbit) DrawLine(new Vector2(mid - 3, h - 5), new Vector2(mid + 3, h - 5), t.C("rail.cell.id"), 1);
            if (Hover == cell.Orbit) DrawRect(box, t.C("strip.link"), false, 1);
            if (touches.Contains(cell.Orbit)) DrawLine(new Vector2(x + 3, 1), new Vector2(x + step - 3, 1), t.C("strip.link"), 2);
            if (touchedBy.Contains(cell.Orbit)) DrawLine(new Vector2(x + 3, h - 1), new Vector2(x + step - 3, h - 1), t.C("strip.link"), 2);
        }
    }

    // One glyph kind per group type: trivial, C2, C5, D5, A5.
    private void Group(string group, Vector2 c, float r, Color colour)
    {
        Vector2[] Pentagon(float radius) => Enumerable.Range(0, 6).Select(k => c + new Vector2(Mathf.Sin(k * Mathf.Tau / 5), -Mathf.Cos(k * Mathf.Tau / 5)) * radius).ToArray();
        switch (group)
        {
            case "trivial": DrawCircle(c, 1.4f, colour); break;
            case "C2": DrawLine(c + new Vector2(-r, 0), c + new Vector2(r, 0), colour, 1, true); DrawCircle(c + new Vector2(-r, 0), 1.6f, colour); DrawCircle(c + new Vector2(r, 0), 1.6f, colour); break;
            case "C5": DrawPolyline(Pentagon(r), colour, 1, true); break;
            case "D5": DrawPolyline(Pentagon(r), colour, 1, true); DrawLine(c + new Vector2(0, -r), c + new Vector2(0, r), colour, 1, true); break;
            default: Vector2[] p = Pentagon(r); DrawPolyline(p, colour, 1, true); DrawPolyline(new[] { p[0], p[2], p[4], p[1], p[3], p[0] }, colour, 1, true); break;
        }
    }

    // ADR-D01.65: a whole-orbit lock is a closed, filled padlock. A partial lock is open, a little wider, with each kind's
    // mark in its own place in the body: positions a dot on the left, identities a bar in the middle, a block a square
    // on the right. Place and shape both tell the kinds apart.
    private void Lock(Vector2 c, ProtectionKinds kinds, TokenSet t)
    {
        if (kinds.HasFlag(ProtectionKinds.Orbit))
        {
            Color whole = t.C("rail.lock.whole");
            DrawRect(new Rect2(c + new Vector2(-3, -.5f), new Vector2(6, 4.5f)), whole); DrawArc(c + new Vector2(0, -.5f), 2.2f, Mathf.Pi, Mathf.Tau, 8, whole, 1.2f, true);
            return;
        }
        Color colour = t.C("rail.lock.partial");
        var body = new Rect2(c + new Vector2(-4, -.5f), new Vector2(8, 5));
        DrawRect(body, colour, false, 1); DrawArc(c + new Vector2(2.6f, -.5f), 2.2f, Mathf.Pi, Mathf.Tau, 8, colour, 1.2f, true);
        float y = body.Position.Y + body.Size.Y / 2;
        if (kinds.HasFlag(ProtectionKinds.Position)) DrawCircle(new Vector2(c.X - 2.1f, y), .9f, colour);
        if (kinds.HasFlag(ProtectionKinds.Identity)) DrawLine(new Vector2(c.X, y - 1.4f), new Vector2(c.X, y + 1.4f), colour, 1);
        if (kinds.HasFlag(ProtectionKinds.Block)) DrawRect(new Rect2(c.X + 1.3f, y - .9f, 1.8f, 1.8f), colour);
    }

    // The locks in words, cell by cell (ADR-D01.65), for the screen text: drawn marks are spelled as their tooltips are.
    public string LockWords => string.Join("\n", rail.Cells().Where(cell => cell.Protection != ProtectionKinds.None)
        .Select(cell => $"{cell.Id} lock: {Protections.Words(cell.Protection)}"));
}
