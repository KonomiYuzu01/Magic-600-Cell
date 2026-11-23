using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// The solving surface (ADR-D01 revision 4, section 15): the facts of the 35 orbits and a demo state for the
// relationship strip, the orbit rail and the draft line. The orbit facts are real (the seed atlas). Everything a
// SolvingState holds is demo data: a host with an engine fills the same shapes from its reads.

public sealed record OrbitFacts(int Id, int Pieces, string Orientation, int Stage, IReadOnlyList<int> Touches);

public sealed class OrbitAtlas
{
    public static readonly IReadOnlyList<string> Groups = Array.AsReadOnly(new[] { "trivial", "C2", "C5", "D5", "A5" });
    public const int Count = 35;
    // By orbit id.
    public IReadOnlyList<OrbitFacts> Orbits { get; }
    // The seed stage order: one admissible order, never a function of the state.
    public IReadOnlyList<int> StageOrder { get; }

    private OrbitAtlas(OrbitFacts[] orbits)
    {
        Orbits = Array.AsReadOnly(orbits);
        StageOrder = Array.AsReadOnly(orbits.OrderBy(o => o.Stage).Select(o => o.Id).ToArray());
    }

    public static string Name(int orbit) => "O" + orbit.ToString("00", CultureInfo.InvariantCulture);
    public static string Position(int position) => "p" + position.ToString("0000", CultureInfo.InvariantCulture);
    public static string Piece(int piece) => "pc" + piece.ToString("0000", CultureInfo.InvariantCulture);

    // The orbits whose seeds touch this one.
    public IReadOnlyList<int> TouchedBy(int orbit) => Orbits.Where(o => o.Touches.Contains(orbit)).Select(o => o.Id).ToArray();

    // Reads orbit, pieces, orientation, stage and collateral of every entry of assets/seed_atlas.json.
    public static OrbitAtlas Parse(string text)
    {
        using JsonDocument d = JsonDocument.Parse(text);
        var found = new OrbitFacts?[Count];
        if (d.RootElement.ValueKind != JsonValueKind.Array || d.RootElement.GetArrayLength() != Count) throw new InvalidDataException("seed atlas: 35 entries required");
        foreach (JsonElement e in d.RootElement.EnumerateArray())
        {
            int id = e.GetProperty("orbit").GetInt32();
            if (id < 0 || id >= Count || found[id] != null) throw new InvalidDataException("seed atlas: orbit ids 0 to 34, each once");
            string group = e.GetProperty("orientation").GetString() ?? "";
            if (!Groups.Contains(group)) throw new InvalidDataException("seed atlas: unknown orientation group " + group);
            int[] touches = e.GetProperty("collateral").EnumerateArray().Select(x => x.GetInt32()).Where(x => x != id).Distinct().OrderBy(x => x).ToArray();
            if (touches.Any(x => x < 0 || x >= Count)) throw new InvalidDataException("seed atlas: collateral names an unknown orbit");
            found[id] = new OrbitFacts(id, e.GetProperty("pieces").GetInt32(), group, e.GetProperty("stage").GetInt32(), Array.AsReadOnly(touches));
        }
        OrbitFacts[] all = found.Select(o => o!).ToArray();
        if (all.Select(o => o.Stage).Distinct().Count() != Count || all.Any(o => o.Pieces < 2)) throw new InvalidDataException("seed atlas: stages must differ and an orbit has pieces");
        return new OrbitAtlas(all);
    }
}

public enum TargetFrameKind { Home, Captured }
public enum OrbitLock { None, Partial, Whole }

// The exact residual classes of one orbit. Null is "unknown", which is never zero.
public sealed class OrbitResidual
{
    public int? NotHome { get; set; }
    public int? WrongBuffer { get; set; }
    public int? Orientation { get; set; }
    public int? UnknownFrames { get; set; }
    public OrbitLock Lock { get; set; }
    // In the endgame: the last buffer as identity, permitted or impossible. Null outside the endgame.
    public string? LastBuffer { get; set; }
    public bool Known => NotHome != null && WrongBuffer != null && Orientation != null && UnknownFrames != null;
    // Stated only when every class is known and zero.
    public bool ExactlySolved => Known && NotHome == 0 && WrongBuffer == 0 && Orientation == 0 && UnknownFrames == 0;
    public int? Solved(int pieces) => NotHome is { } n ? pieces - n : null;
    // The residual level: a quantity drawn as a length. Null when unknown.
    public double? Level(int pieces) => NotHome is { } n ? Math.Clamp(n / (double)pieces, 0, 1) : null;
    public static string Numeral(int? value) => value is { } v ? v.ToString(CultureInfo.InvariantCulture) : "unknown";
}

