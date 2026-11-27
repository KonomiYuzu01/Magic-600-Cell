using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// ADR-D01.83: the live session and the open documents in the set's order.
public sealed partial class DocumentRow : PanelContainer
{
    // 35 short marks per tab (ADR-D01.84's look-cost): narrow enough that a row of log tabs reads as a shelf at 1280 wide.
    private const float TickStep = 6;
    private readonly DocumentSet documents;
    private readonly Func<TokenSet> tokens;
    private readonly Func<SolvingState?> live;
    private readonly Func<bool> byId;
    private readonly Control clip = new() { ClipContents = true, MouseFilter = MouseFilterEnum.Ignore };
    private readonly HBoxContainer tabs = new() { MouseFilter = MouseFilterEnum.Ignore };
    private readonly Button overflow = ChromeWidgets.Tool("More…", "More documents");
    private readonly PopupPanel popup = new() { Name = "DocumentOverflow" };
    private readonly LineEdit query = new() { PlaceholderText = "Search documents", ClearButtonEnabled = true };
    private readonly VBoxContainer matches = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill };
    private readonly List<(Button Tab, float Width)> buttons = new();
    private int fit;

    public int OverflowCount => buttons.Count - fit;
    public event Action<string>? Activated;

    public DocumentRow(DocumentSet documents, Func<TokenSet> tokens, Func<SolvingState?> live, Func<bool> byId)
    {
        this.documents = documents; this.tokens = tokens; this.live = live; this.byId = byId;
        Name = "DocumentRow"; ThemeTypeVariation = "BarTop";
        AddChild(clip); clip.AddChild(tabs); tabs.SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        tabs.AddChild(overflow);
        overflow.SetMeta("slot", "document.overflow");
        overflow.Pressed += OpenOverflow;
        clip.Resized += Fit;

        AddChild(popup);
        var column = new VBoxContainer(); popup.AddChild(column);
        column.AddChild(query);
        var scroll = new ScrollContainer { SizeFlagsVertical = SizeFlags.ExpandFill, HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };
        column.AddChild(scroll); scroll.AddChild(matches);
        query.TextChanged += _ => FilterOverflow();
    }

    public override void _Ready() => Refresh();

    public DocumentSignature Signature(Document document) => document.Signature(live(), byId());

    public void Refresh()
    {
        foreach (var (tab, _) in buttons) { tabs.RemoveChild(tab); tab.QueueFree(); }
        buttons.Clear();
        foreach (Document document in documents.Documents) buttons.Add(MakeTab(document));
        tabs.MoveChild(overflow, -1);
        clip.CustomMinimumSize = new Vector2(0, buttons.Max(item => item.Tab.CustomMinimumSize.Y));
        Fit();
    }

    private (Button, float) MakeTab(Document document)
    {
        TokenSet t = tokens();
        DocumentSignature signature = Signature(document);
        bool active = document == documents.Active;
        string tip = Tooltip(document, signature);
        Button tab = ChromeWidgets.Tool("", tip, true, "TabButton");
        if (document.Kind == DocumentKind.Session) tab.Name = "SessionDocumentTab";
        tab.SetMeta("slot", "document.tab");
        tab.SetMeta("document", document.Id);
        tab.SetPressedNoSignal(active);
        tab.Pressed += () => Safe.Raise(Activated, document.Id);
        tabs.AddChild(tab);

        var column = new VBoxContainer { MouseFilter = MouseFilterEnum.Ignore };
        var head = new HBoxContainer { MouseFilter = MouseFilterEnum.Ignore };
        tab.AddChild(column); column.AddChild(head);
        column.AddThemeConstantOverride("separation", t.Px("space.1"));
        column.SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        int pad = t.Px("space.3"), inset = t.Px("space.1");
        column.OffsetLeft = pad; column.OffsetRight = -pad;
        column.OffsetTop = inset; column.OffsetBottom = -inset;
        Label title = ChromeWidgets.Text(document.Name, "", true);
        title.MouseFilter = MouseFilterEnum.Ignore;
        title.AddThemeColorOverride("font_color", t.C(active ? "text" : "text.dim"));
        head.AddChild(title);
        if (document.Dirty) head.AddChild(Mark("•", "severity.warning", "document.tab.dirty"));
        if (document.Pinned)
        {
            Label pin = Mark("", "accent.strong", "document.tab.pin");
            ChromeIcons.Apply(pin, "palette.pin", t.Px("icon.small"));
            head.AddChild(pin);
        }
        if (signature.Recording) head.AddChild(Mark("◉", "lamp.attention", "document.tab.recording"));

        float signatureWidth = (signature.Ticks?.Count ?? 0) * TickStep;
        if (signature.Ticks != null || signature.Lamp != null)
        {
            var marks = new Control { CustomMinimumSize = new Vector2(0, 10), MouseFilter = MouseFilterEnum.Ignore };
            marks.SetMeta("slot", signature.Ticks != null ? "document.tab.signature" : "document.tab.lamp");
            column.AddChild(marks);
            marks.Draw += () =>
            {
                if (signature.Ticks is { } cells)
                {
                    float step = Math.Min(TickStep, marks.Size.X / cells.Count);
                    for (int i = 0; i < cells.Count; i++) DrawRailTick(marks, t, cells[i], i * step, step);
                }
                else if (signature.Lamp is { } lamp) DrawLamp(marks, t, lamp);
            };
            marks.Resized += marks.QueueRedraw;
        }

        float titleWidth = title.GetThemeFont("font").GetStringSize(document.Name, fontSize: title.GetThemeFontSize("font_size")).X;
        float width = Math.Max(signatureWidth, Math.Min(titleWidth, 160) + head.GetCombinedMinimumSize().X) + 2 * pad;
        tab.CustomMinimumSize = new Vector2(Mathf.Ceil(width), column.GetCombinedMinimumSize().Y + 2 * inset);
        return (tab, tab.CustomMinimumSize.X);
    }

    private Label Mark(string glyph, string colour, string slot)
    {
        Label mark = ChromeWidgets.Text(glyph, "MicroLabel");
        mark.MouseFilter = MouseFilterEnum.Ignore;
        mark.AddThemeColorOverride("font_color", tokens().C(colour));
        mark.SetMeta("slot", slot);
        return mark;
    }

    private static string Tooltip(Document document, DocumentSignature signature)
    {
        string tip = document.Name + "\n" + signature.Tip;
        string? reason = document.Reason;
        if (reason == null && signature.Lamp == CompileLamp.Problems) reason = document.Compile?.Problems.FirstOrDefault();
        if (reason != null) tip += "\n" + reason;
        if (document.Dirty) tip += "\nUnsaved changes";
        if (document.Pinned) tip += "\nPinned";
        return tip;
    }

    private void Fit()
    {
        if (buttons.Count == 0) return;
        float separation = tabs.GetThemeConstant("separation");
        float width = clip.Size.X;
        float more = overflow.GetCombinedMinimumSize().X + separation;
        bool needsOverflow = buttons.Sum(item => item.Width) + separation * (buttons.Count - 1) > width && buttons.Count > 1;
        float remaining = Math.Max(0, width - (needsOverflow ? more : 0));
        fit = 0;
        foreach (var (tab, preferred) in buttons)
        {
            // Keep the session and the button visible even when the strip is narrower than the session's signature.
            tab.Visible = fit == 0 || preferred <= remaining;
            if (!tab.Visible) { remaining = 0; continue; }
            tab.CustomMinimumSize = new Vector2(Math.Min(preferred, remaining), tab.CustomMinimumSize.Y);
            remaining -= tab.CustomMinimumSize.X + separation;
            fit++;
        }
        overflow.Visible = needsOverflow;
        if (popup.Visible)
        {
            if (needsOverflow) FilterOverflow();
            else popup.Hide();
        }
    }

    private void OpenOverflow()
    {
        query.Text = "";
        FilterOverflow();
        const int width = 280, height = 220;
        Vector2 end = overflow.GetGlobalRect().End;
        popup.Popup(new Rect2I(Math.Max(0, (int)end.X - width), (int)end.Y, width, height));
        query.GrabFocus();
    }

    private void FilterOverflow()
    {
        foreach (Node child in matches.GetChildren()) { matches.RemoveChild(child); child.QueueFree(); }
        var hidden = documents.Overflow(fit).ToHashSet();
        foreach (Document document in documents.Search(query.Text).Where(hidden.Contains))
        {
            Button choice = ChromeWidgets.Tool(document.Name, Tooltip(document, Signature(document)), true, "TabButton");
            choice.SetPressedNoSignal(document == documents.Active);
            choice.ClipText = true;
            choice.Pressed += () => { popup.Hide(); Safe.Raise(Activated, document.Id); };
            matches.AddChild(choice);
        }
    }

    // The rail's residual track, hatch and solved mark, at the spacing of a signature tab.
    private static void DrawRailTick(CanvasItem on, TokenSet t, RailCell cell, float x, float step)
    {
        var track = new Rect2(x + 1, 3, Math.Max(0, step - 2), 4);
        if (cell.Solved)
        {
            Vector2 c = track.GetCenter();
            on.DrawPolyline(new[] { c + new Vector2(-3, 0), c + new Vector2(-1, 2.5f), c + new Vector2(3.5f, -3) }, t.C("rail.cell.solved"), 1.6f, true);
            return;
        }
        on.DrawRect(track, t.C("panel.border"), false, 1);
        if (cell.Level is { } level)
        {
            if (level > 0) on.DrawRect(new Rect2(track.Position, new Vector2(Math.Max(2, track.Size.X * (float)level), track.Size.Y)),
                t.C("rail.status." + (cell.Status == "permitted_residual" ? "permitted_residual" : "open")));
        }
        else for (float hx = track.Position.X; hx < track.End.X - 1; hx += 4)
            on.DrawLine(new Vector2(hx, track.End.Y), new Vector2(Math.Min(hx + 4, track.End.X), track.Position.Y), t.C("rail.status.unknown_frame"), 1);
    }

    // The same three shapes and roles as the top-bar readouts.
    private static void DrawLamp(CanvasItem on, TokenSet t, CompileLamp lamp)
    {
        Vector2 c = new(5, 5);
        const float r = 3;
        if (lamp == CompileLamp.NotCompiled) { on.DrawArc(c, r, 0, Mathf.Tau, 16, t.C("lamp.idle"), 1, true); return; }
        Color lit = t.C(lamp == CompileLamp.Clean ? "lamp.active" : "lamp.attention");
        on.DrawCircle(c, r * (lamp == CompileLamp.Clean ? 1 : .6f), lit);
        if (lamp == CompileLamp.Problems) on.DrawArc(c, r * 1.25f, 0, Mathf.Tau, 16, lit, 1, true);
    }
}

