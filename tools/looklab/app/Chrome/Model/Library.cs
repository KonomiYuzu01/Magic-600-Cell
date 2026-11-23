using System;
using System.Collections.Generic;
using System.Linq;

namespace Magic600.Chrome;

// The kinds of record the Library holds (ADR-D01.69), each with its own glyph. Sets are of three kinds.
public enum LibraryKind { Macro, Template, QuerySet, PieceSet, PositionSet, Block, SavedState, BuiltInTool }

public sealed record LibraryRecord(string Name, LibraryKind Kind, IReadOnlyList<string> Word, string Note)
{
    // Only a macro joins a phase: a Template binds its roles first, and the other records are not words.
    public bool IsMacro => Kind == LibraryKind.Macro && Word.Count > 0;
}

// The Library dock (ADR-D01.69): the solver's records in one tree, macros, Templates, sets, blocks and saved
// states, with a Built-in branch for the shipped tools. An input box filters in place; the keyboard moves the
// selection over the rows shown; Enter adds the selected macro to the draft's active phase and Shift+Enter
// previews it alone. A branch folds; while a filter is typed every branch with a match is shown, so every
// record is reached from the keyboard. The records are sample data: the lab has no store.
public sealed class LibraryModel
{
    public const string Mark = " (sample records, demo data)";
    public static readonly IReadOnlyList<(string Title, LibraryKind[] Kinds)> Branches = Array.AsReadOnly(new[]
    {
        ("Macros", new[] { LibraryKind.Macro }), ("Templates", new[] { LibraryKind.Template }),
        ("Sets", new[] { LibraryKind.QuerySet, LibraryKind.PieceSet, LibraryKind.PositionSet }), ("Blocks", new[] { LibraryKind.Block }),
        ("Saved states", new[] { LibraryKind.SavedState }), ("Built-in", new[] { LibraryKind.BuiltInTool }),
    });

    public static string Glyph(LibraryKind kind) => kind switch
    {
        LibraryKind.Macro => "glyph.macro", LibraryKind.Template => "library.template", LibraryKind.QuerySet => "library.set.query",
        LibraryKind.PieceSet => "library.set.piece", LibraryKind.PositionSet => "library.set.position", LibraryKind.Block => "library.block",
        LibraryKind.SavedState => "library.state", _ => "library.tool",
    };

    public static string Word(LibraryKind kind) => kind switch
    {
        LibraryKind.Macro => "macro", LibraryKind.Template => "Template", LibraryKind.QuerySet => "Query set", LibraryKind.PieceSet => "Piece set",
        LibraryKind.PositionSet => "Position set", LibraryKind.Block => "block", LibraryKind.SavedState => "saved state", _ => "built-in tool",
    };

    private static LibraryRecord M(string name, string note, params string[] word) => new(name, LibraryKind.Macro, Array.AsReadOnly(word), note);
    private static LibraryRecord R(string name, LibraryKind kind, string note) => new(name, kind, Array.Empty<string>(), note);

    // Sample records. The macros are words of typed twists that the sample rule evaluates; none moves the
    // locked sample orbit. The built-in tools are the shipped ones of ADR-D01.76.
    public static readonly IReadOnlyList<LibraryRecord> Sample = Array.AsReadOnly(new[]
    {
        M("Three-cycle A", "a commutator of two twists", "twist(cell 5, axis 1, +1)", "twist(cell 40, axis 2, -1)", "twist(cell 5, axis 1, -1)", "twist(cell 40, axis 2, +1)"),
        M("Swap pair B", "a conjugated double turn", "twist(cell 77, axis 3, +1)", "twist(cell 22, axis 1, +2)", "twist(cell 77, axis 3, -1)"),
        M("Orient twin", "a commutator that turns two pieces in place", "twist(cell 117, axis 1, -1)", "twist(cell 8, axis 2, +1)", "twist(cell 117, axis 1, +1)", "twist(cell 8, axis 2, -1)"),
        R("Three-cycle on roles", LibraryKind.Template, "parameters Target, Buffer A and Buffer B bind to the strip"),
        R("Not at Home in O07", LibraryKind.QuerySet, "a query over the head"),
        R("Corner block pieces", LibraryKind.PieceSet, "pieces the solver named"),
        R("Home of the corner block", LibraryKind.PositionSet, "positions the solver named"),
        R("Corner block", LibraryKind.Block, "drawn in the views as an outline"),
        R("Before the O07 cycle", LibraryKind.SavedState, "a saved state of this session"),
        R("Residuals", LibraryKind.BuiltInTool, "a shipped tool"), R("Protection check", LibraryKind.BuiltInTool, "a shipped tool"),
        R("Setup search", LibraryKind.BuiltInTool, "a shipped tool"), R("Macro facts", LibraryKind.BuiltInTool, "a shipped tool"),
        R("Compare macros", LibraryKind.BuiltInTool, "a shipped tool"), R("Cycles", LibraryKind.BuiltInTool, "a shipped tool"),
        R("Structure", LibraryKind.BuiltInTool, "a shipped tool"),
    });

    private readonly HashSet<string> folded = new(StringComparer.Ordinal) { "Templates", "Sets", "Blocks", "Saved states", "Built-in" };
    public IReadOnlyList<LibraryRecord> Records { get; }
    public string Filter { get; private set; } = "";
    public string? Selected { get; private set; }
    public event Action? Changed;

