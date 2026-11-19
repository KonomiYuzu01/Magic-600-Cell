namespace Magic600.Shell.Lx4;

/// <summary>
/// LX-4's module in the product shell (C9 U8). This is the stub with the shell's hook points and no behaviour: it
/// consumes no message and writes no row. The C9 measurement line owns this folder from here on, and the docstring of
/// tools/perf/c9/lx4.py is its spec. The shell creates it when M600_LX4 is 1 (launch.py --lx4).
/// </summary>
public sealed class Lx4Module : IShellModule
{
    public Lx4Module(ShellContext context)
    {
        Context = context;
    }

    /// <summary>The window, the root control and the event log the shell gave this module.</summary>
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
