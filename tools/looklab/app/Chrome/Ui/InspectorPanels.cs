using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// With the keyboard on a field: Left and Right step by the ladder's rung, Up and Down change the rung, Enter
// types, = starts an expression, Delete resets. The small ladder beside the value shows the rung.
// One numeric field with the shared gesture set (ADR-D01.23): drag the value, double-click to type a number or
// "=" and an expression, Up and Down as the ladder while typing (Shift coarser, Alt one step), click the label to
// reset. The mark beside the label names where the value comes from.
public sealed partial class ValueField : HBoxContainer
{
    public static readonly IReadOnlyDictionary<Provenance, (string Icon, string Colour, string Text)> Marks = new Dictionary<Provenance, (string, string, string)>
    {
        [Provenance.Default] = ("prov.default", "text.dim", "default"), [Provenance.Theme] = ("prov.theme", "text.dim", "from the theme"),
        [Provenance.Session] = ("prov.session", "accent.strong", "set in this session"), [Provenance.Override] = ("prov.override", "severity.warning", "override"),
        [Provenance.Script] = ("prov.script", "state.preview", "bound to a script expression"),
    };
    public const float DragPixels = 6;
    public InspectField Field { get; }
    public Button Caption { get; }
    public Label Mark { get; } = ChromeWidgets.Text("", "MicroLabel");
    public Button Value { get; }
    public LineEdit? Editor { get; private set; }
    public Control Ladder { get; } = new() { Name = "Ladder", CustomMinimumSize = new Vector2(9, 0), MouseFilter = MouseFilterEnum.Pass };
    private readonly InspectorModel model;
    private readonly Func<TokenSet> tokens;
    private readonly Action<string> refuse;
    private float dragged;

    public ValueField(InspectorModel model, InspectField field, Func<TokenSet> tokens, Action<string> refuse, bool wide)
    {
        this.model = model; this.tokens = tokens; this.refuse = refuse; Field = field; Name = "Field_" + field.Id.Replace('.', '_');
        AddThemeConstantOverride("separation", tokens().Px("space.1"));
        Caption = ChromeWidgets.Tool(field.Label, field.IsFact ? field.Label : field.Label + ": click to reset to its default", false, "FieldCaption");
        Caption.Alignment = HorizontalAlignment.Left; Caption.ClipText = true;
        if (wide) { Caption.SizeFlagsHorizontal = SizeFlags.ExpandFill; }
        Caption.Pressed += () => model.Reset(field.Id);
        Value = ChromeWidgets.Tool("", "", false, field.IsFact ? "FieldFact" : "FieldValue");
        Value.Alignment = HorizontalAlignment.Right; Value.CustomMinimumSize = new Vector2(tokens().Px("control.height") * (field.IsFact ? 3f : 2.4f), 0);
        if (!field.IsFact) { Value.MouseDefaultCursorShape = CursorShape.Hsize; Value.GuiInput += Gesture; Value.FocusMode = FocusModeEnum.All; AddChild(Mark); }
        AddChild(Caption); AddChild(Value);
        if (!field.IsFact) { AddChild(Ladder); Ladder.Draw += DrawLadder; }
        Refresh();
    }

    public void Refresh()
    {
        TokenSet t = tokens();
        Value.Text = Field.Display;
        if (Field.IsFact) return;
        (string icon, string colour, string text) = Marks[Field.Provenance];
        ChromeIcons.Apply(Mark, icon, t.Px("icon.small")); Mark.AddThemeColorOverride("font_color", t.C(colour));
        Mark.TooltipText = "Value " + text; Mark.MouseFilter = MouseFilterEnum.Pass;
        Value.TooltipText = Field.Binding != null ? $"Live binding: = {Field.Binding}. Its value is derived each time and never stored."
            : Field.Choices is { } named ? $"{Field.Label}: drag, or double-click to type one of {string.Join(", ", named)}"
            : $"{Field.Label}: drag, or double-click to type a value or = and an expression ({Field.Min}..{Field.Max})";
        Value.AddThemeColorOverride("font_color", t.C(Field.Binding != null ? "state.preview" : Field.Provenance == Provenance.Default ? "text.dim" : "text"));
        Ladder.TooltipText = $"Ladder: {InspectorModel.Rungs[model.Rung(Field.Id)]:0} of its steps for each notch. Up and Down change the rung; Left and Right step.";
        Ladder.QueueRedraw();
    }

