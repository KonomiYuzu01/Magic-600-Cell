using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using LookLab.Core;
using LookLab.App.LL8;

internal static partial class Program
{
    private static Shot G2 => Shot.Load(FileAt("data/shots/g2.json"));
    private static string LL8Edit(Action<JsonObject> edit)
    {
        JsonObject root = JsonNode.Parse(Read("data/shots/g2.json"))!.AsObject();
        edit(root); return root.ToJsonString();
    }

    private static void LL8Committed()
    {
        Shot shot = G2;
        Equal(shot.Id, "g2"); Equal(shot.Width, 1600); Equal(shot.Height, 1000); Equal(shot.Frames, 240); Equal(shot.HeroFrame, 60);
        Equal(Shot.FrameRate, 24); Equal(Shot.MaxSeconds, 10.0); Equal(shot.Seconds, 10.0);
        Sequence(shot.SpotFrames, new[] { 0, 60, 239 }); Sequence(shot.Camera, new[] { new ShotSegment(240, .5, .15) });
        Sequence(shot.Turns, new[] { new ShotTurn(48, 24), new ShotTurn(144, 24) });
    }

    private static void LL8Track()
    {
        Shot shot = G2; ShotState[] track = shot.Track(Turn);
        Equal(track.Length, 240);
        double[] q = Geometry.Identity(), spin = Geometry.Identity();
        var arrays = new HashSet<double[]>(ReferenceEqualityComparer.Instance);
        for (int k = 0; k < track.Length; k++)
        {
            if (k > 0) GlobalLayout.Drag(q, spin, .5 / 240, .15 / 240);
            Equal(track[k].Frame, k); Sequence(track[k].Q, q); Sequence(track[k].Spin, spin);
            Check(arrays.Add(track[k].Q) && arrays.Add(track[k].Spin), "track aliases a camera array");
            Turn.ValidateBinding(track[k].Turn.BoundRevision, track[k].Turn.Labels.Span);
            Equal(track[k].Turn.TurnIndex, track[k].Turn.BoundRevision);
        }
        foreach (var (k, revision) in new[] { (0, 0L), (47, 0L), (72, 1L), (168, 2L), (239, 2L) })
        {
            Equal(track[k].Turn.BoundRevision, revision); Equal(track[k].Turn.Theta, 0.0); Equal(track[k].Turn.Phase, 0.0);
        }
        Equal(track[60].Turn.BoundRevision, 0L); Equal(track[60].Turn.Phase, .5);
        Equal(track[60].Turn.Theta, Turn.Angle * Easing.Ease(.5, 1.0 / 3, 1.0 / 3));
        Check(track[71].Turn.Theta > 0, "turn ends early");
        Equal(track[150].Turn.BoundRevision, 1L); Check(track[150].Turn.Theta < 0, "inverse must turn backwards");
        Shot split = Shot.Parse(LL8Edit(r => r["camera"] = JsonNode.Parse("[{\"seconds\":1,\"dragX\":0.2,\"dragY\":-0.1},{\"seconds\":9,\"dragX\":-0.3,\"dragY\":0.4}]")));
        ShotState[] splitTrack = split.Track(Turn); q = Geometry.Identity(); spin = Geometry.Identity();
        for (int k = 1; k < split.Frames; k++)
        {
            GlobalLayout.Drag(q, spin, k <= 24 ? .2 / 24 : -.3 / 216, k <= 24 ? -.1 / 24 : .4 / 216);
            Sequence(splitTrack[k].Q, q); Sequence(splitTrack[k].Spin, spin);
        }
    }

