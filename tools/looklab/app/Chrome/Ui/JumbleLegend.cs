using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;
using Godot;

namespace Magic600.Chrome;

// B15 (ADR-D02.20, .21, .23, .24, .30): the host's tint channel. The bound state's contact graph and pose ids give each
// domain its class (sticky by domain id through the class memory, kept with the view state) and each piece its tint
// bytes; the legend with its counts shows on both view borders while a mode is on. The renderer draws the bytes
// (the shell's ShellTint); this side keeps the mode, the classes and the focus layer's members.
public sealed partial class JumbleHost
{
    // The bound state's contact graph and pose ids (each piece's pose row), with the state digest the ids belong to.
    public ContactGraph? Contact { get; private set; }
    public IReadOnlyList<int>? PoseOf { get; private set; }
    private string poseDigest = "";
    // The class memory, and the classes of the bound state's domains (null without a graph for the bound state).
    public ContactClasses ClassMemory { get; private set; } = new();
    public IReadOnlyList<int>? Classes { get; private set; }
    public IReadOnlyList<TintLegendEntry> TintLegend { get; private set; } = Array.Empty<TintLegendEntry>();
    public string TintLine { get; private set; } = "";
    // The mode, the classes or the pose ids changed: the renderer sends the bytes again.
    public event Action? TintChanged;
    private (TintMode, PoseStyle?, IReadOnlyList<int>?, IReadOnlyList<int>?) drawnTint;
    private (int Cap, bool[] Set)? straddlerSet;

    // The mode drawn: the researcher's, else the entry slot's armed contact classes, and none while Jumble is off.
    public TintMode Tint => Source.State.Mode ? JumbleTint.Mode(Settings) : TintMode.None;

    // The pose ids are the bound state's.
    private bool PosesFit => PoseOf != null && Style != null && poseDigest == Style.Digest && PoseOf.Count > 0;

    private void WireTint()
    {
        Settings.Changed += key => { if (key is JumbleTint.ModeKey or EntrySlot.ContactClasses) Retint(); };
        Source.Changed += _ => Retint();
        Naming.Changed += _ => { straddlerSet = null; Safe.Raise(TintChanged); };
    }

    // The bound state's contact graph and pose ids (the shell reads them at each binding).
    public void SetContact(ContactGraph? contact, IReadOnlyList<int>? poses, string posesDigest)
    {
        Contact = contact; PoseOf = poses; poseDigest = posesDigest;
        straddlerSet = null;
        Reclass();
        Reoverlay();
    }

    // A reopen: the view state's class memory, then the bound state's classes from it.
    public void LoadClasses(string saved)
    {
        ClassMemory = ContactClasses.Load(saved);
        Reclass();
    }

    // The palette row: a mode, or without one the next mode after the one drawn.
    public void SetTint(string? argument) =>
        JumbleTint.Set(Settings, JumbleTint.Parse(argument) ?? (TintMode)(((int)JumbleTint.Mode(Settings) + 1) % JumbleTint.Words.Count));

    // The bound state's classes, from the memory, which then remembers them (ADR-D02.21); a graph made for another
    // state gives none.
    private void Reclass()
    {
        Classes = null;
        if (Style != null && Contact != null && Contact.Digest == Style.Digest && Contact.Domains == Style.Domains.Count)
        {
            Classes = ClassMemory.Assign(Contact, Style.Domains);
            Settings.Set(JumbleTint.MemoryKey, ClassMemory.Save());
        }
        Retint();
    }

    // The legend for the mode drawn, on both borders (none while no state is bound), and the bytes again when what
    // they draw changed.
    private void Retint()
    {
        TintMode mode = Tint;
        TintLegend = Style == null ? Array.Empty<TintLegendEntry>() : JumbleTint.Legend(mode, Style, Classes);
        TintLine = Style == null ? "" : JumbleTint.Line(mode, TintLegend, Classes != null);
        Legends();
        var now = (mode, Style, Classes, PoseOf);
        if (now == drawnTint) return;
        drawnTint = now;
        Safe.Raise(TintChanged);
    }

    // The tint bytes per piece on the outline and the fill channels, null where the mode draws nothing there.
    public (byte[]? Outline, byte[]? Fill) TintBytes()
    {
        TintMode mode = Tint;
        if (mode == TintMode.None || Style is not { } style || !PosesFit) return (null, null);
        var (outline, fill) = JumbleTint.PerPose(mode, style, Classes);
        return (outline.Any(b => b != 0) ? JumbleTint.PerPiece(outline, PoseOf!) : null, fill.Any(b => b != 0) ? JumbleTint.PerPiece(fill, PoseOf!) : null);
    }

