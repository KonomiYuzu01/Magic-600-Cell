using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// A control that takes its own keys while it has focus (ADR-D02's cap map digits and the repair list's focused-list
// context): the chrome claims them, so no key both acts there and reaches the session.
public interface IOwnKeys { }

public static class ChromeWidgets
{
    public static Label Text(string text, string variation = "", bool clip = false)
    {
        var label = new Label { Text = text, ThemeTypeVariation = variation, VerticalAlignment = VerticalAlignment.Center };
        // No text slot has a fixed width: a label that does not fit is shortened and keeps its full text on hover (ADR-D01.7).
        if (clip) { label.ClipText = true; label.TextOverrunBehavior = TextServer.OverrunBehavior.TrimEllipsis; label.TooltipText = text; label.MouseFilter = Control.MouseFilterEnum.Pass; label.SizeFlagsHorizontal = Control.SizeFlags.ExpandFill; }
        return label;
    }

    public static Button Tool(string text, string tooltip, bool toggle = false, string variation = "ToolButton") =>
        new() { Text = text, TooltipText = tooltip, ToggleMode = toggle, ThemeTypeVariation = variation, FocusMode = Control.FocusModeEnum.None };

    // The designed empty state of a view or panel (ADR-D01.6), drawn: its glyph dimmed, at most six words, and
    // the key that fills it as a key cap.
    public static Control Empty(string glyph, string words, string? key = null)
    {
        var centre = new CenterContainer { SizeFlagsVertical = Control.SizeFlags.ExpandFill, SizeFlagsHorizontal = Control.SizeFlags.ExpandFill };
        var column = new VBoxContainer { Name = "EmptyState" }; centre.AddChild(column);
        var mark = Text("", "DimLabel"); mark.HorizontalAlignment = HorizontalAlignment.Center; ChromeIcons.Apply(mark, glyph, ChromeIcons.EmptySize); column.AddChild(mark);
        var row = new HBoxContainer { Alignment = BoxContainer.AlignmentMode.Center }; column.AddChild(row);
        row.AddChild(Text(words, "MicroLabel"));
        if (!string.IsNullOrEmpty(key)) row.AddChild(KeyCap(key));
        return centre;
    }

    // A key, drawn as a key cap.
    public static Control KeyCap(string key)
    {
        var cap = new PanelContainer { ThemeTypeVariation = "Chip", MouseFilter = Control.MouseFilterEnum.Ignore };
        cap.AddChild(Text(key, "CodeLabel"));
        return cap;
    }

    public static void Detach(Node? node) { if (node != null && GodotObject.IsInstanceValid(node)) node.GetParent()?.RemoveChild(node); }
}

// One main view in its frame: the surface, the marks drawn over it by the host, and its own status line (ADR-D01.45).
public sealed partial class ViewFrame : PanelContainer
{
    public string ViewName { get; }
    public Control Slot { get; } = new() { ClipContents = true, SizeFlagsVertical = SizeFlags.ExpandFill, SizeFlagsHorizontal = SizeFlags.ExpandFill, MouseFilter = MouseFilterEnum.Pass };
    public HBoxContainer Marks { get; } = new() { MouseFilter = MouseFilterEnum.Ignore };
    // A view's readout rows: the footer's full width, above its name line, each row wrapping by whole readouts.
    public VBoxContainer Readouts { get; } = new() { Name = "Readouts", SizeFlagsHorizontal = SizeFlags.ExpandFill };
    public Label Status { get; } = ChromeWidgets.Text("", "DimLabel", true);
    // The footer's tools and readouts wrap onto a second line rather than widen the frame past the window.
    public HFlowContainer Tools { get; } = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsStretchRatio = 4, Alignment = FlowContainer.AlignmentMode.End };
    public bool Focused { get; private set; }
    // With labels off a mark is its glyph alone; its words stay in the tooltip.
    public bool ShowLabels { get; set; } = true;
    // An open view frame draws its border only: the host's picture lies beneath the window.
    public bool Open { get; set; }
    private readonly TokenSet tokens;

    public ViewFrame(string name, TokenSet tokens)
    {
        this.tokens = tokens;
        ViewName = name; Name = name + "Frame"; ThemeTypeVariation = "ViewFrame";
        SizeFlagsHorizontal = SizeFlags.ExpandFill; SizeFlagsVertical = SizeFlags.ExpandFill;
        var column = new VBoxContainer(); column.AddThemeConstantOverride("separation", 0); AddChild(column);
        column.AddChild(Slot);
        var line = new PanelContainer { ThemeTypeVariation = "BarBottom", CustomMinimumSize = new Vector2(0, tokens.Px("status.height")) };
        column.AddChild(line);
        var lines = new VBoxContainer(); lines.AddThemeConstantOverride("separation", 0); line.AddChild(lines);
        Readouts.AddThemeConstantOverride("separation", 0); lines.AddChild(Readouts);
        var row = new HBoxContainer(); lines.AddChild(row);
        var pad = new Control { CustomMinimumSize = new Vector2(tokens.Px("space.2"), 0) }; row.AddChild(pad);
        Status.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        row.AddChild(ChromeWidgets.Text(name, "AccentLabel")); row.AddChild(Status); row.AddChild(Tools);
        Marks.Position = new Vector2(tokens.Px("space.3"), tokens.Px("space.3")); Marks.AddThemeConstantOverride("separation", tokens.Px("space.2"));
    }

    public void SetContent(Control content)
    {
        ChromeWidgets.Detach(content); ChromeWidgets.Detach(Marks);
        Slot.AddChild(content); content.SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        Slot.AddChild(Marks);
    }

    // Hovered element, gestures, then set and effective values; the full line stays on hover of the line itself.
    public void SetStatus(string? hovered, IReadOnlyList<string> gestures, string? values)
    {
        Status.Text = StatusLine.Compose(hovered, gestures, values);
        Status.TooltipText = StatusLine.Compose(hovered, gestures, values, 4096);
    }

    // A mark on the view frame: a state role in its colour with its cue as a glyph and a word, never colour alone.
    public Control AddMark(string glyph, string text, string colourToken, string tooltip)
    {
        var chip = new PanelContainer { ThemeTypeVariation = "Chip", TooltipText = tooltip, MouseFilter = MouseFilterEnum.Pass };
        var row = new HBoxContainer(); chip.AddChild(row);
        Label mark = ChromeWidgets.Text(glyph, "MicroLabel"); ChromeIcons.Apply(mark, glyph, tokens.Px("icon.small")); mark.AddThemeColorOverride("font_color", tokens.C(colourToken)); row.AddChild(mark);
        if (ShowLabels) row.AddChild(ChromeWidgets.Text(text, "MicroLabel")); else chip.TooltipText = text + ": " + tooltip;
        Marks.AddChild(chip);
        return chip;
    }

    public void ClearMarks() { foreach (Node child in Marks.GetChildren()) { Marks.RemoveChild(child); child.QueueFree(); } }

    // The visible focus indicator: keys route to the hovered view first (ADR-D01.19).
    public void SetFocused(bool on) { Focused = on; ThemeTypeVariation = (Open ? "ViewFrameOpen" : "ViewFrame") + (on ? "Focus" : ""); }
}

