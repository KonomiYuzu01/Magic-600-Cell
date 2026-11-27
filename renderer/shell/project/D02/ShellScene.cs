using System;
using System.Collections.Generic;
using System.Linq;
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

    public static void Load(ChromeFrame frame, IReadOnlyList<StickerPalette> palettes,
        Func<(ThemeFamily Theme, string Look)> currentLook, Action<string> changeLook)
    {
        host = frame;
        palette = new StickerPaletteChoice(palettes, Settings);
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
        frame.Say(SceneExports.Chosen("Scene look " + argument, SceneExports.SceneLook));
    }

    public static void ChoosePalette(StickerPalette chosen) => ChoosePalette(host, chosen.Id);

    private static void ChoosePalette(ChromeFrame frame, string id)
    {
        if (palette.Choose(id) is { } reason) { frame.Say(reason, true); return; }
        frame.Say(SceneExports.Chosen(palette.Current.Name + " sticker palette", SceneExports.StickerColours));
    }

    public static void Refresh()
    {
        var (theme, look) = appearance();
        sceneLook = SceneLookValues.Resolve(theme, look);
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
        json.WriteBoolean("sent", false);
        json.WriteString("waits_for", SceneExports.SceneLook);
        json.WriteEndObject();

        StickerPalette current = palette.Current;
        json.WriteStartObject("sticker_palette");
        json.WriteString("id", current.Id);
        json.WriteString("name", current.Name);
        json.WriteBoolean("passes", current.Passes);
        json.WriteStartArray("colours");
        foreach (Rgb colour in current.Colours) json.WriteStringValue(colour.Hex);
        json.WriteEndArray();
        json.WriteBoolean("sent", false);
        json.WriteString("waits_for", SceneExports.StickerColours);
        json.WriteEndObject();
    }
}