public sealed record StripCell(string Slot, string Word, string Value, string Detail, string State);
public sealed record TupleProblem(IReadOnlyList<string> Slots, string Text);
public sealed record OrbitTuple(int? Target, int? BufferA, int? BufferB, TargetFrameKind Frame, int? CapturedWanted, int CapturedRevision);

public sealed class SolvingState
{
    public string Id { get; }
    public string Name { get; }
    public OrbitAtlas Atlas { get; }
    public int Orbit { get; private set; }
    public int? Target { get; set; }
    public int? BufferA { get; set; }
    public int? BufferB { get; set; }
    public TargetFrameKind Frame { get; private set; }
    public int? CapturedWanted { get; private set; }
    public int CapturedRevision { get; private set; }
    public (int Orbit, int Piece)? Next { get; set; }
    public int Revision { get; set; } = 1205;
    public Dictionary<int, OrbitResidual> Residuals { get; } = new();
    private readonly Dictionary<int, Dictionary<int, int>> displaced = new();
    private readonly Dictionary<int, OrbitTuple> kept = new();
    private readonly int seed;

    public SolvingState(string id, string name, OrbitAtlas atlas, int orbit, int seed)
    {
        Id = id; Name = name; Atlas = atlas; Orbit = orbit; this.seed = seed;
        foreach (OrbitFacts o in atlas.Orbits) Residuals[o.Id] = new OrbitResidual();
    }

    public OrbitFacts Facts => Atlas.Orbits[Orbit];
    public OrbitResidual Residual => Residuals[Orbit];

    // The positions of an orbit that hold another piece, with the piece each holds: built once from the orbit's
    // demo count, as cycles of three.
    public IReadOnlyDictionary<int, int> Displaced(int orbit)
    {
        if (displaced.TryGetValue(orbit, out var have)) return have;
        int pieces = Atlas.Orbits[orbit].Pieces, count = Math.Min(Residuals[orbit].NotHome ?? 0, pieces);
        int step = Math.Max(1, (int)(pieces * 0.618)); while (Gcd(step, pieces) != 1) step++;
        int[] at = Enumerable.Range(0, count).Select(i => (int)(((long)seed * 31 + orbit * 17 + (long)i * step) % pieces)).ToArray();
        var map = new Dictionary<int, int>();
        for (int start = 0; start < count; start += 3)
        {
            int length = count - start is 4 or 2 ? count - start : Math.Min(3, count - start);
            for (int i = 0; i < length; i++) map[at[start + i]] = at[start + (i + 1) % length];
            if (length > 3) break;
        }
        displaced[orbit] = map; return map;
    }

    private static int Gcd(int a, int b) { while (b != 0) (a, b) = (b, a % b); return a; }

    public int Occupant(int position) => Displaced(Orbit).TryGetValue(position, out int piece) ? piece : position;
    public int Where(int piece) { foreach (var pair in Displaced(Orbit)) if (pair.Value == piece) return pair.Key; return piece; }
    // The unfinished positions of the working orbit in position-id order: no reasons, no ranking.
    public IReadOnlyList<int> Unfinished() => Displaced(Orbit).Keys.OrderBy(p => p).ToArray();

    // Under Home the Wanted piece follows from the Target; under Captured the capture names it.
    public int? Wanted => Frame == TargetFrameKind.Home ? Target : CapturedWanted;
    public bool WantedDerived => Frame == TargetFrameKind.Home;
    public int? WantedAt => Wanted is { } piece && piece >= 0 && piece < Facts.Pieces ? Where(piece) : null;

    // A fact, never an instruction: where the Wanted piece sits now.
    public string WantedPlace()
    {
        if (WantedAt is not { } at) return "";
        return at == Target ? "in Target" : at == BufferA ? "in A" : at == BufferB ? "in B" : "at " + OrbitAtlas.Position(at);
    }

