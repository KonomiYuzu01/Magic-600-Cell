#include "scene_record.h"
#include <windows.h>
#include <algorithm>
#include <cstring>

namespace sa2 {
void scene_require(bool condition, const std::string& text, int32_t status) {
    if (!condition) throw Failure(status, text);
}
void validate_scene_config(const sa2_scene_config* config) {
    scene_require(config && config->struct_size == sizeof(*config), "invalid scene_config struct_size", SA2_E_INVALID_ARGUMENT);
    scene_require(config->scene >= 1 && config->scene <= 4, "scene must be 1..4", SA2_E_INVALID_ARGUMENT);
    scene_require(config->scene != 3 || (std::isfinite(config->turn_ms) && config->turn_ms > 0 && config->turn_ms <= 10000),
        "W3 turn_ms must be finite and in (0,10000]", SA2_E_INVALID_ARGUMENT);
    scene_require(!(config->flags & ~SA2_SCENE_NO_VRAM), "unknown scene flags", SA2_E_INVALID_ARGUMENT);
    scene_require(config->trace_ms >= 1 && config->trace_ms <= 3600000, "trace_ms must be in 1..3600000", SA2_E_INVALID_ARGUMENT);
    if (config->scene >= 3) label_copies(*config);
    if (config->inject) {
        const std::string fault(config->inject);
        scene_require((config->scene == 3 || config->scene == 4) && (fault == "corrupt-label" || fault == "swap-same-colour"
            || fault == "delay-adoption" || fault == "stale-binding"), "unknown injection or injection outside W3/W4", SA2_E_INVALID_ARGUMENT);
    }
}
uint64_t label_copies(const sa2_scene_config& config) {
    const double copies = std::ceil(double(config.trace_ms) / (config.scene == 3 ? config.turn_ms : 190.0)) + 2;
    scene_require(copies * double(sb::LabelBytes) < 9.0e18, "trace_ms / turn_ms needs a label readback beyond 64-bit sizes",
        SA2_E_INVALID_ARGUMENT);
    return uint64_t(copies);
}
std::filesystem::path output_directory(const char* utf8) {
    scene_require(utf8 && *utf8, "directory_utf8 is null or empty", SA2_E_INVALID_ARGUMENT);
    const int n = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, utf8, -1, nullptr, 0);
    scene_require(n > 0, "directory_utf8 is not valid UTF-8", SA2_E_INVALID_ARGUMENT);
    std::wstring wide(static_cast<size_t>(n), L'\0');
    scene_require(MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, utf8, -1, wide.data(), n) == n,
        "directory_utf8 conversion failed", SA2_E_INVALID_ARGUMENT);
    wide.pop_back();
    return std::filesystem::path(wide);
}
void require_new_file(const std::filesystem::path& file) {
    std::error_code error;
    scene_require(std::filesystem::is_directory(file.parent_path(), error) && !error,
        "output directory does not exist", SA2_E_IO);
    const auto attributes = GetFileAttributesW(file.c_str());
    const auto status = GetLastError();
    scene_require(attributes == INVALID_FILE_ATTRIBUTES && status == ERROR_FILE_NOT_FOUND,
        "output exists or cannot be inspected: " + file.filename().string(), SA2_E_IO);
}
void write_new_file(const std::filesystem::path& file, const std::string& text) {
    HANDLE output = CreateFileW(file.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW, FILE_ATTRIBUTE_NORMAL, nullptr);
    scene_require(output != INVALID_HANDLE_VALUE, "cannot create new output: " + file.filename().string(), SA2_E_IO);
    bool ok = true;
    for (size_t offset = 0; offset < text.size();) {
        const auto count = static_cast<DWORD>(std::min<size_t>(text.size() - offset, MAXDWORD));
        DWORD written = 0;
        if (!WriteFile(output, text.data() + offset, count, &written, nullptr) || written != count) { ok = false; break; }
        offset += written;
    }
    if (!CloseHandle(output)) ok = false;
    scene_require(ok, "cannot write output: " + file.filename().string(), SA2_E_IO);
}
std::vector<uint8_t> scene_bytes(const std::filesystem::path& file) {
    try { return sb::bytes(file); }
    catch (const std::exception& error) { throw Failure(SA2_E_IO, error.what()); }
}
sb::Assets scene_assets() {
    try { return sb::Assets(); }
    catch (const std::exception& error) { throw Failure(SA2_E_IO, error.what()); }
}
std::filesystem::path module_path() {
    HMODULE module = nullptr;
    scene_require(GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        reinterpret_cast<LPCWSTR>(&module_path), &module) != 0, "cannot locate sa2_interop.dll module", SA2_E_IO);
    std::vector<wchar_t> path(32768);
    const auto count = GetModuleFileNameW(module, path.data(), static_cast<DWORD>(path.size()));
    scene_require(count && count < path.size(), "cannot locate sa2_interop.dll file", SA2_E_IO);
    return std::filesystem::path(std::wstring(path.data(), count));
}
const std::array<const char*, 4>& shader_names() {
    static constexpr std::array<const char*, 4> names{"count_vs.dxil", "draw_ps.dxil", "draw_vs.dxil", "geometry_cs.dxil"};
    return names;
}
Json scene_identity() {
    const auto dll = module_path();
    auto part = [](const std::filesystem::path& path) -> Json {
        return Json::Object{{"file", path.filename().string()}, {"sha256", sb::sha256(scene_bytes(path))}};
    };
    Json::Array shaders;
    for (const auto* file : shader_names()) shaders.push_back(part(dll.parent_path() / file));
    return Json::Object{{"dll", part(dll)}, {"shaders", shaders}};
}
int64_t scene_qpc() {
    LARGE_INTEGER now{};
    scene_require(QueryPerformanceCounter(&now) != 0, "QueryPerformanceCounter failed", SA2_E_D3D12);
    return now.QuadPart;
}
SceneRecord::SceneRecord(const sa2_scene_config& value, int64_t hz, uint32_t w, uint32_t h)
    : config(value), injection(value.inject ? value.inject : ""), frequency(hz), width(w), height(h) {
    config.inject = nullptr; // the owned string is authoritative
}
void SceneRecord::begin(int64_t now) {
    scene_require(!begun, "trace_begin is once per load");
    start = now; begun = true;
}
void SceneRecord::end(int64_t now) {
    scene_require(begun && !ended, "trace_end needs one trace_begin");
    stop = now; ended = true;
}
void SceneRecord::require_write(bool drained) const {
    scene_require(ended && drained, "write_run requires trace_end and a confirmed drain");
}
void SceneRecord::require_geometry() const { scene_require(!tracing(), "geometry check is outside the trace window"); }
Json SceneRecord::native(const Json& labels, const Json& identity, int32_t queue_mode, int32_t barriers) const {
    const Json size = Json::Object{{"width", width}, {"height", height}};
    Json result = Json::Object{{"format", "magic600-sa2-scene-native-v1"}, {"scene", "w" + std::to_string(config.scene)},
        {"qpc_frequency", frequency}, {"markers", Json::Object{{"trace_start_qpc", start}, {"trace_stop_qpc", stop}}},
        {"frames", uint64_t(trace.size())}, {"injection_applied", injection_applied},
        {"vram_peak_mb", (config.flags & SA2_SCENE_NO_VRAM) || !vram_samples ? Json() : Json(vram_peak)}, {"vram_samples", vram_samples},
        {"target", size}, {"viewport", size}, {"identity", identity},
        {"queue_mode", queue_mode == SA2_QUEUE_OWN ? "own" : "same"},
        {"barrier_api", barriers == SA2_BARRIERS_ENHANCED ? "enhanced" : "legacy"}};
    if (config.scene == 3) result["turn_ms"] = config.turn_ms;
    if (config.scene == 3 || config.scene == 4) result["label_check"] = labels;
    return result;
}
std::string SceneRecord::trace_text() const {
    std::string text;
    for (const auto& entry : trace) text += entry.json().dump() + '\n';
    return text;
}
}
