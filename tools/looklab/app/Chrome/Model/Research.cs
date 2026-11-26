using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// ADR-D02 build step B28 (ADR-D02.3, .44, .45, .46, .49): the research pane's model. It reads the answers of LP5's
// research rows (rust/c600-research, src/panel/answers.rs and wire.rs: each answer is {row, revision, research,
// answer}, with the rows' own field names) and draws nothing. The pane's tabs are Records, Claims, Runs and
// Obligations; Runs and Obligations are filled in B29.

// One method per research row of the command table, each giving that row's answer {row, revision, research, answer}
// as JSON text. K2 wires the research host (rust/c600-research's panel); the sample answers the four read rows.
public interface IResearchSource
{
    string Inventory(string? manifest = null, int? stage = null);          // research.archive.inventory
    string Claims(string? group = null, int? stage = null);                // research.claims
    string CheckReport(string run, int check);                             // research.check.report
    string Runs(string? group = null);                                     // research.runs
    string Obligations(string? group = null);                              // research.obligations
    string Import(string manifest, IReadOnlyList<string> members);         // research.archive.import
    string CheckRun(string? group, string checker, string certificate, IReadOnlyDictionary<string, string> inputs,
        string level, string digest, string puzzle);                       // research.check.run
    string Reproduce(string run);                                          // research.run.reproduce
    string ExportBundle(IReadOnlyList<string> records, string destination);// research.bundle.export
    string ExportObligation(string statement, string destination);         // research.obligation.export
    // research.record.read, a draft row (B28's row request): the records' one-paragraph findings and part trees.
    string RecordText(string? name = null);
}

// The sample's answers (tools/looklab/data/samples/research/panel.json): synthetic, read whole and unfiltered. The
// rows that run, import or export refuse: they wait for the research host.
public sealed class SampleResearchSource : IResearchSource
{
    public const string Format = "magic600-d02-research-sample/1", Synthetic = "synthetic";
    public const string Refusal = "the sample answers the reading rows only; the research host's rows are wired at K2";
    public static readonly IReadOnlyList<string> Rows = Array.AsReadOnly(new[]
    {
        ResearchRows.Records, ResearchRows.Claims, ResearchRows.Runs, ResearchRows.Obligations, ResearchRows.RecordRead,
    });
    public string Evidence { get; }
    public string Note { get; }
    // The draft rows the sample's pane needs, as a command-rows document.
    public string DraftRows { get; }
    private readonly Dictionary<string, string> answers = new(StringComparer.Ordinal);

    public SampleResearchSource(string text)
    {
        using JsonDocument d = J.Parse(text);
        JsonElement root = J.Root(d, Format, "evidence", "note", "answers", "draft_rows");
        Evidence = J.Text(root, "evidence");
        if (Evidence != Synthetic) throw J.Error("evidence", "the research sample is synthetic");
        Note = J.Text(root, "note");
        DraftRows = root.GetProperty("draft_rows").GetRawText();
        JsonElement all = root.GetProperty("answers");
        J.Keys(all, "answers", Rows.ToArray());
        foreach (string row in Rows) answers[row] = all.GetProperty(row).GetRawText();
    }

    private string Whole(string row, params object?[] filters) =>
        filters.All(f => f == null) ? answers[row] : throw new NotSupportedException("the sample answers " + row + " unfiltered");

    public string Inventory(string? manifest = null, int? stage = null) => Whole(ResearchRows.Records, manifest, stage);
    public string Claims(string? group = null, int? stage = null) => Whole(ResearchRows.Claims, group, stage);
    public string Runs(string? group = null) => Whole(ResearchRows.Runs, group);
    public string Obligations(string? group = null) => Whole(ResearchRows.Obligations, group);
    public string RecordText(string? name = null) => Whole(ResearchRows.RecordRead, name);
    public string CheckReport(string run, int check) => throw new NotSupportedException(Refusal);
    public string Import(string manifest, IReadOnlyList<string> members) => throw new NotSupportedException(Refusal);
    public string CheckRun(string? group, string checker, string certificate, IReadOnlyDictionary<string, string> inputs, string level, string digest, string puzzle) =>
        throw new NotSupportedException(Refusal);
    public string Reproduce(string run) => throw new NotSupportedException(Refusal);
    public string ExportBundle(IReadOnlyList<string> records, string destination) => throw new NotSupportedException(Refusal);
    public string ExportObligation(string statement, string destination) => throw new NotSupportedException(Refusal);
}

public enum ResearchTab { Records, Claims, Runs, Obligations }

// What a check said, or a state that is no verdict (ADR-D02.46). Only Pass, Fail and NotApplicable are verdicts, and
// only Pass and Fail count as a check: NotApplicable counts as no check, and a cut-off is never a verdict.
public enum VerdictKind { Pass, Fail, NotApplicable, NotChecked, CutOff, Reproduced, Differs, Replayed, ModelReview }

