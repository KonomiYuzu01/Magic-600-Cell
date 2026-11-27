using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

// ADR-D02's hosting in the Solve layout (build step B6; ADR-D02.2, .6, .11, .12): the Jumble facts the chrome follows,
// their sources, the entry slot and what the chrome shows for them. The model is pure C#; Ui/JumbleHost.cs and
// Ui/JumbleSurface.cs draw it. Nothing here adds a layout or a dock.

public enum JumbleSessionKind { Solving, Research }

// The adopted revision's Jumble facts: the mode, the exact lattice flag (every piece's pose in K⁺; read from the poses,
// never from N, which may be pending), the named cap, the engine revision and the session's kind.
public sealed record JumbleState(bool Mode, bool LatticeFlag, int? NamedCap, long Revision, JumbleSessionKind SessionKind)
{
    public static JumbleState Off(long revision = 0) => new(false, true, null, revision, JumbleSessionKind.Solving);
}

public interface IJumbleSource
{
    JumbleState State { get; }
    event Action<JumbleState>? Changed;
}

// The source a host feeds at each adopted revision: the product shell adopts the host's state digests (B5) here.
public sealed class JumbleFeed : IJumbleSource
{
    public JumbleState State { get; private set; } = JumbleState.Off();
    public event Action<JumbleState>? Changed;

    public void Adopt(JumbleState state)
    {
        if (state == State) return;
        State = state;
        Safe.Raise(Changed, state);
    }

    // The host's answer to m6h_state_digest (`{"digest", "pose_ids_sha256", "pose_lattice_sha256", "off_lattice"}`)
    // as the facts at `revision`: Jumble is on when the pose digests are there; the lattice flag is exact, from the
    // count of pieces off the lattice. An answer that does not read is Jumble off.
    public static JumbleState Read(string answer, long revision, JumbleSessionKind kind = JumbleSessionKind.Solving)
    {
        try
        {
            using JsonDocument document = JsonDocument.Parse(answer);
            JsonElement root = document.RootElement;
            if (root.GetProperty("pose_ids_sha256").ValueKind != JsonValueKind.String) return JumbleState.Off(revision);
            return new JumbleState(true, root.GetProperty("off_lattice").GetInt64() == 0, null, revision, kind);
        }
        catch (Exception ex) when (ex is JsonException or KeyNotFoundException or InvalidOperationException or FormatException)
        {
            return JumbleState.Off(revision);
        }
    }
}

// A D02 sample state as a source (B1's files): Jumble on, the lattice flag exact from the sample's off-lattice count.
public sealed class SampleJumbleSource : IJumbleSource
{
    public string Sample { get; private set; }
    public JumbleState State { get; private set; }
    public event Action<JumbleState>? Changed;

    public SampleJumbleSource(string sample, string totals, long revision = 0, JumbleSessionKind kind = JumbleSessionKind.Solving)
    {
        Sample = sample; State = Read(totals, revision, kind);
    }

    // A sample's totals.json as the facts at `revision`.
    public static JumbleState Read(string totals, long revision, JumbleSessionKind kind = JumbleSessionKind.Solving)
    {
        using JsonDocument document = JsonDocument.Parse(totals);
        return new JumbleState(true, document.RootElement.GetProperty("off_lattice").GetInt64() == 0, null, revision, kind);
    }

    public void Load(string sample, string totals, long revision)
    {
        JumbleState next = Read(totals, revision, State.SessionKind);
        Sample = sample;
        if (next == State) return;
        State = next;
        Safe.Raise(Changed, next);
    }
}

// What the strip and the rail show in place of a numeral (ADR-D02.6): with Jumble on and the exact lattice flag false,
// `numeral.undefined` as a whole, whatever N is, a pending N included; otherwise the numeral as it is, so a pending N
// on the lattice stays pending and N = 0 shows a true zero. The switch reads the flag only, never N.
public static class JumbleNumerals
{
    public static bool Undefined(JumbleState state) => state.Mode && !state.LatticeFlag;
    public static Numeral Shown(JumbleState state, Numeral numeral) => Undefined(state) ? Numeral.Undefined : numeral;
}

// The chrome's own settings, which the entry slot writes: the dock open on each side and the armed tint modes and
// focus layers, as text. A host maps them onto its frame.
public sealed class ChromeSettings
{
    private readonly Dictionary<string, string> values = new(StringComparer.Ordinal);
    public event Action<string>? Changed;

    public string? Get(string key) => values.TryGetValue(key, out string? value) ? value : null;

    public void Set(string key, string? value)
    {
        if (Get(key) == value) return;
        if (value == null) values.Remove(key); else values[key] = value;
        Safe.Raise(Changed, key);
    }

    public IReadOnlyDictionary<string, string> Snapshot() => new Dictionary<string, string>(values, StringComparer.Ordinal);
}

