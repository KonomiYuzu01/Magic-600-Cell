using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The text area of the editor: a CodeEdit whose hover is asked of the editor.
public sealed partial class SixCsCode : CodeEdit
{
    public Func<Vector2, string>? TipAt { get; set; }
    public override string _GetTooltip(Vector2 atPosition) => TipAt?.Invoke(atPosition) ?? "";
}

// The .6cs editor pane (ADR-D01.25, ADR-D01.57, ADR-D01.58), built on CodeEdit. Syntax colours come from the
// tokens. The gutter carries a mark for each diagnostic (shape and colour by severity), a bar over each evaluated
// region and over a running range. Diagnostics run when typing pauses and show at their source: an underline, a
// note after the end of the line, a tick beside the scrollbar and a hover. A quick fix is read-only ghost text
// until Tab takes it. Ctrl+Enter runs the selection or the region at the caret; the commit key here is
// Ctrl+Shift+Enter, never the run key. The caret is the focus source: after a short drawn wait the host follows it.
public sealed partial class SixCsEditor : VBoxContainer
{
    public SixCsCode Code { get; } = new() { SizeFlagsVertical = SizeFlags.ExpandFill, SizeFlagsHorizontal = SizeFlags.ExpandFill, GuttersDrawLineNumbers = true, HighlightCurrentLine = true,
        IndentAutomatic = true, AutoBraceCompletionEnabled = true, DrawTabs = false, ScrollPastEndOfFile = false, GuttersDrawFoldGutter = true, LineFolding = true, CodeCompletionEnabled = true, Name = "Code" };
    public VBoxContainer Notes { get; } = new() { Name = "Diagnostics", SizeFlagsHorizontal = SizeFlags.ExpandFill };
    public HBoxContainer Tools { get; } = new();
    public HBoxContainer Candidates { get; } = new() { Name = "Candidates", Visible = false };
    public HBoxContainer Strip { get; } = new() { Name = "RunStrip" };
    public IReadOnlyList<ScriptProgram> Programs => programs;
    public ScriptProgram? Open { get; private set; }
    public IReadOnlyList<ScriptDiagnostic> Diagnostics { get; private set; } = Array.Empty<ScriptDiagnostic>();
    public IReadOnlyList<QuickFix> Fixes { get; private set; } = Array.Empty<QuickFix>();
    public RunQueue Queue { get; }
    public ResultStore Results { get; }
    // The fix shown as ghost text, or null. It changes nothing until it is taken.
    public QuickFix? Ghost { get; private set; }
    public bool Paused { get; private set; }
    public string? Pinned { get; private set; }
    public bool Wrapped { get; private set; }
    public string Lower { get; private set; } = "diagnostics";
    public bool Faulted => Open != null && clean.ContainsKey(Open.Id);
    public bool IsChanged(ScriptProgram program) => originals.TryGetValue(program.Id, out string? first) && first != program.Source;
    // Run an entry point (null: none at the caret); the commit key; the text of a program changed; the caret
    // rests in an entry point; abort the queue; run a stale result again.
    public event Action<ScriptProgram, ScriptEntry?>? Run;
    public event Action? CommitKey;
    public event Action<ScriptProgram>? Edited;
    public event Action<ScriptProgram, ScriptEntry>? Focused;
    public event Action? Abort;
    public event Action<ScriptResult>? Rerun;
    private readonly List<ScriptProgram> programs = new();
    private readonly Dictionary<string, string> originals = new(StringComparer.Ordinal), clean = new(StringComparer.Ordinal);
    private readonly Dictionary<int, (int First, int Last)> regions = new();
    private readonly List<(string Id, string Icon, string Tip, Control Content)> lowers = new();
    private readonly Func<TokenSet> tokens;
    private readonly HBoxContainer tabs = new() { Name = "EditorTabs" }, lowerTabs = new() { Name = "LowerTabs" };
    private readonly VBoxContainer lowerBody = new() { SizeFlagsVertical = SizeFlags.ExpandFill };
    private readonly VBoxContainer lower = new() { Name = "EditorLower" };
    private readonly Control notesOverlay = new() { MouseFilter = MouseFilterEnum.Ignore };
    private readonly Timer pause = new() { OneShot = true }, follow = new() { OneShot = true };
    private readonly int marks;
    private bool loading;
    private int fixAt, dismissed = -1;
    private string symbol = "";
    private double flashLeft;
    public (int First, int Last) Flashed { get; private set; }

