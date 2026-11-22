using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace Magic600.Chrome;

// Where a field's value comes from (ADR-D01.23); every field shows it.
public enum Provenance { Default, Theme, Session, Override, Script }

// One field of the inspector's field model. A fact is read-only text about the puzzle state; a number has a range,
// a step and a default, and may carry a live "=" binding whose stored form is the expression itself.
public sealed class InspectField
{
    public string Id { get; }
    public string Section { get; }
    public string Label { get; }
    public bool Essential { get; }
    public bool IsFact { get; }
    public string Fact { get; internal set; } = "";
    public double Min { get; }
    public double Max { get; }
    public double Step { get; }
    public double Default { get; }
    public double Value { get; internal set; }
    public string? Binding { get; internal set; }
    public Provenance Provenance { get; internal set; }

    internal InspectField(string id, string section, string label, string fact)
    { Id = id; Section = section; Label = label; IsFact = true; Fact = fact; }

    internal InspectField(string id, string section, string label, double min, double max, double step, double @default, double value, bool essential, Provenance provenance)
    { Id = id; Section = section; Label = label; Min = min; Max = max; Step = step; Default = @default; Value = value; Essential = essential; Provenance = provenance; }

    public string Display => IsFact ? Fact : Binding != null ? "= " + Binding : Value.ToString(Step >= 1 ? "0" : Step >= .1 ? "0.0" : "0.00", CultureInfo.InvariantCulture);
}

// The two inspector tiers read this one model (ADR-D01.22): the strip shows the selection and the Essential
// preview parameters, the docked inspector shows every field in three fixed sections. Neither tier commits.
public sealed class InspectorModel
{
    public const string Selection = "Selection", View = "View", Preview = "Preview";
    public static readonly IReadOnlyList<string> Sections = Array.AsReadOnly(new[] { Selection, View, Preview });
    public const int EssentialCap = 6;
    public int SelectionCount { get; private set; }
    public string SelectionKind { get; private set; } = "nothing";
    public IReadOnlyList<InspectField> Fields => fields;
    public IEnumerable<InspectField> Essentials => fields.Where(f => f.Essential).Take(EssentialCap);
    // The list of fields changed; a field's value changed in place; the solver edited a field.
    public event Action? Changed;
    public event Action<InspectField>? Updated;
    public event Action<InspectField>? Edited;
    private readonly List<InspectField> fields = new();
    // The value ladder (ADR-D01.23): a notch is a hundred, ten or one of the field's steps, never finer than the
    // step. Each field remembers its rung, and both tiers read the same one.
    public static readonly IReadOnlyList<double> Rungs = Array.AsReadOnly(new[] { 100d, 10d, 1d });
    private readonly Dictionary<string, int> rungs = new(StringComparer.Ordinal);
    public int Rung(string id) => rungs.TryGetValue(id, out int rung) ? rung : Rungs.Count - 1;
    // What the docked inspector holds beyond the strip.
    public int Beyond => fields.Count - Essentials.Count();

    public InspectField this[string id] => fields.FirstOrDefault(f => f.Id == id) ?? throw J.Error(id, "unknown field");

    public void Select(int count, string kind) { SelectionCount = count; SelectionKind = kind; Safe.Raise(Changed); }

    public void AddFact(string id, string label, string fact) => Add(new InspectField(id, Selection, label, fact));

    public InspectField AddNumber(string section, string id, string label, double min, double max, double step, double @default, double value,
        bool essential = false, Provenance provenance = Provenance.Default)
    {
        if (section != View && section != Preview) throw J.Error(id, "a number belongs to View or Preview; puzzle state is read-only");
        if (!(min < max) || !(step > 0) || @default < min || @default > max || value < min || value > max) throw J.Error(id, "range, step, default or value is not valid");
        var field = new InspectField(id, section, label, min, max, step, @default, value, essential, provenance); Add(field);
        return field;
    }

    private void Add(InspectField field)
    {
        if (fields.Any(f => f.Id == field.Id)) throw J.Error(field.Id, "duplicate field");
        fields.Add(field); Safe.Raise(Changed);
    }

    // Removes the fields of a section, for parameters that follow what is staged.
    public void Clear(string section) { if (fields.RemoveAll(f => f.Section == section) > 0) Safe.Raise(Changed); }

