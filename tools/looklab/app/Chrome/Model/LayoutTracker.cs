using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace Magic600.Chrome;

// The layout history's recorder (ADR-D01.14): the frame's own arrangement (appearance, workspace, pane layout, view
// labels and key bindings, FrameLayout's parts) is one step per change, made by hand or by a script, with its own
// undo, redo and roll (layout.undo, layout.redo, layout.roll). It never holds a move, and a move never moves it. One
// copy, shared by Look Lab's sample session (LabDemo) and the product (owner, 26 November 2026); it was Look Lab's
// own before, so the product's layout history stayed empty.
public sealed class LayoutTracker
{
    // The parts of a step, joined into one stored state.
    public const string Cut = "\n\u001e\n";
    public LayoutHistory Layout { get; }
    // A step was put back or a named run recorded: the host draws its History panel again.
    public event Action? Restored;
    public bool Tracking => state != null;
    private readonly Func<string, string> workspaceName;
    private readonly Action flush;
    private Func<IReadOnlyList<string>>? state;
    private Action<IReadOnlyList<string>>? apply;
    private bool restoring;

    // workspaceName: a workspace's shown name; flush: the host lays out what a restored step changed. The host calls
    // Record after each rebuild of its frame (FrameLayout.Attach).
    public LayoutTracker(Func<string, string> workspaceName, Action flush, LayoutHistory? layout = null)
    {
        this.workspaceName = workspaceName; this.flush = flush; Layout = layout ?? new LayoutHistory();
    }

    public static bool Handles(string command) => command is "layout.undo" or "layout.redo" or "layout.roll";

    // Starts the history at the arrangement in force: "Opened".
    public void Track(Func<IReadOnlyList<string>> parts, Action<IReadOnlyList<string>> put)
    {
        state = parts; apply = put; Layout.Record("Opened", string.Join(Cut, parts()));
    }

    // After the frame was built again: one step for what changed, named by the first part that changed. A run of
    // changes to one appearance knob (or the scale) is one step.
    public void Record()
    {
        if (restoring || state == null || Layout.Cursor < 0) return;
        string[] before = Layout.Steps[Layout.Cursor].State.Split(Cut); IReadOnlyList<string> now = state();
        string name = "Arrange panes"; string? merge = null;
        if (now[0] != before[0])
        {
            string changed = now[0].Split('\n').Except(before[0].Split('\n')).FirstOrDefault() ?? "appearance";
            name = "Appearance: " + changed.Replace("=", " ");
            string key = changed.Split('=')[0]; if (key.StartsWith("knob.", StringComparison.Ordinal) || key == "scale") merge = key;
        }
        else if (now[1] != before[1]) name = "Workspace: " + workspaceName(now[1]);
        else if (now.Count > 4 && before.Length > 4 && now[4] != before[4]) name = "Key bindings";
        Layout.Record(name, string.Join(Cut, now), merge);
    }

    public bool Restore(LayoutStep? step)
    {
        if (step == null || apply == null) return false;
        restoring = true;
        try { apply(step.State.Split(Cut)); flush(); } finally { restoring = false; }
        Safe.Raise(Restored);
        return true;
    }

    // One named step for everything the change does.
    public void Run(string name, Action change)
    {
        restoring = true;
        try { change(); flush(); } finally { restoring = false; }
        if (state != null) Layout.Record(name, string.Join(Cut, state()));
        Safe.Raise(Restored);
    }

    // layout.undo, layout.redo and layout.roll: why nothing moved, or null.
    public string? Execute(string command, string? argument) => command switch
    {
        "layout.undo" => Restore(Layout.Undo()) ? null : "No layout change to undo",
        "layout.redo" => Restore(Layout.Redo()) ? null : "No layout change to redo",
        _ => int.TryParse(argument, NumberStyles.None, CultureInfo.InvariantCulture, out int step) && Restore(Layout.RollTo(step)) ? null
            : argument == null ? "Roll the layout: it needs the number of a layout step" : "The layout history is already there",
    };
}
