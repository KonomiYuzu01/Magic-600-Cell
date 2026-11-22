using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

public sealed record ChromeCommand(string Id, string Label, string Kind, IReadOnlyList<string> Contexts, string Help);

// One registry of commands (ADR-D01.17): the rows of the command table, and the radial menus, which are
// commands themselves. The palette, the keymap, the shortcut sheet and the radial menus are generated from it.
public sealed class CommandRegistry
{
    public static readonly IReadOnlyList<string> Kinds = Array.AsReadOnly(new[] { "app", "commit", "file", "query", "view", "workspace" });
    private readonly Dictionary<string, ChromeCommand> commands = new(StringComparer.Ordinal);
    private readonly List<string> order = new();
    public IReadOnlyDictionary<string, ChromeCommand> Commands => commands;
    public IReadOnlyList<string> Order => order;

    public void Add(ChromeCommand command)
    {
        if (string.IsNullOrWhiteSpace(command.Id) || string.IsNullOrWhiteSpace(command.Label)) throw J.Error("command", "an id and a label required");
        if (!Kinds.Contains(command.Kind)) throw J.Error(command.Id, "unknown command kind");
        if (!commands.TryAdd(command.Id, command)) throw J.Error(command.Id, "repeated command");
        order.Add(command.Id);
    }

    public static string Location(string id) => id.Contains('.') ? id.Substring(0, id.IndexOf('.')) : id;

    // Reads id, label, kind, contexts and note of every row of the command table.
    public static CommandRegistry FromTable(string text)
    {
        var registry = new CommandRegistry();
        using JsonDocument d = J.Parse(text);
        if (!d.RootElement.TryGetProperty("commands", out JsonElement rows) || rows.ValueKind != JsonValueKind.Array) throw J.Error("commands", "a command list required");
        foreach (JsonElement row in rows.EnumerateArray())
        {
            string id = J.Text(row, "id");
            string help = row.TryGetProperty("note", out JsonElement note) && note.ValueKind == JsonValueKind.String ? note.GetString()! : "";
            registry.Add(new ChromeCommand(id, J.Text(row, "label"), J.Text(row, "kind"), Array.AsReadOnly(J.Strings(row.GetProperty("contexts"), id)), help));
        }
        return registry;
    }
}

public sealed record Binding(string Key, string Context, string Command, string? Argument, string? Label);
public sealed record RadialEntry(string Command, string? Argument, string? Label);
public sealed record RadialMenu(string Id, string Label, IReadOnlyList<RadialEntry> Entries);

// The default keymap (ADR-D01.19 to ADR-D01.21): single keys scoped by context, and the radial menus. A key is
// looked up in the hovered view's context first and then in the global one, so a view key and a command key
// never collide. User overrides are a second file of the same form, applied over the defaults.
public sealed class Keymap
{
    public const string Global = "global";
    public static readonly IReadOnlyList<string> Contexts = Array.AsReadOnly(new[] { Global, "global-view", "local-view", "editor", "preview" });
    public static readonly IReadOnlyList<string> Modifiers = Array.AsReadOnly(new[] { "Ctrl", "Alt", "Shift" });
    public IReadOnlyList<Binding> Bindings { get; }
    public IReadOnlyDictionary<string, RadialMenu> Radials { get; }

    private Keymap(List<Binding> bindings, SortedDictionary<string, RadialMenu> radials) { Bindings = bindings; Radials = radials; }

    public static string KeyName(bool ctrl, bool alt, bool shift, string key) =>
        (ctrl ? "Ctrl+" : "") + (alt ? "Alt+" : "") + (shift ? "Shift+" : "") + key;

    private static string CheckKey(string key)
    {
        string[] parts = key.Split('+');
        if (parts.Length == 0 || parts.Any(p => p.Length == 0) || parts.Take(parts.Length - 1).Any(p => !Modifiers.Contains(p)) || Modifiers.Contains(parts[^1]))
            throw J.Error(key, "expected modifiers Ctrl, Alt, Shift in that order and one key");
        if (KeyName(parts.Contains("Ctrl"), parts.Contains("Alt"), parts.Contains("Shift"), parts[^1]) != key) throw J.Error(key, "modifiers out of order or repeated");
        return key;
    }

    public static Keymap Load(string path, CommandRegistry registry, Keymap? defaults = null) => Parse(File.ReadAllText(path), registry, defaults);