public sealed record ResearchVerdict(VerdictKind Kind, string? Claim = null, string? Code = null, string? Reason = null, long? Byte = null,
    IReadOnlyList<string>? Reasons = null, IReadOnlyList<string>? Dependencies = null)
{
    public bool IsVerdict => Kind is VerdictKind.Pass or VerdictKind.Fail or VerdictKind.NotApplicable;
    public bool CountsAsCheck => Kind is VerdictKind.Pass or VerdictKind.Fail;
}

// ADR-D02.46's words, exact. A verdict line is one line with its scope; it wraps and is never cut, in one weight and
// with no colour role for any kind.
public static class ResearchWords
{
    public const string NotApplicable = "not applicable (scope only)";
    public const string NotChecked = "not checked";
    public const string CutOff = "not found within budget";
    public const string BytesEqual = "reproduced: bytes equal";
    public const string Replayed = "replayed from record, not re-executed";
    public const string ModelReview = "reviewed by a model, not checked";
    public const string CopyNote = "Solving previews refuse it";
    public const string ReadOnly = "read-only";
    public const string Stale = "stale: the session changed since this answer; choose the tab to ask again";
    public static string Holds(string scope) => "holds within " + scope;
    public static string Fails(string scope) => "fails within " + scope;
    public static string Differs(long k) => "differs at byte " + k.ToString(CultureInfo.InvariantCulture);
    public static string Pinned(string digest) => "pinned: input " + digest;

    public static string VerdictLine(ResearchVerdict verdict, string scope) => verdict.Kind switch
    {
        VerdictKind.Pass => Holds(scope),
        VerdictKind.Fail => Fails(scope),
        VerdictKind.NotApplicable => NotApplicable,
        VerdictKind.NotChecked => NotChecked,
        VerdictKind.CutOff => CutOff,
        VerdictKind.Reproduced => BytesEqual,
        VerdictKind.Differs => Differs(verdict.Byte ?? throw new ArgumentException("a difference names its byte")),
        VerdictKind.Replayed => Replayed,
        VerdictKind.ModelReview => ModelReview,
        _ => throw new ArgumentOutOfRangeException(nameof(verdict)),
    };

    // No verdict has a colour role: the words carry it (ADR-D02.46).
    public static string? ColourRole(VerdictKind kind) => null;

    // What an expanded row adds under its verdict line: a Fail names the first claim that does not hold, its code and
    // the checker's reason; NotApplicable its reason; "not checked" its reasons and dependencies word for word.
    public static IReadOnlyList<string> Expanded(ResearchVerdict verdict) => verdict.Kind switch
    {
        VerdictKind.Fail => new[] { "first claim that does not hold: " + verdict.Claim, "code: " + verdict.Code, "checker's reason: " + verdict.Reason },
        VerdictKind.NotApplicable => new[] { "counted as no check", "reason: " + verdict.Reason },
        VerdictKind.NotChecked when verdict.Reasons is { Count: > 0 } reasons =>
            new[] { "reasons: " + string.Join(", ", reasons) }.Concat((verdict.Dependencies ?? Array.Empty<string>()).Select(d => "dependency: " + d)).ToArray(),
        VerdictKind.NotChecked => new[] { verdict.Reason ?? "preflight: ready; the check has not run" },
        VerdictKind.CutOff => new[] { "the run stopped at its budget: no verdict" + (verdict.Reason is { } covered ? " (" + covered + ")" : "") },
        _ => Array.Empty<string>(),
    };
}

// The rows the pane's controls invoke, through the command layer and the palette (ADR-D02.45).
public static class ResearchRows
{
    public const string Records = "research.archive.inventory", Claims = "research.claims", Runs = "research.runs", Obligations = "research.obligations";
    // A claim group or a part opens in place, and folds again, by inspecting it; a record opens by reading it.
    public const string Open = "object.inspect", RecordRead = "research.record.read";
    // A researcher's name is a workspace record (ADR-D02.3): the M6 write of the record "research.names".
    public const string Name = "workspace.set", NamesRecord = "research.names";
    public const string Copy = "list.row.copy", Save = "research.bundle.export";
    public static readonly IReadOnlyList<(ResearchTab Tab, string Row)> Tabs = Array.AsReadOnly(new[]
    {
        (ResearchTab.Records, Records), (ResearchTab.Claims, Claims), (ResearchTab.Runs, Runs), (ResearchTab.Obligations, Obligations),
    });
    public static readonly IReadOnlyList<string> All = Array.AsReadOnly(new[] { Records, Claims, Runs, Obligations, Open, RecordRead, Name, Copy, Save });
    // Rows the command table does not hold yet: row requests of this step.
    public static readonly IReadOnlyList<string> Drafts = Array.AsReadOnly(new[] { RecordRead });
    public static string RowOf(ResearchTab tab) => Tabs.First(t => t.Tab == tab).Row;
    public static ResearchTab? TabOf(string row) => Tabs.Where(t => t.Row == row).Select(t => (ResearchTab?)t.Tab).FirstOrDefault();
    // A name's argument to workspace.set: research.names:<reference>=<name>.
    public static string NameArgument(string reference, string name) => NamesRecord + ":" + reference + "=" + name;
    public static (string Ref, string Name)? ParseName(string? argument)
    {
        if (argument == null || !argument.StartsWith(NamesRecord + ":", StringComparison.Ordinal)) return null;
        string rest = argument[(NamesRecord.Length + 1)..];
        int at = rest.IndexOf('=');
        return at <= 0 ? null : (rest[..at], rest[(at + 1)..]);
    }
}

