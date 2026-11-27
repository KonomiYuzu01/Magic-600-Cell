using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace Magic600.Chrome;

// The kinds of record the Library holds (ADR-D01.69), each with its own glyph. Sets are of three kinds.
public enum LibraryKind { Macro, Template, QuerySet, PieceSet, PositionSet, Block, SavedState, BuiltInTool }

public sealed record LibraryRecord(string Name, LibraryKind Kind, IReadOnlyList<string> Word, string Note)
{
    // A Template's word names its roles as {Target}, {Buffer A} and {Buffer B}; they bind to the strip.
    public IReadOnlyList<string> Roles { get; init; } = Array.Empty<string>();
    // Only a macro joins a phase: a Template binds its roles first, and the other records are not words.
    public bool IsMacro => Kind == LibraryKind.Macro && Word.Count > 0;
    public bool IsTemplate => Kind == LibraryKind.Template && Word.Count > 0 && Roles.Count > 0;
}

// One slot of an effect signature: what is known of the word's effect on one orbit. Unknown is never zero.
public enum SignatureMark { Unknown, Untouched, Moved }

// A macro's effect signature (ADR-D01.70): 35 slots in the rail's order, each the effect on that orbit with its
// evidence; without a certificate every effect reads as unverified.
public sealed record EffectSignature(IReadOnlyList<(int Orbit, SignatureMark Mark)> Slots, bool Certified)
{
    // The measured effects (pieces moved, by orbit id) laid out in a rail order; an orbit with no measurement is
    // unknown. The order must name each of the 35 orbits once.
    public static EffectSignature Of(IReadOnlyDictionary<int, int> measured, IReadOnlyList<int> railOrder, bool certified)
    {
        if (railOrder.Count != OrbitAtlas.Count || railOrder.Distinct().Count() != OrbitAtlas.Count || railOrder.Any(o => o < 0 || o >= OrbitAtlas.Count))
            throw J.Error("signature", "a rail order names each of the 35 orbits once");
        return new EffectSignature(railOrder.Select(o => (o, measured.TryGetValue(o, out int moved) ? moved > 0 ? SignatureMark.Moved : SignatureMark.Untouched : SignatureMark.Unknown)).ToArray(), certified);
    }

    public static EffectSignature Unknown(IReadOnlyList<int> railOrder) => Of(new Dictionary<int, int>(), railOrder, false);
    public SignatureMark Mark(int orbit) => Slots.First(s => s.Orbit == orbit).Mark;
}

// The Library dock (ADR-D01.69): the solver's records in one tree, macros, Templates, sets, blocks and saved
// states, with a Built-in branch for the shipped tools. An input box filters in place; the keyboard moves the
// selection over the rows shown; Enter adds the selected macro to the draft's active phase and Shift+Enter
// previews it alone. A Template binds its roles to the strip first. A branch folds; while a filter is typed every
// branch with a match is shown, so every record is reached from the keyboard. The records are sample data: the
// lab has no store.
public sealed class LibraryModel
{
    public const string Mark = " (sample records, demo data)";
    // The argument of operation.preview that previews a library record alone; the name follows, and an empty name
    // asks for the selection, which the host refuses with its reason when there is none.
    public const string PreviewArgument = "macro:";
    public static string PreviewRequest(string? name) => PreviewArgument + (name ?? "");

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

    public static readonly IReadOnlyList<string> RoleNames = Array.AsReadOnly(new[] { "Target", "Buffer A", "Buffer B" });