    // Three rungs, the longest the coarsest; the field's own is the bright one.
    private void DrawLadder()
    {
        TokenSet t = tokens(); int at = model.Rung(Field.Id);
        for (int i = 0; i < InspectorModel.Rungs.Count; i++)
        {
            float y = Ladder.Size.Y / 2 + (i - 1) * 5;
            Ladder.DrawLine(new Vector2(0, y), new Vector2(8 - i * 2.5f, y), t.C(i == at ? "accent.strong" : "text.off"), i == at ? 2 : 1);
        }
    }

    private void Gesture(InputEvent input)
    {
        if (input is InputEventMouseButton { ButtonIndex: MouseButton.Left } button)
        {
            if (button.Pressed) dragged = 0;
            if (button.Pressed && button.DoubleClick) BeginEdit();
        }
        else if (input is InputEventMouseMotion motion && (motion.ButtonMask & MouseButtonMask.Left) != 0)
            DragBy(motion.Relative.X, motion.ShiftPressed ? 10 : motion.AltPressed ? 1 : InspectorModel.Rungs[model.Rung(Field.Id)]);
        else if (input is InputEventKey { Pressed: true } key && Editor == null && Keyed(key.Keycode)) Value.AcceptEvent();
    }

    // The keyboard alternative to the drag and the ladder; false when the key is not the field's.
    public bool Keyed(Key key)
    {
        switch (key)
        {
            case Key.Left: model.Step(Field.Id, -1); return true;
            case Key.Right: model.Step(Field.Id, 1); return true;
            case Key.Up: model.Climb(Field.Id, -1); return true;
            case Key.Down: model.Climb(Field.Id, 1); return true;
            case Key.Enter: case Key.KpEnter: BeginEdit(); return true;
            case Key.Equal: BeginEdit("= "); return true;
            case Key.Delete: case Key.Backspace: model.Reset(Field.Id); return true;
            default: return false;
        }
    }

    public void DragBy(float pixels, double scale = 1)
    {
        dragged += pixels;
        int steps = (int)(dragged / DragPixels);
        if (steps != 0) { dragged -= steps * DragPixels; model.Nudge(Field.Id, steps, scale); }
    }

    public void BeginEdit(string? start = null)
    {
        if (Editor != null || Field.IsFact) return;
        Editor = new LineEdit { Text = start ?? Field.Display, CustomMinimumSize = Value.CustomMinimumSize, SelectAllOnFocus = start == null };
        Value.Visible = false; AddChild(Editor); Editor.GrabFocus();
        if (start != null) Editor.CaretColumn = start.Length;
        Editor.TextSubmitted += text => Type(text);
        Editor.FocusExited += EndEdit;
        Editor.GuiInput += input =>
        {
            if (input is not InputEventKey { Pressed: true } key) return;
            if (key.Keycode == Key.Escape) { EndEdit(); AcceptEvent(); }
            else if (key.Keycode is Key.Up or Key.Down)
            {
                model.Nudge(Field.Id, key.Keycode == Key.Up ? 1 : -1, key.ShiftPressed ? 10 : 1);
                if (Editor != null) Editor.Text = Field.Display;
                AcceptEvent();
            }
        };
    }

    // The typed value; returns why it was refused. A refused text stays in the editor with its reason shown.
    public string? Type(string text)
    {
        string? refusal = model.Enter(Field.Id, text);
        if (refusal != null) refuse($"{Field.Label}: {refusal}"); else EndEdit();
        return refusal;
    }

    private void EndEdit()
    {
        if (Editor == null) return;
        LineEdit old = Editor; Editor = null; Value.Visible = true;
        old.QueueFree();
    }
}

