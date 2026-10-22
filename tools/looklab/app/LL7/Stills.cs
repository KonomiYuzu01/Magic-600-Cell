using System;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using LookLab.Core;
using LookLab.App.LL3;

namespace LookLab.App.LL7;

internal static class Stills
{
    public static async Task Run(Node host, string root)
    {
        var schema = ParameterSchema.Load(Path.Combine(root, "tools/looklab/data/parameters.json"));
        var capture = new CaptureService(host, root);
        string[] files = Directory.GetFiles(Path.Combine(root, "tools/looklab/presets"), "g1-*.json").OrderBy(p => p, StringComparer.Ordinal).ToArray();
        if (files.Length != 7) throw new Exception("stills: expected seven G1 presets");
        foreach (string file in files)
        {
            Preset preset = Presets.Load(file, schema);
            var viewport = new SubViewport { Size = new Vector2I(1600, 1000), OwnWorld3D = true, TransparentBg = false,
                RenderTargetUpdateMode = SubViewport.UpdateMode.Always };
            host.AddChild(viewport);
            var view = new LookViewport(root, new LookState(schema, preset), animate: false);
            viewport.AddChild(view); view.SetProcess(false); view.SetProcessUnhandledInput(false); view.UseClock(() => 0);
            var frame = new TurnClock(view.Turn, preset.Params.Number("turnMs"), preset.Params.Number("easeA"), preset.Params.Number("easeB"), () => 0).Frame();
            foreach (string pose in new[] { "rest", "mid" })
            {
                view.Bind(pose == "rest" ? frame : frame with { Theta = view.Turn.Angle / 2 });
                for (int i = 0; i < 2; i++) await host.ToSignal(RenderingServer.Singleton, RenderingServer.SignalName.FramePostDraw);
                string path = await capture.Still(viewport) ?? throw new Exception("stills: capture was disabled");
                GD.Print($"LOOKLAB_STILL preset={Path.GetFileNameWithoutExtension(file)} pose={pose} path={Path.GetRelativePath(root, path).Replace('\\', '/')}");
            }
            host.RemoveChild(viewport); viewport.QueueFree();
        }
        GD.Print("LOOKLAB_STILLS_PASS presets=7 stills=14 size=1600x1000");
        host.GetTree().Quit();
    }
}
