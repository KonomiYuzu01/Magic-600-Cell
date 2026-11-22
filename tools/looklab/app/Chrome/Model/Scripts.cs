using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;

namespace Magic600.Chrome;

// What the chrome shows of 600 Cell Script. The language services belong to the Rust host (ADR-D01.57); the
// chrome only needs the words to colour, the entry points a program registers and where a diagnostic sits.
public enum ScriptKind { Read, Preview, View }

public sealed record ScriptEntry(string Program, string Name, ScriptKind Kind, int Line)
{
    public string Command => "script." + Program + "." + Name;
}

public sealed record ScriptDiagnostic(Severity Severity, int Line, string Text);

public sealed record ScriptProgram(string Id, string File, string Source)
{
    public IReadOnlyList<ScriptEntry> Entries => SixCs.Entries(Id, Source);
}

public static class SixCs
{
    public static readonly IReadOnlyList<string> Keywords = Array.AsReadOnly(new[]
    {
        "edition", "export", "fn", "command", "let", "return", "record", "effects", "if", "else", "for", "in", "match", "macro", "panel", "stage", "use", "true", "false",
    });
    public static readonly IReadOnlyList<string> Types = Array.AsReadOnly(new[] { "Nat", "Int", "Bool", "Text", "Cell", "Sticker", "Orbit", "Twist", "Recipe", "List", "Set", "Approx", "Preview", "View" });
    private static readonly Regex Export = new(@"^\s*export\s+(?:fn|command)\s+([a-z_][a-z0-9_]*)\s*\(.*\beffects\s*\[([^\]]*)\]", RegexOptions.Compiled);

    // An exported fn or command is an entry point; its declared effects say what a run may end in.
    public static IReadOnlyList<ScriptEntry> Entries(string program, string source)
    {
        var entries = new List<ScriptEntry>(); string[] lines = source.Split('\n');
        for (int i = 0; i < lines.Length; i++)
        {
            Match m = Export.Match(lines[i]);
            if (!m.Success) continue;
            string[] effects = m.Groups[2].Value.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
            ScriptKind kind = effects.Contains("preview") ? ScriptKind.Preview : effects.Contains("view") || effects.Contains("workspace") ? ScriptKind.View : ScriptKind.Read;
            entries.Add(new ScriptEntry(program, m.Groups[1].Value, kind, i + 1));
        }
        return entries;
    }

    // The checks the chrome can make without the host: brackets, strings, the edition line and the commit
    // boundary. A program never commits (ADR-D01.4), so the word is an error wherever it is written as an effect.
    public static IReadOnlyList<ScriptDiagnostic> Diagnose(string source)
    {
        var found = new List<ScriptDiagnostic>(); string[] lines = source.Split('\n');
        var open = new Stack<(char Bracket, int Line)>(); bool edition = false;
        for (int i = 0; i < lines.Length; i++)
        {
            string line = lines[i]; int comment = CommentStart(line);
            string code = comment >= 0 ? line.Substring(0, comment) : line;
            if (code.TrimStart().StartsWith("edition ", StringComparison.Ordinal)) edition = true;
            bool text = false;
            foreach (char c in code)
            {
                if (c == '"') text = !text;
                if (text) continue;
                if (c is '{' or '(' or '[') open.Push((c, i + 1));
                else if (c is '}' or ')' or ']')
                {
                    char wanted = c == '}' ? '{' : c == ')' ? '(' : '[';
                    if (open.Count == 0 || open.Peek().Bracket != wanted) found.Add(new ScriptDiagnostic(Severity.Error, i + 1, $"`{c}` closes nothing"));
                    else open.Pop();
                }
            }
            if (text) found.Add(new ScriptDiagnostic(Severity.Error, i + 1, "the text is not closed on this line"));
            Match effects = Regex.Match(code, @"\beffects\s*\[([^\]]*)\]");
            if (effects.Success && effects.Groups[1].Value.Split(',', StringSplitOptions.TrimEntries).Contains("commit"))
                found.Add(new ScriptDiagnostic(Severity.Error, i + 1, "a program never commits: a run ends in a read result or a preview"));
            if (Regex.IsMatch(code, @"\blet\s+[a-z_][a-z0-9_]*\s*;")) found.Add(new ScriptDiagnostic(Severity.Warning, i + 1, "this `let` has no value"));
        }
        foreach ((char bracket, int line) in open.Reverse()) found.Add(new ScriptDiagnostic(Severity.Error, line, $"`{bracket}` is never closed"));
        if (!edition && source.Trim().Length > 0) found.Add(new ScriptDiagnostic(Severity.Info, 1, "no edition line; the session's edition is used"));
        return found.OrderBy(d => d.Line).ToArray();
    }

    // The index where a # comment starts outside a text, or -1.
    public static int CommentStart(string line)
    {
        bool text = false;
        for (int i = 0; i < line.Length; i++)
        {
            if (line[i] == '"') text = !text;
            else if (line[i] == '#' && !text) return i;
        }
        return -1;
    }
}

