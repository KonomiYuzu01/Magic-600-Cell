using System;
using Godot;

namespace Magic600.Shell;

/// <summary>What the shell gives a module when it creates it (C9 U8's hook points, a draft).</summary>
/// <param name="Window">The shell's window (its HWND); the window hook delivers this window's messages.</param>
/// <param name="Root">The shell's root control; a module may add its own nodes under it.</param>
/// <param name="Log">The event log, or null when M600_EVENTS_DIR is not set; <see cref="EventLog.Append"/> writes a
/// module's own rows.</param>
public sealed record ShellContext(IntPtr Window, Control Root, EventLog? Log);

/// <summary>
/// A module the shell drives, such as LX-4's (<see cref="Lx4.Lx4Module"/>). The shell creates its modules when the core
/// opens and disposes them before the event log closes. Every call is on Godot's main thread, which is the window's
/// thread.
/// </summary>
public interface IShellModule : IDisposable
{
    /// <summary>
    /// Every message the shell's window procedure receives, before Godot sees it; <paramref name="qpc"/> is
    /// <see cref="EventLog.Now"/> at receipt. Return true to consume the message (Godot never sees it, and the window
    /// procedure answers 0), false to pass it on. Godot's loop runs TranslateMessage first, so a posted WM_KEYDOWN of
    /// a character key is followed by its WM_CHAR.
    /// </summary>
    bool OnWindowMessage(uint message, nuint wParam, nint lParam, long qpc);

    /// <summary>After each 3D frame the host submitted (m6h_frame answered ok), with the frame row's QPC.</summary>
    void OnFrame(HostLibrary.Submitted frame, long qpc);

    /// <summary>After each presentation the core confirmed.</summary>
    void OnPresented(HostLibrary.Presented row);
}