    // The focused predicate's members, piece by piece, for the renderer's dim set; null where these layers focus
    // nothing the host can draw (no bound pose ids, a layer that does not read here).
    public Func<int, bool>? FocusMember(FocusLayers layers)
    {
        if (layers.Focused is not string id || Style is not { } style || !PosesFit) return null;
        IReadOnlyList<int> poses = PoseOf!;
        bool[]? rows = id switch
        {
            JumbleFocus.Domain when style.DomainIndex(domainShown) is int d => Enumerable.Range(0, style.RowCount).Select(i => style.RowDomain(i) == d).ToArray(),
            JumbleFocus.Height when heightShown is int k => Enumerable.Range(0, style.RowCount).Select(i => style.RowHeight(i) >= k).ToArray(),
            PieceFilterModel.LayerId when Filter?.Applied is { } applied => style.Rows.Select(r => applied.Expr.Holds(r, style.LatticeDomain)).ToArray(),
            _ => null,
        };
        if (rows != null) return piece => rows[poses[piece]];
        if (id == JumbleFocus.Straddlers && StraddlerSet() is { } set) return piece => set[piece];
        return null;
    }

    // The named cap's straddlers from its record's mask (ADR-D02.30: the layer applies only at a blocked cap).
    private bool[]? StraddlerSet()
    {
        if (Naming.Named is not int cap || !PosesFit) return null;
        if (straddlerSet is { } kept && kept.Cap == cap) return kept.Set;
        if (RecordSource?.Invoke(cap) is not { } record) return null;
        using JsonDocument document = JsonDocument.Parse(record);
        if (!document.RootElement.TryGetProperty("masks", out JsonElement masks) || !masks.TryGetProperty("straddlers", out JsonElement mask)) return null;
        bool[] set = JumbleTint.Mask(mask, PoseOf!.Count);
        straddlerSet = (cap, set);
        return set;
    }

    // CR-14's domain row: one domain at full colour, the others dimmed. A view whose cycle shows everything dims.
    private void DimOthers()
    {
        foreach (FocusLayers layers in Layers)
            if (layers.Focused == JumbleFocus.Domain && layers.Inactive == InactiveMode.Normal) layers.Cycle();
    }

    // The legend row on Global's and Local's borders while a mode is on; Local draws the same tint for its cap.
    private void Legends()
    {
        if (!GodotObject.IsInstanceValid(frame)) return;
        foreach (ViewFrame? view in new[] { frame.Global, frame.Local })
        {
            if (view == null || !GodotObject.IsInstanceValid(view)) continue;
            TintLegendRow? row = view.Readouts.GetNodeOrNull<TintLegendRow>(TintLegendRow.NodeName);
            if (!JumbleTint.LegendKept(Tint, row != null, TintLine))
            {
                if (row != null) { view.Readouts.RemoveChild(row); row.QueueFree(); }
                continue;
            }
            if (row == null) { row = new TintLegendRow(() => frame.Tokens) { SizeFlagsHorizontal = Control.SizeFlags.ExpandFill }; view.Readouts.AddChild(row); }
            row.Show(Tint, TintLegend, TintLine);
        }
    }
}

// The tint legend on a view's border: the mode's title, then each entry's swatch (an outline square for the outline
// modes, a filled one for the height ramp) with its label and its count of pieces. Its line is the legend in words.
public sealed partial class TintLegendRow : HFlowContainer
{
    public const string NodeName = "JumbleTintLegend";
    private readonly Func<TokenSet> tokens;
    public string Line { get; private set; } = "";
    // SF6 (27 Nov 2026): the row's title and entry groups stay between shows. A legend of the same shape (entries, each
    // one's parts and their widest forms, swatch kind and size) changes its words and colours in place, and one with
    // no entries drawn hides its groups; a bind rebuilt 20-80 of these nodes at each twist before (G1: 7-19 ms of
    // Godot's layout after the frame's submission).
    private Label? title;
    private string shape = "";
    private readonly List<(HBoxContainer Group, TintSwatch Swatch, List<Label> Labels)> groups = new();

    public TintLegendRow(Func<TokenSet> tokens)
    {
        Name = NodeName; this.tokens = tokens; MouseFilter = MouseFilterEnum.Pass;
    }