// The fixed frame (ADR-D01.36): top bar, three docks that each show one panel at a time, and a centre pane group
// around the locked Global and Local pair. Look Lab and the product shell fill it with their own views and panels.
public sealed partial class ChromeFrame : Control
{
    private sealed record DockPanel(DockSide Side, string Id, string Title, string Glyph, Control Content);
    private sealed record Pane(string Id, string Title, Control Content);

    public FrameModel Model { get; } = new();
    public TokenSet Tokens { get; private set; }
    public WorkspaceSet Workspaces { get; }
    public string Workspace { get; private set; }
    public HBoxContainer BarTools { get; } = new();
    public Label Health { get; } = ChromeWidgets.Text("", "DimLabel");
    public ViewFrame Global { get; private set; } = null!;
    public ViewFrame Local { get; private set; } = null!;
    public ViewFrame? HoveredView { get; private set; }
    public Label Notice { get; private set; } = null!;
    public event Action? Rebuilt;
    public event Action<string>? WorkspaceSelected;
    private readonly List<DockPanel> dockPanels = new();
    private readonly Dictionary<string, Pane> panes = new(StringComparer.Ordinal);
    private Control? globalContent, localContent;
    private Control? root;
    private bool dirty;
    private string baseline = "";
    private double noticeLeft;
    private string? message; private bool messageWarns;
    private string? focusedContext;
    // In-window overlays (palette, shortcut sheet, radial menus) live above the frame and survive its rebuilds.
    public Control Overlays { get; } = new() { Name = "Overlays", MouseFilter = MouseFilterEnum.Ignore };
    // The state of each dock panel: empty, loading, error, stale or busy (ADR-D01.33).
    public PanelStates PanelStates { get; } = new();
    private Keymap? keys;
    public Keymap? Keys { get => keys; set { keys = value; dirty = true; } }
    public ChromeOverlay? OpenOverlay => Overlays.GetChildren().OfType<ChromeOverlay>().LastOrDefault(o => o.IsOpen);
    // The commands that still act while an overlay is open; every other key waits until it is closed.
    public ISet<string> OverlayCommands { get; } = new HashSet<string>(StringComparer.Ordinal);
    private string Hint(string tooltip, string command, string? argument) =>
        keys?.KeysFor(command, argument) is { Length: > 0 } bound ? $"{tooltip} ({bound})" : tooltip;
    public event Action<Binding>? KeyCommand;
    // A row the frame's own controls carry out through the host's registry (the layout reset: workspace.reset), so the
    // row's unavailable reason and echo apply; with no listener the frame carries it out itself.
    public event Action<string, string?>? RowCommand;
    // Contexts the host adds in front of the view contexts, such as "preview" while a preview waits.
    public Func<IEnumerable<string>>? Contexts { get; set; }
    private Control? strip, topStrip;
    // Labels are off by default (ADR-D01.44): tabs and marks show icons, and every word stays in a tooltip.
    private bool labels;
    public bool Labels { get => labels; set { if (labels != value) { labels = value; dirty = true; Safe.Raise(LabelsChanged); } } }
    public event Action? LabelsChanged;
    private readonly Dictionary<string, (string Text, string Colour, string Tip)> badges = new(StringComparer.Ordinal);
    // A count on a dock panel's button, shown while the panel is collapsed too (ADR-D01.27).
    public void SetBadge(string panel, string? text, string colourToken = "text.dim", string tooltip = "")
    {
        bool had = badges.ContainsKey(panel);
        if (text == null ? !badges.Remove(panel) : had && badges[panel] == (text, colourToken, tooltip)) return;
        if (text != null) badges[panel] = (text, colourToken, tooltip);
        // A badge that only changes its words is written in place: a count that changes often (the error lamp's
        // Problems count) must not lay the frame out again, or each failure that the new layout causes feeds the next.
        if (text != null && had && !dirty)
        {
            var live = badgeLabels.Where(label => IsInstanceValid(label) && label.IsInsideTree() && label.GetMeta("badge").AsString() == panel).ToList();
            if (live.Count > 0)
            {
                foreach (Label label in live) { label.Text = text; label.AddThemeColorOverride("font_color", Tokens.C(colourToken)); label.TooltipText = tooltip; }
                return;
            }
        }
        dirty = true;
    }
    public string? Badge(string panel) => badges.TryGetValue(panel, out var badge) ? badge.Text : null;
    // The badge labels of the current build, so a badge's words change in place.
    private readonly List<Label> badgeLabels = new();
    private Label BadgeLabel(string panel)
    {
        (string text, string colour, string tip) = badges[panel];
        Label label = ChromeWidgets.Text(text, "MicroLabel"); label.AddThemeColorOverride("font_color", Tokens.C(colour)); label.TooltipText = tip; label.SetMeta("badge", panel);
        badgeLabels.Add(label);
        return label;
    }
    // One thin strip at the top edge of the views: the inspector's first tier (ADR-D01.22).
    private int alerts; private string alertTip = "";
    public int Alerts => alerts;
    public event Action? AlertPressed;
    // The error lamp of the top bar: how many failures the host has counted, and what the first one was.
    public void SetAlert(int count, string tooltip) { if (count == alerts && tooltip == alertTip) return; alerts = count; alertTip = tooltip; dirty = true; }
    public void SetTopStrip(Control control) { topStrip = control; dirty = true; }
    private Control? documentRow;
    public void SetDocumentRow(Control? control) { ChromeWidgets.Detach(documentRow); documentRow = control; dirty = true; }
    // A host-drawn cell in the top bar naming the session's mode with its glyph (ADR-D02.6), or none.
    public void SetModeCell(string? text, string tooltip, string glyph = "mark.jumble")
    {
        if (text == modeCell && tooltip == modeTip && glyph == modeGlyph) return;
        modeCell = text; modeTip = tooltip; modeGlyph = glyph; dirty = true;
    }
    public string? ModeCell => modeCell;
    private string? modeCell; private string modeTip = "", modeGlyph = "mark.jumble";
    // One slim strip under the views and above the bottom dock: the commit bar (ADR-D01.11).
    public void SetStrip(Control control) { strip = control; dirty = true; }
    // One control on the shared edge of the locked pair: the relationship strip (ADR-D01.62). It adds no dock.
    public void SetSeam(Control? control)
    {
        if (control is RelationStrip) HelpTopics.Mark(control, "relationship-strip");
        seam = control; dirty = true;
    }
    private Control? seam;
    // One control along the head of the locked pair, over both views: the orbit rail (ADR-D01.64). It adds no dock.
    public void SetPairHead(Control? control) { pairHead = control; dirty = true; }
    private Control? pairHead;
    // Whether Global and Local sit side by side, so the seam runs down.
    public bool PairSideBySide => Model.PairArrangement == "side";
    // Replaces the content of a dock panel or of a pane, keeping its place.
    public void SetContent(string id, Control content)
    {
        content.SizeFlagsVertical = SizeFlags.ExpandFill; content.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        int at = dockPanels.FindIndex(p => p.Id == id); Control old;
        if (at >= 0) { old = dockPanels[at].Content; dockPanels[at] = dockPanels[at] with { Content = new PanelStateBox(id, content, PanelStates, () => Tokens) }; }
        else if (panes.TryGetValue(id, out Pane? pane)) { old = pane.Content; panes[id] = pane with { Content = content }; }
        else throw new ArgumentException("no panel or pane is named " + id);
        ChromeWidgets.Detach(old); old.QueueFree(); dirty = true;
    }

