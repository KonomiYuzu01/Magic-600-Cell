using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using Godot;

namespace Magic600.Chrome;

// Icons by meaning (ADR-D01.52): the design data maps each icon the chrome uses to a glyph of the installed
// placeholder icon font and to a short text that stands in when the font is not installed.
public static class ChromeIcons
{
    // The size of the glyph of a drawn empty state; the frame sets it from the token set.
    public static int EmptySize { get; set; } = 48;
    public static IconSet? Set { get; private set; }
    public static bool Installed => font != null;
    private static FontFile? font;
    private static readonly Dictionary<string, int> Codepoints = new(StringComparer.Ordinal);

    public static void Use(IconSet set)
    {
        Set = set; font = null; Codepoints.Clear();
        if (ChromeFonts.Find(set.Font) is not { } fontPath || ChromeFonts.Find(set.Codepoints) is not { } table) return;
        try
        {
            using JsonDocument d = JsonDocument.Parse(File.ReadAllText(table));
            foreach (JsonProperty p in d.RootElement.EnumerateObject()) if (p.Value.TryGetInt32(out int code)) Codepoints[p.Name] = code;
        }
        catch (Exception ex) when (ex is JsonException or IOException or InvalidOperationException) { Codepoints.Clear(); return; }
        var loaded = new FontFile();
        if (loaded.LoadDynamicFont(fontPath) == Error.Ok) { loaded.Antialiasing = TextServer.FontAntialiasing.Gray; font = loaded; }
    }

    // The glyph of an icon, or null when the icon, its glyph or the font is missing.
    public static string? Glyph(string icon) =>
        font != null && Set != null && Set.Icons.TryGetValue(icon, out IconEntry? entry) && Codepoints.TryGetValue(entry.Glyph, out int code) ? char.ConvertFromUtf32(code) : null;

    // Text that is not an icon id is shown as it is.
    public static string Fallback(string icon) => Set != null && Set.Icons.TryGetValue(icon, out IconEntry? entry) ? entry.Fallback : icon;

    private static string Dress(Control control, string icon, int size)
    {
        if (Glyph(icon) is not { } glyph) { control.RemoveThemeFontOverride("font"); control.RemoveThemeFontSizeOverride("font_size"); return Fallback(icon); }
        control.AddThemeFontOverride("font", font!); control.AddThemeFontSizeOverride("font_size", size);
        return glyph;
    }

    public static void Apply(Button button, string icon, int size) { button.Text = Dress(button, icon, size); button.SetMeta("icon", icon); }
    public static void Apply(Label label, string icon, int size) { label.Text = Dress(label, icon, size); label.SetMeta("icon", icon); }
}