// A named part of a record with its length in twists; long steps are trees of named parts (ADR-D02.49).
public sealed record ResearchPart(string Name, long Length, IReadOnlyList<ResearchPart> Parts);

// A record: the inventory's row (name, role, stage, sha256, bytes, integrity) and its own text from the draft row
// research.record.read (family, one-paragraph finding, twists and parts).
public sealed record ResearchRecord(string Name, string Role, int? Stage, string Sha256, long Bytes, string Integrity, string Family, string Finding,
    long Twists, IReadOnlyList<ResearchPart> Parts);

// A claim group as research.claims answers it, with the verdict the pane words from its latest check or its preflight.
public sealed record ResearchClaim(string Group, string Family, int Stage, string Statement, string Scope, string? Label, string? CheckedAs, string DataStatus,
    ResearchVerdict Verdict, string? Run)
{
    // The source record and pointer the group's id names: "<family>:<path>#<pointer>".
    public string Path => Group[(Group.IndexOf(':') + 1)..].Split('#')[0];
    public string Pointer => Group.Contains('#') ? Group[(Group.IndexOf('#') + 1)..] : "";
}

// A run as research.runs answers it: its branch (the pinned input) is read-only and never turns stale.
public sealed record ResearchRun(string Run, string? Group, string Level, string Digest, string Identity, string Outcome, string Reading);

public enum ResearchLineKind { Stage, Family, Row, Detail, Finding, Part, Actions, Note }

// One line of the pane's flat list; the pane builds only the lines in view.
public sealed record ResearchLine(ResearchLineKind Kind, string Text, string? Ref = null, int Depth = 0, string? Verdict = null);

// A control of a line: its words and the row it invokes. A control that takes text (a name) appends it to Argument.
public sealed record ResearchControl(string Label, string Row, string? Argument, bool TakesText = false);

// A copy or a save: its row, the object's reference and digest, the origin Research and its note.
public sealed record ResearchTransfer(string Row, string Ref, string Digest, Origin Origin, string Note);

public sealed class ResearchModel
{
    public const string RecordRef = "research.record/", PartRef = "research.part/", ClaimRef = "research.claim/";
    public static readonly IReadOnlyList<string> AnswerKeys = Array.AsReadOnly(new[] { "row", "revision", "research", "answer" });

    public IResearchSource Source { get; }
    public string Evidence { get; }
    public string Research { get; private set; } = "";
    public IReadOnlyList<ResearchRecord> Records { get; private set; } = Array.Empty<ResearchRecord>();
    public IReadOnlyList<ResearchClaim> Claims { get; private set; } = Array.Empty<ResearchClaim>();
    public IReadOnlyList<ResearchRun> Runs { get; private set; } = Array.Empty<ResearchRun>();
    public int Obligations { get; private set; }
    public ResearchTab Tab { get; private set; } = ResearchTab.Claims;
    public string? Open { get; private set; }
    public ResearchTransfer? Transfer { get; private set; }
    public event Action? Changed;
    private readonly Dictionary<string, string> names = new(StringComparer.Ordinal);
    // The session's changes seen, and the change at which each row was last asked (ONE.12's current and stale).
    private long head;
    private readonly Dictionary<string, long> asked = new(StringComparer.Ordinal);

    public ResearchModel(IResearchSource source, string evidence)
    {
        Source = source; Evidence = evidence;
        foreach (var (_, row) in ResearchRows.Tabs) Ask(row);
    }

    public static ResearchModel FromSample(string text)
    {
        var sample = new SampleResearchSource(text);
        return new ResearchModel(sample, sample.Evidence);
    }

    public IReadOnlyDictionary<string, string> Names => names;

    // --- reading the answers ---------------------------------------------------------------------------------------

