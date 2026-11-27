using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// ADR-D01.68 (revision 4), with revision 6's J-7: the setup slot under the Target cell, filled by the built-in setup
// search, the one solving-side exception of LB1.5 and a declared BoundaryClass of its own (ADR-D01.8).
// - The solver names the target: the Target, or with one key the Wanted piece's present place.
// - The alphabet excludes the caps that move the buffers.
// - Rows come in a neutral order, length and then menu order, each with its length shown. A length is called minimal
//   only within the guarded alphabet, and only in a completed result.
// - The solver picks a row and the host submits that word to the preview. A row fills Prepare and Cleanup around the
//   macro the solver placed in Core, or previews the setup alone; never a whole placement.
// - A completed result carries its coverage: the family examined, its guarded alphabet and its length bound. An empty
//   answer names the family and claims only that.
// - The tool ships only with its BoundaryClass and its PrincipleRecord.
// This build has no engine: the cap menu, the buffer guard and the answer rule are demo data, marked where they show.
// The enumeration and its counts are exact for that sample family.

// One letter of the guarded alphabet: a cap of the menu (its index), the cell and axis it turns, and the turn.
public sealed record SetupLetter(int Cap, int Cell, int Axis, int Turns)
{
    public string Twist => new SampleCheck.Twist(Cell, Axis, Turns).ToString();
    public string Short => "c" + Cell.ToString(CultureInfo.InvariantCulture) + "." + Axis.ToString(CultureInfo.InvariantCulture) + (Turns > 0 ? "+" : "-");
}

public sealed record SetupRow(int Number, IReadOnlyList<SetupLetter> Letters)
{
    public int Length => Letters.Count;
    public IReadOnlyList<string> Word => Letters.Select(l => l.Twist).ToArray();
    // The setup undone: the letters in reverse order, each turned back.
    public IReadOnlyList<string> Inverse => Enumerable.Reverse(Letters).Select(l => (l with { Turns = -l.Turns }).Twist).ToArray();
    public string Short => string.Join(" ", Letters.Select(l => l.Short));
}

// What a completed search covered (J-7): the family examined, its guarded alphabet (the cells of the caps kept), the
// caps excluded and the buffer each moves, the length bound, and how many words of the family were tried of how many.
public sealed record SetupCoverage(string Family, IReadOnlyList<int> Alphabet, IReadOnlyList<(int Cell, string Moves)> Excluded, int Bound, int Tried, int Total);

public enum SetupNaming { Target, Wanted }

public sealed record SetupResult(string Digest, int Place, SetupNaming Naming, IReadOnlyList<SetupRow> Rows, SetupCoverage Coverage)
{
    // The shortest length found: minimal within the guarded alphabet, since every shorter word of the family was tried.
    public int? Minimal => Rows.Count == 0 || Coverage.Tried != Coverage.Total ? null : Rows.Min(r => r.Length);
}

public sealed class SetupSearch
{
    public const string SlotId = "slot.tool.setup";
    public const string Search = "setup.search", Use = "setup.row.use";
    public static readonly IReadOnlyList<string> Ids = Array.AsReadOnly(new[] { Search, Use });
    // operation.preview's argument for a row alone, as the Library's "macro:" is for a macro alone.
    public const string PreviewArgument = "setup:";
    public const string WantedArgument = "wanted";
    public const string TieRule = "length, then menu order";
    // The sample family: a menu of six caps at the named place, words of length 1 to 3.
    public const int Menu = 6, Bound = 3;

    public static readonly PrincipleRecord SamplePrinciple = new(
        Specification: "Setup words for a named target: every reduced word of length 1 to 3 over the guarded alphabet, in length order and then menu order; the sample answer rule keeps the rows (demo data).",
        Justification: PrincipleRecord.None + "the answer rule here is demo data; the shipped tool's certificate is a replay of each row at the named target.",
        Instance: PrincipleRecord.None + "this build has no engine; the real-geometry instance comes with the tool's module (LB6.1).",
        Decomposition: "an enumeration of the guarded alphabet's reduced words and one answer test per word (sample).",
        Reason: "a word over caps that move no buffer leaves both buffers where they are (the guard, by definition).",
        NotMeant: "an empty answer means that no word of the examined family answers; it says nothing about longer words or caps outside the guarded alphabet.",
        Sample: true);

