using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// The 0.4 Piece Filter as a focus layer (owner, 26 November 2026: "Focus layer加别名就行"; ADR-D01 revision 4, .46 and
// G21: the 0.4 Boolean filters are focus layers, an `=` expression with a display state and its count). Dispositions
// U6-022, U5-048, U5-050, U5-051 and U2-009; command rows filter.open, filter.preview, filter.apply, filter.edit,
// filter.preset, filter.save, filter.load and structure.filter.compose.
//
// Sample mode: an expression reads the bound sample state's pose rows (style.json), so every count is exact for that
// state; the engine's filter (c600-command's filter.preview and filter.apply, KN0.9) is not reached through the host
// yet. A filter only adds a focus layer: it never relabels a piece, rebinds a key, or narrows the analysis, the
// protection or the collateral of the full 259,800-label model.

// One row of the bound state's pose rows: a piece class by its domain and its (left) height, and how many pieces.
public readonly record struct PoseRow(int Domain, int Height, long Count);

// An `=` expression over a piece's fields: `domain`, `height` and `lattice`, with `and`, `or`, `not` and brackets.
public abstract record FilterExpr
{
    public abstract bool Holds(PoseRow row, int lattice);
    public abstract string Text { get; }
    public virtual bool NeedsPieces => false;

    public sealed record Compare(string Field, string Op, int Value) : FilterExpr
    {
        public override bool Holds(PoseRow row, int lattice)
        {
            return Matches(Field == "domain" ? row.Domain : row.Height);
        }
        public bool Matches(int value) =>
            Op switch { "=" => value == Value, "!=" => value != Value, ">=" => value >= Value, "<=" => value <= Value, ">" => value > Value, _ => value < Value };
        public override string Text => Field + " " + Op + " " + Value.ToString(CultureInfo.InvariantCulture);
    }

    public sealed record PieceField(string Name, int? Cap = null, Compare? Comparison = null, string? SetName = null) : FilterExpr
    {
        public override bool NeedsPieces => true;
        public override bool Holds(PoseRow row, int lattice) => throw new InvalidOperationException("This field needs a piece set.");
        public override string Text => Comparison?.Text ?? (SetName != null ? Name + " " + JsonSerializer.Serialize(SetName)
            : Cap is int cap ? Name + " " + cap.ToString(CultureInfo.InvariantCulture) : Name);
    }

    public sealed record Lattice : FilterExpr
    {
        public override bool Holds(PoseRow row, int lattice) => row.Domain == lattice;
        public override string Text => "lattice";
    }

    public sealed record Not(FilterExpr Inner) : FilterExpr
    {
        public override bool NeedsPieces => Inner.NeedsPieces;
        public override bool Holds(PoseRow row, int lattice) => !Inner.Holds(row, lattice);
        public override string Text => "not " + Wrap(Inner);
    }

    public sealed record And(FilterExpr Left, FilterExpr Right) : FilterExpr
    {
        public override bool NeedsPieces => Left.NeedsPieces || Right.NeedsPieces;
        public override bool Holds(PoseRow row, int lattice) => Left.Holds(row, lattice) && Right.Holds(row, lattice);
        public override string Text => Wrap(Left) + " and " + Wrap(Right);
    }

    public sealed record Or(FilterExpr Left, FilterExpr Right) : FilterExpr
    {
        public override bool NeedsPieces => Left.NeedsPieces || Right.NeedsPieces;
        public override bool Holds(PoseRow row, int lattice) => Left.Holds(row, lattice) || Right.Holds(row, lattice);
        public override string Text => Wrap(Left) + " or " + Wrap(Right);
    }

    private static string Wrap(FilterExpr e) => e is And or Or ? "(" + e.Text + ")" : e.Text;
}

// Reads a whole expression before anything applies (U6-022): an error names the place and the words it takes.
public static class FilterParser
{
    public const string Fields = "domain, height, lattice, inside-a-cap, straddler, moved, class, rim, seam, set";
    public const string Words = "the words are " + Fields + ", and, or, not, brackets and = != >= <= > <";

