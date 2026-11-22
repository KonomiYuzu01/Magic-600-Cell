using System;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The draft line (ADR-D01.66): the route switch on the left cap, the three phases with the active one marked, the
// Goal slot with its own result, the check-mode chip on the right cap and the settling lamp. It draws no commit
// control: those are the host's, on the commit bar under it.
public sealed partial class DraftLine : PanelContainer
{
    public OperationDraft Draft { get; }
    private readonly Func<TokenSet> tokens; private readonly HBoxContainer row = new(); private double settleUntil; private Control? lamp;
    public event Action<string, string?>? Command;
    public Button RouteDraft { get; private set; } = null!;
    public Button RouteDirect { get; private set; } = null!;
    public Button GoalSlot { get; private set; } = null!;
    public Button CheckChip { get; private set; } = null!;
    public bool Settling => Time.GetTicksMsec() / 1000.0 < settleUntil;
    public string PhaseText(int phase) => Draft.Twists(phase).Count == 0 ? "empty" : string.Join("; ", Draft.Twists(phase));

    public DraftLine(OperationDraft draft, Func<TokenSet> tokens)
    {
        Draft = draft; this.tokens = tokens; Name = "DraftLine"; ThemeTypeVariation = "BarBottom"; AddChild(row);
        draft.Changed += () => { if (IsInstanceValid(this)) Refresh(); };
        Refresh();
    }

    // The settle interval after a change of the draft: the lamp is lit while it runs.
    public void Settle(double seconds) { settleUntil = Time.GetTicksMsec() / 1000.0 + seconds; lamp?.QueueRedraw(); SetProcess(true); }
    public override void _Process(double delta) { if (!Settling) { SetProcess(false); lamp?.QueueRedraw(); } }

