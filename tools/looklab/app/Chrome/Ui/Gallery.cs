using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// Parts of the frame's body that more than one host uses.
public static class ChromeBody
{
    // The command to icon map radial menus and rows read (ADR-D01.53).
    public static CommandIcons? Commands { get; set; }

    // The one place that turns a colour of the design data into an engine colour.
    public static Color Engine(Rgb c) => new Color((float)c.R, (float)c.G, (float)c.B);

    // The five designed states of a pane (ADR-D01.6).
    public static readonly IReadOnlyList<(string Id, string Icon, string Words)> PaneStates = Array.AsReadOnly(new[]
    {
        ("empty", "state.empty", "Nothing here yet"), ("loading", "state.loading", "Loading"), ("notdrawn", "state.notdrawn", "Not drawn yet"),
        ("error", "state.error", "Could not show this"), ("stale", "state.stale", "Shown from before the last change"),
    });
    public static Control PaneState(string id, string? words = null)
    {
        var state = PaneStates.First(s => s.Id == id);
        Control shown = ChromeWidgets.Empty(state.Icon, words ?? state.Words); shown.Name = "PaneState" + id;
        return shown;
    }
}

// A readout cell of the instrument strip: a caption glyph and a value in the code face, in an inset cell.
public sealed partial class Readout : PanelContainer
{
    private readonly Label icon = new() { ThemeTypeVariation = "DimLabel", VerticalAlignment = VerticalAlignment.Center };
    private readonly Label value = new() { ThemeTypeVariation = "ReadoutLabel", VerticalAlignment = VerticalAlignment.Center };
    private readonly string glyph; private readonly Func<TokenSet> tokens;
    public string Value => value.Text;
    // The expanded state: a click (or Expand) opens a second line of words beside the value.
    private readonly Label detail = new() { ThemeTypeVariation = "DimLabel", VerticalAlignment = VerticalAlignment.Center, Visible = false };
    private readonly Control lamp = new() { MouseFilter = MouseFilterEnum.Ignore };
    private readonly string about;
    public LampState Lamp { get; private set; }
    public bool Expanded => detail.Visible;
    public string Detail => detail.Text;
    public void Expand(bool open) { detail.Visible = open && detail.Text.Length > 0; }
    public void Show(ReadoutFact fact)
    {
        Set(fact.Value); if (detail.Text != fact.Detail) detail.Text = fact.Detail;
        string tip = $"{about}\n{fact.Detail}\nLamp: {fact.LampSays}. Click to open or close the detail."; if (TooltipText != tip) TooltipText = tip;
        if (Lamp != fact.Lamp) { Lamp = fact.Lamp; lamp.QueueRedraw(); }
    }
    // The lamp is told apart by shape: a ring when idle, filled when active, filled inside a ring for attention.
    private void DrawLamp()
    {
        TokenSet t = tokens(); Vector2 c = lamp.Size / 2; float r = Math.Min(lamp.Size.X, lamp.Size.Y) * .3f;
        if (Lamp == LampState.Idle) { lamp.DrawArc(c, r, 0, Mathf.Tau, 16, t.C("lamp.idle"), 1, true); return; }
        Color lit = t.C(Lamp == LampState.Active ? "lamp.active" : "lamp.attention"); lamp.DrawCircle(c, r * (Lamp == LampState.Active ? 1 : .6f), lit);
        if (Lamp == LampState.Attention) lamp.DrawArc(c, r * 1.25f, 0, Mathf.Tau, 16, lit, 1, true);
    }

    public Readout(string name, string glyph, string tooltip, Func<TokenSet> tokens)
    {
        Name = name; this.glyph = glyph; this.tokens = tokens; about = tooltip; ThemeTypeVariation = "ReadoutCell"; TooltipText = tooltip; SizeFlagsVertical = SizeFlags.ShrinkCenter;
        var row = new HBoxContainer { MouseFilter = MouseFilterEnum.Ignore }; AddChild(row); lamp.CustomMinimumSize = new Vector2(12, 12); lamp.Draw += DrawLamp;
        row.AddChild(lamp); row.AddChild(icon); row.AddChild(value); row.AddChild(detail);
        MouseDefaultCursorShape = CursorShape.PointingHand; GuiInput += e => { if (e is InputEventMouseButton { Pressed: true, ButtonIndex: MouseButton.Left }) Expand(!Expanded); };
        Dress();
    }
    public void Dress() => ChromeIcons.Apply(icon, glyph, tokens().Px("icon.small"));
    public void Set(string text) { if (value.Text != text) value.Text = text; }
}

// Pick-layer keys beside a view (ADR-D01.46): one layer picks at a time, and one key cycles how the rest shows.
public sealed partial class LayerKeys : HBoxContainer
{
    public FocusLayers Layers { get; }
    private readonly Func<TokenSet> tokens; private readonly Func<string, string> keyOf;
    private readonly Dictionary<string, Button> keys = new(StringComparer.Ordinal);
    private readonly Button cycle; private readonly Label mode = ChromeWidgets.Text("", "MicroLabel");

