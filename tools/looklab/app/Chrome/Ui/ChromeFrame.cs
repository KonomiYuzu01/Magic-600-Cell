using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

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

    // The designed empty state of a view or panel (ADR-D01.6): what it is for and how it fills, in place.
    public static Control Empty(string title, string hint)
    {
        var centre = new CenterContainer { SizeFlagsVertical = Control.SizeFlags.ExpandFill, SizeFlagsHorizontal = Control.SizeFlags.ExpandFill };
        var column = new VBoxContainer(); centre.AddChild(column);
        var head = Text(title, "DimLabel"); head.HorizontalAlignment = HorizontalAlignment.Center; column.AddChild(head);
        var body = Text(hint, "MicroLabel"); body.HorizontalAlignment = HorizontalAlignment.Center; body.AutowrapMode = TextServer.AutowrapMode.WordSmart;
        body.CustomMinimumSize = new Vector2(160, 0); column.AddChild(body);
        return centre;
    }

    public static void Detach(Node? node) { if (node != null && GodotObject.IsInstanceValid(node)) node.GetParent()?.RemoveChild(node); }
}

// One main view in its frame: the surface, the marks drawn over it by the host, and its own status line (ADR-D01.45).
public sealed partial class ViewFrame : PanelContainer
{
    public string ViewName { get; }
    public Control Slot { get; } = new() { ClipContents = true, SizeFlagsVertical = SizeFlags.ExpandFill, SizeFlagsHorizontal = SizeFlags.ExpandFill, MouseFilter = MouseFilterEnum.Pass };
    public HBoxContainer Marks { get; } = new() { MouseFilter = MouseFilterEnum.Ignore };
    public Label Status { get; } = ChromeWidgets.Text("", "DimLabel", true);
    public HBoxContainer Tools { get; } = new();
    public bool Focused { get; private set; }
    // With labels off a mark is its glyph alone; its words stay in the tooltip.
    public bool ShowLabels { get; set; } = true;
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
        var row = new HBoxContainer(); line.AddChild(row);
        var pad = new Control { CustomMinimumSize = new Vector2(tokens.Px("space.2"), 0) }; row.AddChild(pad);
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
    public void SetFocused(bool on) { Focused = on; ThemeTypeVariation = on ? "ViewFrameFocus" : "ViewFrame"; }
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
    private Keymap? keys;
    public Keymap? Keys { get => keys; set { keys = value; dirty = true; } }
    public ChromeOverlay? OpenOverlay => Overlays.GetChildren().OfType<ChromeOverlay>().LastOrDefault(o => o.IsOpen);
    // The commands that still act while an overlay is open; every other key waits until it is closed.
    public ISet<string> OverlayCommands { get; } = new HashSet<string>(StringComparer.Ordinal);
    private string Hint(string tooltip, string command, string? argument) =>
        keys?.KeysFor(command, argument) is { Length: > 0 } bound ? $"{tooltip} ({bound})" : tooltip;
    public event Action<Binding>? KeyCommand;
    // Contexts the host adds in front of the view contexts, such as "preview" while a preview waits.
    public Func<IEnumerable<string>>? Contexts { get; set; }
    private Control? strip, topStrip;
    // Labels are off by default (ADR-D01.44): tabs and marks show icons, and every word stays in a tooltip.
    private bool labels;
    public bool Labels { get => labels; set { if (labels != value) { labels = value; dirty = true; LabelsChanged?.Invoke(); } } }
    public event Action? LabelsChanged;
    private readonly Dictionary<string, (string Text, string Colour, string Tip)> badges = new(StringComparer.Ordinal);
    // A count on a dock panel's button, shown while the panel is collapsed too (ADR-D01.27).
    public void SetBadge(string panel, string? text, string colourToken = "text.dim", string tooltip = "")
    {
        if (text == null ? !badges.Remove(panel) : badges.TryGetValue(panel, out var old) && old == (text, colourToken, tooltip)) return;
        if (text != null) badges[panel] = (text, colourToken, tooltip);
        dirty = true;
    }
    public string? Badge(string panel) => badges.TryGetValue(panel, out var badge) ? badge.Text : null;
    private Label BadgeLabel(string panel)
    {
        (string text, string colour, string tip) = badges[panel];
        Label label = ChromeWidgets.Text(text, "MicroLabel"); label.AddThemeColorOverride("font_color", Tokens.C(colour)); label.TooltipText = tip; label.SetMeta("badge", panel);
        return label;
    }
    // One thin strip at the top edge of the views: the inspector's first tier (ADR-D01.22).
    public void SetTopStrip(Control control) { topStrip = control; dirty = true; }
    // One slim strip under the views and above the bottom dock: the commit bar (ADR-D01.11).
    public void SetStrip(Control control) { strip = control; dirty = true; }
    // Replaces the content of a dock panel or of a pane, keeping its place.
    public void SetContent(string id, Control content)
    {
        content.SizeFlagsVertical = SizeFlags.ExpandFill; content.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        int at = dockPanels.FindIndex(p => p.Id == id); Control old;
        if (at >= 0) { old = dockPanels[at].Content; dockPanels[at] = dockPanels[at] with { Content = content }; }
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
        dockPanels.Add(new DockPanel(side, id, title, glyph, content)); dirty = true;
    }

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
        Recoloured?.Invoke();
    }
    public event Action? Recoloured;

    // Applying a workspace is the solver's action; it changes the arrangement and nothing else (ADR-D01.38, ADR-D01.39).
    public bool SelectWorkspace(string id)
    {
        if (!Workspaces.Stored.TryGetValue(id, out string? stored)) return false;
        if (!Model.Apply(stored)) return false;
        Workspace = id; baseline = Model.Store(); dirty = true;
        WorkspaceSelected?.Invoke(id);
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

    public bool Press(string key)
    {
        if (KeyCapture is { } capture) { KeyCapture = null; capture(key); return true; }
        ChromeOverlay? overlay = OpenOverlay;
        if (overlay != null && overlay.Key(key)) return true;
        if (Keys?.Resolve(key, KeyContexts()) is not { } binding) return false;
        if (overlay != null && !OverlayCommands.Contains(binding.Command)) return false;
        KeyCommand?.Invoke(binding);
        return true;
    }

    public override void _UnhandledKeyInput(InputEvent input)
    {
        if (input is not InputEventKey { Pressed: true, Echo: false } key || key.Keycode is Key.Shift or Key.Ctrl or Key.Alt or Key.Meta or Key.None) return;
        if (Press(Keymap.KeyName(key.CtrlPressed, key.AltPressed, key.ShiftPressed, OS.GetKeycodeString(key.Keycode)))) GetViewport().SetInputAsHandled();
    }

    public void Flush() { if (dirty) Build(); }

    public override void _Process(double delta)
    {
        Flush();
        Vector2 mouse = GetGlobalMousePosition();
        Hover(Global.GetGlobalRect().HasPoint(mouse) ? Global : Local.GetGlobalRect().HasPoint(mouse) ? Local : null);
        if (noticeLeft > 0 && (noticeLeft -= delta) <= 0) { Notice.Visible = false; message = null; }
    }

    private Container Tight(Container container) { container.AddThemeConstantOverride("separation", 0); return container; }

    private void Build()
    {
        dirty = false; HoveredView = null;
        foreach (DockPanel panel in dockPanels) ChromeWidgets.Detach(panel.Content);
        foreach (Pane pane in panes.Values) ChromeWidgets.Detach(pane.Content);
        ChromeWidgets.Detach(globalContent); ChromeWidgets.Detach(localContent); ChromeWidgets.Detach(BarTools); ChromeWidgets.Detach(Health); ChromeWidgets.Detach(strip); ChromeWidgets.Detach(topStrip);
        if (root != null) { RemoveChild(root); root.QueueFree(); }
        Theme = ChromeTheme.Build(Tokens);
        bool focus = Model.FocusMode;
        var window = new PanelContainer { ThemeTypeVariation = "Window" }; root = window;
        AddChild(window); MoveChild(window, 0); window.SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        var column = (VBoxContainer)Tight(new VBoxContainer()); window.AddChild(column);
        if (!focus) column.AddChild(TopBar());
        var middle = (HBoxContainer)Tight(new HBoxContainer { SizeFlagsVertical = SizeFlags.ExpandFill }); column.AddChild(middle);
        if (!focus) middle.AddChild(SideDock(DockSide.Left));
        var centre = (VBoxContainer)Tight(new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill }); middle.AddChild(centre);
        Notice = ChromeWidgets.Text(Model.Refusal ?? message ?? "", "DimLabel", true); Notice.Name = "FrameNotice";
        Notice.AddThemeColorOverride("font_color", Tokens.C(Model.Refusal != null || messageWarns ? "severity.warning" : "text.dim"));
        Notice.Visible = Model.Refusal != null || message != null; if (Model.Refusal != null) noticeLeft = 6;
        centre.AddChild(Notice);
        if (topStrip != null && !focus) centre.AddChild(topStrip);
        Global = View("Global", globalContent); Local = View("Local", localContent);
        centre.AddChild(Model.OnlyPairShown ? PairControl() : Node(Model.Centre));
        if (strip != null) centre.AddChild(strip);
        if (!focus) { centre.AddChild(BottomDock()); middle.AddChild(SideDock(DockSide.Right)); }
        Rebuilt?.Invoke();
    }

    private Control TopBar()
    {
        var bar = new PanelContainer { Name = "TopBar", ThemeTypeVariation = "BarTop", CustomMinimumSize = new Vector2(0, Tokens.Px("bar.height")) };
        var row = new HBoxContainer(); bar.AddChild(row);
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
            reset.Pressed += () => SelectWorkspace(Workspace); row.AddChild(reset);
        }
        row.AddChild(new Control { SizeFlagsHorizontal = SizeFlags.ExpandFill });
        Button words = ChromeWidgets.Tool("", Labels ? "Labels are on: click for icons alone" : "Labels are off: click to show them", true); words.Name = "LabelsToggle";
        ChromeIcons.Apply(words, "labels", Tokens.Px("icon.small")); words.SetPressedNoSignal(Labels); words.Pressed += () => Labels = !Labels; row.AddChild(words);
        row.AddChild(BarTools); row.AddChild(new VSeparator()); row.AddChild(Health);
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
            button.Pressed += () => Model.ToggleDock(side, panel.Id);
            buttons.AddChild(button);
            if (badges.ContainsKey(panel.Id)) { Label badge = BadgeLabel(panel.Id); badge.MouseFilter = MouseFilterEnum.Ignore; badge.Position = new Vector2(Tokens.Px("rail.width") * .55f, 0); button.AddChild(badge); }
        }
        Control? content = null;
        if (dockPanels.FirstOrDefault(p => p.Side == side && p.Id == state.Open) is { } open)
        {
            var holder = new PanelContainer { ThemeTypeVariation = side == DockSide.Left ? "DockLeft" : "DockRight", CustomMinimumSize = new Vector2(Tokens.Px("dock.width"), 0) };
            holder.AddChild(PanelBody(open)); content = holder;
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
            var body = new MarginContainer { CustomMinimumSize = new Vector2(0, Tokens.Px("dock.height")) };
            body.AddChild(open.Content); column.AddChild(new HSeparator()); column.AddChild(body);
        }
        return holder;
    }

    private ViewFrame View(string name, Control? content)
    {
        var view = new ViewFrame(name, Tokens) { ShowLabels = Labels };
        if (content != null) view.SetContent(content); else view.Slot.AddChild(ChromeWidgets.Empty(name, "No view is attached."));
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
        split.AddChild(Global); split.AddChild(Local);
        split.DragEnded += () => { if (!Model.SetPairArrangement(Model.PairArrangement, Math.Clamp(Ratio(split), .2, .8))) dirty = true; };
        int icon = Tokens.Px("icon.small");
        Button arrange = ChromeWidgets.Tool("", side ? "Stack Global over Local" : "Put Global and Local side by side"); ChromeIcons.Apply(arrange, side ? "pair.stacked" : "pair.side", icon);
        arrange.Pressed += () => Model.SetPairArrangement(side ? "stacked" : "side", Model.PairRatio);
        Button maximise = ChromeWidgets.Tool("", Hint(Model.PairMaximised ? "Restore the other panes" : "Maximise the pair", "workspace.set", "layout:maximise"), true); ChromeIcons.Apply(maximise, Model.PairMaximised ? "pair.restore" : "pair.maximise", icon);
        maximise.SetPressedNoSignal(Model.PairMaximised); maximise.Pressed += () => Model.SetPairMaximised(!Model.PairMaximised);
        Button focus = ChromeWidgets.Tool("", Hint(Model.FocusMode ? "Leave focus mode" : "Focus mode: only the pair", "workspace.set", "layout:focus"), true); ChromeIcons.Apply(focus, "pair.focus", icon);
        focus.SetPressedNoSignal(Model.FocusMode); focus.Pressed += () => Model.SetFocusMode(!Model.FocusMode);
        Local.Tools.AddChild(arrange); Local.Tools.AddChild(maximise); Local.Tools.AddChild(focus);
        return split;
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
        body.AddChild(panes.TryGetValue(active, out Pane? pane) ? pane.Content : ChromeWidgets.Empty(active, "This pane is not available in this build."));
        return group;
    }

    public override void _ExitTree()
    {
        // Contents belong to their host; release them before the frame is freed.
        foreach (DockPanel panel in dockPanels) ChromeWidgets.Detach(panel.Content);
        foreach (Pane pane in panes.Values) ChromeWidgets.Detach(pane.Content);
        ChromeWidgets.Detach(globalContent); ChromeWidgets.Detach(localContent);
    }
}
