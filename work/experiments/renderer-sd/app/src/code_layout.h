#pragma once
#include "sa2_interop.h"
#include <array>
#include <cstddef>
#include <cstdint>
#include <vector>

namespace sd {
constexpr int BLOCK_PX = SA2_CODE_BLOCK_PX;
constexpr int GRID_COLS = SA2_CODE_GRID_COLS;
constexpr int GRID_ROWS = SA2_CODE_GRID_ROWS;
constexpr int SEQUENCE_BITS = SA2_CODE_SEQUENCE_BITS;
constexpr int SLOT_BITS = SA2_CODE_SLOT_BITS;
constexpr int GENERATION_BITS = SA2_CODE_GENERATION_BITS;
constexpr int CHECK_BITS = SA2_CODE_CHECK_BITS;
constexpr int MIN_TEXTURE_PX = SA2_MIN_TEXTURE_PX;
constexpr int RING_SLOTS = SA2_RING_SLOTS;
using Pixel = std::array<std::uint8_t, 4>;
struct Fields {
    std::uint32_t sequence, slot, generation;
    bool operator==(const Fields&) const = default;
};
std::uint16_t crc16(const std::uint8_t* bytes, std::size_t size);
std::uint64_t code(Fields fields);
std::vector<Pixel> expectedImage(unsigned width, unsigned height, Fields fields);
// width/height describe the composite, ringWidth/ringHeight its ring at (0,0).
bool decode(const void* bytes, std::size_t byteSize, unsigned width, unsigned height,
            unsigned ringWidth, unsigned ringHeight, bool bgra, Fields& fields);
}
