using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using LookLab.Core;
using LookLab.App.LL4;
using LookLab.App.LL5;
using LabColour = LookLab.Core.Colour;

namespace LookLab.App.LL6;

internal static class LL6Checks
{
    public static bool Handles(string mode) => mode is "ll6-views" or "ll6-link" or "ll6-colour" or "ll6-framing";

    public static void Run(LabApp app, string mode, string fault)
    {
        app.View.SetProcess(false);
        if (mode == "ll6-views") Views(app, fault);
        else if (mode == "ll6-link") Link(app, fault);
        else if (mode == "ll6-colour") Colour(app, fault);
        else Framing(app, fault);
    }

    private static void Require(bool condition, string check, string message)
    {
        if (!condition) throw new Exception(check + ": " + message);
    }

    private static void Select(LabApp app, string id) => app.State.Set("layoutId", new EnumValue(id));
    private static GreyboxControls Greybox(LabApp app) => (GreyboxControls)app.FindChild("LL4Controls", true, false);
    private static void Rect(Control actual, RegionRect expected, string check)
    {
        Require(actual.Position.IsEqualApprox(new Vector2((float)expected.X, (float)expected.Y)) &&
            actual.Size.IsEqualApprox(new Vector2((float)expected.Width, (float)expected.Height)), check, "container rectangle differs from core arrangement");
    }

    private static IEnumerable<Node> Descendants(Node node)
    {
        foreach (Node child in node.GetChildren())
        {
            yield return child;
            foreach (Node nested in Descendants(child)) yield return nested;
        }
    }

    private static void Views(LabApp app, string fault)
    {
        string check = fault is "ll6-local-hidden" or "ll6-panel-covers-local" ? fault : "ll6-views";
        app.Linked.LocalHidden = fault == "ll6-local-hidden";
        app.Linked.ReservePanel = fault != "ll6-panel-covers-local";
        Require(app.Linked.Local == null, check, "Local must be created lazily");
        int arrangements = 0;
        foreach (Vector2 size in new[] { new Vector2(1280, 720), new Vector2(1920, 1080) })
            foreach (bool greybox in new[] { false, true })
                foreach (string placement in new[] { "overlay", "docked" })
                    foreach (string id in app.State.Schema["layoutId"].Options!)
                    {
                        app.Size = size; Greybox(app).Toggle.ButtonPressed = greybox;
                        app.State.Set("panelPlacement", new EnumValue(placement)); Select(app, id);
                        LayoutSpec layout = app.Linked.Layout;
                        bool paired = layout.Regions.Values.Any(r => r.Kind == "local");
                        Require((app.Linked.LocalStage?.Visible == true) == paired, check, "Local visibility differs in " + id);
                        Require(!app.View.IsProcessingUnhandledInput(), check, "Global input can rotate twice");
                        float panelWidth = Math.Min(size.X * .65f, Math.Max(300, size.X * (float)(1 - app.State.Preset.Params.Number("viewportShare"))));
                        bool docked = placement == "docked" ||
                            layout.Regions.Values.Any(r => r.Kind == "panel" && r.Placement == "docked" || r.Kind == "local");
                        RegionRect stage = greybox ? new RegionRect((size.X - panelWidth) / 2, 0, (size.X - panelWidth) / 2, size.Y) :
                            new RegionRect(0, 0, docked ? size.X - panelWidth : size.X, size.Y);
                        ViewArrangement expected = ViewFraming.Arrange(layout, stage);
                        if (paired)
                        {
                            // Global and Local stay visible together: the parameter panel covers neither.
                            Rect2 panel = app.ParameterPanel.GetRect();
                            foreach ((string label, Control view) in new (string, Control)[] { ("Local", app.Linked.LocalStage!), ("Global", app.GlobalStage) })
                                Require(!view.GetRect().Grow(-.5f).Intersects(panel), check, $"parameter panel covers {label} in {id} ({placement})");
                        }
                        Rect(app.GlobalStage, expected.Global, check);
                        if (paired)
                        {
                            Rect(app.Linked.LocalStage!, expected.Local!.Value, check);
                            Require(app.Linked.LocalViewport!.RenderTargetUpdateMode == SubViewport.UpdateMode.Always &&
                                !app.Linked.Local!.Animate && !app.Linked.Local.IsProcessing() && !app.Linked.Local.IsProcessingUnhandledInput(), check,
                                $"Local update={app.Linked.LocalViewport.RenderTargetUpdateMode} animate={app.Linked.Local!.Animate} processing={app.Linked.Local.IsProcessing()} in {id}");
                            if (expected.LocalOverlay) Require(app.Linked.LocalStage!.GetIndex() > app.GlobalStage.GetIndex(), check, "inset is below Global");
                            foreach (string label in new[] { "Global", "Local" })
                                Require(Descendants(Greybox(app).Canvas).OfType<Label>().Any(l => l.Text.StartsWith(label + " (", StringComparison.Ordinal)), check, "greybox view label missing");
                        }
                        else if (app.Linked.LocalViewport != null)
                            Require(app.Linked.LocalViewport.RenderTargetUpdateMode == SubViewport.UpdateMode.Disabled, check, "hidden Local is rendering");
                        arrangements++;
                    }
        app.State.Set("panelPlacement", new EnumValue("overlay"));
        foreach (string id in new[] { "central-stage", "docked-workbench" })
        {
            Greybox(app).Toggle.ButtonPressed = false; Select(app, id);
            Require(app.Linked.LocalStage?.Visible == false && app.Linked.LocalViewport!.RenderTargetUpdateMode == SubViewport.UpdateMode.Disabled,
                check, "switch back did not hide and disable Local");
            Rect(app.GlobalStage, app.Linked.StageRect, check);
        }
        GD.Print($"LOOKLAB_LL6_VIEWS_PASS layouts=4 sizes=2 modes=2 placements=2 arrangements={arrangements} hidden=2 checks=1");
    }

