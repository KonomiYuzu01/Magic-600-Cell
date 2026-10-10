using System;
using System.Buffers.Binary;
using System.Diagnostics;
using System.Linq;
using Godot;
using LookLab.Core;
using LabColour = LookLab.Core.Colour;

namespace LookLab.App;

internal sealed partial class LookViewport : Node3D
{
    public const int LabelWidth = 1024;
    public const int LabelHeight = (Geometry.Slots + LabelWidth - 1) / LabelWidth;
    public Geometry Geometry { get; }
    public TurnData Turn { get; }
    public MultiMesh Instances { get; }
    public ShaderMaterial Material { get; }
    public Camera3D Camera { get; }
    public Godot.Environment Environment { get; }
    public double[] Q { get; } = Geometry.Identity();
    private readonly LookState state;
    private readonly Stopwatch stopwatch = Stopwatch.StartNew();
    private TurnClock clock;
    private long revision = -1;
    private readonly ImageTexture labels;
    private readonly ImageTexture palette;
    private Vector2 velocity;
    public bool Animate { get; set; } = true;
    public Action<string>? Status { get; set; }

    public LookViewport(string root, LookState state)
    {
        this.state = state;
        Geometry = new Geometry(root);
        Turn = new TurnData(System.IO.Path.Combine(root, "tools/looklab/fixtures/w3-turn.json"));
        if (Turn.ModelId != Geometry.ModelId) throw new Exception("turn model does not match verified geometry");
        clock = NewClock();
        Material = new ShaderMaterial { Shader = GD.Load<Shader>("res://shaders/cells.gdshader") };
        Material.SetShaderParameter("geometry_check", false);
        Material.SetShaderParameter("transpose_q", false);
        // The colour vision preview is off until LL5's controls select a mode; the geometry check reads these too.
        Material.SetShaderParameter("cvd_enabled", false);
        Material.SetShaderParameter("cvd_r0", new Godot.Vector4(1, 0, 0, 0));
        Material.SetShaderParameter("cvd_r1", new Godot.Vector4(0, 1, 0, 0));
        Material.SetShaderParameter("cvd_r2", new Godot.Vector4(0, 0, 1, 0));
        Material.SetShaderParameter("geometry_width", 300);
        Material.SetShaderParameter("q", Matrix(Q));
        Material.SetShaderParameter("aspect", 1.6f);
        Material.SetShaderParameter("base_normal", Vector4(Geometry.BaseNormal.Span));
        Material.SetShaderParameter("radius", (float)Geometry.Radius);
        Material.SetShaderParameter("anchors", FloatTexture(Geometry.StickerAnchors.Span, Geometry.StickersPerCell, 1, Image.Format.Rgbaf));
        Material.SetShaderParameter("plane_u", Vector4(Turn.PlaneU.Span));
        Material.SetShaderParameter("plane_v", Vector4(Turn.PlaneV.Span));
        var moving = new double[LabelWidth * LabelHeight];
        for (int slot = 0; slot < Geometry.Slots; slot++) moving[slot] = Turn.IsMoving(slot) ? 1 : 0;
        Material.SetShaderParameter("moving_slots", FloatTexture(moving, LabelWidth, LabelHeight, Image.Format.Rf));
        var orbits = new double[LabelWidth * LabelHeight];
        for (int slot = 0; slot < Geometry.Slots; slot++) orbits[slot] = Geometry.SlotOrbits.Span[slot];
        Material.SetShaderParameter("slot_orbits", FloatTexture(orbits, LabelWidth, LabelHeight, Image.Format.Rf));
        Material.SetShaderParameter("cell_rings", FloatTexture(Geometry.CellStructure.RingOf.Select(ring => (double)ring).ToArray(), Geometry.Cells, 1, Image.Format.Rf));
        labels = FloatTexture(new double[LabelWidth * LabelHeight], LabelWidth, LabelHeight, Image.Format.Rf);
        palette = FloatTexture(new double[8 * 4], 8, 1, Image.Format.Rgbaf);
        Material.SetShaderParameter("labels", labels);
        Material.SetShaderParameter("palette", palette);
        var vertices = new Vector3[Geometry.BaseVertices];
        var uv = new Vector2[Geometry.BaseVertices];
        var uv2 = new Vector2[Geometry.BaseVertices];
        for (int vi = 0; vi < vertices.Length; vi++)
        {
            ReadOnlySpan<double> v = Geometry.BaseVertexData.Span.Slice(vi * 4, 4);
            vertices[vi] = new Vector3((float)v[0], (float)v[1], (float)v[2]);
            uv[vi] = new Vector2((float)v[3], Geometry.BaseStickerIds.Span[vi]);
            uv2[vi] = new Vector2(vi, vi % 3);
        }
        var arrays = new Godot.Collections.Array(); arrays.Resize((int)Mesh.ArrayType.Max);
        arrays[(int)Mesh.ArrayType.Vertex] = vertices;
        arrays[(int)Mesh.ArrayType.TexUV] = uv;
        arrays[(int)Mesh.ArrayType.TexUV2] = uv2;
        var mesh = new ArrayMesh(); mesh.AddSurfaceFromArrays(Mesh.PrimitiveType.Triangles, arrays);
        mesh.SurfaceSetMaterial(0, Material);
        Instances = new MultiMesh
        {
            TransformFormat = MultiMesh.TransformFormatEnum.Transform3D, UseCustomData = true,
            Mesh = mesh, InstanceCount = Geometry.Cells, VisibleInstanceCount = Geometry.Cells,
            CustomAabb = new Aabb(new Vector3(-100, -100, -100), new Vector3(200, 200, 200))
        };
        for (int cell = 0; cell < Geometry.Cells; cell++)
        {
            ReadOnlySpan<double> f = Geometry.CellFrames.Span.Slice(cell * 16, 16);
            Instances.SetInstanceTransform(cell, new Transform3D(new Basis(
                new Vector3((float)f[0], (float)f[1], (float)f[2]),
                new Vector3((float)f[4], (float)f[5], (float)f[6]),
                new Vector3((float)f[8], (float)f[9], (float)f[10])),
                new Vector3((float)f[12], (float)f[13], (float)f[14])));
            Instances.SetInstanceCustomData(cell, new Color((float)f[3], (float)f[7], (float)f[11], (float)f[15]));
        }
        AddChild(new MultiMeshInstance3D { Multimesh = Instances, IgnoreOcclusionCulling = true });
        Camera = new Camera3D { Position = new Vector3(0, 0, 5), Current = true, Near = .05f, Far = 100f,
            Attributes = new CameraAttributesPractical() };
        AddChild(Camera);
        Environment = new Godot.Environment { BackgroundMode = Godot.Environment.BGMode.Color,
            TonemapMode = Godot.Environment.ToneMapper.Linear, AmbientLightSource = Godot.Environment.AmbientSource.Disabled };
        AddChild(new WorldEnvironment { Environment = Environment });
        state.Changed += ParameterChanged;
        ApplyLook(); Bind(clock.Frame());
    }

