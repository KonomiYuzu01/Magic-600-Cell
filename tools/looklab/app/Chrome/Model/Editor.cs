using System;
using System.Collections.Generic;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Magic600.Chrome;

// What the editor needs beside the text (ADR-D01.25, ADR-D01.57, ADR-D01.58): completion and hovers, symbol input,
// diagnostics with their reason and remedy, quick fixes, the run queue, the results of reads and the rules that
// keep a run and a commit apart. This service is the sample one: it knows the words of the sample programs.
public sealed record Completion(string Word, string Kind, string Signature, string Help);
public sealed record DiagnosticHelp(string Why, string Do);
public enum FixKind { Syntax, Contract, Invert, Mirror, Conjugate, Repeat, Rescope }
public sealed record QuickFix(FixKind Kind, int Line, string Title, string NewLine);

public static class SixCsService
{
    public static readonly IReadOnlyList<Completion> Builtins = Array.AsReadOnly(new[]
    {
        new Completion("ring_cells", "fn", "ring_cells(ring: Nat)", "The cells of one ring."),
        new Completion("orbits_of", "fn", "orbits_of(cells)", "The orbits the cells belong to."),
        new Completion("stickers_of", "fn", "stickers_of(cells)", "Every sticker slot of the cells."),
        new Completion("count", "fn", "count(items)", "How many items."),
        new Completion("sweep", "fn", "sweep(cells, axis)", "Twist every cell about one axis."),
        new Completion("then", "fn", "then(next)", "Continue a recipe with another."),
        new Completion("inverse", "fn", "inverse(recipe)", "The recipe backwards."),
        new Completion("mirror", "fn", "mirror(recipe)", "The mirrored recipe."),
        new Completion("conjugate", "fn", "conjugate(setup, recipe)", "Setup, recipe, setup backwards."),
        new Completion("repeat", "fn", "repeat(recipe, times: Nat)", "The recipe several times."),
        new Completion("preview", "fn", "preview(recipe)", "Stage a recipe as a preview. Never commits."),
        new Completion("selection", "fn", "selection()", "The cells selected now."),
        new Completion("cells", "fn", "cells()", "All 600 cells."),
        new Completion("vertices", "fn", "vertices()", "All 120 vertices."),
        new Completion("hopf_fibres", "fn", "hopf_fibres(points)", "The points grouped into fibres."),
        new Completion("panel", "fn", "panel(id: Text, parts)", "A docked panel drawn by the host."),
        new Completion("table", "fn", "table(record)", "A table of a record."),
        new Completion("chart", "fn", "chart(values)", "A chart of values."),
        new Completion("canvas", "fn", "canvas(marks)", "A drawing. Display only."),
        new Completion("annotate", "fn", "annotate(cells, label: Text)", "Mark cells in both views."),
        new Completion("arrange", "fn", "arrange(layout)", "Change the layout. One layout step."),
        new Completion("cell", "fn", "cell(id: Nat)", "One cell by its number."),
        new Completion("history", "fn", "history(id: Text)", "The recipe of a history entry."),
        new Completion("fact", "fn", "fact(id: Text)", "A fact the inspector shows."),
    });

    public static readonly IReadOnlyDictionary<string, string> Symbols = new SortedDictionary<string, string>(StringComparer.Ordinal)
    {
        ["alpha"] = "α", ["beta"] = "β", ["gamma"] = "γ", ["delta"] = "δ", ["theta"] = "θ", ["lambda"] = "λ", ["pi"] = "π", ["sigma"] = "σ", ["phi"] = "φ", ["omega"] = "ω",
        ["to"] = "→", ["mapsto"] = "↦", ["inv"] = "⁻¹", ["circ"] = "∘", ["in"] = "∈", ["times"] = "×", ["cdot"] = "·", ["tau"] = "τ",
    };

    public const int Offered = 8;

    public static IReadOnlyList<Completion> Completions(string prefix)
    {
        if (prefix.Length == 0) return Array.Empty<Completion>();
        IEnumerable<Completion> words = SixCs.Keywords.Select(w => new Completion(w, "keyword", w, "")).Concat(SixCs.Types.Select(w => new Completion(w, "type", w, ""))).Concat(Builtins);
        return words.Where(c => c.Word.StartsWith(prefix, StringComparison.Ordinal) && c.Word != prefix).OrderBy(c => c.Word, StringComparer.Ordinal).Take(Offered).ToArray();
    }