    public SixCsEditor(Func<TokenSet> tokens, RunQueue? queue = null, ResultStore? results = null)
    {
        this.tokens = tokens; Queue = queue ?? new RunQueue(); Results = results ?? new ResultStore();
        Name = "ScriptEditor"; AddThemeConstantOverride("separation", 0);
        var split = new VSplitContainer { SizeFlagsVertical = SizeFlags.ExpandFill, Name = "EditorSplit" };
        var top = new VBoxContainer { SizeFlagsVertical = SizeFlags.ExpandFill }; top.AddThemeConstantOverride("separation", 0);
        AddChild(tabs); AddChild(Candidates); AddChild(split); split.AddChild(top); split.AddChild(lower);
        top.AddChild(Code); top.AddChild(Strip); AddChild(pause); AddChild(follow);
        lower.AddChild(lowerTabs); lower.AddChild(lowerBody);
        var notes = new ScrollContainer { SizeFlagsVertical = SizeFlags.ExpandFill, HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled, Name = "DiagnosticsScroll" }; notes.AddChild(Notes);
        AddLower("diagnostics", "panel.problems", "Diagnostics of this program", notes);
        Code.AddChild(notesOverlay); notesOverlay.SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect); notesOverlay.Draw += DrawNotes;
        Code.AddGutter(); marks = Code.GetGutterCount() - 1;
        Code.SetGutterType(marks, TextEdit.GutterType.Custom); Code.SetGutterDraw(marks, true); Code.SetGutterClickable(marks, true);
        Code.SetGutterCustomDraw(marks, Callable.From<int, int, Rect2>(DrawGutter));
        Code.GutterClicked += (line, gutter) => { if (gutter == marks && ResultAt((int)line + 1) is { } r) Results.Toggle(r.Command); };
        Code.TipAt = Tip;
        Code.TextChanged += () => { if (!loading) { Changed(); Suggest(); } };
        Code.CaretChanged += CaretMoved;
        Code.CodeCompletionRequested += Complete;
        Code.GetVScrollBar().ValueChanged += _ => notesOverlay.QueueRedraw();
        // Diagnostics run when editing pauses (ADR-D01.25); the host follows the caret after its own wait.
        pause.Timeout += Check;
        follow.Timeout += () => { notesOverlay.QueueRedraw(); if (!Paused && Pinned == null && Open != null && EntryAtCaret() is { } entry && InRegion(entry, Code.GetCaretLine() + 1)) Safe.Raise(Focused, Open, entry); };
        Code.GuiInput += Keys;
        Queue.Changed += Redraw; Results.Changed += Redraw;
        Queue.Finished += item => { if (item.Program == Open?.Id) Flash(item.First, item.Last); };
        Refresh();
    }

    public override void _ExitTree() { Queue.Changed -= Redraw; Results.Changed -= Redraw; }
    public override void _EnterTree() { Queue.Changed -= Redraw; Results.Changed -= Redraw; Queue.Changed += Redraw; Results.Changed += Redraw; }

    private void Keys(InputEvent input)
    {
        if (input is not InputEventKey { Pressed: true } key) return;
        bool enter = key.Keycode is Key.Enter or Key.KpEnter;
        if (enter && key.CtrlPressed && !key.Echo) { if (key.ShiftPressed) Safe.Raise(CommitKey); else RunAtCaret(); Code.AcceptEvent(); }
        else if (enter && key.AltPressed && !key.Echo) { NextFix(); Code.AcceptEvent(); }
        else if (key.Keycode == Key.Tab && !key.CtrlPressed && Code.GetCodeCompletionSelectedIndex() < 0 && (TakeCandidate(0) || TakeGhost())) Code.AcceptEvent();
        else if (key.Keycode == Key.Escape && Drop()) Code.AcceptEvent();
    }

    // Another tab of the panel under the text: the console sits beside the diagnostics.
    public void AddLower(string id, string icon, string tip, Control content)
    {
        content.SizeFlagsVertical = SizeFlags.ExpandFill; lowers.Add((id, icon, tip, content)); lowerBody.AddChild(content);
        Refresh();
    }

    public void ShowLower(string id) { if (lowers.Any(l => l.Id == id)) { Lower = id; Refresh(); } }

    private void Changed()
    {
        if (Open == null) return;
        int at = programs.FindIndex(p => p.Id == Open.Id); Open = Open with { Source = Code.Text }; programs[at] = Open;
        regions.Clear(); Results.Follow(Open.Id, Open.Entries);
        if (pause.IsInsideTree()) pause.Start(Math.Max(.05, tokens().Duration("debounce.diagnostics") / 1000));
        Safe.Raise(Edited, Open);
        MarkTabs();
    }

    // Replaces the text of the open program, as typing would.
    public void Replace(string source)
    {
        loading = true; Code.Text = source; loading = false;
        Changed(); Check();
    }

    public void SetPrograms(IEnumerable<ScriptProgram> all)
    {
        programs.Clear(); programs.AddRange(all); originals.Clear(); clean.Clear();
        foreach (ScriptProgram p in programs) originals[p.Id] = p.Source;
        Show(Open != null && programs.Any(p => p.Id == Open.Id) ? Open.Id : programs.FirstOrDefault()?.Id);
    }

    public void Show(string? id, int line = 0)
    {
        Open = programs.FirstOrDefault(p => p.Id == id);
        loading = true; Code.Text = Open?.Source ?? ""; loading = false; regions.Clear(); Ghost = null; dismissed = -1;
        if (line > 0 && line <= Code.GetLineCount()) { Code.SetCaretLine(line - 1); Code.CenterViewportToCaret(); }
        Check(); Refresh();
    }

    // Shows the line of an entry point and flashes its region: a picked mark reveals the line that drew it.
    public void Reveal(ScriptEntry entry)
    {
        if (Open?.Id != entry.Program) Show(entry.Program, entry.Line);
        else { Code.SetCaretLine(entry.Line - 1); Code.CenterViewportToCaret(); }
        (int first, int last) = Region(entry.Line); Flash(first, last);
    }

    private (int First, int Last) Region(int line)
    {
        if (!regions.TryGetValue(line, out var region)) regions[line] = region = SixCsService.Region(Open?.Source ?? "", line);
        return region;
    }

    private bool InRegion(ScriptEntry entry, int line) { (int a, int b) = Region(entry.Line); return line >= a && line <= b; }
    private ScriptResult? ResultAt(int line) => Results.Results.FirstOrDefault(r => r.Program == Open?.Id && line >= Region(r.Line).First && line <= Region(r.Line).Last);

    public ScriptEntry? EntryAtCaret() => Open?.Entries.LastOrDefault(e => e.Line <= Code.GetCaretLine() + 1);

    // Runs the entry points inside the selection, or the one whose region holds the caret.
    public void RunAtCaret()
    {
        if (Open == null) return;
        if (Code.HasSelection())
        {
            int a = Code.GetSelectionFromLine() + 1, b = Code.GetSelectionToLine() + 1;
            ScriptEntry[] inside = Open.Entries.Where(e => e.Line >= a && e.Line <= b).ToArray();
            if (inside.Length > 0) { foreach (ScriptEntry entry in inside) Safe.Raise(Run, Open, entry); return; }
        }
        Safe.Raise(Run, Open, EntryAtCaret());
    }

    public void Check()
    {
        Diagnostics = Open == null ? Array.Empty<ScriptDiagnostic>() : SixCsService.Diagnose(Open.Source);
        Fixes = Open == null ? Array.Empty<QuickFix>() : SixCsService.Fixes(Open.Source);
        TokenSet t = tokens();
        for (int i = 0; i < Code.GetLineCount(); i++) Code.SetLineBackgroundColor(i, default);
        foreach (ScriptDiagnostic d in Diagnostics.Where(d => d.Severity != Severity.Info && d.Line <= Code.GetLineCount()))
            Code.SetLineBackgroundColor(d.Line - 1, t.C(FeedbackPanel.Shapes[d.Severity].Colour, .14f));
        SessionWidgets.Clear(Notes);
        foreach (ScriptDiagnostic d in Diagnostics)
        {
            var row = new HBoxContainer { MouseFilter = MouseFilterEnum.Stop }; row.SetMeta("line", d.Line);
            string colour = FeedbackPanel.Shapes[d.Severity].Colour;
            row.AddChild(new MarkGlyph(MarkShapes.Of(d.Severity), () => tokens().C(colour), t.Px("icon.small")));
            row.AddChild(ChromeWidgets.Text($"{d.Line}", "CodeLabel"));
            DiagnosticHelp help = SixCsService.Explain(d);
            Label text = ChromeWidgets.Text(d.Text, "MicroLabel", true); text.TooltipText = $"{d.Text}\n{help.Why}\n{help.Do}"; row.AddChild(text);
            if (Fixes.Any(f => f.Line == d.Line && f.Kind is FixKind.Syntax or FixKind.Contract)) row.AddChild(ChromeWidgets.KeyCap("Tab"));
            row.GuiInput += input => { if (input is InputEventMouseButton { Pressed: true }) { Code.SetCaretLine(d.Line - 1); Code.CenterViewportToCaret(); Code.GrabFocus(); } };
            Notes.AddChild(row);
        }
        if (Diagnostics.Count == 0) Notes.AddChild(ChromeWidgets.Empty("task.ok", "No diagnostics"));
        UpdateGhost(); MarkTabs(); Redraw();
    }

    // The fix a diagnostic of the caret's line offers shows by itself; the rewrites of a line show when asked.
    private void UpdateGhost()
    {
        int line = Code.GetCaretLine() + 1;
        if (Ghost != null && Ghost.Line == line && Fixes.Contains(Ghost)) return;
        Ghost = line == dismissed ? null : Fixes.FirstOrDefault(f => f.Line == line && f.Kind is FixKind.Syntax or FixKind.Contract);
    }

    // Alt+Enter: the next fix of the caret's line as ghost text.
    public QuickFix? NextFix()
    {
        QuickFix[] here = Fixes.Where(f => f.Line == Code.GetCaretLine() + 1).ToArray(); dismissed = -1;
        if (here.Length == 0) { Ghost = null; Redraw(); return null; }
        fixAt = Ghost != null && here.Contains(Ghost) ? (Array.IndexOf(here, Ghost) + 1) % here.Length : 0;
        Ghost = here[fixAt]; Redraw();
        return Ghost;
    }

    // Tab: the ghost text becomes the line.
    public bool TakeGhost()
    {
        if (Ghost == null || Open == null || Ghost.Line > Code.GetLineCount()) return false;
        QuickFix fix = Ghost; Ghost = null;
        loading = true; Code.SetLine(fix.Line - 1, fix.NewLine); loading = false;
        Changed(); Check();
        return true;
    }

    // Esc: drops the candidates, then the ghost text, then the runs that wait.
    public bool Drop()
    {
        if (Candidates.Visible) { Candidates.Visible = false; return true; }
        if (Ghost != null) { dismissed = Ghost.Line; Ghost = null; Redraw(); return true; }
        if (Queue.Active.Any()) { Safe.Raise(Abort); return true; }
        return false;
    }

    private void CaretMoved()
    {
        UpdateGhost(); Suggest();
        if (follow.IsInsideTree() && !Paused && Pinned == null) follow.Start(Math.Max(.05, tokens().Duration("debounce.caret") / 1000));
        Redraw();
    }

    // Backslash input: the name typed after a backslash offers its symbols; Tab takes the first.
    private void Suggest()
    {
        string line = Code.GetLine(Code.GetCaretLine()); int column = Math.Min(Code.GetCaretColumn(), line.Length);
        string? prefix = SixCsService.SymbolPrefix(line.Substring(0, column));
        SessionWidgets.Clear(Candidates); symbol = prefix ?? "";
        var found = prefix == null ? Array.Empty<(string Name, string Symbol)>() : SixCsService.SymbolCandidates(prefix);
        Candidates.Visible = found.Count > 0;
        if (found.Count == 0) return;
        Candidates.AddChild(ChromeWidgets.KeyCap("Tab"));
        for (int i = 0; i < found.Count; i++)
        {
            int index = i; Button pick = ChromeWidgets.Tool($"{found[i].Symbol}  {found[i].Name}", "Insert " + found[i].Symbol);
            pick.Pressed += () => TakeCandidate(index); Candidates.AddChild(pick);
        }
    }

    public bool TakeCandidate(int index)
    {
        if (!Candidates.Visible) return false;
        var found = SixCsService.SymbolCandidates(symbol);
        if (index >= found.Count) return false;
        int line = Code.GetCaretLine(), column = Code.GetCaretColumn();
        Code.Select(line, Math.Max(0, column - symbol.Length - 1), line, column); Code.InsertTextAtCaret(found[index].Symbol);
        Candidates.Visible = false;
        return true;
    }

    private void Complete()
    {
        string line = Code.GetLine(Code.GetCaretLine()); int column = Math.Min(Code.GetCaretColumn(), line.Length), start = column;
        while (start > 0 && (char.IsLetterOrDigit(line[start - 1]) || line[start - 1] == '_')) start--;
        if (column - start < 2) return;
        foreach (Completion c in SixCsService.Completions(line.Substring(start, column - start)))
            Code.AddCodeCompletionOption(c.Kind == "fn" ? CodeEdit.CodeCompletionKind.Function : c.Kind == "type" ? CodeEdit.CodeCompletionKind.Class : CodeEdit.CodeCompletionKind.PlainText, c.Signature, c.Word);
        Code.UpdateCodeCompletionOptions(true);
    }

    // The hover: a diagnostic says what, why and what to do; a word of the library shows its signature.
    private string Tip(Vector2 at)
    {
        Vector2I place = Code.GetLineColumnAtPos((Vector2I)at, false);
        if (place.Y < 0) return "";
        if (Diagnostics.FirstOrDefault(d => d.Line == place.Y + 1) is { } d) { DiagnosticHelp help = SixCsService.Explain(d); return $"{d.Text}\n{help.Why}\n{help.Do}"; }
        return SixCsService.Hover(Code.GetWordAtPos(at)) is { } word ? word.Signature + "\n" + word.Help : "";
    }

    public void InsertReference(string text) { if (Open != null) { Code.InsertTextAtCaret(text); Code.GrabFocus(); } }

    // One fault of each severity in the open program, and back.
    public void ToggleFaults()
    {
        if (Open == null) return;
        if (clean.Remove(Open.Id, out string? source)) Replace(source);
        else { string kept = Open.Source; Replace(SixCsService.Faults(kept)); clean[Open.Id] = kept; }
        Refresh();
    }

    public int SelectUnused()
    {
        if (Open == null) return 0;
        var unused = SixCsService.UnusedLets(Open.Source); Code.RemoveSecondaryCarets();
        for (int i = 0; i < unused.Count; i++)
        {
            int line = unused[i].Line - 1, end = Code.GetLine(line).Length;
            if (i == 0) Code.Select(line, 0, line, end);
            else { int caret = Code.AddCaret(line, end); if (caret >= 0) Code.Select(line, 0, line, end, caret); }
        }
        return unused.Count;
    }

    public void SetWrap(bool on) { Wrapped = on; Code.WrapMode = on ? TextEdit.LineWrappingMode.Boundary : TextEdit.LineWrappingMode.None; Refresh(); }
    public void SetPaused(bool on) { Paused = on; RefreshStrip(); }
    public void Pin(bool on) { Pinned = on ? EntryAtCaret()?.Command : null; RefreshStrip(); }
    public void GoPinned() { if (Pinned != null && programs.SelectMany(p => p.Entries).FirstOrDefault(e => e.Command == Pinned) is { } entry) Reveal(entry); }

    private void Flash(int first, int last)
    {
        TokenSet t = tokens(); Flashed = (first, last); flashLeft = t.Duration("flash") / 1000;
        for (int i = first - 1; i < Math.Min(last, Code.GetLineCount()); i++) Code.SetLineBackgroundColor(i, t.C("state.preview", .22f));
    }

    public override void _Process(double delta)
    {
        if (flashLeft > 0 && (flashLeft -= delta) <= 0) Check();
        if (!follow.IsStopped() || Queue.Active.Any()) Redraw();
    }

    private void Redraw() { if (!IsInstanceValid(this) || !IsInstanceValid(Code)) return; Code.QueueRedraw(); notesOverlay.QueueRedraw(); if (stripKey != StripKey()) RefreshStrip(); }

    private void DrawGutter(int line, int gutter, Rect2 area)
    {
        if (Open == null) return;
        TokenSet t = tokens(); int n = line + 1; float bar = Math.Max(2, t.Px("focus.width")), r = Math.Min(area.Size.X, area.Size.Y) * .3f;
        foreach (ScriptResult result in Results.Results.Where(x => x.Program == Open.Id))
        {
            (int a, int b) = Region(result.Line);
            if (n < a || n > b) continue;
            (string colour, string shape) = OutputMarks.For(result.Mark); Color c = t.C(result.Since != null ? "state.stale" : colour);
            Code.DrawRect(new Rect2(area.Position, new Vector2(bar, area.Size.Y)), c);
            if (n == a) MarkShapes.Draw(Code, shape, area.Position + new Vector2(area.Size.X * .34f, area.Size.Y / 2), r, c);
        }
        foreach (RunItem item in Queue.Active.Where(x => x.Program == Open.Id && n >= x.First && n <= x.Last))
        {
            bool lit = item.State == RunState.Running && n - item.First < item.Progress * (item.Last - item.First + 1);
            Code.DrawRect(new Rect2(area.Position + new Vector2(bar, 0), new Vector2(bar, area.Size.Y)), t.C("state.pending", lit ? 1 : .4f));
        }
        if (Diagnostics.FirstOrDefault(x => x.Line == n) is { } d)
            MarkShapes.Draw(Code, MarkShapes.Of(d.Severity), area.Position + new Vector2(area.Size.X * .72f, area.Size.Y / 2), r, t.C(FeedbackPanel.Shapes[d.Severity].Colour));
    }

    private Rect2I EndOf(int index) => Code.GetRectAtLineColumn(index, Code.GetLine(index).Length);

    private void DrawNotes()
    {
        TokenSet t = tokens(); Font font = Code.GetThemeFont("font"); int size = Math.Max(8, Code.GetThemeFontSize("font_size") - 2);
        float gap = t.Px("space.4"), lift = t.Px("space.1"), thin = Math.Max(1, t.Px("border.width")); int count = Code.GetLineCount();
        var taken = new HashSet<int>();
        foreach (ScriptDiagnostic d in Diagnostics.Where(d => d.Line <= count))
        {
            Color colour = t.C(FeedbackPanel.Shapes[d.Severity].Colour);
            // A tick beside the scrollbar for every diagnostic, wherever the text is scrolled to.
            float y = (d.Line - .5f) / count * notesOverlay.Size.Y;
            notesOverlay.DrawRect(new Rect2(notesOverlay.Size.X - t.Px("scrollbar.width") - gap / 2, y - thin, gap / 2, thin * 2), colour);
            Rect2I end = EndOf(d.Line - 1);
            if (end.Position.X < 0 || end.Position.Y < 0 || !taken.Add(d.Line)) continue;
            notesOverlay.DrawString(font, new Vector2(end.End.X + gap, end.End.Y - lift), d.Text, HorizontalAlignment.Left, -1, size, colour);
        }
        // The first error line is underlined.
        if (Diagnostics.FirstOrDefault(d => d.Severity == Severity.Error && d.Line <= count) is { } first)
        {
            string text = Code.GetLine(first.Line - 1); int from = text.Length - text.TrimStart().Length;
            Rect2I a = Code.GetRectAtLineColumn(first.Line - 1, Math.Min(from + 1, text.Length)), b = EndOf(first.Line - 1);
            if (a.Position.Y >= 0 && b.Position.Y >= 0 && text.Trim().Length > 0)
                notesOverlay.DrawLine(new Vector2(a.Position.X, b.End.Y), new Vector2(b.End.X, b.End.Y), t.C("severity.error"), thin * 2);
        }
        if (Ghost != null && Ghost.Line <= count)
        {
            Rect2I end = EndOf(Ghost.Line - 1);
            if (end.Position.Y >= 0)
            {
                float x = end.End.X + gap * (taken.Contains(Ghost.Line) ? 12 : 1);
                notesOverlay.DrawString(font, new Vector2(x, end.End.Y - lift), $"Tab ⇥ {Ghost.Title}:  {Ghost.NewLine.Trim()}", HorizontalAlignment.Left, -1, size, t.C("text.off"));
            }
        }
        // A read result sits under its region; the one of the caret's region is the bright one.
        int caret = Code.GetCaretLine() + 1;
        foreach (ScriptResult result in Results.Results.Where(x => x.Program == Open?.Id))
        {
            (int a, int b) = Region(result.Line);
            int index = b < count && Code.GetLine(b).Trim().Length == 0 ? b : b - 1;
            if (index < 0 || index >= count) continue;
            Rect2I end = EndOf(index);
            if (end.Position.Y < 0) continue;
            (string colour, string shape) = OutputMarks.For(result.Mark); bool stale = result.Since != null, here = caret >= a && caret <= b;
            Vector2 at = new(Math.Max(end.End.X, 0) + gap, end.End.Y - lift);
            MarkShapes.Draw(notesOverlay, shape, at + new Vector2(size * .4f, -size * .35f), size * .4f, t.C(stale ? "state.stale" : colour));
            string text = (result.Collapsed ? "…" : result.Text) + (stale ? "   · before " + result.Since : "");
            notesOverlay.DrawString(font, at + new Vector2(size * 1.2f, 0), "⇒ " + text, HorizontalAlignment.Left, -1, size, t.C(stale ? "state.stale" : here ? "text" : "text.dim"));
        }
        // The wait before the host follows the caret, drawn.
        if (!follow.IsStopped() && follow.WaitTime > 0)
            notesOverlay.DrawRect(new Rect2(0, 0, notesOverlay.Size.X * (float)(1 - follow.TimeLeft / follow.WaitTime), thin * 2), t.C("state.pending"));
    }

    private void MarkTabs()
    {
        foreach (Button tab in tabs.GetChildren().OfType<Button>().Where(b => b.HasMeta("program")))
            if (programs.FirstOrDefault(p => p.Id == tab.GetMeta("program").AsString()) is { } program) tab.Text = program.File + (IsChanged(program) ? " ●" : "");
    }

    private string stripKey = "";
    private string StripKey() => string.Join(",", Queue.Items.TakeLast(4).Select(i => i.Id + ":" + i.State)) + "|" + Results.Results.Count(r => r.Since != null) + "|" + Paused + "|" + Pinned;

    private void RefreshStrip()
    {
        if (!IsInstanceValid(this)) return;
        TokenSet t = tokens(); SessionWidgets.Clear(Strip); stripKey = StripKey();
        foreach (RunItem item in Queue.Items.TakeLast(4))
        {
            (string icon, string colour) = item.State switch
            {
                RunState.Running => ("task.running", "state.pending"), RunState.Queued => ("task.running", "text.dim"), RunState.Done => ("task.ok", "severity.ok"),
                RunState.Failed => ("severity.error", "severity.error"), _ => ("task.cancelled", "text.dim"),
            };
            var chip = new PanelContainer { ThemeTypeVariation = "Chip", TooltipText = $"{item.Program}.{item.Entry}: {item.State.ToString().ToLowerInvariant()}" }; chip.SetMeta("run", item.Id);
            var row = new HBoxContainer(); chip.AddChild(row);
            Label mark = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(mark, icon, t.Px("icon.small")); mark.AddThemeColorOverride("font_color", t.C(colour)); row.AddChild(mark);
            row.AddChild(ChromeWidgets.Text(item.Entry, "MicroLabel")); Strip.AddChild(chip);
        }
        Button abort = SessionWidgets.Tool("script.abort", "Abort every run that waits or runs (Esc)", t, () => Safe.Raise(Abort)); abort.Name = "AbortButton"; abort.Disabled = !Queue.Active.Any(); Strip.AddChild(abort);
        Strip.AddChild(new Control { SizeFlagsHorizontal = SizeFlags.ExpandFill });
        ScriptResult[] stale = Results.Results.Where(r => r.Since != null).ToArray();
        Button rerun = SessionWidgets.Tool("script.rerun", stale.Length == 0 ? "Run stale results again" : "Run again: " + string.Join(", ", stale.Select(r => r.Entry)), t, () => { foreach (ScriptResult r in stale) Safe.Raise(Rerun, r); });
        rerun.Name = "RerunButton"; rerun.Disabled = stale.Length == 0; Strip.AddChild(rerun);
        Button pauseFollow = SessionWidgets.Tool("follow.pause", "Pause: the preview stops following the caret", t, () => SetPaused(!Paused)); pauseFollow.Name = "PauseFollow"; pauseFollow.ToggleMode = true; pauseFollow.SetPressedNoSignal(Paused);
        Button pin = SessionWidgets.Tool("palette.pin", "Pin: the preview stays on this entry point", t, () => Pin(Pinned == null)); pin.Name = "PinFollow"; pin.ToggleMode = true; pin.SetPressedNoSignal(Pinned != null);
        Button go = SessionWidgets.Tool("follow.goto", "Go to the pinned line", t, GoPinned); go.Name = "GoPinned"; go.Disabled = Pinned == null;
        Strip.AddChild(pauseFollow); Strip.AddChild(pin); Strip.AddChild(go);
    }

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        TokenSet t = tokens();
        ChromeWidgets.Detach(Tools); SessionWidgets.Clear(tabs);
        foreach (ScriptProgram program in programs)
        {
            Button tab = ChromeWidgets.Tool(program.File + (IsChanged(program) ? " ●" : ""), program.File, true, "TabButton"); tab.SetPressedNoSignal(program.Id == Open?.Id); tab.SetMeta("program", program.Id);
            string id = program.Id; tab.Pressed += () => Show(id); tabs.AddChild(tab);
        }
        tabs.AddChild(new Control { SizeFlagsHorizontal = SizeFlags.ExpandFill });
        Button wrap = SessionWidgets.Tool("editor.wrap", "Wrap long lines", t, () => SetWrap(!Wrapped)); wrap.Name = "WrapButton"; wrap.ToggleMode = true; wrap.SetPressedNoSignal(Wrapped);
        Button unused = SessionWidgets.Tool("editor.unused", "Select the definitions nothing uses", t, () => SelectUnused()); unused.Name = "UnusedButton";
        Button faults = SessionWidgets.Tool("editor.faults", "Sample faults: one diagnostic of each severity (F6)", t, ToggleFaults); faults.Name = "FaultsButton"; faults.ToggleMode = true; faults.SetPressedNoSignal(Faulted);
        Button run = SessionWidgets.Tool("script.run", "Run the selection or the region at the caret (Ctrl+Enter). Commit is Ctrl+Shift+Enter.", t, RunAtCaret); run.Name = "RunButton";
        foreach (Button b in new[] { wrap, unused, faults, run }) { b.Disabled = Open == null; tabs.AddChild(b); }
        tabs.AddChild(Tools);
        var colours = new CodeHighlighter { NumberColor = t.C("syntax.number"), SymbolColor = t.C("syntax.operator"), FunctionColor = t.C("syntax.identifier"), MemberVariableColor = t.C("syntax.reference") };
        foreach (string word in SixCs.Keywords) colours.AddKeywordColor(word, t.C("syntax.keyword"));
        foreach (string word in SixCs.Types) colours.AddKeywordColor(word, t.C("syntax.type"));
        // The words of an effect list and the notation symbols have the reference colour.
        foreach (string word in new[] { "snapshot", "preview", "view", "workspace" }.Concat(SixCsService.Symbols.Values)) colours.AddKeywordColor(word, t.C("syntax.reference"));
        colours.AddColorRegion("\"", "\"", t.C("syntax.string")); colours.AddColorRegion("#", "", t.C("syntax.comment"), true);
        Code.SyntaxHighlighter = colours;
        Code.Editable = Open != null; Code.PlaceholderText = "";
        Code.SetGutterWidth(marks, t.Px("icon.medium") + t.Px("icon.small"));
        lower.CustomMinimumSize = new Vector2(0, t.Px("row.height") * 5);
        SessionWidgets.Clear(lowerTabs);
        foreach ((string id, string icon, string tip, Control content) in lowers)
        {
            content.Visible = id == Lower;
            Button tab = SessionWidgets.Tool(icon, tip, t, () => ShowLower(id)); tab.ThemeTypeVariation = "TabButton"; tab.ToggleMode = true; tab.SetPressedNoSignal(id == Lower); tab.Name = "Lower_" + id;
            lowerTabs.AddChild(tab);
            if (id == "diagnostics" && Diagnostics.Count > 0) lowerTabs.AddChild(ChromeWidgets.Text(Diagnostics.Count.ToString(), "MicroLabel"));
        }
        RefreshStrip(); Redraw();
    }
}

