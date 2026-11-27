using System;
using System.Collections.Generic;
using System.Text.Json;
using Godot;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

/// <summary>
/// ADR-D02.18 and CR-14 (build step B21): Local's raw view in the shell. Local draws the stereographic chart from the
/// named cap's pole (SA2_PROJECTION_STEREOGRAPHIC), clipped at the half-space x·p̂ ≥ h, the named cap's cut by default;
/// one key adds the antipode chart beside it (<see cref="ViewPanes.Split"/>, the right pane with SA2_PANE_ANTIPODE).
/// The raw view replaces the cut slice while it is on (Local's slot draws no slice), and Local's line and border carry
/// the clip, the meeting poles' tick, the antipode's label and the caption. The keys are placeholders (row requests):
/// <see cref="RawView.RawKey"/>, <see cref="RawView.AntipodeKey"/>, <see cref="RawView.NarrowKey"/> and
/// <see cref="RawView.WidenKey"/>; the palette's rows are view.mode.raw, view.mode.raw.antipode and view.mode.raw.clip.
/// </summary>
public static class ShellRaw
{
    /// <summary>The pure model: on, the antipode chart and the clip.</summary>
    public static RawView Model { get; } = new();

    /// <summary>Raised when the raw view changed, so the chrome redraws Local's marks.</summary>
    public static event Action? Applied;

    /// <summary>Whether Local draws the raw view.</summary>
    public static bool On => Model.On;

    /// <summary>The raw view while it is on, for Local's border marks; null while it is off.</summary>
    public static RawView? Shown => Model.On ? Model : null;

    /// <summary>Local's clip h while the raw view is on (the focus Local draws with); null while it is off.</summary>
    public static double? Focus => Model.On ? Model.H : null;

    /// <summary>What Local's pane table depends on: the raw view and its antipode chart.</summary>
    public static (bool On, bool Antipode) PaneKey => (Model.On, Model.Antipode);

    /// <summary>Local's charts: one, or the chart and the antipode chart side by side.</summary>
    public static IReadOnlyList<RawPane> Panes(PaneSpec local) => Model.Panes(local);

    /// <summary>
    /// The raw view's keys, after the one router gave the key to the session (the chrome's keys and text focus come
    /// first), at a cap Local draws. The clip keys act only while the raw view is on and repeat while held.
    /// </summary>
    public static bool Follow(InputEventKey key)
    {
        var code = key.PhysicalKeycode;
        var toggle = code is Key.Home or Key.End;
        var clip = code is Key.Pageup or Key.Pagedown;
        if (!toggle && !(clip && Model.On) || ShellTurn.Cap is null)
        {
            return false;
        }
        if (key.Pressed && (key.CtrlPressed || key.AltPressed || key.ShiftPressed || key.MetaPressed))
        {
            return false;
        }
        if (!key.Pressed || key.Echo && toggle)
        {
            return true;
        }
        switch (code)
        {
            case Key.Home: Model.Toggle(); break;
            case Key.End: Model.ToggleAntipode(); break;
            case Key.Pageup: Model.Widen(-1); break;
            default: Model.Widen(1); break;
        }
        Changed();
        return true;
    }

    /// <summary>The palette's rows (JumbleHost's RawRow): answers why not, or null once carried out.</summary>
    public static string? Request(string row, string? argument)
    {
        if (ShellTurn.Cap is null)
        {
            return "Local draws no cap";
        }
        switch (row)
        {
            case RawView.Row: Model.Toggle(); break;
            case RawView.AntipodeRow: Model.ToggleAntipode(); break;
            case RawView.ClipRow when RawView.ParseClip(argument) is double h:
                if (!Model.On) Model.Toggle();
                Model.SetClip(h);
                break;
            default: return "it needs h from 0 (90°) to the named cap's cut, cut or tick";
        }
        Changed();
        return null;
    }

    private static void Changed()
    {
        ShellSlice.Restyle = true;
        Applied?.Invoke();
    }

    /// <summary>Local's slots while the raw view is on: no slice and no ghost, every sticker inside the clip drawn.</summary>
    public static unsafe void Style(ref HostLibrary.ViewStyle style)
    {
        if (!Model.On)
        {
            return;
        }
        style.SliceMode = HostLibrary.SliceOff;
        style.Ghost[3] = 0;
    }

    /// <summary>Local's line while the raw view is on; null while it is off.</summary>
    public static string? Line(int? cap) => Model.On ? Model.Line(cap) : null;

    /// <summary>The dump's raw row: on, the antipode chart, the clip, the tick, Local's charts, the caption and the line.</summary>
    public static void Dump(Utf8JsonWriter json)
    {
        json.WriteStartObject("raw");
        json.WriteBoolean("on", Model.On);
        json.WriteBoolean("antipode", Model.Antipode);
        json.WriteNumber("h", Model.H);
        json.WriteNumber("degrees", Math.Round(Model.Degrees, 3));
        json.WriteNumber("tick", Math.Round(RawView.MeetingTick, 6));
        json.WriteNumber("charts", Model.On ? Model.Antipode ? 2 : 1 : 0);
        json.WriteString("label", Model.On && Model.Antipode ? RawView.AntipodeLabel : "");
        json.WriteString("caption", Model.On ? RawView.Caption : "");
        json.WriteString("line", Model.Line(ShellTurn.Cap));
        json.WriteEndObject();
    }
}
