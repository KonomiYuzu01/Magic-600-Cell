using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Magic600.Chrome;

public enum DocumentKind { Session, Log, Script }
// In order: empty ring, solid, solid with an outer ring.
public enum CompileLamp { NotCompiled, Clean, Problems }

public static class CompileLampWords
{
    public static string Word(this CompileLamp lamp) => lamp switch
    {
        CompileLamp.NotCompiled => "not compiled",
        CompileLamp.Clean => "compiled clean",
        CompileLamp.Problems => "compiled with problems",
        _ => throw new ArgumentOutOfRangeException(nameof(lamp)),
    };
}

public sealed record LogGate(string Name, bool Passed, string Detail);
public sealed record LogFacts(int Moves, string Time, string ModelIdentity, SolvingState Saved);
public sealed record CompileResult(int Number, string Digest, IReadOnlyList<string> Problems, IReadOnlyList<LogGate> Gates, LogFacts? Facts)
{
    public bool Passed => Problems.Count == 0 && Gates.All(gate => gate.Passed);
}

public interface IDocumentStore
{
    byte[]? Read(string path);
    void Write(string path, byte[] bytes);
}

public sealed class MemoryDocumentStore : IDocumentStore
{
    private readonly Dictionary<string, byte[]> files = new(StringComparer.OrdinalIgnoreCase);
    public int Writes { get; private set; }
    public byte[]? Read(string path) => files.TryGetValue(path, out byte[]? bytes) ? bytes.ToArray() : null;
    public void Write(string path, byte[] bytes)
    {
        files[path] = bytes.ToArray();
        Writes++;
    }
}

public interface IDocumentCompiler
{
    CompileResult Script(int number, string digest, string text);
    CompileResult Log(int number, string digest, string text);
}

public sealed class SampleDocumentCompiler(string modelIdentity, IReadOnlyList<SolvingState> states) : IDocumentCompiler
{
    public CompileResult Script(int number, string digest, string text) => new(number, digest,
        SixCs.Diagnose(text).Where(d => d.Severity == Severity.Error)
            .Select(d => FormattableString.Invariant($"line {d.Line} · {d.Text}")).ToArray(),
        Array.Empty<LogGate>(), null);

    public CompileResult Log(int number, string digest, string text)
    {
        CompileResult BadFormat(string detail) => new(number, digest, Array.Empty<string>(), new[] { new LogGate("format", false, detail) }, null);
        try
        {
            using JsonDocument json = JsonDocument.Parse(text);
            JsonElement log = json.RootElement;
            if (log.ValueKind != JsonValueKind.Object || Value(log, "format") != SampleDocuments.LogFormat)
                return BadFormat("expected the " + SampleDocuments.LogFormat + " format");
            string? stateId = Value(log, "state");
            SolvingState? saved = states.FirstOrDefault(state => state.Id == stateId);
            if (saved == null) return BadFormat("the sample log names an unknown state");
            string? time = Value(log, "time");
            if (time == null || !Regex.IsMatch(time, @"\A[0-9]{2}:[0-5][0-9]:[0-5][0-9]\z"))
                return BadFormat("the sample log needs a time in hh:mm:ss");

            bool matches = Value(log, "model") == modelIdentity;
            string[]? moves = null;
            if (log.TryGetProperty("moves", out JsonElement list) && list.ValueKind == JsonValueKind.Array &&
                list.EnumerateArray().All(move => move.ValueKind == JsonValueKind.String))
                moves = list.EnumerateArray().Select(move => move.GetString()!).ToArray();
            bool hashMatches = moves != null && string.Equals(SampleDocuments.LogHash(saved.Id, moves), Value(log, "hash"),
                StringComparison.OrdinalIgnoreCase);
            var gates = new[]
            {
                new LogGate("format", true, "sample log format and state recognised"),
                new LogGate("model_match", matches, matches ? "model identity matches" : "model identity does not match"),
                new LogGate("moves", moves != null, moves == null ? "the sample move list must contain strings" :
                    "the sample counts moves. LP4's compiler replays each move with its legal witness"),
                new LogGate("final_hash", hashMatches, hashMatches ? "final hash matches" :
                    moves == null ? "final hash unchecked because the move list is invalid" : "final hash does not match"),
                new LogGate("scripts", true, "none embedded"),
            };
            LogFacts? facts = matches && moves != null && hashMatches ? new LogFacts(moves.Length, time, modelIdentity, saved) : null;
            return new CompileResult(number, digest, Array.Empty<string>(), gates, facts);
        }
        catch (JsonException)
        {
            return BadFormat("the sample log is not valid JSON");
        }
    }

