using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Magic600.Chrome;

public enum HelpTopicKind { Term, Command }
public enum HelpFigureKind { Polytope, Cell, Grip }

public sealed record HelpTopic(HelpTopicKind Kind, string Id, string Title, string Text, string? Level, IReadOnlyList<string> LinkedTermIds);
public sealed record HelpLevel(string Id, string Name, IReadOnlyList<HelpTopic> Terms);
public sealed record HelpReading(HelpTopic Topic, string Body, bool Cut, HelpFigure? Figure)
{
    public string FigureLine => Figure?.Caption ?? (Topic.Kind == HelpTopicKind.Term
        ? "No geometry figure for this term yet." : "No geometry figure for this command yet.");
}
public sealed record HelpLearnLink(string Command, string Argument, string TopicId);
public sealed record HelpCardModel(HelpReading Reading, HelpLearnLink Learn)
{
    public const string LearnLabel = "Open in Learn";
}

public sealed class HelpCatalog
{
    public const string Keys = "keys";
    public const int CardWords = 50, LearnWords = 80;
    private static readonly Regex Links = new(@"\[\[([^\]|]+)(?:\|([^\]]+))?\]\]");
    private readonly Dictionary<string, HelpTopic> terms = new(StringComparer.Ordinal);
    private readonly Dictionary<string, string> names = new(StringComparer.OrdinalIgnoreCase);
    private readonly Dictionary<string, string> aliases = new(StringComparer.OrdinalIgnoreCase);
    private readonly Dictionary<string, HelpFigureKind> figures = new(StringComparer.Ordinal);
    private readonly CommandRegistry commands;
    public IReadOnlyList<HelpLevel> Levels { get; }

    public static HelpCatalog Load(string glossaryPath, string figuresPath, CommandRegistry commands) =>
        new(File.ReadAllText(glossaryPath), File.ReadAllText(figuresPath), commands);

    public HelpCatalog(string glossaryText, string figuresText, CommandRegistry commands)
    {
        this.commands = commands;
        using JsonDocument glossary = J.Parse(glossaryText);
        foreach (JsonElement row in glossary.RootElement.GetProperty("terms").EnumerateArray())
        {
            string id = J.Text(row, "id"), title = J.Text(row, "term");
            terms.Add(id, new HelpTopic(HelpTopicKind.Term, id, title, J.Text(row, "definition"), J.Text(row, "level"), Array.Empty<string>()));
            names.Add(title, id);
            if (row.TryGetProperty("aliases", out JsonElement words))
                foreach (JsonElement alias in words.EnumerateArray()) aliases.Add(J.String(alias, id), id);
        }
        foreach (HelpTopic term in terms.Values.ToArray())
        {
            var linked = new List<string>();
            string plain = Links.Replace(term.Text, match =>
            {
                string shown = match.Groups[1].Value;
                string target = (match.Groups[2].Success ? match.Groups[2].Value : shown).Trim();
                // The glossary uses [[word|~]] for ordinary words that share a term's spelling.
                if (target != "~") linked.Add(NamedId(target) ?? throw J.Error(term.Id, "unknown glossary link " + target));
                return shown;
            });
            terms[term.Id] = term with { Text = plain.Replace("`", "", StringComparison.Ordinal), LinkedTermIds = linked.AsReadOnly() };
        }
        Levels = Array.AsReadOnly(glossary.RootElement.GetProperty("levels").EnumerateArray().Select(row =>
        {
            string id = J.Text(row, "id");
            return new HelpLevel(id, J.Text(row, "name"), Array.AsReadOnly(terms.Values.Where(term => term.Level == id).ToArray()));
        }).ToArray());
        using JsonDocument map = J.Parse(figuresText);
        JsonElement root = J.Root(map, "magic600-design-help-figures", "note", "figures");
        foreach (JsonProperty entry in J.Map(root.GetProperty("figures"), "figures"))
        {
            if (!terms.ContainsKey(entry.Name)) throw J.Error(entry.Name, "a figure needs a glossary term id");
            figures.Add(entry.Name, J.String(entry.Value, entry.Name) switch
            {
                "polytope" => HelpFigureKind.Polytope,
                "cell" => HelpFigureKind.Cell,
                "grip" => HelpFigureKind.Grip,
                _ => throw J.Error(entry.Name, "unknown help figure kind"),
            });
        }
    }

    private string? NamedId(string name) => names.GetValueOrDefault(name) ?? aliases.GetValueOrDefault(name);

    private HelpTopic? Command(string id) => commands.Commands.TryGetValue(id, out ChromeCommand? command)
        ? new HelpTopic(HelpTopicKind.Command, id, command.Label, command.Help, null, Array.Empty<string>()) : null;

    public HelpTopic? Resolve(string topic) => topic == Keys ? null
        : terms.GetValueOrDefault(topic) ?? (NamedId(topic) is { } id ? terms[id] : Command(topic));

    internal HelpTopic? ById(string id) => id == Keys ? null : terms.GetValueOrDefault(id) ?? Command(id);