    private TurnClock NewClock() => new(Turn, state.Preset.Params.Number("turnMs"), state.Preset.Params.Number("easeA"),
        state.Preset.Params.Number("easeB"), () => stopwatch.Elapsed.TotalMilliseconds);

    private void ParameterChanged(string id, ParameterValue value)
    {
        if (id is "turnMs" or "easeA" or "easeB") { clock = NewClock(); revision = -1; }
        ApplyLook();
    }

    public void ApplyLook()
    {
        ParameterSet p = state.Preset.Params;
        StructureValues structure = p.Structure(1);
        Material.SetShaderParameter("cs", (float)structure.Cs);
        Material.SetShaderParameter("ss", (float)structure.Ss);
        Material.SetShaderParameter("d4", (float)structure.D4);
        Material.SetShaderParameter("zoom", (float)structure.Zoom);
        Material.SetShaderParameter("projection_kind", (int)structure.Projection);
        foreach (string id in new[] { "edgeWeight", "edgeBrightness", "gloss", "glow", "fog", "keyLight", "fillLight", "sliceMin", "sliceMax" })
            Material.SetShaderParameter(id, (float)p.Number(id));
        foreach (string id in new[] { "cellsVisible", "sliceVisible" }) Material.SetShaderParameter(id, p.Boolean(id));
        Material.SetShaderParameter("finish_kind", state.Schema["finish"].Options!.ToList().IndexOf(p.Choice("finish")));
        Material.SetShaderParameter("highlight_kind", state.Schema["highlight"].Options!.ToList().IndexOf(p.Choice("highlight")));
        Material.SetShaderParameter("accent", ToColor(LabColour.MapToGamut(p.Colour("accentPrimary").L, p.Colour("accentPrimary").C, p.Colour("accentPrimary").H).Linear));
        GamutColour[] colours = LabColour.Palette(p);
        var bytes = new double[8 * 4];
        for (int i = 0; i < colours.Length; i++)
        {
            double[] rgb = colours[i].Ok ? colours[i].Linear : new[] { 1.0, 0.0, 1.0 };
            Array.Copy(rgb, 0, bytes, i * 4, 3); bytes[i * 4 + 3] = 1;
        }
        palette.Update(FloatImage(bytes, 8, 1, Image.Format.Rgbaf));
        int[] rings = CellStructure.ProperColouring(20, Geometry.CellStructure.RingGraph, p.Integer("classes"))!;
        Material.SetShaderParameter("cell_classes", FloatTexture(Enumerable.Range(0, Geometry.Cells)
            .Select(cell => (double)rings[Geometry.CellStructure.RingOf[cell]]).ToArray(), Geometry.Cells, 1, Image.Format.Rf));
        GamutColour background = LabColour.Background(p);
        Color bg = ToColor(background.Linear);
        Material.SetShaderParameter("background", bg); Environment.BackgroundColor = bg.LinearToSrgb();
        Environment.SsaoEnabled = p.Number("ambientOcclusion") > 0;
        Environment.SsaoIntensity = (float)p.Number("ambientOcclusion") * 2;
        var attributes = (CameraAttributesPractical)Camera.Attributes;
        attributes.DofBlurFarEnabled = p.Number("depthOfField") > 0;
        attributes.DofBlurFarDistance = 5;
        attributes.DofBlurFarTransition = 1;
        attributes.DofBlurAmount = (float)p.Number("depthOfField") * .1f;
        Status?.Invoke(colours.Any(c => !c.Ok) ? "Palette lightness is outside the sRGB gamut." : "Full detail: 259,800 slots · cost not measured");
    }