// The solver's own key bindings, kept apart from the default keymap and exported as a versioned keymap file
// (ADR-D01.3, ADR-D01.21). A binding may name a command row or a registered .6cs entry point.
public sealed class KeymapOverrides
{
    private readonly List<Binding> bindings = new();
    public IReadOnlyList<Binding> Bindings => bindings;

    public void Bind(string key, string context, string command, string? argument = null, string? label = null)
    {
        bindings.RemoveAll(b => b.Key == key && b.Context == context);
        bindings.Add(new Binding(key, context, command, argument, argument != null ? label ?? command : null));
    }

    public bool Unbind(string key, string context) => bindings.RemoveAll(b => b.Key == key && b.Context == context) > 0;
    public void Clear() => bindings.Clear();

    private static string Quote(string text) => "\"" + text.Replace("\\", "\\\\").Replace("\"", "\\\"") + "\"";

    // The override file: the same form as the default keymap, applied over it.
    public string ToJson()
    {
        var text = new StringBuilder("{\n  \"format\": \"magic600-design-keymap\",\n  \"version\": 1,\n  \"radials\": {},\n  \"bindings\": [");
        for (int i = 0; i < bindings.Count; i++)
        {
            Binding b = bindings[i];
            text.Append(i == 0 ? "\n" : ",\n").Append($"    {{\"key\": {Quote(b.Key)}, \"context\": {Quote(b.Context)}, \"command\": {Quote(b.Command)}");
            if (b.Argument != null) text.Append($", \"argument\": {Quote(b.Argument)}, \"label\": {Quote(b.Label ?? b.Command)}");
            text.Append('}');
        }
        return text.Append(bindings.Count == 0 ? "]\n}\n" : "\n  ]\n}\n").ToString();
    }

    // Reads an override file back; every binding must name a command the registry has.
    public static KeymapOverrides Parse(string text, CommandRegistry registry, Keymap defaults)
    {
        var own = new KeymapOverrides();
        Keymap all = Keymap.Parse(text, registry, defaults);
        foreach (Binding b in all.Bindings.Where(b => !defaults.Bindings.Contains(b))) own.bindings.Add(b);
        return own;
    }

    public Keymap Over(Keymap defaults, CommandRegistry registry) => Keymap.Parse(ToJson(), registry, defaults);
}

// The typed line of the console and of the palette's typed mode keeps its own history.
public sealed class LineHistory
{
    public const int Kept = 200;
    private readonly List<string> lines = new();
    private int cursor;
    public IReadOnlyList<string> Lines => lines;

    public void Add(string line)
    {
        if (string.IsNullOrWhiteSpace(line)) return;
        lines.Remove(line); lines.Add(line); if (lines.Count > Kept) lines.RemoveAt(0);
        cursor = lines.Count;
    }

    // One line back or forward; the empty line sits after the newest.
    public string Step(int by)
    {
        cursor = Math.Clamp(cursor + by, 0, lines.Count);
        return cursor < lines.Count ? lines[cursor] : "";
    }
}

// A panel a program declares (ADR-D01.30): the host draws it with design-system widgets. Rows are a small table;
// bars are a chart; both are display only.
public sealed record ScriptPanelSpec(string Id, string Title, string Program, IReadOnlyList<(string Label, string Value)> Rows, IReadOnlyList<(string Label, double Value)> Bars);

// One mark of the canvas primitive (ADR-D01.31): vector content in design tokens, display only.
public sealed record CanvasMark(string Shape, double X, double Y, double A, double B, string Colour, string? Text = null)
{
    public static readonly IReadOnlyList<string> Shapes = Array.AsReadOnly(new[] { "line", "circle", "dot", "text" });
}

// The two docked slots for script panels (ADR-D01.32) with stable ids: a panel whose program is missing keeps
// its slot and shows a placeholder.
public sealed class ScriptSlots
{
    public const int Count = 2;
    private readonly string?[] mounted = new string?[Count];
    public IReadOnlyList<string?> Mounted => mounted;
    public event Action? Changed;

    public int Mount(string panel)
    {
        int at = Array.IndexOf(mounted, panel);
        if (at < 0) at = Array.IndexOf(mounted, null);
        if (at < 0) return -1;
        if (mounted[at] != panel) { mounted[at] = panel; Safe.Raise(Changed); }
        return at;
    }

    public bool Unmount(string panel)
    {
        int at = Array.IndexOf(mounted, panel);
        if (at < 0) return false;
        mounted[at] = null; Safe.Raise(Changed);
        return true;
    }

    public string Store() => string.Join("|", mounted.Select(m => m ?? ""));

    public void Restore(string stored)
    {
        string[] parts = stored.Split('|');
        if (parts.Length != Count) return;
        for (int i = 0; i < Count; i++) mounted[i] = parts[i].Length == 0 ? null : parts[i];
        Safe.Raise(Changed);
    }
}
