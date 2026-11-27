using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// Local's Jumble content (ADR-D02.6, .12): the empty sentence while Local draws no cap (no state bound; Local is never
// empty otherwise, owner, 26 November 2026), then the four sections, one open at a time, each closed one a one-line
// header with its count. The cut slice (B20) and the raw view (B21) draw later;
// until then Local shows this. Every section is empty until its build step fills it.
public sealed partial class JumbleSurface : VBoxContainer
{
    public LocalAccordion Accordion { get; }
    private readonly Func<TokenSet> tokens;
    private readonly IReadOnlyDictionary<string, string> names;
    private readonly string emptyBody;
    private readonly Label sentence;
    // Room above the empty sentence for the view's marks, which the frame draws over the top edge of Local.
    private readonly Control gap = new() { Name = "LocalTopGap", MouseFilter = MouseFilterEnum.Ignore };
    private readonly VBoxContainer sections = new() { Name = "LocalSections" };
    // The named cap's Tier 2 facts in their fixed order (ADR-D02.19), shown in the Named cap section; none while no
    // cap is named.
    private IReadOnlyList<(string Label, string Fact)>? facts;
    // Whether Local draws a cap, named or the default (LocalCap).
    private bool drawn;
    // Whether the Jumble sections show: not on the base puzzle, where they have nothing to show (B17).
    private bool jumbleSections = true;
    // The named cap's kept twist checks (B10), shown in the Twist check section.
    private KeptChecks? checks;
    // The named cap's covers (B11), shown in the Cap covers section, with its hover's inspect line and the table's fold.
    private CapCovers? covers;
    private CoversInspect? inspect;
    private Func<CapMapModel> map = CapMapModel.Pending;
    private bool coversTable;
    // The named cap's repair list (B12), its steps and the step focus, and the coverage line's fold.
    private RepairListModel? repair;
    private RepairSteps? repairSteps;
    private RepairScrub? scrub;
    private bool repairCoverage;
    // The solver picked a repair row in the list, with the key that did it.
    public event Action<RepairRow, UserEvent>? RepairPicked;

    public JumbleSurface(LocalAccordion accordion, Func<TokenSet> tokens, IReadOnlyDictionary<string, string> names, string emptySentence, string emptyBody)
    {
        Accordion = accordion; this.tokens = tokens; this.names = names; this.emptyBody = emptyBody;
        Name = "JumbleLocal"; SizeFlagsHorizontal = SizeFlags.ExpandFill; SizeFlagsVertical = SizeFlags.ExpandFill;
        sentence = ChromeWidgets.Text(emptySentence, "DimLabel"); sentence.Name = "LocalEmpty";
        sentence.AutowrapMode = TextServer.AutowrapMode.WordSmart; sentence.HorizontalAlignment = HorizontalAlignment.Center;
        AddChild(gap); AddChild(sentence); AddChild(sections);
        // A section's count changes in a bind too (SF3): it refreshes with the rest, once, at Release.
        accordion.Changed += () => RefreshOrHold();
        Refresh();
    }

    // The height the view's marks take at the top of Local: their offset and one control's height.
    public static int MarksRoom(TokenSet t) => t.Px("space.3") + t.Px("control.height");

    // SF3 (27 Nov 2026): held by the host while a bind updates several sections; they refresh once, at Release.
    private int holding;
    private bool refreshDue;

    public void Hold() => holding++;

    public void Release()
    {
        if (holding > 0 && --holding == 0 && refreshDue) { refreshDue = false; if (IsInstanceValid(this)) Refresh(); }
    }

    private void RefreshOrHold()
    {
        if (holding > 0) { refreshDue = true; return; }
        if (IsInstanceValid(this)) Refresh();
    }

    public void SetChecks(KeptChecks? value)
    {
        checks = value;
        RefreshOrHold();
    }

    public void SetCovers(CapCovers? value, CoversInspect inspect, Func<CapMapModel> map)
    {
        covers = value; this.inspect = inspect; this.map = map;
        RefreshOrHold();
    }

    public void SetRepair(RepairListModel? value, RepairSteps steps, RepairScrub scrub)
    {
        repair = value; repairSteps = steps; this.scrub = scrub;
        RefreshOrHold();
    }

    // B32 findings: the repair list's key route gives the open list keyboard focus once it is built (the palette has
    // closed by then), as a click on it does.
    public void FocusRepair() => Callable.From(() =>
    {
        if (IsInstanceValid(sections) && sections.FindChild("LocalBody_repair_list", false, false) is Control list && list.IsVisibleInTree())
            list.GrabFocus();
    }).CallDeferred();

    public void SetDrawn(bool value)
    {
        if (drawn == value) return;
        drawn = value;
        RefreshOrHold();
    }

    // The shell hides the Jumble sections on the base puzzle (Jumble off) and shows them otherwise (B17).
    public void SetJumbleSections(bool value)
    {
        if (jumbleSections == value) return;
        jumbleSections = value;
        RefreshOrHold();
    }

