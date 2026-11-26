using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// ADR-D02 build step B28 (ADR-D02.44 to .46): the research pane `pane.research`, a centre peer of the pair in Analyse.
// Ported from the Library dock (LibraryPanel): a head over a list the model fills, where every control sends its row
// (Command) and the host runs it as the palette does, so the pane is reachable from the keyboard and from scripts.
// The list builds only the lines in view (VirtualRows, ADR-D01.33) and scrolls by whole lines, since a verdict line
// wraps and is never cut. Every verdict takes the one body weight and no colour role (ADR-D02.46).
public sealed partial class ResearchPane : VBoxContainer
{
    public const string PaneId = "research", Title = "Research";
    // Without a research source the pane says what it waits for (six words at most, ADR-D01.6).
    public const string Waiting = "Research rows wait for K2";
    public ResearchModel? Model { get; private set; }
    public event Action<string, string?>? Command;
    public HBoxContainer Tabs { get; } = new() { Name = "ResearchTabs" };
    public Label Head { get; } = ChromeWidgets.Text("", "MicroLabel");
    public Label Carried { get; } = ChromeWidgets.Text("", "MicroLabel");
    public VBoxContainer List { get; } = new() { Name = "ResearchRows" };
    // The list's window: a plain control, so the lines' wrapped heights never reach the layout (a container that held the
    // list grew with each rebuild of fresh, zero-width labels, and the rebuild on its resize never settled: the shell hung
    // when Analyse first showed the pane). The list hangs from its top and the window clips it.
    private readonly Control view = new() { Name = "ResearchView", ClipContents = true, SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsVertical = SizeFlags.ExpandFill };
    // The first line in view, and the lines the list holds now.
    public int First { get; private set; }
    public int Built => List.GetChildCount();
    public IReadOnlyList<ResearchLine> Shown => lines;
    private readonly VScrollBar bar = new() { Name = "ResearchScroll", Step = 1 };
    private readonly Func<TokenSet> tokens;
    private IReadOnlyList<ResearchLine> lines = Array.Empty<ResearchLine>();
    private string? opened;
    private bool laying;

    public ResearchPane(Func<TokenSet> tokens)
    {
        this.tokens = tokens; Name = "ResearchPane";
        SizeFlagsHorizontal = SizeFlags.ExpandFill; SizeFlagsVertical = SizeFlags.ExpandFill;
        AddChild(ChromeWidgets.Empty("workspace.analyse", Waiting));
    }

    // Shows a research model: the tabs, the head and the list. The model's rows are run by the host, never here.
    public void Show(ResearchModel model)
    {
        Model = model;
        SessionWidgets.Clear(this);
        Head.Name = "ResearchHead"; Head.AutowrapMode = TextServer.AutowrapMode.WordSmart; Head.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        Carried.Name = "ResearchCarried"; Carried.AutowrapMode = TextServer.AutowrapMode.WordSmart; Carried.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        AddChild(Tabs); AddChild(Head); AddChild(Carried);
        var body = new HBoxContainer { Name = "ResearchBody", SizeFlagsVertical = SizeFlags.ExpandFill, SizeFlagsHorizontal = SizeFlags.ExpandFill };
        List.AddThemeConstantOverride("separation", 0);
        view.AddChild(List); List.SetAnchorsAndOffsetsPreset(LayoutPreset.TopWide);
        body.AddChild(view); body.AddChild(bar); AddChild(body);
        bar.ValueChanged += value => { if (!laying) { First = (int)value; Lay(); } };
        view.Resized += Lay;
        view.GuiInput += Wheel;
        List.GuiInput += Wheel;
        model.Changed += Refresh;
        Refresh();
    }

    private void Send(string command, string? argument) => Safe.Raise(Command, command, argument);

    private void Wheel(InputEvent e)
    {
        if (e is not InputEventMouseButton { Pressed: true } mouse) return;
        if (mouse.ButtonIndex == MouseButton.WheelDown) ScrollTo(First + 3);
        else if (mouse.ButtonIndex == MouseButton.WheelUp) ScrollTo(First - 3);
        else return;
        List.AcceptEvent();
    }

    public void ScrollTo(int line) { First = Math.Clamp(line, 0, Math.Max(0, lines.Count - 1)); Lay(); }

    public void Refresh()
    {
        if (!IsInstanceValid(this) || Model is not { } model) return;
        TokenSet t = tokens();
        SessionWidgets.Clear(Tabs);
        foreach (ResearchControl control in ResearchModel.TabControls())
        {
            Button tab = ChromeWidgets.Tool(control.Label, $"{control.Label}: the row {control.Row}", true);
            tab.Name = "ResearchTab" + control.Label; tab.ButtonPressed = model.Tab.ToString() == control.Label;
            tab.SetMeta("research_row", control.Row);
            tab.Pressed += () => Send(control.Row, control.Argument);
            Tabs.AddChild(tab);
        }
        Head.Text = $"{model.CountsLine()} · evidence: {model.Evidence} · {model.AnswerLine()}";
        Carried.Text = model.TransferLine(); Carried.Visible = Carried.Text.Length > 0;
        lines = model.Lines();
        // A row opened by its row (a click, the palette or a script) comes into view with what it opened.
        if (model.Open != opened)
        {
            opened = model.Open;
            int at = lines.Select((line, i) => (line, i)).Where(x => x.line.Kind == ResearchLineKind.Row && x.line.Ref == model.Open).Select(x => x.i).DefaultIfEmpty(-1).First();
            if (at >= 0 && (at < First || at > First + Math.Max(1, Window(t) - 4))) First = at;
        }
        First = Math.Clamp(First, 0, Math.Max(0, lines.Count - 1));
        Lay();
    }

