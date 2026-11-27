using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;
using Magic600.Chrome;

// ADR-D02 build step B2: every slot, pattern, motion row, draft row and sentence of the record exists as data.
internal static partial class Program
{
    private static readonly string[] D02SlotStates = { "empty", "pending", "ready", "stale", "refused", "budget_stopped", "error" };

    // The roles ADR-D02 names, and the revision 6 slots it rests on.
    private static IEnumerable<string> D02Roles() =>
        new[] { "numeral.undefined", "commit.bar.whole_only", "mark.practice", "mark.jumble" }
            .Concat(D02SlotStates.Select(s => "slot.tool." + s))
            .Concat(new[] { "blocked", "closed", "at_floor", "above_floor" }.Select(s => "pattern.cap." + s))
            .Concat(new[] { "pattern.cut.inside_only", "pattern.cut.outside_only", "pattern.preview" })
            .Concat(new[] { "1", "2", "3", "4", "5", "6", "overflow" }.Select(s => "tint.jumble.contact_class." + s))
            .Concat(new[] { "2", "4", "6", "above" }.Select(s => "tint.jumble.height." + s))
            .Concat(new[] { "material.exposed_face", "material.exposed_face.tinted", "state.exposed_face.unavailable" });

    // ADR-D02.7, .21, .23, .25, .27 and revision 6's slots: each role is shared, cued, and assigned by both themes.
    private static void D02RolesExistInBothThemes()
    {
        DesignSystem system = Design; string[] roles = D02Roles().ToArray();
        Equal(roles.Length, 32, "the D02 role count");
        foreach (string role in roles)
        {
            Check(system.Roles.TryGetValue(role, out ColourRole? r), role + " is not a role");
            Check(r!.Group is "state" or "encoding" && !string.IsNullOrWhiteSpace(r.Cue), role + " is not a cued state or encoding role");
        }
        // The slot states are revision 6's seven, by name; no slot word is marked by the ring family (ADR-D02.40).
        Sequence(system.Roles.Keys.Where(k => k.StartsWith("slot.tool.", StringComparison.Ordinal)).OrderBy(k => k, StringComparer.Ordinal),
            D02SlotStates.Select(s => "slot.tool." + s).OrderBy(k => k, StringComparer.Ordinal), "the tool slot states are not revision 6's seven");
        Check(D02SlotStates.All(s => !Regex.IsMatch(system.Roles["slot.tool." + s].Cue!, @"\b(ring|circle|lamp)\b")), "a slot state is marked by the ring family");
        // The two slash hatches belong to the slice's one-sided cut cells only (ADR-D02.27).
        string[] hatched = roles.Where(r => system.Roles[r].Cue!.Contains("\"/\"", StringComparison.Ordinal) || system.Roles[r].Cue!.Contains("\"\\\"", StringComparison.Ordinal)).ToArray();
        Sequence(hatched, new[] { "pattern.cut.inside_only", "pattern.cut.outside_only" }, "a slash hatch outside the slice's one-sided states");
        foreach (string id in new[] { "graphite", "paper" })
        {
            ThemeFamily theme = ThemeFamily.Parse(Read("data/design/themes/" + id + ".json"), system);
            using JsonDocument raw = JsonDocument.Parse(Read("data/design/themes/" + id + ".json"));
            JsonElement mapped = raw.RootElement.GetProperty("roles");
            var tokens = new TokenSet(system, theme, system.ShippedDensity, 1);
            foreach (string role in roles)
            {
                Check(mapped.TryGetProperty(role, out _), id + " does not map " + role);
                Check(tokens.Colour(role) != TokenSet.MissingColour, id + " resolves no colour for " + role);
            }
            // Six contact classes and the overflow differ pairwise (ADR-D02.21).
            string[] classes = new[] { "1", "2", "3", "4", "5", "6", "overflow" }.Select(c => tokens.Colour("tint.jumble.contact_class." + c).Hex).ToArray();
            Equal(classes.Distinct().Count(), classes.Length, id + ": two contact classes share a colour");
            // The height ramp is one neutral-hue lightness ramp, in order (ADR-D02.23).
            string[] steps = new[] { "2", "4", "6", "above" }.Select(h => mapped.GetProperty("tint.jumble.height." + h).GetString()!).ToArray();
            Check(steps.All(s => s.StartsWith("neutral.", StringComparison.Ordinal)), id + ": the height ramp leaves the neutral hue");
            int[] at = steps.Select(s => int.Parse(s.Substring("neutral.".Length), System.Globalization.CultureInfo.InvariantCulture)).ToArray();
            Check(at.Zip(at.Skip(1)).All(p => p.First < p.Second), id + ": the height ramp is not in order");
        }
    }

