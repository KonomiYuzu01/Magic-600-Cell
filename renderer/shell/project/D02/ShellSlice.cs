using System;
using System.Collections.Generic;
using System.Text.Json;
using Godot;
using Magic600.Chrome;

namespace Magic600.Shell.D02;

/// <summary>
/// ADR-D02.17 and CR-14 (build step B20): Local's cut slice in the shell. Local draws the outside layer filled and the
/// inside layer (the cap's inside set, read from <see cref="ShellTurn"/>'s twist row) as outlines at a smaller shrink,
/// clipped to a radius shell about the cap's pole; Global keeps its chain or chart. A held key splits Local into two
/// linked copies of the one view (<see cref="ViewPanes.Split"/>): the left draws the outside layer, the right the inside
/// layer, with the same camera, scale and shell. A second held key previews the twist as a ghost of the inside layer
/// (pattern.preview) and names it on release only; a blocked cap has no row, so no ghost, only B16's straddlers.
/// The keys are placeholders (row requests): <see cref="CutSlice.SplitKey"/> and <see cref="CutSlice.PreviewKey"/>.
/// </summary>
public static class ShellSlice
{
    private static (int Cap, ulong Revision, bool Row)? _inside;
    // LF1: the cap and pending revision whose inside layer the host holds.
    private static (int Cap, ulong Revision, bool Row)? _early;
    private static (int Cap, ulong Revision)? _ghost;
    private static int _insideCount;
    private static int _code = HostLibrary.Ok;

    /// <summary>The pure model: the split, the shell, the preview and the named twist.</summary>
    public static CutSlice Model { get; } = new();

    /// <summary>Whether the split key is held (Local's pane table holds two copies).</summary>
    public static bool Split => Model.Split;

    /// <summary>Set when the slots must be sent again (a held key, the inside layer or the ghost changed).</summary>
    public static bool Restyle { get; set; }

    /// <summary>Local's drawn panes: one slice pane, or the held split's two linked copies.</summary>
    public static IReadOnlyList<SlicePane> Panes(PaneSpec local) => Model.Panes(local);

    /// <summary>
    /// The palette's slice rows (JumbleHost's SliceRow, B32 findings): jumble.slice.split latches the held split on or
    /// off (a press and release of the held key also ends it), jumble.slice.shell sets the shell's outer radius.
    /// Answers why not, or null once carried out.
    /// </summary>
    public static string? Request(string row, string? argument)
    {
        if (ShellTurn.Cap is null)
        {
            return "Local draws no cap";
        }
        if (row == JumbleHost.ShellRow)
        {
            if (CutSlice.ParseShell(argument) is not double max)
            {
                return "it needs the shell's outer radius in radians, 0.05 to 3.14, or default";
            }
            Model.SetShell(max);
        }
        else
        {
            Model.ToggleSplit();
        }
        Restyle = true;
        return null;
    }

    /// <summary>
    /// The held keys, after the one router gave the key to the session (the chrome's keys and text focus come first).
    /// A press with a modifier is not the slice's; a release always is, so a held key never sticks.
    /// </summary>
    public static bool Follow(InputEventKey key)
    {
        var split = key.PhysicalKeycode == Key.Backslash;
        var preview = key.PhysicalKeycode == Key.Down;
        if (!split && !preview || ShellTurn.Cap is not int cap)
        {
            return false;
        }
        if (key.Pressed && (key.CtrlPressed || key.AltPressed || key.ShiftPressed || key.MetaPressed))
        {
            return false;
        }
        if (key.Echo)
        {
            return true;
        }
        if (split)
        {
            if (key.Pressed) Model.HoldSplit(); else Model.ReleaseSplit();
        }
        else if (key.Pressed)
        {
            Model.Press(cap, ShellTurn.Row is { } row ? row.Angle : null);
        }
        else
        {
            Model.Release();
        }
        Restyle = true;
        return true;
    }

