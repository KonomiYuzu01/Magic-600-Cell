using System;
using Godot;
using LookLab.Core;
using LookLab.App.Platform;
using LabColour = LookLab.Core.Colour;

namespace LookLab.App;

internal sealed partial class LabApp : Control
{
    public LookState State { get; }
    public LookViewport View { get; }
    public ParameterPanels Panels { get; }
    private readonly SubViewportContainer stage;
    private readonly SubViewport viewport;
    private readonly PanelContainer panel;
    private readonly Label status;
    private readonly LineEdit name;
    private readonly PresetFiles files;
    private readonly FileDialog load;
    private readonly FileDialog save;

    public LabApp(string root)
    {
        var schema = ParameterSchema.Load(System.IO.Path.Combine(root, "tools/looklab/data/parameters.json"));
        State = new LookState(schema, Presets.Load(System.IO.Path.Combine(root, "tools/looklab/presets/default.json"), schema));
        View = new LookViewport(root, State);
        SetAnchorsAndOffsetsPreset(LayoutPreset.FullRect);
        Theme = new Theme();
        stage = new SubViewportContainer { Stretch = true, MouseFilter = MouseFilterEnum.Stop };
        AddChild(stage);
        viewport = new SubViewport { Size = new Vector2I(640, 360), OwnWorld3D = true, TransparentBg = false,
            RenderTargetUpdateMode = SubViewport.UpdateMode.Always, HandleInputLocally = true };
        stage.AddChild(viewport); viewport.AddChild(View);
        panel = new PanelContainer(); AddChild(panel);
        var margin = new MarginContainer(); panel.AddChild(margin);
        foreach (string side in new[] { "margin_left", "margin_right", "margin_top", "margin_bottom" }) margin.AddThemeConstantOverride(side, 12);
        var column = new VBoxContainer(); margin.AddChild(column);
        column.AddChild(new Label { Text = "Look Lab · full detail" });
        status = new Label { Text = "Full detail: 259,800 slots · cost not measured", AutowrapMode = TextServer.AutowrapMode.WordSmart };
        column.AddChild(status);
        name = new LineEdit { Text = State.Preset.Name, PlaceholderText = "Preset name" }; column.AddChild(name);
        var buttons = new HBoxContainer(); column.AddChild(buttons);
        var loadButton = new Button { Text = "Load preset", SizeFlagsHorizontal = SizeFlags.ExpandFill };
        var saveButton = new Button { Text = "Save preset", SizeFlagsHorizontal = SizeFlags.ExpandFill };
        buttons.AddChild(loadButton); buttons.AddChild(saveButton);
        LL3.ModeControls.Attach(this, root, column);
        var scroll = new ScrollContainer { SizeFlagsVertical = SizeFlags.ExpandFill, HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };
        column.AddChild(scroll);
        Panels = new ParameterPanels(State) { SizeFlagsHorizontal = SizeFlags.ExpandFill }; scroll.AddChild(Panels);
        files = new PresetFiles(root, schema);
        load = Dialog(FileDialog.FileModeEnum.OpenFile); save = Dialog(FileDialog.FileModeEnum.SaveFile);
        AddChild(load); AddChild(save);
        loadButton.Pressed += () => Guard(() =>
        {
            if (!files.Exists) { status.Text = "Save your first preset to create the presets folder."; return; }
            load.RootSubfolder = files.Folder; load.CurrentDir = files.Folder; load.PopupCentered(new Vector2I(600, 340));
        });
        saveButton.Pressed += () => Guard(() =>
        {
            files.PrepareSave(); State.Rename(name.Text);
            save.RootSubfolder = files.Folder; save.CurrentDir = files.Folder; save.CurrentFile = "look.json"; save.PopupCentered(new Vector2I(600, 340));
        });
        load.FileSelected += path => Guard(() => { State.Load(files.Load(path)); name.Text = State.Preset.Name; status.Text = "Preset loaded."; });
        save.FileSelected += path => Guard(() => { files.Save(path, State.Preset); status.Text = "Preset saved."; });
        var diagnostics = LL5.LabControls.Attach(root, State, View, Panels, status);
        View.Status = diagnostics.UpdateStatus;
        State.Changed += Change;
        Resized += Layout;
        ApplyStyle(); Layout();
        LL4.GreyboxControls.Attach(this, stage, panel, Panels, root);
    }

    private FileDialog Dialog(FileDialog.FileModeEnum mode) => new()
    {
        Access = FileDialog.AccessEnum.Filesystem, FileMode = mode,
        UseNativeDialog = false, Filters = new[] { "*.json ; Look Lab preset" }, ShowHiddenFiles = false
    };

    private void Guard(Action action)
    {
        try { action(); }
        catch (Exception ex) { status.Text = ex.Message; }
    }

    private void Change(string id, ParameterValue value) { ApplyStyle(); Layout(); }

    private void ApplyStyle()
    {
        ParameterSet p = State.Preset.Params;
        Theme.DefaultFontSize = (int)Math.Round(14 * p.Number("typeScale"));
        Panels.AddThemeConstantOverride("separation", (int)Math.Round(10 * p.Number("panelDensity")));
        OklchColour accent = p.Colour("accentSecondary");
        GamutColour mapped = LabColour.MapToGamut(accent.L, accent.C, accent.H);
        var colour = new Color((float)mapped.Srgb[0], (float)mapped.Srgb[1], (float)mapped.Srgb[2]);
        foreach (Label header in Panels.Headers)
        {
            header.AddThemeColorOverride("font_color", colour);
            header.AddThemeFontSizeOverride("font_size", (int)Math.Round(17 * p.Number("typeScale")));
        }
    }

    private void Layout()
    {
        if (Size.X <= 0 || Size.Y <= 0) return;
        double share = State.Preset.Params.Number("viewportShare");
        float panelWidth = Math.Min(Size.X * .65f, Math.Max(300, Size.X * (float)(1 - share)));
        bool docked = State.Preset.Params.Choice("panelPlacement") == "docked" || State.Preset.Params.Choice("layoutId") == "docked-workbench";
        panel.Position = new Vector2(Size.X - panelWidth, 0); panel.Size = new Vector2(panelWidth, Size.Y);
        stage.Position = Vector2.Zero; stage.Size = new Vector2(docked ? Size.X - panelWidth : Size.X, Size.Y);
    }

    public override void _ExitTree() => State.Changed -= Change;
}
