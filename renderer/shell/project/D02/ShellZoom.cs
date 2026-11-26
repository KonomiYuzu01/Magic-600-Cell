using System;
using System.Collections.Generic;
using Godot;

namespace Magic600.Shell.D02;

/// <summary>
/// The wheel zoom the owner asked for on 26 November 2026 ("我现在没法zoom in"). Look Lab's own viewport has no wheel
/// zoom (it takes only a left drag; its zoom is the preset's), so the shell adds one in Look Lab's terms: the wheel
/// scales the zoom of the view under the pointer, which the core multiplies into the projected picture
/// (geometry.hlsl: clip.xy = p.xy · zoom), so a larger zoom draws a larger picture. Wheel up zooms in, one notch
/// multiplies the zoom by <see cref="Step"/>, and the zoom stays within <see cref="MinFactor"/> to
/// <see cref="MaxFactor"/> of the view's opening zoom. The wheel takes no keyboard focus and never moves the
/// foreground: it does not reach the focus hand-back. Pure apart from Godot's input types.
/// </summary>
public sealed class ShellZoom
{
    /// <summary>One wheel notch multiplies the zoom by this; wheel up multiplies, wheel down divides.</summary>
    public const double Step = 1.1;

    /// <summary>The smallest zoom, as a factor of the view's opening zoom.</summary>
    public const double MinFactor = .5;

    /// <summary>The largest zoom, as a factor of the view's opening zoom.</summary>
    public const double MaxFactor = 4;

    private readonly Dictionary<ulong, double> _factors = new();
    private readonly HashSet<ulong> _dirty = new();

    /// <summary>How many wheel notches changed a view's zoom since the start.</summary>
    public int Notches { get; private set; }

    /// <summary>A view's zoom as a factor of its opening zoom (1 until the wheel moves it).</summary>
    public double Factor(ulong view) => _factors.TryGetValue(view, out var factor) ? factor : 1;

    /// <summary>
    /// Follows one input event: a wheel press over a drawn view zooms that view. <paramref name="viewAt"/> answers the
    /// view under a window point, or null between and outside the views. Returns true for an event the zoom took,
    /// which the shell then marks handled.
    /// </summary>
    public bool Follow(InputEvent input, Func<Vector2, ulong?> viewAt)
    {
        if (input is not InputEventMouseButton { Pressed: true } button
            || button.ButtonIndex is not (MouseButton.WheelUp or MouseButton.WheelDown)
            || viewAt(button.Position) is not ulong view)
        {
            return false;
        }
        // A precise wheel or a touchpad reports a fraction of a notch in Factor; a plain wheel reports 0 or 1.
        double notches = button.Factor > 0 ? button.Factor : 1;
        Wheel(view, button.ButtonIndex == MouseButton.WheelUp ? notches : -notches);
        return true;
    }

    /// <summary>
    /// Zooms a view by a number of notches, positive in, negative out, within the limits. Answers true when the zoom
    /// changed; a notch at a limit changes nothing.
    /// </summary>
    public bool Wheel(ulong view, double notches)
    {
        if (!double.IsFinite(notches))
        {
            throw new ArgumentException("a finite number of notches required", nameof(notches));
        }
        double before = Factor(view);
        double after = Math.Clamp(before * Math.Pow(Step, notches), MinFactor, MaxFactor);
        if (after == before)
        {
            return false;
        }
        _factors[view] = after;
        _dirty.Add(view);
        Notches++;
        return true;
    }

    /// <summary>The views whose zoom changed since the shell last sent it; taking them clears the list.</summary>
    public IReadOnlyList<ulong> TakeDirty()
    {
        var views = new List<ulong>(_dirty);
        _dirty.Clear();
        return views;
    }
}
