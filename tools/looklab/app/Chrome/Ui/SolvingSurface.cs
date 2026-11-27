using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The solving surface on the body of the locked pair (ADR-D01 revision 4, items 62 and 63): the relationship
// strip on the pair's shared edge, and the anchors it links to in the two views.

// A control on the seam between Global and Local is told which way the seam runs.
public interface IPairSeam { void Arrange(bool vertical); }

// The instruments of the strip: one glyph per item, the same in the strip and on its anchor.
public static class StripGlyphs
{
    public static string Token(string slot) => slot == "next" ? "strip.next" : "strip.cell." + slot;

    public static void Draw(CanvasItem on, string slot, Vector2 centre, float size, Color colour, float width = 1.5f)
    {
        float r = size / 2;
        switch (slot)
        {
            case "target":      // a ring with a centre: a position
                on.DrawArc(centre, r, 0, Mathf.Tau, 24, colour, width, true); on.DrawCircle(centre, r * .28f, colour); break;
            case "occupant":    // a filled cell: what sits there
                on.DrawRect(new Rect2(centre - new Vector2(r, r) * .72f, new Vector2(r, r) * 1.44f), colour); break;
            case "wanted":      // an outlined diamond: a piece that is looked for
                on.DrawPolyline(new[] { centre + new Vector2(0, -r), centre + new Vector2(r, 0), centre + new Vector2(0, r), centre + new Vector2(-r, 0), centre + new Vector2(0, -r) }, colour, width, true); break;
            case "buffer_a": case "buffer_b":   // a bracket, single for A and double for B
                Bracket(on, centre, r, colour, width);
                if (slot == "buffer_b") Bracket(on, centre, r * .5f, colour, width);
                break;
            case "target_frame":    // two axes
                on.DrawLine(centre + new Vector2(-r, r), centre + new Vector2(-r, -r), colour, width, true); on.DrawLine(centre + new Vector2(-r, r), centre + new Vector2(r, r), colour, width, true);
                on.DrawLine(centre + new Vector2(-r, r), centre + new Vector2(r * .5f, -r * .5f), colour, width, true); break;
            case "residual":    // a delta
                on.DrawPolyline(new[] { centre + new Vector2(0, -r), centre + new Vector2(r, r * .85f), centre + new Vector2(-r, r * .85f), centre + new Vector2(0, -r) }, colour, width, true); break;
            default:            // Next: a bookmark
                on.DrawPolyline(new[] { centre + new Vector2(-r * .6f, -r), centre + new Vector2(r * .6f, -r), centre + new Vector2(r * .6f, r), centre + new Vector2(0, r * .4f), centre + new Vector2(-r * .6f, r), centre + new Vector2(-r * .6f, -r) }, colour, width, true); break;
        }
    }

    private static void Bracket(CanvasItem on, Vector2 c, float r, Color colour, float width)
    {
        on.DrawPolyline(new[] { c + new Vector2(-r * .4f, -r), c + new Vector2(-r, -r), c + new Vector2(-r, r), c + new Vector2(-r * .4f, r) }, colour, width, true);
        on.DrawPolyline(new[] { c + new Vector2(r * .4f, -r), c + new Vector2(r, -r), c + new Vector2(r, r), c + new Vector2(r * .4f, r) }, colour, width, true);
    }
}

// One item of the strip: glyph, the owner's word, the value in the code face and a fact beside it.
internal sealed partial class StripCellView : PanelContainer
{
    public StripCell Cell { get; private set; }
    private readonly Func<TokenSet> tokens; private readonly Control glyph; private readonly Label word, value, detail;
    private float flash; private bool hovered;
    public event Action<string, bool>? Hover;
    public event Action<string>? Chosen;