    private static void Shared(LabApp app, string check)
    {
        LookViewport global = app.View, local = app.Linked.Local!;
        Require(global.Q.SequenceEqual(local.Q), check, "Global and Local Q differ");
        Require(global.Material.GetShaderParameter("q").AsProjection() == local.Material.GetShaderParameter("q").AsProjection(), check, "bound Q differs");
        Require(global.BoundFrame.TurnIndex == local.BoundFrame.TurnIndex && global.BoundFrame.Theta == local.BoundFrame.Theta &&
            global.BoundFrame.BoundRevision == local.BoundFrame.BoundRevision && ReferenceEquals(global.BoundFrame, local.BoundFrame),
            check, "views did not bind the same turn frame");
    }

    private static void Link(LabApp app, string fault)
    {
        string check = fault is "ll6-local-drag-ignored" or "ll6-local-clock" ? fault : "ll6-link";
        app.Linked.LocalDragEnabled = fault != "ll6-local-drag-ignored";
        app.Linked.LocalClockEnabled = fault == "ll6-local-clock";
        Select(app, "split-views");
        app.State.Set("inertia", new NumberValue(.65));
        double now = 0; app.View.UseClock(() => now);
        double duration = app.State.Preset.Params.Number("turnMs");
        int drags = 0, steps = 0;
        foreach (SubViewportContainer pane in new[] { app.Linked.LocalStage!, app.GlobalStage })
        {
            Vector2 relative = new(32, -17);
            double[] expected = (double[])app.View.Q.Clone();
            Geometry.Rotate(expected, 0, 3, relative.X * .004 * .35); Geometry.Rotate(expected, 1, 3, relative.Y * .004 * .35);
            using var motion = new InputEventMouseMotion { Relative = relative, ButtonMask = MouseButtonMask.Left };
            pane.EmitSignal(Control.SignalName.GuiInput, motion);
            Require(expected.Zip(app.View.Q).All(pair => Math.Abs(pair.First - pair.Second) < 1e-12), check, "pane drag did not rotate Q exactly once");
            // The velocity set by that drag is integrated once by Global.
            Geometry.Rotate(expected, 0, 3, relative.X * .65 * .3 * .016); Geometry.Rotate(expected, 1, 3, relative.Y * .65 * .3 * .016);
            now += .016 * 1000; app.View._Process(.016); Shared(app, check);
            Require(expected.Zip(app.View.Q).All(pair => Math.Abs(pair.First - pair.Second) < 1e-8), check, "inertia did not advance exactly once on Global");
            drags++; steps++;
        }
        foreach (double time in new[] { .4, .9, 1.0, 1.1, 1.9, 2.0, 2.1 })
        {
            now = time * duration;
            double[] before = (double[])app.View.Q.Clone(); app.View._Process(.016); Shared(app, check);
            Require(!before.SequenceEqual(app.View.Q), check, "inertia stopped prematurely");
            Require(app.View.BoundFrame.TurnIndex == (long)Math.Floor(time), check, "stepped clock did not cross turn boundary");
            steps++;
        }
        GD.Print($"LOOKLAB_LL6_LINK_PASS drags={drags} steps={steps} boundaries=2 checks=1");
    }

