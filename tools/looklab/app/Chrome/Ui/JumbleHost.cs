using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The Jumble host in Solve (ADR-D02.6, .11, .12; build step B6): from a Jumble source it keeps the entry slot on the
// adopted revision's mode, draws the mode cell in the top bar and the Jumble mark on every view border, switches the
// strip and the rail to the undefined numeral on the exact lattice flag, and puts Local's sections in Local. It adds
// one panel, the cap map, to the existing bottom dock, and no layout and no dock.
public sealed partial class JumbleHost
{
    public IJumbleSource Source { get; }
    public ChromeSettings Settings { get; } = new();
    public EntrySlot Entry { get; }
    public LocalAccordion Local { get; } = new();
    public JumbleSurface Surface { get; }
    public CapMapPanel CapMap { get; }
    // jumble.cap.name without a number (owner's bug report, 26 November 2026): the prompt of the caps in cap-id order.
    public CapPromptOverlay Prompt { get; }
    // B9: the named cap, the threshold, the meeting caps and the job gate (ADR-D02.15, .16, .19).
    public CapNaming Naming { get; } = new();
    // Local is never empty (owner, 26 November 2026): the cap Local draws, the named one or the default.
    public LocalCap LocalCap { get; } = new();
    // Whether a state is bound, so that Local can draw; the shell gives it, and a host without one binds none.
    private Func<bool> stateBound = () => false;
    // Whether Local draws without a Jumble cap (the base puzzle's LocalPlan draws cell 0 there): its empty sentence then
    // never shows, since Local is drawn.
    private bool drawnWithoutCap;
    // Local's empty sentence, which is also the hint on its readout while it draws a cap that is not named.
    private readonly string hint;
    public CapThreshold Threshold { get; private set; } = CapThreshold.None;
    public CapJobs Jobs { get; }
    private CapLayout? layout;
    private CapMapModel map = CapMapModel.Pending();
    private MeetingCaps? meeting;
    private string? refused;
    // The draft rows (CR-14, and CR-19's seat key as B10's stand-in) this host carries out.
    public static readonly IReadOnlySet<string> Carried = new HashSet<string>(StringComparer.Ordinal)
        { "jumble.cap.name", "jumble.cap.unname", "jumble.capmap.threshold.set", "jumble.capmap.threshold.clear", "jumble.seat.hold",
          "jumble.focus.straddlers", "jumble.focus.domain", "jumble.focus.height",
          // B15: the one tint channel's mode (CR-14's draft row).
          JumbleTint.Row,
          // The Focus layer, the 0.4 Piece Filter (owner, 26 November 2026), on the bound state's pose rows.
          "filter.open", "filter.preview", "filter.apply", "filter.edit", "filter.preset", "filter.save", "filter.load", "structure.filter.compose",
          // B26: the ν-corrections' set (CR-14's mark row) and its check.
          "jumble.corrections.mark", "jumble.corrections.check",
          // B22 rest: Global's chart presets, the palette's chart row and the way back to the chain (CR-14's draft rows).
          JumbleViews.SheetRow, JumbleViews.RingsRow, JumbleViews.ChartRow, JumbleViews.ChainRow,
          // B21: Local's raw view, its antipode chart and its clip (CR-14's draft rows).
          RawView.Row, RawView.AntipodeRow, RawView.ClipRow,
          // B32 findings: the entry slot, the cut slice's split and shell (B20), and the repair list's key route.
          EntrySlot.Id, SplitRow, ShellRow, RepairFocusRow, RepairStageRow,
          GripFrame.SelectRow, GripFrame.NextRow };
    public GripFrame? Grip { get; set; }
    // The product shell checks the arguments for the palette, then applies an explicit request at the host.
    public Func<string, string?, bool, string?>? GripRow { get; set; }
    public const string SplitRow = "jumble.slice.split", ShellRow = "jumble.slice.shell",
        RepairFocusRow = "jumble.repair.focus", RepairStageRow = "jumble.repair.stage";
    // B32 findings: the shell's cut slice rows (answers why not, or null once carried out).
    public Func<string, string?, string?>? SliceRow { get; set; }
    public string? SliceRefused { get; private set; }
    // B22 rest: the shell's Global layout switch (answers why not, or null) and the chart Global draws (null: the chain).
    public Func<string, string?>? ChartRow { get; set; }
    public Func<string?>? Chart { get; set; }
    public string? ChartRefused { get; private set; }
    // B21: the shell's raw view rows (answers why not, or null) and the raw view Local draws (null: off).
    public Func<string, string?, string?>? RawRow { get; set; }
    public Func<RawView?>? Raw { get; set; }
    public string? RawRefused { get; private set; }
    public static readonly IReadOnlySet<string> FilterRows = new HashSet<string>(StringComparer.Ordinal)
        { "filter.open", "filter.preview", "filter.apply", "filter.edit", "filter.preset", "filter.save", "filter.load", "structure.filter.compose" };
    // The Focus layer: its model for the bound state (null while none is bound), and its editor in the right dock.
    private PieceFilterModel? filter;
    public PieceFilterModel? Filter => Style == null ? null : filter;
    public FocusLayerEditor FilterEditor { get; }
    // B25: a stored word played by part, in the left dock beside the Library.
    public WordPanel Words { get; } = new();
    // B26: the ν band under the rail at N = 0 (ADR-D02.28) and the ν-corrections rows in the left dock (ADR-D02.39).
    public NuBandStrip NuStrip { get; }
    public NuCorrectionsPanel Corrections { get; } = new();
    public NuBand? Nu { get; private set; }
    private NuConstants? nuConstants;
    private WordState? nuState;
    private long nuRevision;
    // B11: the named cap's covers with the hover's inspect line, its raw record (the straddlers' count), the bound
    // state's pose rows (the domain and height counts), and the views' focus layers the three predicates join.
    public CapCovers? Covers { get; private set; }
    public CoversInspect Inspect { get; } = new();
    public Func<int, CapCovers?>? CoversSource { get; set; }
    public Func<int, string?>? RecordSource { get; set; }
    public PoseStyle? Style { get; private set; }
    public IReadOnlyList<FocusLayers> Layers { get; set; } = Array.Empty<FocusLayers>();
    private string? domainShown;
    private int? heightShown;
    // B12: the named cap's repair list at the shown revision, its steps and the step focus; the variant names which of
    // the sample's results stands in for the job (completed, cut_off or cancelled).
    public RepairListModel? Repair { get; private set; }
    public RepairSteps RepairSteps { get; private set; } = new();
    public RepairScrub Scrub { get; } = new();
    public string RepairVariant { get; set; } = "completed";
    // The solver's picks, in order, and B13's stage: each pick stages its row as a sealed, whole-only span of the
    // sample preview, through the boundary ledger.
    public IReadOnlyList<RepairRow> Picks => picks;
    private readonly List<RepairRow> picks = new();
    public event Action<RepairRow>? RepairPicked;
    public BoundaryLedger Ledger { get; } = new();
    public RepairStage Stage { get; }
    // B10: the kept twist checks at the named cap and revision, the session's release record, and the sample jobs'
    // tick clock with the cap record's results.
    public KeptChecks? Checks { get; private set; }
    public Func<int, IReadOnlyDictionary<int, TwistCheck>>? CheckSource { get; set; }
    private readonly ReleaseRecord releases = new();
    private readonly SampleJobs jobs = new(3);
    private IReadOnlyDictionary<int, TwistCheck> results = new Dictionary<int, TwistCheck>();
    private readonly List<(long Tick, int Item)> seat = new();
    // A seat's twelve twists at the rhythm of B10's Q3.b test: faster than a check (three ticks), with a few pauses.
    public static readonly IReadOnlyList<int> SeatRhythm = Array.AsReadOnly(new[] { 0, 1, 2, 6, 7, 12, 13, 14, 15, 20, 21, 22 });
    // The facts changed: the host draws its view marks again (Marks).
    public event Action? Changed;
    private readonly ChromeFrame frame;
    private readonly string unit, above;
    // The readouts' slots at this revision and the positions that changed since the last (ADR-D02.22).
    private IReadOnlyList<ReadoutSlot> totals;
    private IReadOnlySet<int> totalsMarked = new HashSet<int>();
    private IReadOnlyList<ReadoutSlot>? cap;
    private IReadOnlySet<int> capMarked = new HashSet<int>();
    private RelationStrip? strip;
    private OrbitRail? rail;
    private bool following;

