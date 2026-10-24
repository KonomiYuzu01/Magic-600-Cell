using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using LookLab.Core;

namespace LookLab.App.LL8;

internal static class CaptureProbe
{
    public static async Task Run(Node host, string root, string fault)
    {
        var viewport = new SubViewport { Size = new Vector2I(64, 64), OwnWorld3D = true, TransparentBg = false,
            RenderTargetUpdateMode = SubViewport.UpdateMode.Always };
        var rect = new ColorRect { Position = Vector2.Zero, Size = new Vector2(64, 64) };
        host.AddChild(viewport); viewport.AddChild(rect);
        var decoded = new List<int>();
        byte[]? previous = null, first = null;
        try
        {
            for (int k = 0; k < 48; k++)
            {
                rect.Color = Color.Color8((byte)(5 * k), (byte)(255 - 5 * k), 90);
                using Image captured = await ShotPipeline.CaptureFrame(host, viewport, ShotManifest.Waits);
                byte[] current = captured.GetData(); first ??= current;
                byte[] data = fault == "ll8-capture-delayed" ? previous ?? current : fault == "ll8-capture-frozen" ? first : current;
                previous = current;
                int? value = null;
                foreach (var (x, y) in new[] { (0, 0), (63, 0), (0, 63), (63, 63), (32, 32) })
                {
                    int offset = (y * 64 + x) * 4, n = (int)Math.Round(data[offset] / 5.0);
                    if (n < 0 || n > 47 || Math.Abs(data[offset] - 5 * n) > 2 || Math.Abs(data[offset + 1] - (255 - 5 * n)) > 2 ||
                        Math.Abs(data[offset + 2] - 90) > 2 || (value.HasValue && value.Value != n))
                        throw new Exception("ll8-capture: frame " + k + " has inconsistent encoded pixels");
                    value = n;
                }
                decoded.Add(value!.Value);
            }
        }
        finally { host.RemoveChild(viewport); viewport.QueueFree(); }
        if (!decoded.SequenceEqual(Enumerable.Range(0, 48)))
        {
            if (decoded.Distinct().Count() == 1) throw new Exception((fault == "ll8-capture-frozen" ? fault : "ll8-capture") + ": readback is frozen");
            for (int d = 1; d < 48; d++)
                if (decoded.SequenceEqual(Enumerable.Range(0, 48).Select(k => Math.Max(0, k - d))))
                    throw new Exception((fault == "ll8-capture-delayed" ? fault : "ll8-capture") + $": readback lags by {d} frame(s)");
            throw new Exception("ll8-capture: readback out of order");
        }
        var schema = ParameterSchema.Load(Path.Combine(root, ShotManifest.ParametersPath));
        Preset preset = Presets.Load(Path.Combine(root, ShotManifest.PresetsFolder, "default.json"), schema);
        preset = preset with { Params = preset.Params.With("ambientOcclusion", new NumberValue(1)).With("depthOfField", new NumberValue(1)) };
        Shot shot = Shot.Load(Path.Combine(root, ShotManifest.ShotPath));
        var turn = new TurnData(Path.Combine(root, ShotManifest.TurnPath));
        try { using Image hero = await ShotPipeline.RenderPreset(host, root, schema, preset, shot, shot.Track(turn), ShotPipeline.GraphicsCapture(host)); }
        catch (ShotSpotException ex) { throw new Exception("ll8-capture: effects: " + ex.Message, ex); }
        GD.Print("LOOKLAB_LL8_CAPTURE_PASS frames=48 waits=3 effects=identical checks=1");
    }
}
