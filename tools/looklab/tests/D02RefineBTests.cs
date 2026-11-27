using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Nodes;
using Magic600.Chrome;

internal static partial class Program
{
    private static string RefineShell(string path) => File.ReadAllText(Path.Combine(Root, "renderer/shell", path));

    private static void D02ShownDocksRevealAndFocus()
    {
        foreach (bool folded in new[] { false, true })
        {
            var model = new FrameModel();
            foreach (var (side, panel) in new[] { (DockSide.Left, "jumble.corrections"), (DockSide.Left, "jumble.word"),
                (DockSide.Right, "focus_layer.editor") })
            {
                model.AddDockPanel(side, panel);
                Check(model.RevealDock(side, panel, folded), panel);
                Equal(model.Docks[side].Open, panel);
                Equal(model.Docks[side].Revealed, folded ? panel : null);
            }
        }
        string frame = Read("app/Chrome/Ui/ChromeFrame.cs");
        string host = Read("app/Chrome/Ui/JumbleHost.cs");
        Check(frame.Contains("Model.RevealDock(side, id, Folds(side))") && frame.Contains("Flush();\n            if (IsInstanceValid(target) && target.IsVisibleInTree()) target.GrabFocus();"),
            "the panel needs to be built and visible when it takes keys");
        Check(host.Contains("RevealAndFocus(DockSide.Right, FocusLayerEditor.PanelId, FilterEditor.Expression)")
            && host.Contains("RevealAndFocus(DockSide.Left, NuCorrectionsPanel.PanelId, Corrections)")
            && RefineShell("project/D02/ShellJumble.cs").Contains("RevealAndFocus(DockSide.Left, WordPanel.PanelId, wordHost.Words)"),
            "each pane opens through the reveal route");
    }

    private static void D02ClassMemoryRestoresOnBinding()
    {
        var graph = new ContactGraph(3, null, new[] { (0, 1, true, false), (1, 2, true, false) });
        string[] ids = { "a", "b", "c" };
        var memory = new ContactClasses();
        memory.Assign(new ContactGraph(1, null, Array.Empty<(int, int, bool, bool)>()), new[] { "a" });
        var reopened = ContactClasses.Load(memory.Save());
        Sequence(reopened.Assign(graph, ids), new[] { 1, 2, 1 });
        Sequence(new ContactClasses().Assign(graph, ids), new[] { 2, 1, 2 });

        string tint = RefineShell("project/D02/ShellTint.cs");
        string view = Read("app/Chrome/Ui/JumbleLegend.cs");
        int load = tint.IndexOf("host.LoadClasses(File.ReadAllText(path))", StringComparison.Ordinal);
        Check(load >= 0 && load < tint.IndexOf("        Bind();", StringComparison.Ordinal), "class memory loads before the first bind");
        Check(tint.Contains("M600_SESSION_DIR") && tint.Contains("File.Move(path + \".part\", path, overwrite: true)")
            && view.Contains("Settings.Set(JumbleTint.MemoryKey, ClassMemory.Save())"), "the class memory is kept with the session's view state");
    }

    private static void D02PieceInspectorShowsStableDomainId()
    {
        var style = PoseStyle.Read(Read("data/samples/jumble/wj-ia-300/style.json"));
        int row = Enumerable.Range(0, style.RowCount).First(i => style.RowDomain(i) == 5);
        string id = style.Domains[5];
        var inspection = new PieceInspection(79606, 112151, null, true, style.InspectRow(row));
        Check(inspection.Line.EndsWith($"domain {id} · height {style.RowHeight(row)}", StringComparison.Ordinal), inspection.Line);

        string reversed = JsonSerializer.Serialize(new { domains = new[] { id, "other" }, lattice_domain = 1,
            rows = new[] { new { domain = 0, heights = new[] { style.RowHeight(row) }, count = 1 } } });
        var moved = PoseStyle.Read(reversed);
        Equal(moved.InspectRow(0), style.InspectRow(row));
        Equal(moved.DomainIndex(id), (int?)0);
        Equal(style.DomainIndex(id), (int?)5);
        Check(RefineShell("project/D02/ShellJumble.cs").Contains("sample = style.InspectRow(row)"), "the shell uses the inspected row's ID");
    }

