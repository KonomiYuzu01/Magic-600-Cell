using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

// ADR-D02 build step B6: the Jumble host in Solve (ADR-D02.2, .6, .11, .12): the entry slot, the undefined numeral's
// switch, the marks and the free placement of the panels.
internal static partial class Program
{
    private static JumbleState D02On(long revision, bool lattice = false) => new(true, lattice, null, revision, JumbleSessionKind.Solving);

    // ADR-D02.11: Jumble off reverts only what the entry set and the researcher has not changed since.
    private static void D02EntryRevertsOnlyUnchanged()
    {
        var settings = new ChromeSettings(); settings.Set("dock.bottom", "problems"); settings.Set("tint.mode", "height");
        var entry = new EntrySlot(settings);
        entry.Follow(D02On(2));
        Check(entry.Applied && settings.Get("dock.bottom") == EntrySlot.CapMapPanel && settings.Get(EntrySlot.ContactClasses) == EntrySlot.Armed
            && settings.Get(EntrySlot.NeverHidden) == EntrySlot.Armed, "Jumble on did not apply the entry");
        settings.Set(EntrySlot.ContactClasses, "off");
        entry.Follow(JumbleState.Off(3));
        Check(!entry.Applied, "Jumble off left the entry applied");
        Check(settings.Get("dock.bottom") == "problems", "the bottom dock did not return");
        Check(settings.Get(EntrySlot.ContactClasses) == "off", "a setting the researcher changed was reverted");
        Check(settings.Get(EntrySlot.NeverHidden) == null, "the never-hidden set stayed armed");
        Check(settings.Get("tint.mode") == "height", "a setting the entry never touched changed");
        // A second revert, or Jumble off while not applied, changes nothing.
        var before = settings.Snapshot(); entry.Revert(); entry.Follow(JumbleState.Off(4));
        Check(Same(before, settings.Snapshot()), "a revert of nothing changed the settings");
    }

    // ADR-D02.11: the slot follows the adopted revision's mode, so undo, redo, a branch switch and a reopen agree.
    private static void D02EntryFollowsUndoRedoBranchReopen()
    {
        var settings = new ChromeSettings(); settings.Set("dock.bottom", "console");
        var entry = new EntrySlot(settings);
        var feed = new JumbleFeed(); feed.Changed += entry.Follow;
        IReadOnlyDictionary<string, string> off = settings.Snapshot();
        feed.Adopt(D02On(2)); IReadOnlyDictionary<string, string> on = settings.Snapshot();
        Check(entry.Applied && !Same(on, off), "jumble.on did not apply the entry");
        feed.Adopt(D02On(3, lattice: false)); feed.Adopt(D02On(4, lattice: true));
        Check(Same(on, settings.Snapshot()), "a twist under Jumble on applied the entry again");
        feed.Adopt(JumbleState.Off(5));   // undo back past jumble.on
        Check(!entry.Applied && Same(off, settings.Snapshot()), "the undo did not revert the entry");
        feed.Adopt(D02On(6));             // redo
        Check(entry.Applied && Same(on, settings.Snapshot()), "the redo did not apply the entry");
        feed.Adopt(JumbleState.Off(7));   // a branch whose head is Jumble off
        Check(Same(off, settings.Snapshot()), "the branch switch did not revert the entry");
        // A reopen at a Jumble-on head: a fresh slot over the stored settings applies once and agrees.
        var reopened = new ChromeSettings(); foreach ((string key, string value) in off) reopened.Set(key, value);
        var again = new EntrySlot(reopened); again.Follow(D02On(8)); again.Follow(D02On(9));
        Check(Same(on, reopened.Snapshot()), "a reopen did not reach the same settings");
        // The facts come from the host's state digests: the lattice flag from the exact off-lattice count.
        Check(JumbleFeed.Read("{\"digest\":\"d\",\"pose_ids_sha256\":\"a\",\"pose_lattice_sha256\":\"b\",\"off_lattice\":0}", 5) == new JumbleState(true, true, null, 5, JumbleSessionKind.Solving), "a host answer on the lattice");
        Check(JumbleFeed.Read("{\"digest\":\"d\",\"pose_ids_sha256\":\"a\",\"pose_lattice_sha256\":\"b\",\"off_lattice\":7}", 5).LatticeFlag == false, "a host answer off the lattice");
        Check(!JumbleFeed.Read("{\"digest\":\"d\",\"pose_ids_sha256\":null,\"pose_lattice_sha256\":null,\"off_lattice\":null}", 5).Mode, "a Jumble-off answer");
        Check(!JumbleFeed.Read("not json", 5).Mode, "an answer that does not read is Jumble off");
    }

    // ADR-D02.12: the entry's value is chrome only: it names chrome settings, sets no threshold or filter, and never
    // changes a projection, a layout or a shrink; applying it touches nothing else.
    private static void D02EntryIsChromeOnly()
    {
        foreach ((string key, string value) in EntrySlot.Value)
        {
            Check(EntrySlot.ChromePrefixes.Any(p => key.StartsWith(p, StringComparison.Ordinal)), key + " is not a chrome setting");
            Check(!EntrySlot.Forbidden.Any(f => key.Contains(f, StringComparison.OrdinalIgnoreCase) || value.Contains(f, StringComparison.OrdinalIgnoreCase)), key + " names a threshold, filter, projection, layout or shrink");
        }
        var settings = new ChromeSettings(); settings.Set("view.projection", "stereographic"); settings.Set("layout", "solve"); settings.Set("tint.threshold", "3");
        var changed = new List<string>(); settings.Changed += changed.Add;
        new EntrySlot(settings).Apply();
        Sequence(changed.ToArray(), EntrySlot.Value.Select(v => v.Key).ToArray(), "the keys the entry set");
        Check(settings.Get("view.projection") == "stereographic" && settings.Get("layout") == "solve" && settings.Get("tint.threshold") == "3", "the entry changed a projection, a layout or a threshold");
        Check(EntrySlot.Value.Single(v => v.Key == "dock.bottom").Value == EntrySlot.CapMapPanel, "the entry does not open the cap map in the bottom dock");
    }