// The Scripts dock (ADR-D01.59): the session's programs with their state, the entry points each one registers
// with their kind, keys and last run, the mounts of each program with a switch for each, and the packages by
// others. An entry point is a command row like any other (ADR-D01.17).
public sealed partial class ScriptList : VBoxContainer
{
    public static readonly IReadOnlyDictionary<MountKind, (string Icon, string Tip)> MountMarks = new Dictionary<MountKind, (string, string)>
    {
        [MountKind.Palette] = ("search", "Palette entries"), [MountKind.Context] = ("mount.context", "Context action"), [MountKind.Inspector] = ("panel.inspector", "Inspector section"),
        [MountKind.Panel] = ("panel.slot1", "Docked panel"), [MountKind.View] = ("mount.view", "Free view"), [MountKind.Overlay] = ("mount.overlay", "View annotation"),
    };
    public static readonly IReadOnlyDictionary<ScriptKind, (string Icon, string Colour, string Tip)> KindMarks = new Dictionary<ScriptKind, (string, string, string)>
    {
        [ScriptKind.Read] = ("kind.read", "text.dim", "A read: ends in a result"), [ScriptKind.Preview] = ("preview.moves", "state.preview", "Ends in a preview"), [ScriptKind.View] = ("kind.view", "accent.strong", "Changes a view or the layout"),
    };
    public HBoxContainer Tools { get; } = new();
    public VBoxContainer List { get; } = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill };
    public Func<ScriptEntry, string> KeysOf { get; set; } = _ => "";
    public Func<ScriptEntry, string?> ConflictOf { get; set; } = _ => null;
    // "", "pending", "ok", "failed" or "stale", and the output mark of the entry (-1: none).
    public Func<ScriptEntry, string> StateOf { get; set; } = _ => "";
    public Func<ScriptEntry, int> MarkOf { get; set; } = _ => -1;
    public MountSet? Mounts { get; set; }
    public IReadOnlyList<(PackageManifest Package, bool Installed)> Packages { get; set; } = Array.Empty<(PackageManifest, bool)>();
    public string? Binding { get; private set; }
    public event Action<ScriptProgram, int>? Opened;
    public event Action<ScriptEntry>? RunEntry;
    public event Action<ScriptEntry>? BindEntry;
    public event Action<PackageManifest>? InstallAsked;
    public event Action<PackageManifest>? Revoked;
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

    private static Label Glyph(string icon, Color colour, string tip, TokenSet t)
    {
        Label mark = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(mark, icon, t.Px("icon.small")); mark.AddThemeColorOverride("font_color", colour);
        mark.TooltipText = tip; mark.MouseFilter = MouseFilterEnum.Pass;
        return mark;
    }

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        SessionWidgets.Clear(List);
        TokenSet t = tokens();
        foreach (ScriptProgram program in programs)
        {
            IReadOnlyList<ScriptDiagnostic> found = SixCsService.Diagnose(program.Source);
            bool failed = found.Any(d => d.Severity == Severity.Error);
            var head = new PanelContainer { ThemeTypeVariation = "Row", MouseFilter = MouseFilterEnum.Stop }; head.SetMeta("program", program.Id);
            var line = new HBoxContainer(); head.AddChild(line);
            line.AddChild(Glyph(failed ? "severity.error" : "task.ok", t.C(failed ? "severity.error" : "state.committed"), failed ? "Failed its check" : "Checked", t));
            line.AddChild(ChromeWidgets.Text(program.File, "CodeLabel", true));
            if (found.Count > 0) line.AddChild(SessionWidgets.Chip(found.Count.ToString(), t.C(failed ? "severity.error" : "text.dim"), found.Count + " diagnostics"));
            // The places this program mounts, each with its switch.
            // More than two mounts get a row of their own under the name.
            Container switches = line; HBoxContainer? under = null;
            if (Mounts != null && SixCsService.Mounts(program.Source).Count > 2) { under = new HBoxContainer(); under.SetMeta("mounts", program.Id); under.AddChild(new Control { CustomMinimumSize = new Vector2(t.Px("icon.medium"), 0) }); switches = under; }
            if (Mounts != null)
                foreach (MountKind kind in SixCsService.Mounts(program.Source))
                {
                    (string icon, string tip) = MountMarks[kind]; MountSet mounts = Mounts; string id = program.Id;
                    Button mount = SessionWidgets.Tool(icon, tip + ": on or off", t, () => mounts.Set(id, kind, !mounts.IsOn(id, kind)));
                    mount.ToggleMode = true; mount.SetPressedNoSignal(mounts.IsOn(id, kind)); mount.SetMeta("mount", (int)kind); switches.AddChild(mount);
                }
            head.GuiInput += input => { if (input is InputEventMouseButton { Pressed: true, ButtonIndex: MouseButton.Left }) Safe.Raise(Opened, program, 0); };
            List.AddChild(head); if (under != null) List.AddChild(under);
            foreach (ScriptEntry entry in program.Entries)
            {
                var row = new HBoxContainer(); row.SetMeta("entry", entry.Command); List.AddChild(row);
                row.AddChild(new Control { CustomMinimumSize = new Vector2(t.Px("icon.small"), 0) });
                (string kindIcon, string kindColour, string kindTip) = KindMarks[entry.Kind];
                row.AddChild(Glyph(kindIcon, t.C(kindColour), kindTip, t));
                Button name = ChromeWidgets.Tool(entry.Name, $"Line {entry.Line}", false, "FieldCaption"); name.Alignment = HorizontalAlignment.Left; name.ClipText = true;
                name.SizeFlagsHorizontal = SizeFlags.ExpandFill; name.Pressed += () => Safe.Raise(Opened, program, entry.Line); row.AddChild(name);
                int output = MarkOf(entry);
                if (output >= 0) { (string colour, string shape) = OutputMarks.For(output); row.AddChild(new MarkGlyph(shape, () => tokens().C(colour), t.Px("icon.small"))); }
                string state = StateOf(entry);
                if (state == "pending") row.AddChild(Glyph("task.running", t.C("state.pending"), "Running", t));
                else if (state == "ok") row.AddChild(Glyph("task.ok", t.C("severity.ok"), "Ran", t));
                else if (state == "failed") row.AddChild(Glyph("severity.error", t.C("severity.error"), "Failed", t));
                else if (state == "stale") row.AddChild(Glyph("script.rerun", t.C("state.stale"), "Stale: computed before the last commit", t));
                if (ConflictOf(entry) is { } clash) row.AddChild(Glyph("severity.warning", t.C("severity.warning"), clash, t));
                bool waiting = Binding == entry.Command; string keys = KeysOf(entry);
                Button bind = ChromeWidgets.Tool(waiting ? "…" : keys, waiting ? "Press the key to bind; Esc leaves it" : "Bind a key", true, "FieldValue");
                if (!waiting && keys.Length == 0) ChromeIcons.Apply(bind, "bind.key", t.Px("icon.small"));
                bind.SetPressedNoSignal(waiting); bind.SetMeta("bind", entry.Command); bind.Pressed += () => Safe.Raise(BindEntry, entry); row.AddChild(bind);
                row.AddChild(SessionWidgets.Tool("script.run", "Run", t, () => Safe.Raise(RunEntry, entry)));
            }
        }
        if (programs.Count == 0) List.AddChild(ChromeWidgets.Empty("panel.scripts", "No programs yet"));
        // Packages by others: a local file, installed only through its consent sheet.
        foreach ((PackageManifest package, bool installed) in Packages)
        {
            var row = new PanelContainer { ThemeTypeVariation = "Row" }; row.SetMeta("package", package.Id); List.AddChild(row);
            var line = new HBoxContainer(); row.AddChild(line);
            line.AddChild(Glyph("package", t.C(installed ? "script.frame" : "text.dim"), installed ? "Installed package" : "Package, not installed", t));
            line.AddChild(ChromeWidgets.Text(package.Title, "", true));
            foreach (string capability in package.Capabilities) line.AddChild(Glyph("cap." + capability, t.C("text.dim"), PackageManifest.Known[capability], t));
            Button act = installed ? SessionWidgets.Tool("package.revoke", "Revoke this package", t, () => Safe.Raise(Revoked, package))
                : SessionWidgets.Tool("package.install", "Install: shows what it asks for first", t, () => Safe.Raise(InstallAsked, package));
            act.SetMeta("package", package.Id); line.AddChild(act);
        }
    }
}

