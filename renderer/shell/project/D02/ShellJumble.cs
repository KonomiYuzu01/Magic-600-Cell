using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using Godot;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

/// <summary>
/// ADR-D02's surfaces in the product shell (build step B4), all behind M600_D02=1 so that the measured C9 runs are
/// unchanged. It registers the D02 draft rows after the command table (tools/looklab/data/design/command-rows-d02.json,
/// staged as design/), shows the table's Jumble rows by their display names, marks the draft rows in the palette, and
/// adds a driver node that keeps the owner's window rule, answers the watch command and plays a D02 script. B5 binds
/// a sample state to the session by the head's two pose-array digests (<see cref="Bind"/>), never by its name.
/// </summary>
public static class ShellJumble
{
    public const string DraftReason = "a draft row: this build does not carry it out yet";
    /// <summary>The binding chip's words with no sample bound: every D02 slot is stale and its readouts pending.</summary>
    public const string Unbound = "unbound · stale";
    private static readonly HashSet<string> DraftIds = new(StringComparer.Ordinal);
    // The sample states by their pose-array digests ("<pose ids>/<lattice>"), in the index's order.
    private static readonly Dictionary<string, List<string>> Samples = new(StringComparer.Ordinal);

    public static bool On => System.Environment.GetEnvironmentVariable("M600_D02") == "1";
    public static IReadOnlyCollection<string> Drafts => DraftIds;
    /// <summary>The sample state the launch asked for (M600_D02_STATE), or null.</summary>
    public static string? Asked => System.Environment.GetEnvironmentVariable("M600_D02_STATE") is { Length: > 0 } named ? named : null;
    /// <summary>The sample state bound to the session, or null (B5).</summary>
    public static string? Bound { get; private set; }

    /// <summary>B30: the tuning panel, for the dump; set by the shell's chrome with D02 on.</summary>
    internal static TuningPanel? TuningHost { get; set; }

    /// <summary>B30: the head's state digest and revision as m6h_state_digest last gave them (the tuning run's check).</summary>
    public static string StateDigest { get; private set; } = "";
    public static long StateRevision { get; private set; } = -1;
    /// <summary>The binding changed.</summary>
    public static event Action? BindingChanged;

    /// <summary>
    /// SF3 (27 Nov 2026): the bind's parts at a newly adopted revision, each a name and its two QPC readings; the shell
    /// sets a list while the event log is on and writes the parts in its "steps" row.
    /// </summary>
    internal static List<(string Name, long From, long To)>? BindParts { get; set; }

    /// <summary>
    /// SF6 (27 Nov 2026): called after each part of the bind while the shell collects them; the shell queues a deferred
    /// mark there, so the deferred calls (layout, redraws) each part queued are timed apart in its "hole" row.
    /// </summary>
    internal static Action<string>? PartEnded { get; set; }

    /// <summary>Ends one part of the bind begun at <paramref name="from"/>, while someone collects them; answers now.</summary>
    internal static long Part(string name, long from)
    {
        var now = EventLog.Now();
        BindParts?.Add((name, from, now));
        PartEnded?.Invoke(name);
        return now;
    }

    /// <summary>Runs one part of the bind, timed into <see cref="BindParts"/> while someone collects them.</summary>
    internal static void Timed(string name, Action part)
    {
        if (BindParts is not { } parts)
        {
            part();
            return;
        }
        var from = EventLog.Now();
        part();
        parts.Add((name, from, EventLog.Now()));
        PartEnded?.Invoke(name);
    }
    /// <summary>The view rotation a drag in the Global opening turns (owner, 26 November 2026).</summary>
    public static ShellCamera Camera { get; } = new();
    /// <summary>The wheel zoom of the view under the pointer (owner, 26 November 2026).</summary>
    public static ShellZoom Zoom { get; } = new();
    private static JumbleHost? _host;
    private static InspectorModel? _pickInspector;
    internal static string PickInspectLine => _pickInspector?.Piece?.Line ?? "";

    /// <summary>The Jumble facts of each newly shown revision (B6), from the host's state digests.</summary>
    public static JumbleFeed Feed { get; } = new();