    public StripCellView(StripCell cell, Func<TokenSet> tokens)
    {
        Cell = cell; this.tokens = tokens; Name = "Strip_" + cell.Slot; MouseFilter = MouseFilterEnum.Stop; FocusMode = FocusModeEnum.None;
        AddThemeStyleboxOverride("panel", new StyleBoxEmpty());
        TokenSet t = tokens(); int pad = t.Px("space.2");
        var margin = new MarginContainer { MouseFilter = MouseFilterEnum.Ignore };
        foreach (string side in new[] { "left", "right" }) margin.AddThemeConstantOverride("margin_" + side, pad);
        foreach (string side in new[] { "top", "bottom" }) margin.AddThemeConstantOverride("margin_" + side, Math.Max(2, pad / 2));
        AddChild(margin);
        var row = new HBoxContainer { MouseFilter = MouseFilterEnum.Ignore }; row.AddThemeConstantOverride("separation", pad); margin.AddChild(row);
        glyph = new Control { CustomMinimumSize = new Vector2(t.Px("icon.small"), t.Px("icon.small")), MouseFilter = MouseFilterEnum.Ignore, SizeFlagsVertical = SizeFlags.ShrinkCenter };
        glyph.Draw += () => StripGlyphs.Draw(glyph, Cell.Slot, glyph.Size / 2, Math.Min(glyph.Size.X, glyph.Size.Y) - 3, Colour());
        row.AddChild(glyph);
        var column = new VBoxContainer { MouseFilter = MouseFilterEnum.Ignore, SizeFlagsHorizontal = SizeFlags.ExpandFill }; column.AddThemeConstantOverride("separation", 0); row.AddChild(column);
        word = ChromeWidgets.Text("", "MicroLabel"); column.AddChild(word);
        var line = new HBoxContainer { MouseFilter = MouseFilterEnum.Ignore }; line.AddThemeConstantOverride("separation", pad); column.AddChild(line);
        value = ChromeWidgets.Text("", "ReadoutLabel"); detail = ChromeWidgets.Text("", "DimLabel"); line.AddChild(value); line.AddChild(detail);
        MouseEntered += () => { hovered = true; QueueRedraw(); Safe.Raise(Hover, Cell.Slot, true); };
        MouseExited += () => { hovered = false; QueueRedraw(); Safe.Raise(Hover, Cell.Slot, false); };
        Set(cell);
    }

    private Color Colour() => tokens().C(Cell.State == "invalid" ? "state.invalid" : StripGlyphs.Token(Cell.Slot), Cell.State == "unset" ? .45f : 1);

    // With `undefined` (ADR-D02.6) the cell's numeral is numeral.undefined, said in its word, whatever the value.
    public void Set(StripCell cell, bool undefined = false)
    {
        Cell = cell; TokenSet t = tokens();
        // The state is said in words too, never by colour alone.
        string mark = cell.State switch { "derived" => "derived", "match" => "is Wanted", "mismatch" => "not Wanted", "invalid" => "invalid", "other_orbit" => "other orbit", _ => "" };
        word.Text = cell.Word + (mark.Length > 0 ? " · " + mark : "");
        word.AddThemeColorOverride("font_color", t.C(cell.State == "invalid" ? "state.invalid" : "text.dim"));
        value.Text = undefined ? Numeral.Undefined.Word : cell.Value;
        value.AddThemeColorOverride("font_color", t.C(undefined ? Numeral.Undefined.Role : cell.State == "unset" ? "text.dim" : "text.primary"));
        detail.Text = undefined ? "" : cell.Detail;
        glyph.QueueRedraw(); QueueRedraw();
    }

    // A strip change (motion row strip.change): the cell's edge shows once and settles. No motion when reduced.
    public void Flash(double seconds)
    {
        if (seconds <= 0) return;
        flash = 1; Tween tween = CreateTween(); tween.TweenMethod(Callable.From<float>(v => { flash = v; QueueRedraw(); }), 1f, 0f, seconds);
    }

    public override void _Draw()
    {
        TokenSet t = tokens(); var all = new Rect2(Vector2.Zero, Size);
        if (Cell.State == "invalid") DrawRect(all.Grow(-1), t.C("state.invalid"), false, 1);
        else if (Cell.State == "unset") DrawDashedLine(new Vector2(2, Size.Y - 1), new Vector2(Size.X - 2, Size.Y - 1), t.C("panel.border"), 1, 3);
        if (hovered) DrawRect(all.Grow(-1), t.C("strip.link"), false, 1);
        if (flash > 0) DrawRect(all.Grow(-1), t.C(StripGlyphs.Token(Cell.Slot), flash), false, 2);
    }

