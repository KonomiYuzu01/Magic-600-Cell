using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;

namespace Magic600.Chrome;

public enum DockSide { Left, Right, Bottom }
public enum DropZone { Left, Right, Top, Bottom, Tab }

// One node of the centre pane group: a split of two nodes, or a tab group of panes.
public sealed class PaneNode
{
    public bool IsSplit => A != null;
    public bool Horizontal { get; set; }
    public double Ratio { get; set; } = .5;
    public PaneNode? A { get; set; }
    public PaneNode? B { get; set; }
    public List<string> Tabs { get; } = new();
    public int Active { get; set; }

    public static PaneNode Group(params string[] panes) { var n = new PaneNode(); n.Tabs.AddRange(panes); return n; }
    public static PaneNode Split(bool horizontal, double ratio, PaneNode a, PaneNode b) => new() { Horizontal = horizontal, Ratio = ratio, A = a, B = b };
    public IEnumerable<string> Panes() => IsSplit ? A!.Panes().Concat(B!.Panes()) : Tabs;
    public PaneNode Clone()
    {
        if (IsSplit) return Split(Horizontal, Ratio, A!.Clone(), B!.Clone());
        PaneNode n = Group(Tabs.ToArray()); n.Active = Active; return n;
    }
}

public sealed class DockState
{
    public List<string> Panels { get; } = new();
    public string? Open { get; set; }
}

// The app-owned dock model (ADR-D01.33, ADR-D01.36): a fixed frame of three docks around a centre pane group.
// The main pair is one locked pane: it cannot be closed, hidden behind a tab or left out, and a change that
// would do so is refused with a reason the frame shows. A workspace is one stored value of this model; it is
// arrangement only and never changes commands, capabilities or commits (ADR-D01.39).
public sealed class FrameModel
{
    public const string Pair = "pair";
    public static readonly IReadOnlyList<string> PairArrangements = Array.AsReadOnly(new[] { "side", "stacked" });
    public IReadOnlyDictionary<DockSide, DockState> Docks { get; } = new SortedDictionary<DockSide, DockState>
        { [DockSide.Left] = new(), [DockSide.Right] = new(), [DockSide.Bottom] = new() };
    public PaneNode Centre { get; private set; } = PaneNode.Group(Pair);
    public string PairArrangement { get; private set; } = "side";
    public double PairRatio { get; private set; } = .5;
    public bool FocusMode { get; private set; }
    public bool PairMaximised { get; private set; }
    // The last refusal, for the frame to show; cleared by the next accepted change.
    public string? Refusal { get; private set; }
    public event Action? Changed;

    private bool Refuse(string reason) { Refusal = reason; Changed?.Invoke(); return false; }
    private bool Accept() { Refusal = null; Changed?.Invoke(); return true; }

    public void AddDockPanel(DockSide side, string id)
    {
        if (string.IsNullOrWhiteSpace(id) || Docks.Values.Any(d => d.Panels.Contains(id))) throw J.Error(id, "empty or repeated dock panel");
        Docks[side].Panels.Add(id);
    }

    // Each dock shows one panel at a time; opening the open panel again collapses the dock.
    public bool ToggleDock(DockSide side, string id)
    {
        DockState dock = Docks[side];
        if (!dock.Panels.Contains(id)) return Refuse($"{id} is not a panel of the {side.ToString().ToLowerInvariant()} dock");
        dock.Open = dock.Open == id ? null : id;
        return Accept();
    }

    public bool ShowDock(DockSide side, string? id)
    {
        if (id != null && !Docks[side].Panels.Contains(id)) return Refuse($"{id} is not a panel of the {side.ToString().ToLowerInvariant()} dock");
        Docks[side].Open = id;
        return Accept();
    }

