using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Numerics;
using System.Text.Json;
using System.Text.Json.Nodes;
using Magic600.Chrome;

// ADR-D02 build step B27: the four held seat entries (ADR-D02.42), Global's projection chain (ADR-D02.8) and the
// exposed faces' unavailable state (ADR-D02.25). The glossary check of ADR-D02.43 is tests/test_d02_words.py.
internal static partial class Program
{
    private static SeatKeys Seats() => SeatKeys.Read(Read("data/design/keymap.json")) ?? throw new Exception("keymap.json has no held seat entries");

    // An element of Q(√5), (a + b√5) / d, in lowest terms with d > 0.
    private sealed record Q5(BigInteger A, BigInteger B, BigInteger D)
    {
        public static Q5 Of(BigInteger a, BigInteger b, BigInteger d)
        {
            if (d.Sign < 0) { a = -a; b = -b; d = -d; }
            BigInteger g = BigInteger.GreatestCommonDivisor(BigInteger.GreatestCommonDivisor(a, b), d);
            return g.IsOne ? new Q5(a, b, d) : new Q5(a / g, b / g, d / g);
        }
        public static Q5 operator +(Q5 x, Q5 y) => Of(x.A * y.D + y.A * x.D, x.B * y.D + y.B * x.D, x.D * y.D);
        public static Q5 operator *(Q5 x, Q5 y) => Of(x.A * y.A + 5 * x.B * y.B, x.A * y.B + x.B * y.A, x.D * y.D);
    }

    private static Q5[] Rotation(JsonElement matrix)
    {
        Q5[] m = matrix.EnumerateArray().SelectMany(row => row.EnumerateArray())
            .Select(e => e.EnumerateArray().Select(n => new BigInteger(n.GetInt64())).ToArray()).Select(v => Q5.Of(v[0], v[1], v[2])).ToArray();
        Check(m.Length == 16, "a menu matrix is not 4 by 4");
        return m;
    }

    // Matrices act on column vectors: in x·y, y is performed first.
    private static Q5[] Times(Q5[] x, Q5[] y)
    {
        var m = new Q5[16];
        for (int r = 0; r < 4; r++)
            for (int c = 0; c < 4; c++)
            {
                Q5 sum = Q5.Of(0, 0, 1);
                for (int k = 0; k < 4; k++) sum += x[r * 4 + k] * y[k * 4 + c];
                m[r * 4 + c] = sum;
            }
        return m;
    }

    private static string Exact(Q5[] m) => string.Join(",", m.Select(q => q.A + "/" + q.B + "/" + q.D));

    // The pinned I_a menu (research/jumbling/fixtures/wj-I_a.json): its 60 items, exactly, and their rotation orders.
    private static (string Name, string Identity, Q5[][] Items, int[] Orders) Menu()
    {
        using JsonDocument d = JsonDocument.Parse(File.ReadAllText(Path.Combine(Root, "research/jumbling/fixtures/wj-I_a.json")));
        JsonElement menu = d.RootElement.GetProperty("menu");
        Q5[][] items = menu.GetProperty("base").EnumerateArray().Select(b => Rotation(b.GetProperty("matrix"))).ToArray();
        string one = Exact(Enumerable.Range(0, 16).Select(i => Q5.Of(i % 5 == 0 ? 1 : 0, 0, 1)).ToArray());
        Check(Exact(items[0]) == one, "item 0 of the menu is not the identity");
        int Order(Q5[] m)
        {
            Q5[] p = m; int n = 1;
            while (Exact(p) != one) { p = Times(p, m); n++; Check(n <= 60, "an item has no order within 60"); }
            return n;
        }
        return (menu.GetProperty("name").GetString()!, menu.GetProperty("identity").GetString()!, items, items.Select(Order).ToArray());
    }

    private static IEnumerable<(int, int)> Counts(IEnumerable<int> orders) => orders.GroupBy(o => o).OrderBy(g => g.Key).Select(g => (g.Key, g.Count()));

