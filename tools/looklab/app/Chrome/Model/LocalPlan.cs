namespace Magic600.Chrome;

// ADR-D01.40: Local is the view the solver reads at all times; it always shows the committed state. ADR-D01.41: it
// re-frames on a selection change by default, and auto-framing can be turned off per view or by script. One rule for
// both puzzles (ADR-D02 build, track C, step C-.40), so Local is not a D02-only view:
// - Jumble on (ADR-D02): Local follows the named cap, else the cap the Jumble host gives it (LocalCap: the cap named
//   last, else the default cap 0 while a state is bound).
// - The base puzzle (600-cell-Full, Jumble off): no cap is ever named there (naming is ADR-D02.12's), and the committed
//   state is always there to draw, so Local draws cell 0's turning region: the lowest cell id, a default by number as
//   LocalCap's is, never chosen from the state. The puzzles share the retained geometry, so cell c's turning region is
//   the region the core draws for cap c.
// Either way a selection newer than the cap frames the cell its sticker occupies while auto-framing is on; clearing the
// selection, or naming another cap, returns Local to the cap. Local is never empty while it has something to show.
public sealed class LocalPlan
{
    // The lowest cell id; never chosen from the state.
    public const int Default = LocalCap.Default;
    public const string Hint = "Select a sticker to frame its cell.";

    public LocalPlan(bool jumble) { Jumble = jumble; }

    // Whether the session runs the jumbled puzzle (Jumble on).
    public bool Jumble { get; }
    // ADR-D01.41: Local re-frames on a selection change while this is on (the default).
    public bool AutoFrame { get; set; } = true;
    // The cap Local follows without a selection, or none (Jumble on with no state bound and no cap named).
    public int? Cap { get; private set; }
    // The cell Local frames: the cap, or the newer selection's cell; none while Local draws nothing.
    public int? Framed { get; private set; }
    // Whether Local frames the selection's cell rather than the cap.
    public bool BySelection { get; private set; }

    private int? named, selected;
    private bool selectionNewer;

    // namedCap: the Jumble naming; hostCap: the Jumble host's LocalCap.Shown; selectedCell: the cell the selected
    // sticker occupies (none with nothing selected or off the lattice). The base puzzle reads neither cap.
    public int? Follow(int? namedCap, int? hostCap, int? selectedCell)
    {
        if (Jumble && namedCap != named)
        {
            named = namedCap;
            selectionNewer = false;
        }
        Cap = Jumble ? namedCap ?? hostCap : Default;
        if (selectedCell != selected)
        {
            selected = selectedCell;
            selectionNewer = selectedCell != null;
        }
        BySelection = AutoFrame && selectionNewer && selected != null;
        Framed = BySelection ? selected : Cap;
        return Framed;
    }

    // Local's readout on the base puzzle while it draws; none with Jumble on, where the Jumble host's LocalCap says it.
    public string? Words => Jumble || Framed is not int cell ? null
        : BySelection ? "Local frames cell " + cell + ", the selected sticker's cell. Clear the selection to return to cell " + Default + "."
        : "Local shows cell " + cell + "'s turning region by default. " + (AutoFrame ? Hint : "Auto-framing is off.");
}
