using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Godot;
using LookLab.Core;
using LookLab.App.LL3;
using LookLab.App.LL5;

namespace LookLab.App.LL7;

internal static class LL7Checks
{
    public static bool Handles(string mode) => mode is "ll7-presets" or "ll7-spin" or "ll7-layout" or "ll7-framing";

    public static void Run(LabApp app, string root, string mode, string fault)
    {
        app.View.SetProcess(false);
        if (mode == "ll7-presets") PresetBindings(app, root, fault);
        else if (mode == "ll7-spin") Spin(app, root, fault);
        else if (mode == "ll7-layout") Layout(app, root, fault);
        else Framing(app, fault);
    }

    private static void Require(bool condition, string check, string message)
    {
        if (!condition) throw new Exception(check + ": " + message);
    }

    private static void PresetBindings(LabApp app, string root, string fault)
    {
        string check = fault == "ll7-projection-clamped" ? fault : "ll7-presets";
        app.View.LayoutKindsBound = fault != "ll7-projection-clamped";
        var source = new ShaderSource(Godot.FileAccess.GetFileAsString("res://shaders/cells.gdshader"));
        string[] files = Directory.GetFiles(Path.Combine(root, "tools/looklab/presets"), "g1-*.json").OrderBy(p => p, StringComparer.Ordinal).ToArray();
        var kinds = new HashSet<int>();
        foreach (string file in files)
        {
            Preset preset = Presets.Load(file, app.State.Schema); app.State.Load(preset);
            ShaderMaterial material = app.View.Material;
            int kind = (int)preset.Params.Structure(1).Projection; kinds.Add(kind);
            Require(material.GetShaderParameter("projection_kind").AsInt32() == kind, check, "preset projection was not bound: " + preset.Name);
            Require(material.GetShaderParameter("spin").AsProjection() == LookViewport.Matrix(app.View.Spin), check, "Spin was not bound");
            foreach (var (name, value) in new[] { ("turn_angle", app.View.Turn.Angle), ("hemisphere_offset", GlobalLayout.HemisphereOffset),
                ("hemisphere_scale", GlobalLayout.HemisphereScale), ("hemisphere_clamp", GlobalLayout.HemisphereClamp) })
                Require(material.GetShaderParameter(name).AsDouble() == (float)value, check, name + " differs from core");
            Require(material.GetShaderParameter("cell_layout").AsGodotObject() is ImageTexture texture &&
                texture.GetWidth() == 600 && texture.GetHeight() == 10 && texture.GetFormat() == Image.Format.Rgbaf, check, "cell_layout must be 600 x 10 RGBAF");
            foreach (ShaderUniform uniform in source.Uniforms.Where(u => u.Type != "sampler2D"))
                Require(material.GetShaderParameter(uniform.Name).VariantType != Variant.Type.Nil, check, "unbound drawing uniform: " + uniform.Name);
        }
        Require(files.Length == 6 && kinds.OrderBy(k => k).SequenceEqual(Enumerable.Range(0, 6)), check, "G1 presets must cover every kind once");
        GD.Print("LOOKLAB_LL7_PRESETS_PASS presets=6 kinds=6 uniforms=bound checks=1");
    }