    private static string? Value(JsonElement log, string key) =>
        log.TryGetProperty(key, out JsonElement value) && value.ValueKind == JsonValueKind.String ? value.GetString() : null;
}

public static class SampleDocuments
{
    public const string LogExtension = ".sample-log"; // The sample extension. LP4 names the real one.
    public const string MidLogPath = "mid" + LogExtension;
    public const string EndgameLogPath = "endgame" + LogExtension;
    public const string BadLogPath = "bad-hash" + LogExtension;
    internal const string LogFormat = "magic600-sample-log/1";

    public static string LogText(string modelIdentity, SolvingState state, IReadOnlyList<string> moves, string time) =>
        WriteLog(modelIdentity, state.Id, moves, time, LogHash(state.Id, moves));

    public static MemoryDocumentStore Store(string modelIdentity, IReadOnlyList<SolvingState> states, IReadOnlyDictionary<string, string> scripts)
    {
        var store = new MemoryDocumentStore();
        void Put(string path, string text) => store.Write(path, Encoding.UTF8.GetBytes(text));
        string[] moves = { "C01 R01", "C02 R02" };
        Put(MidLogPath, LogText(modelIdentity, states[0], moves, "00:12:34"));
        Put(EndgameLogPath, LogText(modelIdentity, states[1], moves, "01:23:45"));
        Put(BadLogPath, WriteLog(modelIdentity, states[0].Id, moves, "00:12:34", new string('0', 64)));
        foreach ((string path, string text) in scripts) Put(path, text);
        return store;
    }

    internal static string LogHash(string stateId, IReadOnlyList<string> moves) =>
        Document.Hash(Encoding.UTF8.GetBytes(stateId + "\n" + string.Join("\n", moves)));

    private static string WriteLog(string model, string state, IReadOnlyList<string> moves, string time, string hash) =>
        JsonSerializer.Serialize(new { format = LogFormat, model, state, moves, time, hash });
}

public sealed record DocumentSignature(IReadOnlyList<RailCell>? Ticks, CompileLamp? Lamp, bool Recording, string Tip);
public sealed record DocumentProblem(string DocumentId, int Number, string Text, bool Dimmed);

public sealed class Document
{
    public string Id { get; }
    public DocumentKind Kind { get; }
    public string Name { get; internal set; }
    public string? Path { get; internal set; }
    public string? PathMark => Kind != DocumentKind.Session && Path == null ? "never saved" : null;
    public string Text { get; internal set; } = "";
    public string? SavedDigest { get; internal set; }
    public bool Pinned { get; internal set; }
    public CompileResult? Compile { get; internal set; }
    public string? DiskText { get; internal set; }

    internal Document(string id, DocumentKind kind, string name)
    {
        Id = id;
        Kind = kind;
        Name = name;
    }

    public string Digest => Hash(Encoding.UTF8.GetBytes(Text));
    public bool Dirty => Kind != DocumentKind.Session && (SavedDigest == null ? Text.Length > 0 : SavedDigest != Digest);
    private CompileResult? CurrentCompile => Compile is { } result && result.Digest == Digest ? result : null;
    private LogFacts? VerifiedFacts => Kind == DocumentKind.Log && CurrentCompile is { Passed: true } result ? result.Facts : null;
    public CompileLamp Lamp => CurrentCompile switch
    {
        null => CompileLamp.NotCompiled,
        { Passed: true } => CompileLamp.Clean,
        _ => CompileLamp.Problems,
    };
    public bool Verified => VerifiedFacts != null;
    public string? Binding => Verified ? "saved" : null;
    public bool ReadOnly => Kind == DocumentKind.Log && !Verified;
    public string? Reason
    {
        get
        {
            if (Kind != DocumentKind.Log) return null;
            CompileResult? result = CurrentCompile;
            if (result == null) return "not compiled yet";
            if (result.Passed && result.Facts != null) return null;
            return result.Gates.FirstOrDefault(gate => !gate.Passed)?.Detail ?? result.Problems.FirstOrDefault() ?? "no verified state";
        }
    }

