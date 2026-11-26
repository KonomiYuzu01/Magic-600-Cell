using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

public enum KeyCapState { Bound, Unbound, Held, OutOfContext, Twist }

// One keycap of the key map: what the key does now, in the contexts and with the modifiers given.
public sealed record KeyCap(string Key, string Legend, float Width, KeyCapState State, bool Down, string? Label, string? Context);

// The key map (ADR-D01.74): the keymap in force drawn as a keyboard. What a key does depends only on the key set,
// the contexts and the held modifiers, and the header names all three, so a key never changes meaning silently.
public static class KeyMapModel
{
    private static IReadOnlyList<(string Key, string Legend, float Width)> Row(string spec) => spec.Split(' ').Select(s =>
    {
        string[] p = s.Split('|');
        return (p[0], p.Length > 1 ? p[1] : p[0], p.Length > 2 ? float.Parse(p[2], CultureInfo.InvariantCulture) : 1f);
    }).ToArray();

    public static readonly IReadOnlyList<IReadOnlyList<(string Key, string Legend, float Width)>> Rows = new[]
    {
        Row("Escape|Esc F1 F2 F3 F4 F5 F6 F7 F8 F9 F10 F11 F12"),
        Row("QuoteLeft|` 1 2 3 4 5 6 7 8 9 0 Minus|- Equal|="),
        Row("Tab|Tab Q W E R T Y U I O P BracketLeft|[ BracketRight|]"),
        Row("A A|A|0 S D F G H J K L Semicolon|; Apostrophe|' Enter|Enter|1.5").Where(k => k.Width > 0).ToArray(),
        Row("Z Z|Z|0 X C V B N M Comma|, Period|. Slash|/").Where(k => k.Width > 0).ToArray(),
        Row("Space|Space|5 Left Right Up Down"),
    };

    public const string Host = "host";
    public static readonly IReadOnlyDictionary<string, string> Where = new Dictionary<string, string>
    {
        ["global-view"] = "pointer over Global", ["local-view"] = "pointer over Local", ["editor"] = "in the editor", ["preview"] = "a preview is open", [Host] = "this window",
    };

    // Twist keys have their own mark: they change the puzzle or the draft, the others do not.
    public static bool IsTwist(string command) => command == "operation.phase.append_twist";

    public static string Label(Binding binding, CommandRegistry registry) =>
        binding.Label ?? (registry.Commands.TryGetValue(binding.Command, out ChromeCommand? c) ? c.Label : binding.Command);

    // ADR-D02.42: holding seat k shows its twelve twist keys, the seat key alone first, each with its rotation order.
    public static IReadOnlyList<SeatTwistKey> HeldSeat(SeatKeys seats, int seat)
    {
        SeatEntry entry = seats.Seat(seat);
        return entry.Twists.Select(t => t.With == null ? new SeatTwistKey(entry.Key, entry.Key, t.Item, t.Order) : new SeatTwistKey(t.With, entry.Key + "+" + t.With, t.Item, t.Order)).ToArray();
    }

    public static IReadOnlyList<IReadOnlyList<KeyCap>> Caps(Keymap keymap, CommandRegistry registry, IReadOnlyList<string> contexts, bool ctrl, bool alt, bool shift,
        IReadOnlyCollection<string> down, IReadOnlyDictionary<string, string>? host = null, SeatKeys? seats = null)
    {
        // One held seat key with no modifier lights its seat's twelve twist keys with their orders (ADR-D02.42).
        bool plain = !ctrl && !alt && !shift;
        SeatEntry? seat = seats != null && plain ? seats.Down(down) : null;
        Dictionary<string, SeatTwistKey> lit = seat != null ? HeldSeat(seats!, seat.Seat).ToDictionary(t => t.Key, StringComparer.Ordinal) : new Dictionary<string, SeatTwistKey>(StringComparer.Ordinal);
        return Rows.Select(row => (IReadOnlyList<KeyCap>)row.Select(k =>
        {
            string full = Keymap.KeyName(ctrl, alt, shift, k.Key); bool held = down.Contains(k.Key);
            KeyCap Cap(KeyCapState state, string? label, string? context) => new(k.Key, k.Legend, k.Width, held ? KeyCapState.Held : state, held, label, context);
            if (seat != null && lit.TryGetValue(k.Key, out SeatTwistKey? twist))
                return k.Key == seat.Key
                    ? new KeyCap(k.Key, k.Legend + " " + seat.Glyph, k.Width, KeyCapState.Held, true, seat.Label + " · " + twist.Numeral, SeatKeys.Context)
                    : new KeyCap(k.Key, k.Legend, k.Width, KeyCapState.Twist, held, twist.Numeral, SeatKeys.Context);
            if (keymap.Resolve(full, contexts) is { } binding) return Cap(IsTwist(binding.Command) ? KeyCapState.Twist : KeyCapState.Bound, Label(binding, registry), binding.Context);
            if (host != null && host.TryGetValue(full, out string? own)) return Cap(KeyCapState.Bound, own, Host);
            if (seats != null && plain && seats.OfKey(k.Key) is { } entry) return Cap(KeyCapState.Bound, entry.Label, SeatKeys.Context) with { Legend = k.Legend + " " + entry.Glyph };
            // Bound, but not where the pointer is: the cap says where it acts.
            if (keymap.Bindings.FirstOrDefault(b => b.Key == full) is { } other) return Cap(KeyCapState.OutOfContext, Label(other, registry), other.Context);
            return Cap(KeyCapState.Unbound, null, null);
        }).ToArray()).ToArray();
    }

