using System;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// A button that keeps the keys while it has keyboard focus (ChromeFrame.Claims takes IOwnKeys), so Tab, Enter and
// Space work the Focus layer editor and never reach the session as twists.
public sealed partial class OwnedButton : Button, IOwnKeys
{
    public OwnedButton(string name, string text, string tooltip, bool toggle = false)
    {
        Name = name; Text = text; TooltipText = tooltip; ToggleMode = toggle; ThemeTypeVariation = "ToolButton"; FocusMode = FocusModeEnum.All;
    }
}

// A choice that keeps the keys while it has keyboard focus.
public sealed partial class OwnedChoice : OptionButton, IOwnKeys
{
    public OwnedChoice(string name, string tooltip) { Name = name; TooltipText = tooltip; FocusMode = FocusModeEnum.All; }
}

// The Focus layer entry on Global's border (owner, 26 November 2026: the 0.4 Piece Filter belongs in the default view;
// "Focus layer加别名就行": ADR-D01's word on screen, "piece filter" a palette alias). It opens the editor.
public sealed partial class FocusLayerEntry : Button, IOwnKeys
{
    public const string Words = "Focus layer =";

    public FocusLayerEntry(Action open)
    {
        Name = "FocusLayerEntry"; Text = Words; ThemeTypeVariation = "ToolButton"; FocusMode = FocusModeEnum.All;
        TooltipText = "Compose a focus layer from an = expression: preview its exact count, then apply (Open focus layer)";
        Pressed += open;
    }
}

// The Focus layer editor (filter.open; U6-022, U5-048, U5-050, U5-051): the expression, how it composes with the
// applied layer, its kind, the exact count before apply, and the presets, save and load. Every control is reachable
// by Tab; Enter in the expression previews it.
public sealed partial class FocusLayerEditor : VBoxContainer
{
    public const string PanelId = "focus_layer.editor";
    public PieceFilterModel? Model { get; private set; }
    public LineEdit Expression { get; } = new() { Name = "FocusLayerExpression", PlaceholderText = "lattice and height >= 2", FocusMode = FocusModeEnum.All };
    private readonly Label line = ChromeWidgets.Text("", "", true), applied = ChromeWidgets.Text("", "MicroLabel", true);
    private readonly OwnedButton[] compose, kinds;
    private readonly OwnedButton preview, apply, clear, edit, save, load;
    private readonly OwnedChoice presets, saved;
    private readonly LineEdit name = new() { Name = "FocusLayerSaveName", PlaceholderText = "name", FocusMode = FocusModeEnum.All };