// The entry slot `view.preset.jumble_entry` (ADR-D02.11, .12), keyed to the mode. Jumble on applies its value; Jumble
// off reverts what it set that the researcher has not changed since. It follows the adopted revision's mode, so undo,
// redo, a branch switch and a reopen agree. Its value is chrome only.
public sealed class EntrySlot
{
    public const string Id = "view.preset.jumble_entry";
    public const string CapMapPanel = "jumble.cap_map";
    public const string ContactClasses = "tint.contact_class";
    public const string NeverHidden = "layer.never_hidden";
    public const string Armed = "armed";
    // The value: the cap map open in Solve's bottom dock, the contact-class mode and the never-hidden set armed. It sets
    // no threshold and no filter and never changes a projection, a layout or a shrink.
    public static readonly IReadOnlyList<KeyValuePair<string, string>> Value = new KeyValuePair<string, string>[]
    {
        new("dock.bottom", CapMapPanel), new(ContactClasses, Armed), new(NeverHidden, Armed),
    };
    // The keys the value may name: chrome settings only.
    public static readonly IReadOnlyList<string> ChromePrefixes = new[] { "dock.", "tint.", "layer." };
    public static readonly IReadOnlyList<string> Forbidden = new[] { "threshold", "filter", "projection", "layout", "shrink" };

    private readonly ChromeSettings settings;
    // What the slot set, with the value each key had before: present while it is applied.
    private Dictionary<string, (string? Before, string Set)>? applied;

    public EntrySlot(ChromeSettings settings) { this.settings = settings; }

    public bool Applied => applied != null;

    public void Apply()
    {
        if (applied != null) return;
        applied = new Dictionary<string, (string?, string)>(StringComparer.Ordinal);
        foreach ((string key, string value) in Value)
        {
            applied[key] = (settings.Get(key), value);
            settings.Set(key, value);
        }
    }

    // Reverts only the keys still holding what the slot set; a key the researcher changed since stays theirs.
    public void Revert()
    {
        if (applied == null) return;
        foreach ((string key, (string? before, string set)) in applied)
            if (settings.Get(key) == set) settings.Set(key, before);
        applied = null;
    }

    public void Follow(JumbleState state) { if (state.Mode) Apply(); else Revert(); }
}

// What the Jumble chrome shows for the facts (ADR-D02.6): the mode cell in the top bar, a mark on every view border,
// the undefined numeral, Local's empty sentence and its four headers.
public static class JumbleChrome
{
    public const string Mark = "mark.jumble";
    public const string MarkWords = "jumble";
    // Local's sections in order: the named-cap cell, the kept twist checks, Cap covers and the repair slot.
    public static readonly IReadOnlyList<string> LocalSections = new[] { "named_cap", "twist_check", "cap_covers", "repair_list" };

    // The host-drawn cell in the top bar, or none with Jumble off.
    public static string? ModeCell(JumbleState state) =>
        !state.Mode ? null : "Jumble · " + (state.LatticeFlag ? "on the lattice" : "off the lattice");

    // The views that carry the Jumble mark: every one while Jumble is on, none otherwise.
    public static IReadOnlyList<string> Marked(JumbleState state, IEnumerable<string> views) =>
        state.Mode ? views.ToArray() : Array.Empty<string>();

    // A section's display name: the record's names where it has one.
    public static string SectionName(string section, IReadOnlyDictionary<string, string> names) =>
        section == "named_cap" ? "Named cap" : names.TryGetValue(section, out string? name) ? name : section;
}

// Local's accordion (ADR-D02.6): the four sections expand one at a time; a closed one keeps a one-line header with its
// count.
public sealed class LocalAccordion
{
    public const string Unavailable = "unavailable";
    private readonly Dictionary<string, (int Count, string? Status)> counts =
        JumbleChrome.LocalSections.ToDictionary(s => s, _ => (0, (string?)null), StringComparer.Ordinal);
    public string? Open { get; private set; }
    public event Action? Changed;

    public IReadOnlyList<string> Sections => JumbleChrome.LocalSections;
    public int Count(string section) => counts[section].Count;

    public void SetCount(string section, int count, string? status = null)
    {
        if (!counts.ContainsKey(section)) throw new ArgumentException("not a Local section: " + section);
        if (counts[section] == (count, status)) return;
        counts[section] = (count, status); Safe.Raise(Changed);
    }

    // Opens a section and closes the open one; the open section again closes it.
    public void Toggle(string section)
    {
        if (!counts.ContainsKey(section)) throw new ArgumentException("not a Local section: " + section);
        Open = Open == section ? null : section;
        Safe.Raise(Changed);
    }

    public string Header(string section, IReadOnlyDictionary<string, string> names) =>
        JumbleChrome.SectionName(section, names) + " · " + (counts[section].Status ?? Count(section).ToString(CultureInfo.InvariantCulture));
}