    public override void _GuiInput(InputEvent @event)
    {
        if (@event is InputEventMouseButton { Pressed: true, ButtonIndex: MouseButton.Left }) { Safe.Raise(Chosen, Cell.Slot); AcceptEvent(); }
    }
}

// The relationship strip: six items and the Next bookmark at its end, on the seam of the pair.
public sealed partial class RelationStrip : PanelContainer, IPairSeam
{
    public SolvingDemo Demo { get; }
    private readonly Func<TokenSet> tokens; private readonly Func<string, double> seconds;
    private readonly Dictionary<string, StripCellView> cells = new(StringComparer.Ordinal);
    private BoxContainer box = new VBoxContainer(); private bool vertical = true; private bool expanded; private string built = "";
    private Button? demoChip; private Label? problems; private Control? classes;
    // The item under the pointer, for the anchors' link; null when none.
    public string? Linked { get; private set; }
    public event Action? LinkChanged;
    // A click on an item asks for its command: (command, argument).
    public event Action<string, string?>? Command;
    public event Action? DemoNext;
    public IReadOnlyList<string> Slots => cells.Keys.ToArray();
    public string Said(string slot) => cells[slot].Cell.Word + " " + cells[slot].Cell.Value + " " + cells[slot].Cell.Detail + " " + cells[slot].Cell.State;
    public bool Expanded => expanded;
    // ADR-D02.6: with Jumble on and pieces off the lattice the strip shows numeral.undefined as a whole; the host
    // switches it on the exact lattice flag (JumbleNumerals), never on N.
    public bool Undefined { get; private set; }
    public void SetUndefined(bool on) { if (Undefined == on) return; Undefined = on; Refresh(false); }

    public RelationStrip(SolvingDemo demo, Func<TokenSet> tokens, Func<string, double> seconds)
    {
        Demo = demo; this.tokens = tokens; this.seconds = seconds; Name = "RelationStrip"; ThemeTypeVariation = "StripGround";
        demo.Changed += () => { if (IsInstanceValid(this)) Refresh(true); };
        Refresh(false);
    }

    public void Arrange(bool vertical) { if (this.vertical != vertical) { this.vertical = vertical; built = ""; Refresh(false); } }
    public void ToggleResidual() { expanded = !expanded; built = ""; Refresh(false); }

    public void Refresh(bool changed)
    {
        TokenSet t = tokens(); SolvingState state = Demo.State; IReadOnlyList<StripCell> now = state.Cells();
        string key = (vertical ? "v" : "h") + t.GetHashCode() + (expanded ? "x" : "");
        if (key != built) Build(t, now);
        built = key;
        foreach (StripCell cell in now)
        {
            cells[cell.Slot].Set(cell, Undefined);
            if (changed && Demo.LastChanged.Contains(cell.Slot)) cells[cell.Slot].Flash(seconds("strip.change"));
        }
        demoChip!.Text = vertical ? state.Name : SolvingDemo.Mark + " · " + state.Name;
        var bad = state.Problems();
        problems!.Text = string.Join("\n", bad.Select(p => "invalid: " + p.Text)); problems.Visible = bad.Count > 0;
        if (classes != null)
        {
            foreach (Node child in classes.GetChildren()) { classes.RemoveChild(child); child.QueueFree(); }
            foreach ((string name, string numeral) in state.Classes(state.Orbit))
            {
                var row = new HBoxContainer(); Label words = ChromeWidgets.Text(name, "MicroLabel"); words.SizeFlagsHorizontal = SizeFlags.ExpandFill; row.AddChild(words);
                Label number = ChromeWidgets.Text(Undefined ? Numeral.Undefined.Word : numeral, "ReadoutLabel");
                number.AddThemeColorOverride("font_color", t.C(Undefined ? Numeral.Undefined.Role : numeral == "unknown" ? "text.dim" : "text.primary")); row.AddChild(number);
                classes.AddChild(row);
            }
        }
    }

    public void Recolour() { built = ""; Refresh(false); }

