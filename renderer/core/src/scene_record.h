#pragma once
#include "sa2_interop.h"
#include "probe.h"
#include "wj/probe.h"
#include <optional>
#include <stdexcept>

static_assert(sizeof(sa2_scene_config) == 32);
static_assert(sizeof(sa2_wj_config) == 48);

namespace sa2 {
struct Failure : std::runtime_error {
    int32_t status;
    Failure(int32_t code, const std::string& text) : std::runtime_error(text), status(code) {}
};
constexpr uint32_t SceneWj = 5; // internal scene number of a W-J scene; never an ABI value
void scene_require(bool condition, const std::string& text, int32_t status = SA2_E_WRONG_STATE);
void validate_scene_config(const sa2_scene_config* config);
void validate_wj_config(const sa2_wj_config* config);
uint64_t label_copies(const sa2_scene_config& config); // S-B: ceil(trace_ms / turn_ms) + 2, W4 at 190 ms
uint64_t wj_copies(const sa2_wj_config& config); // W-J: ceil(trace_ms / turn_ms) + 2 bundles
std::filesystem::path utf8_path(const char* utf8, const char* name);
std::filesystem::path output_directory(const char* utf8);
void require_new_file(const std::filesystem::path& file);
void write_new_file(const std::filesystem::path& file, const std::string& text);
std::vector<uint8_t> scene_bytes(const std::filesystem::path& file);
sb::Assets scene_assets();
std::vector<uint32_t> scene_slot_pieces(); // W1..W4's slot -> piece map for the pose path, under the manifest digest
wj::Assets wj_assets(const std::string& menu, const std::filesystem::path& data);
wj::PoseState wj_injected(const wj::Assets& assets, const std::string& injection); // poses[0] as uploaded at turn 20
std::filesystem::path module_path();
const std::array<const char*, 5>& shader_names();
Json scene_identity();
int64_t scene_qpc();

// W-J's revision protocol (gpu.cpp runGpu), without a device: the scene
// performs the uploads, draws and copies each step asks for.
struct WjStep { bool upload = false, fault = false, copy = false; };
struct WjProtocol {
    std::string injection;
    bool initial = true, injection_applied = false;
    uint64_t authoritative = 0;
    unsigned expected = 0, bound = 0;
    std::optional<unsigned> pending;
    std::array<uint64_t, 3> revision{};
    wj::LabelRecords records;
    WjStep timed(uint64_t frame, uint64_t turn); // one timed frame, before its draw
    void copied(uint64_t frame, uint64_t slot, uint32_t poses); // after the draw copied the bound bundle
    void finish(Json& labels, Json& poses) const; // an injection not reached fails both checks
};

struct SceneRecord {
    sa2_scene_config config{}; // a W-J scene stores scene SceneWj, turn_ms, flags and trace_ms here
    std::string injection, menu;
    std::filesystem::path data;
    int64_t frequency = 0, start = 0, stop = 0;
    bool begun = false, ended = false, injection_applied = false;
    uint32_t width = 0, height = 0;
    uint64_t vram_samples = 0;
    double vram_peak = 0;
    std::vector<sb::Trace> trace;
    SceneRecord(const sa2_scene_config& config, int64_t frequency, uint32_t width, uint32_t height);
    SceneRecord(const sa2_wj_config& config, int64_t frequency, uint32_t width, uint32_t height);
    bool jumbling() const { return config.scene == SceneWj; }
    bool tracing() const { return begun && !ended; }
    void begin(int64_t now);
    void end(int64_t now);
    void require_write(bool drained) const;
    void require_geometry() const;
    void require_lattice() const;
    Json native(const Json& labels, const Json& identity, int32_t queue_mode, int32_t barriers, const Json& poses = Json()) const;
    std::string trace_text() const;
};
}
