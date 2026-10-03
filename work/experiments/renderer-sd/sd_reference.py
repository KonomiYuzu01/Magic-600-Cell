"""Independent byte-level oracle for the immutable SA2 ABI-1 code image."""
import sys
sys.dont_write_bytecode = True

BLOCK_PX = 8
GRID_COLS = 8
GRID_ROWS = 8
SEQUENCE_BITS = 32
SLOT_BITS = 4
GENERATION_BITS = 12
CHECK_BITS = 16
MIN_TEXTURE_PX = 128
RING_SLOTS = 3


def crc16(data):
    crc = 0xFFFF
    for byte in data:
        crc ^= byte << 8
        for _ in range(8):
            crc = ((crc << 1) ^ (0x1021 if crc & 0x8000 else 0)) & 0xFFFF
    return crc


def code(sequence, slot, generation):
    low = (sequence & 0xFFFFFFFF) | ((slot & 15) << 32) | ((generation & 4095) << 36)
    return low | (crc16(low.to_bytes(6, "little")) << 48)


def expected_image(width, height, sequence, slot, generation):
    if min(width, height) < MIN_TEXTURE_PX:
        raise ValueError("ring too small")
    fill = bytes(((32 + 64 * slot) & 255, sequence % 251, 16 * (generation % 16), 255))
    image = bytearray(fill * (width * height))
    bits = code(sequence, slot, generation)
    for x0, y0 in ((0, 0), (width - 64, height - 64)):
        for bit in range(64):
            value = 255 if bits & (1 << bit) else 0
            row = bytes((value, value, value, 255)) * BLOCK_PX
            for y in range(BLOCK_PX):
                start = ((y0 + (bit // GRID_COLS) * BLOCK_PX + y) * width
                         + x0 + (bit % GRID_COLS) * BLOCK_PX) * 4
                image[start:start + len(row)] = row
    return bytes(image)


def decode(data, width, height, ring_width=None, ring_height=None, format="RGBA8"):
    """Decode corners of the ring at (0, 0), even in a larger composite."""
    rw = width if ring_width is None else ring_width
    rh = height if ring_height is None else ring_height
    if format not in ("RGBA8", "BGRA8") or min(rw, rh) < MIN_TEXTURE_PX:
        raise ValueError("invalid format or ring")
    if width < rw or height < rh or len(data) != width * height * 4:
        raise ValueError("invalid composite extent")
    channel = 0 if format == "RGBA8" else 2
    corners = []
    for x0, y0 in ((0, 0), (rw - 64, rh - 64)):
        value = 0
        for bit in range(64):
            x = x0 + (bit % GRID_COLS) * BLOCK_PX + BLOCK_PX // 2
            y = y0 + (bit // GRID_COLS) * BLOCK_PX + BLOCK_PX // 2
            if data[(y * width + x) * 4 + channel] >= 128:
                value |= 1 << bit
        corners.append(value)
    value = corners[0]
    fields = (value & 0xFFFFFFFF, (value >> 32) & 15, (value >> 36) & 4095)
    if value != corners[1] or value != code(*fields):
        raise ValueError("invalid code")
    return fields
