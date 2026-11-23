using System;
using Godot;

namespace Magic600.Chrome;

// The Library dock (ADR-D01.69): one tree of the solver's records under an input box that filters in place. The
// keyboard works from the box: Up and Down move the selection, Enter adds the selected macro to the active phase
// (macro.add) and Shift+Enter previews it alone (operation.preview). A row leads with its kind's glyph and the
// effect signature. The panel shows the model and sends commands; it never changes the draft itself.
public sealed partial class LibraryPanel : VBoxContainer
{
    public LibraryModel Library { get; }
    public LineEdit Filter { get; } = new() { Name = "LibraryFilter", PlaceholderText = "Filter", ClearButtonEnabled = true, SizeFlagsHorizontal = SizeFlags.ExpandFill };
    public VBoxContainer List { get; } = new() { Name = "LibraryRows", SizeFlagsHorizontal = SizeFlags.ExpandFill };
    public event Action<string, string?>? Command;
    private readonly Func<TokenSet> tokens;

    public LibraryPanel(LibraryModel library, Func<TokenSet> tokens)
    {
        Library = library; this.tokens = tokens; Name = "LibraryPanel";
        Filter.TooltipText = "Filter the library in place. Up and Down select; Enter adds the selected macro to the active phase; Shift+Enter previews it alone.";
        Filter.TextChanged += library.SetFilter;
        Filter.GuiInput += Key;
        AddChild(Filter);
        var scroll = new ScrollContainer { SizeFlagsVertical = SizeFlags.ExpandFill, HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };
        List.AddThemeConstantOverride("separation", 0); scroll.AddChild(List); AddChild(scroll);
        library.Changed += Refresh; Refresh();
    }

    private void Send(string command, string? argument) => Safe.Raise(Command, command, argument);

    // The filter takes the keyboard: focused and in edit mode, so typing filters at once.
    public void Focus()
    {
        if (!Filter.IsInsideTree()) return;
        Filter.GrabFocus(); Filter.Edit();
    }

    // Enter and Shift+Enter send the selected record's name; with none selected the host says why.
    public void Key(InputEvent e)
    {
        if (e is not InputEventKey { Pressed: true } key || key.CtrlPressed || key.AltPressed || key.MetaPressed) return;
        if (key.Keycode is not (Godot.Key.Up or Godot.Key.Down or Godot.Key.Enter or Godot.Key.KpEnter)) return;
        // The box tells the model of an edit one frame late (text_changed is deferred): a key that acts on the rows
        // reads the text as typed, so a name typed and entered at once finds its row.
        if (Filter.Text != Library.Filter) Library.SetFilter(Filter.Text);
        string? name = Library.Current?.Name;
        switch (key.Keycode)
        {
            case Godot.Key.Up: Library.Move(-1); break;
            case Godot.Key.Down: Library.Move(1); break;
            case Godot.Key.Enter or Godot.Key.KpEnter when key.ShiftPressed: Send("operation.preview", name == null ? null : "macro:" + name); break;
            case Godot.Key.Enter or Godot.Key.KpEnter: Send("macro.add", name); break;
            default: return;
        }
        if (Filter.IsInsideTree()) Filter.AcceptEvent();
    }

    public void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        TokenSet t = tokens();
        // A filter set from elsewhere (the macro.filter command) shows in the box; the box is never written while
        // the solver is typing in it.
        if (Filter.Text != Library.Filter && !Filter.HasFocus()) Filter.Text = Library.Filter;
        SessionWidgets.Clear(List);
        foreach ((string title, _) in LibraryModel.Branches)
        {
            int count = Library.Matching(title).Count;
            if (count == 0) continue;
            bool open = Library.IsOpen(title); string branch = title;
            Button head = SessionWidgets.Tool(open ? "fold.open" : "fold.closed", open ? "Fold this branch" : "Open this branch", t, () => Library.Fold(branch), $"{title} {count}");
            head.Alignment = HorizontalAlignment.Left; head.SetMeta("library_branch", title); List.AddChild(head);
            if (!open) continue;
            foreach (LibraryRecord record in Library.Matching(title)) List.AddChild(Row(t, record));
        }
        if (List.GetChildCount() == 0) List.AddChild(ChromeWidgets.Text("Nothing matches.", "DimLabel"));
    }

    private Control Row(TokenSet t, LibraryRecord record)
    {
        bool selected = record.Name == Library.Selected;
        var holder = new PanelContainer { ThemeTypeVariation = selected ? "RowSelected" : "Row", CustomMinimumSize = new Vector2(0, t.Px("row.height")) };
        holder.SetMeta("library_record", record.Name);
        var line = new HBoxContainer(); holder.AddChild(line);
        Label kind = ChromeWidgets.Text("", "DimLabel"); ChromeIcons.Apply(kind, LibraryModel.Glyph(record.Kind), t.Px("icon.small"));
        kind.TooltipText = LibraryModel.Word(record.Kind); kind.MouseFilter = MouseFilterEnum.Pass; line.AddChild(kind);
        if (record.IsMacro) line.AddChild(new SignatureMarks(LibraryModel.Signature(record.Word), t) { TooltipText = LibraryModel.SignatureText(record.Word) });
        Button name = ChromeWidgets.Tool(record.Name, $"{record.Name}: {LibraryModel.Word(record.Kind)}, {record.Note}{LibraryModel.Mark}");
        name.Alignment = HorizontalAlignment.Left; name.ClipText = true; name.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        name.Pressed += () => Send("macro.select", record.Name); line.AddChild(name);
        return holder;
    }
}

// An effect signature drawn as marks, one per sample orbit: filled where the word moves the orbit, an outline
// where it does not (ADR-D01.70). Drawn, not spelled, so no font decides its shape.
public sealed partial class SignatureMarks : Control
{
    private readonly bool[] moved; private readonly TokenSet tokens;

    public SignatureMarks(System.Collections.Generic.IReadOnlyList<bool> moved, TokenSet tokens)
    {
        this.moved = new bool[moved.Count]; for (int i = 0; i < moved.Count; i++) this.moved[i] = moved[i];
        this.tokens = tokens; Name = "EffectSignature"; MouseFilter = MouseFilterEnum.Pass;
        float cell = Mathf.Max(4, tokens.Px("icon.small") / 3);
        CustomMinimumSize = new Vector2(moved.Count * (cell + 2), tokens.Px("icon.small"));
    }

    public override void _Draw()
    {
        float cell = Mathf.Max(4, tokens.Px("icon.small") / 3), y = (Size.Y - cell) / 2;
        Color ink = tokens.C("text.dim");
        for (int i = 0; i < moved.Length; i++)
        {
            var box = new Rect2(i * (cell + 2), y, cell, cell);
            if (moved[i]) DrawRect(box, ink); else DrawRect(box, ink, false, 1);
        }
    }
}