    // With defaults given, the text is the user's override file: a binding replaces the default on the same key
    // and context, and a binding to the empty command removes it.
    public static Keymap Parse(string text, CommandRegistry registry, Keymap? defaults = null)
    {
        using JsonDocument d = J.Parse(text);
        JsonElement root = J.Root(d, "magic600-design-keymap", "radials", "bindings");
        var radials = new SortedDictionary<string, RadialMenu>(StringComparer.Ordinal);
        if (defaults != null) foreach (var radial in defaults.Radials) radials.Add(radial.Key, radial.Value);
        foreach (JsonProperty p in J.Map(root.GetProperty("radials"), "radials"))
        {
            J.Keys(p.Value, p.Name, "label", "entries");
            var entries = new List<RadialEntry>();
            foreach (JsonElement e in p.Value.GetProperty("entries").EnumerateArray()) entries.Add(Entry(e, p.Name, registry));
            if (entries.Count is < 2 or > 8) throw J.Error(p.Name, "a radial menu holds two to eight entries");
            if (!p.Name.StartsWith("radial.", StringComparison.Ordinal)) throw J.Error(p.Name, "a radial menu id starts with radial.");
            var menu = new RadialMenu(p.Name, J.Text(p.Value, "label"), entries.AsReadOnly());
            // A radial menu is itself a command.
            if (!registry.Commands.ContainsKey(p.Name)) registry.Add(new ChromeCommand(p.Name, menu.Label, "view", new[] { "any" }, "Hold the key, move toward an entry and release."));
            radials[p.Name] = menu;
        }
        var bindings = defaults?.Bindings.ToList() ?? new List<Binding>();
        if (root.GetProperty("bindings").ValueKind != JsonValueKind.Array) throw J.Error("bindings", "expected a list");
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonElement b in root.GetProperty("bindings").EnumerateArray())
        {
            string key = CheckKey(J.Text(b, "key")), context = J.Text(b, "context");
            if (!Contexts.Contains(context)) throw J.Error(key, "unknown context " + context);
            if (!seen.Add(context + " " + key)) throw J.Error(key, "bound twice in " + context);
            bindings.RemoveAll(x => x.Key == key && x.Context == context);
            if (defaults != null && b.TryGetProperty("command", out JsonElement none) && none.ValueKind == JsonValueKind.String && none.GetString() == "") continue;
            RadialEntry entry = Entry(b, key, registry, "key", "context");
            bindings.Add(new Binding(key, context, entry.Command, entry.Argument, entry.Label));
        }
        return new Keymap(bindings, radials);
    }

    private static RadialEntry Entry(JsonElement value, string owner, CommandRegistry registry, params string[] more)
    {
        if (value.ValueKind != JsonValueKind.Object) throw J.Error(owner, "expected an object");
        foreach (JsonProperty p in value.EnumerateObject())
            if (p.Name is not ("command" or "argument" or "label") && !more.Contains(p.Name)) throw J.Error(owner, "unknown key " + p.Name);
        string command = J.Text(value, "command");
        if (!registry.Commands.ContainsKey(command)) throw J.Error(owner, "unknown command " + command);
        string? argument = value.TryGetProperty("argument", out JsonElement a) ? J.String(a, owner) : null;
        string? label = value.TryGetProperty("label", out JsonElement l) ? J.String(l, owner) : null;
        if (argument != null && (label == null || argument.Contains('|') || argument.Contains('\n'))) throw J.Error(owner, "a binding with an argument needs a label, and the argument no | or line break");
        return new RadialEntry(command, argument, label);
    }

    // The first context of the chain that binds the key wins: the hovered view, then the focused one, then global.
    public Binding? Resolve(string key, IEnumerable<string> contexts)
    {
        foreach (string context in contexts)
            if (Bindings.FirstOrDefault(b => b.Key == key && b.Context == context) is { } hit) return hit;
        return null;
    }

    public string KeysFor(string command, string? argument) =>
        string.Join(", ", Bindings.Where(b => b.Command == command && b.Argument == argument).Select(b => b.Key).Distinct());
}

public sealed record PaletteRow(string Command, string? Argument, string Label, string Location, string Keys, string? Unavailable, string Help, bool Pinned)
{
    public string Id => Command + "|" + (Argument ?? "");
}

