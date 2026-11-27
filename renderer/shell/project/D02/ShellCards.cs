using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

internal sealed class ShellCards
{
    private const string Waiting = "waits for its row";
    private readonly ChromeFrame _frame;
    private readonly JumbleHost _host;
    private readonly string _unit;
    private readonly PinnedCards _cards = new();
    private readonly PinnedCardsPanel _panel;
    private readonly Dictionary<int, ShellPick.Picked> _locations = new();
    private ShellPick.Picked? _selection, _hover;
    private Reader _current;

    public ShellCards(ChromeFrame frame, JumbleHost host, string unit)
    {
        _frame = frame; _host = host; _unit = unit;
        _current = ReadState();
        _panel = new PinnedCardsPanel(_cards, () => _current, () => frame.Tokens);
        frame.AddDockPanel(DockSide.Right, PinnedCardsPanel.PanelId, "Cards", "palette.pin", _panel);
        frame.Rebuilt += _panel.Refresh;
        frame.Recoloured += _panel.Refresh;
        ShellPick.Changed += ObservePicks;
        // The host and ShellTint have already subscribed, so the map, style and pose ids are ready here.
        ShellJumble.BindingChanged += Bind;
        frame.TreeExiting += () =>
        {
            ShellPick.Changed -= ObservePicks;
            ShellJumble.BindingChanged -= Bind;
        };
        ObservePicks();
    }

    public static bool Handles(string command) => command is
        "card.pin" or "card.unpin" or "card.track" or "card.reference.set" or "card.reference.clear";

    // A typed number names the subject (a cap 0 to 599, a piece 0 to 177,119); without one the selection names it.
    private int? PickedCap(string? argument) => Typed(argument, CapLayout.Caps) ?? ShellPick.Selection?.Cell ?? _host.Naming.Named;
    private int? PickedPiece(string? argument) => Typed(argument, ShellStyle.PieceCount)
        ?? (ShellPick.Selection is { } pick ? (int)pick.Piece : null);

    private static int? Typed(string? argument, int count) =>
        int.TryParse(argument?.Trim(), NumberStyles.None, CultureInfo.InvariantCulture, out var n) ? (n < count ? n : -1) : null;

    private PinnedCard? ToUnpin(string? argument) => string.IsNullOrWhiteSpace(argument) ? _cards.Cards.LastOrDefault()
        : _cards.Cards.FirstOrDefault(c => c.Id == argument.Trim());

    public string? Unavailable(string command, string? argument) => command switch
    {
        "card.pin" => _cards.PinReason(PinnedCardKind.Card, PickedCap(argument)),
        "card.track" => _cards.PinReason(PinnedCardKind.TrackedPiece, PickedPiece(argument)),
        "card.unpin" when ToUnpin(argument) == null => string.IsNullOrWhiteSpace(argument) ? "no pinned cards" : "no pinned card with that id",
        "card.reference.set" when ShellJumble.Bound == null => "no state is bound",
        "card.reference.clear" when _cards.Reference == null => "no reference state is set",
        _ => null,
    };

    public void Execute(string command, string? argument)
    {
        switch (command)
        {
            case "card.pin": _cards.Pin(PickedCap(argument)); break;
            case "card.track": _cards.Track(PickedPiece(argument)); break;
            case "card.unpin": _cards.Unpin(ToUnpin(argument)!.Id); break;
            case "card.reference.set":
                ObservePicks();
                _cards.SetReference(_current.Freeze(string.IsNullOrWhiteSpace(argument) ? _current.Name : argument.Trim()));
                break;
            case "card.reference.clear": _cards.ClearReference(); break;
        }
        _frame.Model.RevealDock(DockSide.Right, PinnedCardsPanel.PanelId, _frame.Folds(DockSide.Right));
    }

    // A pick stays with its piece when the pointer leaves. The driver also checks the selection each frame,
    // since ShellPick changes its record on another click on the same piece without raising Changed.
    public void ObservePicks()
    {
        bool refresh = Remember(ShellPick.Hover, ref _hover);
        refresh |= Remember(ShellPick.Selection, ref _selection);
        if (refresh) _panel.Refresh();
    }