// The host's frame of a script-drawn area, chosen from the run's recorded author class: the mark alone, a thin
// border, or a patterned border drawn over the box. The script has no say in it.
public static class ScriptFrames
{
    public static void Apply(PanelContainer frame, AuthorClass author)
    {
        frame.ThemeTypeVariation = ScriptMarks.Frame(author); frame.SetMeta("author", author.ToString()); frame.QueueRedraw();
    }

    // The patterned border of the other class: dashes along every edge, in the script frame's colour.
    public static void Pattern(Control on, AuthorClass author, TokenSet t)
    {
        if (!ScriptMarks.Patterned(author)) return;
        float width = Math.Max(1, t.Px("focus.width")), dash = Math.Max(3, t.Px("space.1") * 1.5f); Rect2 box = new Rect2(Vector2.Zero, on.Size).Grow(-width / 2);
        Vector2 a = box.Position, b = new(box.End.X, box.Position.Y), c = box.End, d = new(box.Position.X, box.End.Y);
        foreach ((Vector2 from, Vector2 to) in new[] { (a, b), (b, c), (c, d), (d, a) }) on.DrawDashedLine(from, to, t.C("script.frame"), width, dash);
    }

    public static string Tip(AuthorClass author, string? program) => (program != null ? "Drawn by the host from " + program : "Its program is missing") + "; author: " + ScriptMarks.Says(author);
}