    public static (FilterExpr? Expr, string? Error) Parse(string text, int domains)
    {
        var tokens = new List<(string Text, int At)>();
        for (int i = 0; i < text.Length;)
        {
            char c = text[i];
            if (char.IsWhiteSpace(c)) { i++; continue; }
            if (c is '(' or ')') { tokens.Add((c.ToString(), i)); i++; continue; }
            if (c == '"')
            {
                int end = i + 1;
                while (end < text.Length && text[end] != '"') end += text[end] == '\\' ? 2 : 1;
                if (end >= text.Length) return (null, "a quote is missing at " + (i + 1));
                tokens.Add((text[i..(end + 1)], i)); i = end + 1; continue;
            }
            if (c is '≥' or '≤' or '≠') { tokens.Add((c == '≥' ? ">=" : c == '≤' ? "<=" : "!=", i)); i++; continue; }
            if (c is '=' or '!' or '<' or '>')
            {
                string raw = i + 1 < text.Length && text[i + 1] == '=' ? text.Substring(i, 2) : c.ToString();
                if (raw == "!") return (null, "'!' at " + (i + 1) + " needs '=': " + Words);
                tokens.Add((raw == "==" ? "=" : raw, i)); i += raw.Length; continue;
            }
            int start = i;
            while (i < text.Length && (char.IsLetterOrDigit(text[i]) || text[i] is '_' or '-')) i++;
            if (i == start) return (null, "'" + c + "' at " + (i + 1) + " is not a word: " + Words);
            tokens.Add((text[start..i], start));
        }
        if (tokens.Count == 0) return (null, "the expression is empty: " + Words);
        int at = 0;
        string? error = null;
        (string Text, int At) Peek() => at < tokens.Count ? (tokens[at].Text.ToLowerInvariant(), tokens[at].At) : ("", text.Length);
        bool Next(string word) { if (Peek().Text == word) { at++; return true; } return false; }
        string Where() => " at " + (Peek().At + 1);

        FilterExpr? Or()
        {
            FilterExpr? left = And(); if (left == null) return null;
            while (Next("or")) { FilterExpr? right = And(); if (right == null) return null; left = new FilterExpr.Or(left, right); }
            return left;
        }
        FilterExpr? And()
        {
            FilterExpr? left = Unary(); if (left == null) return null;
            while (Next("and")) { FilterExpr? right = Unary(); if (right == null) return null; left = new FilterExpr.And(left, right); }
            return left;
        }
        FilterExpr? Unary() => Next("not") ? (Unary() is { } inner ? new FilterExpr.Not(inner) : null) : Primary();
        FilterExpr? Primary()
        {
            var (word, _) = Peek();
            if (Next("("))
            {
                FilterExpr? inner = Or(); if (inner == null) return null;
                if (!Next(")")) { error = "a ')' is missing" + Where(); return null; }
                return inner;
            }
            if (Next("lattice")) return new FilterExpr.Lattice();
            if (word is "moved" or "seam")
            {
                at++;
                return new FilterExpr.PieceField(word);
            }
            if (word is "inside-a-cap" or "straddler" or "rim")
            {
                at++;
                if (!int.TryParse(Peek().Text, NumberStyles.None, CultureInfo.InvariantCulture, out int cap))
                    return new FilterExpr.PieceField(word);
                if (cap >= CorePlaces.Cells) { error = "cap " + cap + " is unavailable" + Where(); return null; }
                at++;
                return new FilterExpr.PieceField(word, cap);
            }
            if (Next("set"))
            {
                string name = at < tokens.Count ? tokens[at].Text : "";
                if (name.StartsWith('"'))
                {
                    try { name = JsonSerializer.Deserialize<string>(name) ?? ""; }
                    catch (JsonException) { error = "the saved set name is invalid" + Where(); return null; }
                }
                else if (!name.All(c => char.IsLetterOrDigit(c) || c is '_' or '-')) name = "";
                if (string.IsNullOrWhiteSpace(name)) { error = "set needs a saved name" + Where(); return null; }
                at++;
                return new FilterExpr.PieceField(word, SetName: name);
            }
            if (word is "domain" or "height" or "class")
            {
                at++;
                var (op, _) = Peek();
                if (op is not ("=" or "!=" or ">=" or "<=" or ">" or "<")) { error = "'" + word + "' needs a comparison" + Where() + ", as in " + word + " >= 2"; return null; }
                at++;
                var (number, _) = Peek();
                if (!int.TryParse(number, NumberStyles.None, CultureInfo.InvariantCulture, out int value)) { error = "'" + word + " " + op + "' needs a whole number" + Where(); return null; }
                at++;
                if (word == "domain" && value >= domains) { error = "there is no domain " + value + "; the bound state's domains are 0 to " + (domains - 1); return null; }
                var comparison = new FilterExpr.Compare(word, op, value);
                return word == "class" ? new FilterExpr.PieceField(word, Comparison: comparison) : comparison;
            }
            error = (word.Length == 0 ? "the expression ends too soon" : "'" + word + "' is not a word here") + Where() + ": " + Words;
            return null;
        }

        Next("=");
        FilterExpr? expr = Or();
        if (expr != null && at < tokens.Count) { error = "'" + tokens[at].Text + "' is left over" + Where() + ": " + Words; expr = null; }
        return expr == null ? (null, error ?? "the expression is not complete") : (expr, null);
    }
}