    public void SetFrame(TargetFrameKind frame)
    {
        if (frame == TargetFrameKind.Captured) { CapturedWanted = Target; CapturedRevision = Revision; }
        Frame = frame;
    }

    // Why the tuple is ill-formed. It is shown as invalid and never corrected.
    public IReadOnlyList<TupleProblem> Problems()
    {
        var found = new List<TupleProblem>();
        (string Slot, string Word, int? At)[] places = { ("target", "Target", Target), ("buffer_a", "Buffer A", BufferA), ("buffer_b", "Buffer B", BufferB) };
        foreach (var place in places)
            if (place.At is { } p && (p < 0 || p >= Facts.Pieces)) found.Add(new TupleProblem(new[] { place.Slot }, $"{place.Word} is not a position of {OrbitAtlas.Name(Orbit)}"));
        for (int i = 0; i < places.Length; i++)
            for (int j = i + 1; j < places.Length; j++)
                if (places[i].At != null && places[i].At == places[j].At) found.Add(new TupleProblem(new[] { places[i].Slot, places[j].Slot }, $"{places[i].Word} and {places[j].Word} are one position"));
        if (Frame == TargetFrameKind.Captured && CapturedWanted == null) found.Add(new TupleProblem(new[] { "target_frame", "wanted" }, "Captured names no capture"));
        return found;
    }

    // The six items of the strip and the Next bookmark, as the strip draws them.
    public IReadOnlyList<StripCell> Cells()
    {
        var problems = Problems(); bool Bad(string slot) => problems.Any(p => p.Slots.Contains(slot));
        bool inside = Target is { } t0 && t0 >= 0 && t0 < Facts.Pieces;
        var cells = new List<StripCell>();
        string Place(int? at) => at is { } p ? OrbitAtlas.Position(p) : "unset";
        string Held(int? at) => at is { } p && p >= 0 && p < Facts.Pieces ? OrbitAtlas.Piece(Occupant(p)) : "";
        cells.Add(new StripCell("target", "Target", Place(Target), "", Bad("target") ? "invalid" : Target == null ? "unset" : "set"));
        int? occupant = inside ? Occupant(Target!.Value) : null;
        cells.Add(new StripCell("occupant", "Occupant", occupant is { } o ? OrbitAtlas.Piece(o) : "unset", "",
            occupant == null ? "unset" : Wanted == null ? "set" : occupant == Wanted ? "match" : "mismatch"));
        cells.Add(new StripCell("wanted", "Wanted", Wanted is { } w ? OrbitAtlas.Piece(w) : "unset", WantedPlace(),
            Bad("wanted") ? "invalid" : Wanted == null ? "unset" : WantedDerived ? "derived" : "set"));
        cells.Add(new StripCell("buffer_a", "Buffer A", Place(BufferA), Held(BufferA), Bad("buffer_a") ? "invalid" : BufferA == null ? "unset" : "set"));
        cells.Add(new StripCell("buffer_b", "Buffer B", Place(BufferB), Held(BufferB), Bad("buffer_b") ? "invalid" : BufferB == null ? "unset" : "set"));
        cells.Add(new StripCell("target_frame", "Target frame", Frame == TargetFrameKind.Home ? "Home" : "Captured", Frame == TargetFrameKind.Captured && CapturedWanted != null ? "r " + CapturedRevision.ToString(CultureInfo.InvariantCulture) : "",
            Bad("target_frame") ? "invalid" : "set"));
        cells.Add(new StripCell("residual", "Residual", OrbitResidual.Numeral(Residual.NotHome), OrbitAtlas.Name(Orbit), Residual.Known ? "set" : "unset"));
        cells.Add(new StripCell("next", "Next", Next is { } n ? OrbitAtlas.Piece(n.Piece) : "unset", Next is { } m && m.Orbit != Orbit ? OrbitAtlas.Name(m.Orbit) : "",
            Next == null ? "unset" : Next.Value.Orbit != Orbit ? "other_orbit" : "set"));
        return cells;
    }