    // Holding seat k shows its twelve twist keys with their rotation orders: the seat key alone types g^k and, held
    // with retained twist key r, r·g^k, recomputed here exactly from the pinned menu; the four seats are the right cosets
    // of A4_c for the pinned fifth-turn g and carry the menu's 48 jumble twists.
    private static void D02HeldSeatShowsTwelveTwistsWithOrders()
    {
        SeatKeys seats = Seats();
        var (name, identity, items, orders) = Menu();
        Equal(seats.Menu, name, "the seats' menu");
        Equal(seats.MenuIdentity, identity, "the seats' menu identity");
        Equal(items.Length, SeatKeys.MenuItems, "the menu's items");
        var index = new Dictionary<string, int>(StringComparer.Ordinal);
        for (int i = 0; i < items.Length; i++) index.Add(Exact(items[i]), i);
        Sequence(Counts(orders), new[] { (1, 1), (2, 15), (3, 20), (5, 24) }, "the menu's orders are not 1 identity, 15 of order 2, 20 of order 3 and 24 of order 5");
        // A4_c is items 0 to 11: closed under products; the retained twist keys are its eleven other items, in order.
        for (int a = 0; a <= SeatKeys.RetainedKeys; a++)
            for (int b = 0; b <= SeatKeys.RetainedKeys; b++)
                Check(index[Exact(Times(items[a], items[b]))] <= SeatKeys.RetainedKeys, $"A4_c is not closed: items {a} and {b}");
        Sequence(seats.Retained.Select(r => r.Item), Enumerable.Range(1, SeatKeys.RetainedKeys), "the retained twist keys' items");
        Sequence(seats.Retained.Select(r => r.Order), orders.Skip(1).Take(SeatKeys.RetainedKeys), "the retained twist keys' orders");
        Equal(orders[seats.FifthTurn], 5, "the fifth-turn's order");
        var power = new List<Q5[]> { items[0] };
        for (int k = 1; k <= SeatKeys.Seats; k++) power.Add(Times(power[^1], items[seats.FifthTurn]));

        CommandRegistry registry = LabRegistry(); registry.AddRows(Read("data/design/command-rows-d02.json"));
        Keymap keymap = Keymap.Parse(Read("data/design/keymap.json"), registry);
        string[] global = { Keymap.Global };
        var all = new HashSet<int>();
        for (int k = 1; k <= SeatKeys.Seats; k++)
        {
            SeatEntry entry = seats.Seat(k);
            IReadOnlyList<SeatTwistKey> keys = KeyMapModel.HeldSeat(seats, k);
            Equal(keys.Count, SeatKeys.PerSeat, $"seat {k}'s twist keys");
            Equal(keys[0].Key, entry.Key, $"seat {k}'s first twist key");
            Equal(keys[0].Chord, entry.Key, $"seat {k}'s key alone");
            Equal(keys[0].Item, index[Exact(power[k])], $"seat {k}'s key alone does not type g^{k}");
            for (int r = 1; r < SeatKeys.PerSeat; r++)
            {
                RetainedKey with = seats.Retained[r - 1];
                Equal(keys[r].Key, with.Key, $"seat {k}'s twist key {r}");
                Equal(keys[r].Chord, entry.Key + "+" + with.Key, $"seat {k}'s chord {r}");
                Equal(keys[r].Item, index[Exact(Times(items[with.Item], power[k]))], $"seat {k} with {with.Key} does not type r·g^{k}");
            }
            foreach (SeatTwistKey t in keys)
            {
                Equal(t.Order, orders[t.Item], $"the order of item {t.Item}");
                Equal(t.Numeral, orders[t.Item].ToString(CultureInfo.InvariantCulture), $"the numeral of item {t.Item}");
                Check(all.Add(t.Item), $"item {t.Item} is on two seats");
            }
            Sequence(Counts(keys.Select(t => t.Order)), new[] { (2, 3), (3, 3), (5, 6) }, $"seat {k} does not carry 3 twists of order 2, 3 of order 3 and 6 of order 5");

            // The key map: holding the seat key lights its twelve keys with their numerals; the seat key shows held.
            KeyCap[] caps = KeyMapModel.Caps(keymap, registry, global, false, false, false, new[] { entry.Key }, null, seats).SelectMany(row => row).ToArray();
            KeyCap[] lit = caps.Where(c => c.Context == SeatKeys.Context && c.State is KeyCapState.Held or KeyCapState.Twist).ToArray();
            Equal(lit.Length, SeatKeys.PerSeat, $"seat {k}'s lit keys");
            KeyCap seatCap = caps.Single(c => c.Key == entry.Key);
            Check(seatCap.State == KeyCapState.Held && seatCap.Down && seatCap.Label == entry.Label + " · " + keys[0].Numeral, $"seat {k}'s own cap reads {seatCap}");
            foreach (SeatTwistKey t in keys.Skip(1))
            {
                KeyCap cap = caps.Single(c => c.Key == t.Key);
                Check(cap.State == KeyCapState.Twist && cap.Label == t.Numeral, $"seat {k}'s {t.Key} cap reads {cap}");
            }
        }
        Equal(all.Count, SeatKeys.Seats * SeatKeys.PerSeat, "the jumble twists of the four seats");
        Check(all.All(i => i > SeatKeys.RetainedKeys), "a seat carries a retained twist");

        // No seat held: each seat key shows its seat and glyph, and the retained keys stay unbound. Two seat keys held, or
        // a modifier held, light nothing.
        KeyCap[] rest = KeyMapModel.Caps(keymap, registry, global, false, false, false, Array.Empty<string>(), null, seats).SelectMany(row => row).ToArray();
        foreach (SeatEntry entry in seats.Held)
        {
            KeyCap cap = rest.Single(c => c.Key == entry.Key);
            Check(cap.State == KeyCapState.Bound && cap.Label == entry.Label && cap.Legend.EndsWith(entry.Glyph, StringComparison.Ordinal), $"seat {entry.Seat}'s key at rest reads {cap}");
        }
        Check(seats.Retained.All(r => rest.Single(c => c.Key == r.Key).State == KeyCapState.Unbound), "a retained twist key is bound with no seat held");
        int Lit(bool shift, params string[] down) =>
            KeyMapModel.Caps(keymap, registry, global, false, false, shift, down, null, seats).SelectMany(row => row).Count(c => c.Context == SeatKeys.Context && c.State == KeyCapState.Twist);
        Equal(Lit(false, "Z", "X"), 0, "two held seat keys lit keys");
        Equal(Lit(true, seats.Seat(1).Key), 0, "a held seat key with Shift lit keys");
        Check(KeyMapModel.Caps(keymap, registry, global, false, false, false, new[] { "Z" }).SelectMany(row => row).All(c => c.Context != SeatKeys.Context),
            "the key map shows seats without D02's entries");
    }