    public static readonly ToolDeclaration Declared = new("Setup search", BoundaryClass.SetupSearchNamedTarget, SamplePrinciple);

    public ToolDeclaration Tool { get; }
    public ToolSlot Slot { get; } = new(SlotId);
    // Every row the solver applies or stages names the input that asked for it.
    public BoundaryLedger Ledger { get; } = new();
    public SetupResult? Result { get; private set; }
    public SetupNaming Naming { get; private set; } = SetupNaming.Target;
    public event Action? Changed;
    private readonly ReleaseRecord releases = new();
    private (string[] Prepare, string[] Cleanup)? filled;

    public SetupSearch(ToolDeclaration? tool = null) { Tool = tool ?? Declared; Slot.Changed += _ => Safe.Raise(Changed); }

    // The rows shown now: a completed result for the revision on screen; none while pending, stale, refused or failed.
    public SetupResult? Current => Slot.State == SlotState.Ready && Result != null && Slot.Result?.Digest == Result.Digest ? Result : null;

    // The place the solver named, or why there is none.
    public static (int? Place, string? Why) Named(SolvingState s, SetupNaming naming) => naming switch
    {
        SetupNaming.Target => s.Target is int t ? (t, null) : (null, "name a Target first: the setup search runs only for a target the solver names"),
        _ => s.Wanted == null ? (null, "Wanted is unset: there is no piece whose place to name")
            : s.WantedAt is not int at ? (null, "Wanted's present place is not known")
            : at == s.Target ? (null, "Wanted is in Target already") : (at, null),
    };

    private static string N(int? value) => value?.ToString(CultureInfo.InvariantCulture) ?? "-";

    // The revision a result answers: the demo state, its orbit and revision, the place named and both buffers.
    public static string Digest(SolvingState s, SetupNaming naming) =>
        string.Join("/", s.Id, N(s.Orbit), N(s.Revision), naming.ToString().ToLowerInvariant(), N(Named(s, naming).Place), N(s.BufferA), N(s.BufferB));

    private static string Parameters(SetupNaming naming) => "naming=" + naming.ToString().ToLowerInvariant() + ";menu=" + Menu + ";bound=" + Bound;

    // setup.search: no argument names the Target, "wanted" names the Wanted piece's present place.
    public string? Run(SolvingState state, string? argument)
    {
        switch (argument?.Trim().ToLowerInvariant())
        {
            case null or "" or "target": Naming = SetupNaming.Target; break;
            case WantedArgument: Naming = SetupNaming.Wanted; break;
            default: return "it names the Target (no argument) or Wanted's present place (wanted)";
        }
        if (Tool.Refusal() is { } refused) { Slot.Refuse(refused); return refused; }
        (int? place, string? why) = Named(state, Naming);
        if (place == null) { Slot.Refuse(why!); return why; }
        string digest = Digest(state, Naming), parameters = Parameters(Naming);
        long release = releases.Next();
        Slot.Request(release, digest, parameters);
        SetupResult result = Sample(state, place.Value, Naming, digest);
        SetupCoverage c = result.Coverage;
        Result = result;
        Slot.Finish(new ToolResult(release, digest, parameters, JobEnd.Completed,
            new Coverage(OrbitAtlas.Position(place.Value), state.Revision, c.Family, TieRule, AlphabetText(c), c.Tried, c.Total),
            null, result.Rows.Select(r => r.Short).ToArray()));
        Safe.Raise(Changed);
        return null;
    }

    // The revision on screen changed: a shown result or a refusal goes stale; back at its revision a result shows again.
    public void Follow(SolvingState state)
    {
        string now = Digest(state, Naming);
        if (now == Slot.Shown || (Slot.Shown == null && Slot.State != SlotState.Refused)) return;
        Slot.Invalidate(now);
    }

