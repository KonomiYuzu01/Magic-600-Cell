using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Security.Cryptography;
using System.Text.Json;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

// ADR-D01.65: one shared, view-only byte per piece. The keys bind protection to a current B17 pick.
internal static class ShellStatus
{
    private const string Protect = StatusPatterns.ProtectionRow;
    private const string Sample = StatusPatterns.SampleRow;
    private const int Pieces = CorePlaces.Pieces;
    private const int Words = (Pieces + 31) / 32;

    private sealed record PieceBytes(ulong Revision, byte[] Bytes, string[] Sources)
    {
        public bool Empty { get; } = !Bytes.Any(b => b != 0);
    }

    private static readonly HashSet<int>[] Marks = { new(), new(), new(), new() };
    private static readonly HashSet<uint> Positions = new();
    private static HashSet<int> conflicts = new();
    private static readonly byte[] EmptyBytes = new byte[Pieces];
    private static ChromeFrame? chrome;
    private static StatusPatterns? table;
    private static StatusPatterns.Fixture? fixture, activeFixture;
    private static HostLibrary? library;
    private static IntPtr host;
    private static HostLibrary.Submitted lastFrame;
    private static PieceBytes? prepared, sent, drawn;
    private static TokenSet? sentTokens, drawnTokens;
    private static byte[]? moving;
    private static bool collect, sampleOn, dirty;
    private static ulong? positionsAt;
    private static int commandCode;
    private static string command = "";

