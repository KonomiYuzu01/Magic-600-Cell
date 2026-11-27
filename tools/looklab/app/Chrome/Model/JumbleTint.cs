using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Security.Cryptography;
using System.Text.Json;

namespace Magic600.Chrome;

// ADR-D02.20: the one per-pose tint channel. One field holds the mode, so modes never stack.
public enum TintMode { None, ContactClass, Height, OffLattice }

// The tint channel's mode, its style bytes and its legend (ADR-D02.20, .21, .23, .24; build step B15). Contact classes
// draw on the outline channel, so the fill keeps the sticker label; the height ramp draws on the fill; off-lattice is a
// dark outline read from the pose rows' exact lattice flag, never from heights.
public static class JumbleTint
{
    // The chrome setting that holds the researcher's mode. While it is unset, the entry slot's armed contact-class mode
    // is the mode (ADR-D02.12), else none.
    public const string ModeKey = "tint.mode";
    // The class memory's view-state text (ContactClasses.Save), kept beside the mode.
    public const string MemoryKey = "tint.contact_class.memory";
    // The palette row that switches the mode (CR-14's draft row).
    public const string Row = "jumble.tint.mode";
    public static readonly IReadOnlyList<string> Words = Array.AsReadOnly(new[] { "none", "contact", "height", "off-lattice" });

    public static string Word(TintMode mode) => Words[(int)mode];

    // The legend's title for a mode.
    public static string Title(TintMode mode) => mode switch
    {
        TintMode.ContactClass => "Contact classes",
        TintMode.Height => "Height",
        TintMode.OffLattice => "Off-lattice outline",
        _ => "No tint",
    };

    public static TintMode? Parse(string? text) => (text ?? "").Trim().ToLowerInvariant() switch
    {
        "none" or "off" => TintMode.None,
        "contact" or "contact-class" or "contact_class" or "contact class" or "classes" => TintMode.ContactClass,
        "height" or "heights" => TintMode.Height,
        "off-lattice" or "off_lattice" or "offlattice" or "off lattice" => TintMode.OffLattice,
        _ => null,
    };

    public static TintMode Mode(ChromeSettings settings) =>
        Parse(settings.Get(ModeKey)) ?? (settings.Get(EntrySlot.ContactClasses) == EntrySlot.Armed ? TintMode.ContactClass : TintMode.None);

    // The researcher's mode; the contact-class setting follows it, so the cap map's legend says what is on.
    public static void Set(ChromeSettings settings, TintMode mode)
    {
        settings.Set(ModeKey, Word(mode));
        settings.Set(EntrySlot.ContactClasses, mode == TintMode.ContactClass ? EntrySlot.Armed : null);
    }

    // The core's 16-entry style palette: 1 and 2 are the selection's and the hover's outlines (ShellStyle); 3 to 8 the
    // contact classes 1 to 6, 9 the overflow state, 10 to 13 the height steps 2, 4, 6 and above 6, 14 off-lattice.
    public const byte FirstClass = 3, OverflowEntry = 9, FirstHeight = 10, OffLatticeEntry = 14;

    // Each tinted palette entry's design slot. Off-lattice has no slot: it takes the theme's darkest neutral.
    public static readonly IReadOnlyList<(byte Entry, string Slot)> Slots = Array.AsReadOnly(new (byte, string)[]
    {
        (3, "tint.jumble.contact_class.1"), (4, "tint.jumble.contact_class.2"), (5, "tint.jumble.contact_class.3"),
        (6, "tint.jumble.contact_class.4"), (7, "tint.jumble.contact_class.5"), (8, "tint.jumble.contact_class.6"),
        (9, "tint.jumble.contact_class.overflow"), (10, "tint.jumble.height.2"), (11, "tint.jumble.height.4"),
        (12, "tint.jumble.height.6"), (13, "tint.jumble.height.above"),
    });
    // The off-lattice outline's two candidates: the darker of them is the theme's darkest neutral.
    public static readonly IReadOnlyList<string> OffLatticeCandidates = Array.AsReadOnly(new[] { "surface.window", "text.primary" });

    // The height ramp's opacity on the fill, so the sticker's label still shows through it.
    public const double HeightAlpha = 0.6;