// The palette's rows (ADR-D01.18). A query only filters; the order comes from the solver's own pins, recency
// and frequency and from nothing else, and rows never used keep the registry's order.
public sealed class PaletteModel
{
    private readonly CommandRegistry registry;
    private Keymap keymap;
    // The keymap in force; it changes when the solver binds a key or swaps the preset.
    public Keymap Keymap { get => keymap; set => keymap = value; }
    private readonly Func<string, string?, string?> unavailable;
    private readonly Dictionary<string, (int Count, long Last)> uses = new(StringComparer.Ordinal);
    private readonly SortedSet<string> pins = new(StringComparer.Ordinal);
    private long clock;

    public PaletteModel(CommandRegistry registry, Keymap keymap, Func<string, string?, string?> unavailable)
    {
        this.registry = registry; this.keymap = keymap; this.unavailable = unavailable;
    }

    private PaletteRow Row(string command, string? argument, string? label)
    {
        ChromeCommand c = registry.Commands[command];
        string id = command + "|" + (argument ?? "");
        return new PaletteRow(command, argument, label ?? c.Label, CommandRegistry.Location(command), keymap.KeysFor(command, argument), unavailable(command, argument), c.Help, pins.Contains(id));
    }

    public IReadOnlyList<PaletteRow> Rows(string query)
    {
        var rows = new List<PaletteRow>(); var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (string id in registry.Order) { rows.Add(Row(id, null, null)); seen.Add(id + "|"); }
        // A command bound with an argument is a row of its own, under the binding's label.
        foreach (RadialEntry e in keymap.Bindings.Select(b => new RadialEntry(b.Command, b.Argument, b.Label)).Concat(keymap.Radials.Values.SelectMany(r => r.Entries)))
            if (e.Argument != null && seen.Add(e.Command + "|" + e.Argument)) rows.Add(Row(e.Command, e.Argument, e.Label));
        string[] terms = query.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        bool Matches(PaletteRow r) => terms.All(t => r.Label.Contains(t, StringComparison.OrdinalIgnoreCase) || r.Command.Contains(t, StringComparison.OrdinalIgnoreCase));
        return rows.Where(Matches).Select((row, index) => (row, index))
            .OrderByDescending(x => x.row.Pinned)
            .ThenByDescending(x => uses.TryGetValue(x.row.Id, out var u) ? u.Last : 0)
            .ThenByDescending(x => uses.TryGetValue(x.row.Id, out var u) ? u.Count : 0)
            .ThenBy(x => x.index).Select(x => x.row).ToArray();
    }

    public void Used(PaletteRow row) { uses[row.Id] = ((uses.TryGetValue(row.Id, out var u) ? u.Count : 0) + 1, ++clock); }
    public void Pin(PaletteRow row, bool pinned) { if (pinned) pins.Add(row.Id); else pins.Remove(row.Id); }

    public string Store() => string.Join("\n", pins.Select(p => "pin|" + p)
        .Concat(uses.OrderBy(u => u.Key, StringComparer.Ordinal).Select(u => $"use|{u.Key}|{u.Value.Count.ToString(CultureInfo.InvariantCulture)}|{u.Value.Last.ToString(CultureInfo.InvariantCulture)}")));

    // Rows of commands this build no longer has are dropped; a malformed line refuses the whole text.
    public void Restore(string stored)
    {
        var newUses = new Dictionary<string, (int, long)>(StringComparer.Ordinal); var newPins = new List<string>(); long newest = 0;
        foreach (string line in stored.Split('\n', StringSplitOptions.RemoveEmptyEntries))
        {
            string[] p = line.Split('|');
            if (p[0] == "pin" && p.Length == 3) { if (registry.Commands.ContainsKey(p[1])) newPins.Add(p[1] + "|" + p[2]); }
            else if (p[0] == "use" && p.Length == 5 && int.TryParse(p[3], NumberStyles.None, CultureInfo.InvariantCulture, out int count) && long.TryParse(p[4], NumberStyles.None, CultureInfo.InvariantCulture, out long last) && count > 0)
            { if (registry.Commands.ContainsKey(p[1])) newUses[p[1] + "|" + p[2]] = (count, last); newest = Math.Max(newest, last); }
            else throw J.Error("palette", "unreadable line");
        }
        uses.Clear(); foreach (var u in newUses) uses[u.Key] = u.Value;
        pins.Clear(); foreach (string pin in newPins) pins.Add(pin);
        clock = newest;
    }
}