    // Asks a tab's row again (Records also reads the records' text); its answer is then current at the head.
    private void Ask(string row)
    {
        switch (row)
        {
            case ResearchRows.Records: Records = ReadRecords(Answer(row, Source.Inventory()), Answer(ResearchRows.RecordRead, Source.RecordText())); break;
            case ResearchRows.Claims: Claims = Answer(row, Source.Claims()).GetProperty("rows").EnumerateArray().Select(ReadClaim).ToArray(); break;
            case ResearchRows.Runs: Runs = Answer(row, Source.Runs()).GetProperty("rows").EnumerateArray().Select(ReadRun).ToArray(); break;
            case ResearchRows.Obligations: Obligations = Answer(row, Source.Obligations()).GetProperty("rows").GetArrayLength(); break;
        }
        asked[row] = head;
    }

    // The answer inside a row's envelope; every answer must come from the same research data.
    private JsonElement Answer(string row, string text)
    {
        JsonElement envelope = JsonDocument.Parse(text).RootElement;
        J.Keys(envelope, row, AnswerKeys.ToArray());
        if (J.Text(envelope, "row") != row) throw J.Error(row, "the answer names another row");
        string sha = J.Text(envelope, "research");
        if (Research.Length == 0) Research = sha;
        else if (sha != Research) throw J.Error(row, "the answers were made from different research data");
        return envelope.GetProperty("answer");
    }

    private static IReadOnlyList<ResearchRecord> ReadRecords(JsonElement inventory, JsonElement text)
    {
        var own = new Dictionary<string, (string Family, string Finding, long Twists, IReadOnlyList<ResearchPart> Parts)>(StringComparer.Ordinal);
        foreach (JsonElement row in text.GetProperty("rows").EnumerateArray())
        {
            IReadOnlyList<ResearchPart> parts = Parts(row.GetProperty("parts"));
            long twists = row.GetProperty("twists").GetInt64();
            if (Length(parts) != twists) throw J.Error(J.Text(row, "name"), "its parts' lengths do not add up to its twists");
            own.Add(J.Text(row, "name"), (J.Text(row, "family"), J.Text(row, "finding"), twists, parts));
        }
        var records = new List<ResearchRecord>();
        foreach (JsonElement row in inventory.GetProperty("rows").EnumerateArray())
        {
            string name = J.Text(row, "name");
            if (!own.TryGetValue(name, out var t)) throw J.Error(name, "the record's text is not in research.record.read's answer");
            JsonElement stage = row.GetProperty("stage");
            records.Add(new ResearchRecord(name, J.Text(row, "role"), stage.ValueKind == JsonValueKind.Null ? null : stage.GetInt32(), J.Text(row, "sha256"),
                row.GetProperty("bytes").GetInt64(), J.Text(row, "integrity"), t.Family, t.Finding, t.Twists, t.Parts));
        }
        return records;
    }

    private static ResearchClaim ReadClaim(JsonElement row)
    {
        JsonElement latest = row.GetProperty("latest_check");
        return new ResearchClaim(J.Text(row, "group"), J.Text(row, "family"), row.GetProperty("stage").GetInt32(), J.Text(row.GetProperty("statement"), "id"),
            J.Text(row, "scope_phrase"), Optional(row, "label"), Optional(row, "checked_as"), J.Text(row, "data_status"), VerdictOf(row),
            latest.ValueKind == JsonValueKind.Null ? null : J.Text(latest, "run"));
    }

    private static ResearchRun ReadRun(JsonElement row)
    {
        JsonElement branch = row.GetProperty("branch");
        return new ResearchRun(J.Text(row, "run"), Optional(row, "group"), J.Text(branch, "level"), J.Text(branch, "digest"), J.Text(branch, "identity"),
            J.Text(row, "outcome"), J.Text(row.GetProperty("reading"), "kind"));
    }

    private static string? Optional(JsonElement row, string id) =>
        row.TryGetProperty(id, out JsonElement value) && value.ValueKind == JsonValueKind.String ? value.GetString() : null;

    private static IReadOnlyList<ResearchPart> Parts(JsonElement list) =>
        list.EnumerateArray().Select(p => new ResearchPart(J.Text(p, "name"), p.GetProperty("length").GetInt64(), Parts(p.GetProperty("parts")))).ToArray();