// Tier 1 (ADR-D01.22): a thin strip at the edge of the main views with the selection and the Essential preview
// parameters. It has no commit control.
public sealed partial class InspectorStrip : PanelContainer
{
    public InspectorModel Model { get; }
    public Label Selection { get; private set; } = null!;
    public HBoxContainer Tools { get; } = new();
    public IReadOnlyList<ValueField> Fields => fields;
    // The strip's count of what the docked inspector holds beyond it was pressed.
    public event Action? More;
    private readonly List<ValueField> fields = new();
    private readonly Func<TokenSet> tokens;
    private readonly Action<string> refuse;
    private readonly HBoxContainer row = new();

    public InspectorStrip(InspectorModel model, Func<TokenSet> tokens, Action<string> refuse)
    {
        Model = model; this.tokens = tokens; this.refuse = refuse; Name = "InspectorStrip"; ThemeTypeVariation = "BarBottom";
        AddChild(row);
        model.Changed += Refresh; model.Updated += Update;
        Refresh();
    }

    private void Update(InspectField field) { foreach (ValueField f in fields.Where(f => IsInstanceValid(f) && f.Field == field)) f.Refresh(); }

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        ChromeWidgets.Detach(Tools); SessionWidgets.Clear(row); fields.Clear();
        TokenSet t = tokens();
        CustomMinimumSize = new Vector2(0, t.Px("status.height"));
        row.AddChild(new Control { CustomMinimumSize = new Vector2(t.Px("space.2"), 0) });
        Label icon = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(icon, "panel.inspector", t.Px("icon.small")); row.AddChild(icon);
        Selection = ChromeWidgets.Text(Model.SelectionCount == 0 ? "nothing selected" : $"{Model.SelectionCount} {Model.SelectionKind}", "DimLabel");
        Selection.TooltipText = "What is selected"; Selection.MouseFilter = MouseFilterEnum.Pass; row.AddChild(Selection);
        foreach (InspectField field in Model.Essentials)
        {
            row.AddChild(new VSeparator());
            var value = new ValueField(Model, field, tokens, refuse, false); fields.Add(value); row.AddChild(value);
        }
        if (Model.Beyond > 0)
        {
            row.AddChild(new VSeparator());
            Button more = ChromeWidgets.Tool($"+{Model.Beyond}", $"{Model.Beyond} more fields in the Inspector dock: show or hide it"); more.Name = "InspectorMore";
            more.Pressed += () => Safe.Raise(More); row.AddChild(more);
        }
        row.AddChild(new Control { SizeFlagsHorizontal = SizeFlags.ExpandFill });
        row.AddChild(Tools);
        row.AddChild(new Control { CustomMinimumSize = new Vector2(t.Px("space.2"), 0) });
    }
}

