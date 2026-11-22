using System;
using Godot;

namespace Magic600.Chrome;

// Holds a dock panel's content and shows the panel's state (ADR-D01.33): empty, loading and error take the
// content's place with a word, a glyph and the reason; stale and busy keep the content under a banner. Nothing
// here moves: a loading or busy panel has no spinner.
public sealed partial class PanelStateBox : VBoxContainer
{
    public string Id { get; }
    public Control Content { get; }
    public PanelCondition Condition => states.Of(Id).Condition;
    private readonly PanelStates states; private readonly Func<TokenSet> tokens;
    private readonly PanelContainer banner = new() { Name = "PanelStateBanner", ThemeTypeVariation = "BarBottom", Visible = false };
    private readonly HBoxContainer row = new(); private readonly Control glyph; private readonly Label word, text; private readonly Button act;
    private readonly CenterContainer place = new() { Name = "PanelStatePlace", Visible = false, SizeFlagsVertical = SizeFlags.ExpandFill };
    private readonly Label placeText = new() { ThemeTypeVariation = "DimLabel", HorizontalAlignment = HorizontalAlignment.Center, AutowrapMode = TextServer.AutowrapMode.WordSmart };

    public PanelStateBox(string id, Control content, PanelStates states, Func<TokenSet> tokens)
    {
        Id = id; Content = content; this.states = states; this.tokens = tokens; Name = "PanelState_" + id;
        SizeFlagsVertical = SizeFlags.ExpandFill; SizeFlagsHorizontal = SizeFlags.ExpandFill; AddThemeConstantOverride("separation", 0);
        glyph = new Control { CustomMinimumSize = new Vector2(18, 18), SizeFlagsVertical = SizeFlags.ShrinkCenter }; glyph.Draw += DrawGlyph;
        word = ChromeWidgets.Text("", "AccentLabel"); text = ChromeWidgets.Text("", "DimLabel", true); text.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        act = new Button { ThemeTypeVariation = "ToolButton", FocusMode = FocusModeEnum.None, Name = "PanelStateAction", Visible = false };
        act.Pressed += () => { if (PanelStates.Action(Condition) is { } action) states.Ask(Id, action); };
        banner.AddChild(row); row.AddChild(glyph); row.AddChild(word); row.AddChild(text); row.AddChild(act);
        placeText.CustomMinimumSize = new Vector2(160, 0); place.AddChild(placeText);
        ChromeWidgets.Detach(content); AddChild(banner); AddChild(place); AddChild(content);
        states.Register(id); states.Changed += () => { if (IsInstanceValid(this)) Refresh(); };
        Refresh();
    }

    public void Refresh()
    {
        PanelState state = states.Of(Id); bool ready = state.Condition == PanelCondition.Ready, hides = PanelStates.HidesContent(state.Condition);
        banner.Visible = !ready; place.Visible = hides;
        if (IsInstanceValid(Content)) { Content.Visible = !hides; Content.Modulate = Content.Modulate with { A = state.Condition == PanelCondition.Stale ? .55f : 1 }; }
        if (ready) return;
        word.Text = PanelStates.Word(state.Condition); text.Text = state.Text; text.TooltipText = state.Text; placeText.Text = state.Text;
        string? action = PanelStates.Action(state.Condition); act.Visible = action != null;
        if (action != null) { act.Text = char.ToUpperInvariant(action[0]) + action.Substring(1); act.TooltipText = action == "retry" ? "Try again" : "Bring the panel up to the committed state"; }
        glyph.QueueRedraw();
    }

    // One glyph for each state, told apart by shape: a ring, three dots, a triangle, a broken ring, a filled lamp.
    private void DrawGlyph()
    {
        TokenSet t = tokens(); Vector2 c = glyph.Size / 2; float r = Math.Min(glyph.Size.X, glyph.Size.Y) * .34f;
        switch (Condition)
        {
            case PanelCondition.Empty: glyph.DrawArc(c, r, 0, Mathf.Tau, 20, t.C("text.dim"), 1.5f, true); break;
            case PanelCondition.Loading: for (int i = -1; i <= 1; i++) glyph.DrawCircle(c + new Vector2(i * r * .9f, 0), 1.6f, t.C("text.dim")); break;
            case PanelCondition.Error:
                Vector2 a = c + new Vector2(0, -r), b = c + new Vector2(r, r * .8f), d = c + new Vector2(-r, r * .8f); Color bad = t.C("state.invalid");
                glyph.DrawLine(a, b, bad, 1.5f, true); glyph.DrawLine(b, d, bad, 1.5f, true); glyph.DrawLine(d, a, bad, 1.5f, true); break;
            case PanelCondition.Stale: for (int i = 0; i < 4; i++) glyph.DrawArc(c, r, i * Mathf.Tau / 4, i * Mathf.Tau / 4 + Mathf.Tau / 7, 6, t.C("state.stale"), 1.5f, true); break;
            case PanelCondition.Busy: glyph.DrawCircle(c, r * .7f, t.C("commit.status.settling")); break;
        }
    }
}