    public static string? UnderCursor(IEnumerable<string?> nearestFirst) => nearestFirst.FirstOrDefault(topic => !string.IsNullOrWhiteSpace(topic));

    public HelpCardModel Card(HelpTopic topic) => new(Reading(topic, CardWords, HelpCardModel.LearnLabel),
        new HelpLearnLink("workspace.set", "layout:reference", topic.Id));

    internal HelpReading Reading(HelpTopic topic, int budget, string otherText)
    {
        HelpFigure? figure = topic.Kind == HelpTopicKind.Term && figures.TryGetValue(topic.Id, out HelpFigureKind kind)
            ? HelpGeometry.Exact.Figure(kind) : null;
        var reading = new HelpReading(topic, topic.Text, false, figure);
        int bodyBudget = budget - HelpText.Words(topic.Title) - HelpText.Words(otherText)
            - HelpText.Words(reading.FigureLine) - HelpText.Words(figure?.Basis ?? "");
        (string body, bool cut) = HelpText.Fit(topic.Text, bodyBudget);
        return reading with { Body = body, Cut = cut };
    }
}

public sealed class LearnModel(HelpCatalog catalog)
{
    public const string ChooseTerm = "Choose a term.";
    public IReadOnlyList<HelpLevel> Levels => catalog.Levels;
    public HelpReading? Selected { get; private set; }
    public string LevelName { get; private set; } = "";
    public HelpFigure? Figure => Selected == null ? HelpGeometry.Exact.Figure(HelpFigureKind.Polytope) : Selected.Figure;

    public bool Select(string id)
    {
        if (catalog.ById(id) is not { } topic) return false;
        LevelName = Levels.FirstOrDefault(level => level.Id == topic.Level)?.Name ?? "";
        Selected = catalog.Reading(topic, HelpCatalog.LearnWords, LevelName);
        return true;
    }
}

internal static class HelpText
{
    private static readonly Regex Word = new(@"\S+");
    private static readonly Regex SentenceEnd = new(@"[.!?][""”’')\]]*(?=\s|$)");

    internal static int Words(string text) => Word.Matches(text).Count;

    internal static (string Text, bool Cut) Fit(string text, int limit)
    {
        MatchCollection words = Word.Matches(text);
        if (words.Count <= limit) return (text, false);
        if (limit <= 0) return ("", true);
        Match last = words[limit - 1];
        string prefix = text[..(last.Index + last.Length)];
        MatchCollection endings = SentenceEnd.Matches(prefix);
        if (endings.Count == 0) return (prefix.TrimEnd(',', ';', ':') + "…", true);
        Match end = endings[^1];
        return (prefix[..(end.Index + end.Length)], true);
    }
}

public readonly record struct HelpPosition(double X, double Y);
public readonly record struct HelpSegment(HelpPosition A, HelpPosition B, bool Emphasised);
public readonly record struct HelpPoint(HelpPosition Position, bool Emphasised);
public sealed record HelpFigure(HelpFigureKind Kind, IReadOnlyList<HelpSegment> Segments, IReadOnlyList<HelpPoint> Points, string Caption, string Basis);

// A separate construction for the explanatory plates, of the unit 600-cell in the Euclidean 4-space.
internal sealed class HelpGeometry
{
    private static readonly double Phi = (1 + Math.Sqrt(5)) / 2;
    internal static HelpGeometry Exact { get; } = new();
    internal IReadOnlyList<double[]> Vertices { get; }
    internal IReadOnlyList<(int A, int B)> Edges { get; }
    internal IReadOnlyList<int[]> Cells { get; }
    internal IReadOnlyList<List<int>> Neighbours { get; }
    private readonly Dictionary<HelpFigureKind, HelpFigure> figures;

    private HelpGeometry()
    {
        Vertices = UnitVertices().ToArray();
        var adjacent = new bool[Vertices.Count, Vertices.Count];
        var edges = new List<(int, int)>();
        double lengthSquared = 1 / (Phi * Phi);
        for (int a = 0; a < Vertices.Count; a++)
            for (int b = a + 1; b < Vertices.Count; b++)
            {
                double square = Vertices[a].Zip(Vertices[b], (p, q) => (p - q) * (p - q)).Sum();
                if (Math.Abs(square - lengthSquared) > 1e-12) continue;
                adjacent[a, b] = true;
                edges.Add((a, b));
            }
        Edges = edges.AsReadOnly();

        var cells = new List<int[]>();
        foreach (var (a, b) in Edges)
            for (int c = b + 1; c < Vertices.Count; c++)
            {
                if (!adjacent[a, c] || !adjacent[b, c]) continue;
                for (int d = c + 1; d < Vertices.Count; d++)
                    if (adjacent[a, d] && adjacent[b, d] && adjacent[c, d]) cells.Add(new[] { a, b, c, d });
            }
        Cells = cells.AsReadOnly();

        var neighbours = cells.Select(_ => new List<int>()).ToArray();
        var faces = new Dictionary<(int, int, int), int>();
        for (int i = 0; i < cells.Count; i++)
        {
            int[] cell = cells[i];
            int a = cell[0], b = cell[1], c = cell[2], d = cell[3];
            foreach (var face in new[] { (a, b, c), (a, b, d), (a, c, d), (b, c, d) })
            {
                if (faces.Remove(face, out int other))
                {
                    neighbours[i].Add(other);
                    neighbours[other].Add(i);
                }
                else faces.Add(face, i);
            }
        }
        Neighbours = neighbours;
        figures = Enum.GetValues<HelpFigureKind>().ToDictionary(kind => kind, Build);
    }

