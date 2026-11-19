using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using Godot;

namespace Magic600.Shell;

/// <summary>
/// The shell's one window-procedure subclass (C9 U8's hook point, a draft that was never launched). It hands every
/// message of the shell's window to the modules in order, and to Godot's own procedure unless a module consumed it.
/// It is the shell's own C#; the host library makes no Windows call (CT-RPI-20). One shell window per process.
/// </summary>
internal static unsafe class WindowHook
{
    private const int GwlpWndProc = -4;

    private static IntPtr _window;
    private static IntPtr _original;
    private static IShellModule[] _modules = Array.Empty<IShellModule>();

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SetWindowLongPtrW(IntPtr window, int index, IntPtr value);

    [DllImport("user32.dll")]
    private static extern IntPtr GetWindowLongPtrW(IntPtr window, int index);

    [DllImport("user32.dll")]
    private static extern IntPtr CallWindowProcW(IntPtr previous, IntPtr window, uint message, nuint wParam, nint lParam);

    private static IntPtr Procedure() => (IntPtr)(delegate* unmanaged[Stdcall]<IntPtr, uint, nuint, nint, nint>)&Receive;

    /// <summary>Subclasses <paramref name="window"/> for <paramref name="modules"/>; throws when it cannot.</summary>
    public static void Install(IntPtr window, IReadOnlyList<IShellModule> modules)
    {
        if (_original != IntPtr.Zero)
        {
            throw new InvalidOperationException("the window hook is already installed");
        }
        var list = new IShellModule[modules.Count];
        for (var i = 0; i < list.Length; i++)
        {
            list[i] = modules[i];
        }
        _modules = list;
        _window = window;
        Marshal.SetLastPInvokeError(0);
        var previous = SetWindowLongPtrW(window, GwlpWndProc, Procedure());
        if (previous == IntPtr.Zero)
        {
            _modules = Array.Empty<IShellModule>();
            throw new Win32Exception(Marshal.GetLastPInvokeError(), "SetWindowLongPtrW(GWLP_WNDPROC) failed");
        }
        _original = previous;
    }

    /// <summary>
    /// Stops handing messages to the modules and gives Godot its procedure back. When another subclass came after this
    /// one, the procedure stays in place and only forwards, since removing it would cut that subclass's chain.
    /// </summary>
    public static void Remove()
    {
        _modules = Array.Empty<IShellModule>();
        if (_original == IntPtr.Zero)
        {
            return;
        }
        if (GetWindowLongPtrW(_window, GwlpWndProc) == Procedure())
        {
            SetWindowLongPtrW(_window, GwlpWndProc, _original);
            _original = IntPtr.Zero;
            _window = IntPtr.Zero;
        }
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvStdcall) })]
    private static nint Receive(IntPtr window, uint message, nuint wParam, nint lParam)
    {
        var qpc = EventLog.Now();
        foreach (var module in _modules)
        {
            try
            {
                if (module.OnWindowMessage(message, wParam, lParam, qpc))
                {
                    return 0;
                }
            }
            catch (Exception error)
            {
                // An exception must not cross into the native caller; the message goes on to the next module.
                GD.PushError($"a shell module failed on message 0x{message:X4}: {error.Message}");
            }
        }
        return CallWindowProcW(_original, window, message, wParam, lParam);
    }
}
