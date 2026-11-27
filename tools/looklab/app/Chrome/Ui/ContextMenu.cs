using System;
using System.Collections.Generic;
using System.Globalization;
using Godot;

namespace Magic600.Chrome;

public sealed partial class ContextMenuOverlay : ChromeOverlay
{
    private readonly CommandRegistry registry;
    private readonly Func<string, string?, string?> unavailable;
    private readonly ScrollContainer scroll = new() { HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };
    private readonly VBoxContainer list = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill };
    private readonly List<PanelContainer> rows = new();
    private IReadOnlyList<RadialEntry> entries = Array.Empty<RadialEntry>();
    private Vector2 pointer;
    private int selected;

    public event Action<RadialEntry>? Chosen;

    public ContextMenuOverlay(CommandRegistry registry, Func<string, string?, string?> unavailable, Func<TokenSet> tokens)
        : base(tokens, 0, 0)
    {
        this.registry = registry;
        this.unavailable = unavailable;
        Name = "ContextMenu";
        FocusMode = FocusModeEnum.Click;
        Panel.SetAnchorsAndOffsetsPreset(LayoutPreset.TopLeft);
        Panel.AddChild(scroll);
        scroll.AddChild(list);
        Resized += Place;
        list.MinimumSizeChanged += Place;
    }

    public void Open(ContextMenu menu, Keymap keys, Vector2 at)
    {
        pointer = at;
        entries = menu.Menu.Entries;
        foreach (Node child in list.GetChildren()) { list.RemoveChild(child); child.QueueFree(); }
        rows.Clear();
        list.AddChild(ChromeWidgets.Text(menu.Menu.Label, "HeadingLabel"));
        TokenSet tokens = Tokens();
        for (int i = 0; i < entries.Count; i++)
        {
            if (i == menu.SeparatorBefore) list.AddChild(new HSeparator());
            RadialEntry entry = entries[i];
            ChromeCommand command = registry.Commands[entry.Command];
            string? why = unavailable(entry.Command, entry.Argument);
            var row = new PanelContainer
            {
                Name = "ContextMenuEntry" + i.ToString(CultureInfo.InvariantCulture), ThemeTypeVariation = "Row",
                CustomMinimumSize = new Vector2(0, tokens.Px("row.height")), MouseFilter = MouseFilterEnum.Stop,
                TooltipText = why == null ? command.Help : "Unavailable: " + why,
            };
            var content = new HBoxContainer { MouseFilter = MouseFilterEnum.Ignore };
            row.AddChild(content);
            Label icon = ChromeWidgets.Text("");
            ChromeIcons.Apply(icon, ChromeBody.Commands?.Of(entry.Command, entry.Argument) ?? "command.bar", tokens.Px("icon.small"));
            content.AddChild(icon);
            content.AddChild(ChromeWidgets.Text(entry.Label ?? command.Label, "", true));
            string binding = keys.KeysFor(entry.Command, entry.Argument);
            if (binding.Length > 0) content.AddChild(ChromeWidgets.Text(binding, "MicroLabel"));
            if (i < 9) content.AddChild(ChromeWidgets.Text((i + 1).ToString(CultureInfo.InvariantCulture), "MicroLabel"));
            foreach (Label label in content.GetChildren())
            {
                label.MouseFilter = MouseFilterEnum.Ignore;
                if (why != null) label.AddThemeColorOverride("font_color", tokens.C("text.off"));
            }
            int index = i;
            row.GuiInput += input =>
            {
                if (input is not InputEventMouseButton { ButtonIndex: MouseButton.Left, Pressed: false }) return;
                row.AcceptEvent();
                Choose(index);
            };
            rows.Add(row);
            list.AddChild(row);
        }
        selected = -1;
        base.Open();
        Place();
        scroll.ScrollVertical = 0;
        GrabFocus();
    }

    private void Place()
    {
        if (!IsOpen) return;
        Vector2 area = GetViewportRect().Size;
        float width = Math.Min(area.X, 14 * Tokens().Px("row.height"));
        float height = list.GetCombinedMinimumSize().Y + Panel.GetThemeStylebox("panel").GetMinimumSize().Y;
        Panel.Size = new Vector2(width, Math.Min(area.Y, height));
        Panel.Position = new Vector2(Math.Clamp(pointer.X, 0, Math.Max(0, area.X - Panel.Size.X)),
            Math.Clamp(pointer.Y, 0, Math.Max(0, area.Y - Panel.Size.Y)));
    }

    private void Move(int by)
    {
        if (selected >= 0) rows[selected].ThemeTypeVariation = "Row";
        selected = selected == -1 && by < 0 ? rows.Count - 1 : (selected + by + rows.Count) % rows.Count;
        rows[selected].ThemeTypeVariation = "RowSelected";
        scroll.EnsureControlVisible(rows[selected]);
    }

    private bool Choose(int index)
    {
        if (index < 0 || index >= entries.Count) return false;
        RadialEntry entry = entries[index];
        Close();
        Safe.Raise(Chosen, entry);
        return true;
    }

    public override bool Key(string key)
    {
        switch (key)
        {
            case "Up": Move(-1); return true;
            case "Down": Move(1); return true;
            case "Enter" or "Kp Enter": Choose(selected); return true;
            default: return key.Length == 1 && key[0] is >= '1' and <= '9' && Choose(key[0] - '1');
        }
    }
}
