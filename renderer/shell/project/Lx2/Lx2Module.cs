namespace Magic600.Shell.Lx2;

/// <summary>
/// LX-2's module in the product shell (C9; RTE.42 conditions 3 and 4: the renderer gate, LX-4's echo bar and charter
/// E2's commits during the runtime's fault windows). This is the stub with the shell's hook points and no behaviour:
/// it consumes no message, writes no row and sets no fault variable. The C9 measurement line owns this folder from
/// here on. The shell creates it when M600_LX2 is 1 (launch.py --lx2-injection), and the host's runtime is then
/// c600-lx2's injection child.
/// </summary>
public sealed class Lx2Module : IShellModule
{
    public Lx2Module(ShellContext context)
    {
        Context = context;
    }

    /// <summary>The window, the root control, the event log and the host the shell gave this module.</summary>
    public ShellContext Context { get; }

    public bool OnWindowMessage(uint message, nuint wParam, nint lParam, long qpc) => false;

    public void OnFrame(HostLibrary.Submitted frame, long qpc)
    {
    }

    public void OnPresented(HostLibrary.Presented row)
    {
    }

    public void Dispose()
    {
    }
}