    // The words a claim group's row shows: its latest check (a cut-off first: never a verdict), else its preflight.
    public static ResearchVerdict VerdictOf(JsonElement row)
    {
        JsonElement latest = row.GetProperty("latest_check");
        if (latest.ValueKind != JsonValueKind.Null)
        {
            JsonElement reading = latest.GetProperty("reading");
            switch (J.Text(reading, "kind"))
            {
                case "cut_off": return new ResearchVerdict(VerdictKind.CutOff, Reason: "covered " + J.Text(reading, "covered"));
                case "not_run": return new ResearchVerdict(VerdictKind.NotChecked, Reason: "the check did not run");
                case "open": return new ResearchVerdict(VerdictKind.NotChecked, Reason: "open: " + J.Text(reading, "code"));
            }
            JsonElement[] verdicts = latest.GetProperty("verdicts").EnumerateArray().Where(v => v.ValueKind == JsonValueKind.Object).ToArray();
            if (verdicts.FirstOrDefault(v => J.Text(v, "kind") == "fail") is { ValueKind: JsonValueKind.Object } fail)
                return new ResearchVerdict(VerdictKind.Fail, J.Text(fail, "claim"), J.Text(fail, "code"), J.Text(fail, "reason"));
            if (verdicts.FirstOrDefault(v => J.Text(v, "kind") == "not_applicable") is { ValueKind: JsonValueKind.Object } na)
                return new ResearchVerdict(VerdictKind.NotApplicable, Reason: J.Text(na, "reason"));
            if (verdicts.Length > 0 && verdicts.All(v => J.Text(v, "kind") == "pass")) return new ResearchVerdict(VerdictKind.Pass);
            return new ResearchVerdict(VerdictKind.NotChecked, Reason: "the check reached no report");
        }
        JsonElement preflight = row.GetProperty("preflight");
        if (J.Text(preflight, "kind") == "ready") return new ResearchVerdict(VerdictKind.NotChecked, Reason: "preflight: ready; the check has not run");
        string[] reasons = J.Strings(preflight.GetProperty("reasons"), "reasons");
        string[] dependencies = preflight.GetProperty("dependencies").EnumerateArray().Select(Dependency).ToArray();
        return new ResearchVerdict(VerdictKind.NotChecked, Reasons: reasons, Dependencies: dependencies);
    }

    // A dependency word for word: its kind, then its own fields.
    private static string Dependency(JsonElement d) => J.Text(d, "kind") switch
    {
        "missing_input" => "missing_input: " + J.Text(d, "name"),
        "input" => "input: " + J.Text(d, "path") + " (" + J.Text(d, "state") + ")",
        "historical_gate" => "historical_gate: " + J.Text(d, "source") + " " + J.Text(d, "pointer"),
        string other => other,
    };

    // --- what the rows do ------------------------------------------------------------------------------------------

    // A tab's row: the pane shows the tab and asks its row again, so its answer is current at the head.
    public void Select(ResearchTab tab)
    {
        Tab = tab; Ask(ResearchRows.RowOf(tab));
        if (Open != null && TabOfRef(Open) != tab) Open = null;
        Raise();
    }

    // The session changed (a commit, an undo, a redo): every answer asked before is stale until it is asked again.
    // A run's pinned input is a fact of what ran, not an answer, so it never turns stale.
    public void Committed() { head++; Raise(); }

    public bool IsStale(string row) => asked.TryGetValue(row, out long at) && at != head;

    public string AnswerLine() => IsStale(ResearchRows.RowOf(Tab)) ? ResearchWords.Stale : "current";

    // The run's branch: its pinned input, read-only (ADR-D02.45).
    public IReadOnlyList<string> PinnedLines(string run)
    {
        ResearchRun r = Runs.FirstOrDefault(x => x.Run == run) ?? throw new ArgumentException("no run " + run);
        return new[] { ResearchWords.Pinned(r.Digest), $"branch: {r.Level} · identity {r.Identity[..Math.Min(12, r.Identity.Length)]} · {ResearchWords.ReadOnly}" };
    }

    // Opens a claim group, a record or a part in place; asking for the open one again folds it (a part to its parent).
    public bool Toggle(string reference)
    {
        if (!Known(reference)) return false;
        ResearchTab tab = TabOfRef(reference);
        if (Tab != tab) { Tab = tab; Ask(ResearchRows.RowOf(tab)); }
        if (Open == reference) Open = reference.StartsWith(PartRef, StringComparison.Ordinal) ? Parent(reference) : null;
        else Open = reference;
        Raise();
        return true;
    }

    // The researcher's name for a record or a part: data shown wherever it shows; no digest and no row changes.
    public bool Rename(string reference, string name)
    {
        if (!reference.StartsWith(RecordRef, StringComparison.Ordinal) && !reference.StartsWith(PartRef, StringComparison.Ordinal) || !Known(reference)) return false;
        if (string.IsNullOrWhiteSpace(name)) names.Remove(reference); else names[reference] = name.Trim();
        Raise();
        return true;
    }

    public ResearchTransfer Copy(string reference) => Carry(ResearchRows.Copy, reference);
    public ResearchTransfer Save(string reference) => Carry(ResearchRows.Save, reference);

