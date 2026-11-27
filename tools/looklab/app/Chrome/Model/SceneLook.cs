using System.Collections.Generic;
using System.Linq;

namespace Magic600.Chrome;

public sealed record SceneLookValues(string Theme, string Look, Rgb Background, double Fog, double Glow, double EdgeBrightness, double EdgeWeight)
{
    public static SceneLookValues Resolve(ThemeFamily theme, string look)
    {
        if (!theme.Scenes.TryGetValue(look, out var values)) throw J.Error(look, "unknown scene look");
        double Number(string key) => J.Number(values[key], key);
        var background = new Oklch(Number("bgLightness"), Number("bgTint"), Number("bgHue")).ToSrgb();
        return new SceneLookValues(theme.Id, look, background, Number("fog"), Number("glow"), Number("edgeBrightness"), Number("edgeWeight"));
    }
}

public sealed class StickerPaletteChoice
{
    private const string SettingKey = "look.sticker_palette";
    private readonly IReadOnlyList<StickerPalette> palettes;
    private readonly ChromeSettings settings;

    public StickerPaletteChoice(IReadOnlyList<StickerPalette> palettes, ChromeSettings settings)
    {
        this.palettes = palettes;
        this.settings = settings;
    }

    public StickerPalette? Selected => palettes.FirstOrDefault(p => p.Id == settings.Get(SettingKey) && p.Passes);

    public StickerPalette Current => Selected ?? palettes.First(p => p.Passes);

    public string? Choose(string id)
    {
        StickerPalette? palette = palettes.FirstOrDefault(p => p.Id == id);
        if (palette == null)
            return $"Unknown sticker palette '{id}'. Choose {string.Join(", ", palettes.Where(p => p.Passes).Select(p => p.Id))}.";
        if (!palette.Passes) return $"Sticker palette '{id}' fails the G4 colour-distance checks.";
        settings.Set(SettingKey, id);
        return null;
    }
}

public static class SceneExports
{
    public const string SceneLook = "m6h_set_scene_look";
    public const string StickerColours = "m6h_set_sticker_colours";

    public static string Chosen(string what, string export) =>
        $"{what} sent through {export}";
}