    public bool SetPairArrangement(string arrangement, double ratio)
    {
        if (!PairArrangements.Contains(arrangement)) return Refuse("the main pair is shown side by side or stacked");
        if (!(ratio >= .2 && ratio <= .8)) return Refuse("each main view keeps at least a fifth of the pair");
        PairArrangement = arrangement; PairRatio = ratio;
        return Accept();
    }

    // Maximising either main view maximises the pair; focus mode keeps only the pair (ADR-D01.36).
    public bool SetPairMaximised(bool on) { PairMaximised = on; return Accept(); }
    public bool SetFocusMode(bool on) { FocusMode = on; return Accept(); }
    public bool OnlyPairShown => FocusMode || PairMaximised;

    private static PaneNode? Find(PaneNode node, string pane) =>
        node.IsSplit ? Find(node.A!, pane) ?? Find(node.B!, pane) : node.Tabs.Contains(pane) ? node : null;

    private static PaneNode? Without(PaneNode node, string pane)
    {
        if (!node.IsSplit)
        {
            if (!node.Tabs.Remove(pane)) return node;
            node.Active = Math.Clamp(node.Active, 0, Math.Max(0, node.Tabs.Count - 1));
            return node.Tabs.Count == 0 ? null : node;
        }
        PaneNode? a = Without(node.A!, pane), b = Without(node.B!, pane);
        if (a == null) return b;
        if (b == null) return a;
        node.A = a; node.B = b;
        return node;
    }

    private static bool Replace(PaneNode node, PaneNode target, PaneNode with)
    {
        if (!node.IsSplit) return false;
        if (ReferenceEquals(node.A, target)) { node.A = with; return true; }
        if (ReferenceEquals(node.B, target)) { node.B = with; return true; }
        return Replace(node.A!, target, with) || Replace(node.B!, target, with);
    }

    // Opens a pane, or moves an open one, beside or into the group that holds the target pane.
    public bool Drop(string pane, string target, DropZone zone)
    {
        if (string.IsNullOrWhiteSpace(pane)) throw J.Error("pane", "empty pane id");
        if (pane == Pair) return Refuse("the main pair stays where the workspace puts it; arrange other panes around it");
        if (pane == target) return Refuse("a pane cannot be dropped on itself");
        if (Find(Centre, target) == null) return Refuse($"{target} is not open");
        if (zone == DropZone.Tab && target == Pair) return Refuse("a tab here would hide the Global and Local views; drop beside the pair instead");
        PaneNode centre = Centre.Clone();
        centre = Without(centre, pane) ?? PaneNode.Group(Pair);
        PaneNode group = Find(centre, target)!;
        if (zone == DropZone.Tab) { group.Tabs.Add(pane); group.Active = group.Tabs.Count - 1; }
        else
        {
            PaneNode added = PaneNode.Group(pane);
            bool first = zone is DropZone.Left or DropZone.Top;
            PaneNode split = PaneNode.Split(zone is DropZone.Left or DropZone.Right, first ? .4 : .6, first ? added : group, first ? group : added);
            if (ReferenceEquals(centre, group)) centre = split; else Replace(centre, group, split);
        }
        Centre = centre;
        return Accept();
    }

    public bool Close(string pane)
    {
        if (pane == Pair) return Refuse("the Global and Local views cannot be closed");
        if (Find(Centre, pane) == null) return Refuse($"{pane} is not open");
        Centre = Without(Centre.Clone(), pane) ?? PaneNode.Group(Pair);
        return Accept();
    }

    public bool Activate(string pane)
    {
        PaneNode? group = Find(Centre, pane);
        if (group == null) return Refuse($"{pane} is not open");
        group.Active = group.Tabs.IndexOf(pane);
        return Accept();
    }

    public bool IsOpen(string pane) => Find(Centre, pane) != null;

    // ----- stored form: a workspace, the session's layout and each layout-history step -----