    public static void Load(ChromeFrame frame, string folder)
    {
        table = StatusPatterns.Parse(File.ReadAllText(Path.Combine(folder, "status-patterns.json")), frame.Tokens);
        chrome = frame;
        collect = !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("M600_D02_SCRIPT"));
        foreach (var pieces in Marks) pieces.Clear();
        Positions.Clear();
        conflicts.Clear();
        positionsAt = null;
        fixture = activeFixture = null;
        prepared = sent = drawn = null;
        (preparedHeld, sentHeld, heldKey, heldPositions) = (null, null, null, null);
        sentTokens = drawnTokens = null;
        lastFrame = default;
        moving = null;
        library = null;
        command = "";
        sampleOn = false;
        dirty = true;
    }

    public static bool Handles(string row) => row is Protect or Sample;
    public static bool Conflict(int piece) => conflicts.Contains(piece);

    public static void Execute(string row, string? argument)
    {
        command = row + " " + argument;
        commandCode = HostLibrary.InvalidArgument;
        if (row == Sample)
        {
            if (argument is not ("on" or "off")) return;
            if (argument == "on")
                fixture = StatusPatterns.ReadFixture(File.ReadAllText(Path.Combine(ShellJumble.JumbleFolder(), "status-fixture.json")), Pieces);
            sampleOn = argument == "on";
        }
        else if (argument == "clear")
        {
            foreach (var pieces in Marks) pieces.Clear();
            Positions.Clear();
            positionsAt = null;
        }
        else
        {
            if (table is null || library is null || lastFrame.Sequence == 0 || ShellPick.Selection is not { } picked
                || picked.Revision != lastFrame.EngineRevision)
            {
                commandCode = HostLibrary.Unavailable;
                return;
            }
            var entry = table.Marks.FirstOrDefault(m => m.Id == argument);
            if (entry is null) return;

            var pieces = Marks[entry.Code - 1];
            if (StatusPatterns.KindOf(entry.Code) == ProtectionKinds.Identity)
                pieces.Add((int)picked.Piece);
            else
            {
                uint? target = StatusPatterns.KindOf(entry.Code) switch
                {
                    ProtectionKinds.Orbit => picked.Piece,
                    ProtectionKinds.Position => picked.Slot,
                    ProtectionKinds.Block => picked.Cell is int cell ? (uint)cell : null,
                    _ => null,
                };
                if (target is not uint id) return;

                var mask = new byte[Pieces];
                commandCode = library.StatusTargets(host, picked.Revision, entry.Code, new[] { id }, mask);
                if (commandCode != HostLibrary.Ok) return;
                if (StatusPatterns.KindOf(entry.Code) == ProtectionKinds.Orbit && !mask.Contains((byte)1))
                {
                    commandCode = HostLibrary.Unavailable; // a fixed piece has no orbit
                    return;
                }
                pieces.UnionWith(PieceIds(mask));
                if (StatusPatterns.KindOf(entry.Code) == ProtectionKinds.Position)
                {
                    if (Positions.Count == 0) positionsAt = picked.Revision;
                    Positions.Add(id);
                }
            }
        }
        commandCode = HostLibrary.Ok;
        dirty = true;
    }

    private static IEnumerable<int> PieceIds(byte[] bytes) => Enumerable.Range(0, bytes.Length).Where(i => bytes[i] != 0);

    // Runs before the focus/fill send. The turn row is a preview only while the slice holds it at this revision.
    public static void Prepare(HostLibrary bridge, IntPtr handle, ulong revision, bool sliceReady)
    {
        if (table is null) return;
        PrepareAdopted(bridge, handle, revision, sliceReady);
        PreparePending(bridge, handle, revision, sliceReady);
    }

    private static void PrepareAdopted(HostLibrary bridge, IntPtr handle, ulong revision, bool sliceReady)
    {
        (library, host) = (bridge, handle);
        var preview = sliceReady && ShellSlice.Model.Previewing is { } held && ShellTurn.Cap == held.Cap
            ? ShellTurn.InsideFor(ShellTurn.Row, revision) : null;
        var sample = sampleOn && fixture is { } f && f.State == ShellJumble.Bound ? f : null;
        if (!dirty && prepared?.Revision == revision && moving == preview && activeFixture == sample) return;

        if (Positions.Count != 0 && positionsAt != revision)
        {
            var mask = new byte[Pieces];
            // LF1: the position marks the pending send already resolved on this revision's head are reused.
            if (heldPositions is { } early && early.Revision == revision && early.Positions == Positions.Count)
                mask = early.Mask;
            else if (bridge.StatusTargets(handle, revision, StatusPatterns.MarkCode(ProtectionKinds.Position), Positions.ToArray(), mask) != HostLibrary.Ok) return;
            Marks[StatusPatterns.MarkCode(ProtectionKinds.Position) - 1].Clear();
            Marks[StatusPatterns.MarkCode(ProtectionKinds.Position) - 1].UnionWith(PieceIds(mask));
            positionsAt = revision;
        }

        var (bytes, sources, nextConflicts) = Encode(Marks, preview, sample);
        if (!conflicts.SetEquals(nextConflicts))
        {
            conflicts = nextConflicts;
            ShellTint.RefreshFocus();
        }

        prepared = new PieceBytes(revision, bytes, sources);
        (moving, activeFixture, dirty) = (preview, sample, false);
        stamp++;
    }

    // LF1 (checkpoint C3/C4's F1 for this channel): after a committed press, the bytes for the revision handed out too,
    // with the position marks resolved on that head (m6h_status_targets reads it before its adoption) and the turn
    // row's preview read there (ShellTurn.PendingRow), so Apply sends them for that revision and the host holds them
    // for its adoption: the revision's first frame never draws the old revision's statuses on the new poses.
    private static PieceBytes? preparedHeld, sentHeld;
    private static (ulong Revision, int Stamp, byte[]? Preview)? heldKey;
    private static (ulong Revision, int Positions, byte[] Mask)? heldPositions;
    private static int stamp;

    private static void PreparePending(HostLibrary bridge, IntPtr handle, ulong revision, bool sliceReady)
    {
        var pending = ShellStyle.PendingAfter(revision);
        if (pending == 0 || prepared is null)
        {
            (preparedHeld, heldKey) = (null, null);
            return;
        }
        var preview = sliceReady && ShellSlice.Model.Previewing is { } held && ShellTurn.Cap == held.Cap
            ? ShellTurn.InsideFor(ShellTurn.PendingRow, pending) : null;
        if (heldKey is { } key && key.Revision == pending && key.Stamp == stamp && key.Preview == preview) return;
        // Tried once per pending revision, marks and preview: a refusal keeps the adopted-only send, never a retry storm.
        (heldKey, preparedHeld) = ((pending, stamp, preview), null);
        var marks = Marks.Select(set => new HashSet<int>(set)).ToArray();
        if (Positions.Count != 0)
        {
            var mask = new byte[Pieces];
            if (bridge.StatusTargets(handle, pending, StatusPatterns.MarkCode(ProtectionKinds.Position), Positions.ToArray(), mask) != HostLibrary.Ok) return;
            heldPositions = (pending, Positions.Count, mask);
            marks[StatusPatterns.MarkCode(ProtectionKinds.Position) - 1] = PieceIds(mask).ToHashSet();
        }
        var (bytes, sources, _) = Encode(marks, preview, activeFixture);
        preparedHeld = new PieceBytes(pending, bytes, sources);
    }

    // The status bytes of one revision from its marks, the turn row's preview and the sample fixture, with the
    // conflict set (the protected pieces the preview moves).
    private static (byte[] Bytes, string[] Sources, HashSet<int> Conflicts) Encode(HashSet<int>[] marks, byte[]? preview, StatusPatterns.Fixture? sample)
    {
        var protectedPieces = marks.SelectMany(set => set).ToHashSet();
        var statuses = new int[table.Patterns.Count][];
        var sources = new string[statuses.Length];
        void Use(byte status, int[]? fed, bool hasFeed)
        {
            bool fromFixture = fed is null && (sample is not null || !hasFeed);
            int i = status - 1;
            statuses[i] = fed ?? sample?.Statuses[i] ?? Array.Empty<int>();
            sources[i] = fromFixture ? "fixture" : "fed";
        }
        Use(StatusPatterns.Protected, protectedPieces.Count > 0 ? protectedPieces.ToArray() : null, true);
        Use(StatusPatterns.Conflict, preview is null ? null : protectedPieces.Where(p => preview[p] != 0).ToArray(), true);
        Use(StatusPatterns.UncheckedPrefix, null, false);
        Use(StatusPatterns.Preview, preview is null ? null : PieceIds(preview).ToArray(), true);
        Use(StatusPatterns.Stale, null, false);

        var nextConflicts = statuses[StatusPatterns.Conflict - 1].ToHashSet();
        return (table!.Encode(Pieces, statuses, marks.Select(set => set.ToArray()).ToArray()), sources, nextConflicts);
    }

    // After the other channels, for the adopted revision and (LF1) the one a committed press handed out. Token changes
    // resend the independent table.
    public static void Apply(HostLibrary bridge, IntPtr handle, ulong revision)
    {
        if (table is null || chrome is null) return;
        if (sentTokens != chrome.Tokens && bridge.SetStatusStyle(handle, table, chrome.Tokens, collect) == HostLibrary.Ok)
            sentTokens = chrome.Tokens;
        if (prepared is { } next && next.Revision == revision && next != sent
            && bridge.SetPieceStyle(handle, revision, HostLibrary.StyleStatus, next.Empty ? null : next.Bytes, false) == HostLibrary.Ok)
            sent = next;
        if (preparedHeld is { } held && held != sentHeld
            && bridge.SetPieceStyle(handle, held.Revision, HostLibrary.StyleStatus, held.Empty ? null : held.Bytes, false) == HostLibrary.Ok)
            sentHeld = held;
    }

    public static void Framed(HostLibrary.Submitted frame)
    {
        if (table is null || frame.Held != 0) return;
        lastFrame = frame;
        drawn = sent?.Revision == frame.EngineRevision ? sent : sentHeld?.Revision == frame.EngineRevision ? sentHeld : null;
        drawnTokens = sentTokens;
    }

    public static void Dump(Utf8JsonWriter json)
    {
        if (!collect || table is null || library is null) return;
        var bytes = drawn?.Bytes ?? EmptyBytes;
        var global = new uint[Words];
        var local = new uint[Words];
        int readCode = HostLibrary.Ok;
        if (drawn is { Empty: false } && drawnTokens is not null)
        {
            readCode = library.StatusDrawn(host, 0, global);
            var pane = new uint[Words];
            for (uint i = 1; readCode == HostLibrary.Ok && i < lastFrame.Panes; i++)
            {
                readCode = library.StatusDrawn(host, i, pane);
                for (var word = 0; word < Words; word++) local[word] |= pane[word];
            }
        }

        json.WriteStartObject("status_patterns");
        json.WriteString("command", command);
        json.WriteNumber("command_code", commandCode);
        json.WriteBoolean("sample", sampleOn);
        json.WriteString("fixture_state", activeFixture?.State ?? "");
        json.WriteString("state_digest", ShellJumble.StateDigest);
        json.WriteNumber("revision", lastFrame.EngineRevision);
        json.WriteNumber("panes", lastFrame.Panes);
        json.WriteNumber("channel", HostLibrary.StyleStatus);
        json.WriteNumber("draw_code", readCode);
        json.WriteBoolean("sent", drawn is not null && drawnTokens is not null);
        json.WriteString("bytes_sha256", Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant());
        using (var packed = new MemoryStream())
        {
            using (var zip = new ZLibStream(packed, CompressionLevel.Fastest, leaveOpen: true)) zip.Write(bytes);
            json.WriteBase64String("bytes_zlib", packed.ToArray());
        }
        WriteEntries("statuses", table.Patterns, 0, drawn?.Sources);
        WriteEntries("kinds", table.Marks, 3, null);
        json.WriteEndObject();

        void WriteEntries(string name, IReadOnlyList<StatusPatterns.Entry> entries, int shift, string[]? sources)
        {
            json.WriteStartObject(name);
            foreach (var entry in entries)
            {
                var ids = Enumerable.Range(0, Pieces).Where(p => ((bytes[p] >> shift) & 7) == entry.Code).ToArray();
                json.WriteStartObject(entry.Id);
                json.WriteString("source", sources?[entry.Code - 1]
                    ?? (shift == 0 && (entry.Code is StatusPatterns.UncheckedPrefix or StatusPatterns.Stale) ? "fixture" : "fed"));
                json.WriteNumber("code", entry.Code);
                json.WriteNumber("count", ids.Length);
                json.WriteNumber("global", ids.Count(p => (global[p / 32] & (1u << (p % 32))) != 0));
                json.WriteNumber("local", ids.Count(p => (local[p / 32] & (1u << (p % 32))) != 0));
                json.WriteStartArray("pieces");
                foreach (int piece in ids.Take(32)) json.WriteNumberValue(piece);
                json.WriteEndArray();
                if (drawnTokens is { } tokens)
                {
                    json.WriteStartObject("pattern");
                    json.WriteString("shape", entry.Shape);
                    json.WriteNumber("shape_code", entry.ShapeCode);
                    json.WriteNumber("spacing", entry.Spacing);
                    json.WriteNumber("width", entry.Width);
                    json.WriteNumber("angle", entry.Angle);
                    json.WriteString("token", entry.Colour);
                    var colour = tokens.Colour(entry.Colour);
                    json.WriteStartArray("rgba");
                    json.WriteNumberValue((float)colour.R);
                    json.WriteNumberValue((float)colour.G);
                    json.WriteNumberValue((float)colour.B);
                    json.WriteNumberValue(1);
                    json.WriteEndArray();
                    json.WriteEndObject();
                }
                json.WriteEndObject();
            }
            json.WriteEndObject();
        }
    }
}
