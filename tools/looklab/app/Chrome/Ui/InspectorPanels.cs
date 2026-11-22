using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// One numeric field with the shared gesture set (ADR-D01.23): drag the value, double-click to type a number or
// "=" and an expression, Up and Down as the ladder while typing (Shift coarser, Alt finer), click the label to
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
        if (!field.IsFact) { Value.MouseDefaultCursorShape = CursorShape.Hsize; Value.GuiInput += Gesture; AddChild(Mark); }
        AddChild(Caption); AddChild(Value);
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
            : $"{Field.Label}: drag, or double-click to type a value or = and an expression ({Field.Min}..{Field.Max})";
        Value.AddThemeColorOverride("font_color", t.C(Field.Binding != null ? "state.preview" : Field.Provenance == Provenance.Default ? "text.dim" : "text"));
    }

    private void Gesture(InputEvent input)
    {
        if (input is InputEventMouseButton { ButtonIndex: MouseButton.Left } button)
        {
            if (button.Pressed) dragged = 0;
            if (button.Pressed && button.DoubleClick) BeginEdit();
        }
        else if (input is InputEventMouseMotion motion && (motion.ButtonMask & MouseButtonMask.Left) != 0) DragBy(motion.Relative.X, motion.ShiftPressed ? 10 : motion.AltPressed ? .1 : 1);
    }

    public void DragBy(float pixels, double scale = 1)
    {
        dragged += pixels;
        int steps = (int)(dragged / DragPixels);
        if (steps != 0) { dragged -= steps * DragPixels; model.Nudge(Field.Id, steps, scale); }
    }

    public void BeginEdit()
    {
        if (Editor != null || Field.IsFact) return;
        Editor = new LineEdit { Text = Field.Display, CustomMinimumSize = Value.CustomMinimumSize, SelectAllOnFocus = true };
        Value.Visible = false; AddChild(Editor); Editor.GrabFocus();
        Editor.TextSubmitted += text => Type(text);
        Editor.FocusExited += EndEdit;
        Editor.GuiInput += input =>
        {
            if (input is not InputEventKey { Pressed: true } key) return;
            if (key.Keycode == Key.Escape) { EndEdit(); AcceptEvent(); }
            else if (key.Keycode is Key.Up or Key.Down)
            {
                model.Nudge(Field.Id, key.Keycode == Key.Up ? 1 : -1, key.ShiftPressed ? 10 : key.AltPressed ? .1 : 1);
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
        Selection.TooltipText = "The selection both inspector tiers read"; Selection.MouseFilter = MouseFilterEnum.Pass; row.AddChild(Selection);
        foreach (InspectField field in Model.Essentials)
        {
            row.AddChild(new VSeparator());
            var value = new ValueField(Model, field, tokens, refuse, false); fields.Add(value); row.AddChild(value);
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

    public InspectorPanel(InspectorModel model, Func<TokenSet> tokens, Action<string> refuse)
    {
        Model = model; this.tokens = tokens; this.refuse = refuse; Name = "InspectorPanel";
        HorizontalScrollMode = ScrollMode.Disabled; AddChild(column);
        model.Changed += Refresh; model.Updated += Update;
        Refresh();
    }

    private void Update(InspectField field) { foreach (ValueField f in fields.Where(f => IsInstanceValid(f) && f.Field == field)) f.Refresh(); }

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        SessionWidgets.Clear(column); fields.Clear();
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
            Label note = ChromeWidgets.Text(Notes[section], "MicroLabel", true); note.HorizontalAlignment = HorizontalAlignment.Right; head.AddChild(note);
            if (folded) continue;
            if (own.Length == 0) column.AddChild(ChromeWidgets.Text(section == InspectorModel.Preview ? "  No preview is staged." : "  Nothing selected.", "MicroLabel"));
            foreach (InspectField field in own)
            {
                var value = new ValueField(Model, field, tokens, refuse, true); fields.Add(value); column.AddChild(value);
            }
            column.AddChild(new HSeparator());
        }
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
    public void Enter(string line)
    {
        if (string.IsNullOrWhiteSpace(line)) return;
        History.Add(line.Trim()); Input.Text = ""; Entered?.Invoke(line.Trim());
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
        Model.Filter(Severity.Error, true); Found = last.Id; Refresh(); Opened?.Invoke(last);
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
                row.GuiInput += input => { if (input is InputEventMouseButton { Pressed: true, ButtonIndex: MouseButton.Left }) { Found = p.Id; Refresh(); Opened?.Invoke(p); } };
                List.AddChild(row);
            }
            if (!Model.Shown.Any()) List.AddChild(ChromeWidgets.Text(Model.Problems.Count == 0 ? "  No problems." : "  Every row is filtered out.", "MicroLabel"));
        }
        else if (Shows == Kind.Console)
        {
            foreach (string text in Model.Console) List.AddChild(ChromeWidgets.Text(text, "CodeLabel", true));
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
