using System;
using Godot;

namespace Magic600.Chrome;

public sealed partial class PinnedCardsPanel : VBoxContainer
{
    public const string PanelId = "cards";
    private const string Empty = "No pinned cards. Pin card (palette) pins the selection's cap; Follow piece pins the Tracked piece.";
    private readonly PinnedCards model;
    private readonly Func<ICardReader> current;
    private readonly Func<TokenSet> tokens;

    public PinnedCardsPanel(PinnedCards model, Func<ICardReader> current, Func<TokenSet> tokens)
    {
        this.model = model; this.current = current; this.tokens = tokens; Name = "PinnedCards";
        model.Changed += Refresh;
        Refresh();
    }

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        SessionWidgets.Clear(this);
        if (model.Reference is { } name) AddChild(Text("Reference: " + name, "HeadingLabel"));
        if (model.Cards.Count == 0)
        {
            Control empty = ChromeBody.PaneState("empty", Empty);
            Label words = empty.GetChild<VBoxContainer>(0).GetChild<HBoxContainer>(1).GetChild<Label>(0);
            words.AutowrapMode = TextServer.AutowrapMode.WordSmart;
            words.CustomMinimumSize = new Vector2(tokens().Px("row.height") * 5, 0);
            AddChild(empty);
            return;
        }

        ICardReader reader = current();
        for (int i = 0; i < model.Cards.Count; i++)
        {
            PinnedCard card = model.Cards[i];
            var body = new VBoxContainer { Name = "PinnedCard" + i };
            AddChild(body);
            var head = new HBoxContainer(); body.AddChild(head);
            head.AddChild(Text(card.Title, "HeadingLabel"));
            Button unpin = ChromeWidgets.Tool("Unpin", "Unpin " + card.Title + " (" + card.Id + ")");
            unpin.Name = "Unpin";
            unpin.Pressed += () => model.Unpin(card.Id);
            head.AddChild(unpin);

            var grid = new GridContainer { Columns = model.Reference == null ? 2 : 3 };
            body.AddChild(grid);
            foreach (CardRow row in model.Rows(card, reader))
            {
                grid.AddChild(Text(row.Label, "DimLabel"));
                grid.AddChild(Text(row.Current));
                if (row.Reference is not { } value) continue;
                Label reference = Text(value);
                if (row.Differs) reference.AddThemeColorOverride("font_color", tokens().C("accent.strong"));
                grid.AddChild(reference);
            }
            body.AddChild(new HSeparator());
        }
    }

    private static Label Text(string text, string variation = "MicroLabel")
    {
        Label label = ChromeWidgets.Text(text, variation);
        label.AutowrapMode = TextServer.AutowrapMode.WordSmart;
        label.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        return label;
    }
}
