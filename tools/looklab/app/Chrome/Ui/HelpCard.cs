using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

public static class HelpTopics
{
    private const string Meta = "help_topic";

    public static void Mark(Control control, string topic) => control.SetMeta(Meta, topic);

    public static string? At(Viewport viewport) => HelpCatalog.UnderCursor(Ancestors(viewport.GuiGetHoveredControl()));

    private static IEnumerable<string?> Ancestors(Control? hovered)
    {
        for (Node? node = hovered; node != null; node = node.GetParent())
            if (node is Control control && control.HasMeta(Meta)) yield return control.GetMeta(Meta).AsString();
    }
}

public sealed partial class HelpCard : ChromeOverlay
{
    private readonly VBoxContainer column = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill };
    private readonly Label title = new() { Name = "HelpCardTitle", ThemeTypeVariation = "TitleLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    private readonly Label body = new() { ThemeTypeVariation = "ContentLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    private readonly HelpFigurePlate figure;
    private readonly ScrollContainer scroll = new() { HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };
    private HelpLearnLink? link;
    private Vector2 pointer;
    public event Action<HelpLearnLink>? Learn;

    public HelpCard(Func<TokenSet> tokens) : base(tokens, 0, 0)
    {
        Name = "HelpCard";
        FocusMode = FocusModeEnum.Click;
        Panel.SetAnchorsAndOffsetsPreset(LayoutPreset.TopLeft);
        Panel.AddChild(scroll); scroll.AddChild(column);
        column.AddChild(title); column.AddChild(body);
        figure = new HelpFigurePlate(tokens) { Name = "HelpCardFigure" }; column.AddChild(figure);
        Button learn = ChromeWidgets.Tool(HelpCardModel.LearnLabel, ""); learn.Name = "HelpCardLearn";
        learn.FocusMode = FocusModeEnum.All;
        learn.Pressed += () =>
        {
            if (link is not { } chosen) return;
            Close(); Safe.Raise(Learn, chosen);
        };
        column.AddChild(learn);
        Resized += Place; column.MinimumSizeChanged += Place;
    }

    public void Open(HelpCardModel card, Vector2 at)
    {
        pointer = at; link = card.Learn;
        title.Text = card.Reading.Topic.Title;
        body.Text = card.Reading.Body; body.Visible = body.Text.Length > 0;
        figure.Show(card.Reading.Figure, card.Reading.FigureLine);
        base.Open();
        Place(); scroll.ScrollVertical = 0;
        // Keep Escape on the chrome's transient command even if a text field had focus.
        GrabFocus();
    }

    private void Place()
    {
        if (!IsOpen) return;
        Vector2 area = GetViewportRect().Size;
        float width = Math.Min(area.X, 15 * Tokens().Px("row.height"));
        float height = column.GetCombinedMinimumSize().Y + Panel.GetThemeStylebox("panel").GetMinimumSize().Y;
        Panel.Size = new Vector2(width, Math.Min(area.Y, height));
        Panel.Position = new Vector2(Math.Clamp(pointer.X, 0, Math.Max(0, area.X - Panel.Size.X)),
            Math.Clamp(pointer.Y, 0, Math.Max(0, area.Y - Panel.Size.Y)));
    }
}

internal sealed partial class HelpFigurePlate : PanelContainer
{
    private readonly HelpFigureDrawing drawing;
    private readonly Label caption = new() { ThemeTypeVariation = "DimLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    private readonly Label basis = new() { ThemeTypeVariation = "MicroLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };

    internal HelpFigurePlate(Func<TokenSet> tokens)
    {
        // ADR-D01.77: the figure plate is a surface the art line paints, with a style of its own.
        ThemeTypeVariation = "HelpPlate";
        drawing = new HelpFigureDrawing(tokens);
        var column = new VBoxContainer(); AddChild(column);
        column.AddChild(drawing); column.AddChild(caption); column.AddChild(basis);
    }

    internal void Show(HelpFigure? figure, string line)
    {
        drawing.Visible = basis.Visible = figure != null;
        drawing.Figure = figure; drawing.TooltipText = figure?.Basis ?? "";
        drawing.QueueRedraw(); caption.Text = line; basis.Text = figure?.Basis ?? "";
    }
}

internal sealed partial class HelpFigureDrawing : Control
{
    private readonly Func<TokenSet> tokens;
    internal HelpFigure? Figure { get; set; }

    internal HelpFigureDrawing(Func<TokenSet> tokens)
    {
        this.tokens = tokens; MouseFilter = MouseFilterEnum.Pass;
        Resized += QueueRedraw;
    }

    public override Vector2 _GetMinimumSize() => new(0, 7 * tokens().Px("row.height"));
    public override void _Notification(int what)
    {
        if (what != NotificationThemeChanged) return;
        UpdateMinimumSize(); QueueRedraw();
    }

    public override void _Draw()
    {
        if (Figure is not { } figure) return;
        TokenSet t = tokens();
        float span = Math.Min(Size.X, Size.Y) - 2 * t.Px("space.3");
        if (span <= 0) return;
        Vector2 origin = (Size - new Vector2(span, span)) / 2;
        Vector2 Place(HelpPosition p) => origin + new Vector2((float)p.X, (float)p.Y) * span;
        Color ground = t.C("field.background"), ink = t.C("text.dim", figure.Kind == HelpFigureKind.Polytope ? .45f : .8f);
        Color bold = t.C("accent.strong");
        float line = t.Px("border.width");
        DrawRect(new Rect2(Vector2.Zero, Size), ground);
        foreach (HelpSegment segment in figure.Segments.OrderBy(s => s.Emphasised))
            DrawLine(Place(segment.A), Place(segment.B), segment.Emphasised ? bold : ink, segment.Emphasised ? 2 * line : line, true);
        foreach (HelpPoint point in figure.Points.OrderBy(p => p.Emphasised))
            DrawCircle(Place(point.Position), point.Emphasised ? 2 * line : line, point.Emphasised ? bold : ink);
        if (figure.Kind == HelpFigureKind.Grip)
        {
            Vector2 pole = Place(figure.Points.Single(p => p.Emphasised).Position);
            DrawCircle(pole, 4 * line, bold);
            DrawCircle(pole, 3 * line, ground);
            DrawCircle(pole, line, bold);
        }
    }
}
