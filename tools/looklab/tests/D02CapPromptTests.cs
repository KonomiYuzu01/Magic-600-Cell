using System;
using System.Linq;
using System.Reflection;
using Magic600.Chrome;

// The owner's bug report of 26 November 2026: jumble.cap.name without a number opens a prompt of the caps (no dead
// row), and a cap map cell says on hover that a click names it, and why a click names nothing when it does not.
internal static partial class Program
{
    // The prompt lists every cap in cap-id order with its N_c, with nothing highlighted; the human-solve boundary:
    // the program never ranks or pre-selects a cap.
    private static void D02CapPromptListsCapsInIdOrder()
    {
        CapMapModel map = Rim("ia-40");
        var prompt = new CapPrompt(() => map, cap => CapNaming.Refusal(cap, true, null));
        Check(prompt.Rows.Select(r => r.Cap).SequenceEqual(Enumerable.Range(0, 600)), "the prompt's rows are not caps 0 to 599 in id order");
        Check(prompt.Highlighted == null && prompt.First == 0, "a row is highlighted when the prompt opens");
        Check(prompt.Rows.All(r => r.Refusal == null), "a cap is refused while Jumble is on and none is named");
        Equal(prompt.Rows[0].Label, "cap 0 · blocked · N_c undefined", "a blocked cap's row");
        Equal(prompt.Rows[30].Label, "cap 30 · " + CapMapModel.Word(map.Class(30)) + " · N_c " + map.Nc(30), "an admissible cap's row");
        Equal(prompt.Window.Count, CapPrompt.Shown, "the shown rows");
        // Typing narrows to the ids that start with the digits, still in id order and with nothing highlighted.
        prompt.Type("15");
        Check(prompt.Rows.Select(r => r.Cap).SequenceEqual(new[] { 15 }.Concat(Enumerable.Range(150, 10))), "15 narrowed wrongly");
        Check(prompt.Highlighted == null, "typing highlighted a row");
        prompt.Type("cap 7");
        Check(prompt.Rows.Select(r => r.Cap).SequenceEqual(new[] { 7 }.Concat(Enumerable.Range(70, 10))), "cap 7 narrowed wrongly");
        prompt.Type("ca");
        Equal(prompt.Rows.Count, 600, "a start of the word cap");
        prompt.Type("x");
        Equal(prompt.Rows.Count, 0, "a text that starts no cap number");
        Check(prompt.Accept() == null && prompt.Said == "no cap is numbered x; caps are 0 to 599", "Enter on no rows said " + prompt.Said);
        // The model has no member that orders, ranks or picks a best cap.
        string[] forbidden = { "sort", "order", "filter", "rank", "best", "compare", "score", "top", "suggest", "recommend" };
        foreach (Type type in new[] { typeof(CapPrompt), typeof(CapPromptRow) })
            foreach (MemberInfo member in type.GetMembers(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly))
            {
                string name = member.Name.ToLowerInvariant();
                if (name.Contains("equalitycontract", StringComparison.Ordinal)) continue;
                Check(!forbidden.Any(f => name.Contains(f, StringComparison.Ordinal)), type.Name + " has a member " + member.Name);
            }
    }

    // Enter names the highlighted cap, or else the cap typed exactly; Down from none goes to the first row, Up to the
    // last; the already named cap is refused with its reason; Enter with nothing chosen says what to do.
    private static void D02CapPromptEnterNamesTheChosenCap()
    {
        CapMapModel map = Rim("ia-40");
        int? named = null;
        var prompt = new CapPrompt(() => map, cap => CapNaming.Refusal(cap, true, named));
        Check(prompt.Accept() == null && prompt.Said == "type a cap number, or choose a row with ↑↓", "Enter with nothing chosen said " + prompt.Said);
        prompt.Type("150");
        Equal(prompt.Accept(), (int?)150, "the typed cap");
        prompt.Type("15");
        Equal(prompt.Accept(), (int?)15, "the cap typed exactly among the narrowed rows");
        prompt.Move(1); prompt.Move(1);
        Equal(prompt.Highlighted, (int?)1, "two Downs");
        Equal(prompt.Accept(), (int?)150, "the highlighted cap");
        prompt.Open();
        prompt.Move(-1);
        Equal(prompt.Highlighted, (int?)599, "Up from none");
        Equal(prompt.First, 599 - CapPrompt.Shown + 1, "the shown rows follow the highlight");
        prompt.Scroll(-1000);
        Check(prompt.First == 0 && prompt.Highlighted == 599, "the wheel moved the highlight");
        named = 30;
        prompt.Open();
        Equal(prompt.Rows[30].Refusal, "cap 30 is already named; F1 › jumble.cap.unname un-names it", "the named cap's row");
        prompt.Type("30");
        Check(prompt.Accept() == null && prompt.Said == prompt.Rows[0].Refusal, "the named cap was accepted");
        prompt.Move(1);
        Check(prompt.Said == null, "a key kept the old reason");
    }

    // A cap map cell's tooltip says that a click names the cap; a click that names nothing says why: Jumble is off,
    // or the cap is already named. A blocked cap is named like any other (ADR-D02.30's named blocked cap).
    private static void D02CapMapClickSaysWhy()
    {
        Equal(CapNaming.Hint, "click to name this cap", "the hint");
        Check(CapNaming.Refusal(0, true, null) == null && CapNaming.Refusal(150, true, 30) == null, "a cap refused while Jumble is on");
        Equal(CapNaming.Refusal(150, false, null), "Jumble is off: a cap is named only while Jumble is on", "Jumble off");
        Equal(CapNaming.Refusal(150, true, 150), "cap 150 is already named; F1 › jumble.cap.unname un-names it", "the named cap");
        Equal(CapNaming.Refusal(600, true, null), "no cap is numbered 600; caps are 0 to 599", "a cap outside the map");
        Equal(CapNaming.Tip(null, "cap 0 · ring 1, place 1 · blocked"), "click to name this cap\ncap 0 · ring 1, place 1 · blocked", "a nameable cell's tip");
        Equal(CapNaming.Tip("Jumble is off: a cap is named only while Jumble is on", "cap 7"),
            "Jumble is off: a cap is named only while Jumble is on\ncap 7", "a refused cell's tip");
        // The blocked cap in ia-40's first ring is nameable.
        CapMapModel map = Rim("ia-40");
        Check(map.Class(0) == CapClass.Blocked && CapNaming.Refusal(0, true, null) == null, "a blocked cap is refused");
    }
}
