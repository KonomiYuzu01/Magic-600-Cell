using System;
using System.IO;
using LookLab.Core;

namespace LookLab.App.Platform;

// The dialog restriction is also enforced at the IO boundary. Native dialogs
// are deliberately disabled because they ignore Godot's RootSubfolder.
internal sealed class PresetFiles
{
    public string Folder { get; }
    private readonly string root;
    private readonly ParameterSchema schema;

    public PresetFiles(string checkoutRoot, ParameterSchema schema)
    {
        root = Path.GetFullPath(checkoutRoot);
        Folder = Path.GetFullPath(Path.Combine(checkoutRoot, "work/loop-memory/looklab/presets"));
        this.schema = schema;
    }

    public bool Exists => Directory.Exists(Folder);

    public void PrepareSave()
    {
        NoLinks(Folder);
        Directory.CreateDirectory(Folder);
    }

    private string Checked(string path)
    {
        string absolute = Path.GetFullPath(path);
        string relative = Path.GetRelativePath(Folder, absolute);
        if (relative == ".." || relative.StartsWith(".." + Path.DirectorySeparatorChar, StringComparison.Ordinal) ||
            Path.IsPathRooted(relative) || Path.GetExtension(absolute).ToLowerInvariant() != ".json")
            throw new IOException("Choose a JSON file inside the Look Lab presets folder.");
        NoLinks(absolute);
        return absolute;
    }

    // Only the part below the checkout root is checked: a link above it cannot
    // lead a write out of the checkout.
    private void NoLinks(string path)
    {
        for (string? current = path; current != null && Below(current); current = Path.GetDirectoryName(current))
            if ((File.Exists(current) || Directory.Exists(current)) && (File.GetAttributes(current) & FileAttributes.ReparsePoint) != 0)
                throw new IOException("Preset paths must not pass through links.");
    }

    private bool Below(string path)
    {
        string relative = Path.GetRelativePath(root, path);
        return relative != "." && relative != ".." && !Path.IsPathRooted(relative) &&
            !relative.StartsWith(".." + Path.DirectorySeparatorChar, StringComparison.Ordinal);
    }

    public Preset Load(string path) => Presets.Load(Checked(path), schema);
    public void Save(string path, Preset preset) => Presets.Save(Checked(path), preset, schema);
}
