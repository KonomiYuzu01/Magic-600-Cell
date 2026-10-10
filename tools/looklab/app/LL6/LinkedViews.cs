using System;
using System.IO;
using System.Linq;
using Godot;
using LookLab.Core;
using LookLab.App.LL5;

namespace LookLab.App.LL6;

internal sealed class LinkedViews
{
    private readonly LabApp app;
    private readonly string root;
    private readonly CvdPreview preview;
    private readonly FrameFollower follower = new();
    private double cachedGap = double.NaN;
    private double[] anchors = Array.Empty<double>();
    public LayoutSpec[] Layouts { get; }
    public LayoutSpec Layout => Layouts.Single(l => l.Id == app.State.Preset.Params.Choice("layoutId"));
    // A paired layout keeps both views clear of the parameter panel in either placement.
    public bool Docked => app.State.Preset.Params.Choice("panelPlacement") == "docked" ||
        Layout.Regions.Values.Any(r => r.Kind == "panel" && r.Placement == "docked" || ReservePanel && r.Kind == "local");
    public RegionRect StageRect { get; private set; }
    public SubViewportContainer? LocalStage { get; private set; }
    public SubViewport? LocalViewport { get; private set; }
    public LookViewport? Local { get; private set; }
    public FocusFrame Frame => follower.Frame;
    internal bool LocalHidden { get; set; }
    internal bool LocalDragEnabled { get; set; } = true;
    internal bool LocalClockEnabled { get; set; }
    internal bool FramingEnabled { get; set; } = true;
    internal bool ReservePanel { get; set; } = true;

    public LinkedViews(LabApp app, string root, CvdPreview preview)
    {
        this.app = app; this.root = root; this.preview = preview;
        var catalogue = CommandCatalogue.Load(Path.Combine(root, "docs/progress/1.0/command-table.json"));
        Layouts = Directory.GetFiles(Path.Combine(root, "tools/looklab/data/layouts"), "*.json")
            .OrderBy(p => p, StringComparer.Ordinal).Select(p => LayoutSpec.Load(p, catalogue)).ToArray();
        app.View.SetProcessUnhandledInput(false);
        app.GlobalStage.GuiInput += GlobalInput;
        app.View.FrameStepped += Step;
        app.Ready += Ready;
    }

    private void Ready() => app.View.SetProcessUnhandledInput(false);

    private void GlobalInput(InputEvent input) => PaneInput(input, false);
    private void LocalInput(InputEvent input) => PaneInput(input, true);
    private void PaneInput(InputEvent input, bool local)
    {
        if ((!local || LocalDragEnabled) && input is InputEventMouseMotion motion && (motion.ButtonMask & MouseButtonMask.Left) != 0)
            app.View.Drag(motion.Relative);
    }

    private void CreateLocal()
    {
        LocalStage = new SubViewportContainer { Name = "LocalStage", Stretch = true, MouseFilter = Control.MouseFilterEnum.Stop };
        app.AddChild(LocalStage); app.MoveChild(LocalStage, app.GlobalStage.GetIndex() + 1);
        LocalViewport = new SubViewport { Size = new Vector2I(640, 360), OwnWorld3D = true, TransparentBg = false,
            RenderTargetUpdateMode = SubViewport.UpdateMode.Disabled, HandleInputLocally = true };
        LocalStage.AddChild(LocalViewport);
        Local = new LookViewport(root, app.State, animate: false);
        LocalViewport.AddChild(Local); Local.SetProcess(false); Local.SetProcessUnhandledInput(false);
        Array.Copy(app.View.Q, Local.Q, 16); Local.Bind(app.View.BoundFrame);
        LocalStage.GuiInput += LocalInput;
        preview.AddView(Local);
    }

    public void Arrange(RegionRect stage)
    {
        StageRect = stage;
        ViewArrangement arrangement = ViewFraming.Arrange(Layout, stage);
        Place(app.GlobalStage, arrangement.Global);
        if (arrangement.Local is { } rect)
        {
            if (Local == null) CreateLocal();
            if (!LocalStage!.Visible) follower.Reset();
            Place(LocalStage, rect); LocalStage.Visible = !LocalHidden;
            LocalViewport!.RenderTargetUpdateMode = LocalStage.Visible ? SubViewport.UpdateMode.Always : SubViewport.UpdateMode.Disabled;
        }
        else if (LocalStage != null)
        {
            LocalStage.Visible = false; LocalViewport!.RenderTargetUpdateMode = SubViewport.UpdateMode.Disabled;
        }
    }

    private static void Place(Control container, RegionRect rect)
    {
        container.Position = new Vector2((float)rect.X, (float)rect.Y);
        container.Size = new Vector2((float)rect.Width, (float)rect.Height);
    }

    public FocusFrame Target()
    {
        double gap = app.State.Preset.Params.Number("gap");
        if (gap != cachedGap)
        {
            anchors = ViewFraming.MovingRestAnchors(app.View.Geometry, app.View.Turn, gap); cachedGap = gap;
        }
        Vector2I size = LocalViewport!.Size;
        double aspect = size.Y > 0 ? (float)size.X / size.Y : 1;
        return ViewFraming.Target(anchors, app.View.Q, app.State.Preset.Params.Structure(aspect));
    }

    private void Step(double dt)
    {
        if (LocalStage?.Visible != true) return;
        Array.Copy(app.View.Q, Local!.Q, 16);
        if (LocalClockEnabled) { Local.Animate = true; Local._Process(dt); }
        else Local.Bind(app.View.BoundFrame);
        Local.Material.SetShaderParameter("q", LookViewport.Matrix(Local.Q));
        Vector2I size = LocalViewport!.Size;
        Local.Material.SetShaderParameter("aspect", size.Y > 0 ? (float)size.X / size.Y : 1);
        if (FramingEnabled) Local.SetViewFrame(follower.Step(Target(), dt));
    }

    public void Detach()
    {
        app.GlobalStage.GuiInput -= GlobalInput; app.View.FrameStepped -= Step; app.Ready -= Ready;
        if (LocalStage != null) LocalStage.GuiInput -= LocalInput;
    }
}