// A live query is evaluated again at every revision; fixed pieces follow the pieces it held when applied; watched
// positions keep the positions it held when applied. One is never silently put in another's place (U6-022).
public enum FilterKind { LiveQuery, FixedPieces, WatchedPositions }

// How a new expression meets the filter already applied (structure.filter.compose; U5-048).
public enum FilterComposition { Replace, Intersect, Union, Subtract }

// A preview: the staged expression composed with the applied filter, its exact count of the bound state's pieces, at
// one revision; it applies nothing (filter.preview; U5-050).
public sealed record FilterPreview(string Staged, FilterExpr Combined, FilterComposition Composition, FilterKind Kind, long Revision, long Count, long Of);

// The filter that applies now, as the view shows it.
public sealed record AppliedFilter(FilterExpr Expr, FilterKind Kind, long Revision, long Count, long Of);

// A saved filter: its name, the exact expression saved and its kind, and the state it was made for (U5-051).
public sealed record SavedFilter(string Name, string Expression, FilterKind Kind, string Compatibility)
{
    public IReadOnlyList<bool>? Members { get; init; }
}

// What a piece is to the solver's work under the applied focus layer (U5-048): a member is work; the rest are
// references, visible while dimmed and never picked; a hidden non-member is neither drawn nor picked.
public enum Interactivity { Work, Reference, Hidden }

public sealed class PieceFilterModel
{
    public const string LayerId = "focus.expression";
    public static readonly IReadOnlyList<(string Name, string Expression)> Presets = Array.AsReadOnly(new[]
    {
        ("Lattice domain", "lattice"), ("Off the lattice", "not lattice"), ("Height ≥ 2", "height >= 2"), ("Lattice, height 0", "lattice and height = 0"),
    });

    private IReadOnlyList<PoseRow>? rows;
    private IReadOnlyList<int>? poses;
    private Func<FilterExpr.PieceField, Func<int, bool>?>? readField;
    private bool[]? previewMask, appliedMask;
    private string? appliedUnavailable;
    private int lattice, domains;
    private readonly Func<long> revision;
    private readonly List<SavedFilter> saved = new();
    public string Compatibility { get; private set; }
    public string Staged { get; private set; } = "";
    public FilterComposition Composition { get; private set; } = FilterComposition.Replace;
    public FilterKind Kind { get; private set; } = FilterKind.LiveQuery;
    public FilterPreview? Preview { get; private set; }
    public string? Error { get; private set; }
    public AppliedFilter? Applied { get; private set; }
    public IReadOnlyList<SavedFilter> Saved => saved;
    public long Pieces { get; private set; }
    public event Action? Changed;
    public event Action? AppliedChanged;

