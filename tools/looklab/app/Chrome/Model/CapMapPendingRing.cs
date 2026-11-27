using System;

namespace Magic600.Chrome;

// The pending pattern's transparent ring, rendered once per cell size and border width. The painter supplies the ink.
internal static class CapMapPendingRing
{
    internal static float Radius(float width, float height) => Math.Min(width, height) * .3f;

    internal static byte[] Pixels(int width, int height, int line)
    {
        float radius = Radius(width - 2 * line, height - 2 * line);
        float edge = (line + 1) / 2f;
        var rgba = new byte[width * height * 4];
        for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
            {
                float dx = x + .5f - width / 2f, dy = y + .5f - height / 2f;
                float distance = MathF.Sqrt(dx * dx + dy * dy);
                float coverage = Math.Clamp(edge - Math.Abs(distance - radius), 0f, 1f);
                int at = (y * width + x) * 4;
                rgba[at] = rgba[at + 1] = rgba[at + 2] = 255;
                rgba[at + 3] = (byte)MathF.Round(coverage * 255);
            }
        return rgba;
    }
}
