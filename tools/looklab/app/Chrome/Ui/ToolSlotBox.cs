using System;
using Godot;

namespace Magic600.Chrome;

// One tool slot drawn (ADR-D02.7, .40): its display name, its state's glyph and its word. Each of the seven states
// has one glyph told apart by shape and none from the ring family: a dash, an hourglass, a left bar, a dashed
// frame, a stop square, a stop bar at the line's end and a cross. A result appears in place: nothing here moves.
// A tool with words of its own (the setup slot, ADR-D01.68) passes them instead of the Jumble slot words.
public sealed partial class ToolSlotBox : HBoxContainer
{
    public ToolSlot Slot { get; }
    private readonly SlotWords? words; private readonly Func<(string Text, string Mark)>? own; private readonly Func<TokenSet> tokens;
    private readonly Control glyph; private readonly Label name, word;

    public ToolSlotBox(ToolSlot slot, string displayName, SlotWords? words, Func<TokenSet> tokens, Func<(string Text, string Mark)>? own = null)
    {
        if (words == null && own == null) throw new ArgumentException("a slot box needs its words");
        Slot = slot; this.words = words; this.own = own; this.tokens = tokens; Name = "ToolSlot_" + slot.Id;
        SizeFlagsHorizontal = SizeFlags.ExpandFill;
        glyph = new Control { CustomMinimumSize = new Vector2(16, 16), SizeFlagsVertical = SizeFlags.ShrinkCenter }; glyph.Draw += DrawGlyph;
        name = ChromeWidgets.Text(displayName, "AccentLabel"); word = ChromeWidgets.Text("", "DimLabel", true);
        AddChild(glyph); AddChild(name); AddChild(word);
        slot.Changed += _ => { if (IsInstanceValid(this)) Refresh(); };
        Refresh();
    }

    public void Refresh()
    {
        (string text, string mark) = own?.Invoke() ?? Slot.Word(words!);
        word.Text = text; word.TooltipText = text + " (" + mark + ")";
        glyph.QueueRedraw();
    }

    private void DrawGlyph()
    {
        TokenSet t = tokens(); Color c = t.C(SlotWords.Mark(Slot.State)); Vector2 s = glyph.Size; float w = 1.5f;
        float l = s.X * .2f, r = s.X * .8f, top = s.Y * .2f, bottom = s.Y * .8f, mid = s.Y / 2;
        switch (Slot.State)
        {
            case SlotState.Empty:
                glyph.DrawLine(new Vector2(l, mid), new Vector2(r, mid), c, w, true); break;
            case SlotState.Pending:
                glyph.DrawPolyline(new[] { new Vector2(l, top), new Vector2(r, top), new Vector2(l, bottom), new Vector2(r, bottom), new Vector2(l, top) }, c, w, true); break;
            case SlotState.Ready:
                glyph.DrawRect(new Rect2(l, top, s.X * .18f, bottom - top), c); break;
            case SlotState.Stale:
                for (int i = 0; i < 4; i++)
                {
                    Vector2 a = i switch { 0 => new(l, top), 1 => new(r, top), 2 => new(r, bottom), _ => new(l, bottom) };
                    Vector2 b = i switch { 0 => new(r, top), 1 => new(r, bottom), 2 => new(l, bottom), _ => new(l, top) };
                    glyph.DrawDashedLine(a, b, c, w, 2.5f);
                }
                break;
            case SlotState.Refused:
                glyph.DrawRect(new Rect2(l + 1, top + 1, r - l - 2, bottom - top - 2), c); break;
            case SlotState.BudgetStopped:
                glyph.DrawLine(new Vector2(l, mid), new Vector2(r, mid), c, w, true);
                glyph.DrawLine(new Vector2(r, top), new Vector2(r, bottom), c, w * 1.5f, true); break;
            default:
                glyph.DrawLine(new Vector2(l, top), new Vector2(r, bottom), c, w, true);
                glyph.DrawLine(new Vector2(r, top), new Vector2(l, bottom), c, w, true); break;
        }
    }
}
