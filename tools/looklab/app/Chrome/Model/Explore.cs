using System;
using System.Collections.Generic;

namespace Magic600.Chrome;

// ADR-D01.39: an extra refusal at the command boundary, independent of the layout and move history.
public sealed class ExploreState
{
    public const string Slot = "mark.explore";
    public const string CommitRefusal = "explore state: commits are paused";
    public bool On { get; private set; }
    public event Action? Changed;

    public void Set(bool on)
    {
        if (On == on) return;
        On = on;
        Safe.Raise(Changed);
    }

    public static string? ArgumentReason(string? argument) => argument is null or "on" or "off" or "true" or "false"
        ? null : "it takes on or off (true or false), or nothing to switch";

    public string? SetArgument(string? argument)
    {
        if (ArgumentReason(argument) is { } why) return why;
        Set(argument == null ? !On : argument is "on" or "true");
        return null;
    }

    public IReadOnlyList<ViewMark> BorderMarks(string view) => On
        ? new[] { new ViewMark("follow.pause", "explore: commits paused", "severity.warning",
            "Explore state: commits are paused. Set explore state off resumes them.") }
        : Array.Empty<ViewMark>();

    public string? Gate(ChromeCommand row) => On && row.Kind == "commit" && row.Id is not ("layout.undo" or "layout.redo")
        ? CommitRefusal : null;

    public Func<string, string?, string?> Compose(Func<string, string?, string?> inner, CommandRegistry registry) => (command, argument) =>
        (registry.Commands.TryGetValue(command, out ChromeCommand? row) ? Gate(row) : null) ?? inner(command, argument);
}
