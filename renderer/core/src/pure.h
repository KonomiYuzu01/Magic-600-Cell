#pragma once

#include "sa2_interop.h"
#include <d3d12.h>
#include <array>
#include <cstddef>
#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

static_assert(sizeof(sa2_device_info) == 48);
static_assert(sizeof(sa2_config) == 28);
static_assert(sizeof(struct sa2_debug_counts) == 128);

namespace sa2 {
using Pixel = std::array<std::uint8_t, 4>;
struct Fields {
    std::uint32_t sequence, slot, generation;
    bool operator==(const Fields&) const = default;
};
std::uint16_t crc16(const std::uint8_t* bytes, std::size_t size);
std::uint64_t code(Fields fields);
Pixel fill(Fields fields);
std::vector<Pixel> image(std::uint32_t width, std::uint32_t height, Fields fields);
bool decode(const void* bytes, std::size_t row_pitch, std::uint32_t width, std::uint32_t height, Fields& out);
std::string mask_hex(std::string_view text);

struct State {
    D3D12_RESOURCE_STATES legacy;
    D3D12_BARRIER_LAYOUT layout;
};
bool state(std::int32_t value, State& out);

struct Check {
    std::string name, status, reason;
};
std::string json_string(std::string_view text);
std::string json_report(std::string_view mode, const std::vector<Check>& checks);
}
