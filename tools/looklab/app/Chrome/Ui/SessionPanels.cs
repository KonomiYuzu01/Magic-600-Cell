using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

internal static class SessionWidgets
{
    public static readonly IReadOnlyDictionary<PreviewStatus, (string Text, string Colour)> Status = new Dictionary<PreviewStatus, (string, string)>
    {
        [PreviewStatus.Ready] = ("ready", "state.preview"), [PreviewStatus.Impossible] = ("impossible", "state.invalid"),
        [PreviewStatus.Stale] = ("stale", "state.stale"), [PreviewStatus.Refused] = ("refused", "severity.error"),
    };

    public static Button Tool(string icon, string tooltip, TokenSet tokens, Action pressed, string? text = null)
    {
        Button button = ChromeWidgets.Tool("", tooltip);
        ChromeIcons.Apply(button, icon, tokens.Px("icon.small"));
        if (text != null) { button.Text = text; button.RemoveThemeFontOverride("font"); }
        button.Pressed += pressed;
        return button;
    }

    public static PanelContainer Chip(string text, Color colour, string tooltip = "")
    {
        var chip = new PanelContainer { ThemeTypeVariation = "Chip", TooltipText = tooltip };
        Label label = ChromeWidgets.Text(text, "MicroLabel"); label.AddThemeColorOverride("font_color", colour); chip.AddChild(label);
        return chip;
    }

    public static void Clear(Node node) { foreach (Node child in node.GetChildren()) { node.RemoveChild(child); child.QueueFree(); } }
}

// The commit bar (ADR-D01.11): one slim bar that summarises the pending preview and carries commit, discard
// and promote. Commit is disabled with its reason shown when the preview is impossible, stale or refused.
public sealed partial class CommitBar : PanelContainer
{
    public PreviewModel Model { get; }
    public Button Commit { get; private set; } = null!;
    public Label State { get; private set; } = null!;
    public Label Reason { get; private set; } = null!;
    public HSlider? Scrub { get; private set; }
    // The host's own buttons, at the right end of the bar.
    public HBoxContainer Tools { get; } = new();
    public string CommitKeys { get; set; } = "";
    public event Action<string, string?>? Command;
    // Inline parameters (ADR-D01.11): the numbers of the staged statement, edited in place. The host rebuilds the
    // preview when one changes; the bar never does.
    public event Action<int, int>? Parameter;
    public IReadOnlyList<StatementParameter> Parameters { get; private set; } = Array.Empty<StatementParameter>();
    public HBoxContainer ParameterRow { get; private set; } = new();
    private readonly Func<TokenSet> tokens;
    private readonly HBoxContainer row = new();