    // The tinted palette entries in the theme's colours: the classes and the height steps from their design slots, the
    // off-lattice outline in the darker of its two candidates.
    public static IReadOnlyList<(byte Entry, Rgb Colour, double Alpha)> Palette(TokenSet tokens)
    {
        var entries = Slots.Select(s => (s.Entry, tokens.Colour(s.Slot), s.Entry >= FirstHeight ? HeightAlpha : 1.0)).ToList();
        entries.Add((OffLatticeEntry, OffLatticeColour(tokens), 1.0));
        return entries;
    }

    public static Rgb OffLatticeColour(TokenSet tokens) => OffLatticeCandidates.Select(tokens.Colour).OrderBy(Oklch.Luminance).First();

    public static byte ClassEntry(int c) => c is >= 1 and <= ContactClasses.Count ? (byte)(FirstClass + c - 1) : c == ContactClasses.Overflow ? OverflowEntry : (byte)0;

    // The ramp's step for a height: 2, 4, 6, or the overflow step above 6; height 0 is untinted.
    public static byte HeightEntry(int height) => height <= 0 ? (byte)0 : height <= 2 ? FirstHeight : height <= 4 ? (byte)(FirstHeight + 1)
        : height <= 6 ? (byte)(FirstHeight + 2) : (byte)(FirstHeight + 3);

    // Per pose row (the style rows' order): the outline byte and the fill byte the mode draws. Contact classes need the
    // domains' classes (index = the style's domain index); without them nothing is drawn.
    public static (byte[] Outline, byte[] Fill) PerPose(TintMode mode, PoseStyle style, IReadOnlyList<int>? classes)
    {
        var outline = new byte[style.RowCount];
        var fill = new byte[style.RowCount];
        for (int i = 0; i < style.RowCount; i++)
        {
            switch (mode)
            {
                case TintMode.ContactClass when classes != null:
                    outline[i] = ClassEntry(classes[style.RowDomain(i)]);
                    break;
                case TintMode.Height:
                    fill[i] = HeightEntry(style.RowHeight(i));
                    break;
                case TintMode.OffLattice:
                    outline[i] = style.RowOnLattice(i) ? (byte)0 : OffLatticeEntry;
                    break;
            }
        }
        return (outline, fill);
    }

    // Per piece: each piece takes its pose row's byte.
    public static byte[] PerPiece(byte[] perPose, IReadOnlyList<int> poseOf)
    {
        var bytes = new byte[poseOf.Count];
        for (int p = 0; p < bytes.Length; p++) bytes[p] = perPose[poseOf[p]];
        return bytes;
    }

    // The legend of a mode with its piece counts (the counts the pose rows give), for the view borders.
    public static IReadOnlyList<TintLegendEntry> Legend(TintMode mode, PoseStyle style, IReadOnlyList<int>? classes)
    {
        var entries = new List<TintLegendEntry>();
        long Count(Func<int, bool> row) => Enumerable.Range(0, style.RowCount).Where(row).Sum(style.RowPieces);
        switch (mode)
        {
            case TintMode.ContactClass when classes != null:
                for (int c = 1; c <= ContactClasses.Count; c++)
                {
                    int cls = c;
                    int domains = classes.Count(x => x == cls);
                    if (domains > 0) entries.Add(new TintLegendEntry(Slots[c - 1].Slot, cls.ToString(CultureInfo.InvariantCulture), Count(i => classes[style.RowDomain(i)] == cls), domains));
                }
                int over = classes.Count(x => x == ContactClasses.Overflow);
                if (over > 0) entries.Add(new TintLegendEntry("tint.jumble.contact_class.overflow", "overflow", Count(i => classes[style.RowDomain(i)] == ContactClasses.Overflow), over));
                if (style.LatticeDomain >= 0) entries.Add(new TintLegendEntry(null, "lattice neutral", style.InDomain(style.LatticeDomain), 1));
                break;
            case TintMode.Height:
                string[] slots = { "tint.jumble.height.2", "tint.jumble.height.4", "tint.jumble.height.6", "tint.jumble.height.above" };
                int largest = Enumerable.Range(0, style.RowCount).Select(style.RowHeight).DefaultIfEmpty(0).Max();
                for (int s = 0; s < 4; s++)
                {
                    byte entry = (byte)(FirstHeight + s);
                    string label = s < 3 ? (2 * s + 2).ToString(CultureInfo.InvariantCulture) : "above 6" + (largest > 6 ? " (largest " + largest.ToString(CultureInfo.InvariantCulture) + ")" : "");
                    entries.Add(new TintLegendEntry(slots[s], label, Count(i => HeightEntry(style.RowHeight(i)) == entry), 0));
                }
                break;
            case TintMode.OffLattice:
                entries.Add(new TintLegendEntry(null, "off-lattice", Count(i => !style.RowOnLattice(i)), 0));
                break;
        }
        return entries;
    }

