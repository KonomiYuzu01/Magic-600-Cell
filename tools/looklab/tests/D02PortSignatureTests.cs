using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    // ADR-D01.70: the signature built from a measured effect (a preview's) is the signature of the word it measured,
    // a measured zero stays untouched, and an effect that was not measured or not evaluated stays unknown.
    private static void D02PortSignatureFromEffectsMatchesWord()
    {
        foreach (bool byId in new[] { false, true })
        {
            IReadOnlyList<int> order = OrbitRailModel.Order(Atlas, byId);
            foreach (LibraryRecord m in new LibraryModel().Records.Where(r => r.IsMacro))
            {
                EffectSignature fromWord = LibraryModel.Signature(m.Word, order);
                EffectSignature fromEffects = LibraryModel.Signature(SampleCheck.Effects(m.Word)!, order);
                EffectSignature fromCheck = LibraryModel.Signature(SampleCheck.Check(m.Word, locked: false).Effects, order);
                Check(fromEffects.Slots.SequenceEqual(fromWord.Slots) && fromCheck.Slots.SequenceEqual(fromWord.Slots) && !fromEffects.Certified,
                    m.Name + ": the preview's signature differs from the Library row's");
            }
            EffectSignature none = LibraryModel.Signature(Array.Empty<OrbitEffect>(), order);
            Check(none.Slots.Select(s => s.Orbit).SequenceEqual(order) && none.Slots.All(s => s.Mark == SignatureMark.Unknown) && !none.Certified,
                "a preview that was not evaluated does not read every orbit unknown");
            Equal(LibraryModel.SignatureText(none), "Effect signature: every orbit unknown.");
            var effects = new[] { new OrbitEffect(SampleCheck.Orbits[0], 0, SampleCheck.Totals[0]), new OrbitEffect(SampleCheck.Orbits[2], 5, SampleCheck.Totals[2]), new OrbitEffect("not a sample orbit", 9, 9) };
            EffectSignature some = LibraryModel.Signature(effects, order);
            Check(some.Mark(SampleCheck.RailOrbits[0]) == SignatureMark.Untouched && some.Mark(SampleCheck.RailOrbits[2]) == SignatureMark.Moved &&
                some.Slots.Count(s => s.Mark == SignatureMark.Unknown) == OrbitAtlas.Count - 2, "a measured zero reads unknown, or an unmeasured orbit reads measured");
            Check(LibraryModel.SignatureText(some).Contains("unverified", StringComparison.Ordinal) && LibraryModel.SignatureText(some).Contains("33 orbits unknown", StringComparison.Ordinal),
                LibraryModel.SignatureText(some));
        }
    }

    // ADR-D01.70: the preview view and History draw the Library rows' signature marks, in the Library's rail order.
    private static void D02PortSignatureMountedInPreviewAndHistory()
    {
        string panels = Read("app/Chrome/Ui/SessionPanels.cs");
        string preview = Body(panels, "public sealed partial class PreviewPanel", "public sealed partial class HistoryPanel");
        string history = panels[panels.IndexOf("public sealed partial class HistoryPanel", StringComparison.Ordinal)..];
        Check(preview.Contains("public Func<IReadOnlyList<int>?>? RailOrder", StringComparison.Ordinal) &&
            preview.Contains("SessionWidgets.Signature(t, LibraryModel.Signature(p.Effects, order), \"preview\")", StringComparison.Ordinal), "the preview view does not draw the signature");
        Check(history.Contains("public Func<IReadOnlyList<int>?>? RailOrder", StringComparison.Ordinal) &&
            history.Contains("SessionWidgets.Signature(t, LibraryModel.Signature(entry.Moves, order), entry.Id)", StringComparison.Ordinal) &&
            history.Contains("if (signature != null) line.AddChild(signature);", StringComparison.Ordinal), "History's entries do not draw the signature");
        Check(panels.Contains("new SignatureMarks(signature, tokens) { TooltipText = LibraryModel.SignatureText(signature) }", StringComparison.Ordinal) &&
            panels.Contains("marks.SetMeta(\"slot\", \"signature.slot\");", StringComparison.Ordinal) && !panels.Contains("class SignatureMarks", StringComparison.Ordinal),
            "the signature is not the Library rows' marks in the signature.slot slot");
        string solve = Read("app/Live/LabSolve.cs");
        Check(solve.Contains("session.PreviewView.RailOrder = session.History.RailOrder = LibraryView.RailOrder;", StringComparison.Ordinal) &&
            solve.Contains("Rail.OrderChanged += session.PreviewView.Refresh; Rail.OrderChanged += session.History.Refresh;", StringComparison.Ordinal),
            "the preview view and History do not follow the rail's order");
    }
}
