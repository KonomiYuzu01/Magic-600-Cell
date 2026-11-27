using System;
using System.Collections.Generic;
using System.Globalization;
using Godot;

namespace Magic600.Chrome;

// The setup slot under the Target cell (ADR-D01.68): the tool slot and its word, the target it was named for, two
// buttons that name the target (the Target, or Wanted's present place), the rows in their neutral order with their
// lengths, each with Fill (Prepare and Cleanup around Core) and Alone (the setup alone as the preview), then the
// coverage line, the minimal or empty line and the tool's declaration. Nothing here picks, orders or stages a row:
// each button asks for its command, and the host carries it out.
public sealed partial class SetupSlotView : VBoxContainer
{
    // At most this many rows show at once; the rest scroll.
    private const int ShownRows = 4, RowHeight = 26;
    public SetupSearch Search { get; }
    // A button asks for its command: (command, argument).
    public event Action<string, string?>? Command;
    private readonly Func<TokenSet> tokens;
    private readonly Label named, coverage, verdict, declaration;
    private readonly VBoxContainer rows = new() { Name = "SetupRows" };
    private readonly ScrollContainer scroll = new() { Name = "SetupRowsScroll", HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };
    private readonly Action changed;

    public SetupSlotView(SetupSearch search, Func<TokenSet> tokens)
    {
        Search = search; this.tokens = tokens; Name = "SetupSlot";
        AddThemeConstantOverride("separation", 1);
        AddChild(new ToolSlotBox(search.Slot, "Setup", null, tokens, search.Word));
        named = Line("SetupNamed", "MicroLabel"); AddChild(named);
        var buttons = new HBoxContainer { Name = "SetupButtons" };
        Button run = ChromeWidgets.Tool("Run", "Setup search for the Target you named (setup.search)"); run.Name = "SetupRun";
        run.Pressed += () => Safe.Raise(Command, SetupSearch.Search, null);
        Button wanted = ChromeWidgets.Tool("Wanted's place", "Setup search for the Wanted piece's present place (setup.search wanted)"); wanted.Name = "SetupWanted";
        wanted.Pressed += () => Safe.Raise(Command, SetupSearch.Search, SetupSearch.WantedArgument);
        buttons.AddChild(run); buttons.AddChild(wanted); AddChild(buttons);
        rows.AddThemeConstantOverride("separation", 0); scroll.AddChild(rows); AddChild(scroll);
        rows.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        verdict = Line("SetupVerdict", "MicroLabel"); AddChild(verdict);
        coverage = Line("SetupCoverage", "MicroLabel"); AddChild(coverage);
        declaration = Line("SetupDeclaration", "MicroLabel"); AddChild(declaration);
        changed = () => { if (IsInstanceValid(this)) Refresh(); };
        search.Changed += changed;
        Refresh();
    }

    public override void _ExitTree() => Search.Changed -= changed;

    private static Label Line(string name, string variation)
    {
        Label label = ChromeWidgets.Text("", variation); label.Name = name; label.AutowrapMode = TextServer.AutowrapMode.WordSmart;
        label.SizeFlagsHorizontal = SizeFlags.ExpandFill; return label;
    }

    private static void Set(Label label, string? text) { label.Text = text ?? ""; label.Visible = text != null; }

    public void Refresh()
    {
        TokenSet t = tokens();
        Set(named, Search.NamedLine());
        foreach (Node child in rows.GetChildren()) { rows.RemoveChild(child); child.QueueFree(); }
        IReadOnlyList<string> lines = Search.RowLines();
        for (int i = 0; i < lines.Count; i++)
        {
            string n = (i + 1).ToString(CultureInfo.InvariantCulture);
            var row = new HBoxContainer { Name = "SetupRow" + n };
            row.AddChild(ChromeWidgets.Text(lines[i], "ReadoutLabel", true));
            Button fill = ChromeWidgets.Tool("Fill", "Prepare and Cleanup around your Core: row " + n + " and its inverse (setup.row.use " + n + ")"); fill.Name = "Fill";
            fill.Pressed += () => Safe.Raise(Command, SetupSearch.Use, n);
            Button alone = ChromeWidgets.Tool("Alone", "Row " + n + "'s setup alone as the preview (operation.preview " + SetupSearch.PreviewArgument + n + ")"); alone.Name = "Alone";
            alone.Pressed += () => Safe.Raise(Command, "operation.preview", SetupSearch.PreviewArgument + n);
            row.AddChild(fill); row.AddChild(alone); rows.AddChild(row);
        }
        scroll.Visible = lines.Count > 0;
        scroll.CustomMinimumSize = new Vector2(0, Math.Min(lines.Count, ShownRows) * RowHeight);
        Set(verdict, Search.MinimalLine() ?? Search.EmptyLine());
        Set(coverage, Search.CoverageLine());
        Set(declaration, Search.Tool.Line());
        coverage.AddThemeColorOverride("font_color", t.C("text.dim"));
        declaration.AddThemeColorOverride("font_color", t.C("text.dim"));
    }
}
