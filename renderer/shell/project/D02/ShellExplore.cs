using System;
using System.Linq;
using System.Text.Json;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

internal static class ShellExplore
{
    public static ExploreState State { get; } = new();
    private static ChromeFrame? frame;

    public static event Action? Changed
    {
        add => State.Changed += value;
        remove => State.Changed -= value;
    }

    public static void Load(ChromeFrame host)
    {
        frame = host;
        State.Set(false);
    }

    public static bool Handles(string command) => command == "explore.set";

    public static string? Unavailable(string? argument) => ExploreState.ArgumentReason(argument);

    public static void Execute(ChromeFrame frame, string? argument)
    {
        if (State.SetArgument(argument) is { } why) { frame.Say(why, true); return; }
        frame.Say(State.On ? "Explore state on: commits are paused" : "Explore state off");
    }

    public static void Marks(ViewFrame view)
    {
        foreach (ViewMark mark in State.BorderMarks(view.ViewName))
            view.AddMark(mark.Glyph, mark.Words, mark.Colour, mark.Tooltip).SetMeta("slot", ExploreState.Slot);
    }

    public static void Dump(Utf8JsonWriter json)
    {
        if (frame is not { } host) return;
        json.WriteStartObject("explore");
        json.WriteBoolean("on", State.On);
        json.WriteNumber("global", Count(host.Global, ExploreState.Slot));
        json.WriteNumber("local", Count(host.Local, ExploreState.Slot));
        json.WriteEndObject();

        json.WriteStartObject("colours");
        json.WriteBoolean("nonstandard", host.Tokens.NonStandardColours);
        json.WriteString("digest", ColourMark.Digest(host.Tokens));
        json.WriteNumber("global", Count(host.Global, ColourMark.Slot));
        json.WriteNumber("local", Count(host.Local, ColourMark.Slot));
        json.WriteEndObject();
    }

    private static int Count(ViewFrame? view, string slot) => view?.Marks.GetChildren().Count(mark =>
        mark.HasMeta("slot") && mark.GetMeta("slot").AsString() == slot) ?? 0;
}
