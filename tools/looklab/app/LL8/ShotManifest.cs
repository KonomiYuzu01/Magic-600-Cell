using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;
using LookLab.Core;

namespace LookLab.App.LL8;

internal sealed record ShotSource(string Commit, bool Clean, string Staged);

internal sealed record ShotContext(string Command, ShotSource Source)
{
    public static ShotContext Parse(string? text)
    {
        if (string.IsNullOrWhiteSpace(text)) throw new FormatException("context: LOOKLAB_SHOT_CONTEXT is required");
        try
        {
            using var document = JsonDocument.Parse(text);
            JsonElement root = document.RootElement;
            Keys(root, "command", "source");
            string command = Text(root, "command");
            const string prefix = "python tools/looklab/check.py --shot";
            if (!command.StartsWith(prefix, StringComparison.Ordinal) || (command.Length > prefix.Length && command[prefix.Length] != ' ') ||
                command.Length > 4096 || command.Any(char.IsControl)) throw new FormatException("command: invalid shot command");
            JsonElement source = root.GetProperty("source");
            Keys(source, "commit", "clean", "staged");
            string commit = Text(source, "commit"), staged = Text(source, "staged");
            if (!Regex.IsMatch(commit, @"\A[0-9a-f]{40}\z", RegexOptions.CultureInvariant)) throw new FormatException("commit: expected 40 lowercase hex characters");
            if (!Regex.IsMatch(staged, @"\A[0-9a-f]{64}\z", RegexOptions.CultureInvariant)) throw new FormatException("staged: expected SHA-256");
            JsonElement clean = source.GetProperty("clean");
            if (clean.ValueKind is not (JsonValueKind.True or JsonValueKind.False)) throw new FormatException("clean: expected boolean");
            return new ShotContext(command, new ShotSource(commit, clean.GetBoolean(), staged));
        }
        catch (JsonException ex) { throw new FormatException("context: malformed JSON", ex); }
    }

    private static void Keys(JsonElement value, params string[] keys)
    {
        if (value.ValueKind != JsonValueKind.Object || value.EnumerateObject().Count() != keys.Length ||
            !value.EnumerateObject().Select(p => p.Name).OrderBy(k => k, StringComparer.Ordinal).SequenceEqual(keys.OrderBy(k => k, StringComparer.Ordinal)))
            throw new FormatException("context: unexpected or missing keys");
    }

    private static string Text(JsonElement value, string key)
    {
        JsonElement field = value.GetProperty(key);
        if (field.ValueKind != JsonValueKind.String) throw new FormatException(key + ": expected string");
        return field.GetString()!;
    }
}

internal sealed record ShotTools(string Godot, string Dotnet, string Driver, string Adapter);
internal sealed record ShotPresetResult(string Stem, string Digest, string Name, string StillDigest);

internal static class ShotManifest
{
    public const string Format = "magic600-look-shot-run";
    public const int Version = 1;
    public const string ShotPath = "tools/looklab/data/shots/g2.json";
    public const string TurnPath = "tools/looklab/fixtures/w3-turn.json";
    public const string ParametersPath = "tools/looklab/data/parameters.json";
    public const string ShaderPath = "tools/looklab/app/shaders/cells.gdshader";
    public const string PresetsFolder = "tools/looklab/presets";
    public const string RunsFolder = "work/loop-memory/looklab/shots";
    public const string GalleryFolder = "work/gallery/looklab/shots";
    public const string ClipsNote = "not made: no clip encoder until FFmpeg is approved";
    public const int Waits = 3, SpotWaits = 8;

    public static string Build(string run, DateTime created, ShotContext context, Shot shot, string shotDigest, string trackDigest,
        string modelId, string turnDigest, string parametersDigest, string shaderDigest, ShotTools tools, IReadOnlyList<ShotPresetResult> presets)
    {
        if (created.Kind != DateTimeKind.Utc || !Regex.IsMatch(run, @"\A" + Regex.Escape(shot.Id) + "-" +
            created.ToString("yyyyMMdd'T'HHmmss'Z'", CultureInfo.InvariantCulture) + @"-[0-9a-f]{8}\z", RegexOptions.CultureInvariant))
            throw new FormatException("run: expected shot id, UTC creation time and eight hex characters");
        if (new[] { tools.Godot, tools.Dotnet, tools.Driver, tools.Adapter }.Any(string.IsNullOrWhiteSpace)) throw new FormatException("tools: nonempty versions, driver and adapter required");
        foreach (string digest in new[] { shotDigest, trackDigest, turnDigest, parametersDigest, shaderDigest }.Concat(presets.SelectMany(p => new[] { p.Digest, p.StillDigest })))
            if (!Regex.IsMatch(digest, @"\A[0-9a-f]{64}\z", RegexOptions.CultureInvariant)) throw new FormatException("sha256: expected lowercase SHA-256");
        if (presets.Any(p => !Regex.IsMatch(p.Stem, @"\A[a-z0-9][a-z0-9-]*\z", RegexOptions.CultureInvariant))) throw new FormatException("stem: invalid preset stem");
        return JsonSerializer.Serialize(new
        {
            format = Format, version = Version, run, created = created.ToString("yyyy-MM-dd'T'HH:mm:ss'Z'", CultureInfo.InvariantCulture),
            command = context.Command, source = new { commit = context.Source.Commit, clean = context.Source.Clean, staged = context.Source.Staged }, formal = context.Source.Clean,
            shot = new { id = shot.Id, path = ShotPath, sha256 = shotDigest, frames = shot.Frames, frameRate = Shot.FrameRate,
                heroFrame = shot.HeroFrame, spotFrames = shot.SpotFrames, size = new[] { shot.Width, shot.Height }, track = trackDigest },
            inputs = new { modelId, turn = new { path = TurnPath, sha256 = turnDigest }, parameters = new { path = ParametersPath, sha256 = parametersDigest },
                shader = new { path = ShaderPath, sha256 = shaderDigest } },
            tools = new { godot = tools.Godot, dotnet = tools.Dotnet, driver = tools.Driver, adapter = tools.Adapter },
            capture = new { waits = Waits, spotWaits = SpotWaits },
            presets = presets.Select(p => new { stem = p.Stem, path = PresetsFolder + "/" + p.Stem + ".json", sha256 = p.Digest, name = p.Name,
                framesRendered = shot.Frames, still = new { file = p.Stem + ".png", sha256 = p.StillDigest }, clip = (string?)null, spot = "identical" }),
            clips = ClipsNote
        }, new JsonSerializerOptions { WriteIndented = true }).Replace("\r\n", "\n", StringComparison.Ordinal);
    }
}
