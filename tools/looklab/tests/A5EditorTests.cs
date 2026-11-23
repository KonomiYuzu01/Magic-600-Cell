using System;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    private static string Sample(string program) => Read("data/samples/" + program + ".6cs").Replace("\r\n", "\n");

    private static ScriptEntry Entry(string program, string name) => new ScriptProgram(program, program + ".6cs", Sample(program)).Entries.Single(e => e.Name == name);

    // The language service of the editor: diagnostics with a fix, sample faults of each severity, regions.
    private static void A5EditorServiceDiagnosesFixesAndRegions()
    {
        foreach (string program in new[] { "sweep", "census", "layout", "mounts" })
            Check(SixCsService.Diagnose(Sample(program)).Count == 0, program + ".6cs is not clean");
        string sweep = Sample("sweep");
        string broken = sweep.Replace("let cells = ring_cells(ring);\n    return record", "let cells = ring_cells(ring)\n    return record");
        Check(SixCsService.Diagnose(broken).Count(d => d.Severity == Severity.Error && d.Text.Contains("is missing")) == 1, "a missing `;` was not found");
        Equal(SixCsService.Apply(broken, SixCsService.Fixes(broken).First(f => f.Kind == FixKind.Syntax)), sweep);
        string contract = sweep.Replace("effects [preview]", "effects [snapshot]");
        Check(SixCsService.Diagnose(contract).Any(d => d.Text.Contains("not in its effects")), "a preview outside its effects was not found");
        Equal(SixCsService.Apply(contract, SixCsService.Fixes(contract).First(f => f.Kind == FixKind.Contract)), sweep.Replace("effects [preview]", "effects [snapshot, preview]"));
        Check(new[] { FixKind.Invert, FixKind.Mirror, FixKind.Conjugate, FixKind.Repeat, FixKind.Rescope }.All(k => SixCsService.Fixes(sweep).Any(f => f.Kind == k)), "a structural edit is not offered");
        var faults = SixCsService.Diagnose(SixCsService.Faults(sweep));
        Check(Enum.GetValues<Severity>().All(s => faults.Any(d => d.Severity == s)), "the sample faults miss a severity");
        Check(faults.All(d => SixCsService.Explain(d) is { Why.Length: > 0, Do.Length: > 0 }), "a diagnostic does not say why and what to do");
        ScriptEntry reach = Entry("sweep", "reach"); (int first, int last) = SixCsService.Region(sweep, reach.Line);
        Check(first <= reach.Line && last > reach.Line && sweep.Split('\n')[last - 1].Trim() == "}", "the region of an entry point does not end at its brace");
        Check(SixCsService.Completions("sw").Any(c => c.Word == "sweep") && SixCsService.Completions("").Count <= SixCsService.Offered, "completion");
        Sequence(SixCsService.Mounts(Sample("mounts")).OrderBy(m => m).ToArray(), Enum.GetValues<MountKind>());
    }

    // Runs wait in a visible queue, results go stale by name, and no key both runs and commits.
    private static void A5EditorQueueResultsAndKeyRules()
    {
        ScriptEntry reach = Entry("sweep", "reach"), ring = Entry("sweep", "ring");
        var queue = new RunQueue(); int done = 0; queue.Finished += _ => done++;
        RunItem one = queue.Enqueue(reach, reach.Line, reach.Line + 3); queue.Enqueue(ring, ring.Line, ring.Line + 4);
        queue.Advance(RunQueue.Cost / 2);
        Check(queue.Items.Count(i => i.State == RunState.Running) == 1 && queue.Items.Count(i => i.State == RunState.Queued) == 1 && done == 0, "the queue does not run one at a time");
        queue.Advance(RunQueue.Cost);
        Check(done >= 1 && queue.Items.First(i => i.Id == one.Id).State == RunState.Done, "a run did not finish");
        Check(queue.Abort() == queue.Items.Count(i => i.State == RunState.Aborted) && !queue.Active.Any(), "an abort left a run");
        var results = new ResultStore(); results.Set(reach, "record { cells: 30 }");
        Check(results.Of(reach.Command) is { Since: null } && results.StateChanged("sweep.ring") == 1 && results.Of(reach.Command)!.Since == "sweep.ring" && results.StateChanged("later") == 0,
            "a result did not go stale with the name of what came between");
        results.Set(reach, "record { cells: 30 }");
        Check(results.Of(reach.Command)!.Since == null && results.Results.Count == 1, "a rerun did not refresh its result");
        CommandRegistry registry = CommandRegistry.FromTable(System.IO.File.ReadAllText(System.IO.Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        registry.Add(new ChromeCommand(ring.Command, "sweep.6cs: ring", "query", new[] { "any" }, ""));
        Keymap keys = Keymap.Parse(Read("data/design/keymap.json"), registry);
        Check(KeyRules.RunCommitClashes(keys).Count == 0, "a shipped key both runs and commits");
        string commitKey = keys.Bindings.First(b => b.Command == KeyRules.Commit).Key;
        Check(KeyRules.Conflict(keys, commitKey, "global", ring.Command) != null && KeyRules.Runs(ring.Command) && !KeyRules.Runs(KeyRules.Commit), "a run on the commit key was not seen");
    }

    // A package by another author: verified digest, consent, refusal of its preview in a solving session; mounts, slots, console rows.
    private static void A5EditorPackagesMountsSlotsAndConsole()
    {
        PackageManifest package = PackageManifest.Parse(Read("data/samples/packages/ring-tools.json"));
        Check(package.Verified && !(package with { Source = package.Source + " " }).Verified, "the digest does not bind the source");
        Refuse(() => PackageManifest.Parse(Read("data/samples/packages/ring-tools.json").Replace("\"preview\"", "\"commit\"")), "unknown capability");
        var store = new PackageStore();
        Check(!store.Install(package with { Digest = "sha256:0" }) && store.Install(package) && !store.Install(package) && store.Store() == "ring-tools", "install");
        Check(PackageStore.PreviewRefusal(package, ScriptKind.Preview, true) != null && PackageStore.PreviewRefusal(package, ScriptKind.Preview, false) == null &&
            PackageStore.PreviewRefusal(package, ScriptKind.Read, true) == null, "a package preview in a solving session was not refused");
        Check(store.Revoke("ring-tools") && !store.Revoke("ring-tools"), "revoke");
        var mounts = new MountSet(); mounts.Set("mounts", MountKind.Overlay, false);
        var again = new MountSet(); again.Restore(mounts.Store());
        Check(!again.IsOn("mounts", MountKind.Overlay) && again.IsOn("mounts", MountKind.Panel), "a mount switch was not kept");
        var slots = new SlotTabs(); int a = slots.Mount("a"), b = slots.Mount("b"), c = slots.Mount("c");
        Check(a != b && c >= 0 && slots.Tabs(c).Count == 2 && slots.Shown(c) == "c" && slots.SlotOf("a") == a, "two slots, then tabs");
        var kept = new SlotTabs(); kept.Restore(slots.Store()); Equal(kept.Store(), slots.Store());
        Check(slots.Unmount("c") && !slots.Unmount("c") && slots.Tabs(c).Count == 1, "an unmounted panel kept its tab");
        foreach (ConsoleKind kind in Enum.GetValues<ConsoleKind>())
            Check(ConsoleRows.Read(ConsoleRows.Write(kind, "x.y()")) == (kind, "x.y()"), "console row " + kind);
        Check(OutputMarks.For(0) != OutputMarks.For(1), "two outputs share a mark");
    }
}