    public DocumentSignature Signature(SolvingState? live, bool byId)
    {
        if (Kind == DocumentKind.Session)
            return new DocumentSignature(live == null ? null : OrbitRailModel.Cells(live, byId), null, true, "recording");

        string DiskNote(string tip) => DiskText == null ? tip : tip + " · changed on disk since; both versions kept";
        if (VerifiedFacts is { } facts)
        {
            string tip = FormattableString.Invariant($"{facts.Moves} moves · {facts.Time} · {facts.ModelIdentity}");
            return new DocumentSignature(OrbitRailModel.Cells(facts.Saved, byId), null, false, DiskNote(tip));
        }
        CompileLamp lamp = Lamp;
        return new DocumentSignature(null, lamp, false, DiskNote(lamp.Word()));
    }

    internal static string Hash(byte[] bytes) => Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
}

public sealed class DocumentSet
{
    private const string NewRow = "file.document.new", OpenRow = "file.document.open", SaveRow = "file.document.save",
        SaveAsRow = "file.document.save_as", CompileRow = "file.document.compile", CloseRow = "file.document.close";
    private const string InventoryFormat = "magic600-document-inventory/1";
    private const string UntitledPrefix = "untitled-";
    private static readonly JsonSerializerOptions InventoryJson = new() { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
    private sealed record Inventory(string Format, InventoryDocument[] Documents, string Active);
    private sealed record InventoryDocument(string Id, string Kind, string? Path, string? Mark, string? SavedDigest, string Text, bool Pinned);

    private readonly IDocumentStore store;
    private readonly IDocumentCompiler compiler;
    private readonly List<Document> opened = new();
    private int untitledNumber, compileNumber;

    public DocumentSet(IDocumentStore store, IDocumentCompiler compiler)
    {
        this.store = store;
        this.compiler = compiler;
        Active = new Document("session", DocumentKind.Session, "Session");
        opened.Add(Active);
    }

    public IReadOnlyList<Document> Documents => opened.Take(1).Concat(opened.Skip(1).OrderByDescending(doc => doc.Pinned)).ToArray();
    public Document Active { get; private set; }

    public bool Activate(string id)
    {
        if (Find(id) is not { } doc) return false;
        Active = doc;
        return true;
    }

    public Document New(DocumentKind kind)
    {
        if (kind is not (DocumentKind.Log or DocumentKind.Script)) throw new ArgumentOutOfRangeException(nameof(kind));
        string number = (++untitledNumber).ToString(CultureInfo.InvariantCulture);
        string id = UntitledPrefix + number;
        var doc = new Document(id, kind, UntitledName(id, kind));
        opened.Add(doc);
        return Active = doc;
    }

    public Document? Open(string path)
    {
        if (string.IsNullOrWhiteSpace(path)) return null;
        if (opened.FirstOrDefault(doc => SamePath(doc.Path, path)) is { } alreadyOpen) return Active = alreadyOpen;
        if (store.Read(path) is not { } bytes) return null;

        DocumentKind kind = path.EndsWith(".6cs", StringComparison.OrdinalIgnoreCase) ? DocumentKind.Script : DocumentKind.Log;
        // A saved document's identity is its path (ADR-D01.80, rev 6 R-3); a path reused after Save as gets a suffix.
        string id = path;
        for (int n = 2; Find(id) != null; n++) id = path + "#" + n.ToString(CultureInfo.InvariantCulture);
        var doc = new Document(id, kind, Path.GetFileName(path))
        {
            Path = path, Text = Encoding.UTF8.GetString(bytes), SavedDigest = Document.Hash(bytes),
        };
        if (kind == DocumentKind.Log) Compile(doc);
        opened.Add(doc);
        return Active = doc;
    }

    public string? Edit(string id, string text)
    {
        if (Find(id) is not { } doc) return "no such document";
        if (doc.Kind == DocumentKind.Session) return "the session records live";
        if (doc.ReadOnly) return "log is read-only · " + doc.Reason;
        doc.Text = text;
        return null;
    }

    public string? Save(string id)
    {
        if (Find(id) is not { } doc) return "no such document";
        if (Unavailable(doc, SaveRow, null) is { } reason) return reason;
        Write(doc, doc.Path!);
        return null;
    }

    public string? SaveAs(string id, string path)
    {
        if (Find(id) is not { } doc) return "no such document";
        if (Unavailable(doc, SaveAsRow, path) is { } reason) return reason;
        Write(doc, path);
        return null;
    }

    private void Write(Document doc, string path)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(doc.Text);
        string digest = Document.Hash(bytes);
        string name = Path.GetFileName(path);
        store.Write(path, bytes);
        doc.Path = path;
        doc.Name = name;
        doc.SavedDigest = digest;
        doc.DiskText = null;
    }