    // setup.row.use n: row n fills Prepare and Cleanup around the macro in Core, as one change. Core is never filled,
    // and the solver's own Prepare or Cleanup twists are never replaced (a row's earlier fill is).
    public string? Fill(string? argument, OperationDraft draft, UserEvent? asked)
    {
        if (Pick(argument, out SetupRow? row) is { } why) return why;
        if (draft.Route == InputRoute.Direct) return "the draft is in Direct: a setup row fills the draft's Prepare and Cleanup";
        if (draft.Twists(1).Count == 0) return "Core is empty: place your macro in Core first; a setup row fills Prepare and Cleanup around it, never a whole placement";
        string[] prepare = draft.Twists(0).ToArray(), cleanup = draft.Twists(2).ToArray();
        bool ours = filled is { } f && f.Prepare.SequenceEqual(prepare) && f.Cleanup.SequenceEqual(cleanup);
        if ((prepare.Length > 0 || cleanup.Length > 0) && !ours) return "Prepare or Cleanup holds your own twists: clear them first; a setup row never replaces them";
        Ledger.Record("apply", "setup row " + row!.Number + " into Prepare and Cleanup", asked);
        filled = (row.Word.ToArray(), row.Inverse.ToArray());
        draft.Surround(row.Word, row.Inverse);
        return null;
    }

    // operation.preview setup:n: row n's setup alone as the preview; the draft keeps its twists.
    public string? Alone(string? argument, UserEvent? asked, out SetupRow? row)
    {
        if (Pick(argument, out row) is { } why) return why;
        Ledger.Record("stage", "setup row " + row!.Number + " alone", asked);
        return null;
    }

    private string? Pick(string? argument, out SetupRow? row)
    {
        row = null;
        if (Current is not { } r)
            return Slot.State switch
            {
                SlotState.Stale => "the setup rows are stale: run the setup search again",
                SlotState.Empty => "run the setup search first",
                SlotState.Pending => "the setup search has not answered yet",
                _ => "the setup slot shows no rows",
            };
        if (!int.TryParse(argument?.Trim(), NumberStyles.None, CultureInfo.InvariantCulture, out int n)) return "it needs a row number";
        if (r.Rows.Count == 0) return "the setup search found no row";
        if (n < 1 || n > r.Rows.Count) return "no row " + n + ": the rows are 1 to " + r.Rows.Count;
        row = r.Rows[n - 1];
        return null;
    }

    // The sample family at a place (demo data): six caps of the menu, the caps that move a buffer taken out, and the
    // answer rule. Menu caps 2 and 5 move Buffer A and cap 4 moves Buffer B (counting from 1), whenever that buffer is set.
    private static readonly int[] MovesA = { 1, 4 }, MovesB = { 3 };

    public static SetupResult Sample(SolvingState s, int place, SetupNaming naming, string digest)
    {
        int[] cells = Enumerable.Range(0, Menu).Select(j => (place * 13 + j * 75) % SampleCheck.Cells).ToArray();
        var excluded = new List<(int Cell, string Moves)>(); var guarded = new HashSet<int>();
        if (s.BufferA != null) foreach (int j in MovesA) { guarded.Add(j); excluded.Add((cells[j], "Buffer A")); }
        if (s.BufferB != null) foreach (int j in MovesB) { guarded.Add(j); excluded.Add((cells[j], "Buffer B")); }
        int[] alphabet = Enumerable.Range(0, Menu).Where(j => !guarded.Contains(j)).ToArray();
        SetupLetter[] letters = alphabet.SelectMany(j => new[] { +1, -1 }.Select(t => new SetupLetter(j, cells[j], (place + j) % SampleCheck.Axes, t))).ToArray();
        int seed = place % 5, total = 0; var rows = new List<SetupRow>();
        foreach (IReadOnlyList<SetupLetter> word in Words(letters, Bound))
        {
            total++;
            if (Answers(word, seed)) rows.Add(new SetupRow(rows.Count + 1, word));
        }
        string family = "reduced words of length 1 to " + Bound + " over the guarded alphabet of the sample cap menu at " + OrbitAtlas.Position(place);
        return new SetupResult(digest, place, naming, rows, new SetupCoverage(family, alphabet.Select(j => cells[j]).ToArray(), excluded, Bound, total, total));
    }

    // Every reduced word (no two adjacent letters on one cap) of length 1 to bound, in length order and then menu order.
    public static IEnumerable<IReadOnlyList<SetupLetter>> Words(IReadOnlyList<SetupLetter> letters, int bound)
    {
        var level = new List<SetupLetter[]> { Array.Empty<SetupLetter>() };
        for (int length = 1; length <= bound; length++)
        {
            var next = new List<SetupLetter[]>();
            foreach (SetupLetter[] w in level)
                foreach (SetupLetter l in letters)
                    if (w.Length == 0 || w[^1].Cap != l.Cap) next.Add(w.Append(l).ToArray());
            foreach (SetupLetter[] w in next) yield return w;
            level = next;
        }
    }