    // The sample's own host keys (SampleMount) and the live session's twist keys (the shell's status: J K L).
    private static readonly string[] HostKeys = { "O", "F4", "F10", "F11", "F12", "6", "7", "8", "9", "0" };
    private static readonly string[] LiveTwistKeys = { "J", "K", "L" };

    private static string WithSeats(string text, Action<JsonNode> change)
    {
        JsonNode node = JsonNode.Parse(text)!;
        change(node["seats"]!);
        return node.ToJsonString();
    }

    // No D02 chord is bound twice: the seat keys and the retained twist keys are bound nowhere in the keymap, in any
    // context, and are neither host keys, live twist keys nor digits (the repair slot's digits); the 48 chords are
    // distinct and type 48 distinct jumble twists. A seats section that repeats a key or a twist does not read.
    private static void D02NoDuplicateChordAmongD02Bindings()
    {
        SeatKeys seats = Seats();
        CommandRegistry registry = LabRegistry(); registry.AddRows(Read("data/design/command-rows-d02.json"));
        string text = Read("data/design/keymap.json");
        Keymap keymap = Keymap.Parse(text, registry);
        string[] own = seats.Held.Select(h => h.Key).Concat(seats.Retained.Select(r => r.Key)).ToArray();
        Equal(own.Distinct(StringComparer.Ordinal).Count(), SeatKeys.Seats + SeatKeys.RetainedKeys, "the seat and retained twist keys");
        foreach (string key in own)
        {
            Check(keymap.Bindings.All(b => b.Key != key), key + " is bound in the keymap as well");
            Check(!HostKeys.Contains(key), key + " is a host key of the sample");
            Check(!LiveTwistKeys.Contains(key), key + " twists the live session");
            Check(!(key.Length == 1 && char.IsDigit(key[0])), key + " is a digit, which acts in " + RepairListModel.Context);
        }
        var chords = new HashSet<string>(StringComparer.Ordinal);
        var typed = new HashSet<int>();
        for (int k = 1; k <= SeatKeys.Seats; k++)
            foreach (SeatTwistKey t in KeyMapModel.HeldSeat(seats, k))
            {
                Check(chords.Add(t.Chord), "the chord " + t.Chord + " twice");
                Check(typed.Add(t.Item), "two chords type item " + t.Item);
                Check(keymap.Bindings.All(b => b.Key != t.Chord), "the chord " + t.Chord + " is a keymap key");
            }
        Equal(chords.Count, SeatKeys.Seats * SeatKeys.PerSeat, "the chords");
        Check(typed.All(i => i > SeatKeys.RetainedKeys && i < SeatKeys.MenuItems), "a chord types a retained twist or no menu item");
        // The draft row the chords route through waits for CR-19.
        Check(Read("data/design/command-rows-d02.json").Contains("\"jumble.seat.hold\"", StringComparison.Ordinal), "the held seat row is not a draft row");

        Refuse(() => SeatKeys.Read(WithSeats(text, s => s["held"]![1]!["key"] = "Z")), "repeated seat key");
        Refuse(() => SeatKeys.Read(WithSeats(text, s => s["held"]![1]!["key"] = "I")), "is a retained twist key");
        Refuse(() => SeatKeys.Read(WithSeats(text, s => s["retained"]![1]!["key"] = "I")), "repeated retained twist key");
        Refuse(() => SeatKeys.Read(WithSeats(text, s => s["held"]![1]!["twists"]![0]!["item"] = 12)), "a jumble twist on two seats");
        Refuse(() => SeatKeys.Read(WithSeats(text, s => s["held"]![0]!["twists"]![2]!["with"] = "Comma")), "follow the retained twist keys");
        Refuse(() => SeatKeys.Read(WithSeats(text, s => s["held"]![0]!["key"] = "Ctrl+Z")), "not a plain key");
        Refuse(() => SeatKeys.Read(WithSeats(text, s => s["held"]![0]!["twists"]![0]!["order"] = 4)), "2, 3 or 5");
        // An override file carries no seats.
        Refuse(() => Keymap.Parse(text, registry, keymap), "unknown or repeated key");
        Check(SeatKeys.Read("{\"format\": \"magic600-design-keymap\", \"version\": 1, \"radials\": {}, \"bindings\": []}") == null, "a keymap without seats read some");
    }