    private static void D02FiltersFollowBindings()
    {
        var (model, style, _, move) = FilterAt300();
        model.SetText("not lattice");
        Check(model.Run() != null && model.Apply(UserEvent.Key("Enter")) == null, "apply the live query");
        Check(model.Save("off") == null, "save the applied definition");
        model.SetText("height >= 4");
        model.Run();
        var layers = new FocusLayers();
        model.FocusApplied(layers);
        layers.Cycle();
        model.Changed += () => layers.AddPredicate(model.Layer()!);

        move(301);
        model.Rebind(null);
        Check(model.Preview == null && model.Applied != null && model.Saved.Count == 1, "the unbound state keeps definitions and retires the preview");
        Equal(model.Layer()!.Count, Numeral.Pending);
        Check(!layers.Dimming && layers.Focused == PieceFilterModel.LayerId, "a pending count leaves the view undimmed");

        var next = PoseStyle.Read(Read("data/samples/jumble/ia-40/style.json"));
        model.Rebind(next);
        Equal(model.Layer()!.Count, Numeral.Of(next.Pieces - next.InDomain(next.LatticeDomain)));
        Equal(model.Applied!.Revision, 301L);
        Equal(model.Staged, "height >= 4");
        Check(layers.Dimming && layers.Inactive == InactiveMode.Hidden && model.Preview == null, "reapplying the count keeps the view's focus mode");
        model.Rebind(style);
        Equal(model.Load("off"), (string?)null);
        Equal(model.Staged, "not lattice");
    }

    private static void D02GlobalKeepsClearedThresholdField()
    {
        var map = Rim("wj-ia-300");
        Check(CapThreshold.Parse("N_c >= 50").Counts(map) != null && CapThreshold.None.Counts(map) == null
            && CapThreshold.Parse("N_c = F_c").Counts(map) != null, "set, clear and re-enter a threshold");
        Check(CapThreshold.OnGlobalBorder(null) && CapThreshold.OnGlobalBorder("history")
            && !CapThreshold.OnGlobalBorder(EntrySlot.CapMapPanel), "Global shows the field when the map panel is closed");
        string host = Read("app/Chrome/Ui/JumbleHost.cs");
        Check(host.Contains("Border(frame.Global, Source.State.Mode && CapThreshold.OnGlobalBorder("), "the field's visibility follows the dock and Jumble mode");
    }

    private static void D02CoversUndefinedHasDirection()
    {
        var covers = CapCovers.FromRecord(RefineCap(150))!;
        var undefined = covers.UndefinedLines().Single();
        Equal(undefined.Direction, "covers");
        Check(covers.CoveredBy.Contains(8) && undefined.Caps.Contains(8), "cap 8 has different answers by direction");
        var row = covers.Table().Single(r => r[0] == "8");
        Sequence(row, new[] { "8", "yes", "undefined" });
        Check(covers.Lines()[3].StartsWith("undefined · 77 (blocked): covers · 8 ", StringComparison.Ordinal), "the summary names the undefined direction");

        var data = JsonNode.Parse(RefineCap(150))!;
        var answer = data["covers"]!["answers"]!.AsArray().Single(a => (int)a!["cap"]! == 8)!;
        answer["d_covers_c"] = JsonNode.Parse("""{"status":"not_applicable","reason":"closed"}""");
        var two = CapCovers.FromRecord(data.ToJsonString())!.UndefinedLines().ToArray();
        Sequence(two.Select(l => l.Direction), new[] { "covered by", "covers" });
        Check(two[0].Head.Contains("(closed)") && two[1].Head.Contains("(blocked)"), "each direction keeps its reason");
    }

    private static void D02UnboundOffLatticeNuShowsUndefined()
    {
        var adopted = new JumbleState(true, false, null, 300, JumbleSessionKind.Solving);
        var order = Enumerable.Range(0, OrbitAtlas.Count).ToArray();
        var bound = NuBand.ForState(Catalogue().Constants, order, adopted, 42, null)!;
        var unbound = NuBand.ForState(null, order, adopted with { Revision = 301 }, null, null)!;
        Check(bound.Undefined && unbound.Undefined && !unbound.Shown && unbound.Row == null, "undefined is a band state without sample coordinates");
        Equal(unbound.Line(null), "ν band: undefined (pieces off the lattice)");
        Check(NuBand.ForState(null, order, adopted with { LatticeFlag = true }, null, null) == null, "an unbound lattice state has no ν row");
        Check(Read("app/Chrome/Ui/JumbleHost.cs").Contains("NuBand.ForState(nuConstants, order, Source.State"), "the strip follows the adopted lattice flag");
    }