    // `compatibility` names the state the counts are for (the bound sample's state digest).
    public PieceFilterModel(IReadOnlyList<PoseRow> rows, int lattice, int domains, Func<long> revision, string compatibility)
    {
        this.rows = rows; this.lattice = lattice; this.domains = domains; this.revision = revision; Compatibility = compatibility;
        Pieces = rows.Sum(r => r.Count);
    }

    public void Rebind(PoseStyle? style)
    {
        if (style?.Digest != Compatibility) poses = null;
        rows = style?.Rows;
        Preview = null;
        Error = null;
        if (style != null)
        {
            lattice = style.LatticeDomain;
            domains = style.Domains.Count;
            Compatibility = style.Digest;
            Pieces = style.Pieces;
        }
        UpdateApplied();
        Safe.Raise(Changed);
    }

    public void BindPieces(IReadOnlyList<int>? poseIds, Func<FilterExpr.PieceField, Func<int, bool>?> fields)
    {
        poses = poseIds;
        readField = fields;
        RefreshFields();
    }

    // Cap naming, focused repair step and preview changes can change a field without changing the state revision.
    public void RefreshFields()
    {
        if (Preview?.Combined.NeedsPieces == true)
        {
            Preview = null;
            Error = "the filter fields changed after the preview; preview it again";
        }
        else if (Applied?.Expr.NeedsPieces != true) return;
        UpdateApplied();
        Safe.Raise(Changed);
    }

    private void UpdateApplied()
    {
        appliedMask = null;
        appliedUnavailable = null;
        if (rows == null || Applied is not { } filter) return;
        appliedUnavailable = Read(filter.Expr, out appliedMask, out long count);
        Applied = filter with
        {
            Count = count, Of = Pieces,
            Revision = filter.Kind == FilterKind.LiveQuery ? revision() : filter.Revision,
        };
    }

    private string? Read(FilterExpr expr, out bool[]? mask, out long count)
    {
        mask = null;
        count = 0;
        if (!expr.NeedsPieces)
        {
            if (rows != null) count = rows.Where(r => expr.Holds(r, lattice)).Sum(r => r.Count);
            return null;
        }
        string? error = null;
        var test = Resolve(expr);
        if (error != null) return error;
        mask = new bool[poses!.Count];
        for (int piece = 0; piece < mask.Length; piece++)
            if (mask[piece] = test(piece)) count++;
        return null;

        Func<int, bool> Resolve(FilterExpr node)
        {
            switch (node)
            {
                case FilterExpr.And and:
                    var left = Resolve(and.Left);
                    var right = Resolve(and.Right);
                    return p => left(p) && right(p);
                case FilterExpr.Or or:
                    var either = Resolve(or.Left);
                    var other = Resolve(or.Right);
                    return p => either(p) || other(p);
                case FilterExpr.Not not:
                    var inner = Resolve(not.Inner);
                    return p => !inner(p);
                case FilterExpr.PieceField field:
                    if (poses != null && poses.Count == Pieces)
                    {
                        if (field.SetName is string name)
                        {
                            if (saved.Find(s => s.Name == name && s.Compatibility == Compatibility)?.Members is { } set
                                && set.Count == poses.Count) return p => set[p];
                        }
                        else if (readField?.Invoke(field) is { } predicate) return predicate;
                    }
                    error ??= field.Text + " unavailable for this sample";
                    return _ => false;
                default:
                    var matches = rows!.Select(row => node.Holds(row, lattice)).ToArray();
                    return p => matches[poses![p]];
            }
        }
    }

    public Func<int, bool>? Member()
    {
        if (rows == null || poses is not { } poseIds || Applied is not { } filter || appliedUnavailable != null) return null;
        if (appliedMask is { } mask) return p => mask[p];
        var poseMatches = rows.Select(row => filter.Expr.Holds(row, lattice)).ToArray();
        return p => poseMatches[poseIds[p]];
    }

