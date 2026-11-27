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

    // ADR-D01.70: one effect signature, drawn with the Library rows' marks and its facts in the tooltip; `owner`
    // names whose signature it is (the preview, or a History entry's id).
    public static SignatureMarks Signature(TokenSet tokens, EffectSignature signature, string owner)
    {
        var marks = new SignatureMarks(signature, tokens) { TooltipText = LibraryModel.SignatureText(signature) };
        marks.SetMeta("slot", "signature.slot"); marks.SetMeta("signature", owner);
        return marks;
    }
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
// and on protection. The previewed state itself is drawn by the core into Stage; the comparison above it sets
// before beside after, and one line names the range the preview touches. Nothing here commits.
public sealed partial class PreviewPanel : VBoxContainer
{
    public PreviewModel Model { get; }
    public Control Stage { get; } = new PanelContainer { ThemeTypeVariation = "ViewFrame", SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsVertical = SizeFlags.ExpandFill };
    public VBoxContainer Moves { get; } = new() { Name = "PreviewMoves" };
    public VBoxContainer Effects { get; } = new() { Name = "PreviewEffects" };
    public PreviewCompare Compare { get; }
    public Label Title { get; } = ChromeWidgets.Text("", "HeadingLabel");
    private readonly Func<TokenSet> tokens;
    private readonly HBoxContainer head = new();
    private readonly HSplitContainer body = new() { SizeFlagsVertical = SizeFlags.ExpandFill };
    private readonly Label stageNote = ChromeWidgets.Text("", "DimLabel");
    // Every action the view offers is a command of the registry (ADR-D01.17): the side of the comparison and the
    // move steps. A click on a move line is direct manipulation of the scrub.
    public event Action<string, string?>? Command;
    // ADR-D01.70: the rail order the effect signature is laid out in (the Library's), or null where no rail is shown.
    public Func<IReadOnlyList<int>?>? RailOrder { get; set; }