    public string? CompileDocument(string id)
    {
        if (Find(id) is not { } doc) return "no such document";
        if (Unavailable(doc, CompileRow, null) is { } reason) return reason;
        Compile(doc);
        return null;
    }

    private void Compile(Document doc)
    {
        int number = ++compileNumber;
        doc.Compile = doc.Kind == DocumentKind.Script ? compiler.Script(number, doc.Digest, doc.Text) : compiler.Log(number, doc.Digest, doc.Text);
    }

    public string? Close(string id, bool discard)
    {
        if (Find(id) is not { } doc) return "no such document";
        if (Unavailable(doc, CloseRow, discard ? "discard" : null) is { } reason) return reason;
        opened.Remove(doc);
        if (Active == doc) Active = opened[^1];
        return null;
    }

    public string? Pin(string id)
    {
        if (Find(id) is not { } doc) return "no such document";
        if (doc.Kind == DocumentKind.Session) return "the session is always first";
        doc.Pinned = !doc.Pinned;
        return null;
    }

    public IReadOnlyList<DocumentProblem> Problems()
    {
        var rows = new List<DocumentProblem>();
        foreach (Document doc in Documents)
        {
            if (doc.Compile is not { } result) continue;
            bool dimmed = result.Digest != doc.Digest;
            foreach (string problem in result.Problems.Concat(result.Gates.Where(g => !g.Passed).Select(g => g.Detail)))
            {
                string line = dimmed ? FormattableString.Invariant($"{problem} · compile {result.Number}") : problem;
                rows.Add(new DocumentProblem(doc.Id, result.Number, line, dimmed));
            }
        }
        return rows;
    }

    public IReadOnlyList<Document> Visible(int fit) => Documents.Take(Math.Max(1, fit)).ToArray();
    public IReadOnlyList<Document> Overflow(int fit) => Documents.Skip(Math.Max(1, fit)).ToArray();
    public IReadOnlyList<Document> Search(string query)
    {
        string term = query.Trim();
        return Documents.Where(doc => doc.Name.Contains(term, StringComparison.OrdinalIgnoreCase) ||
            (doc.Path?.Contains(term, StringComparison.OrdinalIgnoreCase) ?? false)).ToArray();
    }

    public string ExitInventory() => JsonSerializer.Serialize(new Inventory(InventoryFormat, opened.Skip(1).Select(doc =>
        new InventoryDocument(doc.Id, KindWord(doc.Kind), doc.Path, doc.PathMark, doc.SavedDigest, doc.Text, doc.Pinned)).ToArray(), Active.Id), InventoryJson);

