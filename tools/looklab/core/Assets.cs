using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;

namespace LookLab.Core;

public sealed class AssetCatalogue
{
    private readonly string folder;
    private readonly Dictionary<string, string> digests = new(StringComparer.Ordinal);
    public string ModelId { get; }

    public AssetCatalogue(string repositoryRoot)
    {
        folder = Path.Combine(Path.GetFullPath(repositoryRoot), "assets");
        using var manifest = Json.Parse(File.ReadAllText(Path.Combine(folder, "manifest.json")));
        ModelId = Json.Text(manifest.RootElement, "model_id");
        foreach (var entry in Json.Get(manifest.RootElement, "files").EnumerateObject())
            digests.Add(entry.Name, Json.String(entry.Value, entry.Name));
    }

    public byte[] ReadVerified(string name)
    {
        string key = "assets/" + name;
        if (name != Path.GetFileName(name) || name.Contains('/') || name.Contains('\\'))
            throw Json.Error(key, "expected an asset filename");
        if (!digests.TryGetValue(key, out string? expected)) throw Json.Error(key, "missing manifest entry");
        string path = Path.Combine(folder, name);
        if (!File.Exists(path)) throw Json.Error(key, "missing file");
        byte[] bytes = File.ReadAllBytes(path);
        if (!string.Equals(Digest(bytes), expected, StringComparison.OrdinalIgnoreCase))
            throw Json.Error(key, "SHA-256 mismatch with manifest.json");
        return bytes;
    }

    public static string Digest(ReadOnlySpan<byte> bytes) => Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
}

public sealed record NpyArray(string Dtype, int[] Shape, byte[] Data)
{
    public double[] Doubles()
    {
        if (Dtype != "<f8") throw Json.Error("dtype", "expected <f8");
        var result = new double[Data.Length / 8];
        for (int i = 0; i < result.Length; i++)
        {
            result[i] = BinaryPrimitives.ReadDoubleLittleEndian(Data.AsSpan(i * 8, 8));
            if (!double.IsFinite(result[i])) throw Json.Error("normals.npy", "non-finite value");
        }
        return result;
    }

    public int[] Integers()
    {
        if (Dtype != "<i2" && Dtype != "<i4") throw Json.Error("dtype", "expected signed integer");
        int width = Dtype == "<i2" ? 2 : 4;
        var result = new int[Data.Length / width];
        for (int i = 0; i < result.Length; i++) result[i] = width == 2
            ? BinaryPrimitives.ReadInt16LittleEndian(Data.AsSpan(i * width, width))
            : BinaryPrimitives.ReadInt32LittleEndian(Data.AsSpan(i * width, width));
        return result;
    }
}

public static class NpyReader
{
    // The retained arrays have only these three header fields. No Python evaluation.
    private static readonly Regex Header = new(
        "^\\s*\\{\\s*['\"]descr['\"]\\s*:\\s*['\"](?<dtype>[^'\"]+)['\"]\\s*,\\s*['\"]fortran_order['\"]\\s*:\\s*(?<order>True|False)\\s*,\\s*['\"]shape['\"]\\s*:\\s*\\((?<shape>[0-9, ]+)\\)\\s*,?\\s*\\}\\s*$",
        RegexOptions.CultureInvariant);

    public static NpyArray Read(byte[] bytes, string expectedDtype, string name)
    {
        if (bytes.Length < 10 || !bytes.AsSpan(0, 6).SequenceEqual(new byte[] { 147, 78, 85, 77, 80, 89 }))
            throw Json.Error(name, "invalid .npy magic");
        int major = bytes[6];
        if (bytes[7] != 0 || major < 1 || major > 3) throw Json.Error(name, "unsupported .npy version");
        int offset = major == 1 ? 10 : 12;
        if (bytes.Length < offset) throw Json.Error(name, "truncated .npy header");
        long length = major == 1 ? BinaryPrimitives.ReadUInt16LittleEndian(bytes.AsSpan(8, 2))
            : BinaryPrimitives.ReadUInt32LittleEndian(bytes.AsSpan(8, 4));
        if (length > bytes.Length - offset) throw Json.Error(name, "truncated .npy header");
        Match match = Header.Match(Encoding.UTF8.GetString(bytes, offset, (int)length));
        if (!match.Success) throw Json.Error(name, "unsupported .npy header");
        string dtype = match.Groups["dtype"].Value;
        if (dtype != expectedDtype) throw Json.Error(name, "dtype " + dtype + "; expected " + expectedDtype);
        if (match.Groups["order"].Value != "False") throw Json.Error(name, "Fortran order is unsupported");
        int width = dtype switch { "<f8" => 8, "<i2" => 2, "<i4" => 4, _ => throw Json.Error(name, "unsupported dtype") };
        string[] dimensions = match.Groups["shape"].Value.Split(',', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries);
        var shape = new List<int>();
        long count = 1;
        foreach (string dimension in dimensions)
        {
            if (!int.TryParse(dimension, out int size) || size <= 0 || count > int.MaxValue / size)
                throw Json.Error(name, "invalid shape");
            shape.Add(size);
            count *= size;
        }
        if (shape.Count == 0) throw Json.Error(name, "array shape required");
        offset += (int)length;
        if (count * width != bytes.Length - offset) throw Json.Error(name, "shape/data length mismatch");
        return new NpyArray(dtype, shape.ToArray(), bytes.AsSpan(offset).ToArray());
    }

    internal static NpyArray Entry(ZipArchive archive, string name, string dtype, params int[] shape)
    {
        ZipArchiveEntry? entry = archive.GetEntry(name);
        if (entry == null) throw Json.Error("model.npz/" + name, "missing entry");
        using Stream stream = entry.Open();
        using var memory = new MemoryStream();
        stream.CopyTo(memory);
        NpyArray array = Read(memory.ToArray(), dtype, name);
        if (!array.Shape.SequenceEqual(shape)) throw Json.Error(name, "unexpected shape");
        return array;
    }
}
