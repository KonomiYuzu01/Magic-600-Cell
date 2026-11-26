using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

// B15 (ADR-D02.20, .21, .23, .24, .30): the one tint channel, the contact classes, the height ramp, the off-lattice
// outline and the straddler layer, on the samples' pose rows and contact graphs.
internal static partial class Program
{
    private static readonly string[] TintStates = { "wj-ia-300", "wj-ia-end", "ia-40", "deep-1", "deep-2" };

    private static (PoseStyle Style, ContactGraph Contact) TintAt(string state) =>
        (PoseStyle.Read(Read("data/samples/jumble/" + state + "/style.json")), ContactGraph.Read(Read("data/samples/jumble/" + state + "/contact.json")));

    private static void ProperColouring(ContactGraph contact, IReadOnlyList<int> classes, string label)
    {
        Equal(classes.Count, contact.Domains, label + ": classes per domain");
        for (int d = 0; d < classes.Count; d++)
        {
            Check((classes[d] == ContactClasses.Neutral) == (d == contact.Lattice), label + ": domain " + d + " is neutral exactly when it is the lattice domain");
            Check(classes[d] is >= 0 and <= ContactClasses.Overflow, label + ": domain " + d + " has class " + classes[d]);
        }
        foreach (var (a, b, _, _) in contact.Edges)
            if (a != contact.Lattice && b != contact.Lattice && classes[a] != ContactClasses.Overflow)
                Check(classes[a] != classes[b], label + ": touching domains " + a + " and " + b + " share class " + classes[a]);
    }

    // A synthetic state: domain 0 the lattice domain, the others off it; one pose row per domain.
    private static ContactGraph Graph(int domains, params (int A, int B)[] edges) =>
        new(domains, 0, edges.Select(e => (Math.Min(e.A, e.B), Math.Max(e.A, e.B), true, false)));

    private static PoseStyle StyleOf(int domains, Func<int, int> height, Func<int, bool> onLattice) => PoseStyle.Read(System.Text.Json.JsonSerializer.Serialize(new
    {
        domains = Enumerable.Range(0, domains).Select(d => "d" + d).ToArray(),
        lattice_domain = 0,
        rows = Enumerable.Range(0, domains).Select(d => new Dictionary<string, object?>
            { ["domain"] = d, ["heights"] = new[] { height(d), height(d) }, ["count"] = 10 + d, ["k"] = onLattice(d) ? d : null }).ToArray(),
    }));

    // One mode field: the entry's armed contact classes are the mode at open; a mode the researcher sets replaces it,
    // and each mode draws on its own channel with its own palette entries only.
    private static void D02TintModesExclusive()
    {
        var settings = new ChromeSettings();
        Equal(JumbleTint.Mode(settings), TintMode.None, "the mode with nothing armed");
        settings.Set(EntrySlot.ContactClasses, EntrySlot.Armed);
        Equal(JumbleTint.Mode(settings), TintMode.ContactClass, "the mode the entry arms");
        var (style, contact) = TintAt("wj-ia-300");
        int[] classes = ContactClasses.Assign(contact, style.Domains, null).Classes;
        var allowed = new Dictionary<TintMode, (byte[] Outline, byte[] Fill)>
        {
            [TintMode.None] = (new byte[] { 0 }, new byte[] { 0 }),
            [TintMode.ContactClass] = (new byte[] { 0, 3, 4, 5, 6, 7, 8, 9 }, new byte[] { 0 }),
            [TintMode.Height] = (new byte[] { 0 }, new byte[] { 0, 10, 11, 12, 13 }),
            [TintMode.OffLattice] = (new byte[] { 0, 14 }, new byte[] { 0 }),
        };
        foreach (TintMode mode in Enum.GetValues<TintMode>())
        {
            JumbleTint.Set(settings, mode);
            Equal(JumbleTint.Mode(settings), mode, "the mode set");
            Equal(JumbleTint.Parse(JumbleTint.Word(mode)), (TintMode?)mode, "the mode's word");
            Check((settings.Get(EntrySlot.ContactClasses) == EntrySlot.Armed) == (mode == TintMode.ContactClass), "the contact-class setting does not follow " + mode);
            var (outline, fill) = JumbleTint.PerPose(mode, style, classes);
            Check(outline.All(allowed[mode].Outline.Contains) && fill.All(allowed[mode].Fill.Contains), mode + " draws outside its own palette entries");
            Check(mode == TintMode.None || outline.Concat(fill).Any(b => b != 0), mode + " draws nothing at wj-ia-300");
            Check((JumbleTint.Line(mode, JumbleTint.Legend(mode, style, classes)) == "") == (mode == TintMode.None), mode + "'s legend line");
        }
        Check(JumbleTint.Parse("contact height") == null && JumbleTint.Parse("") == null, "two modes, or none, read as one");
        Equal(JumbleTint.Slots.Select(s => s.Entry).Distinct().Count(), 11, "the tinted palette entries");
    }