    public ChromeFrame(TokenSet tokens, WorkspaceSet workspaces)
    {
        Tokens = tokens; Workspaces = workspaces; Workspace = workspaces.Order[0];
        Name = "ChromeFrame";
        SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        Model.Changed += () => dirty = true;
        Build();
        AddChild(Overlays); Overlays.SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
    }

    public void SetMainViews(Control global, Control local) { globalContent = global; localContent = local; Build(); }

    public void AddDockPanel(DockSide side, string id, string title, string glyph, Control content)
    {
        Model.AddDockPanel(side, id);
        content.SizeFlagsVertical = SizeFlags.ExpandFill; content.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        dockPanels.Add(new DockPanel(side, id, title, glyph, new PanelStateBox(id, content, PanelStates, () => Tokens))); dirty = true;
    }

    public IReadOnlyList<string> PaneIds => panes.Keys.ToArray();

    public void AddPane(string id, string title, Control content)
    {
        content.SizeFlagsVertical = SizeFlags.ExpandFill; content.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        panes[id] = new Pane(id, title, content); dirty = true;
    }

    // A new token set. Sizes and faces need the frame rebuilt; a colour change only needs the theme, so a knob
    // can be dragged without the control under the pointer being replaced.
    public void ApplyTokens(TokenSet tokens, bool rebuild = true)
    {
        Tokens = tokens;
        if (rebuild) { Build(); return; }
        Theme = ChromeTheme.Build(tokens); Notice.AddThemeColorOverride("font_color", tokens.C("severity.warning"));
        Safe.Raise(Recoloured);
    }
    public event Action? Recoloured;

    // Applying a workspace is the solver's action; it changes the arrangement and nothing else (ADR-D01.38, ADR-D01.39).
    public bool SelectWorkspace(string id)
    {
        if (!Workspaces.Stored.TryGetValue(id, out string? stored)) return false;
        if (!Model.Apply(stored)) return false;
        Workspace = id; baseline = Model.Store(); dirty = true;
        Safe.Raise(WorkspaceSelected, id);
        return true;
    }

    // A workspace the solver has rearranged is marked on its tab and can be reset to its preset (ADR-D01.38).
    public bool WorkspaceModified => Model.Store() != baseline;
    public void MarkBaseline() { baseline = Model.Store(); dirty = true; }

    private static double Ratio(SplitContainer split)
    {
        var a = (Control)split.GetChild(0); var b = (Control)split.GetChild(1);
        double first = split.Vertical ? a.Size.Y : a.Size.X, second = split.Vertical ? b.Size.Y : b.Size.X;
        return first + second > 0 ? first / (first + second) : .5;
    }

