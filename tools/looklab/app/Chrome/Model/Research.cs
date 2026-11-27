using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Magic600.Chrome;

// ADR-D02 build step B28 (ADR-D02.3, .44, .45, .46, .49): the research pane's model. It reads the answers of LP5's
// research rows (rust/c600-research, src/panel/answers.rs and wire.rs: each answer is {row, revision, research,
// answer}, with the rows' own field names) and draws nothing. The pane's tabs are Records, Claims, Runs and
// Obligations. B29 (ADR-D02.4, .45, .46, .50, .52) fills Runs (a declared budget, seed and depth, the start, a cut-off,
// reproduction and the two CR-17 controls) and Obligations.

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
    // research.run.start, a draft row (B29's row request): starts a research run on a seed of the pool at a depth,
    // within a budget declared before the start (ADR-D02.4, .50, .52). Its answer is the run as research.runs shows it,
    // with the seed, the depth and the budget it recorded.
    string StartRun(int seed, int depth, long budget);
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
        ResearchRows.Start, ResearchRows.Reproduce,
    });
    // B29: the sample's one run. It never finishes: whatever budget is declared, it reaches it and stops as cut off. It
    // is named by the digest of its declaration, so a script can name it before it starts.
    public const string RunName = "magic600-d02-research-sample-run/1";
    public static string RunId(int seed, int depth, long budget) => Sha(FormattableString.Invariant($"{RunName} seed={seed} depth={depth} budget={budget}"));
    private static string Sha(string text) =>
        Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(System.Text.Encoding.UTF8.GetBytes(text))).ToLowerInvariant();
    private readonly HashSet<string> started = new(StringComparer.Ordinal);
    public string Evidence { get; }
    public string Note { get; }
    // The draft rows the pane needs are ADR-D02's, in data/design/command-rows-d02.json (ResearchRows.AddDrafts, C5-7).
    private readonly Dictionary<string, string> answers = new(StringComparer.Ordinal);

    public SampleResearchSource(string text)
    {
        using JsonDocument d = J.Parse(text);
        JsonElement root = J.Root(d, Format, "evidence", "note", "answers");
        Evidence = J.Text(root, "evidence");
        if (Evidence != Synthetic) throw J.Error("evidence", "the research sample is synthetic");
        Note = J.Text(root, "note");
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
    // The sample's run, as research.run.start answers it: the one run of the sample with the declared seed, depth and
    // budget written in, its covered steps equal to the budget, and its name the declaration's digest.
    public string StartRun(int seed, int depth, long budget)
    {
        if (!SeedPool.Contains(seed) || depth <= 0 || budget <= 0) throw new ArgumentException("the sample's run takes a seed of the pool, a depth and a budget");
        var envelope = System.Text.Json.Nodes.JsonNode.Parse(answers[ResearchRows.Start])!;
        var run = envelope["answer"]!["rows"]![0]!;
        string id = RunId(seed, depth, budget);
        run["run"] = id; run["record"] = ResearchModel.RunRef + id;
        run["seed"] = seed; run["depth"] = depth; run["budget"] = budget;
        run["reading"]!["covered"] = budget.ToString(CultureInfo.InvariantCulture);
        started.Add(id);
        return envelope.ToJsonString();
    }

    // The sample reproduces only a run it started: the same bytes again (DET.7's `same`, with the payload's digest).
    public string Reproduce(string run)
    {
        if (!started.Contains(run)) throw new NotSupportedException("the sample reproduces only a run it started in this session");
        var envelope = System.Text.Json.Nodes.JsonNode.Parse(answers[ResearchRows.Reproduce])!;
        envelope["answer"]!["run"] = run;
        envelope["answer"]!["answer"]!["payload"] = Sha("payload " + run);
        return envelope.ToJsonString();
    }
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

    public DiagnosticState CheckState => Kind switch
    {
        VerdictKind.Fail or VerdictKind.Differs => DiagnosticState.Failed,
        VerdictKind.Pass or VerdictKind.Reproduced => DiagnosticState.Ok,
        _ => DiagnosticState.Pending,
    };

    // KN6's wire verdict names locations, if any, in the claim and reason. Only explicit ids are locations.
    private static readonly Regex Location = new(@"\b(cap|cell|piece) ([0-9]+)\b", RegexOptions.CultureInvariant);

    public IReadOnlyList<DiagnosticCell> NamedCells()
    {
        if (Kind != VerdictKind.Fail) return Array.Empty<DiagnosticCell>();
        var names = new List<DiagnosticCell>();
        foreach (Match match in Location.Matches(Claim + "\n" + Reason))
        {
            if (!int.TryParse(match.Groups[2].Value, NumberStyles.None, CultureInfo.InvariantCulture, out int id)) continue;
            DiagnosticCellKind kind = match.Groups[1].Value switch
            {
                "cap" => DiagnosticCellKind.Cap,
                "cell" => DiagnosticCellKind.Cell,
                _ => DiagnosticCellKind.Piece,
            };
            names.Add(new DiagnosticCell(kind, id));
        }
        return names.Distinct().ToArray();
    }
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
    // B29 (ADR-D02.4, .45): the run's declaration, the two CR-17 controls and History.
    public const string Cr17 = "row not in the command table yet (CR-17)";
    public const string Envelope = "refused past the four-view envelope: four views are open";
    public const string NoBudget = "declare a budget before the run starts (ADR-D02.4)";
    public const string NotInHistory = "History shows no research runs: a run changes no state and records no move";
    public const string DerivedCaption = "derived; reachability open";
    public static string Budget(long steps) => "budget " + steps.ToString("N0", CultureInfo.InvariantCulture) + " steps";

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
        VerdictKind.Fail => new[]
        {
            verdict.Claim is { Length: > 0 } claim ? "first claim that does not hold: " + claim : null,
            verdict.Code is { Length: > 0 } code ? "code: " + code : null,
            verdict.Reason is { Length: > 0 } reason ? "checker's reason: " + reason : null,
        }.OfType<string>().ToArray(),
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
    // B29: the run's declaration and start (the sample's draft rows), reproduction (a table row) and the two CR-17 rows
    // (ADR-D02's draft rows in data/design/command-rows-d02.json).
    public const string Declare = "research.run.declare", Start = "research.run.start", Reproduce = "research.run.reproduce";
    public const string CheckRun = "research.check.run", CheckReport = "research.check.report";
    public const string Trace = "research.check.trace", RunView = "research.run.view";
    public const string ExportObligation = "research.obligation.export";
    public static readonly IReadOnlyList<string> Cr17 = Array.AsReadOnly(new[] { RunView, Trace });
    public static readonly IReadOnlyList<string> All = Array.AsReadOnly(new[]
    {
        Records, Claims, Runs, Obligations, Open, RecordRead, Name, Copy, Save, Declare, Start, Reproduce, Trace, RunView,
    });
    // Rows the command table does not hold yet, all in ADR-D02's draft file data/design/command-rows-d02.json: the row
    // requests of B28 and B29 (record read, declare, start) and the two CR-17 rows (C5-7).
    public static readonly IReadOnlyList<string> Drafts = Array.AsReadOnly(new[] { RecordRead, Declare, Start, Trace, RunView });

    // Registers the research pane's draft rows from command-rows-d02.json whenever the pane is mounted, D02 on or off
    // (C5-5): a row the registry already holds (D02 on registers the whole file) is not added again. Gives the rows,
    // for the palette's draft mark; a draft row missing from the file is refused.
    public static IReadOnlyList<string> AddDrafts(CommandRegistry registry, string d02Rows)
    {
        using JsonDocument d = J.Parse(d02Rows);
        JsonElement[] rows = d.RootElement.GetProperty("commands").EnumerateArray().Where(r => Drafts.Contains(J.Text(r, "id"))).ToArray();
        foreach (string row in Drafts)
            if (!rows.Any(r => J.Text(r, "id") == row)) throw J.Error(row, "a research draft row missing from command-rows-d02.json");
        JsonElement[] missing = rows.Where(r => !registry.Commands.ContainsKey(J.Text(r, "id"))).ToArray();
        if (missing.Length > 0) registry.AddRows(JsonSerializer.Serialize(new { commands = missing }));
        return Drafts;
    }
    // A declaration's argument to research.run.declare: seed=<n>, depth=<n> or budget=<steps>.
    public static readonly IReadOnlyList<string> Fields = Array.AsReadOnly(new[] { "seed", "depth", "budget" });
    public static (string Field, long Value)? ParseField(string? argument)
    {
        int at = argument?.IndexOf('=') ?? -1;
        if (at <= 0 || !Fields.Contains(argument![..at])) return null;
        return long.TryParse(argument[(at + 1)..].Trim().Replace(",", ""), NumberStyles.None, CultureInfo.InvariantCulture, out long value) ? (argument[..at], value) : (argument[..at], -1);
    }
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

// A run as research.runs answers it: its branch (the pinned input) is read-only and never turns stale. B29 adds the
// run's code, the steps its cut-off covered, its checks' verdict kinds and, for a run of research.run.start, the seed,
// depth and budget it recorded. Its Verdict is what its checks said, by the rule that words a claim group's latest check.
public sealed record ResearchRun(string Run, string? Group, string Level, string Digest, string Identity, string Outcome, string Reading,
    string? Code = null, string? Covered = null, IReadOnlyList<string>? Checks = null, int? Seed = null, int? Depth = null, RunBudget? Budget = null,
    ResearchVerdict? Verdict = null)
{
    // ADR-D01.61's budget-stopped state: a run that reached its budget is cut off, never a verdict (ADR-D02.4). A run the
    // researcher cancelled (outcome `cancelled`, reading `cut_off`) is budget-stopped too: ADR-D02.45 and .46 have no
    // words for a run stopped by its researcher (C5-3, a freeze-card item), and it is never a Pass or a Fail.
    public SlotState State => Reading == "cut_off" || Budget is { } b && long.TryParse(Covered, NumberStyles.None, CultureInfo.InvariantCulture, out long c) && c >= b.Steps
        ? SlotState.BudgetStopped : Outcome == "completed" ? SlotState.Ready : SlotState.Error;
    public bool Started => Budget != null;
    public IReadOnlyList<ResearchCheck> CheckItems { get; init; } = Array.Empty<ResearchCheck>();
    public DiagnosticState CheckState => State == SlotState.Error ? DiagnosticState.Failed
        : State == SlotState.Ready ? Verdict?.CheckState ?? DiagnosticState.Pending : DiagnosticState.Pending;
}

public sealed record ResearchCheck(string Checker, string Outcome, ResearchVerdict Verdict)
{
    public DiagnosticState State => Outcome == "completed" ? Verdict.CheckState
        : Outcome == "error" ? DiagnosticState.Failed : DiagnosticState.Pending;
}

// A budget declared before a run starts (ADR-D02.4); the run records it, and a run that reaches it stops as cut off.
public sealed record RunBudget(long Steps)
{
    public string Words => ResearchWords.Budget(Steps);
}

// ADR-D02.52's seed pool, 7401 to 7499: the scored blocks of Track J (J2: 7401 to 7406 at depths 10 and 20; J3: 7407 to
// 7421, three at each of depths 40, 100, 300, 1,000 and 3,000) and free exploration from 7422 to 7499.
public static class SeedPool
{
    public const int First = 7401, Last = 7499, Free = 7422;
    public static readonly IReadOnlyList<(string Batch, int From, int To, int Depth)> Scored = Array.AsReadOnly(new[]
    {
        ("J2", 7401, 7403, 10), ("J2", 7404, 7406, 20), ("J3", 7407, 7409, 40), ("J3", 7410, 7412, 100),
        ("J3", 7413, 7415, 300), ("J3", 7416, 7418, 1000), ("J3", 7419, 7421, 3000),
    });
    public static bool Contains(int seed) => seed >= First && seed <= Last;
    public static (string Batch, int From, int To, int Depth)? Block(int seed) =>
        Scored.Where(b => seed >= b.From && seed <= b.To).Select(b => ((string, int, int, int)?)b).FirstOrDefault();
    public static string Words(long seed) => seed < First || seed > Last
        ? $"seed {seed} is outside the pool {First} to {Last}"
        : Block((int)seed) is { } b
            ? FormattableString.Invariant($"seed {seed}: scored block {b.Batch}, seeds {b.From} to {b.To} at depth {b.Depth:N0}")
            : $"seed {seed}: free exploration ({Free} to {Last}), not scored";
    public const string Line = "seeds 7401 to 7499: J2 scores 7401 to 7406 (depths 10, 20), J3 scores 7407 to 7421 (depths 40, 100, 300, 1,000, 3,000); 7422 to 7499 are free";
}

// What is declared for the next run, field by field, before it starts.
public sealed record RunDeclaration(int? Seed, int? Depth, RunBudget? Budget);

// An obligation as research.obligations answers it: a record-only anchor of a claim group, what it is missing and its
// exports (written with their sha256, or refused with their code).
public sealed record ResearchObligation(string Group, string Family, int Stage, string Statement, string? Label, IReadOnlyList<string> Missing,
    IReadOnlyList<string> Exports)
{
    public string Path => Group[(Group.IndexOf(':') + 1)..].Split('#')[0];
    public string Pointer => Group.Contains('#') ? Group[(Group.IndexOf('#') + 1)..] : "";
}

public enum ResearchLineKind { Stage, Family, Row, Detail, Finding, Part, Actions, Note, Declare }

// One line of the pane's flat list; the pane builds only the lines in view.
public sealed record ResearchLine(ResearchLineKind Kind, string Text, string? Ref = null, int Depth = 0, string? Verdict = null,
    DiagnosticState? Status = null);

// A control of a line: its words and the row it invokes. A control that takes text (a name, a declared field) appends
// it to Argument. A control with a reason (Why) is drawn unavailable, its reason shown.
public sealed record ResearchControl(string Label, string Row, string? Argument, bool TakesText = false, string? Why = null);

// A copy or a save: its row, the object's reference and digest, the origin Research, its note and the evidence class
// of the answers it came from (C5-11: a copy of the synthetic sample says so). A save also names the granted file the
// research host wrote and that file's sha256.
public sealed record ResearchTransfer(string Row, string Ref, string Digest, Origin Origin, string Note, string Evidence,
    string? Destination = null, string? Written = null, string? Text = null);

public sealed class ResearchModel
{
    public const string RecordRef = "research.record/", PartRef = "research.part/", ClaimRef = "research.claim/";
    // B29: a run is named as LP5 records it (research.run/<run>); an obligation by its claim group.
    public const string RunRef = "research.run/", ObligationRef = "research.obligation/";
    public static readonly IReadOnlyList<string> AnswerKeys = Array.AsReadOnly(new[] { "row", "revision", "research", "answer" });

    public IResearchSource Source { get; }
    public string Evidence { get; }
    public string Research { get; private set; } = "";
    public IReadOnlyList<ResearchRecord> Records { get; private set; } = Array.Empty<ResearchRecord>();
    public IReadOnlyList<ResearchClaim> Claims { get; private set; } = Array.Empty<ResearchClaim>();
    // The runs research.runs answers, and before them the runs this session started (research.run.start), newest last.
    public IReadOnlyList<ResearchRun> Runs { get; private set; } = Array.Empty<ResearchRun>();
    public IReadOnlyList<ResearchRun> Started => started;
    public IReadOnlyList<ResearchObligation> Obligations { get; private set; } = Array.Empty<ResearchObligation>();
    // B29: what is declared for the next run, each run's latest reproduction, and how many views are open (the host's
    // count; a fifth view is refused by the four-view envelope, ViewPanes.MaxPanes).
    public RunDeclaration Declared { get; private set; } = new(null, null, null);
    public IReadOnlyDictionary<string, ResearchVerdict> Reproductions => reproductions;
    public Func<int> OpenViews { get; set; } = () => 1;
    private readonly List<ResearchRun> started = new();
    private readonly Dictionary<string, ResearchVerdict> reproductions = new(StringComparer.Ordinal);
    public ResearchTab Tab { get; private set; } = ResearchTab.Claims;
    public string? Open { get; private set; }
    public ResearchTransfer? Transfer { get; private set; }
    public event Action? Changed;
    public event Action? DiagnosticsChanged;
    private readonly Dictionary<string, string> names = new(StringComparer.Ordinal);
    // The session's changes seen, and the change at which each row was last asked (ONE.12's current and stale).
    private long head;
    private readonly Dictionary<string, long> asked = new(StringComparer.Ordinal);
    private readonly Action<string>? copyText;

    public ResearchModel(IResearchSource source, string evidence, Action<string>? copyText = null)
    {
        Source = source; Evidence = evidence; this.copyText = copyText;
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
            case ResearchRows.Obligations: Obligations = Answer(row, Source.Obligations()).GetProperty("rows").EnumerateArray().Select(ReadObligation).ToArray(); break;
        }
        asked[row] = head;
        if (row is ResearchRows.Claims or ResearchRows.Runs) Safe.Raise(DiagnosticsChanged);
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
        JsonElement branch = row.GetProperty("branch"), reading = row.GetProperty("reading");
        JsonElement[] checkRows = row.TryGetProperty("checks", out JsonElement list) ? list.EnumerateArray().ToArray() : Array.Empty<JsonElement>();
        JsonElement[] verdicts = checkRows.Select(c => c.GetProperty("verdict")).ToArray();
        string[] checks = verdicts.Select(v => v.ValueKind == JsonValueKind.Object ? J.Text(v, "kind") : "none").ToArray();
        int? Number(string id) => row.TryGetProperty(id, out JsonElement n) && n.ValueKind == JsonValueKind.Number ? n.GetInt32() : null;
        RunBudget? budget = row.TryGetProperty("budget", out JsonElement b) && b.ValueKind == JsonValueKind.Number ? new RunBudget(b.GetInt64()) : null;
        return new ResearchRun(J.Text(row, "run"), Optional(row, "group"), J.Text(branch, "level"), J.Text(branch, "digest"), J.Text(branch, "identity"),
            J.Text(row, "outcome"), J.Text(reading, "kind"), Optional(row, "code"), Optional(reading, "covered"), checks, Number("seed"), Number("depth"), budget,
            CheckVerdict(reading, verdicts))
        {
            CheckItems = checkRows.Select((check, i) => new ResearchCheck(
                check.TryGetProperty("checker", out JsonElement checker) ? J.Text(checker, "id") : Optional(check, "kind") ?? "Check",
                Optional(check, "outcome") ?? "completed", ReadCheckVerdict(verdicts[i]))).ToArray(),
        };
    }

    // research.check.run's children carry the same verdict object as research.check.report's report.
    private static ResearchVerdict ReadCheckVerdict(JsonElement entry)
    {
        if (entry.ValueKind != JsonValueKind.Object) return new ResearchVerdict(VerdictKind.NotChecked);
        return J.Text(entry, "kind") switch
        {
            "fail" => new ResearchVerdict(VerdictKind.Fail, Optional(entry, "claim"), Optional(entry, "code"), Optional(entry, "reason")),
            "pass" => new ResearchVerdict(VerdictKind.Pass),
            "not_applicable" => new ResearchVerdict(VerdictKind.NotApplicable, Reason: J.Text(entry, "reason")),
            _ => new ResearchVerdict(VerdictKind.NotChecked),
        };
    }

    public IEnumerable<(string Source, string Label, DiagnosticState State, IReadOnlyList<DiagnosticCell> Names, string? PoseDigest)> Diagnostics()
    {
        var runs = Started.Concat(Runs).DistinctBy(r => r.Run).ToArray();
        foreach (ResearchClaim claim in Claims)
            yield return (Ref(claim), Shown(claim), claim.Verdict.CheckState, claim.Verdict.NamedCells(),
                runs.FirstOrDefault(r => r.Run == claim.Run && r.Level == "poses")?.Digest);
        foreach (ResearchRun run in runs)
        {
            string? digest = run.Level == "poses" ? run.Digest : null;
            yield return (ResearchRows.CheckRun + "/" + run.Run, Shown(run), run.CheckState,
                run.Verdict?.NamedCells() ?? Array.Empty<DiagnosticCell>(), digest);
            for (int i = 0; i < run.CheckItems.Count; i++)
            {
                ResearchCheck check = run.CheckItems[i];
                yield return (ResearchRows.CheckReport + "/" + run.Run + "/" + i.ToString(CultureInfo.InvariantCulture),
                    check.Checker, check.State, check.Verdict.NamedCells(), digest);
            }
            if (reproductions.TryGetValue(run.Run, out var reproduced))
                yield return (ResearchRows.Reproduce + "/" + run.Run, "Reproduce " + Shown(run), reproduced.CheckState,
                    Array.Empty<DiagnosticCell>(), null);
        }
    }

    private static ResearchObligation ReadObligation(JsonElement row) =>
        new(J.Text(row, "group"), J.Text(row, "family"), row.GetProperty("stage").GetInt32(), J.Text(row, "statement"), Optional(row, "label"),
            row.GetProperty("missing").EnumerateArray().Select(m => m.GetString() ?? throw J.Error("missing", "expected text")).ToArray(),
            row.GetProperty("exports").EnumerateArray().Select(e => J.Text(e.GetProperty("result"), "kind") == "written"
                ? "written to " + J.Text(e, "destination") + " · sha256 " + J.Text(e.GetProperty("result"), "sha256")
                : "refused for " + J.Text(e, "destination") + " · " + J.Text(e.GetProperty("result"), "code")).ToArray());

    // DET.7's answer as research.run.reproduce gives it (answers.rs, reproduction_json), in ADR-D02.46's words: `same`
    // is "reproduced: bytes equal"; a mismatch names its first differing byte (the byte is B29's row request: LP5's
    // refusal LG4:evidence.reproducibility_mismatch names none yet); `not_run` and `refused` are no reproduction.
    public const string Mismatch = "LG4:evidence.reproducibility_mismatch";
    public static ResearchVerdict ReproductionOf(JsonElement answer) => J.Text(answer, "kind") switch
    {
        "same" => new ResearchVerdict(VerdictKind.Reproduced),
        "refused" when J.Text(answer, "code") == Mismatch && answer.TryGetProperty("byte", out JsonElement at) && at.ValueKind == JsonValueKind.Number =>
            new ResearchVerdict(VerdictKind.Differs, Byte: at.GetInt64()),
        string kind => new ResearchVerdict(VerdictKind.NotChecked, Reason: "not reproduced: " + kind + " " + Optional(answer, "code")),
    };

    private static string? Optional(JsonElement row, string id) =>
        row.TryGetProperty(id, out JsonElement value) && value.ValueKind == JsonValueKind.String ? value.GetString() : null;

    private static IReadOnlyList<ResearchPart> Parts(JsonElement list) =>
        list.EnumerateArray().Select(p => new ResearchPart(J.Text(p, "name"), p.GetProperty("length").GetInt64(), Parts(p.GetProperty("parts")))).ToArray();

    // The words a claim group's row shows: its latest check (a cut-off first: never a verdict), else its preflight.
    public static ResearchVerdict VerdictOf(JsonElement row)
    {
        JsonElement latest = row.GetProperty("latest_check");
        if (latest.ValueKind != JsonValueKind.Null)
            return CheckVerdict(latest.GetProperty("reading"), latest.GetProperty("verdicts").EnumerateArray().ToArray());
        JsonElement preflight = row.GetProperty("preflight");
        if (J.Text(preflight, "kind") == "ready") return new ResearchVerdict(VerdictKind.NotChecked, Reason: "preflight: ready; the check has not run");
        string[] reasons = J.Strings(preflight.GetProperty("reasons"), "reasons");
        string[] dependencies = preflight.GetProperty("dependencies").EnumerateArray().Select(Dependency).ToArray();
        return new ResearchVerdict(VerdictKind.NotChecked, Reasons: reasons, Dependencies: dependencies);
    }

    // What a check said, from its reading and its checks' verdicts: a claim group's latest check, or a run's checks.
    // A check stopped by its own cap keeps no verdict and no report (answers.rs gives null; job.rs, Job::reading), so a
    // null entry adds nothing to evidence: the check is a pass only when every entry is an object of kind `pass`, and
    // any null entry leaves it not checked (C5-1). A Fail is still the first claim that does not hold.
    public static ResearchVerdict CheckVerdict(JsonElement reading, IReadOnlyList<JsonElement> entries)
    {
        switch (J.Text(reading, "kind"))
        {
            case "cut_off": return new ResearchVerdict(VerdictKind.CutOff, Reason: "covered " + J.Text(reading, "covered"));
            case "not_run": return new ResearchVerdict(VerdictKind.NotChecked, Reason: "the check did not run");
            case "open": return new ResearchVerdict(VerdictKind.NotChecked, Reason: "open: " + J.Text(reading, "code"));
        }
        JsonElement[] verdicts = entries.Where(v => v.ValueKind == JsonValueKind.Object).ToArray();
        if (verdicts.FirstOrDefault(v => J.Text(v, "kind") == "fail") is { ValueKind: JsonValueKind.Object } fail)
            return ReadCheckVerdict(fail);
        if (verdicts.Length != entries.Count) return new ResearchVerdict(VerdictKind.NotChecked, Reason: "a check reached no report");
        if (verdicts.FirstOrDefault(v => J.Text(v, "kind") == "not_applicable") is { ValueKind: JsonValueKind.Object } na)
            return new ResearchVerdict(VerdictKind.NotApplicable, Reason: J.Text(na, "reason"));
        if (verdicts.Length > 0 && verdicts.All(v => J.Text(v, "kind") == "pass")) return new ResearchVerdict(VerdictKind.Pass);
        return new ResearchVerdict(VerdictKind.NotChecked, Reason: "the check reached no report");
    }

    // A dependency word for word: its kind, then every field LP5 gives it (answers.rs: a missing input's name, source
    // and cites; an input's path, state and sha256; a historical gate's source and pointer).
    private static string Dependency(JsonElement d) => J.Text(d, "kind") switch
    {
        "missing_input" => "missing_input: " + J.Text(d, "name") + " · source " + J.Text(d, "source") + " · cites "
            + (d.GetProperty("cites").EnumerateArray().Select(c => c.GetString() ?? throw J.Error("cites", "expected text")).ToArray() is { Length: > 0 } cites
                ? string.Join(", ", cites) : "none"),
        "input" => "input: " + J.Text(d, "path") + " (" + J.Text(d, "state") + ") · sha256 " + J.Text(d, "sha256"),
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

    // --- B29: a run's declaration, its start and its reproduction (ADR-D02.4, .45, .46, .50, .52) ---------------------

    // Declares one field of the next run before it starts: a seed of the pool (its words name the scored block), a
    // depth, or the budget. A seed outside the pool, or a field that is no positive number, is refused.
    public (bool Done, string Words) Declare(string? argument)
    {
        if (ResearchRows.ParseField(argument) is not { } parsed) return (false, "declare seed=<7401 to 7499>, depth=<twists> or budget=<steps>");
        var (field, value) = parsed;
        if (value <= 0 || value > int.MaxValue && field != "budget") return (false, $"{field} takes a positive whole number");
        if (field == "seed" && !SeedPool.Contains((int)value)) return (false, SeedPool.Words(value));
        Declared = field switch
        {
            "seed" => Declared with { Seed = (int)value },
            "depth" => Declared with { Depth = (int)value },
            _ => Declared with { Budget = new RunBudget(value) },
        };
        Tab = ResearchTab.Runs; Ask(ResearchRows.Runs);
        Raise();
        return (true, "declared " + DeclaredLine());
    }

    public string DeclaredLine() =>
        string.Join(" · ", new[]
        {
            Declared.Seed is int s ? SeedPool.Words(s) : "seed: not declared",
            Declared.Depth is int d ? "depth " + d.ToString("N0", CultureInfo.InvariantCulture) : "depth: not declared",
            Declared.Budget?.Words ?? "budget: not declared",
        });

    // Why the next run cannot start, or null: it needs its budget declared first, then a seed and a depth.
    public string? StartRefusal() => Declared.Budget == null ? ResearchWords.NoBudget
        : Declared.Seed == null ? "declare a seed of the pool before the run starts"
        : Declared.Depth == null ? "declare a depth before the run starts" : null;

    // Starts the declared run through research.run.start. The run records its budget; one that reached it is
    // budget-stopped ("not found within budget"), never a verdict. Nothing is committed: History shows no run.
    public (bool Done, string Words) Start()
    {
        if (StartRefusal() is { } why) return (false, why);
        var (seed, depth, budget) = (Declared.Seed!.Value, Declared.Depth!.Value, Declared.Budget!);
        JsonElement answer = Answer(ResearchRows.Start, Source.StartRun(seed, depth, budget.Steps));
        ResearchRun run = ReadRun(answer.GetProperty("rows")[0]);
        if (run.Budget != budget || run.Seed != seed || run.Depth != depth) throw J.Error(ResearchRows.Start, "the run did not record the declared seed, depth and budget");
        started.Add(run);
        Tab = ResearchTab.Runs; Ask(ResearchRows.Runs); Open = RunRef + run.Run;
        Raise();
        return (true, $"run {Short(run.Run)}: {RunLine(run)}");
    }

    // Reproduces a run through research.run.reproduce, and words its answer.
    public (bool Done, string Words) Reproduce(string reference)
    {
        if (RunOf(reference) is not { } run) return (false, "no research run " + reference);
        JsonElement answer;
        try { answer = Answer(ResearchRows.Reproduce, Source.Reproduce(run.Run)); }
        catch (NotSupportedException e) { return (false, "not reproduced: " + e.Message); }
        if (J.Text(answer, "run") != run.Run) throw J.Error(ResearchRows.Reproduce, "the answer names another run");
        ResearchVerdict verdict = ReproductionOf(answer.GetProperty("answer"));
        reproductions[run.Run] = verdict;
        Safe.Raise(DiagnosticsChanged);
        if (Open != reference) Open = reference;
        Raise();
        return (verdict.Kind is VerdictKind.Reproduced or VerdictKind.Differs, $"run {Short(run.Run)}: {ReproductionLine(verdict)}");
    }

    public static string ReproductionLine(ResearchVerdict verdict) =>
        verdict.Kind == VerdictKind.NotChecked ? verdict.Reason ?? ResearchWords.NotChecked : ResearchWords.VerdictLine(verdict, "");

    // Why a research row is unavailable, or null. The two CR-17 rows wait for the command table; opening a derived
    // state is also refused past the four-view envelope. A start waits for its declaration.
    public string? Unavailable(string row, string? argument) => row switch
    {
        ResearchRows.RunView => OpenViews() >= ViewPanes.MaxPanes ? ResearchWords.Cr17 + "; " + ResearchWords.Envelope : ResearchWords.Cr17,
        ResearchRows.Trace => ResearchWords.Cr17,
        ResearchRows.Start => StartRefusal(),
        _ => null,
    };

    // A run's line: budget-stopped is "not found within budget", never a verdict; a completed run words its checks as
    // the Claims tab words its claim group (C5-2): the same rule, in the group's scope phrase. A run with no claim group
    // has no scope to word a verdict in, so it reads "not checked" and its expanded row says it names no claim group.
    public string RunLine(ResearchRun run)
    {
        if (run.State == SlotState.BudgetStopped) return ResearchWords.CutOff;
        if (run.State != SlotState.Ready) return "stopped: " + run.Outcome + (run.Code is { } code ? " · " + code : "");
        return ClaimOfRun(run) is { } claim ? ResearchWords.VerdictLine(run.Verdict ?? new ResearchVerdict(VerdictKind.NotChecked), claim.Scope) : ResearchWords.NotChecked;
    }

    private ResearchClaim? ClaimOfRun(ResearchRun run) => run.Group == null ? null : Claims.FirstOrDefault(c => c.Group == run.Group);

    private static string Many(int count, string word) => count + " " + word + (count == 1 ? "" : "s");

    private static string Short(string id) => id[..Math.Min(12, id.Length)];

    // The run's branch: its pinned input, read-only (ADR-D02.45).
    public IReadOnlyList<string> PinnedLines(string run)
    {
        ResearchRun r = RunOf(RunRef + run) ?? throw new ArgumentException("no run " + run);
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

    public ResearchTransfer Copy(string reference)
    {
        if (!Known(reference)) throw new ArgumentException("no research object " + reference);
        ResearchLine? row = Lines(TabOfRef(reference)).FirstOrDefault(line =>
            line.Kind is ResearchLineKind.Row or ResearchLineKind.Part && line.Ref == reference);
        if (row == null)
        {
            // A part named from the palette can be under a folded branch.
            var record = RecordOf(reference)!;
            int[] path = PathOf(reference);
            row = PartLine(record, PartAt(record, path)!, path, false);
        }
        string text = row.Verdict is { Length: > 0 } verdict ? row.Text + "\n" + verdict : row.Text;
        copyText?.Invoke(text);
        return Carry(new ResearchTransfer(ResearchRows.Copy, reference, Digest(reference), Origin.Research, ResearchWords.CopyNote, Evidence, Text: text));
    }

    // Saves a record as research.bundle.export does (C5-6): LP5's check bundle of the record, written to a granted file
    // named for it. Only the research host writes one; when the source refuses the row (the sample does), nothing is
    // saved, the carried line stays as it was and the words say why.
    public (bool Done, string Words) Save(string reference)
    {
        if (RecordOf(reference) is not { } record) return (false, "a bundle saves records");
        string destination = BundleFile(record);
        JsonElement answer;
        try { answer = Answer(ResearchRows.Save, Source.ExportBundle(new[] { record.Name }, destination)); }
        catch (NotSupportedException e) { return (false, "not saved: " + e.Message); }
        Carry(new ResearchTransfer(ResearchRows.Save, reference, Digest(reference), Origin.Research, ResearchWords.CopyNote, Evidence,
            J.Text(answer, "destination"), J.Text(answer, "sha256")));
        return (true, TransferLine());
    }

    // The granted file a record's check bundle is written to: the record's own file name, as a bundle.
    public static string BundleFile(ResearchRecord record) =>
        System.IO.Path.GetFileNameWithoutExtension(record.Name) + ".check-bundle.zip";

    private ResearchTransfer Carry(ResearchTransfer transfer)
    {
        Transfer = transfer;
        Raise();
        return transfer;
    }

    public string TransferLine() => Transfer is not { } t ? ""
        : (t.Row == ResearchRows.Copy ? "copied " + Shown(t.Ref) : $"saved {Shown(t.Ref)} to {t.Destination} · sha256 {t.Written}")
            + $" · origin {t.Origin} · evidence {t.Evidence} · {t.Note}";

    // A record's digest is its bytes' SHA-256; a part's is its record's; a claim group's is its statement's id.
    // A run's is its run digest (LP5's research.run/<run>); an obligation's its statement's id.
    public string Digest(string reference) =>
        reference.StartsWith(ClaimRef, StringComparison.Ordinal) ? ClaimOf(reference)!.Statement
        : reference.StartsWith(RunRef, StringComparison.Ordinal) ? RunOf(reference)!.Run
        : reference.StartsWith(ObligationRef, StringComparison.Ordinal) ? ObligationOf(reference)!.Statement : RecordOf(reference)!.Sha256;

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
        if (line.Kind == ResearchLineKind.Declare)
            return new[]
            {
                new ResearchControl("Seed", ResearchRows.Declare, "seed=", true), new ResearchControl("Depth", ResearchRows.Declare, "depth=", true),
                new ResearchControl("Budget", ResearchRows.Declare, "budget=", true), new ResearchControl("Start", ResearchRows.Start, null, Why: StartRefusal()),
            };
        if (line.Ref is not { } reference) return Array.Empty<ResearchControl>();
        if (line.Kind == ResearchLineKind.Actions && reference.StartsWith(RunRef, StringComparison.Ordinal))
            return new[]
            {
                new ResearchControl("Reproduce", ResearchRows.Reproduce, reference),
                new ResearchControl("Open derived state in a free view", ResearchRows.RunView, reference, Why: Unavailable(ResearchRows.RunView, reference)),
                new ResearchControl("Step the checker", ResearchRows.Trace, reference, Why: Unavailable(ResearchRows.Trace, reference)),
                new ResearchControl("Copy", ResearchRows.Copy, reference),
            };
        if (line.Kind == ResearchLineKind.Actions && reference.StartsWith(ObligationRef, StringComparison.Ordinal))
            return new[] { new ResearchControl("Copy", ResearchRows.Copy, reference) };
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
            argument != null && new[] { RecordRef, PartRef, ClaimRef, RunRef, ObligationRef }.Any(p => argument.StartsWith(p, StringComparison.Ordinal)),
        ResearchRows.Name => ResearchRows.ParseName(argument) != null,
        ResearchRows.Declare => argument != null,
        ResearchRows.Start => argument == null,
        ResearchRows.Reproduce or ResearchRows.Trace or ResearchRows.RunView => argument != null && argument.StartsWith(RunRef, StringComparison.Ordinal),
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
        if (Unavailable(row, argument) is { } why) return (false, why);
        if (row == ResearchRows.Declare) return Declare(argument);
        if (row == ResearchRows.Start) return Start();
        if (row == ResearchRows.Reproduce) return Reproduce(argument!);
        string target = argument!;
        if (!Known(target)) return (false, "no research object " + target);
        switch (row)
        {
            case ResearchRows.RecordRead when !target.StartsWith(RecordRef, StringComparison.Ordinal): return (false, "research.record.read reads a record");
            case ResearchRows.RecordRead or ResearchRows.Open: Toggle(target); return (true, (Open == target ? "opened " : "folded ") + Shown(target));
            case ResearchRows.Copy: Copy(target); return (true, TransferLine());
            case ResearchRows.Save when !target.StartsWith(RecordRef, StringComparison.Ordinal): return (false, "a bundle saves records");
            default: return Save(target);
        }
    }

    // --- references --------------------------------------------------------------------------------------------------

    public static string Ref(ResearchRecord record) => RecordRef + record.Name;
    public static string Ref(ResearchClaim claim) => ClaimRef + claim.Group;
    public static string PartOf(ResearchRecord record, IEnumerable<int> path) => PartRef + record.Name + "@" + string.Join(".", path);

    public static string Ref(ResearchRun run) => RunRef + run.Run;
    public static string Ref(ResearchObligation obligation) => ObligationRef + obligation.Group;

    private bool Known(string reference) => reference.StartsWith(ClaimRef, StringComparison.Ordinal) ? ClaimOf(reference) != null
        : reference.StartsWith(RunRef, StringComparison.Ordinal) ? RunOf(reference) != null
        : reference.StartsWith(ObligationRef, StringComparison.Ordinal) ? ObligationOf(reference) != null
        : reference.StartsWith(PartRef, StringComparison.Ordinal) ? RecordOf(reference) is { } r && PartAt(r, PathOf(reference)) != null
        : RecordOf(reference) != null;

    private ResearchClaim? ClaimOf(string reference) => Claims.FirstOrDefault(c => Ref(c) == reference);
    public ResearchRun? RunOf(string reference) => started.Concat(Runs).FirstOrDefault(r => Ref(r) == reference);
    private ResearchObligation? ObligationOf(string reference) => Obligations.FirstOrDefault(o => Ref(o) == reference);

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

    private static ResearchTab TabOfRef(string reference) => reference.StartsWith(ClaimRef, StringComparison.Ordinal) ? ResearchTab.Claims
        : reference.StartsWith(RunRef, StringComparison.Ordinal) ? ResearchTab.Runs
        : reference.StartsWith(ObligationRef, StringComparison.Ordinal) ? ResearchTab.Obligations : ResearchTab.Records;

    // The name a record shows: the researcher's, else the file name its archive path ends in.
    public string Shown(ResearchRecord record) => names.TryGetValue(Ref(record), out string? n) ? n : record.Name[(record.Name.LastIndexOf('/') + 1)..];

    public string Shown(ResearchClaim claim)
    {
        ResearchRecord? record = Records.FirstOrDefault(r => r.Name == claim.Path);
        string name = record != null ? Shown(record) : claim.Path[(claim.Path.LastIndexOf('/') + 1)..];
        return claim.Pointer.Length > 0 ? name + " #" + claim.Pointer : name;
    }

    public string Shown(ResearchRun run) => "run " + Short(run.Run) + (run.Seed is int seed ? FormattableString.Invariant($" · seed {seed} · depth {run.Depth:N0}") : "");

    public string Shown(ResearchObligation obligation)
    {
        ResearchRecord? record = Records.FirstOrDefault(r => r.Name == obligation.Path);
        string name = record != null ? Shown(record) : obligation.Path[(obligation.Path.LastIndexOf('/') + 1)..];
        return obligation.Pointer.Length > 0 ? name + " #" + obligation.Pointer : name;
    }

    public string Shown(string reference)
    {
        if (ClaimOf(reference) is { } claim) return Shown(claim);
        if (RunOf(reference) is { } run) return Shown(run);
        if (ObligationOf(reference) is { } obligation) return Shown(obligation);
        ResearchRecord? record = RecordOf(reference);
        if (record == null) return reference;
        if (!reference.StartsWith(PartRef, StringComparison.Ordinal)) return Shown(record);
        int[] path = PathOf(reference);
        return PartAt(record, path) is { } part ? Shown(record) + " › " + Shown(record, part, path) : reference;
    }

    private string Shown(ResearchRecord record, ResearchPart part, int[] path) =>
        names.TryGetValue(PartOf(record, path), out string? n) ? n : part.Name;

    // --- the flat list -----------------------------------------------------------------------------------------------

    public IReadOnlyList<ResearchLine> Lines() => Lines(Tab);

    private IReadOnlyList<ResearchLine> Lines(ResearchTab tab)
    {
        var lines = new List<ResearchLine>();
        switch (tab)
        {
            case ResearchTab.Claims: ClaimLines(lines); break;
            case ResearchTab.Records: RecordLines(lines); break;
            case ResearchTab.Runs: RunLines(lines); break;
            case ResearchTab.Obligations: ObligationLines(lines); break;
        }
        return lines;
    }

    // B29: the declaration, then this session's runs as they started, then research.runs' runs by stage and family.
    private void RunLines(List<ResearchLine> lines)
    {
        lines.Add(new ResearchLine(ResearchLineKind.Note, "A run's budget is declared before it starts; a run that reaches it stops as cut off, never a verdict. " + ResearchWords.NotInHistory + "."));
        lines.Add(new ResearchLine(ResearchLineKind.Note, SeedPool.Line));
        lines.Add(new ResearchLine(ResearchLineKind.Declare, "next run: " + DeclaredLine(), Verdict: StartRefusal() ?? "ready to start"));
        if (started.Count > 0)
        {
            lines.Add(new ResearchLine(ResearchLineKind.Stage, "This session · " + Many(started.Count, "run")));
            foreach (ResearchRun run in started) RunRow(lines, run);
        }
        int? stage = -1;
        foreach (var group in Groups(Runs, r => StageOf(r.Group), r => FamilyOf(r.Group)))
        {
            if (group.Key.Stage != stage)
            {
                stage = group.Key.Stage;
                lines.Add(new ResearchLine(ResearchLineKind.Stage, (stage is int s ? $"Stage {s}" : "No claim group") + " · " + Many(Runs.Count(r => StageOf(r.Group) == stage), "run")));
            }
            lines.Add(new ResearchLine(ResearchLineKind.Family, $"{group.Key.Family} · {group.Count()}"));
            foreach (ResearchRun run in group) RunRow(lines, run);
        }
    }

    private static int? StageOf(string? group) =>
        group != null && int.TryParse(group.Split('/')[0], NumberStyles.None, CultureInfo.InvariantCulture, out int s) ? s : null;

    private static string FamilyOf(string? group) => group?.Split(':')[0] ?? "no family";

    private void RunRow(List<ResearchLine> lines, ResearchRun run)
    {
        string reference = Ref(run);
        bool open = Open == reference;
        lines.Add(new ResearchLine(ResearchLineKind.Row, (open ? "▾ " : "▸ ") + Shown(run), reference, 0, RunLine(run), run.CheckState));
        if (!open) return;
        if (run.Seed is int seed) lines.Add(new ResearchLine(ResearchLineKind.Detail, SeedPool.Words(seed), reference, 1));
        if (run.Budget is { } budget)
            lines.Add(new ResearchLine(ResearchLineKind.Detail, FormattableString.Invariant($"depth {run.Depth:N0} · {budget.Words}, declared before the start and recorded"), reference, 1));
        if (run.State == SlotState.BudgetStopped)
            foreach (string line in ResearchWords.Expanded(new ResearchVerdict(VerdictKind.CutOff, Reason: run.Covered is { } c ? "covered " + c : null)))
                lines.Add(new ResearchLine(ResearchLineKind.Detail, line, reference, 1));
        else if (run.Verdict is { Kind: VerdictKind.Fail } failed)
            foreach (string line in ResearchWords.Expanded(failed)) lines.Add(new ResearchLine(ResearchLineKind.Detail, line, reference, 1));
        if (run.State == SlotState.Ready && ClaimOfRun(run) == null)
            lines.Add(new ResearchLine(ResearchLineKind.Detail, "no claim group: its checks have no scope to be worded in", reference, 1));
        if (run.Code != null) lines.Add(new ResearchLine(ResearchLineKind.Detail, "code: " + run.Code, reference, 1));
        if (run.Checks is { Count: > 0 } checks)
            lines.Add(new ResearchLine(ResearchLineKind.Detail, "checks: " + string.Join(", ", checks.GroupBy(c => c).Select(g => $"{g.Key} {g.Count()}")), reference, 1));
        foreach (ResearchCheck check in run.CheckItems)
            lines.Add(new ResearchLine(ResearchLineKind.Detail, check.Checker, reference, 1, Status: check.State));
        foreach (string line in PinnedLines(run.Run)) lines.Add(new ResearchLine(ResearchLineKind.Detail, line, reference, 1));
        lines.Add(new ResearchLine(ResearchLineKind.Detail, reproductions.TryGetValue(run.Run, out ResearchVerdict? again)
            ? ReproductionLine(again) : "not reproduced yet: Reproduce asks " + ResearchRows.Reproduce, reference, 1,
            Status: again?.CheckState ?? DiagnosticState.Pending));
        lines.Add(new ResearchLine(ResearchLineKind.Detail, $"Open derived state in a free view (captioned \"{ResearchWords.DerivedCaption}\"), Step the checker: "
            + Unavailable(ResearchRows.RunView, reference), reference, 1));
        lines.Add(new ResearchLine(ResearchLineKind.Actions, "", reference, 1));
    }

    // B29: the obligations, record-only anchors of claim groups, by stage and family, each with what it misses and its
    // exports (research.obligation.export writes one).
    private void ObligationLines(List<ResearchLine> lines)
    {
        lines.Add(new ResearchLine(ResearchLineKind.Note, Many(Obligations.Count, "open obligation") + ": record-only anchors of claim groups, with what each misses and its exports."));
        int? stage = -1;
        foreach (var group in Groups(Obligations, o => o.Stage, o => o.Family))
        {
            if (group.Key.Stage != stage)
            {
                stage = group.Key.Stage;
                lines.Add(new ResearchLine(ResearchLineKind.Stage, $"Stage {stage} · " + Many(Obligations.Count(o => o.Stage == stage), "obligation")));
            }
            lines.Add(new ResearchLine(ResearchLineKind.Family, $"{group.Key.Family} · {group.Count()}"));
            foreach (ResearchObligation obligation in group)
            {
                string reference = Ref(obligation);
                bool open = Open == reference;
                string status = (obligation.Missing.Count == 0 ? "record only · nothing missing" : $"record only · missing {obligation.Missing.Count}")
                    + (obligation.Exports.Count == 0 ? " · not exported" : $" · exports {obligation.Exports.Count}");
                lines.Add(new ResearchLine(ResearchLineKind.Row, (open ? "▾ " : "▸ ") + Shown(obligation), reference, 0, status));
                if (!open) continue;
                foreach (string missing in obligation.Missing) lines.Add(new ResearchLine(ResearchLineKind.Detail, "missing: " + missing, reference, 1));
                if (obligation.Label != null) lines.Add(new ResearchLine(ResearchLineKind.Detail, "label: " + obligation.Label, reference, 1));
                lines.Add(new ResearchLine(ResearchLineKind.Detail, "statement " + Short(obligation.Statement), reference, 1));
                foreach (string export in obligation.Exports.DefaultIfEmpty("exports: none; " + ResearchRows.ExportObligation + " writes one"))
                    lines.Add(new ResearchLine(ResearchLineKind.Detail, export, reference, 1));
                lines.Add(new ResearchLine(ResearchLineKind.Actions, "", reference, 1));
            }
        }
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
                lines.Add(new ResearchLine(ResearchLineKind.Row, (open ? "▾ " : "▸ ") + Shown(claim), reference, 0,
                    ResearchWords.VerdictLine(claim.Verdict, claim.Scope), claim.Verdict.CheckState));
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
            lines.Add(PartLine(record, part, path, open));
            if (open) PartLines(lines, record, part.Parts, path, unfolded);
        }
    }

    private ResearchLine PartLine(ResearchRecord record, ResearchPart part, int[] path, bool open) =>
        new(ResearchLineKind.Part,
            (part.Parts.Count == 0 ? "· " : open ? "▾ " : "▸ ") + Shown(record, part, path) + " · " + part.Length.ToString("N0", CultureInfo.InvariantCulture) + " twists",
            PartOf(record, path), path.Length);

    public static long Length(IReadOnlyList<ResearchPart> parts) => parts.Sum(p => p.Length);

    private void Raise() => Safe.Raise(Changed);
}