    public static Completion? Hover(string word) => Builtins.FirstOrDefault(c => c.Word == word);

    // The name typed after a backslash at the end of the text before the caret, or null.
    public static string? SymbolPrefix(string before)
    {
        Match m = Regex.Match(before, @"\\([A-Za-z]*)$");
        return m.Success ? m.Groups[1].Value : null;
    }

    public static IReadOnlyList<(string Name, string Symbol)> SymbolCandidates(string prefix) =>
        Symbols.Where(s => s.Key.StartsWith(prefix, StringComparison.Ordinal)).Select(s => (s.Key, s.Value)).Take(Offered).ToArray();

    private static string CodeOf(string line) { int cut = SixCs.CommentStart(line); return cut >= 0 ? line.Substring(0, cut) : line; }
    private static bool Heads(string line) => Regex.IsMatch(line, @"^\s*(export\s+)?(fn|command)\s");

    // The chrome's own checks and the sample service's: a statement without its end, a preview outside the
    // declared effects, a value nothing reads, an open note.
    public static IReadOnlyList<ScriptDiagnostic> Diagnose(string source)
    {
        var found = SixCs.Diagnose(source).ToList(); string[] lines = source.Split('\n');
        for (int i = 0; i < lines.Length; i++)
        {
            string code = CodeOf(lines[i]).Trim();
            if ((code.StartsWith("let ", StringComparison.Ordinal) || code.StartsWith("return ", StringComparison.Ordinal)) && !Regex.IsMatch(code, @"(;|\{|,|\(|\|>)$"))
                found.Add(new ScriptDiagnostic(Severity.Error, i + 1, "`;` is missing"));
            Match effects = Regex.Match(code, @"^export\s+(?:fn|command)\b.*\beffects\s*\[([^\]]*)\]");
            if (effects.Success && !effects.Groups[1].Value.Split(',', StringSplitOptions.TrimEntries).Contains("preview"))
            {
                int end = i + 1; while (end < lines.Length && !Heads(lines[end])) end++;
                if (Enumerable.Range(i + 1, end - i - 1).Any(k => Regex.IsMatch(CodeOf(lines[k]), @"\bpreview\(")))
                    found.Add(new ScriptDiagnostic(Severity.Error, i + 1, "`preview` is not in its effects"));
            }
            int note = SixCs.CommentStart(lines[i]);
            if (note >= 0 && Regex.IsMatch(lines[i].Substring(note), @"^#\s*todo\b")) found.Add(new ScriptDiagnostic(Severity.Info, i + 1, "open note"));
        }
        foreach ((int line, string name) in UnusedLets(source)) found.Add(new ScriptDiagnostic(Severity.Warning, line, $"`{name}` is never used"));
        return found.OrderBy(d => d.Line).ThenBy(d => d.Severity).ToArray();
    }

    // Why a diagnostic is one and what to do about it: the hover shows what, why and what to do.
    public static DiagnosticHelp Explain(ScriptDiagnostic d)
    {
        string t = d.Text;
        if (t.Contains("is missing")) return new("A statement ends with `;`.", "Tab takes the fix.");
        if (t.Contains("not in its effects")) return new("A run may end only in what its effects name.", "Add `preview` to the list. Tab takes the fix.");
        if (t.Contains("never commits")) return new("Only the commit key commits.", "Remove `commit` from the effects.");
        if (t.Contains("never closed") || t.Contains("closes nothing")) return new("Brackets come in pairs.", "Add or remove one bracket.");
        if (t.Contains("not closed")) return new("A text ends on the line it starts on.", "Close it with a quote.");
        if (t.Contains("never used")) return new("Nothing reads this value.", "Use it or remove the line.");
        if (t.Contains("no value")) return new("A `let` names a value.", "Give it one.");
        if (t.Contains("open note")) return new("A note you left.", "Finish it or keep it.");
        if (t.Contains("edition")) return new("The edition fixes the meaning of the words.", "Add the edition line.");
        return new("", "");
    }

