using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Threading;

// Windows-only measurement boundary, following S-B's window and power checks.
public sealed class Level2Windows : IDisposable
{
    [StructLayout(LayoutKind.Sequential)] private struct Rect { public int left, top, right, bottom; }
    [StructLayout(LayoutKind.Sequential)] private struct Point { public int x, y; public Point(int a, int b) { x = a; y = b; } }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] private struct MonitorInfo {
        public uint size; public Rect monitor, work; public uint flags;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string device;
    }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] private struct DevMode {
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string device;
        public ushort specVersion, driverVersion, size, driverExtra;
        public uint fields;
        public int positionX, positionY; public uint orientation, fixedOutput;
        public short color, duplex, yResolution, ttOption, collate;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string form;
        public ushort logPixels;
        public uint bitsPerPel, width, height, displayFlags, frequency;
        public uint icmMethod, icmIntent, mediaType, ditherType, reserved1, reserved2, panningWidth, panningHeight;
    }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] private struct DisplayDevice {
        public uint size;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string name;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string description;
        public uint flags;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string id, key;
    }
    [StructLayout(LayoutKind.Sequential)] private struct PowerStatus {
        public byte ac, flag, percent, reserved; public uint life, fullLife;
    }
    [UnmanagedFunctionPointer(CallingConvention.Winapi)] private delegate void PowerCallback(int mode, nint context);
    [DllImport("kernel32.dll")] private static extern bool QueryPerformanceCounter(out long value);
    [DllImport("kernel32.dll")] private static extern bool QueryPerformanceFrequency(out long value);
    [DllImport("kernel32.dll")] private static extern uint SetThreadExecutionState(uint flags);
    [DllImport("kernel32.dll")] private static extern bool GetSystemPowerStatus(out PowerStatus status);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] private static extern nint GetCommandLineW();
    [DllImport("shell32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern nint CommandLineToArgvW(nint commandLine, out int count);
    [DllImport("kernel32.dll")] private static extern nint LocalFree(nint memory);
    [DllImport("user32.dll")] private static extern nint MonitorFromWindow(nint window, uint flags);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern bool GetMonitorInfoW(nint monitor, ref MonitorInfo info);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern bool EnumDisplaySettingsW(string device, int mode, ref DevMode info);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern bool EnumDisplayDevicesW(string device, uint index, ref DisplayDevice info, uint flags);
    [DllImport("user32.dll")] private static extern bool GetClientRect(nint window, out Rect rect);
    [DllImport("user32.dll")] private static extern bool GetWindowRect(nint window, out Rect rect);
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(nint window);
    [DllImport("user32.dll")] private static extern bool IsIconic(nint window);
    [DllImport("user32.dll")] private static extern nint WindowFromPoint(Point point);
    [DllImport("user32.dll")] private static extern nint GetAncestor(nint window, uint flags);
    [DllImport("user32.dll")] private static extern nint GetForegroundWindow();
    [DllImport("user32.dll")] private static extern bool SetForegroundWindow(nint window);
    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")] private static extern nint GetWindowLongPtrW(nint window, int index);
    [DllImport("user32.dll")] private static extern nint GetThreadDpiAwarenessContext();
    [DllImport("user32.dll")] private static extern bool AreDpiAwarenessContextsEqual(nint a, nint b);
    [DllImport("dwmapi.dll")] private static extern int DwmGetWindowAttribute(nint window, uint attribute, out int value, uint size);
    [DllImport("powrprof.dll")] private static extern int PowerRegisterForEffectivePowerModeNotifications(uint version, PowerCallback callback, nint context, out nint registration);
    [DllImport("powrprof.dll")] private static extern int PowerUnregisterFromEffectivePowerModeNotifications(nint registration);

    private readonly nint _window;
    private readonly PowerCallback _callback;
    private nint _registration;
    private int _powerMode = -1, _firstPowerMode = -1;
    private bool _powerChanged;
    private long _samples, _mains, _battery, _notVisible, _covered, _notForeground;
    public long Frequency { get; }
    public bool DisplayRequired { get; }
    public bool Foreground => GetForegroundWindow() == _window;
    public bool Topmost => (GetWindowLongPtrW(_window, -20).ToInt64() & 8) != 0;
    // Godot 4.7.2 makes the process system DPI aware (FRAMEWORK-FACTS G10). Contexts: -4 per-monitor v2, -2 system aware.
    public static string DpiAwareness => Awareness(GetThreadDpiAwarenessContext());
    private static string Awareness(nint context) => AreDpiAwarenessContextsEqual(context, (nint)(-4)) ? "per-monitor-v2"
        : AreDpiAwarenessContextsEqual(context, (nint)(-2)) ? "system" : "unknown";

    public Level2Windows(nint window)
    {
        _window = window;
        if (window == 0 || !QueryPerformanceFrequency(out long frequency) || frequency <= 0)
            throw new InvalidOperationException("window or QPC unavailable");
        Frequency = frequency;
        _callback = (mode, context) => Interlocked.Exchange(ref _powerMode, mode);
        if (PowerRegisterForEffectivePowerModeNotifications(2, _callback, 0, out _registration) != 0) _registration = 0;
        DisplayRequired = SetThreadExecutionState(0x80000000u | 1u | 2u) != 0;
        SetForegroundWindow(window); // Exactly one request; no foreground-lock workaround.
    }

    public static long Clock()
    {
        if (!QueryPerformanceCounter(out long now)) throw new InvalidOperationException("QPC unavailable");
        return now;
    }

    public Dictionary<string, object> Display(out string adapter)
    {
        MonitorInfo monitor = new() { size = (uint)Marshal.SizeOf<MonitorInfo>() };
        if (!GetMonitorInfoW(MonitorFromWindow(_window, 1), ref monitor)) throw new InvalidOperationException("window monitor unavailable");
        DevMode mode = new() { size = (ushort)Marshal.SizeOf<DevMode>() };
        if (!EnumDisplaySettingsW(monitor.device, -1, ref mode)) throw new InvalidOperationException("window monitor mode unavailable");
        DisplayDevice device = new() { size = (uint)Marshal.SizeOf<DisplayDevice>() };
        adapter = null;
        for (uint i = 0; EnumDisplayDevicesW(null, i, ref device, 0); i++)
            if (device.name == monitor.device) { adapter = device.description; break; }
        return new() { ["width"] = (int)mode.width, ["height"] = (int)mode.height, ["refresh_hz"] = (int)mode.frequency };
    }

    public Level2Size ClientSize()
    {
        if (!GetClientRect(_window, out Rect r)) throw new InvalidOperationException("window client rectangle unavailable");
        return new(r.right - r.left, r.bottom - r.top);
    }

    private bool Covered()
    {
        if (!GetWindowRect(_window, out Rect r)) return true;
        int w = r.right - r.left, h = r.bottom - r.top;
        Point[] points = { new(r.left + w / 2, r.top + h / 2), new(r.left + w / 10, r.top + h / 10),
            new(r.right - w / 10, r.top + h / 10), new(r.left + w / 10, r.bottom - h / 10), new(r.right - w / 10, r.bottom - h / 10) };
        foreach (Point point in points)
        {
            nint hit = WindowFromPoint(point);
            if (hit == 0 || GetAncestor(hit, 2) != _window) return true;
        }
        return false;
    }

    public bool Sample(Level2Result result)
    {
        // A failed cloak query is unknown visibility, so enforce mode stops.
        bool hidden = DwmGetWindowAttribute(_window, 14, out int cloaked, 4) != 0 || cloaked != 0;
        bool covered = Covered(), foreground = Foreground;
        bool visible = IsWindowVisible(_window) && !IsIconic(_window) && !hidden && !covered;
        int mode = Volatile.Read(ref _powerMode);
        if (_samples == 0) _firstPowerMode = mode;
        else if (mode != _firstPowerMode) _powerChanged = true;
        _samples++;
        if (!visible) _notVisible++;
        if (covered) _covered++;
        if (!foreground) _notForeground++;
        if (GetSystemPowerStatus(out PowerStatus power))
        {
            if (power.ac == 1) _mains++;
            else if (power.ac == 0) _battery++;
        }
        result.window["samples"] = _samples;
        result.window["samples_not_visible"] = _notVisible;
        result.window["samples_covered"] = _covered;
        result.window["samples_not_foreground"] = _notForeground;
        result.window["visible_throughout"] = _notVisible == 0;
        result.window["foreground_throughout"] = _notForeground == 0;
        result.environment["power_samples"] = new Dictionary<string, object> { ["samples"] = _samples, ["mains"] = _mains, ["battery"] = _battery };
        result.environment["power_source"] = Level2Result.PowerSource(_samples, _mains, _battery);
        result.environment["power_mode"] = _powerChanged ? "changed" : PowerModeName(_firstPowerMode);
        return visible && foreground;
    }

    private static string PowerModeName(int mode) => mode switch {
        0 => "battery_saver", 1 => "better_battery", 2 => "balanced", 3 => "high_performance", 4 => "max_performance",
        5 => "game_mode", 6 => "mixed_reality", _ => "unknown"
    };

    public static string[] ProcessArguments()
    {
        nint arguments = CommandLineToArgvW(GetCommandLineW(), out int count);
        if (arguments == 0) throw new Win32Exception(Marshal.GetLastWin32Error());
        try
        {
            string[] values = new string[count];
            for (int i = 0; i < count; i++) values[i] = Marshal.PtrToStringUni(Marshal.ReadIntPtr(arguments, i * IntPtr.Size));
            return values;
        }
        finally { LocalFree(arguments); }
    }

    public static string EngineArguments(string[] processArguments)
    {
        List<string> kept = new();
        for (int i = 1; i < processArguments.Length && processArguments[i] != "--"; i++)
        {
            string value = processArguments[i];
            if (value == "--path" || value == "--log-file") { i++; continue; }
            if (value == "--gpu-validation" || value == "--gpu-abort") continue;
            kept.Add(value);
        }
        // One flat, unambiguous string; it is identity data, never re-executed.
        return System.Text.Json.JsonSerializer.Serialize(kept);
    }

    public void Dispose()
    {
        if (_registration != 0) { PowerUnregisterFromEffectivePowerModeNotifications(_registration); _registration = 0; }
        SetThreadExecutionState(0x80000000u); // Must run on the main thread that requested it.
        GC.KeepAlive(_callback);
    }
}
