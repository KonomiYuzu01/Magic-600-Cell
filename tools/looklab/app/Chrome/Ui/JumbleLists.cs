using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The kept twist checks under the named cap (ADR-D02.35, .36; build step B10): the columns
// `try | twist | N_c | floor | ΔN | Δn_h | Δu`, a completed row per check in release order with its session release on
// hover, and each run of superseded releases folded in its place as "not checked:" with the twists' glyphs only.
public sealed partial class TwistCheckList : VBoxContainer
{
    private static readonly int[] Widths = { 34, 64, 52, 46, 56, 132, 72 };

    public TwistCheckList(KeptChecks checks, Func<TokenSet> tokens)
    {
        Name = "TwistCheckList";
        TokenSet t = tokens();
        AddThemeConstantOverride("separation", 0);
        AddChild(Line(KeptChecks.Columns, "MicroLabel", t.C("text.dim"), "TwistCheckColumns"));
        int i = 0;
        foreach (KeptEntry entry in checks.Entries)
        {
            Control row = entry switch
            {
                CheckRow c => Line(KeptChecks.Cells(c), "ReadoutLabel", t.C("text"), "TwistCheck" + i),
                FoldedRow f => Folded(f, t),
                _ => new Control(),
            };
            row.TooltipText = entry is CheckRow ? "session release " + entry.Release : "not checked: superseded before its check completed";
            if (entry is CheckRow && row is HBoxContainer check) check.AddChild(ScriptList.StateGlyph(t, DiagnosticState.Ok));
            AddChild(row);
            i++;
        }
        if (checks.Pending is long release)
            AddChild(ErrorOverlays.WithState(ChromeWidgets.Text($"Release {release}", "MicroLabel"), t, DiagnosticState.Pending));
    }

    private static HBoxContainer Line(IReadOnlyList<string> cells, string variation, Color colour, string name)
    {
        var line = new HBoxContainer { Name = name, MouseFilter = MouseFilterEnum.Pass };
        for (int k = 0; k < cells.Count; k++)
        {
            Label cell = ChromeWidgets.Text(cells[k], variation);
            cell.AddThemeColorOverride("font_color", colour);
            cell.CustomMinimumSize = new Vector2(Widths[Math.Min(k, Widths.Length - 1)], 0);
            cell.HorizontalAlignment = k is 0 or 2 or 3 or 4 or 6 ? HorizontalAlignment.Right : HorizontalAlignment.Left;
            cell.MouseFilter = MouseFilterEnum.Pass;
            line.AddChild(cell);
        }
        return line;
    }

    private static Label Folded(FoldedRow f, TokenSet t)
    {
        Label label = ChromeWidgets.Text(KeptChecks.Line(f), "MicroLabel");
        label.Name = "TwistCheckFolded" + f.Release;
        label.AddThemeColorOverride("font_color", t.C("text.dim"));
        label.MouseFilter = MouseFilterEnum.Pass;
        return label;
    }
}

// Cap covers under the named-cap cell (ADR-D02.38; build step B11): the domain line, then "covered by", "covers" and
// "undefined" with their ids in cap-id order, and the inspect line. Hovering an id inspects that cap there; the cap map
// gets no mark. The expanded form is the table of the 80 meeting caps, both directions.
public sealed partial class CapCoversList : VBoxContainer
{
    private static readonly int[] Widths = { 46, 92, 92 };
    private readonly Label inspected;
    private readonly CoversInspect inspect;
    private readonly Func<CapMapModel> map;