    private static void D02NuRebindClearsCheck()
    {
        var corrections = Catalogue();
        corrections.Select(35);
        corrections.Mark(35, 1);
        corrections.Preview();
        corrections.Check(new int[NuConstants.Count]);
        corrections.ClearCheck();
        Check(corrections.Checked == null && corrections.Previewed == corrections.Current && corrections.Set[35] == 1,
            "a new answer is needed while the marked set and selected row stay");
        Check(corrections.Check(null).After == null, "a new off-lattice check is undefined");
        string host = Read("app/Chrome/Ui/JumbleHost.cs");
        Check(host.Contains("catalogue?.ClearCheck();") && host.Contains("Corrections.Model?.ClearCheck();"), "binding and revision changes clear checked output");
    }

    private static void D02FailedRunShowsRecordedDetails()
    {
        string data = Read("data/samples/research/panel.json");
        string[] FailureDetails(ResearchModel model)
        {
            var run = model.Runs.Single(r => r.Verdict?.Kind == VerdictKind.Fail);
            string reference = ResearchModel.Ref(run);
            model.Toggle(reference);
            return model.Lines().Where(l => l.Ref == reference && l.Kind == ResearchLineKind.Detail).Select(l => l.Text).ToArray();
        }
        string[] details = FailureDetails(ResearchModel.FromSample(data));
        Check(details.Contains("first claim that does not hold: cycle.closes") && details.Contains("code: KN6:evidence.certificate_rejected")
            && details.Contains("checker's reason: after 84 twists cap 17 sits where cap 18 started; the cycle does not close"),
            "the run expands the failure from its own checker report");
        var sample = JsonNode.Parse(data)!;
        var failure = sample["answers"]!["research.runs"]!["answer"]!["rows"]!.AsArray()
            .Select(r => r!["checks"]!.AsArray()[0]!["verdict"])
            .OfType<JsonObject>().Single(v => (string?)v["kind"] == "fail");
        failure.Remove("claim");
        failure.Remove("reason");
        details = FailureDetails(ResearchModel.FromSample(sample.ToJsonString()));
        Check(details.Contains("code: KN6:evidence.certificate_rejected")
            && details.All(l => !l.StartsWith("first claim that does not hold:") && !l.StartsWith("checker's reason:")), "absent facts have no expansion lines");
    }

    private static void D02ResearchCopyTransfersShownRow()
    {
        var sample = new SampleResearchSource(Read("data/samples/research/panel.json"));
        string? copied = null;
        var model = new ResearchModel(sample, sample.Evidence, text => copied = text);
        var claim = model.Claims.First(c => c.Verdict.Kind == VerdictKind.Fail);
        string reference = ResearchModel.Ref(claim);
        model.Toggle(reference);
        var row = model.Lines().Single(l => l.Ref == reference && l.Kind == ResearchLineKind.Row);
        var result = model.Execute(ResearchRows.Copy, reference);
        Check(result.Done, result.Words);
        Equal(copied, row.Text + "\n" + row.Verdict);
        Equal(model.Transfer!.Text, copied);
        Equal(model.Transfer.Origin, Origin.Research);
        Check(RefineShell("project/D02/ShellResearch.cs").Contains("DisplayServer.ClipboardSet"), "the shell copies to the clipboard");
    }

