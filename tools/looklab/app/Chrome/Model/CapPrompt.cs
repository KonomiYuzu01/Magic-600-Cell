using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace Magic600.Chrome;

// One row of the cap prompt: a cap, its words, and why naming it does nothing now (null when it names the cap).
public sealed record CapPromptRow(int Cap, string Label, string? Refusal);

// The argument prompt of jumble.cap.name without a number (owner's bug report, 26 November 2026: the row looked dead).
// It lists every cap in cap-id order with its class and N_c, and nothing is highlighted until the solver moves: the
// program never ranks or pre-selects a cap (the human-solve boundary). Typing narrows the list to the ids that start
// with the digits typed ("15" or "cap 15"); Enter names the highlighted cap, or else the cap typed exactly.
public sealed class CapPrompt
{
    // The rows the prompt shows at once, as the palette does.
    public const int Shown = 12;
    private readonly Func<CapMapModel> map;
    private readonly Func<int, string?> refusal;
    public string Query { get; private set; } = "";
    public IReadOnlyList<CapPromptRow> Rows { get; private set; } = Array.Empty<CapPromptRow>();
    // The highlighted row's index in Rows, or null: none until Up, Down or the pointer picks one.
    public int? Highlighted { get; private set; }
    // The index of the first row shown.
    public int First { get; private set; }
    // Why the last Enter named nothing, until the next key.
    public string? Said { get; private set; }

    public CapPrompt(Func<CapMapModel> map, Func<int, string?> refusal)
    {
        this.map = map; this.refusal = refusal;
        Open();
    }

    public void Open() => Type("");

    public void Type(string text)
    {
        Query = text; Highlighted = null; First = 0; Said = null;
        string? digits = Digits(text);
        CapMapModel m = map();
        Rows = digits == null ? Array.Empty<CapPromptRow>() : Enumerable.Range(0, CapLayout.Caps)
            .Where(c => c.ToString(CultureInfo.InvariantCulture).StartsWith(digits, StringComparison.Ordinal))
            .Select(c => new CapPromptRow(c, Label(m, c), refusal(c))).ToArray();
    }

    // The digits a query asks for: "", "15", "cap 15" or "cap15" (a start of "cap" asks for every cap); null when
    // the text cannot start a cap number.
    public static string? Digits(string text)
    {
        string t = text.Trim();
        if (t.Length > 0 && "cap".StartsWith(t, StringComparison.OrdinalIgnoreCase)) return "";
        if (t.StartsWith("cap", StringComparison.OrdinalIgnoreCase)) t = t[3..].Trim();
        return t.All(char.IsAsciiDigit) ? t : null;
    }

    // A row's words: the cap, its class and its N_c, in the words of the cap map's tooltip.
    public static string Label(CapMapModel map, int cap)
    {
        CapClass k = map.Class(cap);
        string nc = k switch
        {
            CapClass.Blocked => "N_c undefined",
            CapClass.Pending => "N_c pending",
            _ => "N_c " + map.Nc(cap)?.ToString(CultureInfo.InvariantCulture),
        };
        return "cap " + cap.ToString(CultureInfo.InvariantCulture) + " · " + CapMapModel.Word(k) + " · " + nc;
    }

    // Down from no highlight goes to the first row and Up to the last; the shown rows follow the highlight.
    public void Move(int by)
    {
        if (Rows.Count == 0 || by == 0) return;
        Highlight(Highlighted is int h ? Math.Clamp(h + by, 0, Rows.Count - 1) : by > 0 ? 0 : Rows.Count - 1);
    }

    public void Highlight(int index)
    {
        if (index < 0 || index >= Rows.Count) return;
        Highlighted = index; Said = null;
        if (index < First) First = index;
        else if (index >= First + Shown) First = index - Shown + 1;
    }

    // The wheel moves the shown rows and leaves the highlight where it is.
    public void Scroll(int by) => First = Math.Clamp(First + by, 0, Math.Max(0, Rows.Count - Shown));

    public IReadOnlyList<CapPromptRow> Window => Rows.Skip(First).Take(Shown).ToArray();

    // Enter: the highlighted cap, or else the cap typed exactly; null, with the reason in Said, when it names nothing.
    public int? Accept()
    {
        CapPromptRow? row = Highlighted is int h ? Rows[h] : CapNaming.Typed(Query) is int typed ? Rows.FirstOrDefault(r => r.Cap == typed) : null;
        if (row == null)
        {
            Said = Rows.Count == 0 ? "no cap is numbered " + Query.Trim() + "; caps are 0 to 599" : "type a cap number, or choose a row with ↑↓";
            return null;
        }
        if (refusal(row.Cap) is { } why) { Said = why; return null; }
        return row.Cap;
    }
}