    /// <summary>
    /// Sends the inside layer (the slice channel) once per cap and revision, and the preview's transform while the
    /// preview key is held at an admissible cap (again after each adoption, which drops it).
    /// </summary>
    public static int Apply(HostLibrary library, IntPtr host, ulong revision)
    {
        (int, ulong, bool)? inside = ShellTurn.Cap is int cap ? (cap, revision, ShellTurn.Row is not null) : null;
        if (inside != _inside)
        {
            var code = ShellTurn.SendInside(library, host, revision, out var count);
            if (code == HostLibrary.Unavailable)
            {
                // B18-04: the row was read at a head the host has not adopted yet; the layer sent for the adopted
                // revision stays, and the row's is sent once its revision is adopted.
            }
            else if (code != HostLibrary.Ok)
            {
                return _code = code;
            }
            else
            {
                (_insideCount, _inside) = (count, inside);
                Restyle = true;
            }
        }
        // LF1: after a committed press, the inside layer for the revision handed out too, held by the host for that
        // revision's adoption, so its first frame draws the new inside set, never the old one on the new poses.
        ShellTurn.NotePending(library, host, ShellStyle.PendingAfter(revision));
        (int, ulong, bool)? early = ShellTurn.Cap is int at && ShellTurn.PendingRevision != 0
            ? (at, ShellTurn.PendingRevision, ShellTurn.PendingRow is not null) : null;
        if (early is not null && early != _early && ShellTurn.SendPendingInside(library, host) == HostLibrary.Ok)
        {
            _early = early;
        }
        (int, ulong)? ghost = Model.Previewing is { } held && ShellTurn.Cap == held.Cap && ShellTurn.Row is not null
            ? (held.Cap, revision) : null;
        if (ghost != _ghost)
        {
            var code = ShellTurn.Preview(library, host, ghost is not null);
            if (code != HostLibrary.Ok)
            {
                return _code = code;
            }
            _ghost = ghost;
            Restyle = true;
        }
        return _code = HostLibrary.Ok;
    }

    /// <summary>Local's slots: 2 draws both layers (or the outside copy while split), 3 the inside copy.</summary>
    public static unsafe void Style(ref HostLibrary.ViewStyle style, uint slot, float[]? palette)
    {
        style.SliceMode = slot == 3 ? HostLibrary.SliceInside : Split ? HostLibrary.SliceOutside : HostLibrary.SliceBoth;
        style.SliceShrink = (float)CutSlice.Shrink;
        style.ShellMin = (float)Model.Shell.Min;
        style.ShellMax = (float)Model.Shell.Max;
        if (_ghost is null)
        {
            return;
        }
        // pattern.preview's cue, a ghost at reduced opacity, in the state.preview colour (palette entry 15).
        ReadOnlySpan<float> fallback = stackalloc float[] { 0.55f, 0.8f, 1.0f };
        for (var i = 0; i < 3; i++)
        {
            style.Ghost[i] = palette?.Length == 64 ? palette[60 + i] : fallback[i];
        }
        style.Ghost[3] = 0.5f;
    }

    /// <summary>The dump's slice row: the split, the ghost, the shell, the inside layer's size, the named twist, the
    /// refusal and Local's line.</summary>
    public static void Dump(Utf8JsonWriter json)
    {
        json.WriteStartObject("slice");
        json.WriteBoolean("split", Split);
        json.WriteBoolean("ghost", _ghost is not null);
        json.WriteNumber("shell_min", Model.Shell.Min);
        json.WriteNumber("shell_max", Model.Shell.Max);
        json.WriteNumber("shrink", CutSlice.Shrink);
        json.WriteNumber("inside", _insideCount);
        json.WriteNumber("code", _code);
        if (Model.Named is { } named)
        {
            json.WriteStartObject("named");
            json.WriteNumber("cap", named.Cap);
            json.WriteNumber("item", named.Item);
            json.WriteNumber("angle", named.Angle);
            json.WriteEndObject();
        }
        else
        {
            json.WriteNull("named");
        }
        if (Model.Refusal is { } refusal) json.WriteString("refusal", refusal); else json.WriteNull("refusal");
        json.WriteString("line", Model.Line(ShellTurn.Cap, ShellTurn.Cap is not null && ShellTurn.Row is null));
        json.WriteEndObject();
    }
}