    public static string Place(IReadOnlyList<string> contexts)
    {
        string[] named = contexts.Where(c => c != Keymap.Global && Where.ContainsKey(c)).Select(c => Where[c]).Distinct().ToArray();
        return named.Length == 0 ? "everywhere" : string.Join(", ", named);
    }

    public static string Header(string keySet, IReadOnlyList<string> contexts, bool ctrl, bool alt, bool shift)
    {
        string[] held = new[] { ctrl ? "Ctrl" : null, alt ? "Alt" : null, shift ? "Shift" : null }.Where(m => m != null).ToArray()!;
        return $"Key set: {keySet} · {Place(contexts)} · " + (held.Length == 0 ? "no modifier held" : "with " + string.Join("+", held));
    }
}

// One twist a held seat carries: the seat key alone (With null) or with one retained twist key. Item indexes the pinned
// menu, and Order is the twist's rotation order.
public sealed record SeatTwist(string? With, int Item, int Order);

// A held seat entry (ADR-D02.42): its key, glyph and label, placeholders until XM1 and G6, and its twelve twists.
public sealed record SeatEntry(int Seat, string Key, string Glyph, string Label, IReadOnlyList<SeatTwist> Twists);

// A retained twist key: held with a seat key it types that seat's twist through this item of the cap's retained group.
public sealed record RetainedKey(string Key, int Item, int Order);

// One of a held seat's twist keys as the key map lights it: the key, the chord it routes as, the item and its order.
public sealed record SeatTwistKey(string Key, string Chord, int Item, int Order)
{
    public string Numeral => Order.ToString(CultureInfo.InvariantCulture);
}

// ADR-D02.42's four held seat keys, read from keymap.json's "seats" section. Seat k is the right coset A4_c·g^k of the
// cap's retained group for the pinned fifth-turn g: its key tapped alone types g^k, and held with retained twist key r
// it types r·g^k, g^k performed first. The four seats carry the menu's 48 jumble twists. CR-19 routes a held seat key
// with a twist key as one chord; until it lands the key map shows the form and no chord routes.
public sealed class SeatKeys
{
    public const string Section = "seats";
    public const string Context = "jumble";
    public const int Seats = 4, PerSeat = 12, RetainedKeys = 11, MenuItems = 60;
    public string Menu { get; }
    public string MenuIdentity { get; }
    public int FifthTurn { get; }
    public IReadOnlyList<RetainedKey> Retained { get; }
    public IReadOnlyList<SeatEntry> Held { get; }

    private SeatKeys(string menu, string identity, int fifthTurn, IReadOnlyList<RetainedKey> retained, IReadOnlyList<SeatEntry> held)
    {
        Menu = menu; MenuIdentity = identity; FifthTurn = fifthTurn; Retained = retained; Held = held;
    }

    public SeatEntry Seat(int seat) => seat is >= 1 and <= Seats ? Held[seat - 1] : throw new ArgumentOutOfRangeException(nameof(seat), "seats are 1 to 4");
    public SeatEntry? OfKey(string key) => Held.FirstOrDefault(h => h.Key == key);

    // The one seat whose key is held; none while no seat key, or more than one, is held.
    public SeatEntry? Down(IReadOnlyCollection<string> down)
    {
        SeatEntry[] held = Held.Where(h => down.Contains(h.Key)).ToArray();
        return held.Length == 1 ? held[0] : null;
    }