    private static IEnumerable<double[]> UnitVertices()
    {
        for (int axis = 0; axis < 4; axis++)
            foreach (int sign in new[] { -1, 1 })
            {
                var point = new double[4];
                point[axis] = sign;
                yield return point;
            }

        static double Signed(double value, int mask, int axis) => (mask & (1 << axis)) == 0 ? value : -value;
        for (int signs = 0; signs < 16; signs++)
            yield return Enumerable.Range(0, 4).Select(axis => Signed(.5, signs, axis)).ToArray();

        double[] magnitudes = { Phi / 2, .5, 1 / (2 * Phi), 0 };
        for (int a = 0; a < 4; a++)
            for (int b = 0; b < 4; b++)
                for (int c = 0; c < 4; c++)
                {
                    if (a == b || a == c || b == c) continue;
                    int[] order = { a, b, c, 6 - a - b - c };
                    int inversions = 0;
                    for (int i = 0; i < order.Length; i++)
                        for (int j = i + 1; j < order.Length; j++)
                            if (order[i] > order[j]) inversions++;
                    if (inversions % 2 != 0) continue;
                    for (int signs = 0; signs < 8; signs++)
                        yield return order.Select(axis => Signed(magnitudes[axis], signs, axis)).ToArray();
                }
    }

    internal double[] Pole(int cell)
    {
        double[] centroid = Enumerable.Range(0, 4).Select(axis => Cells[cell].Average(v => Vertices[v][axis])).ToArray();
        double radius = Math.Sqrt(centroid.Sum(x => x * x));
        return centroid.Select(x => x / radius).ToArray();
    }

    private static HelpPosition Project(double[] point)
    {
        // Fixed rotations in coordinate planes mix all four coordinates before the orthographic projection.
        static void Rotate(ref double u, ref double v, double angle) =>
            (u, v) = (Math.Cos(angle) * u - Math.Sin(angle) * v, Math.Sin(angle) * u + Math.Cos(angle) * v);
        double x = point[0], y = point[1], z = point[2], w = point[3];
        Rotate(ref x, ref z, .31); Rotate(ref y, ref w, .61);
        Rotate(ref x, ref w, .43); Rotate(ref y, ref z, .27);
        Rotate(ref x, ref y, .36);
        return new HelpPosition(x, y);
    }

    internal HelpFigure Figure(HelpFigureKind kind) => figures[kind];

    private HelpFigure Build(HelpFigureKind kind)
    {
        const int cell = 0, pole = -1;
        int[] main = Cells[cell];
        bool CellEdge((int A, int B) edge, int[] vertices) => vertices.Contains(edge.A) && vertices.Contains(edge.B);
        var edges = kind switch
        {
            HelpFigureKind.Cell => Edges.Where(edge => CellEdge(edge, main) || Neighbours[cell].Any(n => CellEdge(edge, Cells[n]))),
            HelpFigureKind.Grip => Edges.Where(edge => CellEdge(edge, main)),
            _ => Edges,
        };
        var drawn = edges.ToArray();
        var positions = drawn.SelectMany(edge => new[] { edge.A, edge.B }).Distinct().ToDictionary(v => v, v => Project(Vertices[v]));
        if (kind == HelpFigureKind.Grip) positions.Add(pole, Project(Pole(cell)));
        double left = positions.Values.Min(p => p.X), right = positions.Values.Max(p => p.X);
        double top = positions.Values.Min(p => p.Y), bottom = positions.Values.Max(p => p.Y);
        double span = Math.Max(right - left, bottom - top);
        HelpPosition Place(HelpPosition p) => new(.5 + (p.X - (left + right) / 2) / span, .5 + (p.Y - (top + bottom) / 2) / span);

        HelpSegment[] segments = drawn.Select(edge => new HelpSegment(Place(positions[edge.A]), Place(positions[edge.B]),
            kind == HelpFigureKind.Cell && CellEdge(edge, main))).ToArray();
        HelpPoint[] points = positions.Select(p => new HelpPoint(Place(p.Value), p.Key == pole || (kind == HelpFigureKind.Cell && main.Contains(p.Key)))).ToArray();
        string caption = kind switch
        {
            HelpFigureKind.Cell => "One of the 600 cells (bold) and its four neighbours.",
            HelpFigureKind.Grip => "A grip: the pole of one cell.",
            _ => "The 600-cell: 120 vertices, 720 edges.",
        };
        return new HelpFigure(kind, Array.AsReadOnly(segments), Array.AsReadOnly(points), caption, "Orthographic projection of the 600-cell's exact vertices");
    }
}
