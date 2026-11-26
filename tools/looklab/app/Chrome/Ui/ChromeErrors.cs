using System;
using Godot;

namespace Magic600.Chrome;

// Failures show while the window is open, in every app that stages the chrome (owner, 26 November 2026): every error
// the engine logs and every listener the chrome caught (Safe.Failed) goes into one ErrorLog, a Problems row and the
// error lamp in the top bar, which opens Problems. Look Lab installs the listener at its start with its log file; the
// product's frame installs it when it mounts the lamp. The lamp and the listener were Look Lab's alone before, so the
// product shell's failures were silent.
public static class ChromeErrors
{
    public static ErrorLog Log { get; } = new();
    private static ChromeErrorListener? listener;
    private static bool leaving;
    public static bool Installed => listener != null;
    public static int Count => Log.Count;
    public static System.Collections.Generic.IReadOnlyList<string> All => Log.All;

    // Listens to the engine's errors and to Safe.Failed; `file` (or null) starts the log file.
    public static void Install(string? file)
    {
        Log.Open(file);
        if (listener != null) return;
        listener = new ChromeErrorListener(); OS.AddLogger(listener);
        Safe.Failed += Log.Add;
    }

    // The engine still logs while it shuts down; the listener leaves before that.
    public static void Remove()
    {
        if (listener == null) return;
        Safe.Failed -= Log.Add; OS.RemoveLogger(listener); listener = null;
    }

    public static void Reset() { Log.Clear(); Safe.Reset(); }

    // Mounts the error lamp on a frame, once per frame: each new failure becomes an error row of `problems` (a host
    // without its own Problems panel passes null and gets one here) under `source`, and the lamp counts them. Without
    // a listener yet, the mount installs one with `file` and removes it when the window closes; with one, a `file`
    // given here starts the log file if none is written yet. A later mount on the same frame may name the host's
    // Problems model, which then takes every failure again.
    public static ErrorLamp Mount(ChromeFrame frame, FeedbackModel? problems, string source, string? file = null)
    {
        bool owns = !Installed;
        if (owns) Install(file);
        else if (file != null && !Log.Writing) Log.Open(file);
        if (frame.GetNodeOrNull<ErrorLamp>(ErrorLamp.NodeName) is not { } lamp)
        {
            lamp = new ErrorLamp(frame, owns);
            frame.AddChild(lamp);
            frame.AlertPressed += () => { if (GodotObject.IsInstanceValid(frame)) frame.Model.ShowDock(DockSide.Bottom, "problems"); };
        }
        lamp.Bind(problems, source);
        return lamp;
    }

    // An owning lamp removes the listener when the scene tree ends, before the engine's own shutdown logs.
    internal static void LeaveWith(SceneTree tree)
    {
        if (leaving) return;
        leaving = true;
        tree.Root.TreeExiting += () => { Remove(); leaving = false; };
    }
}

// The lamp's eye on the log: once a frame it moves the new failures into Problems and sets the top bar's count.
public sealed partial class ErrorLamp : Node
{
    public const string NodeName = "ChromeErrorLamp";
    public FeedbackModel Problems { get; private set; } = new();
    public string Source { get; private set; } = "app";
    // Whether this lamp made its own Problems panel (the host has none).
    public bool OwnPanel { get; private set; }
    private readonly ChromeFrame frame;
    private readonly bool owns;
    private bool bound;
    private int shown, cleared;
    private string first = "";

    public ErrorLamp(ChromeFrame frame, bool owns) { this.frame = frame; this.owns = owns; Name = NodeName; }

    public void Bind(FeedbackModel? problems, string source)
    {
        Source = source;
        if (problems == null)
        {
            if (bound) return;
            bound = OwnPanel = true;
            var panel = new FeedbackPanel(Problems, FeedbackPanel.Kind.Problems, () => frame.Tokens) { Name = "ErrorProblems" };
            frame.SetContent("problems", panel);
            void Refresh() { if (GodotObject.IsInstanceValid(panel)) panel.Refresh(); }
            frame.Rebuilt += Refresh; frame.Recoloured += Refresh;
            Problems.Changed += Badge;
        }
        else if (!ReferenceEquals(problems, Problems))
        {
            if (OwnPanel) Problems.Changed -= Badge;
            Problems = problems; OwnPanel = false; bound = true; shown = 0; cleared = -1;
        }
        Follow();
    }

    public override void _EnterTree() { if (owns) ChromeErrors.LeaveWith(GetTree()); }

    public override void _Process(double delta) => Follow();

    public void Follow()
    {
        ErrorsSince now = ChromeErrors.Log.Since(cleared, shown);
        if (now.Fresh.Count == 0 && !now.Restarted) return;
        if (now.Restarted) first = "";
        if (first.Length == 0 && now.Fresh.Count > 0) first = ErrorLog.Line(now.Fresh[0]);
        foreach (string failure in now.Fresh) Problems.Report(Severity.Error, ErrorLog.Line(failure), Source, 0);
        shown = now.Shown; cleared = now.Cleared;
        if (GodotObject.IsInstanceValid(frame)) frame.SetAlert(shown, shown == 0 ? "" : first);
    }

    // The collapsed Problems tab counts the errors, as the sample inspector's badges do.
    private void Badge()
    {
        if (!GodotObject.IsInstanceValid(frame)) return;
        int errors = Problems.Count(Severity.Error);
        frame.SetBadge("problems", errors > 0 ? errors.ToString(System.Globalization.CultureInfo.InvariantCulture) : null, "severity.error", errors + " errors");
    }
}

internal sealed partial class ChromeErrorListener : Logger
{
    public override void _LogError(string function, string file, int line, string code, string rationale, bool editorNotify, int errorType, Godot.Collections.Array<ScriptBacktrace> scriptBacktraces)
    {
        if (errorType == (int)Logger.ErrorType.Warning) return;
        ChromeErrors.Log.Add($"{(rationale.Length > 0 ? rationale : code)}  ({file}:{line} {function})");
    }
}
