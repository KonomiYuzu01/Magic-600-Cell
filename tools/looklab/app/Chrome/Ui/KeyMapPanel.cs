using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The key map (ADR-D01.74): the keymap in force drawn as a keyboard, over the frame while its key is held, and
// pinnable. Each cap shows what the key does where the pointer is, and the header names the active key set, the
// place and the held modifiers. It takes no key away from the frame: it is not a modal overlay.
public sealed partial class KeyMapPanel : PanelContainer
{
    public KeyMapShow Shown { get; } = new();
    public IDictionary<string, string> Host { get; } = new Dictionary<string, string>(StringComparer.Ordinal);
    public IReadOnlyList<IReadOnlyList<KeyCap>> Caps { get; private set; } = Array.Empty<IReadOnlyList<KeyCap>>();
    public string Header => head.Text;
    public Button PinButton { get; }
    public event Action? List;
    private string keySet = "default";
    public string KeySet { get => keySet; set { if (keySet == value) return; keySet = value; Refresh(); } }
    private readonly ChromeFrame frame; private readonly CommandRegistry registry; private readonly Label head, legend; private readonly Board board;
    private readonly HashSet<string> down = new(StringComparer.Ordinal); private string seen = "";
    // Contexts pinned for a picture or a test; null follows the pointer.
    public IReadOnlyList<string>? ContextsOverride { get; set; }
    public (bool Ctrl, bool Alt, bool Shift)? ModifiersOverride { get; set; }
    // ADR-D02.42's held seat entries, while D02 is on: holding a seat key lights its seat's twelve twist keys.
    private SeatKeys? seats;
    public SeatKeys? Seats
    {
        get => seats;
        set
        {
            seats = value; seen = "";
            legend.Text = LegendWords + (value == null ? "" : " · a held seat key lights its twelve twist keys with their rotation orders");
            if (Visible) Refresh();
        }
    }
    private const string LegendWords = "whole outline: bound here · double outline: a twist key · broken outline: bound in another place · faint: unbound · filled: held";

    public KeyMapPanel(ChromeFrame frame, CommandRegistry registry)
    {
        this.frame = frame; this.registry = registry; Name = "KeyMap"; ThemeTypeVariation = "OverlayBox"; Visible = false;
        var column = new VBoxContainer(); AddChild(column);
        var top = new HBoxContainer(); column.AddChild(top);
        top.AddChild(ChromeWidgets.Text("Key map", "HeadingLabel"));
        head = ChromeWidgets.Text("", "DimLabel", true); head.SizeFlagsHorizontal = SizeFlags.ExpandFill; top.AddChild(head);
        var list = new Button { Text = "List", ThemeTypeVariation = "ToolButton", TooltipText = "Every binding as a list", FocusMode = FocusModeEnum.None, Name = "KeyMapList" };
        list.Pressed += () => Safe.Raise(List); top.AddChild(list);
        PinButton = new Button { Text = "Pin", ToggleMode = true, ThemeTypeVariation = "ToolButton", TooltipText = "Keep the key map open. A short press of its key does the same.", FocusMode = FocusModeEnum.None, Name = "KeyMapPin" };
        PinButton.Pressed += () => Shown.Pin(PinButton.ButtonPressed); top.AddChild(PinButton);
        board = new Board(this); column.AddChild(board);
        legend = ChromeWidgets.Text(LegendWords, "MicroLabel", true);
        column.AddChild(legend);
        Shown.Changed += () => { if (!IsInstanceValid(this)) return; Visible = Shown.Visible; PinButton.SetPressedNoSignal(Shown.Pinned); if (Visible) { MoveToFront(); Refresh(); } };
    }

    private static double Now => Time.GetTicksMsec() / 1000.0;
    // The key that shows the map is the one bound to the shortcut help.
    public string? ShowKey => frame.Keys?.Bindings.FirstOrDefault(b => b.Command == "help.open")?.Key;

    public override void _Input(InputEvent @event)
    {
        if (@event is not InputEventKey key || key.Echo) return;
        string name = OS.GetKeycodeString(key.Keycode);
        if (key.Pressed) down.Add(name); else down.Remove(name);
        if (ShowKey is { } show && show.Split('+')[^1] == name)
        {
            if (!key.Pressed) { if (Shown.Held) { Shown.Release(Now); GetViewport().SetInputAsHandled(); } }
            else if (Keymap.KeyName(key.CtrlPressed, key.AltPressed, key.ShiftPressed, name) == show && frame.OpenOverlay == null && GetViewport().GuiGetFocusOwner() is not (LineEdit or TextEdit))
            { Shown.Press(Now); GetViewport().SetInputAsHandled(); }
        }
        if (Visible) Refresh();
    }