    public JumbleHost(ChromeFrame frame, IJumbleSource source, IReadOnlyDictionary<string, string> names, string emptySentence, string emptyBody,
        string unit = "seams", string above = "above 6")
    {
        this.frame = frame; Source = source; this.unit = unit; this.above = above; hint = emptySentence;
        totals = GlobalTotals.Pending.Slots(unit, above);
        Entry = new EntrySlot(Settings);
        Surface = new JumbleSurface(Local, () => frame.Tokens, names, emptySentence, emptyBody);
        Surface.RepairPicked += Pick;
        Stage = new RepairStage(Ledger, () => Source.State.SessionKind == JumbleSessionKind.Solving);
        Surface.AddChild(new RepairStageBar(Stage, () => frame.Tokens));
        string title = names.TryGetValue("cap_map", out string? name) ? name : "Cap map";
        CapMap = new CapMapPanel(Settings, () => frame.Tokens, title);
        Jobs = new CapJobs(Naming);
        CapMap.Map.Naming = Naming;
        CapMap.Map.Refusal = Refusal;
        CapMap.Map.Picked += cap => Click(cap);
        Prompt = new CapPromptOverlay(new CapPrompt(() => map, Refusal), () => frame.Tokens, () => Naming.Named);
        Prompt.Picked += cap => Click(cap);
        frame.Overlays.AddChild(Prompt);
        CapMap.ThresholdTyped += text => SetThreshold(text);
        Naming.Changed += _ => Named();
        frame.AddDockPanel(DockSide.Bottom, EntrySlot.CapMapPanel, title, "panel.capmap", CapMap);
        FilterEditor = new FocusLayerEditor(() => frame.Tokens);
        frame.AddDockPanel(DockSide.Right, FocusLayerEditor.PanelId, "Focus layer", "layer.cycle", FilterEditor);
        frame.Rebuilt += DressFilter;
        frame.AddDockPanel(DockSide.Left, WordPanel.PanelId, "Word", "panel.history", Words);
        NuStrip = new NuBandStrip(() => frame.Tokens);
        frame.AddDockPanel(DockSide.Left, NuCorrectionsPanel.PanelId, "ν-corrections", "library.tool", Corrections);
        DressFilter();
        // The bottom dock is a chrome setting both ways: the entry opens the cap map there, and a panel the researcher
        // opens there is theirs, so Jumble off leaves it.
        Settings.Set("dock.bottom", frame.Model.Docks[DockSide.Bottom].Open);
        Settings.Changed += key => { if (key == "dock.bottom" && !following) Dock(); };
        frame.Model.Changed += () => { if (!following) Settings.Set("dock.bottom", frame.Model.Docks[DockSide.Bottom].Open); Place(); };
        source.Changed += Follow;
        frame.Rebuilt += Place;
        frame.Rebuilt += () => { if (GodotObject.IsInstanceValid(CapMap)) CapMap.Refresh(); };
        WireTint();
        WireOverlay();
        Follow(source.State);
    }

