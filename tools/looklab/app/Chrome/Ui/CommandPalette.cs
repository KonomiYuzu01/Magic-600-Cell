using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// An overlay inside the window: a dimmed ground that closes it and one panel near the top. Floating tool
// windows are not part of the frame (ADR-D01.37), so the palette and the shortcut sheet are drawn in the window.
public abstract partial class ChromeOverlay : Control
{
    protected PanelContainer Panel { get; } = new() { ThemeTypeVariation = "OverlayBox" };
    protected Func<TokenSet> Tokens { get; }
    public bool IsOpen => Visible;
    public event Action? Closed;
    private Tween? fade;

    protected ChromeOverlay(Func<TokenSet> tokens, float widthShare, float top)
    {
        Tokens = tokens; Visible = false; SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect); MouseFilter = MouseFilterEnum.Stop;
        var ground = new ColorRect { Color = tokens().C("text", .3f), MouseFilter = MouseFilterEnum.Stop }; AddChild(ground);
        ground.SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        ground.GuiInput += e => { if (e is InputEventMouseButton { Pressed: true }) Close(); };
        AddChild(Panel);
        Panel.AnchorLeft = (1 - widthShare) / 2; Panel.AnchorRight = 1 - (1 - widthShare) / 2; Panel.AnchorTop = top; Panel.AnchorBottom = top;
        Panel.GrowVertical = GrowDirection.End;
    }

    // One overlay at a time: opening one closes the others.
    public virtual void Open()
    {
        if (GetParent() is { } parent) foreach (ChromeOverlay other in parent.GetChildren().OfType<ChromeOverlay>()) if (other != this) other.Close();
        Visible = true; MoveToFront();
        fade?.Kill(); Modulate = Modulate with { A = 1 };
        if (!Tokens().ReducedMotion && IsInsideTree()) { Modulate = Modulate with { A = 0 }; fade = CreateTween(); fade.TweenProperty(this, "modulate:a", 1f, Tokens().Duration("fast") / 1000); }
    }
    // A key pressed while the overlay is open and no text field holds the focus; true when the overlay used it.
    public virtual bool Key(string key) => false;
    public void Close() { if (!Visible) return; Visible = false; Safe.Raise(Closed); }
}

