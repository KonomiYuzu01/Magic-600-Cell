using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The .6cs editor pane (ADR-D01.57, ADR-D01.58), built on CodeEdit. Diagnostics show at their source: a tinted
// line, a note after the end of the line that does not move the text, and a list under the editor. Ctrl+Enter
// runs the entry point at the caret; the commit key in the editor is Ctrl+Shift+Enter, never the run key.
public sealed partial class SixCsEditor : VBoxContainer
{
    public CodeEdit Code { get; } = new() { SizeFlagsVertical = SizeFlags.ExpandFill, SizeFlagsHorizontal = SizeFlags.ExpandFill, GuttersDrawLineNumbers = true, HighlightCurrentLine = true,
        IndentAutomatic = true, AutoBraceCompletionEnabled = true, DrawTabs = false, ScrollPastEndOfFile = false, Name = "Code" };
    public Label Result { get; } = ChromeWidgets.Text("", "CodeLabel", true);
    public VBoxContainer Notes { get; } = new();
    public HBoxContainer Tools { get; } = new();
    public IReadOnlyList<ScriptProgram> Programs => programs;
    public ScriptProgram? Open { get; private set; }
    public IReadOnlyList<ScriptDiagnostic> Diagnostics { get; private set; } = Array.Empty<ScriptDiagnostic>();
    // Run the entry point at the caret (null: the whole program); the commit key; the text of a program changed.
    public event Action<ScriptProgram, ScriptEntry?>? Run;
    public event Action? CommitKey;
    public event Action<ScriptProgram>? Edited;
    private readonly List<ScriptProgram> programs = new();
    private readonly Func<TokenSet> tokens;
    private readonly HBoxContainer tabs = new();
    private readonly Control notesOverlay = new() { MouseFilter = MouseFilterEnum.Ignore };
    private readonly Timer pause = new() { OneShot = true, WaitTime = .4 };
    private bool loading;

