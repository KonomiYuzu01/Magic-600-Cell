using Godot;

namespace Magic600.Chrome;

// A readout line of its own on a view's frame, below the status (ported from the Jumble host's default-cap line, which
// now uses it): dim, wrapped, its tooltip the hint. No words remove the line.
public static class ViewLine
{
    public static void Set(ViewFrame view, string name, string? words, string? tip, Color colour)
    {
        if (!GodotObject.IsInstanceValid(view)) return;
        Label? line = view.Readouts.GetNodeOrNull<Label>(name);
        if (words == null)
        {
            if (line != null) { view.Readouts.RemoveChild(line); line.QueueFree(); }
            return;
        }
        if (line == null)
        {
            line = ChromeWidgets.Text("", "MicroLabel"); line.Name = name;
            line.AutowrapMode = TextServer.AutowrapMode.WordSmart; line.MouseFilter = Control.MouseFilterEnum.Pass;
            line.SizeFlagsHorizontal = Control.SizeFlags.ExpandFill; view.Readouts.AddChild(line);
        }
        line.Text = words; line.TooltipText = tip ?? "";
        line.AddThemeColorOverride("font_color", colour);
    }
}