// Tier 2 (ADR-D01.22): the docked inspector with its three fixed sections. It has no commit control.
public sealed partial class InspectorPanel : ScrollContainer
{
    public static readonly IReadOnlyDictionary<string, string> Notes = new Dictionary<string, string>
    {
        [InspectorModel.Selection] = "read-only puzzle state", [InspectorModel.View] = "both views", [InspectorModel.Preview] = "from typed .6cs declarations",
    };
    public InspectorModel Model { get; }
    public IReadOnlyList<ValueField> Fields => fields;
    public ISet<string> Folded { get; } = new HashSet<string>();
    private readonly List<ValueField> fields = new();
    private readonly Func<TokenSet> tokens;
    private readonly Action<string> refuse;
    private readonly VBoxContainer column = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill };
    private readonly Dictionary<string, (Label Away, Button Reset)> heads = new();

    public InspectorPanel(InspectorModel model, Func<TokenSet> tokens, Action<string> refuse)
    {
        Model = model; this.tokens = tokens; this.refuse = refuse; Name = "InspectorPanel";
        HorizontalScrollMode = ScrollMode.Disabled; AddChild(column);
        model.Changed += Refresh; model.Updated += Update;
        Refresh();
    }

    private void Update(InspectField field) { foreach (ValueField f in fields.Where(f => IsInstanceValid(f) && f.Field == field)) f.Refresh(); Heads(); }

    // A section's head counts the numbers away from their defaults and resets them together.
    private void Heads()
    {
        foreach ((string section, (Label away, Button reset)) in heads)
        {
            if (!IsInstanceValid(away)) continue;
            int n = Model.Away(section); away.Text = n == 0 ? "" : $"{n} changed"; away.Visible = reset.Visible = n > 0;
        }
    }

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        SessionWidgets.Clear(column); fields.Clear(); heads.Clear();
        TokenSet t = tokens();
        foreach (string section in InspectorModel.Sections)
        {
            InspectField[] own = Model.Fields.Where(f => f.Section == section).ToArray();
            bool folded = Folded.Contains(section);
            var head = new HBoxContainer(); column.AddChild(head);
            Button fold = SessionWidgets.Tool(folded ? "fold.closed" : "fold.open", folded ? "Show " + section : "Fold " + section, t,
                () => { if (!Folded.Remove(section)) Folded.Add(section); Refresh(); });
            fold.SetMeta("section", section); head.AddChild(fold);
            head.AddChild(ChromeWidgets.Text(section.ToUpperInvariant(), "MicroLabel"));
            string said = section == InspectorModel.View ? ViewFields.Note(Model.Scope) : Notes[section];
            Label note = ChromeWidgets.Text(said, "MicroLabel", true); note.HorizontalAlignment = HorizontalAlignment.Right; head.AddChild(note);
            // The View section's scope switch (revision 6, Q3): Both, Global or Local, when the host's views keep their own values.
            if (section == InspectorModel.View && Model.Scopes.Count > 1)
                foreach (ViewScope scope in Model.Scopes)
                {
                    Button choose = ChromeWidgets.Tool(scope.ToString(), scope == ViewScope.Both ? "Edit both views" : $"Edit the {scope} view only", true);
                    choose.Name = "Scope" + scope; choose.ButtonPressed = Model.Scope == scope; choose.SetMeta("scope", scope.ToString());
                    choose.Pressed += () => { if (!Model.SetScope(scope)) Refresh(); };
                    head.AddChild(choose);
                }
            Label away = ChromeWidgets.Text("", "MicroLabel"); away.Name = "Away"; away.AddThemeColorOverride("font_color", t.C("accent.strong")); head.AddChild(away);
            Button reset = ChromeWidgets.Tool("Reset", $"Reset every {section} field to its default"); reset.Name = "ResetSection";
            reset.Pressed += () => Model.ResetSection(section); head.AddChild(reset); heads[section] = (away, reset);
            if (folded) continue;
            if (own.Length == 0) column.AddChild(ChromeWidgets.Text(section == InspectorModel.Preview ? "  No preview is staged." : "  Nothing selected.", "MicroLabel"));
            foreach (InspectField field in own)
            {
                var value = new ValueField(Model, field, tokens, refuse, true); fields.Add(value); column.AddChild(value);
            }
            column.AddChild(new HSeparator());
        }
        Heads();
    }
}

