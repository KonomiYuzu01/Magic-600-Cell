using System;
using Godot;

namespace Magic600.Chrome;

internal static class SessionCellWidgets
{
    public static void Lamp(Label label, string slot, bool lit, string tip, TokenSet tokens)
    {
        label.Text = lit ? "●" : "○";
        label.TooltipText = tip;
        label.MouseFilter = Control.MouseFilterEnum.Pass;
        label.SetMeta("slot", slot);
        label.AddThemeColorOverride("font_color", tokens.C(lit ? "lamp.active" : "lamp.idle"));
    }
}

public sealed partial class TimerCell : PanelContainer
{
    private readonly SessionTimer timer;
    private readonly Func<TokenSet> tokens;
    private readonly Func<double> now;
    private readonly Label time = ChromeWidgets.Text("", "CodeLabel");
    private readonly Label lamp = ChromeWidgets.Text("", "MicroLabel");
    private readonly Button key = ChromeWidgets.Tool("", SessionTimer.Certifies);
    private long shownSecond = -1;
    public event Action<string>? Command;

    public TimerCell(SessionTimer timer, Func<TokenSet> tokens, Func<double> now)
    {
        this.timer = timer;
        this.tokens = tokens;
        this.now = now;
        Name = "TimerCell";
        ThemeTypeVariation = "Chip";
        TooltipText = SessionTimer.Certifies;
        SetMeta("slot", "cell.timer");
        var line = new HBoxContainer { MouseFilter = MouseFilterEnum.Pass };
        AddChild(line);
        line.AddChild(time);
        line.AddChild(lamp);
        key.Name = "TimerKey";
        key.FocusMode = FocusModeEnum.All;
        key.Pressed += () => Safe.Raise(Command, timer.Row);
        line.AddChild(key);
        Refresh();
    }

    public override void _EnterTree()
    {
        timer.Changed += Refresh;
        Refresh();
    }

    public override void _ExitTree() => timer.Changed -= Refresh;
    public override void _Process(double delta) => RefreshTime();

    public void Refresh()
    {
        bool running = timer.Running;
        string word = running ? "running" : "paused";
        SessionCellWidgets.Lamp(lamp, "lamp.running", running, $"Timer is {word}. {SessionTimer.Certifies}", tokens());
        key.Text = running ? "Pause" : "Start";
        key.TooltipText = key.Text + " the timer. " + SessionTimer.Certifies;
        RefreshTime();
        SetProcess(running);
    }

    private void RefreshTime()
    {
        double seconds = timer.Elapsed(now());
        long whole = (long)seconds;
        if (shownSecond == whole) return;
        shownSecond = whole;
        time.Text = SessionTimer.Text(seconds);
    }
}

public sealed partial class SessionCell : PanelContainer
{
    private readonly SessionCellModel model;
    private readonly Func<TokenSet> tokens;

    public SessionCell(SessionCellModel model, Func<TokenSet> tokens)
    {
        this.model = model;
        this.tokens = tokens;
        Name = "SessionCell";
        ThemeTypeVariation = "Chip";
        FocusMode = FocusModeEnum.All;
        SetMeta("slot", "cell.session");
        Refresh();
    }

    public override void _EnterTree()
    {
        model.Changed += Refresh;
        Refresh();
    }

    public override void _ExitTree() => model.Changed -= Refresh;

    public override void _GuiInput(InputEvent e)
    {
        if (e is InputEventMouseButton { ButtonIndex: MouseButton.Left, Pressed: true } || e.IsActionPressed("ui_accept"))
        {
            AcceptEvent();
            model.Toggle();
        }
    }

    public void Refresh()
    {
        SessionWidgets.Clear(this);
        TokenSet t = tokens();
        SizeFlagsHorizontal = model.Expanded ? SizeFlags.ExpandFill : SizeFlags.Fill;
        TooltipText = model.Expanded ? "Hide session details" : "Show session details";
        if (!model.Expanded)
        {
            var lamps = new HBoxContainer { MouseFilter = MouseFilterEnum.Pass };
            foreach (SessionLamp lamp in model.Lamps)
            {
                Label label = ChromeWidgets.Text("", "MicroLabel");
                SessionCellWidgets.Lamp(label, lamp.Slot, lamp.Lit, lamp.Tip, t);
                lamps.AddChild(label);
            }
            AddChild(lamps);
            return;
        }

        var facts = new GridContainer { Columns = 3, MouseFilter = MouseFilterEnum.Pass };
        foreach (SessionFact fact in model.Facts())
        {
            var pair = new HBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill, MouseFilter = MouseFilterEnum.Pass };
            pair.AddChild(ChromeWidgets.Text(fact.Name, "MicroLabel"));
            Label value = ChromeWidgets.Text(fact.Value, fact.Waiting ? "DimLabel" : "MicroLabel", clip: !fact.Waiting);
            value.CustomMinimumSize = new Vector2(t.Px("row.height"), 0);
            pair.AddChild(value);
            facts.AddChild(pair);
        }
        AddChild(facts);
    }
}