    public SixCsEditor(Func<TokenSet> tokens)
    {
        this.tokens = tokens; Name = "ScriptEditor"; AddThemeConstantOverride("separation", 0);
        AddChild(tabs); AddChild(Code); AddChild(Result); AddChild(Notes); AddChild(pause);
        Code.AddChild(notesOverlay); notesOverlay.SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect); notesOverlay.Draw += DrawNotes;
        Code.TextChanged += () => { if (!loading) Changed(); };
        // Diagnostics run when editing pauses (ADR-D01.25).
        pause.Timeout += Check;
        Code.GuiInput += input =>
        {
            if (input is not InputEventKey { Pressed: true, Echo: false, CtrlPressed: true } key || key.Keycode is not (Key.Enter or Key.KpEnter)) return;
            if (key.ShiftPressed) CommitKey?.Invoke(); else RunAtCaret();
            Code.AcceptEvent();
        };
        Refresh();
    }

    private void Changed()
    {
        if (Open == null) return;
        int at = programs.FindIndex(p => p.Id == Open.Id); Open = Open with { Source = Code.Text }; programs[at] = Open;
        if (pause.IsInsideTree()) pause.Start();
        Edited?.Invoke(Open);
    }

    // Replaces the text of the open program, as typing would.
    public void Replace(string source)
    {
        loading = true; Code.Text = source; loading = false;
        Changed(); Check();
    }

    public void SetPrograms(IEnumerable<ScriptProgram> all)
    {
        programs.Clear(); programs.AddRange(all);
        Show(Open != null && programs.Any(p => p.Id == Open.Id) ? Open.Id : programs.FirstOrDefault()?.Id);
    }

    public void Show(string? id, int line = 0)
    {
        Open = programs.FirstOrDefault(p => p.Id == id);
        loading = true; Code.Text = Open?.Source ?? ""; loading = false;
        if (line > 0 && line <= Code.GetLineCount()) { Code.SetCaretLine(line - 1); Code.CenterViewportToCaret(); }
        Result.Text = ""; Check(); Refresh();
    }

    public ScriptEntry? EntryAtCaret() => Open?.Entries.LastOrDefault(e => e.Line <= Code.GetCaretLine() + 1);
    public void RunAtCaret() { if (Open != null) Run?.Invoke(Open, EntryAtCaret()); }

    // A read result is shown inline; one computed before a later commit is dimmed and names its entry.
    public void ShowResult(string text, bool stale = false)
    {
        Result.Text = text; Result.TooltipText = text;
        Result.AddThemeColorOverride("font_color", tokens().C(stale ? "state.stale" : "text"));
    }

    public void Check()
    {
        Diagnostics = Open == null ? Array.Empty<ScriptDiagnostic>() : SixCs.Diagnose(Open.Source);
        TokenSet t = tokens();
        for (int i = 0; i < Code.GetLineCount(); i++) Code.SetLineBackgroundColor(i, new Color(0, 0, 0, 0));
        foreach (ScriptDiagnostic d in Diagnostics.Where(d => d.Severity != Severity.Info && d.Line <= Code.GetLineCount()))
            Code.SetLineBackgroundColor(d.Line - 1, new Color(t.C(FeedbackPanel.Shapes[d.Severity].Colour), .16f));
        SessionWidgets.Clear(Notes);
        foreach (ScriptDiagnostic d in Diagnostics)
        {
            var row = new HBoxContainer { MouseFilter = MouseFilterEnum.Stop }; row.SetMeta("line", d.Line);
            (string icon, string colour) = FeedbackPanel.Shapes[d.Severity];
            Label shape = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(shape, icon, t.Px("icon.small")); shape.AddThemeColorOverride("font_color", t.C(colour)); row.AddChild(shape);
            row.AddChild(ChromeWidgets.Text($"{d.Line}", "CodeLabel")); row.AddChild(ChromeWidgets.Text(d.Text, "MicroLabel", true));
            row.GuiInput += input => { if (input is InputEventMouseButton { Pressed: true }) { Code.SetCaretLine(d.Line - 1); Code.CenterViewportToCaret(); Code.GrabFocus(); } };
            Notes.AddChild(row);
        }
        notesOverlay.QueueRedraw();
    }

    private void DrawNotes()
    {
        TokenSet t = tokens(); Font font = Code.GetThemeFont("font"); int size = Math.Max(8, Code.GetThemeFontSize("font_size") - 2);
        foreach (ScriptDiagnostic d in Diagnostics.Where(d => d.Line <= Code.GetLineCount()))
        {
            Rect2I end = Code.GetRectAtLineColumn(d.Line - 1, Code.GetLine(d.Line - 1).Length);
            if (end.Position.X < 0 || end.Position.Y < 0) continue;
            notesOverlay.DrawString(font, new Vector2(end.End.X + t.Px("space.4"), end.End.Y - t.Px("space.1")), d.Text, HorizontalAlignment.Left, -1, size, t.C(FeedbackPanel.Shapes[d.Severity].Colour));
        }
    }

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        TokenSet t = tokens();
        ChromeWidgets.Detach(Tools); SessionWidgets.Clear(tabs);
        foreach (ScriptProgram program in programs)
        {
            Button tab = ChromeWidgets.Tool(program.File, program.File, true, "TabButton"); tab.SetPressedNoSignal(program.Id == Open?.Id); tab.SetMeta("program", program.Id);
            string id = program.Id; tab.Pressed += () => Show(id); tabs.AddChild(tab);
        }
        tabs.AddChild(new Control { SizeFlagsHorizontal = SizeFlags.ExpandFill });
        Button run = SessionWidgets.Tool("script.run", "Run the entry point at the caret: a read result or a preview (Ctrl+Enter). Commit is Ctrl+Shift+Enter here.", t, RunAtCaret); run.Name = "RunButton";
        run.Disabled = Open == null; tabs.AddChild(run); tabs.AddChild(Tools);
        var colours = new CodeHighlighter { NumberColor = t.C("syntax.number"), SymbolColor = t.C("syntax.operator"), FunctionColor = t.C("syntax.identifier"), MemberVariableColor = t.C("syntax.reference") };
        foreach (string word in SixCs.Keywords) colours.AddKeywordColor(word, t.C("syntax.keyword"));
        foreach (string word in SixCs.Types) colours.AddKeywordColor(word, t.C("syntax.type"));
        colours.AddColorRegion("\"", "\"", t.C("syntax.string")); colours.AddColorRegion("#", "", t.C("syntax.comment"), true);
        Code.SyntaxHighlighter = colours;
        Code.Editable = Open != null; Code.PlaceholderText = "Open a .6cs program from the Scripts dock to edit it here.";
        notesOverlay.QueueRedraw();
    }
}

