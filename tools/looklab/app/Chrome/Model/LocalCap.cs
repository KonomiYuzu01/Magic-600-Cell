namespace Magic600.Chrome;

// Local is never empty (owner, 26 November 2026). While no cap has been named in this session Local draws the
// lowest-numbered cap, neutral by number as every cap list is in cap-id order; once a cap is named Local keeps it until
// another is named, across undo and twists and after an un-naming. The default is drawn, never named: the Named cap
// count, the cap map's line, the threshold panel and the jobs read the named cap only (ADR-D02.12 "Readouts show true
// zeros", .16 "No job starts before a cap is named", .19 "A cap's cell names the cap"). With no state bound Local
// cannot draw, and its empty sentence stays.
public sealed class LocalCap
{
    // The lowest cap id; never chosen from the state.
    public const int Default = 0;
    // The cap named last in this session, or none.
    public int? Last { get; private set; }
    // The named cap at the last Follow, or none.
    public int? Named { get; private set; }
    // The cap Local draws, or none while it cannot draw.
    public int? Shown { get; private set; }

    // The named cap (or none) and whether a state is bound; gives the cap Local draws.
    public int? Follow(int? named, bool bound)
    {
        Named = named;
        if (named != null) Last = named;
        Shown = named ?? (bound ? Last ?? Default : null);
        return Shown;
    }

    // Local's readout while it draws a cap that is not named, with the hint that says how to name one; none while it
    // draws the named cap or draws nothing.
    public string? Words(string hint) => Shown is not int cap || Named != null ? null
        : Last == null ? "Local shows cap " + cap + " by default; no cap is named. " + hint
        : "Local shows cap " + cap + ", the cap named last; no cap is named now. " + hint;
}
