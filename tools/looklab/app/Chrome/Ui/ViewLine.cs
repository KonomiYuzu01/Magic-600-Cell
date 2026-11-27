using Godot;

namespace Magic600.Chrome;

// A readout line of its own on a view's frame, below the status (ported from the Jumble host's default-cap line, which
// now uses it): dim, wrapped, its tooltip the hint. No words remove the line, unless the caller keeps it: SF5 (27 Nov
// 2026), a kept line stays blank at the height it had, so a view's opening keeps its size while what the line reads
// is still asked for (a named cap at a state no sample holds, as after a twist).
public static class ViewLine
{
    public static void Set(ViewFrame view, string name, string? words, string? tip, Color colour, bool keep = false)
    {
        if (!GodotObject.IsInstanceValid(view)) return;
        Label? line = view.Readouts.GetNodeOrNull<Label>(name);
        if (words == null)
        {
            if (line == null) return;
            if (!JumbleTint.BorderRowKept(keep, true, "")) { view.Readouts.RemoveChild(line); line.QueueFree(); return; }
            line.CustomMinimumSize = new Vector2(0, System.Math.Max(line.CustomMinimumSize.Y, line.Size.Y));
            line.Text = ""; line.TooltipText = "";
            return;
        }
        if (line == null)
        {
            line = ChromeWidgets.Text("", "MicroLabel"); line.Name = name;
            line.AutowrapMode = TextServer.AutowrapMode.WordSmart; line.MouseFilter = Control.MouseFilterEnum.Pass;
            line.SizeFlagsHorizontal = Control.SizeFlags.ExpandFill; view.Readouts.AddChild(line);
        }
        line.CustomMinimumSize = Vector2.Zero;
        line.Text = words; line.TooltipText = tip ?? "";
        line.AddThemeColorOverride("font_color", colour);
    }
}