    // Typing changes the staged text only; an earlier preview no longer stands for it. Rejected text stays.
    public void SetText(string text) { if (text == Staged) return; Staged = text; Preview = null; Error = null; Safe.Raise(Changed); }
    public void Compose(FilterComposition composition) { if (composition == Composition) return; Composition = composition; Preview = null; Safe.Raise(Changed); }
    public void SetKind(FilterKind kind) { if (kind == Kind) return; Kind = kind; Preview = null; Safe.Raise(Changed); }

    // filter.preview: the whole expression and its composition are checked, and the exact count shown; nothing applies.
    public FilterPreview? Run()
    {
        Preview = null;
        if (rows == null) return null;
        var (expr, error) = FilterParser.Parse(Staged, domains);
        if (expr == null) { Error = error; Safe.Raise(Changed); return null; }
        FilterExpr? combined = Combine(expr, out error);
        if (combined == null) { Error = error; Safe.Raise(Changed); return null; }
        Error = Read(combined, out previewMask, out long count);
        if (Error == null) Preview = new FilterPreview(Staged, combined, Composition, Kind, revision(), count, Pieces);
        Safe.Raise(Changed);
        return Preview;
    }

    private FilterExpr? Combine(FilterExpr expr, out string? error)
    {
        error = null;
        if (Composition == FilterComposition.Replace || Applied == null) return expr;
        if (Applied.Kind != Kind)
        {
            error = "a " + Word(Kind) + " does not compose with the applied " + Word(Applied.Kind) + "; replace it, or apply one alone";
            return null;
        }
        return Composition switch
        {
            FilterComposition.Intersect => new FilterExpr.And(Applied.Expr, expr),
            FilterComposition.Union => new FilterExpr.Or(Applied.Expr, expr),
            _ => new FilterExpr.And(Applied.Expr, new FilterExpr.Not(expr)),
        };
    }

    // filter.apply: only a shown preview applies, and only while its context holds: the same text, composition and kind,
    // the same applied filter and the same revision. Otherwise it is refused and nothing changes.
    public string? Apply(UserEvent? asked)
    {
        string? refused = Applying(asked);
        if (refused != null) { Error = refused; Safe.Raise(Changed); }
        return refused;
    }

    private string? Applying(UserEvent? asked)
    {
        if (asked == null) return "a filter applies only on the solver's key or click";
        if (Preview is not { } p) return Error ?? "preview the filter first: its count shows before it applies";
        if (p.Staged != Staged || p.Composition != Composition || p.Kind != Kind) return "the expression changed after its preview; preview it again";
        if (p.Revision != revision()) { Preview = null; Safe.Raise(Changed); return "the state changed after the preview; preview it again"; }
        var (expr, _) = FilterParser.Parse(p.Staged, domains);
        FilterExpr? again = expr == null ? null : Combine(expr, out _);
        if (again == null || again != p.Combined) { Preview = null; Safe.Raise(Changed); return "the applied filter changed after the preview; preview it again"; }
        if (again.NeedsPieces && (Read(again, out var mask, out _) != null ||
            mask == null || previewMask == null || !mask.AsSpan().SequenceEqual(previewMask)))
        {
            Preview = null;
            return "the filter fields changed after the preview; preview it again";
        }
        Applied = new AppliedFilter(p.Combined, p.Kind, p.Revision, p.Count, p.Of);
        appliedMask = previewMask;
        appliedUnavailable = null;
        Preview = null;
        Safe.Raise(Changed);
        Safe.Raise(AppliedChanged);
        return null;
    }

    public void FocusApplied(FocusLayers layers)
    {
        if (Layer() is not { } layer) return;
        layers.AddPredicate(layer);
        if (layers.Focused != LayerId) layers.Focus(LayerId);
        if (layers.Inactive == InactiveMode.Normal) layers.Cycle();
    }

    // Clears the applied filter: the focus layer goes, nothing else changes.
    public void Clear() { if (Applied == null) return; Applied = null; Preview = null; Safe.Raise(Changed); }

