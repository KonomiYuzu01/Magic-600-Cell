using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

// ADR-D02 build, track C, item 4 (ADR-D01.22 with revision 6's Q3): the inspector's View fields and their scope,
// shared by Look Lab and the product.
internal static partial class Program
{
    // Each view keeps its own values; the section shows the scope's view (Global's under Both) against that view's
    // defaults; an edit writes the scope's view, or both under Both; a choice takes its names; a host with one set of
    // values offers Both only.
    private static void D02PortViewFieldsKeepEachViewAndWriteTheScope()
    {
        static bool Near(double a, double b) => Math.Abs(a - b) < 1e-9;
        static bool Is(ViewChange c, ViewScope scope, string id, double value) => c.Scope == scope && c.Id == id && Near(c.Value, value);
        IReadOnlyDictionary<string, double> local = ViewFields.LocalShape(Read("data/local-view.json")) ?? throw new Exception("local-view.json is not read");
        Check(local[ViewFields.Projection] == 1 && Near(local[ViewFields.CellShrink], .7) && Near(local[ViewFields.StickerShrink], .6) && local[ViewFields.Frame] == 0,
            "Local's start is not local-view.json's shape");
        Check(ViewFields.LocalShape("{\"format\":\"magic600-look-opening\"}") == null && ViewFields.LocalShape("not json") == null &&
            ViewFields.LocalShape("{\"format\":\"magic600-look-local-view\",\"projection\":\"fisheye\",\"gap\":0.3,\"stickerShrink\":0.6}") == null, "a file that is not a local view is read");
        double Start(string view, string id) => view == "Local" ? local[id] : id == ViewFields.CellShrink ? .76 : id == ViewFields.StickerShrink ? .82 : 0;

        var inspector = new InspectorModel(); var changes = new List<ViewChange>();
        var views = new ViewFields(inspector, ViewFields.Product, true, Start); views.Changed += changes.Add;
        Check(inspector.Scopes.SequenceEqual(new[] { ViewScope.Both, ViewScope.Global, ViewScope.Local }) && inspector.Scope == ViewScope.Both, "the scopes are not Both, Global and Local with Both first");
        Check(inspector.Fields.Where(f => f.Section == InspectorModel.View).Select(f => f.Id).SequenceEqual(ViewFields.Product.Select(f => f.Id)), "the View section does not hold the eligible fields");
        Check(inspector[ViewFields.CellShrink].Value == .76 && inspector[ViewFields.Projection].Display == "perspective" && inspector[ViewFields.Frame].Display == "shown", "Both does not show Global's values");
        Check(ViewFields.Note(ViewScope.Both) == "both views" && ViewFields.Note(ViewScope.Local) == "Local only", "the head's note");

        // Under Both an edit writes both views, once.
        Check(inspector.Set(ViewFields.CellShrink, .5) && Near(views.Value("Global", ViewFields.CellShrink), .5) && Near(views.Value("Local", ViewFields.CellShrink), .5) &&
            changes.Count == 1 && Is(changes[0], ViewScope.Both, ViewFields.CellShrink, .5), "an edit under Both does not write both views");
        // Local shows Local's values against Local's defaults; the switch is not an edit.
        Check(inspector.SetScope(ViewScope.Local) && !inspector.SetScope(ViewScope.Local) && changes.Count == 1, "the switch to Local");
        InspectField sticker = inspector[ViewFields.StickerShrink], projection = inspector[ViewFields.Projection];
        Check(Near(sticker.Value, .6) && Near(sticker.Default, .6) && sticker.Provenance == Provenance.Default && projection.Display == "orthographic" &&
            Near(inspector[ViewFields.CellShrink].Default, .7) && inspector[ViewFields.CellShrink].Provenance == Provenance.Session, "Local does not show its own values and defaults");
        Check(inspector.Set(ViewFields.StickerShrink, .4) && Near(views.Value("Local", ViewFields.StickerShrink), .4) && Near(views.Value("Global", ViewFields.StickerShrink), .82) &&
            Is(changes[^1], ViewScope.Local, ViewFields.StickerShrink, .4), "an edit under Local reaches Global");
        // Global, and a choice typed by name.
        Check(inspector.SetScope(ViewScope.Global) && Near(inspector[ViewFields.StickerShrink].Value, .82) && inspector[ViewFields.StickerShrink].Provenance == Provenance.Default, "Global does not show its own value");
        Check(inspector.Enter(ViewFields.Projection, "Orthographic") == null && views.Value("Global", ViewFields.Projection) == 1 && views.Value("Local", ViewFields.Projection) == 1 &&
            inspector[ViewFields.Projection].Display == "orthographic" && changes[^1].Scope == ViewScope.Global, "a choice is not taken by its name");
        Check(inspector.Enter(ViewFields.Projection, "fisheye") is { } why && why.Contains("perspective, orthographic", StringComparison.Ordinal) && inspector.Enter(ViewFields.Frame, "hidden") == null &&
            views.Value("Global", ViewFields.Frame) == 1 && views.Value("Local", ViewFields.Frame) == 0, "the frame's visibility or a refused name");
        // A host's own change is shown when its view is, and is not an edit.
        int edits = changes.Count; views.Sync("Local", ViewFields.CellShrink, .9);
        Check(Near(inspector[ViewFields.CellShrink].Value, .5) && Near(views.Value("Local", ViewFields.CellShrink), .9) && changes.Count == edits, "a Local value reached Global's fields");
        Check(inspector.SetScope(ViewScope.Local) && Near(inspector[ViewFields.CellShrink].Value, .9) && changes.Count == edits, "a host's Local value is not shown under Local");
        // A reset under the scope goes to that view's default.
        Check(inspector.Reset(ViewFields.CellShrink) && Near(views.Value("Local", ViewFields.CellShrink), .7) && Near(views.Value("Global", ViewFields.CellShrink), .5), "a reset under Local");

        // One set of values for both views: Both only, and an edit writes both.
        var shared = new InspectorModel(); var one = new ViewFields(shared, new[] { new ViewFieldSpec("view.gap", "Cell gap", 0, 1, .01) }, false, (_, _) => .3);
        Check(shared.Scopes.SequenceEqual(new[] { ViewScope.Both }) && !shared.SetScope(ViewScope.Local), "a shared set offers more than Both");
        one.Sync("Global", "view.gap", .4);
        Check(Near(one.Value("Local", "view.gap"), .4) && Near(shared["view.gap"].Value, .4) && Near(shared["view.gap"].Default, .3), "a shared set keeps two values");
        bool refused = false;
        try { new InspectorModel().OfferScopes(new[] { ViewScope.Global, ViewScope.Both }); } catch (Exception) { refused = true; }
        Check(refused, "scopes without Both first are accepted");
        Check(new InspectorModel().AddChoice(InspectorModel.View, "c", "C", new[] { "a", "b" }, 0, 1).Display == "b", "a choice shows its name");
    }
}
