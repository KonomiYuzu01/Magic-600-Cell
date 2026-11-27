using System;
using System.Globalization;
using System.Runtime.InteropServices;
using Godot;

namespace Magic600.Shell.D02;

/// <summary>
/// The owner's window rule for a watched D02 start (26 November 2026): the shell's window is never on top and never
/// takes the foreground. The launcher starts the process with SW_SHOWNOACTIVATE and names, in M600_D02_FOREGROUND, the
/// window that had the foreground just before. Godot 4.7.2 still shows its main window with SW_SHOW and then calls
/// SetForegroundWindow (display_server_windows.cpp, show_window), which Windows allows when the launch descends from
/// the foreground process, for example while the owner is in the Claude app, or after 200 s without input.
/// So the shell undoes it during its first seconds:
/// - at its first check, in Shell._Ready, it always hands the foreground back when its own window has it, since only
///   Godot's showing can have activated it then;
/// - at every later check, when its own window has the foreground it hands it back, and otherwise it places itself
///   just behind the window that has the foreground, so it does not cover the owner's work.
/// Each hand-back places the shell's window just behind the window that gets the foreground back. The later checks stop
/// for good once the owner presses a mouse button anywhere, once the window moves or changes size (a drag, a
/// maximise), or once the shell's window comes back to the front after a check found another window there: only the
/// owner brings it back (a click, the taskbar, Alt+Tab), even while a long session open held the checks up (checkpoint
/// C1, 26 November 2026). From then on the owner's clicks and switches stand. Keys alone do not stop them, since keys
/// typed elsewhere land in the shell while Godot holds the foreground. The shell never activates itself, never raises its own window,
/// never asks for the foreground for itself and never sets a topmost flag; it stops the taskbar flash a refused
/// activation starts. Those checks are <see cref="Settle"/>; after the first seconds <see cref="Watch"/> takes over for
/// the window's life (B32 findings): it notes the owner's acts on the window and moves no window.
/// </summary>
internal static class D02Window
{
    private const int GwlExStyle = -20;
    private const long WsExTopmost = 0x8;
    private const uint SwpNoSize = 0x1, SwpNoMove = 0x2, SwpNoActivate = 0x10;
    private const uint FlashwStop = 0;
    private const int VkLButton = 0x01, VkRButton = 0x02, VkMButton = 0x04;
    /// <summary>SetWindowPos's HWND_NOTOPMOST: behind every topmost window, and it clears the topmost style.</summary>
    private static readonly IntPtr HwndNoTopmost = new(-2);
    /// <summary>SetWindowPos's HWND_BOTTOM: behind every other window.</summary>
    private static readonly IntPtr HwndBottom = new(1);

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

    [DllImport("user32.dll")]
    private static extern short GetAsyncKeyState(int key);

    /// <summary>How many times the shell handed the foreground back.</summary>
    public static int GaveBack { get; private set; }

    /// <summary>True once the owner pressed a mouse button or moved or resized the window: the checks have stopped.
    /// </summary>
    public static bool OwnerActed { get; private set; }

    private static bool _checked;
    // A check found another window in front: from then on the shell's window comes back only by the owner's act.
    private static bool _sawElsewhere;
    private static Vector2I _position, _size;

    private static IntPtr Own() => (IntPtr)DisplayServer.WindowGetNativeHandle(DisplayServer.HandleType.WindowHandle);

    private static IntPtr Previous() =>
        long.TryParse(System.Environment.GetEnvironmentVariable("M600_D02_FOREGROUND"), NumberStyles.Integer, CultureInfo.InvariantCulture, out var value)
            ? (IntPtr)value : IntPtr.Zero;

    private static bool MouseButtonDown() =>
        (GetAsyncKeyState(VkLButton) & 0x8000) != 0 || (GetAsyncKeyState(VkRButton) & 0x8000) != 0 ||
        (GetAsyncKeyState(VkMButton) & 0x8000) != 0;