    // ADR-D02.6: the strip and the rail show the undefined numeral as a whole when the exact lattice flag is false,
    // whatever N is; a pending N on the lattice stays pending, and N = 0 is a true zero.
    private static void D02UndefinedSwitchesOnLatticeFlagNotPendingN()
    {
        Check(JumbleNumerals.Shown(D02On(3, lattice: false), Numeral.Of(98048)).Kind == NumeralKind.Undefined, "an off-lattice N was not undefined");
        Check(JumbleNumerals.Shown(D02On(3, lattice: false), Numeral.Pending).Kind == NumeralKind.Undefined, "an off-lattice pending N was not undefined");
        Check(JumbleNumerals.Shown(D02On(3, lattice: true), Numeral.Pending).Kind == NumeralKind.Pending, "a pending N on the lattice switched");
        Check(JumbleNumerals.Shown(D02On(3, lattice: true), Numeral.Of(0)) == Numeral.Of(0), "N = 0 on the lattice is not a true zero");
        Check(JumbleNumerals.Shown(JumbleState.Off(3), Numeral.Of(12)) == Numeral.Of(12), "Jumble off changed a numeral");
        Check(JumbleNumerals.Undefined(D02On(3)) && !JumbleNumerals.Undefined(D02On(3, lattice: true)) && !JumbleNumerals.Undefined(JumbleState.Off(3)), "the switch reads more than the mode and the lattice flag");
        // A sample's flag is exact from its off-lattice count: the W-J state is off the lattice, the zero-twist state on it.
        Check(!SampleJumbleSource.Read(Read("data/samples/jumble/wj-ia-300/totals.json"), 1).LatticeFlag, "wj-ia-300 read as on the lattice");
        Check(SampleJumbleSource.Read(Read("data/samples/jumble/on-zero/totals.json"), 1).LatticeFlag, "on-zero read as off the lattice");
    }

    // ADR-D02.6: a Jumble mark on every view border while Jumble is on, differing in shape from the practice mark; the
    // mode cell says the mode; the design data names the mark.
    private static void D02MarkOnEveryViewBorder()
    {
        string[] views = { "global", "local", "preview" };
        Sequence(JumbleChrome.Marked(D02On(2), views).ToArray(), views, "the marked views");
        Check(JumbleChrome.Marked(JumbleState.Off(2), views).Count == 0, "Jumble off still marks a view");
        Check(JumbleChrome.Mark != "mark.practice", "the Jumble mark is the practice mark");
        string system = Read("data/design/system.json");
        Check(system.Contains("\"mark.jumble\"", StringComparison.Ordinal) && system.Contains("notched corner", StringComparison.Ordinal), "the design data does not name the Jumble mark's cue");
        Check(JumbleChrome.ModeCell(JumbleState.Off(1)) == null, "Jumble off shows a mode cell");
        Check(JumbleChrome.ModeCell(D02On(2)) == "Jumble · off the lattice" && JumbleChrome.ModeCell(D02On(2, lattice: true)) == "Jumble · on the lattice", "the mode cell's words");
    }

    // ADR-D02.2 and .6: every arrangement is a default: the cap map goes to any dock and a stored layout keeps it; Local's
    // four sections expand one at a time, each closed one with its count.
    private static void D02PanelsMoveFreely()
    {
        foreach (DockSide side in new[] { DockSide.Left, DockSide.Right, DockSide.Bottom })
        {
            var frame = new FrameModel(); frame.AddDockPanel(side, EntrySlot.CapMapPanel);
            Check(frame.ShowDock(side, EntrySlot.CapMapPanel), "the cap map does not open in the " + side + " dock");
            string stored = frame.Store();
            var reopened = new FrameModel(); reopened.AddDockPanel(side, EntrySlot.CapMapPanel);
            Check(reopened.Apply(stored) && reopened.Docks[side].Open == EntrySlot.CapMapPanel, "a stored layout lost the cap map in the " + side + " dock");
        }
        var names = new Dictionary<string, string> { ["twist_check"] = "Twist check", ["cap_covers"] = "Cap covers", ["repair_list"] = "Repair list" };
        var local = new LocalAccordion();
        Sequence(local.Sections.ToArray(), new[] { "named_cap", "twist_check", "cap_covers", "repair_list" }, "Local's sections");
        local.Toggle("cap_covers"); local.Toggle("repair_list");
        Check(local.Open == "repair_list", "two sections are open at once");
        local.Toggle("repair_list"); Check(local.Open == null, "the open section did not close");
        local.SetCount("twist_check", 3);
        Sequence(local.Sections.Select(s => local.Header(s, names)).ToArray(), new[] { "Named cap · 0", "Twist check · 3", "Cap covers · 0", "Repair list · 0" }, "the closed headers");
        RefuseArgument(() => local.Toggle("straddlers"), "not a Local section");
    }

    private static bool Same(IReadOnlyDictionary<string, string> a, IReadOnlyDictionary<string, string> b) =>
        a.Count == b.Count && a.All(kv => b.TryGetValue(kv.Key, out string? v) && v == kv.Value);
}