    // SF3 (27 Nov 2026): while the shell binds a state (the feed, the totals, the style, the cap map, the ν band, the
    // tint), the border rows are placed and Local's sections refreshed once, at Release, as the last update left them;
    // each update placed and refreshed them itself before, four times in a bind (20-28 ms a bind in G1).
    private int holding;
    private bool placeDue;

    public void Hold() { holding++; Surface.Hold(); }

    public void Release()
    {
        Surface.Release();
        if (holding > 0 && --holding == 0 && placeDue) { placeDue = false; Place(); }
    }

    // Global's totals for the bound state, or pending while none is bound (ADR-D02.22).
    public void SetTotals(GlobalTotals value)
    {
        IReadOnlyList<ReadoutSlot> slots = value.Slots(unit, above);
        totalsMarked = ReadoutChanges.Marked(totals, slots); totals = slots;
        Place();
    }

    // The named cap's readout on Local's border, or none while no cap is named.
    public void SetCap(CapReadout? value)
    {
        IReadOnlyList<ReadoutSlot>? slots = value?.Slots(unit);
        capMarked = slots == null ? new HashSet<int>() : ReadoutChanges.Marked(cap, slots); cap = slots;
        Place();
    }

    // The cap map for the bound state, or every cap pending while none is bound (ADR-D02.13, .14). A named cap stays
    // named, and its readout follows the new state.
    public void SetCapMap(CapLayout? layout, CapMapModel map)
    {
        this.layout = layout; this.map = map;
        if (GodotObject.IsInstanceValid(CapMap)) CapMap.Show(layout, map);
        Named();
    }

    public void SetMeeting(MeetingCaps? value) { meeting = value; Named(); }

    // The bound state's pose rows; the domain and height layers count again from them.
    public void SetStyle(PoseStyle? value)
    {
        Style = value;
        if (filter == null && value != null)
        {
            filter = new PieceFilterModel(value.Rows, value.LatticeDomain, value.Domains.Count, () => Source.State.Revision, value.Digest);
            filter.Changed += SyncFilter;
            filter.AppliedChanged += FocusAppliedFilter;
        }
        else filter?.Rebind(value);
        FilterEditor.Bind(Filter);
        Recount();
        Reclass();
    }

    // The Focus layer entry on Global's border, beside its focus-layer keys (the default Solve view).
    private void DressFilter()
    {
        if (!GodotObject.IsInstanceValid(frame) || frame.Global is not { } global || !GodotObject.IsInstanceValid(global)) return;
        if (global.Tools.GetNodeOrNull("FocusLayerEntry") != null) return;
        global.Tools.AddChild(new FocusLayerEntry(OpenFilter));
    }

    // filter.open: the editor shows in the right dock with the expression focused.
    public void OpenFilter() => frame.RevealAndFocus(DockSide.Right, FocusLayerEditor.PanelId, FilterEditor.Expression);

    // The applied filter is a focus layer in every view, focused when it first applies; cleared, it goes.
    private void SyncFilter()
    {
        PredicateLayer? layer = filter?.Layer();
        foreach (FocusLayers layers in Layers)
        {
            if (layer == null) { layers.RemovePredicate(PieceFilterModel.LayerId); continue; }
            bool fresh = layers.Predicates.All(p => p.Id != PieceFilterModel.LayerId);
            layers.AddPredicate(layer);
            if (fresh && layers.Focused != PieceFilterModel.LayerId) layers.Focus(PieceFilterModel.LayerId);
        }
    }

    private void FocusAppliedFilter()
    {
        if (Layers.ElementAtOrDefault(frame.Local.Focused ? 1 : 0) is { } layers)
            Filter?.FocusApplied(layers);
    }

    // Every predicate a view already has, counted at the named cap and the bound state; a layer that no longer reads
    // (a domain the new state lacks) keeps its label with a pending count and dims nothing.
    private void Recount()
    {
        int? named = Naming.Named;
        foreach (FocusLayers layers in Layers)
            foreach (PredicateLayer layer in layers.Predicates.ToArray())
            {
                PredicateLayer next = layer.Id switch
                {
                    JumbleFocus.Straddlers => JumbleFocus.StraddlersOf(named, named is int n && RecordSource != null ? RecordSource(n) : null),
                    JumbleFocus.Domain when Style is { } style && style.DomainIndex(domainShown) is int d => JumbleFocus.DomainOf(style, d),
                    JumbleFocus.Height when Style != null && heightShown is int k => JumbleFocus.HeightOf(Style, k),
                    JumbleFocus.Domain or JumbleFocus.Height => layer with { Count = Numeral.Pending, Applies = false },
                    _ => layer,
                };
                layers.AddPredicate(next);
            }
    }

    // The solver picked a repair row (a digit or Enter in the list's context), with the key that did it. Only a pick
    // reaches here, and only here is a row staged (ADR-D02.33).
    public void Pick(RepairRow row, UserEvent asked)
    {
        if (Repair == null) return;
        picks.Add(row);
        // The solver picked the row from the repair list: the solver's own word (the 17 October exception).
        Stage.Stage(Repair, row, RepairSteps, asked, Origin.Solver);
        Safe.Raise(RepairPicked, row);
    }

