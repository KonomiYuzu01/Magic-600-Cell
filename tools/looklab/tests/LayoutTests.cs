using System;
using System.IO;
using System.Linq;
using System.Text.Json.Nodes;
using LookLab.Core;

internal static partial class Program
{
    private static void LayoutDefaults()
    {
        string[] files = Directory.GetFiles(FileAt("data/layouts"), "*.json"); Equal(files.Length, 2);
        foreach (string file in files)
        {
            LayoutSpec layout = LayoutSpec.Load(file, Commands);
            Equal(layout.Contexts.Count, Commands.Contexts.Count);
            foreach (CommandDefinition command in Commands.Commands.Values) foreach (string context in command.Contexts)
            {
                ContextPlacement placements = layout.Contexts[context];
                Check(placements.Placed.ContainsKey(command.Id) ^ placements.Hidden.Contains(command.Id), "command coverage missing or repeated");
                if (placements.Placed.ContainsKey(command.Id)) Check(layout.Resolve(command.Id, context).Contexts.Contains(context), "region unavailable");
            }
        }
        LayoutSpec central = LayoutSpec.Load(FileAt("data/layouts/central-stage.json"), Commands);
        LayoutSpec docked = LayoutSpec.Load(FileAt("data/layouts/docked-workbench.json"), Commands);
        Check(central.Regions.Values.Any(r => r.Kind == "overlay"), "central layout lacks contextual overlay instruments");
        Check(docked.Regions.Values.All(r => r.Placement == "docked") && docked.Regions.Values.Count(r => r.Kind == "panel") >= 2, "docked workbench structure missing");
    }

    private static void LayoutRefusals()
    {
        void Bad(Action<JsonNode> change, string id)
        {
            JsonNode node = JsonNode.Parse(Read("data/layouts/central-stage.json"))!; change(node); Refuse(() => LayoutSpec.Parse(node.ToJsonString(), Commands), id);
        }
        Bad(n => n["version"] = 2, "version"); Bad(n => n["format"] = "unknown", "format");
        Bad(n => n["grid"]!["columns"] = 0, "grid");
        Bad(n => n["regions"]![0]!["rect"]!["width"] = 1.1, "stage");
        Bad(n => n["regions"]![1]!["kind"] = "unknown", "instruments");
        Bad(n => n["regions"]![1]!["placement"] = "docked", "instruments");
        Bad(n => n["regions"]![1]!["contexts"] = new JsonArray("unknown"), "instruments");
        Bad(n => n["regions"]![1]!["id"] = "stage", "stage");
        Bad(n => n["contexts"]!["any"]!["placed"]!.AsObject().Remove("session.resume"), "session.resume");
        Bad(n => n["contexts"]!["any"]!["placed"]!["session.resume"] = "missing", "session.resume");
        Bad(n => n["contexts"]!["any"]!["placed"]!["unknown.command"] = "stage", "unknown.command");
        Bad(n => n["contexts"]!["any"]!["placed"]!["camera.rotate"] = "stage", "camera.rotate");
        Bad(n => n["contexts"]!["any"]!["hidden"]!.AsArray().Add("session.resume"), "any");
        Bad(n => n["contexts"]!.AsObject().Remove("editor"), "editor");
    }

    private static void FlowDefaults()
    {
        string[] files = Directory.GetFiles(FileAt("data/flows"), "*.json"); Equal(files.Length, 6);
        LayoutSpec[] layouts = Directory.GetFiles(FileAt("data/layouts"), "*.json").Select(p => LayoutSpec.Load(p, Commands)).ToArray();
        foreach (string file in files)
        {
            FlowScript flow = FlowScript.Load(file, Commands); Check(flow.Draft, "owner storyboards are still pending; defaults must remain draft");
            foreach (LayoutSpec layout in layouts)
            {
                FlowReport report = FlowMetrics.Report(flow, layout, 2560, 1600);
                Equal(report.Steps.Count, flow.Steps.Count); Check(report.TravelPixels >= 0 && report.EstimatedSeconds > 0, "invalid metrics");
                Near(report.ASeconds, .1); Near(report.BSecondsPerBit, .15);
            }
        }
    }