    public CommitBar(PreviewModel model, Func<TokenSet> tokens)
    {
        Model = model; this.tokens = tokens; Name = "CommitBar"; ThemeTypeVariation = "BarBottom";
        AddChild(row); model.Changed += Refresh; Refresh();
    }

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        ChromeWidgets.Detach(Tools); SessionWidgets.Clear(row); Scrub = null; Parameters = Array.Empty<StatementParameter>(); ParameterRow = new HBoxContainer { Name = "InlineParameters" };
        TokenSet t = tokens();
        CustomMinimumSize = new Vector2(0, t.Px("bar.height"));
        row.AddChild(new Control { CustomMinimumSize = new Vector2(t.Px("space.2"), 0) });
        Preview? p = Model.Pending;
        State = ChromeWidgets.Text("", "MicroLabel"); Reason = ChromeWidgets.Text("", "DimLabel", true);
        if (p == null)
        {
            row.AddChild(SessionWidgets.Chip("no preview", t.C("text.dim")));
            row.AddChild(new Control { SizeFlagsHorizontal = SizeFlags.ExpandFill });
            Commit = ChromeWidgets.Tool("Commit", "There is no preview"); Commit.Disabled = true;
        }
        else
        {
            (string text, string colour) = SessionWidgets.Status[p.Status];
            PanelContainer chip = SessionWidgets.Chip(text, t.C(colour), p.Reason ?? "The preview can be committed."); row.AddChild(chip);
            State = (Label)chip.GetChild(0);
            Label statement = ChromeWidgets.Text(p.Statement, "CodeLabel", true); statement.TooltipText = p.Statement;
            statement.SizeFlagsHorizontal = SizeFlags.ExpandFill; statement.SizeFlagsStretchRatio = 2; row.AddChild(statement);
            Parameters = InlineParameters.Find(p.Statement); row.AddChild(ParameterRow);
            for (int i = 0; i < Math.Min(Parameters.Count, InlineParameters.Shown); i++)
            {
                int index = i; StatementParameter q = Parameters[i];
                var box = new SpinBox { Name = "Parameter" + i, MinValue = q.Min, MaxValue = q.Max, Step = 1, Value = q.Value, Prefix = q.Name, SizeFlagsVertical = SizeFlags.ShrinkCenter,
                    TooltipText = $"{q.Name}: {q.Min} to {q.Max}. A change rebuilds the preview." };
                box.ValueChanged += v => { if ((int)v != q.Value) Safe.Raise(Parameter, index, (int)v); };
                ParameterRow.AddChild(box);
            }
            if (Parameters.Count > InlineParameters.Shown) ParameterRow.AddChild(ChromeWidgets.Text($"+{Parameters.Count - InlineParameters.Shown}", "DimLabel"));
            row.AddChild(ChromeWidgets.Text($"{p.Source} · {Model.Scrub}/{p.Moves.Count} moves · #{p.Id}", "DimLabel"));
            if (p.Moves.Count > 0)
            {
                Scrub = new HSlider { MinValue = 0, MaxValue = p.Moves.Count, Step = 1, Value = Model.Scrub, CustomMinimumSize = new Vector2(t.Px("dock.width") * .5f, 0),
                    SizeFlagsVertical = SizeFlags.ShrinkCenter, TooltipText = "Scrub from before to after; Commit takes the moves up to here", FocusMode = FocusModeEnum.None };
                Scrub.ValueChanged += v => Model.SetScrub((int)v); row.AddChild(Scrub);
            }
            if (p.Reason != null)
            {
                Reason.Text = p.Reason; Reason.TooltipText = p.Reason; Reason.SizeFlagsHorizontal = SizeFlags.ExpandFill;
                Reason.AddThemeColorOverride("font_color", t.C(colour)); row.AddChild(Reason);
            }
            row.AddChild(SessionWidgets.Tool("preview.moves", "Open the preview view with the move strip", t, () => Safe.Raise(Command, "view.preview.open", null)));
            row.AddChild(SessionWidgets.Tool("preview.promote", "Promote: save this preview as a macro", t, () => Safe.Raise(Command, "macro.save", null)));
            row.AddChild(SessionWidgets.Tool("preview.discard", "Discard the preview (Esc)", t, () => Safe.Raise(Command, "operation.preview.cancel", null), "Discard"));
            string label = Model.Scrub < p.Moves.Count && Model.Scrub > 0 ? $"Commit {Model.Scrub}" : "Commit";
            Commit = SessionWidgets.Tool("preview.commit", p.Status == PreviewStatus.Ready ? $"Commit one labelled entry ({CommitKeys})" : "Commit is off: " + p.Reason, t,
                () => Safe.Raise(Command, "operation.preview.commit", null), label);
            Commit.Disabled = p.Status != PreviewStatus.Ready || Model.Scrub == 0;
            if (!Commit.Disabled) Commit.AddThemeColorOverride("font_color", t.C("accent.strong"));
        }
        Commit.Name = "CommitButton"; row.AddChild(Commit);
        row.AddChild(new VSeparator()); row.AddChild(Tools);
        row.AddChild(new Control { CustomMinimumSize = new Vector2(t.Px("space.2"), 0) });
    }
}

// The preview view (ADR-D01.10): the witness sequence with move-by-move stepping, the effect on every orbit
// and on protection. The previewed state itself is drawn by the core into Stage.
public sealed partial class PreviewPanel : VBoxContainer
{
    public PreviewModel Model { get; }
    public Control Stage { get; } = new PanelContainer { ThemeTypeVariation = "ViewFrame", SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsVertical = SizeFlags.ExpandFill };
    public VBoxContainer Moves { get; } = new() { Name = "PreviewMoves" };
    public VBoxContainer Effects { get; } = new() { Name = "PreviewEffects" };
    public Label Title { get; } = ChromeWidgets.Text("", "HeadingLabel");
    private readonly Func<TokenSet> tokens;
    private readonly HBoxContainer head = new();
    private readonly HSplitContainer body = new() { SizeFlagsVertical = SizeFlags.ExpandFill };
    private readonly Label stageNote = ChromeWidgets.Text("", "DimLabel");