// The bottom index (ADR-D01.27): Problems with severity filters, merged repeats and Find Last Error; Console;
// Tasks with progress drawn on each item and a cancel control.
public sealed partial class FeedbackPanel : VBoxContainer
{
    public enum Kind { Problems, Console, Tasks }
    public static readonly IReadOnlyDictionary<Severity, (string Icon, string Colour)> Shapes = new Dictionary<Severity, (string, string)>
    {
        [Severity.Error] = ("severity.error", "severity.error"), [Severity.Warning] = ("severity.warning", "severity.warning"), [Severity.Info] = ("severity.info", "severity.info"),
    };
    public FeedbackModel Model { get; }
    public Kind Shows { get; }
    public HBoxContainer Tools { get; } = new();
    public VBoxContainer List { get; } = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill };
    public int? Found { get; private set; }
    // A row was chosen: the host opens its source line and marks its puzzle element.
    public event Action<Problem>? Opened;
    // The console's typed line (ADR-D01.57): Enter sends it to the host, Up and Down step its history.
    public LineEdit Input { get; } = new() { PlaceholderText = "A 600 Cell Script line, then Enter. A line never commits.", Name = "ConsoleInput" };
    public LineHistory History { get; } = new();
    public event Action<string>? Entered;
    // Tab completes the typed word from what the host offers.
    public Func<string, IReadOnlyList<string>>? Completer { get; set; }
    private static readonly IReadOnlyDictionary<ConsoleKind, (string Icon, string Colour)> RowShapes = new Dictionary<ConsoleKind, (string, string)>
    {
        [ConsoleKind.Echo] = ("console.echo", "text.dim"), [ConsoleKind.Typed] = ("console.typed", "accent.strong"), [ConsoleKind.Result] = ("console.result", "severity.ok"),
        [ConsoleKind.Error] = ("severity.error", "severity.error"), [ConsoleKind.Lock] = ("lock", "severity.warning"),
    };
    public void Enter(string line)
    {
        if (string.IsNullOrWhiteSpace(line)) return;
        History.Add(line.Trim()); Input.Text = ""; Safe.Raise(Entered, line.Trim());
    }
    private readonly Func<TokenSet> tokens;
    private readonly HBoxContainer bar = new();
    private readonly ScrollContainer scroll = new() { SizeFlagsVertical = SizeFlags.ExpandFill, HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };

    public FeedbackPanel(FeedbackModel model, Kind shows, Func<TokenSet> tokens)
    {
        Model = model; Shows = shows; this.tokens = tokens; Name = shows + "Panel";
        AddThemeConstantOverride("separation", 0);
        AddChild(bar);
        AddChild(scroll); scroll.AddChild(List);
        if (shows == Kind.Console)
        {
            AddChild(Input); Input.TextSubmitted += Enter;
            Input.GuiInput += e =>
            {
                if (e is InputEventKey { Pressed: true, Keycode: Key.Tab } && Completer != null)
                {
                    if (Completer(Input.Text) is { Count: > 0 } found) { Input.Text = found[0]; Input.CaretColumn = Input.Text.Length; }
                    Input.AcceptEvent(); return;
                }
                if (e is not InputEventKey { Pressed: true } key || key.Keycode is not (Key.Up or Key.Down)) return;
                Input.Text = History.Step(key.Keycode == Key.Up ? -1 : 1); Input.CaretColumn = Input.Text.Length; Input.AcceptEvent();
            };
        }
        model.Changed += Refresh;
        Refresh();
    }

    public bool FindLastError()
    {
        if (Model.LastError is not { } last) return false;
        Model.Filter(Severity.Error, true); Found = last.Id; Refresh(); Safe.Raise(Opened, last);
        return true;
    }

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        ChromeWidgets.Detach(Tools); SessionWidgets.Clear(bar); SessionWidgets.Clear(List);
        TokenSet t = tokens();
        if (Shows == Kind.Problems)
        {
            foreach (Severity severity in Enum.GetValues<Severity>())
            {
                (string icon, string colour) = Shapes[severity];
                Button filter = ChromeWidgets.Tool("", $"Show {severity.ToString().ToLowerInvariant()} rows ({Model.Count(severity)})", true); ChromeIcons.Apply(filter, icon, t.Px("icon.small"));
                filter.AddThemeColorOverride("font_color", t.C(colour)); filter.AddThemeColorOverride("font_pressed_color", t.C(colour));
                filter.SetPressedNoSignal(!Model.Hidden.Contains(severity)); filter.SetMeta("severity", (int)severity);
                filter.Toggled += on => Model.Filter(severity, on);
                bar.AddChild(filter); bar.AddChild(ChromeWidgets.Text(Model.Count(severity).ToString(), "MicroLabel"));
            }
            bar.AddChild(new VSeparator());
            Button last = SessionWidgets.Tool("problems.last", "Find the last error", t, () => FindLastError()); last.Name = "FindLastError"; last.Disabled = Model.LastError == null; bar.AddChild(last);
            foreach (Problem p in Model.Shown.Reverse())
            {
                var row = new PanelContainer { ThemeTypeVariation = p.Id == Found ? "RowSelected" : "Row", MouseFilter = MouseFilterEnum.Stop };
                row.SetMeta("problem", p.Id);
                var line = new HBoxContainer(); row.AddChild(line);
                (string icon, string colour) = Shapes[p.Severity];
                Label shape = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(shape, icon, t.Px("icon.small")); shape.AddThemeColorOverride("font_color", t.C(colour)); line.AddChild(shape);
                line.AddChild(ChromeWidgets.Text(p.Text, "", true));
                if (p.Count > 1) line.AddChild(SessionWidgets.Chip("×" + p.Count, t.C("text.dim"), $"Reported {p.Count} times"));
                if (p.Element != null) line.AddChild(SessionWidgets.Chip(p.Element, t.C(colour), "The puzzle element this is about"));
                line.AddChild(ChromeWidgets.Text($"{p.Source}:{p.Line}", "CodeLabel"));
                row.GuiInput += input => { if (input is InputEventMouseButton { Pressed: true, ButtonIndex: MouseButton.Left }) { Found = p.Id; Refresh(); Safe.Raise(Opened, p); } };
                List.AddChild(row);
            }
            if (!Model.Shown.Any()) List.AddChild(ChromeWidgets.Text(Model.Problems.Count == 0 ? "  No problems." : "  Every row is filtered out.", "MicroLabel"));
        }
        else if (Shows == Kind.Console)
        {
            foreach (string text in Model.Console)
            {
                // What was typed, what the frame echoed, a result, an error and a refusal each have a shape.
                (ConsoleKind kind, string body) = ConsoleRows.Read(text); (string icon, string colour) = RowShapes[kind];
                var line = new HBoxContainer(); line.SetMeta("console", (int)kind); List.AddChild(line);
                Label shape = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(shape, icon, t.Px("icon.small")); shape.AddThemeColorOverride("font_color", t.C(colour));
                shape.TooltipText = kind switch { ConsoleKind.Typed => "typed", ConsoleKind.Result => "result", ConsoleKind.Error => "error", ConsoleKind.Lock => "refused", _ => "done in the window" };
                shape.MouseFilter = MouseFilterEnum.Pass; line.AddChild(shape);
                Label words = ChromeWidgets.Text(body, "CodeLabel", true); if (kind is ConsoleKind.Echo) words.AddThemeColorOverride("font_color", t.C("text.dim")); line.AddChild(words);
            }
            if (Model.Console.Count == 0) List.AddChild(ChromeWidgets.Text("  The console is empty.", "MicroLabel"));
            // The newest line stays in view.
            Callable.From(() => { if (IsInstanceValid(scroll)) scroll.ScrollVertical = (int)scroll.GetVScrollBar().MaxValue; }).CallDeferred();
        }
        else
        {
            foreach (TaskItem task in Model.Tasks.Reverse())
            {
                var line = new HBoxContainer(); line.SetMeta("task", task.Id); List.AddChild(line);
                (string icon, string colour) = task.State switch
                {
                    TaskState.Running => ("task.running", "state.pending"), TaskState.Ok => ("task.ok", "severity.ok"),
                    TaskState.Failed => ("severity.error", "severity.error"), _ => ("task.cancelled", "text.dim"),
                };
                Label shape = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(shape, icon, t.Px("icon.small")); shape.AddThemeColorOverride("font_color", t.C(colour));
                shape.TooltipText = task.State.ToString().ToLowerInvariant(); shape.MouseFilter = MouseFilterEnum.Pass; line.AddChild(shape);
                line.AddChild(ChromeWidgets.Text(task.Name, "", true));
                line.AddChild(new ProgressBar { MinValue = 0, MaxValue = 1, Value = task.Progress, ShowPercentage = false, SizeFlagsVertical = SizeFlags.ShrinkCenter,
                    CustomMinimumSize = new Vector2(t.Px("dock.width") * .5f, t.Px("slider.height")) });
                line.AddChild(ChromeWidgets.Text(task.State == TaskState.Running ? $"{task.Progress * 100:0}%" : task.State.ToString().ToLowerInvariant(), "MicroLabel"));
                if (task.State == TaskState.Running) line.AddChild(SessionWidgets.Tool("task.cancel", "Cancel this task (Esc cancels the latest)", t, () => Model.Cancel(task.Id)));
            }
            if (Model.Tasks.Count == 0) List.AddChild(ChromeWidgets.Text("  No task has run.", "MicroLabel"));
        }
        bar.AddChild(new Control { SizeFlagsHorizontal = SizeFlags.ExpandFill }); bar.AddChild(Tools);
    }
}
