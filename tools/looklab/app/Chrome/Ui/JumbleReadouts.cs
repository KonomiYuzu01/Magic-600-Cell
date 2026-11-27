using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// A row of readout slots on a view's border (ADR-D02.22): each slot is its label, one neutral change mark and a
// tabular numeral in a slot of fixed width, so a numeral that changes never moves its neighbours. A slot that holds
// no value says its state's word (undefined, pending, incomplete) in that state's role; nothing is coloured by value
// and nothing is a gauge.
public sealed partial class ReadoutRow : HFlowContainer
{
    // The widest numeral a slot keeps room for: 259,800 slots and 600 caps fit in six digits.
    public const string WidestNumeral = "000000";
    public const string ChangeMark = "•";
    private readonly Func<TokenSet> tokens;

    public ReadoutRow(string name, Func<TokenSet> tokens)
    {
        Name = name; this.tokens = tokens; MouseFilter = MouseFilterEnum.Pass;
    }

    // SF3 (27 Nov 2026): what the row shows. The same slots and marks again change nothing, and slots with the same
    // labels and units change only their numerals and marks in place; a rebuild of both rows cost 5-6 ms, four times
    // a bind. A new token set (a recolour) or another shape rebuilds the row.
    private ReadoutSlot[] shownSlots = Array.Empty<ReadoutSlot>();
    private readonly HashSet<int> shownMarked = new();
    private TokenSet? shownTokens;

    // The slots, with the positions whose numeral changed at this revision marked (ReadoutChanges).
    public void Show(IReadOnlyList<ReadoutSlot> slots, IReadOnlySet<int> marked)
    {
        TokenSet t = tokens();
        if (ReferenceEquals(t, shownTokens) && SameShape(slots))
        {
            for (int i = 0; i < slots.Count; i++)
            {
                if (slots[i] == shownSlots[i] && marked.Contains(i) == shownMarked.Contains(i)) continue;
                Node group = GetChild(i);
                group.GetNode<Label>("Changed" + i).Text = marked.Contains(i) ? ChangeMark : "";
                Label numeral = group.GetNode<Label>("Slot" + i);
                numeral.Text = slots[i].Value.Word;
                numeral.AddThemeColorOverride("font_color", t.C(slots[i].Value.Role));
                numeral.TooltipText = slots[i].Label + ": " + slots[i].Value.Word;
            }
            Remember(slots, marked, t);
            return;
        }
        // The row wraps by whole slots, so a view's footer never widens the frame past the window.
        AddThemeConstantOverride("h_separation", t.Px("space.1")); AddThemeConstantOverride("v_separation", 0);
        foreach (Node child in GetChildren()) { RemoveChild(child); child.QueueFree(); }
        for (int i = 0; i < slots.Count; i++)
        {
            ReadoutSlot slot = slots[i];
            var group = new HBoxContainer { Name = "Readout" + i, MouseFilter = MouseFilterEnum.Pass }; group.AddThemeConstantOverride("separation", t.Px("space.1")); AddChild(group);
            Label label = ChromeWidgets.Text(slot.Label, "MicroLabel"); label.AddThemeColorOverride("font_color", t.C("text.dim")); group.AddChild(label);
            Label mark = ChromeWidgets.Text(marked.Contains(i) ? ChangeMark : "", "MicroLabel"); mark.Name = "Changed" + i;
            mark.AddThemeColorOverride("font_color", t.C("text.dim"));
            mark.CustomMinimumSize = new Vector2(Width(mark, ChangeMark), 0); group.AddChild(mark);
            Label numeral = ChromeWidgets.Text(slot.Value.Word, "ReadoutLabel"); numeral.Name = "Slot" + i;
            numeral.AddThemeColorOverride("font_color", t.C(slot.Value.Role));
            numeral.HorizontalAlignment = HorizontalAlignment.Right;
            numeral.CustomMinimumSize = new Vector2(Width(numeral, WidestNumeral), 0);
            numeral.TooltipText = slot.Label + ": " + slot.Value.Word;
            group.AddChild(numeral);
            if (slot.Unit is { } unit)
            {
                Label word = ChromeWidgets.Text(unit, "MicroLabel"); word.AddThemeColorOverride("font_color", t.C("text.dim")); group.AddChild(word);
            }
            if (i < slots.Count - 1) group.AddChild(new VSeparator());
        }
        Remember(slots, marked, t);
    }

    private bool SameShape(IReadOnlyList<ReadoutSlot> slots)
    {
        if (slots.Count != shownSlots.Length || GetChildCount() != slots.Count) return false;
        for (int i = 0; i < slots.Count; i++)
            if (slots[i].Label != shownSlots[i].Label || slots[i].Unit != shownSlots[i].Unit) return false;
        return true;
    }

    private void Remember(IReadOnlyList<ReadoutSlot> slots, IReadOnlySet<int> marked, TokenSet t)
    {
        shownSlots = slots.ToArray(); shownMarked.Clear(); shownMarked.UnionWith(marked); shownTokens = t;
    }

    internal static float Width(Label label, string text)
    {
        Font font = label.GetThemeFont("font");
        int size = label.GetThemeFontSize("font_size");
        return font == null ? 0 : font.GetStringSize(text, HorizontalAlignment.Left, -1, size).X;
    }
}