    private static void Spin(LabApp app, string root, string fault)
    {
        string check = fault == "ll7-compare-spin-not-copied" ? fault : "ll7-spin";
        app.View.Animate = false; app.State.Set("inertia", new NumberValue(0));
        Vector2 relative = new(30, -20);
        double[] q = app.View.Q.ToArray(), spin = app.View.Spin.ToArray();
        GlobalLayout.Drag(q, spin, relative.X * .004, relative.Y * .004);
        app.View.Drag(relative);
        Require(app.View.Q.SequenceEqual(q) && app.View.Spin.SequenceEqual(spin) && !spin.SequenceEqual(Geometry.Identity()), check, "drag did not turn Q and Spin exactly");
        app.State.Set("inertia", new NumberValue(.65)); app.View.Drag(relative);
        q = app.View.Q.ToArray(); spin = app.View.Spin.ToArray();
        Vector2 velocity = relative * (float)(.65 * .3);
        const double dt = 1.0 / 60;
        double[] expectedQ = q.ToArray(), expectedSpin = spin.ToArray();
        GlobalLayout.Drag(expectedQ, expectedSpin, velocity.X * dt, velocity.Y * dt);
        app.View._Process(dt);
        Require(!app.View.Q.SequenceEqual(q) && !app.View.Spin.SequenceEqual(spin) &&
            app.View.Q.SequenceEqual(expectedQ) && app.View.Spin.SequenceEqual(expectedSpin), check, "inertia did not turn both matrices exactly");
        app.Size = new Vector2(1280, 720); app.State.Set("layoutId", new EnumValue("split-views")); app.View._Process(dt);
        LookViewport local = app.Linked.Local!;
        Require(local.Q.SequenceEqual(app.View.Q) && local.Spin.SequenceEqual(app.View.Spin), check, "Local did not copy Q and Spin");
        foreach (string name in new[] { "q", "spin" })
            Require(local.Material.GetShaderParameter(name).AsProjection() == app.View.Material.GetShaderParameter(name).AsProjection(), check, "Local binding differs: " + name);

        Preset[] presets = new[] { "g1-hopf-rings.json", "g1-perspective.json" }
            .Select(name => Presets.Load(Path.Combine(root, "tools/looklab/presets", name), app.State.Schema)).ToArray();
        var stage = new CompareStage(root, app.State.Schema, presets, () => 0) { CopySpin = fault != "ll7-compare-spin-not-copied" };
        app.AddChild(stage); stage.SetProcess(false);
        q = Geometry.Identity(); spin = Geometry.Identity(); GlobalLayout.Drag(q, spin, 25 * .004, 10 * .004);
        stage.Session.Camera.Drag(25, 10);
        Require(stage.Session.Camera.Q.SequenceEqual(q) && stage.Session.Camera.Spin.SequenceEqual(spin), check, "compare drag differs from core");
        stage.Step();
        foreach (LookViewport view in stage.Views)
            Require(view.Q.SequenceEqual(q) && view.Spin.SequenceEqual(spin) &&
                view.Material.GetShaderParameter("q").AsProjection() == LookViewport.Matrix(q) &&
                view.Material.GetShaderParameter("spin").AsProjection() == LookViewport.Matrix(spin), check, "compare pane did not bind both matrices");
        app.RemoveChild(stage); stage.QueueFree();
        GD.Print("LOOKLAB_LL7_SPIN_PASS global=passed inertia=passed local=passed compare=2 checks=1");
    }

    private static byte[] Bytes(float[] values)
    {
        var bytes = new byte[values.Length * 4];
        for (int i = 0; i < values.Length; i++) BinaryPrimitives.WriteSingleLittleEndian(bytes.AsSpan(i * 4, 4), values[i]);
        return bytes;
    }

    private static byte[] TextureData(LookViewport view, string name, ReadOnlyMemory<byte> uploaded, string check, ref int readbacks, ref int uploads)
    {
        var texture = (Texture2D)view.Material.GetShaderParameter(name).AsGodotObject();
        using Image? image = texture.GetImage();
        if (image != null && !image.IsEmpty())
        {
            byte[] data = image.GetData();
            Require(data.AsSpan().SequenceEqual(uploaded.Span), check, name + " readback differs from its upload");
            readbacks++; return data;
        }
        uploads++; return uploaded.ToArray();
    }

