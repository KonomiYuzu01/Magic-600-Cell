using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using System.Text.Json;

namespace LookLab.Core;

public sealed record CommandDefinition(string Id, IReadOnlyList<string> Contexts);
public sealed class CommandCatalogue
{
    public IReadOnlyList<string> Contexts { get; }
    public IReadOnlyDictionary<string, CommandDefinition> Commands { get; }
    public CommandCatalogue(IEnumerable<string> contexts, IEnumerable<CommandDefinition> commands)
    {
        string[] names = contexts.ToArray();
        if (names.Length == 0 || names.Any(string.IsNullOrWhiteSpace) || names.Distinct().Count() != names.Length)
            throw Json.Error("contexts", "unique nonempty contexts required");
        var table = new Dictionary<string, CommandDefinition>(StringComparer.Ordinal);
        foreach (CommandDefinition command in commands)
        {
            if (string.IsNullOrWhiteSpace(command.Id) || !table.TryAdd(command.Id, command)) throw Json.Error(command.Id, "empty or duplicate command");
            if (command.Contexts.Count == 0 || command.Contexts.Distinct().Count() != command.Contexts.Count || command.Contexts.Any(c => !names.Contains(c)))
                throw Json.Error(command.Id, "unknown or duplicate command context");
        }
        Contexts = Array.AsReadOnly(names);
        Commands = new ReadOnlyDictionary<string, CommandDefinition>(table);
    }

    public static CommandCatalogue Load(string path)
    {
        using var document = Json.Parse(File.ReadAllText(path));
        var root = document.RootElement;
        string[] contexts = Json.Array(Json.Get(Json.Get(root, "vocabularies"), "contexts"), "contexts").Select(c => Json.String(c, "contexts")).ToArray();
        var commands = Json.Array(Json.Get(root, "commands"), "commands").Select(c => new CommandDefinition(Json.Text(c, "id"),
            Array.AsReadOnly(Json.Array(Json.Get(c, "contexts"), "contexts").Select(x => Json.String(x, "contexts")).ToArray()))).ToArray();
        return new CommandCatalogue(contexts, commands);
    }
}

public readonly record struct RegionRect(double X, double Y, double Width, double Height);
public sealed record LayoutRegion(string Id, string Kind, RegionRect Rect, string Placement, IReadOnlyList<string> Contexts);
public sealed record ContextPlacement(IReadOnlyDictionary<string, string> Placed, IReadOnlyList<string> Hidden);

public sealed class LayoutSpec
{
    public string Id { get; }
    public int Columns { get; }
    public int Rows { get; }
    public IReadOnlyDictionary<string, LayoutRegion> Regions { get; }
    public IReadOnlyDictionary<string, ContextPlacement> Contexts { get; }
    private LayoutSpec(string id, int columns, int rows, Dictionary<string, LayoutRegion> regions, Dictionary<string, ContextPlacement> contexts)
    {
        Id = id; Columns = columns; Rows = rows;
        Regions = new ReadOnlyDictionary<string, LayoutRegion>(regions);
        Contexts = new ReadOnlyDictionary<string, ContextPlacement>(contexts);
    }

