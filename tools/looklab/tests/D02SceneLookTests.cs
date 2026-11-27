using System;
using System.Linq;
using System.Text.Json;
using LookLab.Core;
using Magic600.Chrome;

internal static partial class Program
{
    private static void D02SceneLookBackgrounds()
    {
        foreach (ThemeFamily theme in Themes())
        {
            var looks = ThemeFamily.SceneLooks.Select(look => SceneLookValues.Resolve(theme, look)).ToArray();
            Equal(looks.Select(look => look.Background.Hex).Distinct().Count(), 3, theme.Id + " scene backgrounds");
            foreach (SceneLookValues look in looks)
            {
                var values = theme.Scenes[look.Look];
                double Number(string key) => values[key].GetDouble();
                Equal(look.Theme, theme.Id);
                Equal(look.Background, new Oklch(Number("bgLightness"), Number("bgTint"), Number("bgHue")).ToSrgb());
                Equal(look.Fog, Number("fog"));
                Equal(look.Glow, Number("glow"));
                Equal(look.EdgeBrightness, Number("edgeBrightness"));
                Equal(look.EdgeWeight, Number("edgeWeight"));
            }
            Refuse(() => SceneLookValues.Resolve(theme, "nosuch"), "nosuch");
        }
    }

    private static void D02StickerPaletteChoice()
    {
        var palettes = StickerPalette.Parse(Read("data/design/palettes.json"));
        var settings = new ChromeSettings();
        var choice = new StickerPaletteChoice(palettes, settings);
        var first = palettes.First(p => p.Passes);
        Equal(choice.Selected, null);
        Equal(choice.Current, first);

        Equal(choice.Choose("graphite.b"), null);
        Equal(choice.Current.Id, "graphite.b");
        Equal(choice.Selected, choice.Current);
        Equal(settings.Get("look.sticker_palette"), "graphite.b");
        Check(choice.Choose("graphite.narrow")?.Contains("G4", StringComparison.Ordinal) == true, "a failing palette was not refused");
        Equal(choice.Current.Id, "graphite.b");
        Check(choice.Choose("nosuch")?.Contains("nosuch", StringComparison.Ordinal) == true, "an unknown palette was not refused");
        Equal(choice.Current.Id, "graphite.b");
        Equal(new StickerPaletteChoice(palettes, settings).Current, choice.Current);

        foreach (string id in new[] { "graphite.narrow", "nosuch" })
        {
            settings.Set("look.sticker_palette", id);
            Equal(choice.Selected, null);
            Equal(choice.Current, first);
        }
    }

    private static void D02StickerColouring()
    {
        string rings = Read("fixtures/hopf-rings.json");
        using JsonDocument document = JsonDocument.Parse(rings);
        var root = document.RootElement;
        int[] ringOf = root.GetProperty("ringOf").EnumerateArray().Select(r => r.GetInt32()).ToArray();
        var graph = root.GetProperty("graph").EnumerateArray().Select(p => (A: p[0].GetInt32(), B: p[1].GetInt32())).ToArray();
        foreach (var palette in StickerPalette.Parse(Read("data/design/palettes.json")).Where(p => p.Passes))
        {
            float[] table = StickerColouring.Table(rings, palette);
            Equal(table.Length, 600 * 3);
            var colours = palette.Colours.Select(c => ((float)c.R, (float)c.G, (float)c.B)).ToArray();
            var actual = Enumerable.Range(0, 600).Select(i => Array.IndexOf(colours, (table[3 * i], table[3 * i + 1], table[3 * i + 2]))).ToArray();
            var reference = new CellColouring(ringOf, GlobalLayout.EquitableColouring(20, graph, colours.Length)!);
            Check(actual.SequenceEqual(Enumerable.Range(0, 600).Select(reference.ClassOf)), palette.Id + " differs from Look Lab");
            int[] perRing = Enumerable.Range(0, 20).Select(r => actual.Where((_, i) => ringOf[i] == r).Distinct().Single()).ToArray();
            Check(graph.All(edge => perRing[edge.A] != perRing[edge.B]), palette.Id + " gives adjacent rings the same class");
            var counts = Enumerable.Range(0, colours.Length).Select(c => perRing.Count(r => r == c)).ToArray();
            Check(counts.Max() - counts.Min() <= 1, palette.Id + " has inequitable class counts");
        }
    }

    private static void D02SceneLookDraftRequests()
    {
        using JsonDocument document = JsonDocument.Parse(Read("data/design/command-rows-d02.json"));
        var rows = document.RootElement.GetProperty("commands").EnumerateArray().ToDictionary(row => row.GetProperty("id").GetString()!);
        Equal(rows["look.sticker_palette.set"].GetProperty("request").GetString(), "ADR-D01.52");
        Equal(rows["look.scene.set"].GetProperty("request").GetString(), "ADR-D01.49");
    }
}
