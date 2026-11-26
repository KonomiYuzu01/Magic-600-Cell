using System;
using System.Collections.Generic;
using Godot;

namespace Magic600.Chrome;

// The Jumble host in Solve (ADR-D02.6, .11, .12; build step B6): from a Jumble source it keeps the entry slot on the
// adopted revision's mode, draws the mode cell in the top bar and the Jumble mark on every view border, switches the
// strip and the rail to the undefined numeral on the exact lattice flag, and puts Local's sections in Local. It adds
// one panel, the cap map, to the existing bottom dock, and no layout and no dock.
public sealed class JumbleHost
{
    public IJumbleSource Source { get; }
    public ChromeSettings Settings { get; } = new();
    public EntrySlot Entry { get; }
    public LocalAccordion Local { get; } = new();
    public JumbleSurface Surface { get; }
    public CapMapPanel CapMap { get; }
    // The facts changed: the host draws its view marks again (Marks).
    public event Action? Changed;
    private readonly ChromeFrame frame;
    private readonly string unit, above;
    // The readouts' slots at this revision and the positions that changed since the last (ADR-D02.22).
    private IReadOnlyList<ReadoutSlot> totals;
    private IReadOnlySet<int> totalsMarked = new HashSet<int>();
    private IReadOnlyList<ReadoutSlot>? cap;
    private IReadOnlySet<int> capMarked = new HashSet<int>();
    private RelationStrip? strip;
    private OrbitRail? rail;
    private bool following;

    public JumbleHost(ChromeFrame frame, IJumbleSource source, IReadOnlyDictionary<string, string> names, string emptySentence, string emptyBody,
        string unit = "seams", string above = "above 6")
    {
        this.frame = frame; Source = source; this.unit = unit; this.above = above;
        totals = GlobalTotals.Pending.Slots(unit, above);
        Entry = new EntrySlot(Settings);
        Surface = new JumbleSurface(Local, () => frame.Tokens, names, emptySentence, emptyBody);
        string title = names.TryGetValue("cap_map", out string? name) ? name : "Cap map";
        CapMap = new CapMapPanel(Settings, () => frame.Tokens, title);
        frame.AddDockPanel(DockSide.Bottom, EntrySlot.CapMapPanel, title, "panel.capmap", CapMap);
        // The bottom dock is a chrome setting both ways: the entry opens the cap map there, and a panel the researcher
        // opens there is theirs, so Jumble off leaves it.
        Settings.Set("dock.bottom", frame.Model.Docks[DockSide.Bottom].Open);
        Settings.Changed += key => { if (key == "dock.bottom" && !following) Dock(); };
        frame.Model.Changed += () => { if (!following) Settings.Set("dock.bottom", frame.Model.Docks[DockSide.Bottom].Open); };
        source.Changed += Follow;
        frame.Rebuilt += Place;
        frame.Rebuilt += () => { if (GodotObject.IsInstanceValid(CapMap)) CapMap.Refresh(); };
        Follow(source.State);
    }

    // Global's totals for the bound state, or pending while none is bound (ADR-D02.22).
    public void SetTotals(GlobalTotals value)
    {
        IReadOnlyList<ReadoutSlot> slots = value.Slots(unit, above);
        totalsMarked = ReadoutChanges.Marked(totals, slots); totals = slots;
        Place();
    }

    // The named cap's readout on Local's border, or none while no cap is named.
    public void SetCap(CapReadout? value)
    {
        IReadOnlyList<ReadoutSlot>? slots = value?.Slots(unit);
        capMarked = slots == null ? new HashSet<int>() : ReadoutChanges.Marked(cap, slots); cap = slots;
        Place();
    }

    // The cap map for the bound state, or every cap pending while none is bound (ADR-D02.13, .14).
    public void SetCapMap(CapLayout? layout, CapMapModel map)
    {
        if (GodotObject.IsInstanceValid(CapMap)) CapMap.Show(layout, map);
    }

    public IReadOnlyList<ReadoutSlot> Totals => totals;
    public IReadOnlyList<ReadoutSlot>? Cap => cap;

    // The readout rows on the two borders, while Jumble is on; a rebuilt frame gets fresh rows.
    private void Place()
    {
        if (!GodotObject.IsInstanceValid(frame) || frame.Global == null || frame.Local == null) return;
        Row(frame.Global, "JumbleTotals", totals, totalsMarked, Source.State.Mode);
        Row(frame.Local, "JumbleCap", cap ?? Array.Empty<ReadoutSlot>(), capMarked, Source.State.Mode && cap != null);
    }

    private void Row(ViewFrame view, string name, IReadOnlyList<ReadoutSlot> slots, IReadOnlySet<int> marked, bool on)
    {
        if (!GodotObject.IsInstanceValid(view)) return;
        ReadoutRow? row = view.Tools.GetNodeOrNull<ReadoutRow>(name);
        if (!on)
        {
            if (row != null) { view.Tools.RemoveChild(row); row.QueueFree(); }
            return;
        }
        if (row == null) { row = new ReadoutRow(name, () => frame.Tokens); view.Tools.AddChild(row); view.Tools.MoveChild(row, 0); }
        row.Show(slots, marked);
    }

    // The strip and the rail to switch to the undefined numeral, where the frame has them.
    public void Solving(RelationStrip? strip, OrbitRail? rail)
    {
        this.strip = strip; this.rail = rail;
        Follow(Source.State);
    }

    private void Dock()
    {
        string? open = Settings.Get("dock.bottom");
        following = true;
        try { frame.Model.ShowDock(DockSide.Bottom, open != null && frame.Model.Docks[DockSide.Bottom].Panels.Contains(open) ? open : null); }
        finally { following = false; }
    }

    private void Follow(JumbleState state)
    {
        if (!GodotObject.IsInstanceValid(frame)) return;
        Entry.Follow(state);
        frame.SetModeCell(JumbleChrome.ModeCell(state), state.LatticeFlag
            ? "Jumble is on, and every piece's pose is on the lattice"
            : "Jumble is on, and some pieces are off the lattice: N and the solving numerals are undefined");
        bool undefined = JumbleNumerals.Undefined(state);
        if (strip != null && GodotObject.IsInstanceValid(strip)) strip.SetUndefined(undefined);
        if (rail != null && GodotObject.IsInstanceValid(rail)) rail.SetUndefined(undefined);
        Place();
        Safe.Raise(Changed);
    }

    // The Jumble mark on a view's border while Jumble is on: a notched-corner glyph with its word, never colour alone.
    public void Marks(ViewFrame view)
    {
        if (JumbleChrome.Marked(Source.State, new[] { view.ViewName }).Count == 0) return;
        view.AddMark(JumbleChrome.Mark, JumbleChrome.MarkWords, "accent.strong", "Jumble is on: this view shows the jumbled puzzle");
    }
}