    public PreviewPanel(PreviewModel model, Func<TokenSet> tokens)
    {
        Model = model; this.tokens = tokens; Name = "PreviewPanel";
        AddChild(head); AddChild(new HSeparator()); AddChild(body);
        var left = new ScrollContainer { CustomMinimumSize = new Vector2(120, 0), SizeFlagsHorizontal = SizeFlags.ExpandFill, HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };
        Moves.SizeFlagsHorizontal = SizeFlags.ExpandFill; Moves.AddThemeConstantOverride("separation", 0); left.AddChild(Moves);
        var right = new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsStretchRatio = 1.6f };
        stageNote.AutowrapMode = TextServer.AutowrapMode.WordSmart; stageNote.HorizontalAlignment = HorizontalAlignment.Center; stageNote.VerticalAlignment = VerticalAlignment.Center;
        Stage.AddChild(stageNote); right.AddChild(Stage); right.AddChild(Effects);
        body.AddChild(left); body.AddChild(right);
        model.Changed += Refresh; Refresh();
    }

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        TokenSet t = tokens(); Preview? p = Model.Pending;
        ChromeWidgets.Detach(Title); SessionWidgets.Clear(head); SessionWidgets.Clear(Moves); SessionWidgets.Clear(Effects);
        head.AddChild(new Control { CustomMinimumSize = new Vector2(t.Px("space.2"), 0) });
        Title.Text = p == null ? "No preview" : $"Preview #{p.Id}"; ChromeWidgets.Detach(Title); head.AddChild(Title);
        if (p == null)
        {
            stageNote.Text = "no preview staged";
            return;
        }
        (string text, string colour) = SessionWidgets.Status[p.Status];
        head.AddChild(SessionWidgets.Chip(text, t.C(colour), p.Reason ?? ""));
        if (Model.Replaced) head.AddChild(SessionWidgets.Chip("replaced the earlier preview", t.C("state.stale"), "The earlier preview can no longer be committed."));
        head.AddChild(new Control { SizeFlagsHorizontal = SizeFlags.ExpandFill });
        head.AddChild(ChromeWidgets.Text($"{Model.Scrub} of {p.Moves.Count}", "DimLabel"));
        head.AddChild(SessionWidgets.Tool("step.back", "One move back (Left)", t, () => Model.SetScrub(Model.Scrub - 1)));
        head.AddChild(SessionWidgets.Tool("step.forward", "One move forward (Right)", t, () => Model.SetScrub(Model.Scrub + 1)));
        stageNote.Text = p.Status == PreviewStatus.Ready
            ? $"after move {Model.Scrub} of {p.Moves.Count}"
            : "Nothing to draw: " + p.Reason;
        for (int i = 0; i < p.Moves.Count; i++)
        {
            int index = i; bool done = i < Model.Scrub, at = i == Model.Scrub - 1;
            var holder = new PanelContainer { ThemeTypeVariation = at ? "RowSelected" : "Row" }; holder.SetMeta("move", i);
            var line = new HBoxContainer(); holder.AddChild(line);
            Label number = ChromeWidgets.Text((i + 1).ToString(System.Globalization.CultureInfo.InvariantCulture), "MicroLabel");
            number.CustomMinimumSize = new Vector2(t.Px("icon.medium") * 1.5f, 0); number.HorizontalAlignment = HorizontalAlignment.Right; line.AddChild(number);
            Label move = ChromeWidgets.Text(p.Moves[i], "CodeLabel", true); move.SizeFlagsHorizontal = SizeFlags.ExpandFill;
            if (!done) move.AddThemeColorOverride("font_color", t.C("text.off"));
            line.AddChild(move);
            holder.GuiInput += e => { if (e is InputEventMouseButton { Pressed: true, ButtonIndex: MouseButton.Left }) Model.SetScrub(index + 1); };
            Moves.AddChild(holder);
        }
        Effects.AddChild(ChromeWidgets.Text("EFFECT ON EVERY ORBIT", "MicroLabel"));
        foreach (OrbitEffect effect in p.Effects)
        {
            var line = new HBoxContainer();
            Label name = ChromeWidgets.Text(effect.Orbit, "", true); name.CustomMinimumSize = new Vector2(t.Px("dock.width") * .45f, 0); line.AddChild(name);
            var bar = new ProgressBar { MinValue = 0, MaxValue = effect.Total, Value = effect.Moved, ShowPercentage = false, SizeFlagsHorizontal = SizeFlags.ExpandFill,
                SizeFlagsVertical = SizeFlags.ShrinkCenter, CustomMinimumSize = new Vector2(0, t.Px("space.2")) };
            line.AddChild(bar);
            Label count = ChromeWidgets.Text(effect.Moved == 0 ? "untouched" : $"{effect.Moved:N0} of {effect.Total:N0}", effect.Moved == 0 ? "DimLabel" : "CodeLabel");
            count.CustomMinimumSize = new Vector2(t.Px("dock.width") * .5f, 0); count.HorizontalAlignment = HorizontalAlignment.Right; line.AddChild(count);
            Effects.AddChild(line);
        }
        Label protection = ChromeWidgets.Text(p.Protection, "", true);
        protection.AddThemeColorOverride("font_color", t.C(p.Status == PreviewStatus.Impossible ? "state.invalid" : "state.protected"));
        var guard = new HBoxContainer(); Label lockMark = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(lockMark, "mark.protected", t.Px("icon.small"));
        lockMark.AddThemeColorOverride("font_color", t.C("state.protected")); guard.AddChild(lockMark); guard.AddChild(protection);
        Effects.AddChild(guard);
    }
}