    public string Store()
    {
        using var memory = new MemoryStream();
        using (var w = new Utf8JsonWriter(memory, new JsonWriterOptions { Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping }))
        {
            w.WriteStartObject();
            w.WriteStartObject("docks");
            foreach (var dock in Docks) { if (dock.Value.Open == null) w.WriteNull(Name(dock.Key)); else w.WriteString(Name(dock.Key), dock.Value.Open); }
            w.WriteEndObject();
            w.WriteStartObject("pair"); w.WriteString("arrangement", PairArrangement); w.WriteNumber("ratio", PairRatio); w.WriteEndObject();
            w.WriteBoolean("focus", FocusMode);
            w.WritePropertyName("centre"); Write(w, Centre);
            w.WriteEndObject();
        }
        return Encoding.UTF8.GetString(memory.ToArray());
    }

    private static string Name(DockSide side) => side.ToString().ToLowerInvariant();

    private static void Write(Utf8JsonWriter w, PaneNode node)
    {
        w.WriteStartObject();
        if (node.IsSplit)
        {
            w.WriteString("split", node.Horizontal ? "horizontal" : "vertical"); w.WriteNumber("ratio", node.Ratio);
            w.WritePropertyName("a"); Write(w, node.A!); w.WritePropertyName("b"); Write(w, node.B!);
        }
        else
        {
            w.WriteStartArray("tabs"); foreach (string tab in node.Tabs) w.WriteStringValue(tab); w.WriteEndArray();
            w.WriteNumber("active", node.Active);
        }
        w.WriteEndObject();
    }

    // Applies a stored arrangement. A dock panel this build does not have is left closed; a centre that breaks
    // the locked-pair rule is refused whole and the current arrangement stays.
    public bool Apply(string stored)
    {
        PaneNode centre; string arrangement; double ratio; bool focus;
        var open = new Dictionary<DockSide, string?>();
        try
        {
            using JsonDocument d = J.Parse(stored);
            JsonElement root = d.RootElement;
            J.Keys(root, "arrangement", "docks", "pair", "focus", "centre");
            JsonElement docks = root.GetProperty("docks"); J.Keys(docks, "docks", "left", "right", "bottom");
            foreach (DockSide side in Docks.Keys)
            {
                JsonElement value = docks.GetProperty(Name(side));
                open[side] = value.ValueKind == JsonValueKind.Null ? null : J.String(value, Name(side));
            }
            JsonElement pair = root.GetProperty("pair"); J.Keys(pair, "pair", "arrangement", "ratio");
            arrangement = J.Text(pair, "arrangement"); ratio = J.Number(pair.GetProperty("ratio"), "ratio");
            JsonElement f = root.GetProperty("focus");
            if (f.ValueKind is not (JsonValueKind.True or JsonValueKind.False)) throw J.Error("focus", "expected true or false");
            focus = f.GetBoolean();
            centre = Read(root.GetProperty("centre"));
        }
        catch (Exception ex) when (ex is FormatException or JsonException or KeyNotFoundException) { return Refuse("the stored arrangement cannot be read: " + ex.Message); }
        string[] panes = centre.Panes().ToArray();
        if (panes.Distinct().Count() != panes.Length) return Refuse("the stored arrangement opens a pane twice");
        PaneNode? pairGroup = Find(centre, Pair);
        if (pairGroup == null) return Refuse("the stored arrangement leaves out the Global and Local views");
        if (pairGroup.Tabs.Count != 1) return Refuse("the stored arrangement tabs a pane over the Global and Local views");
        if (!PairArrangements.Contains(arrangement) || !(ratio >= .2 && ratio <= .8)) return Refuse("the stored arrangement gives a main view less than a fifth of the pair");
        foreach (DockSide side in Docks.Keys) Docks[side].Open = open[side] is { } id && Docks[side].Panels.Contains(id) ? id : null;
        Centre = centre; PairArrangement = arrangement; PairRatio = ratio; FocusMode = focus; PairMaximised = false;
        return Accept();
    }