// A panel a program declared, drawn by the host inside the script frame (ADR-D01.30): rows, bars and the host's
// widgets. It has no commit control. A panel whose program is missing keeps its place and shows a placeholder.
public sealed partial class ScriptPanelHost : PanelContainer
{
    public AuthorClass Author { get; }
    public ScriptPanelSpec? Spec { get; }
    public string PanelId { get; }
    public bool Pinned { get; private set; }
    public string Target { get; private set; } = "";
    public event Action<string>? Unmount;
    // A widget of the panel was used: its id and its new value.
    public event Action<string, string>? Edited;
    private readonly Func<TokenSet> tokens;
    private readonly bool closable;

    // An author class the host did not record reads as unknown origin: the patterned border.
    public ScriptPanelHost(string id, ScriptPanelSpec? spec, Func<TokenSet> tokens, bool closable = true, AuthorClass author = AuthorClass.Other)
    {
        PanelId = id; Spec = spec; Name = "ScriptPanel_" + id.Replace('.', '_'); Author = author; ScriptFrames.Apply(this, author);
        this.tokens = tokens; this.closable = closable;
        Draw += () => ScriptFrames.Pattern(this, Author, this.tokens());
        Refresh();
    }

    // What the panel is about: the selection, unless it is pinned.
    public void SetTarget(string target) { if (Pinned || Target == target) return; Target = target; Refresh(); }