    public PreviewPanel(PreviewModel model, Func<TokenSet> tokens)
    {
        Model = model; this.tokens = tokens; Name = "PreviewPanel";
        AddChild(head); AddChild(new HSeparator()); AddChild(body);
        var left = new ScrollContainer { CustomMinimumSize = new Vector2(120, 0), SizeFlagsHorizontal = SizeFlags.ExpandFill, HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };
        Moves.SizeFlagsHorizontal = SizeFlags.ExpandFill; Moves.AddThemeConstantOverride("separation", 0); left.AddChild(Moves);
        var right = new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsStretchRatio = 1.6f };
        stageNote.AutowrapMode = TextServer.AutowrapMode.WordSmart; stageNote.HorizontalAlignment = HorizontalAlignment.Center; stageNote.VerticalAlignment = VerticalAlignment.Center;
        Compare = new PreviewCompare(model, tokens); var stageBox = new VBoxContainer(); stageBox.AddChild(Compare); stageBox.AddChild(stageNote);
        Stage.AddChild(stageBox); right.AddChild(Stage); right.AddChild(Effects);
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
        Compare.Visible = p is { Status: PreviewStatus.Ready };
        if (p == null)
        {
            stageNote.Text = "no preview staged";
            return;
        }
        (string text, string colour) = SessionWidgets.Status[p.Status];
        head.AddChild(SessionWidgets.Chip(text, t.C(colour), p.Reason ?? ""));
        if (Model.Replaced) head.AddChild(SessionWidgets.Chip("replaced the earlier preview", t.C("state.stale"), "The earlier preview can no longer be committed."));
        head.AddChild(new Control { SizeFlagsHorizontal = SizeFlags.ExpandFill });
        if (p.Status == PreviewStatus.Ready)
        {
            Button side = ChromeWidgets.Tool(Model.Side.ToString(), "What the stage compares: both, before or after. Press for the next."); side.Name = "PreviewSide";
            side.Pressed += () => Safe.Raise(Command, "preview.side.next", null); head.AddChild(side);
        }
        head.AddChild(ChromeWidgets.Text($"{Model.Scrub} of {p.Moves.Count}", "DimLabel"));
        head.AddChild(SessionWidgets.Tool("step.back", "One move back (Left)", t, () => Safe.Raise(Command, "view.preview.scrub", "back")));
        head.AddChild(SessionWidgets.Tool("step.forward", "One move forward (Right)", t, () => Safe.Raise(Command, "view.preview.scrub", "forward")));
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
        if (Model.Range() is { } range && p.Status == PreviewStatus.Ready)
        {
            Label touched = ChromeWidgets.Text($"{range.Orbits} of {range.OrbitsTotal} orbits, {range.Moved:N0} of {range.Total:N0} pieces", "CodeLabel"); touched.Name = "PreviewRange"; touched.AutowrapMode = TextServer.AutowrapMode.WordSmart;
            touched.TooltipText = "The range this preview touches over its whole sequence"; touched.MouseFilter = MouseFilterEnum.Pass; Effects.AddChild(touched);
        }
        Effects.AddChild(ChromeWidgets.Text("EFFECT ON EVERY ORBIT", "MicroLabel"));
        // ADR-D01.70: the preview's complete effect as the one effect signature, in the rail's order; a preview that
        // was not evaluated reads every orbit unknown, never untouched.
        if (RailOrder?.Invoke() is { } order) Effects.AddChild(SessionWidgets.Signature(t, LibraryModel.Signature(p.Effects, order), "preview"));
        // Only evaluated effects may say an orbit is untouched: a preview refused before anything was prepared
        // says that nothing was evaluated, and lists no orbit.
        if (!p.Evaluated)
        {
            Label none = ChromeWidgets.Text("Not evaluated: " + (p.Reason ?? "no effect was computed"), "DimLabel", true); none.Name = "EffectsNotEvaluated";
            none.AutowrapMode = TextServer.AutowrapMode.WordSmart; Effects.AddChild(none);
        }
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
    // ADR-D01.70: the rail order of the entries' effect signatures (the Library's), or null where no rail is shown.
    public Func<IReadOnlyList<int>?>? RailOrder { get; set; }
    public bool Rewards { get; set; } = true;
    public event Action<string, string?>? Command;
    private readonly Func<TokenSet> tokens;
    private readonly HBoxContainer head = new();
    private readonly HashSet<string> open = new(StringComparer.Ordinal);
    private CompletionLog? completions;
    public CompletionLog? Completions
    {
        get => completions;
        set
        {
            if (completions == value) return;
            if (completions != null) completions.Changed -= Refresh;
            completions = value;
            if (completions != null) completions.Changed += Refresh;
            Refresh();
        }
    }
    // The confirm gate of a branch switch is the move history's own (ADR-D01.16): the branch being asked about,
    // or -1. The panel only shows the question and sends the answer.
    public int Asking => Moves.Asking;
    public void Ask(int branch) => Moves.Ask(branch);