    // The exact residual classes of an orbit, in a fixed order, with exact numerals.
    public IReadOnlyList<(string Class, string Numeral)> Classes(int orbit)
    {
        OrbitResidual r = Residuals[orbit];
        var rows = new List<(string, string)>
        {
            ("Positions not at Home", OrbitResidual.Numeral(r.NotHome)), ("Wrong buffer occupants", OrbitResidual.Numeral(r.WrongBuffer)),
            ("Orientation errors", OrbitResidual.Numeral(r.Orientation)), ("Unknown frames", OrbitResidual.Numeral(r.UnknownFrames)),
            ("Exactly solved", OrbitResidual.Numeral(r.Solved(Atlas.Orbits[orbit].Pieces))),
        };
        if (r.LastBuffer != null) rows.Add(("Last buffer", r.LastBuffer));
        return rows;
    }

    // Switching the working orbit keeps the tuple of the orbit that is left and brings back the other's.
    public bool SwitchOrbit(int orbit)
    {
        if (orbit < 0 || orbit >= OrbitAtlas.Count || orbit == Orbit) return false;
        kept[Orbit] = new OrbitTuple(Target, BufferA, BufferB, Frame, CapturedWanted, CapturedRevision);
        Orbit = orbit;
        OrbitTuple back = kept.TryGetValue(orbit, out OrbitTuple? found) ? found : new OrbitTuple(null, null, null, TargetFrameKind.Home, null, 0);
        (Target, BufferA, BufferB, Frame, CapturedWanted, CapturedRevision) = (back.Target, back.BufferA, back.BufferB, back.Frame, back.CapturedWanted, back.CapturedRevision);
        return true;
    }
}

// Three demo states and the commands of the strip on them. Demo data: never measured, never the engine's.
public sealed class SolvingDemo
{
    public const string Mark = "demo data";
    public OrbitAtlas Atlas { get; }
    public IReadOnlyList<SolvingState> States { get; private set; } = Array.Empty<SolvingState>();
    public int Index { get; private set; }
    public SolvingState State => States[Index];
    // What the last command changed, for the strip's change motion: slot names.
    public IReadOnlyList<string> LastChanged { get; private set; } = Array.Empty<string>();
    public event Action? Changed;
    // The operation draft of the working orbit. Each orbit owns its draft (ADR-D01.63): leaving an orbit, by a
    // switch or by activating Next, saves the draft and its settings, and the orbit entered gets its own back.
    public OperationDraft? Draft { get; set; }
    private readonly Dictionary<(string State, int Orbit), DraftSnapshot> drafts = new();

    public SolvingDemo(OrbitAtlas atlas) { Atlas = atlas; Reset(); }

    private void Carry(int from)
    {
        if (Draft == null || from == State.Orbit) return;
        drafts[(State.Id, from)] = Draft.Save();
        Draft.Load(drafts.TryGetValue((State.Id, State.Orbit), out DraftSnapshot? back) ? back : null);
    }

    public void Reset() { States = new[] { MidSolve(Atlas), Endgame(Atlas), Fresh(Atlas) }; Index = 0; }

    private static SolvingState MidSolve(OrbitAtlas atlas)
    {
        IReadOnlyList<int> order = atlas.StageOrder; const int working = 11;
        var s = new SolvingState("mid", "mid-solve", atlas, order[working], 3);
        for (int i = 0; i < order.Count; i++)
        {
            OrbitFacts o = atlas.Orbits[order[i]]; OrbitResidual r = s.Residuals[o.Id];
            if (i < working) { (r.NotHome, r.WrongBuffer, r.Orientation, r.UnknownFrames) = (0, 0, 0, 0); r.Lock = i == 3 ? OrbitLock.None : i == 4 ? OrbitLock.Partial : OrbitLock.Whole; }
            else if (i == working) (r.NotHome, r.WrongBuffer, r.Orientation, r.UnknownFrames, r.Lock) = (14, 1, o.Orientation == "trivial" ? 0 : 3, 0, OrbitLock.Partial);
            else if (i == 20) (r.NotHome, r.WrongBuffer, r.Orientation, r.UnknownFrames) = (null, null, null, null);
            else (r.NotHome, r.WrongBuffer, r.Orientation, r.UnknownFrames) = (o.Pieces - o.Pieces / (5 + i % 7), 2, o.Orientation == "trivial" ? 0 : o.Pieces / 3, i % 5 == 0 ? 12 : 0);
        }
        IReadOnlyList<int> open = s.Unfinished();
        s.Target = open[0]; s.BufferA = s.Where(open[0]); s.BufferB = open.First(p => p != s.Target && p != s.BufferA);
        s.Next = (order[working + 1], 100);
        return s;
    }

