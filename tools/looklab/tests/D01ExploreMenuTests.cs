using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    private static void D01ColourMarkTracksStateAndEncodingOverrides()
    {
        ThemeFamily theme = Themes()[0];
        TokenSet Resolve(string? role = null) => new(Design, theme, Design.ShippedDensity, 1, overrides:
            role == null ? null : new Dictionary<string, Oklch> { [role] = new(.6, .15, 30) });
        TokenSet shipped = Resolve();
        TokenSet chrome = Resolve("surface.panel");
        Equal(ColourMark.BorderMarks(shipped, "Global").Count, 0);
        Equal(ColourMark.BorderMarks(chrome, "Local").Count, 0);

        string digest = ColourMark.Digest(shipped);
        Check(digest.Length == 12 && digest.All(Uri.IsHexDigit), "the token digest is not 12 hex digits");
        Equal(ColourMark.Digest(Resolve()), digest);
        Check(ColourMark.Digest(chrome) != digest, "a chrome colour change left the token digest unchanged");

        string encoding = Design.Roles.Values.First(role => role.Group == "encoding").Id;
        foreach (string role in new[] { "state.committed", encoding })
        {
            TokenSet tokens = Resolve(role);
            ViewMark mark = ColourMark.BorderMarks(tokens, "Global").Single();
            Equal(mark.Words, "non-standard colours");
            Check(mark.Tooltip.Contains(role, StringComparison.Ordinal), "the colour mark misses " + role);
            Check(mark.Tooltip.EndsWith("Token set " + ColourMark.Digest(tokens), StringComparison.Ordinal), "the mark misses its token digest");
        }
    }

    private static void D01ContextMenusUseRegisteredRowsInOrder()
    {
        var registry = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        registry.AddRows(Read("data/design/command-rows-d02.json"));
        string json = Read("data/design/context-menus.json");
        ContextMenuSet menus = ContextMenuSet.Parse(json, registry);
        var view = menus.View.Menu.Entries;
        var pick = menus.Pick.Menu.Entries;
        string[] selection = { "target.current.set", "card.track", "card.pin", "jumble.cap.name" };
        Equal(menus.View.SeparatorBefore, null);
        Equal(menus.Pick.SeparatorBefore, (int?)selection.Length);
        Sequence(pick.Take(selection.Length).Select(e => e.Command), selection);
        Sequence(pick.Skip(selection.Length), view, "the pick menu does not end with the view entries");
        Check(pick.All(entry => registry.Commands.ContainsKey(entry.Command)), "a context menu entry is outside the registry");
        Check(view.Any(entry => entry.Command == "help.open" && entry.Argument == HelpCatalog.Keys), "the context menu does not open the key sheet");
        Refuse(() => ContextMenuSet.Parse(json.Replace("card.track", "missing.command", StringComparison.Ordinal), registry), "missing.command");
        const string empty = """
            {"format":"magic600-context-menus/1","menus":{"view":{"label":"View","entries":[]},"pick":{"label":"Pick","entries":[]}}}
            """;
        Refuse(() => ContextMenuSet.Parse(empty, registry), "nonempty list of entries");
    }

    private static void D01ExploreOnlyAddsCommitRefusals()
    {
        CommandRegistry registry = LabRegistry();
        var explore = new ExploreState();
        const string commit = "operation.preview.commit";
        var row = registry.Commands[commit];
        Equal(explore.Gate(row), null);
        Equal(explore.BorderMarks("Global").Count, 0);
        int checks = 0;
        var unavailable = explore.Compose((id, argument) => { checks++; return argument; }, registry);
        Equal(unavailable(commit, "no preview"), "no preview");
        explore.Set(true);
        Equal(explore.Gate(row), ExploreState.CommitRefusal);
        Equal(unavailable(commit, "no preview"), ExploreState.CommitRefusal);
        Equal(checks, 1, "the inner check ran before the explore gate");
        foreach (string id in new[] { "layout.undo", "layout.redo", "command.find" })
        {
            Equal(explore.Gate(registry.Commands[id]), null);
            Equal(unavailable(id, null), null);
            Equal(unavailable(id, "another refusal"), "another refusal");
        }
        foreach (string view in new[] { "Global", "Local" })
            Equal(explore.BorderMarks(view).Single().Words, "explore: commits paused");
        explore.Set(false);
        Equal(unavailable(commit, "no preview"), "no preview");
    }

    private static void D01ExploreArgumentsSetOrToggle()
    {
        var explore = new ExploreState();
        int changes = 0;
        explore.Changed += () => changes++;
        foreach ((string? argument, bool on) in new (string?, bool)[]
            { ("on", true), ("on", true), ("off", false), ("true", true), ("false", false), (null, true), (null, false) })
        {
            Equal(explore.SetArgument(argument), null);
            Equal(explore.On, on);
        }
        foreach (string invalid in new[] { "", "yes", "toggle" })
        {
            Check(explore.SetArgument(invalid) is { Length: > 0 }, "explore accepted an invalid argument");
            Check(!explore.On, "an invalid argument changed the explore state");
        }
        Equal(changes, 6);
    }
}