    public CapCoversList(CapCovers covers, CoversInspect inspect, Func<CapMapModel> map, bool expanded, Action<bool> expand, Func<TokenSet> tokens)
    {
        Name = "CapCoversList"; this.inspect = inspect; this.map = map;
        TokenSet t = tokens();
        AddThemeConstantOverride("separation", t.Px("space.1"));
        Label domain = ChromeWidgets.Text(covers.DomainLine(), "MicroLabel"); domain.Name = "CoversDomain";
        domain.AddThemeColorOverride("font_color", t.C("text.dim")); domain.AutowrapMode = TextServer.AutowrapMode.WordSmart;
        AddChild(domain);
        ErrorOverlays.MarkLine(domain, t, DiagnosticState.Ok);
        AddChild(Ids("CoversCoveredBy", covers.CoveredByLine(), t));
        AddChild(Ids("CoversCovers", covers.CoversLine(), t));
        AddChild(Ids("CoversUndefined", covers.UndefinedLine(), t));
        inspected = ChromeWidgets.Text("", "ReadoutLabel"); inspected.Name = "CoversInspect";
        AddChild(inspected);
        Button table = ChromeWidgets.Tool((expanded ? "▾ " : "▸ ") + "The " + covers.Rows.Count + " meeting caps, both directions", expanded ? "Fold the table" : "Show the table of every meeting cap's answers", true);
        table.Name = "CoversTableToggle"; table.Alignment = HorizontalAlignment.Left; table.SetPressedNoSignal(expanded);
        table.Pressed += () => expand(!expanded);
        AddChild(table);
        if (expanded)
        {
            var grid = new GridContainer { Name = "CoversTable", Columns = 3 };
            foreach (string column in covers.Columns()) grid.AddChild(Cell(column, "MicroLabel", t.C("text.dim"), grid.GetChildCount()));
            foreach (var row in covers.Table())
                for (int k = 0; k < row.Count; k++)
                {
                    Label cell = Cell(row[k], "ReadoutLabel", t.C(row[k] == "undefined" ? "text.dim" : "text"), k);
                    if (k == 0) Hoverable(cell, int.Parse(row[0], System.Globalization.CultureInfo.InvariantCulture));
                    grid.AddChild(cell);
                }
            AddChild(grid);
        }
        // Subscribed while in the tree: a frame rebuild takes the view out and puts it back.
        TreeEntered += () => { inspect.Changed += Refill; Refill(); };
        TreeExiting += () => inspect.Changed -= Refill;
    }

    private void Refill()
    {
        if (!IsInstanceValid(this)) return;
        inspected.Text = inspect.Line(map());
    }

    private Control Ids(string name, (string Head, IReadOnlyList<int> Caps) line, TokenSet t)
    {
        var flow = new HFlowContainer { Name = name };
        flow.AddThemeConstantOverride("h_separation", t.Px("space.1"));
        Label head = ChromeWidgets.Text(line.Head + (line.Caps.Count > 0 ? ":" : ""), "MicroLabel");
        head.AddThemeColorOverride("font_color", t.C("text.dim"));
        flow.AddChild(head);
        foreach (int cap in line.Caps)
        {
            Label id = ChromeWidgets.Text(cap.ToString(System.Globalization.CultureInfo.InvariantCulture), "ReadoutLabel");
            id.Name = name + "_" + cap;
            Hoverable(id, cap);
            flow.AddChild(id);
        }
        return flow;
    }

    // Hovering an id inspects it; leaving it inspects nothing. Nothing else listens.
    private void Hoverable(Label id, int cap)
    {
        id.MouseFilter = MouseFilterEnum.Pass;
        id.TooltipText = "Hover to inspect cap " + cap + "; the cap map gets no mark";
        id.MouseEntered += () => inspect.Hover(cap);
        id.MouseExited += () => { if (inspect.Inspected == cap) inspect.Hover(null); };
    }

    private static Label Cell(string text, string variation, Color colour, int column)
    {
        Label cell = ChromeWidgets.Text(text, variation);
        cell.AddThemeColorOverride("font_color", colour);
        cell.CustomMinimumSize = new Vector2(Widths[Math.Min(column % 3, Widths.Length - 1)], 0);
        cell.HorizontalAlignment = column % 3 == 0 ? HorizontalAlignment.Right : HorizontalAlignment.Left;
        return cell;
    }
}

// The repair list under the named-cap cell (ADR-D02.32, .34; build step B12): the columns `# | x d | y z | ΔN` in the
// result's order, no row focused on open, arrow keys to focus, digits 1 to 9 to pick in this list's focused context,
// Enter for the focused row, and the preview's scrub keys for the focused row's steps. The solver picks; the list never
// picks, orders or stages a row itself.
public sealed partial class RepairListControl : VBoxContainer, IOwnKeys
{
    private static readonly int[] Widths = { 34, 72, 72, 64 };
    private readonly RepairListModel list;
    private readonly RepairSteps steps;
    private readonly RepairScrub scrub;
    private readonly Func<TokenSet> tokens;
    private readonly VBoxContainer rows = new() { Name = "RepairRows" };
    private readonly ScrollContainer scroll = new() { Name = "RepairScroll", HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };
    private readonly Label step = new() { Name = "RepairStep" };
    // The solver picked a row: by a digit or Enter, in the list's context, with the key that did it.
    public event Action<RepairRow, UserEvent>? Picked;

