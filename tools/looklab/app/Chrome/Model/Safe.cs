using System;
using System.Collections.Generic;

namespace Magic600.Chrome;

// Every model and frame event is raised through here: each subscriber is called, a subscriber that fails is
// recorded and the rest still run. A command never fails because one panel could not repaint.
public static class Safe
{
    public const int Kept = 200;
    private static readonly List<string> failures = new();
    private static readonly object gate = new();
    public static int Failures { get { lock (gate) return failures.Count; } }
    public static IReadOnlyList<string> Log { get { lock (gate) return failures.ToArray(); } }
    // The host shows a failure: an error lamp, a Problems row, a log line.
    public static event Action<string>? Failed;

    public static void Reset() { lock (gate) failures.Clear(); }

    public static void Report(string text)
    {
        lock (gate) { if (failures.Count < Kept) failures.Add(text); }
        Action<string>? listeners = Failed;
        if (listeners == null) return;
        foreach (Action<string> listener in listeners.GetInvocationList())
            try { listener(text); } catch (Exception) { }
    }

    public static void Report(Exception ex) => Report(ex.GetType().Name + ": " + ex.Message + (ex.StackTrace is { } trace ? "\n" + trace : ""));

    public static void Raise(Action? handlers)
    {
        if (handlers == null) return;
        foreach (Action handler in handlers.GetInvocationList())
            try { handler(); } catch (Exception ex) { Report(ex); }
    }

    public static void Raise<T>(Action<T>? handlers, T value)
    {
        if (handlers == null) return;
        foreach (Action<T> handler in handlers.GetInvocationList())
            try { handler(value); } catch (Exception ex) { Report(ex); }
    }

    public static void Raise<T1, T2>(Action<T1, T2>? handlers, T1 first, T2 second)
    {
        if (handlers == null) return;
        foreach (Action<T1, T2> handler in handlers.GetInvocationList())
            try { handler(first, second); } catch (Exception ex) { Report(ex); }
    }
}
