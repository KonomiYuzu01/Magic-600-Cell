using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Magic600.Chrome;

// B28 (ADR-D02.3, .44 to .46, .49): the research pane's model on the synthetic sample (data/samples/research/panel.json):
// the verdict words, what counts as a check, the pinned input, the researcher's names and the rows the controls invoke.
internal static partial class Program
{
    private static ResearchModel ResearchSample() => ResearchModel.FromSample(Read("data/samples/research/panel.json"));

    private static ResearchClaim ClaimOfKind(ResearchModel model, VerdictKind kind) =>
        model.Claims.FirstOrDefault(c => c.Verdict.Kind == kind) ?? throw new InvalidOperationException("the sample has no " + kind + " claim group");

    private static void D02VerdictWordsExact()
    {
        const string scope = "on one instance";
        Equal(ResearchWords.VerdictLine(new ResearchVerdict(VerdictKind.Pass), scope), "holds within on one instance", "Pass");
        Equal(ResearchWords.VerdictLine(new ResearchVerdict(VerdictKind.Fail), scope), "fails within on one instance", "Fail");
        Equal(ResearchWords.VerdictLine(new ResearchVerdict(VerdictKind.NotApplicable), scope), "not applicable (scope only)", "NotApplicable");
        Equal(ResearchWords.VerdictLine(new ResearchVerdict(VerdictKind.NotChecked), scope), "not checked", "not checked");
        Equal(ResearchWords.VerdictLine(new ResearchVerdict(VerdictKind.CutOff), scope), "not found within budget", "cut-off");
        Equal(ResearchWords.VerdictLine(new ResearchVerdict(VerdictKind.Reproduced), scope), "reproduced: bytes equal", "reproduced");
        Equal(ResearchWords.VerdictLine(new ResearchVerdict(VerdictKind.Differs, Byte: 4096), scope), "differs at byte 4096", "differs");
        Equal(ResearchWords.VerdictLine(new ResearchVerdict(VerdictKind.Replayed), scope), "replayed from record, not re-executed", "replayed");
        Equal(ResearchWords.VerdictLine(new ResearchVerdict(VerdictKind.ModelReview), scope), "reviewed by a model, not checked", "model review");
        foreach (VerdictKind kind in Enum.GetValues<VerdictKind>()) Check(ResearchWords.ColourRole(kind) == null, kind + " has a colour role");
        // The sample's Fail row: its line, and on expand the first claim that does not hold, its code and the reason.
        ResearchModel model = ResearchSample();
        ResearchClaim fail = ClaimOfKind(model, VerdictKind.Fail);
        string reference = ResearchModel.Ref(fail);
        Check(model.Toggle(reference) && model.Open == reference, "the Fail row does not open");
        List<ResearchLine> lines = model.Lines().Where(l => l.Ref == reference).ToList();
        Equal(lines[0].Verdict, "fails within " + fail.Scope, "the Fail row's verdict line");
        Sequence(lines.Where(l => l.Kind == ResearchLineKind.Detail).Take(3).Select(l => l.Text),
            new[] { "first claim that does not hold: cycle.closes", "code: KN6:evidence.certificate_rejected", "checker's reason: " + fail.Verdict.Reason },
            "the Fail row's expanded lines");
    }

    private static void D02NotApplicableCountsAsNoCheck()
    {
        ResearchModel model = ResearchSample();
        ResearchClaim na = ClaimOfKind(model, VerdictKind.NotApplicable);
        Check(na.Verdict.IsVerdict && !na.Verdict.CountsAsCheck, "NotApplicable is a verdict that counts as a check");
        var (holds, fails, none) = model.Counts();
        Equal(holds + fails + none, model.Claims.Count, "the counts cover every claim group");
        Equal(none, model.Claims.Count(c => c.Verdict.Kind is not (VerdictKind.Pass or VerdictKind.Fail)), "no check counts");
        Check(model.CountsLine().Contains("no check " + none), "the counts line: " + model.CountsLine());
        Equal(ResearchWords.Expanded(na.Verdict)[0], "counted as no check", "NotApplicable's expanded first line");
        Check(ResearchWords.Expanded(na.Verdict)[1].StartsWith("reason: ", StringComparison.Ordinal), "NotApplicable shows its reason on expand");
    }

    private static void D02CutOffIsNeverAVerdict()
    {
        ResearchModel model = ResearchSample();
        ResearchClaim cut = ClaimOfKind(model, VerdictKind.CutOff);
        Check(!cut.Verdict.IsVerdict && !cut.Verdict.CountsAsCheck, "a cut-off is a verdict or counts as a check");
        ResearchLine row = model.Lines().First(l => l.Kind == ResearchLineKind.Row && l.Ref == ResearchModel.Ref(cut));
        Equal(row.Verdict, "not found within budget", "the cut-off row's words");
        Check(!row.Verdict!.Contains("holds") && !row.Verdict.Contains("fails"), "the cut-off row reads as a verdict");
        Check(ResearchWords.Expanded(cut.Verdict)[0].StartsWith("the run stopped at its budget: no verdict", StringComparison.Ordinal), "the cut-off's expanded line");
    }