// The Scripts dock: the session's programs with their state, and the entry points each one registers with the
// keys bound to them. An entry point is a command row like any other (ADR-D01.17).
public sealed partial class ScriptList : VBoxContainer
{
    public HBoxContainer Tools { get; } = new();
    public VBoxContainer List { get; } = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill };
    public Func<ScriptEntry, string> KeysOf { get; set; } = _ => "";
    public string? Binding { get; private set; }
    public event Action<ScriptProgram, int>? Opened;
    public event Action<ScriptEntry>? RunEntry;
    public event Action<ScriptEntry>? BindEntry;
    private IReadOnlyList<ScriptProgram> programs = Array.Empty<ScriptProgram>();
    private readonly Func<TokenSet> tokens;

    public ScriptList(Func<TokenSet> tokens)
    {
        this.tokens = tokens; Name = "ScriptList"; AddThemeConstantOverride("separation", 0);
        AddChild(Tools);
        var scroll = new ScrollContainer { SizeFlagsVertical = SizeFlags.ExpandFill, HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled }; AddChild(scroll); scroll.AddChild(List);
    }

    public void SetPrograms(IReadOnlyList<ScriptProgram> all) { programs = all; Refresh(); }
    // The entry whose next key press becomes its binding, or null.
    public void SetBinding(string? command) { Binding = command; Refresh(); }

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        SessionWidgets.Clear(List);
        TokenSet t = tokens();
        foreach (ScriptProgram program in programs)
        {
            IReadOnlyList<ScriptDiagnostic> found = SixCs.Diagnose(program.Source);
            bool failed = found.Any(d => d.Severity == Severity.Error);
            var head = new PanelContainer { ThemeTypeVariation = "Row", MouseFilter = MouseFilterEnum.Stop }; head.SetMeta("program", program.Id);
            var line = new HBoxContainer(); head.AddChild(line);
            Label state = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(state, failed ? "severity.error" : "task.ok", t.Px("icon.small"));
            state.AddThemeColorOverride("font_color", t.C(failed ? "severity.error" : "state.committed")); state.TooltipText = failed ? "failed its check" : "checked"; state.MouseFilter = MouseFilterEnum.Pass;
            line.AddChild(state); line.AddChild(ChromeWidgets.Text(program.File, "CodeLabel", true));
            if (found.Count > 0) line.AddChild(SessionWidgets.Chip(found.Count.ToString(), t.C(failed ? "severity.error" : "text.dim"), found.Count + " notes"));
            head.GuiInput += input => { if (input is InputEventMouseButton { Pressed: true, ButtonIndex: MouseButton.Left }) Opened?.Invoke(program, 0); };
            List.AddChild(head);
            foreach (ScriptEntry entry in program.Entries)
            {
                var row = new HBoxContainer(); row.SetMeta("entry", entry.Command); List.AddChild(row);
                row.AddChild(new Control { CustomMinimumSize = new Vector2(t.Px("icon.medium"), 0) });
                string colour = entry.Kind == ScriptKind.Preview ? "state.preview" : entry.Kind == ScriptKind.View ? "accent.strong" : "text.dim";
                row.AddChild(SessionWidgets.Chip(entry.Kind.ToString().ToLowerInvariant(), t.C(colour), "What a run of this entry point may end in"));
                Button name = ChromeWidgets.Tool(entry.Name, $"Line {entry.Line}: click to show it", false, "FieldCaption"); name.Alignment = HorizontalAlignment.Left; name.ClipText = true;
                name.SizeFlagsHorizontal = SizeFlags.ExpandFill; name.Pressed += () => Opened?.Invoke(program, entry.Line); row.AddChild(name);
                bool waiting = Binding == entry.Command; string keys = KeysOf(entry);
                Button bind = ChromeWidgets.Tool(waiting ? "press a key" : keys.Length > 0 ? keys : "bind", waiting ? "Press the key to bind; Esc leaves it" : "Bind a key to this entry point", true, "FieldValue");
                bind.SetPressedNoSignal(waiting); bind.SetMeta("bind", entry.Command); bind.Pressed += () => BindEntry?.Invoke(entry); row.AddChild(bind);
                row.AddChild(SessionWidgets.Tool("script.run", "Run this entry point", t, () => RunEntry?.Invoke(entry)));
            }
        }
        if (programs.Count == 0) List.AddChild(ChromeWidgets.Empty("Scripts", "The session's .6cs programs and panels appear here."));
    }
}

// A panel a program declared, drawn by the host inside the script frame (ADR-D01.30). A panel whose program is
// missing keeps its place and says so.
public sealed partial class ScriptPanelHost : PanelContainer
{
    public ScriptPanelSpec? Spec { get; }
    public string PanelId { get; }
    public bool Pinned { get; private set; }
    public event Action<string>? Unmount;

    public ScriptPanelHost(string id, ScriptPanelSpec? spec, Func<TokenSet> tokens)
    {
        PanelId = id; Spec = spec; Name = "ScriptPanel_" + id.Replace('.', '_'); ThemeTypeVariation = "ScriptFrame";
        SizeFlagsVertical = SizeFlags.ExpandFill; this.tokens = tokens;
        Refresh();
    }

    private readonly Func<TokenSet> tokens;