    private void Build(TokenSet t, IReadOnlyList<StripCell> now)
    {
        foreach (Node child in GetChildren()) { RemoveChild(child); child.QueueFree(); }
        cells.Clear(); classes = null;
        box = vertical ? new VBoxContainer() : new HBoxContainer(); box.AddThemeConstantOverride("separation", vertical ? 1 : t.Px("space.2")); AddChild(box);
        CustomMinimumSize = vertical ? new Vector2(t.Px("strip.width"), 0) : Vector2.Zero;
        demoChip = new Button { Name = "DemoState", ThemeTypeVariation = "ToolButton", TooltipText = "Demo data, not the engine's: switch between the three demo states (F11)", ClipText = true, Alignment = HorizontalAlignment.Left };
        demoChip.AddThemeColorOverride("font_color", t.C("state.stale")); demoChip.Pressed += () => Safe.Raise(DemoNext);
        if (vertical) { Label mark = ChromeWidgets.Text(" " + SolvingDemo.Mark, "MicroLabel"); mark.Name = "DemoMark"; mark.AddThemeColorOverride("font_color", t.C("state.stale")); box.AddChild(mark); }
        box.AddChild(demoChip);
        foreach (StripCell cell in now)
        {
            if (cell.Slot == "next")
            {
                // Next is kept apart from the relation: a quiet bookmark at the strip's end.
                box.AddChild(new Control { SizeFlagsVertical = vertical ? SizeFlags.ExpandFill : SizeFlags.Fill, SizeFlagsHorizontal = vertical ? SizeFlags.Fill : SizeFlags.ExpandFill, MouseFilter = MouseFilterEnum.Ignore });
                box.AddChild(vertical ? new HSeparator() : new VSeparator());
            }
            var view = new StripCellView(cell, tokens);
            view.TooltipText = cell.Slot switch
            {
                "target" => "Target: a position. Click or press the key to step through the unfinished positions.",
                "occupant" => "Occupant: the piece that sits in the Target now.",
                "wanted" => "Wanted: the piece that belongs in the Target, and where it sits now. Under Home it is derived from the Target.",
                "buffer_a" => "Buffer A: a position, with its Occupant. Click to move it to the following unfinished position.",
                "buffer_b" => "Buffer B: a position, with its Occupant. Click to move it to the following unfinished position.",
                "target_frame" => "Target frame: Home or Captured. Click to change it.",
                "residual" => "Residual of the working orbit: positions not at Home. Click for the exact classes.",
                _ => "Next: a bookmarked piece. Click to activate it.",
            };
            view.Hover += (slot, on) => { string? to = on ? slot : Linked == slot ? null : Linked; if (to != Linked) { Linked = to; Safe.Raise(LinkChanged); } };
            view.Chosen += Choose;
            cells[cell.Slot] = view; box.AddChild(view);
            // ADR-D01.68: the setup slot sits under the Target cell; its buttons ask for their commands like the cells.
            if (cell.Slot == "target" && Demo.Setup is { } setup)
            {
                var slot = new SetupSlotView(setup, tokens);
                slot.Command += (command, argument) => Safe.Raise(Command, command, argument);
                box.AddChild(slot);
            }
            if (cell.Slot == "residual" && expanded)
            {
                classes = new VBoxContainer { Name = "ResidualClasses" }; var inset = new MarginContainer();
                inset.AddThemeConstantOverride("margin_left", t.Px("space.3")); inset.AddThemeConstantOverride("margin_right", t.Px("space.2")); inset.AddChild(classes); box.AddChild(inset);
            }
        }
        problems = ChromeWidgets.Text("", "MicroLabel"); problems.Name = "TupleProblems"; problems.AutowrapMode = TextServer.AutowrapMode.WordSmart;
        problems.AddThemeColorOverride("font_color", t.C("state.invalid")); box.AddChild(problems); box.MoveChild(problems, vertical ? 2 : 1);
    }

    private void Choose(string slot)
    {
        switch (slot)
        {
            case "target": case "occupant": case "wanted": Safe.Raise(Command, "target.current.set", null); break;
            case "buffer_a": Safe.Raise(Command, "buffer.assign", "A"); break;
            case "buffer_b": Safe.Raise(Command, "buffer.assign", "B"); break;
            case "target_frame": Safe.Raise(Command, "target.frame.set", null); break;
            case "residual": ToggleResidual(); break;
            default: Safe.Raise(Command, "target.next.activate", null); break;
        }
    }
}

