using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;

namespace LookLab.App.LL3;

internal static class CaptureEncoding
{
    public const int FrameRate = 24;
    public const int MaxSeconds = 10;
    public const string Folder = "work/loop-memory/looklab/captures";
    public const string GalleryFolder = "work/gallery/looklab";

    public static int Frames(double seconds)
    {
        if (!double.IsFinite(seconds) || seconds <= 0 || seconds > MaxSeconds)
            throw new FormatException("capture: clip duration must be positive and at most 10 seconds");
        return Math.Max(1, (int)Math.Floor(seconds * FrameRate));
    }

    public static string? FindFfmpeg()
    {
        string? configured = Environment.GetEnvironmentVariable("LOOKLAB_FFMPEG");
        if (!string.IsNullOrWhiteSpace(configured)) return File.Exists(configured) ? Path.GetFullPath(configured) : null;
        string executable = OperatingSystem.IsWindows() ? "ffmpeg.exe" : "ffmpeg";
        return (Environment.GetEnvironmentVariable("PATH") ?? "").Split(Path.PathSeparator)
            .Where(folder => !string.IsNullOrWhiteSpace(folder)).Select(folder => Path.Combine(folder.Trim('"'), executable))
            .FirstOrDefault(File.Exists);
    }

    public static IReadOnlyList<string> Arguments(int width, int height, double seconds, string output)
    {
        if (width <= 0 || height <= 0) throw new FormatException("capture: positive viewport size required");
        int frames = Frames(seconds);
        return new[] { "-nostdin", "-hide_banner", "-loglevel", "error", "-n", "-f", "rawvideo", "-pixel_format", "rgba",
            "-video_size", width.ToString(CultureInfo.InvariantCulture) + "x" + height.ToString(CultureInfo.InvariantCulture),
            "-framerate", "24", "-i", "pipe:0", "-an", "-vf", "pad=ceil(iw/2)*2:ceil(ih/2)*2", "-c:v", "libx264",
            "-pix_fmt", "yuv420p", "-r", "24", "-frames:v", frames.ToString(CultureInfo.InvariantCulture),
            "-t", (frames / (double)FrameRate).ToString("R", CultureInfo.InvariantCulture), "-movflags", "+faststart", output };
    }

    public static ProcessStartInfo ProcessInfo(string executable, IEnumerable<string> arguments)
    {
        var info = new ProcessStartInfo(executable) { UseShellExecute = false, CreateNoWindow = true,
            RedirectStandardInput = true, RedirectStandardError = true, RedirectStandardOutput = true };
        foreach (string argument in arguments) info.ArgumentList.Add(argument);
        foreach (string key in info.Environment.Keys.ToArray())
            if (System.Text.RegularExpressions.Regex.IsMatch(key, "KEY|SECRET|TOKEN", System.Text.RegularExpressions.RegexOptions.IgnoreCase))
                info.Environment.Remove(key);
        return info;
    }

    public static string Publish(string root, string path)
    {
        string gallery = OutputPaths.FileAt(root, GalleryFolder, Path.GetFileName(path));
        Directory.CreateDirectory(Path.GetDirectoryName(gallery)!);
        File.Copy(path, gallery, false);
        return gallery;
    }
}
