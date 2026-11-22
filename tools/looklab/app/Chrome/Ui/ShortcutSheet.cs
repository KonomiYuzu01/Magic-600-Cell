using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The shortcut sheet (ADR-D01.20): every binding of the keymap in force, grouped by the context it acts in.
// It is generated from the keymap, so a rebound key shows here without another edit.
public sealed partial class ShortcutSheet : ChromeOverlay
{
    public static readonly IReadOnlyDictionary<string, string> Titles = new Dictionary<string, string>
    {
        ["global"] = "Everywhere", ["global-view"] = "Pointer over Global", ["local-view"] = "Pointer over Local",
        ["editor"] = "In the editor", ["preview"] = "While a preview is open",
    };
    public int Shown { get; private set; }
    private readonly CommandRegistry registry;
    private Keymap keymap;
    public Keymap Keymap { get => keymap; set => keymap = value; }
    private readonly HBoxContainer columns = new() { Name = "ShortcutColumns" };

    public ShortcutSheet(CommandRegistry registry, Keymap keymap, Func<TokenSet> tokens) : base(tokens, .8f, .08f)
    {
        this.registry = registry; this.keymap = keymap; Name = "ShortcutSheet";
        var column = new VBoxContainer(); Panel.AddChild(column);
        column.AddChild(ChromeWidgets.Text("Keyboard shortcuts", "TitleLabel"));
        column.AddChild(ChromeWidgets.Text("Keys act on the view under the pointer first.", "DimLabel"));
        column.AddChild(new HSeparator()); column.AddChild(columns);
    }

    public override void Open()
    {
        base.Open();
        foreach (Node child in columns.GetChildren()) { columns.RemoveChild(child); child.QueueFree(); }
        TokenSet tokens = Tokens(); Shown = 0;
        columns.AddThemeConstantOverride("separation", tokens.Px("space.4"));
        foreach (string context in Keymap.Contexts)
        {
            Binding[] bindings = keymap.Bindings.Where(b => b.Context == context).ToArray();
            if (bindings.Length == 0) continue;
            var group = new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill }; group.SetMeta("context", context); columns.AddChild(group);
            group.AddChild(ChromeWidgets.Text(Titles[context], "HeadingLabel"));
            foreach (Binding binding in bindings)
            {
                var line = new HBoxContainer(); line.SetMeta("key", binding.Key);
                var chip = new PanelContainer { ThemeTypeVariation = "Chip", CustomMinimumSize = new Vector2(tokens.Px("row.height") * 3, 0) };
                Label key = ChromeWidgets.Text(binding.Key, "CodeLabel"); key.HorizontalAlignment = HorizontalAlignment.Center; chip.AddChild(key);
                line.AddChild(chip);
                Label label = ChromeWidgets.Text(binding.Label ?? registry.Commands[binding.Command].Label, "", true); label.SizeFlagsHorizontal = SizeFlags.ExpandFill;
                label.TooltipText = binding.Command + (binding.Argument == null ? "" : " " + binding.Argument);
                line.AddChild(label); group.AddChild(line); Shown++;
            }
        }
    }
}

// A radial menu (ADR-D01.21): the entries of one radial command in a ring around the pointer. An entry is
// chosen by a click or by its number; an unavailable entry stays in its place, dimmed, and says why.
public sealed partial class RadialOverlay : ChromeOverlay
{
    public RadialMenu? Menu { get; private set; }
    public IReadOnlyList<Button> Entries => entries;
    public event Action<RadialEntry>? Chosen;
    private readonly CommandRegistry registry;
    private readonly Func<string, string?, string?> unavailable;
    private readonly List<Button> entries = new();
    private readonly Control ring = new() { Name = "RadialRing", MouseFilter = MouseFilterEnum.Ignore };

    public RadialOverlay(CommandRegistry registry, Func<string, string?, string?> unavailable, Func<TokenSet> tokens) : base(tokens, .2f, .5f)
    {
        this.registry = registry; this.unavailable = unavailable; Name = "RadialOverlay";
        Panel.Visible = false; AddChild(ring); ring.SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
    }

    public string Label(RadialEntry entry) => entry.Label ?? registry.Commands[entry.Command].Label;

    public void Open(RadialMenu menu, Vector2 at)
    {
        base.Open(); Menu = menu;
        foreach (Node child in ring.GetChildren()) { ring.RemoveChild(child); child.QueueFree(); }
        entries.Clear();
        TokenSet tokens = Tokens();
        var size = new Vector2(tokens.Px("row.height") * 7, tokens.Px("row.height") * 1.4f);
        float radius = size.X * .5f + size.Y * (menu.Entries.Count * .55f);
        Vector2 area = Size.X > 0 ? Size : GetViewportRect().Size;
        var centre = new Vector2(Math.Clamp(at.X, radius + size.X / 2, Math.Max(radius + size.X / 2, area.X - radius - size.X / 2)),
            Math.Clamp(at.Y, radius + size.Y, Math.Max(radius + size.Y, area.Y - radius - size.Y)));
        var hub = new PanelContainer { ThemeTypeVariation = "Chip" }; hub.AddChild(ChromeWidgets.Text(menu.Label, "HeadingLabel"));
        ring.AddChild(hub); hub.Position = centre - hub.GetCombinedMinimumSize() / 2;
        for (int i = 0; i < menu.Entries.Count; i++)
        {
            RadialEntry entry = menu.Entries[i]; int index = i;
            double angle = Math.PI * 2 * i / menu.Entries.Count;
            string? why = unavailable(entry.Command, entry.Argument);
            var button = new Button { Text = $"{i + 1}  {Label(entry)}", ThemeTypeVariation = "RadialButton", CustomMinimumSize = size, ClipText = true, FocusMode = FocusModeEnum.None };
            button.TooltipText = why == null ? registry.Commands[entry.Command].Help : "Unavailable: " + why;
            if (why != null) button.AddThemeColorOverride("font_color", tokens.C("text.off"));
            button.SetMeta("entry", index); button.Pressed += () => Choose(index);
            ring.AddChild(button); button.Size = size;
            button.Position = centre + new Vector2((float)Math.Sin(angle), -(float)Math.Cos(angle) * .8f) * radius - size / 2;
            entries.Add(button);
        }
    }

    public bool Choose(int index)
    {
        if (Menu == null || index < 0 || index >= Menu.Entries.Count) return false;
        RadialEntry entry = Menu.Entries[index];
        Close(); Safe.Raise(Chosen, entry);
        return true;
    }

    public override bool Key(string key) => key.Length == 1 && key[0] is >= '1' and <= '8' && Choose(key[0] - '1');
}
