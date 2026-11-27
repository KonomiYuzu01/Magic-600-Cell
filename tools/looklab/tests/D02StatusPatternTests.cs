using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

internal static partial class Program
{
    private static void D02StatusPatterns()
    {
        const string path = "data/design/status-patterns.json";
        string text = Read(path);
        foreach (var theme in Themes())
        {
            var tokens = new TokenSet(Design, theme, Design.Densities[0], 1);
            var table = StatusPatterns.Parse(text, tokens);
            Equal(table.Patterns.Select(p => p.Shape).Distinct().Count(), 5);
            Equal(table.Marks.Select(m => m.Shape).Distinct().Count(), 4);
            var fixture = StatusPatterns.ReadFixture(Read("data/samples/jumble/status-fixture.json"), CorePlaces.Pieces);
            Equal(fixture.Cap, 0);
            Equal(fixture.State, "wj-ia-300");
            Check(fixture.Statuses.SelectMany(x => x).Distinct().Count() == fixture.Statuses.Sum(x => x.Length),
                "fixture patterns overlap");

            int[][] patterns = { new[] { 1, 2 }, new[] { 2 }, new[] { 3, 4 }, new[] { 1, 3 }, new[] { 2, 4, 5 } };
            int[][] marks = { new[] { 1, 3 }, new[] { 2 }, new[] { 3 }, new[] { 4 } };
            var bytes = table.Encode(7, patterns, marks);
            Check(bytes.SequenceEqual(new byte[] { 0, 9, 18, 28, 35, 5, 0 }), "status priority or mark bits differ");
            var kinds = new[] { ProtectionKinds.Orbit, ProtectionKinds.Position, ProtectionKinds.Identity, ProtectionKinds.Block };
            Check(kinds.All(kind => StatusPatterns.KindOf(StatusPatterns.MarkCode(kind)) == kind)
                  && table.Marks.Select(mark => StatusPatterns.KindOf(mark.Code)).SequenceEqual(kinds),
                "the scene's marks do not name the rail's four ProtectionKinds");
            Check(table.Encode(7, patterns.Select(_ => Array.Empty<int>()).ToArray(), marks.Select(_ => Array.Empty<int>()).ToArray())
                .All(b => b == 0), "cleared sets left a status byte");

            foreach (var bad in new[] {
                text.Replace("\"hatch\"", "\"cross_hatch\"", StringComparison.Ordinal),
                text.Replace("\"state.invalid\"", "\"not.a.token\"", StringComparison.Ordinal),
                text.Replace("\"spacing\": 6", "\"spacing\": 1", StringComparison.Ordinal),
            })
            {
                bool refused = false;
                try { StatusPatterns.Parse(bad, tokens); } catch (InvalidDataException) { refused = true; }
                Check(refused, "an invalid status table was accepted");
            }
        }
    }

    private static void D02DraftRowsWithStatusRequests()
    {
        const string request = "ADR-D01.65";
        using var rows = JsonDocument.Parse(Read("data/design/command-rows-d02.json"));
        var commands = rows.RootElement.GetProperty("commands").EnumerateArray().ToArray();
        var end = commands.TakeLast(2).ToArray();
        Equal(end[0].GetProperty("id").GetString(), "protection.pick.set");
        Equal(end[1].GetProperty("id").GetString(), "status.sample.show");
        Check(end.All(row => row.GetProperty("request").GetString() == request), "the status row request differs");
        Equal(commands.Count(row => row.TryGetProperty("request", out var value) && value.GetString() == request), 2);
        // This request spans two prefixes. Its exact two IDs are pinned above before the shared registry check.
        D01ItemRequests.Add(request, "");
        d02StatusCommandLabels = true;
        try { D02DraftRowsAbsentFromTable(); }
        finally
        {
            d02StatusCommandLabels = false;
            D01ItemRequests.Remove(request);
        }
    }
}
