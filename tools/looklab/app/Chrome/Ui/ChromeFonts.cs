using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The font files of the type roles, loaded from the installed packages (tools/node of a checkout). The design
// data names each face's WOFF2 files by weight; the host names the folders to look in. A face whose files are
// not found falls back to the role's chain of system faces, and the specimen says so.
public static class ChromeFonts
{
    public static IReadOnlyList<string> Roots { get; private set; } = Array.Empty<string>();
    private static readonly Dictionary<string, FontFile?> Files = new(StringComparer.Ordinal);
    private static readonly Dictionary<string, Font?> Faces = new(StringComparer.Ordinal);

    public static void SetRoots(IEnumerable<string> folders)
    {
        string[] roots = folders.Where(Directory.Exists).Select(Path.GetFullPath).Distinct(StringComparer.OrdinalIgnoreCase).ToArray();
        if (roots.SequenceEqual(Roots)) return;
        Roots = roots; Files.Clear(); Faces.Clear();
    }

    public static string? Find(string relative) => Roots.Select(r => Path.Combine(r, relative)).FirstOrDefault(File.Exists);

    private static FontFile? Load(string relative)
    {
        if (Files.TryGetValue(relative, out FontFile? known)) return known;
        FontFile? font = null;
        if (Find(relative) is { } path)
        {
            font = new FontFile();
            if (font.LoadDynamicFont(path) != Error.Ok) font = null;
            else { font.Antialiasing = TextServer.FontAntialiasing.Gray; font.Hinting = TextServer.Hinting.Light; font.SubpixelPositioning = TextServer.SubpixelPositioning.Auto; }
        }
        return Files[relative] = font;
    }

    // The role's face at a weight: the first file is the face, the others are its further Unicode subsets, and
    // the role's system faces come last for anything the files do not cover. Null when the face is not installed.
    public static Font? Face(TypeRole role, bool bold)
    {
        string weight = bold && role.Files.ContainsKey("600") ? "600" : "400", key = role.Id + "/" + weight;
        if (Faces.TryGetValue(key, out Font? known)) return known;
        FontFile? first = Load(role.Files[weight][0]);
        if (first != null)
        {
            var fallbacks = new Godot.Collections.Array<Font>();
            foreach (string file in role.Files[weight].Skip(1)) if (Load(file) is { } subset) fallbacks.Add(subset);
            fallbacks.Add(new SystemFont { FontNames = role.Faces.Skip(1).ToArray(), FontWeight = bold ? 600 : 400, AllowSystemFallback = true });
            first.Fallbacks = fallbacks;
        }
        return Faces[key] = first;
    }

    public static bool Installed(TypeRole role) => Face(role, false) != null;
}