    public void SetFacts(IReadOnlyList<(string Label, string Fact)>? value)
    {
        facts = value;
        RefreshOrHold();
    }

    public void Refresh()
    {
        TokenSet t = tokens();
        sentence.Visible = facts == null && !drawn;
        sections.Visible = jumbleSections;
        // The marks row sits space.3 below the top edge and is at most a control high (owner, 26 November 2026: the
        // marks covered the sentence at 2000x1200).
        gap.Visible = sentence.Visible; gap.CustomMinimumSize = new Vector2(0, MarksRoom(t));
        AddThemeConstantOverride("separation", t.Px("space.2"));
        foreach (Node child in sections.GetChildren()) { sections.RemoveChild(child); child.QueueFree(); }
        foreach (string section in Accordion.Sections)
        {
            bool open = Accordion.Open == section;
            Button header = ChromeWidgets.Tool((open ? "▾ " : "▸ ") + Accordion.Header(section, names), open ? "Close this section" : "Open this section; the open one closes", true);
            header.Name = "LocalSection_" + section; header.Alignment = HorizontalAlignment.Left; header.SetPressedNoSignal(open);
            string id = section; header.Pressed += () => Accordion.Toggle(id);
            sections.AddChild(header);
            if (open && section == "named_cap" && facts != null)
            {
                var grid = new GridContainer { Name = "LocalBody_" + section, Columns = 2 };
                foreach ((string label, string fact) in facts)
                {
                    Label name = ChromeWidgets.Text(label, "MicroLabel"); name.AddThemeColorOverride("font_color", t.C("text.dim")); grid.AddChild(name);
                    grid.AddChild(ChromeWidgets.Text(fact, "ReadoutLabel"));
                }
                sections.AddChild(grid);
            }
            else if (open && section == "twist_check" && checks != null && checks.Entries.Count > 0)
                sections.AddChild(new TwistCheckList(checks, tokens) { Name = "LocalBody_" + section });
            else if (open && section == "repair_list" && repair != null && repairSteps != null && scrub != null && repair.End != JobEnd.Cancelled)
            {
                var control = new RepairListControl(repair, repairSteps, scrub, repairCoverage, wide => { repairCoverage = wide; Refresh(); }, tokens) { Name = "LocalBody_" + section };
                control.Picked += (row, asked) => RepairPicked?.Invoke(row, asked);
                sections.AddChild(control);
            }
            else if (open && section == "cap_covers" && covers != null && inspect != null)
                sections.AddChild(new CapCoversList(covers, inspect, map, coversTable, wide => { coversTable = wide; Refresh(); }, tokens) { Name = "LocalBody_" + section });
            else if (open)
            {
                Label body = ChromeWidgets.Text(emptyBody, "DimLabel"); body.Name = "LocalBody_" + section;
                body.AddThemeColorOverride("font_color", t.C("text.dim")); sections.AddChild(body);
            }
        }
    }
}

// The cap map's panel in Solve's bottom dock, which the entry slot opens (ADR-D02.12, .13): its head, the map (B8),
// the legend of the classes with their counts, and the legend of what the entry armed.
public sealed partial class CapMapPanel : VBoxContainer
{
    public CapMapControl Map { get; }
    private readonly ChromeSettings settings;
    private readonly Func<TokenSet> tokens;
    private readonly string title;
    private readonly Label head, legend, counts, status;
    private readonly HFlowContainer classes = new() { Name = "CapMapClasses" };
    public LineEdit ThresholdField { get; }
    // The adopted revision's Jumble facts: the named cap's Tier 1 says what is unavailable while Jumble is on.
    public JumbleState State { get; set; } = JumbleState.Off();
    // The researcher wrote formulas in the threshold field and pressed Enter (an empty field clears it).
    public event Action<string>? ThresholdTyped;
    private string? said;

    public CapMapPanel(ChromeSettings settings, Func<TokenSet> tokens, string title)
    {
        this.settings = settings; this.tokens = tokens; this.title = title; Name = "CapMapPanel";
        head = ChromeWidgets.Text(title, "DimLabel"); head.Name = "CapMapState"; AddChild(head);
        // The map on the left and its words beside it, so the dock stays one map high and Global keeps its opening.
        var body = new HBoxContainer { Name = "CapMapBody" }; AddChild(body);
        Map = new CapMapControl(tokens); body.AddChild(Map);
        var side = new VBoxContainer { Name = "CapMapSide", SizeFlagsHorizontal = SizeFlags.ExpandFill }; body.AddChild(side);
        var line = new HBoxContainer { Name = "CapMapThreshold" }; side.AddChild(line);
        ThresholdField = new LineEdit
        {
            Name = "CapMapThresholdField", PlaceholderText = "threshold: N_c ≥ k, N_c ≤ k, N_c = F_c",
            TooltipText = "The cap map's threshold: formulas a member meets together; Enter sets it, an empty field clears it. There is no default.",
            CustomMinimumSize = new Vector2(260, 0),
        };
        ThresholdField.TextSubmitted += text => ThresholdTyped?.Invoke(text);
        line.AddChild(ThresholdField);
        counts = ChromeWidgets.Text("", "MicroLabel"); counts.Name = "CapMapThresholdCounts"; side.AddChild(counts);
        status = ChromeWidgets.Text("", "MicroLabel"); status.Name = "CapMapNamed"; side.AddChild(status);
        Map.TypedChanged += () => { if (IsInstanceValid(this)) Refresh(); };
        classes.SizeFlagsHorizontal = SizeFlags.ExpandFill; side.AddChild(classes);
        legend = ChromeWidgets.Text("", "MicroLabel"); legend.Name = "CapMapLegend"; side.AddChild(legend);
        settings.Changed += _ => { if (IsInstanceValid(this)) Refresh(); };
        Refresh();
    }