    // The host reports a value that changed elsewhere; it is not an edit.
    public void Sync(string id, double value, Provenance provenance)
    {
        InspectField field = this[id];
        if (field.Value == value && field.Provenance == provenance) return;
        field.Value = Math.Clamp(value, field.Min, field.Max); field.Provenance = provenance; Safe.Raise(Updated, field);
    }

    public bool Set(string id, double value)
    {
        InspectField field = this[id];
        if (field.IsFact || !double.IsFinite(value)) return false;
        double snapped = Math.Clamp(field.Min + Math.Round((value - field.Min) / field.Step) * field.Step, field.Min, field.Max);
        if (snapped == field.Value && field.Binding == null) return false;
        field.Value = snapped; field.Binding = null; field.Provenance = snapped == field.Default ? Provenance.Default : Provenance.Session;
        Safe.Raise(Updated, field); Safe.Raise(Edited, field);
        return true;
    }

    // The value ladder: steps of the field's step, ten times coarser or finer on request.
    public bool Nudge(string id, int steps, double scale = 1)
    {
        InspectField field = this[id];
        if (field.IsFact || steps == 0) return false;
        double to = Math.Clamp(field.Value + steps * field.Step * scale, field.Min, field.Max);
        if (to == field.Value) return false;
        field.Value = to; field.Binding = null; field.Provenance = to == field.Default ? Provenance.Default : Provenance.Session;
        Safe.Raise(Updated, field); Safe.Raise(Edited, field);
        return true;
    }

    // Up the ladder is coarser. Changing the rung is not an edit.
    public bool Climb(string id, int by)
    {
        InspectField field = this[id];
        int to = Math.Clamp(Rung(id) + by, 0, Rungs.Count - 1);
        if (field.IsFact || to == Rung(id)) return false;
        rungs[id] = to; Safe.Raise(Updated, field);
        return true;
    }

    public bool Step(string id, int notches) => Nudge(id, notches, Rungs[Rung(id)]);

    // How many numbers of a section are away from their defaults, and one reset for all of them.
    public int Away(string section) => fields.Count(f => f.Section == section && !f.IsFact && (f.Binding != null || f.Value != f.Default));
    public int ResetSection(string section) => fields.Where(f => f.Section == section).ToArray().Count(f => Reset(f.Id));

    public bool Reset(string id)
    {
        InspectField field = this[id];
        if (field.IsFact || (field.Value == field.Default && field.Binding == null)) return false;
        field.Value = field.Default; field.Binding = null; field.Provenance = Provenance.Default;
        Safe.Raise(Updated, field); Safe.Raise(Edited, field);
        return true;
    }

    // A typed value: a number, or "=" and a .6cs expression that stays a live binding. Returns why it was refused.
    public string? Enter(string id, string text)
    {
        InspectField field = this[id];
        if (field.IsFact) return "puzzle state is read-only";
        text = text.Trim();
        if (text.StartsWith('='))
        {
            string expression = text.Substring(1).Trim();
            if (expression.Length == 0) return "an expression follows the = sign";
            field.Binding = expression; field.Provenance = Provenance.Script;
            Safe.Raise(Updated, field); Safe.Raise(Edited, field);
            return null;
        }
        if (!double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out double value) || !double.IsFinite(value)) return "expected a number, or = and an expression";
        if (value < field.Min || value > field.Max) return $"outside {field.Min.ToString(CultureInfo.InvariantCulture)}..{field.Max.ToString(CultureInfo.InvariantCulture)}";
        Set(id, value);
        return null;
    }

    // Live bindings are stored as their expressions, never as values (ADR-D01.23).
    public string Store() => string.Join("\n", fields.Where(f => f.Binding != null).OrderBy(f => f.Id, StringComparer.Ordinal).Select(f => f.Id + "=" + f.Binding));

    public void Restore(string stored)
    {
        foreach (string line in stored.Split('\n', StringSplitOptions.RemoveEmptyEntries))
        {
            int cut = line.IndexOf('=');
            if (cut <= 0 || fields.FirstOrDefault(f => f.Id == line.Substring(0, cut) && !f.IsFact) is not { } field) continue;
            field.Binding = line.Substring(cut + 1); field.Provenance = Provenance.Script; Safe.Raise(Updated, field);
        }
    }
}