    // Where no sample gives the views their focus layers, the host keeps one set per view and dresses each view with
    // the cycling key and the predicate keys only; the cycling key is the sample's F10 (ADR-D01.46), aimed at the view
    // with focus.
    public void OwnLayers()
    {
        var own = new Dictionary<string, FocusLayers>(StringComparer.Ordinal) { ["global"] = new FocusLayers(), ["local"] = new FocusLayers() };
        Layers = new[] { own["global"], own["local"] };
        void Dress()
        {
            if (!GodotObject.IsInstanceValid(frame)) return;
            foreach ((ViewFrame? view, string id) in new[] { (frame.Global, "global"), (frame.Local, "local") })
                if (view != null && GodotObject.IsInstanceValid(view) && view.Tools.GetNodeOrNull("LayerKeys") == null)
                {
                    var keys = new LayerKeys(own[id], () => frame.Tokens, key => key == "cycle" ? "F10" : "", picks: false);
                    view.Tools.AddChild(keys); view.Tools.MoveChild(keys, 0);
                }
        }
        Dress();
        frame.Rebuilt += Dress;
        if (!frame.HostKeys.ContainsKey("F10")) frame.HostKeys["F10"] = () => own[frame.Local.Focused ? "local" : "global"].Cycle();
    }

    // A focus row: the predicate joins every view's layers with its count; run again with the same predicate, it
    // un-focuses. The cycling key then dims or hides the rest (ADR-D01.46, ADR-D02.30).
    private void FocusOn(PredicateLayer layer)
    {
        foreach (FocusLayers layers in Layers)
        {
            bool same = layers.Focused == layer.Id && layers.Predicates.First(p => p.Id == layer.Id).Label == layer.Label;
            layers.AddPredicate(layer);
            if (same || layers.Focused != layer.Id) layers.Focus(layer.Id);
        }
    }

    private int? FocusDomain(string? argument) => Style is { } style
        ? style.DomainIndex(argument) ?? (Whole(argument) is int d && d < style.Domains.Count ? d : (int?)null)
        : null;

    private static int? Whole(string? argument) =>
        int.TryParse(argument?.Trim(), System.Globalization.NumberStyles.None, System.Globalization.CultureInfo.InvariantCulture, out int value) ? value : null;

    public bool Name(int cap) => Naming.Name(cap);

    // Words that send the reader to the palette by a key another surface holds, rerouted to the palette's own key; the
    // shell sets it in sample mode, where the key map holds F1 (B32 findings, ShellJumble.PaletteRoute). Null keeps them.
    public Func<string, string>? PaletteWords { get; set; }

    // Why a click on a cap names nothing now, or null; the cap map's tooltip, the prompt's rows and the status line read it.
    public string? Refusal(int cap) =>
        CapNaming.Refusal(cap, Source.State.Mode, Naming.Named) is not { } why ? null : PaletteWords is { } route ? route(why) : why;

    // A click on a cap map cell, a typed number and Enter there, or the prompt's choice: it names the cap, or says why
    // it names nothing in the status line (owner's bug report, 26 November 2026: a click did nothing visible).
    public bool Click(int cap)
    {
        if (Refusal(cap) is { } why) { frame.Say(why, true); return false; }
        return Name(cap);
    }

    // Formulas from the field or the palette; an empty text clears the threshold. A formula that does not read
    // leaves the threshold as it was and says why.
    public bool SetThreshold(string text)
    {
        try { Threshold = CapThreshold.Parse(text); refused = null; }
        catch (FormatException e) { refused = e.Message; }
        Remark();
        return refused == null;
    }

    public void ClearThreshold() { Threshold = CapThreshold.None; refused = null; Remark(); }

    // The named cap changed, or the state under it: the readout, Local's facts, the section count and the map.
    private void Named()
    {
        int? cap = Naming.Named;
        int revision = (int)Source.State.Revision;
        CapReadout? readout = cap is int c ? map.Readout(c) : null;
        string? unavailable = readout?.CertificateState == DiagnosticState.Failed ? LocalAccordion.Unavailable : null;
        if (Checks?.Cap != cap || Checks?.Revision != revision)
        {
            // Try numbers restart at each commit: a new cap or revision has its own list.
            Checks = cap is int named ? new KeptChecks(releases, named, revision) : null;
            seat.Clear();
            if (Checks != null) Checks.Changed += () => { Local.SetCount("twist_check", Checks?.Entries.Count ?? 0); Surface.SetChecks(Checks); };
            if (Checks != null) Checks.Changed += MarkMissingChecks;
            Surface.SetChecks(Checks);
        }
        Local.SetCount("twist_check", Checks?.Entries.Count ?? 0, unavailable);
        // The bound state's results for the named cap: a binding that comes after the naming brings them.
        results = cap is int c2 && CheckSource != null ? CheckSource(c2) : new Dictionary<int, TwistCheck>();
        Local.SetCount("named_cap", cap == null ? 0 : 1);
        // Cap covers: a completed answer for the named cap, or none; the inspect line starts empty.
        Covers = cap is int c3 && CoversSource != null ? CoversSource(c3) : null;
        Inspect.Hover(null);
        Local.SetCount("cap_covers", Covers?.Rows.Count ?? 0, unavailable);
        Surface.SetCovers(Covers, Inspect, () => map);
        // The straddlers layer stays focused across named caps and counts at the new one (ADR-D02.30).
        Recount();
        // The repair list: the record's result for the named cap at this revision; no row is focused when it opens.
        string? record = cap is int c4 && RecordSource != null ? RecordSource(c4) : null;
        Repair = record != null ? RepairListModel.From(record, revision, RepairVariant) : null;
        if (Repair != null) Repair.Ledger = Ledger;
        RepairSteps = record != null ? RepairSteps.From(record) : new RepairSteps();
        Scrub.Show(null);
        Local.SetCount("repair_list", Repair?.Rows.Count ?? 0, unavailable ?? Repair?.SectionStatus);
        Surface.SetRepair(Repair, RepairSteps, Scrub);
        SetCap(readout);
        Surface.SetFacts(readout != null && layout != null ? CapFacts.Tier2(map, layout, readout) : null);
        FollowLocal();
        Remark();
        PublishErrorReadouts(readout, record);
    }