    // ADR-D02.37: version 2 of the event-to-ID table; every row turns off with reduced motion (revision 6, J-9).
    private static void D02MotionRowsStaticUnderReducedMotion()
    {
        DesignSystem system = Design; string text = Read("data/design/motion.json"); MotionTable motions = MotionTable.Parse(text, system);
        Equal(motions.Version, 2, "the motion table version");
        var cues = new Dictionary<string, string?>
        {
            ["slice.turn"] = null, ["jumble.sweep"] = "cue.jumble_twist", ["jumble.half_turn"] = "cue.jumble_twist", ["jumble.enter"] = "cue.jumble",
            ["jumble.leave"] = "cue.jumble", ["repair.step"] = "cue.step", ["cap.name"] = "cue.cap", ["slot.fill"] = "cue.check",
        };
        foreach (var (id, cue) in cues)
        {
            Check(motions.Entries.Any(e => e.Id == id), "the motion table misses " + id);
            Equal(motions[id].Cue, cue, id + "'s cue");
        }
        Check(motions.Entries.All(e => motions.Seconds(e.Id, system, true) == 0 && e.Static.Length > 0), "reduced motion leaves a motion running");
        // The reserved row is off: a result appears in place with no motion; it is the only row off.
        Sequence(motions.Entries.Where(e => e.Off).Select(e => e.Id), new[] { "slot.fill" }, "the rows that are off");
        Check(motions.Seconds("slot.fill", system, false) == 0, "the reserved row moves");
        Check(motions.Seconds("jumble.sweep", system, false) > 0 && motions.Seconds("jumble.half_turn", system, false) > 0, "a jumble twist has no time");
        // A reserved row is a version 2 key, and only true.
        Refuse(() => MotionTable.Parse(text.Replace("\"version\": 2", "\"version\": 1"), system), "off");
        Refuse(() => MotionTable.Parse(text.Replace("\"off\": true", "\"off\": false"), system), "off");
        Refuse(() => MotionTable.Parse(text.Replace("\"version\": 2", "\"version\": 3"), system), "version");
    }

    private static readonly string[] D02Requests = { "CR-14", "CR-15", "CR-16", "CR-17", "CR-19" };
    // The research pane's own row requests (C5-7): build steps B28 (research.record.read) and B29 (declare, start).
    private static readonly string[] D02StepRequests = { "B28", "B29" };
    // ADR-D01 items ported into the shell, each with the prefix its rows carry: the cards (.24) and the setup slot (.68).
    private static readonly Dictionary<string, string> D01ItemRequests = new(StringComparer.Ordinal) { ["ADR-D01.24"] = "card.", ["ADR-D01.68"] = "setup." };

    // Each draft row names its section 15 request, research build step or cards item. The test fails the day any
    // of them lands in the command table, so the draft row is deleted.
    private static void D02DraftRowsAbsentFromTable()
    {
        string text = Read("data/design/command-rows-d02.json");
        using JsonDocument d = JsonDocument.Parse(text);
        Equal(d.RootElement.GetProperty("format").GetString(), "magic600-design-command-rows", "the draft rows format");
        var requests = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (JsonElement row in d.RootElement.GetProperty("commands").EnumerateArray())
        {
            string id = row.GetProperty("id").GetString()!;
            string? request = row.TryGetProperty("request", out JsonElement value) ? value.GetString() : null;
            Check(request != null && (D02Requests.Contains(request)
                || D02StepRequests.Contains(request) && id.StartsWith("research.", StringComparison.Ordinal)
                || D01ItemRequests.TryGetValue(request, out string? prefix) && id.StartsWith(prefix, StringComparison.Ordinal)),
                id + " names no draft request");
            requests.Add(id, request!);
        }
        Check(D02Requests.All(r => requests.ContainsValue(r)), "a request has no draft row");
        var d02 = new CommandRegistry(); d02.AddRows(text);
        Sequence(d02.Order, requests.Keys, "the draft rows");
        CommandRegistry table = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        string[] landed = d02.Order.Where(table.Commands.ContainsKey).ToArray();
        Check(landed.Length == 0, "now in the command table, delete the draft row: " + string.Join(", ", landed));
        // The shell adds them after the table and the record's own drafts: no repeated id.
        CommandRegistry registry = LabRegistry(); registry.AddRows(text);
        Check(d02.Order.All(registry.Commands.ContainsKey), "a draft row is not registered");
        // The record's named rows are draft rows (ADR-D02.9, .11, .15, .18, .33).
        foreach (string id in new[] { "view.preset.global.ring_sheet", "view.preset.global.hopf_rings", "view.preset.jumble_entry", "view.mode.raw",
            "jumble.capmap.threshold.set", "jumble.capmap.threshold.clear", "jumble.repair.stage" })
            Check(d02.Commands.ContainsKey(id), id + " is not a draft row");
        Equal(requests["jumble.repair.stage"], "CR-15", "the staging row's request");
        // No draft row commits by itself: staging prepares a preview, and the solver commits it (ADR-D02.33).
        Check(d02.Commands.Values.All(c => c.Kind != "commit"), "a draft row commits");
        Check(CommandIcons.Parse(Read("data/design/command-icons.json")).Missing(d02.Order).Count == 0, "a draft row has neither an icon nor the label-only mark");
        var palette = new PaletteModel(registry, Keymap.Parse(Read("data/design/keymap.json"), registry), (_, _) => null);
        foreach (string id in d02.Order) Check(palette.Rows(registry.Commands[id].Label).Any(r => r.Command == id), id + " is not in the palette");
    }