    // The bound state's map, or every cap pending while none is bound; no layout, no sheet.
    public void Show(CapLayout? layout, CapMapModel map)
    {
        Map.Show(layout, map);
        Refresh();
    }

    // The threshold, the named cap and its meeting caps over the map; `say` is a refusal to show (a formula that does
    // not read).
    public void Mark(CapThreshold threshold, int? named, IReadOnlyList<int> meeting, string? say = null)
    {
        said = say;
        if (!ThresholdField.HasFocus()) ThresholdField.Text = threshold.Formula;
        Map.Mark(threshold, named, meeting);
        Refresh();
    }

    public void Refresh()
    {
        TokenSet t = tokens();
        head.Text = Map.Layout == null ? title + ": no cap layout in this build" : title + " · 20 rings of 30 caps";
        string Word(string key) => settings.Get(key) == EntrySlot.Armed ? "armed" : "off";
        legend.Text = "contact classes · " + Word(EntrySlot.ContactClasses) + "   never hidden · " + Word(EntrySlot.NeverHidden);
        legend.AddThemeColorOverride("font_color", t.C("text.dim"));
        ThresholdCounts? c = Map.Threshold.Counts(Map.Map);
        counts.Text = said ?? (c is ThresholdCounts n ? CapThreshold.Line(n) : "no threshold");
        counts.AddThemeColorOverride("font_color", t.C(said != null ? "state.invalid" : "text.dim"));
        status.Text = Map.Named is int cap && Map.Layout != null ? JumbleViews.Tier1(Tier1(cap) + " · " + Map.Meeting.Count + " meeting caps", State)
            : Map.Typed.Length > 0 ? "cap " + Map.Typed + "…  Enter names it" : "no cap named";
        status.AddThemeColorOverride("font_color", t.C(Map.Named != null ? "text" : "text.dim"));
        // SF6 (27 Nov 2026): the class rows stay while a layout is drawn and change their words in place; a bind rebuilt
        // them at each twist before.
        if (Map.Layout == null)
        {
            foreach (Node child in classes.GetChildren()) { classes.RemoveChild(child); child.QueueFree(); }
            return;
        }
        if (!classes.HasThemeConstantOverride("h_separation") || classes.GetThemeConstant("h_separation") != t.Px("space.3"))
            classes.AddThemeConstantOverride("h_separation", t.Px("space.3"));
        IReadOnlyDictionary<CapClass, int> byClass = Map.Map.Counts();
        foreach (CapClass k in new[] { CapClass.Blocked, CapClass.Closed, CapClass.AtFloor, CapClass.AboveFloor, CapClass.Pending })
        {
            string word = k == CapClass.AtFloor ? "at floor (N_c = F_c > 0)" : CapMapModel.Word(k);
            Entry(() => new CapSwatch(tokens, k), word + " · " + byClass[k], "CapMapClass_" + k);
        }
        Entry(() => new CapSwatch(tokens, null), "columns wrap", "CapMapWrap");
        Map.Restyle();

        string Tier1(int named)
        {
            var (kind, id, numerals) = CapFacts.Tier1(Map.Map, named);
            return kind + " " + id + string.Concat(numerals.Select(s => " · " + s.Label + " " + (s.Value.Kind == NumeralKind.Value ? s.Value.Text : s.Value.Word)));
        }

        void Entry(Func<Control> swatch, string text, string name)
        {
            if (classes.GetNodeOrNull<HBoxContainer>(name) is { } kept && kept.GetChildCount() == 2 && kept.GetChild(1) is Label words)
            {
                words.Text = text;
                TintLegendRow.Colour(words, t.C("text.dim"));
                if (kept.GetChild(0) is CanvasItem drawn) drawn.QueueRedraw();
                return;
            }
            var row = new HBoxContainer { Name = name }; row.AddThemeConstantOverride("separation", t.Px("space.1"));
            row.AddChild(swatch());
            Label label = ChromeWidgets.Text(text, "MicroLabel"); label.AddThemeColorOverride("font_color", t.C("text.dim")); row.AddChild(label);
            classes.AddChild(row);
        }
    }
}