    // SF5 (27 Nov 2026): the widest numbers a legend entry keeps room for (259,800 slots; 600 caps), so a count that
    // changes at a twist never widens its entry and never rewraps the row on the view's border.
    public const string WidestCount = "000,000";
    public const string WidestDomains = "000";

    // An entry's words on the border as parts: fixed words, and each number with the widest form its slot keeps room for.
    public static IReadOnlyList<LegendPart> EntryParts(TintLegendEntry entry)
    {
        static string N(long n) => n.ToString("N0", CultureInfo.InvariantCulture);
        return entry.Label == "overflow"
            ? new[] { new LegendPart("overflow ·", null), new LegendPart(N(entry.Domains), WidestDomains), new LegendPart("domains ·", null), new LegendPart(N(entry.Pieces), WidestCount) }
            : new[] { new LegendPart(entry.Label + " ·", null), new LegendPart(N(entry.Pieces), WidestCount) };
    }

    // The row's form with every number at its widest: the row's height follows only this, so two states whose forms
    // are equal give the border, and the view's opening, the same height.
    public static string ReservedForm(TintMode mode, IReadOnlyList<TintLegendEntry> entries) =>
        Title(mode) + " | " + string.Join(" | ", entries.Select(e => string.Join(" ", EntryParts(e).Select(p => p.Widest ?? p.Text))));

    // Whether a shown legend row stays on the border: while a mode is drawn it keeps its place (and its height) even
    // when no state is bound, as after a twist to a state the samples do not hold; only no mode removes it.
    public static bool LegendKept(TintMode mode, bool shown, string line) => BorderRowKept(mode != TintMode.None, shown, line);

    // A row on a view's border with words stays; one that is shown keeps its place blank while what it reads is still
    // asked for (a tint mode on, a cap named); a row asked for nothing goes, so only a choice changes the opening.
    public static bool BorderRowKept(bool asked, bool shown, string words) => words.Length > 0 || (shown && asked);

    // The legend as one line: the mode's name, then each entry with its count of pieces.
    public static string Line(TintMode mode, IReadOnlyList<TintLegendEntry> entries, bool graphed = true)
    {
        static string N(long n) => n.ToString("N0", CultureInfo.InvariantCulture);
        return mode switch
        {
            TintMode.None => "",
            TintMode.ContactClass when !graphed => "Contact classes · this state has no contact graph",
            TintMode.ContactClass when entries.All(e => e.Slot == null) => "Contact classes · nothing to draw until the first jumble twist",
            TintMode.ContactClass => "Contact classes · " + string.Join(" · ", entries.Select(e =>
                e.Label == "overflow" ? "overflow " + N(e.Domains) + (e.Domains == 1 ? " domain " : " domains ") + N(e.Pieces) : e.Label + " " + N(e.Pieces))),
            TintMode.Height => "Height · " + string.Join(" · ", entries.Select(e => e.Label + " " + N(e.Pieces))),
            _ => "Off-lattice outline · " + N(entries.Sum(e => e.Pieces)) + " pieces",
        };
    }