    // Lets whose name nothing below them reads, up to the next fn or command.
    public static IReadOnlyList<(int Line, string Name)> UnusedLets(string source)
    {
        var unused = new List<(int, string)>(); string[] lines = source.Split('\n');
        for (int i = 0; i < lines.Length; i++)
        {
            Match m = Regex.Match(CodeOf(lines[i]), @"^\s*let\s+([a-z_][a-z0-9_]*)\s*=");
            if (!m.Success) continue;
            var word = new Regex(@"\b" + Regex.Escape(m.Groups[1].Value) + @"\b"); bool used = false;
            for (int k = i + 1; k < lines.Length && !Heads(lines[k]) && !used; k++) used = word.IsMatch(CodeOf(lines[k]));
            if (!used) unused.Add((i + 1, m.Groups[1].Value));
        }
        return unused;
    }

    // The quick fixes of a program: one for each diagnostic that has one, and the recipe rewrites of a line.
    public static IReadOnlyList<QuickFix> Fixes(string source)
    {
        var fixes = new List<QuickFix>(); string[] lines = source.Split('\n');
        foreach (ScriptDiagnostic d in Diagnose(source))
        {
            string line = lines[d.Line - 1]; int cut = SixCs.CommentStart(line);
            if (d.Text.Contains("is missing"))
                fixes.Add(new QuickFix(FixKind.Syntax, d.Line, "add `;`", cut >= 0 ? line.Substring(0, cut).TrimEnd() + "; " + line.Substring(cut) : line.TrimEnd() + ";"));
            else if (d.Text.Contains("not in its effects"))
                fixes.Add(new QuickFix(FixKind.Contract, d.Line, "add `preview` to effects", Regex.Replace(line, @"effects\s*\[([^\]]*)\]",
                    m => "effects [" + (m.Groups[1].Value.Trim().Length == 0 ? "preview" : m.Groups[1].Value.Trim() + ", preview") + "]")));
        }
        for (int i = 0; i < lines.Length; i++)
        {
            Match m = Regex.Match(CodeOf(lines[i]).TrimEnd(), @"^(\s*let\s+[a-z_][a-z0-9_]*\s*=\s*)((?:sweep|twist|recipe)\(.*\))\s*;$");
            if (m.Success)
            {
                string head = m.Groups[1].Value, e = m.Groups[2].Value;
                fixes.Add(new QuickFix(FixKind.Invert, i + 1, "invert", head + "inverse(" + e + ");"));
                fixes.Add(new QuickFix(FixKind.Mirror, i + 1, "mirror", head + "mirror(" + e + ");"));
                fixes.Add(new QuickFix(FixKind.Conjugate, i + 1, "conjugate", head + "conjugate(setup, " + e + ");"));
                fixes.Add(new QuickFix(FixKind.Repeat, i + 1, "repeat", head + "repeat(" + e + ", 2);"));
            }
            if (CodeOf(lines[i]).Contains("ring_cells(ring)")) fixes.Add(new QuickFix(FixKind.Rescope, i + 1, "re-scope to the selection", lines[i].Replace("ring_cells(ring)", "selection()")));
        }
        return fixes.OrderBy(f => f.Line).ThenBy(f => f.Kind).ToArray();
    }

    public static string Apply(string source, QuickFix fix)
    {
        string[] lines = source.Split('\n');
        if (fix.Line < 1 || fix.Line > lines.Length) return source;
        lines[fix.Line - 1] = fix.NewLine;
        return string.Join("\n", lines);
    }

    // The same program with one sample fault of each severity: a statement without its end and a preview outside
    // its effects (errors), a value nothing reads (warning) and an open note (info).
    public static string Faults(string source)
    {
        List<string> lines = source.Split('\n').ToList();
        int ret = lines.FindIndex(l => l.TrimStart().StartsWith("return ", StringComparison.Ordinal) && l.TrimEnd().EndsWith(';'));
        if (ret >= 0) lines[ret] = lines[ret].TrimEnd().TrimEnd(';');
        int contract = lines.FindIndex(l => l.StartsWith("export ", StringComparison.Ordinal) && l.Contains("effects [preview]"));
        if (contract >= 0) lines[contract] = lines[contract].Replace("effects [preview]", "effects []");
        int first = lines.FindIndex(l => l.StartsWith("export ", StringComparison.Ordinal));
        if (first >= 0) lines.Insert(first + 1, "    let spare = 0;");
        lines.Insert(Math.Min(1, lines.Count), "# todo: check the count");
        return string.Join("\n", lines);
    }