    public void Refresh()
    {
        TokenSet t = tokens(); foreach (Node child in row.GetChildren()) { row.RemoveChild(child); child.QueueFree(); }
        CustomMinimumSize = new Vector2(0, t.Px("bar.height")); row.AddThemeConstantOverride("separation", t.Px("space.2"));
        bool draft = Draft.Route == InputRoute.Draft;
        Button Route(string word, bool on, string tip)
        {
            var b = new Button { Text = word, ToggleMode = true, ButtonPressed = on, ThemeTypeVariation = "ToolButton", TooltipText = tip, FocusMode = FocusModeEnum.None, Name = "Route" + word };
            b.AddThemeColorOverride("font_color", t.C(on ? (word == "Draft" ? "mark.route.draft" : "mark.route.direct") : "text.dim"));
            b.AddThemeColorOverride("font_pressed_color", t.C(word == "Draft" ? "mark.route.draft" : "mark.route.direct"));
            b.Pressed += () => Safe.Raise(Command, "input.destination.set", word.ToLowerInvariant()); row.AddChild(b); return b;
        }
        row.AddChild(new Control { CustomMinimumSize = new Vector2(t.Px("space.2"), 0) });
        var mark = new Control { CustomMinimumSize = new Vector2(t.Px("icon.small"), 0), TooltipText = "The route mark: the grip marker in Local has the same pattern" };
        mark.Draw += () => RouteMark.Draw(mark, new Rect2(Vector2.Zero, mark.Size).Grow(-3), draft, t.C(draft ? "mark.route.draft" : "mark.route.direct"));
        row.AddChild(mark);
        RouteDraft = Route("Draft", draft, "Draft: grip and twist keys append to the active phase and the preview rebuilds");
        RouteDirect = Route("Direct", !draft, "Direct: a twist is one complete operation");
        row.AddChild(new VSeparator());
        for (int i = 0; i < OperationDraft.Phases.Count; i++)
        {
            int phase = i; string name = OperationDraft.Phases[i]; bool active = draft && Draft.Active == i;
            var cell = new Button { Name = "Phase" + name, ThemeTypeVariation = "ToolButton", ToggleMode = true, ButtonPressed = active, ClipText = true, Alignment = HorizontalAlignment.Left, FocusMode = FocusModeEnum.None,
                SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsStretchRatio = i == 1 ? 1.4f : 1, Disabled = !draft,
                Text = (active ? "\u25b8 " : "") + name + (draft ? "  " + PhaseText(i) : ""), TooltipText = draft ? $"{name}: {PhaseText(i)}. Click to make it the active phase." : "Direct: twists do not join a phase" };
            cell.AddThemeColorOverride("font_color", t.C("draft.phase." + name.ToLowerInvariant())); cell.AddThemeColorOverride("font_pressed_color", t.C("accent.strong"));
            cell.Pressed += () => Safe.Raise(Command, "operation.phase.select", name);
            row.AddChild(cell);
        }
        row.AddChild(new VSeparator());
        // The Goal has its own slot and its own result, apart from the commit status.
        GoalResult goal = Draft.GoalResult; string result = goal.ToString().ToLowerInvariant();
        GoalSlot = new Button { Name = "GoalSlot", ThemeTypeVariation = "ToolButton", FocusMode = FocusModeEnum.None,
            Text = Draft.Goal == null ? "Goal: unset" : $"Goal: {Draft.Goal} \u00b7 {result}",
            TooltipText = "The Goal is a predicate you declare. Its result is its own and never gates a commit. Here the result is demo data." };
        GoalSlot.AddThemeColorOverride("font_color", t.C("commit.goal." + result)); GoalSlot.Pressed += () => Safe.Raise(Command, "intent.set", null); row.AddChild(GoalSlot);
        CheckResult check = Draft.Checked; string mode = Draft.Mode.ToString(), said = check.ToString().ToLowerInvariant();
        CheckChip = new Button { Name = "CheckChip", ThemeTypeVariation = "ToolButton", FocusMode = FocusModeEnum.None, Text = $"{mode} \u00b7 {said}",
            TooltipText = $"Check mode. Net: {Draft.Result(CheckMode.Net).ToString().ToLowerInvariant()}. Strict: {Draft.Result(CheckMode.Strict).ToString().ToLowerInvariant()}. A word can pass Net and fail Strict. Click to change the mode. Demo data." };
        CheckChip.AddThemeColorOverride("font_color", t.C(check == CheckResult.Failed ? "state.invalid" : check == CheckResult.Unchecked ? "text.dim" : "chip.check_mode." + mode.ToLowerInvariant()));
        CheckChip.Pressed += () => Draft.SetMode(Draft.Mode == CheckMode.Net ? CheckMode.Strict : CheckMode.Net); row.AddChild(CheckChip);
        lamp = new Control { Name = "SettlingLamp", CustomMinimumSize = new Vector2(t.Px("icon.small"), 0), TooltipText = "Settling lamp: lit while the settle interval runs after a change" };
        Control mine = lamp;
        mine.Draw += () =>
        {
            Vector2 c = mine.Size / 2; float r = Math.Min(mine.Size.X, mine.Size.Y) * .22f;
            if (Settling) mine.DrawCircle(c, r, tokens().C("commit.status.settling")); else mine.DrawArc(c, r, 0, Mathf.Tau, 16, tokens().C("panel.border"), 1, true);
        };
        row.AddChild(lamp); row.AddChild(new Control { CustomMinimumSize = new Vector2(t.Px("space.2"), 0) });
    }
}

// The route mark, one drawing for the draft line's cap and the grip marker: broken for Draft, whole for Direct.
public static class RouteMark
{
    public static void Draw(CanvasItem on, Rect2 box, bool draft, Color colour, float width = 1.5f)
    {
        Vector2 a = box.Position, b = new(box.End.X, box.Position.Y), c = box.End, d = new(box.Position.X, box.End.Y);
        foreach ((Vector2 from, Vector2 to) in new[] { (a, b), (b, c), (c, d), (d, a) })
            if (draft) on.DrawDashedLine(from, to, colour, width, Math.Max(2, box.Size.X / 5)); else on.DrawLine(from, to, colour, width, true);
    }
}