    private static string Built(string id)
    {
        using JsonDocument words = JsonDocument.Parse(Read("data/design/jumble-words.json"));
        return words.RootElement.GetProperty("built").EnumerateArray().Single(b => b.GetProperty("id").GetString() == id).GetProperty("text").GetString()!;
    }

    private static string Sentence(string id)
    {
        using JsonDocument words = JsonDocument.Parse(Read("data/design/jumble-words.json"));
        return words.RootElement.GetProperty("sentences").EnumerateArray().Single(s => s.GetProperty("id").GetString() == id).GetProperty("text").GetString()!;
    }

    private static readonly JumbleState JumbleOn = new(true, true, null, 1, JumbleSessionKind.Research);

    // Global's border names its chain while Jumble is on: "projection chain (600-cell-Full)" by default, first of the
    // marks D02 adds there, and on Global only; off the lattice too, since the deep state is read through the tints.
    private static void D02GlobalNamesItsChain()
    {
        string built = Built("global.chain");
        Equal(built, "projection chain (600-cell-Full)", "Global's chain words");
        Equal(JumbleViews.ProjectionChain, built, "the model's chain words");
        Equal(JumbleViews.Chain(JumbleOn, "Global"), built, "Global's chain with Jumble on");
        Equal(JumbleViews.Chain(JumbleOn with { LatticeFlag = false }, "Global"), built, "Global's chain off the lattice");
        Equal(JumbleViews.Chain(JumbleOn, "Local"), (string?)null, "Local's chain");
        Equal(JumbleViews.Chain(JumbleState.Off(), "Global"), (string?)null, "Global's chain with Jumble off");
        IReadOnlyList<ViewMark> marks = JumbleViews.BorderMarks(JumbleOn, "Global");
        Check(marks.Count > 0 && marks[0].Words == built && marks[0].Glyph == JumbleViews.ChainGlyph, "Global's first D02 mark is not its chain");
        Check(JumbleViews.BorderMarks(JumbleOn, "Local").All(m => m.Words != built), "Local names Global's chain");
        Check(JumbleViews.BorderMarks(JumbleState.Off(), "Global").Count == 0, "a mark with Jumble off");
        Check(Read("data/design/icons.json").Contains("\"" + JumbleViews.ChainGlyph + "\"", StringComparison.Ordinal), "the chain's glyph is no icon");
    }