    // The lines of the fn or command that starts on the line: from its head to its closing brace.
    public static (int First, int Last) Region(string source, int line)
    {
        string[] lines = source.Split('\n'); int depth = 0; bool opened = false;
        for (int i = Math.Max(0, line - 1); i < lines.Length; i++)
        {
            foreach (char c in CodeOf(lines[i]))
            {
                if (c == '{') { depth++; opened = true; }
                else if (c == '}') depth--;
            }
            if (opened && depth <= 0) return (line, i + 1);
        }
        return (line, line);
    }

    // The places of the interface a program mounts, read from its calls.
    public static IReadOnlyList<MountKind> Mounts(string source)
    {
        var kinds = new List<MountKind>();
        if (SixCs.Entries("p", source).Count > 0) kinds.Add(MountKind.Palette);
        foreach ((string word, MountKind kind) in new[] { ("context", MountKind.Context), ("inspector", MountKind.Inspector), ("panel", MountKind.Panel), ("view", MountKind.View), ("annotate", MountKind.Overlay) })
            if (source.Split('\n').Any(l => Regex.IsMatch(CodeOf(l), @"\b" + word + @"\("))) kinds.Add(kind);
        return kinds;
    }

    // A typed reference to something dragged into the editor.
    public static string Reference(string kind, string id) => kind switch
    {
        "cell" => $"cell({id})", "history" => $"history(\"{id}\")", "fact" => $"fact(\"{id}\")", "orbit" => $"orbit(\"{id}\")",
        _ => throw J.Error(kind, "unknown reference kind"),
    };

    // The words a typed line may start with, for the console prompt.
    public static IReadOnlyList<string> LineCompletions(string prefix, IEnumerable<string> words) =>
        prefix.Length == 0 ? Array.Empty<string>() : words.Where(w => w.StartsWith(prefix, StringComparison.Ordinal) && w != prefix).OrderBy(w => w, StringComparer.Ordinal).Take(Offered).ToArray();
}

// One colour and one shape for each script output: its gutter mark, its result and its marks in the views agree.
public static class OutputMarks
{
    public static readonly IReadOnlyList<string> Colours = Array.AsReadOnly(new[] { "state.preview", "accent.strong", "severity.info", "severity.ok", "severity.warning", "state.selected" });
    public static readonly IReadOnlyList<string> Shapes = Array.AsReadOnly(new[] { "dot", "square", "triangle", "diamond" });
    public static (string Colour, string Shape) For(int index) => (Colours[index % Colours.Count], Shapes[index % Shapes.Count]);
}

public enum RunState { Queued, Running, Done, Failed, Aborted }
public sealed record RunItem(int Id, string Command, string Program, string Entry, ScriptKind Kind, int First, int Last, RunState State, double Progress);

// The visible run queue (ADR-D01.58): runs wait in order, one runs at a time, and Abort ends them all.
public sealed class RunQueue
{
    public const int Kept = 12;
    public const double Cost = .6;
    private readonly List<RunItem> items = new();
    private int next = 1;
    public IReadOnlyList<RunItem> Items => items;
    public IEnumerable<RunItem> Active => items.Where(i => i.State is RunState.Queued or RunState.Running);
    public event Action? Changed;
    public event Action<RunItem>? Finished;

    public RunItem Enqueue(ScriptEntry entry, int first, int last)
    {
        var item = new RunItem(next++, entry.Command, entry.Program, entry.Name, entry.Kind, first, last, RunState.Queued, 0);
        items.Add(item);
        while (items.Count > Kept && items[0].State is not (RunState.Queued or RunState.Running)) items.RemoveAt(0);
        Safe.Raise(Changed);
        return item;
    }

