using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace Magic600.Chrome;

public enum KeyCapState { Bound, Unbound, Held, OutOfContext, Twist }

// One keycap of the key map: what the key does now, in the contexts and with the modifiers given.
public sealed record KeyCap(string Key, string Legend, float Width, KeyCapState State, bool Down, string? Label, string? Context);

// The key map (ADR-D01.74): the keymap in force drawn as a keyboard. What a key does depends only on the key set,
// the contexts and the held modifiers, and the header names all three, so a key never changes meaning silently.
public static class KeyMapModel
{
    private static IReadOnlyList<(string Key, string Legend, float Width)> Row(string spec) => spec.Split(' ').Select(s =>
    {
        string[] p = s.Split('|');
        return (p[0], p.Length > 1 ? p[1] : p[0], p.Length > 2 ? float.Parse(p[2], CultureInfo.InvariantCulture) : 1f);
    }).ToArray();

    public static readonly IReadOnlyList<IReadOnlyList<(string Key, string Legend, float Width)>> Rows = new[]
    {
        Row("Escape|Esc F1 F2 F3 F4 F5 F6 F7 F8 F9 F10 F11 F12"),
        Row("QuoteLeft|` 1 2 3 4 5 6 7 8 9 0 Minus|- Equal|="),
        Row("Tab|Tab Q W E R T Y U I O P BracketLeft|[ BracketRight|]"),
        Row("A A|A|0 S D F G H J K L Semicolon|; Apostrophe|' Enter|Enter|1.5").Where(k => k.Width > 0).ToArray(),
        Row("Z Z|Z|0 X C V B N M Comma|, Period|. Slash|/").Where(k => k.Width > 0).ToArray(),
        Row("Space|Space|5 Left Right Up Down"),
    };

    public const string Host = "host";
    public static readonly IReadOnlyDictionary<string, string> Where = new Dictionary<string, string>
    {
        ["global-view"] = "pointer over Global", ["local-view"] = "pointer over Local", ["editor"] = "in the editor", ["preview"] = "a preview is open", [Host] = "this window",
    };

    // Twist keys have their own mark: they change the puzzle or the draft, the others do not.
    public static bool IsTwist(string command) => command == "operation.phase.append_twist";

    public static string Label(Binding binding, CommandRegistry registry) =>
        binding.Label ?? (registry.Commands.TryGetValue(binding.Command, out ChromeCommand? c) ? c.Label : binding.Command);

    public static IReadOnlyList<IReadOnlyList<KeyCap>> Caps(Keymap keymap, CommandRegistry registry, IReadOnlyList<string> contexts, bool ctrl, bool alt, bool shift,
        IReadOnlyCollection<string> down, IReadOnlyDictionary<string, string>? host = null) =>
        Rows.Select(row => (IReadOnlyList<KeyCap>)row.Select(k =>
        {
            string full = Keymap.KeyName(ctrl, alt, shift, k.Key); bool held = down.Contains(k.Key);
            KeyCap Cap(KeyCapState state, string? label, string? context) => new(k.Key, k.Legend, k.Width, held ? KeyCapState.Held : state, held, label, context);
            if (keymap.Resolve(full, contexts) is { } binding) return Cap(IsTwist(binding.Command) ? KeyCapState.Twist : KeyCapState.Bound, Label(binding, registry), binding.Context);
            if (host != null && host.TryGetValue(full, out string? own)) return Cap(KeyCapState.Bound, own, Host);
            // Bound, but not where the pointer is: the cap says where it acts.
            if (keymap.Bindings.FirstOrDefault(b => b.Key == full) is { } other) return Cap(KeyCapState.OutOfContext, Label(other, registry), other.Context);
            return Cap(KeyCapState.Unbound, null, null);
        }).ToArray()).ToArray();

    public static string Place(IReadOnlyList<string> contexts)
    {
        string[] named = contexts.Where(c => c != Keymap.Global && Where.ContainsKey(c)).Select(c => Where[c]).Distinct().ToArray();
        return named.Length == 0 ? "everywhere" : string.Join(", ", named);
    }

    public static string Header(string keySet, IReadOnlyList<string> contexts, bool ctrl, bool alt, bool shift)
    {
        string[] held = new[] { ctrl ? "Ctrl" : null, alt ? "Alt" : null, shift ? "Shift" : null }.Where(m => m != null).ToArray()!;
        return $"Key set: {keySet} · {Place(contexts)} · " + (held.Length == 0 ? "no modifier held" : "with " + string.Join("+", held));
    }
}

// Shown while its key is held; a short press pins it and the next short press lets it go.
public sealed class KeyMapShow
{
    public const double Tap = .3;
    private double since;
    public bool Held { get; private set; }
    public bool Pinned { get; private set; }
    public bool Visible => Held || Pinned;
    public event Action? Changed;
    public void Press(double now) { if (Held) return; Held = true; since = now; Safe.Raise(Changed); }
    public void Release(double now) { if (!Held) return; Held = false; if (now - since < Tap) Pinned = !Pinned; Safe.Raise(Changed); }
    public void Pin(bool on) { if (Pinned == on) return; Pinned = on; Safe.Raise(Changed); }
}