    // Drawn again from the tokens in force.
    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        SessionWidgets.Clear(this);
        TokenSet t = tokens(); ScriptPanelSpec? spec = Spec; string id = PanelId;
        var column = new VBoxContainer(); AddChild(column);
        var head = new HBoxContainer(); column.AddChild(head);
        Label glyph = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(glyph, "script.mark", t.Px("icon.small")); glyph.AddThemeColorOverride("font_color", t.C("script.frame"));
        glyph.TooltipText = ScriptFrames.Tip(Author, spec?.Program); glyph.MouseFilter = MouseFilterEnum.Pass; head.AddChild(glyph);
        head.AddChild(ChromeWidgets.Text(spec?.Title ?? id, "MicroLabel", true));
        if (Target.Length > 0) head.AddChild(SessionWidgets.Chip(Target, t.C(Pinned ? "accent.strong" : "text.dim"), Pinned ? "Pinned target" : "Target: follows the selection"));
        Button pin = ChromeWidgets.Tool("", "Pin: keep this target when the selection changes", true); ChromeIcons.Apply(pin, "palette.pin", t.Px("icon.small"));
        pin.SetPressedNoSignal(Pinned); pin.Toggled += on => { Pinned = on; Refresh(); }; head.AddChild(pin);
        if (closable) head.AddChild(SessionWidgets.Tool("pane.close", "Unmount this panel", t, () => Safe.Raise(Unmount, id)));
        column.AddChild(new HSeparator());
        if (spec == null) { column.AddChild(ChromeWidgets.Empty("panel.scripts", "Program missing, slot kept")); return; }
        if (spec.Rows.Count > 0)
        {
            var grid = new GridContainer { Columns = 2 }; column.AddChild(grid);
            foreach ((string label, string value) in spec.Rows)
            {
                grid.AddChild(ChromeWidgets.Text(label, "DimLabel", true)); Label v = ChromeWidgets.Text(value, "CodeLabel"); v.HorizontalAlignment = HorizontalAlignment.Right; grid.AddChild(v);
            }
        }
        if (spec.Bars.Count > 0) column.AddChild(new ChartView(new ChartWidget("bars", "", "bars", spec.Bars), tokens));
        foreach (PanelWidget widget in spec.Widgets) column.AddChild(ScriptWidgets.Build(widget, tokens, (w, v) => Safe.Raise(Edited, w, v)));
    }
}