    public override void _Process(double delta)
    {
        if (Animate) Bind(clock.Frame());
        if (velocity.LengthSquared() > 1e-8f)
        {
            double damping = state.Preset.Params.Number("cameraDamping");
            BezierCurve settle = state.Preset.Params.Curve("settle");
            double response = Easing.Ease(Math.Min(1, delta * (8 + 24 * damping)), settle.A, settle.B);
            Geometry.Rotate(Q, 0, 3, velocity.X * delta);
            Geometry.Rotate(Q, 1, 3, velocity.Y * delta);
            velocity *= (float)(1 - response);
        }
        Vector2 size = GetViewport().GetVisibleRect().Size;
        Material.SetShaderParameter("aspect", size.Y > 0 ? size.X / size.Y : 1);
        Material.SetShaderParameter("q", Matrix(Q));
    }

    public void Drag(Vector2 relative)
    {
        double inertia = state.Preset.Params.Number("inertia");
        Geometry.Rotate(Q, 0, 3, relative.X * .004 * (1 - inertia));
        Geometry.Rotate(Q, 1, 3, relative.Y * .004 * (1 - inertia));
        velocity = relative * (float)(inertia * .3);
    }

    public override void _UnhandledInput(InputEvent input)
    {
        if (input is InputEventMouseMotion motion && (motion.ButtonMask & MouseButtonMask.Left) != 0) Drag(motion.Relative);
    }

    public void Bind(TurnFrame frame)
    {
        Turn.ValidateBinding(frame.BoundRevision, frame.Labels.Span);
        Material.SetShaderParameter("theta", (float)frame.Theta);
        if (revision == frame.BoundRevision) return;
        var values = new double[LabelWidth * LabelHeight];
        for (int slot = 0; slot < Geometry.Slots; slot++) values[slot] = frame.Labels.Span[slot];
        labels.Update(FloatImage(values, LabelWidth, LabelHeight, Image.Format.Rf));
        revision = frame.BoundRevision;
    }

    public static Vector4 Vector4(ReadOnlySpan<double> v) => new((float)v[0], (float)v[1], (float)v[2], (float)v[3]);
    public static Godot.Projection Matrix(ReadOnlySpan<double> q) => new(Vector4(q[..4]), Vector4(q.Slice(4, 4)), Vector4(q.Slice(8, 4)), Vector4(q.Slice(12, 4)));
    private static Color ToColor(double[] rgb) => rgb.Length == 3 ? new Color((float)rgb[0], (float)rgb[1], (float)rgb[2]) : Colors.Magenta;
    public static Image FloatImage(ReadOnlySpan<double> values, int width, int height, Image.Format format)
    {
        var bytes = new byte[values.Length * 4];
        for (int i = 0; i < values.Length; i++) BinaryPrimitives.WriteSingleLittleEndian(bytes.AsSpan(i * 4, 4), (float)values[i]);
        return Image.CreateFromData(width, height, false, format, bytes);
    }
    public static ImageTexture FloatTexture(ReadOnlySpan<double> values, int width, int height, Image.Format format) => ImageTexture.CreateFromImage(FloatImage(values, width, height, format));
    public override void _ExitTree() => state.Changed -= ParameterChanged;
}
