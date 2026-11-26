using System;
using System.Linq;
using Magic600.Chrome;

// ADR-D02.17 (build step B20): the cut slice's pure model.
internal static partial class Program
{
    private static void D02SliceNamesTwistOnReleaseOnly()
    {
        var slice = new CutSlice();
        Check(slice.Press(150, Math.PI), "an admissible cap's preview did not start");
        Check(slice.Ghost && slice.Named == null, "a twist was named while the preview key is held");
        var named = slice.Release();
        Equal(named, new NamedTwist(150, 1, Math.PI), "the released preview's twist");
        Equal(slice.Named, named, "the named twist");
        Check(!slice.Ghost && slice.Release() == null, "a second release named a twist again");
    }

    private static void D02BlockedCapHasNoGhost()
    {
        var slice = new CutSlice();
        Check(!slice.Press(0, null), "a blocked cap started a preview");
        Check(!slice.Ghost && slice.Release() == null && slice.Named == null, "a blocked cap drew a ghost or named a twist");
        Equal(slice.Refusal, "Cap 0 is blocked: no turn preview, only its straddlers.", "the blocked cap's words");
        Check(slice.Line(0, true).Contains("straddlers only, no ghost"), "the blocked cap's line");
    }

    private static void D02SplitCopiesShareRotationScaleShell()
    {
        var slice = new CutSlice();
        var local = new PaneSpec("Local", new PaneRect(10, 20, 301, 200));
        var one = slice.Panes(local);
        Check(one.Count == 1 && one[0].Layer == SliceLayer.Both && one[0].Pane == local, "the unsplit slice is not one pane");
        slice.Widen(2);
        slice.HoldSplit();
        var two = slice.Panes(local);
        Check(two.Count == 2 && two[0].Layer == SliceLayer.Outside && two[1].Layer == SliceLayer.Inside, "the split's layers");
        Check(two.All(p => p.Pane.View == "Local"), "a split copy is not Local's view (its camera and scale)");
        Check(two[0].Shell == two[1].Shell && two[0].Shell == slice.Shell && two[0].Shrink == two[1].Shrink, "the copies' shells differ");
        Equal(two[0].Pane.Rect.Width + two[1].Pane.Rect.Width, 301, "the copies fill Local's opening");
        Equal(two[1].Pane.Rect.X, 10 + 150, "the right copy's place");
        Equal(slice.Shell, new SliceShell(0, 0.7), "the widened shell");
        slice.ReleaseSplit();
        Check(slice.Panes(local).Count == 1, "release did not return to one pane");
        slice.Widen(-100);
        Equal(slice.Shell.Max, CutSlice.ShellStep, "the shell narrows to one step, never to nothing");
    }
}
