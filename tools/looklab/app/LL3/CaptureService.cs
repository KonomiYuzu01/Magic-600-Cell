using System;
using System.Diagnostics;
using System.IO;
using System.Threading.Tasks;
using Godot;

namespace LookLab.App.LL3;

internal sealed class CaptureService
{
    private readonly Node host;
    private readonly string root;
    public bool Enabled { get; set; } = true;
    public string? Ffmpeg { get; } = CaptureEncoding.FindFfmpeg();
    public bool Busy { get; private set; }

    public CaptureService(Node host, string outputRoot) { this.host = host; root = outputRoot; }

    private string PathFor(string extension)
    {
        string name = "capture-" + DateTime.UtcNow.ToString("yyyyMMddTHHmmssZ", System.Globalization.CultureInfo.InvariantCulture) + "-" + Guid.NewGuid().ToString("N") + extension;
        string path = OutputPaths.FileAt(root, CaptureEncoding.Folder, name);
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        return path;
    }

    private static Image NativeImage(SubViewport viewport, Vector2I size)
    {
        Image image = viewport.GetTexture().GetImage();
        if (image == null || image.IsEmpty() || image.GetWidth() != size.X || image.GetHeight() != size.Y)
            throw new Exception("capture: viewport image must have its native pixel size");
        image.Convert(Image.Format.Rgba8);
        return image;
    }

    public async Task<string?> Still(SubViewport viewport)
    {
        if (!Enabled) return null;
        if (Busy) throw new Exception("A capture is already running.");
        Busy = true;
        try
        {
            await host.ToSignal(RenderingServer.Singleton, RenderingServer.SignalName.FramePostDraw);
            using Image image = NativeImage(viewport, viewport.Size);
            string path = PathFor(".png");
            if (image.SavePng(path) != Error.Ok) throw new IOException("capture: PNG could not be saved");
            CaptureEncoding.Publish(root, path);
            return path;
        }
        finally { Busy = false; }
    }

    public async Task<string?> Clip(SubViewport viewport, double seconds)
    {
        int frames = CaptureEncoding.Frames(seconds);
        if (!Enabled) return null;
        if (Ffmpeg == null) throw new Exception("Clip unavailable: FFmpeg is not installed locally.");
        if (Busy) throw new Exception("A capture is already running.");
        Busy = true;
        string? path = null;
        Process? process = null;
        try
        {
            Vector2I size = viewport.Size;
            path = PathFor(".mp4");
            process = Process.Start(CaptureEncoding.ProcessInfo(Ffmpeg, CaptureEncoding.Arguments(size.X, size.Y, seconds, path)))
                ?? throw new Exception("capture: FFmpeg could not start");
            Task<string> error = process.StandardError.ReadToEndAsync();
            Task<string> output = process.StandardOutput.ReadToEndAsync();
            var timer = Stopwatch.StartNew();
            for (int i = 0; i < frames; i++)
            {
                double delay = i / (double)CaptureEncoding.FrameRate - timer.Elapsed.TotalSeconds;
                if (delay > 0) await host.ToSignal(host.GetTree().CreateTimer(delay), SceneTreeTimer.SignalName.Timeout);
                await host.ToSignal(RenderingServer.Singleton, RenderingServer.SignalName.FramePostDraw);
                if (viewport.Size != size) throw new Exception("capture: keep the viewport size fixed until the clip finishes");
                using Image image = NativeImage(viewport, size);
                await process.StandardInput.BaseStream.WriteAsync(image.GetData()).AsTask().WaitAsync(TimeSpan.FromSeconds(15));
            }
            process.StandardInput.Close();
            await process.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(30));
            await error; await output;
            if (process.ExitCode != 0 || !File.Exists(path) || new FileInfo(path).Length == 0)
                throw new Exception("capture: local FFmpeg could not encode the clip (libx264 required)");
            CaptureEncoding.Publish(root, path);
            return path;
        }
        catch
        {
            if (process != null && !process.HasExited) { process.Kill(true); await process.WaitForExitAsync(); }
            if (path != null && File.Exists(path)) File.Delete(path);
            throw;
        }
        finally { process?.Dispose(); Busy = false; }
    }
}