    // Sample records. The macros are words of typed twists that the sample rule evaluates; none moves the
    // locked sample orbit. The Template's twists name its roles; bound, they never use axis 0, so it never moves
    // the locked orbit either. The built-in tools are the shipped ones of ADR-D01.76.
    public static readonly IReadOnlyList<LibraryRecord> Sample = Array.AsReadOnly(new[]
    {
        M("Three-cycle A", "a commutator of two twists", "twist(cell 5, axis 1, +1)", "twist(cell 40, axis 2, -1)", "twist(cell 5, axis 1, -1)", "twist(cell 40, axis 2, +1)"),
        M("Swap pair B", "a conjugated double turn", "twist(cell 77, axis 3, +1)", "twist(cell 22, axis 1, +2)", "twist(cell 77, axis 3, -1)"),
        M("Orient twin", "a commutator that turns two pieces in place", "twist(cell 117, axis 1, -1)", "twist(cell 8, axis 2, +1)", "twist(cell 117, axis 1, +1)", "twist(cell 8, axis 2, -1)"),
        new LibraryRecord("Three-cycle on roles", LibraryKind.Template,
            Array.AsReadOnly(new[] { "twist(cell {Target}, axis 1, +1)", "twist(cell {Buffer A}, axis 2, -1)", "twist(cell {Target}, axis 1, -1)", "twist(cell {Buffer B}, axis 2, +1)" }),
            "its roles Target, Buffer A and Buffer B bind to the strip") { Roles = RoleNames },
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

    // library.branch.toggle: folds or opens a branch by its title; or why not.
    public string? Toggle(string? branch)
    {
        string? title = branch == null ? null : Branches.Select(b => b.Title).FirstOrDefault(t => string.Equals(t, branch.Trim(), StringComparison.OrdinalIgnoreCase));
        if (title == null) return "it needs a branch: " + string.Join(", ", Branches.Select(b => b.Title));
        Fold(title); return null;
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

    public const string NoSelection = "no library record is selected";

    // The named record, or the selected one, as a macro: a macro as it is, a Template bound to the roles of the
    // strip; or why it is not one.
    public (LibraryRecord? Macro, string? Why) Macro(string? name, SolvingState? roles = null)
    {
        LibraryRecord? r = string.IsNullOrWhiteSpace(name) ? Current : Find(name);
        if (r == null) return (null, string.IsNullOrWhiteSpace(name) ? NoSelection : $"the library has no record named {name!.Trim()}");
        if (r.IsTemplate) return Bind(r, roles);
        return r.IsMacro ? (r, null) : (null, $"{r.Name} is a {Word(r.Kind)}: only a macro or a bound Template joins a phase");
    }

    // The cell of the sample rule that a position of an orbit names (sample mapping, demo data).
    public static int SampleCell(int orbit, int position) => (int)(((long)position * 7 + orbit * 13) % SampleCheck.Cells);

    // A Template's roles bind to the strip of the working orbit (ADR-D01.69): each role takes the position the
    // strip holds, so nothing is typed twice. The bound word is a new macro, named by its binding; a Template bound
    // to another Target is another word, checked again as its own preview, never a reuse of an earlier check.
    public static (LibraryRecord? Macro, string? Why) Bind(LibraryRecord template, SolvingState? roles)
    {
        if (!template.IsTemplate) return (null, $"{template.Name} is not a Template");
        if (roles == null) return (null, $"{template.Name} is a Template: its roles bind to the strip, and no strip is here");
        var places = new Dictionary<string, int?>(StringComparer.Ordinal) { ["Target"] = roles.Target, ["Buffer A"] = roles.BufferA, ["Buffer B"] = roles.BufferB };
        foreach (string role in template.Roles)
        {
            if (!places.TryGetValue(role, out int? at)) return (null, $"{template.Name} has the role {role}, which the strip does not hold");
            if (at == null) return (null, $"{template.Name} needs its role {role}: set {role} on the strip");
        }
        if (roles.Problems() is { Count: > 0 } bad) return (null, $"{template.Name} cannot bind an invalid tuple: {bad[0].Text}");
        string Fill(string twist) => template.Roles.Aggregate(twist, (w, role) => w.Replace("{" + role + "}", SampleCell(roles.Orbit, places[role]!.Value).ToString(CultureInfo.InvariantCulture), StringComparison.Ordinal));
        string[] word = template.Word.Select(Fill).ToArray();
        string name = $"{template.Name}(Target {OrbitAtlas.Position(roles.Target!.Value)}, A {OrbitAtlas.Position(roles.BufferA!.Value)}, B {OrbitAtlas.Position(roles.BufferB!.Value)})";
        return (new LibraryRecord(name, LibraryKind.Macro, Array.AsReadOnly(word), $"{template.Name} bound to the strip of {OrbitAtlas.Name(roles.Orbit)} (sample mapping)"), null);
    }

    // Enter (macro.add): the macro's twists, or the bound Template's, join the draft's active phase as one change.
    // Nothing is executed: the draft stages itself as the preview, and the commit bar commits it.
    public string? Insert(OperationDraft draft, string? name = null, SolvingState? roles = null)
    {
        (LibraryRecord? macro, string? why) = Macro(name, roles);
        if (why != null) return why;
        if (draft.Route == InputRoute.Direct) return "in Direct a twist is one complete operation; switch to Draft to add a macro to a phase";
        return draft.AppendAll(macro!.Word) ? null : "the macro has no twist";
    }

    // The effect signature (ADR-D01.70) in a rail order: the sample rule's effects stand on the rail orbits the
    // five sample orbits are drawn on (SampleCheck.RailOrbits); every other orbit is unknown, never zero. It is
    // never certified here, so it reads as unverified.
    public static EffectSignature Signature(IReadOnlyList<string> word, IReadOnlyList<int> railOrder)
    {
        if (word.Count == 0 || SampleCheck.Effects(word) is not { } effects) return EffectSignature.Unknown(railOrder);
        return Signature(effects, railOrder);
    }

    // The same signature from a complete effect a check already measured (a preview's, ADR-D01.70): each sample
    // orbit's effect stands on its rail orbit, an orbit the check did not measure is unknown, and no effect at all
    // (a preview that was not evaluated) leaves every orbit unknown. Never certified here.
    public static EffectSignature Signature(IReadOnlyList<OrbitEffect> effects, IReadOnlyList<int> railOrder)
    {
        var measured = new Dictionary<int, int>();
        foreach (OrbitEffect effect in effects)
        {
            int sample = SampleCheck.Orbits.ToList().IndexOf(effect.Orbit);
            if (sample >= 0) measured[SampleCheck.RailOrbits[sample]] = effect.Moved;
        }
        return EffectSignature.Of(measured, railOrder, false);
    }

    public static string SignatureText(EffectSignature signature)
    {
        string Names(SignatureMark mark) => string.Join(", ", signature.Slots.Where(s => s.Mark == mark).Select(s => OrbitAtlas.Name(s.Orbit)));
        int unknown = signature.Slots.Count(s => s.Mark == SignatureMark.Unknown);
        if (unknown == signature.Slots.Count) return "Effect signature: every orbit unknown.";
        string moved = Names(SignatureMark.Moved), untouched = Names(SignatureMark.Untouched);
        return $"Effect signature, {(signature.Certified ? "certified" : "unverified")}: moves {(moved.Length == 0 ? "no orbit" : moved)}; untouched {(untouched.Length == 0 ? "none" : untouched)}; " +
            $"{unknown} orbits unknown{(signature.Certified ? "" : SampleCheck.Mark)}.";
    }
}