// The History dock (ADR-D01.15): the move history as .6cs, one header row per transaction that opens to its
// move lines, with a rollback cursor; and the layout history beside it, with its own undo and redo.
public sealed partial class HistoryPanel : VBoxContainer
{
    public MoveHistory Moves { get; }
    public LayoutHistory Layout { get; }
    public bool ShowLayout { get; private set; }
    // The plain list of named transactions a script may switch the dock to (history.dock.mode).
    public bool PlainList { get; private set; }
    public VBoxContainer List { get; } = new() { Name = "HistoryRows", SizeFlagsHorizontal = SizeFlags.ExpandFill };
    public event Action<string, string?>? Command;
    private readonly Func<TokenSet> tokens;
    private readonly HBoxContainer head = new();
    private readonly HashSet<string> open = new(StringComparer.Ordinal);

    public HistoryPanel(MoveHistory moves, LayoutHistory layout, Func<TokenSet> tokens)
    {
        Moves = moves; Layout = layout; this.tokens = tokens; Name = "HistoryPanel";
        AddChild(head); AddChild(new HSeparator());
        var scroll = new ScrollContainer { SizeFlagsVertical = SizeFlags.ExpandFill, HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };
        List.AddThemeConstantOverride("separation", 0); scroll.AddChild(List); AddChild(scroll);
        moves.Changed += Refresh; layout.Changed += Refresh; Refresh();
    }

    public void Show(bool layout) { ShowLayout = layout; Refresh(); }
    public void SetPlainList(bool plain) { PlainList = plain; Refresh(); }
    public void Toggle(string entry) { if (!open.Add(entry)) open.Remove(entry); Refresh(); }
    public bool IsOpen(string entry) => open.Contains(entry);