    private static SolvingState Endgame(OrbitAtlas atlas)
    {
        IReadOnlyList<int> order = atlas.StageOrder;
        var s = new SolvingState("endgame", "endgame, last buffer", atlas, order[^1], 5) { Revision = 48211 };
        foreach (int id in order)
        {
            OrbitResidual r = s.Residuals[id]; (r.NotHome, r.WrongBuffer, r.Orientation, r.UnknownFrames, r.Lock) = (0, 0, 0, 0, OrbitLock.Whole);
        }
        OrbitResidual last = s.Residual; (last.NotHome, last.WrongBuffer, last.Lock, last.LastBuffer) = (2, 1, OrbitLock.Partial, "permitted");
        IReadOnlyList<int> open = s.Unfinished();
        s.Target = open[0]; s.BufferA = open[1]; s.BufferB = (open[1] + 1) % s.Facts.Pieces == open[0] ? (open[1] + 2) % s.Facts.Pieces : (open[1] + 1) % s.Facts.Pieces;
        return s;
    }

    private static SolvingState Fresh(OrbitAtlas atlas)
    {
        IReadOnlyList<int> order = atlas.StageOrder;
        var s = new SolvingState("fresh", "fresh scramble", atlas, order[0], 7) { Revision = 40 };
        foreach (OrbitFacts o in atlas.Orbits)
        {
            OrbitResidual r = s.Residuals[o.Id];
            (r.NotHome, r.WrongBuffer, r.Orientation, r.UnknownFrames) = (o.Pieces - 3 - o.Id % 4 * 3, 2, o.Orientation == "trivial" ? 0 : o.Pieces / 2, 0);
        }
        return s;
    }

    public void NextState() { Index = (Index + 1) % States.Count; Tell("target", "occupant", "wanted", "buffer_a", "buffer_b", "target_frame", "residual", "next"); }
    public bool Show(string id) { int at = States.ToList().FindIndex(s => s.Id == id); if (at < 0) return false; Index = at; Tell(); return true; }
    private void Tell(params string[] slots) { LastChanged = slots; Safe.Raise(Changed); }

    private static int? Number(string? text) => text != null && int.TryParse(text.TrimStart('p', 'c', 'O'), NumberStyles.None, CultureInfo.InvariantCulture, out int n) ? n : null;
    // The next unfinished position after a place, in position-id order, round and round.
    private int? Step(int? from)
    {
        IReadOnlyList<int> open = State.Unfinished();
        if (open.Count == 0) return null;
        foreach (int p in open) if (from == null || p > from) return p;
        return open[0];
    }

    // Each returns null when done, or why not. An ill-formed result is kept and shown.
    public string? SetTarget(string? argument)
    {
        int? to = argument == null ? Step(State.Target) : Number(argument);
        if (to == null) return argument == null ? "no unfinished position is listed in this orbit" : "it needs a position such as p0412";
        State.Target = to; Tell("target", "occupant", "wanted"); return null;
    }

    public string? AssignBuffer(string? argument)
    {
        string[] parts = (argument ?? "").Split(':'); string role = parts[0].ToUpperInvariant();
        if (role != "A" && role != "B") return "it needs the role A or B";
        int? now = role == "A" ? State.BufferA : State.BufferB;
        int? to = parts.Length > 1 ? Number(parts[1]) : Step(now);
        if (to == null) return parts.Length > 1 ? "it needs a position such as A:p0412" : "no unfinished position is listed in this orbit";
        if (role == "A") State.BufferA = to; else State.BufferB = to;
        Tell(role == "A" ? "buffer_a" : "buffer_b", "wanted"); return null;
    }

    public string? SetFrame(string? argument)
    {
        TargetFrameKind to = argument?.ToLowerInvariant() switch { "home" => TargetFrameKind.Home, "captured" => TargetFrameKind.Captured, _ => State.Frame == TargetFrameKind.Home ? TargetFrameKind.Captured : TargetFrameKind.Home };
        if (argument != null && argument.ToLowerInvariant() is not ("home" or "captured")) return "it needs Home or Captured";
        State.SetFrame(to); Tell("target_frame", "wanted", "occupant"); return null;
    }