    // The shell says whether a state is bound; Local draws the default cap while one is.
    public void SetStateBound(Func<bool> bound)
    {
        stateBound = bound;
        FollowLocal();
        Place();
    }

    // Local's readout while it draws a cap that is not named, or none.
    public string? LocalWords => LocalCap.Words(hint);

    // The shell says whether Local draws without a Jumble cap (the base puzzle, ADR-D01.40).
    public void SetDrawnWithoutCap(bool value)
    {
        drawnWithoutCap = value;
        // B17: the base puzzle has no Jumble cap, covers, checks or repair rows, so Local hides those sections.
        Surface.SetJumbleSections(!value);
        FollowLocal();
    }

    // The cap Local draws follows the naming and the binding; the empty sentence shows only while Local draws nothing.
    private void FollowLocal()
    {
        LocalCap.Follow(Naming.Named, stateBound());
        Surface.SetDrawn(LocalCap.Shown != null || drawnWithoutCap);
    }

    // The threshold, the named cap and its meeting caps over the cap map, and the threshold row on Global's border.
    private void Remark()
    {
        if (!GodotObject.IsInstanceValid(CapMap)) return;
        IReadOnlyList<int> meets = Naming.Named is int c && meeting != null ? meeting.Of(c) : Array.Empty<int>();
        CapMap.Mark(Threshold, Naming.Named, meets, refused);
        Place();
    }

    public bool Handles(string command) => Carried.Contains(command);

    public string? Unavailable(string command, string? argument) => command switch
    {
        GripFrame.SelectRow or GripFrame.NextRow => GripRow == null ? "Grip needs a live Jumble session" : GripRow(command, argument, false),
        _ when FilterRows.Contains(command) && Filter == null => "no state is bound: the focus layer counts the bound sample state's pieces",
        "filter.preset" when PieceFilterModel.Presets.All(p => p.Name != argument) => "it needs a preset: " + string.Join(", ", PieceFilterModel.Presets.Select(p => p.Name)),
        "filter.save" when string.IsNullOrWhiteSpace(argument) => "it needs a name, as in > filter.save lattice",
        "filter.load" when string.IsNullOrWhiteSpace(argument) => "it needs a saved name",
        "structure.filter.compose" when !Enum.TryParse<FilterComposition>(argument, true, out _) => "it needs replace, intersect, union or subtract",
        _ when JumbleViews.ChartRows.Contains(command) && ChartRow == null => "this build draws no Global chart",
        _ when JumbleViews.ChartRows.Contains(command) => null,
        _ when FilterRows.Contains(command) => null,
        _ when !Source.State.Mode => "Jumble is off",
        _ when RawView.Rows.Contains(command) && RawRow == null => "this build draws no raw view",
        SplitRow or ShellRow when SliceRow == null => "this build draws no cut slice",
        ShellRow when CutSlice.ParseShell(argument) == null => "it needs the shell's outer radius in radians, 0.05 to 3.14, or default, as in > jumble.slice.shell 0.8",
        RepairFocusRow or RepairStageRow when Naming.Named == null => "name a cap first",
        RepairFocusRow or RepairStageRow when Repair == null => "the samples hold no repair list at cap " + Naming.Named,
        RepairFocusRow or RepairStageRow when Repair!.Error != null || Repair!.End == JobEnd.Cancelled => "the repair list shows no rows to focus or stage",
        RepairFocusRow when argument != null && RepairPlace(argument) == null => "it needs a row 1 to " + Repair!.Rows.Count + " and, if you like, its step, as in > jumble.repair.focus 2 2",
        RepairStageRow when Repair!.End != JobEnd.Completed => "only a completed repair list stages a row",
        RepairStageRow when argument == null && Repair!.Focused == null => "it needs a row 1 to " + Repair!.Rows.Count + ", or a focused row",
        RepairStageRow when argument != null && RepairPlace(argument) is not (_, 0) => "it needs a row 1 to " + Repair!.Rows.Count + ", as in > jumble.repair.stage 2",
        RawView.ClipRow when RawView.ParseClip(argument) == null => "it needs h from 0 (90°) to 0.968 (the named cap's cut), cut or tick, as in > view.mode.raw.clip 0.5",
        // Without a number the row opens the cap prompt.
        "jumble.cap.name" when argument != null && CapNaming.Typed(argument) == null => "no cap is numbered " + argument + "; caps are 0 to 599",
        "jumble.cap.unname" when Naming.Named == null => "no cap is named",
        "jumble.capmap.threshold.set" when argument == null => "it needs formulas: N_c ≥ k, N_c ≤ k or N_c = F_c",
        "jumble.capmap.threshold.clear" when !Threshold.IsSet => "no threshold is set",
        "jumble.seat.hold" when Checks == null => "name a cap first",
        "jumble.seat.hold" when argument != "1" => "the samples hold seat 1's twelve checks only",
        "jumble.seat.hold" when results.Count == 0 => "the samples hold no twist checks at cap " + Checks!.Cap,
        "jumble.focus.straddlers" or "jumble.focus.domain" or "jumble.focus.height" when Layers.Count == 0 => "no view here takes focus layers",
        "jumble.focus.domain" or "jumble.focus.height" when Style == null => "no state is bound",
        "jumble.focus.domain" when FocusDomain(argument) == null => "it needs the domain ID from the inspector or a number from 0 to " + (Style!.Domains.Count - 1),
        "jumble.focus.height" when Whole(argument) == null => "it needs k, as in > jumble.focus.height 4",
        JumbleTint.Row when argument != null && JumbleTint.Parse(argument) == null => "it needs a mode: none, contact, height or off-lattice; without one it steps to the next",
        "jumble.corrections.mark" or "jumble.corrections.check" when Corrections.Model == null => "the build has no ν-correction catalogue",
        "jumble.corrections.mark" when MarkOf(argument) == null => "it needs a row id and a multiplicity, as in > jumble.corrections.mark 35 +1",
        _ => null,
    };