    public LayerKeys(FocusLayers layers, Func<TokenSet> tokens, Func<string, string> keyOf)
    {
        Layers = layers; this.tokens = tokens; this.keyOf = keyOf; Name = "LayerKeys";
        foreach (var layer in FocusLayers.Layers)
        {
            Button key = ChromeWidgets.Tool("", $"Pick {layer.Name.ToLowerInvariant()} ({keyOf(layer.Id)})", true); key.Name = "Layer" + layer.Id;
            ChromeIcons.Apply(key, layer.Icon, tokens().Px("icon.small"));
            string id = layer.Id; key.Pressed += () => { Layers.SetPick(id); Refresh(); };
            keys[id] = key; AddChild(key);
        }
        cycle = ChromeWidgets.Tool("", ""); cycle.Name = "LayerCycle"; ChromeIcons.Apply(cycle, "layer.cycle", tokens().Px("icon.small"));
        cycle.Pressed += () => Layers.Cycle();
        AddChild(cycle); AddChild(mode);
        Layers.Changed += Refresh; TreeExiting += () => Layers.Changed -= Refresh;
        Refresh();
    }

    private void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        foreach (var key in keys) key.Value.SetPressedNoSignal(key.Key == Layers.Pick);
        string now = Layers.Inactive.ToString().ToLowerInvariant();
        cycle.TooltipText = $"The other layers show {now}. Press for the next way ({keyOf("cycle")})";
        mode.Text = Layers.Inactive == InactiveMode.Normal ? "" : now;
    }
}

// One motion of the table, played on a track: a mark that travels, goes out and back, or fades.
public sealed partial class MotionStage : Control
{
    private readonly Func<TokenSet> tokens;
    private double began = -1, seconds; private double[] curve = { 0, 0, 1, 1 }; private string kind = "move";
    public bool Playing => began >= 0;

    public MotionStage(Func<TokenSet> tokens) { this.tokens = tokens; Name = "MotionStage"; CustomMinimumSize = new Vector2(0, tokens().Px("control.height") * 1.5f); SetProcess(false); }

    public static string KindOf(MotionEntry entry) =>
        entry.Id.Contains("preview", StringComparison.Ordinal) || entry.Id.Contains("pending", StringComparison.Ordinal) ? "return" :
        entry.Duration is "flash" or "settle" ? "fade" : "move";

    public void Play(MotionEntry entry, double seconds, double[] curve)
    {
        this.seconds = seconds; this.curve = curve; kind = KindOf(entry);
        began = seconds <= 0 ? -1 : Time.GetTicksMsec() / 1000.0; SetProcess(began >= 0); QueueRedraw();
    }

    public override void _Process(double delta)
    {
        if (began >= 0 && Time.GetTicksMsec() / 1000.0 - began > seconds) { began = -1; SetProcess(false); }
        QueueRedraw();
    }

    public override void _Draw()
    {
        TokenSet t = tokens(); float pad = t.Px("space.3"), r = t.Px("icon.small") / 2f, y = Size.Y / 2;
        double share = began < 0 ? 1 : MotionTable.Ease(curve, (Time.GetTicksMsec() / 1000.0 - began) / seconds);
        DrawLine(new Vector2(pad, y), new Vector2(Size.X - pad, y), t.C("separator"), t.Px("border.width"));
        // The static equivalent is the picture at rest: the mark at its end, back at its start, or the lasting bar.
        if (kind == "fade")
        {
            DrawRect(new Rect2(pad, y - r, Size.X - 2 * pad, 2 * r), t.C("state.selected", (float)(1 - share) * .6f));
            DrawRect(new Rect2(pad, y - r, t.Px("focus.width"), 2 * r), t.C("state.selected"));
            return;
        }
        double at = kind == "return" ? 1 - Math.Abs(2 * share - 1) : share;
        var centre = new Vector2((float)(pad + r + (Size.X - 2 * pad - 2 * r) * at), y);
        DrawRect(new Rect2(centre - new Vector2(r, r), new Vector2(2 * r, 2 * r)), t.C(kind == "return" ? "state.preview" : "state.committed"));
    }
}

// A list of many rows that builds only the rows in view (ADR-D01.33).
public sealed partial class VirtualList : Control
{
    public int Rows { get; }
    public int Built => pool.Count;
    public event Action? Laid;
    private readonly Func<int, string> text; private readonly Func<TokenSet> tokens;
    private readonly VScrollBar bar = new(); private readonly List<Label> pool = new();

    public VirtualList(int rows, Func<int, string> text, Func<TokenSet> tokens)
    {
        Rows = rows; this.text = text; this.tokens = tokens; Name = "VirtualList"; ClipContents = true; FocusMode = FocusModeEnum.All;
        AddChild(bar); bar.SetAnchorsAndOffsetsPreset(LayoutPreset.RightWide);
        bar.ValueChanged += _ => Lay(); Resized += Lay;
    }

    public void ScrollTo(int row) { bar.Value = row * (double)tokens().Px("row.height"); Lay(); }

