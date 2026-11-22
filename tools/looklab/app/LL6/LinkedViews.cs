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
    private readonly int[] cells;
    private readonly int[] focus;
    private readonly float[] centres;
    // The Local view's own shape, used inside the window frame; null keeps the re-framed copy of Global.
    public LocalShape? Shape => shapes.Length == 0 ? null : shapes[Math.Clamp(LocalChoice, 0, shapes.Length - 1)];
    // The Local shapes on file, the cap-centred one first; the lab steps through them to compare.
    private readonly LocalShape[] shapes;
    public int LocalChoices => shapes.Length;
    public int LocalChoice { get; set; }
    public static readonly string[] ShapeFiles = { "tools/looklab/data/local-view-cap.json", ShapeFile };
    // The largest distance of a cell from the middle of each Global layout, so the whole layout fits its pane.
    private readonly double ringReach, sheetReach;
    public const double GlobalFill = .93;
    public int FocusCells => focus.Length;
    public const string ShapeFile = "tools/looklab/data/local-view.json";
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
    internal bool FramingUsesSpin { get; set; } = true;
    internal bool ReservePanel { get; set; } = true;

    public LinkedViews(LabApp app, string root, CvdPreview preview)
    {
        this.app = app; this.root = root; this.preview = preview;
        cells = ViewFraming.MovingRestCells(app.View.Turn);
        focus = cells.Distinct().ToArray();
        centres = app.View.Layout.CellLayout(app.View.Turn).AsSpan(0, Geometry.Cells * 4).ToArray();
        shapes = ShapeFiles.Select(f => Path.Combine(root, f)).Where(File.Exists).Select(f => LocalShape.Parse(File.ReadAllText(f), app.State.Schema)).ToArray();
        float[] layout = app.View.Layout.CellLayout(app.View.Turn);
        double Reach(int row) => Enumerable.Range(0, Geometry.Cells).Max(c => Math.Sqrt(Enumerable.Range(0, 3).Sum(i => Math.Pow(layout[(row * Geometry.Cells + c) * 4 + i], 2))));
        ringReach = Reach(5); sheetReach = Reach(9);
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
        Array.Copy(app.View.Spin, Local.Spin, 16);
        LocalStage.GuiInput += LocalInput;
        preview.AddView(Local);
    }

    // The window frame shows both main views at all times and sizes them itself.
    public void EnterFrame()
    {
        if (Local == null) CreateLocal();
        follower.Reset(); LocalStage!.Visible = true; LocalViewport!.RenderTargetUpdateMode = SubViewport.UpdateMode.Always;
    }

    public void Arrange(RegionRect stage)
    {
        if (app.Framed) return;
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
        return ViewFraming.Target(anchors, cells, app.View.Q, FramingUsesSpin ? app.View.Spin : Geometry.Identity(),
            app.State.Preset.Params.Structure(aspect), app.View.Layout);
    }

    private void Step(double dt)
    {
        if (LocalStage?.Visible != true) return;
        // A view made before its window was in the tree starts processing when it becomes ready, and would then
        // write its own camera over the Local frame; Local is driven from here alone.
        if (!LocalClockEnabled && Local!.IsProcessing()) { Local.SetProcess(false); Local.SetProcessUnhandledInput(false); }
        Array.Copy(app.View.Q, Local!.Q, 16);
        Array.Copy(app.View.Spin, Local.Spin, 16);
        if (LocalClockEnabled) { Local.Animate = true; Local._Process(dt); }
        else Local.Bind(app.View.BoundFrame);
        Local.Material.SetShaderParameter("q", LookViewport.Matrix(Local.Q));
        Local.Material.SetShaderParameter("spin", LookViewport.Matrix(Local.Spin));
        Vector2I size = LocalViewport!.Size;
        double aspect = size.Y > 0 ? (double)size.X / size.Y : 1;
        Local.Material.SetShaderParameter("aspect", (float)aspect);
        if (app.Framed && Shape != null)
        {
            // Local keeps the moving cells and the cells around them centred; the linked rotation turns them in 3D.
            LocalFrame frame = Shape.Frame(centres, focus, app.View.Spin, aspect);
            ShaderMaterial m = Local.Material;
            m.SetShaderParameter("q", LookViewport.Matrix(frame.Q));
            m.SetShaderParameter("projection_kind", (int)frame.Projection);
            m.SetShaderParameter("cs", (float)frame.Cs); m.SetShaderParameter("ss", (float)frame.Ss);
            m.SetShaderParameter("d4", (float)frame.D4); m.SetShaderParameter("zoom", (float)frame.Zoom);
            m.SetShaderParameter("focus_axis", LookViewport.Vector4(frame.Axis)); m.SetShaderParameter("focus_cos", (float)frame.CapCos);
            m.SetShaderParameter("focus_spread", (float)frame.Spread);
            m.SetShaderParameter("ghost", (float)Shape.Ghost);
            Local.SetViewFrame(FocusFrame.Neutral);
            // Global is fitted by scale alone, in both directions: the whole layout stays inside its pane, and the
            // field of view can only make it smaller.
            Vector2 pane = app.View.GetViewport().GetVisibleRect().Size;
            StructureValues global = app.State.Preset.Params.Structure(1);
            double reach = global.Projection switch
            {
                ProjectionKind.HopfRings => ringReach + .35, ProjectionKind.RingSheet => sheetReach + .35,
                ProjectionKind.Perspective => global.D4 / Math.Sqrt(global.D4 * global.D4 - 1) + .05, ProjectionKind.Orthographic => 1.05,
                _ => 0,
            };
            double fit = reach > 0 && reach < 4.9 ? GlobalFill * Math.Sqrt(25 - reach * reach) / reach : global.Zoom;
            app.View.Material.SetShaderParameter("zoom", (float)(Math.Min(global.Zoom, fit) * Math.Min(1, pane.Y > 0 ? pane.X / pane.Y : 1)));
        }
        else if (FramingEnabled) Local.SetViewFrame(follower.Step(Target(), dt));
    }

    public void Detach()
    {
        app.GlobalStage.GuiInput -= GlobalInput; app.View.FrameStepped -= Step; app.Ready -= Ready;
        if (LocalStage != null) LocalStage.GuiInput -= LocalInput;
    }
}
