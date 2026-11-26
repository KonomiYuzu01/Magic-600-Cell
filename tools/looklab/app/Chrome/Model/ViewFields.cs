using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// The View section's scope (ADR-D01.22, revision 6 Q3): Both (the default), Global or Local.
public enum ViewScope { Both, Global, Local }

// One View field: a number, or a choice among named values (its value is the choice's index).
public sealed record ViewFieldSpec(string Id, string Label, double Min, double Max, double Step, IReadOnlyList<string>? Choices = null)
{
    public static ViewFieldSpec Choice(string id, string label, params string[] choices) => new(id, label, 0, choices.Length - 1, 1, choices);
}

// An edit of a View field under a scope: the host writes the value to that view, or to both under Both.
public sealed record ViewChange(ViewScope Scope, string Id, double Value);

// The inspector's View section over the two main views (ADR-D01.22 with revision 6's Q3): each view keeps its own
// value of each field; the section shows the chosen scope's values (Global's under Both), and an edit writes the
// chosen view, or both under Both. A host whose two views share one set of values offers Both only. The host acts on
// Changed. One copy, shared by Look Lab (its parameters, one set for both views) and the product (the core's view
// shape per view); the View fields were Look Lab's own before, and the product had none.
public sealed class ViewFields
{
    public const string Projection = "view.projection", CellShrink = "view.cellShrink", StickerShrink = "view.stickerShrink", Frame = "view.frame";
    public static readonly IReadOnlyList<string> Views = new[] { "Global", "Local" };
    // The product's eligible fields (rev4 and Q3: projection, shrink, frame visibility). The ranges are round-3
    // placeholders inside the core's own range for a view shape.
    public static readonly IReadOnlyList<ViewFieldSpec> Product = new[]
    {
        ViewFieldSpec.Choice(Projection, "Projection", "perspective", "orthographic"),
        new ViewFieldSpec(CellShrink, "Cell shrink", .3, 1, .01),
        new ViewFieldSpec(StickerShrink, "Sticker shrink", .3, 1, .01),
        ViewFieldSpec.Choice(Frame, "Frame", "shown", "hidden"),
    };
    public InspectorModel Inspector { get; }
    public IReadOnlyList<ViewFieldSpec> Fields { get; }
    public bool PerView { get; }
    public event Action<ViewChange>? Changed;
    private readonly Dictionary<string, Dictionary<string, double>> values = new(StringComparer.Ordinal), defaults = new(StringComparer.Ordinal);

    // `value` gives each view's value at the start and `@default` its default (the value at the start when null).
    public ViewFields(InspectorModel inspector, IReadOnlyList<ViewFieldSpec> fields, bool perView, Func<string, string, double> value, Func<string, string, double>? @default = null)
    {
        Inspector = inspector; Fields = fields; PerView = perView;
        foreach (string view in Views)
        {
            values[view] = fields.ToDictionary(f => f.Id, f => Math.Clamp(value(view, f.Id), f.Min, f.Max), StringComparer.Ordinal);
            defaults[view] = fields.ToDictionary(f => f.Id, f => Math.Clamp((@default ?? value)(view, f.Id), f.Min, f.Max), StringComparer.Ordinal);
        }
        inspector.OfferScopes(perView ? new[] { ViewScope.Both, ViewScope.Global, ViewScope.Local } : new[] { ViewScope.Both });
        string shown = Shown(inspector.Scope);
        foreach (ViewFieldSpec f in fields)
        {
            double now = values[shown][f.Id], first = defaults[shown][f.Id];
            if (f.Choices is { } choices) inspector.AddChoice(InspectorModel.View, f.Id, f.Label, choices, (int)first, (int)now, false, Of(shown, f.Id));
            else inspector.AddNumber(InspectorModel.View, f.Id, f.Label, f.Min, f.Max, f.Step, first, now, false, Of(shown, f.Id));
        }
        inspector.ScopeChanged += _ => Show();
        inspector.Edited += Edited;
    }

    public double Value(string view, string id) => values[view][id];
    public double Default(string view, string id) => defaults[view][id];
    // The views an edit under the scope writes.
    public static IReadOnlyList<string> Written(ViewScope scope) => scope switch { ViewScope.Global => new[] { "Global" }, ViewScope.Local => new[] { "Local" }, _ => Views };
    // The view whose values the section shows.
    public static string Shown(ViewScope scope) => scope == ViewScope.Local ? "Local" : "Global";
    // Whether the two views hold different values of a field.
    public bool Differs(string id) => values["Global"][id] != values["Local"][id];
    // The section's note for the scope in force (InspectorPanel shows it in the View head).
    public static string Note(ViewScope scope) => scope switch { ViewScope.Global => "Global only", ViewScope.Local => "Local only", _ => "both views" };

    // A product view's shape at the start from a Look Lab local-view.json (Local's: its projection, 1 - gap and its
    // sticker shrink), or null when the text is not one.
    public static IReadOnlyDictionary<string, double>? LocalShape(string json)
    {
        try
        {
            using JsonDocument document = JsonDocument.Parse(json);
            JsonElement root = document.RootElement;
            if (root.GetProperty("format").GetString() != "magic600-look-local-view") return null;
            double projection = root.GetProperty("projection").GetString() switch { "perspective" => 0, "orthographic" => 1, _ => -1 };
            if (projection < 0) return null;
            return new Dictionary<string, double>(StringComparer.Ordinal)
            {
                [Projection] = projection, [CellShrink] = 1 - root.GetProperty("gap").GetDouble(), [StickerShrink] = root.GetProperty("stickerShrink").GetDouble(), [Frame] = 0,
            };
        }
        catch (Exception ex) when (ex is JsonException or KeyNotFoundException or InvalidOperationException) { return null; }
    }

    private Provenance Of(string view, string id) => values[view][id] == defaults[view][id] ? Provenance.Default : Provenance.Session;

    // A host's value that changed elsewhere (not an edit): kept, and shown when the scope shows that view.
    public void Sync(string view, string id, double value)
    {
        values[view][id] = value;
        if (!PerView) values[Views.First(v => v != view)][id] = value;
        if (PerView && Shown(Inspector.Scope) != view) return;
        Inspector.Rebase(id, value, defaults[view][id], Of(view, id));
    }

    // The scope changed: the section shows that view's values against that view's defaults.
    private void Show()
    {
        string view = Shown(Inspector.Scope);
        foreach (ViewFieldSpec f in Fields) Inspector.Rebase(f.Id, values[view][f.Id], defaults[view][f.Id], Of(view, f.Id));
    }

    private void Edited(InspectField field)
    {
        if (field.Section != InspectorModel.View || field.Binding != null || Fields.All(f => f.Id != field.Id)) return;
        IReadOnlyList<string> written = Written(Inspector.Scope);
        if (written.All(view => values[view][field.Id] == field.Value)) return;
        foreach (string view in written) values[view][field.Id] = field.Value;
        Safe.Raise(Changed, new ViewChange(Inspector.Scope, field.Id, field.Value));
    }
}
