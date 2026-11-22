using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// An overlay inside the window: a dimmed ground that closes it and one panel near the top. Floating tool
// windows are not part of the frame (ADR-D01.37), so the palette and the shortcut sheet are drawn in the window.
public abstract partial class ChromeOverlay : Control
{
    protected PanelContainer Panel { get; } = new() { ThemeTypeVariation = "Popup" };
    protected Func<TokenSet> Tokens { get; }
    public bool IsOpen => Visible;
    public event Action? Closed;

    protected ChromeOverlay(Func<TokenSet> tokens, float widthShare, float top)
    {
        Tokens = tokens; Visible = false; SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect); MouseFilter = MouseFilterEnum.Stop;
        var ground = new ColorRect { Color = new Color(0, 0, 0, .35f), MouseFilter = MouseFilterEnum.Stop }; AddChild(ground);
        ground.SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        ground.GuiInput += e => { if (e is InputEventMouseButton { Pressed: true }) Close(); };
        AddChild(Panel);
        Panel.AnchorLeft = (1 - widthShare) / 2; Panel.AnchorRight = 1 - (1 - widthShare) / 2; Panel.AnchorTop = top; Panel.AnchorBottom = top;
        Panel.GrowVertical = GrowDirection.End;
    }

    public virtual void Open() { Visible = true; MoveToFront(); }
    public void Close() { if (!Visible) return; Visible = false; Closed?.Invoke(); }
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
    private readonly VBoxContainer list = new() { Name = "PaletteRows" };
    private readonly Label count = new() { ThemeTypeVariation = "MicroLabel" };

    public CommandPalette(PaletteModel model, Func<TokenSet> tokens) : base(tokens, .5f, .08f)
    {
        Model = model; Name = "CommandPalette";
        var column = new VBoxContainer(); Panel.AddChild(column);
        column.AddChild(Query); column.AddChild(list); column.AddChild(new HSeparator()); column.AddChild(Help); column.AddChild(count);
        list.AddThemeConstantOverride("separation", 0);
        Query.TextChanged += _ => { Selected = 0; Refresh(); };
        Query.GuiInput += Key;
    }

    public override void Open() { base.Open(); Query.Text = ""; Selected = 0; Refresh(); Query.GrabFocus(); }

    public void Type(string text) { Query.Text = text; Selected = 0; Refresh(); }

    private void Key(InputEvent e)
    {
        if (e is not InputEventKey { Pressed: true } key) return;
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
        if (Selected >= Rows.Count) return false;
        PaletteRow row = Rows[Selected];
        if (row.Unavailable != null) { Describe(); return false; }
        Model.Used(row); Close(); Run?.Invoke(row);
        return true;
    }

    public void TogglePin(int index) { if (index < Rows.Count) { Model.Pin(Rows[index], !Rows[index].Pinned); Refresh(); } }

    private IReadOnlyList<PaletteRow> Find()
    {
        string text = Query.Text;
        if (!text.StartsWith('>')) return Model.Rows(text);
        // Typed mode: "> command.id argument" names one row exactly.
        string[] parts = text.Substring(1).Trim().Split(' ', 2, StringSplitOptions.RemoveEmptyEntries);
        if (parts.Length == 0) return Array.Empty<PaletteRow>();
        string? argument = parts.Length > 1 ? parts[1].Trim() : null;
        PaletteRow[] exact = Model.Rows("").Where(r => r.Command == parts[0] && r.Argument == argument).ToArray();
        return exact.Length > 0 ? exact : Model.Rows("").Where(r => r.Command.StartsWith(parts[0], StringComparison.Ordinal) && r.Argument == null).ToArray();
    }

    private void Describe()
    {
        if (Rows.Count == 0)
        {
            Help.Text = Query.Text.StartsWith('>') ? "Typed mode: a command id, then its argument. A run ends in a read result or a preview, never a commit." : "No command matches. The palette always holds every command.";
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
            if (row.Unavailable != null) { Label tag = ChromeWidgets.Text("unavailable", "MicroLabel"); tag.AddThemeColorOverride("font_color", tokens.C("severity.warning")); line.AddChild(tag); }
            line.AddChild(ChromeWidgets.Text(row.Location, "MicroLabel"));
            if (row.Keys.Length > 0)
            {
                var chip = new PanelContainer { ThemeTypeVariation = "Chip" }; chip.AddChild(ChromeWidgets.Text(row.Keys, "CodeLabel")); line.AddChild(chip);
            }
            holder.GuiInput += e =>
            {
                if (e is not InputEventMouseButton { Pressed: true } click) return;
                if (click.ButtonIndex == MouseButton.Right) { TogglePin(index); return; }
                Selected = index; Rows = Rows.ToArray(); Refresh(false); Accept();
            };
            list.AddChild(holder);
        }
        count.Text = $"{Rows.Count} commands   ↑↓ choose   Enter run   right-click pin   Esc close";
        Describe();
    }
}