public enum Severity { Error, Warning, Info }

// One row of Problems: repeats of the same report are merged into a count (ADR-D01.27).
public sealed record Problem(int Id, Severity Severity, string Text, string Source, int Line, string? Element, int Count);

public enum TaskState { Running, Ok, Failed, Cancelled }

public sealed record TaskItem(int Id, string Name, string Origin, bool ReadsState, double Progress, TaskState State);

// What the bottom panel indexes (ADR-D01.25 to ADR-D01.27): problems, console lines and tasks. The product fills
// it from the runtime process; the chrome draws it and asks for cancels.
public sealed class FeedbackModel
{
    public const int ConsoleKept = 500;
    public IReadOnlyList<Problem> Problems => problems;
    public IReadOnlyList<string> Console => console;
    public IReadOnlyList<TaskItem> Tasks => tasks;
    public ISet<Severity> Hidden { get; } = new HashSet<Severity>();
    public event Action? Changed;
    // A task ended: the status flash (ADR-D01.27).
    public event Action<TaskItem>? Ended;
    private readonly List<Problem> problems = new();
    private readonly List<string> console = new();
    private readonly List<TaskItem> tasks = new();
    private int next = 1;

    public int Count(Severity severity) => problems.Where(p => p.Severity == severity).Sum(p => p.Count);
    public IEnumerable<Problem> Shown => problems.Where(p => !Hidden.Contains(p.Severity));
    public Problem? LastError => problems.LastOrDefault(p => p.Severity == Severity.Error);
    public int Running => tasks.Count(t => t.State == TaskState.Running);

    public Problem Report(Severity severity, string text, string source, int line, string? element = null)
    {
        int at = problems.FindIndex(p => p.Severity == severity && p.Text == text && p.Source == source && p.Line == line);
        Problem problem = at >= 0 ? problems[at] with { Count = problems[at].Count + 1 } : new Problem(next++, severity, text, source, line, element, 1);
        if (at >= 0) problems.RemoveAt(at);
        problems.Add(problem); Safe.Raise(Changed);
        return problem;
    }

    public void ClearSource(string source) { if (problems.RemoveAll(p => p.Source == source) > 0) Safe.Raise(Changed); }

    public void Filter(Severity severity, bool shown) { if (shown ? Hidden.Remove(severity) : Hidden.Add(severity)) Safe.Raise(Changed); }

    public void Log(string line)
    {
        console.Add(line); if (console.Count > ConsoleKept) console.RemoveAt(0);
        Safe.Raise(Changed);
    }

    public TaskItem Start(string name, string origin, bool readsState)
    {
        var task = new TaskItem(next++, name, origin, readsState, 0, TaskState.Running);
        tasks.Add(task); Safe.Raise(Changed);
        return task;
    }

    private bool Replace(int id, Func<TaskItem, TaskItem> change, bool ended)
    {
        int at = tasks.FindIndex(t => t.Id == id && t.State == TaskState.Running);
        if (at < 0) return false;
        tasks[at] = change(tasks[at]); Safe.Raise(Changed);
        if (ended) Safe.Raise(Ended, tasks[at]);
        return true;
    }

    public bool Progress(int id, double progress) => Replace(id, t => t with { Progress = Math.Clamp(progress, 0, 1) }, false);
    public bool Finish(int id, bool ok) => Replace(id, t => t with { Progress = ok ? 1 : t.Progress, State = ok ? TaskState.Ok : TaskState.Failed }, true);
    public bool Cancel(int id) => Replace(id, t => t with { State = TaskState.Cancelled }, true);

    // Esc cancels the task started last.
    public bool CancelLatest() => tasks.LastOrDefault(t => t.State == TaskState.Running) is { } task && Cancel(task.Id);

    // A commit changed the state: every running task that reads it is cancelled (ADR-D01.26).
    public int StateChanged()
    {
        int[] reading = tasks.Where(t => t.State == TaskState.Running && t.ReadsState).Select(t => t.Id).ToArray();
        foreach (int id in reading) Cancel(id);
        return reading.Length;
    }
}