    // The record's text with its line wraps joined, and one item's paragraph of it.
    private static string D02Record() => Regex.Replace(File.ReadAllText(Path.Combine(Root, "docs/architecture/1.0/design/adr/ADR-D02-jumble-research.md")), @"\s*\n\s*", " ");
    private static string D02Item(string record, string item)
    {
        int start = record.IndexOf("**" + item + "**", StringComparison.Ordinal); Check(start >= 0, item + " is not in the record");
        int[] ends = new[] { record.IndexOf("**ADR-D02.", start + 5, StringComparison.Ordinal), record.IndexOf(" ### ", start, StringComparison.Ordinal) }.Where(i => i > 0).ToArray();
        return ends.Length == 0 ? record.Substring(start) : record.Substring(start, ends.Min() - start);
    }

    // Every sentence the record quotes, and the display names, are its words byte for byte.
    private static void D02WordsMatchRecord()
    {
        string record = D02Record();
        using JsonDocument d = JsonDocument.Parse(Read("data/design/jumble-words.json"));
        JsonElement root = d.RootElement;
        Equal(root.GetProperty("format").GetString(), "magic600-design-jumble-words", "the words format");
        var ids = new HashSet<string>(StringComparer.Ordinal); var texts = new List<string>();
        foreach (JsonElement s in root.GetProperty("sentences").EnumerateArray())
        {
            string id = s.GetProperty("id").GetString()!, item = s.GetProperty("item").GetString()!, text = s.GetProperty("text").GetString()!;
            Check(ids.Add(id), "repeated sentence " + id);
            Check(D02Item(record, item).Contains(text, StringComparison.Ordinal), id + " is not the words of " + item + ": " + text);
            if (s.TryGetProperty("template", out JsonElement template))
            {
                string built = template.GetString()!;
                foreach (JsonProperty p in s.GetProperty("example").EnumerateObject()) built = built.Replace("{" + p.Name + "}", p.Value.GetString(), StringComparison.Ordinal);
                Equal(built, text, id + "'s template does not rebuild its text");
            }
            texts.Add(text);
        }
        Check(ids.Count >= 40, "too few sentences");
        string names41 = D02Item(record, "ADR-D02.41");
        var names = root.GetProperty("names").EnumerateArray().Select(n => n.GetProperty("text").GetString()!).ToArray();
        foreach (string name in names) Check(names41.Contains(name, StringComparison.Ordinal), name + " is not a display name of ADR-D02.41");
        Equal(names.Length, 14, "the display names");
        foreach (JsonProperty synonym in root.GetProperty("synonyms").EnumerateObject())
        {
            Check(names.Contains(synonym.Name), synonym.Name + " is not a display name");
            foreach (JsonElement word in synonym.Value.EnumerateArray()) Check(names41.Contains(word.GetString()!, StringComparison.Ordinal), word.GetString() + " is not a synonym of ADR-D02.41");
        }
        // A word the record forbids is never shown.
        foreach (JsonElement never in root.GetProperty("never").EnumerateArray())
        {
            string word = never.GetProperty("text").GetString()!;
            Check(D02Item(record, never.GetProperty("item").GetString()!).Contains("never \"" + word + "\"", StringComparison.Ordinal), word + " is not forbidden by its item");
            Check(texts.Concat(names).All(t => !t.Contains(word, StringComparison.OrdinalIgnoreCase)), "a shown word uses " + word);
        }
        // A table row shows its display name; the table keeps the row's own label.
        using JsonDocument labels = JsonDocument.Parse(Read("data/design/command-labels.json"));
        CommandRegistry table = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        CommandRegistry shown = LabRegistry();
        foreach (JsonProperty row in root.GetProperty("rows").EnumerateObject())
        {
            Check(table.Commands.ContainsKey(row.Name) && names.Contains(row.Value.GetString() ?? ""), row.Name + " is not a table row with a display name");
            Equal(labels.RootElement.GetProperty("labels").GetProperty(row.Name).GetString(), row.Value.GetString(), row.Name + "'s label");
            Equal(shown.Commands[row.Name].Label, row.Value.GetString(), row.Name + " shown");
        }
    }
}