    public string? SwitchOrbit(string? argument)
    {
        IReadOnlyList<int> order = Atlas.StageOrder;
        int? to = argument == null ? order[(order.ToList().IndexOf(State.Orbit) + 1) % order.Count] : Number(argument);
        if (to == null || to < 0 || to >= OrbitAtlas.Count) return "it needs an orbit from O00 to O34";
        int from = State.Orbit;
        if (State.SwitchOrbit(to.Value)) { Carry(from); Tell("target", "occupant", "wanted", "buffer_a", "buffer_b", "target_frame", "residual", "next"); }
        return null;
    }

    // Next bookmarks a piece: the Wanted piece when there is one, or the piece an argument names.
    public string? SetNext(string? argument)
    {
        int? piece = argument == null ? State.Wanted : Number(argument);
        if (piece == null) return argument == null ? "there is no Wanted piece to bookmark" : "it needs a piece such as pc0412";
        State.Next = (State.Orbit, piece.Value); Tell("next"); return null;
    }

    public string? ClearNext() { if (State.Next == null) return "no Next is set"; State.Next = null; Tell("next"); return null; }

    // Activating Next goes to its orbit, which keeps the present orbit's tuple, and names its home as the Target.
    public string? ActivateNext()
    {
        if (State.Next is not { } next) return "no Next is set";
        int from = State.Orbit;
        State.SwitchOrbit(next.Orbit); Carry(from); State.Target = next.Piece; State.Next = null;
        Tell("target", "occupant", "wanted", "buffer_a", "buffer_b", "target_frame", "residual", "next"); return null;
    }
}

// One cell of the orbit rail (ADR-D01.64). Solved, lock and working are three separate marks.
public sealed record RailCell(int Orbit, string Id, string Group, double? Level, bool Solved, OrbitLock Lock, bool Working, bool Next, string Status);

public static class OrbitRailModel
{
    public const string OrderLabel = "one admissible order";

    // The order never depends on the state: the seed stage order, or orbit-id order as a setting.
    public static IReadOnlyList<int> Order(OrbitAtlas atlas, bool byId) => byId ? Enumerable.Range(0, OrbitAtlas.Count).ToArray() : atlas.StageOrder;

    public static IReadOnlyList<RailCell> Cells(SolvingState state, bool byId) => Order(state.Atlas, byId).Select(id =>
    {
        OrbitFacts facts = state.Atlas.Orbits[id]; OrbitResidual r = state.Residuals[id];
        string status = !r.Known || r.UnknownFrames > 0 ? "unknown_frame" : r.ExactlySolved ? "solved" : r.LastBuffer == "permitted" ? "permitted_residual" : "open";
        return new RailCell(id, OrbitAtlas.Name(id), facts.Orientation, r.Level(facts.Pieces), r.ExactlySolved, r.Lock, id == state.Orbit, state.Next is { } n && n.Orbit == id, status);
    }).ToArray();
}

// The typed orbit jump: a held orbit key followed by two digits.
public sealed class OrbitJump
{
    public bool Held { get; private set; }
    public string Typed { get; private set; } = "";
    public void Hold() { Held = true; Typed = ""; }
    public void Release() { Held = false; Typed = ""; }
    // Returns the orbit when two digits name one; a pair that names none is dropped.
    public int? Digit(int digit)
    {
        if (!Held || digit < 0 || digit > 9) return null;
        Typed += (char)('0' + digit);
        if (Typed.Length < 2) return null;
        int orbit = int.Parse(Typed, CultureInfo.InvariantCulture); Typed = "";
        return orbit < OrbitAtlas.Count ? orbit : null;
    }
}

public enum InputRoute { Draft, Direct }
public enum GoalResult { Unset, Met, Unmet, Unknown }
public enum CheckMode { Net, Strict }
public enum CheckResult { Unchecked, Passed, Failed }
public sealed record DraftSnapshot(IReadOnlyList<string>[] Phases, int Active, string? Goal, CheckMode Mode, InputRoute Route);