    public void Execute(string command, string? argument)
    {
        switch (command)
        {
            case var row when JumbleViews.ChartRows.Contains(row): ChartRefused = ChartRow != null ? ChartRow(row) : null; break;
            case var row when RawView.Rows.Contains(row): RawRefused = RawRow != null ? RawRow(row, argument) : null; break;
            // B32 findings: the entry slot applies its value again (a key the researcher changed since is set again).
            case EntrySlot.Id: Entry.Revert(); Entry.Apply(); break;
            case SplitRow or ShellRow: SliceRefused = SliceRow != null ? SliceRow(command, argument) : null; break;
            case RepairFocusRow: FocusRepair(argument); break;
            case RepairStageRow when Repair != null && (argument != null ? RepairPlace(argument) is (int k, 0) ? k : 0 : Repair.Focused ?? 0) is int pick and > 0:
                // The solver named the row (or focused it) and pressed Enter in the palette: the solver's pick.
                Pick(Repair.Rows[pick - 1], UserEvent.Key("palette Enter"));
                break;
            case "filter.open": OpenFilter(); break;
            case "jumble.corrections.mark": if (MarkOf(argument) is (int id, int by)) Corrections.Model?.Mark(id, by); OpenCorrections(); break;
            case "jumble.corrections.check": Corrections.CheckSet(); OpenCorrections(); break;
            case "filter.preview": if (argument != null) Filter?.SetText(argument); Filter?.Run(); OpenFilter(); break;
            // The palette's Enter asked for it.
            case "filter.apply": Filter?.Apply(UserEvent.Key("palette Enter")); OpenFilter(); break;
            case "filter.edit": Filter?.Edit(); OpenFilter(); break;
            case "filter.preset": Filter?.Preset(argument!); OpenFilter(); break;
            case "filter.save": Filter?.Save(argument!); OpenFilter(); break;
            case "filter.load": Filter?.Load(argument!); OpenFilter(); break;
            case "structure.filter.compose": if (Enum.TryParse(argument, true, out FilterComposition c)) Filter?.Compose(c); OpenFilter(); break;
            case "jumble.cap.name" when argument == null: Prompt.Open(); break;
            case "jumble.cap.name": if (CapNaming.Typed(argument) is int cap) Name(cap); break;
            case "jumble.cap.unname": Naming.Unname(); break;
            case "jumble.capmap.threshold.set": SetThreshold(argument ?? ""); break;
            case "jumble.capmap.threshold.clear": ClearThreshold(); break;
            case "jumble.seat.hold": HoldSeat(); break;
            case "jumble.focus.straddlers":
                FocusOn(JumbleFocus.StraddlersOf(Naming.Named, Naming.Named is int n && RecordSource != null ? RecordSource(n) : null));
                break;
            case "jumble.focus.domain" when Style != null && FocusDomain(argument) is int d:
                domainShown = Style.Domains[d]; FocusOn(JumbleFocus.DomainOf(Style, d)); DimOthers();
                break;
            case JumbleTint.Row: SetTint(argument); break;
            case GripFrame.SelectRow or GripFrame.NextRow:
                if (GripRow != null && GripRow(command, argument, true) is { } why) frame.Say(why, true);
                break;
            case "jumble.focus.height" when Style != null && Whole(argument) is int k:
                heightShown = k; FocusOn(JumbleFocus.HeightOf(Style, k));
                break;
        }
    }

    // B32 findings: `jumble.repair.focus [k [s]]` opens Local's repair list and gives it keyboard focus with no row
    // focused (ADR-D02.32); with k it focuses row k, and with s it shows step s of k's moving sets (ADR-D02.34). Then the
    // list's own keys work: arrows to focus, digits 1 to 9 or Enter to pick, Left and Right for the steps.
    private void FocusRepair(string? argument)
    {
        if (Repair == null) return;
        if (Local.Open != "repair_list") Local.Toggle("repair_list");
        if (argument != null && RepairPlace(argument) is (int row, int step) && Repair.Focus(row, UserEvent.Key("palette Enter")))
        {
            Scrub.Show(Repair.Rows[row - 1]);
            for (int k = 1; k < step; k++) Scrub.Scrub("forward");
        }
        Surface.FocusRepair();
    }