// The command palette (ADR-D01.18): every command of the registry, filtered by what is typed and ordered only
// by the solver's own pins, recency and frequency. Each row shows label, location, binding and an unavailable
// tag; the highlighted row shows its help and the full reason it is unavailable. A line that starts with ">"
// is the typed mode: a command id and its argument (ADR-D01.21).
public sealed partial class CommandPalette : ChromeOverlay
{
    public const int Shown = 12;
    public PaletteModel Model { get; }
    public LineEdit Query { get; } = new() { PlaceholderText = "Search commands, or > to type one", ClearButtonEnabled = true, Name = "PaletteQuery" };
    public Label Help { get; } = new() { ThemeTypeVariation = "DimLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart, Name = "PaletteHelp" };
    public IReadOnlyList<PaletteRow> Rows { get; private set; } = Array.Empty<PaletteRow>();
    public int Selected { get; private set; }
    public event Action<PaletteRow>? Run;
    // Typed mode also takes a whole 600 Cell Script line (ADR-D01.59) and keeps its own history.
    public event Action<string>? TypedLine;
    // The row menu (ADR-D01.18): a right click, the Menu key or Shift+F10 opens it on a row; it pins, aliases or binds.
    public int MenuRow { get; private set; } = -1;
    public PaletteRow? AliasFor { get; private set; }
    public event Action<PaletteRow>? Bind;
    public void OpenMenu(int index) { MenuRow = index >= 0 && index < Rows.Count ? index : -1; if (MenuRow >= 0) Selected = MenuRow; Refresh(false); }
    public void MenuPin() { if (MenuRow < 0) return; int at = MenuRow; MenuRow = -1; TogglePin(at); }
    public void MenuAlias()
    {
        if (MenuRow < 0 || MenuRow >= Rows.Count) return;
        AliasFor = Rows[MenuRow]; MenuRow = -1; Query.Text = "=" + (AliasFor.Alias ?? ""); Query.CaretColumn = Query.Text.Length; Selected = 0; Refresh(); Query.GrabFocus();
    }
    public void MenuBind() { if (MenuRow < 0 || MenuRow >= Rows.Count) return; PaletteRow row = Rows[MenuRow]; MenuRow = -1; Close(); Safe.Raise(Bind, row); }
    private void Leave() { MenuRow = -1; AliasFor = null; Query.Text = ""; Selected = 0; Refresh(); }
    public LineHistory History { get; } = new();
    private readonly VBoxContainer list = new() { Name = "PaletteRows" };
    private readonly Label count = new() { ThemeTypeVariation = "MicroLabel" };

    public CommandPalette(PaletteModel model, Func<TokenSet> tokens) : base(tokens, .5f, .08f)
    {
        Model = model; Name = "CommandPalette";
        var column = new VBoxContainer(); Panel.AddChild(column);
        column.AddChild(Query); column.AddChild(list); column.AddChild(new HSeparator()); column.AddChild(Help); column.AddChild(count);
        list.AddThemeConstantOverride("separation", 0);
        Query.TextChanged += _ => { Selected = 0; Refresh(); };
        Query.GuiInput += Typed;
    }

    public override void Open() { base.Open(); MenuRow = -1; AliasFor = null; Query.Text = ""; Selected = 0; Refresh(); Query.GrabFocus(); }

    public void Type(string text) { Query.Text = text; Selected = 0; Refresh(); }

    private void Typed(InputEvent e)
    {
        if (e is not InputEventKey { Pressed: true } key) return;
        if (key.Keycode == Godot.Key.Menu || (key.Keycode == Godot.Key.F10 && key.ShiftPressed)) { if (AliasFor == null) OpenMenu(MenuRow == Selected ? -1 : Selected); Query.AcceptEvent(); return; }
        if (key.Keycode == Godot.Key.Escape && (MenuRow >= 0 || AliasFor != null)) { Leave(); Query.AcceptEvent(); return; }
        if (MenuRow >= 0 && key.Keycode is Godot.Key.P or Godot.Key.A or Godot.Key.B)
        {
            if (key.Keycode == Godot.Key.P) MenuPin(); else if (key.Keycode == Godot.Key.A) MenuAlias(); else MenuBind();
            Query.AcceptEvent(); return;
        }
        if (Query.Text.StartsWith('>') && Rows.Count == 0 && key.Keycode is Godot.Key.Up or Godot.Key.Down)
        {
            Type("> " + History.Step(key.Keycode == Godot.Key.Up ? -1 : 1)); Query.CaretColumn = Query.Text.Length; Query.AcceptEvent();
            return;
        }
        if (key.Keycode == Godot.Key.Down) Move(1);
        else if (key.Keycode == Godot.Key.Up) Move(-1);
        else if (key.Keycode is Godot.Key.Enter or Godot.Key.KpEnter) Accept();
        else if (key.Keycode == Godot.Key.Escape) Close();
        else return;
        Query.AcceptEvent();
    }

    public void Move(int by) { if (Rows.Count > 0) { Selected = Math.Clamp(Selected + by, 0, Rows.Count - 1); Refresh(false); } }

    // Runs the highlighted row. An unavailable row stays on screen with its reason; nothing runs.
    public bool Accept()
    {
        if (AliasFor != null) { Model.Alias(AliasFor, Query.Text.TrimStart('=')); Leave(); return true; }
        if (Query.Text.StartsWith('>') && Query.Text.Substring(1).Trim() is { Length: > 0 } line && (Rows.Count == 0 || line.Contains('(')))
        {
            History.Add(line); Close(); Safe.Raise(TypedLine, line);
            return true;
        }
        if (Selected >= Rows.Count) return false;
        PaletteRow row = Rows[Selected];
        if (row.Unavailable != null) { Describe(); return false; }
        Model.Used(row); Close(); Safe.Raise(Run, row);
        return true;
    }

    public void TogglePin(int index) { if (index < Rows.Count) { Model.Pin(Rows[index], !Rows[index].Pinned); Refresh(); } }

    private IReadOnlyList<PaletteRow> Find()
    {
        string text = Query.Text;
        if (AliasFor != null) return new[] { AliasFor };
        if (!text.StartsWith('>')) return Model.Rows(text);
        // Typed mode: "> command.id argument" names one row exactly.
        string[] parts = text.Substring(1).Trim().Split(' ', 2, StringSplitOptions.RemoveEmptyEntries);
        if (parts.Length == 0) return Array.Empty<PaletteRow>();
        string? argument = parts.Length > 1 ? parts[1].Trim() : null;
        PaletteRow[] exact = Model.Rows("").Where(r => r.Command == parts[0] && r.Argument == argument).ToArray();
        // A command with an argument of the solver's own is a row too: "> orbit.switch O05".
        if (exact.Length == 0 && argument != null && Model.Typed(parts[0], argument) is { } own) return new[] { own };
        return exact.Length > 0 ? exact : Model.Rows("").Where(r => r.Command.StartsWith(parts[0], StringComparison.Ordinal) && r.Argument == null).ToArray();
    }

    private void Describe()
    {
        if (AliasFor != null) { Help.Text = $"Alias for \"{AliasFor.Label}\": type it after = and press Enter. An empty alias takes it away. Esc leaves."; return; }
        if (Rows.Count == 0)
        {
            Help.Text = Query.Text.StartsWith('>') ? "Type a command or a script line. Up recalls. A run never commits." : "No command matches.";
            return;
        }
        PaletteRow row = Rows[Math.Min(Selected, Rows.Count - 1)];
        string help = string.IsNullOrWhiteSpace(row.Help) ? row.Command : row.Help;
        Help.Text = row.Unavailable == null ? help : $"Unavailable: {row.Unavailable}\n{help}";
    }

    private void Refresh(bool search = true)
    {
        if (search) Rows = Find();
        Selected = Rows.Count == 0 ? 0 : Math.Clamp(Selected, 0, Rows.Count - 1);
        foreach (Node child in list.GetChildren()) { list.RemoveChild(child); child.QueueFree(); }
        TokenSet tokens = Tokens();
        int first = Math.Clamp(Selected - Shown / 2, 0, Math.Max(0, Rows.Count - Shown));
        for (int i = first; i < Math.Min(Rows.Count, first + Shown); i++)
        {
            PaletteRow row = Rows[i]; int index = i;
            var holder = new PanelContainer { ThemeTypeVariation = i == Selected ? "RowSelected" : "Row", CustomMinimumSize = new Vector2(0, tokens.Px("row.height")) };
            holder.SetMeta("row", row.Id);
            var line = new HBoxContainer(); holder.AddChild(line);
            Label pin = ChromeWidgets.Text("", "MicroLabel"); pin.CustomMinimumSize = new Vector2(tokens.Px("icon.medium"), 0);
            if (row.Pinned) { ChromeIcons.Apply(pin, "palette.pin", tokens.Px("icon.small")); pin.AddThemeColorOverride("font_color", tokens.C("accent.strong")); }
            line.AddChild(pin);
            Label label = ChromeWidgets.Text(row.Label, row.Unavailable == null ? "" : "DimLabel", true); line.AddChild(label);
            if (row.Alias != null) { Label alias = ChromeWidgets.Text("= " + row.Alias, "AccentLabel"); alias.TooltipText = "Your alias for this row"; line.AddChild(alias); }
            if (row.Unavailable != null) { Label tag = ChromeWidgets.Text("unavailable", "MicroLabel"); tag.AddThemeColorOverride("font_color", tokens.C("severity.warning")); line.AddChild(tag); }
            line.AddChild(ChromeWidgets.Text(row.Location, "MicroLabel"));
            if (row.Keys.Length > 0)
            {
                var chip = new PanelContainer { ThemeTypeVariation = "Chip" }; chip.AddChild(ChromeWidgets.Text(row.Keys, "CodeLabel")); line.AddChild(chip);
            }
            holder.GuiInput += e =>
            {
                if (e is not InputEventMouseButton { Pressed: true } click) return;
                if (click.ButtonIndex == MouseButton.Right) { OpenMenu(MenuRow == index ? -1 : index); return; }
                Selected = index; Rows = Rows.ToArray(); Refresh(false); Accept();
            };
            list.AddChild(holder);
            if (i != MenuRow || AliasFor != null) continue;
            var menu = new HBoxContainer { Name = "PaletteRowMenu", Alignment = BoxContainer.AlignmentMode.End };
            void Item(string name, string text, string tip, Action act) { Button b = ChromeWidgets.Tool(text, tip); b.Name = name; b.FocusMode = FocusModeEnum.None; b.Pressed += act; menu.AddChild(b); }
            Item("RowPin", row.Pinned ? "Unpin (P)" : "Pin (P)", "A pinned row stays at the top", MenuPin);
            Item("RowAlias", "Alias (A)", "Give the row a second name of your own", MenuAlias);
            Item("RowBind", "Bind a key (B)", "The next key you press runs this command", MenuBind);
            list.AddChild(menu);
        }
        count.Text = $"{Rows.Count} commands   \u2191\u2193 choose   Enter run   right-click or Shift+F10: pin, alias, bind   Esc close";
        Describe();
    }
}