    /// <summary>
    /// The words the Jumble host shows, from the design folder's jumble-words.json: the record's display names by id,
    /// Local's empty sentence and the words of a section with nothing in it.
    /// </summary>
    public static (Dictionary<string, string> Names, string Empty, string Nothing) Words(string folder)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(Path.Combine(folder, "jumble-words.json")));
        var root = document.RootElement;
        var names = root.GetProperty("names").EnumerateArray().ToDictionary(n => n.GetProperty("id").GetString()!, n => n.GetProperty("text").GetString()!, StringComparer.Ordinal);
        string Sentence(string id) => root.GetProperty("sentences").EnumerateArray().First(s => s.GetProperty("id").GetString() == id).GetProperty("text").GetString()!;
        return (names, Sentence("local.empty"), Sentence("result.not_run"));
    }

    /// <summary>
    /// Words that send the reader to the palette by <paramref name="taken"/> (a key another surface holds) name the
    /// palette's own key instead (B32 findings: the sample's key map holds F1).
    /// </summary>
    public static string PaletteRoute(string words, string? taken, string paletteKeys) =>
        taken is { Length: > 0 } && paletteKeys.Split(", ")[0] is { Length: > 0 } key ? words.Replace(taken + " ›", key + " ›") : words;

    /// <summary>The binding chip's words for a bound sample state.</summary>
    public static string BoundWords(string state) => "bound · " + state;

    /// <summary>The sentences of the design folder's jumble-words.json, by id.</summary>
    public static Dictionary<string, string> Sentences(string folder)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(Path.Combine(folder, "jumble-words.json")));
        return document.RootElement.GetProperty("sentences").EnumerateArray()
            .ToDictionary(s => s.GetProperty("id").GetString()!, s => s.GetProperty("text").GetString()!, StringComparer.Ordinal);
    }

    /// <summary>
    /// Global's totals (B7): the bound sample state's totals, whose seam facts cover the whole state (the sample
    /// builder refuses a state whose seams are incomplete, cut off or cancelled); pending while no state is bound.
    /// </summary>
    public static GlobalTotals Totals()
    {
        if (Bound is not { } state)
        {
            return GlobalTotals.Pending;
        }
        var path = Path.Combine(JumbleFolder(), state, "totals.json");
        return Kept(path, text => GlobalTotals.Read(text, SeamCoverage.Complete)) ?? GlobalTotals.Pending;
    }

    // SF (27 Nov 2026): a sample state's files, parsed once per path while the samples stay loaded (each model read
    // here is immutable); LoadSamples clears them. A binding that changes at a twist re-read them on the frame thread.
    private static readonly Dictionary<string, object?> Reads = new();

    private static T? Kept<T>(string path, Func<string, T> read, string tag = "") where T : class
    {
        var key = tag + "|" + path;
        if (Reads.TryGetValue(key, out var kept))
        {
            return (T?)kept;
        }
        var value = File.Exists(path) ? read(File.ReadAllText(path)) : null;
        Reads[key] = value;
        return value;
    }

    /// <summary>
    /// The cap map's ring sheet (B8): the samples' cap-layout.json, which maps each cap to its Hopf ring and its place
    /// along it; none when the build has no layout.
    /// </summary>
    public static CapLayout? Layout()
    {
        var path = Path.Combine(JumbleFolder(), "cap-layout.json");
        return File.Exists(path) ? CapLayout.Read(File.ReadAllText(path)) : null;
    }

    /// <summary>
    /// The cap map's classes (B8): from the bound sample state's rim record, certified and completed rows only; every
    /// cap pending while no state is bound or the state has no rim record.
    /// </summary>
    public static CapMapModel CapMap()
    {
        if (Bound is not { } state)
        {
            return CapMapModel.Pending();
        }
        var path = Path.Combine(JumbleFolder(), state, "rim.json");
        CapReadout? RecordAt(int cap) => CapRecord(state, cap);
        return Kept(path, text => CapMapModel.Read(text, RecordAt)) ?? CapMapModel.Pending(RecordAt);
    }

    /// <summary>The Jumble host; set by the chrome. B25: setting it binds the seed 7001 restoration to its Word panel.</summary>
    public static JumbleHost? Host
    {
        get => _host;
        set
        {
            _host = value;
            // Local is never empty: the host draws the default cap while a state is bound (owner, 26 November 2026).
            value?.SetStateBound(() => Bound != null);
            if (value != null && SeedWord() is { } word)
            {
                value.Words.Bind(word, done => StateAfter("s7001-" + done.ToString(CultureInfo.InvariantCulture)));
            }
        }
    }

    /// <summary>B26: the ν-correction catalogue (corrections.json) with the ν constants, or null where the samples lack it.</summary>
    public static NuCorrections? Corrections()
    {
        var path = Path.Combine(JumbleFolder(), "corrections.json");
        return Kept(path, NuCorrections.Read);
    }

    /// <summary>B25: the seed 7001 restoration's nine words (seed7001-words.json), or null where the samples lack it.</summary>
    public static WordTree? SeedWord()
    {
        var path = Path.Combine(JumbleFolder(), "seed7001-words.json");
        return File.Exists(path) ? WordTree.Read(File.ReadAllText(path)) : null;
    }

    /// <summary>B25: a sample state's N, lattice and ν row from its totals, or null without them.</summary>
    public static WordState? StateAfter(string state) =>
        Kept(Path.Combine(JumbleFolder(), state, "totals.json"), text => new AfterRead(ReadStateAfter(text)), "after")?.State;

    private sealed record AfterRead(WordState State);

    private static WordState ReadStateAfter(string text)
    {
        using var document = JsonDocument.Parse(text);
        var nu = document.RootElement.GetProperty("nu");
        var on = nu.GetProperty("result").GetString() != "undefined";
        return new WordState(document.RootElement.GetProperty("N").GetInt64(), on,
            on ? nu.GetProperty("nu").EnumerateArray().Select(v => v.GetInt32()).ToArray() : null);
    }

    /// <summary>A named cap's twelve release checks (B10) from the bound sample's record of the cap; none without one.</summary>
    public static IReadOnlyDictionary<int, TwistCheck> TwistChecks(int cap)
    {
        if (Bound is not { } state)
        {
            return new Dictionary<int, TwistCheck>();
        }
        var path = Path.Combine(JumbleFolder(), state, "cap-" + cap + ".json");
        return File.Exists(path) ? TwistCheck.FromRecord(File.ReadAllText(path)) : new Dictionary<int, TwistCheck>();
    }

    /// <summary>A named cap's raw record in the bound sample (cap-&lt;id&gt;.json), or null without one.</summary>
    public static string? Record(int cap)
    {
        if (Bound is not { } state)
        {
            return null;
        }
        var path = Path.Combine(JumbleFolder(), state, "cap-" + cap + ".json");
        // SF3 (27 Nov 2026): read once per state and cap; the bind's cap covers and repair list read it again.
        return Kept(path, text => text, "record");
    }

    /// <summary>A named cap's completed covers answer (B11), from its record; none at a blocked cap or without one.</summary>
    public static CapCovers? Covers(int cap) => Record(cap) is { } record ? CapCovers.FromRecord(record) : null;

    /// <summary>The bound state's pose rows (style.json), which count the domain and height focus layers (B11).</summary>
    public static PoseStyle? Style()
    {
        if (Bound is not { } state)
        {
            return null;
        }
        var path = Path.Combine(JumbleFolder(), state, "style.json");
        return Kept(path, PoseStyle.Read);
    }

    /// <summary>B15: the bound state's domain contact graph (contact.json), or null.</summary>
    public static ContactGraph? Contact()
    {
        if (Bound is not { } state)
        {
            return null;
        }
        var path = Path.Combine(JumbleFolder(), state, "contact.json");
        return Kept(path, ContactGraph.Read);
    }

    /// <summary>
    /// B15: the bound state's pose ids (poses.json), each piece's pose row checked against the digest and the style's
    /// row count, with the state digest they belong to; null without a bound state or the file.
    /// </summary>
    public static (int[] Poses, string Digest)? Poses(int rows)
    {
        if (Bound is not { } state)
        {
            return null;
        }
        var path = Path.Combine(JumbleFolder(), state, "poses.json");
        return Kept(path, text => ReadPoses(text, rows), rows.ToString(CultureInfo.InvariantCulture)) is { } read
            ? (read.Poses, read.Digest)
            : null;
    }

    private sealed record PoseRead(int[] Poses, string Digest);

    private static PoseRead ReadPoses(string text, int rows)
    {
        using var document = JsonDocument.Parse(text);
        return new PoseRead(JumbleTint.Poses(text, rows), document.RootElement.GetProperty("digest").GetString() ?? "");
    }

    /// <summary>The 80 caps meeting each cap (B9), from the samples' meeting-caps.json; none when the build has none.</summary>
    public static MeetingCaps? Meeting()
    {
        var path = Path.Combine(JumbleFolder(), "meeting-caps.json");
        return File.Exists(path) ? MeetingCaps.Read(File.ReadAllText(path)) : null;
    }

    /// <summary>
    /// A named cap's readout from the bound sample's own record of it (cap-&lt;id&gt;.json), which carries a blocked
    /// cap's straddler count; null when the sample has no record of that cap, and the rim row stands in.
    /// </summary>
    private static CapReadout? CapRecord(string state, int cap)
    {
        var path = Path.Combine(JumbleFolder(), state, "cap-" + cap + ".json");
        return Kept(path, CapReadout.Read, "readout");
    }

    /// <summary>The folder of the D02 jumble samples: under M600_D02_DATA when set, else the build's staged copy.</summary>
    public static string JumbleFolder()
    {
        var data = System.Environment.GetEnvironmentVariable("M600_D02_DATA") is { Length: > 0 } source
            ? source : ProjectSettings.GlobalizePath("res://samples");
        return Path.Combine(data, data.EndsWith("samples", StringComparison.Ordinal) ? "" : "samples", "jumble");
    }

    /// <summary>Reads every sample state's three digests from its state.json header (B1); answers how many.</summary>
    public static int LoadSamples()
    {
        Samples.Clear();
        Reads.Clear();
        var folder = JumbleFolder();
        using var index = JsonDocument.Parse(File.ReadAllText(Path.Combine(folder, "states.json")));
        var count = 0;
        foreach (var entry in index.RootElement.GetProperty("states").EnumerateArray())
        {
            var id = entry.GetProperty("state").GetString() ?? "";
            using var state = JsonDocument.Parse(File.ReadAllText(Path.Combine(folder, id, "state.json")));
            var digests = state.RootElement.GetProperty("digests");
            var key = Key(digests.GetProperty("adapter").GetString(), digests.GetProperty("pose_ids_sha256").GetString(),
                digests.GetProperty("pose_lattice_sha256").GetString());
            if (!Samples.TryGetValue(key, out var ids))
            {
                Samples[key] = ids = new List<string>();
            }
            ids.Add(id);
            count++;
        }
        return count;
    }

    // A pose id is a rank in a table compacted per state, so two states can share both pose-array digests; the state
    // digest tells them apart (checkpoint C1). A fixture's three digests equal its sample's (build_d02_fixtures.py).
    private static string Key(string? state, string? ids, string? lattice) => state + "/" + ids + "/" + lattice;

    /// <summary>
    /// Binds the sample state whose state digest and two pose-array digests equal the session head's (B5).
    /// <paramref name="digests"/> is m6h_state_digest's answer, read at each newly shown revision, which also feeds the
    /// Jumble facts (B6). Several states with the same digests bind the asked one when it is among them, else the first
    /// in the index's order. With none, or with Jumble off, nothing is bound. An answer that does not read unbinds and
    /// keeps the numerals undefined (<see cref="Unknown"/>).
    /// </summary>
    public static void Bind(string digests, long revision)
    {
        string? bound = null;
        try
        {
            using var document = JsonDocument.Parse(digests);
            var root = document.RootElement;
            StateDigest = root.TryGetProperty("digest", out var stateDigest) ? stateDigest.GetString() ?? "" : "";
            StateRevision = revision;
            if (root.GetProperty("pose_ids_sha256").ValueKind == JsonValueKind.String
                && Samples.TryGetValue(Key(root.GetProperty("digest").GetString(), root.GetProperty("pose_ids_sha256").GetString(),
                    root.GetProperty("pose_lattice_sha256").GetString()), out var ids))
            {
                bound = Asked is { } asked && ids.Contains(asked) ? asked : ids[0];
            }
        }
        catch (Exception error) when (error is JsonException or KeyNotFoundException or InvalidOperationException)
        {
            Unknown(revision);
            return;
        }
        // SF3 (27 Nov 2026): the host places its border rows and refreshes Local's sections once, at the bind's end.
        _host?.Hold();
        try
        {
            var fed = EventLog.Now();
            Feed.Adopt(JumbleFeed.Read(digests, revision));
            Part("feed", fed);
            Rebind(bound);
        }
        finally
        {
            Timed("flush", () => _host?.Release());
        }
        Timed("inspect", InspectPicked);
    }

    /// <summary>
    /// The head's facts could not be read at this revision (checkpoint C1): nothing is bound, and with Jumble on the
    /// numerals are undefined rather than the last revision's.
    /// </summary>
    public static void Unknown(long revision)
    {
        _host?.Hold();
        try
        {
            Feed.Adopt(Feed.State with { LatticeFlag = false, Revision = revision });
            Rebind(null);
        }
        finally
        {
            _host?.Release();
        }
        InspectPicked();
    }

    private static void Rebind(string? bound)
    {
        if (bound != Bound)
        {
            Bound = bound;
            BindingChanged?.Invoke();
        }
    }

    /// <summary>ADR-D02 B17: the selected sticker's session facts and any matching sample numerals.</summary>
    private static void InspectPicked()
    {
        if (_pickInspector is not { } inspector)
        {
            return;
        }
        if (ShellPick.Selection is not { } picked || picked.Revision != (ulong)Feed.State.Revision)
        {
            inspector.SelectPiece(null);
            return;
        }
        (string Domain, int Height)? sample = null;
        try
        {
            if (Style() is { } style && style.Digest == StateDigest && Poses(style.RowCount) is { } poses
                && poses.Digest == StateDigest && picked.Piece < poses.Poses.Length)
            {
                int row = poses.Poses[(int)picked.Piece];
                sample = style.InspectRow(row);
            }
        }
        catch (Exception error) when (error is FormatException or JsonException or IOException or KeyNotFoundException or InvalidOperationException)
        {
            Safe.Report(error);
        }
        inspector.SelectPiece(new PieceInspection(picked.Piece, picked.Sticker, picked.Cell, Feed.State.Mode, sample));
    }

    /// <summary>ADR-D01.67: the Target key names the cell of the current selection.</summary>
    private static void NamePickedTarget(ChromeFrame frame)
    {
        if (ShellPick.Selection is not { } picked || picked.Revision != (ulong)Feed.State.Revision)
        {
            frame.Say("Pick a sticker at the current revision to name the Target.", true);
        }
        else if (picked.Cell is not int cell)
        {
            frame.Say("The picked piece is off the lattice and has no cell.", true);
        }
        else if (Host?.Name(cell) == true)
        {
            frame.Say(FormattableString.Invariant($"Target · cap {cell}"));
        }
    }

    /// <summary>Adds the draft rows after the table's, then relabels with the owner's words and the display names.</summary>
    public static void AddRows(CommandRegistry registry, string folder)
    {
        var rows = File.ReadAllText(Path.Combine(folder, "command-rows-d02.json"));
        var drafts = new CommandRegistry();
        drafts.AddRows(rows);
        registry.AddRows(rows);
        DraftIds.Clear();
        DraftIds.UnionWith(drafts.Order);
        DraftIds.UnionWith(GripFrame.AddDraftRows(registry, rows));
        registry.Relabel(File.ReadAllText(Path.Combine(folder, "command-labels.json")));
    }

    /// <summary>Marks the draft rows in the palette, shows that D02 is on, and adds the driver.</summary>
    public static void Attach(Control parent, ChromeFrame frame, CommandPalette palette, Action observePicks)
    {
        palette.DraftRows.UnionWith(DraftIds);
        if (frame.OpenGround && !ShellChrome.SampleOn)
        {
            var inspector = _pickInspector = new InspectorModel();
            var strip = new InspectorStrip(inspector, () => frame.Tokens, why => frame.Say(why, true));
            frame.SetTopStrip(strip);
            frame.Rebuilt += strip.Refresh;
            ShellPick.SelectionChanged += InspectPicked;
            frame.TreeExiting += () =>
            {
                ShellPick.SelectionChanged -= InspectPicked;
                _pickInspector = null;
            };
            // ADR-D01.67: the command table's Set Target row already has E in the key map.
            frame.HostKeys["E"] = () => NamePickedTarget(frame);
        }
        var state = System.Environment.GetEnvironmentVariable("M600_D02_STATE") is { Length: > 0 } named ? named : "no state";
        var chip = SessionWidgets.Chip("D02 · " + state, frame.Tokens.C("accent.strong"),
            "ADR-D02's build: the Jumble and research surfaces; the palette marks the draft rows");
        chip.Name = "D02Chip";
        frame.BarTools.AddChild(chip);
        var binding = SessionWidgets.Chip(Unbound, frame.Tokens.C("text.dim"), "No sample state has this session's pose digests");
        binding.Name = "D02BindChip";
        frame.BarTools.AddChild(binding);
        var words = binding.GetChild<Label>(0);
        BindingChanged += () => Timed("chip", () =>
        {
            if (!GodotObject.IsInstanceValid(binding))
            {
                return;
            }
            words.Text = Bound is { } state ? BoundWords(state) : Unbound;
            binding.TooltipText = Bound is { } named
                ? $"The session's poses are sample state {named}'s, by their two digests"
                : "No sample state has this session's pose digests: every D02 slot is stale and its readouts pending";
            // SF (27 Nov 2026): the label and tooltip change in place; a frame rebuild here cost 160-560 ms per twist.
        });
        try
        {
            LoadSamples();
        }
        catch (Exception error) when (error is IOException or JsonException or KeyNotFoundException or InvalidOperationException)
        {
            frame.Say("D02 samples did not load: " + error.Message, true);
        }
        parent.AddChild(new D02Driver(frame, observePicks));
    }
}