    public LibraryModel(IReadOnlyList<LibraryRecord>? records = null)
    {
        Records = records ?? Sample;
        if (Records.Select(r => r.Name).Distinct(StringComparer.OrdinalIgnoreCase).Count() != Records.Count) throw J.Error("library", "two library records share a name");
        Selected = Shown.FirstOrDefault()?.Name;
    }

    public static string BranchOf(LibraryKind kind) => Branches.First(b => b.Kinds.Contains(kind)).Title;

    private bool Matches(LibraryRecord r)
    {
        string f = Filter.Trim();
        return f.Length == 0 || r.Name.Contains(f, StringComparison.OrdinalIgnoreCase) || Word(r.Kind).Contains(f, StringComparison.OrdinalIgnoreCase) ||
            BranchOf(r.Kind).Contains(f, StringComparison.OrdinalIgnoreCase);
    }

    public bool Filtering => Filter.Trim().Length > 0;
    // The records of a branch that match the filter, in their stored order.
    public IReadOnlyList<LibraryRecord> Matching(string branch) => Records.Where(r => BranchOf(r.Kind) == branch && Matches(r)).ToArray();
    public bool IsOpen(string branch) => Filtering || !folded.Contains(branch);
    // The rows on screen, in tree order: the open branches' matching records.
    public IReadOnlyList<LibraryRecord> Shown => Branches.Where(b => IsOpen(b.Title)).SelectMany(b => Matching(b.Title)).ToArray();
    public LibraryRecord? Current => Shown.FirstOrDefault(r => r.Name == Selected);

    public void Fold(string branch)
    {
        if (!Branches.Any(b => b.Title == branch)) return;
        if (!folded.Add(branch)) folded.Remove(branch);
        if (Current == null) Selected = Shown.FirstOrDefault()?.Name;
        Safe.Raise(Changed);
    }

    // The filter keeps the selection when it is still shown, and otherwise selects the first row shown.
    public void SetFilter(string text)
    {
        Filter = text ?? "";
        if (Current == null) Selected = Shown.FirstOrDefault()?.Name;
        Safe.Raise(Changed);
    }

    public void Move(int by)
    {
        IReadOnlyList<LibraryRecord> shown = Shown;
        if (shown.Count == 0) { Selected = null; Safe.Raise(Changed); return; }
        int at = shown.ToList().FindIndex(r => r.Name == Selected);
        Selected = shown[at < 0 ? 0 : Math.Clamp(at + by, 0, shown.Count - 1)].Name; Safe.Raise(Changed);
    }

    private LibraryRecord? Find(string name) => Records.FirstOrDefault(r => string.Equals(r.Name, name.Trim(), StringComparison.OrdinalIgnoreCase));

    // Selects a record by name and shows it: its branch opens and a filter that hides it is cleared.
    public string? Select(string? name)
    {
        if (string.IsNullOrWhiteSpace(name)) return "it needs the name of a library record";
        if (Find(name) is not { } r) return $"the library has no record named {name.Trim()}";
        folded.Remove(BranchOf(r.Kind));
        if (!Matches(r)) Filter = "";
        Selected = r.Name; Safe.Raise(Changed); return null;
    }

    // The named record, or the selected one, as a macro; or why it is not one.
    public (LibraryRecord? Macro, string? Why) Macro(string? name)
    {
        LibraryRecord? r = string.IsNullOrWhiteSpace(name) ? Current : Find(name);
        if (r == null) return (null, string.IsNullOrWhiteSpace(name) ? "no library record is selected" : $"the library has no record named {name!.Trim()}");
        return r.IsMacro ? (r, null) : (null, $"{r.Name} is a {Word(r.Kind)}: only a macro joins a phase");
    }

    // Enter (macro.add): the macro's twists join the draft's active phase as one change. Nothing is executed:
    // the draft stages itself as the preview, and the commit bar commits it.
    public string? Insert(OperationDraft draft, string? name = null)
    {
        (LibraryRecord? macro, string? why) = Macro(name);
        if (why != null) return why;
        if (draft.Route == InputRoute.Direct) return "in Direct a twist is one complete operation; switch to Draft to add a macro to a phase";
        return draft.AppendAll(macro!.Word) ? null : "the macro has no twist";
    }

    // The effect signature on the sample orbits (ADR-D01.70): whether the word moves each orbit, by the sample
    // rule. It is never certified here, so it reads as unverified.
    public static IReadOnlyList<bool> Signature(IReadOnlyList<string> word) =>
        word.Count > 0 && SampleCheck.Effects(word) is { } effects ? effects.Select(e => e.Moved > 0).ToArray() : Array.Empty<bool>();

    public static string SignatureText(IReadOnlyList<string> word)
    {
        IReadOnlyList<bool> moved = Signature(word);
        if (moved.Count == 0) return "No effect signature: not a word of twists.";
        string[] names = moved.Select((m, i) => (m, i)).Where(x => x.m).Select(x => SampleCheck.Orbits[x.i]).ToArray();
        return "Effect signature, unverified: moves " + (names.Length == 0 ? "no sample orbit" : string.Join(", ", names)) + SampleCheck.Mark + ".";
    }
}