    private static void D02UnavailableSectionsAndTwistReadouts()
    {
        var readout = CapReadout.Read(RefineCap(0));
        var map = CapMapModel.Read(Read("data/samples/jumble/wj-ia-300/rim.json"), cap => cap == 0 ? readout : null);
        var local = map.Readout(0);
        Equal(local.U, Numeral.Of(231));
        Equal(CapFacts.Tier1(map, 0).Numerals.Single(slot => slot.Label == "u").Value, local.U);
        var sections = new LocalAccordion();
        var names = new Dictionary<string, string>();
        foreach (string section in new[] { "twist_check", "cap_covers", "repair_list" })
        {
            sections.SetCount(section, 0, LocalAccordion.Unavailable);
            Check(sections.Header(section, names).EndsWith(" · unavailable", StringComparison.Ordinal), section);
        }
        foreach (var (kind, expected) in new[] { ("cut_off", "not found within budget"), ("cancelled", "unavailable") })
        {
            var repair = RepairListModel.From(RefineCap(150), 300, kind)!;
            sections.SetCount("repair_list", repair.Rows.Count, repair.SectionStatus);
            Equal(sections.Header("repair_list", names), "repair_list · " + expected);
        }
        using var record = JsonDocument.Parse("""{"status":"refused","delta_n_h":{"n_0":0}}""");
        var refused = TwistCheck.Read(record.RootElement, 1);
        Equal(KeptChecks.Cells(new CheckRow(1, 1, 0, 1, refused))[5], "undefined");
        var checkedTwist = TwistCheck.FromRecord(RefineCap(150))[1];
        Equal(KeptChecks.Cells(new CheckRow(1, 1, 150, 1, checkedTwist))[5], "0");
    }

    private static void D02RepairRebuildKeepsKeyFocus()
    {
        var repair = RepairListModel.From(RefineCap(150), 300)!;
        var sections = new LocalAccordion();
        sections.Toggle("repair_list");
        repair.Focus(2, UserEvent.Key("Down"));
        sections.SetCount("repair_list", repair.Rows.Count);
        Equal(repair.Focused, (int?)2);
        Equal(repair.Digit(3, new[] { RepairListModel.Context }), repair.Rows[2]);
        Check(repair.Digit(3, Array.Empty<string>()) == null, "the digit requires the list's keyboard context");

        // The console runs without Godot. Check focus transfer at the control replacement that determines that context.
        string source = Read("app/Chrome/Ui/JumbleSurface.cs");
        int remove = source.IndexOf("sections.RemoveChild(child)", StringComparison.Ordinal);
        int capture = source.IndexOf("oldList.HasFocus()", StringComparison.Ordinal);
        int mounted = source.IndexOf("sections.AddChild(control)", StringComparison.Ordinal);
        int restore = source.IndexOf("if (restoreRepairFocus && control.IsVisibleInTree()) control.GrabFocus();", StringComparison.Ordinal);
        Check(capture >= 0 && capture < remove && remove < mounted && mounted < restore,
            "removing the focused repair control rebuilds it without giving keyboard focus back");
        Check(source.Contains("oldList.Model == repair && oldList.HasFocus()"), "a different list starts with its own focus");
    }

    private static void D02TrackJTracePreservesTintAndFocus()
    {
        var (code, _, error) = Run("python", new[] { "-B", "-c", """
            import sys
            sys.path.insert(0, "renderer/shell")
            import d02_run as run
            import track_j as track
            state = "wj-ia-300"
            tables = run.tint_tables(state)
            for mode in ("contact", "height", "off-lattice"):
                tint = run.tint_expected(tables, mode, run.OVERLAY_DOMAIN, None, None)
                assert track.style_expected(tables, state, mode, 150, "hidden") == (tint["outline_sha256"], tint["fill_sha256"])
            want = run.overlay_expected(tables, state, 150, None, "hidden", None, None)
            assert want["trace_candidates"] == 1909 and want["kept"] == 0
            line = "Cut of cap 150 \u00b7 surface trace"
            tint = {key: want.pop(key) for key in ("outline_sha256", "fill_sha256", "modes")}
            tint["mode"] = "contact"
            dump = {"overlay": dict(want, line=line, global_line=line, local_line=line), "tint": tint}
            rule = {"at": [("step 0", 150, "hidden")]}
            for check in (track.never_hidden, track.j0_never_hidden):
                problems = check([dump], [], rule, state)
                assert not problems, problems
                dump["overlay"]["kept"] = 1909
                assert any("kept" in p for p in check([dump], [], rule, state))
                dump["overlay"]["kept"] = 0
            """ });
        Equal(code, 0, error);
    }

    private static void D02ObsoleteOverlayTestsAreRemoved()
    {
        string source = Read("tests/D02OverlayTests.cs");
        Check(!source.Contains("D02OverlayTraceAtANamedCapOnly") && !source.Contains("D02OverlayHiddenCountIsExactAndNothingHides"),
            "the superseded overlay test bodies were removed");
    }
}