    // The default keymap's held seat entries, or null when its text has none.
    public static SeatKeys? Read(string keymapText)
    {
        using JsonDocument d = J.Parse(keymapText);
        if (d.RootElement.ValueKind != JsonValueKind.Object || !d.RootElement.TryGetProperty(Section, out JsonElement s)) return null;
        J.Keys(s, Section, "note", "menu", "menu_identity", "fifth_turn", "retained", "held");
        J.Text(s, "note");
        int fifth = Int(s.GetProperty("fifth_turn"), "seats.fifth_turn", RetainedKeys + 1, MenuItems - 1);
        RetainedKey[] retained = List(s, "retained", RetainedKeys).Select((r, i) =>
        {
            J.Keys(r, "seats.retained", "key", "item", "order");
            var key = new RetainedKey(Key(r, "seats.retained"), Int(r.GetProperty("item"), "seats.retained.item", 1, RetainedKeys), Int(r.GetProperty("order"), "seats.retained.order", 2, 3));
            return key.Item == i + 1 ? key : throw J.Error("seats.retained", "retained twist keys follow items 1 to 11 in order");
        }).ToArray();
        string[] keys = retained.Select(r => r.Key).ToArray();
        if (keys.Distinct(StringComparer.Ordinal).Count() != RetainedKeys) throw J.Error("seats.retained", "repeated retained twist key");
        SeatEntry[] held = List(s, "held", Seats).Select((h, i) =>
        {
            J.Keys(h, "seats.held", "seat", "key", "glyph", "label", "twists");
            int seat = Int(h.GetProperty("seat"), "seats.held.seat", 1, Seats);
            if (seat != i + 1) throw J.Error("seats.held", "seats follow 1 to 4 in order");
            string key = Key(h, "seats.held");
            if (keys.Contains(key)) throw J.Error("seats.held", "seat key " + key + " is a retained twist key");
            SeatTwist[] twists = List(h, "twists", PerSeat).Select((t, j) =>
            {
                if (j == 0) J.Keys(t, "seats.held.twists", "item", "order"); else J.Keys(t, "seats.held.twists", "with", "item", "order");
                string? with = j == 0 ? null : J.Text(t, "with");
                if (j > 0 && with != keys[j - 1]) throw J.Error("seats.held.twists", "seat " + seat + "'s twists follow the retained twist keys in order");
                int order = Int(t.GetProperty("order"), "seats.held.twists.order", 2, 5);
                if (order == 4) throw J.Error("seats.held.twists.order", "a rotation order of the menu is 2, 3 or 5");
                return new SeatTwist(with, Int(t.GetProperty("item"), "seats.held.twists.item", RetainedKeys + 1, MenuItems - 1), order);
            }).ToArray();
            return new SeatEntry(seat, key, J.Text(h, "glyph"), J.Text(h, "label"), twists);
        }).ToArray();
        if (held.Select(h => h.Key).Distinct(StringComparer.Ordinal).Count() != Seats) throw J.Error("seats.held", "repeated seat key");
        if (held.SelectMany(h => h.Twists).Select(t => t.Item).Distinct().Count() != Seats * PerSeat) throw J.Error("seats.held", "a jumble twist on two seats, or twice on one");
        if (held[0].Twists[0].Item != fifth) throw J.Error("seats.held", "seat 1's key alone types the fifth-turn");
        return new SeatKeys(J.Text(s, "menu"), J.Text(s, "menu_identity"), fifth, retained, held);
    }

    private static JsonElement[] List(JsonElement parent, string name, int count)
    {
        JsonElement list = parent.GetProperty(name);
        if (list.ValueKind != JsonValueKind.Array || list.GetArrayLength() != count) throw J.Error("seats." + name, $"expected {count} entries");
        return list.EnumerateArray().ToArray();
    }

    private static int Int(JsonElement value, string id, int low, int high) =>
        value.ValueKind == JsonValueKind.Number && value.TryGetInt32(out int v) && v >= low && v <= high ? v : throw J.Error(id, $"expected a whole number from {low} to {high}");

    // A plain key of the key map's keyboard: no modifier, and drawn there.
    private static string Key(JsonElement parent, string id)
    {
        string key = J.Text(parent, "key");
        return !key.Contains('+') && KeyMapModel.Rows.Any(r => r.Any(k => k.Key == key)) ? key : throw J.Error(id, key + " is not a plain key of the keyboard");
    }
}

// Shown while its key is held; a short press pins it and the next short press lets it go.
public sealed class KeyMapShow
{
    public const double Tap = .3;
    private double since;
    public bool Held { get; private set; }
    public bool Pinned { get; private set; }
    public bool Visible => Held || Pinned;
    public event Action? Changed;
    public void Press(double now) { if (Held) return; Held = true; since = now; Safe.Raise(Changed); }
    public void Release(double now) { if (!Held) return; Held = false; if (now - since < Tap) Pinned = !Pinned; Safe.Raise(Changed); }
    public void Pin(bool on) { if (Pinned == on) return; Pinned = on; Safe.Raise(Changed); }
}