    private int Window(TokenSet t) => VirtualRows.Range(lines.Count, t.Px("row.height"), First * (double)t.Px("row.height"), Math.Max(view.Size.Y, t.Px("row.height") * 8), 0).Count;

    // Builds the lines from First that the list's height can show; a wrapped line may push the last ones out of view.
    private void Lay()
    {
        if (!IsInstanceValid(this) || Model is not { } model) return;
        TokenSet t = tokens();
        int count = Window(t);
        laying = true;
        bar.MaxValue = lines.Count; bar.Page = Math.Max(1, count - 1); bar.Value = First;
        laying = false;
        SessionWidgets.Clear(List);
        for (int i = First; i < Math.Min(lines.Count, First + count); i++) List.AddChild(Line(t, model, lines[i]));
        if (lines.Count == 0) List.AddChild(ChromeWidgets.Text("Nothing to show.", "DimLabel"));
    }

    private static Label Wrapped(string text, string variation = "")
    {
        Label label = ChromeWidgets.Text(text, variation);
        label.AutowrapMode = TextServer.AutowrapMode.WordSmart; label.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        return label;
    }

    private static MarginContainer Indent(TokenSet t, int depth, Control content)
    {
        var margin = new MarginContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill };
        margin.AddThemeConstantOverride("margin_left", depth * t.Px("space.4"));
        margin.AddChild(content);
        return margin;
    }

    private Control Line(TokenSet t, ResearchModel model, ResearchLine line)
    {
        IReadOnlyList<ResearchControl> controls = model.Controls(line);
        switch (line.Kind)
        {
            case ResearchLineKind.Stage: return Wrapped(line.Text, "DimLabel");
            case ResearchLineKind.Family: return Indent(t, 1, Wrapped(line.Text, "DimLabel"));
            case ResearchLineKind.Row:
            {
                bool open = model.Open == line.Ref;
                var holder = new PanelContainer { ThemeTypeVariation = open ? "RowSelected" : "Row", SizeFlagsHorizontal = SizeFlags.ExpandFill };
                holder.SetMeta("research_ref", line.Ref!);
                var column = new VBoxContainer(); holder.AddChild(column);
                ResearchControl control = controls[0];
                Button name = ChromeWidgets.Tool(control.Label, $"{control.Label.TrimStart('▸', '▾', ' ')}: {(open ? "fold it" : "open it in place")} (the row {control.Row})");
                name.Name = "ResearchRowName"; name.Alignment = HorizontalAlignment.Left; name.ClipText = true; name.SizeFlagsHorizontal = SizeFlags.ExpandFill;
                name.SetMeta("research_row", control.Row);
                name.Pressed += () => Send(control.Row, control.Argument);
                column.AddChild(name);
                // The verdict and its scope: one line, wrapping, never cut; one weight, no colour role.
                Label verdict = Wrapped(line.Verdict ?? "");
                verdict.Name = "ResearchVerdict";
                column.AddChild(Indent(t, 1, verdict));
                return holder;
            }
            case ResearchLineKind.Part:
            {
                ResearchControl control = controls[0];
                Button part = ChromeWidgets.Tool(control.Label, $"{control.Label.TrimStart('▸', '▾', '·', ' ')} (the row {control.Row})");
                part.Name = "ResearchPart"; part.Alignment = HorizontalAlignment.Left; part.ClipText = true; part.SizeFlagsHorizontal = SizeFlags.ExpandFill;
                part.SetMeta("research_row", control.Row);
                part.Pressed += () => Send(control.Row, control.Argument);
                return Indent(t, line.Depth + 1, part);
            }
            case ResearchLineKind.Actions:
            {
                var row = new HBoxContainer { Name = "ResearchActions" };
                foreach (ResearchControl control in controls)
                {
                    if (control.TakesText)
                    {
                        var box = new LineEdit { Name = "ResearchNameBox", PlaceholderText = "Your name for it", Text = model.Names.TryGetValue(line.Ref!, out string? named) ? named : "",
                            CustomMinimumSize = new Vector2(t.Px("space.4") * 8, 0), TooltipText = "Your own name for this record or part: it changes no digest and no row (ADR-D02.3)" };
                        Button set = ChromeWidgets.Tool(control.Label, $"Name it (the row {control.Row}, the record {ResearchRows.NamesRecord})");
                        set.SetMeta("research_row", control.Row);
                        set.Pressed += () => Send(control.Row, control.Argument + box.Text);
                        box.TextSubmitted += text => Send(control.Row, control.Argument + text);
                        row.AddChild(box); row.AddChild(set);
                        continue;
                    }
                    Button button = ChromeWidgets.Tool(control.Label, $"{control.Label} (the row {control.Row}): origin Research; {ResearchWords.CopyNote}");
                    button.SetMeta("research_row", control.Row);
                    button.Pressed += () => Send(control.Row, control.Argument);
                    row.AddChild(button);
                }
                return Indent(t, line.Depth, row);
            }
            case ResearchLineKind.Note: return Wrapped(line.Text, "DimLabel");
            default: return Indent(t, line.Depth, Wrapped(line.Text));
        }
    }
}
