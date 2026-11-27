using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using Godot;
using LookLab.App.Live;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

internal static class ShellDocuments
{
    private const string Waiting = "waits for LP4's document services and K2's wiring (ADR-D01.84)";
    private const string SampleModel = "600-cell-Full sample";
    private static readonly HashSet<string> Rows = new(StringComparer.Ordinal);
    private static readonly Dictionary<string, DocumentBody> Bodies = new(StringComparer.Ordinal);
    private static DocumentSet? documents;
    private static DocumentRow? row;
    private static bool sampleOn;
    private static string? shownPane;

    public static void Load(ChromeFrame frame, CommandRegistry registry, SampleMount? sample)
    {
        Rows.Clear();
        foreach (string id in registry.Order)
            if (DocumentSet.Handles(id)) Rows.Add(id);
        Bodies.Clear();
        shownPane = null;
        sampleOn = sample != null;

        OrbitRail? rail = sample?.Solve.Rail;
        SolvingDemo? demo = sample?.Solve.Demo;
        IReadOnlyList<SolvingState> states = demo?.States ?? Array.Empty<SolvingState>();
        var store = new MemoryDocumentStore();
        if (demo != null)
        {
            string folder = ProjectSettings.GlobalizePath("res://samples");
            var scripts = new Dictionary<string, string>
            {
                ["census.6cs"] = File.ReadAllText(Path.Combine(folder, "census.6cs")),
                ["sweep.6cs"] = File.ReadAllText(Path.Combine(folder, "sweep.6cs")),
            };
            store = SampleDocuments.Store(SampleModel, states, scripts);
        }
        var set = documents = new DocumentSet(store, new SampleDocumentCompiler(SampleModel, states));
        if (demo != null)
        {
            set.Open(SampleDocuments.MidLogPath);
            set.Open(SampleDocuments.EndgameLogPath);
            set.Open(SampleDocuments.BadLogPath);
            set.Open("census.6cs");
            set.Open("sweep.6cs");
            set.Activate("session");
        }

        var tabs = row = new DocumentRow(set, () => frame.Tokens, () => demo?.State, () => rail?.ByOrbitId ?? false);
        tabs.Activated += id => { if (set.Activate(id)) ShowActive(frame, set); };
        frame.SetDocumentRow(tabs);
        frame.Rebuilt += tabs.Refresh;
        frame.Recoloured += tabs.Refresh;
        if (demo != null) demo.Changed += tabs.Refresh;
        if (rail != null) rail.OrderChanged += tabs.Refresh;
    }

    public static bool Handles(string command) => Rows.Contains(command);

    internal static bool Recording => documents is { } set && set.Documents[0].Signature(null, false).Recording;

    public static string? Unavailable(string command, string? argument) =>
        sampleOn ? documents?.Unavailable(command, argument) : Waiting;

    public static void Execute(ChromeFrame frame, string command, string? argument)
    {
        if (documents is not { } set) return;
        string line = set.Execute(command, argument);
        ShowActive(frame, set);
        frame.Say(line);
    }

    private static void ShowActive(ChromeFrame frame, DocumentSet set)
    {
        Document document = set.Active;
        string? pane = document.Kind == DocumentKind.Session ? null : "document:" + document.Id;
        if (shownPane is { } old && old != pane && frame.Model.IsOpen(old)) frame.Model.Close(old);
        shownPane = pane;
        if (pane == null)
        {
            frame.Model.Activate(FrameModel.Pair);
            return;
        }
        if (!Bodies.TryGetValue(pane, out DocumentBody? body))
        {
            body = new DocumentBody();
            Bodies.Add(pane, body);
        }
        body.Show(document, set.Problems());
        // A verified log opens as a free view bound to its saved state (ADR-D01.42, rev5); a log whose compile fails
        // carries no chip (rev6), and a script is an editor pane, no view of a state.
        frame.AddPane(pane, document.Name, body, ViewBinding.Of(document.Binding));
        if (!frame.Model.IsOpen(pane)) frame.Model.Drop(pane, FrameModel.Pair, DropZone.Right);
        else frame.Model.Activate(pane);
    }

    public static void Dump(Utf8JsonWriter json)
    {
        json.WriteStartObject("documents");
        json.WriteBoolean("row", row != null && GodotObject.IsInstanceValid(row) && row.IsInsideTree());
        if (documents is { } set && row is { } tabs)
        {
            json.WriteString("active", set.Active.Id);
            json.WriteStartArray("tabs");
            foreach (Document document in set.Documents)
            {
                DocumentSignature signature = tabs.Signature(document);
                json.WriteStartObject();
                json.WriteString("id", document.Id);
                json.WriteString("kind", document.Kind.ToString());
                json.WriteString("name", document.Name);
                json.WriteBoolean("dirty", document.Dirty);
                json.WriteBoolean("pinned", document.Pinned);
                json.WriteString("lamp", signature.Lamp?.ToString());
                json.WriteNumber("ticks", signature.Ticks?.Count ?? 0);
                json.WriteString("binding", document.Binding);
                json.WriteBoolean("readOnly", document.ReadOnly);
                json.WriteBoolean("recording", signature.Recording);
                json.WriteBoolean("diskChanged", document.DiskText != null);
                json.WriteEndObject();
            }
            json.WriteEndArray();
            json.WriteNumber("overflow", tabs.OverflowCount);
            json.WriteStartArray("problems");
            foreach (DocumentProblem problem in set.Problems()) json.WriteStringValue(problem.Text);
            json.WriteEndArray();
        }
        json.WriteEndObject();
    }
}
