using System;
using System.Globalization;
using Godot;

namespace Magic600.Chrome;

// The cap prompt (jumble.cap.name without a number; owner's bug report, 26 November 2026): the palette's overlay with a
// field, the rows in cap-id order with nothing highlighted, and a help line. Enter or a click names a cap, the wheel
// moves the rows, Escape closes. It reuses ChromeOverlay and the palette's row styles (Row, RowSelected).
public sealed partial class CapPromptOverlay : ChromeOverlay
{
    public CapPrompt Model { get; }
    public LineEdit Query { get; } = new() { PlaceholderText = "Type a cap number, or choose a row", ClearButtonEnabled = true, Name = "CapPromptQuery" };
    private readonly Label head = new() { ThemeTypeVariation = "DimLabel", Name = "CapPromptHead" };
    private readonly Label help = new() { ThemeTypeVariation = "MicroLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart, Name = "CapPromptHelp" };
    private readonly VBoxContainer list = new() { Name = "CapPromptRows" };
    private readonly Func<int?> named;
    // The solver chose a cap: Enter on it, or a click.
    public event Action<int>? Picked;

    public CapPromptOverlay(CapPrompt model, Func<TokenSet> tokens, Func<int?> named) : base(tokens, .36f, .08f)
    {
        Model = model; this.named = named; Name = "CapPrompt";
        var column = new VBoxContainer(); Panel.AddChild(column);
        column.AddChild(head); column.AddChild(Query); column.AddChild(list); column.AddChild(new HSeparator()); column.AddChild(help);
        list.AddThemeConstantOverride("separation", 0);
        Query.TextChanged += text => { Model.Type(text); Refresh(); };
        Query.GuiInput += Typed;
    }

    public override void Open() { base.Open(); Query.Text = ""; Model.Open(); Refresh(); Query.GrabFocus(); }

    // The keys while the field does not hold the focus (after a click elsewhere in the panel).
    public override bool Key(string key)
    {
        switch (key)
        {
            case "Down": Move(1); return true;
            case "Up": Move(-1); return true;
            case "Enter" or "Kp Enter": Accept(); return true;
            case "Escape": Close(); return true;
            default: return false;
        }
    }

    private void Typed(InputEvent e)
    {
        if (e is not InputEventKey { Pressed: true } key) return;
        if (key.Keycode == Godot.Key.Down) Move(1);
        else if (key.Keycode == Godot.Key.Up) Move(-1);
        else if (key.Keycode == Godot.Key.Pagedown) Move(CapPrompt.Shown);
        else if (key.Keycode == Godot.Key.Pageup) Move(-CapPrompt.Shown);
        else if (key.Keycode is Godot.Key.Enter or Godot.Key.KpEnter) Accept();
        else if (key.Keycode == Godot.Key.Escape) Close();
        else return;
        Query.AcceptEvent();
    }

    public void Move(int by) { Model.Move(by); Refresh(); }

    // Names the highlighted or typed cap; a refused one stays on screen with its reason.
    public bool Accept()
    {
        if (Model.Accept() is not int cap) { Refresh(); return false; }
        Close(); Safe.Raise(Picked, cap);
        return true;
    }

    private void Refresh()
    {
        TokenSet t = Tokens();
        head.Text = "Name a cap" + (named() is int cap ? " · cap " + cap.ToString(CultureInfo.InvariantCulture) + " is named now" : "");
        foreach (Node child in list.GetChildren()) { list.RemoveChild(child); child.QueueFree(); }
        for (int i = Model.First; i < Math.Min(Model.Rows.Count, Model.First + CapPrompt.Shown); i++)
        {
            CapPromptRow row = Model.Rows[i]; int index = i;
            var holder = new PanelContainer
            {
                ThemeTypeVariation = Model.Highlighted == i ? "RowSelected" : "Row", CustomMinimumSize = new Vector2(0, t.Px("row.height")),
                Name = "CapPromptRow_" + row.Cap.ToString(CultureInfo.InvariantCulture), MouseDefaultCursorShape = CursorShape.PointingHand,
                TooltipText = row.Refusal ?? CapNaming.Hint,
            };
            var line = new HBoxContainer(); holder.AddChild(line);
            line.AddChild(ChromeWidgets.Text(row.Label, row.Refusal == null ? "" : "DimLabel", true));
            if (row.Refusal != null) { Label tag = ChromeWidgets.Text("unavailable", "MicroLabel"); tag.AddThemeColorOverride("font_color", t.C("severity.warning")); line.AddChild(tag); }
            holder.GuiInput += e =>
            {
                if (e is not InputEventMouseButton { Pressed: true } click) return;
                if (click.ButtonIndex is MouseButton.WheelUp or MouseButton.WheelDown) { Model.Scroll(click.ButtonIndex == MouseButton.WheelUp ? -3 : 3); Refresh(); return; }
                if (click.ButtonIndex != MouseButton.Left) return;
                Model.Highlight(index); Refresh(); Accept();
            };
            list.AddChild(holder);
        }
        string? highlighted = Model.Highlighted is int h ? Model.Rows[h].Refusal : null;
        help.Text = Model.Said ?? (highlighted != null ? "Unavailable: " + highlighted
            : $"{Model.Rows.Count} caps in cap-id order   type to narrow   \u2191\u2193 choose   Enter or a click names   Esc close");
        help.AddThemeColorOverride("font_color", t.C(Model.Said != null || highlighted != null ? "severity.warning" : "text.dim"));
    }
}
