using Magic600.Chrome;

internal static partial class Program
{
    private static void D02PendingRingKeepsItsGeometry()
    {
        const int width = 16, height = 12, line = 1;
        byte[] pixels = CapMapPendingRing.Pixels(width, height, line);
        byte Alpha(int x, int y) => pixels[(y * width + x) * 4 + 3];

        Check(CapMapPendingRing.Radius(width - 2 * line, height - 2 * line) == 3, "the ring's original radius");
        Check(Alpha(7, 5) == 0 && Alpha(0, 0) == 0, "transparent centre and surrounding cell");
        Check(Alpha(4, 5) is > 0 and < 255 && Alpha(5, 4) > Alpha(4, 5), "antialiased edge and stronger stroke");
        Check(pixels[(5 * width + 4) * 4] == 255, "white mask takes the active theme's ink");
        for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
                Check(Alpha(x, y) == Alpha(width - 1 - x, height - 1 - y), "the ring is centred in each cell");
    }
}