    public static LayoutSpec Load(string path, CommandCatalogue commands) => Parse(File.ReadAllText(path), commands);
    public static LayoutSpec Parse(string text, CommandCatalogue commands)
    {
        using var document = Json.Parse(text);
        var root = document.RootElement;
        Json.Keys(root, "format", "version", "id", "grid", "regions", "contexts");
        Json.Header(root, "magic600-look-layout");
        string id = Json.Text(root, "id");
        if (string.IsNullOrWhiteSpace(id)) throw Json.Error("id", "nonempty layout id required");
        var grid = Json.Get(root, "grid"); Json.Keys(grid, "columns", "rows");
        int columns = Json.Integer(Json.Get(grid, "columns"), "columns"), rows = Json.Integer(Json.Get(grid, "rows"), "rows");
        if (columns < 1 || rows < 1) throw Json.Error("grid", "positive dimensions required");
        var regions = new Dictionary<string, LayoutRegion>(StringComparer.Ordinal);
        foreach (JsonElement r in Json.Array(Json.Get(root, "regions"), "regions"))
        {
            Json.Keys(r, "id", "kind", "rect", "placement", "contexts");
            string rid = Json.Text(r, "id"), kind = Json.Text(r, "kind"), placement = Json.Text(r, "placement");
            if (string.IsNullOrWhiteSpace(rid) || regions.ContainsKey(rid)) throw Json.Error(rid, "empty or duplicate region id");
            if (kind != "stage" && kind != "panel" && kind != "overlay") throw Json.Error(rid, "unknown region kind");
            if ((placement != "docked" && placement != "overlay") || (kind == "overlay" && placement != "overlay")) throw Json.Error(rid, "invalid region placement");
            var rect = Json.Get(r, "rect"); Json.Keys(rect, "x", "y", "width", "height");
            var bounds = new RegionRect(Json.Number(Json.Get(rect, "x"), rid), Json.Number(Json.Get(rect, "y"), rid),
                Json.Number(Json.Get(rect, "width"), rid), Json.Number(Json.Get(rect, "height"), rid));
            if (bounds.X < 0 || bounds.Y < 0 || bounds.Width <= 0 || bounds.Height <= 0 || bounds.X + bounds.Width > 1 + 1e-12 || bounds.Y + bounds.Height > 1 + 1e-12)
                throw Json.Error(rid, "rect must fit inside fractional window bounds");
            string[] contexts = Json.Array(Json.Get(r, "contexts"), rid).Select(c => Json.String(c, rid)).ToArray();
            if (contexts.Length == 0 || contexts.Distinct().Count() != contexts.Length || contexts.Any(c => !commands.Contexts.Contains(c)))
                throw Json.Error(rid, "unknown or duplicate context");
            regions.Add(rid, new LayoutRegion(rid, kind, bounds, placement, Array.AsReadOnly(contexts)));
        }
        if (regions.Count == 0 || !regions.Values.Any(r => r.Kind == "stage")) throw Json.Error("regions", "a stage region is required");
        var result = new Dictionary<string, ContextPlacement>(StringComparer.Ordinal);
        JsonElement all = Json.Get(root, "contexts");
        Json.Keys(all, commands.Contexts.ToArray());
        foreach (string context in commands.Contexts)
        {
            var value = Json.Get(all, context); Json.Keys(value, "placed", "hidden");
            var placed = new Dictionary<string, string>(StringComparer.Ordinal);
            var p = Json.Get(value, "placed");
            if (p.ValueKind != JsonValueKind.Object) throw Json.Error(context, "placed must be an object");
            foreach (JsonProperty entry in p.EnumerateObject())
            {
                RequireCommand(commands, entry.Name, context);
                string region = Json.String(entry.Value, entry.Name);
                if (!regions.TryGetValue(region, out var r) || !r.Contexts.Contains(context)) throw Json.Error(entry.Name, "region unavailable in context " + context);
                placed.Add(entry.Name, region);
            }
            string[] hidden = Json.Array(Json.Get(value, "hidden"), context).Select(v => Json.String(v, context)).ToArray();
            foreach (string command in hidden) RequireCommand(commands, command, context);
            if (hidden.Distinct().Count() != hidden.Length || hidden.Any(placed.ContainsKey)) throw Json.Error(context, "command is duplicated or both placed and hidden");
            foreach (CommandDefinition command in commands.Commands.Values.Where(c => c.Contexts.Contains(context)))
                if (!placed.ContainsKey(command.Id) && !hidden.Contains(command.Id)) throw Json.Error(command.Id, "not placed or hidden in " + context);
            result.Add(context, new ContextPlacement(new ReadOnlyDictionary<string, string>(placed), Array.AsReadOnly(hidden)));
        }
        return new LayoutSpec(id, columns, rows, regions, result);
    }

    internal static void RequireCommand(CommandCatalogue commands, string command, string context)
    {
        if (!commands.Commands.TryGetValue(command, out var definition)) throw Json.Error(command, "unknown command");
        if (!definition.Contexts.Contains(context)) throw Json.Error(command, "unavailable in context " + context);
    }

    public LayoutRegion Resolve(string command, string context)
    {
        if (!Contexts.TryGetValue(context, out var placements) || !placements.Placed.TryGetValue(command, out string? region))
            throw Json.Error(command, "hidden or unreachable in context " + context);
        return Regions[region];
    }
}

public sealed record FlowStep(string Command, string Context);
public sealed record FlowScript(string Id, string Name, bool Draft, IReadOnlyList<FlowStep> Steps)
{
    public static FlowScript Load(string path, CommandCatalogue commands) => Parse(File.ReadAllText(path), commands);
    public static FlowScript Parse(string text, CommandCatalogue commands)
    {
        using var document = Json.Parse(text);
        var root = document.RootElement;
        Json.Keys(root, "format", "version", "id", "name", "draft", "steps");
        Json.Header(root, "magic600-look-flow");
        string id = Json.Text(root, "id"), name = Json.Text(root, "name");
        if (string.IsNullOrWhiteSpace(id) || string.IsNullOrWhiteSpace(name)) throw Json.Error("id/name", "nonempty flow id and name required");
        bool draft = Json.Boolean(Json.Get(root, "draft"), "draft");
        var steps = new List<FlowStep>();
        foreach (JsonElement s in Json.Array(Json.Get(root, "steps"), "steps"))
        {
            Json.Keys(s, "command", "context");
            string command = Json.Text(s, "command"), context = Json.Text(s, "context");
            LayoutSpec.RequireCommand(commands, command, context);
            steps.Add(new FlowStep(command, context));
        }
        if (steps.Count == 0) throw Json.Error("steps", "at least one command required");
        return new FlowScript(id, name, draft, steps.AsReadOnly());
    }
}

