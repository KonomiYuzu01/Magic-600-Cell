using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The Library dock (ADR-D01.69): one tree of the solver's records under an input box that filters in place. The
// keyboard works from the box: Up and Down move the selection, Enter adds the selected macro to the active phase
// (macro.add) and Shift+Enter previews it alone (operation.preview). A row leads with its kind's glyph and the
// effect signature in the rail's order. The panel shows the model and sends registry commands, a branch head
// included (library.branch.toggle); it never changes the draft or the tree itself.
public sealed partial class LibraryPanel : VBoxContainer
{
    public LibraryModel Library { get; }
    public LineEdit Filter { get; } = new() { Name = "LibraryFilter", PlaceholderText = "Filter", ClearButtonEnabled = true, SizeFlagsHorizontal = SizeFlags.ExpandFill };
    public VBoxContainer List { get; } = new() { Name = "LibraryRows", SizeFlagsHorizontal = SizeFlags.ExpandFill };
    public event Action<string, string?>? Command;
    // The rail's order of the 35 orbits, for the signatures; without it no signature is drawn.
    public Func<IReadOnlyList<int>?>? RailOrder { get; set; }
    private readonly Func<TokenSet> tokens;

    public LibraryPanel(LibraryModel library, Func<TokenSet> tokens)
    {
        Library = library; this.tokens = tokens; Name = "LibraryPanel";
        Filter.TooltipText = "Filter the library in place. Up and Down select; Enter adds the selected macro to the active phase; Shift+Enter previews it alone. A Template binds its roles to the strip.";
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

    // Enter and Shift+Enter send the selected record's name; with none selected they send none, and the host says
    // why and changes nothing.
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
            case Godot.Key.Enter or Godot.Key.KpEnter when key.ShiftPressed: Send("operation.preview", LibraryModel.PreviewRequest(name)); break;
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
            Button head = SessionWidgets.Tool(open ? "fold.open" : "fold.closed", open ? "Fold this branch" : "Open this branch", t, () => Send("library.branch.toggle", branch), $"{title} {count}");
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
        if ((record.IsMacro || record.IsTemplate) && RailOrder?.Invoke() is { } order)
        {
            EffectSignature signature = record.IsMacro ? LibraryModel.Signature(record.Word, order) : EffectSignature.Unknown(order);
            line.AddChild(new SignatureMarks(signature, t) { TooltipText = record.IsMacro ? LibraryModel.SignatureText(signature) : "Effect signature: unknown until its roles are bound to the strip." });
        }
        Button name = ChromeWidgets.Tool(record.Name, $"{record.Name}: {LibraryModel.Word(record.Kind)}, {record.Note}{LibraryModel.Mark}");
        name.Alignment = HorizontalAlignment.Left; name.ClipText = true; name.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        name.Pressed += () => Send("macro.select", record.Name); line.AddChild(name);
        return holder;
    }
}

// An effect signature drawn as 35 slots in the rail's order (ADR-D01.70): a full bar where the word moves the
// orbit, a tick on the base line where it was measured not to, and a short upright hair where nothing is known, so
// unknown never reads as zero. Drawn, not spelled, so no font decides its shape.
public sealed partial class SignatureMarks : Control
{
    public const float Slot = 3, Gap = 1;
    public IReadOnlyList<SignatureMark> Marks { get; }
    private readonly TokenSet tokens;

    public SignatureMarks(EffectSignature signature, TokenSet tokens)
    {
        Marks = signature.Slots.Select(s => s.Mark).ToArray();
        this.tokens = tokens; Name = "EffectSignature"; MouseFilter = MouseFilterEnum.Pass;
        CustomMinimumSize = new Vector2(Marks.Count * (Slot + Gap), tokens.Px("icon.small"));
    }

    public override void _Draw()
    {
        float h = Mathf.Max(6, Size.Y * .6f), top = (Size.Y - h) / 2;
        Color ink = tokens.C("text.dim"), off = tokens.C("text.off");
        for (int i = 0; i < Marks.Count; i++)
        {
            float x = i * (Slot + Gap);
            switch (Marks[i])
            {
                case SignatureMark.Moved: DrawRect(new Rect2(x, top, Slot, h), ink); break;
                case SignatureMark.Untouched: DrawRect(new Rect2(x, top + h - 1, Slot, 1), ink); break;
                default: DrawRect(new Rect2(x + 1, top + h / 3, 1, h / 3), off); break;
            }
        }
    }
}