    private static void Layout(LabApp app, string root, string fault)
    {
        string check = fault == "ll7-palette-old-colouring" ? fault : "ll7-layout";
        PalettePanel panel = app.Panels.GetChildren().OfType<LabControls>().Single().Palette;
        panel.UseBoundColouring = fault != "ll7-palette-old-colouring";
        LookViewport view = app.View; CellStructure structure = view.Geometry.CellStructure;
        int readbacks = 0, uploads = 0;
        byte[] layout = TextureData(view, "cell_layout", view.CellLayoutUpload, check, ref readbacks, ref uploads);
        Require(layout.SequenceEqual(Bytes(view.Layout.CellLayout(view.Turn))), check, "cell_layout differs from the core little-endian f32 data");
        foreach (string projection in new[] { "perspective", "hemispheres", "hopfRings", "ringSheet" })
            for (int classes = 4; classes <= 8; classes++)
            {
                app.State.Set("projection", new EnumValue(projection)); app.State.Set("classes", new IntegerValue(classes));
                VerifyColouring();
            }
        // Preset loads must refresh the same report as individual parameter edits.
        foreach (string name in new[] { "g1-hopf-rings.json", "g1-perspective.json" })
        {
            app.State.Load(Presets.Load(Path.Combine(root, "tools/looklab/presets", name), app.State.Schema)); VerifyColouring();
        }
        GD.Print($"LOOKLAB_LL7_TEXTURE readbacks={readbacks} uploads={uploads}");
        GD.Print("LOOKLAB_LL7_LAYOUT_PASS kinds=4 classes=5 texture=exact palette=passed checks=1");

        void VerifyColouring()
        {
            ParameterSet p = app.State.Preset.Params;
            CellColouring expected = view.Layout.Colouring(p, structure), actual = view.Colouring;
            Require(actual.RingOf.SequenceEqual(expected.RingOf) && actual.RingClass.SequenceEqual(expected.RingClass), check, "bound colouring differs from core");
            bool hopf = p.Structure(1).Projection is ProjectionKind.HopfRings or ProjectionKind.RingSheet;
            int[] ringOf = hopf ? view.Layout.Hopf.RingOf : structure.RingOf;
            int[] ringClass = (hopf ? GlobalLayout.EquitableColouring(20, view.Layout.Hopf.Graph, p.Integer("classes")) :
                CellStructure.ProperColouring(20, structure.RingGraph, p.Integer("classes")))!;
            Require(actual.RingOf.SequenceEqual(ringOf) && actual.RingClass.SequenceEqual(ringClass), check, "projection selected the wrong ring assignment");
            byte[] bytes = TextureData(view, "cell_classes", view.CellClassesUpload, check, ref readbacks, ref uploads);
            Require(bytes.Length == 600 * 4, check, "cell_classes has wrong length");
            var bound = new int[600];
            for (int cell = 0; cell < 600; cell++)
            {
                float value = BinaryPrimitives.ReadSingleLittleEndian(bytes.AsSpan(cell * 4, 4));
                Require(value == actual.ClassOf(cell), check, "cell_classes differs at cell " + cell); bound[cell] = (int)value;
            }
            var pairs = new HashSet<(int A, int B)>(); int sameRing = 0;
            for (int cell = 0; cell < 600; cell++) foreach (int neighbor in structure.FaceNeighbors[cell].Where(n => n > cell))
            {
                if (actual.RingOf[cell] == actual.RingOf[neighbor]) { sameRing++; continue; }
                pairs.Add((Math.Min(bound[cell], bound[neighbor]), Math.Max(bound[cell], bound[neighbor])));
            }
            Require(panel.Report.ClassPairs.SequenceEqual(pairs.OrderBy(pair => pair.A).ThenBy(pair => pair.B)) &&
                panel.Report.SameRingAdjacencies == sameRing, check, "palette report differs from the colouring drawn");
        }
    }

    private static FocusFrame UniformFrame(LookViewport view)
    {
        Vector4 value = view.Material.GetShaderParameter("view_frame").AsVector4();
        return new FocusFrame(value.X, value.Y, value.Z);
    }

    private static bool Near(FocusFrame a, FocusFrame b, double tolerance) =>
        Math.Abs(a.X - b.X) <= tolerance && Math.Abs(a.Y - b.Y) <= tolerance && Math.Abs(a.M - b.M) <= tolerance;

    private static void Framing(LabApp app, string fault)
    {
        string check = fault == "ll7-framing-ignores-spin" ? fault : "ll7-framing";
        app.Linked.FramingUsesSpin = fault != "ll7-framing-ignores-spin";
        app.Size = new Vector2(1280, 720); app.State.Set("layoutId", new EnumValue("split-views")); app.View.Animate = false;
        foreach (string projection in new[] { "hemispheres", "hopfRings", "ringSheet" })
        {
            app.State.Set("projection", new EnumValue(projection)); GlobalLayout.Drag(app.View.Q, app.View.Spin, .4, -.2);
            for (int i = 0; i < 180; i++) app.View._Process(1.0 / 60);
            Vector2I size = app.Linked.LocalViewport!.Size;
            FocusFrame target = ViewFraming.Target(ViewFraming.MovingRestAnchors(app.View.Geometry, app.View.Turn, app.State.Preset.Params.Number("gap")),
                ViewFraming.MovingRestCells(app.View.Turn), app.View.Q, app.View.Spin,
                app.State.Preset.Params.Structure(size.Y > 0 ? (float)size.X / size.Y : 1), app.View.Layout);
            Require(!Near(target, FocusFrame.Neutral, .1), check, "layout did not exercise non-neutral framing: " + projection);
            Require(Near(UniformFrame(app.Linked.Local!), target, 1e-6), check, "Local did not settle to the core target: " + projection);
            Require(UniformFrame(app.View) == FocusFrame.Neutral, check, "Global frame is not neutral");
        }
        GD.Print("LOOKLAB_LL7_FRAMING_PASS kinds=3 settled=1e-6 global=neutral checks=1");
    }
}