    private ResearchTransfer Carry(string row, string reference)
    {
        if (!Known(reference)) throw new ArgumentException("no research object " + reference);
        Transfer = new ResearchTransfer(row, reference, Digest(reference), Origin.Research, ResearchWords.CopyNote);
        Raise();
        return Transfer;
    }

    public string TransferLine() => Transfer is not { } t ? ""
        : $"{(t.Row == ResearchRows.Copy ? "copied" : "saved")} {Shown(t.Ref)} · origin {t.Origin} · {t.Note}";

    // A record's digest is its bytes' SHA-256; a part's is its record's; a claim group's is its statement's id.
    public string Digest(string reference) =>
        reference.StartsWith(ClaimRef, StringComparison.Ordinal) ? ClaimOf(reference)!.Statement : RecordOf(reference)!.Sha256;

    public (int Holds, int Fails, int NoCheck) Counts() =>
        (Claims.Count(c => c.Verdict.Kind == VerdictKind.Pass), Claims.Count(c => c.Verdict.Kind == VerdictKind.Fail), Claims.Count(c => !c.Verdict.CountsAsCheck));

    public string CountsLine()
    {
        var (holds, fails, none) = Counts();
        return $"{Claims.Count} claim groups: holds {holds} · fails {fails} · no check {none}";
    }

    // --- the rows the controls invoke --------------------------------------------------------------------------------

    public static IReadOnlyList<ResearchControl> TabControls() =>
        ResearchRows.Tabs.Select(t => new ResearchControl(t.Tab.ToString(), t.Row, null)).ToArray();

    // The controls a line carries; the pane draws exactly these and nothing else that acts.
    public IReadOnlyList<ResearchControl> Controls(ResearchLine line)
    {
        if (line.Ref is not { } reference) return Array.Empty<ResearchControl>();
        switch (line.Kind)
        {
            case ResearchLineKind.Row when reference.StartsWith(RecordRef, StringComparison.Ordinal):
                return new[] { new ResearchControl(line.Text, ResearchRows.RecordRead, reference) };
            case ResearchLineKind.Row or ResearchLineKind.Part:
                return new[] { new ResearchControl(line.Text, ResearchRows.Open, reference) };
            case ResearchLineKind.Actions:
                var controls = new List<ResearchControl>();
                if (!reference.StartsWith(ClaimRef, StringComparison.Ordinal))
                    controls.Add(new ResearchControl("Name", ResearchRows.Name, ResearchRows.NameArgument(reference, ""), true));
                controls.Add(new ResearchControl("Copy", ResearchRows.Copy, reference));
                if (RecordOf(reference) is { } record) controls.Add(new ResearchControl("Save", ResearchRows.Save, Ref(record)));
                else if (ClaimOf(reference) is { } claim && Records.FirstOrDefault(r => r.Name == claim.Path) is { } source)
                    controls.Add(new ResearchControl("Save", ResearchRows.Save, Ref(source)));
                return controls;
            default:
                return Array.Empty<ResearchControl>();
        }
    }

    // Whether a row with this argument is the pane's: a tab row with no argument, or a row naming a research object.
    public static bool Handles(string row, string? argument) => row switch
    {
        _ when ResearchRows.TabOf(row) != null => argument == null,
        ResearchRows.RecordRead or ResearchRows.Open or ResearchRows.Copy or ResearchRows.Save =>
            argument != null && (argument.StartsWith(RecordRef, StringComparison.Ordinal) || argument.StartsWith(PartRef, StringComparison.Ordinal) || argument.StartsWith(ClaimRef, StringComparison.Ordinal)),
        ResearchRows.Name => ResearchRows.ParseName(argument) != null,
        _ => false,
    };

    // Carries out a row the pane handles; the words say what it did, or why it did nothing.
    public (bool Done, string Words) Execute(string row, string? argument)
    {
        if (!Handles(row, argument)) return (false, "not a research row");
        if (ResearchRows.TabOf(row) is { } tab) { Select(tab); return (true, "Research: " + tab); }
        if (row == ResearchRows.Name)
        {
            var (reference, name) = ResearchRows.ParseName(argument)!.Value;
            return Rename(reference, name) ? (true, $"named {Shown(reference)}") : (false, "only a record or a part takes a name");
        }
        string target = argument!;
        if (!Known(target)) return (false, "no research object " + target);
        switch (row)
        {
            case ResearchRows.RecordRead when !target.StartsWith(RecordRef, StringComparison.Ordinal): return (false, "research.record.read reads a record");
            case ResearchRows.RecordRead or ResearchRows.Open: Toggle(target); return (true, (Open == target ? "opened " : "folded ") + Shown(target));
            case ResearchRows.Copy: Copy(target); return (true, TransferLine());
            case ResearchRows.Save when !target.StartsWith(RecordRef, StringComparison.Ordinal): return (false, "a bundle saves records");
            default: Save(target); return (true, TransferLine());
        }
    }