public sealed record FlowStepReport(string Command, string Context, string Region, double TravelPixels, double TargetWidthPixels, double EstimatedSeconds);
public sealed record FlowReport(IReadOnlyList<FlowStepReport> Steps, double TravelPixels, double EstimatedSeconds, double ASeconds, double BSecondsPerBit);

public static class FlowMetrics
{
    public const double ASeconds = 0.1, BSecondsPerBit = 0.15;
    public static FlowReport Report(FlowScript flow, LayoutSpec layout, double windowWidth, double windowHeight)
    {
        if (!double.IsFinite(windowWidth) || !double.IsFinite(windowHeight) || windowWidth <= 0 || windowHeight <= 0) throw Json.Error("window", "positive finite dimensions required");
        var result = new List<FlowStepReport>();
        double? previousX = null, previousY = null;
        foreach (FlowStep step in flow.Steps)
        {
            LayoutRegion region = layout.Resolve(step.Command, step.Context);
            RegionRect rect = region.Rect;
            double x = (rect.X + rect.Width / 2) * windowWidth, y = (rect.Y + rect.Height / 2) * windowHeight;
            double distance = previousX == null ? 0 : Math.Sqrt((x - previousX.Value) * (x - previousX.Value) + (y - previousY!.Value) * (y - previousY.Value));
            double width = Math.Min(rect.Width * windowWidth, rect.Height * windowHeight);
            double time = ASeconds + BSecondsPerBit * Math.Log2(1 + distance / width);
            result.Add(new FlowStepReport(step.Command, step.Context, region.Id, distance, width, time));
            previousX = x; previousY = y;
        }
        return new FlowReport(result.AsReadOnly(), result.Sum(s => s.TravelPixels), result.Sum(s => s.EstimatedSeconds), ASeconds, BSecondsPerBit);
    }
}

public sealed record CostMeasurement(double Milliseconds, string Source);
public sealed class CostTable
{
    public IReadOnlyDictionary<string, CostMeasurement?> Features { get; }
    private CostTable(Dictionary<string, CostMeasurement?> features) => Features = new ReadOnlyDictionary<string, CostMeasurement?>(features);
    public static CostTable Load(string path, ParameterSchema schema) => Parse(File.ReadAllText(path), schema);
    public static CostTable Parse(string text, ParameterSchema schema)
    {
        using var document = Json.Parse(text);
        var root = document.RootElement;
        Json.Keys(root, "format", "version", "features"); Json.Header(root, "magic600-look-cost");
        var features = new Dictionary<string, CostMeasurement?>(StringComparer.Ordinal);
        JsonElement values = Json.Get(root, "features");
        if (values.ValueKind != JsonValueKind.Object) throw Json.Error("features", "expected feature map");
        foreach (JsonProperty p in values.EnumerateObject())
        {
            if (string.IsNullOrWhiteSpace(p.Name)) throw Json.Error("features", "nonempty feature id required");
            CostMeasurement? measurement = null;
            if (p.Value.ValueKind != JsonValueKind.Null)
            {
                try
                {
                    Json.Keys(p.Value, "milliseconds", "source");
                    double ms = Json.Number(Json.Get(p.Value, "milliseconds"), p.Name);
                    string source = Json.Text(p.Value, "source");
                    if (ms < 0 || string.IsNullOrWhiteSpace(source)) throw Json.Error(p.Name, "nonnegative milliseconds and a source required");
                    measurement = new CostMeasurement(ms, source);
                }
                catch (FormatException ex) { throw Json.Error(p.Name, ex.Message); }
            }
            features.Add(p.Name, measurement);
        }
        foreach (string feature in schema.Parameters.Where(p => p.CostFeature != null).Select(p => p.CostFeature!).Distinct())
            if (!features.ContainsKey(feature)) throw Json.Error(feature, "missing cost feature");
        return new CostTable(features);
    }
    public string Status(string feature) => Features.TryGetValue(feature, out CostMeasurement? value)
        ? value == null ? "not measured" : "measured" : throw Json.Error(feature, "unknown cost feature");
}