    public override void _Process(double delta) { if (Visible) Refresh(); }

    public void Refresh()
    {
        if (frame.Keys is not { } keys) return;
        IReadOnlyList<string> contexts = ContextsOverride ?? frame.KeyContexts();
        (bool ctrl, bool alt, bool shift) = ModifiersOverride ?? (Input.IsKeyPressed(Key.Ctrl), Input.IsKeyPressed(Key.Alt), Input.IsKeyPressed(Key.Shift) && !Shown.Held);
        string now = string.Join("/", contexts) + $"|{ctrl}{alt}{shift}|{string.Join(",", down.OrderBy(k => k, StringComparer.Ordinal))}|{keySet}|{keys.Bindings.Count}|{frame.Tokens.GetHashCode()}";
        Vector2 room = GetParent() is Control parent ? parent.Size : new Vector2(1600, 900);
        if (now != seen || Caps.Count == 0)
        {
            seen = now; Caps = KeyMapModel.Caps(keys, registry, contexts, ctrl, alt, shift, down, (IReadOnlyDictionary<string, string>)new Dictionary<string, string>(Host), seats);
            head.Text = "  " + KeyMapModel.Header(keySet, contexts, ctrl, alt, shift);
            board.Unit = Math.Clamp(room.X * .86f / 13.2f, 46, 96); board.CustomMinimumSize = new Vector2(board.Unit * 13.2f, board.Unit * .82f * KeyMapModel.Rows.Count); board.QueueRedraw();
        }
        Size = GetCombinedMinimumSize(); Position = new Vector2((room.X - Size.X) / 2, Math.Max(0, room.Y - Size.Y - frame.Tokens.Px("bar.height") * 3));
    }

    private sealed partial class Board : Control
    {
        private readonly KeyMapPanel map; public float Unit = 64;
        public Board(KeyMapPanel map) { this.map = map; MouseFilter = MouseFilterEnum.Ignore; }

        public override void _Draw()
        {
            TokenSet t = map.frame.Tokens; Font font = GetThemeFont("font", "MicroLabel"); int size = Math.Max(8, GetThemeFontSize("font_size", "MicroLabel"));
            float high = Unit * .82f, gap = 3;
            for (int r = 0; r < map.Caps.Count; r++)
            {
                float x = r is 2 ? 0 : r is 3 ? Unit * .35f : r is 4 ? Unit * .7f : r is 5 ? Unit * 2.2f : 0;
                foreach (KeyCap cap in map.Caps[r])
                {
                    var box = new Rect2(x + gap / 2, r * high + gap / 2, cap.Width * Unit - gap, high - gap); x += cap.Width * Unit;
                    Color colour = t.C(cap.State switch
                    {
                        KeyCapState.Held => "keymap.key.held", KeyCapState.Twist => "keymap.key.twist", KeyCapState.OutOfContext => "keymap.key.out_of_context",
                        KeyCapState.Unbound => "keymap.key.unbound", _ => "keymap.key.bound",
                    });
                    switch (cap.State)
                    {
                        case KeyCapState.Held: DrawRect(box, colour with { A = .3f }); DrawRect(box, colour, false, 2); break;
                        case KeyCapState.Twist: DrawRect(box, colour, false, 1.5f); DrawRect(box.Grow(-3), colour, false, 1); break;
                        case KeyCapState.OutOfContext: RouteMark.Draw(this, box, true, colour, 1); break;
                        case KeyCapState.Unbound: DrawRect(box, colour with { A = .45f }, false, 1); break;
                        default: DrawRect(box, colour, false, 1.5f); break;
                    }
                    DrawString(font, box.Position + new Vector2(5, size + 2), cap.Legend, HorizontalAlignment.Left, box.Size.X - 8, size, cap.Label == null ? colour : t.C("text"));
                    if (cap.Label == null) continue;
                    string said = cap.State == KeyCapState.OutOfContext && cap.Context != null && KeyMapModel.Where.TryGetValue(cap.Context, out string? place) ? cap.Label + " (" + place + ")" : cap.Label;
                    DrawMultilineString(font, box.Position + new Vector2(5, size * 2 + 4), said, HorizontalAlignment.Left, box.Size.X - 8, size - 1, 2, cap.State == KeyCapState.OutOfContext ? colour : t.C("text.dim"));
                }
            }
        }
    }
}
