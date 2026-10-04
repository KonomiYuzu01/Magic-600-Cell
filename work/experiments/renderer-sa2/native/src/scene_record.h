#pragma once
#include "sa2_interop.h"
#include "probe.h"
#include <stdexcept>

static_assert(sizeof(sa2_scene_config) == 32);

namespace sa2 {
struct Failure : std::runtime_error {
    int32_t status;
    Failure(int32_t code, const std::string& text) : std::runtime_error(text), status(code) {}
};
void scene_require(bool condition, const std::string& text, int32_t status = SA2_E_WRONG_STATE);
void validate_scene_config(const sa2_scene_config* config);
std::filesystem::path output_directory(const char* utf8);
void require_new_file(const std::filesystem::path& file);
void write_new_file(const std::filesystem::path& file, const std::string& text);
std::vector<uint8_t> scene_bytes(const std::filesystem::path& file);
sb::Assets scene_assets();
std::filesystem::path module_path();
const std::array<const char*, 4>& shader_names();
Json scene_identity();
int64_t scene_qpc();

struct SceneRecord {
    sa2_scene_config config{};
    std::string injection;
    int64_t frequency = 0, start = 0, stop = 0;
    bool begun = false, ended = false, injection_applied = false;
    uint32_t width = 0, height = 0;
    uint64_t vram_samples = 0;
    double vram_peak = 0;
    std::vector<sb::Trace> trace;
    SceneRecord(const sa2_scene_config& config, int64_t frequency, uint32_t width, uint32_t height);
    bool tracing() const { return begun && !ended; }
    void begin(int64_t now);
    void end(int64_t now);
    void require_write(bool drained) const;
    void require_geometry() const;
    Json native(const Json& labels, const Json& identity, int32_t queue_mode, int32_t barriers) const;
    std::string trace_text() const;
};
}
