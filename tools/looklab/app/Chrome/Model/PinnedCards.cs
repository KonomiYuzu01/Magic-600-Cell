using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

public enum PinnedCardKind { Card, TrackedPiece }
public enum InPlaceControl { FloatingBand, HoverReadouts, SwatchPopovers }

public sealed record PinnedCard(string Id, PinnedCardKind Kind, int Subject, string Title);
public sealed record CardField(string Label, string Value);
public sealed record CardRow(string Label, string Current, string? Reference, bool Differs);

public interface ICardReader
{
    string Name { get; }
    IReadOnlyList<CardField> Read(PinnedCard card);
}

// ADR-D01.24: pinned subjects follow the current reader. The reference reader belongs to one saved state.
public sealed class PinnedCards
{
    public const int DefaultCap = 6;
    public int Cap { get; private set; }
    public IReadOnlyList<PinnedCard> Cards { get; }
    public string? Reference => reference?.Name;
    public IReadOnlyCollection<InPlaceControl> Enabled => enabled.ToArray();
    public event Action? Changed;
    private readonly List<PinnedCard> cards = new();
    private readonly HashSet<InPlaceControl> enabled = new();
    private ICardReader? reference;

    public PinnedCards(int cap = DefaultCap)
    {
        if (cap < 1) throw new ArgumentOutOfRangeException(nameof(cap));
        Cap = cap; Cards = cards.AsReadOnly();
    }

    public string? PinReason(PinnedCardKind kind, int? subject)
    {
        if (subject is not int key || key < 0)
            return kind == PinnedCardKind.TrackedPiece ? "select a piece first" : "select a cap or name one first";
        if (cards.Any(c => c.Kind == kind && c.Subject == key)) return "that subject is already pinned";
        return cards.Count >= Cap ? FormattableString.Invariant($"at most {Cap} pinned cards; unpin one first") : null;
    }

    public string? Pin(int? subject, string? title = null) => Add(PinnedCardKind.Card, subject, title);
    public string? Track(int? piece) => Add(PinnedCardKind.TrackedPiece, piece, null);

    private string? Add(PinnedCardKind kind, int? subject, string? title)
    {
        if (PinReason(kind, subject) is { } reason) return reason;
        cards.Add(Card(kind, subject!.Value, title)); Safe.Raise(Changed);
        return null;
    }

    private static PinnedCard Card(PinnedCardKind kind, int subject, string? title)
    {
        string key = subject.ToString(CultureInfo.InvariantCulture);
        bool tracked = kind == PinnedCardKind.TrackedPiece;
        return new PinnedCard((tracked ? "piece-" : "card-") + key, kind, subject,
            string.IsNullOrWhiteSpace(title) ? (tracked ? "Tracked piece " : "Cap ") + key : title);
    }

    public bool Unpin(string id)
    {
        if (cards.RemoveAll(c => c.Id == id) == 0) return false;
        Safe.Raise(Changed);
        return true;
    }

    public IReadOnlyList<CardRow> Rows(PinnedCard card, ICardReader current)
    {
        IReadOnlyList<CardField>? saved = reference?.Read(card);
        return current.Read(card).Select(field =>
        {
            string? value = saved?.FirstOrDefault(f => f.Label == field.Label)?.Value;
            return new CardRow(field.Label, field.Value, saved == null ? null : value ?? "-", value != null && value != field.Value);
        }).ToArray();
    }

    public void SetReference(ICardReader saved) { reference = saved; Safe.Raise(Changed); }

    public void ClearReference()
    {
        if (reference == null) return;
        reference = null; Safe.Raise(Changed);
    }

    public string? Enable(InPlaceControl control, Provenance source)
    {
        if (source != Provenance.Script) return "only a script may enable in-place controls";
        if (enabled.Add(control)) Safe.Raise(Changed);
        return null;
    }

    public void Disable(InPlaceControl control)
    {
        if (enabled.Remove(control)) Safe.Raise(Changed);
    }

    public string Store() => string.Join("\n", cards.Select(c =>
        FormattableString.Invariant($"{c.Kind}|{c.Subject}|{JsonSerializer.Serialize(c.Title)}"))
        .Prepend(FormattableString.Invariant($"cap|{Cap}")));

    // Each line stands on its own. The order of the readable subjects is kept for unpinning the newest.
    public void Restore(string text)
    {
        int? cap = null;
        var restored = new List<PinnedCard>();
        foreach (string line in text.Split('\n', StringSplitOptions.RemoveEmptyEntries))
        {
            string[] parts = line.TrimEnd('\r').Split('|', 3);
            if (parts.Length < 2 || !int.TryParse(parts[1], NumberStyles.None, CultureInfo.InvariantCulture, out int number)) continue;
            if (parts[0] == "cap" && parts.Length == 2 && number > 0) cap = number;
            if (parts.Length != 3 || parts[0] is not (nameof(PinnedCardKind.Card) or nameof(PinnedCardKind.TrackedPiece))) continue;
            try
            {
                if (JsonSerializer.Deserialize<string>(parts[2]) is { } title)
                    restored.Add(Card(parts[0] == nameof(PinnedCardKind.Card) ? PinnedCardKind.Card : PinnedCardKind.TrackedPiece, number, title));
            }
            catch (JsonException) { }
        }
        if (cap == null && restored.Count == 0) return;
        Cap = cap ?? Cap;
        cards.Clear(); cards.AddRange(restored.DistinctBy(c => c.Id).Take(Cap));
        Safe.Raise(Changed);
    }
}