// The anchors of the strip in one view. Position anchors (Target, A, B) stay on positions; piece anchors (Wanted,
// Next) sit where the piece sits now, so they move when it moves. Each sits where the view's own projection puts
// its point (StripAnchors), so it stays on its geometry when the camera turns; with no projection none is drawn.
public sealed partial class AnchorLayer : Control
{
    private readonly RelationStrip strip; private readonly Func<TokenSet> tokens; private readonly bool marker; private readonly Func<Vector2> seam;
    // The view's projection of a position of an orbit, as a place from 0 to 1: set by the host.
    public Func<int, int, (double X, double Y)?>? Project { get; set; }
    private string drawn = "";
    // The grip marker of the Local view takes the route's pattern: broken for Draft, whole for Direct. Null draws none.
    public Func<bool>? GripDraft { get; set; }

    // marker: the smaller marks of the Global view. seam: the point of this view's edge that lies on the seam, 0 to 1.
    public AnchorLayer(RelationStrip strip, Func<TokenSet> tokens, bool marker, Func<Vector2> seam)
    {
        this.strip = strip; this.tokens = tokens; this.marker = marker; this.seam = seam; Name = "AnchorLayer"; MouseFilter = MouseFilterEnum.Ignore;
        SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        strip.Demo.Changed += Again; strip.LinkChanged += Again;
        TreeExiting += () => { strip.Demo.Changed -= Again; strip.LinkChanged -= Again; };
    }

    private void Again() { if (IsInstanceValid(this)) QueueRedraw(); }

    // Where each anchor is, by slot: (place in the view from 0 to 1, on a piece).
    public IReadOnlyDictionary<string, (Vector2 At, bool OnPiece)> Anchors() =>
        StripAnchors.Of(strip.Demo.State, Project).ToDictionary(a => a.Key, a => (new Vector2((float)a.Value.X, (float)a.Value.Y), a.Value.OnPiece), StringComparer.Ordinal);

    // The camera turns without a change of the state: the anchors are placed again each frame and drawn again when
    // one has moved by half a pixel.
    private string Placed() => string.Join(";", Anchors().OrderBy(a => a.Key, StringComparer.Ordinal).Select(a => $"{a.Key}:{Mathf.Round(a.Value.At.X * Size.X * 2)},{Mathf.Round(a.Value.At.Y * Size.Y * 2)}"));
    public override void _Process(double delta) { if (Project != null && IsVisibleInTree() && Placed() != drawn) QueueRedraw(); }

    public override void _Draw()
    {
        drawn = Placed();
        TokenSet t = tokens(); float size = t.Px("icon.small") * (marker ? .8f : 1.2f);
        if (GripDraft is { } route)
        {
            bool draft = route(); float grip = size * 2.2f;
            RouteMark.Draw(this, new Rect2(Size / 2 - new Vector2(grip, grip) / 2, new Vector2(grip, grip)), draft, t.C(draft ? "mark.route.draft" : "mark.route.direct"), 2);
        }
        foreach (var anchor in Anchors())
        {
            Vector2 at = anchor.Value.At * Size; bool linked = strip.Linked == anchor.Key || (strip.Linked == "occupant" && anchor.Key == "target");
            Color colour = t.C("anchor." + anchor.Key);
            // A piece anchor is drawn around the position it sits on, so both stay readable in one place.
            float mine = anchor.Value.OnPiece ? size * 1.7f : size;
            if (linked) { DrawLine(seam() * Size, at, t.C("strip.link"), 1, true); DrawArc(at, mine * .95f, 0, Mathf.Tau, 32, t.C("strip.link"), 1, true); }
            DrawCircle(at, mine * .5f + 2, t.C("window.background", anchor.Value.OnPiece ? 0 : .55f));
            StripGlyphs.Draw(this, anchor.Key, at, mine, colour, linked ? 2.2f : 1.6f);
        }
    }
}
