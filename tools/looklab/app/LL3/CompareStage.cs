using System;
using System.Collections.Generic;
using System.Diagnostics;
using Godot;
using LookLab.Core;

namespace LookLab.App.LL3;

internal sealed partial class CompareStage : HBoxContainer
{
    public CompareSession Session { get; }
    public List<LookState> States { get; } = new();
    public List<LookViewport> Views { get; } = new();
    public List<SubViewport> Viewports { get; } = new();
    private readonly List<Label> captions = new();
    private readonly List<TurnClock> clocks = new();
    private readonly bool sharedTiming;
    private readonly Func<double> nowMs;
    private readonly Stopwatch timer = Stopwatch.StartNew();

    public CompareStage(string root, ParameterSchema schema, IReadOnlyList<Preset> presets,
        Func<double>? now = null, bool sharedTiming = true)
    {
        this.sharedTiming = sharedTiming; nowMs = now ?? (() => timer.Elapsed.TotalMilliseconds);
        SizeFlagsHorizontal = SizeFlags.ExpandFill; SizeFlagsVertical = SizeFlags.ExpandFill;
        foreach (Preset preset in presets)
        {
            var state = new LookState(schema, preset); States.Add(state);
            var view = new LookViewport(root, state) { Animate = false }; Views.Add(view);
            view.SetProcess(false); view.SetProcessUnhandledInput(false);
            var column = new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsVertical = SizeFlags.ExpandFill };
            AddChild(column);
            var caption = new Label { Text = preset.Name, AutowrapMode = TextServer.AutowrapMode.WordSmart }; captions.Add(caption); column.AddChild(caption);
            var container = new SubViewportContainer { Stretch = true, CustomMinimumSize = new Vector2(100, 140),
                SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsVertical = SizeFlags.ExpandFill, MouseFilter = MouseFilterEnum.Stop };
            column.AddChild(container);
            var viewport = new SubViewport { Size = new Vector2I(320, 240), OwnWorld3D = true, TransparentBg = false,
                RenderTargetUpdateMode = SubViewport.UpdateMode.Always, HandleInputLocally = false };
            Viewports.Add(viewport); container.AddChild(viewport); viewport.AddChild(view);
            container.GuiInput += PaneInput;
            clocks.Add(NewClock(view, preset));
        }
        Session = new CompareSession(schema, presets, Views[0].Turn, nowMs);
    }

    private TurnClock NewClock(LookViewport view, Preset preset) => new(view.Turn, preset.Params.Number("turnMs"),
        preset.Params.Number("easeA"), preset.Params.Number("easeB"), nowMs);

    private void PaneInput(InputEvent input)
    {
        if (input is InputEventMouseMotion motion && (motion.ButtonMask & MouseButtonMask.Left) != 0)
            Session.Camera.Drag(motion.Relative.X, motion.Relative.Y);
    }

    public void UpdatePresets(IReadOnlyList<Preset> presets)
    {
        if (presets.Count != States.Count) throw new FormatException("compare: pane count changed");
        for (int i = 0; i < presets.Count; i++)
        {
            States[i].Load(presets[i]); captions[i].Text = presets[i].Name;
            clocks[i] = NewClock(Views[i], presets[i]);
        }
    }

    public void Step()
    {
        Session.Advance();
        for (int i = 0; i < Views.Count; i++)
        {
            Views[i].Bind(sharedTiming ? Session.Panes[i].Frame : clocks[i].Frame());
            Array.Copy(Session.Camera.Q, Views[i].Q, 16);
            Views[i].Material.SetShaderParameter("q", LookViewport.Matrix(Session.Camera.Q));
            Vector2I size = Viewports[i].Size;
            Views[i].Material.SetShaderParameter("aspect", size.Y > 0 ? (float)size.X / size.Y : 1);
        }
    }

    public override void _Process(double delta) => Step();
}
