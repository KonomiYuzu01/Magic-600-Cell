using System;
using System.Collections.Generic;

namespace Magic600.Chrome;

// The arrangement parts the layout history keeps, in order: appearance, workspace, pane layout, view labels and key
// bindings. The host gives the appearance and the key bindings (stored as text), the frame has the rest.
public static class FrameLayout
{
    // A tracker on this frame: it records after each rebuild, names workspaces as the frame shows them and lays out a
    // restored step at once.
    public static LayoutTracker Tracker(ChromeFrame frame, LayoutHistory? layout = null)
    {
        var tracker = new LayoutTracker(id => frame.Workspaces.Names.TryGetValue(id, out string? name) ? name : id, frame.Flush, layout);
        Attach(frame, tracker);
        return tracker;
    }

    public static void Attach(ChromeFrame frame, LayoutTracker tracker) { frame.Rebuilt += tracker.Record; frame.Recoloured += tracker.Record; }

    public static Func<IReadOnlyList<string>> State(ChromeFrame frame, Func<string> appearance, Func<string> keys) =>
        () => new[] { appearance(), frame.Workspace, frame.Model.Store(), frame.Labels ? "on" : "off", keys() };

    public static Action<IReadOnlyList<string>> Apply(ChromeFrame frame, Action<string> appearance, Action<string> keys) => parts =>
    {
        appearance(parts[0]);
        if (frame.Workspaces.Stored.ContainsKey(parts[1])) frame.SelectWorkspace(parts[1]);
        frame.Model.Apply(parts[2]);
        frame.Labels = parts[3] == "on";
        if (parts.Count > 4) keys(parts[4]);
    };
}
