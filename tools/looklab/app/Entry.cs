using System;
using System.Text.Json;
using Godot;

namespace LookLab.App;

public partial class Entry : Node
{
    public override async void _Ready()
    {
        string[] args = OS.GetCmdlineUserArgs();
        string mode = args.Length > 2 ? args[2] : "interactive";
        string fault = args.Length > 3 ? args[3] : "";
        try
        {
            if (mode == "stage0-cpu")
            {
                GD.Print("LOOKLAB_STAGE0_CPU_PASS");
                GetTree().Quit();
                return;
            }
            if (mode is not ("stage0-gpu" or "geometry"))
            {
                if (args.Length < 2) throw new Exception("expected checkout root and scratch folder from harness");
                var app = new LabApp(args[0]); AddChild(app);
                if (mode == "smoke") { AppSmoke.Run(app, args[1], fault); GetTree().Quit(); }
                return;
            }
            if (mode == "stage0-gpu") AddChild(new Label { Text = "Look Lab staged probe", Position = new Vector2(24, 24) });
            GetWindow().Mode = Window.ModeEnum.Windowed;
            GetWindow().Show();
            GD.Print("LOOKLAB_GRAPHICS " + JsonSerializer.Serialize(new
            {
                driver = RenderingServer.GetCurrentRenderingDriverName().ToString(),
                display = DisplayServer.GetName(), adapter = RenderingServer.GetVideoAdapterName()
            }));
            if (DisplayServer.GetName().ToLowerInvariant() != "windows" ||
                RenderingServer.GetVideoAdapterName().Trim().Length == 0 ||
                (RenderingServer.GetCurrentRenderingDriverName() != "vulkan" && RenderingServer.GetCurrentRenderingDriverName() != "d3d12"))
                throw new Exception("graphics-environment: Windows display and real driver required");
            await ToSignal(GetTree(), SceneTree.SignalName.ProcessFrame);
            if (GetWindow().Mode == Window.ModeEnum.Minimized || !GetWindow().Visible)
                throw new Exception("graphics-environment: the check window must remain visible");
            await ToSignal(RenderingServer.Singleton, RenderingServer.SignalName.FramePostDraw);
            if (mode == "stage0-gpu") { FloatProbe.Check(fault == "float-offset"); GetTree().Quit(); }
            else { await ShaderGeometryCheck.Run(this, args[0], fault); }
        }
        catch (Exception ex)
        {
            GD.Print("LOOKLAB_FAIL " + ex.Message);
            GetTree().Quit(1);
        }
    }
}