// The draft line (ADR-D01.66): the operation draft in three phases, the route, the Goal and the check mode.
// The Goal has its own result and never gates a commit: permission is not progress.
public sealed class OperationDraft
{
    public static readonly IReadOnlyList<string> Phases = Array.AsReadOnly(new[] { "Prepare", "Core", "Cleanup" });
    private readonly List<string>[] twists = { new(), new(), new() };
    private readonly HashSet<string> held = new(StringComparer.Ordinal);
    public InputRoute Route { get; private set; } = InputRoute.Draft;
    public int Active { get; private set; } = 1;
    public string? Goal { get; private set; }
    public CheckMode Mode { get; private set; } = CheckMode.Net;
    public IReadOnlyCollection<string> Held => held;
    public event Action? Changed;
    // Raised when the route changes: every held key is let go.
    public event Action? HeldCleared;

    public IReadOnlyList<string> Twists(int phase) => twists[phase];
    public int Count => twists.Sum(t => t.Count);
    public IReadOnlyList<string> Word => twists.SelectMany(t => t).ToArray();
    public void Hold(string key) => held.Add(key);
    public void Release(string key) => held.Remove(key);

    // Demo evaluation, marked as demo wherever it shows: no engine reads the head here.
    public GoalResult GoalResult => Goal == null ? GoalResult.Unset : Count == 0 ? GoalResult.Unknown : twists[1].Count % 2 == 0 ? GoalResult.Met : GoalResult.Unmet;
    // A word can pass Net and fail Strict: here a Prepare without a Cleanup fails Strict.
    public CheckResult Result(CheckMode mode) => Count == 0 ? CheckResult.Unchecked : mode == CheckMode.Net || twists[0].Count == twists[2].Count ? CheckResult.Passed : CheckResult.Failed;
    public CheckResult Checked => Result(Mode);
    // A complete operation in Draft is staged as the preview whatever its check says, so a failure is seen.
    public bool CanStage => Route == InputRoute.Draft && Count > 0;
    // The complete operation is what a commit takes, and only when the protection check in force passes: a failed
    // check stops at the preview (ADR-D01.4 items 1 and 2). Nothing about the Goal is asked here.
    public bool CanCommit => CanStage && Checked == CheckResult.Passed;

    // The check of the staged preview (demo): its effects by the sample rule, its protection by the check in
    // force. A failed check makes the preview impossible, so the commit stops there; the Goal is not asked.
    public PreviewCheck StagedCheck()
    {
        PreviewCheck check = SampleCheck.Check(Word, locked: false);
        if (check.Status != PreviewStatus.Ready) return check;
        return Checked switch
        {
            CheckResult.Passed => check with { Protection = $"{Mode} protection check passed (demo rule)." + SampleCheck.Mark },
            CheckResult.Failed => check with { Status = PreviewStatus.Impossible, Reason = $"the {Mode} protection check failed (demo rule: Prepare and Cleanup differ)",
                Protection = $"{Mode} protection check failed (demo rule): the draft stops at the preview." },
            _ => check with { Status = PreviewStatus.Refused, Reason = "the draft's protection is not checked", Protection = "Nothing was checked." },
        };
    }

    // What an orbit keeps of its draft while the solver works elsewhere (ADR-D01.63).
    public DraftSnapshot Save() => new(twists.Select(t => (IReadOnlyList<string>)t.ToArray()).ToArray(), Active, Goal, Mode, Route);

    // The draft of the orbit entered: its own, or an empty one that keeps the route in force.
    public void Load(DraftSnapshot? from)
    {
        InputRoute route = from?.Route ?? Route;
        for (int i = 0; i < twists.Length; i++) { twists[i].Clear(); if (from != null) twists[i].AddRange(from.Phases[i]); }
        Active = from?.Active ?? 1; Goal = from?.Goal; Mode = from?.Mode ?? CheckMode.Net;
        if (route != Route) { Route = route; held.Clear(); Safe.Raise(HeldCleared); }
        Safe.Raise(Changed);
    }

    // The staged .6cs statement is the draft's projection.
    public string Statement => "preview operation { " + string.Join(" ", Phases.Select((name, i) => $"{name.ToLowerInvariant()} {{ {string.Join("; ", twists[i])} }}")) + " }";