    // Drawn again from the tokens in force.
    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        SessionWidgets.Clear(this);
        TokenSet t = tokens(); ScriptPanelSpec? spec = Spec; string id = PanelId;
        var column = new VBoxContainer(); AddChild(column);
        var head = new HBoxContainer(); column.AddChild(head);
        head.AddChild(ChromeWidgets.Text(spec?.Title ?? id, "MicroLabel", true));
        head.AddChild(SessionWidgets.Chip("script", t.C("script.frame"), spec != null ? "Drawn by the host from " + spec.Program : "Its program is missing"));
        Button pin = ChromeWidgets.Tool("", "Pin: keep this panel on its target when the selection changes", true); ChromeIcons.Apply(pin, "palette.pin", t.Px("icon.small"));
        pin.SetPressedNoSignal(Pinned); pin.Toggled += on => Pinned = on; head.AddChild(pin);
        head.AddChild(SessionWidgets.Tool("pane.close", "Unmount this panel", t, () => Unmount?.Invoke(id)));
        column.AddChild(new HSeparator());
        if (spec == null) { column.AddChild(ChromeWidgets.Empty(id, "The program of this panel is missing. Its slot is kept.")); return; }
        column.AddChild(ChromeWidgets.Text("target: the selection", "MicroLabel"));
        var grid = new GridContainer { Columns = 2 }; column.AddChild(grid);
        foreach ((string label, string value) in spec.Rows)
        {
            grid.AddChild(ChromeWidgets.Text(label, "DimLabel", true)); Label v = ChromeWidgets.Text(value, "CodeLabel"); v.HorizontalAlignment = HorizontalAlignment.Right; grid.AddChild(v);
        }
        if (spec.Bars.Count == 0) return;
        double top = Math.Max(1e-9, spec.Bars.Max(b => b.Value));
        foreach ((string label, double value) in spec.Bars)
        {
            var row = new HBoxContainer(); column.AddChild(row);
            Label name = ChromeWidgets.Text(label, "MicroLabel", true); name.SizeFlagsStretchRatio = .8f; row.AddChild(name);
            row.AddChild(new ProgressBar { MinValue = 0, MaxValue = top, Value = value, ShowPercentage = false, SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsVertical = SizeFlags.ShrinkCenter,
                CustomMinimumSize = new Vector2(0, t.Px("slider.height")), TooltipText = $"{label}: {value}" });
            row.AddChild(ChromeWidgets.Text(value.ToString("0"), "MicroLabel"));
        }
    }
}

// The canvas primitive (ADR-D01.31): a script's vector marks in design tokens, inside the script frame. It is
// display only, and it is drawn again from the tokens in force whenever the theme changes.
public sealed partial class ScriptCanvas : PanelContainer
{
    public IReadOnlyList<CanvasMark> Marks { get; private set; } = Array.Empty<CanvasMark>();
    public string Source { get; private set; } = "";
    private readonly Func<TokenSet> tokens;
    private readonly Control surface = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsVertical = SizeFlags.ExpandFill, ClipContents = true };
    private readonly Label caption = ChromeWidgets.Text("", "MicroLabel", true);

    public ScriptCanvas(Func<TokenSet> tokens)
    {
        this.tokens = tokens; Name = "ScriptCanvas"; ThemeTypeVariation = "ScriptFrame";
        var column = new VBoxContainer(); AddChild(column); column.AddChild(caption); column.AddChild(surface);
        surface.Draw += Paint;
        Show(Array.Empty<CanvasMark>(), "");
    }

    public void Show(IReadOnlyList<CanvasMark> marks, string source)
    {
        foreach (CanvasMark mark in marks) if (!CanvasMark.Shapes.Contains(mark.Shape)) throw J.Error(mark.Shape, "unknown canvas shape");
        Marks = marks; Source = source;
        caption.Text = marks.Count == 0 ? "script canvas: nothing is drawn. A script draws here, display only." : $"script canvas: {source}, {marks.Count} marks, display only";
        surface.QueueRedraw();
    }

    public void Repaint() { if (IsInstanceValid(this)) surface.QueueRedraw(); }

    private void Paint()
    {
        TokenSet t = tokens(); Vector2 size = surface.Size; float unit = Math.Min(size.X, size.Y);
        Vector2 At(double x, double y) => new((float)(size.X / 2 + (x - .5) * unit), (float)(size.Y / 2 + (y - .5) * unit));
        Font font = GetThemeFont("font", "Label"); int text = Math.Max(8, GetThemeFontSize("font_size", "Label") - 2);
        foreach (CanvasMark m in Marks)
        {
            Color colour = t.C(m.Colour);
            if (m.Shape == "line") surface.DrawLine(At(m.X, m.Y), At(m.A, m.B), colour, Math.Max(1, t.Px("border.width")), true);
            else if (m.Shape == "circle") surface.DrawArc(At(m.X, m.Y), (float)(m.A * unit), 0, Mathf.Tau, 64, colour, Math.Max(1, t.Px("border.width")), true);
            else if (m.Shape == "dot") surface.DrawCircle(At(m.X, m.Y), (float)(m.A * unit), colour);
            else surface.DrawString(font, At(m.X, m.Y), m.Text ?? "", HorizontalAlignment.Left, -1, text, colour);
        }
    }
}