    /// <summary>One check: undoes Godot's own activation, keeps the window behind the owner's, stops a taskbar flash.
    /// Does nothing once the owner acted.</summary>
    public static void Settle()
    {
        var own = Own();
        if (own == IntPtr.Zero || OwnerActed)
        {
            return;
        }
        var first = !_checked;
        if (first)
        {
            _checked = true;
            _position = DisplayServer.WindowGetPosition();
            _size = DisplayServer.WindowGetSize();
        }
        else if (MouseButtonDown() || DisplayServer.WindowGetPosition() != _position || DisplayServer.WindowGetSize() != _size)
        {
            OwnerActed = true;
            return;
        }
        var previous = Previous();
        var foreground = GetForegroundWindow();
        if (!first && foreground == own && _sawElsewhere)
        {
            // Back in front after another window had it: the owner switched to the shell, and the switch stands.
            OwnerActed = true;
            return;
        }
        if (foreground == own && previous != IntPtr.Zero && previous != own && IsWindow(previous))
        {
            // The previous window gets its foreground back; the shell sits just behind it, not activated.
            SetForegroundWindow(previous);
            Behind(own, previous);
            GaveBack++;
        }
        else if (foreground != own && foreground != IntPtr.Zero)
        {
            // Shown without activation, the window may still sit above the owner's work: it goes just behind it.
            Behind(own, foreground);
        }
        if (GetForegroundWindow() is var now && now != own && now != IntPtr.Zero)
        {
            _sawElsewhere = true;
        }
        var flash = new FlashInfo { Size = (uint)Marshal.SizeOf<FlashInfo>(), Window = own, Flags = FlashwStop };
        FlashWindowEx(ref flash);
    }

    /// <summary>
    /// After the first seconds (B32 findings): notes the owner's acts on the shell's window for its life, and moves no
    /// window. A move or resize of the window, or the window in front (the shell never activates itself after the
    /// first seconds, so only the owner's switch or click puts it there), is the owner's act. A mouse button elsewhere
    /// on the desktop is not: <see cref="Settle"/> counts it only while it may still hand the foreground back.
    /// </summary>
    public static void Watch()
    {
        var own = Own();
        if (own == IntPtr.Zero || OwnerActed || !_checked)
        {
            return;
        }
        if (GetForegroundWindow() == own || DisplayServer.WindowGetPosition() != _position || DisplayServer.WindowGetSize() != _size)
        {
            OwnerActed = true;
        }
    }

    /// <summary>
    /// Places the shell's window just behind <paramref name="other"/>, without activating it. Windows makes a window
    /// topmost when it lands among topmost windows, so behind a topmost window (the taskbar, for one) the shell goes
    /// behind the window the owner had in front at the launch, or behind every window when that one is gone; it never
    /// goes to the top of the normal windows (checkpoint C1). A topmost style that appears anyway is cleared at once.
    /// </summary>
    private static void Behind(IntPtr own, IntPtr other)
    {
        var previous = Previous();
        var after = !IsTopmost(other) ? other
            : previous != IntPtr.Zero && previous != own && IsWindow(previous) && !IsTopmost(previous) ? previous
            : HwndBottom;
        SetWindowPos(own, after, 0, 0, 0, 0, SwpNoSize | SwpNoMove | SwpNoActivate);
        if (IsTopmost(own))
        {
            SetWindowPos(own, HwndNoTopmost, 0, 0, 0, 0, SwpNoSize | SwpNoMove | SwpNoActivate);
            SetWindowPos(own, HwndBottom, 0, 0, 0, 0, SwpNoSize | SwpNoMove | SwpNoActivate);
        }
    }

    private static bool IsTopmost(IntPtr window) => (GetWindowLongPtrW(window, GwlExStyle).ToInt64() & WsExTopmost) != 0;

    /// <summary>Whether the shell's own window has the foreground.</summary>
    public static bool Foreground() => Own() is var own && own != IntPtr.Zero && GetForegroundWindow() == own;

    /// <summary>Whether the shell's own window carries the topmost style (it never should).</summary>
    public static bool Topmost() => Own() is var own && own != IntPtr.Zero && IsTopmost(own);
}
