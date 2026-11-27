using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    // ADR-D01.74: a key never changes meaning silently. A key set change names each key it gave another meaning, the
    // key map marks those keys under the modifiers held, and the key sets are the default keymap and the presets.
    private static void D01KeySetsSayEveryChangedKey()
    {
        CommandRegistry registry = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        registry.Relabel(Read("data/design/command-labels.json"));
        Keymap keys = Keymap.Parse(Read("data/design/keymap.json"), registry);
        string leftText = Read("data/design/keymaps/left-hand.json");
        Keymap left = Keymap.Parse(leftText, registry, keys);
        Equal(KeyMapModel.Changes(keys, keys, registry).Count, 0, "a keymap against itself");
        IReadOnlyList<KeyChange> changes = KeyMapModel.Changes(keys, left, registry);
        // Each change is one of the preset's keys, in key order, with what it did before and what it does now.
        Check(changes.Count > 0 && changes.All(c => c.Context == Keymap.Global && "EQXZ".Contains(c.Key, StringComparison.Ordinal)), "the left-hand changes");
        Check(changes.Select(c => c.Key).SequenceEqual(changes.Select(c => c.Key).OrderBy(k => k, StringComparer.Ordinal)), "the changes are out of key order");
        KeyChange z = changes.Single(c => c.Key == "Z");
        Equal(z.Before, null, "plain Z before the preset");
        Equal(z.After, registry.Commands["session.undo"].Label, "plain Z after the preset");
        // Back to the default set is the same changes the other way round.
        Check(KeyMapModel.Changes(left, keys, registry).Select(c => (c.Key, c.Before, c.After)).SequenceEqual(changes.Select(c => (c.Key, c.After, c.Before))), "the way back");
        // The line says the set, the count and the first three; the key map holds the rest.
        string said = KeyMapModel.Said("left-hand", changes);
        Check(said.StartsWith($"Key set: left-hand · {changes.Count} keys changed meaning: ", StringComparison.Ordinal), said);
        Check(said.Contains($"Z was unbound, now {z.After}", StringComparison.Ordinal) == (changes.ToList().IndexOf(z) < 3), said);
        Check(said.EndsWith($"; {changes.Count - 3} more marked on the key map", StringComparison.Ordinal) == (changes.Count > 3), said);
        Equal(KeyMapModel.Said("default", Array.Empty<KeyChange>()), "Key set: default · no key changed meaning");
        Equal(KeyMapModel.Said("x", new[] { new KeyChange("R", "local-view", "Spin", null) }), "Key set: x · 1 key changed meaning: R was Spin, now unbound (pointer over Local)");
        // The marks follow the modifiers held: the plain keys changed, their Ctrl layer did not.
        string[] none = { Keymap.Global };
        IEnumerable<KeyCap> Board(bool ctrl) => KeyMapModel.Mark(KeyMapModel.Caps(left, registry, none, ctrl, false, false, Array.Empty<string>()), changes, ctrl, false, false).SelectMany(r => r);
        Check(Board(false).Where(c => c.Changed).Select(c => c.Key).OrderBy(k => k, StringComparer.Ordinal).SequenceEqual(changes.Select(c => c.Key)), "the marked caps");
        Check(!Board(true).Any(c => c.Changed), "a Ctrl cap is marked by a plain key's change");
        Check(KeyMapModel.Mark(KeyMapModel.Caps(left, registry, none, false, false, false, Array.Empty<string>()), Array.Empty<KeyChange>(), false, false, false).SelectMany(r => r).All(c => !c.Changed), "a cap is marked with no change");
        // The key sets: default first, then the presets by name; no name takes the next, and the last wraps round.
        const string empty = "{\"format\": \"magic600-design-keymap\", \"version\": 1, \"radials\": {}, \"bindings\": []}";
        var sets = new KeySets(keys, new[] { ("left-hand", leftText), ("b-set", empty) }, registry);
        Check(sets.Names.SequenceEqual(new[] { "default", "b-set", "left-hand" }), string.Join(", ", sets.Names));
        Check(sets.Name == "default" && ReferenceEquals(sets.Keys, keys), "the key sets start at the default");
        sets.Choose(null); Equal(sets.Name, "b-set", "the next set");
        Check(KeyMapModel.Changes(keys, sets.Choose("left-hand"), registry).Select(c => c.Key).SequenceEqual(changes.Select(c => c.Key)), "a set chosen by name");
        sets.Choose(null); Check(sets.Name == "default" && ReferenceEquals(sets.Keys, keys), "the set after the last is the default");
        Equal(KeySets.Unavailable(sets.Names, null), null, "no name");
        Equal(KeySets.Unavailable(sets.Names, "left-hand"), null, "a set's name");
        Equal(KeySets.Unavailable(sets.Names, "right-hand"), "no key set is named right-hand; the key sets are default, b-set, left-hand");
        Equal(KeySets.Unavailable(null, "left-hand"), "no key sets are loaded here");
        bool refused = false;
        try { sets.Choose("right-hand"); } catch (ArgumentException) { refused = true; }
        Check(refused && sets.Name == "default", "an unknown set was chosen");
        refused = false;
        try { _ = new KeySets(keys, new[] { ("default", empty) }, registry); } catch (FormatException) { refused = true; }
        Check(refused, "a preset named default was taken");
        // The shipped folder: every preset file loads, by its file name.
        string folder = Path.Combine(Lab, "data/design/keymaps");
        Check(KeySets.Load(folder, registry, keys).Names.SequenceEqual(Directory.GetFiles(folder, "*.json").Select(p => Path.GetFileNameWithoutExtension(p)).OrderBy(n => n, StringComparer.Ordinal).Prepend("default")), "the keymaps folder's sets");
        Check(KeySets.Load(Path.Combine(Lab, "no-such-folder"), registry, keys).Names.SequenceEqual(new[] { "default" }), "a missing folder");
    }
}
