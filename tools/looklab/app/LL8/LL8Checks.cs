using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using Godot;
using LookLab.Core;
using File = System.IO.File;

namespace LookLab.App.LL8;

internal static class LL8Checks
{
    public static bool Handles(string mode) => mode == "ll8-shot";
    private static void Require(bool condition, string message) { if (!condition) throw new Exception("ll8-shot: " + message); }

    private static byte[] Pixels(double[] q, double[] spin, double theta, long revision, Shot shot)
    {
        var state = new byte[272]; int offset = 0;
        foreach (double value in q.Concat(spin).Append(theta))
        {
            BinaryPrimitives.WriteDoubleLittleEndian(state.AsSpan(offset, 8), value); offset += 8;
        }
        BinaryPrimitives.WriteInt64LittleEndian(state.AsSpan(offset, 8), revision);
        byte[] h = System.Security.Cryptography.SHA256.HashData(state), pixels = new byte[shot.Width * shot.Height * 4];
        for (int i = 0; i < pixels.Length; i += 4) { pixels[i] = h[0]; pixels[i + 1] = h[1]; pixels[i + 2] = h[2]; pixels[i + 3] = 255; }
        return pixels;
    }

    public static async Task Run(Node host, string root, string scratch, string mode, string fault)
    {
        Shot shot = Shot.Load(Path.Combine(root, ShotManifest.ShotPath));
        var turn = new TurnData(Path.Combine(root, ShotManifest.TurnPath));
        ShotState[] track = shot.Track(turn);
        var schema = ParameterSchema.Load(Path.Combine(root, ShotManifest.ParametersPath));
        string[] stems = { "default", "g1-hopf-rings" };
        Preset[] presets = stems.Select(s => Presets.Load(Path.Combine(root, ShotManifest.PresetsFolder, s + ".json"), schema)).ToArray();
        var context = ShotContext.Parse(JsonSerializer.Serialize(new { command = "python tools/looklab/check.py --shot default g1-hopf-rings",
            source = new { commit = new string('a', 40), clean = true, staged = new string('b', 64) } }));
        var tools = new ShotTools("synthetic Godot", "synthetic .NET", "synthetic driver", "synthetic adapter");
        var requests = new List<(int Frame, int Waits)>();
        byte[]? previous = null, first = null;
        Task<Image> Capture(LookViewport view, SubViewport viewport, int frame, int waits)
        {
            ShotState state = track[frame];
            string check = fault == "ll8-shot-stale-camera" ? fault : "ll8-shot";
            void Bound(bool condition, string message) { if (!condition) throw new Exception(check + ": frame " + frame + " " + message); }
            Bound(view.Material.GetShaderParameter("q").AsProjection() == LookViewport.Matrix(state.Q) &&
                view.Material.GetShaderParameter("spin").AsProjection() == LookViewport.Matrix(state.Spin), "camera matrices were not bound");
            Bound(view.Material.GetShaderParameter("aspect").AsSingle() == (float)(shot.Width / (double)shot.Height), "aspect was not bound");
            Bound(view.Material.GetShaderParameter("theta").AsSingle() == (float)state.Turn.Theta, "turn angle was not bound");
            Bound(view.Material.GetShaderParameter("projection_kind").AsInt32() == (int)presets[requests.Count / (shot.Frames + 3)].Params.Structure(1).Projection,
                "preset projection was not bound");
            Bound(view.BoundFrame.BoundRevision == state.Turn.BoundRevision, "turn revision was not bound");
            requests.Add((frame, waits));
            byte[] current = Pixels(view.Q, view.Spin, view.BoundFrame.Theta, view.BoundFrame.BoundRevision, shot);
            first ??= current;
            byte[] data = fault == "ll8-shot-stale-capture" ? previous ?? current : fault == "ll8-shot-frozen-capture" ? first : current;
            previous = current;
            return Task.FromResult(Image.CreateFromData(shot.Width, shot.Height, false, Image.Format.Rgba8, data));
        }
        string output = Path.Combine(scratch, "ll8-shot");
        ShotRun run;
        try { run = await ShotPipeline.Render(host, root, output, stems, context, tools, Capture, line => GD.Print(line), fault != "ll8-shot-stale-camera"); }
        catch (ShotSpotException ex)
        {
            bool targeted = (fault == "ll8-shot-stale-capture" && ex.Kind == ShotSpotKind.Stale) ||
                (fault == "ll8-shot-frozen-capture" && ex.Kind == ShotSpotKind.Frozen);
            throw new Exception((targeted ? fault : "ll8-shot") + ": " + ex.Message, ex);
        }
        var expectedRequests = Enumerable.Range(0, shot.Frames).Select(k => (k, ShotManifest.Waits))
            .Concat(new[] { (shot.HeroFrame, ShotManifest.SpotWaits), (0, ShotManifest.SpotWaits), (shot.Frames - 1, ShotManifest.SpotWaits) }).ToArray();
        Require(requests.SequenceEqual(expectedRequests.Concat(expectedRequests)), "capture order or waits differ");
        byte[] expected = Pixels(track[shot.HeroFrame].Q, track[shot.HeroFrame].Spin, track[shot.HeroFrame].Turn.Theta, track[shot.HeroFrame].Turn.BoundRevision, shot);
        foreach (string stem in stems)
        {
            string path = Path.Combine(output, ShotManifest.RunsFolder, run.Name, stem + ".png");
            using Image image = Image.LoadFromFile(path);
            image.Convert(Image.Format.Rgba8);
            Require(image.GetWidth() == shot.Width && image.GetHeight() == shot.Height && image.GetData().SequenceEqual(expected), "hero pixels differ: " + stem);
            Require(File.ReadAllBytes(path).SequenceEqual(File.ReadAllBytes(Path.Combine(output, ShotManifest.GalleryFolder, run.Name, stem + ".png"))), "gallery differs: " + stem);
        }
        CheckManifest(root, output, run, context, tools, shot, track, turn, stems, presets);
        GD.Print("LOOKLAB_LL8_SHOT_PASS presets=2 frames=240 camera=bound turn=bound spot=identical stills=2 manifest=exact checks=1");
    }