    // "exposed faces unavailable" on both views' borders and at the end of Tier 1 while Jumble is on, with its reason's
    // code, the frozen error table's KN2 refusal; nothing of it with Jumble off.
    private static void D02ExposedFacesUnavailableOnBorderAndTier1()
    {
        string sentence = Sentence("exposed.unavailable");
        Equal(JumbleViews.ExposedUnavailable, sentence, "the unavailable words");
        Equal(JumbleViews.ExposedReason, "input.exposed_geometry_unavailable", "the reason's code");
        Check(File.ReadAllText(Path.Combine(Root, "docs/architecture/1.0/language/architecture/13-errors.md")).Contains("KN2:" + JumbleViews.ExposedReason, StringComparison.Ordinal),
            "the reason is not the error table's code");
        Check(!JumbleViews.ExposedGeometry, "this build claims exposed-face geometry");
        using (JsonDocument system = JsonDocument.Parse(Read("data/design/system.json")))
        {
            string cue = system.RootElement.GetProperty("roles").GetProperty(JumbleViews.ExposedRole).GetProperty("cue").GetString()!;
            Check(cue.Contains(sentence, StringComparison.Ordinal), "the role's cue does not carry the words: " + cue);
        }
        foreach (string view in new[] { "Global", "Local" })
        {
            ViewMark mark = JumbleViews.BorderMarks(JumbleOn, view).Single(m => m.Words == sentence);
            Check(mark.Glyph == JumbleViews.ExposedGlyph && mark.Colour == JumbleViews.ExposedRole && mark.Tooltip.Contains(JumbleViews.ExposedReason, StringComparison.Ordinal),
                view + "'s mark reads " + mark);
            Check(JumbleViews.BorderMarks(JumbleState.Off(), view).Count == 0, view + " has a mark with Jumble off");
        }
        Equal(JumbleViews.Tier1("cap 30 · N_c 4", JumbleOn), "cap 30 · N_c 4 · " + sentence, "Tier 1 with Jumble on");
        Equal(JumbleViews.Tier1("cap 30 · N_c 4", JumbleState.Off()), "cap 30 · N_c 4", "Tier 1 with Jumble off");
        Equal(JumbleViews.Tier1("", JumbleOn), "", "an empty Tier 1");
        // The inspected cap's Tier 1 in Cap covers.
        var map = CapMapModel.Read(Read(CoversState + "rim.json"));
        var inspect = new CoversInspect();
        inspect.Hover(8);
        string off = inspect.Line(map);
        inspect.State = JumbleOn;
        Equal(inspect.Line(map), off + " · " + sentence, "the inspected cap's Tier 1 with Jumble on");
        Check(off.StartsWith("cap 8 · ", StringComparison.Ordinal) && !off.Contains(sentence, StringComparison.Ordinal), "the inspected cap's Tier 1 with Jumble off reads " + off);
        inspect.Hover(null);
        Equal(inspect.Line(map), "", "Tier 1 with nothing inspected");
    }
}
