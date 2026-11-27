using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

internal static class ShellScene
{
    private const string PaletteCommand = "look.sticker_palette.set";
    private const string SceneCommand = "look.scene.set";
    private static readonly ChromeSettings Settings = new();
    private static ChromeFrame host = null!;
    private static StickerPaletteChoice palette = null!;
    private static Func<(ThemeFamily Theme, string Look)> appearance = null!;
    private static Action<string> setLook = null!;
    private static SceneLookValues? sceneLook;
    private static StickerPalette? chosenPalette;
    private static float[]? colourTable;
    private static string? tableDigest;
    private static int? sceneStatus, paletteStatus;
    private static string? sceneNotice, paletteNotice;

    public static void Load(ChromeFrame frame, IReadOnlyList<StickerPalette> palettes,
        Func<(ThemeFamily Theme, string Look)> currentLook, Action<string> changeLook)
    {
        host = frame;
        palette = new StickerPaletteChoice(palettes, Settings);
        chosenPalette = palette.Selected;
        colourTable = null;
        tableDigest = null;
        sceneStatus = paletteStatus = null;
        sceneNotice = paletteNotice = null;
        appearance = currentLook;
        setLook = changeLook;
        Refresh();
    }

    public static bool Handles(string command) => command is PaletteCommand or SceneCommand;

    public static void Execute(ChromeFrame frame, string command, string? argument)
    {
        if (command == PaletteCommand)
        {
            ChoosePalette(frame, argument ?? "");
            return;
        }
        if (argument == null || !ThemeFamily.SceneLooks.Contains(argument))
        {
            frame.Say($"Unknown scene look '{argument}'. Choose {string.Join(", ", ThemeFamily.SceneLooks)}.", true);
            return;
        }
        setLook(argument);
        sceneNotice = "Scene look " + argument;
        frame.Say(sceneNotice + " chosen");
    }

    public static void ChoosePalette(StickerPalette chosen) => ChoosePalette(host, chosen.Id);

    private static void ChoosePalette(ChromeFrame frame, string id)
    {
        if (palette.Choose(id) is { } reason) { frame.Say(reason, true); return; }
        if (chosenPalette?.Id != id)
        {
            chosenPalette = palette.Current;
            colourTable = null;
            tableDigest = null;
            paletteStatus = null;
        }
        paletteNotice = palette.Current.Name + " sticker palette";
        frame.Say(paletteNotice + " chosen");
    }

    public static void Refresh()
    {
        var (theme, look) = appearance();
        var resolved = SceneLookValues.Resolve(theme, look);
        if (sceneLook != resolved)
        {
            sceneLook = resolved;
            sceneStatus = null;
        }
    }

    public static void Apply(HostLibrary library, IntPtr handle)
    {
        if (sceneLook is not { } look) return;
        if (sceneStatus != HostLibrary.Ok)
            sceneStatus = library.SetSceneLook(handle, look);
        if (sceneStatus == HostLibrary.Ok && sceneNotice is { } scene)
        {
            host.Say(SceneExports.Chosen(scene, SceneExports.SceneLook));
            sceneNotice = null;
        }

        if (paletteStatus != HostLibrary.Ok)
        {
            if (chosenPalette is { } choice)
                colourTable ??= StickerColouring.Table(File.ReadAllText(ShellHopf.RingsPath()), choice);
            paletteStatus = library.SetStickerColours(handle, colourTable);
            if (paletteStatus == HostLibrary.Ok && colourTable is { } table)
                tableDigest = Convert.ToHexString(SHA256.HashData(MemoryMarshal.AsBytes(table.AsSpan()))).ToLowerInvariant();
        }
        if (paletteStatus == HostLibrary.Ok && paletteNotice is { } sticker)
        {
            host.Say(SceneExports.Chosen(sticker, SceneExports.StickerColours));
            paletteNotice = null;
        }
    }

    private static void WriteStatus(Utf8JsonWriter json, string export, int? status)
    {
        json.WriteBoolean("sent", status == HostLibrary.Ok);
        json.WriteString("export", export);
        if (status is int code) json.WriteNumber("status", code);
        else json.WriteNull("status");
    }

    public static void Dump(Utf8JsonWriter json)
    {
        if (sceneLook is not { } scene) return;
        json.WriteStartObject("scene_look");
        json.WriteString("theme", scene.Theme);
        json.WriteString("look", scene.Look);
        json.WriteString("background", scene.Background.Hex);
        json.WriteNumber("fog", scene.Fog);
        json.WriteNumber("glow", scene.Glow);
        json.WriteNumber("edgeBrightness", scene.EdgeBrightness);
        json.WriteNumber("edgeWeight", scene.EdgeWeight);
        WriteStatus(json, SceneExports.SceneLook, sceneStatus);
        json.WriteStartArray("drawn");
        json.WriteStringValue("background");
        json.WriteEndArray();
        json.WriteStartArray("held");
        foreach (string field in new[] { "fog", "glow", "edgeBrightness", "edgeWeight" }) json.WriteStringValue(field);
        json.WriteEndArray();
        json.WriteEndObject();

        StickerPalette current = chosenPalette ?? palette.Current;
        json.WriteStartObject("sticker_palette");
        json.WriteString("id", current.Id);
        json.WriteString("name", current.Name);
        json.WriteBoolean("passes", current.Passes);
        json.WriteStartArray("colours");
        foreach (Rgb colour in current.Colours) json.WriteStringValue(colour.Hex);
        json.WriteEndArray();
        WriteStatus(json, SceneExports.StickerColours, paletteStatus);
        json.WriteString("table", chosenPalette?.Id ?? "core default");
        if (tableDigest is { } digest) json.WriteString("table_sha256", digest);
        json.WriteEndObject();
    }
}