    // Feedback in place (ADR-D01.24): one line above the views, never a dialog.
    public void Say(string text, bool warning = false)
    {
        message = text; messageWarns = warning; noticeLeft = 6;
        Notice.Text = text; Notice.TooltipText = text; Notice.Visible = true;
        Notice.AddThemeColorOverride("font_color", Tokens.C(warning ? "severity.warning" : "text.dim"));
    }

    // Keys route to the hovered view first, then to the view that was hovered last, then to the global context
    // (ADR-D01.19). A key typed into a text field never arrives here.
    public IReadOnlyList<string> KeyContexts()
    {
        var chain = new List<string>();
        if (Contexts != null && OpenOverlay == null) chain.AddRange(Contexts());
        if (HoveredView != null) chain.Add(ReferenceEquals(HoveredView, Global) ? "global-view" : "local-view");
        if (focusedContext != null && !chain.Contains(focusedContext)) chain.Add(focusedContext);
        chain.Add(Keymap.Global);
        return chain;
    }

    // While set, the next key goes to it instead of the keymap: the Scripts dock binds a key this way.
    public Action<string>? KeyCapture { get; set; }

    // Keys of the host itself, outside the command table: a lab uses them for its compare and tour keys.
    // Host keys that also work while an overlay is open (a tour key that closes the overlay itself).
    public ISet<string> HostKeysOverOverlays { get; } = new HashSet<string>(StringComparer.Ordinal);
    public IDictionary<string, Action> HostKeys { get; } = new Dictionary<string, Action>(StringComparer.Ordinal);

    // A host whose picture lies beneath a transparent window (the product shell on route B) opens the ground over
    // the Global view: every other region stays opaque chrome.
    public bool OpenGround { get; set; }
    public Rect2 Opening => OpenGround && Global != null && IsInstanceValid(Global) && Global.Slot.IsVisibleInTree() ? Global.Slot.GetGlobalRect() : default;
    // ADR-D02 B18c: a host that draws Local as well opens the top of Local's slot, a square of the slot's width capped at
    // 60% of its height; Local's content scrolls below it. While LocalOpeningShown is false (no cap named) the opening
    // collapses and Local's content, its empty sentence first, takes its place.
    public bool LocalDrawn { get; set; }
    private bool localOpeningShown;
    public bool LocalOpeningShown
    {
        get => localOpeningShown;
        set { if (value == localOpeningShown) return; localOpeningShown = value; if (localLayout != null && IsInstanceValid(localLayout)) localLayout.QueueSort(); }
    }
    private LocalOpeningLayout? localLayout;
    public Rect2 LocalOpening => OpenGround && LocalDrawn && LocalOpeningShown && localLayout != null && IsInstanceValid(localLayout) && localLayout.IsVisibleInTree()
        ? localLayout.Opening.GetGlobalRect() : default;

    // Whether the chrome takes this key. A host that also routes keys elsewhere asks first, so that no key does both.
    public bool Claims(string key, Func<Binding, bool>? can = null)
    {
        if (KeyCapture != null || OpenOverlay != null || HostKeys.ContainsKey(key)) return true;
        if (GetViewport()?.GuiGetFocusOwner() is LineEdit or TextEdit or IOwnKeys) return true;
        return Keys?.Resolve(key, KeyContexts()) is { } binding && (can == null || can(binding));
    }

    public bool Press(string key)
    {
        if (KeyCapture is { } capture) { KeyCapture = null; capture(key); return true; }
        if ((OpenOverlay == null || HostKeysOverOverlays.Contains(key)) && HostKeys.TryGetValue(key, out Action? host)) { Safe.Raise(host); return true; }
        ChromeOverlay? overlay = OpenOverlay;
        if (overlay != null && overlay.Key(key)) return true;
        if (Keys?.Resolve(key, KeyContexts()) is not { } binding) return false;
        if (overlay != null && !OverlayCommands.Contains(binding.Command)) return false;
        Safe.Raise(KeyCommand, binding);
        return true;
    }

    public override void _UnhandledKeyInput(InputEvent input)
    {
        if (input is not InputEventKey { Pressed: true, Echo: false } key || key.Keycode is Key.Shift or Key.Ctrl or Key.Alt or Key.Meta or Key.None) return;
        if (Press(Keymap.KeyName(key.CtrlPressed, key.AltPressed, key.ShiftPressed, OS.GetKeycodeString(key.Keycode)))) GetViewport().SetInputAsHandled();
    }

    public void Invalidate() => dirty = true;
    public void Flush() { if (dirty) Build(); }

    // The most of the window's width one side dock's panel takes (owner, 26 November 2026: the pair always fits).
    public const float DockShare = .16f;
    // Below these widths the right dock's panel, then the left's, folds to its rail.
    public const float RightDockBelow = 1600, LeftDockBelow = 1440;
    // Whether the side dock's panel folds to its rail at this window width.
    public bool Folds(DockSide side) => side != DockSide.Bottom && Window.X > 0 && Window.X < (side == DockSide.Right ? RightDockBelow : LeftDockBelow);
    private Vector2 builtFor;
    // The pointer as the window's own mouse events give it, in the frame's canvas, or none before the first event and
    // after the pointer leaves the window (ADR-D01.19's hovered view). The root viewport's mouse position is the system
    // cursor, which a scripted run's pointer steps never move.
    private Vector2? pointer;
    // The pointer in the frame's own coordinates: an overlay opens where the window's mouse events last put it.
    public Vector2 LocalPointer => pointer is { } at ? GetGlobalTransform().AffineInverse() * at : GetLocalMousePosition();

    public override void _Input(InputEvent input)
    {
        if (input is InputEventMouse mouse) pointer = GetCanvasTransform().AffineInverse() * mouse.Position;
    }

    public override void _Notification(int what)
    {
        if (what == NotificationWMMouseExit) pointer = null;
    }