    private static PaneNode Read(JsonElement value)
    {
        if (value.ValueKind == JsonValueKind.Object && value.TryGetProperty("split", out _))
        {
            J.Keys(value, "split", "split", "ratio", "a", "b");
            string kind = J.Text(value, "split");
            double ratio = J.Number(value.GetProperty("ratio"), "ratio");
            if ((kind != "horizontal" && kind != "vertical") || !(ratio >= .1 && ratio <= .9)) throw J.Error("split", "expected horizontal or vertical and a ratio in 0.1..0.9");
            return PaneNode.Split(kind == "horizontal", ratio, Read(value.GetProperty("a")), Read(value.GetProperty("b")));
        }
        J.Keys(value, "tabs", "tabs", "active");
        PaneNode node = PaneNode.Group(J.Strings(value.GetProperty("tabs"), "tabs"));
        if (!value.GetProperty("active").TryGetInt32(out int active) || active < 0 || active >= node.Tabs.Count) throw J.Error("active", "not a tab of its group");
        node.Active = active;
        return node;
    }
}

// A main view's status line (ADR-D01.45): the hovered element, its gestures and the set and effective values,
// under a capped length. The hovered element is kept longest; gestures are dropped from the end first.
public static class StatusLine
{
    public const int Cap = 96;
    public const string Separator = "  ·  ";
    public static string Compose(string? hovered, IReadOnlyList<string> gestures, string? values, int cap = Cap)
    {
        if (cap < 8) throw J.Error("cap", "a status line needs at least 8 characters");
        string head = string.IsNullOrWhiteSpace(hovered) ? "nothing hovered" : hovered!;
        for (int kept = gestures.Count; kept >= 0; kept--)
        {
            var parts = new List<string> { head }; parts.AddRange(gestures.Take(kept));
            if (!string.IsNullOrWhiteSpace(values)) parts.Add(values!);
            string line = string.Join(Separator, parts);
            if (line.Length <= cap) return line;
        }
        return head.Length <= cap ? head : head.Substring(0, cap - 1) + "…";
    }
}

// The shipped workspace presets (ADR-D01.38): Solve, Script, Analyse, Reference.
public sealed class WorkspaceSet
{
    public IReadOnlyList<string> Order { get; }
    public IReadOnlyDictionary<string, string> Stored { get; }
    public IReadOnlyDictionary<string, string> Names { get; }

    public static WorkspaceSet Load(string path) => Parse(File.ReadAllText(path));
    public static WorkspaceSet Parse(string text)
    {
        using JsonDocument d = J.Parse(text);
        JsonElement root = J.Root(d, "magic600-design-workspaces", "workspaces");
        var order = new List<string>(); var stored = new Dictionary<string, string>(StringComparer.Ordinal); var names = new Dictionary<string, string>(StringComparer.Ordinal);
        if (root.GetProperty("workspaces").ValueKind != JsonValueKind.Array) throw J.Error("workspaces", "expected a list");
        foreach (JsonElement w in root.GetProperty("workspaces").EnumerateArray())
        {
            J.Keys(w, "workspace", "id", "name", "arrangement");
            string id = J.Text(w, "id");
            if (!stored.TryAdd(id, w.GetProperty("arrangement").GetRawText())) throw J.Error(id, "repeated workspace");
            names.Add(id, J.Text(w, "name")); order.Add(id);
            var probe = new FrameModel();
            foreach (DockSide side in probe.Docks.Keys) probe.AddDockPanel(side, "probe." + side);
            // A preset must be a legal arrangement whatever panels the host adds.
            if (!probe.Apply(stored[id])) throw J.Error(id, probe.Refusal!);
        }
        if (order.Count == 0) throw J.Error("workspaces", "at least one workspace required");
        return new WorkspaceSet(order, stored, names);
    }

    private WorkspaceSet(List<string> order, Dictionary<string, string> stored, Dictionary<string, string> names)
    {
        Order = order.AsReadOnly(); Stored = stored; Names = names;
    }
}