    private static void LL8Digest()
    {
        ShotState[] track = G2.Track(Turn);
        string digest = Shot.TrackDigest(track);
        Equal(Shot.TrackDigest(G2.Track(Turn)), digest);
        Check(Regex.IsMatch(digest, @"\A[0-9a-f]{64}\z"), "digest must be lowercase SHA-256");
        Check(Shot.TrackDigest(Shot.Parse(LL8Edit(r => r["camera"]![0]!["dragX"] = .5000001)).Track(Turn)) != digest, "drag missing from digest");
        Check(Shot.TrackDigest(Shot.Parse(LL8Edit(r => r["turns"]![1]!["start"] = 6.5)).Track(Turn)) != digest, "turn schedule missing from digest");
        using var bytes = new MemoryStream();
        var number = new byte[8];
        foreach (ShotState s in track)
        {
            foreach (double value in s.Q.Concat(s.Spin)) { BinaryPrimitives.WriteDoubleLittleEndian(number, value); bytes.Write(number); }
            BinaryPrimitives.WriteInt64LittleEndian(number, s.Turn.BoundRevision); bytes.Write(number);
            BinaryPrimitives.WriteDoubleLittleEndian(number, s.Turn.Theta); bytes.Write(number);
        }
        Equal(digest, AssetCatalogue.Digest(bytes.ToArray()));
        Refuse(() => Shot.TrackDigest(Array.Empty<ShotState>()), "track");
    }

    private static void LL8ParseRules()
    {
        void Bad(Action<JsonObject> edit, string id) => Refuse(() => Shot.Parse(LL8Edit(edit)), id);
        Bad(r => r["format"] = "other", "format"); Bad(r => r["version"] = 2, "version");
        Bad(r => r["extra"] = true, "extra");
        foreach (string key in new[] { "format", "version", "id", "name", "size", "seconds", "heroFrame", "ease", "camera", "turns" }) Bad(r => r.Remove(key), key);
        foreach (string id in new[] { "G2", "-g2", "g2\n" }) Bad(r => r["id"] = id, "id");
        Bad(r => r["name"] = " \t ", "name");
        foreach (double seconds in new[] { 0, -1, 10.5, 9.99 }) Bad(r => r["seconds"] = seconds, "seconds");
        foreach (string size in new[] { "[1601,1000]", "[1600,62]", "[3842,1000]", "[1600]" }) Bad(r => r["size"] = JsonNode.Parse(size), "size");
        foreach (double hero in new[] { -1, 0, 239, 240, 1.5 }) Bad(r => r["heroFrame"] = hero, "heroFrame");
        Bad(r => r["ease"] = JsonNode.Parse("[1.2,0.3]"), "ease");
        Bad(r => r["camera"] = new JsonArray(), "camera");
        Bad(r => r["camera"]![0]!["seconds"] = .01, "seconds");
        Bad(r => r["camera"]![0]!["extra"] = true, "extra");
        Bad(r => r["camera"]![0]!["seconds"] = 9, "camera");
        Bad(r => r["camera"]![0]!["dragX"] = 7, "dragX");
        Bad(r => r["turns"]![0]!["start"] = 1.99, "start");
        Bad(r => r["turns"]![0]!["seconds"] = 0, "seconds");
        Bad(r => r["turns"]![1]!["start"] = 2.5, "turns");
        Bad(r => r["turns"]![1]!["start"] = 1, "turns");
        Bad(r => r["turns"]![1]!["start"] = 9.5, "turns");
        ShotState[] noTurns = Shot.Parse(LL8Edit(r => r["turns"] = new JsonArray())).Track(Turn);
        Check(noTurns.All(s => s.Turn.BoundRevision == 0 && s.Turn.Theta == 0), "empty turns must stay at rest");
    }

    private static void LL8IntegerFrames()
    {
        Shot shot = Shot.Parse(LL8Edit(r => { r["seconds"] = 9.999999999999998; r["camera"]![0]!["seconds"] = 9.999999999999998; }));
        Equal(shot.Frames, 240); Equal(shot.Camera[0].Frames, 240); Equal(shot.Track(Turn).Length, 240);
    }

