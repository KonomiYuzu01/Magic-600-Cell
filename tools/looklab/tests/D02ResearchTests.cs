using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Magic600.Chrome;

// B28 (ADR-D02.3, .44 to .46, .49) and B29 (ADR-D02.4, .50, .52): the research pane's model on the synthetic sample (data/samples/research/panel.json):
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
        registry.AddRows(Read("data/design/command-rows-d02.json"));
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
            .Concat(model.Records.SelectMany(r => r.Parts.Select((_, i) => ResearchModel.PartOf(r, new[] { i }))))
            .Concat(model.Runs.Select(ResearchModel.Ref)).Concat(model.Obligations.Select(ResearchModel.Ref));
        foreach (string reference in opens)
        {
            if (model.Open != reference) model.Toggle(reference);
            controls.AddRange(model.Lines().SelectMany(model.Controls));
        }
        Check(controls.Count > 20, "only " + controls.Count + " controls");
        foreach (string row in new[] { ResearchRows.Open, ResearchRows.RecordRead, ResearchRows.Name, ResearchRows.Copy, ResearchRows.Save,
            ResearchRows.Declare, ResearchRows.Start, ResearchRows.Reproduce, ResearchRows.Trace, ResearchRows.RunView })
            Check(controls.Any(c => c.Row == row), "no control invokes " + row);
        foreach (ResearchControl control in controls)
        {
            Check(registry.Commands.ContainsKey(control.Row), control.Label + " invokes " + control.Row + ", which is no row");
            string? argument = control.TakesText ? control.Argument + "a name" : control.Argument;
            Check(ResearchModel.Handles(control.Row, argument), control.Label + ": the pane does not handle " + control.Row + " " + argument);
        }
    }

    // --- B29 (ADR-D02.4, .45, .46, .50, .52): Runs and Obligations ---------------------------------------------------

    // Declares seed 7430, depth 40 and the given budget (budget first), then starts the run.
    private static ResearchRun StartSampleRun(ResearchModel model, long budget)
    {
        foreach (string field in new[] { "budget=" + budget, "seed=7430", "depth=40" })
            Check(model.Execute(ResearchRows.Declare, field).Done, "declaring " + field + " was refused");
        var (done, words) = model.Execute(ResearchRows.Start, null);
        Check(done, "the declared run did not start: " + words);
        return model.Started[^1];
    }

    private static void D02RunNeedsDeclaredBudget()
    {
        ResearchModel model = ResearchSample();
        Check(model.Execute(ResearchRows.Declare, "seed=7430").Done && model.Execute(ResearchRows.Declare, "depth=40").Done, "seed and depth refused");
        // A seed and a depth are not enough: the budget is declared before the run starts.
        Equal(model.Unavailable(ResearchRows.Start, null), ResearchWords.NoBudget, "the start's reason without a budget");
        var (done, words) = model.Execute(ResearchRows.Start, null);
        Check(!done && words == ResearchWords.NoBudget && model.Started.Count == 0, "a run started without a declared budget: " + words);
        ResearchControl start = model.Lines().Where(l => l.Kind == ResearchLineKind.Declare).SelectMany(model.Controls).Single(c => c.Row == ResearchRows.Start);
        Equal(start.Why, ResearchWords.NoBudget, "the Start control's reason");
        Check(!model.Execute(ResearchRows.Declare, "budget=0").Done && !model.Execute(ResearchRows.Declare, "budget=many").Done, "a budget that is no positive number was taken");
        Check(model.Execute(ResearchRows.Declare, "budget=4096").Done, "the budget was refused");
        Check(model.Unavailable(ResearchRows.Start, null) == null, "the start is still unavailable with every field declared");
        // The run records the budget it was declared with.
        ResearchRun run = StartSampleRun(model, 4096);
        Equal(run.Budget, new RunBudget(4096), "the recorded budget");
        Check(run.Seed == 7430 && run.Depth == 40, "the run did not record its seed and depth");
        Equal(run.Run, SampleResearchSource.RunId(7430, 40, 4096), "the sample run's name");
        Check(model.Lines().Any(l => l.Ref == ResearchModel.Ref(run) && l.Text.Contains("budget 4,096 steps, declared before the start and recorded")), "the run's lines do not name its budget");
    }

    private static void D02RunAtBudgetIsCutOffNotVerdict()
    {
        ResearchModel model = ResearchSample();
        var counts = model.Counts();
        ResearchRun run = StartSampleRun(model, 2048);
        Equal(run.State, SlotState.BudgetStopped, "the run at its budget");
        Equal(run.Covered, "2048", "the steps the cut-off covered");
        Equal(model.RunLine(run), "not found within budget", "the run's line");
        Check(model.Open == ResearchModel.Ref(run) && model.Tab == ResearchTab.Runs, "the started run is not open in Runs");
        List<ResearchLine> lines = model.Lines().Where(l => l.Ref == ResearchModel.Ref(run)).ToList();
        Equal(lines[0].Verdict, "not found within budget", "the run's row");
        Check(lines.Any(l => l.Text == "the run stopped at its budget: no verdict (covered 2048)"), "the expanded run does not say it stopped at its budget");
        Check(lines.Any(l => l.Text == "code: KN7:budget.steps"), "the cut-off's code is missing");
        Equal(model.Counts(), counts, "a cut-off changed the claim counts");
        // A listed run cut off by its reading is budget-stopped too, whatever its checks said.
        ResearchRun listed = model.Runs.Single(r => r.Reading == "cut_off");
        Check(listed.State == SlotState.BudgetStopped && model.RunLine(listed) == ResearchWords.CutOff, "a listed cut-off run reads as a verdict");
        // Reproduce: the sample's run again, "reproduced: bytes equal"; a mismatch's words name the byte.
        var (done, words) = model.Execute(ResearchRows.Reproduce, ResearchModel.Ref(run));
        Check(done && words.EndsWith("reproduced: bytes equal", StringComparison.Ordinal), "the reproduction said " + words);
        Equal(model.Reproductions[run.Run].Kind, VerdictKind.Reproduced, "the reproduction's kind");
        Check(model.Lines().Any(l => l.Ref == ResearchModel.Ref(run) && l.Text == "reproduced: bytes equal"), "the run's lines miss its reproduction");
        Check(!model.Execute(ResearchRows.Reproduce, ResearchModel.Ref(listed)).Done, "the sample reproduced a run it did not start");
        using var differs = System.Text.Json.JsonDocument.Parse("{\"kind\":\"refused\",\"code\":\"LG4:evidence.reproducibility_mismatch\",\"byte\":17}");
        Equal(ResearchModel.ReproductionLine(ResearchModel.ReproductionOf(differs.RootElement)), "differs at byte 17", "a mismatch's words");
    }

    private static void D02SeedOutsidePoolRefused()
    {
        ResearchModel model = ResearchSample();
        foreach (int seed in new[] { 7400, 7500, 7001, 7301, 9000 })
        {
            var (done, words) = model.Execute(ResearchRows.Declare, "seed=" + seed);
            Check(!done && words == $"seed {seed} is outside the pool 7401 to 7499", $"seed {seed}: {words}");
            Equal(model.Declared.Seed, null, "a refused seed was declared");
        }
        Equal(SeedPool.Words(7401), "seed 7401: scored block J2, seeds 7401 to 7403 at depth 10", "7401's words");
        Equal(SeedPool.Words(7406), "seed 7406: scored block J2, seeds 7404 to 7406 at depth 20", "7406's words");
        Equal(SeedPool.Words(7421), "seed 7421: scored block J3, seeds 7419 to 7421 at depth 3,000", "7421's words");
        Equal(SeedPool.Words(7422), "seed 7422: free exploration (7422 to 7499), not scored", "7422's words");
        Equal(SeedPool.Words(7499), "seed 7499: free exploration (7422 to 7499), not scored", "7499's words");
        Equal(SeedPool.Scored.Sum(b => b.To - b.From + 1), 21, "the scored seeds (J2 six, J3 fifteen)");
        Check(model.Execute(ResearchRows.Declare, "seed=7401").Done && model.DeclaredLine().StartsWith("seed 7401: scored block J2", StringComparison.Ordinal), "7401 was refused or not named");
    }

    private static void D02HistoryShowsNoResearchRuns()
    {
        var history = new MoveHistory();
        int revision = history.Revision;
        ResearchModel model = ResearchSample();
        ResearchRun run = StartSampleRun(model, 4096);
        model.Execute(ResearchRows.Reproduce, ResearchModel.Ref(run));
        Check(history.Entries.Count == 0 && history.Revision == revision, "a research run reached History");
        Check(!MoveHistory.Origins.Contains("research"), "History has a research origin");
        var sample = new SampleResearchSource(Read("data/samples/research/panel.json"));
        CommandRegistry registry = LabRegistry();
        registry.AddRows(sample.DraftRows);
        foreach (string row in new[] { ResearchRows.Declare, ResearchRows.Start, ResearchRows.Reproduce })
            Equal(registry.Commands[row].Kind, "workspace", row + "'s kind");
        // No commit happened, so the answers stay current.
        Check(!model.IsStale(ResearchRows.Runs), "the run made the answers stale");
        Check(model.Lines().Any(l => l.Kind == ResearchLineKind.Note && l.Text.Contains(ResearchWords.NotInHistory)), "Runs does not say History shows no research runs");
    }

    private static void D02Cr17ControlsUnavailableWithReason()
    {
        ResearchModel model = ResearchSample();
        ResearchRun run = StartSampleRun(model, 4096);
        string reference = ResearchModel.Ref(run);
        ResearchControl[] controls = model.Lines().Where(l => l.Kind == ResearchLineKind.Actions && l.Ref == reference).SelectMany(model.Controls).ToArray();
        ResearchControl view = controls.Single(c => c.Label == "Open derived state in a free view"), step = controls.Single(c => c.Label == "Step the checker");
        Check(view.Row == ResearchRows.RunView && step.Row == ResearchRows.Trace, "the CR-17 controls invoke other rows");
        Equal(view.Why, "row not in the command table yet (CR-17)", "the free view's reason");
        Equal(step.Why, "row not in the command table yet (CR-17)", "the checker step's reason");
        foreach (string row in ResearchRows.Cr17)
        {
            var (done, words) = model.Execute(row, reference);
            Check(!done && words == ResearchWords.Cr17, row + " ran or said " + words);
        }
        // Past the four-view envelope the free view is also refused.
        model.OpenViews = () => ViewPanes.MaxPanes;
        Equal(model.Unavailable(ResearchRows.RunView, reference), "row not in the command table yet (CR-17); refused past the four-view envelope: four views are open", "the reason with four views open");
        Equal(model.Unavailable(ResearchRows.Trace, reference), ResearchWords.Cr17, "the checker step's reason with four views open");
        // Neither row is in the command table; both are ADR-D02's draft rows with request CR-17.
        string table = File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json"));
        string drafts = Read("data/design/command-rows-d02.json");
        foreach (string row in ResearchRows.Cr17) Check(!table.Contains("\"" + row + "\"") && drafts.Contains("\"" + row + "\""), row + " is in the table or no draft row");
        // The Obligations tab: the sample's four record-only anchors, the missing inputs word for word.
        model.Select(ResearchTab.Obligations);
        Equal(model.Obligations.Count, 4, "the sample's obligations");
        ResearchObligation missing = model.Obligations.First(o => o.Missing.Count > 0);
        Check(model.Toggle(ResearchModel.Ref(missing)), "an obligation does not open");
        Check(model.Lines().Any(l => l.Text == "missing: " + missing.Missing[0]), "the obligation's missing input is not shown");
    }
}
