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
    public SubViewportContainer GlobalStage => stage;
    internal Control ParameterPanel => panel;
    public LL6.LinkedViews Linked { get; }
    private readonly SubViewportContainer stage;
    private readonly SubViewport viewport;
    private readonly PanelContainer panel;
    private readonly Label status;
    private readonly LineEdit name;
    private readonly PresetFiles files;
    private readonly FileDialog load;
    private readonly FileDialog save;
    private readonly ScrollContainer parameterScroll;
    // Panel state that a reload or a restart keeps: each part reads and restores one value as text.
    internal System.Collections.Generic.SortedDictionary<string, (Func<string> Get, Action<string> Set)> ViewParts { get; } = new(StringComparer.Ordinal);
    public string DefaultDigest { get; }

    public LabApp(string root, bool opening = false)
    {
        var schema = ParameterSchema.Load(System.IO.Path.Combine(root, "tools/looklab/data/parameters.json"));
        byte[] defaultBytes = System.IO.File.ReadAllBytes(System.IO.Path.Combine(root, "tools/looklab/presets/default.json"));
        Preset initial = Presets.Parse(System.Text.Encoding.UTF8.GetString(defaultBytes), schema);
        string openingPath = System.IO.Path.Combine(root, "tools/looklab/data/opening.json");
        if (opening && System.IO.File.Exists(openingPath))
        {
            byte[] openingBytes = System.IO.File.ReadAllBytes(openingPath);
            initial = OpeningLook.Apply(initial, System.Text.Encoding.UTF8.GetString(openingBytes), schema);
            defaultBytes = System.Linq.Enumerable.ToArray(System.Linq.Enumerable.Concat(defaultBytes, openingBytes));
        }
        DefaultDigest = ViewStates.Digest(defaultBytes);
        State = new LookState(schema, initial);
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
        column.AddChild(scroll); parameterScroll = scroll;
        ViewParts["parameters.scroll"] = (() => scroll.ScrollVertical.ToString(System.Globalization.CultureInfo.InvariantCulture),
            v => { if (int.TryParse(v, System.Globalization.NumberStyles.None, System.Globalization.CultureInfo.InvariantCulture, out int y)) scroll.SetDeferred(ScrollContainer.PropertyName.ScrollVertical, y); });
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
        Linked = new LL6.LinkedViews(this, root, diagnostics.Preview);
        State.Changed += Change;
        Resized += Layout;
        ApplyStyle(); Layout();
        LL4.GreyboxControls.Attach(this, panel, Panels, root);
    }

    internal ViewState CaptureView()
    {
        var ui = new System.Collections.Generic.SortedDictionary<string, string>(StringComparer.Ordinal);
        foreach (var part in ViewParts) ui[part.Key] = part.Value.Get();
        return new ViewState(Array.AsReadOnly((double[])View.Q.Clone()), Array.AsReadOnly((double[])View.Spin.Clone()), DefaultDigest, State.Preset, ui);
    }

    // Camera, look and panel state from before a reload or restart. A part this build does not have is ignored.
    internal void RestoreView(ViewState kept)
    {
        Preset look = ViewStates.Resolve(kept, State.Preset, DefaultDigest);
        if (!ReferenceEquals(look, State.Preset)) { State.Load(look); name.Text = State.Preset.Name; }
        for (int i = 0; i < 16; i++) { View.Q[i] = kept.Q[i]; View.Spin[i] = kept.Spin[i]; }
        if (Linked.Local is { } local) { Array.Copy(View.Q, local.Q, 16); Array.Copy(View.Spin, local.Spin, 16); }
        foreach (var entry in kept.Ui)
            if (ViewParts.TryGetValue(entry.Key, out var part)) part.Set(entry.Value);
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
        bool docked = Linked.Docked;
        panel.Position = new Vector2(Size.X - panelWidth, 0); panel.Size = new Vector2(panelWidth, Size.Y);
        Linked.Arrange(new RegionRect(0, 0, docked ? Size.X - panelWidth : Size.X, Size.Y));
    }

    public override void _ExitTree() { State.Changed -= Change; Linked.Detach(); }
}