    public override void _GuiInput(InputEvent e)
    {
        float row = tokens().Px("row.height");
        if (e is InputEventMouseButton { Pressed: true, ButtonIndex: MouseButton.WheelDown }) { bar.Value += 3 * row; AcceptEvent(); }
        else if (e is InputEventMouseButton { Pressed: true, ButtonIndex: MouseButton.WheelUp }) { bar.Value -= 3 * row; AcceptEvent(); }
        else if (e is InputEventKey { Pressed: true } key && key.Keycode is Key.Down or Key.Up or Key.Pagedown or Key.Pageup)
        {
            bar.Value += key.Keycode switch { Key.Down => row, Key.Up => -row, Key.Pagedown => Size.Y, _ => -Size.Y }; AcceptEvent();
        }
    }

    private void Lay()
    {
        float row = tokens().Px("row.height");
        bar.MaxValue = Rows * (double)row; bar.Page = Math.Max(1, Size.Y);
        (int first, int count) = VirtualRows.Range(Rows, row, bar.Value, Size.Y);
        while (pool.Count < count) { Label label = ChromeWidgets.Text("", "ReadoutLabel", true); label.MouseFilter = MouseFilterEnum.Ignore; AddChild(label); pool.Add(label); }
        while (pool.Count > count) { Label last = pool[^1]; pool.RemoveAt(pool.Count - 1); RemoveChild(last); last.QueueFree(); }
        for (int i = 0; i < count; i++)
        {
            pool[i].Text = text(first + i); pool[i].Position = new Vector2(tokens().Px("space.2"), (float)((first + i) * (double)row - bar.Value));
            pool[i].Size = new Vector2(Math.Max(0, Size.X - bar.Size.X - 2 * tokens().Px("space.2")), row);
        }
        Safe.Raise(Laid);
    }
}

// A progress ring, a lamp and a strip cell drawn from tokens.
public sealed partial class GalleryDraw : Control
{
    private readonly Func<TokenSet> tokens; private readonly string shape; private readonly string role; private readonly double share;
    public GalleryDraw(string shape, string role, double share, Func<TokenSet> tokens)
    {
        this.shape = shape; this.role = role; this.share = share; this.tokens = tokens;
        int side = tokens().Px("control.height"); CustomMinimumSize = new Vector2(shape == "cell" ? side * 2 : side, side);
    }
    public override void _Draw()
    {
        TokenSet t = tokens(); Vector2 c = Size / 2; float r = Math.Min(Size.X, Size.Y) / 2 - t.Px("space.1");
        if (shape == "ring")
        {
            DrawArc(c, r, 0, Mathf.Tau, 48, t.C("separator"), t.Px("focus.width"), true);
            DrawArc(c, r, -Mathf.Pi / 2, (float)(-Mathf.Pi / 2 + Mathf.Tau * share), 48, t.C(role), t.Px("focus.width"), true);
        }
        else if (shape == "lamp") DrawCircle(c, r * .5f, t.C(role));
        else
        {
            DrawRect(new Rect2(Vector2.Zero, Size), t.C("field.background"));
            DrawRect(new Rect2(0, Size.Y - t.Px("focus.width"), (float)(Size.X * share), t.Px("focus.width")), t.C(role));
            DrawRect(new Rect2(Vector2.Zero, Size), t.C("control.border"), false, t.Px("border.width"));
        }
    }
}

// The component gallery (ADR-D01.33): every widget in every state under the appearance in force, the type and
// notation page, both icon tiers at 16 px, the motion table with a play button for each entry, the candidate
// sticker palettes with their distances, a list of 10,000 rows and the five designed states of a pane.
public sealed partial class ComponentGallery : VBoxContainer
{
    public static readonly IReadOnlyList<(string Id, string Name)> Pages = Array.AsReadOnly(new[]
    {
        ("widgets", "Widgets"), ("type", "Type"), ("icons", "Icons"), ("motion", "Motion"), ("palette", "Palette"), ("list", "List"), ("panes", "Panes"),
    });
    public static readonly string[] States = { "default", "hover", "pressed", "focused", "disabled", "error", "busy", "stale", "empty" };
    public static readonly string[] Kinds = { "Button", "Icon button", "Toggle", "Text field", "Number field", "Choice", "Slider", "Switch", "Row" };
    public const string Notation = "⇄ ↶ ↷ ⟲ ⟳ ∘ ⁻¹ ′ ² ³ × · → ↦ ∈ ⊂ ≅ ≠ ≤ ≥ ⟨ ⟩ ½ ° ± √ φ τ π ℤ … – — • ▸ ▾";
    public const int ListRows = 10000;

    public string Page { get; private set; } = "widgets";
    public string Vision { get; private set; } = "normal";
    public MotionSettings Motion { get; } = new();
    public MotionStage? Stage { get; private set; }
    public VirtualList? List { get; private set; }
    // The host carries these out: "preset", "contrast", "character", "density".
    public event Action<string>? Asked;
    public event Action<StickerPalette>? PaletteChosen;
    public event Action? MotionChanged;
    public string Said => said.Text;
    public string? Palette { get; set; }

