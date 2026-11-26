using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using Godot;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

/// <summary>
/// ADR-D02 build step B28 (ADR-D02.3, .44 to .46, .49): the research pane `pane.research`, a centre peer of the pair in
/// Analyse. The pane is the shared library's (Chrome/Ui/ResearchPane.cs); the shell mounts it in every mode, so the
/// Analyse layout never shows a missing pane. In sample mode it reads the synthetic sample
/// (tools/looklab/data/samples/research/panel.json, staged as res://samples/research) and registers the sample's draft
/// rows; without the sample it says that its rows wait for K2. The first failing claim group opens, so the owner sees
/// the Fail row expanded one workspace tab away.
/// </summary>
public static class ShellResearch
{
    /// <summary>The mounted pane, or null before Load.</summary>
    public static ResearchPane? Pane { get; private set; }
    /// <summary>The pane's model in sample mode, or null when no research source is loaded.</summary>
    public static ResearchModel? Model { get; private set; }
    /// <summary>The draft rows the sample's pane registers (research.record.read).</summary>
    public static HashSet<string> DraftIds { get; } = new(StringComparer.Ordinal);

    /// <summary>The research sample's folder: under M600_D02_DATA when set, else the build's staged samples.</summary>
    public static string ResearchFolder()
    {
        var data = System.Environment.GetEnvironmentVariable("M600_D02_DATA") is { Length: > 0 } source
            ? source : ProjectSettings.GlobalizePath("res://samples");
        return Path.Combine(data, data.EndsWith("samples", StringComparison.Ordinal) ? "" : "samples", "research");
    }

    /// <summary>Mounts the pane; with the sample on, loads the sample, registers its draft rows and shows the model.</summary>
    public static void Load(ChromeFrame frame, CommandRegistry registry, bool sampleOn)
    {
        var pane = Pane = new ResearchPane(() => frame.Tokens);
        frame.AddPane(ResearchPane.PaneId, ResearchPane.Title, pane);
        Model = null;
        DraftIds.Clear();
        if (!sampleOn)
        {
            return;
        }
        try
        {
            var source = new SampleResearchSource(File.ReadAllText(Path.Combine(ResearchFolder(), "panel.json")));
            var drafts = new CommandRegistry();
            drafts.AddRows(source.DraftRows);
            registry.AddRows(source.DraftRows);
            DraftIds.UnionWith(drafts.Order);
            var model = new ResearchModel(source, source.Evidence);
            if (model.Claims.FirstOrDefault(c => c.Verdict.Kind == VerdictKind.Fail) is { } failing)
            {
                model.Toggle(ResearchModel.Ref(failing));
            }
            Model = model;
            pane.Show(model);
        }
        catch (Exception e) when (e is IOException or JsonException or FormatException)
        {
            Model = null;
            frame.Say("Research sample: " + e.Message, true);
        }
    }

    /// <summary>Carries out a research row and says what it did, or why it did nothing.</summary>
    public static void Execute(ChromeFrame frame, string command, string? argument)
    {
        if (Model is not { } model)
        {
            return;
        }
        var (done, words) = model.Execute(command, argument);
        frame.Say(words, !done);
    }

    /// <summary>The pane for a run's dump: tab, open object, counts, staleness, the carried line, names and the lines.</summary>
    public static void Dump(Utf8JsonWriter json)
    {
        json.WriteStartObject("research");
        json.WriteBoolean("pane", Pane != null && GodotObject.IsInstanceValid(Pane) && Pane.IsVisibleInTree());
        if (Model is { } model)
        {
            json.WriteString("evidence", model.Evidence);
            json.WriteString("tab", model.Tab.ToString());
            json.WriteString("open", model.Open ?? "");
            var (holds, fails, none) = model.Counts();
            json.WriteStartObject("counts");
            json.WriteNumber("groups", model.Claims.Count);
            json.WriteNumber("holds", holds);
            json.WriteNumber("fails", fails);
            json.WriteNumber("no_check", none);
            json.WriteEndObject();
            json.WriteString("head", Pane?.Head.Text ?? "");
            json.WriteBoolean("stale", model.IsStale(ResearchRows.RowOf(model.Tab)));
            json.WriteString("transfer", model.TransferLine());
            json.WriteStartObject("names");
            foreach (var (reference, name) in model.Names)
            {
                json.WriteString(reference, name);
            }
            json.WriteEndObject();
            json.WriteStartArray("lines");
            foreach (var line in Pane?.Shown ?? model.Lines())
            {
                json.WriteStartObject();
                json.WriteString("kind", line.Kind.ToString());
                json.WriteString("text", line.Text);
                if (line.Ref != null)
                {
                    json.WriteString("ref", line.Ref);
                }
                if (line.Verdict != null)
                {
                    json.WriteString("verdict", line.Verdict);
                }
                json.WriteEndObject();
            }
            json.WriteEndArray();
        }
        json.WriteEndObject();
    }
}
