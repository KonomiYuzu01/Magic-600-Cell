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
/// the Fail row expanded one workspace tab away. B29 (ADR-D02.4, .45, .50, .52) fills Runs and Obligations: a run's
/// seed, depth and budget are declared before it starts, the sample's run stops at its budget as cut off and reproduces
/// with bytes equal, and the two CR-17 controls are unavailable with their reason. A research row run from another
/// layout brings the pane into view (ShellChrome), so the Runs tab is one palette row (research.runs) away.
/// </summary>
public static class ShellResearch
{
    /// <summary>The mounted pane, or null before Load.</summary>
    public static ResearchPane? Pane { get; private set; }
    /// <summary>The pane's model in sample mode, or null when no research source is loaded.</summary>
    public static ResearchModel? Model { get; private set; }
    /// <summary>The draft rows the sample's pane registers (research.record.read).</summary>
    public static HashSet<string> DraftIds { get; } = new(StringComparer.Ordinal);
    private static ChromeFrame? _frame;
    private static Func<int> _history = () => 0;

    /// <summary>The research sample's folder: under M600_D02_DATA when set, else the build's staged samples.</summary>
    public static string ResearchFolder()
    {
        var data = System.Environment.GetEnvironmentVariable("M600_D02_DATA") is { Length: > 0 } source
            ? source : ProjectSettings.GlobalizePath("res://samples");
        return Path.Combine(data, data.EndsWith("samples", StringComparison.Ordinal) ? "" : "samples", "research");
    }

    /// <summary>Mounts the pane and registers its draft rows from ADR-D02's draft file whether or not D02 is on (C5-5),
    /// so a research control never names an unregistered row; with the sample on, loads the sample and shows the model.</summary>
    public static void Load(ChromeFrame frame, CommandRegistry registry, string folder, bool sampleOn, Func<int> historyEntries)
    {
        _frame = frame;
        _history = historyEntries;
        var pane = Pane = new ResearchPane(() => frame.Tokens);
        frame.AddPane(ResearchPane.PaneId, ResearchPane.Title, pane);
        Model = null;
        DraftIds.Clear();
        DraftIds.UnionWith(ResearchRows.AddDrafts(registry, File.ReadAllText(Path.Combine(folder, "command-rows-d02.json"))));
        if (!sampleOn)
        {
            return;
        }
        try
        {
            var source = new SampleResearchSource(File.ReadAllText(Path.Combine(ResearchFolder(), "panel.json")));
            var model = new ResearchModel(source, source.Evidence, DisplayServer.ClipboardSet);
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
        // C5-9: the sample chip's words, which name Research among the parts holding sample data.
        json.WriteString("chip", _frame != null && GodotObject.IsInstanceValid(_frame.BarTools)
            && _frame.BarTools.GetNodeOrNull<Control>("SampleChip") is { } chip ? chip.TooltipText : "");
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
            // B29: the declaration, the runs this session started, their reproductions, History's entries (a research
            // run never adds one), the open run's CR-17 reasons and the frame's last notice.
            json.WriteString("declared", model.DeclaredLine());
            json.WriteString("start", model.Unavailable(ResearchRows.Start, null) ?? "ready");
            json.WriteNumber("history_entries", _history());
            json.WriteString("notice", _frame != null && GodotObject.IsInstanceValid(_frame.Notice) ? _frame.Notice.Text : "");
            json.WriteStartArray("started");
            foreach (var run in model.Started)
            {
                json.WriteStartObject();
                json.WriteString("run", run.Run);
                json.WriteString("state", run.State.ToString());
                json.WriteString("line", model.RunLine(run));
                json.WriteNumber("seed", run.Seed ?? 0);
                json.WriteNumber("depth", run.Depth ?? 0);
                json.WriteNumber("budget", run.Budget?.Steps ?? 0);
                json.WriteString("covered", run.Covered ?? "");
                json.WriteString("reproduction", model.Reproductions.TryGetValue(run.Run, out var again) ? ResearchModel.ReproductionLine(again) : "");
                json.WriteEndObject();
            }
            json.WriteEndArray();
            if (model.Open is { } open && open.StartsWith(ResearchModel.RunRef, StringComparison.Ordinal))
            {
                json.WriteStartObject("cr17");
                foreach (var row in ResearchRows.Cr17)
                {
                    json.WriteString(row, model.Unavailable(row, open) ?? "");
                }
                json.WriteEndObject();
            }
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
                if (line.Status is { } status)
                {
                    json.WriteString("status", ErrorOverlay.StateName(status));
                }
                json.WriteEndObject();
            }
            json.WriteEndArray();
        }
        json.WriteEndObject();
    }
}