    // --- references --------------------------------------------------------------------------------------------------

    public static string Ref(ResearchRecord record) => RecordRef + record.Name;
    public static string Ref(ResearchClaim claim) => ClaimRef + claim.Group;
    public static string PartOf(ResearchRecord record, IEnumerable<int> path) => PartRef + record.Name + "@" + string.Join(".", path);

    private bool Known(string reference) => reference.StartsWith(ClaimRef, StringComparison.Ordinal) ? ClaimOf(reference) != null
        : reference.StartsWith(PartRef, StringComparison.Ordinal) ? RecordOf(reference) is { } r && PartAt(r, PathOf(reference)) != null
        : RecordOf(reference) != null;

    private ResearchClaim? ClaimOf(string reference) => Claims.FirstOrDefault(c => Ref(c) == reference);

    private ResearchRecord? RecordOf(string reference)
    {
        string name = reference.StartsWith(PartRef, StringComparison.Ordinal) && reference.LastIndexOf('@') > PartRef.Length
            ? reference[PartRef.Length..reference.LastIndexOf('@')]
            : reference.StartsWith(RecordRef, StringComparison.Ordinal) ? reference[RecordRef.Length..] : "";
        return Records.FirstOrDefault(r => r.Name == name);
    }

    private static int[] PathOf(string part)
    {
        string text = part[(part.LastIndexOf('@') + 1)..];
        return text.Split('.').Select(s => int.TryParse(s, NumberStyles.None, CultureInfo.InvariantCulture, out int i) ? i : -1).ToArray();
    }

    private static ResearchPart? PartAt(ResearchRecord record, int[] path)
    {
        IReadOnlyList<ResearchPart> level = record.Parts; ResearchPart? at = null;
        foreach (int i in path) { if (i < 0 || i >= level.Count) return null; at = level[i]; level = at.Parts; }
        return at;
    }

    private string Parent(string part)
    {
        ResearchRecord record = RecordOf(part)!;
        int[] path = PathOf(part);
        return path.Length <= 1 ? Ref(record) : PartOf(record, path[..^1]);
    }

    private static ResearchTab TabOfRef(string reference) => reference.StartsWith(ClaimRef, StringComparison.Ordinal) ? ResearchTab.Claims : ResearchTab.Records;

    // The name a record shows: the researcher's, else the file name its archive path ends in.
    public string Shown(ResearchRecord record) => names.TryGetValue(Ref(record), out string? n) ? n : record.Name[(record.Name.LastIndexOf('/') + 1)..];

    public string Shown(ResearchClaim claim)
    {
        ResearchRecord? record = Records.FirstOrDefault(r => r.Name == claim.Path);
        string name = record != null ? Shown(record) : claim.Path[(claim.Path.LastIndexOf('/') + 1)..];
        return claim.Pointer.Length > 0 ? name + " #" + claim.Pointer : name;
    }

    public string Shown(string reference)
    {
        if (ClaimOf(reference) is { } claim) return Shown(claim);
        ResearchRecord? record = RecordOf(reference);
        if (record == null) return reference;
        if (!reference.StartsWith(PartRef, StringComparison.Ordinal)) return Shown(record);
        int[] path = PathOf(reference);
        return PartAt(record, path) is { } part ? Shown(record) + " › " + Shown(record, part, path) : reference;
    }

    private string Shown(ResearchRecord record, ResearchPart part, int[] path) =>
        names.TryGetValue(PartOf(record, path), out string? n) ? n : part.Name;

    // --- the flat list -----------------------------------------------------------------------------------------------

    public IReadOnlyList<ResearchLine> Lines()
    {
        var lines = new List<ResearchLine>();
        switch (Tab)
        {
            case ResearchTab.Claims: ClaimLines(lines); break;
            case ResearchTab.Records: RecordLines(lines); break;
            case ResearchTab.Runs:
                lines.Add(new ResearchLine(ResearchLineKind.Note, $"{Runs.Count} runs in the order they ran. Their budgets, reproduction and derived states come in build step B29."));
                break;
            case ResearchTab.Obligations:
                lines.Add(new ResearchLine(ResearchLineKind.Note, $"{Obligations} open obligations. Their anchors and export status come in build step B29."));
                break;
        }
        return lines;
    }

    // Grouped by stage, then family, each in the answer's own order (claim groups in the claim matrix's): no sort.
    private static IEnumerable<IGrouping<(int? Stage, string Family), T>> Groups<T>(IEnumerable<T> items, Func<T, int?> stage, Func<T, string> family) =>
        items.GroupBy(i => (stage(i), family(i)));