    private void Send(string command, string? argument = null) => Safe.Raise(Command, command, argument);

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        TokenSet t = tokens();
        SessionWidgets.Clear(head); SessionWidgets.Clear(List);
        foreach ((bool layout, string title, string tip) in new[] { (false, "Moves", "The move history: committed transactions"), (true, "Layout", "The layout history: views, workspaces and appearance") })
        {
            Button tab = ChromeWidgets.Tool(title, tip, true, "TabButton"); tab.SetPressedNoSignal(ShowLayout == layout); tab.SetMeta("history", title);
            tab.Pressed += () => Show(layout); head.AddChild(tab);
        }
        head.AddChild(new Control { SizeFlagsHorizontal = SizeFlags.ExpandFill });
        if (ShowLayout)
        {
            head.AddChild(SessionWidgets.Tool("history.undo", "Undo the last layout change (Ctrl+Alt+Z)", t, () => Send("layout.undo")));
            head.AddChild(SessionWidgets.Tool("history.redo", "Redo the layout change (Ctrl+Alt+Y)", t, () => Send("layout.redo")));
            for (int i = Layout.Steps.Count - 1; i >= 0; i--)
            {
                int index = i;
                Control row = Row(t, i == Layout.Cursor, i > Layout.Cursor, null, Layout.Steps[i].Name, i == Layout.Cursor ? "now" : "", null, () => Send("layout.roll", index.ToString(System.Globalization.CultureInfo.InvariantCulture)));
                row.SetMeta("layout", i); List.AddChild(row);
            }
            if (Layout.Steps.Count == 0) List.AddChild(ChromeWidgets.Text("No layout change yet.", "DimLabel"));
            return;
        }
        head.AddChild(SessionWidgets.Tool("history.partial", "Step back one move inside the last entry", t, () => Send("session.undo.partial")));
        head.AddChild(SessionWidgets.Tool("history.undo", "Undo the last entry (Ctrl+Z)", t, () => Send("session.undo")));
        head.AddChild(SessionWidgets.Tool("history.redo", "Redo (Ctrl+Y)", t, () => Send("session.redo")));
        // Newest first; the rollback cursor sits between the applied entries and the undone ones.
        for (int i = Moves.Entries.Count - 1; i >= 0; i--)
        {
            HistoryEntry entry = Moves.Entries[i]; int index = i; bool undone = i >= Moves.Cursor;
            if (i == Moves.Cursor - 1) List.AddChild(CursorLine(t));
            bool partial = i == Moves.Cursor - 1 && Moves.Inner > 0;
            Control row = Row(t, false, undone, entry.Origin, entry.Name, $"{entry.Id}  {entry.Time}", PlainList ? null : () => Toggle(entry.Id),
                () => Send("session.rollback", (index + 1).ToString(System.Globalization.CultureInfo.InvariantCulture)), IsOpen(entry.Id));
            row.SetMeta("entry", entry.Id); List.AddChild(row);
            if (PlainList || !(IsOpen(entry.Id) || partial)) continue;
            for (int m = 0; m < entry.Moves.Count; m++)
            {
                bool off = undone || m >= entry.Moves.Count - (partial ? Moves.Inner : 0);
                Label move = ChromeWidgets.Text("    " + entry.Moves[m], "CodeLabel", true); move.SetMeta("move_of", entry.Id);
                if (off) move.AddThemeColorOverride("font_color", t.C("text.off"));
                List.AddChild(move);
            }
        }
        if (Moves.Cursor == 0 && Moves.Entries.Count > 0) List.AddChild(CursorLine(t));
        Control start = Row(t, false, false, null, "Start of the session", "", null, () => Send("session.rollback", "0")); List.AddChild(start);
        foreach (HistoryBranch branch in Moves.Branches)
        {
            var line = new HBoxContainer(); line.SetMeta("branch", branch.From);
            Label mark = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(mark, "history.branch", t.Px("icon.small")); mark.AddThemeColorOverride("font_color", t.C("state.stale")); line.AddChild(mark);
            Label text = ChromeWidgets.Text($"kept branch after {branch.From}: {string.Join(", ", branch.Entries.Select(e => e.Name))}", "DimLabel", true);
            text.SizeFlagsHorizontal = SizeFlags.ExpandFill; text.TooltipText = "Abandoned redo steps stay recoverable; nothing is rewritten."; line.AddChild(text);
            List.AddChild(line);
        }
    }

    private static Control CursorLine(TokenSet t)
    {
        var line = new HBoxContainer { Name = "RollbackCursor" };
        var rule = new ColorRect { Color = t.C("accent.base"), CustomMinimumSize = new Vector2(0, t.Px("focus.width")), SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsVertical = SizeFlags.ShrinkCenter };
        Label label = ChromeWidgets.Text("committed state", "MicroLabel"); label.AddThemeColorOverride("font_color", t.C("accent.strong"));
        line.AddChild(label); line.AddChild(rule);
        return line;
    }

    private static Control Row(TokenSet t, bool selected, bool dim, string? origin, string name, string detail, Action? toggle, Action roll, bool opened = false)
    {
        var holder = new PanelContainer { ThemeTypeVariation = selected ? "RowSelected" : "Row", CustomMinimumSize = new Vector2(0, t.Px("row.height")) };
        var line = new HBoxContainer(); holder.AddChild(line);
        if (toggle != null)
        {
            Button fold = SessionWidgets.Tool(opened ? "fold.open" : "fold.closed", opened ? "Hide the moves" : "Show the moves", t, toggle); line.AddChild(fold);
        }
        if (origin != null) line.AddChild(SessionWidgets.Chip(origin, t.C(origin == "hand" ? "text.dim" : origin == "macro" ? "accent.strong" : "state.preview"), "Origin: " + origin));
        Label label = ChromeWidgets.Text(name, dim ? "DimLabel" : "", true); label.SizeFlagsHorizontal = SizeFlags.ExpandFill; label.TooltipText = name; line.AddChild(label);
        if (detail.Length > 0) line.AddChild(ChromeWidgets.Text(detail, "MicroLabel"));
        Button here = SessionWidgets.Tool("history.roll", "Roll to here", t, roll); here.SetMeta("roll", true); line.AddChild(here);
        return holder;
    }
}
