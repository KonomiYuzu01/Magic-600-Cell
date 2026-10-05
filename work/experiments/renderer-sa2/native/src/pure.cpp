#include "pure.h"
#include <algorithm>
#include <stdexcept>

namespace sa2 {
std::uint16_t crc16(const std::uint8_t* bytes, std::size_t size) {
    std::uint16_t crc = 0xffff;
    for (std::size_t i = 0; i < size; ++i) {
        crc ^= static_cast<std::uint16_t>(bytes[i]) << 8;
        for (int bit = 0; bit < 8; ++bit)
            crc = static_cast<std::uint16_t>((crc << 1) ^ ((crc & 0x8000) ? 0x1021 : 0));
    }
    return crc;
}
std::uint64_t code(Fields fields) {
    const auto low = std::uint64_t(fields.sequence) | (std::uint64_t(fields.slot & 15) << 32)
        | (std::uint64_t(fields.generation & 4095) << 36);
    std::array<std::uint8_t, 6> bytes{};
    for (unsigned i = 0; i < bytes.size(); ++i) bytes[i] = static_cast<std::uint8_t>(low >> (8 * i));
    return low | (std::uint64_t(crc16(bytes.data(), bytes.size())) << 48);
}
Pixel fill(Fields fields) {
    return {static_cast<std::uint8_t>(32 + 64 * fields.slot),
        static_cast<std::uint8_t>(fields.sequence % 251),
        static_cast<std::uint8_t>(16 * (fields.generation % 16)), 255};
}
std::vector<Pixel> image(std::uint32_t width, std::uint32_t height, Fields fields) {
    if (width < SA2_MIN_TEXTURE_PX || height < SA2_MIN_TEXTURE_PX)
        throw std::invalid_argument("image dimensions must be at least 128");
    std::vector<Pixel> result(std::size_t(width) * height, fill(fields));
    const auto bits = code(fields);
    for (unsigned corner = 0; corner < 2; ++corner) {
        const auto x0 = corner ? width - 64 : 0;
        const auto y0 = corner ? height - 64 : 0;
        for (unsigned bit = 0; bit < 64; ++bit) {
            const std::uint8_t k = ((bits >> bit) & 1) ? 255 : 0;
            for (unsigned y = 0; y < 8; ++y)
                for (unsigned x = 0; x < 8; ++x)
                    result[std::size_t(y0 + (bit / 8) * 8 + y) * width + x0 + (bit % 8) * 8 + x] = {k, k, k, 255};
        }
    }
    return result;
}
bool decode(const void* bytes, std::size_t row_pitch, std::uint32_t width, std::uint32_t height, Fields& out) {
    if (!bytes || width < 128 || height < 128 || row_pitch < std::size_t(width) * 4) return false;
    const auto* pixels = static_cast<const std::uint8_t*>(bytes);
    std::array<std::uint64_t, 2> corners{};
    for (unsigned corner = 0; corner < 2; ++corner) {
        const auto x0 = corner ? width - 64 : 0;
        const auto y0 = corner ? height - 64 : 0;
        for (unsigned bit = 0; bit < 64; ++bit)
            if (pixels[std::size_t(y0 + (bit / 8) * 8 + 4) * row_pitch + (x0 + (bit % 8) * 8 + 4) * 4] >= 128)
                corners[corner] |= std::uint64_t(1) << bit;
    }
    if (corners[0] != corners[1]) return false;
    Fields fields{static_cast<std::uint32_t>(corners[0]), static_cast<std::uint32_t>((corners[0] >> 32) & 15),
        static_cast<std::uint32_t>((corners[0] >> 36) & 4095)};
    if (code(fields) != corners[0]) return false;
    out = fields;
    return true;
}
std::string mask_hex(std::string_view text) {
    const auto is_hex = [](char c) { return (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F'); };
    std::string result;
    for (std::size_t i = 0; i < text.size();) {
        if (i + 2 < text.size() && text[i] == '0' && (text[i + 1] == 'x' || text[i + 1] == 'X') && is_hex(text[i + 2])) {
            result += "0x?";
            i += 2;
            while (i < text.size() && is_hex(text[i])) ++i;
        } else result += text[i++];
    }
    return result;
}
bool state(std::int32_t value, State& out) {
    switch (value) {
    case SA2_STATE_COMMON: out = {D3D12_RESOURCE_STATE_COMMON, D3D12_BARRIER_LAYOUT_COMMON}; break;
    case SA2_STATE_RENDER_TARGET: out = {D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_BARRIER_LAYOUT_RENDER_TARGET}; break;
    case SA2_STATE_PIXEL_SHADER_RESOURCE: out = {D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_BARRIER_LAYOUT_SHADER_RESOURCE}; break;
    case SA2_STATE_ALL_SHADER_RESOURCE:
        out = {D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE | D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE, D3D12_BARRIER_LAYOUT_SHADER_RESOURCE}; break;
    case SA2_STATE_COPY_SOURCE: out = {D3D12_RESOURCE_STATE_COPY_SOURCE, D3D12_BARRIER_LAYOUT_COPY_SOURCE}; break;
    default: return false;
    }
    return true;
}
std::string json_string(std::string_view text) {
    constexpr char hex[] = "0123456789abcdef";
    std::string result = "\"";
    for (unsigned char c : text) {
        if (c == '"' || c == '\\') { result += '\\'; result += static_cast<char>(c); }
        else if (c < 32) { result += "\\u00"; result += hex[c >> 4]; result += hex[c & 15]; }
        else result += static_cast<char>(c);
    }
    return result + '"';
}
std::string json_report(std::string_view mode, const std::vector<Check>& checks) {
    std::string result = "{\"format\":\"magic600-sa2-native-selftest-v1\",\"mode\":" + json_string(mode) + ",\"checks\":[";
    bool first = true;
    for (const auto& check : checks) {
        if (!first) result += ',';
        first = false;
        result += "{\"name\":" + json_string(check.name) + ",\"status\":" + json_string(check.status) + ",\"reason\":" + json_string(check.reason) + '}';
    }
    return result + "]}\n";
}
}