    // A piece set as `bits-le-zlib-base64` (piece i is bit i % 8 of byte i / 8), checked against its count and digest.
    public static bool[] Mask(JsonElement mask, int pieces)
    {
        byte[] raw = Inflate(mask.GetProperty("data").GetString()!);
        if (raw.Length != (pieces + 7) / 8) throw new FormatException("a piece mask of " + raw.Length + " bytes");
        if (mask.TryGetProperty("sha256", out JsonElement sha) && Convert.ToHexString(SHA256.HashData(raw)).ToLowerInvariant() != sha.GetString()) throw new FormatException("a piece mask's digest differs");
        var set = new bool[pieces];
        long count = 0;
        for (int p = 0; p < pieces; p++) if ((raw[p >> 3] & (1 << (p & 7))) != 0) { set[p] = true; count++; }
        if (mask.TryGetProperty("count", out JsonElement n) && n.GetInt64() != count) throw new FormatException("a piece mask counts " + count + ", its record " + n.GetInt64());
        return set;
    }

    // A state's pose ids (poses.json): each piece's pose row, u32le, checked against the sha256 the state's record names.
    public static int[] Poses(string text, int rows)
    {
        using JsonDocument document = JsonDocument.Parse(text);
        JsonElement root = document.RootElement;
        byte[] raw = Inflate(root.GetProperty("data").GetString()!);
        int count = root.GetProperty("count").GetInt32();
        if (raw.Length != 4 * count) throw new FormatException("poses: " + raw.Length + " bytes for " + count + " pieces");
        if (Convert.ToHexString(SHA256.HashData(raw)).ToLowerInvariant() != root.GetProperty("sha256").GetString()) throw new FormatException("poses: the pose ids' digest differs");
        var ids = new int[count];
        for (int p = 0; p < count; p++)
        {
            uint id = BitConverter.ToUInt32(raw, 4 * p);
            if (!BitConverter.IsLittleEndian) id = System.Buffers.Binary.BinaryPrimitives.ReverseEndianness(id);
            if (id >= (uint)rows) throw new FormatException("poses: piece " + p + " names pose row " + id + " of " + rows);
            ids[p] = (int)id;
        }
        return ids;
    }

    private static byte[] Inflate(string base64)
    {
        using var input = new MemoryStream(Convert.FromBase64String(base64));
        using var zlib = new ZLibStream(input, CompressionMode.Decompress);
        using var output = new MemoryStream();
        zlib.CopyTo(output);
        return output.ToArray();
    }
}

// One legend entry: its design slot (none for the neutral lattice and the off-lattice outline), its label, the pieces
// it covers and, for contact classes, the domains that hold it.
public sealed record TintLegendEntry(string? Slot, string Label, long Pieces, int Domains);

// One part of a legend entry's words: fixed words (no widest form) or a number in a slot as wide as its widest form.
public sealed record LegendPart(string Text, string? Widest);

// The domain contact graph of a bound state (contact.json; ADR-D02.21): domains by the style's domain index, the
// lattice domain, and the edges with whether each contact lies on an on-grip cut, an off-grip cut or both.
public sealed class ContactGraph
{
    public int Domains { get; }
    public int? Lattice { get; }
    public string Digest { get; }
    public IReadOnlyList<(int A, int B, bool OnGrip, bool OffGrip)> Edges { get; }
    // The sample builder's first assignment (the one an empty memory gives), for the checks.
    public IReadOnlyList<int> SampleClasses { get; }
    private readonly List<int>[] adjacent;

    public ContactGraph(int domains, int? lattice, IEnumerable<(int A, int B, bool OnGrip, bool OffGrip)> edges, string digest = "", IReadOnlyList<int>? sample = null)
    {
        Domains = domains; Lattice = lattice; Digest = digest;
        Edges = edges.ToArray();
        SampleClasses = sample ?? Array.Empty<int>();
        adjacent = Enumerable.Range(0, domains).Select(_ => new List<int>()).ToArray();
        foreach (var (a, b, _, _) in Edges)
        {
            if (a < 0 || b < 0 || a >= domains || b >= domains || a == b) throw new FormatException("contact: an edge outside the domains");
            adjacent[a].Add(b); adjacent[b].Add(a);
        }
    }