// The drawing surface of the canvas primitive (ADR-D01.31): vector marks in design tokens, display only. It is
// drawn again from the tokens in force whenever it is repainted.
public sealed partial class CanvasSurface : Control
{
    public IReadOnlyList<CanvasMark> Marks { get; private set; } = Array.Empty<CanvasMark>();
    private readonly Func<TokenSet> tokens;

    public CanvasSurface(Func<TokenSet> tokens, bool passive = false)
    {
        this.tokens = tokens; SizeFlagsHorizontal = SizeFlags.ExpandFill; SizeFlagsVertical = SizeFlags.ExpandFill; ClipContents = true;
        if (passive) MouseFilter = MouseFilterEnum.Ignore;
    }

    public void Show(IReadOnlyList<CanvasMark> marks)
    {
        foreach (CanvasMark mark in marks) if (!CanvasMark.Shapes.Contains(mark.Shape)) throw J.Error(mark.Shape, "unknown canvas shape");
        Marks = marks; QueueRedraw();
    }

    public override void _Draw()
    {
        TokenSet t = tokens(); Vector2 size = Size; float unit = Math.Min(size.X, size.Y);
        Vector2 At(double x, double y) => new((float)(size.X / 2 + (x - .5) * unit), (float)(size.Y / 2 + (y - .5) * unit));
        Font font = GetThemeFont("font", "Label"); int text = Math.Max(8, GetThemeFontSize("font_size", "Label") - 2);
        float thin = Math.Max(1, t.Px("border.width"));
        foreach (CanvasMark m in Marks)
        {
            Color colour = t.C(m.Colour);
            if (m.Shape == "line") DrawLine(At(m.X, m.Y), At(m.A, m.B), colour, thin, true);
            else if (m.Shape == "circle") DrawArc(At(m.X, m.Y), (float)(m.A * unit), 0, Mathf.Tau, 64, colour, thin, true);
            else if (m.Shape == "dot") DrawCircle(At(m.X, m.Y), (float)(m.A * unit), colour);
            else DrawString(font, At(m.X, m.Y), m.Text ?? "", HorizontalAlignment.Left, -1, text, colour);
        }
    }
}