    private static void D02PinnedInputNeverStaleOnCommit()
    {
        ResearchModel model = ResearchSample();
        ResearchRun run = model.Runs.First();
        IReadOnlyList<string> before = model.PinnedLines(run.Run);
        Equal(before[0], "pinned: input " + run.Digest, "the pinned line");
        Check(before[1].EndsWith("read-only", StringComparison.Ordinal), "the branch is not read-only: " + before[1]);
        Check(model.AnswerLine() == "current" && !model.IsStale(ResearchRows.Claims), "a fresh answer is stale");
        model.Committed();
        Check(model.IsStale(ResearchRows.Claims) && model.AnswerLine() == ResearchWords.Stale, "a commit leaves the answer current");
        Sequence(model.PinnedLines(run.Run), before, "a commit changed the pinned input");
        Check(model.Lines().Where(l => l.Text.StartsWith("pinned: ", StringComparison.Ordinal)).All(l => !l.Text.Contains("stale")), "a pinned line turned stale");
        model.Select(ResearchTab.Claims);
        Check(!model.IsStale(ResearchRows.Claims), "asking the tab again leaves it stale");
    }

    private static void D02RenameChangesNoDigest()
    {
        ResearchModel model = ResearchSample();
        ResearchRecord record = model.Records.First(r => r.Parts.Count > 0);
        string reference = ResearchModel.Ref(record), part = ResearchModel.PartOf(record, new[] { 0 });
        string digest = model.Digest(reference), partDigest = model.Digest(part);
        string[] shas = model.Records.Select(r => r.Sha256).ToArray();
        var (done, _) = model.Execute(ResearchRows.Name, ResearchRows.NameArgument(reference, "my long step"));
        Check(done && model.Shown(reference) == "my long step", "the record's name did not take");
        Check(model.Rename(part, "opening part") && model.Shown(part).EndsWith("opening part", StringComparison.Ordinal), "the part's name did not take");
        Equal(model.Digest(reference), digest, "a name changed the record's digest");
        Equal(model.Digest(part), partDigest, "a name changed the part's digest");
        Sequence(model.Records.Select(r => r.Sha256), shas, "a name changed a record's sha256");
        Check(!model.Rename(ResearchModel.Ref(model.Claims[0]), "x"), "a claim group took a name");
        Check(model.Rename(reference, " ") && model.Shown(reference) == record.Name[(record.Name.LastIndexOf('/') + 1)..], "an empty name does not restore the record's own");
    }

    private static void D02EveryResearchControlInvokesARow()
    {
        var sample = new SampleResearchSource(Read("data/samples/research/panel.json"));
        CommandRegistry registry = LabRegistry();
        var drafts = new CommandRegistry();
        drafts.AddRows(sample.DraftRows);
        registry.AddRows(sample.DraftRows);
        Sequence(drafts.Order, ResearchRows.Drafts, "the sample's draft rows");
        string table = File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json"));
        foreach (string row in ResearchRows.Drafts) Check(!table.Contains("\"" + row + "\""), row + " is in the command table; it is no draft");
        foreach (string row in ResearchRows.All) Check(registry.Commands.ContainsKey(row), row + " is no row");
        var model = new ResearchModel(sample, sample.Evidence);
        var controls = new List<ResearchControl>(ResearchModel.TabControls());
        // Every line of every tab, with each claim group, record and part opened in turn.
        foreach (ResearchTab tab in Enum.GetValues<ResearchTab>())
        {
            model.Select(tab);
            controls.AddRange(model.Lines().SelectMany(model.Controls));
        }
        IEnumerable<string> opens = model.Claims.Select(ResearchModel.Ref).Concat(model.Records.Select(ResearchModel.Ref))
            .Concat(model.Records.SelectMany(r => r.Parts.Select((_, i) => ResearchModel.PartOf(r, new[] { i }))));
        foreach (string reference in opens)
        {
            if (model.Open != reference) model.Toggle(reference);
            controls.AddRange(model.Lines().SelectMany(model.Controls));
        }
        Check(controls.Count > 20, "only " + controls.Count + " controls");
        foreach (string row in new[] { ResearchRows.Open, ResearchRows.RecordRead, ResearchRows.Name, ResearchRows.Copy, ResearchRows.Save })
            Check(controls.Any(c => c.Row == row), "no control invokes " + row);
        foreach (ResearchControl control in controls)
        {
            Check(registry.Commands.ContainsKey(control.Row), control.Label + " invokes " + control.Row + ", which is no row");
            string? argument = control.TakesText ? control.Argument + "a name" : control.Argument;
            Check(ResearchModel.Handles(control.Row, argument), control.Label + ": the pane does not handle " + control.Row + " " + argument);
        }
    }
}