    public override void _Process(double delta)
    {
        // A new window size lays the frame out again, so the docks follow it.
        if (GetViewportRect().Size != builtFor) { builtFor = GetViewportRect().Size; dirty = true; }
        Flush();
        Hover(pointer is not { } mouse ? null : Global.GetGlobalRect().HasPoint(mouse) ? Global : Local.GetGlobalRect().HasPoint(mouse) ? Local : null);
        if (noticeLeft > 0 && (noticeLeft -= delta) <= 0) { Notice.Text = ""; Notice.TooltipText = ""; message = null; }
    }

    private Container Tight(Container container) { container.AddThemeConstantOverride("separation", 0); return container; }

    // The window's size, or zero before the frame is in the tree (a build from the constructor): then no dock folds and
    // the docks take their token sizes, and _Process builds again once the real size is known.
    private Vector2 Window => IsInsideTree() ? GetViewportRect().Size : Vector2.Zero;

    private void Build()
    {
        dirty = false; HoveredView = null; builtFor = Window; badgeLabels.Clear();
        foreach (DockPanel panel in dockPanels) ChromeWidgets.Detach(panel.Content);
        foreach (Pane pane in panes.Values) ChromeWidgets.Detach(pane.Content);
        ChromeWidgets.Detach(globalContent); ChromeWidgets.Detach(localContent); ChromeWidgets.Detach(BarTools); ChromeWidgets.Detach(Health); ChromeWidgets.Detach(strip); ChromeWidgets.Detach(topStrip); ChromeWidgets.Detach(seam); ChromeWidgets.Detach(pairHead);
        ChromeWidgets.Detach(documentRow);
        if (root != null) { RemoveChild(root); root.QueueFree(); }
        Theme = ChromeTheme.Build(Tokens);
        bool focus = Model.FocusMode;
        var window = new PanelContainer { ThemeTypeVariation = OpenGround ? "FrameOpen" : "FrameGround" }; root = window;
        AddChild(window); MoveChild(window, 0); window.SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        if (OpenGround) window.AddChild(new GroundMask(this));
        var column = (VBoxContainer)Tight(new VBoxContainer()); window.AddChild(column);
        // Feedback in place: the notice has the quiet slot of the top bar, so it never moves the layout.
        if (Notice != null && IsInstanceValid(Notice) && Notice.GetParent() == null) Notice.QueueFree();
        Notice = ChromeWidgets.Text(Model.Refusal ?? message ?? "", "DimLabel", true); Notice.Name = "FrameNotice";
        Notice.AddThemeColorOverride("font_color", Tokens.C(Model.Refusal != null || messageWarns ? "severity.warning" : "text.dim"));
        Notice.HorizontalAlignment = HorizontalAlignment.Right;
        if (Model.Refusal != null) noticeLeft = 6;
        if (!focus) column.AddChild(TopBar());
        var middle = (HBoxContainer)Tight(new HBoxContainer { SizeFlagsVertical = SizeFlags.ExpandFill }); column.AddChild(middle);
        if (!focus) middle.AddChild(SideDock(DockSide.Left));
        var centre = (VBoxContainer)Tight(new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill }); middle.AddChild(centre);
        if (topStrip != null && !focus) centre.AddChild(topStrip);
        if (documentRow != null && !focus) centre.AddChild(documentRow);
        bool localDrawn = OpenGround && LocalDrawn;
        Global = View("Global", globalContent); Local = View("Local", localDrawn ? LocalWrap(localContent) : localContent);
        if (OpenGround) { Global.Open = true; Global.SetFocused(false); }
        if (localDrawn) { Local.Open = true; Local.SetFocused(false); }
        centre.AddChild(Model.OnlyPairShown ? PairControl() : Node(Model.Centre));
        if (strip != null) centre.AddChild(strip);
        if (!focus) { centre.AddChild(BottomDock()); middle.AddChild(SideDock(DockSide.Right)); }
        Safe.Raise(Rebuilt);
    }

    private Control TopBar()
    {
        var bar = new PanelContainer { Name = "TopBar", ThemeTypeVariation = "BarTop", CustomMinimumSize = new Vector2(0, Tokens.Px("bar.height")) };
        // The bar clips at the window's edge instead of widening the frame (owner, 26 November 2026).
        var clip = new ScrollContainer { Name = "TopBarClip", HorizontalScrollMode = ScrollContainer.ScrollMode.ShowNever, VerticalScrollMode = ScrollContainer.ScrollMode.Disabled };
        bar.AddChild(clip);
        var row = new HBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill }; clip.AddChild(row);
        row.AddChild(new Control { CustomMinimumSize = new Vector2(Tokens.Px("space.3"), 0) });
        row.AddChild(ChromeWidgets.Text("600", "AccentLabel"));
        row.AddChild(new VSeparator());
        foreach (string id in Workspaces.Order)
        {
            bool modified = id == Workspace && WorkspaceModified;
            Button tab = ChromeWidgets.Tool(Workspaces.Names[id] + (modified ? " •" : ""), Hint("Workspace: " + Workspaces.Names[id] + (modified ? " (rearranged)" : ""), "workspace.set", "layout:" + id), true, "TabButton");
            if (!Labels && ChromeIcons.Glyph("workspace." + id) != null) ChromeIcons.Apply(tab, "workspace." + id, Tokens.Px("icon.medium"));
            tab.SetPressedNoSignal(id == Workspace); tab.SetMeta("workspace", id);
            tab.Pressed += () => { if (!SelectWorkspace(id)) dirty = true; };
            row.AddChild(tab);
        }
        if (WorkspaceModified)
        {
            Button reset = ChromeWidgets.Tool("", "Reset " + Workspaces.Names[Workspace] + " to its preset"); reset.Name = "WorkspaceReset"; ChromeIcons.Apply(reset, "workspace.reset", Tokens.Px("icon.small"));
            reset.Pressed += () => { if (RowCommand != null) RowCommand("workspace.reset", null); else SelectWorkspace(Workspace); };
            row.AddChild(reset);
        }
        row.AddChild(Notice);
        if (alerts > 0)
        {
            // The error lamp: something failed while the window was open.
            Button lamp = ChromeWidgets.Tool(alerts.ToString(System.Globalization.CultureInfo.InvariantCulture), alertTip); lamp.Name = "ErrorLamp";
            ChromeIcons.Apply(lamp, "severity.error", Tokens.Px("icon.small")); lamp.Text = " " + alerts.ToString(System.Globalization.CultureInfo.InvariantCulture);
            lamp.AddThemeColorOverride("font_color", Tokens.C("severity.error")); lamp.Pressed += () => Safe.Raise(AlertPressed);
            row.AddChild(lamp);
        }
        Button words = ChromeWidgets.Tool("", Labels ? "Labels are on: click for icons alone" : "Labels are off: click to show them", true); words.Name = "LabelsToggle";
        ChromeIcons.Apply(words, "labels", Tokens.Px("icon.small")); words.SetPressedNoSignal(Labels); words.Pressed += () => Labels = !Labels; row.AddChild(words);
        if (modeCell != null)
        {
            var cell = new PanelContainer { Name = "ModeCell", ThemeTypeVariation = "Chip", TooltipText = modeTip };
            var line = new HBoxContainer(); cell.AddChild(line);
            Label glyph = ChromeWidgets.Text(modeGlyph, "MicroLabel"); ChromeIcons.Apply(glyph, modeGlyph, Tokens.Px("icon.small"));
            glyph.AddThemeColorOverride("font_color", Tokens.C("accent.strong")); line.AddChild(glyph);
            Label mode = ChromeWidgets.Text(modeCell, "MicroLabel"); mode.AddThemeColorOverride("font_color", Tokens.C("accent.strong")); line.AddChild(mode);
            row.AddChild(cell);
        }
        row.AddChild(BarTools); if (Health.Text.Length > 0) { row.AddChild(new VSeparator()); row.AddChild(Health); }
        row.AddChild(new Control { CustomMinimumSize = new Vector2(Tokens.Px("space.3"), 0) });
        return bar;
    }

    private Control PanelBody(DockPanel panel)
    {
        var body = (VBoxContainer)Tight(new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsVertical = SizeFlags.ExpandFill });
        var head = new MarginContainer(); body.AddChild(head);
        head.AddChild(ChromeWidgets.Text(panel.Title.ToUpperInvariant(), "MicroLabel"));
        body.AddChild(new HSeparator()); body.AddChild(panel.Content);
        return body;
    }

    private Control SideDock(DockSide side)
    {
        DockState state = Model.Docks[side];
        var row = (HBoxContainer)Tight(new HBoxContainer { Name = side + "Dock" });
        var rail = new PanelContainer { ThemeTypeVariation = side == DockSide.Left ? "DockLeft" : "DockRight", CustomMinimumSize = new Vector2(Tokens.Px("rail.width"), 0) };
        var buttons = new VBoxContainer(); rail.AddChild(buttons);
        foreach (DockPanel panel in dockPanels.Where(p => p.Side == side))
        {
            Button button = ChromeWidgets.Tool(panel.Glyph, panel.Title, true); ChromeIcons.Apply(button, panel.Glyph, Tokens.Px("icon.medium"));
            button.SetPressedNoSignal(state.Open == panel.Id); button.SetMeta("dock_panel", panel.Id);
            button.Name = "DockRail_" + panel.Id.Replace('.', '_'); // a script's click_control finds the rail button by name
            button.Pressed += () => Model.PressRail(side, panel.Id, Folds(side));
            buttons.AddChild(button);
            if (badges.ContainsKey(panel.Id)) { Label badge = BadgeLabel(panel.Id); badge.MouseFilter = MouseFilterEnum.Ignore; badge.Position = new Vector2(Tokens.Px("rail.width") * .55f, 0); button.AddChild(badge); }
        }
        Control? content = null;
        // A narrow window keeps the pair whole: the right dock's panel folds to its rail first, then the left's. The
        // open panel stays open in the model and returns when the window is wide enough.
        Vector2 window = Window;
        // B31: an explicit open (a rail press, a row that opens the panel) shows the panel over the fold.
        bool squeezed = Folds(side) && (state.Revealed == null || state.Revealed != state.Open);
        if (!squeezed && dockPanels.FirstOrDefault(p => p.Side == side && p.Id == state.Open) is { } open)
        {
            // A side dock is at most DockShare of the window, so the pair keeps its room in a narrow window.
            var holder = new PanelContainer { ThemeTypeVariation = side == DockSide.Left ? "DockLeft" : "DockRight", CustomMinimumSize = new Vector2(window.X > 0 ? Math.Min(Tokens.Px("dock.width"), window.X * DockShare) : Tokens.Px("dock.width"), 0) };
            // A panel wider or taller than the dock scrolls in it; it never pushes the frame past the window.
            var scroll = new ScrollContainer { Name = "DockScroll", HorizontalScrollMode = ScrollContainer.ScrollMode.Auto, VerticalScrollMode = ScrollContainer.ScrollMode.Auto };
            Control panelBody = PanelBody(open); panelBody.SizeFlagsHorizontal = SizeFlags.ExpandFill; scroll.AddChild(panelBody);
            holder.AddChild(scroll); content = holder;
        }
        if (side == DockSide.Left) { row.AddChild(rail); if (content != null) row.AddChild(content); }
        else { if (content != null) row.AddChild(content); row.AddChild(rail); }
        return row;
    }

    private Control BottomDock()
    {
        DockState state = Model.Docks[DockSide.Bottom];
        var holder = new PanelContainer { Name = "BottomDock", ThemeTypeVariation = "DockBottom" };
        var column = (VBoxContainer)Tight(new VBoxContainer()); holder.AddChild(column);
        var tabs = new HBoxContainer { CustomMinimumSize = new Vector2(0, Tokens.Px("status.height")) }; column.AddChild(tabs);
        foreach (DockPanel panel in dockPanels.Where(p => p.Side == DockSide.Bottom))
        {
            Button tab = ChromeWidgets.Tool(panel.Title, panel.Title, true, "TabButton");
            tab.SetPressedNoSignal(state.Open == panel.Id); tab.SetMeta("dock_panel", panel.Id);
            tab.Pressed += () => Model.ToggleDock(DockSide.Bottom, panel.Id);
            if (!Labels) ChromeIcons.Apply(tab, panel.Glyph, Tokens.Px("icon.small"));
            tabs.AddChild(tab);
            if (badges.ContainsKey(panel.Id)) { Label badge = BadgeLabel(panel.Id); badge.MouseFilter = MouseFilterEnum.Pass; tabs.AddChild(badge); }
        }
        if (dockPanels.FirstOrDefault(p => p.Side == DockSide.Bottom && p.Id == state.Open) is { } open)
        {
            // At most a quarter of the window's height, so the pair keeps its room in a low window.
            var body = new MarginContainer { CustomMinimumSize = new Vector2(0, Window.Y > 0 ? Math.Min(Tokens.Px("dock.height"), Window.Y * .25f) : Tokens.Px("dock.height")) };
            var scroll = new ScrollContainer { Name = "DockScroll", HorizontalScrollMode = ScrollContainer.ScrollMode.Auto, VerticalScrollMode = ScrollContainer.ScrollMode.Auto };
            open.Content.SizeFlagsHorizontal = SizeFlags.ExpandFill; open.Content.SizeFlagsVertical = SizeFlags.ExpandFill; scroll.AddChild(open.Content);
            body.AddChild(scroll); column.AddChild(new HSeparator()); column.AddChild(body);
        }
        return holder;
    }

    // B18c: Local's opening on top and its content in a scroll below, so an open section never covers the opening.
    private Control LocalWrap(Control? content)
    {
        var layout = localLayout = new LocalOpeningLayout(this) { Name = "LocalDrawn" };
        var scroll = new ScrollContainer { Name = "LocalScroll", HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };
        layout.AddChild(layout.Opening); layout.AddChild(scroll);
        if (content != null) { content.SizeFlagsHorizontal = SizeFlags.ExpandFill; scroll.AddChild(content); }
        return layout;
    }

    private ViewFrame View(string name, Control? content)
    {
        var view = new ViewFrame(name, Tokens) { ShowLabels = Labels };
        if (content != null) view.SetContent(content); else view.Slot.AddChild(ChromeWidgets.Empty("pane.close", "No view attached"));
        return view;
    }

    private void Hover(ViewFrame? view)
    {
        if (ReferenceEquals(view, HoveredView)) return;
        if (HoveredView != null && IsInstanceValid(HoveredView)) HoveredView.SetFocused(false);
        HoveredView = view; view?.SetFocused(true);
        if (view != null) focusedContext = ReferenceEquals(view, Global) ? "global-view" : "local-view";
    }

    // The locked pair: both views, always, side by side or stacked.
    private Control PairControl()
    {
        bool side = Model.PairArrangement == "side";
        SplitContainer split = side ? new HSplitContainer() : new VSplitContainer();
        split.Name = "MainPair"; split.SizeFlagsVertical = SizeFlags.ExpandFill; split.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        Global.SizeFlagsStretchRatio = (float)Model.PairRatio; Local.SizeFlagsStretchRatio = (float)(1 - Model.PairRatio);
        split.AddChild(Global);
        if (seam != null)
        {
            BoxContainer edge = (BoxContainer)Tight(side ? new HBoxContainer() : new VBoxContainer()); edge.Name = "PairSeam";
            edge.SizeFlagsHorizontal = SizeFlags.ExpandFill; edge.SizeFlagsVertical = SizeFlags.ExpandFill; edge.SizeFlagsStretchRatio = (float)(1 - Model.PairRatio);
            (seam as IPairSeam)?.Arrange(side);
            // Side by side, the seam's column scrolls when the window is lower than its readouts.
            if (side)
            {
                var seamScroll = new ScrollContainer { Name = "SeamScroll", HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled, VerticalScrollMode = ScrollContainer.ScrollMode.Auto };
                seam.SizeFlagsVertical = SizeFlags.ExpandFill; seamScroll.AddChild(seam); edge.AddChild(seamScroll);
            }
            else edge.AddChild(seam);
            edge.AddChild(Local); split.AddChild(edge);
        }
        else split.AddChild(Local);
        split.DragEnded += () => { if (!Model.SetPairArrangement(Model.PairArrangement, Math.Clamp(Ratio(split), .2, .8))) dirty = true; };
        int icon = Tokens.Px("icon.small");
        Button arrange = ChromeWidgets.Tool("", side ? "Stack Global over Local" : "Put Global and Local side by side"); ChromeIcons.Apply(arrange, side ? "pair.stacked" : "pair.side", icon);
        arrange.Pressed += () => Model.SetPairArrangement(side ? "stacked" : "side", Model.PairRatio);
        Button maximise = ChromeWidgets.Tool("", Hint(Model.PairMaximised ? "Restore the other panes" : "Maximise the pair", "workspace.set", "layout:maximise"), true); ChromeIcons.Apply(maximise, Model.PairMaximised ? "pair.restore" : "pair.maximise", icon);
        maximise.SetPressedNoSignal(Model.PairMaximised); maximise.Pressed += () => Model.SetPairMaximised(!Model.PairMaximised);
        Button focus = ChromeWidgets.Tool("", Hint(Model.FocusMode ? "Leave focus mode" : "Focus mode: only the pair", "workspace.set", "layout:focus"), true); ChromeIcons.Apply(focus, "pair.focus", icon);
        focus.SetPressedNoSignal(Model.FocusMode); focus.Pressed += () => Model.SetFocusMode(!Model.FocusMode);
        Local.Tools.AddChild(arrange); Local.Tools.AddChild(maximise); Local.Tools.AddChild(focus);
        if (pairHead == null) return split;
        var body = (VBoxContainer)Tight(new VBoxContainer { Name = "PairBody", SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsVertical = SizeFlags.ExpandFill });
        body.AddChild(pairHead); body.AddChild(split);
        return body;
    }

    private Control Node(PaneNode node)
    {
        if (node.IsSplit)
        {
            SplitContainer split = node.Horizontal ? new HSplitContainer() : new VSplitContainer();
            split.SizeFlagsVertical = SizeFlags.ExpandFill; split.SizeFlagsHorizontal = SizeFlags.ExpandFill;
            Control a = Node(node.A!), b = Node(node.B!);
            a.SizeFlagsStretchRatio = (float)node.Ratio; b.SizeFlagsStretchRatio = (float)(1 - node.Ratio);
            split.AddChild(a); split.AddChild(b);
            split.DragEnded += () => Model.Resize(node, Ratio(split));
            return split;
        }
        if (node.Tabs.Contains(FrameModel.Pair)) return PairControl();
        var group = (VBoxContainer)Tight(new VBoxContainer { SizeFlagsVertical = SizeFlags.ExpandFill, SizeFlagsHorizontal = SizeFlags.ExpandFill });
        var head = new PanelContainer { ThemeTypeVariation = "BarTop", CustomMinimumSize = new Vector2(0, Tokens.Px("status.height")) }; group.AddChild(head);
        var tabs = new HBoxContainer(); head.AddChild(tabs);
        string active = node.Tabs[Math.Clamp(node.Active, 0, node.Tabs.Count - 1)];
        foreach (string id in node.Tabs)
        {
            Button tab = ChromeWidgets.Tool(panes.TryGetValue(id, out Pane? known) ? known.Title : id, id, true, "TabButton");
            tab.SetPressedNoSignal(id == active); tab.SetMeta("pane", id);
            tab.Pressed += () => Model.Activate(id);
            tabs.AddChild(tab);
        }
        tabs.AddChild(new Control { SizeFlagsHorizontal = SizeFlags.ExpandFill });
        Button close = ChromeWidgets.Tool("", "Close " + active); ChromeIcons.Apply(close, "pane.close", Tokens.Px("icon.small")); close.Pressed += () => Model.Close(active); tabs.AddChild(close);
        var body = new PanelContainer { SizeFlagsVertical = SizeFlags.ExpandFill, SizeFlagsHorizontal = SizeFlags.ExpandFill, ClipContents = true };
        group.AddChild(body);
        body.AddChild(panes.TryGetValue(active, out Pane? pane) ? pane.Content : ChromeWidgets.Empty("pane.close", "Pane not available"));
        return group;
    }

    public override void _ExitTree()
    {
        // Contents belong to their host; release them before the frame is freed.
        foreach (DockPanel panel in dockPanels) ChromeWidgets.Detach(panel.Content);
        foreach (Pane pane in panes.Values) ChromeWidgets.Detach(pane.Content);
        ChromeWidgets.Detach(globalContent); ChromeWidgets.Detach(localContent);
        ChromeWidgets.Detach(documentRow);
    }
}