    // The contact classes are a proper colouring of every sample state's contact graph, with the lattice domain
    // neutral; an empty memory gives the sample builder's assignment (make_d02_samples.py's contact_classes), and the
    // rule is deterministic on graphs of any shape.
    private static void D02ContactColouringIsProper()
    {
        int offGrip = 0;
        foreach (string state in TintStates)
        {
            var (style, contact) = TintAt(state);
            Equal(contact.Domains, style.Domains.Count, state + ": the contact graph's domains");
            Equal(contact.Lattice, (int?)style.LatticeDomain, state + ": the lattice domain");
            offGrip += contact.Edges.Count(e => e.OffGrip);
            int[] classes = ContactClasses.Assign(contact, style.Domains, null).Classes;
            ProperColouring(contact, classes, state);
            Check(classes.SequenceEqual(contact.SampleClasses), state + ": the C# classes differ from the sample builder's");
        }
        // Contacts on off-grip cuts count (wj-ia-300 has 7 of its 54).
        Check(offGrip > 0, "no contact on an off-grip cut was kept");
        var random = new Random(15);
        for (int round = 0; round < 40; round++)
        {
            int n = 2 + random.Next(14);
            var edges = new List<(int, int)>();
            for (int a = 0; a < n; a++) for (int b = a + 1; b < n; b++) if (random.NextDouble() < .45) edges.Add((a, b));
            ContactGraph graph = Graph(n, edges.ToArray());
            string[] keys = Enumerable.Range(0, n).Select(d => "r" + round + "." + d).ToArray();
            int[] first = ContactClasses.Assign(graph, keys, null).Classes;
            ProperColouring(graph, first, "random graph " + round);
            Check(first.SequenceEqual(ContactClasses.Assign(graph, keys, null).Classes), "random graph " + round + ": two assignments differ");
        }
    }

    // Four outline classes at wj-ia-300 and at the deep exports, with their legend and counts.
    private static void D02FourClassesAtWj300AndDeepExports()
    {
        foreach (string state in new[] { "wj-ia-300", "deep-1", "deep-2" })
        {
            var (style, contact) = TintAt(state);
            int[] classes = ContactClasses.Assign(contact, style.Domains, null).Classes;
            Equal(classes.Where(c => c is >= 1 and <= 6).Distinct().Count(), 4, state + ": outline classes");
            IReadOnlyList<TintLegendEntry> legend = JumbleTint.Legend(TintMode.ContactClass, style, classes);
            Equal(legend.Count(e => e.Slot != null), 4, state + ": legend classes");
            Equal(legend.Sum(e => e.Pieces), 177_120L, state + ": the legend's pieces");
            Equal(legend.Where(e => e.Slot != null).Sum(e => e.Domains), contact.Domains - 1, state + ": the legend's domains");
            string line = JumbleTint.Line(TintMode.ContactClass, legend);
            Check(line.StartsWith("Contact classes · 1 ", StringComparison.Ordinal) && line.Contains("lattice neutral"), state + ": the legend line reads " + line);
        }
    }

    // ia-40's three contacts all touch the lattice domain: one class.
    private static void D02OneClassAtIa40()
    {
        var (style, contact) = TintAt("ia-40");
        Equal(contact.Edges.Count, 3, "ia-40's contacts");
        int[] classes = ContactClasses.Assign(contact, style.Domains, null).Classes;
        Equal(classes.Where(c => c is >= 1 and <= 6).Distinct().Count(), 1, "ia-40's classes");
        Equal(JumbleTint.Legend(TintMode.ContactClass, style, classes).Count(e => e.Slot != null), 1, "ia-40's legend classes");
    }