    private static void CheckManifest(string root, string output, ShotRun run, ShotContext context, ShotTools tools, Shot shot,
        ShotState[] track, TurnData turn, string[] stems, Preset[] presets)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(Path.Combine(output, run.ManifestPath)));
        JsonElement m = document.RootElement;
        void Keys(JsonElement e, params string[] keys) => Require(e.ValueKind == JsonValueKind.Object && e.EnumerateObject().Select(p => p.Name).SequenceEqual(keys), "manifest keys differ");
        void Text(JsonElement e, string key, string value) => Require(e.GetProperty(key).GetString() == value, "manifest " + key + " differs");
        void Number(JsonElement e, string key, int value) => Require(e.GetProperty(key).GetInt32() == value, "manifest " + key + " differs");
        void Input(JsonElement e, string path)
        {
            Keys(e, "path", "sha256"); Text(e, "path", path);
            Text(e, "sha256", AssetCatalogue.Digest(File.ReadAllBytes(Path.Combine(root, path))));
        }
        Keys(m, "format", "version", "run", "created", "command", "source", "formal", "shot", "inputs", "tools", "capture", "presets", "clips");
        Text(m, "format", ShotManifest.Format); Number(m, "version", 1); Text(m, "run", run.Name); Text(m, "command", context.Command);
        DateTime created = DateTime.ParseExact(m.GetProperty("created").GetString()!, "yyyy-MM-dd'T'HH:mm:ss'Z'", CultureInfo.InvariantCulture, DateTimeStyles.AssumeUniversal | DateTimeStyles.AdjustToUniversal);
        Require(Regex.IsMatch(run.Name, @"\A" + shot.Id + "-" + created.ToString("yyyyMMdd'T'HHmmss'Z'", CultureInfo.InvariantCulture) + @"-[0-9a-f]{8}\z"), "run instant differs");
        JsonElement source = m.GetProperty("source"); Keys(source, "commit", "clean", "staged");
        Text(source, "commit", context.Source.Commit); Text(source, "staged", context.Source.Staged);
        Require(source.GetProperty("clean").GetBoolean() == context.Source.Clean && m.GetProperty("formal").GetBoolean() == context.Source.Clean && run.Formal, "formal differs from clean");
        JsonElement s = m.GetProperty("shot"); Keys(s, "id", "path", "sha256", "frames", "frameRate", "heroFrame", "spotFrames", "size", "track");
        Text(s, "id", shot.Id); Text(s, "path", ShotManifest.ShotPath); Text(s, "sha256", AssetCatalogue.Digest(File.ReadAllBytes(Path.Combine(root, ShotManifest.ShotPath))));
        Number(s, "frames", shot.Frames); Number(s, "frameRate", 24); Number(s, "heroFrame", shot.HeroFrame); Text(s, "track", Shot.TrackDigest(track));
        Require(s.GetProperty("spotFrames").EnumerateArray().Select(x => x.GetInt32()).SequenceEqual(shot.SpotFrames) &&
            s.GetProperty("size").EnumerateArray().Select(x => x.GetInt32()).SequenceEqual(new[] { shot.Width, shot.Height }), "shot dimensions or spots differ");
        JsonElement inputs = m.GetProperty("inputs"); Keys(inputs, "modelId", "turn", "parameters", "shader"); Text(inputs, "modelId", turn.ModelId);
        Input(inputs.GetProperty("turn"), ShotManifest.TurnPath); Input(inputs.GetProperty("parameters"), ShotManifest.ParametersPath); Input(inputs.GetProperty("shader"), ShotManifest.ShaderPath);
        JsonElement t = m.GetProperty("tools"); Keys(t, "godot", "dotnet", "driver", "adapter");
        Text(t, "godot", tools.Godot); Text(t, "dotnet", tools.Dotnet); Text(t, "driver", tools.Driver); Text(t, "adapter", tools.Adapter);
        JsonElement c = m.GetProperty("capture"); Keys(c, "waits", "spotWaits"); Number(c, "waits", 3); Number(c, "spotWaits", 8);
        JsonElement entries = m.GetProperty("presets"); Require(entries.GetArrayLength() == stems.Length, "preset count differs");
        for (int i = 0; i < stems.Length; i++)
        {
            JsonElement p = entries[i]; Keys(p, "stem", "path", "sha256", "name", "framesRendered", "still", "clip", "spot");
            string path = ShotManifest.PresetsFolder + "/" + stems[i] + ".json";
            Text(p, "stem", stems[i]); Text(p, "path", path); Text(p, "sha256", AssetCatalogue.Digest(File.ReadAllBytes(Path.Combine(root, path))));
            Text(p, "name", presets[i].Name); Number(p, "framesRendered", shot.Frames); Text(p, "spot", "identical");
            Require(p.GetProperty("clip").ValueKind == JsonValueKind.Null, "clip must be null");
            JsonElement still = p.GetProperty("still"); Keys(still, "file", "sha256"); Text(still, "file", stems[i] + ".png");
            Text(still, "sha256", AssetCatalogue.Digest(File.ReadAllBytes(Path.Combine(output, ShotManifest.RunsFolder, run.Name, stems[i] + ".png"))));
        }
        Text(m, "clips", ShotManifest.ClipsNote);
    }
}