    // Time passes: the first waiting run starts, the running one moves on and ends.
    public void Advance(double seconds)
    {
        int at = items.FindIndex(i => i.State == RunState.Running);
        if (at < 0) { at = items.FindIndex(i => i.State == RunState.Queued); if (at < 0) return; items[at] = items[at] with { State = RunState.Running }; }
        double progress = items[at].Progress + seconds / Cost;
        if (progress < 1) { items[at] = items[at] with { Progress = progress }; Safe.Raise(Changed); return; }
        RunItem done = items[at] = items[at] with { Progress = 1, State = RunState.Done };
        Safe.Raise(Changed); Safe.Raise(Finished, done);
    }

    public int Abort(Func<RunItem, bool>? which = null)
    {
        int count = 0;
        for (int i = 0; i < items.Count; i++)
            if (items[i].State is RunState.Queued or RunState.Running && (which == null || which(items[i]))) { items[i] = items[i] with { State = RunState.Aborted }; count++; }
        if (count > 0) Safe.Raise(Changed);
        return count;
    }

    public void Clear() { items.Clear(); Safe.Raise(Changed); }
}

// A read result, shown under its region. A commit or an undo after it dims it and names what came between.
public sealed record ScriptResult(string Command, string Program, string Entry, int Line, string Text, string? Since, bool Collapsed, int Mark);

public sealed class ResultStore
{
    private readonly List<ScriptResult> results = new();
    private int marks;
    public IReadOnlyList<ScriptResult> Results => results;
    public event Action? Changed;

    public ScriptResult? Of(string command) => results.FirstOrDefault(r => r.Command == command);

    public ScriptResult Set(ScriptEntry entry, string text)
    {
        int at = results.FindIndex(r => r.Command == entry.Command);
        var result = new ScriptResult(entry.Command, entry.Program, entry.Name, entry.Line, text, null, at >= 0 && results[at].Collapsed, at >= 0 ? results[at].Mark : marks++);
        if (at >= 0) results[at] = result; else results.Add(result);
        Safe.Raise(Changed);
        return result;
    }

    // A commit or an undo: every result computed before it is stale and says since what.
    public int StateChanged(string name)
    {
        int count = 0;
        for (int i = 0; i < results.Count; i++) if (results[i].Since == null) { results[i] = results[i] with { Since = name }; count++; }
        if (count > 0) Safe.Raise(Changed);
        return count;
    }

    public bool Toggle(string command)
    {
        int at = results.FindIndex(r => r.Command == command);
        if (at < 0) return false;
        results[at] = results[at] with { Collapsed = !results[at].Collapsed }; Safe.Raise(Changed);
        return true;
    }

    // The entries of a program moved or went: results follow their entry and leave with it.
    public void Follow(string program, IReadOnlyList<ScriptEntry> entries)
    {
        for (int i = results.Count - 1; i >= 0; i--)
        {
            if (results[i].Program != program) continue;
            ScriptEntry? entry = entries.FirstOrDefault(e => e.Command == results[i].Command);
            if (entry == null) results.RemoveAt(i); else results[i] = results[i] with { Line = entry.Line };
        }
    }

    public void Clear() { results.Clear(); Safe.Raise(Changed); }
}

// A run and the commit never share a key in one context (ADR-D01.58).
public static class KeyRules
{
    public const string Commit = "operation.preview.commit";
    public static bool Runs(string command) => command.StartsWith("script.", StringComparison.Ordinal);

    public static IReadOnlyList<string> RunCommitClashes(Keymap keymap)
    {
        Binding[] commits = keymap.Bindings.Where(b => b.Command == Commit).ToArray();
        return keymap.Bindings.Where(b => Runs(b.Command) && commits.Any(c => c.Key == b.Key && (c.Context == b.Context || c.Context == Keymap.Global || b.Context == Keymap.Global)))
            .Select(b => b.Key + " in " + b.Context).Distinct().ToArray();
    }

    // The command a key already has in the context or in the global one, or null: a conflict is shown at once.
    public static Binding? Conflict(Keymap keymap, string key, string context, string command) =>
        keymap.Bindings.FirstOrDefault(b => b.Key == key && b.Command != command && (b.Context == context || b.Context == Keymap.Global || context == Keymap.Global));
}

