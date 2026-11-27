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
// chosen by a click or by its number; an unavailable entry stays in its place, dimmed, and says why. Held on the key
// that opened it (ADR-D01.20), the entry the pointer points at is marked and the hub says what release runs; release
// runs it, release in the hub runs nothing, and a short tap leaves the menu open.
public sealed partial class RadialOverlay : ChromeOverlay
{
    public RadialMenu? Menu { get; private set; }
    public IReadOnlyList<Button> Entries => entries;
    public event Action<RadialEntry>? Chosen;
    // ADR-D01.20: the entry release runs while the opening key is held (-1: the hub), and a release in the hub.
    public int Pointed { get; private set; } = -1;
    public bool Held => heldKey != null;
    public event Action<RadialMenu>? Let;
    private string? heldKey;
    private ulong heldAt;
    private Vector2 centre;
    private float dead;
    private PanelContainer? hub;
    private Label? hint;
    private readonly List<(double X, double Y)> directions = new();
    private readonly CommandRegistry registry;
    private readonly Func<string, string?, string?> unavailable;
    private readonly List<Button> entries = new();
    private readonly Control ring = new() { Name = "RadialRing", MouseFilter = MouseFilterEnum.Ignore };

    public RadialOverlay(CommandRegistry registry, Func<string, string?, string?> unavailable, Func<TokenSet> tokens) : base(tokens, .2f, .5f)
    {
        this.registry = registry; this.unavailable = unavailable; Name = "RadialOverlay";
        Panel.Visible = false; AddChild(ring); ring.SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        Closed += () => { heldKey = null; Pointed = -1; };
    }

    public string Label(RadialEntry entry) => entry.Label ?? registry.Commands[entry.Command].Label;

    public void Open(RadialMenu menu, Vector2 at)
    {
        base.Open(); Menu = menu; heldKey = null; Pointed = -1; directions.Clear();
        foreach (Node child in ring.GetChildren()) { ring.RemoveChild(child); child.QueueFree(); }
        entries.Clear();
        TokenSet tokens = Tokens();
        var size = new Vector2(tokens.Px("row.height") * 7, tokens.Px("row.height") * 1.4f);
        float radius = size.X * .5f + size.Y * (menu.Entries.Count * .55f);
        Vector2 area = Size.X > 0 ? Size : GetViewportRect().Size;
        var centre = new Vector2(Math.Clamp(at.X, radius + size.X / 2, Math.Max(radius + size.X / 2, area.X - radius - size.X / 2)),
            Math.Clamp(at.Y, radius + size.Y, Math.Max(radius + size.Y, area.Y - radius - size.Y)));
        this.centre = centre; dead = tokens.Px("row.height") * 1.5f;
        hub = new PanelContainer { ThemeTypeVariation = "Chip", Name = "RadialHub" };
        var lines = new VBoxContainer { MouseFilter = MouseFilterEnum.Ignore }; hub.AddChild(lines);
        lines.AddChild(ChromeWidgets.Text(menu.Label, "HeadingLabel"));
        hint = ChromeWidgets.Text("", "DimLabel"); hint.Name = "RadialHint"; hint.Visible = false; lines.AddChild(hint);
        ring.AddChild(hub); Centre(hub);
        for (int i = 0; i < menu.Entries.Count; i++)
        {
            RadialEntry entry = menu.Entries[i]; int index = i;
            (double dx, double dy) = RadialHold.Direction(i, menu.Entries.Count); directions.Add((dx, dy));
            string? why = unavailable(entry.Command, entry.Argument);
            string icon = ChromeBody.Commands?.Of(entry.Command, entry.Argument) ?? "command.bar";
            var button = new Button { Text = "       " + Label(entry), Alignment = HorizontalAlignment.Left, ThemeTypeVariation = "RadialButton", Name = "RadialEntry" + (i + 1).ToString(System.Globalization.CultureInfo.InvariantCulture), CustomMinimumSize = size, ClipText = true, FocusMode = FocusModeEnum.None };
            button.TooltipText = why == null ? registry.Commands[entry.Command].Help : "Unavailable: " + why;
            if (why != null) button.AddThemeColorOverride("font_color", tokens.C("text.off"));
            button.SetMeta("entry", index); button.Pressed += () => Choose(index);
            ring.AddChild(button); button.Size = size;
            var mark = new Label { MouseFilter = MouseFilterEnum.Ignore }; ChromeIcons.Apply(mark, icon, tokens.Px("icon.medium")); button.AddChild(mark);
            mark.Position = new Vector2(tokens.Px("space.2"), (size.Y - mark.GetCombinedMinimumSize().Y) / 2);
            Label number = ChromeWidgets.Text((i + 1).ToString(System.Globalization.CultureInfo.InvariantCulture), "MicroLabel"); number.MouseFilter = MouseFilterEnum.Ignore; button.AddChild(number);
            number.Position = new Vector2(size.X - tokens.Px("space.3") - number.GetCombinedMinimumSize().X, (size.Y - number.GetCombinedMinimumSize().Y) / 2);
            if (why != null) mark.Modulate = mark.Modulate with { A = .4f };
            button.Position = centre + new Vector2((float)dx, (float)dy) * radius - size / 2;
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

    // The key that opened the menu is still down (ADR-D01.20): until it comes up the menu follows the pointer. A key
    // with modifiers is held by its last part (Shift+V by V).
    public void Hold(string key)
    {
        if (!IsOpen) return;
        heldKey = key[(key.LastIndexOf('+') + 1)..]; heldAt = Time.GetTicksMsec();
        Point(-1);
    }

    public override void _Input(InputEvent input)
    {
        if (!IsOpen || heldKey == null) return;
        if (input is InputEventMouseMotion motion)
        {
            Vector2 at = ((InputEventMouseMotion)MakeInputLocal(motion)).Position - centre;
            Point(RadialHold.Pointed(at.X, at.Y, directions, dead));
            return;
        }
        if (input is not InputEventKey { Pressed: false } key || OS.GetKeycodeString(key.Keycode) != heldKey) return;
        GetViewport().SetInputAsHandled();
        RadialRelease release = RadialHold.Release((Time.GetTicksMsec() - heldAt) / 1000.0, Pointed);
        int pointed = Pointed; heldKey = null; Point(-1);
        if (release == RadialRelease.Run) Choose(pointed);
        else if (release == RadialRelease.Nothing && Menu is { } menu) { Close(); Safe.Raise(Let, menu); }
    }

    private void Point(int index)
    {
        Pointed = index;
        for (int i = 0; i < entries.Count; i++)
        {
            if (i == index) entries[i].AddThemeStyleboxOverride("normal", entries[i].GetThemeStylebox("hover"));
            else entries[i].RemoveThemeStyleboxOverride("normal");
        }
        if (hint == null || hub == null || Menu == null) return;
        RadialEntry? entry = index < 0 ? null : Menu.Entries[index];
        hint.Visible = heldKey != null;
        hint.Text = RadialHold.Hint(entry == null ? null : Label(entry), entry == null ? null : unavailable(entry.Command, entry.Argument));
        Centre(hub);
    }

    private void Centre(Control box) { box.Size = Vector2.Zero; box.Position = centre - box.GetCombinedMinimumSize() / 2; }
}
