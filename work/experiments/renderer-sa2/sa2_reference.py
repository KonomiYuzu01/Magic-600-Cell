"""CPU reference for ABI-v1 code images, including composited viewport crops."""

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
    for value in data:
        crc ^= value << 8
        for _ in range(8):
            crc = ((crc << 1) ^ (0x1021 if crc & 0x8000 else 0)) & 0xFFFF
    return crc


def encode(sequence, slot, generation):
    low = (sequence & 0xFFFFFFFF) | ((slot & 15) << 32) | ((generation & 4095) << 36)
    return low | (crc16(low.to_bytes(6, "little")) << 48)


def valid_code(code):
    return crc16((code & 0xFFFFFFFFFFFF).to_bytes(6, "little")) == code >> 48


def fill_rgba(sequence, slot, generation):
    return bytes((32 + 64 * slot, sequence % 251, 16 * (generation % 16), 255))


def expected_image(width, height, sequence, slot, generation):
    if width < MIN_TEXTURE_PX or height < MIN_TEXTURE_PX or slot not in range(RING_SLOTS):
        raise ValueError("invalid image extent or ring slot")
    code = encode(sequence, slot, generation)
    image = bytearray(fill_rgba(sequence & 0xFFFFFFFF, slot, generation) * (width * height))
    for x0, y0 in ((0, 0), (width - 64, height - 64)):
        for bit in range(64):
            x = x0 + BLOCK_PX * (bit % GRID_COLS)
            y = y0 + BLOCK_PX * (bit // GRID_COLS)
            pixel = bytes((255, 255, 255, 255) if code & (1 << bit) else (0, 0, 0, 255))
            row = pixel * BLOCK_PX
            for dy in range(BLOCK_PX):
                start = ((y + dy) * width + x) * 4
                image[start:start + len(row)] = row
    return bytes(image)


def decode_viewport(data, viewport_width, viewport_height, ring_width, ring_height, x=0, y=0):
    if (len(data) != viewport_width * viewport_height * 4
            or ring_width < MIN_TEXTURE_PX or ring_height < MIN_TEXTURE_PX
            or x < 0 or y < 0 or x + ring_width > viewport_width or y + ring_height > viewport_height):
        return None

    def corner(x0, y0):
        code = 0
        for bit in range(64):
            cx = x0 + BLOCK_PX * (bit % GRID_COLS) + BLOCK_PX // 2
            cy = y0 + BLOCK_PX * (bit // GRID_COLS) + BLOCK_PX // 2
            if data[(cy * viewport_width + cx) * 4] >= 128:
                code |= 1 << bit
        return code

    first = corner(x, y)
    second = corner(x + ring_width - 64, y + ring_height - 64)
    return first if first == second and valid_code(first) else None


def decode_image(data, width, height):
    return decode_viewport(data, width, height, width, height)
