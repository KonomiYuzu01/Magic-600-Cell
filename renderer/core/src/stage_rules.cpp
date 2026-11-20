#include "stage_rules.h"
#include <algorithm>
#include <cmath>
#include <cstring>
#include <limits>
#include <stdexcept>

namespace sa2::stages {
namespace {
constexpr Pass VertexPasses[] = {{"m600_eval_stage", "cs_6_2"}};
constexpr Pass FragmentPasses[] = {{"m600_fragment_ps", "ps_6_2"}};
constexpr Pass CorePasses[] = {{"m600_eval_builtin", "cs_6_2"}, {"m600_bound", "cs_6_2"}, {"m600_decide", "cs_6_2"},
                               {"m600_pass_vs", "vs_6_2"}, {"psMain", "ps_6_2"}};
constexpr char GeometryInclude[] = "#include \"geometry.hlsl\"";

// The length of the well-formed UTF-8 sequence at text[0], or 0 (overlong forms, surrogates and code points past
// U+10FFFF are not well formed).
size_t sequence(const unsigned char* text, size_t size) {
    const unsigned lead = text[0];
    if (lead < 0x80) return 1;
    size_t length = 0;
    uint32_t point = 0, least = 0;
    if ((lead & 0xE0) == 0xC0) length = 2, point = lead & 0x1F, least = 0x80;
    else if ((lead & 0xF0) == 0xE0) length = 3, point = lead & 0x0F, least = 0x800;
    else if ((lead & 0xF8) == 0xF0) length = 4, point = lead & 0x07, least = 0x10000;
    else return 0;
    if (size < length) return 0;
    for (size_t i = 1; i < length; ++i) {
        if ((text[i] & 0xC0) != 0x80) return 0;
        point = (point << 6) | (text[i] & 0x3F);
    }
    if (point < least || point > 0x10FFFF || (point >= 0xD800 && point <= 0xDFFF)) return 0;
    return length;
}

bool identifier_char(char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_';
}

void append(std::string& out, const char* name, std::string_view text) {
    out += "#line 1 \"";
    out += name;
    out += "\"\n";
    out += text;
    if (!text.empty() && text.back() != '\n') out += '\n';
}

// Rust's float-to-integer cast: saturating, NaN to 0.
int64_t saturate(float value) {
    if (std::isnan(value)) return 0;
    if (value >= 9223372036854775807.0f) return std::numeric_limits<int64_t>::max();
    if (value <= -9223372036854775808.0f) return std::numeric_limits<int64_t>::min();
    return int64_t(value);
}

uint64_t checked_add(uint64_t a, uint64_t b) {
    if (a > std::numeric_limits<uint64_t>::max() - b) throw std::overflow_error("raster bound exceeds 64 bits");
    return a + b;
}

uint64_t checked_multiply(uint64_t a, uint64_t b) {
    if (a != 0 && b > std::numeric_limits<uint64_t>::max() / a) throw std::overflow_error("raster bound exceeds 64 bits");
    return a * b;
}
}

std::span<const Pass> passes(uint32_t kind) {
    if (kind == SA2_STAGE_VERTEX) return VertexPasses;
    if (kind == SA2_STAGE_FRAGMENT) return FragmentPasses;
    if (kind == CoreKind) return CorePasses;
    throw std::invalid_argument("unknown stage kind");
}

std::vector<std::string> arguments(const Pass& pass) {
    return {"-T", pass.profile, "-E", pass.entry, "-HV", "2021", "-Gis", "-denorm", "preserve", "-O3", "-Ges", "-WX"};
}

const char* kind_name(uint32_t kind) {
    if (kind == SA2_STAGE_VERTEX) return "vertex";
    if (kind == SA2_STAGE_FRAGMENT) return "fragment";
    if (kind == CoreKind) return "core";
    throw std::invalid_argument("unknown stage kind");
}

std::string text_refusal(const char* text, uint32_t size) {
    if (!text) return "the stage text is null";
    if (size == 0) return "the stage text is empty";
    if (size > SA2_STAGE_HLSL_MAX_BYTES) return "the stage text is longer than SA2_STAGE_HLSL_MAX_BYTES";
    const auto* bytes = reinterpret_cast<const unsigned char*>(text);
    for (size_t at = 0; at < size;) {
        if (bytes[at] == 0) return "the stage text contains a NUL byte";
        const size_t length = sequence(bytes + at, size - at);
        if (length == 0) return "the stage text is not UTF-8";
        at += length;
    }
    return {};
}

bool names_identifier(std::string_view text, std::string_view identifier) {
    for (size_t at = text.find(identifier); at != std::string_view::npos; at = text.find(identifier, at + 1)) {
        const bool before = at > 0 && identifier_char(text[at - 1]);
        const size_t end = at + identifier.size();
        const bool after = end < text.size() && identifier_char(text[end]);
        if (!before && !after) return true;
    }
    return false;
}

std::string compose(uint32_t kind, std::string_view stage, const CoreSources& core) {
    std::string out;
    if (kind == SA2_STAGE_VERTEX) out += "#define M600_STAGE_VERTEX 1\n";
    else if (kind == SA2_STAGE_FRAGMENT) out += "#define M600_STAGE_FRAGMENT 1\n";
    else if (kind != CoreKind) throw std::invalid_argument("unknown stage kind");
    if (kind != CoreKind) {
        append(out, "stage.hlsl", stage);
        constexpr std::pair<const char*, const char*> guards[] = {
            {"m600_vertex_arrays", "M600_VERTEX_ARRAYS_DECLARED"},
            {"m600_fragment_arrays", "M600_FRAGMENT_ARRAYS_DECLARED"}};
        for (const auto& [name, guard] : guards) {
            if (!names_identifier(stage, name)) continue;
            out += "#ifndef ";
            out += guard;
            out += "\n#define ";
            out += guard;
            out += " 1\n#endif\n";
        }
    }
    append(out, "geometry.hlsl", core.geometry);
    std::string draw(core.draw);
    const size_t include = draw.find(GeometryInclude);
    if (include == std::string::npos || (include > 0 && draw[include - 1] != '\n'))
        throw std::runtime_error("draw.hlsl has no geometry include line");
    draw.erase(include, sizeof(GeometryInclude) - 1);
    append(out, "draw.hlsl", draw);
    append(out, "stages.hlsl", core.stages);
    return out;
}

std::string utf8_clean(std::string_view text) {
    std::string out;
    out.reserve(text.size());
    const auto* bytes = reinterpret_cast<const unsigned char*>(text.data());
    for (size_t at = 0; at < text.size();) {
        const size_t length = sequence(bytes + at, text.size() - at);
        if (length == 0) {
            out += '?';
            ++at;
        } else {
            out.append(text.data() + at, length);
            at += length;
        }
    }
    return out;
}

std::string fit_record(Json record, uint32_t capacity) {
    record["error_truncated"] = false;
    std::string text = record.dump();
    if (text.size() <= capacity) return text;
    const std::string error = record.at("error").string();
    auto boundary = [&](size_t cut) {
        while (cut > 0 && cut < error.size() && (static_cast<unsigned char>(error[cut]) & 0xC0) == 0x80) --cut;
        return cut;
    };
    record["error_truncated"] = true;
    auto size_at = [&](size_t cut) {
        record["error"] = error.substr(0, boundary(cut));
        return record.dump().size();
    };
    size_t low = 0, high = error.size();
    if (size_at(0) > capacity) throw std::runtime_error("the compile record does not fit its buffer");
    while (low < high) {
        const size_t mid = low + (high - low + 1) / 2;
        if (size_at(mid) <= capacity) low = mid;
        else high = mid - 1;
    }
    record["error"] = error.substr(0, boundary(low));
    return record.dump();
}

uint64_t extent(float low, float high, uint32_t size) {
    const int64_t limit = size;
    int64_t l = std::clamp<int64_t>(saturate(std::floor(low)), 0, limit);
    int64_t h = std::clamp<int64_t>(saturate(std::ceil(high)), 0, limit);
    if (l >= h) return 0;
    l &= ~int64_t(1);
    h = (h + 1) & ~int64_t(1);
    return uint64_t(h - l);
}

uint64_t count(uint32_t width, uint32_t height, const Triangle& triangle) {
    if (triangle.shape == Shape::CrossesNear) return uint64_t(width) * height;
    if (triangle.shape != Shape::Placed) return 0;
    for (const auto& corner : triangle.corners)
        for (float c : corner)
            if (!std::isfinite(c)) return 0;
    auto axis = [&](size_t i, uint32_t size) {
        float low = std::numeric_limits<float>::infinity(), high = -std::numeric_limits<float>::infinity();
        for (const auto& corner : triangle.corners) {
            low = std::min(low, corner[i]);
            high = std::max(high, corner[i]);
        }
        return extent(low, high, size);
    };
    return axis(0, width) * axis(1, height);
}

uint64_t bound(uint32_t width, uint32_t height, std::span<const RasterPass> passes) {
    uint64_t total = 0;
    for (const auto& pass : passes) {
        uint64_t pixels = 0;
        for (const auto& triangle : pass.triangles) pixels = checked_add(pixels, count(width, height, triangle));
        total = checked_add(total, checked_multiply(pixels, pass.samples));
    }
    return total;
}

uint64_t allowance(uint32_t width, uint32_t height, uint64_t overdraw) {
    return checked_multiply(uint64_t(width) * height, overdraw);
}

std::array<float, 2> pixel(const Clip& clip, uint32_t width, uint32_t height) {
    const float nx = clip[0] / clip[3];
    const float ny = clip[1] / clip[3];
    const float px = (nx * 0.5f + 0.5f) * float(width);
    const float py = (0.5f - ny * 0.5f) * float(height);
    return {px, py};
}

bool defined(const Clip& clip) {
    return std::isfinite(clip[0]) && std::isfinite(clip[1]) && std::isfinite(clip[2]) && std::isfinite(clip[3]);
}

Triangle classify(const std::array<Clip, 3>& clip, uint32_t width, uint32_t height) {
    Triangle triangle;
    if (!defined(clip[0]) || !defined(clip[1]) || !defined(clip[2])) return triangle;
    auto all = [&](auto beyond) { return beyond(clip[0]) && beyond(clip[1]) && beyond(clip[2]); };
    if (all([](const Clip& c) { return c[0] > c[3]; }) || all([](const Clip& c) { return c[0] < -c[3]; })
        || all([](const Clip& c) { return c[1] > c[3]; }) || all([](const Clip& c) { return c[1] < -c[3]; })
        || all([](const Clip& c) { return c[2] < 0; }) || all([](const Clip& c) { return c[2] > c[3]; })) {
        triangle.shape = Shape::Outside;
        return triangle;
    }
    for (const auto& c : clip) {
        if (c[2] < 0 || !(c[3] > 0)) {
            triangle.shape = Shape::CrossesNear;
            return triangle;
        }
    }
    triangle.shape = Shape::Placed;
    for (size_t i = 0; i < 3; ++i) triangle.corners[i] = pixel(clip[i], width, height);
    return triangle;
}

Clip record_clip(const uint32_t* words) {
    Clip clip;
    std::memcpy(clip.data(), words, sizeof(clip));
    return clip;
}

uint32_t coverage_code(const uint32_t* words) {
    return defined(record_clip(words)) ? 0u : (words[3] & 0x003FFFFFu);
}

uint64_t eval_bound(std::span<const uint32_t> words, uint32_t width, uint32_t height) {
    if (words.size() % 12 != 0) throw std::invalid_argument("an eval buffer holds whole triangles");
    uint64_t total = 0;
    for (size_t at = 0; at < words.size(); at += 12) {
        const std::array<Clip, 3> clip{record_clip(&words[at]), record_clip(&words[at + 4]), record_clip(&words[at + 8])};
        total = checked_add(total, count(width, height, classify(clip, width, height)));
    }
    return total;
}
}
