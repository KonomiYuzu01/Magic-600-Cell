using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;
using Godot;

namespace Magic600.Shell.Editor;

/// <summary>
/// LP4-3's greybox editor panel (a draft): the write-check-run flow of a .6cs macro in the product shell. The shell
/// creates it when M600_EDITOR is 1 (launch.py --editor); it loads the editor library from M600_EDITOR_DLL and, when
/// M600_RUNTIME names the language runtime's binary, runs the macro there (route RT1), otherwise in the library's own
/// process. The look is a placeholder; it stays with the design line (ADR-D01's later rounds).
///
/// Write: a code field (Godot's CodeEdit) whose text goes to the library after each change; the diagnostics list shows
/// every diagnostic of EV1's diagnose with its code, line and column, and selecting one moves the caret there. Code
/// completion (Ctrl+Space, or as the solver types) asks the library's complete at the caret.
///
/// Check and activate: the panel's palette list starts with two entries of its own, "Check" and "Activate", which
/// have no default key (EV1.13; the LP4 plan's integrator default). Check checks the buffer with its first exported
/// function or command as the entry and creates a candidate; it activates nothing. Activate is the explicit switch
/// to the last checked candidate. After the first activation of an entry, the host registers it as a language overlay
/// row, tool.module.entry, with the key <see cref="Key"/> for the first such row, and the list shows the session's
/// language rows from the host's palette.
///
/// Run: invoking a language row (the list's Invoke button, an activated list item, or its key through the host's
/// router) queues a program job in the host's command layer. Each frame the panel takes the jobs from the host, runs
/// them through the library on its worker thread, and reports each run's end to the host as the job's result or
/// error. A run holds no token and the panel commits nothing: a word the macro returns becomes the editor's draft,
/// which the solver previews with operation.preview's key (Ctrl+Enter) and commits with the commit key
/// (Ctrl+Shift+Enter, the key route's token) once a frame showed the preview, both outside the code field, whose text
/// focus keeps a commit row's key (DT-M1 C4). Undo is session.undo's key (Ctrl+KeyZ).
/// </summary>
public sealed class EditorPanel : IShellModule
{
    /// <summary>The key the first registered row gets (the checked key map's form, LP4-8).</summary>
    public const string Key = "Ctrl+Shift+KeyR";
    /// <summary>The panel's own palette entries: no row of the command table and no default key.</summary>
    public const string CheckEntry = "editor.check";
    public const string ActivateEntry = "editor.activate";

    private readonly EditorLibrary _library;
    private IntPtr _editor;
    private readonly CodeEdit _code;
    private readonly ItemList _diagnostics;
    private readonly ItemList _palette;
    private readonly Label _status;
    private readonly PanelContainer _panel;
    private bool _dirty = true;
    private bool _focused;
    private string? _entry;
    private Checked? _checked;
    private readonly HashSet<string> _registered = new();
    private readonly List<(ulong Job, ulong Ticket)> _runs = new();
    private readonly List<(int Line, int Column)> _places = new();

    /// <summary>What the last successful check answered: the candidate the Activate entry switches to.</summary>
    private sealed record Checked(string Module, string Entry, string Candidate, string Reference, string? Active);