    // The sample answer rule (demo data): a weighted sum of the letters' menu places.
    private static bool Answers(IReadOnlyList<SetupLetter> word, int seed) =>
        (word.Select((l, p) => (p + 1) * (3 * l.Cap + (l.Turns > 0 ? 1 : 2))).Sum() + 5 * seed) % 25 == 0;

    private static string AlphabetText(SetupCoverage c) =>
        c.Alphabet.Count.ToString(CultureInfo.InvariantCulture) + " of " + Menu + " caps; " + (c.Excluded.Count == 0 ? "none excluded"
            : "excluded " + string.Join(", ", c.Excluded.Select(e => "c" + e.Cell.ToString(CultureInfo.InvariantCulture) + " (moves " + e.Moves + ")")));

    // The slot's word and its mark (ADR-D02.40's marks): a result's own count, a refusal's reason.
    public (string Text, string Mark) Word()
    {
        string mark = SlotWords.Mark(Slot.State);
        return Slot.State switch
        {
            SlotState.Empty => ("not run", mark),
            SlotState.Pending => ("pending", mark),
            SlotState.Ready when Current is { } r => (r.Rows.Count == 0 ? "no setup" : r.Rows.Count + (r.Rows.Count == 1 ? " row" : " rows"), mark),
            SlotState.Stale => ("stale", mark),
            SlotState.Refused => (Slot.Reason ?? "refused", mark),
            _ => ("error: " + (Slot.Reason ?? ""), mark),
        };
    }

    public string? NamedLine() => Current is { } r
        ? "for " + OrbitAtlas.Position(r.Place) + (r.Naming == SetupNaming.Target ? ", the Target" : ", Wanted's present place") : null;

    public IReadOnlyList<string> RowLines() => Current is { } r
        ? r.Rows.Select(row => row.Number.ToString(CultureInfo.InvariantCulture) + "  len " + row.Length.ToString(CultureInfo.InvariantCulture) + "  " + row.Short).ToArray()
        : Array.Empty<string>();

    public string? MinimalLine() => Current is { Minimal: int m } ? "Length " + m.ToString(CultureInfo.InvariantCulture) + " is minimal within the guarded alphabet." : null;

    public string? EmptyLine() => Current is { Rows.Count: 0 } r ? "No setup found. Examined: " + r.Coverage.Family + "." : null;

    public string? CoverageLine() => Current is { Coverage: var c }
        ? "Guarded alphabet: " + AlphabetText(c) + ". Length bound " + c.Bound.ToString(CultureInfo.InvariantCulture) + "; "
          + c.Tried.ToString(CultureInfo.InvariantCulture) + " of " + c.Total.ToString(CultureInfo.InvariantCulture) + " words tried; order: " + TieRule + " (" + SolvingDemo.Mark + ")."
        : null;
}

// The setup slot's draft rows, in ADR-D02's draft file data/design/command-rows-d02.json (request ADR-D01.68): they
// are registered wherever the solving surface is mounted, D02 on or off; a row the registry holds already (D02 on
// registers the whole file) is not added again. Gives the rows, for the palette's draft mark.
public static class SetupRows
{
    public static IReadOnlyList<string> AddDrafts(CommandRegistry registry, string d02Rows)
    {
        using JsonDocument d = J.Parse(d02Rows);
        JsonElement[] rows = d.RootElement.GetProperty("commands").EnumerateArray().Where(r => SetupSearch.Ids.Contains(J.Text(r, "id"))).ToArray();
        foreach (string row in SetupSearch.Ids)
            if (!rows.Any(r => J.Text(r, "id") == row)) throw J.Error(row, "a setup draft row missing from command-rows-d02.json");
        JsonElement[] missing = rows.Where(r => !registry.Commands.ContainsKey(J.Text(r, "id"))).ToArray();
        if (missing.Length > 0) registry.AddRows(JsonSerializer.Serialize(new { commands = missing }));
        return SetupSearch.Ids;
    }
}
