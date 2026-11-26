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
/// adds a driver node that keeps the owner's window rule, answers the watch command and plays a D02 script.
/// </summary>
public static class ShellJumble
{
    public const string DraftReason = "a draft row: this build does not carry it out yet";
    private static readonly HashSet<string> DraftIds = new(StringComparer.Ordinal);

    public static bool On => System.Environment.GetEnvironmentVariable("M600_D02") == "1";
    public static IReadOnlyCollection<string> Drafts => DraftIds;

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
    /// build's staged copy. B4 has no surface that draws it yet, so it counts what it read.</summary>
    private void Reload()
    {
        var data = System.Environment.GetEnvironmentVariable("M600_D02_DATA") is { Length: > 0 } source
            ? source : ProjectSettings.GlobalizePath("res://samples");
        var states = Path.Combine(data, data.EndsWith("samples", StringComparison.Ordinal) ? "" : "samples", "jumble", "states.json");
        try
        {
            using var document = JsonDocument.Parse(File.ReadAllText(states));
            _reloads++;
            _frame.Say($"D02 data reloaded ({_reloads}): {document.RootElement.GetProperty("states").GetArrayLength()} sample states");
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
            Press(chord.GetString()!);
        }
        else if (step.TryGetProperty("text", out var text))
        {
            _doing = "text " + text.GetString();
            foreach (var character in text.GetString()!)
            {
                Type(character);
            }
        }
        else if (step.TryGetProperty("wait_ms", out var wait))
        {
            _doing = "wait " + wait.GetInt32().ToString(CultureInfo.InvariantCulture) + " ms";
            _sleep = wait.GetInt32() / 1000.0;
        }
        else
        {
            throw new FormatException("a D02 script step is a chord, a text or a wait_ms");
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
            json.WriteStartArray("lines");
            foreach (var (group, text) in ScreenText.Of(_frame))
            {
                json.WriteStartArray();
                json.WriteStringValue(group);
                json.WriteStringValue(text);
                json.WriteEndArray();
            }
            json.WriteEndArray();
            json.WriteEndObject();
        }
        using var stream = new FileStream(path, FileMode.Append, System.IO.FileAccess.Write);
        stream.Write(buffer.ToArray());
        stream.WriteByte((byte)'\n');
    }
}