    public EditorPanel(ShellContext context)
    {
        Context = context;
        var dll = System.Environment.GetEnvironmentVariable("M600_EDITOR_DLL");
        if (string.IsNullOrEmpty(dll))
        {
            throw new ArgumentException("the editor panel needs M600_EDITOR_DLL");
        }
        _library = EditorLibrary.Load(dll);
        try
        {
            if (_library.Create(EditorLibrary.UnitsScalar, out _editor) != EditorLibrary.Ok)
            {
                throw new InvalidOperationException("m6e_create failed");
            }
            var runtime = System.Environment.GetEnvironmentVariable("M600_RUNTIME");
            if (!string.IsNullOrEmpty(runtime))
            {
                Require("m6e_load_runtime", _library.LoadRuntime(_editor, runtime));
            }
        }
        catch
        {
            Dispose();
            throw;
        }
        _panel = new PanelContainer { Position = new Vector2(16, 16), Size = new Vector2(720, 860) };
        var column = new VBoxContainer();
        _panel.AddChild(column);
        column.AddChild(new Label { Text = "Macro (.6cs)" });
        _code = new CodeEdit
        {
            CustomMinimumSize = new Vector2(700, 360),
            CodeCompletionEnabled = true,
            Text = "edition \"6cs-draft-1\";\nmodule mine requires [];\n\nexport command shown() -> Word effects [] {\n"
                + "    return word[];\n}\n",
        };
        _code.TextChanged += () => _dirty = true;
        _code.CodeCompletionRequested += Complete;
        _code.FocusEntered += () => Focus(true);
        _code.FocusExited += () => Focus(false);
        column.AddChild(_code);
        column.AddChild(new Label { Text = "Diagnostics" });
        _diagnostics = new ItemList { CustomMinimumSize = new Vector2(700, 140) };
        _diagnostics.ItemSelected += index => GoTo((int)index);
        column.AddChild(_diagnostics);
        column.AddChild(new Label { Text = "Palette" });
        _palette = new ItemList { CustomMinimumSize = new Vector2(700, 160) };
        _palette.ItemActivated += index => Invoke((int)index);
        column.AddChild(_palette);
        var invoke = new Button { Text = "Invoke" };
        invoke.Pressed += () =>
        {
            if (_palette.GetSelectedItems() is [var index, ..])
            {
                Invoke(index);
            }
        };
        column.AddChild(invoke);
        _status = new Label { CustomMinimumSize = new Vector2(700, 80), AutowrapMode = TextServer.AutowrapMode.WordSmart };
        column.AddChild(_status);
        Context.Root.AddChild(_panel);
        RefreshPalette();
    }

    /// <summary>The window, the root control, the log and the host the shell gave this panel.</summary>
    public ShellContext Context { get; }

    /// <summary>Whether the code field has the keyboard focus: the shell then leaves the keys to it.</summary>
    public bool TextFocused => _focused;

    private void Require(string name, int code)
    {
        if (code != EditorLibrary.Ok)
        {
            throw new InvalidOperationException($"{name} answered {code}: {_library.LastError(_editor)}");
        }
    }

    private void Say(string text) => _status.Text = text;

    /// <summary>The refusal's code and account, from the library's {"code", "detail"} or a plain reason.</summary>
    private string Refusal(string error)
    {
        try
        {
            using var json = JsonDocument.Parse(error);
            return $"{json.RootElement.GetProperty("code").GetString()}: {json.RootElement.GetProperty("detail").GetString()}";
        }
        catch (Exception)
        {
            return error;
        }
    }

    private void Focus(bool text)
    {
        _focused = text;
        Context.Host.SetFocus(Context.HostHandle, text ? HostLibrary.FocusText : HostLibrary.FocusPuzzle);
    }

    /// <summary>The caret as a scalar offset of the whole text (the library's declared units).</summary>
    private ulong Caret()
    {
        var line = _code.GetCaretLine();
        ulong offset = 0;
        for (var i = 0; i < line; i++)
        {
            offset += (ulong)_code.GetLine(i).EnumerateRunes().Count() + 1;
        }
        // Godot's columns count scalars, as the library's units do.
        var scalars = _code.GetLine(line).EnumerateRunes().Count();
        return offset + (ulong)Math.Min(_code.GetCaretColumn(), scalars);
    }

    private void Complete()
    {
        Sync();
        if (_library.Complete(_editor, Caret()) != EditorLibrary.Ok)
        {
            return;
        }
        using var answer = JsonDocument.Parse(_library.Answer(_editor));
        foreach (var candidate in answer.RootElement.GetProperty("candidates").EnumerateArray())
        {
            var label = candidate.GetProperty("label").GetString() ?? string.Empty;
            _code.AddCodeCompletionOption(CodeEdit.CodeCompletionKind.PlainText, label, label);
        }
        _code.UpdateCodeCompletionOptions(false);
    }