    // A repair row 1 to m and an optional step 1 to its m (0: none given), or null.
    private (int Row, int Step)? RepairPlace(string argument)
    {
        string[] parts = argument.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        if (Repair == null || parts.Length is < 1 or > 2 || Whole(parts[0]) is not int row || row < 1 || row > Repair.Rows.Count) return null;
        if (parts.Length == 1) return (row, 0);
        return Whole(parts[1]) is int step && step >= 1 && step <= Repair.Rows[row - 1].Steps ? (row, step) : null;
    }

    // B10's stand-in for holding seat 1 and pressing its twelve twist keys (CR-19 routes the chords): the twelve
    // releases at the Q3.b rhythm, from the next tick. The Twist check section opens to show them.
    public void HoldSeat()
    {
        if (Checks == null) return;
        seat.Clear();
        for (int i = 0; i < SeatRhythm.Count; i++) seat.Add((jobs.Now + 1 + SeatRhythm[i], i + 1));
        if (Local.Open != "twist_check") Local.Toggle("twist_check");
    }

    // The jobs' clock stands still while Paused: the shell's Q3.b check holds it between its two reads of the screen,
    // so that no check completes in between (C2-1).
    public bool Paused { get; set; }

    // A newer request supersedes the older ones (Q3.b): the job layer withdraws every release at the named cap that is
    // no longer the latest request, and the kept list takes each cancel, which N-1 says leaves the screen unchanged.
    // Answers how many releases were cancelled.
    public int CancelOlder()
    {
        if (Checks is not { } kept) return 0;
        long[] older = kept.Released.Where(r => r != kept.Pending).ToArray();
        foreach (long release in older)
        {
            jobs.Cancel(release);
            kept.Cancel(release);
        }
        return older.Length;
    }

    // One tick of the sample jobs' clock: due checks complete (only the latest is taken), then due releases are made.
    public void Tick()
    {
        if (Paused) return;
        foreach (var (release, item) in jobs.Tick())
            if (Checks != null && results.TryGetValue(item, out TwistCheck? check)) Checks.Complete(release, check);
        foreach (var (_, item) in seat.Where(s => s.Tick == jobs.Now).ToArray())
        {
            if (Checks == null) break;
            jobs.Request(Checks.Release(item), item);
        }
        seat.RemoveAll(s => s.Tick <= jobs.Now);
    }

    public IReadOnlyList<ReadoutSlot> Totals => totals;
    public IReadOnlyList<ReadoutSlot>? Cap => cap;

    // The readout rows on the two borders, while Jumble is on; a rebuilt frame gets fresh rows.
    private void Place()
    {
        if (holding > 0) { placeDue = true; return; }
        if (!GodotObject.IsInstanceValid(frame) || frame.Global == null || frame.Local == null) return;
        Row(frame.Global, "JumbleTotals", totals, totalsMarked, Source.State.Mode);
        Row(frame.Local, "JumbleCap", cap ?? Array.Empty<ReadoutSlot>(), capMarked, Source.State.Mode && cap != null);
        MarkCertificate();
        DefaultLine(frame.Local);
        Legends();
        Border(frame.Global, Source.State.Mode && CapThreshold.OnGlobalBorder(frame.Model.Docks[DockSide.Bottom].Open));
    }

    // Local's readout while it draws a cap that is not named: it says so, with the hint, which its tooltip repeats.
    private void DefaultLine(ViewFrame view) => ViewLine.Set(view, "JumbleLocalDefault", LocalWords, hint, frame.Tokens.C("text.dim"));

    // ADR-D02.15: while the cap map's dock is closed, the threshold field and its count stay on Global's border.
    private void Border(ViewFrame view, bool on)
    {
        if (!GodotObject.IsInstanceValid(view)) return;
        HBoxContainer? row = view.Tools.GetNodeOrNull<HBoxContainer>("JumbleThreshold");
        if (!on)
        {
            if (row != null) { view.Tools.RemoveChild(row); row.QueueFree(); }
            return;
        }
        TokenSet t = frame.Tokens;
        if (row == null)
        {
            row = new HBoxContainer { Name = "JumbleThreshold" };
            var field = new LineEdit { Name = "Field", CustomMinimumSize = new Vector2(180, 0), TooltipText = "The cap map's threshold; Enter sets it, an empty field clears it" };
            field.TextSubmitted += text => SetThreshold(text);
            row.AddChild(field);
            row.AddChild(ChromeWidgets.Text("", "MicroLabel"));
            view.Tools.AddChild(row); view.Tools.MoveChild(row, 0);
        }
        var edit = row.GetNode<LineEdit>("Field");
        if (!edit.HasFocus()) edit.Text = Threshold.Formula;
        var count = row.GetChild<Label>(1);
        count.Text = refused ?? (Threshold.Counts(map) is ThresholdCounts c ? CapThreshold.Line(c) : "");
        count.AddThemeColorOverride("font_color", t.C(refused != null ? "state.invalid" : "text.dim"));
    }