/// <summary>
/// The D02 driver. During the first seconds it keeps the owner's window rule (<see cref="D02Window"/>). Once a second it
/// writes the window's place to d02-window.json in the run folder (a watched restart opens there), quits when the watch
/// command writes d02-quit, and reloads the D02 data when d02-reload changes. With M600_D02_SCRIPT it plays the script:
/// each step's chord goes to the frame's own key entry, through the shell's router as a typed key does, and after each
/// step it writes the screen's text to d02-screen.jsonl; after the last step it quits.
/// </summary>
internal sealed partial class D02Driver : Node
{
    private const double SettleSeconds = 6, PollSeconds = 1, StartSeconds = 1.5;
    private const int StepFrames = 8;
    private readonly ChromeFrame _frame;
    private readonly string? _folder;
    private readonly List<JsonElement> _steps = new();
    private string _scriptName = "";
    private double _age, _poll;
    private string _window = "", _reload = "";
    private int _reloads;
    // The script's progress: the next step, and the frames left before the screen is written.
    private int _next = -1, _wait = -1;
    private double _sleep;
    private string _doing = "start";
    // B10: the older releases the script cancelled. C2-1: the whole screen as it read before a cancel (or the negative
    // control's change), with the jobs' clock held until the dump reads it again after frames have passed.
    private int _cancels;
    private IReadOnlyList<(string Group, string Text)>? _screenBefore;
    // The frame's rebuilds since the driver started: a frame that rebuilds every frame never lays its views out.
    private int _rebuilds;

    private readonly Action _observePicks;

    public D02Driver(ChromeFrame frame, Action observePicks)
    {
        _frame = frame;
        _observePicks = observePicks;
        frame.Rebuilt += () => _rebuilds++;
        Name = "D02Driver";
        _folder = System.Environment.GetEnvironmentVariable("M600_EVENTS_DIR") is { Length: > 0 } folder && Directory.Exists(folder) ? folder : null;
        if (System.Environment.GetEnvironmentVariable("M600_D02_SCRIPT") is { Length: > 0 } script)
        {
            Load(script);
        }
    }

