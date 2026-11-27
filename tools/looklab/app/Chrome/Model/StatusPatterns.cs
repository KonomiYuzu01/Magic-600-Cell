using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Magic600.Chrome;

/// <summary>ADR-D01.65's scene cues, their priority and their byte indices. A mark is independent of the pattern.</summary>
public sealed class StatusPatterns
{
    public sealed record Entry(string Id, byte Code, string Shape, uint ShapeCode, string Colour, float Spacing, float Width, float Angle);
    public sealed record Fixture(string State, int Cap, IReadOnlyList<int[]> Statuses);

    private static readonly string[] PatternIds = { "protected", "conflict", "unchecked_prefix", "preview", "stale" };
    private static readonly string[] MarkIds = { "orbit", "position", "identity", "block" };
    private static readonly string[] PatternShapes = { "hatch", "cross_hatch", "dots", "dashed_edge", "stripes" };
    private static readonly string[] MarkShapes = { "ring", "dot", "cross", "corner_ticks" };
    // A piece carries one mark. A direct identity or position takes precedence over a block or an orbit.
    private static readonly int[] MarkOrder = { 0, 3, 1, 2 };
    public const string ProtectionRow = "protection.pick.set", SampleRow = "status.sample.show";
    public const byte Protected = 1, Conflict = 2, UncheckedPrefix = 3, Preview = 4, Stale = 5;
    // The marks are the rail's ProtectionKinds (Solving.cs): one type names the four kinds for the rail's locks and the
    // stickers. MarkCode gives a kind's code in channel 3's bits 3..5 (and the host's target kind); KindOf reads it back.
    public static byte MarkCode(ProtectionKinds kind) => kind switch
    {
        ProtectionKinds.Orbit => 1, ProtectionKinds.Position => 2, ProtectionKinds.Identity => 3, ProtectionKinds.Block => 4,
        _ => throw new ArgumentOutOfRangeException(nameof(kind), "a mark names exactly one protection kind"),
    };
    public static ProtectionKinds KindOf(byte code) => code switch
    {
        1 => ProtectionKinds.Orbit, 2 => ProtectionKinds.Position, 3 => ProtectionKinds.Identity, 4 => ProtectionKinds.Block,
        _ => ProtectionKinds.None,
    };

    private readonly int[] priority;
    public IReadOnlyList<Entry> Patterns { get; }
    public IReadOnlyList<Entry> Marks { get; }

    private StatusPatterns(Entry[] patterns, Entry[] marks, int[] priority)
    {
        Patterns = Array.AsReadOnly(patterns);
        Marks = Array.AsReadOnly(marks);
        this.priority = priority;
    }

    // These two D02 draft rows use labels on purpose, alongside the shared command icon manifest.
    public static string AddCommandLabels(string text)
    {
        var root = JsonNode.Parse(text)!;
        var labels = root["labelOnly"]!.AsArray();
        labels.Add(ProtectionRow);
        labels.Add(SampleRow);
        return root.ToJsonString();
    }

    public static StatusPatterns Parse(string text, TokenSet tokens)
    {
        using var file = JsonDocument.Parse(text);
        var root = file.RootElement;
        if (root.GetProperty("format").GetString() != "magic600-status-patterns/1")
            throw new InvalidDataException("unknown status pattern format");
        int[] priority = root.GetProperty("priority").EnumerateArray()
            .Select(e => Array.IndexOf(PatternIds, e.GetString())).ToArray();
        if (priority.Length != PatternIds.Length || priority[0] != Conflict - 1
            || priority.Contains(-1) || priority.Distinct().Count() != PatternIds.Length)
            throw new InvalidDataException("status priority needs each pattern once, with conflict first");
        return new StatusPatterns(
            ReadEntries(root.GetProperty("patterns"), PatternIds, PatternShapes, 2, 64, tokens),
            ReadEntries(root.GetProperty("marks"), MarkIds, MarkShapes, 0.05f, 0.45f, tokens), priority);
    }

    private static Entry[] ReadEntries(JsonElement table, string[] ids, string[] shapes, float min, float max, TokenSet tokens)
    {
        if (table.EnumerateObject().Count() != ids.Length)
            throw new InvalidDataException("the status table needs every entry once");
        var entries = new Entry[ids.Length];
        var used = new HashSet<int>();
        for (var i = 0; i < ids.Length; i++)
        {
            string id = ids[i];
            var row = table.GetProperty(id);
            string? shape = row.GetProperty("shape").GetString();
            int shapeIndex = Array.IndexOf(shapes, shape);
            if (shapeIndex < 0 || !used.Add(shapeIndex)) throw new InvalidDataException(id + ": unknown or repeated shape");
            string colour = row.GetProperty("colour").GetString() ?? "";
            if (!tokens.Colours.ContainsKey(colour)) throw new InvalidDataException(id + ": unknown colour token " + colour);
            entries[i] = new Entry(id, (byte)(i + 1), shapes[shapeIndex], (uint)(shapeIndex + 1), colour,
                Number(row, "spacing", min, max), Number(row, "width", 0.05f, 0.95f),
                Number(row, "angle", -MathF.PI, MathF.PI));
        }
        return entries;
    }

    private static float Number(JsonElement row, string key, float min, float max)
    {
        float value = row.GetProperty(key).GetSingle();
        if (!float.IsFinite(value) || value < min || value > max)
            throw new InvalidDataException("status geometry out of range: " + key);
        return value;
    }

    public static Fixture ReadFixture(string text, int pieceCount)
    {
        using var file = JsonDocument.Parse(text);
        var root = file.RootElement;
        if (root.GetProperty("format").GetString() != "magic600-status-fixture/1")
            throw new InvalidDataException("unknown status fixture format");
        string state = root.GetProperty("state").GetString() ?? "";
        int cap = root.GetProperty("cap").GetInt32();
        if (state.Length == 0 || cap < 0 || cap >= CorePlaces.Cells)
            throw new InvalidDataException("a status fixture needs a state and cap");
        var statuses = root.GetProperty("statuses");
        var sets = PatternIds.Select(id => statuses.GetProperty(id).EnumerateArray().Select(e => e.GetInt32()).ToArray()).ToArray();
        if (sets.Any(set => set.Length == 0 || set.Distinct().Count() != set.Length || set.Any(p => p < 0 || p >= pieceCount)))
            throw new InvalidDataException("a status fixture needs a nonempty list of distinct, valid piece IDs per pattern");
        return new Fixture(state, cap, sets);
    }

    // The core's channel 3: paint from lower to higher priority, preserving the non-overlapping bit fields.
    public byte[] Encode(int pieceCount, IReadOnlyList<int[]> patterns, IReadOnlyList<int[]> marks)
    {
        var bytes = new byte[pieceCount];
        foreach (int i in MarkOrder)
            foreach (int piece in marks[i]) bytes[piece] = (byte)((i + 1) << 3);
        foreach (int i in priority.Reverse())
            foreach (int piece in patterns[i]) bytes[piece] = (byte)((bytes[piece] & 0x38) | (i + 1));
        return bytes;
    }
}