// A package by someone else (ADR-D01.32): a local file with its digest and the capabilities it asks for.
public sealed record PackageManifest(string Id, string Title, string Author, string Digest, IReadOnlyList<string> Capabilities, string File, string Source)
{
    public static readonly IReadOnlyDictionary<string, string> Known = new SortedDictionary<string, string>(StringComparer.Ordinal)
    {
        ["canvas"] = "draws diagrams", ["panel"] = "draws panels", ["preview"] = "prepares previews", ["read"] = "reads the puzzle state", ["view"] = "changes views and layout",
    };
    public static string DigestOf(string source) => "sha256:" + Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(source))).ToLowerInvariant();
    public bool Verified => Digest == DigestOf(Source);

    public static PackageManifest Parse(string text)
    {
        using JsonDocument doc = JsonDocument.Parse(text); JsonElement root = doc.RootElement;
        string Text(string key) => root.ValueKind == JsonValueKind.Object && root.TryGetProperty(key, out JsonElement v) && v.ValueKind == JsonValueKind.String ? v.GetString()! : throw J.Error(key, "expected a text");
        if (Text("format") != "magic600-script-package") throw J.Error("format", "not a script package");
        if (!root.TryGetProperty("capabilities", out JsonElement caps) || caps.ValueKind != JsonValueKind.Array) throw J.Error("capabilities", "expected a list");
        string[] asked = caps.EnumerateArray().Select(c => c.GetString() ?? "").ToArray();
        foreach (string c in asked) if (!Known.ContainsKey(c)) throw J.Error(c, "unknown capability");
        return new PackageManifest(Text("id"), Text("title"), Text("author"), Text("digest"), asked, Text("file"), Text("source").Replace("\r\n", "\n"));
    }
}

public sealed class PackageStore
{
    private readonly SortedSet<string> installed = new(StringComparer.Ordinal);
    public IReadOnlyCollection<string> Installed => installed;
    public event Action? Changed;
    // A package installs only with a digest that matches its source.
    public bool Install(PackageManifest package) { if (!package.Verified || !installed.Add(package.Id)) return false; Safe.Raise(Changed); return true; }
    public bool Revoke(string id) { if (!installed.Remove(id)) return false; Safe.Raise(Changed); return true; }
    // In a solving session a package's tool words are refused when its preview is prepared.
    public static string? PreviewRefusal(PackageManifest package, ScriptKind kind, bool solving) =>
        solving && kind == ScriptKind.Preview ? $"{package.Title}: a package prepares no preview in a solving session" : null;
    public string Store() => string.Join(",", installed);
    public void Restore(string stored) { installed.Clear(); foreach (string id in stored.Split(',', StringSplitOptions.RemoveEmptyEntries)) installed.Add(id); Safe.Raise(Changed); }
}

// The six places a program can mount a part of the interface (ADR-D01.29), each with its own switch.
public enum MountKind { Palette, Context, Inspector, Panel, View, Overlay }

public sealed class MountSet
{
    private readonly SortedSet<string> off = new(StringComparer.Ordinal);
    public event Action? Changed;
    public bool IsOn(string program, MountKind kind) => !off.Contains(program + ":" + kind);
    public void Set(string program, MountKind kind, bool on) { if (on ? off.Remove(program + ":" + kind) : off.Add(program + ":" + kind)) Safe.Raise(Changed); }
    public string Store() => string.Join(",", off);
    public void Restore(string stored) { off.Clear(); foreach (string id in stored.Split(',', StringSplitOptions.RemoveEmptyEntries)) off.Add(id); Safe.Raise(Changed); }
}