    // A class follows its domain's id: a commit keeps every class it can and moves only a domain whose remembered
    // class a touching domain kept first; an undo, a redo, a branch switch and a reopen draw each state as before.
    private static void D02ClassStickyAcrossCommitUndoRedoBranchReopen()
    {
        var memory = new ContactClasses();
        string[] a = { "L", "x", "y" }, b = { "L", "x", "y", "z" }, c = { "L", "y", "w", "x" };
        ContactGraph before = Graph(3), commit = Graph(4, (1, 2), (2, 3)), branch = Graph(4, (1, 2), (2, 3), (1, 3));
        int[] atA = memory.Assign(before, a);
        Check(atA.SequenceEqual(new[] { 0, 1, 1 }), "the first state's classes: " + string.Join(",", atA));
        int[] atB = memory.Assign(commit, b);
        Check(atB.SequenceEqual(new[] { 0, 1, 2, 1 }) && memory.Moved.SequenceEqual(new[] { 2 }), "the commit's classes: " + string.Join(",", atB) + ", moved " + string.Join(",", memory.Moved));
        Check(memory.Assign(before, a).SequenceEqual(atA), "an undo draws the first state otherwise");
        Check(memory.Assign(commit, b).SequenceEqual(atB), "a redo draws the commit otherwise");
        int[] atC = memory.Assign(branch, c);
        ProperColouring(branch, atC, "the branch");
        Check(atC[1] == 1 && atC[3] != 1, "the branch moved y (index 1) instead of x: " + string.Join(",", atC));
        Check(memory.Assign(commit, b).SequenceEqual(atB) && memory.Assign(branch, c).SequenceEqual(atC), "a branch switch draws a state otherwise");
        ContactClasses reopened = ContactClasses.Load(memory.Save());
        Check(reopened.Memory.SequenceEqual(memory.Memory), "the saved view state differs from the memory");
        Check(reopened.Assign(before, a).SequenceEqual(atA) && reopened.Assign(commit, b).SequenceEqual(atB) && reopened.Assign(branch, c).SequenceEqual(atC), "a reopen draws a state otherwise");
        // The same on the samples: wj-ia-300, then its later state, then deep-1 and back.
        var samples = new ContactClasses();
        var (s300, g300) = TintAt("wj-ia-300");
        var (sEnd, gEnd) = TintAt("wj-ia-end");
        var (sDeep, gDeep) = TintAt("deep-1");
        int[] first = samples.Assign(g300, s300.Domains), later = samples.Assign(gEnd, sEnd.Domains);
        samples.Assign(gDeep, sDeep.Domains);
        var again = ContactClasses.Load(samples.Save());
        Check(again.Assign(g300, s300.Domains).SequenceEqual(first) && again.Assign(gEnd, sEnd.Domains).SequenceEqual(later), "a sample state draws otherwise when seen again");
    }

    // Past six classes a domain takes the overflow state, which never stands for a class: no two touching domains
    // share a class, and the legend states the overflow's count.
    private static void D02OverflowNeverReusesAcrossContact()
    {
        var edges = new List<(int, int)>();
        for (int x = 1; x <= 8; x++) for (int y = x + 1; y <= 8; y++) edges.Add((x, y));
        ContactGraph clique = Graph(9, edges.ToArray());
        string[] keys = Enumerable.Range(0, 9).Select(d => "d" + d).ToArray();
        int[] classes = ContactClasses.Assign(clique, keys, null).Classes;
        ProperColouring(clique, classes, "eight touching domains");
        Check(classes.Skip(1).Where(c => c != ContactClasses.Overflow).OrderBy(c => c).SequenceEqual(Enumerable.Range(1, 6)), "the six classes are not used once each: " + string.Join(",", classes));
        Equal(classes.Count(c => c == ContactClasses.Overflow), 2, "domains in the overflow state");
        for (int d = 1; d < 9; d++)
            if (classes[d] == ContactClasses.Overflow)
                Check(clique.Neighbours(d).Select(n => classes[n]).Where(c => c is >= 1 and <= 6).Distinct().Count() == 6, "domain " + d + " overflowed with a class free");
        var memory = new ContactClasses();
        memory.Assign(clique, keys);
        Check(memory.Memory.Values.All(c => c is >= 1 and <= 6) && memory.Memory.Count == 6, "the overflow state was remembered as a class");
        PoseStyle style = StyleOf(9, _ => 0, d => d == 0);
        IReadOnlyList<TintLegendEntry> legend = JumbleTint.Legend(TintMode.ContactClass, style, classes);
        TintLegendEntry over = legend.Single(e => e.Label == "overflow");
        Equal(over.Domains, 2, "the overflow's domains");
        Check(JumbleTint.Line(TintMode.ContactClass, legend).Contains("overflow 2 domains"), "the legend does not state the overflow's count");
        var (o, _) = JumbleTint.PerPose(TintMode.ContactClass, style, classes);
        Check(Enumerable.Range(0, 9).All(d => o[d] == JumbleTint.ClassEntry(classes[d])) && o.Count(x => x == JumbleTint.OverflowEntry) == 2, "the overflow's outline");
    }