    public RepairListControl(RepairListModel list, RepairSteps steps, RepairScrub scrub, bool expanded, Action<bool> expand, Func<TokenSet> tokens)
    {
        Name = "RepairList"; this.list = list; this.steps = steps; this.scrub = scrub; this.tokens = tokens;
        FocusMode = FocusModeEnum.All; MouseFilter = MouseFilterEnum.Stop;
        TooltipText = "Click, then arrow keys focus a row; 1 to 9 or Enter picks it; the preview's step keys walk its steps";
        TokenSet t = tokens();
        AddThemeConstantOverride("separation", t.Px("space.1"));
        if (list.Error is string error)
        {
            AddChild(Line(error, "state.invalid", "RepairError"));
            ErrorOverlays.MarkLine(GetNode<Label>("RepairError"), t, list.CheckState);
            return;
        }
        if (list.CutOffLine() is string cut) AddChild(Line(cut, "text.dim", "RepairCutOff"));
        if (GetNodeOrNull<Label>("RepairCutOff") is { } cutOff) ErrorOverlays.MarkLine(cutOff, t, list.CheckState);
        if (list.EmptyLine() is string empty)
        {
            AddChild(Line(empty, "text", "RepairEmpty"));
            ErrorOverlays.MarkLine(GetNode<Label>("RepairEmpty"), t, list.CheckState);
            AddChild(Line(list.CoverageLine() ?? "", "text.dim", "RepairCoverage"));
        }
        if (list.Rows.Count > 0)
        {
            AddChild(Row(RepairListModel.Columns, "MicroLabel", t.C("text.dim"), "RepairColumns"));
            GetNode<HBoxContainer>("RepairColumns").AddChild(ScriptList.StateGlyph(t, list.CheckState));
            foreach (RepairRow row in list.Rows) rows.AddChild(Row(RepairListModel.Cells(row), "ReadoutLabel", t.C("text"), "RepairRow" + row.Number));
            scroll.AddChild(rows);
            scroll.CustomMinimumSize = new Vector2(0, (float)(Math.Min(list.Rows.Count, 10) * (t.TypeSize("ui", "micro") + 6)));
            AddChild(scroll);
            step.AddThemeColorOverride("font_color", t.C("text.dim"));
            AddChild(step);
            Button more = ChromeWidgets.Tool((expanded ? "▾ " : "▸ ") + "Coverage", expanded ? "Fold the coverage line" : "Show what the result ran under", true);
            more.Name = "RepairCoverageToggle"; more.Alignment = HorizontalAlignment.Left; more.SetPressedNoSignal(expanded);
            more.Pressed += () => expand(!expanded);
            AddChild(more);
            if (expanded) AddChild(Line(list.CoverageLine() ?? "", "text.dim", "RepairCoverage"));
        }
        TreeEntered += () => { list.Changed += Refill; scrub.Changed += Refill; Refill(); };
        TreeExiting += () => { list.Changed -= Refill; scrub.Changed -= Refill; };
    }

    private Label Line(string text, string role, string name)
    {
        Label line = ChromeWidgets.Text(text, "MicroLabel"); line.Name = name;
        line.AddThemeColorOverride("font_color", tokens().C(role)); line.AutowrapMode = TextServer.AutowrapMode.WordSmart;
        return line;
    }

    private static HBoxContainer Row(IReadOnlyList<string> cells, string variation, Color colour, string name)
    {
        var line = new HBoxContainer { Name = name, MouseFilter = MouseFilterEnum.Pass };
        for (int k = 0; k < cells.Count; k++)
        {
            Label cell = ChromeWidgets.Text(cells[k], variation);
            cell.AddThemeColorOverride("font_color", colour);
            cell.CustomMinimumSize = new Vector2(Widths[Math.Min(k, Widths.Length - 1)], 0);
            cell.HorizontalAlignment = k is 0 or 3 ? HorizontalAlignment.Right : HorizontalAlignment.Left;
            cell.MouseFilter = MouseFilterEnum.Pass;
            line.AddChild(cell);
        }
        return line;
    }

