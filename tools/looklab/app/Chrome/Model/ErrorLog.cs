using System;
using System.Collections.Generic;
using System.IO;

namespace Magic600.Chrome;

// The failures that show while the window is open (ADR-D01.25 to ADR-D01.27: Problems, the top bar's lamp): every
// error the engine logs and every listener the chrome caught (Safe.Failed), kept in order up to Safe.Kept and written
// to a log file when the host names one. One copy, shared by Look Lab and the product shell (owner, 26 November 2026:
// the product is the only place where UI is built, shown and accepted); ChromeErrors feeds it and shows it.
public sealed class ErrorLog
{
    private readonly object gate = new();
    private readonly List<string> errors = new();
    private string? file;
    private int clears;
    public int Count { get { lock (gate) return errors.Count; } }
    public IReadOnlyList<string> All { get { lock (gate) return errors.ToArray(); } }
    // Whether a log file takes each failure.
    public bool Writing { get { lock (gate) return file != null; } }

    // Starts the log file (null: none). A file that cannot be written is dropped, and the failures still show.
    public void Open(string? path)
    {
        lock (gate)
        {
            file = path;
            if (path == null) return;
            try { Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(path))!); File.WriteAllText(path, ""); }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or ArgumentException or NotSupportedException) { file = null; }
        }
    }

    // How many times the log was cleared: a lamp that showed an earlier log starts again.
    public int Clears { get { lock (gate) return clears; } }

    public void Clear() { lock (gate) { errors.Clear(); clears++; } }

    // Any thread may add: the engine's logger calls from where the error happened.
    public void Add(string text)
    {
        lock (gate)
        {
            if (errors.Count < Safe.Kept) errors.Add(text);
            if (file == null) return;
            try { File.AppendAllText(file, DateTime.Now.ToString("HH:mm:ss ", System.Globalization.CultureInfo.InvariantCulture) + text + "\n"); }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException) { }
        }
    }

    // What a lamp that has shown `shown` failures of the log as it was after `cleared` clears has to show now: the
    // failures it has not shown, the count and clears it then has shown, and whether the log was cleared since (then it
    // shows the log again from its start).
    public ErrorsSince Since(int cleared, int shown)
    {
        lock (gate)
        {
            bool restarted = cleared != clears || shown > errors.Count;
            int from = restarted ? 0 : shown;
            return new ErrorsSince(errors.GetRange(from, errors.Count - from).ToArray(), errors.Count, clears, restarted);
        }
    }

    // A failure as one line, the way the lamp's tip and a Problems row show it.
    public static string Line(string text) => text.Split('\n')[0];
}

public sealed record ErrorsSince(IReadOnlyList<string> Fresh, int Shown, int Cleared, bool Restarted);
