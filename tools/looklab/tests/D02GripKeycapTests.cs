using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    // ADR-D01.74/.75: keycaps and the Grip frame share one set of ordered marks. Where the chrome leaves a twist chord
    // to the live session, its cap carries the grip's twist (the chord's item and its cycle over the numbered marks);
    // the header names the grip; a frame change marks the keys it re-bound under the modifiers held.
    private static void D02GripKeycapsShareTheMarks()
    {
        CommandRegistry registry = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        registry.Relabel(Read("data/design/command-labels.json"));
        Keymap keys = Keymap.Parse(Read("data/design/keymap.json"), registry);
        string[] chords = { "KeyJ", "Shift+KeyJ", "KeyK", "Shift+KeyK", "KeyL", "Shift+KeyL" };
        string[] cycles = { "(14)(23)", "(14)(23)", "(123)", "(132)", "(134)", "(143)" };
        GripOrder Order(int frame, params ulong[] items) => new(frame,
            Enumerable.Range(1, 4).Select(i => new GripMark(i, i.ToString(), 10 * i, new double[4])).ToArray(),
            chords.Select((chord, i) => new GripKey(chord, items[i], cycles[i])).ToArray());
        string[] local = { Keymap.Global, "local-view" }, global = { Keymap.Global, "global-view" };
        IEnumerable<KeyCap> Board(GripFrame? g, IReadOnlyList<string> contexts, bool ctrl = false, bool shift = false,
            IReadOnlyDictionary<string, string>? host = null, params string[] down) =>
            KeyMapModel.Caps(keys, registry, contexts, ctrl, false, shift, down, host, null, g).SelectMany(r => r);
        KeyCap Cap(GripFrame? g, string key, IReadOnlyList<string> contexts, bool shift = false) => Board(g, contexts, shift: shift).Single(c => c.Key == key);

        // The host's chord names and the key map's.
        Equal(KeyMapModel.GripChord("J"), "KeyJ");
        Equal(KeyMapModel.GripChord("Shift+J"), "Shift+KeyJ");
        Equal(KeyMapModel.GripChord("F1"), "F1");
        Equal(KeyMapModel.GripChord("Shift+Slash"), "Shift+Slash");
        Equal(KeyMapModel.GripName("Shift+KeyL"), "Shift+L");
        Equal(KeyMapModel.GripName("KeyJ"), "J");
        Equal(KeyMapModel.GripName("Escape"), "Escape");

        // A grip that is not ready changes no cap.
        var grip = new GripFrame();
        Check(Board(grip, local).Select(c => (c.Key, c.Label, c.State)).SequenceEqual(Board(null, local).Select(c => (c.Key, c.Label, c.State))), "a grip that is not ready");
        Check(Board(grip, local).All(c => c.Grip == null), "a cap carries a grip that is not ready");
        Equal(KeyMapModel.Header("default", local, false, false, false, grip), KeyMapModel.Header("default", local, false, false, false));

        Check(grip.Set(150, 0, new[] { Order(0, 1, 1, 2, 5, 3, 9), Order(2, 1, 1, 4, 7, 6, 8) }), "set the grip");
        // J binds the draft twist row, K binds nothing, and over Local L binds nothing: each carries the grip's twist.
        foreach ((string key, ulong item, string cycle) in new[] { ("J", 1UL, "(14)(23)"), ("K", 2UL, "(123)"), ("L", 3UL, "(134)") })
        {
            KeyCap cap = Cap(grip, key, local);
            Check(cap.Grip is { } turn && turn.Chord == "Key" + key && turn.Item == item && turn.Meaning == cycle, key + "'s grip twist");
            Check(cap.State == KeyCapState.Twist && cap.Label == cycle && cap.Context == KeyMapModel.GripContext, key + "'s cap");
        }
        // Over Global the chrome takes L (Local view): its cap is the keymap's.
        KeyCap globalL = Cap(grip, "L", global);
        Check(globalL.Grip == null && globalL.State == KeyCapState.Bound && globalL.Label == registry.Commands["view.local"].Label, "L over Global");
        // Shift shows the inverse chords; Ctrl shows no grip twist.
        Check(Cap(grip, "K", local, shift: true).Grip is { Chord: "Shift+KeyK", Item: 5, Meaning: "(132)" }, "Shift+K");
        Check(Cap(grip, "J", local, shift: true).Grip is { Chord: "Shift+KeyJ", Item: 1 }, "Shift+J");
        Check(Board(grip, local, ctrl: true).All(c => c.Grip == null), "a Ctrl cap carries a grip twist");
        // A held key stays held and keeps its twist; a host key is the chrome's.
        KeyCap held = Board(grip, local, down: "J").Single(c => c.Key == "J");
        Check(held.State == KeyCapState.Held && held.Down && held.Grip?.Chord == "KeyJ", "a held twist key");
        Check(Board(grip, local, host: new Dictionary<string, string> { ["K"] = "Own" }).Single(c => c.Key == "K") is { Grip: null, Label: "Own", Context: KeyMapModel.Host }, "a host key");
        // The header names the grip.
        Check(KeyMapModel.Header("default", local, false, false, false, grip).EndsWith(" · no modifier held · grip: cap 150, frame 0", StringComparison.Ordinal), "the header names the grip");

        // The next frame re-binds K and L (both ways) and keeps J; the cycles stay, the items say the new twists.
        IReadOnlyList<GripKey> before = grip.KeyMeanings;
        Check(grip.Next() && grip.Frame == 2, "the next frame");
        IReadOnlyList<KeyChange> changes = KeyMapModel.GripChanges(before, grip.KeyMeanings);
        Check(changes.Select(c => c.Key).SequenceEqual(new[] { "K", "Shift+K", "L", "Shift+L" }), string.Join(", ", changes.Select(c => c.Key)));
        Check(changes.All(c => c.Context == KeyMapModel.GripContext), "the changes' context");
        Equal(changes[0].Before, "menu item 2, (123)");
        Equal(changes[0].After, "menu item 4, (123)");
        Check(Cap(grip, "K", local) is { Label: "(123)", Grip.Item: 4 }, "K after the next frame");
        Equal(KeyMapModel.GripChanges(grip.KeyMeanings, grip.KeyMeanings).Count, 0, "a grip against itself");
        var gone = KeyMapModel.GripChanges(before, before.Take(5).ToArray());
        Check(gone.Count == 1 && gone[0] is { Key: "Shift+L", After: null }, "a chord the frame drops");
        // The marks follow the modifiers held, as a key set change's do, and land only on caps that carry the grip.
        IEnumerable<string> Marked(IReadOnlyList<string> contexts, bool shift, IReadOnlyList<KeyChange> by) =>
            KeyMapModel.Mark(KeyMapModel.Caps(keys, registry, contexts, false, false, shift, Array.Empty<string>(), null, null, grip), by, false, false, shift)
                .SelectMany(r => r).Where(c => c.Changed).Select(c => c.Key);
        Check(Marked(local, false, changes).SequenceEqual(new[] { "K", "L" }), "the plain marks");
        Check(Marked(local, true, changes).SequenceEqual(new[] { "K", "L" }), "the Shift marks");
        Check(Marked(global, false, changes).SequenceEqual(new[] { "K" }), "over Global the chrome's L carries no grip mark");
        var both = changes.Append(new KeyChange("L", "global-view", "Local view", "Global view")).ToArray();
        Check(Marked(global, false, both).SequenceEqual(new[] { "K", "L" }), "a key set change still marks the chrome's L");
    }
}