    /// <summary>Hands the code field's text to the library and diagnoses it, when it changed.</summary>
    private void Sync()
    {
        if (!_dirty)
        {
            return;
        }
        _dirty = false;
        Require("m6e_set_text", _library.SetText(_editor, _code.Text));
        Require("m6e_diagnose", _library.Diagnose(_editor));
        using var answer = JsonDocument.Parse(_library.Answer(_editor));
        var root = answer.RootElement;
        _diagnostics.Clear();
        _places.Clear();
        foreach (var diagnostic in root.GetProperty("diagnostics").EnumerateArray())
        {
            var at = diagnostic.GetProperty("at");
            var (line, column) = at.ValueKind == JsonValueKind.Object
                ? (at.GetProperty("line").GetInt32(), at.GetProperty("column").GetInt32())
                : (0, 0);
            _places.Add((line, column));
            _diagnostics.AddItem(
                $"{line}:{column}  {diagnostic.GetProperty("code").GetString()}  {diagnostic.GetProperty("summary").GetString()}");
        }
        _entry = root.GetProperty("exports").EnumerateArray().Select(e => e.GetString()).FirstOrDefault();
        var verdict = root.GetProperty("verdict").GetString();
        Say(verdict == "checked"
            ? $"no diagnostics; Check makes {root.GetProperty("module").GetString()}.{_entry} a candidate"
            : $"{verdict}: {_diagnostics.ItemCount} diagnostics");
    }

    private void GoTo(int index)
    {
        if (index < 0 || index >= _places.Count)
        {
            return;
        }
        var (line, column) = _places[index];
        if (line <= 0 || column <= 0)
        {
            return;
        }
        _code.SetCaretLine(line - 1);
        _code.SetCaretColumn(column - 1);
        _code.GrabFocus();
    }

    /// <summary>The panel's own entries, then the session's language rows from the host's palette.</summary>
    private void RefreshPalette()
    {
        _palette.Clear();
        _palette.SetItemMetadata(_palette.AddItem("Check the macro"), CheckEntry);
        _palette.SetItemMetadata(_palette.AddItem("Activate the checked macro"), ActivateEntry);
        if (Context.Host.Palette(Context.HostHandle) != HostLibrary.Ok)
        {
            return;
        }
        using var palette = JsonDocument.Parse(Context.Host.Answer(Context.HostHandle));
        foreach (var entry in palette.RootElement.GetProperty("entries").EnumerateArray())
        {
            if (!entry.GetProperty("language_defined").GetBoolean())
            {
                continue;
            }
            var keys = string.Join(", ", entry.GetProperty("keys").EnumerateArray().Select(k => k.GetString()));
            var label = entry.GetProperty("label").GetString();
            var row = entry.GetProperty("row").GetString() ?? string.Empty;
            var text = keys.Length == 0 ? $"{label}  ({row})" : $"{label}  ({row}; {keys})";
            _palette.SetItemMetadata(_palette.AddItem(text), row);
        }
    }

    private void Invoke(int index)
    {
        if (index < 0 || index >= _palette.ItemCount)
        {
            return;
        }
        var row = _palette.GetItemMetadata(index).AsString();
        switch (row)
        {
            case CheckEntry:
                Check();
                return;
            case ActivateEntry:
                Activate();
                return;
        }
        var code = Context.Host.Invoke(Context.HostHandle, row, string.Empty, out var job);
        if (code != HostLibrary.Ok)
        {
            Say($"{row} was not invoked: {Context.Host.LastError(Context.HostHandle)}");
            return;
        }
        Say(job == 0 ? $"{row}: {Context.Host.Answer(Context.HostHandle)}" : $"{row}: job {job} is running");
    }

    private void Check()
    {
        Sync();
        if (_entry is not { } entry)
        {
            Say("the macro exports no function or command to check");
            return;
        }
        if (_library.Check(_editor, entry) != EditorLibrary.Ok)
        {
            _checked = null;
            Say($"not checked: {Refusal(_library.LastError(_editor))}");
            return;
        }
        using var answer = JsonDocument.Parse(_library.Answer(_editor));
        var root = answer.RootElement;
        var active = root.GetProperty("active");
        _checked = new Checked(root.GetProperty("module").GetString()!, root.GetProperty("entry").GetString()!,
            root.GetProperty("candidate").GetString()!, root.GetProperty("reference").GetString()!,
            active.ValueKind == JsonValueKind.String ? active.GetString() : null);
        Say($"checked: {_checked.Module}.{_checked.Entry} is a candidate ({_checked.Candidate[..12]}); "
            + "Activate makes it the active version");
    }