    private static void FlowRefusals()
    {
        void Bad(Action<JsonNode> change, string id)
        {
            JsonNode node = JsonNode.Parse(Read("data/flows/read-session.json"))!; change(node); Refuse(() => FlowScript.Parse(node.ToJsonString(), Commands), id);
        }
        Bad(n => n["format"] = "unknown", "format"); Bad(n => n["version"] = 2, "version");
        Bad(n => n["draft"] = "true", "draft"); Bad(n => n["steps"] = new JsonArray(), "steps");
        Bad(n => n["steps"]![0]!["command"] = "unknown.command", "unknown.command");
        Bad(n => n["steps"]![0]!["context"] = "puzzle", "session.resume");
        JsonNode node = JsonNode.Parse(Read("data/layouts/central-stage.json"))!;
        node["contexts"]!["any"]!["placed"]!.AsObject().Remove("session.resume");
        node["contexts"]!["any"]!["hidden"]!.AsArray().Add("session.resume");
        LayoutSpec hidden = LayoutSpec.Parse(node.ToJsonString(), Commands);
        FlowScript flow = FlowScript.Load(FileAt("data/flows/read-session.json"), Commands);
        Refuse(() => FlowMetrics.Report(flow, hidden, 100, 100), "session.resume");
        Refuse(() => FlowMetrics.Report(flow, hidden, 0, 100), "window");
    }

    private static void FlowTiny()
    {
        var catalogue = new CommandCatalogue(new[] { "any" }, new[] { new CommandDefinition("a", new[] { "any" }), new CommandDefinition("b", new[] { "any" }) });
        const string text = """
        {"format":"magic600-look-layout","version":1,"id":"tiny","grid":{"columns":5,"rows":5},"regions":[
          {"id":"left","kind":"stage","rect":{"x":0,"y":0,"width":0.2,"height":0.2},"placement":"docked","contexts":["any"]},
          {"id":"right","kind":"panel","rect":{"x":0.4,"y":0,"width":0.2,"height":0.2},"placement":"docked","contexts":["any"]}],
          "contexts":{"any":{"placed":{"a":"left","b":"right"},"hidden":[]}}}
        """;
        LayoutSpec layout = LayoutSpec.Parse(text, catalogue);
        FlowScript flow = FlowScript.Parse("""
        {"format":"magic600-look-flow","version":1,"id":"ab","name":"A to B","draft":true,"steps":[{"command":"a","context":"any"},{"command":"b","context":"any"}]}
        """, catalogue);
        FlowReport report = FlowMetrics.Report(flow, layout, 100, 100);
        Equal(report.Steps.Count, 2); Near(report.Steps[0].TravelPixels, 0); Near(report.Steps[1].TravelPixels, 40);
        Near(report.TravelPixels, 40); Near(report.Steps[1].TargetWidthPixels, 20);
        Near(report.Steps[0].EstimatedSeconds, .1); Near(report.EstimatedSeconds, .2 + .15 * Math.Log2(3));
        FlowReport scaled = FlowMetrics.Report(flow, layout, 200, 200); Near(scaled.TravelPixels, 80); Near(scaled.EstimatedSeconds, report.EstimatedSeconds);
    }

    private static void CostDefaults()
    {
        CostTable costs = CostTable.Load(FileAt("data/cost.json"), Schema);
        Sequence(costs.Features.Keys.OrderBy(x => x), Schema.Parameters.Where(p => p.CostFeature != null).Select(p => p.CostFeature!).Distinct().OrderBy(x => x));
        foreach (string feature in costs.Features.Keys) { Check(costs.Features[feature] == null, "cost table invented a measurement"); Equal(costs.Status(feature), "not measured"); }
        JsonNode node = JsonNode.Parse(Read("data/cost.json"))!;
        node["features"]!["gloss"] = new JsonObject { ["milliseconds"] = 1.25, ["source"] = "synthetic test fixture; not a product measurement" };
        CostTable measured = CostTable.Parse(node.ToJsonString(), Schema); Equal(measured.Features["gloss"]!.Milliseconds, 1.25); Equal(measured.Status("gloss"), "measured");
        Refuse(() => measured.Status("unknown"), "unknown");
    }

    private static void CostRefusals()
    {
        void Bad(Action<JsonNode> change, string id)
        {
            JsonNode node = JsonNode.Parse(Read("data/cost.json"))!; change(node); Refuse(() => CostTable.Parse(node.ToJsonString(), Schema), id);
        }
        Bad(n => n["format"] = "unknown", "format"); Bad(n => n["version"] = 2, "version");
        Bad(n => n["features"]!.AsObject().Remove("gloss"), "gloss");
        Bad(n => n["features"]!["gloss"] = 1.2, "gloss");
        Bad(n => n["features"]!["gloss"] = new JsonObject { ["milliseconds"] = -1, ["source"] = "fixture" }, "gloss");
        Bad(n => n["features"]!["gloss"] = new JsonObject { ["milliseconds"] = 1, ["source"] = "" }, "gloss");
        Bad(n => n["features"]!["gloss"] = new JsonObject { ["milliseconds"] = 1 }, "gloss");
        Bad(n => n["features"]!["gloss"] = new JsonObject { ["milliseconds"] = "1", ["source"] = "fixture" }, "gloss");
    }
}