// The ground of a frame over a transparent window: the window colour everywhere but the opening.
internal sealed partial class GroundMask : Control
{
    private readonly ChromeFrame frame; private Rect2 last, lastLocal;
    public GroundMask(ChromeFrame frame) { this.frame = frame; Name = "GroundMask"; MouseFilter = MouseFilterEnum.Ignore; }

    public override void _Process(double delta)
    {
        Rect2 now = frame.Opening, local = frame.LocalOpening;
        if (now != last || local != lastLocal) { last = now; lastLocal = local; QueueRedraw(); }
    }

    // B18c: the ground with both openings cut out, Global's and Local's (ViewPanes.Ground).
    public override void _Draw()
    {
        Color ground = frame.Tokens.C("window.background");
        var all = new PaneRect(0, 0, (int)Math.Ceiling(Size.X), (int)Math.Ceiling(Size.Y));
        var holes = new List<PaneRect>();
        foreach (Rect2 open in new[] { last, lastLocal })
        {
            if (!open.HasArea()) continue;
            Vector2 at = open.Position - GlobalPosition;
            holes.Add(new PaneRect((int)Math.Round(at.X), (int)Math.Round(at.Y), (int)Math.Floor(open.Size.X), (int)Math.Floor(open.Size.Y)));
        }
        foreach (PaneRect piece in ViewPanes.Ground(all, holes)) DrawRect(new Rect2(piece.X, piece.Y, piece.Width, piece.Height), ground);
    }
}

// B18c: Local's slot with the host's drawing on top (the opening) and Local's content below it.
internal sealed partial class LocalOpeningLayout : Container
{
    private readonly ChromeFrame frame;
    public Control Opening { get; } = new() { Name = "LocalOpening", MouseFilter = MouseFilterEnum.Pass };
    public LocalOpeningLayout(ChromeFrame frame) { this.frame = frame; MouseFilter = MouseFilterEnum.Pass; }

    public override void _Notification(int what)
    {
        if (what != NotificationSortChildren || GetChildCount() < 2) return;
        int height = ViewPanes.OpeningHeight((int)Size.X, (int)Size.Y, frame.LocalOpeningShown);
        FitChildInRect(GetChild<Control>(0), new Rect2(0, 0, Size.X, height));
        FitChildInRect(GetChild<Control>(1), new Rect2(0, height, Size.X, Math.Max(0, Size.Y - height)));
    }
}