    private void Row(ViewFrame view, string name, IReadOnlyList<ReadoutSlot> slots, IReadOnlySet<int> marked, bool on)
    {
        if (!GodotObject.IsInstanceValid(view)) return;
        // The readouts take the footer's full width and wrap there (owner, 26 November 2026: the frame fits the window).
        ReadoutRow? row = view.Readouts.GetNodeOrNull<ReadoutRow>(name);
        if (!on)
        {
            if (row != null) { view.Readouts.RemoveChild(row); row.QueueFree(); }
            return;
        }
        if (row == null) { row = new ReadoutRow(name, () => frame.Tokens) { SizeFlagsHorizontal = Control.SizeFlags.ExpandFill }; view.Readouts.AddChild(row); }
        row.Show(slots, marked);
    }

    // The strip and the rail to switch to the undefined numeral, where the frame has them.
    public void Solving(RelationStrip? strip, OrbitRail? rail)
    {
        this.strip = strip; this.rail = rail;
        if (rail != null && GodotObject.IsInstanceValid(rail)) { rail.AddBand(NuStrip); rail.OrderChanged += BandNu; }
        BandNu();
        Follow(Source.State);
    }

    // B26: the bound state's ν facts (N, the lattice flag, the ν row) and the correction catalogue, or none.
    public void SetNu(NuCorrections? catalogue, WordState? state)
    {
        nuConstants = catalogue?.Constants; nuState = state;
        catalogue?.ClearCheck();
        Corrections.Bind(catalogue, () => Source.State.LatticeFlag && nuState is { OnLattice: true, Nu: { } nu } ? nu : null);
        WatchCorrectionCheck();
        BandNu();
    }

    // The band in the rail's current order; in orbit-id order where the frame has no rail.
    private void BandNu()
    {
        IReadOnlyList<int> order = rail != null && GodotObject.IsInstanceValid(rail) ? rail.Cells().Select(c => c.Orbit).ToArray() : Enumerable.Range(0, OrbitAtlas.Count).ToArray();
        Nu = NuBand.ForState(nuConstants, order, Source.State, nuState?.N, nuState?.Nu);
        if (GodotObject.IsInstanceValid(NuStrip)) NuStrip.SetBand(Nu);
    }

    public void OpenCorrections() => frame.RevealAndFocus(DockSide.Left, NuCorrectionsPanel.PanelId, Corrections);

    // "35 +1", "0 -1" or "7" (+1): a row id and a nonzero signed multiplicity.
    private static (int Id, int By)? MarkOf(string? argument)
    {
        string[] parts = (argument ?? "").Split(' ', StringSplitOptions.RemoveEmptyEntries);
        if (parts.Length is < 1 or > 2 || !int.TryParse(parts[0], System.Globalization.NumberStyles.None, System.Globalization.CultureInfo.InvariantCulture, out int id) || id >= NuCorrections.Expected) return null;
        int by = 1;
        if (parts.Length == 2 && (!int.TryParse(parts[1], System.Globalization.NumberStyles.AllowLeadingSign, System.Globalization.CultureInfo.InvariantCulture, out by) || by == 0)) return null;
        return (id, by);
    }

    private void Dock()
    {
        string? open = Settings.Get("dock.bottom");
        following = true;
        try { frame.Model.ShowDock(DockSide.Bottom, open != null && frame.Model.Docks[DockSide.Bottom].Panels.Contains(open) ? open : null); }
        finally { following = false; }
    }

    private void Follow(JumbleState state)
    {
        if (!GodotObject.IsInstanceValid(frame)) return;
        Entry.Follow(state);
        frame.SetModeCell(JumbleChrome.ModeCell(state), state.LatticeFlag
            ? "Jumble is on, and every piece's pose is on the lattice"
            : "Jumble is on, and some pieces are off the lattice: N and the solving numerals are undefined");
        // Tier 1 says what is unavailable while Jumble is on (ADR-D02.25): the named cap's line and the inspected cap's.
        Inspect.State = state;
        if (GodotObject.IsInstanceValid(CapMap))
        {
            bool flipped = CapMap.State.Mode != state.Mode;
            CapMap.State = state;
            if (flipped) CapMap.Refresh();
        }
        bool undefined = JumbleNumerals.Undefined(state);
        if (strip != null && GodotObject.IsInstanceValid(strip)) strip.SetUndefined(undefined);
        if (rail != null && GodotObject.IsInstanceValid(rail)) rail.SetUndefined(undefined);
        if (state.Revision != nuRevision)
        {
            nuRevision = state.Revision;
            Corrections.Model?.ClearCheck();
        }
        BandNu();
        Place();
        Safe.Raise(Changed);
    }

    // The Jumble mark on a view's border while Jumble is on: a notched-corner glyph with its word, never colour alone.
    // B27: then Global's projection chain (ADR-D02.8) and the exposed faces' unavailable state (ADR-D02.25).
    public void Marks(ViewFrame view)
    {
        if (JumbleChrome.Marked(Source.State, new[] { view.ViewName }).Count != 0)
        {
            view.AddMark(JumbleChrome.Mark, JumbleChrome.MarkWords, "accent.strong", "Jumble is on: this view shows the jumbled puzzle");
            foreach (ViewMark mark in JumbleViews.BorderMarks(Source.State, view.ViewName, Chart != null ? Chart() : null)) view.AddMark(mark.Glyph, mark.Words, mark.Colour, mark.Tooltip);
        }
        // B21: Local's raw view names its caption and the antipode chart; C3/C4 finding F6: whenever Local draws the raw
        // view, Jumble's mode on or off (the raw view's Home acts at any cap Local draws).
        if (view.ViewName == "Local" && Raw != null && Raw() is { } raw)
            foreach (ViewMark mark in raw.Marks()) view.AddMark(mark.Glyph, mark.Words, mark.Colour, mark.Tooltip);
    }
}
