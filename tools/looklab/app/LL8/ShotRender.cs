using System;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using Godot;

namespace LookLab.App.LL8;

internal static class ShotRender
{
    public static async Task Run(Node host, string root, string[] stems)
    {
        ShotContext context = ShotContext.Parse(System.Environment.GetEnvironmentVariable("LOOKLAB_SHOT_CONTEXT"));
        var tools = new ShotTools(Engine.GetVersionInfo()["string"].AsString(), RuntimeInformation.FrameworkDescription,
            RenderingServer.GetCurrentRenderingDriverName().ToString(), RenderingServer.GetVideoAdapterName());
        ShotRun run = await ShotPipeline.Render(host, root, root, stems, context, tools, ShotPipeline.GraphicsCapture(host), line => GD.Print(line));
        GD.Print($"LOOKLAB_SHOT_RUN manifest={run.ManifestPath} formal={(run.Formal ? 1 : 0)}");
        GD.Print($"LOOKLAB_SHOT_PASS presets={stems.Length} stills={stems.Length} clips=0 frames={run.Shot.Frames} size={run.Shot.Width}x{run.Shot.Height}");
        host.GetTree().Quit();
    }
}