    // filter.edit: the applied expression comes back into the editor, staged as a replacement.
    public void Edit() { if (Applied == null) return; Compose(FilterComposition.Replace); SetKind(Applied.Kind); SetText(Applied.Expr.Text); }

    // filter.preset: a preset stages its expression for review; it applies nothing.
    public bool Preset(string name)
    {
        var preset = Presets.FirstOrDefault(p => p.Name == name);
        if (preset.Name == null) return false;
        SetText(preset.Expression); return true;
    }

    // filter.save: saves exactly the applied expression, the one the view shows, never the staged one.
    public string? Save(string name)
    {
        string clean = name.Trim();
        if (clean.Length == 0) return "a saved filter needs a name";
        if (Applied == null) return "apply a filter first: Save keeps the filter the view shows";
        if (appliedUnavailable is { } reason) return reason;
        var members = Member() is { } member ? Enumerable.Range(0, poses!.Count).Select(member).ToArray() : null;
        saved.RemoveAll(s => s.Name == clean);
        saved.Add(new SavedFilter(clean, Applied.Expr.Text, Applied.Kind, Compatibility) { Members = members });
        if (Preview?.Combined.NeedsPieces == true || Applied.Expr.NeedsPieces) RefreshFields();
        else Safe.Raise(Changed);
        return null;
    }

    // Keeps a saved filter from elsewhere (an earlier binding); Load checks its state.
    public void Keep(SavedFilter filter) { saved.RemoveAll(s => s.Name == filter.Name); saved.Add(filter); RefreshFields(); }

    // filter.load: a saved filter comes back for review, staged with its kind; it applies nothing. One saved for another
    // state is refused.
    public string? Load(string name)
    {
        if (saved.FirstOrDefault(s => s.Name == name) is not { } s) return "no filter is saved as " + name;
        if (s.Compatibility != Compatibility) return s.Name + " was saved for another state";
        Compose(FilterComposition.Replace); SetKind(s.Kind); SetText(s.Expression);
        return null;
    }

    // The focus layer the applied filter is: its label and exact count, or null with none applied.
    public PredicateLayer? Layer() => Applied is not { } a ? null
        : new PredicateLayer(LayerId, "= " + a.Expr.Text + (a.Kind == FilterKind.LiveQuery ? "" : " · " + Word(a.Kind) + " at rev " + a.Revision.ToString(CultureInfo.InvariantCulture)),
            rows == null || appliedUnavailable != null ? Numeral.Pending : Numeral.Of(a.Count), rows != null && appliedUnavailable == null);

    // The preview line: the exact count and its scope, or the rejection.
    public string PreviewLine() => rows == null ? "No state is bound: the focus layer counts the bound sample state's pieces."
        : Error is { } e ? "Not applied: " + e
        : Preview is { } p ? (p.Count.ToString("N0", CultureInfo.InvariantCulture) + " of " + p.Of.ToString("N0", CultureInfo.InvariantCulture) + " pieces · "
            + Composition.ToString().ToLowerInvariant() + " · " + Word(p.Kind) + " · rev " + p.Revision.ToString(CultureInfo.InvariantCulture)
            + (p.Count == 0 ? " · it holds no piece" : ""))
        : appliedUnavailable is { } missing ? missing
        : Applied is { } a ? "Applied: = " + a.Expr.Text + " · " + a.Count.ToString("N0", CultureInfo.InvariantCulture) + " pieces"
        : "Type an = expression and preview its count";

    public static string Word(FilterKind kind) => kind switch { FilterKind.LiveQuery => "live query", FilterKind.FixedPieces => "fixed pieces", _ => "watched positions" };

    // A piece under the applied focus layer and the cycling key's mode (U5-048): members are work, the rest references
    // while dimmed and hidden while hidden; with no layer or the Normal mode every piece is work.
    public static Interactivity Of(bool member, bool layerFocused, InactiveMode mode) =>
        member || !layerFocused || mode == InactiveMode.Normal ? Interactivity.Work : mode == InactiveMode.Dimmed ? Interactivity.Reference : Interactivity.Hidden;
}