    private void ClaimLines(List<ResearchLine> lines)
    {
        int? stage = -1;
        foreach (var group in Groups(Claims, c => c.Stage, c => c.Family))
        {
            if (group.Key.Stage != stage)
            {
                stage = group.Key.Stage;
                lines.Add(new ResearchLine(ResearchLineKind.Stage, $"Stage {stage} · {Claims.Count(c => c.Stage == stage)} claim groups"));
            }
            lines.Add(new ResearchLine(ResearchLineKind.Family, $"{group.Key.Family} · {group.Count()}"));
            foreach (ResearchClaim claim in group)
            {
                string reference = Ref(claim);
                bool open = Open == reference;
                lines.Add(new ResearchLine(ResearchLineKind.Row, (open ? "▾ " : "▸ ") + Shown(claim), reference, 0, ResearchWords.VerdictLine(claim.Verdict, claim.Scope)));
                if (!open) continue;
                foreach (string line in ResearchWords.Expanded(claim.Verdict)) lines.Add(new ResearchLine(ResearchLineKind.Detail, line, reference, 1));
                if (claim.Label != null) lines.Add(new ResearchLine(ResearchLineKind.Detail, "label: " + claim.Label, reference, 1));
                lines.Add(new ResearchLine(ResearchLineKind.Detail, $"checked as {claim.CheckedAs ?? "none"} · data {claim.DataStatus} · statement {claim.Statement[..Math.Min(12, claim.Statement.Length)]}", reference, 1));
                if (claim.Run != null) foreach (string line in PinnedLines(claim.Run)) lines.Add(new ResearchLine(ResearchLineKind.Detail, line, reference, 1));
                lines.Add(new ResearchLine(ResearchLineKind.Actions, "", reference, 1));
            }
        }
    }

    private void RecordLines(List<ResearchLine> lines)
    {
        int? stage = -1;
        foreach (var group in Groups(Records, r => r.Stage, r => r.Family))
        {
            if (group.Key.Stage != stage)
            {
                stage = group.Key.Stage;
                lines.Add(new ResearchLine(ResearchLineKind.Stage, (stage is int s ? $"Stage {s}" : "No stage") + $" · {Records.Count(r => r.Stage == stage)} records"));
            }
            lines.Add(new ResearchLine(ResearchLineKind.Family, $"{group.Key.Family} · {group.Count()}"));
            foreach (ResearchRecord record in group)
            {
                string reference = Ref(record);
                bool open = Open != null && (Open == reference || Open.StartsWith(PartRef + record.Name + "@", StringComparison.Ordinal));
                string info = $"{record.Twists.ToString("N0", CultureInfo.InvariantCulture)} twists · {record.Parts.Count} parts · {record.Bytes.ToString("N0", CultureInfo.InvariantCulture)} bytes · integrity {record.Integrity}";
                lines.Add(new ResearchLine(ResearchLineKind.Row, (open ? "▾ " : "▸ ") + Shown(record), reference, 0, info));
                if (!open) continue;
                lines.Add(new ResearchLine(ResearchLineKind.Finding, record.Finding, reference, 1));
                int[] unfolded = Open!.StartsWith(PartRef, StringComparison.Ordinal) ? PathOf(Open) : Array.Empty<int>();
                PartLines(lines, record, record.Parts, Array.Empty<int>(), unfolded);
                lines.Add(new ResearchLine(ResearchLineKind.Detail, "sha256 " + record.Sha256, reference, 1));
                lines.Add(new ResearchLine(ResearchLineKind.Actions, "", Open!, 1));
            }
        }
    }

    // The folded tree: every part of a level shows; a part's own parts show while it, or one under it, is open.
    private void PartLines(List<ResearchLine> lines, ResearchRecord record, IReadOnlyList<ResearchPart> parts, int[] at, int[] unfolded)
    {
        for (int i = 0; i < parts.Count; i++)
        {
            int[] path = at.Append(i).ToArray();
            ResearchPart part = parts[i];
            bool open = unfolded.Length >= path.Length && unfolded.Take(path.Length).SequenceEqual(path);
            string mark = part.Parts.Count == 0 ? "· " : open ? "▾ " : "▸ ";
            lines.Add(new ResearchLine(ResearchLineKind.Part, mark + Shown(record, part, path) + " · " + part.Length.ToString("N0", CultureInfo.InvariantCulture) + " twists",
                PartOf(record, path), path.Length));
            if (open) PartLines(lines, record, part.Parts, path, unfolded);
        }
    }

    public static long Length(IReadOnlyList<ResearchPart> parts) => parts.Sum(p => p.Length);

    private void Raise() => Safe.Raise(Changed);
}
