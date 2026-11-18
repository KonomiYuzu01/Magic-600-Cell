using Godot;

namespace Magic600.Shell;

/// <summary>
/// The shell's root node. So far it only makes the window's background transparent, so that the core's composition
/// layer beneath the window shows through where the UI draws nothing (route B, accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks). Loading the core, the bridge and
/// the frame loop come in the next steps of C9's shell.
/// </summary>
public partial class Shell : Control
{
    public override void _Ready()
    {
        GetViewport().TransparentBg = true;
    }
}
