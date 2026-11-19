#pragma once
#include "sa2_interop.h"
#include "json.h"
#include <array>
#include <cstdint>
#include <span>
#include <string>
#include <string_view>
#include <vector>

// C9 U5's device-free rules: the frozen compiler profile, the pass source a stage compile composes, the compile
// record, and RPI.72's raster bound (rust/c600-display/src/raster.rs) with the core's clip classification. The
// self-test checks them without a device ("stage arguments", "raster bound").
namespace sa2::stages {
constexpr uint32_t CoreKind = 0; // the core's own passes, composed without stage text
constexpr char RecordFormat[] = "magic600-stage-compile/1";
constexpr char CompileFailed[] = "EV3:input.stage_compile_failed";

struct Pass {
    const char* entry;
    const char* profile;
};
// The passes one compile builds: a vertex stage's eval, a fragment stage's pixel stage, or the core's own passes
// (eval with the built-in vertex stage, bound, decide, the pass-through vertex stage and the built-in psMain).
std::span<const Pass> passes(uint32_t kind);
// The frozen profile: -T <profile> -E <entry> -HV 2021 -Gis -denorm preserve -O3 -Ges -WX, nothing else.
std::vector<std::string> arguments(const Pass& pass);
const char* kind_name(uint32_t kind); // "vertex", "fragment" or "core"

// The stage text's own rules before any compile: 1 to SA2_STAGE_HLSL_MAX_BYTES bytes of UTF-8 without NUL. An empty
// string accepts it; otherwise the reason.
std::string text_refusal(const char* text, uint32_t size);
// True when identifier appears in text as a whole identifier (not inside a longer one).
bool names_identifier(std::string_view text, std::string_view identifier);
struct CoreSources {
    std::string_view geometry, draw, stages;
};
// The source one compile hands DXC: the kind's define, the stage text, the array guards for every buffer the stage
// text names, then geometry.hlsl, draw.hlsl (its #include line blanked) and stages.hlsl, each after a #line marker.
std::string compose(uint32_t kind, std::string_view stage, const CoreSources& core);
// Replaces every byte that is not part of a well-formed UTF-8 sequence with '?'.
std::string utf8_clean(std::string_view text);
// The record's JSON text, at most capacity bytes: when it does not fit, "error" is cut at a character boundary and
// "error_truncated" is true.
std::string fit_record(Json record, uint32_t capacity);

// RPI.72 step 3, exactly as raster.rs.
enum class Shape : uint32_t { Placed, Outside, CrossesNear, Undefined };
struct Triangle {
    Shape shape = Shape::Undefined;
    std::array<std::array<float, 2>, 3> corners{}; // surface pixels, when placed
};
using Clip = std::array<float, 4>;
uint64_t extent(float low, float high, uint32_t size);
uint64_t count(uint32_t width, uint32_t height, const Triangle& triangle);
struct RasterPass {
    std::span<const Triangle> triangles;
    uint32_t samples;
};
uint64_t bound(uint32_t width, uint32_t height, std::span<const RasterPass> passes); // throws past 64 bits
uint64_t allowance(uint32_t width, uint32_t height, uint64_t overdraw);
// The core's clip classification (the adapter's convention, C9), the same in stages.hlsl's m600_count: a nonfinite
// component makes the triangle undefined; three vertices beyond one clip plane (x or y past +-w, z below 0 or past w)
// make it outside; a vertex with z below 0 or w not above 0 makes it cross the near plane; otherwise its vertices are
// placed with D3D12's viewport transform, ((x/w)/2 + 1/2) * width and (1/2 - (y/w)/2) * height, in binary32.
std::array<float, 2> pixel(const Clip& clip, uint32_t width, uint32_t height);
Triangle classify(const std::array<Clip, 3>& clip, uint32_t width, uint32_t height);
// The device's eval records, four 32-bit words per (cell, vertex), three vertices per triangle: a defined vertex holds
// its clip position, an undefined one (0, 0, 0, NaN) with its coverage code in the NaN's payload.
Clip record_clip(const uint32_t* words);
bool defined(const Clip& clip);
uint32_t coverage_code(const uint32_t* words); // 0 for a defined vertex
// U over a whole eval buffer: one pass at one sample.
uint64_t eval_bound(std::span<const uint32_t> words, uint32_t width, uint32_t height);
}
