using System;
using System.Globalization;
using System.Runtime.InteropServices;
using Godot;

namespace Magic600.Shell.D02;

/// <summary>
/// The owner's window rule for a watched D02 start (26 November 2026): the shell's window is never on top and never
/// takes the foreground. The launcher starts the process with SW_SHOWNOACTIVATE and names, in M600_D02_FOREGROUND, the
/// window that had the foreground just before. Godot 4.7.2 still shows its main window with SW_SHOW and then calls
/// SetForegroundWindow (display_server_windows.cpp, show_window), which Windows allows for example after 200 s
/// without input. So the shell checks during its first seconds: when its own window has the foreground, it hands the
/// foreground back to that window and places itself just behind it, without activating itself. It stops the taskbar
/// flash a refused activation starts. It never raises its own window, never asks for the foreground for itself and
/// never sets a topmost flag.
/// </summary>
internal static class D02Window
{
    private const int GwlExStyle = -20;
    private const long WsExTopmost = 0x8;
    private const uint SwpNoSize = 0x1, SwpNoMove = 0x2, SwpNoActivate = 0x10;
    private const uint FlashwStop = 0;

    [StructLayout(LayoutKind.Sequential)]
    private struct FlashInfo
    {
        public uint Size;
        public IntPtr Window;
        public uint Flags;
        public uint Count;
        public uint Timeout;
    }

    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetForegroundWindow(IntPtr window);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr window);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);

    [DllImport("user32.dll")]
    private static extern IntPtr GetWindowLongPtrW(IntPtr window, int index);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool FlashWindowEx(ref FlashInfo info);

    /// <summary>How many times the shell handed the foreground back.</summary>
    public static int GaveBack { get; private set; }

    private static IntPtr Own() => (IntPtr)DisplayServer.WindowGetNativeHandle(DisplayServer.HandleType.WindowHandle);

    private static IntPtr Previous() =>
        long.TryParse(System.Environment.GetEnvironmentVariable("M600_D02_FOREGROUND"), NumberStyles.Integer, CultureInfo.InvariantCulture, out var value)
            ? (IntPtr)value : IntPtr.Zero;

    /// <summary>Hands the foreground back when the shell's own window has it, and stops a taskbar flash.</summary>
    public static void Settle()
    {
        var own = Own();
        if (own == IntPtr.Zero)
        {
            return;
        }
        var previous = Previous();
        if (GetForegroundWindow() == own && previous != IntPtr.Zero && previous != own && IsWindow(previous))
        {
            // The previous window gets its foreground back; the shell sits just behind it, not activated.
            SetForegroundWindow(previous);
            SetWindowPos(own, previous, 0, 0, 0, 0, SwpNoSize | SwpNoMove | SwpNoActivate);
            GaveBack++;
        }
        var flash = new FlashInfo { Size = (uint)Marshal.SizeOf<FlashInfo>(), Window = own, Flags = FlashwStop };
        FlashWindowEx(ref flash);
    }

    /// <summary>Whether the shell's own window has the foreground.</summary>
    public static bool Foreground() => Own() is var own && own != IntPtr.Zero && GetForegroundWindow() == own;

    /// <summary>Whether the shell's own window carries the topmost style (it never should).</summary>
    public static bool Topmost() => Own() is var own && own != IntPtr.Zero && (GetWindowLongPtrW(own, GwlExStyle).ToInt64() & WsExTopmost) != 0;
}