// The host's widgets (ADR-D01.30): a program declares them, the host draws them. None of them commits.
public abstract record PanelWidget(string Id, string Label);
public sealed record NumberWidget(string Id, string Label, double Min, double Max, double Step, double Value) : PanelWidget(Id, Label);
public sealed record ChoiceWidget(string Id, string Label, IReadOnlyList<string> Options, int Chosen) : PanelWidget(Id, Label);
public sealed record SwitchWidget(string Id, string Label, bool On) : PanelWidget(Id, Label);
public sealed record ReferenceWidget(string Id, string Label, string Kind, string Value) : PanelWidget(Id, Label);
public sealed record TokenWidget(string Id, string Label, IReadOnlyList<string> Tokens, string Chosen) : PanelWidget(Id, Label);
public sealed record TableWidget(string Id, string Label, IReadOnlyList<string> Columns, IReadOnlyList<IReadOnlyList<string>> Rows) : PanelWidget(Id, Label);
public sealed record TreeWidget(string Id, string Label, IReadOnlyList<(int Depth, string Text, string Value)> Nodes) : PanelWidget(Id, Label);
public sealed record ChartWidget(string Id, string Label, string Form, IReadOnlyList<(string Label, double Value)> Points, string Colour = "accent.strong") : PanelWidget(Id, Label)
{
    public static readonly IReadOnlyList<string> Forms = Array.AsReadOnly(new[] { "bars", "line", "heat" });
}
public sealed record CanvasWidget(string Id, string Label, IReadOnlyList<CanvasMark> Marks) : PanelWidget(Id, Label);

// The rows of the console (ADR-D01.57), typed by what they are: an echoed action, a typed line, a result, an
// error, and the refusal of a line that would commit.
public enum ConsoleKind { Echo, Typed, Result, Error, Lock }

public static class ConsoleRows
{
    private static readonly (string Prefix, ConsoleKind Kind)[] Prefixes = { ("> ", ConsoleKind.Typed), ("= ", ConsoleKind.Result), ("! ", ConsoleKind.Error), ("~ ", ConsoleKind.Lock) };
    public static string Write(ConsoleKind kind, string text) => Prefixes.FirstOrDefault(p => p.Kind == kind).Prefix + text;
    public static (ConsoleKind Kind, string Text) Read(string line)
    {
        foreach ((string prefix, ConsoleKind kind) in Prefixes) if (line.StartsWith(prefix, StringComparison.Ordinal)) return (kind, line.Substring(prefix.Length));
        return (ConsoleKind.Echo, line);
    }
}

// The slots for script panels (ADR-D01.32): two docked slots, then tabs inside them. Ids are stable, so a
// panel whose program is missing keeps its place.
public sealed class SlotTabs
{
    public const int Count = 2;
    private readonly List<string>[] tabs = { new(), new() };
    private readonly int[] active = new int[Count];
    public event Action? Changed;
    public IReadOnlyList<string> Tabs(int slot) => tabs[slot];
    public string? Shown(int slot) => tabs[slot].Count == 0 ? null : tabs[slot][Math.Clamp(active[slot], 0, tabs[slot].Count - 1)];
    public int SlotOf(string panel) => Array.FindIndex(tabs, t => t.Contains(panel));

    public int Mount(string panel)
    {
        int at = SlotOf(panel);
        if (at < 0)
        {
            at = Array.FindIndex(tabs, t => t.Count == 0);
            if (at < 0) at = tabs[0].Count <= tabs[1].Count ? 0 : 1;
            tabs[at].Add(panel);
        }
        if (Shown(at) == panel && tabs[at].Count > 0 && active[at] == tabs[at].IndexOf(panel)) { Safe.Raise(Changed); return at; }
        active[at] = tabs[at].IndexOf(panel); Safe.Raise(Changed);
        return at;
    }

    public bool Unmount(string panel)
    {
        int at = SlotOf(panel);
        if (at < 0) return false;
        tabs[at].Remove(panel); active[at] = Math.Clamp(active[at], 0, Math.Max(0, tabs[at].Count - 1)); Safe.Raise(Changed);
        return true;
    }

    public string Store() => string.Join("|", tabs.Select((t, i) => string.Join(",", t) + ":" + active[i]));

    public void Restore(string stored)
    {
        string[] parts = stored.Split('|');
        if (parts.Length != Count) return;
        for (int i = 0; i < Count; i++)
        {
            int cut = parts[i].LastIndexOf(':');
            if (cut < 0) return;
            tabs[i].Clear(); tabs[i].AddRange(parts[i].Substring(0, cut).Split(',', StringSplitOptions.RemoveEmptyEntries));
            active[i] = int.TryParse(parts[i].Substring(cut + 1), out int a) ? a : 0;
        }
        Safe.Raise(Changed);
    }
}
