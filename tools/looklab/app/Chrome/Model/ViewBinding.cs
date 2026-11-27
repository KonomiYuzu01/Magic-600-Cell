using System;

namespace Magic600.Chrome;

// The source a free view shows (ADR-D01.42): the committed state, a preview, a saved reference or a history position.
// The main pair always shows the committed state (ADR-D01.40) and carries no chip; a free view names its source on a
// binding chip in its head, in one or two words, with the full sentence as the chip's tooltip.
public enum BindingSource { Committed, Preview, Saved, History }

public sealed record ViewBinding
{
    public BindingSource Source { get; }
    // A history position: the state after Position of Count moves (0 is the start). Zero for the other sources.
    public int Position { get; }
    public int Count { get; }

    private ViewBinding(BindingSource source, int position, int count) { Source = source; Position = position; Count = count; }

    public static ViewBinding Committed { get; } = new(BindingSource.Committed, 0, 0);
    public static ViewBinding Preview { get; } = new(BindingSource.Preview, 0, 0);
    public static ViewBinding Saved { get; } = new(BindingSource.Saved, 0, 0);

    public static ViewBinding At(int position, int count) =>
        count < 0 || position < 0 || position > count
            ? throw new ArgumentOutOfRangeException(nameof(position), $"history position {position} of {count}")
            : new(BindingSource.History, position, count);

    public string Words => Source switch
    {
        BindingSource.Committed => "committed",
        BindingSource.Preview => "preview",
        BindingSource.Saved => "saved",
        _ => $"history {Position} of {Count}",
    };

    public string Tip => Source switch
    {
        BindingSource.Committed => "Shows the committed state",
        BindingSource.Preview => "Shows a preview: nothing here is committed",
        BindingSource.Saved => "Shows a saved reference, not the session's state",
        _ => Position == 0 ? $"Shows the start, before move 1 of {Count}" : $"Shows the state after move {Position} of {Count}",
    };

    // A model's binding words back to a binding: a verified log's Document.Binding is "saved". Anything else has none.
    public static ViewBinding? Of(string? words) => words switch
    {
        "committed" => Committed,
        "preview" => Preview,
        "saved" => Saved,
        _ => null,
    };
}