    // The off-lattice outline reads the rows' exact lattice flag: a row off the lattice at height 0 is outlined, a
    // lattice row at any height is not, and on the samples the outlined pieces are the totals' off-lattice count.
    private static void D02OffLatticeFromFlagNotHeights()
    {
        PoseStyle style = StyleOf(4, d => d == 1 ? 0 : 4, d => d is 0 or 2);
        var (outline, fill) = JumbleTint.PerPose(TintMode.OffLattice, style, null);
        Check(outline.SequenceEqual(new byte[] { 0, JumbleTint.OffLatticeEntry, 0, JumbleTint.OffLatticeEntry }) && fill.All(x => x == 0), "the off-lattice outline: " + string.Join(",", outline));
        foreach (string state in TintStates)
        {
            var (s, _) = TintAt(state);
            long off = System.Text.Json.JsonDocument.Parse(Read("data/samples/jumble/" + state + "/totals.json")).RootElement.GetProperty("off_lattice").GetInt64();
            var (o, _) = JumbleTint.PerPose(TintMode.OffLattice, s, null);
            Equal(Enumerable.Range(0, s.RowCount).Where(r => o[r] != 0).Sum(s.RowPieces), off, state + ": outlined pieces");
            Equal(JumbleTint.Legend(TintMode.OffLattice, s, null).Single().Pieces, off, state + ": the legend's count");
        }
    }

    // The straddler layer is no default: a view's focus layers start without it, and it dims only at a named blocked
    // cap with its record (wj-ia-300's cap 0), never at an unblocked one (cap 150) or with no cap named. Its members
    // are the record's straddler mask, with the record's count.
    private static void D02StraddlerLayerOnlyAtBlockedCap()
    {
        var layers = new FocusLayers();
        Check(layers.Predicates.Count == 0 && layers.Focused == null && layers.Inactive == InactiveMode.Normal, "a view starts with a predicate layer");
        string blocked = Read("data/samples/jumble/wj-ia-300/cap-0.json"), open = Read("data/samples/jumble/wj-ia-300/cap-150.json");
        PredicateLayer atBlocked = JumbleFocus.StraddlersOf(0, blocked), atOpen = JumbleFocus.StraddlersOf(150, open), unnamed = JumbleFocus.StraddlersOf(null, null);
        Check(atBlocked.Applies && !atOpen.Applies && !unnamed.Applies, "the straddler layer applies away from a blocked cap");
        using var record = System.Text.Json.JsonDocument.Parse(blocked);
        bool[] members = JumbleTint.Mask(record.RootElement.GetProperty("masks").GetProperty("straddlers"), 177_120);
        Equal((long)members.Count(m => m), atBlocked.Count.Value, "the straddler mask's members");
        foreach (var (layer, dims) in new[] { (atBlocked, true), (atOpen, false), (unnamed, false) })
        {
            var view = new FocusLayers();
            view.AddPredicate(layer);
            view.Focus(layer.Id);
            view.Cycle();
            Equal(view.Dimming, dims, "the straddler layer dims at " + layer.Count.Word);
        }
    }
}