    private bool Remember(ShellPick.Picked? pick, ref ShellPick.Picked? before)
    {
        if (ReferenceEquals(pick, before)) return false;
        before = pick;
        if (pick == null) return false;
        int piece = (int)pick.Piece;
        if (_locations.TryGetValue(piece, out var latest) && (latest.Revision > pick.Revision || latest == pick)) return false;
        _locations[piece] = pick;
        return _cards.Cards.Any(c => c.Kind == PinnedCardKind.TrackedPiece && c.Subject == piece);
    }

    private void Bind() => ShellJumble.Timed("cards", () =>
    {
        _current = ReadState();
        _panel.Refresh();
    });

    private Reader ReadState()
    {
        CapMapModel map = _host.CapMap.Map.Map;
        var caps = new Dictionary<int, IReadOnlyList<CardField>>();
        IReadOnlyList<CardField> Cap(int key) => caps.TryGetValue(key, out var fields) ? fields : caps[key] = ReadCap(map, key);
        return new Reader(ShellJumble.Bound ?? "Current", Cap, _host.Style, _host.PoseOf, _locations);
    }

    private IReadOnlyList<CardField> ReadCap(CapMapModel map, int cap)
    {
        string? record = ShellJumble.Record(cap);
        CapReadout readout = record == null ? map.Readout(cap) : CapReadout.Read(record);
        var fields = readout.Slots(_unit).Select(slot => new CardField(slot.Label, slot.Value.Kind switch
        {
            NumeralKind.Pending => Waiting,
            NumeralKind.Value when slot.Unit is { } unit => slot.Value.Word + " " + unit,
            _ => slot.Value.Word,
        })).ToList();
        CapCovers? covers = record == null ? null : CapCovers.FromRecord(record);
        fields.Add(new CardField("covered by", CapIds(covers?.CoveredBy)));
        fields.Add(new CardField("covers", CapIds(covers?.CoversCaps)));
        fields.Add(new CardField("undefined covers", CapIds(covers?.Undefined)));
        return fields;
    }

    private static string CapIds(IReadOnlyList<int>? caps) => caps == null ? Waiting : caps.Count == 0 ? "none"
        : string.Join(" ", caps.Select(c => c.ToString(CultureInfo.InvariantCulture)));

    private sealed class Reader(string name, Func<int, IReadOnlyList<CardField>> cap, PoseStyle? style,
        IReadOnlyList<int>? poses, IReadOnlyDictionary<int, ShellPick.Picked> locations) : ICardReader
    {
        public string Name => name;

        // Read every cap into the saved state, including caps the solver may pin later. The pose style's rows and
        // pick records are immutable, and the pose ids and locations get their own copies.
        public Reader Freeze(string savedName)
        {
            var savedCaps = Enumerable.Range(0, CapLayout.Caps).Select(cap).ToArray();
            return new Reader(savedName, key => savedCaps[key], style, poses?.ToArray(), new Dictionary<int, ShellPick.Picked>(locations));
        }

        public IReadOnlyList<CardField> Read(PinnedCard card)
        {
            int piece = card.Subject;
            if (card.Kind == PinnedCardKind.Card) return cap(piece);
            locations.TryGetValue(piece, out var pick);
            int? row = style != null && poses != null && piece < poses.Count ? poses[piece] : null;
            return new CardField[]
            {
                new("Piece", Number(piece)),
                new("Cell", pick?.Cell is int cell ? Number(cell) : Waiting),
                new("Slot", pick?.Slot is uint slot ? Number(slot) : Waiting),
                new("Pick revision", pick?.Revision.ToString(CultureInfo.InvariantCulture) ?? Waiting),
                new("Domain", row is int domain ? Number(style!.RowDomain(domain)) : Waiting),
                new("Height", row is int height ? Number(style!.RowHeight(height)) : Waiting),
                new("On lattice", row is int lattice ? (style!.RowOnLattice(lattice) ? "yes" : "no") : Waiting),
            };
        }

        private static string Number(long value) => value.ToString(CultureInfo.InvariantCulture);
    }
}