    public static ContactGraph Read(string text)
    {
        using JsonDocument document = JsonDocument.Parse(text);
        JsonElement root = document.RootElement;
        JsonElement lattice = root.GetProperty("lattice_domain");
        var edges = root.GetProperty("edges").EnumerateArray()
            .Select(e => (e[0].GetInt32(), e[1].GetInt32(), e[2].GetBoolean(), e[3].GetBoolean())).ToArray();
        int[] sample = root.TryGetProperty("classes", out JsonElement classes) ? classes.EnumerateArray().Select(c => c.GetInt32()).ToArray() : Array.Empty<int>();
        return new ContactGraph(root.GetProperty("domains").GetInt32(), lattice.ValueKind == JsonValueKind.Number ? lattice.GetInt32() : null, edges,
            root.TryGetProperty("digest", out JsonElement digest) ? digest.GetString() ?? "" : "", sample);
    }

    public IReadOnlyList<int> Neighbours(int domain) => adjacent[domain];
}

// ADR-D02.21's class rule: a proper colouring of the contact graph's off-lattice domains with six classes, the lattice
// domain neutral, deterministic and sticky by the domain's JI.53 id. The memory holds the class each domain id was
// first given and is saved with the view state, so a state seen again (an undo, a redo, a branch switch, a reopen)
// draws as it did; a remembered class stays while no touching domain, in index order, already holds it. The rest are
// coloured by DSatur (the most distinct classes among coloured neighbours, then the most neighbours, then the lowest
// index), each taking the lowest class its neighbours leave, or the overflow state when all six are taken. Overflow is
// never remembered.
public sealed class ContactClasses
{
    public const int Neutral = 0, Count = 6, Overflow = 7;
    private readonly SortedDictionary<string, int> memory = new(StringComparer.Ordinal);
    public IReadOnlyDictionary<string, int> Memory => memory;
    // The domains whose remembered class a touching domain had already kept at the last assignment (they moved).
    public IReadOnlyList<int> Moved { get; private set; } = Array.Empty<int>();
    public event Action? Changed;

    // Assigns the classes of a state's domains (keys: the style's domain ids, by index) and remembers them.
    public int[] Assign(ContactGraph contact, IReadOnlyList<string> keys)
    {
        var (classes, moved) = Assign(contact, keys, memory);
        for (int d = 0; d < classes.Length; d++)
            if (classes[d] is >= 1 and <= Count) memory.TryAdd(keys[d], classes[d]);
        Moved = moved;
        Safe.Raise(Changed);
        return classes;
    }

    public static (int[] Classes, int[] Moved) Assign(ContactGraph contact, IReadOnlyList<string> keys, IReadOnlyDictionary<string, int>? previous)
    {
        int n = contact.Domains;
        if (keys.Count != n) throw new ArgumentException("contact: " + keys.Count + " domain ids for " + n + " domains");
        int lattice = contact.Lattice ?? -1;
        int[] Near(int d) => d == lattice ? Array.Empty<int>() : contact.Neighbours(d).Where(x => x != lattice).ToArray();
        int[][] near = Enumerable.Range(0, n).Select(Near).ToArray();
        var classes = new int[n];
        var moved = new List<int>();
        if (previous != null)
            for (int d = 0; d < n; d++)
            {
                if (d == lattice || !previous.TryGetValue(keys[d], out int c) || c is < 1 or > Count) continue;
                if (near[d].Any(x => classes[x] == c)) moved.Add(d); else classes[d] = c;
            }
        var todo = Enumerable.Range(0, n).Where(d => d != lattice && classes[d] == 0).ToList();
        while (todo.Count > 0)
        {
            int Saturation(int d) => near[d].Select(x => classes[x]).Where(c => c != 0).Distinct().Count();
            int next = todo.OrderByDescending(Saturation).ThenByDescending(d => near[d].Length).ThenBy(d => d).First();
            var taken = near[next].Select(x => classes[x]).ToHashSet();
            classes[next] = Enumerable.Range(1, Count).Where(c => !taken.Contains(c)).DefaultIfEmpty(Overflow).First();
            todo.Remove(next);
        }
        return (classes, moved.ToArray());
    }

    // The memory as view-state text, and back (a reopen).
    public string Save() => JsonSerializer.Serialize(memory);

    public static ContactClasses Load(string text)
    {
        var loaded = new ContactClasses();
        foreach (var (key, value) in JsonSerializer.Deserialize<Dictionary<string, int>>(text) ?? new Dictionary<string, int>())
            if (value is >= 1 and <= Count) loaded.memory[key] = value;
        return loaded;
    }
}