    public FocusLayerEditor(Func<TokenSet> tokens)
    {
        Name = "FocusLayerEditor";
        AddChild(ChromeWidgets.Text(FilterParser.Fields + " · and, or, not", "MicroLabel", true));
        Expression.TooltipText = "Examples\ninside-a-cap 150 and height >= 2\nclass = 2 or set \"my pieces\"\n"
            + "Cap fields use the named cap when you omit its number. Moved uses the focused repair step or preview.";
        AddChild(Expression);
        Expression.TextChanged += text => Model?.SetText(text);
        Expression.TextSubmitted += _ => Model?.Run();

        var composeRow = new HBoxContainer { Name = "FocusLayerCompose" };
        var group = new ButtonGroup();
        compose = Enum.GetValues<FilterComposition>().Select(c =>
        {
            var b = new OwnedButton("Compose" + c, c.ToString(), c + " the applied focus layer with this expression (Compose focus layer)", true) { ButtonGroup = group };
            b.Pressed += () => Model?.Compose(c);
            composeRow.AddChild(b);
            return b;
        }).ToArray();
        AddChild(composeRow);

        var kindRow = new HBoxContainer { Name = "FocusLayerKind" };
        var kindGroup = new ButtonGroup();
        kinds = Enum.GetValues<FilterKind>().Select(k =>
        {
            string word = PieceFilterModel.Word(k);
            var b = new OwnedButton("Kind" + k, char.ToUpperInvariant(word[0]) + word[1..], k switch
            {
                FilterKind.LiveQuery => "Evaluated again at every revision",
                FilterKind.FixedPieces => "Follows the pieces it holds when applied",
                _ => "Keeps the positions it holds when applied",
            }, true) { ButtonGroup = kindGroup };
            b.Pressed += () => Model?.SetKind(k);
            kindRow.AddChild(b);
            return b;
        }).ToArray();
        AddChild(kindRow);

        AddChild(line); line.Name = "FocusLayerLine";
        var actions = new HBoxContainer { Name = "FocusLayerActions" };
        preview = new OwnedButton("FocusLayerPreview", "Preview", "Count the pieces exactly; nothing applies (Enter; Preview focus layer counts)");
        preview.Pressed += () => Model?.Run();
        apply = new OwnedButton("FocusLayerApply", "Apply", "Apply the previewed layer; the cycling key then dims or hides the rest (Apply focus layer)");
        apply.Pressed += () => Model?.Apply(UserEvent.Click("filter.apply"));
        clear = new OwnedButton("FocusLayerClear", "Clear", "Take the applied layer away");
        clear.Pressed += () => Model?.Clear();
        edit = new OwnedButton("FocusLayerEdit", "Edit applied", "Bring the applied expression back to edit (Edit focus layer)");
        edit.Pressed += () => Model?.Edit();
        foreach (var b in new[] { preview, apply, clear, edit }) actions.AddChild(b);
        AddChild(actions);
        AddChild(applied); applied.Name = "FocusLayerApplied";

        var presetRow = new HBoxContainer { Name = "FocusLayerPresets" };
        presets = new OwnedChoice("FocusLayerPreset", "Stage a preset for review; it applies nothing (Stage focus layer preset)");
        presets.AddItem("Presets");
        foreach (var p in PieceFilterModel.Presets) presets.AddItem(p.Name);
        presets.ItemSelected += index => { if (index > 0) Model?.Preset(PieceFilterModel.Presets[(int)index - 1].Name); presets.Select(0); };
        presetRow.AddChild(presets);
        AddChild(presetRow);

        var saveRow = new HBoxContainer { Name = "FocusLayerSave" };
        name.SizeFlagsHorizontal = SizeFlags.ExpandFill;
        save = new OwnedButton("FocusLayerSaveButton", "Save", "Save the applied layer, exactly as the view shows it (Save focus layer)");
        save.Pressed += () => { if (Model?.Save(name.Text) is string refused) line.Text = "Not saved: " + refused; };
        saved = new OwnedChoice("FocusLayerSaved", "A saved layer");
        load = new OwnedButton("FocusLayerLoad", "Load", "Stage the saved layer for review; it applies nothing (Load focus layer for review)");
        load.Pressed += () =>
        {
            if (Model is { } m && saved.Selected >= 0 && saved.Selected < m.Saved.Count && m.Load(m.Saved[saved.Selected].Name) is string refused) line.Text = "Not loaded: " + refused;
        };
        foreach (Control c in new Control[] { name, save, saved, load }) saveRow.AddChild(c);
        AddChild(saveRow);
        AddChild(ChromeWidgets.Text("Counts come from the bound sample. Save keeps the shown piece set.", "MicroLabel", true));
        TreeEntered += () => { if (Model != null) Model.Changed += Refresh; Refresh(); };
        TreeExiting += () => { if (Model != null) Model.Changed -= Refresh; };
    }

    public void Bind(PieceFilterModel? model)
    {
        if (Model != null) Model.Changed -= Refresh;
        Model = model;
        if (Model != null && IsInsideTree()) Model.Changed += Refresh;
        Refresh();
    }

    private void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        bool bound = Model != null;
        Expression.Editable = name.Editable = bound;
        preview.Disabled = presets.Disabled = save.Disabled = !bound;
        if (Model == null)
        {
            line.Text = "No state is bound: the focus layer counts the bound sample state's pieces.";
            applied.Text = "";
            return;
        }
        if (Expression.Text != Model.Staged) Expression.Text = Model.Staged;
        for (int i = 0; i < compose.Length; i++) compose[i].SetPressedNoSignal((int)Model.Composition == i);
        for (int i = 0; i < kinds.Length; i++) kinds[i].SetPressedNoSignal((int)Model.Kind == i);
        line.Text = Model.PreviewLine();
        apply.Disabled = Model.Preview == null;
        clear.Disabled = edit.Disabled = Model.Applied == null;
        applied.Text = Model.Layer() is { } layer ? "Applied: " + FocusLayers.KeyText(layer) : "No focus layer applied";
        saved.Clear();
        foreach (var s in Model.Saved) saved.AddItem(s.Name + " · = " + s.Expression);
        load.Disabled = Model.Saved.Count == 0;
    }
}
