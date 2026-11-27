using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

internal static partial class Program
{
    private static HelpCatalog D02HelpGlossary(out CommandRegistry commands)
    {
        commands = CommandRegistry.FromTable(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/command-table.json")));
        return HelpCatalog.Load(Path.Combine(Root, "docs/progress/1.0/glossary/terms.json"), FileAt("data/design/help-figures.json"), commands);
    }

    private static void D02HelpResolvesGlossaryAndCommands()
    {
        var help = D02HelpGlossary(out var commands);
        HelpTopic cap = help.Resolve("cap") ?? throw new Exception("missing Cap");
        Equal(cap, help.Resolve("CAP"));
        Equal(cap.Kind, HelpTopicKind.Term); Equal(cap.Title, "Cap"); Equal(cap.Level, "L1");
        Equal(cap.Text, "A world-fixed grip and cut; which pieces lie inside it depends on the state.");
        Sequence(cap.LinkedTermIds, new[] { "grip", "piece" });
        Equal(help.Resolve("PRIMITIVE MOVE")?.Id, "generator");
        Check(help.Resolve("residual")!.Text.EndsWith("covers.", StringComparison.Ordinal), "an ordinary glossary word retained its markup");
        Check(help.Resolve("census-rank")!.Text.Contains("orbit field rank in the census", StringComparison.Ordinal), "inline code retained its markup");
        Check(help.Resolve("help.open") is { Kind: HelpTopicKind.Command, Title: "Read contextual help", Level: null }, "command help did not resolve");
        Equal(help.Resolve("theme.select")?.Text, commands.Commands["theme.select"].Help);
        Check(help.Resolve("Read contextual help") == null && help.Resolve("unknown-topic") == null, "an unknown topic resolved");
        Check(help.Levels.SelectMany(level => level.Terms).All(term => term.Id != HelpCatalog.Keys) &&
            !commands.Commands.ContainsKey(HelpCatalog.Keys) && help.Resolve(HelpCatalog.Keys) == null, "keys must stay reserved for the shortcut sheet");

        const string glossary = """
            {"levels":[{"id":"L1","name":"First"}],"terms":[
                {"id":"a","term":"Alpha","level":"L1","definition":"See [[start|alternative]] and [[ALTERNATIVE]]."},
                {"id":"b","term":"Beta","level":"L1","definition":"A term.","aliases":["Alternative"]}
            ]}
            """;
        const string map = """{"format":"magic600-design-help-figures","version":1,"note":"","figures":{}}""";
        var aliasLinks = new HelpCatalog(glossary, map, commands).Resolve("a")!;
        Equal(aliasLinks.Text, "See start and ALTERNATIVE.");
        Sequence(aliasLinks.LinkedTermIds, new[] { "b", "b" });
    }

    private static void D02HelpDrawsTheUnit600Cell()
    {
        var geometry = HelpGeometry.Exact;
        Equal(geometry.Vertices.Count, 120); Equal(geometry.Edges.Count, 720); Equal(geometry.Cells.Count, 600);
        Equal(geometry.Vertices.Select(p => (p[0], p[1], p[2], p[3])).Distinct().Count(), 120);
        foreach (double[] point in geometry.Vertices) Near(point.Sum(x => x * x), 1);
        var edges = geometry.Edges.ToHashSet();
        Equal(edges.Count, 720);
        double edgeLength = 2 / (1 + Math.Sqrt(5));
        foreach (var (a, b) in edges)
            Near(geometry.Vertices[a].Zip(geometry.Vertices[b], (u, v) => (u - v) * (u - v)).Sum(), edgeLength * edgeLength);
        for (int i = 0; i < geometry.Cells.Count; i++)
        {
            int[] cell = geometry.Cells[i];
            Check(cell.Length == 4 && cell.SelectMany((a, j) => cell.Skip(j + 1).Select(b => (a, b))).All(edges.Contains), "a cell is not a tetrahedron");
            Equal(geometry.Neighbours[i].Distinct().Count(), 4);
            foreach (int other in geometry.Neighbours[i])
                Check(cell.Intersect(geometry.Cells[other]).Count() == 3 && geometry.Neighbours[other].Contains(i), "a neighbour does not share a face");
        }
        double[] pole = geometry.Pole(0);
        Near(pole.Sum(x => x * x), 1);
        double[] centre = Enumerable.Range(0, 4).Select(axis => geometry.Cells[0].Average(v => geometry.Vertices[v][axis])).ToArray();
        Near(pole.Zip(centre, (a, b) => a * b).Sum(), Math.Sqrt(centre.Sum(x => x * x)));

        HelpFigure polytope = geometry.Figure(HelpFigureKind.Polytope), cellPlate = geometry.Figure(HelpFigureKind.Cell), grip = geometry.Figure(HelpFigureKind.Grip);
        Equal(polytope.Segments.Count, 720); Equal(polytope.Points.Count, 120);
        Equal(cellPlate.Segments.Count, 18); Equal(cellPlate.Segments.Count(segment => segment.Emphasised), 6);
        Equal(grip.Segments.Count, 6); Equal(grip.Points.Count, 5); Equal(grip.Points.Count(point => point.Emphasised), 1);
        foreach (HelpFigure figure in new[] { polytope, cellPlate, grip })
        {
            var points = figure.Points.Select(point => point.Position).ToHashSet();
            Equal(points.Count, figure.Points.Count);
            Check(points.All(point => point.X >= -1e-12 && point.X <= 1 + 1e-12 && point.Y >= -1e-12 && point.Y <= 1 + 1e-12), "point outside the unit box");
            Check(figure.Segments.All(s => points.Contains(s.A) && points.Contains(s.B) && s.A != s.B), "invalid projected edge");
            Check(figure.Basis.StartsWith("Orthographic projection", StringComparison.Ordinal), "figure lacks its drawing basis");
        }
    }

    private static void D02HelpFigureMapUsesKnownTermsAndKinds()
    {
        var help = D02HelpGlossary(out var commands);
        string map = Read("data/design/help-figures.json");
        using var document = JsonDocument.Parse(map);
        var figures = document.RootElement.GetProperty("figures").EnumerateObject().ToArray();
        Sequence(figures.Select(pair => pair.Name), new[] { "cell", "grip", "cap", "twist-full" });
        foreach (var pair in figures)
        {
            Check(help.Resolve(pair.Name) is { Kind: HelpTopicKind.Term } term && term.Id == pair.Name, "a figure key is not a glossary id");
            var kind = help.Card(help.Resolve(pair.Name)!).Reading.Figure?.Kind;
            Equal(kind, pair.Name == "cell" ? HelpFigureKind.Cell : HelpFigureKind.Grip);
        }
        string glossary = File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/glossary/terms.json"));
        Refuse(() => new HelpCatalog(glossary, map.Replace("\"cap\":", "\"missing-term\":"), commands), "missing-term");
        Refuse(() => new HelpCatalog(glossary, map.Replace("\"cap\": \"grip\"", "\"cap\": \"schematic\""), commands), "unknown help figure kind");
    }

    private static void D02HelpCardAndLearnFitTheirBudgets()
    {
        var help = D02HelpGlossary(out var commands);
        var learn = new LearnModel(help);
        Check(learn.Selected == null && learn.Figure is { Kind: HelpFigureKind.Polytope, Caption: "The 600-cell: 120 vertices, 720 edges." }, "Learn opens with the full 600-cell");
        Equal(LearnModel.ChooseTerm, "Choose a term.");
        using var glossary = JsonDocument.Parse(File.ReadAllText(Path.Combine(Root, "docs/progress/1.0/glossary/terms.json")));
        Sequence(learn.Levels.Select(level => (level.Id, level.Name)), glossary.RootElement.GetProperty("levels").EnumerateArray()
            .Select(level => (level.GetProperty("id").GetString()!, level.GetProperty("name").GetString()!)));
        Equal(learn.Levels.Sum(level => level.Terms.Count), 298);
        foreach (HelpLevel level in learn.Levels)
            Check(level.Terms.All(term => term.Level == level.Id), "a term is shown under the wrong level");

        HelpCardModel cell = help.Card(help.Resolve("cell")!);
        Equal(cell.Learn, new HelpLearnLink("workspace.set", "layout:reference", "cell"));
        Equal(cell.Reading.FigureLine, "One of the 600 cells (bold) and its four neighbours.");
        Equal(help.Card(help.Resolve("cap")!).Reading.FigureLine, "A grip: the pole of one cell.");
        Equal(help.Card(help.Resolve("orbit")!).Reading.FigureLine, "No geometry figure for this term yet.");
        Check(learn.Select(cell.Learn.TopicId) && learn.Selected?.Topic.Title == "Cell" && learn.LevelName == "Turning the puzzle", "the Learn link does not select Cell");
        Sequence(learn.Selected!.Topic.LinkedTermIds, new[] { "sticker-slot" });
        Check(!learn.Select("unknown-topic") && learn.Selected?.Topic.Id == "cell", "an unknown selection changed Learn");
        Check(learn.Select("help.open") && learn.Selected?.Topic.Kind == HelpTopicKind.Command, "command help cannot link to Learn");

        static int Words(HelpReading reading, string label) => new[] { reading.Topic.Title, reading.Body, reading.FigureLine, reading.Figure?.Basis ?? "", label }
            .Sum(text => text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).Length);
        foreach (var topic in help.Levels.SelectMany(level => level.Terms).Concat(commands.Order.Select(id => help.Resolve(id)!)))
        {
            Check(Words(help.Card(topic).Reading, HelpCardModel.LearnLabel) <= 50, "card budget exceeded for " + topic.Id);
            Check(learn.Select(topic.Id) && Words(learn.Selected!, learn.LevelName) <= 80, "Learn budget exceeded for " + topic.Id);
        }
        Equal(HelpText.Fit("First sentence. Second one! Here is the last sentence.", 6), ("First sentence. Second one!", true));
        Equal(HelpText.Fit("One two three four five.", 3), ("One two three…", true));
        Equal(HelpText.Fit("One full sentence.", 3), ("One full sentence.", false));
    }

    private static void D02HelpUsesNearestTopicUnderCursor()
    {
        Equal(HelpCatalog.UnderCursor(new[] { "cap", "relationship-strip", "orbit" }), "cap");
        Equal(HelpCatalog.UnderCursor(new[] { null, "", " ", "orbit", "relationship-strip" }), "orbit");
        Check(HelpCatalog.UnderCursor(new[] { null, "" }) == null && HelpCatalog.UnderCursor(Array.Empty<string>()) == null, "an unmarked control has a topic");
    }
}
