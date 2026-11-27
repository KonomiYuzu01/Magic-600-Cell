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
            seats = value; seen = ""; legend.Text = Legend();
            if (Visible) Refresh();
        }
    }
    private const string LegendWords = "whole outline: bound here · double outline: a twist key · broken outline: bound in another place · faint: unbound · filled: held";
    private const string GripWords = " · a cycle such as (123): how a twist key moves the grip's numbered marks";
    private string Legend()
    {
        string marks = (Changed.Any(c => c.Context != KeyMapModel.GripContext), Changed.Any(c => c.Context == KeyMapModel.GripContext)) switch
        {
            (true, false) => " · corner mark: changed meaning with the last key set change",
            (false, true) => " · corner mark: changed meaning since the grip frame changed",
            (true, true) => " · corner mark: changed meaning with the last key set change or since the grip frame changed",
            _ => "",
        };
        return LegendWords + (seats == null ? "" : " · a held seat key lights its twelve twist keys with their rotation orders")
            + (grip is { Ready: true } ? GripWords : "") + marks;
    }
    public string LegendText => legend.Text;
    // ADR-D01.74/.75: the selected grip while D02's live session has one. Its twist keys' caps carry their cycles over
    // the grip's numbered marks, stamped as the views stamp them, and a frame change on the same cap marks each of those
    // caps whose twist differs from the one the map last showed, as a key set change does. A grip on another cap drops
    // those marks.
    private GripFrame? grip; private int gripCap = -1; private IReadOnlyList<GripKey> gripKeys = Array.Empty<GripKey>();
    public GripFrame? Grip
    {
        get => grip;
        set
        {
            if (ReferenceEquals(grip, value)) return;
            if (grip != null) grip.Changed -= GripChanged;
            grip = value; Changed = Changed.Where(c => c.Context != KeyMapModel.GripContext).ToArray(); Remember();
            if (grip != null && IsInsideTree()) grip.Changed += GripChanged;
            seen = ""; legend.Text = Legend();
            if (Visible) Refresh();
        }
    }
    // The stamps the board drew last: the key, the twist, the cap's box and the stamp's centre, in the board's space.
    public IReadOnlyList<(string Key, GripKey Turn, Rect2 Box, Vector2 At)> GripStamps => board.Stamps;

    private void Remember()
    {
        if (grip is { Ready: true }) { gripCap = grip.Cap; gripKeys = grip.KeyMeanings; }
        else { gripCap = -1; gripKeys = Array.Empty<GripKey>(); }
    }

    // The grip marks: each chord whose twist differs from the one the map last showed (or, before that, from the grip's
    // first frame on this cap). A mark not yet seen survives a second frame change; a frame changed back marks nothing.
    // The base moves on when the shown marks are cleared or the grip takes another cap.
    private void MarkGrip()
    {
        KeyChange[] kept = Changed.Where(c => c.Context != KeyMapModel.GripContext).ToArray();
        if (grip is { Ready: true } && grip.Cap == gripCap)
        {
            IReadOnlyList<KeyChange> changes = KeyMapModel.GripChanges(gripKeys, grip.KeyMeanings);
            Changed = kept.Concat(changes).ToArray();
            if (changes.Count > 0) marksSeen = false;
        }
        else { Changed = kept; Remember(); }
        seen = ""; legend.Text = Legend();
    }

    private void GripChanged()
    {
        if (!IsInstanceValid(this) || grip == null) return;
        MarkGrip();
        if (Visible) Refresh();
    }

    public override void _EnterTree()
    {
        if (grip == null) return;
        grip.Changed -= GripChanged; grip.Changed += GripChanged;
        // A frame change while the map was out of the tree is marked as one made in it.
        MarkGrip();
    }

    public override void _ExitTree()
    {
        if (grip != null) grip.Changed -= GripChanged;
    }
    // ADR-D01.74: a key never changes meaning silently. The keys the last keymap change gave another meaning stay
    // marked until the map has been shown with them and closed; whoever changes the keymap says the change in words.
    public IReadOnlyList<KeyChange> Changed { get; private set; } = Array.Empty<KeyChange>();
    private Keymap? last; private bool marksSeen;
    // The help row whose key shows the map: null takes the first help key (F1 in the default keymap). The product
    // outside sample mode gives the key list's topic, so F1 keeps ADR-D02.12's help route.
    public string? ShowArgument { get; set; }

    public KeyMapPanel(ChromeFrame frame, CommandRegistry registry)
    {
        this.frame = frame; this.registry = registry; Name = "KeyMap"; ThemeTypeVariation = "KeyMapBody"; Visible = false;
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
        Shown.Changed += () =>
        {
            if (!IsInstanceValid(this)) return;
            Visible = Shown.Visible; PinButton.SetPressedNoSignal(Shown.Pinned);
            if (Visible) { MoveToFront(); Refresh(); }
            else if (marksSeen) { Changed = Array.Empty<KeyChange>(); marksSeen = false; Remember(); seen = ""; legend.Text = Legend(); }
        };
    }

    private static double Now => Time.GetTicksMsec() / 1000.0;
    // The key that shows the map is the one bound to the shortcut help.
    public string? ShowKey => frame.Keys?.Bindings.FirstOrDefault(b => b.Command == "help.open" && (ShowArgument == null || b.Argument == ShowArgument))?.Key;

    // Marks the keys a new keymap gave another meaning; the first keymap the frame takes marks none.
    private void Watch()
    {
        if (frame.Keys is not { } keys || ReferenceEquals(keys, last)) return;
        if (last != null && KeyMapModel.Changes(last, keys, registry) is { Count: > 0 } changes)
        {
            Changed = changes.Concat(Changed.Where(c => c.Context == KeyMapModel.GripContext)).ToArray(); marksSeen = false; seen = ""; legend.Text = Legend();
        }
        last = keys;
    }

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

    public override void _Process(double delta) { Watch(); if (Visible) Refresh(); }

    public void Refresh()
    {
        Watch();
        if (frame.Keys is not { } keys) return;
        IReadOnlyList<string> contexts = ContextsOverride ?? frame.KeyContexts();
        (bool ctrl, bool alt, bool shift) = ModifiersOverride ?? (Input.IsKeyPressed(Key.Ctrl), Input.IsKeyPressed(Key.Alt), Input.IsKeyPressed(Key.Shift) && !Shown.Held);
        string gripNow = grip is { Ready: true } g ? $"{g.Cap}:{g.Frame}:{string.Join(",", g.KeyMeanings.Select(k => k.Item))}" : "-";
        string now = string.Join("/", contexts) + $"|{ctrl}{alt}{shift}|{string.Join(",", down.OrderBy(k => k, StringComparer.Ordinal))}|{keySet}|{keys.Bindings.Count}|{frame.Tokens.GetHashCode()}|{gripNow}";
        Vector2 room = GetParent() is Control parent ? parent.Size : new Vector2(1600, 900);
        if (now != seen || Caps.Count == 0)
        {
            seen = now; Caps = KeyMapModel.Mark(KeyMapModel.Caps(keys, registry, contexts, ctrl, alt, shift, down, (IReadOnlyDictionary<string, string>)new Dictionary<string, string>(Host), seats, grip), Changed, ctrl, alt, shift);
            head.Text = "  " + KeyMapModel.Header(keySet, contexts, ctrl, alt, shift, grip);
            board.Unit = Math.Clamp(room.X * .86f / 13.2f, 46, 96); board.CustomMinimumSize = new Vector2(board.Unit * 13.2f, board.Unit * .82f * KeyMapModel.Rows.Count); board.QueueRedraw();
        }
        if (Visible && Changed.Count > 0) marksSeen = true;
        Size = GetCombinedMinimumSize(); Position = new Vector2((room.X - Size.X) / 2, Math.Max(0, room.Y - Size.Y - frame.Tokens.Px("bar.height") * 3));
    }

    private sealed partial class Board : Control
    {
        private readonly KeyMapPanel map; public float Unit = 64;
        public readonly List<(string Key, GripKey Turn, Rect2 Box, Vector2 At)> Stamps = new();
        public Board(KeyMapPanel map) { this.map = map; MouseFilter = MouseFilterEnum.Ignore; }

        public override void _Draw()
        {
            TokenSet t = map.frame.Tokens; Font font = GetThemeFont("font", "MicroLabel"); int size = Math.Max(8, GetThemeFontSize("font_size", "MicroLabel"));
            float high = Unit * .82f, gap = 3;
            Stamps.Clear();
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
                    if (cap.Changed)
                    {
                        float corner = Math.Min(10, box.Size.Y * .3f);
                        DrawColoredPolygon(new[] { box.End with { Y = box.Position.Y } - new Vector2(corner, 0), box.End with { Y = box.Position.Y }, box.End with { Y = box.Position.Y + corner } }, t.C("text"));
                    }
                    DrawString(font, box.Position + new Vector2(5, size + 2), cap.Legend, HorizontalAlignment.Left, box.Size.X - 8, size, cap.Label == null ? colour : t.C("text"));
                    if (cap.Grip is { } turn)
                    {
                        // ADR-D01.74: the cap stamps the twist's cycle over the grip's marks with the views' own mark.
                        int radius = GripMarks.Radius(t);
                        var at = new Vector2(box.GetCenter().X, Math.Min(box.Position.Y + size + 6 + radius, box.End.Y - radius - 1));
                        GripMarks.Direction(this, at, turn, t);
                        Stamps.Add((cap.Key, turn, box, at));
                        continue;
                    }
                    if (cap.Label == null) continue;
                    string said = cap.State == KeyCapState.OutOfContext && cap.Context != null && KeyMapModel.Where.TryGetValue(cap.Context, out string? place) ? cap.Label + " (" + place + ")" : cap.Label;
                    DrawMultilineString(font, box.Position + new Vector2(5, size * 2 + 4), said, HorizontalAlignment.Left, box.Size.X - 8, size - 1, 2, cap.State == KeyCapState.OutOfContext ? colour : t.C("text.dim"));
                }
            }
        }
    }
}
