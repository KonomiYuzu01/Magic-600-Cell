using System.Collections.Generic;
using Godot;

namespace Magic600.Chrome;

// The visible text of a frame, pane by pane (ADR-D02 build step B4): each visible label, button, text field and rich
// text in tree order, under the group of its nearest view, dock panel or overlay. A scripted run dumps it after every
// step, so a check reads the screen as the owner sees it, in words.
public static class ScreenText
{
    public static IReadOnlyList<(string Group, string Text)> Of(Control root)
    {
        var lines = new List<(string, string)>();
        Walk(root, "frame", lines);
        return lines;
    }

    private static void Walk(Node node, string group, List<(string, string)> lines)
    {
        if (node is CanvasItem { Visible: false }) return;
        if (node is Control control) group = Group(control) ?? group;
        string? text = node switch
        {
            Label label => label.Text,
            Button button => button.Text,
            LineEdit field => field.Text,
            RichTextLabel rich => rich.GetParsedText(),
            _ => null,
        };
        if (!string.IsNullOrWhiteSpace(text)) lines.Add((group, text.Trim()));
        foreach (Node child in node.GetChildren()) Walk(child, group, lines);
    }

    private static string? Group(Control control) => control switch
    {
        ViewFrame view => "view:" + view.ViewName,
        PanelStateBox panel => "panel:" + panel.Id,
        ChromeOverlay overlay => "overlay:" + overlay.Name,
        _ => null,
    };
}