    private static void LL8Spots()
    {
        Shot shot = G2; ShotState[] track = shot.Track(Turn);
        Check(shot.HeroShowsTurn(track), "the committed hero must show the turn");
        Check(track[0].Turn.Theta == 0 && track[239].Turn.Theta == 0 && track[0].Turn.BoundRevision % 2 == track[239].Turn.BoundRevision % 2,
            "the committed shot starts and ends at rest with the same labels, which the camera-free ring sheet renders alike");
        Shot rest = Shot.Parse(LL8Edit(r => r["heroFrame"] = 100));
        Check(!rest.HeroShowsTurn(rest.Track(Turn)), "a hero at rest must be refused although the camera moved");
        Shot still = Shot.Parse(LL8Edit(r => { r["camera"]![0]!["dragX"] = 0; r["camera"]![0]!["dragY"] = 0; r["turns"] = new JsonArray(); }));
        Check(!still.HeroShowsTurn(still.Track(Turn)), "a shot without turns has no hero turn");
    }

    private static void LL8ManifestContext()
    {
        void Keys(JsonElement e, params string[] keys) => Sequence(e.EnumerateObject().Select(p => p.Name), keys);
        string contextText = JsonSerializer.Serialize(new { command = "python tools/looklab/check.py --shot default", source = new { commit = new string('a', 40), clean = true, staged = new string('b', 64) } });
        ShotContext context = ShotContext.Parse(contextText); Equal(context.Command, "python tools/looklab/check.py --shot default");
        Equal(context.Source.Commit, new string('a', 40)); Equal(context.Source.Staged, new string('b', 64)); Check(context.Source.Clean, "clean context");
        foreach (string? text in new string?[] { null, "", " ", "{broken" }) Refuse(() => ShotContext.Parse(text), "context");
        void BadContext(Action<JsonObject> edit)
        {
            JsonObject c = JsonNode.Parse(contextText)!.AsObject(); edit(c); Refuse(() => ShotContext.Parse(c.ToJsonString()), "");
        }
        BadContext(c => c["extra"] = 1); BadContext(c => c.Remove("command")); BadContext(c => c.Remove("source"));
        BadContext(c => c["source"]!["extra"] = 1);
        foreach (string key in new[] { "commit", "clean", "staged" }) BadContext(c => c["source"]!.AsObject().Remove(key));
        foreach (string commit in new[] { "short", new string('A', 40), new string('g', 40), new string('a', 40) + "\n" }) BadContext(c => c["source"]!["commit"] = commit);
        foreach (string staged in new[] { "short", new string('B', 64), new string('g', 64) }) BadContext(c => c["source"]!["staged"] = staged);
        BadContext(c => c["source"]!["clean"] = "true"); BadContext(c => c["source"]!["clean"] = 1);
        foreach (string command in new[] { "python other.py", "python tools/looklab/check.py --shots", context.Command + "\n", context.Command + "\0", context.Command + new string(' ', 4096) })
            BadContext(c => c["command"] = command);
        Refuse(() => ShotContext.Parse(contextText.Replace("\"clean\":true", "\"clean\":true,\"clean\":true", StringComparison.Ordinal)), "context");
        Shot shot = G2;
        foreach (bool clean in new[] { false, true })
        {
            using var document = JsonDocument.Parse(ShotManifest.Build("g2-20261022T120000Z-0123abcd", new DateTime(2026, 10, 22, 12, 0, 0, DateTimeKind.Utc),
                context with { Source = context.Source with { Clean = clean } }, shot, new string('1', 64), new string('2', 64), "model", new string('3', 64),
                new string('4', 64), new string('5', 64), new ShotTools("godot", "dotnet", "driver", "adapter"),
                new[] { new ShotPresetResult("default", new string('6', 64), "Default", new string('7', 64)) }));
            JsonElement m = document.RootElement;
            Keys(m, "format", "version", "run", "created", "command", "source", "formal", "shot", "inputs", "tools", "capture", "presets", "clips");
            Equal(m.GetProperty("format").GetString(), "magic600-look-shot-run"); Equal(m.GetProperty("version").GetInt32(), 1);
            Equal(m.GetProperty("run").GetString(), "g2-20261022T120000Z-0123abcd"); Equal(m.GetProperty("created").GetString(), "2026-10-22T12:00:00Z");
            Equal(m.GetProperty("command").GetString(), context.Command);
            JsonElement source = m.GetProperty("source"); Keys(source, "commit", "clean", "staged");
            Equal(source.GetProperty("commit").GetString(), context.Source.Commit); Equal(source.GetProperty("staged").GetString(), context.Source.Staged);
            Equal(source.GetProperty("clean").GetBoolean(), clean); Equal(m.GetProperty("formal").GetBoolean(), clean);
            JsonElement s = m.GetProperty("shot"); Keys(s, "id", "path", "sha256", "frames", "frameRate", "heroFrame", "spotFrames", "size", "track");
            Equal(s.GetProperty("id").GetString(), shot.Id); Equal(s.GetProperty("path").GetString(), ShotManifest.ShotPath);
            Equal(s.GetProperty("sha256").GetString(), new string('1', 64)); Equal(s.GetProperty("track").GetString(), new string('2', 64));
            Equal(s.GetProperty("frames").GetInt32(), shot.Frames); Equal(s.GetProperty("frameRate").GetInt32(), 24); Equal(s.GetProperty("heroFrame").GetInt32(), shot.HeroFrame);
            Sequence(s.GetProperty("spotFrames").EnumerateArray().Select(x => x.GetInt32()), shot.SpotFrames);
            Sequence(s.GetProperty("size").EnumerateArray().Select(x => x.GetInt32()), new[] { shot.Width, shot.Height });
            JsonElement inputs = m.GetProperty("inputs"); Keys(inputs, "modelId", "turn", "parameters", "shader"); Equal(inputs.GetProperty("modelId").GetString(), "model");
            foreach (var (key, path, digest) in new[] { ("turn", ShotManifest.TurnPath, '3'), ("parameters", ShotManifest.ParametersPath, '4'), ("shader", ShotManifest.ShaderPath, '5') })
            {
                JsonElement input = inputs.GetProperty(key); Keys(input, "path", "sha256"); Equal(input.GetProperty("path").GetString(), path); Equal(input.GetProperty("sha256").GetString(), new string(digest, 64));
            }
            JsonElement tools = m.GetProperty("tools"); Keys(tools, "godot", "dotnet", "driver", "adapter");
            foreach (string key in new[] { "godot", "dotnet", "driver", "adapter" }) Equal(tools.GetProperty(key).GetString(), key);
            JsonElement capture = m.GetProperty("capture"); Keys(capture, "waits", "spotWaits"); Equal(capture.GetProperty("waits").GetInt32(), 3); Equal(capture.GetProperty("spotWaits").GetInt32(), 8);
            Equal(m.GetProperty("presets").GetArrayLength(), 1);
            JsonElement p = m.GetProperty("presets")[0]; Keys(p, "stem", "path", "sha256", "name", "framesRendered", "still", "clip", "spot");
            Equal(p.GetProperty("stem").GetString(), "default"); Equal(p.GetProperty("path").GetString(), "tools/looklab/presets/default.json");
            Equal(p.GetProperty("sha256").GetString(), new string('6', 64)); Equal(p.GetProperty("name").GetString(), "Default"); Equal(p.GetProperty("framesRendered").GetInt32(), shot.Frames);
            JsonElement still = p.GetProperty("still"); Keys(still, "file", "sha256"); Equal(still.GetProperty("file").GetString(), "default.png"); Equal(still.GetProperty("sha256").GetString(), new string('7', 64));
            Equal(p.GetProperty("clip").ValueKind, JsonValueKind.Null); Equal(p.GetProperty("spot").GetString(), "identical"); Equal(m.GetProperty("clips").GetString(), ShotManifest.ClipsNote);
        }
    }

    private static void LL8Readme()
    {
        string readme = Read("README.md");
        foreach (string text in new[] { "magic600-look-shot", "magic600-look-shot-run", "check.py --shot" }) Check(readme.Contains(text, StringComparison.Ordinal), "README missing " + text);
    }
}
