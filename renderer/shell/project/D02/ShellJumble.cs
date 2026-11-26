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
    /// <summary>The binding changed.</summary>
    public static event Action? BindingChanged;
    /// <summary>The view rotation a drag in the Global opening turns (owner, 26 November 2026).</summary>
    public static ShellCamera Camera { get; } = new();
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
        return File.Exists(path) ? GlobalTotals.Read(File.ReadAllText(path), SeamCoverage.Complete) : GlobalTotals.Pending;
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
        return File.Exists(path) ? CapMapModel.Read(File.ReadAllText(path)) : CapMapModel.Pending();
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
    public static CapReadout? CapRecord(int cap)
    {
        if (Bound is not { } state)
        {
            return null;
        }
        var path = Path.Combine(JumbleFolder(), state, "cap-" + cap + ".json");
        return File.Exists(path) ? CapReadout.Read(File.ReadAllText(path)) : null;
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
        Feed.Adopt(JumbleFeed.Read(digests, revision));
        Rebind(bound);
    }

    /// <summary>
    /// The head's facts could not be read at this revision (checkpoint C1): nothing is bound, and with Jumble on the
    /// numerals are undefined rather than the last revision's.
    /// </summary>
    public static void Unknown(long revision)
    {
        Feed.Adopt(Feed.State with { LatticeFlag = false, Revision = revision });
        Rebind(null);
    }

    private static void Rebind(string? bound)
    {
        if (bound != Bound)
        {
            Bound = bound;
            BindingChanged?.Invoke();
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
        registry.Relabel(File.ReadAllText(Path.Combine(folder, "command-labels.json")));
    }

    /// <summary>Marks the draft rows in the palette, shows that D02 is on, and adds the driver.</summary>
    public static void Attach(Control parent, ChromeFrame frame, CommandPalette palette)
    {
        palette.DraftRows.UnionWith(DraftIds);
        var state = System.Environment.GetEnvironmentVariable("M600_D02_STATE") is { Length: > 0 } named ? named : "no state";
        var chip = SessionWidgets.Chip("D02 · " + state, frame.Tokens.C("accent.strong"),
            "ADR-D02's build: the Jumble and research surfaces; the palette marks the draft rows");
        chip.Name = "D02Chip";
        frame.BarTools.AddChild(chip);
        var binding = SessionWidgets.Chip(Unbound, frame.Tokens.C("text.dim"), "No sample state has this session's pose digests");
        binding.Name = "D02BindChip";
        frame.BarTools.AddChild(binding);
        var words = binding.GetChild<Label>(0);
        BindingChanged += () =>
        {
            if (!GodotObject.IsInstanceValid(binding))
            {
                return;
            }
            words.Text = Bound is { } state ? BoundWords(state) : Unbound;
            binding.TooltipText = Bound is { } named
                ? $"The session's poses are sample state {named}'s, by their two digests"
                : "No sample state has this session's pose digests: every D02 slot is stale and its readouts pending";
            frame.Invalidate();
        };
        try
        {
            LoadSamples();
        }
        catch (Exception error) when (error is IOException or JsonException or KeyNotFoundException or InvalidOperationException)
        {
            frame.Say("D02 samples did not load: " + error.Message, true);
        }
        parent.AddChild(new D02Driver(frame));
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

    public D02Driver(ChromeFrame frame)
    {
        _frame = frame;
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
        _age += delta;
        if (_age < SettleSeconds)
        {
            D02Window.Settle();
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
            Dump();
            _wait = -1;
        }
        if (_next == 0 && _doing == "start")
        {
            Dump();
            _doing = "";
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
            _doing = "drag " + dx.ToString(CultureInfo.InvariantCulture) + " " + dy.ToString(CultureInfo.InvariantCulture);
            Drag(_frame.Opening, new Vector2(dx, dy));
        }
        else if (step.TryGetProperty("wait_ms", out var wait))
        {
            _doing = "wait " + wait.GetInt32().ToString(CultureInfo.InvariantCulture) + " ms";
            _sleep = wait.GetInt32() / 1000.0;
        }
        else
        {
            throw new FormatException("a D02 script step is a chord, a text, a drag or a wait_ms");
        }
        _wait = StepFrames;
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

    /// <summary>A left-button drag from the opening's centre, in ten motion steps, through the shell's input.</summary>
    private static void Drag(Rect2 opening, Vector2 by)
    {
        var at = opening.Position + opening.Size / 2;
        Input.ParseInputEvent(new InputEventMouseButton { ButtonIndex = MouseButton.Left, Pressed = true, Position = at, GlobalPosition = at, ButtonMask = MouseButtonMask.Left });
        for (var i = 0; i < 10; i++)
        {
            at += by / 10;
            Input.ParseInputEvent(new InputEventMouseMotion { Position = at, GlobalPosition = at, Relative = by / 10, ButtonMask = MouseButtonMask.Left });
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
            json.WriteStartObject("window");
            json.WriteBoolean("foreground", D02Window.Foreground());
            json.WriteBoolean("topmost", D02Window.Topmost());
            json.WriteNumber("gave_back", D02Window.GaveBack);
            json.WriteBoolean("owner_acted", D02Window.OwnerActed);
            json.WriteEndObject();
            json.WriteNumber("camera_turns", ShellJumble.Camera.Turns);
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
            json.WriteEndObject();
        }
        using var stream = new FileStream(path, FileMode.Append, System.IO.FileAccess.Write);
        stream.Write(buffer.ToArray());
        stream.WriteByte((byte)'\n');
    }
}