    private void Load(string path)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(path));
        var root = document.RootElement;
        if (root.GetProperty("format").GetString() != "magic600-d02-script/1")
        {
            throw new FormatException("a D02 script needs the format magic600-d02-script/1");
        }
        _scriptName = root.GetProperty("name").GetString() ?? "";
        foreach (var step in root.GetProperty("steps").EnumerateArray())
        {
            _steps.Add(step.Clone());
        }
        _next = 0;
    }

    public override void _Ready() => D02Window.Settle();

    public override void _Process(double delta)
    {
        _observePicks();
        _age += delta;
        if (_age < SettleSeconds)
        {
            D02Window.Settle();
        }
        else
        {
            // B32 findings: past the first seconds the shell reorders nothing, but it keeps noting the owner's acts on
            // its window for the window's life.
            D02Window.Watch();
        }
        if ((_poll += delta) >= PollSeconds)
        {
            _poll = 0;
            Poll();
        }
        if (_next >= 0)
        {
            Play(delta);
        }
    }

    private string? RunFile(string name) => _folder == null ? null : Path.Combine(_folder, name);

    private void Poll()
    {
        if (RunFile("d02-window.json") is { } place)
        {
            var position = DisplayServer.WindowGetPosition();
            var size = DisplayServer.WindowGetSize();
            var text = string.Create(CultureInfo.InvariantCulture,
                $"{{\"position\":[{position.X},{position.Y}],\"size\":[{size.X},{size.Y}],\"topmost\":{(D02Window.Topmost() ? "true" : "false")},\"gave_back\":{D02Window.GaveBack},\"owner_acted\":{(D02Window.OwnerActed ? "true" : "false")}}}\n");
            if (text != _window)
            {
                _window = text;
                File.WriteAllText(place, text, new UTF8Encoding(false));
            }
        }
        if (RunFile("d02-quit") is { } quit && File.Exists(quit))
        {
            GetTree().Quit();
            return;
        }
        if (RunFile("d02-reload") is { } reload && File.Exists(reload))
        {
            var asked = File.ReadAllText(reload).Trim();
            if (asked != _reload)
            {
                _reload = asked;
                Reload();
            }
        }
    }

    /// <summary>Reads the D02 data again (the watch command's data-only change): from M600_D02_DATA when set, else the
    /// build's staged copy. It reads the samples' digests again; the next shown revision binds against them.</summary>
    private void Reload()
    {
        try
        {
            var count = ShellJumble.LoadSamples();
            _reloads++;
            _frame.Say($"D02 data reloaded ({_reloads}): {count} sample states");
        }
        catch (Exception error) when (error is IOException or JsonException or KeyNotFoundException or InvalidOperationException)
        {
            _frame.Say("D02 data did not reload: " + error.Message, true);
        }
    }

    private void Play(double delta)
    {
        if (_age < StartSeconds)
        {
            return;
        }
        if (_sleep > 0)
        {
            _sleep -= delta;
            return;
        }
        if (_wait > 0)
        {
            _wait--;
            return;
        }
        if (_wait == 0)
        {
            // Brief SF2: a dump records committed state, so it waits for the answers of the keys posted before it.
            if (ShellMotion.Posted > 0) return;
            Dump();
            _wait = -1;
            if (_screenBefore != null)
            {
                // The second read is written: the jobs' clock runs again.
                _screenBefore = null;
                if (ShellJumble.Host is { } host)
                {
                    host.Paused = false;
                }
            }
        }
        if (_next == 0 && _doing == "start")
        {
            Dump();
            _doing = "";
            // A run never inherits the real pointer: it starts with the pointer parked outside the window, so F1's
            // help topic (ADR-D01.73's interim route) and the hover pick read only the script's own pointer steps.
            var parked = new Vector2(-8, -8);
            Input.ParseInputEvent(new InputEventMouseMotion { Position = parked, GlobalPosition = parked });
        }
        if (_next >= _steps.Count)
        {
            _next = -1;
            GetTree().Quit();
            return;
        }
        var step = _steps[_next++];
        if (step.TryGetProperty("chord", out var chord))
        {
            _doing = "chord " + chord.GetString();
            try
            {
                Press(chord.GetString()!);
            }
            catch (FormatException error)
            {
                // A key Godot does not name (KeyJ, not J) is written as such, so the run's expectation fails on it.
                _doing += " (not pressed: " + error.Message + ")";
            }
        }
        else if (step.TryGetProperty("hold", out var hold) || step.TryGetProperty("release", out hold))
        {
            // ADR-D02 B20: a held key, down at "hold" and up at "release", so frames are drawn while it is held.
            var down = step.TryGetProperty("hold", out _);
            _doing = (down ? "hold " : "release ") + hold.GetString();
            var key = hold.GetString() == GripFrame.Modifier ? Key.Ctrl : OS.FindKeycodeFromString(hold.GetString()!);
            if (key == Key.None)
            {
                _doing += " (not pressed: no key is named " + hold.GetString() + ")";
            }
            else
            {
                Input.ParseInputEvent(new InputEventKey { Keycode = key, PhysicalKeycode = key, Pressed = down });
            }
        }
        else if (step.TryGetProperty("text", out var text))
        {
            _doing = "text " + text.GetString();
            foreach (var character in text.GetString()!)
            {
                Type(character);
            }
        }
        else if (step.TryGetProperty("drag", out var drag))
        {
            var dx = drag[0].GetSingle();
            var dy = drag[1].GetSingle();
            // ADR-D01.41: "in": "local" starts the drag in Local's drawn opening instead of Global's.
            var inLocal = step.TryGetProperty("in", out var view) && view.GetString() == "local";
            _doing = "drag " + dx.ToString(CultureInfo.InvariantCulture) + " " + dy.ToString(CultureInfo.InvariantCulture)
                + (inLocal ? " in local" : "");
            Drag(inLocal ? _frame.LocalOpening : _frame.Opening, new Vector2(dx, dy));
        }
        else if (step.TryGetProperty("link", out var link))
        {
            // ADR-D01.41: links or unlinks Local's rotation (no command row yet: row request camera.link).
            _doing = "link " + (link.GetBoolean() ? "on" : "off");
            ShellLocal.Linked = link.GetBoolean();
        }
        else if (step.TryGetProperty("wheel", out var wheel))
        {
            // Wheel notches at the opening's centre through the shell's input: positive up (in), negative down (out).
            var notches = wheel.GetInt32();
            _doing = "wheel " + notches.ToString(CultureInfo.InvariantCulture);
            var at = _frame.Opening.Position + _frame.Opening.Size / 2;
            var button = notches > 0 ? MouseButton.WheelUp : MouseButton.WheelDown;
            for (var i = 0; i < Math.Abs(notches); i++)
            {
                Input.ParseInputEvent(new InputEventMouseButton { ButtonIndex = button, Pressed = true, Position = at, GlobalPosition = at, Factor = 1 });
                Input.ParseInputEvent(new InputEventMouseButton { ButtonIndex = button, Pressed = false, Position = at, GlobalPosition = at, Factor = 1 });
            }
        }
        else if (step.TryGetProperty("click", out var click))
        {
            // ADR-D02 B17: a left click at fractions of the Global opening (D02/ShellPick.cs); "in": "local" clicks in
            // Local's drawn opening instead (checkpoint C3.9, F2).
            var inLocal = step.TryGetProperty("in", out var view) && view.GetString() == "local";
            _doing = ShellPick.ScriptClick(inLocal ? _frame.LocalOpening : _frame.Opening, click, inLocal);
        }
        else if (step.TryGetProperty("cancel_older", out _))
        {
            // B10's Q3.b check in the shell (C2-1): the jobs' clock is held, the whole screen is read, and the host
            // cancels every release at the named cap that is no longer the latest request, as the job layer will;
            // frames pass, and the dump reads the whole screen again. The run compares the two reads (N-1: no cancel
            // of an older release may change the screen).
            _doing = "cancel older";
            if (ShellJumble.Host is { } host)
            {
                host.Paused = true;
                _screenBefore = ScreenText.Of(_frame);
                _cancels += host.CancelOlder();
            }
        }
        else if (step.TryGetProperty("screen_control", out _))
        {
            // C2-1's negative control: the same two reads around a deliberate change (Local's open section closes),
            // which the run must count as a change.
            _doing = "screen control";
            if (ShellJumble.Host is { } host)
            {
                host.Paused = true;
                _screenBefore = ScreenText.Of(_frame);
                if (host.Local.Open is { } open)
                {
                    host.Local.Toggle(open);
                }
            }
        }
        else if (step.TryGetProperty("open", out var open))
        {
            // B11: opens one of Local's sections, as a click on its header does; an open section stays open.
            _doing = "open " + open.GetString();
            if (ShellJumble.Host is { } host && host.Local.Open != open.GetString())
            {
                host.Local.Toggle(open.GetString()!);
            }
        }
        else if (step.TryGetProperty("hover_cover", out var hover))
        {
            // B11: hovering an id in Cap covers, as the pointer over it does. The dump shows the inspect line and the
            // cap map's marks, which must not change.
            _doing = "hover cover " + hover.GetInt32().ToString(CultureInfo.InvariantCulture);
            ShellJumble.Host?.Inspect.Hover(hover.GetInt32());
        }
        else if (step.TryGetProperty("focus", out var focus))
        {
            // B12: gives keyboard focus to a control of Local's sections by name, as a click on it does.
            _doing = "focus " + focus.GetString();
            // B25: a control of a dock panel (the Word panel) is found in the frame, opening its dock first.
            if (focus.GetString() == WordPanel.PanelId && ShellJumble.Host is { } wordHost)
            {
                if (wordHost.Words.IsVisibleInTree())
                {
                    wordHost.Words.GrabFocus();
                }
                else
                {
                    _frame.RevealAndFocus(DockSide.Left, WordPanel.PanelId, wordHost.Words);
                    _doing += " (dock opened)";
                }
            }
            // B26: the ν-corrections panel, the same way.
            else if (focus.GetString() == NuCorrectionsPanel.PanelId && ShellJumble.Host is { } nuFocus)
            {
                if (nuFocus.Corrections.IsVisibleInTree())
                {
                    nuFocus.Corrections.GrabFocus();
                }
                else
                {
                    nuFocus.OpenCorrections();
                    _doing += " (dock opened)";
                }
            }
            else if (ShellJumble.Host?.Surface.FindChild(focus.GetString()!, true, false) is Control target)
            {
                target.GrabFocus();
            }
            else
            {
                _doing += " (not found)";
            }
        }
        else if (step.TryGetProperty("cap_hover", out var capHover))
        {
            // The owner's bug report of 26 November 2026: the pointer over a cap map cell, through the shell's input.
            _doing = "cap hover " + capHover.GetInt32().ToString(CultureInfo.InvariantCulture) + CapPointer(capHover.GetInt32(), false);
        }
        else if (step.TryGetProperty("cap_click", out var capClick))
        {
            // A left click on a cap map cell, through the shell's input.
            _doing = "cap click " + capClick.GetInt32().ToString(CultureInfo.InvariantCulture) + CapPointer(capClick.GetInt32(), true);
        }
        else if (step.TryGetProperty("hover_control", out var hovered))
        {
            var name = hovered.GetString()!;
            _doing = "hover " + name;
            if (_frame.FindChild(name, true, false) is Control control && control.IsVisibleInTree())
            {
                // The cap map extends below the dock's scroll area on a short window.
                var bounds = control.GetGlobalRect();
                var visible = bounds.Intersection(control.GetViewportRect());
                for (Node? parent = control.GetParent(); parent != null; parent = parent.GetParent())
                    if (parent is Control { ClipContents: true } clip) visible = visible.Intersection(clip.GetGlobalRect());
                if (visible.HasArea())
                {
                    var centre = bounds.GetCenter();
                    var at = visible.HasPoint(centre) ? centre : visible.GetCenter();
                    Input.ParseInputEvent(new InputEventMouseMotion { Position = at, GlobalPosition = at });
                }
                else _doing += " (clipped)";
            }
            else _doing += " (not found)";
        }
        else if (step.TryGetProperty("click_control", out var clicked))
        {
            // A left click at the centre of a control of the frame, found by its name (the cap prompt's rows), through
            // the shell's input as the pointer gives it.
            _doing = "click " + clicked.GetString();
            if (_frame.FindChild(clicked.GetString()!, true, false) is Control control && control.IsVisibleInTree())
            {
                Click(control.GetGlobalRect().GetCenter());
            }
            else
            {
                _doing += " (not found)";
            }
        }
        else if (step.TryGetProperty("command", out var command))
        {
            // A row run as the palette runs it (ShellChrome.Run: the unavailable reason, the echo, then the row), with
            // the step's argument if it gives one; a row that throws reaches the error lamp through Safe, as a palette
            // row's does.
            var row = command.GetString()!;
            var argument = step.TryGetProperty("argument", out var given) ? given.GetString() : null;
            _doing = "command " + row + (argument != null ? " " + argument : "");
            if (ShellChrome.Run is { } run)
            {
                Safe.Raise(run, row, argument);
            }
            else
            {
                _doing += " (no Run)";
            }
        }
        else if (step.TryGetProperty("click_local", out var clickLocal))
        {
            _doing = "local " + ShellPick.ScriptClick(_frame.LocalOpening, clickLocal);
        }
        else if (step.TryGetProperty("grip_hover", out var gripHover))
        {
            _doing = GripPointer("hover", gripHover.GetString()!);
        }
        else if (step.TryGetProperty("grip_click", out var gripClick))
        {
            _doing = GripPointer("click", gripClick.GetString()!);
        }
        else if (step.TryGetProperty("wait_ms", out var wait))
        {
            _doing = "wait " + wait.GetInt32().ToString(CultureInfo.InvariantCulture) + " ms";
            _sleep = wait.GetInt32() / 1000.0;
        }
        else
        {
            throw new FormatException("Unknown D02 script step: " + step);
        }
        // ADR-D02 B19/B24: a step marked "quiet" writes no screen dump, so a gate row's repeated motion is timed
        // without the dump's work in its frames (d02_run.py expand_repeat marks them). A "quick" step (ADR-D01.55)
        // dumps at once and flushes the buffered input.
        _wait = step.TryGetProperty("quiet", out var quietStep) && quietStep.ValueKind == JsonValueKind.True ? -1
            : step.TryGetProperty("quick", out var quick) && quick.GetBoolean() ? 0 : StepFrames;
        if (_wait == 0) Input.FlushBufferedEvents();
    }

    private static string GripPointer(string action, string pane)
    {
        var name = "grip " + action + " " + pane;
        Vector2? point = pane == "off" ? Vector2.Zero : ShellGrip.Centre(pane);
        if (point is not { } at) return name + " (clipped)";
        if (action == "hover")
        {
            Input.ParseInputEvent(new InputEventMouseMotion { Position = at, GlobalPosition = at });
        }
        else
        {
            Input.ParseInputEvent(new InputEventMouseButton
            {
                ButtonIndex = MouseButton.Left, ButtonMask = MouseButtonMask.Left, Pressed = true,
                Position = at, GlobalPosition = at, CtrlPressed = true,
            });
            Input.ParseInputEvent(new InputEventMouseButton
            {
                ButtonIndex = MouseButton.Left, Pressed = false, Position = at, GlobalPosition = at, CtrlPressed = true,
            });
        }
        return name;
    }

    /// <summary>A chord such as F1, Escape or Ctrl+Shift+P, pressed and released through the shell's input.</summary>
    private static void Press(string chord)
    {
        var parts = chord.Split('+');
        var key = OS.FindKeycodeFromString(parts[^1]);
        if (key == Key.None)
        {
            throw new FormatException("no key is named " + parts[^1]);
        }
        Send(key, 0, parts.Contains("Ctrl"), parts.Contains("Alt"), parts.Contains("Shift"));
    }

    /// <summary>One typed character: its key with its Unicode value, as a keyboard gives it to a text field.</summary>
    private static void Type(char character)
    {
        var upper = char.ToUpperInvariant(character);
        var key = upper is >= 'A' and <= 'Z' ? (Key)((long)Key.A + (upper - 'A'))
            : character is >= '0' and <= '9' ? (Key)((long)Key.Key0 + (character - '0'))
            : character switch { '.' => Key.Period, ' ' => Key.Space, '-' => Key.Minus, '_' => Key.Minus, _ => Key.None };
        Send(key, character, false, false, char.IsUpper(character) || character == '_');
    }

    /// <summary>A left click at a point of the window: the pointer moves there, then presses and releases.</summary>
    private static void Click(Vector2 at)
    {
        Input.ParseInputEvent(new InputEventMouseMotion { Position = at, GlobalPosition = at });
        Input.ParseInputEvent(new InputEventMouseButton { ButtonIndex = MouseButton.Left, Pressed = true, Position = at, GlobalPosition = at, ButtonMask = MouseButtonMask.Left });
        Input.ParseInputEvent(new InputEventMouseButton { ButtonIndex = MouseButton.Left, Pressed = false, Position = at, GlobalPosition = at });
    }

    /// <summary>The pointer to a cap map cell's centre, and for a click a press and a release there; "" when the cell
    /// is on screen, else why nothing was sent.</summary>
    private static string CapPointer(int cap, bool click)
    {
        if (ShellJumble.Host?.CapMap.Map is not { } map || !GodotObject.IsInstanceValid(map) || !map.IsVisibleInTree() || map.CellRect(cap) is not Rect2 cell)
        {
            return " (no cap map cell)";
        }
        var at = map.GetGlobalRect().Position + cell.GetCenter();
        if (click)
        {
            Click(at);
        }
        else
        {
            Input.ParseInputEvent(new InputEventMouseMotion { Position = at, GlobalPosition = at });
        }
        return "";
    }

    /// <summary>A left-button drag from the opening's centre, in ten motion steps, through the shell's input.</summary>
    private static void Drag(Rect2 opening, Vector2 by)
    {
        var at = opening.Position + opening.Size / 2;
        Input.ParseInputEvent(new InputEventMouseButton { ButtonIndex = MouseButton.Left, Pressed = true, Position = at, GlobalPosition = at, ButtonMask = MouseButtonMask.Left });
        for (var i = 0; i < 10; i++)
        {
            at += by / 10;
            Input.ParseInputEvent(new InputEventMouseMotion { Position = at, GlobalPosition = at, Relative = by / 10, ButtonMask = MouseButtonMask.Left });
            Input.FlushBufferedEvents();
        }
        Input.ParseInputEvent(new InputEventMouseButton { ButtonIndex = MouseButton.Left, Pressed = false, Position = at, GlobalPosition = at });
    }

    private static void Send(Key key, long unicode, bool ctrl, bool alt, bool shift)
    {
        var down = new InputEventKey { Keycode = key, PhysicalKeycode = key, Unicode = (char)unicode, Pressed = true, CtrlPressed = ctrl, AltPressed = alt, ShiftPressed = shift };
        Input.ParseInputEvent(down);
        var up = new InputEventKey { Keycode = key, PhysicalKeycode = key, Pressed = false, CtrlPressed = ctrl, AltPressed = alt, ShiftPressed = shift };
        Input.ParseInputEvent(up);
    }

    /// <summary>One line of d02-screen.jsonl: the step, the window's state and the screen's text, pane by pane.</summary>
    private void Dump()
    {
        if (RunFile("d02-screen.jsonl") is not { } path)
        {
            return;
        }
        using var buffer = new MemoryStream();
        using (var json = new Utf8JsonWriter(buffer))
        {
            json.WriteStartObject();
            json.WriteString("script", _scriptName);
            json.WriteNumber("step", _next);
            json.WriteString("do", _doing);
            json.WriteBoolean("sample", ShellChrome.SampleOn);
            json.WriteStartObject("window");
            json.WriteBoolean("foreground", D02Window.Foreground());
            json.WriteBoolean("topmost", D02Window.Topmost());
            json.WriteNumber("gave_back", D02Window.GaveBack);
            json.WriteBoolean("owner_acted", D02Window.OwnerActed);
            json.WriteEndObject();
            json.WriteNumber("camera_turns", ShellJumble.Camera.Turns);
            // The camera the shell sends to m6h_set_camera (column-major), so a run can check the drag's sense.
            json.WriteStartArray("camera");
            foreach (var value in ShellJumble.Camera.ColumnMajor())
            {
                json.WriteNumberValue(value);
            }
            json.WriteEndArray();
            // The wheel zoom: the notches that changed a zoom, Global's factor and the zoom the core was last sent.
            json.WriteStartObject("zoom");
            json.WriteNumber("notches", ShellJumble.Zoom.Notches);
            json.WriteNumber("factor", ShellJumble.Zoom.Factor(HostLibrary.View));
            if (ShellShape.SentZoom(HostLibrary.View) is float sent)
            {
                json.WriteNumber("sent", sent);
            }
            json.WriteEndObject();
            ShellPick.Dump(json);
            json.WriteString("piece_inspect", ShellJumble.PickInspectLine);
            ShellLocal.Dump(json);
            ShellResearch.Dump(json);
            ShellDocuments.Dump(json);
            ShellSession.Dump(json);
            // SF5: the Global opening's size in pixels; a twist must not change it (the core's surface follows it).
            json.WriteStartArray("global_opening");
            json.WriteNumberValue((int)Math.Floor(_frame.Opening.Size.X));
            json.WriteNumberValue((int)Math.Floor(_frame.Opening.Size.Y));
            json.WriteEndArray();
            // B11: the covers lines, the inspect line, the cap map's marks and the focus layers' keys.
            if (ShellJumble.Host is { } shown)
            {
                json.WriteStartArray("covers");
                foreach (var line in shown.Covers?.Lines() ?? Array.Empty<string>())
                {
                    json.WriteStringValue(line);
                }
                json.WriteEndArray();
                json.WriteString("inspect", shown.Inspect.Line(shown.CapMap.Map.Map));
                var marks = shown.CapMap.Map;
                json.WriteString("map_marks", marks.Threshold.Formula + "|" + marks.Named + "|" + string.Join(",", marks.Meeting) + "|" + shown.Naming.Hovered);
                json.WriteNumber("named", shown.Naming.Named ?? -1);
                // Local is never empty: the cap the host gives Local and the cap the shell frames there (-1: none),
                // whether Local's opening is shown, and Local's readout while the cap is not named.
                json.WriteStartObject("local");
                json.WriteNumber("cap", shown.LocalCap.Shown ?? -1);
                json.WriteNumber("framed", ShellLocal.Cap ?? -1);
                json.WriteBoolean("opening", _frame.LocalOpeningShown);
                json.WriteBoolean("hidden", _frame.LocalHide.Hidden);
                json.WriteString("words", shown.LocalWords ?? "");
                json.WriteEndObject();
                // The cap map's hovered cell, the pointer's shape over it and its tooltip.
                json.WriteStartObject("cap_map");
                json.WriteNumber("hovered", marks.Hovered ?? -1);
                json.WriteString("cursor", marks.MouseDefaultCursorShape.ToString());
                json.WriteString("tooltip", marks.Hovered is int hovered ? marks.TipAt(hovered) : "");
                json.WriteEndObject();
                // The cap prompt (jumble.cap.name without a number): open or not, its field, its rows' count, the
                // highlighted cap (-1: none), the caps shown and what its last Enter said.
                json.WriteStartObject("cap_prompt");
                var prompt = shown.Prompt.Model;
                json.WriteBoolean("open", shown.Prompt.IsOpen);
                json.WriteString("query", prompt.Query);
                json.WriteNumber("rows", prompt.Rows.Count);
                json.WriteNumber("highlighted", prompt.Highlighted is int row ? prompt.Rows[row].Cap : -1);
                json.WriteStartArray("window");
                foreach (var shownRow in prompt.Window)
                {
                    json.WriteNumberValue(shownRow.Cap);
                }
                json.WriteEndArray();
                json.WriteString("said", prompt.Said ?? "");
                json.WriteEndObject();
                json.WriteStartArray("focus");
                foreach (var layers in shown.Layers)
                {
                    json.WriteStringValue(string.Join(" / ", layers.Predicates.Select(FocusLayers.KeyText)) + " | focused " + (layers.Focused ?? "none") + " | " + layers.Inactive);
                }
                json.WriteEndArray();
                // B12: the repair list's focused row, its step label and the solver's picks.
                json.WriteStartObject("repair");
                json.WriteNumber("rows", shown.Repair?.Rows.Count ?? -1);
                json.WriteNumber("focused", shown.Repair?.Focused ?? 0);
                json.WriteString("step", shown.Scrub.Label);
                // B32 findings: which control holds the keys (the repair list's key route gives it the list).
                json.WriteString("key_focus", GetViewport().GuiGetFocusOwner()?.Name ?? "");
                json.WriteStartArray("picks");
                foreach (var pick in shown.Picks)
                {
                    json.WriteNumberValue(pick.Number);
                }
                json.WriteEndArray();
                json.WriteEndObject();
                // B13: the staged word, its bar, the repair-pair records and the boundary ledger's unasked acts.
                json.WriteStartObject("stage");
                json.WriteString("word", shown.Stage.Word.Text());
                json.WriteString("bar", shown.Stage.Bar);
                json.WriteNumber("spans", shown.Stage.Word.Spans.Count);
                json.WriteNumber("records", shown.Stage.Records.Count);
                json.WriteNumber("asked", shown.Ledger.Asked);
                json.WriteNumber("unasked", shown.Ledger.Count);
                json.WriteEndObject();
            }
            // The Focus layer (owner, 26 November 2026): its entry on Global's border, the right dock, the editor's line
            // and Global's focus-layer keys.
            if (ShellJumble.Host is { } filtered)
            {
                json.WriteStartObject("focus_layer");
                var entry = _frame.Global is { } g && GodotObject.IsInstanceValid(g) ? g.Tools.GetNodeOrNull<Button>("FocusLayerEntry") : null;
                json.WriteString("entry", entry?.Text ?? "");
                json.WriteString("dock", _frame.Model.Docks[DockSide.Right].Open ?? "");
                json.WriteBoolean("editor_focused", filtered.FilterEditor.Expression.IsVisibleInTree() && filtered.FilterEditor.Expression.HasFocus());
                json.WriteString("line", filtered.Filter?.PreviewLine() ?? "");
                json.WriteString("expression", filtered.FilterEditor.Expression.Text);
                json.WriteStartArray("keys");
                foreach (var layer in filtered.Layers.Count > 0 ? filtered.Layers[0].Predicates : Array.Empty<PredicateLayer>())
                {
                    json.WriteStringValue(FocusLayers.KeyText(layer));
                }
                json.WriteEndArray();
                json.WriteEndObject();
            }
            // B15: the tint channel as the core was sent it.
            ShellTint.Dump(json, _frame);
            ShellError.Dump(json);
            ShellGrip.Dump(json);
            ShellGripKeys.Dump(json);
            // B30: the tuning panel: whether the right dock shows it, the compare, the live tuning, the frame's window
            // colour, and the head's state digest (a knob must leave it as it was).
            if (ShellJumble.TuningHost is { } tuning && _frame != null && GodotObject.IsInstanceValid(_frame))
            {
                json.WriteStartObject("tuning");
                json.WriteString("right_dock", _frame.Model.Docks[DockSide.Right].Open ?? "");
                json.WriteBoolean("shown", GodotObject.IsInstanceValid(tuning) && tuning.IsVisibleInTree());
                // Below ChromeFrame.RightDockBelow the right dock's panel folds to its rail (owner, 26 November 2026).
                float width = _frame.IsInsideTree() ? _frame.GetViewportRect().Size.X : 0;
                json.WriteNumber("window_width", width);
                json.WriteBoolean("folded", width > 0 && width < ChromeFrame.RightDockBelow);
                json.WriteString("slot", tuning.Model.Shown);
                json.WriteBoolean("holding", tuning.Model.Holding);
                json.WriteString("live", tuning.Model.Live.Text);
                json.WriteString("other", tuning.Model.Other.Text);
                json.WriteString("line", tuning.Compare.Text);
                json.WriteString("status", tuning.Status.Text);
                json.WriteString("window", _frame.Tokens.Colours["surface.window"].Value.Hex);
                json.WriteString("state_digest", ShellJumble.StateDigest);
                json.WriteNumber("revision", ShellJumble.StateRevision);
                // B31: the reveal over the fold, the preset in force, Save's issues, the held simulation (the chrome's
                // filter, the matrix the core's view style was last sent, the window colour through the matrix).
                json.WriteBoolean("revealed", _frame.Model.Docks[DockSide.Right].Revealed == Magic600.Chrome.Tuning.PanelId);
                json.WriteString("preset", tuning.Model.Preset ?? "");
                json.WriteBoolean("preset_shipped", tuning.Model.Current?.Shipped == true);
                json.WriteString("preset_line", tuning.PresetInfo.Text);
                json.WriteNumber("presets", tuning.Model.Presets.Count);
                json.WriteStartArray("issues");
                foreach (var issue in tuning.Model.Issues)
                {
                    json.WriteStringValue(issue);
                }
                json.WriteEndArray();
                json.WriteString("issues_shown", tuning.Issues.Text);
                json.WriteString("overrides", string.Join(" ", tuning.Model.Look.Overrides.Keys));
                json.WriteString("simulating", tuning.Model.Simulating ?? "");
                json.WriteBoolean("filter", tuning.Filter.Visible && tuning.Filter.IsInsideTree());
                json.WriteString("simulation_line", tuning.SimulationInfo.Text);
                json.WriteBoolean("core_simulation", ShellStyle.CoreSimulation);
                json.WriteStartArray("core_matrix");
                foreach (var entry in ShellStyle.SentMatrix)
                {
                    json.WriteNumberValue(entry);
                }
                json.WriteEndArray();
                json.WriteString("window_simulated", tuning.Model.Simulating == null ? ""
                    : ColourMetric.Apply(_frame.Tokens.Colours["surface.window"].Value, tuning.Model.Matrix).Hex);
                // A chromatic colour too: a neutral window is a fixed point of every colour-vision matrix.
                var classOne = _frame.Tokens.Colour(Magic600.Chrome.Tuning.ClassSlots[0]);
                json.WriteString("class1", classOne.Hex);
                json.WriteString("class1_simulated", tuning.Model.Simulating == null ? "" : ColourMetric.Apply(classOne, tuning.Model.Matrix).Hex);
                json.WriteEndObject();
            }
            if (_frame != null && GodotObject.IsInstanceValid(_frame))
            {
                // The Library in the left dock (the command step's sample-mode run): the dock's panel, whether the
                // Library panel is shown, its open branches and the records it lists.
                json.WriteStartObject("library");
                json.WriteString("left_dock", _frame.Model.Docks[DockSide.Left].Open ?? "");
                var library = _frame.FindChild("LibraryPanel", true, false) as LibraryPanel;
                json.WriteBoolean("panel", library != null && library.IsVisibleInTree());
                json.WriteStartArray("open");
                foreach (var (title, _) in LibraryModel.Branches)
                {
                    if (library != null && library.Library.IsOpen(title))
                    {
                        json.WriteStringValue(title);
                    }
                }
                json.WriteEndArray();
                json.WriteStartArray("rows");
                foreach (var node in library?.FindChildren("*", "", true, false) ?? new Godot.Collections.Array<Node>())
                {
                    if (node.HasMeta("library_record"))
                    {
                        json.WriteStringValue(node.GetMeta("library_record").AsString());
                    }
                }
                json.WriteEndArray();
                json.WriteEndObject();
                // The error lamp: whether the frame carries it, the failures it counts and the first line of the last.
                json.WriteNumber("rebuilds", _rebuilds);
                json.WriteStartObject("errors");
                json.WriteBoolean("lamp", _frame.GetNodeOrNull<ErrorLamp>(ErrorLamp.NodeName) != null);
                json.WriteNumber("count", ChromeErrors.Count);
                json.WriteNumber("failures", Safe.Failures);
                var errors = ChromeErrors.All;
                json.WriteString("last", errors.Count > 0 ? errors[errors.Count - 1].Split('\n')[0] : "");
                json.WriteEndObject();
            }
            // The frame's geometry (owner, 26 November 2026: "看不到"): the window, the frame's minimum size and
            // the rectangles of the top bar, the docks and the pair, so a run checks that everything is inside.
            if (_frame != null && GodotObject.IsInstanceValid(_frame))
            {
                json.WriteStartObject("frame");
                var window = _frame.GetViewportRect().Size;
                json.WriteStartArray("window");
                json.WriteNumberValue(window.X);
                json.WriteNumberValue(window.Y);
                json.WriteEndArray();
                foreach (var (key, node) in new (string, Control?)[]
                {
                    ("top_bar", _frame.FindChild("TopBar", true, false) as Control), ("left_dock", _frame.FindChild("LeftDock", true, false) as Control),
                    ("right_dock", _frame.FindChild("RightDock", true, false) as Control), ("bottom_dock", _frame.FindChild("BottomDock", true, false) as Control),
                    ("global", _frame.Global), ("local", _frame.Local),
                    // B24: Global's opening, the rectangle a script's click fractions resolve against (the occlusion grid).
                    ("global_opening", _frame.Global?.Slot),
                })
                {
                    if (node == null || !GodotObject.IsInstanceValid(node) || !node.IsVisibleInTree())
                    {
                        continue;
                    }
                    var rect = node.GetGlobalRect();
                    json.WriteStartArray(key);
                    json.WriteNumberValue(rect.Position.X);
                    json.WriteNumberValue(rect.Position.Y);
                    json.WriteNumberValue(rect.Size.X);
                    json.WriteNumberValue(rect.Size.Y);
                    json.WriteEndArray();
                }
                // The minimum sizes that decide whether the frame fits: each part's combined minimum width and height.
                json.WriteStartObject("min");
                foreach (var (key, node) in new (string, Control?)[]
                {
                    ("frame", _frame.GetChildOrNull<Control>(0)), ("top_bar", _frame.FindChild("TopBar", true, false) as Control),
                    ("left_dock", _frame.FindChild("LeftDock", true, false) as Control), ("right_dock", _frame.FindChild("RightDock", true, false) as Control),
                    ("bottom_dock", _frame.FindChild("BottomDock", true, false) as Control), ("global", _frame.Global), ("local", _frame.Local),
                    ("global_slot", _frame.Global?.Slot), ("global_tools", _frame.Global?.Tools), ("local_slot", _frame.Local?.Slot),
                    ("local_tools", _frame.Local?.Tools), ("seam", _frame.FindChild("PairSeam", true, false) as Control),
                    ("pair", _frame.FindChild("MainPair", true, false) as Control),
                })
                {
                    if (node == null || !GodotObject.IsInstanceValid(node))
                    {
                        continue;
                    }
                    var size = node.GetCombinedMinimumSize();
                    json.WriteStartArray(key);
                    json.WriteNumberValue(size.X);
                    json.WriteNumberValue(size.Y);
                    json.WriteEndArray();
                }
                json.WriteEndObject();
                json.WriteEndObject();
            }
            // B26: the ν band's line and cells, and the ν-corrections panel's lines.
            if (ShellJumble.Host is { } nuHost)
            {
                json.WriteStartObject("nu");
                json.WriteString("band", nuHost.Nu?.Line(nuHost.Corrections.Model?.Constants) ?? "");
                json.WriteBoolean("band_visible", nuHost.NuStrip.IsVisibleInTree());
                json.WriteBoolean("panel_focused", nuHost.Corrections.IsVisibleInTree() && nuHost.Corrections.HasFocus());
                json.WriteNumber("band_width", nuHost.NuStrip.Size.X);
                json.WriteStartArray("cells");
                foreach (var cell in nuHost.Nu?.Cells ?? Array.Empty<NuCell>())
                {
                    json.WriteNumberValue(cell.Orbit);
                }
                json.WriteEndArray();
                foreach (var (key, value) in nuHost.Corrections.Lines())
                {
                    json.WriteString(key, value);
                }
                json.WriteEndObject();
            }
            // B25: the Word panel's lines: the commit bar's length, the counter, the state, the signature, the log tab
            // and the crossing sentence.
            if (ShellJumble.Host is { } worded)
            {
                json.WriteStartObject("word");
                json.WriteBoolean("panel_focused", worded.Words.IsVisibleInTree() && worded.Words.HasFocus());
                foreach (var (key, value) in worded.Words.Lines())
                {
                    json.WriteString(key, value);
                }
                json.WriteEndObject();
            }
            json.WriteNumber("cancels", _cancels);
            // C3/C4 finding F5: the bound state's digest and revision at every step, so a view-only run can show that
            // no step changed the session.
            json.WriteStartObject("bound");
            json.WriteString("digest", ShellJumble.StateDigest);
            json.WriteNumber("revision", ShellJumble.StateRevision);
            json.WriteEndObject();
            if (_screenBefore is { } before)
            {
                // The whole screen as it read before this step's cancel or control, beside "lines" read after it.
                json.WriteStartArray("lines_before");
                foreach (var (group, text) in before)
                {
                    json.WriteStartArray();
                    json.WriteStringValue(group);
                    json.WriteStringValue(text);
                    json.WriteEndArray();
                }
                json.WriteEndArray();
            }
            json.WriteStartArray("kept");
            foreach (var line in ShellJumble.Host?.Checks?.Screen() ?? Array.Empty<string>())
            {
                json.WriteStringValue(line);
            }
            json.WriteEndArray();
            // B22 rest: Global's border marks, each chip's label words and its tooltip (a hidden label's words lie there).
            json.WriteStartArray("global_marks");
            foreach (var chip in _frame.Global.Marks.GetChildren().OfType<Control>())
            {
                var words = chip.FindChildren("*", "Label", true, false).OfType<Label>().Select(label => label.Text);
                json.WriteStringValue(string.Join(" ", words) + " | " + chip.TooltipText);
            }
            json.WriteEndArray();
            // C3/C4 finding F6: Local's border marks as drawn (the raw view's caption is read from here).
            json.WriteStartArray("local_marks");
            foreach (var chip in _frame.Local.Marks.GetChildren().OfType<Control>())
            {
                var words = chip.FindChildren("*", "Label", true, false).OfType<Label>().Select(label => label.Text);
                json.WriteStringValue(string.Join(" ", words) + " | " + chip.TooltipText);
            }
            json.WriteEndArray();
            json.WriteStartArray("lines");
            foreach (var (group, text) in ScreenText.Of(_frame))
            {
                json.WriteStartArray();
                json.WriteStringValue(group);
                json.WriteStringValue(text);
                json.WriteEndArray();
            }
            json.WriteEndArray();
            // Each view's marks by their tooltips: with labels off a mark shows its glyph alone and keeps its word there.
            json.WriteStartObject("marks");
            foreach (ViewFrame? view in new[] { _frame.Global, _frame.Local })
            {
                if (view is null || !GodotObject.IsInstanceValid(view))
                {
                    continue;
                }
                json.WriteStartArray(view.ViewName);
                foreach (Node chip in view.Marks.GetChildren())
                {
                    if (chip is Control mark && !mark.IsQueuedForDeletion())
                    {
                        json.WriteStringValue(mark.TooltipText);
                    }
                }
                json.WriteEndArray();
            }
            json.WriteEndObject();
            ShellPreview.Dump(json);
            DumpMotion(json);
            json.WriteEndObject();
        }
        using var stream = new FileStream(path, FileMode.Append, System.IO.FileAccess.Write);
        stream.Write(buffer.ToArray());
        stream.WriteByte((byte)'\n');
    }

    private void DumpMotion(Utf8JsonWriter json)
    {
        json.WriteStartObject("motion");
        json.WriteBoolean("reduced", _frame.Tokens.ReducedMotion);
        json.WriteNumber("state_revision", ShellJumble.StateRevision);
        json.WriteString("state_digest", ShellJumble.StateDigest);
        ShellTurn.DumpMotion(json);
        ShellJumble.Zoom.DumpMotion(json, HostLibrary.View);
        ShellMotion.DumpRawClip(json);
        json.WriteEndObject();
    }
}
