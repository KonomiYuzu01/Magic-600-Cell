using System;
using System.Linq;
using Magic600.Chrome;

// The Focus layer, the 0.4 Piece Filter in the default view (owner, 26 November 2026; ADR-D01 revision 4, .46),
// on wj-ia-300's pose rows: validation, exact counts, composition, kinds kept apart, presets, save and load.
internal static partial class Program
{
    private static (PieceFilterModel Model, PoseStyle Style, Func<long> Revision, Action<long> Move) FilterAt300()
    {
        PoseStyle style = PoseStyle.Read(Read("data/samples/jumble/wj-ia-300/style.json"));
        long revision = 300;
        var model = new PieceFilterModel(style.Rows, style.LatticeDomain, style.Domains.Count, () => revision, style.Digest);
        return (model, style, () => revision, r => revision = r);
    }

    // The whole expression is read before anything applies; a rejected text stays as typed, with the place it failed.
    private static void D02FocusLayerValidatesWhole()
    {
        var (model, style, _, _) = FilterAt300();
        foreach (string bad in new[] { "", "domain", "domain >= x", "height >= 2 and", "foo", "domain = 99", "(lattice", "lattice )", "height ! 2" })
        {
            model.SetText(bad);
            Check(model.Run() == null && model.Error != null && model.Staged == bad && model.Preview == null, "the expression '" + bad + "' was taken");
        }
        model.SetText("domain = 99");
        model.Run();
        Check(model.Error!.Contains("domains are 0 to " + (style.Domains.Count - 1)), "the domain error reads " + model.Error);
        Check(model.Apply(UserEvent.Click("apply")) != null && model.Applied == null, "a rejected expression applied");
        model.SetText("lattice and not (height >= 2)");
        Check(model.Run() != null && model.Error == null, "a whole expression was refused: " + model.Error);
    }

    // Counts are exact on the pose rows: they equal the sample's own domain and height counts, and complements add up.
    private static void D02FocusLayerCountsExact()
    {
        var (model, style, _, _) = FilterAt300();
        long Count(string text) { model.SetText(text); return model.Run()!.Count; }
        Check(Count("lattice") == style.InDomain(style.LatticeDomain) && Count("domain = " + style.LatticeDomain) == Count("lattice"), "the lattice count differs from the domain count");
        Check(Count("height >= 4") == style.HeightAtLeast(4) && Count("height ≥ 4") == style.HeightAtLeast(4), "the height count differs from the sample's");
        Check(Count("lattice") + Count("not lattice") == style.Pieces && model.Preview!.Of == style.Pieces, "a layer and its complement do not cover the pieces");
        Check(Count("lattice and height >= 2") == 0 || Count("lattice and height >= 2") <= Count("lattice"), "an intersection counts more than its part");
        Check(model.PreviewLine().Contains(" of " + style.Pieces.ToString("N0", System.Globalization.CultureInfo.InvariantCulture) + " pieces · replace · live query · rev 300"), "the preview line reads " + model.PreviewLine());
    }

    // Replace, intersect, union and subtract compose with the applied layer, and inclusion-exclusion holds.
    private static void D02FocusLayerComposes()
    {
        var (model, _, _, _) = FilterAt300();
        long a = 0, e = 0;
        model.SetText("not lattice"); a = model.Run()!.Count;
        Check(model.Apply(UserEvent.Click("apply")) == null && model.Applied!.Count == a, "the first layer did not apply");
        long Composed(FilterComposition c) { model.Compose(c); model.SetText("height >= 2"); return model.Run()!.Count; }
        e = Composed(FilterComposition.Replace);
        long both = Composed(FilterComposition.Intersect), either = Composed(FilterComposition.Union), rest = Composed(FilterComposition.Subtract);
        Check(both + either == a + e && rest == a - both, "the compositions count " + both + ", " + either + ", " + rest + " from " + a + " and " + e);
        Check(model.Applied!.Count == a, "a preview changed the applied layer");
        Check(model.Apply(UserEvent.Click("apply")) == null && model.Applied!.Count == rest && model.Layer()!.Label == "= not lattice and not height >= 2", "subtract applied as " + model.Layer()?.Label);
    }

    // Only a shown preview applies, on a user event, while its context holds; the count shows before apply.
    private static void D02FocusLayerPreviewBeforeApply()
    {
        var (model, _, _, move) = FilterAt300();
        model.SetText("lattice");
        Check(model.Apply(UserEvent.Click("apply")) != null && model.Applied == null, "a layer applied without its preview");
        model.Run();
        Check(model.Apply(null) != null && model.Applied == null, "a layer applied without a user event");
        model.SetText("lattice or height >= 4");
        Check(model.Apply(UserEvent.Click("apply")) != null && model.Applied == null, "a changed expression applied on an old preview");
        model.Run();
        move(301);
        Check(model.Apply(UserEvent.Click("apply")) != null && model.Applied == null && model.Error!.Contains("preview it again"), "a preview from another revision applied");
        model.Run();
        Check(model.Apply(UserEvent.Click("apply")) == null && model.Applied!.Revision == 301, "a fresh preview did not apply");
        model.Clear();
        Check(model.Applied == null && model.Layer() == null, "the layer stayed after Clear");
    }

    // A live query, fixed pieces and watched positions are kept apart: they never compose silently, and the layer says
    // which it is.
    private static void D02FocusLayerKindsApart()
    {
        var (model, _, _, _) = FilterAt300();
        model.SetText("lattice"); model.Run(); model.Apply(UserEvent.Click("apply"));
        model.SetKind(FilterKind.FixedPieces); model.Compose(FilterComposition.Intersect); model.SetText("height >= 2");
        Check(model.Run() == null && model.Error!.Contains("does not compose"), "fixed pieces composed with a live query");
        model.Compose(FilterComposition.Replace);
        Check(model.Run() != null && model.Apply(UserEvent.Click("apply")) == null, "fixed pieces did not replace the live query");
        Check(model.Layer()!.Label == "= height >= 2 · fixed pieces at rev 300", "the layer reads " + model.Layer()!.Label);
        Check(PieceFilterModel.Of(false, true, InactiveMode.Dimmed) == Interactivity.Reference && PieceFilterModel.Of(false, true, InactiveMode.Hidden) == Interactivity.Hidden
            && PieceFilterModel.Of(true, true, InactiveMode.Hidden) == Interactivity.Work && PieceFilterModel.Of(false, true, InactiveMode.Normal) == Interactivity.Work,
            "a hidden or dimmed non-member is pickable");
    }

    // Presets stage for review; Save keeps exactly the applied layer; Load stages it for review and refuses another
    // state's.
    private static void D02FocusLayerPresetsSaveLoad()
    {
        var (model, _, _, _) = FilterAt300();
        Check(model.Preset("Height ≥ 2") && model.Staged == "height >= 2" && model.Applied == null, "a preset applied or did not stage");
        Check(model.Save("mine") != null, "a layer was saved with none applied");
        model.Run(); model.Apply(UserEvent.Click("apply"));
        model.SetText("lattice");
        Check(model.Save("mine") == null && model.Saved.Single().Expression == "height >= 2" && model.Saved.Single().Kind == FilterKind.LiveQuery, "Save kept the staged expression, not the applied one");
        model.SetText("");
        Check(model.Load("mine") == null && model.Staged == "height >= 2" && model.Preview == null && model.Applied!.Expr.Text == "height >= 2", "Load applied or did not stage");
        model.Keep(new SavedFilter("other", "lattice", FilterKind.LiveQuery, "another state"));
        Check(model.Load("other") != null && model.Load("missing") != null, "a layer saved for another state loaded");
    }
}