    // The focused row's outline follows the state roles; its steps' label shows under the rows.
    private void Refill()
    {
        if (!IsInstanceValid(this)) return;
        TokenSet t = tokens();
        foreach (Node child in rows.GetChildren())
            if (child is HBoxContainer line)
            {
                bool focused = line.Name == "RepairRow" + list.Focused;
                foreach (Node cell in line.GetChildren())
                    if (cell is Label label) label.AddThemeColorOverride("font_color", t.C(focused ? "state.selected" : "text"));
                if (focused) scroll.EnsureControlVisible(line);
            }
        scrub.Show(list.Focused is int f ? list.Rows[f - 1] : null);
        step.Text = scrub.Row is RepairRow row
            ? scrub.Label + " · moving " + (steps.Moving(row, scrub.Step) is { Kind: NumeralKind.Value } n ? n.Value.ToString("N0", System.Globalization.CultureInfo.InvariantCulture) : "pending")
            : "";
    }

    public override void _GuiInput(InputEvent e)
    {
        if (e is InputEventMouseButton { ButtonIndex: MouseButton.Left, Pressed: true }) { GrabFocus(); return; }
        if (e is not InputEventKey { Pressed: true } key) return;
        // C2-2: a pick is one plain press: a held key's repeats and a chord with a modifier pick nothing.
        bool plain = !key.Echo && !key.CtrlPressed && !key.AltPressed && !key.MetaPressed && !key.ShiftPressed;
        string[] contexts = HasFocus() ? new[] { RepairListModel.Context } : Array.Empty<string>();
        int digit = key.Keycode is >= Key.Key0 and <= Key.Key9 ? (int)(key.Keycode - Key.Key0) : key.Keycode is >= Key.Kp0 and <= Key.Kp9 ? (int)(key.Keycode - Key.Kp0) : -1;
        switch (key.Keycode)
        {
            case Key.Down: list.Down(UserEvent.Key("Down")); break;
            case Key.Up: list.Up(UserEvent.Key("Up")); break;
            // The preview's step keys (view.preview.scrub: Left back, Right forward).
            case Key.Left: scrub.Scrub("back"); break;
            case Key.Right: scrub.Scrub("forward"); break;
            case Key.Escape: list.Unfocus(); break;
            case Key.Enter or Key.KpEnter when plain && list.Enter(contexts) is RepairRow focused: Picked?.Invoke(focused, UserEvent.Key("Enter")); break;
            case var _ when plain && digit >= 0 && list.Digit(digit, contexts) is RepairRow row: Picked?.Invoke(row, UserEvent.Key(digit.ToString(System.Globalization.CultureInfo.InvariantCulture))); break;
            default: return;
        }
        AcceptEvent();
    }
}

// The staged word under Local's sections (build step B13): the word with each sealed span in brackets, the bar's
// "Whole repair only" while a span is staged, and a remove control that takes the last span away whole. The product
// shell has no draft line until K2, so the sample preview shows here.
public sealed partial class RepairStageBar : HBoxContainer
{
    private readonly RepairStage stage;
    private readonly Func<TokenSet> tokens;
    private readonly Label word = new() { Name = "StageWord" };
    private readonly Label bar = new() { Name = "StageBar" };
    private readonly Button remove;

    public RepairStageBar(RepairStage stage, Func<TokenSet> tokens)
    {
        Name = "RepairStageBar"; this.stage = stage; this.tokens = tokens;
        TokenSet t = tokens();
        AddThemeConstantOverride("separation", t.Px("space.2"));
        word.ClipText = true; word.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        word.TooltipText = "The staged word (sample preview): a copied repair row is a sealed span, removed whole and never edited";
        remove = ChromeWidgets.Tool("Remove span", "Remove the last sealed span, whole");
        remove.Name = "StageRemove";
        remove.Pressed += () => { if (stage.Word.Spans.LastOrDefault() is { } span) stage.Remove(span.Start); };
        AddChild(word); AddChild(bar); AddChild(remove);
        TreeEntered += () => { stage.Changed += Refill; Refill(); };
        TreeExiting += () => stage.Changed -= Refill;
        Refill();
    }

    private void Refill()
    {
        if (!IsInstanceValid(this)) return;
        TokenSet t = tokens();
        Visible = stage.Word.Letters.Count > 0;
        word.Text = "Staged: " + stage.Word.Text();
        bar.Text = stage.Bar;
        bar.AddThemeColorOverride("font_color", t.C("state.pending"));
        remove.Disabled = stage.Word.Spans.Count == 0;
    }
}