    public static DocumentSet Restore(string inventory, IDocumentStore store, IDocumentCompiler compiler)
    {
        Inventory? saved = JsonSerializer.Deserialize<Inventory>(inventory, InventoryJson);
        if (saved?.Format != InventoryFormat) throw new FormatException("unknown document inventory format");

        var set = new DocumentSet(store, compiler);
        foreach (InventoryDocument entry in saved.Documents)
        {
            DocumentKind kind = entry.Kind switch
            {
                "log" => DocumentKind.Log,
                "script" => DocumentKind.Script,
                _ => throw new FormatException("unknown document kind in the inventory"),
            };
            if (entry.Id.StartsWith(UntitledPrefix, StringComparison.Ordinal) &&
                int.TryParse(entry.Id.AsSpan(UntitledPrefix.Length), NumberStyles.None, CultureInfo.InvariantCulture, out int number))
                set.untitledNumber = Math.Max(set.untitledNumber, number);
            string name = entry.Path == null ? UntitledName(entry.Id, kind) : Path.GetFileName(entry.Path);
            var doc = new Document(entry.Id, kind, name)
            {
                Path = entry.Path, Text = entry.Text, SavedDigest = entry.SavedDigest, Pinned = entry.Pinned,
            };
            if (entry.Path is { } path)
            {
                if (store.Read(path) is { } bytes && Document.Hash(bytes) != doc.SavedDigest)
                    doc.DiskText = Encoding.UTF8.GetString(bytes);
                if (kind == DocumentKind.Log) set.Compile(doc);
            }
            set.opened.Add(doc);
        }
        set.Activate(saved.Active);
        return set;
    }

    public static bool Handles(string command) => command is NewRow or OpenRow or SaveRow or SaveAsRow or CompileRow or CloseRow;

    public string? Unavailable(string command, string? argument) => Unavailable(Active, command, argument);

    private string? Unavailable(Document doc, string command, string? argument) => command switch
    {
        NewRow when argument is not ("log" or "script") => "choose log or script",
        SaveRow or SaveAsRow when doc.Kind == DocumentKind.Session => "the session is saved at exit without a prompt",
        OpenRow or SaveAsRow when string.IsNullOrWhiteSpace(argument) => "give a path",
        SaveRow when doc.Path == null => "never saved · use Save as",
        SaveAsRow when opened.Any(other => other != doc && SamePath(other.Path, argument!)) => "that path is already open",
        CompileRow when doc.Kind == DocumentKind.Session => "the session records live",
        CloseRow when doc.Kind == DocumentKind.Session => "the session stays open",
        CloseRow when doc.Dirty && argument != "discard" => "unsaved changes · save or discard before closing",
        _ when !Handles(command) => "unknown document command",
        _ => null,
    };

    public string Execute(string command, string? argument)
    {
        if (Unavailable(command, argument) is { } reason) return reason;
        Document doc = Active;
        switch (command)
        {
            case NewRow:
                return "opened " + New(argument == "log" ? DocumentKind.Log : DocumentKind.Script).Name;
            case OpenRow:
                Document? file = Open(argument!);
                if (file == null) return "file missing · " + argument;
                return file.Reason is { } detail ? $"opened {file.Name} read-only · {detail}" : "opened " + file.Name;
            case SaveRow:
                return Save(doc.Id) ?? "saved " + doc.Name;
            case SaveAsRow:
                return SaveAs(doc.Id, argument!) ?? "saved " + doc.Name;
            case CompileRow:
                return CompileDocument(doc.Id) ?? doc.Lamp.Word();
            case CloseRow:
                return Close(doc.Id, argument == "discard") ?? "closed " + doc.Name;
            default:
                return "unknown document command";
        }
    }

    private Document? Find(string id) => opened.FirstOrDefault(doc => doc.Id == id);
    private static bool SamePath(string? left, string right) => string.Equals(left, right, StringComparison.OrdinalIgnoreCase);
    private static string KindWord(DocumentKind kind) => kind == DocumentKind.Log ? "log" : "script";
    private static string UntitledName(string id, DocumentKind kind) => $"Untitled {KindWord(kind)} {id[UntitledPrefix.Length..]}";
}
