using System;
using Godot;

namespace Magic600.Shell.D02;

/// <summary>
/// The view rotation the owner asked for on 26 November 2026, as Look Lab turns its window: a left-button drag in the
/// Global opening rotates the 4D camera in the x–w and y–w planes, 0.004 rad per pixel, and the 3D spin of a layout
/// projection in the x–z and y–z planes (Look Lab's <c>GlobalLayout.Drag</c> through <c>LookViewport.Drag</c>, with the
/// default preset's inertia of 0). A press becomes a drag only after it moves three pixels, so a click stays a click.
/// Pure apart from Godot's input types; the shell sends <see cref="ColumnMajor"/> to <c>m6h_set_camera</c>.
/// </summary>
public sealed class ShellCamera
{
    /// <summary>Look Lab's drag rate.</summary>
    public const double RadiansPerPixel = .004;

    /// <summary>How far a press moves before it turns the camera.</summary>
    public const float DragThreshold = 3;

    /// <summary>The 4D rotation, row-major (<c>q[row * 4 + column]</c>), as Look Lab keeps <c>q</c>.</summary>
    public double[] Q { get; } = Identity();

    /// <summary>The 3D spin of a layout projection, row-major, as Look Lab keeps <c>spin</c>.</summary>
    public double[] Spin { get; } = Identity();

    /// <summary>True when the camera turned since the shell last sent it.</summary>
    public bool Dirty { get; set; }

    /// <summary>How many drag steps turned the camera since the start.</summary>
    public int Turns { get; private set; }

    /// <summary>True while a press in the opening has become a drag.</summary>
    public bool Dragging { get; private set; }

    private bool _pressed;
    private Vector2 _from;

    public static double[] Identity() => new double[] { 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1 };

    /// <summary>
    /// Follows one input event. A left press inside the opening arms a drag; motion with the button held turns the
    /// camera once it has moved <see cref="DragThreshold"/> pixels; the release ends it. Returns true for an event the
    /// drag took, which the shell then marks handled.
    /// </summary>
    public bool Follow(InputEvent input, Func<Vector2, bool> inOpening)
    {
        switch (input)
        {
            case InputEventMouseButton { ButtonIndex: MouseButton.Left } button:
                if (button.Pressed)
                {
                    _pressed = inOpening(button.Position);
                    _from = button.Position;
                    Dragging = false;
                    return false;
                }
                var dragged = Dragging;
                _pressed = false;
                Dragging = false;
                return dragged;
            case InputEventMouseMotion motion when _pressed && (motion.ButtonMask & MouseButtonMask.Left) != 0:
                if (!Dragging && motion.Position.DistanceTo(_from) < DragThreshold)
                {
                    return false;
                }
                Dragging = true;
                Turn(motion.Relative.X * RadiansPerPixel, motion.Relative.Y * RadiansPerPixel);
                return true;
            default:
                return false;
        }
    }

    /// <summary>
    /// Turns the camera by the two drag angles in Look Lab's sense: the near side (large w, which the projection
    /// magnifies) follows the pointer. The core applies Q as it is (x' = Q x), where Look Lab's shader applies the
    /// transpose of its q, so the x–w turn takes the opposite angle to Look Lab's <c>GlobalLayout.Drag</c>: a rightward
    /// drag moves the near side right, as in Look Lab (owner, 26 November 2026: "转动时左右的方向反了"). A downward
    /// drag already moves the near side down (screen y runs down, the core's y up), so the y–w turn keeps its angle.
    /// The layout spin keeps Look Lab's own convention; the core does not draw a layout yet.
    /// </summary>
    public void Turn(double ax, double ay)
    {
        if (!double.IsFinite(ax) || !double.IsFinite(ay))
        {
            throw new ArgumentException("finite drag angles required");
        }
        Rotate(Q, 0, 3, -ax);
        Rotate(Q, 1, 3, ay);
        Rotate(Spin, 0, 2, ax);
        double c = Math.Cos(ay), s = Math.Sin(ay);
        for (var j = 0; j < 4; j++)
        {
            double y = Spin[j * 4 + 1], z = Spin[j * 4 + 2];
            Spin[j * 4 + 1] = c * y - s * z;
            Spin[j * 4 + 2] = s * y + c * z;
        }
        Dirty = true;
        Turns++;
    }

    /// <summary>Rotates rows <paramref name="a"/> and <paramref name="b"/> of a row-major 4×4 matrix (Look Lab's
    /// <c>Geometry.Rotate</c>): the rotation is applied on the left, in the world's frame.</summary>
    public static void Rotate(double[] q, int a, int b, double angle)
    {
        double c = Math.Cos(angle), s = Math.Sin(angle);
        for (var j = 0; j < 4; j++)
        {
            double x = q[a * 4 + j], y = q[b * 4 + j];
            q[a * 4 + j] = c * x - s * y;
            q[b * 4 + j] = s * x + c * y;
        }
    }

    /// <summary>The 4D rotation in the core's layout: a column-major float4x4 (sa2_interop.h's camera_matrix).</summary>
    public float[] ColumnMajor()
    {
        var m = new float[16];
        for (var row = 0; row < 4; row++)
        {
            for (var column = 0; column < 4; column++)
            {
                m[column * 4 + row] = (float)Q[row * 4 + column];
            }
        }
        return m;
    }
}
