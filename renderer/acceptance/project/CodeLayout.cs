using System;
using System.Buffers.Binary;

public static class CodeLayout
{
    public const int SA2_CODE_BLOCK_PX = 8;
    public const int SA2_CODE_GRID_COLS = 8;
    public const int SA2_CODE_GRID_ROWS = 8;
    public const int SA2_CODE_SEQUENCE_BITS = 32;
    public const int SA2_CODE_SLOT_BITS = 4;
    public const int SA2_CODE_GENERATION_BITS = 12;
    public const int SA2_CODE_CHECK_BITS = 16;
    public const int SA2_MIN_TEXTURE_PX = 128;
    public const int SA2_RING_SLOTS = 3;

    public static ushort Crc16(ReadOnlySpan<byte> data)
    {
        ushort crc = 0xFFFF;
        foreach (byte value in data)
        {
            crc ^= (ushort)(value << 8);
            for (int bit = 0; bit < 8; bit++)
                crc = (ushort)((crc << 1) ^ ((crc & 0x8000) != 0 ? 0x1021 : 0));
        }
        return crc;
    }

    public static ulong Encode(uint sequence, uint slot, uint generation)
    {
        ulong low = sequence | ((ulong)(slot & 15) << 32) | ((ulong)(generation & 4095) << 36);
        Span<byte> bytes = stackalloc byte[8];
        BinaryPrimitives.WriteUInt64LittleEndian(bytes, low);
        return low | ((ulong)Crc16(bytes[..6]) << 48);
    }

    public static uint Fill(uint sequence, uint slot, uint generation) =>
        (32 + 64 * slot) | ((sequence % 251) << 8) | ((16 * (generation % 16)) << 16) | 0xFF000000;

    public static byte[] PushConstants(uint sequence, uint slot, uint generation, int width, int height)
    {
        byte[] data = new byte[32];
        BinaryPrimitives.WriteUInt64LittleEndian(data, Encode(sequence, slot, generation));
        BinaryPrimitives.WriteUInt32LittleEndian(data.AsSpan(8), Fill(sequence, slot, generation));
        BinaryPrimitives.WriteUInt32LittleEndian(data.AsSpan(12), (uint)width);
        BinaryPrimitives.WriteUInt32LittleEndian(data.AsSpan(16), (uint)height);
        return data;
    }

    public static ulong? Decode(byte[] data, int viewportWidth, int viewportHeight, int ringWidth, int ringHeight)
    {
        if (data.Length != (long)viewportWidth * viewportHeight * 4 || ringWidth < 128 || ringHeight < 128
            || viewportWidth < ringWidth || viewportHeight < ringHeight) return null;
        ulong a = Corner(data, viewportWidth, 0, 0);
        ulong b = Corner(data, viewportWidth, ringWidth - 64, ringHeight - 64);
        Span<byte> low = stackalloc byte[8];
        BinaryPrimitives.WriteUInt64LittleEndian(low, a);
        return a == b && Crc16(low[..6]) == (ushort)(a >> 48) ? a : null;
    }

    private static ulong Corner(byte[] data, int width, int x0, int y0)
    {
        ulong code = 0;
        for (int bit = 0; bit < 64; bit++)
        {
            int x = x0 + 8 * (bit % 8) + 4, y = y0 + 8 * (bit / 8) + 4;
            if (data[4 * (y * width + x)] >= 128) code |= 1UL << bit;
        }
        return code;
    }

    public static ulong MismatchedTexels(byte[] data, int width, int height, uint sequence, uint slot, uint generation)
    {
        if (data.Length != (long)width * height * 4) return (ulong)width * (ulong)height;
        ulong code = Encode(sequence, slot, generation), mismatches = 0;
        uint fill = Fill(sequence, slot, generation);
        for (int y = 0; y < height; y++)
        for (int x = 0; x < width; x++)
        {
            int bit = -1;
            if (x < 64 && y < 64) bit = y / 8 * 8 + x / 8;
            else if (x >= width - 64 && y >= height - 64)
                bit = (y - height + 64) / 8 * 8 + (x - width + 64) / 8;
            uint expected = bit < 0 ? fill : ((code >> bit) & 1) != 0 ? 0xFFFFFFFF : 0xFF000000;
            if (BinaryPrimitives.ReadUInt32LittleEndian(data.AsSpan(4 * (y * width + x), 4)) != expected)
                mismatches++;
        }
        return mismatches;
    }
}