    private void Activate()
    {
        if (_checked is not { } candidate)
        {
            Say("nothing to activate: Check the macro first");
            return;
        }
        var expected = candidate.Active is { } active ? Convert.FromHexString(active) : Array.Empty<byte>();
        if (_library.Activate(_editor, candidate.Module, expected, Convert.FromHexString(candidate.Candidate),
                Convert.FromHexString(candidate.Reference)) != EditorLibrary.Ok)
        {
            Say($"not activated: {Refusal(_library.LastError(_editor))}");
            return;
        }
        _checked = candidate with { Active = candidate.Candidate };
        var row = $"tool.{candidate.Module}.{candidate.Entry}";
        if (_registered.Add(row))
        {
            var key = _registered.Count == 1 ? Key : string.Empty;
            var code = Context.Host.RegisterRow(Context.HostHandle, candidate.Module, candidate.Entry,
                $"{candidate.Module}.{candidate.Entry}", key);
            if (code != HostLibrary.Ok)
            {
                _registered.Remove(row);
                Say($"activated, but {row} is not in the palette: {Context.Host.LastError(Context.HostHandle)}");
                return;
            }
        }
        RefreshPalette();
        Say($"activated: {row} runs {candidate.Candidate[..12]}");
    }

    /// <summary>Takes the host's program jobs, runs them, and reports the runs that ended.</summary>
    private void Pump()
    {
        var host = Context.Host;
        while (host.TakeJob(Context.HostHandle, out var job) == HostLibrary.Ok && job != 0)
        {
            var text = host.Answer(Context.HostHandle);
            if (_library.Run(_editor, text, out var ticket) == EditorLibrary.Ok)
            {
                _runs.Add((job, ticket));
            }
            else
            {
                host.FinishJob(Context.HostHandle, job, string.Empty, "KN7:resource_limit.runtime_lost",
                    _library.LastError(_editor));
            }
        }
        for (var i = _runs.Count - 1; i >= 0; i--)
        {
            var (job, ticket) = _runs[i];
            if (_library.RunPoll(_editor, ticket, out var state) != EditorLibrary.Ok || state == EditorLibrary.RunPending)
            {
                continue;
            }
            _runs.RemoveAt(i);
            using var record = JsonDocument.Parse(_library.Answer(_editor));
            var root = record.RootElement;
            if (state == EditorLibrary.RunCompleted)
            {
                host.FinishJob(Context.HostHandle, job, root.GetProperty("output").GetString() ?? string.Empty,
                    string.Empty, string.Empty);
                Shown(root);
            }
            else
            {
                var code = root.GetProperty("code").GetString() ?? string.Empty;
                var detail = root.GetProperty("detail").GetString() ?? string.Empty;
                host.FinishJob(Context.HostHandle, job, string.Empty, code, detail);
                Say($"job {job} ended: {code}: {detail}");
            }
        }
    }

    /// <summary>A completed run: its word becomes the draft, which only the solver's preview key previews.</summary>
    private void Shown(JsonElement run)
    {
        var word = run.GetProperty("word");
        if (word.ValueKind != JsonValueKind.Array)
        {
            Say($"the macro returned {run.GetProperty("output").GetString()?.Trim()}");
            return;
        }
        var letters = word.EnumerateArray().Select(l => l.GetInt32()).ToArray();
        Context.Host.SetDraftWord(Context.HostHandle, letters);
        var text = string.Join(", ", letters.Select(l => l.ToString("+0;-0", CultureInfo.InvariantCulture)));
        Say($"the macro returned word[{text}] ({run.GetProperty("host").GetString()}); it is the draft: "
            + "preview it with Ctrl+Enter, then commit it with Ctrl+Shift+Enter outside the code field");
    }

    public bool OnWindowMessage(uint message, nuint wParam, nint lParam, long qpc) => false;

    public void OnFrame(HostLibrary.Submitted frame, long qpc)
    {
        if (_editor == IntPtr.Zero)
        {
            return;
        }
        Sync();
        Pump();
    }

    public void OnPresented(HostLibrary.Presented row)
    {
    }

    public void Dispose()
    {
        if (_editor != IntPtr.Zero)
        {
            _library.Destroy(_editor);
            _editor = IntPtr.Zero;
        }
        _library.Dispose();
    }
}