    public HistoryPanel(MoveHistory moves, LayoutHistory layout, Func<TokenSet> tokens)
    {
        Moves = moves; Layout = layout; this.tokens = tokens; Name = "HistoryPanel";
        AddChild(head); AddChild(new HSeparator());
        var scroll = new ScrollContainer { SizeFlagsVertical = SizeFlags.ExpandFill, HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };
        List.AddThemeConstantOverride("separation", 0); scroll.AddChild(List); AddChild(scroll);
        moves.Changed += Refresh; moves.Asked += Refresh; layout.Changed += Refresh; Refresh();
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
            tab.Pressed += () => Send("history.show", layout ? "layout" : "moves"); head.AddChild(tab);
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
        ILookup<string?, CompletionEntry>? completedAfter = null;
        if (Completions is { Entries.Count: > 0 } log)
        {
            var moveIds = Moves.Entries.Select(entry => entry.Id).ToHashSet(StringComparer.Ordinal);
            // Keep entries whose anchor is on a kept branch visible at the bottom of the move list.
            completedAfter = log.Entries.Reverse().ToLookup(e => e.AfterEntry is { } id && moveIds.Contains(id) ? id : null);
        }
        IReadOnlyList<int>? order = RailOrder?.Invoke();
        // Newest first; the rollback cursor sits between the applied entries and the undone ones.
        for (int i = Moves.Entries.Count - 1; i >= 0; i--)
        {
            HistoryEntry entry = Moves.Entries[i]; int index = i; bool undone = i >= Moves.Cursor;
            if (i == Moves.Cursor - 1) List.AddChild(CursorLine(t));
            if (completedAfter != null)
                foreach (CompletionEntry completion in completedAfter[entry.Id]) List.AddChild(CompletionRow(t, completion));
            bool partial = i == Moves.Cursor - 1 && Moves.Inner > 0;
            Control row = Row(t, false, undone, entry.Origin, entry.Name, $"{entry.Id}  {entry.Time}", PlainList ? null : () => Send("history.entry.toggle", entry.Id),
                () => Send("session.rollback", (index + 1).ToString(System.Globalization.CultureInfo.InvariantCulture)), IsOpen(entry.Id),
                order == null ? null : SessionWidgets.Signature(t, LibraryModel.Signature(entry.Moves, order), entry.Id));
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
        if (completedAfter != null)
            foreach (CompletionEntry completion in completedAfter[null]) List.AddChild(CompletionRow(t, completion));
        Control start = Row(t, false, false, null, "Start of the session", "", null, () => Send("session.rollback", "0")); List.AddChild(start);
        for (int b = 0; b < Moves.Branches.Count; b++)
        {
            HistoryBranch branch = Moves.Branches[b]; int at = b;
            var line = new HBoxContainer(); line.SetMeta("branch", branch.From);
            Label mark = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(mark, "history.branch", t.Px("icon.small")); mark.AddThemeColorOverride("font_color", t.C("state.stale")); line.AddChild(mark);
            Label text = ChromeWidgets.Text($"branch after {branch.From} \u00b7 {branch.Entries.Count} {(branch.Entries.Count == 1 ? "entry" : "entries")}, {branch.Entries.Sum(e => e.Moves.Count)} moves: {string.Join(", ", branch.Entries.Select(e => e.Name))}", "DimLabel", true);
            text.SizeFlagsHorizontal = SizeFlags.ExpandFill; text.TooltipText = "Abandoned redo steps stay recoverable; nothing is rewritten."; line.AddChild(text);
            var tools = new HBoxContainer { Alignment = BoxContainer.AlignmentMode.End }; tools.SetMeta("branch_tools", at);
            if (Asking == at)
            {
                Label ask = ChromeWidgets.Text("Switch lines?", "AccentLabel"); tools.AddChild(ask);
                Button yes = ChromeWidgets.Tool("Confirm", "Make this branch the line. The later entries of the current line are kept as a branch."); yes.Name = "BranchConfirm";
                yes.Pressed += () => Send("session.branch.select", "confirm " + at.ToString(System.Globalization.CultureInfo.InvariantCulture)); tools.AddChild(yes);
                Button no = ChromeWidgets.Tool("Cancel", "Keep the current line"); no.Name = "BranchCancel"; no.Pressed += () => Send("session.branch.select", "cancel"); tools.AddChild(no);
            }
            else
            {
                Button replay = ChromeWidgets.Tool("Replay", "Bring this branch's moves back as a preview you commit; the replay itself never commits"); replay.Name = "BranchReplay";
                replay.Pressed += () => Send("session.branch.replay", at.ToString(System.Globalization.CultureInfo.InvariantCulture)); tools.AddChild(replay);
                string? why = Moves.SelectRefusal(at);
                Button swap = ChromeWidgets.Tool("Switch", why == null ? "Switch to this branch; it asks first" : "Not now: " + why); swap.Name = "BranchSwitch"; swap.Disabled = why != null;
                swap.Pressed += () => Send("session.branch.select", at.ToString(System.Globalization.CultureInfo.InvariantCulture)); tools.AddChild(swap);
            }
            List.AddChild(line); List.AddChild(tools);
        }
    }

    private Control CompletionRow(TokenSet t, CompletionEntry entry)
    {
        var row = new PanelContainer { ThemeTypeVariation = "Row", CustomMinimumSize = new Vector2(0, t.Px("row.height")) };
        row.SetMeta("completion", entry.Name);
        row.SetMeta("slot", "history.entry.completion");
        var line = new HBoxContainer(); row.AddChild(line);
        string text = CompletionLog.Line(entry) + (entry.Sample ? " · sample" : "");
        Button read = ChromeWidgets.Tool(text, text + "\nRead recorded completion");
        read.Alignment = HorizontalAlignment.Left;
        read.ClipText = true;
        read.Flat = true;
        read.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        read.Pressed += () => Send("session.completion", entry.Name);
        line.AddChild(read);
        if (entry.Moment && MotionSettings.Shows(MotionUses.OrbitComplete, Rewards))
        {
            var moment = new Label
            {
                Name = "MomentSlot",
                CustomMinimumSize = new Vector2(t.Px("icon.small"), 0),
                TooltipText = "The first-completed-orbit moment",
                MouseFilter = MouseFilterEnum.Pass,
            };
            ChromeIcons.Apply(moment, "task.ok", t.Px("icon.small"));
            moment.AddThemeColorOverride("font_color", t.C("rail.cell.solved"));
            moment.SetMeta("moment", MotionUses.OrbitComplete);
            moment.SetMeta("cue", "cue.orbit");
            line.AddChild(moment);
        }
        return row;
    }

    private static Control CursorLine(TokenSet t)
    {
        var line = new HBoxContainer { Name = "RollbackCursor" };
        var rule = new ColorRect { Color = t.C("accent.base"), CustomMinimumSize = new Vector2(0, t.Px("focus.width")), SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsVertical = SizeFlags.ShrinkCenter };
        Label label = ChromeWidgets.Text("committed state", "MicroLabel"); label.AddThemeColorOverride("font_color", t.C("accent.strong"));
        line.AddChild(label); line.AddChild(rule);
        return line;
    }

    private static Control Row(TokenSet t, bool selected, bool dim, string? origin, string name, string detail, Action? toggle, Action roll, bool opened = false, Control? signature = null)
    {
        var holder = new PanelContainer { ThemeTypeVariation = selected ? "RowSelected" : "Row", CustomMinimumSize = new Vector2(0, t.Px("row.height")) };
        var line = new HBoxContainer(); holder.AddChild(line);
        if (toggle != null)
        {
            Button fold = SessionWidgets.Tool(opened ? "fold.open" : "fold.closed", opened ? "Hide the moves" : "Show the moves", t, toggle); line.AddChild(fold);
        }
        if (origin != null) line.AddChild(SessionWidgets.Chip(origin, t.C(origin == "hand" ? "text.dim" : origin == "macro" ? "accent.strong" : "state.preview"), "Origin: " + origin));
        if (signature != null) line.AddChild(signature);
        Label label = ChromeWidgets.Text(name, dim ? "DimLabel" : "", true); label.SizeFlagsHorizontal = SizeFlags.ExpandFill; label.TooltipText = name; line.AddChild(label);
        if (detail.Length > 0) line.AddChild(ChromeWidgets.Text(detail, "MicroLabel"));
        Button here = SessionWidgets.Tool("history.roll", "Roll to here", t, roll); here.SetMeta("roll", true); line.AddChild(here);
        return holder;
    }
}