// The canvas pane: a script's drawing inside the script frame.
public sealed partial class ScriptCanvas : PanelContainer
{
    public IReadOnlyList<CanvasMark> Marks => surface.Marks;
    public string Source { get; private set; } = "";
    public AuthorClass Author { get; private set; } = AuthorClass.Other;
    private readonly CanvasSurface surface;
    private readonly Func<TokenSet> tokens;
    private readonly HBoxContainer head = new();
    private readonly string hint;

    public ScriptCanvas(Func<TokenSet> tokens, string name = "ScriptCanvas", string hint = "Nothing drawn")
    {
        this.tokens = tokens; this.hint = hint; Name = name; ScriptFrames.Apply(this, Author); surface = new CanvasSurface(tokens);
        Draw += () => ScriptFrames.Pattern(this, Author, this.tokens());
        var column = new VBoxContainer(); AddChild(column); column.AddChild(head); column.AddChild(surface);
        Show(Array.Empty<CanvasMark>(), "", AuthorClass.Other);
    }

    // The frame follows the author class of the run that drew the marks.
    public void Show(IReadOnlyList<CanvasMark> marks, string source, AuthorClass author) { surface.Show(marks); Source = source; Author = author; ScriptFrames.Apply(this, author); Repaint(); }

    public void Repaint()
    {
        if (!IsInstanceValid(this)) return;
        TokenSet t = tokens(); SessionWidgets.Clear(head);
        Label glyph = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(glyph, "script.mark", t.Px("icon.small")); glyph.AddThemeColorOverride("font_color", t.C("script.frame"));
        glyph.TooltipText = ScriptFrames.Tip(Author, Marks.Count == 0 ? null : Source); glyph.MouseFilter = MouseFilterEnum.Pass; head.AddChild(glyph);
        head.AddChild(ChromeWidgets.Text(Marks.Count == 0 ? hint : Source, "MicroLabel", true));
        surface.QueueRedraw();
    }
}