    private static int Phase(string? name) => name == null ? -1 : Phases.ToList().FindIndex(p => string.Equals(p, name.Trim(), StringComparison.OrdinalIgnoreCase));

    public string? SetRoute(string? argument)
    {
        InputRoute to;
        switch (argument?.Trim().ToLowerInvariant())
        {
            case null: to = Route == InputRoute.Draft ? InputRoute.Direct : InputRoute.Draft; break;
            case "draft": to = InputRoute.Draft; break;
            case "direct": case "live": to = InputRoute.Direct; break;
            default: return "it needs Draft or Direct";
        }
        Route = to; held.Clear(); Safe.Raise(HeldCleared); Safe.Raise(Changed); return null;
    }

    public string? SelectPhase(string? argument)
    {
        int to = argument == null ? (Active + 1) % Phases.Count : Phase(argument);
        if (to < 0) return "it needs Prepare, Core or Cleanup";
        Active = to; Safe.Raise(Changed); return null;
    }

    // In Draft a twist joins the active phase. In Direct it is one complete operation: the caller commits it.
    public bool Append(string twist)
    {
        if (Route == InputRoute.Direct || string.IsNullOrWhiteSpace(twist)) return false;
        twists[Active].Add(twist.Trim()); Safe.Raise(Changed); return true;
    }

    // A library macro joins the active phase as one change, so the draft stages once (ADR-D01.69).
    public bool AppendAll(IReadOnlyList<string> word)
    {
        if (Route == InputRoute.Direct || word.Count == 0 || word.Any(string.IsNullOrWhiteSpace)) return false;
        twists[Active].AddRange(word.Select(w => w.Trim())); Safe.Raise(Changed); return true;
    }

    // "phase: twist; twist" replaces a phase; no argument takes the last twist of the active phase away.
    public string? Edit(string? argument)
    {
        if (argument == null)
        {
            if (twists[Active].Count == 0) return $"{Phases[Active]} has no twist";
            twists[Active].RemoveAt(twists[Active].Count - 1); Safe.Raise(Changed); return null;
        }
        int cut = argument.IndexOf(':'); int phase = cut < 0 ? -1 : Phase(argument.Substring(0, cut));
        if (phase < 0) return "it needs a phase and its twists, such as core: twist; twist";
        twists[phase].Clear(); twists[phase].AddRange(argument.Substring(cut + 1).Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries));
        Safe.Raise(Changed); return null;
    }

    public const string SampleGoal = "Wanted in Target";
    // No argument declares the sample Goal, or takes the declared one away.
    public void SetGoal(string? argument) { Goal = argument == null ? (Goal == null ? SampleGoal : null) : string.IsNullOrWhiteSpace(argument) ? null : argument.Trim(); Safe.Raise(Changed); }
    public void SetMode(CheckMode mode) { Mode = mode; Safe.Raise(Changed); }
    public void Clear() { foreach (var t in twists) t.Clear(); Active = 1; Safe.Raise(Changed); }
}

// Where each anchor of the strip sits in a view (ADR-D01.62): the view's own projection of a position of the
// working orbit. Position anchors (Target, A, B) project their positions; piece anchors (Wanted, Next) project the
// position the piece sits on now, so they move when it moves. A point the projection does not place (off the view,
// or no projection at all) gets no anchor: an anchor is never placed from an id.
public static class StripAnchors
{
    public static IReadOnlyDictionary<string, (double X, double Y, bool OnPiece)> Of(SolvingState s, Func<int, int, (double X, double Y)?>? project)
    {
        var found = new Dictionary<string, (double X, double Y, bool OnPiece)>(StringComparer.Ordinal);
        if (project == null) return found;
        void Put(string slot, int? position, bool onPiece)
        {
            if (position is { } p && p >= 0 && p < s.Facts.Pieces && project(s.Orbit, p) is { } at) found[slot] = (at.X, at.Y, onPiece);
        }
        Put("target", s.Target, false); Put("buffer_a", s.BufferA, false); Put("buffer_b", s.BufferB, false);
        Put("wanted", s.WantedAt, true);
        if (s.Next is { } n && n.Orbit == s.Orbit && n.Piece >= 0 && n.Piece < s.Facts.Pieces) Put("next", s.Where(n.Piece), true);
        return found;
    }
}
