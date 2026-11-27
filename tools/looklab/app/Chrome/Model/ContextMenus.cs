using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace Magic600.Chrome;

public sealed record ContextMenu(RadialMenu Menu, int? SeparatorBefore);

// ADR-D01.17: a selection adds its own commands ahead of the view's commands.
public sealed class ContextMenuSet
{
    public ContextMenu View { get; }
    public ContextMenu Pick { get; }

    private ContextMenuSet(RadialMenu view, RadialMenu pick)
    {
        View = new(view, null);
        Pick = new(pick with { Entries = Array.AsReadOnly(pick.Entries.Concat(view.Entries).ToArray()) }, pick.Entries.Count);
    }

    public static ContextMenuSet Parse(string json, CommandRegistry registry)
    {
        using JsonDocument document = J.Parse(json);
        JsonElement root = document.RootElement;
        J.Keys(root, "context menus", "format", "menus");
        if (J.Text(root, "format") != "magic600-context-menus/1") throw J.Error("format", "expected magic600-context-menus/1");
        JsonElement menus = root.GetProperty("menus");
        J.Keys(menus, "menus", "view", "pick");
        return new ContextMenuSet(ReadMenu("view"), ReadMenu("pick"));

        RadialMenu ReadMenu(string target)
        {
            JsonElement menu = menus.GetProperty(target);
            J.Keys(menu, target, "label", "entries");
            JsonElement rows = menu.GetProperty("entries");
            if (rows.ValueKind != JsonValueKind.Array || rows.GetArrayLength() == 0) throw J.Error(target, "expected a nonempty list of entries");
            var entries = new List<RadialEntry>();
            foreach (JsonElement row in rows.EnumerateArray())
            {
                foreach (JsonProperty property in J.Map(row, target))
                    if (property.Name is not ("command" or "argument" or "label")) throw J.Error(target, "unknown key " + property.Name);
                if (!row.TryGetProperty("command", out JsonElement id)) throw J.Error(target, "missing command");
                string command = J.String(id, target);
                if (!registry.Commands.ContainsKey(command)) throw J.Error(target, "unknown command " + command);
                string? argument = row.TryGetProperty("argument", out JsonElement a) ? J.String(a, target) : null;
                string? label = row.TryGetProperty("label", out JsonElement l) ? J.String(l, target) : null;
                entries.Add(new RadialEntry(command, argument, label));
            }
            return new RadialMenu(target, J.Text(menu, "label"), entries.AsReadOnly());
        }
    }
}
