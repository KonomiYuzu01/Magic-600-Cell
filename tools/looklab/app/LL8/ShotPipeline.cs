using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using Godot;
using LookLab.Core;
using LookLab.App.LL3;
using File = System.IO.File;

namespace LookLab.App.LL8;

internal delegate Task<Image> ShotCapture(LookViewport view, SubViewport viewport, int frame, int waits);
internal enum ShotSpotKind { Stale, Frozen }
internal sealed class ShotSpotException : Exception
{
    public ShotSpotKind Kind { get; }
    public ShotSpotException(ShotSpotKind kind, string message) : base(message) => Kind = kind;
}
internal sealed record ShotRun(string Name, string ManifestPath, bool Formal, Shot Shot);

internal static class ShotPipeline
{
    public static async Task<Image> CaptureFrame(Node host, SubViewport viewport, int waits)
    {
        for (int i = 0; i < waits; i++) await host.ToSignal(RenderingServer.Singleton, RenderingServer.SignalName.FramePostDraw);
        return CaptureService.NativeImage(viewport, viewport.Size);
    }

    public static ShotCapture GraphicsCapture(Node host) => (_, viewport, _, waits) => CaptureFrame(host, viewport, waits);

    public static async Task<Image> RenderPreset(Node host, string inputRoot, ParameterSchema schema, Preset preset, Shot shot,
        ShotState[] track, ShotCapture capture, bool pushCamera = true)
    {
        var viewport = new SubViewport { Size = new Vector2I(shot.Width, shot.Height), OwnWorld3D = true, TransparentBg = false,
            RenderTargetUpdateMode = SubViewport.UpdateMode.Always };
        host.AddChild(viewport);
        Image? hero = null;
        bool success = false;
        try
        {
            var view = new LookViewport(inputRoot, new LookState(schema, preset), animate: false);
            viewport.AddChild(view); view.SetProcess(false); view.SetProcessUnhandledInput(false);
            async Task<Image> Capture(int k, int waits)
            {
                Array.Copy(track[k].Q, view.Q, 16); Array.Copy(track[k].Spin, view.Spin, 16);
                if (pushCamera) view.PushCamera(shot.Width / (double)shot.Height);
                view.Bind(track[k].Turn);
                Image image = await capture(view, viewport, k, waits);
                if (image.GetFormat() != Image.Format.Rgba8 || image.GetWidth() != shot.Width || image.GetHeight() != shot.Height)
                {
                    image.Dispose();
                    throw new Exception("shot: frame " + k + " must be RGBA8 at the shot size");
                }
                return image;
            }
            var hashes = new Dictionary<int, string>();
            for (int k = 0; k < shot.Frames; k++)
            {
                Image image = await Capture(k, ShotManifest.Waits);
                try
                {
                    if (shot.SpotFrames.Contains(k)) hashes.Add(k, AssetCatalogue.Digest(image.GetData()));
                    if (k == shot.HeroFrame) { hero = image; image = null!; }
                }
                finally { image?.Dispose(); }
            }
            foreach (int k in new[] { shot.HeroFrame, 0, shot.Frames - 1 })
            {
                using Image image = await Capture(k, ShotManifest.SpotWaits);
                if (AssetCatalogue.Digest(image.GetData()) != hashes[k]) throw new ShotSpotException(ShotSpotKind.Stale,
                    $"spot frame {k} differs from its sequence image (stale readback or history-dependent effects)");
            }
            if (hashes[shot.HeroFrame] == hashes[0] || hashes[shot.HeroFrame] == hashes[shot.Frames - 1]) throw new ShotSpotException(ShotSpotKind.Frozen,
                "hero frame matches the first or last frame (frozen readback, or a look in which the turn does not show)");
            success = true;
            return hero!;
        }
        finally
        {
            if (!success) hero?.Dispose();
            host.RemoveChild(viewport); viewport.QueueFree();
        }
    }

    public static async Task<ShotRun> Render(Node host, string inputRoot, string outputRoot, IReadOnlyList<string> stems,
        ShotContext context, ShotTools tools, ShotCapture capture, Action<string> report, bool pushCamera = true)
    {
        if (stems.Count == 0 || stems.Distinct(StringComparer.Ordinal).Count() != stems.Count ||
            stems.Any(s => !Regex.IsMatch(s, @"\A[a-z0-9][a-z0-9-]*\z", RegexOptions.CultureInvariant)))
            throw new FormatException("stems: choose distinct lowercase preset stems");
        string Input(string path) => Path.Combine(inputRoot, path);
        string Digest(string path) => AssetCatalogue.Digest(File.ReadAllBytes(Input(path)));
        var schema = ParameterSchema.Load(Input(ShotManifest.ParametersPath));
        var turn = new TurnData(Input(ShotManifest.TurnPath));
        Preset[] presets = stems.Select(s => Presets.Load(Input(ShotManifest.PresetsFolder + "/" + s + ".json"), schema)).ToArray();
        Shot shot = Shot.Load(Input(ShotManifest.ShotPath));
        ShotState[] track = shot.Track(turn);
        if (!shot.HeroShowsTurn(track)) throw new FormatException("shot: the hero frame must show a turn angle that differs from the first and last frames");
        DateTime created = DateTime.UtcNow;
        string name = shot.Id + "-" + created.ToString("yyyyMMdd'T'HHmmss'Z'", CultureInfo.InvariantCulture) + "-" + Guid.NewGuid().ToString("N")[..8];
        string folder = ShotManifest.RunsFolder + "/" + name;
        string runFolder = OutputPaths.FileAt(outputRoot, ShotManifest.RunsFolder, name);
        if (Directory.Exists(runFolder) || File.Exists(runFolder)) throw new IOException("shot: run folder already exists");
        Directory.CreateDirectory(runFolder);
        var results = new List<ShotPresetResult>();
        for (int i = 0; i < stems.Count; i++)
        {
            string stem = stems[i], file = OutputPaths.FileAt(outputRoot, folder, stem + ".png");
            using Image hero = await RenderPreset(host, inputRoot, schema, presets[i], shot, track, capture, pushCamera);
            if (hero.SavePng(file) != Error.Ok) throw new IOException("shot: hero PNG could not be saved: " + stem);
            results.Add(new ShotPresetResult(stem, Digest(ShotManifest.PresetsFolder + "/" + stem + ".json"), presets[i].Name, AssetCatalogue.Digest(File.ReadAllBytes(file))));
            report($"LOOKLAB_SHOT preset={stem} still={stem}.png spot=identical");
        }
        string manifest = ShotManifest.Build(name, created, context, shot, Digest(ShotManifest.ShotPath), Shot.TrackDigest(track), turn.ModelId,
            Digest(ShotManifest.TurnPath), Digest(ShotManifest.ParametersPath), Digest(ShotManifest.ShaderPath), tools, results);
        File.WriteAllText(OutputPaths.FileAt(outputRoot, folder, "manifest.json"), manifest, new UTF8Encoding(false));
        // Mirror only a complete formal run, so a failed run leaves nothing in the gallery.
        if (context.Source.Clean)
            foreach (string stem in stems)
            {
                string mirror = OutputPaths.FileAt(outputRoot, ShotManifest.GalleryFolder + "/" + name, stem + ".png");
                Directory.CreateDirectory(Path.GetDirectoryName(mirror)!);
                File.Copy(OutputPaths.FileAt(outputRoot, folder, stem + ".png"), mirror, false);
            }
        return new ShotRun(name, folder + "/manifest.json", context.Source.Clean, shot);
    }
}