    public void Show(TintMode mode, IReadOnlyList<TintLegendEntry> entries, string line)
    {
        Line = line; TooltipText = line;
        // SF5 (27 Nov 2026): a row left blank (no state bound) keeps the height it had, so the view's opening keeps its
        // size at a twist; a row with words lays out again from its parts.
        CustomMinimumSize = new Vector2(0, line.Length == 0 ? Math.Max(CustomMinimumSize.Y, Size.Y) : 0);
        TokenSet t = tokens();
        Constant(this, "h_separation", t.Px("space.2")); Constant(this, "v_separation", 0);
        bool drawn = entries.Any(e => e.Slot != null) || (mode == TintMode.OffLattice && entries.Count > 0);
        if (title == null || !IsInstanceValid(title))
        {
            foreach (Node child in GetChildren()) { RemoveChild(child); child.QueueFree(); }
            groups.Clear(); shape = "";
            title = ChromeWidgets.Text("", "MicroLabel"); AddChild(title);
        }
        title.Text = drawn ? JumbleTint.Title(mode) : line;
        Colour(title, t.C("text.dim"));
        if (!drawn)
        {
            foreach (var (group, _, _) in groups) group.Visible = false;
            return;
        }
        int size = Math.Max(8, t.Px("space.2"));
        bool outline = mode != TintMode.Height;
        List<List<LegendPart>> parts = entries.Select(e => JumbleTint.EntryParts(e).ToList()).ToList();
        string wanted = outline + "/" + size + "/" + t.Px("space.1") + "/"
            + string.Join("|", parts.Select(p => string.Join(",", p.Select(x => x.Widest ?? "-"))));
        if (wanted != shape)
        {
            foreach (var (group, _, _) in groups) { RemoveChild(group); group.QueueFree(); }
            groups.Clear(); shape = wanted;
            foreach (List<LegendPart> entryParts in parts)
            {
                var group = new HBoxContainer { MouseFilter = MouseFilterEnum.Pass };
                group.AddThemeConstantOverride("separation", t.Px("space.1"));
                var swatch = new TintSwatch(t.C("text.dim"), outline, size);
                group.AddChild(swatch);
                AddChild(group);
                var labels = new List<Label>();
                // Each number sits right-aligned in a slot as wide as its widest form (JumbleTint.EntryParts).
                foreach (LegendPart part in entryParts)
                {
                    Label label = ChromeWidgets.Text(part.Text, "MicroLabel");
                    group.AddChild(label); labels.Add(label);
                    if (part.Widest is { } widest)
                    {
                        label.HorizontalAlignment = HorizontalAlignment.Right;
                        label.CustomMinimumSize = new Vector2(ReadoutRow.Width(label, widest), 0);
                    }
                }
                groups.Add((group, swatch, labels));
            }
        }
        for (int i = 0; i < groups.Count; i++)
        {
            var (group, swatch, labels) = groups[i];
            TintLegendEntry entry = entries[i];
            swatch.Colour = entry.Slot != null ? t.C(entry.Slot) : mode == TintMode.OffLattice ? Of(JumbleTint.OffLatticeColour(t)) : t.C("text.dim");
            for (int j = 0; j < labels.Count; j++)
            {
                labels[j].Text = parts[i][j].Text;
                Colour(labels[j], t.C("text.primary"));
            }
            group.Visible = true;
        }
    }

    // A theme override set only when it changes: each set notifies the control's theme again, which lays it out again.
    internal static void Colour(Control control, Color colour)
    {
        if (!control.HasThemeColorOverride("font_color") || control.GetThemeColor("font_color") != colour) control.AddThemeColorOverride("font_color", colour);
    }

    private static void Constant(Control control, string name, int value)
    {
        if (!control.HasThemeConstantOverride(name) || control.GetThemeConstant(name) != value) control.AddThemeConstantOverride(name, value);
    }

    private static Color Of(Rgb c) => new((float)c.R, (float)c.G, (float)c.B);
}

// A legend swatch: an outline square or a filled one.
public sealed partial class TintSwatch : Control
{
    private Color colour;
    private readonly bool outline;

    // SF6: a legend of the same shape recolours its swatches in place.
    public Color Colour
    {
        get => colour;
        set { if (colour != value) { colour = value; QueueRedraw(); } }
    }

    public TintSwatch(Color colour, bool outline, int size)
    {
        this.colour = colour; this.outline = outline;
        CustomMinimumSize = new Vector2(size, size); SizeFlagsVertical = SizeFlags.ShrinkCenter; MouseFilter = MouseFilterEnum.Ignore;
    }

    public override void _Draw()
    {
        var area = new Rect2(Vector2.Zero, Size);
        if (outline) DrawRect(area.Grow(-1), colour, false, 2); else DrawRect(area, colour);
    }
}
