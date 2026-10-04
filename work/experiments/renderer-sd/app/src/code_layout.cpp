#include "code_layout.h"
#include <stdexcept>

namespace sd {
std::uint16_t crc16(const std::uint8_t* bytes, std::size_t size) {
    std::uint16_t crc = 0xFFFF;
    for (std::size_t i = 0; i < size; ++i) {
        crc ^= static_cast<std::uint16_t>(bytes[i]) << 8;
        for (int bit = 0; bit < 8; ++bit)
            crc = static_cast<std::uint16_t>((crc << 1) ^ ((crc & 0x8000) ? 0x1021 : 0));
    }
    return crc;
}
std::uint64_t code(Fields fields) {
    const auto low = std::uint64_t(fields.sequence) | (std::uint64_t(fields.slot & 15) << SEQUENCE_BITS)
        | (std::uint64_t(fields.generation & 4095) << (SEQUENCE_BITS + SLOT_BITS));
    std::array<std::uint8_t, 6> bytes{};
    for (unsigned i = 0; i < bytes.size(); ++i) bytes[i] = static_cast<std::uint8_t>(low >> (8 * i));
    return low | (std::uint64_t(crc16(bytes.data(), bytes.size())) << 48);
}
std::vector<Pixel> expectedImage(unsigned width, unsigned height, Fields fields) {
    if (width < MIN_TEXTURE_PX || height < MIN_TEXTURE_PX) throw std::invalid_argument("ring too small");
    const Pixel fill{static_cast<std::uint8_t>(32 + 64 * fields.slot),
        static_cast<std::uint8_t>(fields.sequence % 251), static_cast<std::uint8_t>(16 * (fields.generation % 16)), 255};
    std::vector<Pixel> pixels(std::size_t(width) * height, fill);
    const auto bits = code(fields);
    for (unsigned corner = 0; corner < 2; ++corner) {
        const auto x0 = corner ? width - BLOCK_PX * GRID_COLS : 0;
        const auto y0 = corner ? height - BLOCK_PX * GRID_ROWS : 0;
        for (unsigned bit = 0; bit < GRID_COLS * GRID_ROWS; ++bit) {
            const std::uint8_t value = ((bits >> bit) & 1) ? 255 : 0;
            for (unsigned y = 0; y < BLOCK_PX; ++y)
                for (unsigned x = 0; x < BLOCK_PX; ++x)
                    pixels[std::size_t(y0 + (bit / GRID_COLS) * BLOCK_PX + y) * width
                        + x0 + (bit % GRID_COLS) * BLOCK_PX + x] = {value, value, value, 255};
        }
    }
    return pixels;
}
bool decode(const void* bytes, std::size_t byteSize, unsigned width, unsigned height,
            unsigned ringWidth, unsigned ringHeight, bool bgra, Fields& fields) {
    if (!bytes || ringWidth < MIN_TEXTURE_PX || ringHeight < MIN_TEXTURE_PX || width < ringWidth
        || height < ringHeight || byteSize != std::size_t(width) * height * 4) return false;
    const auto* pixels = static_cast<const std::uint8_t*>(bytes);
    std::array<std::uint64_t, 2> corners{};
    for (unsigned corner = 0; corner < 2; ++corner) {
        const auto x0 = corner ? ringWidth - BLOCK_PX * GRID_COLS : 0;
        const auto y0 = corner ? ringHeight - BLOCK_PX * GRID_ROWS : 0;
        for (unsigned bit = 0; bit < GRID_COLS * GRID_ROWS; ++bit) {
            const auto x = x0 + (bit % GRID_COLS) * BLOCK_PX + BLOCK_PX / 2;
            const auto y = y0 + (bit / GRID_COLS) * BLOCK_PX + BLOCK_PX / 2;
            if (pixels[(std::size_t(y) * width + x) * 4 + (bgra ? 2 : 0)] >= 128)
                corners[corner] |= std::uint64_t(1) << bit;
        }
    }
    Fields decoded{static_cast<std::uint32_t>(corners[0]), static_cast<std::uint32_t>((corners[0] >> 32) & 15),
        static_cast<std::uint32_t>((corners[0] >> 36) & 4095)};
    if (corners[0] != corners[1] || code(decoded) != corners[0]) return false;
    fields = decoded;
    return true;
}
}