public sealed partial class DocumentBody : VBoxContainer
{
    private readonly VBoxContainer details = new();
    private readonly TextEdit text = new() { Editable = false, SizeFlagsVertical = SizeFlags.ExpandFill };

    public DocumentBody()
    {
        Name = "DocumentBody";
        AddChild(details);
        AddChild(text);
    }

    public void Show(Document document, IReadOnlyList<DocumentProblem> problems)
    {
        foreach (Node child in details.GetChildren()) { details.RemoveChild(child); child.QueueFree(); }
        var head = new HBoxContainer(); details.AddChild(head);
        head.AddChild(ChromeWidgets.Text(document.Kind.ToString(), "MicroLabel"));
        if (document.Binding is { } binding)
        {
            var chip = new PanelContainer { ThemeTypeVariation = "Chip" };
            chip.AddChild(ChromeWidgets.Text(binding, "MicroLabel")); head.AddChild(chip);
        }
        if (document.ReadOnly) Line("read-only: " + document.Reason, "DimLabel");
        foreach (DocumentProblem problem in problems.Where(problem => problem.DocumentId == document.Id))
            Line(problem.Text, problem.Dimmed ? "DimLabel" : "");
        if (text.Text != document.Text) text.Text = document.Text;
    }

    private void Line(string words, string variation)
    {
        Label label = ChromeWidgets.Text(words, variation);
        label.AutowrapMode = TextServer.AutowrapMode.WordSmart;
        details.AddChild(label);
    }
}
