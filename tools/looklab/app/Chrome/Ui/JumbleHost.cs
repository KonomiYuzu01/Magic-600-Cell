using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The Jumble host in Solve (ADR-D02.6, .11, .12; build step B6): from a Jumble source it keeps the entry slot on the
// adopted revision's mode, draws the mode cell in the top bar and the Jumble mark on every view border, switches the
// strip and the rail to the undefined numeral on the exact lattice flag, and puts Local's sections in Local. It adds
// one panel, the cap map, to the existing bottom dock, and no layout and no dock.
public sealed class JumbleHost
{
    public IJumbleSource Source { get; }
    public ChromeSettings Settings { get; } = new();
    public EntrySlot Entry { get; }
    public LocalAccordion Local { get; } = new();
    public JumbleSurface Surface { get; }
    public CapMapPanel CapMap { get; }
    // B9: the named cap, the threshold, the meeting caps and the job gate (ADR-D02.15, .16, .19).
    public CapNaming Naming { get; } = new();
    public CapThreshold Threshold { get; private set; } = CapThreshold.None;
    public CapJobs Jobs { get; }
    // A named cap's own record (u at a blocked cap needs its straddlers), where the source has one.
    public Func<int, CapReadout?>? CapRecord { get; set; }
    private CapLayout? layout;
    private CapMapModel map = CapMapModel.Pending();
    private MeetingCaps? meeting;
    private string? refused;
    // The draft rows (CR-14, and CR-19's seat key as B10's stand-in) this host carries out.
    public static readonly IReadOnlySet<string> Carried = new HashSet<string>(StringComparer.Ordinal)
        { "jumble.cap.name", "jumble.cap.unname", "jumble.capmap.threshold.set", "jumble.capmap.threshold.clear", "jumble.seat.hold",
          "jumble.focus.straddlers", "jumble.focus.domain", "jumble.focus.height" };
    // B11: the named cap's covers with the hover's inspect line, its raw record (the straddlers' count), the bound
    // state's pose rows (the domain and height counts), and the views' focus layers the three predicates join.
    public CapCovers? Covers { get; private set; }
    public CoversInspect Inspect { get; } = new();
    public Func<int, CapCovers?>? CoversSource { get; set; }
    public Func<int, string?>? RecordSource { get; set; }
    public PoseStyle? Style { get; private set; }
    public IReadOnlyList<FocusLayers> Layers { get; set; } = Array.Empty<FocusLayers>();
    private int? domainShown, heightShown;
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
        this.frame = frame; Source = source; this.unit = unit; this.above = above;
        totals = GlobalTotals.Pending.Slots(unit, above);
        Entry = new EntrySlot(Settings);
        Surface = new JumbleSurface(Local, () => frame.Tokens, names, emptySentence, emptyBody);
        string title = names.TryGetValue("cap_map", out string? name) ? name : "Cap map";
        CapMap = new CapMapPanel(Settings, () => frame.Tokens, title);
        Jobs = new CapJobs(Naming);
        CapMap.Map.Naming = Naming;
        CapMap.Map.Picked += cap => Name(cap);
        CapMap.ThresholdTyped += text => SetThreshold(text);
        Naming.Changed += _ => Named();
        frame.AddDockPanel(DockSide.Bottom, EntrySlot.CapMapPanel, title, "panel.capmap", CapMap);
        // The bottom dock is a chrome setting both ways: the entry opens the cap map there, and a panel the researcher
        // opens there is theirs, so Jumble off leaves it.
        Settings.Set("dock.bottom", frame.Model.Docks[DockSide.Bottom].Open);
        Settings.Changed += key => { if (key == "dock.bottom" && !following) Dock(); };
        frame.Model.Changed += () => { if (!following) Settings.Set("dock.bottom", frame.Model.Docks[DockSide.Bottom].Open); Place(); };
        source.Changed += Follow;
        frame.Rebuilt += Place;
        frame.Rebuilt += () => { if (GodotObject.IsInstanceValid(CapMap)) CapMap.Refresh(); };
        Follow(source.State);
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
    public void SetStyle(PoseStyle? value) { Style = value; Recount(); }

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
                    JumbleFocus.Domain when Style != null && domainShown is int d && d < Style.Domains.Count => JumbleFocus.DomainOf(Style, d),
                    JumbleFocus.Height when Style != null && heightShown is int k => JumbleFocus.HeightOf(Style, k),
                    JumbleFocus.Domain or JumbleFocus.Height => layer with { Count = Numeral.Pending, Applies = false },
                    _ => layer,
                };
                layers.AddPredicate(next);
            }
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

    private static int? Whole(string? argument) =>
        int.TryParse(argument?.Trim(), System.Globalization.NumberStyles.None, System.Globalization.CultureInfo.InvariantCulture, out int value) ? value : null;

    public bool Name(int cap) => Naming.Name(cap);

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
        if (Checks?.Cap != cap || Checks?.Revision != revision)
        {
            // Try numbers restart at each commit: a new cap or revision has its own list.
            Checks = cap is int named ? new KeptChecks(releases, named, revision) : null;
            seat.Clear();
            if (Checks != null) Checks.Changed += () => { Local.SetCount("twist_check", Checks?.Entries.Count ?? 0); Surface.SetChecks(Checks); };
            Local.SetCount("twist_check", 0);
            Surface.SetChecks(Checks);
        }
        // The bound state's results for the named cap: a binding that comes after the naming brings them.
        results = cap is int c2 && CheckSource != null ? CheckSource(c2) : new Dictionary<int, TwistCheck>();
        Local.SetCount("named_cap", cap == null ? 0 : 1);
        // Cap covers: a completed answer for the named cap, or none; the inspect line starts empty.
        Covers = cap is int c3 && CoversSource != null ? CoversSource(c3) : null;
        Inspect.Hover(null);
        Local.SetCount("cap_covers", Covers?.Rows.Count ?? 0);
        Surface.SetCovers(Covers, Inspect, () => map);
        // The straddlers layer stays focused across named caps and counts at the new one (ADR-D02.30).
        Recount();
        CapReadout? readout = cap is int c ? (CapRecord != null ? CapRecord(c) : null) ?? map.Readout(c) : null;
        SetCap(readout);
        Surface.SetFacts(readout != null && layout != null ? CapFacts.Tier2(map, layout, readout) : null);
        Remark();
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
        _ when !Source.State.Mode => "Jumble is off",
        "jumble.cap.name" when argument == null => "it needs a cap number, as in > jumble.cap.name 150",
        "jumble.cap.name" when CapNaming.Typed(argument) == null => "no cap is numbered " + argument + "; caps are 0 to 599",
        "jumble.cap.unname" when Naming.Named == null => "no cap is named",
        "jumble.capmap.threshold.set" when argument == null => "it needs formulas: N_c ≥ k, N_c ≤ k or N_c = F_c",
        "jumble.capmap.threshold.clear" when !Threshold.IsSet => "no threshold is set",
        "jumble.seat.hold" when Checks == null => "name a cap first",
        "jumble.seat.hold" when argument != "1" => "the samples hold seat 1's twelve checks only",
        "jumble.seat.hold" when results.Count == 0 => "the samples hold no twist checks at cap " + Checks!.Cap,
        "jumble.focus.straddlers" or "jumble.focus.domain" or "jumble.focus.height" when Layers.Count == 0 => "no view here takes focus layers",
        "jumble.focus.domain" or "jumble.focus.height" when Style == null => "no state is bound",
        "jumble.focus.domain" when Whole(argument) is not int d || d >= Style!.Domains.Count => "it needs a domain number, 0 to " + (Style!.Domains.Count - 1) + ", as in > jumble.focus.domain 5",
        "jumble.focus.height" when Whole(argument) == null => "it needs k, as in > jumble.focus.height 4",
        _ => null,
    };

    public void Execute(string command, string? argument)
    {
        switch (command)
        {
            case "jumble.cap.name": if (CapNaming.Typed(argument) is int cap) Name(cap); break;
            case "jumble.cap.unname": Naming.Unname(); break;
            case "jumble.capmap.threshold.set": SetThreshold(argument ?? ""); break;
            case "jumble.capmap.threshold.clear": ClearThreshold(); break;
            case "jumble.seat.hold": HoldSeat(); break;
            case "jumble.focus.straddlers":
                FocusOn(JumbleFocus.StraddlersOf(Naming.Named, Naming.Named is int n && RecordSource != null ? RecordSource(n) : null));
                break;
            case "jumble.focus.domain" when Style != null && Whole(argument) is int d && d < Style.Domains.Count:
                domainShown = d; FocusOn(JumbleFocus.DomainOf(Style, d));
                break;
            case "jumble.focus.height" when Style != null && Whole(argument) is int k:
                heightShown = k; FocusOn(JumbleFocus.HeightOf(Style, k));
                break;
        }
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

    // One tick of the sample jobs' clock: due checks complete (only the latest is taken), then due releases are made.
    public void Tick()
    {
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
        if (!GodotObject.IsInstanceValid(frame) || frame.Global == null || frame.Local == null) return;
        Row(frame.Global, "JumbleTotals", totals, totalsMarked, Source.State.Mode);
        Row(frame.Local, "JumbleCap", cap ?? Array.Empty<ReadoutSlot>(), capMarked, Source.State.Mode && cap != null);
        Border(frame.Global, Source.State.Mode && Threshold.IsSet && CapThreshold.OnGlobalBorder(frame.Model.Docks[DockSide.Bottom].Open));
    }

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
        ReadoutRow? row = view.Tools.GetNodeOrNull<ReadoutRow>(name);
        if (!on)
        {
            if (row != null) { view.Tools.RemoveChild(row); row.QueueFree(); }
            return;
        }
        if (row == null) { row = new ReadoutRow(name, () => frame.Tokens); view.Tools.AddChild(row); view.Tools.MoveChild(row, 0); }
        row.Show(slots, marked);
    }

    // The strip and the rail to switch to the undefined numeral, where the frame has them.
    public void Solving(RelationStrip? strip, OrbitRail? rail)
    {
        this.strip = strip; this.rail = rail;
        Follow(Source.State);
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
        bool undefined = JumbleNumerals.Undefined(state);
        if (strip != null && GodotObject.IsInstanceValid(strip)) strip.SetUndefined(undefined);
        if (rail != null && GodotObject.IsInstanceValid(rail)) rail.SetUndefined(undefined);
        Place();
        Safe.Raise(Changed);
    }

    // The Jumble mark on a view's border while Jumble is on: a notched-corner glyph with its word, never colour alone.
    public void Marks(ViewFrame view)
    {
        if (JumbleChrome.Marked(Source.State, new[] { view.ViewName }).Count == 0) return;
        view.AddMark(JumbleChrome.Mark, JumbleChrome.MarkWords, "accent.strong", "Jumble is on: this view shows the jumbled puzzle");
    }
}
