using System;
using System.Collections.Generic;
using System.IO;
using Godot;
using Magic600.Chrome;

namespace LookLab.App.Live;

// Failures show while the window is open: every error the engine logs and every listener the chrome caught is
// counted here, written to a log file beside the kept view and shown by the error lamp in the top bar.
internal static class LabErrors
{
    private static readonly object Gate = new();
    private static readonly List<string> Errors = new();
    private static ErrorListener? listener;
    private static string? logFile;
    public static int Count { get { lock (Gate) return Errors.Count; } }
    public static IReadOnlyList<string> All { get { lock (Gate) return Errors.ToArray(); } }

    public static void Install(string? file)
    {
        lock (Gate)
        {
            logFile = file;
            if (file != null)
                try { Directory.CreateDirectory(Path.GetDirectoryName(file)!); File.WriteAllText(file, ""); } catch (Exception ex) when (ex is IOException or UnauthorizedAccessException) { logFile = null; }
        }
        if (listener != null) return;
        listener = new ErrorListener(); OS.AddLogger(listener);
        Safe.Failed += Add;
    }

    // The engine still logs while it shuts down; the listener leaves before that.
    public static void Remove()
    {
        if (listener == null) return;
        Safe.Failed -= Add; OS.RemoveLogger(listener); listener = null;
    }

    public static void Reset() { lock (Gate) Errors.Clear(); Safe.Reset(); }

    internal static void Add(string text)
    {
        lock (Gate)
        {
            if (Errors.Count < Safe.Kept) Errors.Add(text);
            if (logFile == null) return;
            try { File.AppendAllText(logFile, DateTime.Now.ToString("HH:mm:ss ") + text + "\n"); } catch (Exception ex) when (ex is IOException or UnauthorizedAccessException) { }
        }
    }
}

internal sealed partial class ErrorListener : Logger
{
    public override void _LogError(string function, string file, int line, string code, string rationale, bool editorNotify, int errorType, Godot.Collections.Array<ScriptBacktrace> scriptBacktraces)
    {
        if (errorType == (int)Logger.ErrorType.Warning) return;
        LabErrors.Add($"{(rationale.Length > 0 ? rationale : code)}  ({file}:{line} {function})");
    }
}