    private static void Colour(LabApp app, string fault)
    {
        string check = fault == "ll6-global-only-cvd" ? fault : "ll6-colour";
        CvdPreview preview = app.Panels.GetChildren().OfType<LabControls>().Single().Preview;
        preview.GlobalOnly = fault == "ll6-global-only-cvd";
        preview.SelectMode("protan"); Select(app, "focus-inset"); // A new Local must inherit a mode already selected.
        void Same()
        {
            foreach (LookViewport view in new[] { app.View, app.Linked.Local! })
            {
                Require(view.Material.GetShaderParameter("cvd_enabled").AsBool() == (preview.Mode != "normal"), check, "CVD mode differs");
                float[][] rows = CvdTransform.Rows(preview.Mode);
                for (int i = 0; i < 3; i++)
                {
                    Vector4 row = view.Material.GetShaderParameter("cvd_r" + i).AsVector4();
                    Require(row == new Vector4(rows[i][0], rows[i][1], rows[i][2], 0), check, "CVD coefficients differ");
                }
                Color background = view.Environment.BackgroundColor.SrgbToLinear();
                CvdCheckData.Require(new[] { background.R, background.G, background.B },
                    CvdCheckData.Reference(LabColour.Background(app.State.Preset.Params).Linear, preview.Mode), check);
            }
            Require(app.View.Environment.BackgroundColor == app.Linked.Local!.Environment.BackgroundColor, check, "clear colours differ");
        }
        Same(); int states = 0;
        foreach (string mode in LabColour.Modes)
        {
            preview.SelectMode(mode); Same(); states++;
            app.State.Set("bgHue", new NumberValue(30 + states * 20)); Same(); states++;
            Select(app, "central-stage"); Select(app, "split-views"); Same(); states++;
        }
        GD.Print($"LOOKLAB_LL6_COLOUR_PASS modes={LabColour.Modes.Count} states={states} late=passed checks=1");
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
        string check = fault == "ll6-neutral-local-frame" ? fault : "ll6-framing";
        app.Linked.FramingEnabled = fault != "ll6-neutral-local-frame";
        Select(app, "split-views"); app.View.Animate = false;
        const double dt = 1.0 / 60;
        app.View._Process(dt);
        Geometry.Rotate(app.View.Q, 0, 3, .4); Geometry.Rotate(app.View.Q, 1, 3, -.2);
        for (int i = 0; i < 180; i++) app.View._Process(dt);
        Vector2I size = app.Linked.LocalViewport!.Size;
        FocusFrame target = ViewFraming.Target(ViewFraming.MovingRestAnchors(app.View.Geometry, app.View.Turn, app.State.Preset.Params.Number("gap")),
            app.View.Q, app.State.Preset.Params.Structure(size.Y > 0 ? (float)size.X / size.Y : 1));
        Require(!Near(target, FocusFrame.Neutral, .1), check, "fixed camera did not exercise non-neutral framing");
        Require(Near(UniformFrame(app.Linked.Local!), target, 1e-6), check, "Local did not settle to the core target");
        int steps = 0;
        for (int i = 0; i < 120; i++)
        {
            FocusFrame before = app.Linked.Frame;
            Geometry.Rotate(app.View.Q, 0, 3, .025); Geometry.Rotate(app.View.Q, 1, 3, .013); app.View._Process(dt);
            FocusFrame after = app.Linked.Frame;
            Require(Math.Abs(after.X - before.X) <= 2 * dt + 1e-12 && Math.Abs(after.Y - before.Y) <= 2 * dt + 1e-12 &&
                Math.Abs(after.M - before.M) <= 4 * dt + 1e-12, check, "Local exceeded follower cap");
            Require(Near(UniformFrame(app.Linked.Local!), after, 1e-6), check, "Local uniform differs from follower");
            Require(UniformFrame(app.View) == FocusFrame.Neutral, check, "Global frame is not neutral"); steps++;
        }
        GD.Print($"LOOKLAB_LL6_FRAMING_PASS settled=1e-6 steps={steps} caps=passed global=neutral checks=1");
    }
}