    private readonly Func<TokenSet> tokens; private readonly MotionTable motions; private readonly IReadOnlyList<StickerPalette> palettes;
    private readonly IconSet icons; private readonly CommandIcons commandIcons; private readonly IReadOnlyCollection<string> commands;
    private bool dirty;
    private readonly HBoxContainer head = new(); private readonly Label said = ChromeWidgets.Text("", "MicroLabel", true);
    private readonly ScrollContainer scroll = new() { SizeFlagsVertical = SizeFlags.ExpandFill, HorizontalScrollMode = ScrollContainer.ScrollMode.Auto };
    private readonly VBoxContainer body = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill };

    public ComponentGallery(Func<TokenSet> tokens, MotionTable motions, IReadOnlyList<StickerPalette> palettes, IconSet icons, CommandIcons commandIcons, IReadOnlyCollection<string> commands)
    {
        this.tokens = tokens; this.motions = motions; this.palettes = palettes; this.icons = icons; this.commandIcons = commandIcons; this.commands = commands;
        Name = "ComponentGallery"; SizeFlagsHorizontal = SizeFlags.ExpandFill; SizeFlagsVertical = SizeFlags.ExpandFill;
        AddChild(head); AddChild(new HSeparator()); AddChild(scroll);
        var margin = new MarginContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsVertical = SizeFlags.ExpandFill }; scroll.AddChild(margin);
        foreach (string side in new[] { "margin_left", "margin_right", "margin_top", "margin_bottom" }) margin.AddThemeConstantOverride(side, tokens().Px("space.3"));
        margin.AddChild(body);
        Ready += Rebuild; VisibilityChanged += () => { if (dirty && IsVisibleInTree()) Rebuild(); };
    }

    public void ShowPage(string page) { if (Pages.Any(p => p.Id == page)) { Page = page; Rebuild(); } }
    private void Say(string text) => said.Text = text;

    // Everything is built again from the tokens in force; the host calls this after a change of the appearance.
    public void Rebuild()
    {
        if (!IsInsideTree()) return;
        // A gallery that is not on screen is built when it shows.
        if (!IsVisibleInTree()) { dirty = true; return; }
        dirty = false;
        TokenSet t = tokens();
        if (said.GetParent() == head) head.RemoveChild(said);
        SessionWidgets.Clear(head); SessionWidgets.Clear(body); Stage = null; List = null;
        foreach (var page in Pages)
        {
            Button tab = ChromeWidgets.Tool(page.Name, "Gallery page: " + page.Name, true, "TabButton"); tab.Name = "Page" + page.Id; tab.SetPressedNoSignal(page.Id == Page);
            string id = page.Id; tab.Pressed += () => ShowPage(id); head.AddChild(tab);
        }
        head.AddChild(new VSeparator());
        foreach (var ask in new[] { ("preset", "preset.next", "Next of the six presets: family and scene look"), ("contrast", "contrast.high", "High contrast on or off"),
            ("character", "character.swap", "Draw this family the other family's way, to compare"), ("density", "density.next", "Next density") })
        {
            string what = ask.Item1; head.AddChild(SessionWidgets.Tool(ask.Item2, ask.Item3, t, () => Safe.Raise(Asked, what)));
            head.GetChild(head.GetChildCount() - 1).Name = "Ask" + what;
        }
        said.SizeFlagsHorizontal = SizeFlags.ExpandFill; said.HorizontalAlignment = HorizontalAlignment.Right; head.AddChild(said);
        said.Text = $"{t.Theme.Name} · {(t.Lines ? "lines" : "tone")} · {t.Density}{(t.HighContrast ? " · high contrast" : "")}";
        scroll.Visible = true; body.SizeFlagsVertical = Page == "list" ? SizeFlags.ExpandFill : SizeFlags.ShrinkBegin;
        switch (Page)
        {
            case "widgets": Widgets(t); break;
            case "type": TypePage(t); break;
            case "icons": IconPage(t); break;
            case "motion": MotionPage(t); break;
            case "palette": PalettePage(t); break;
            case "list": ListPage(t); break;
            default: PanePage(t); break;
        }
    }

    private void Heading(string words) => body.AddChild(ChromeWidgets.Text(words.ToUpperInvariant(), "MicroLabel"));

    private StyleBox Ring(string type, string colour, TokenSet t)
    {
        StyleBoxFlat box = GetThemeStylebox("normal", type).Duplicate() as StyleBoxFlat ?? ChromeTheme.Box(t, "control.background", colour);
        box.BorderColor = t.C(colour); box.SetBorderWidthAll(t.Px("focus.width"));
        return box;
    }

    private static readonly Control Missing = new();
    // One widget in one state; a state the widget does not have gives no specimen.
    private Control? Specimen(string kind, string state, TokenSet t)
    {
        Control made; string type = "Button";
        switch (kind)
        {
            case "Button": made = new Button { Text = state == "empty" ? "" : "Stage" }; break;
            case "Icon button": { Button b = ChromeWidgets.Tool("", "Run"); ChromeIcons.Apply(b, "script.run", t.Px("icon.small")); made = b; break; }
            case "Toggle": { Button b = ChromeWidgets.Tool("", "Labels", true); ChromeIcons.Apply(b, "labels", t.Px("icon.small")); b.SetPressedNoSignal(state == "pressed"); made = b; break; }
            case "Text field": made = new LineEdit { Text = state == "empty" ? "" : "ring 3", PlaceholderText = "name", CustomMinimumSize = new Vector2(t.Px("control.height") * 3, 0) }; type = "LineEdit"; break;
            case "Number field": made = new LineEdit { Text = state == "empty" ? "" : state == "error" ? "= 0.1 +" : "0.125", PlaceholderText = "0.000", ThemeTypeVariation = "FieldValue", CustomMinimumSize = new Vector2(t.Px("control.height") * 3, 0) }; type = "LineEdit"; break;
            case "Choice": { var o = new OptionButton { CustomMinimumSize = new Vector2(t.Px("control.height") * 3, 0) }; if (state != "empty") { o.AddItem("Side by side"); o.AddItem("Stacked"); } made = o; type = "OptionButton"; break; }
            case "Slider": made = new HSlider { MinValue = 0, MaxValue = 1, Step = .01, Value = state == "empty" ? 0 : .6, CustomMinimumSize = new Vector2(t.Px("control.height") * 3, 0) }; type = "HSlider"; break;
            case "Switch": made = new CheckButton { ButtonPressed = state is "pressed" or "default" }; type = "CheckButton"; break;
            default:
            {
                var row = new PanelContainer { ThemeTypeVariation = state == "pressed" ? "RowSelected" : "Row", CustomMinimumSize = new Vector2(t.Px("control.height") * 3, 0) };
                row.AddChild(state == "empty" ? ChromeWidgets.Text("no moves", "DimLabel") : ChromeWidgets.Text("R3 · 72°", "ReadoutLabel")); made = row; type = "PanelContainer"; break;
            }
        }
        bool pressable = made is Button and not CheckButton and not OptionButton, flat = type is "Button" or "LineEdit" or "OptionButton";
        switch (state)
        {
            case "hover": if (!pressable) return null; made.AddThemeStyleboxOverride("normal", GetThemeStylebox("hover", made.ThemeTypeVariation.IsEmpty ? (StringName)"Button" : made.ThemeTypeVariation)); break;
            case "pressed":
                if (kind is "Text field" or "Number field" or "Choice" or "Slider") return null;
                if (kind is "Button" or "Icon button") made.AddThemeStyleboxOverride("normal", GetThemeStylebox("pressed", made.ThemeTypeVariation.IsEmpty ? (StringName)"Button" : made.ThemeTypeVariation));
                break;
            case "focused": if (!flat) return null; made.AddThemeStyleboxOverride("normal", Ring(type, "control.focus", t)); break;
            case "disabled":
                if (made is BaseButton off) off.Disabled = true; else if (made is LineEdit locked) locked.Editable = false; else if (made is Slider still) still.Editable = false; else made.Modulate = made.Modulate with { A = .4f };
                break;
            case "error": if (!flat) return null; made.AddThemeStyleboxOverride("normal", Ring(type, "severity.error", t)); made.TooltipText = "Refused: the value is not complete"; break;
            case "busy":
            {
                if (made is BaseButton held) held.Disabled = true; else if (made is LineEdit read) read.Editable = false; else if (made is Slider fixedAt) fixedAt.Editable = false;
                var line = new HBoxContainer(); Label wait = ChromeWidgets.Text("", "DimLabel"); ChromeIcons.Apply(wait, "state.loading", t.Px("icon.small")); wait.TooltipText = "Working"; wait.MouseFilter = MouseFilterEnum.Stop;
                line.AddChild(made); line.AddChild(wait); return line;
            }
            case "stale": made.Modulate = made.Modulate with { A = .5f }; made.TooltipText = "From before the last change"; break;
            case "empty": if (kind is "Icon button" or "Toggle" or "Switch") return null; break;
        }
        if (made is Button press && made.GetType() == typeof(Button)) press.Pressed += () => Say($"{kind} · {state}");
        return made;
    }

    private void Widgets(TokenSet t)
    {
        Heading("every widget in every state");
        var grid = new GridContainer { Columns = States.Length + 1, Name = "StateGrid" }; grid.AddThemeConstantOverride("h_separation", t.Px("space.3")); grid.AddThemeConstantOverride("v_separation", t.Px("space.2"));
        body.AddChild(grid);
        grid.AddChild(new Control());
        foreach (string state in States) grid.AddChild(ChromeWidgets.Text(state, "DimLabel"));
        foreach (string kind in Kinds)
        {
            grid.AddChild(ChromeWidgets.Text(kind, "DimLabel"));
            foreach (string state in States)
            {
                Control? made = Specimen(kind, state, t);
                if (made == null) { Label none = ChromeWidgets.Text("·", "DimLabel"); none.TooltipText = $"{kind} has no {state} state"; none.MouseFilter = MouseFilterEnum.Stop; made = none; }
                made.SizeFlagsHorizontal = SizeFlags.ShrinkBegin; made.SizeFlagsVertical = SizeFlags.ShrinkCenter; grid.AddChild(made);
            }
        }
        body.AddChild(new HSeparator());
        Heading("chips, marks and readouts");
        var chips = new HFlowContainer(); chips.AddThemeConstantOverride("h_separation", t.Px("space.3")); body.AddChild(chips);
        foreach (var role in new[] { ("committed", "state.committed"), ("preview", "state.preview"), ("script", "script.frame"), ("selection", "state.selected"), ("pending", "state.pending"), ("stale", "state.stale"), ("protected", "state.protected") })
            chips.AddChild(SessionWidgets.Chip(role.Item1, t.C(role.Item2), "The accent with this one role in " + t.Theme.Name));
        foreach (string key in new[] { "Ctrl", "Enter", "F9" }) chips.AddChild(ChromeWidgets.KeyCap(key));
        var revision = new Readout("SampleReadout", "readout.revision", "A readout cell: revision", tokens); revision.Set("r 0142"); chips.AddChild(revision);
        chips.AddChild(new GalleryDraw("lamp", "severity.ok", 1, tokens) { TooltipText = "Lamp: ready" });
        chips.AddChild(new GalleryDraw("lamp", "severity.warning", 1, tokens) { TooltipText = "Lamp: waiting" });
        chips.AddChild(new GalleryDraw("ring", "accent.base", .62, tokens) { TooltipText = "Progress ring: 62%" });
        chips.AddChild(new GalleryDraw("cell", "strip.cell.wanted", .4, tokens) { TooltipText = "Relationship-strip cell" });
        chips.AddChild(new ProgressBar { Value = 62, CustomMinimumSize = new Vector2(t.Px("control.height") * 4, 0), SizeFlagsVertical = SizeFlags.ShrinkCenter, TooltipText = "Progress bar: 62%" });
        body.AddChild(new HSeparator());
        Heading("table, tree and the script frame");
        var lower = new HBoxContainer(); lower.AddThemeConstantOverride("separation", t.Px("space.3")); body.AddChild(lower);
        var table = new Tree { Columns = 3, ColumnTitlesVisible = true, HideRoot = true, CustomMinimumSize = new Vector2(t.Px("dock.width"), t.Px("row.height") * 6), Name = "SampleTable" };
        string[] titles = { "Orbit", "Cells", "Solved" }; for (int i = 0; i < 3; i++) table.SetColumnTitle(i, titles[i]);
        TreeItem root = table.CreateItem();
        for (int i = 1; i <= 4; i++) { TreeItem item = table.CreateItem(root); item.SetText(0, "sample orbit " + i); item.SetText(1, (120 * i).ToString(CultureInfo.InvariantCulture)); item.SetText(2, (i * 17 % 100) + "%"); }
        lower.AddChild(table);
        var tree = new Tree { CustomMinimumSize = new Vector2(t.Px("dock.width"), t.Px("row.height") * 6), Name = "SampleTree" };
        TreeItem top = tree.CreateItem(); top.SetText(0, "sweep.6cs");
        foreach (string entry in new[] { "reach", "ring" }) { TreeItem item = tree.CreateItem(top); item.SetText(0, entry); tree.CreateItem(item).SetText(0, "result") ; }
        lower.AddChild(tree);
        var script = new PanelContainer { ThemeTypeVariation = "ScriptFrame", CustomMinimumSize = new Vector2(t.Px("dock.width"), 0), TooltipText = "A script's own surface always sits in this frame" };
        script.AddChild(ChromeWidgets.Text("script panel", "DimLabel")); lower.AddChild(script);
    }

    private void TypePage(TokenSet t)
    {
        foreach (var role in t.System.Type.Values)
        {
            Font face = ChromeTheme.Face(t, role.Id);
            Heading($"{role.Id} · {TokenSpecimen.FaceInUse(t, role.Id)}{(role.Tabular ? " · tabular figures" : "")}");
            foreach (var step in role.Ramp.OrderBy(s => s.Value))
            {
                Label line = ChromeWidgets.Text($"{step.Key} {t.TypeSize(role.Id, step.Key)}   Ring 3 turns by 72°   0123456789", "");
                line.AddThemeFontOverride("font", face); line.AddThemeFontSizeOverride("font_size", (int)t.TypeSize(role.Id, step.Key)); body.AddChild(line);
            }
            // The notation the UI uses, set in this role's face chain, and the glyphs the chain does not have.
            string[] missing = Notation.Split(' ').Where(s => s.EnumerateRunes().Any(r => !face.HasChar(r.Value))).ToArray();
            Label notation = ChromeWidgets.Text(Notation, ""); notation.AddThemeFontOverride("font", face); notation.AddThemeFontSizeOverride("font_size", (int)t.TypeSize(role.Id, role.Ramp.Keys.Contains("body") ? "body" : role.Ramp.Keys.First()));
            notation.Name = "Notation" + role.Id; body.AddChild(notation);
            Label report = ChromeWidgets.Text(missing.Length == 0 ? "every notation glyph is in the face chain" : "missing: " + string.Join(" ", missing), missing.Length == 0 ? "DimLabel" : "AccentLabel");
            report.Name = "Missing" + role.Id; body.AddChild(report);
            body.AddChild(new HSeparator());
        }
    }

    private void IconPage(TokenSet t)
    {
        int px = t.Px("icon.small");
        bool Second(string id) => id.StartsWith("glyph.", StringComparison.Ordinal) || id.StartsWith("layer.", StringComparison.Ordinal) || id.StartsWith("mark.", StringComparison.Ordinal);
        foreach (bool second in new[] { false, true })
        {
            string[] ids = icons.Icons.Keys.Where(id => Second(id) == second).OrderBy(id => id, StringComparer.Ordinal).ToArray();
            Heading(second ? $"tier 2 · content glyphs · {ids.Length} at {px} px, each with its label" : $"tier 1 · {ids.Length} icons at {px} px");
            var flow = new HFlowContainer(); flow.AddThemeConstantOverride("h_separation", t.Px("space.3")); flow.AddThemeConstantOverride("v_separation", t.Px("space.2")); body.AddChild(flow);
            foreach (string id in ids)
            {
                var cell = new HBoxContainer { TooltipText = id, MouseFilter = MouseFilterEnum.Stop };
                Label icon = new() { MouseFilter = MouseFilterEnum.Ignore }; ChromeIcons.Apply(icon, id, px); cell.AddChild(icon);
                if (second) cell.AddChild(ChromeWidgets.Text(id.Substring(id.IndexOf('.') + 1), "DimLabel"));
                flow.AddChild(cell);
            }
            body.AddChild(new HSeparator());
        }
        Label cover = ChromeWidgets.Text($"{commands.Count} commands: {commands.Count(c => commandIcons.Icons.ContainsKey(c))} with an icon, {commands.Count(c => commandIcons.LabelOnly.Contains(c))} label only on purpose, {commandIcons.Missing(commands).Count} unassigned", "DimLabel");
        cover.Name = "IconCover"; body.AddChild(cover);
    }

    private void MotionPage(TokenSet t)
    {
        Motion.Reduced = t.ReducedMotion;
        var switches = new HBoxContainer(); body.AddChild(switches);
        void Switch(string name, string words, string tip, bool on, Action<bool> set)
        {
            var check = new CheckButton { Name = name, Text = words, TooltipText = tip, ButtonPressed = on };
            check.Toggled += value => { set(value); Safe.Raise(MotionChanged); }; switches.AddChild(check);
        }
        Switch("ReducedSwitch", "Reduced motion", "Every motion shows its static equivalent at once", Motion.Reduced, v => Motion.Reduced = v);
        Switch("MuteSwitch", "Cues muted", "Cue slots hold no audio; unmuted, a cue names itself here", Motion.Muted, v => Motion.Muted = v);
        Switch("RewardSwitch", "Milestone marks", "A small mark changes in place at a milestone; never a dialog", Motion.Rewards, v => Motion.Rewards = v);
        Stage = new MotionStage(tokens); body.AddChild(Stage);
        foreach (MotionEntry entry in motions.Entries)
        {
            var row = new PanelContainer { ThemeTypeVariation = "Row", Name = "Motion" + entry.Id.Replace('.', '_') }; body.AddChild(row);
            var line = new HBoxContainer(); line.AddThemeConstantOverride("separation", t.Px("space.3")); row.AddChild(line);
            MotionEntry played = entry;
            Button play = SessionWidgets.Tool("motion.play", "Play " + entry.Id, t, () => Play(played)); play.Name = "Play"; line.AddChild(play);
            Label id = ChromeWidgets.Text(entry.Id, "CodeLabel"); id.CustomMinimumSize = new Vector2(t.Px("control.height") * 5, 0); line.AddChild(id);
            var words = new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill }; line.AddChild(words);
            words.AddChild(ChromeWidgets.Text(entry.Meaning, "", true));
            words.AddChild(ChromeWidgets.Text($"{entry.Duration} {t.Duration(entry.Duration):0} ms · {entry.Curve} · still: {entry.Static}", "DimLabel", true));
            if (entry.Cue != null) line.AddChild(SessionWidgets.Chip(entry.Cue, t.C("text.dim"), "Cue slot, no audio. Its visual equivalent: " + entry.Static));
        }
    }

    public void Play(MotionEntry entry)
    {
        TokenSet t = tokens();
        if (entry.Id == "reward.mark" && !Motion.Rewards) { Say("reward.mark is switched off"); return; }
        Stage?.Play(entry, motions.Seconds(entry.Id, t.System, Motion.Reduced), t.System.Curves[entry.Curve]);
        Say(entry.Id + (Motion.Reduced ? " · still: " + entry.Static : " · " + entry.Meaning) + (entry.Cue != null && !Motion.Muted ? " · " + entry.Cue + " would sound here" : ""));
    }

    private void PalettePage(TokenSet t)
    {
        var visions = new HBoxContainer(); body.AddChild(visions);
        foreach (string vision in ColourMetric.Visions)
        {
            Button tab = ChromeWidgets.Tool(vision, "Show every palette as " + vision + " colour vision sees it", true, "TabButton"); tab.Name = "Vision" + vision; tab.SetPressedNoSignal(vision == Vision);
            string chosen = vision; tab.Pressed += () => { Vision = chosen; Rebuild(); }; visions.AddChild(tab);
        }
        Label rule = ChromeWidgets.Text($"smallest OKLab distance between two classes: at least {StickerPalette.Needed.ToString(CultureInfo.InvariantCulture)}, and {StickerPalette.NeededSimulated.ToString(CultureInfo.InvariantCulture)} under each simulation", "DimLabel");
        body.AddChild(rule);
        foreach (StickerPalette palette in palettes.OrderBy(p => p.Family != t.Theme.Id).ThenBy(p => p.Id, StringComparer.Ordinal))
        {
            bool own = palette.Family == t.Theme.Id;
            var row = new PanelContainer { ThemeTypeVariation = Palette == palette.Id ? "RowSelected" : "Row", Name = "Palette" + palette.Id.Replace('.', '_') }; body.AddChild(row);
            var line = new HBoxContainer(); line.AddThemeConstantOverride("separation", t.Px("space.3")); row.AddChild(line);
            Label name = ChromeWidgets.Text(palette.Name, own ? "" : "DimLabel"); name.CustomMinimumSize = new Vector2(t.Px("control.height") * 4, 0); line.AddChild(name);
            var swatches = new HBoxContainer(); swatches.AddThemeConstantOverride("separation", 0); line.AddChild(swatches);
            foreach (Rgb colour in palette.Colours)
                swatches.AddChild(new ColorRect { Color = ChromeBody.Engine(ColourMetric.Simulate(colour, Vision)), CustomMinimumSize = new Vector2(t.Px("control.height"), t.Px("control.height")) });
            string numbers = string.Join("  ", ColourMetric.Visions.Select(v => $"{v} {palette.Smallest(v).ToString("0.000", CultureInfo.InvariantCulture)}"));
            Label distances = ChromeWidgets.Text(numbers, "ReadoutLabel"); distances.SizeFlagsHorizontal = SizeFlags.ExpandFill; line.AddChild(distances);
            if (!palette.Passes)
            {
                string worst = ColourMetric.Visions.First(v => palette.Smallest(v) < (v == "normal" ? StickerPalette.Needed : StickerPalette.NeededSimulated));
                line.AddChild(SessionWidgets.Chip("not offered", t.C("severity.warning"), "Two classes are too close under " + worst));
            }
            else if (own)
            {
                StickerPalette chosen = palette;
                Button use = SessionWidgets.Tool("layer.stickers", "Show " + palette.Name + " on the stickers of both views", t, () => { Palette = chosen.Id; Safe.Raise(PaletteChosen, chosen); Rebuild(); });
                use.Name = "Use"; line.AddChild(use);
            }
            else line.AddChild(SessionWidgets.Chip("other family", t.C("text.dim"), "Offered when its family is in force"));
        }
    }

    private void ListPage(TokenSet t)
    {
        Label count = ChromeWidgets.Text("", "DimLabel"); count.Name = "ListCount"; body.AddChild(count);
        var jump = new HBoxContainer(); body.AddChild(jump);
        List = new VirtualList(ListRows, i => $"{i + 1,6}   move {(i * 7919 % 1200) + 1,4}   sample orbit {i % 7 + 1}", tokens) { SizeFlagsVertical = SizeFlags.ExpandFill, CustomMinimumSize = new Vector2(0, t.Px("row.height") * 12) };
        VirtualList list = List;
        foreach (int row in new[] { 0, 2500, 5000, 9999 })
        {
            int to = row; Button go = ChromeWidgets.Tool((row + 1).ToString("N0", CultureInfo.InvariantCulture), "Scroll to row " + (row + 1).ToString("N0", CultureInfo.InvariantCulture)); go.Name = "Row" + row;
            go.Pressed += () => list.ScrollTo(to); jump.AddChild(go);
        }
        list.Laid += () => count.Text = $"{ListRows.ToString("N0", CultureInfo.InvariantCulture)} rows · {list.Built} built";
        body.AddChild(list);
    }

    private void PanePage(TokenSet t)
    {
        Heading("the five designed states of a pane");
        var row = new HBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill }; row.AddThemeConstantOverride("separation", t.Px("space.3")); body.AddChild(row);
        foreach (var state in ChromeBody.PaneStates)
        {
            var cell = new PanelContainer { ThemeTypeVariation = "GalleryCell", SizeFlagsHorizontal = SizeFlags.ExpandFill, CustomMinimumSize = new Vector2(0, t.Px("dock.height")), TooltipText = state.Id };
            cell.AddChild(ChromeBody.PaneState(state.Id)); row.AddChild(cell);
        }
    }
}
