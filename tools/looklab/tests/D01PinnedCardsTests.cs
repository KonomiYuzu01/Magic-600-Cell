using System;
using System.Collections.Generic;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    private sealed record CardReader(string Name, params CardField[] Fields) : ICardReader
    {
        public IReadOnlyList<CardField> Read(PinnedCard card) => Fields;
    }

    private static void D01PinnedCardsCapAndUnpin()
    {
        var cards = new PinnedCards();
        int changes = 0; cards.Changed += () => changes++;
        Equal(cards.Cap, PinnedCards.DefaultCap);
        Equal(cards.Pin(null), "select a cap or name one first");
        Equal(cards.Track(null), "select a piece first");
        for (int cap = 17; cap < 22; cap++) Equal(cards.Pin(cap), null);
        Equal(cards.Cards[0].Title, "Cap 17");
        Equal(cards.Pin(17), "that subject is already pinned");
        Equal(cards.Track(4021), null);
        PinnedCard tracked = cards.Cards[^1];
        Equal((tracked.Kind, tracked.Subject, tracked.Title), (PinnedCardKind.TrackedPiece, 4021, "Tracked piece 4021"));
        Equal(cards.Track(4021), "that subject is already pinned");
        Equal(cards.Pin(22), "at most 6 pinned cards; unpin one first");
        Equal(cards.Track(4022), "at most 6 pinned cards; unpin one first");
        Equal(cards.Cards.Count, 6);
        Equal(changes, 6);
        Check(cards.Unpin(tracked.Id) && !cards.Unpin(tracked.Id), "unpin must remove only an existing card");
        Equal(cards.Pin(22), null);
        Sequence(cards.Cards.Select(c => c.Subject), new[] { 17, 18, 19, 20, 21, 22 });
        Equal(changes, 8);
    }

    private static void D01PinnedCardsReferenceRows()
    {
        var cards = new PinnedCards();
        cards.Pin(17);
        PinnedCard card = cards.Cards[0];
        var current = new CardReader("Current", new("In", "2"), new("Out", "5"), new("A", "7"), new("floor", "0"));
        Equal(cards.Reference, null);
        Check(cards.Rows(card, current).All(row => row.Reference == null && !row.Differs), "a reference column showed without a saved state");

        cards.SetReference(new CardReader("Start", new("Out", "3"), new("In", "2"), new("floor", "-"), new("u", "0")));
        Equal(cards.Reference, "Start");
        Sequence(cards.Rows(card, current), new CardRow[]
        {
            new("In", "2", "2", false), new("Out", "5", "3", true), new("A", "7", "-", false), new("floor", "0", "-", true),
        });

        cards.SetReference(new CardReader("Next", new CardField("Out", "5")));
        Equal(cards.Reference, "Next");
        Sequence(cards.Rows(card, current), new CardRow[]
        {
            new("In", "2", "-", false), new("Out", "5", "5", false), new("A", "7", "-", false), new("floor", "0", "-", false),
        });
        cards.ClearReference();
        Equal(cards.Reference, null);
        Check(cards.Rows(card, current).All(row => row.Reference == null && !row.Differs), "clearing left a reference value");
    }

    private static void D01PinnedCardsInPlaceControls()
    {
        var cards = new PinnedCards();
        Equal(cards.Enabled.Count, 0);
        foreach (InPlaceControl control in Enum.GetValues<InPlaceControl>())
        {
            foreach (Provenance source in Enum.GetValues<Provenance>().Where(source => source != Provenance.Script))
                Equal(cards.Enable(control, source), "only a script may enable in-place controls");
            Check(!cards.Enabled.Contains(control), "a source other than a script enabled a control");
            Equal(cards.Enable(control, Provenance.Script), null);
            Check(cards.Enabled.Contains(control), "the script's control was not enabled");
            cards.Disable(control);
            Equal(cards.Enabled.Count, 0);
        }
    }

    private static void D01PinnedCardsStoreAndRestore()
    {
        var original = new PinnedCards(3);
        original.Track(17);
        original.Pin(17, "Cap focus | one\nWorld");
        original.Pin(0);
        original.SetReference(new CardReader("Saved"));
        original.Enable(InPlaceControl.FloatingBand, Provenance.Script);
        string text = original.Store();

        var restored = new PinnedCards();
        restored.Restore(text);
        Equal(restored.Cap, 3);
        Sequence(restored.Cards, original.Cards);
        Equal(restored.Store(), text);
        Equal(restored.Reference, null);
        Equal(restored.Enabled.Count, 0);

        restored.Restore("garbage\ncap|0\nCard|17|{");
        Sequence(restored.Cards, original.Cards);
        Equal(restored.Cap, 3);
        restored.Restore("garbage\ncap|2\r\nOther|1|\"bad\"\nCard|-5|\"bad\"\nCard|3|42\n" + string.Join("\n", text.Split('\n').Skip(1)));
        Equal(restored.Cap, 2);
        Sequence(restored.Cards, original.Cards.Take(2));
        restored.Restore(new PinnedCards().Store());
        Equal(restored.Cap, PinnedCards.DefaultCap);
        Equal(restored.Cards.Count, 0);
    }
}
