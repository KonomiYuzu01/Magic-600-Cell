#include "test.h"
#include "scene_record.h"
#include "product.h"
#include "stage_rules.h"
#include <algorithm>
#include <cmath>
#include <cstring>
#include <fstream>
#include <iostream>
#include <limits>
#include <memory>
#include <sstream>

namespace sa2test {
void expect(bool condition, const char* reason) { if (!condition) throw std::runtime_error(reason); }
void Suite::check(const std::string& name, const std::function<void()>& body) {
    try { body(); checks.push_back({name, "pass", "all assertions passed"}); }
    catch (const Unsupported& error) { checks.push_back({name, "unsupported", error.what()}); }
    catch (const std::exception& error) { checks.push_back({name, "fail", error.what()}); }
}
bool Suite::passed() const {
    return !checks.empty() && std::all_of(checks.begin(), checks.end(), [](const auto& check) {
        return (check.status == "pass" || check.status == "unsupported") && !check.reason.empty();
    });
}
std::filesystem::path executable_path() {
    std::vector<wchar_t> path(32768);
    const auto count = GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
    expect(count && count < path.size(), "GetModuleFileNameW failed");
    return std::filesystem::path(std::wstring(path.data(), count));
}
namespace {
struct Vector { sa2::Fields fields; uint64_t bits; };
std::vector<Vector> vectors(const std::string& argument) {
    // check_native supplies these numbers directly from the immutable JSON.
    const auto text = argument.empty()
        ? "1,0,0,0x4BB0000000000001;3735928559,2,2748,0x786FABC2DEADBEEF;1000,1,7,0x5DD00071000003E8" : argument;
    std::istringstream rows(text);
    std::vector<Vector> result;
    for (std::string row; std::getline(rows, row, ';');) {
        std::istringstream columns(row);
        std::array<uint64_t, 4> numbers{};
        for (auto& value : numbers) {
            std::string token;
            expect(static_cast<bool>(std::getline(columns, token, ',')), "incomplete --vectors row");
            std::size_t end = 0;
            value = std::stoull(token, &end, 0);
            expect(end == token.size(), "invalid --vectors number");
        }
        std::string extra;
        expect(!std::getline(columns, extra, ','), "extra --vectors column");
        expect(numbers[0] <= UINT32_MAX && numbers[1] <= UINT32_MAX && numbers[2] <= UINT32_MAX, "--vectors field overflow");
        result.push_back({{static_cast<uint32_t>(numbers[0]), static_cast<uint32_t>(numbers[1]), static_cast<uint32_t>(numbers[2])}, numbers[3]});
    }
    expect(result.size() == 3, "expected three code vectors");
    return result;
}
void invalid(int32_t status, const char* call) {
    expect(status == SA2_E_INVALID_ARGUMENT, "invalid input did not return SA2_E_INVALID_ARGUMENT");
    char error[512]{};
    expect(sa2_last_error(nullptr, error, sizeof(error)) == SA2_OK, "could not retrieve DLL error");
    expect(std::string(error).starts_with(std::string(call) + ':'), "DLL-wide last error does not name the failed call");
    expect(std::strchr(error, '\n') == nullptr && std::strchr(error, '\r') == nullptr, "last error is not one line");
}
// ABI 4: synthetic product inputs, without a device.
bool wrong_state(const std::function<void()>& call) {
    try { call(); } catch (const sa2::Failure& error) { return error.status == SA2_E_WRONG_STATE; }
    return false;
}
int32_t failure(const std::function<void()>& call) {
    try { call(); } catch (const sa2::Failure& error) { return error.status; }
    return SA2_OK;
}
bool rejects(const std::function<void()>& call) { return failure(call) == SA2_E_INVALID_ARGUMENT; }
// One engine state: label = slot and, when posed, every piece in pose 0.
struct EngineArrays {
    std::vector<uint32_t> labels = std::vector<uint32_t>(SA2_SLOTS);
    std::vector<int32_t> pose_index, lattice;
    std::vector<float> poses;
    sa2_engine_image image{};
    EngineArrays(uint64_t revision, uint32_t pose_count) {
        for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) labels[slot] = slot;
        image.struct_size = sizeof(image);
        image.revision = revision;
        image.labels = labels.data();
        image.digest[0] = uint8_t(revision);
        if (!pose_count) return;
        pose_index.assign(SA2_PIECES, 0);
        poses.assign(size_t(pose_count) * 16, 0.0f);
        for (size_t pose = 0; pose < pose_count; ++pose)
            for (size_t i = 0; i < 4; ++i) poses[pose * 16 + i * 5] = 1.0f;
        lattice.assign(pose_count, -1);
        lattice[0] = 0;
        image.pose_count = pose_count;
        image.pose_index = pose_index.data();
        image.poses = poses.data();
        image.pose_lattice = lattice.data();
    }
    EngineArrays(const EngineArrays&) = delete;
    EngineArrays& operator=(const EngineArrays&) = delete;
};
// One publication: every slot drawn and interactive, instance = slot + 1.
struct PreparationArrays {
    std::vector<uint8_t> visibility = std::vector<uint8_t>(SA2_SLOTS, 1), interaction = std::vector<uint8_t>(SA2_SLOTS, 1);
    std::vector<uint32_t> instances = std::vector<uint32_t>(SA2_SLOTS);
    sa2_preparation value{};
    PreparationArrays(uint64_t view, uint64_t view_generation, uint64_t publication_generation) {
        for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) instances[slot] = slot + 1;
        value.struct_size = sizeof(value);
        value.command = 7;
        value.view = view;
        value.view_generation = view_generation;
        value.publication_generation = publication_generation;
        value.device_epoch = sa2::DeviceEpoch;
        value.visibility = visibility.data();
        value.interaction = interaction.data();
        value.instances = instances.data();
    }
    PreparationArrays(const PreparationArrays&) = delete;
    PreparationArrays& operator=(const PreparationArrays&) = delete;
};
// A frame of surface 1 and view 1 at 256 x 256 with the identity camera.
sa2_frame_input frame_input(uint64_t revision, uint64_t sequence) {
    sa2_frame_input input{};
    input.struct_size = sizeof(input);
    input.device_epoch = sa2::DeviceEpoch;
    input.surface = 1;
    input.surface_generation = 1;
    input.sequence = sequence;
    input.engine_revision = revision;
    input.view = 1;
    input.view_generation = 1;
    input.publication_generation = 1;
    input.viewport_width = 256;
    input.viewport_height = 256;
    for (size_t i = 0; i < 4; ++i) input.camera_matrix[i * 5] = 1.0f;
    return input;
}
// Prepares a set whose upload signals fence and polls it ready: (ticket, resource set).
std::pair<uint64_t, uint64_t> ready_set(sa2::Product& product, const sa2::PreparedData& data, uint64_t fence) {
    product.check_prepare(data);
    const auto ticket = product.reserve_ticket();
    product.prepared(ticket, data, fence);
    const auto row = product.poll(ticket, fence);
    expect(row.state == SA2_TICKET_READY && row.resource_set != 0, "a completed upload was not ready");
    return {ticket, row.resource_set};
}
void run_cpu(Suite& suite, const Options& options) {
    suite.check("abi", [] { expect(sa2_abi_version() == 8, "ABI version mismatch"); });
    suite.check("exports", [] {
        const auto dll = executable_path().parent_path() / "sa2_interop.dll";
        HMODULE module = LoadLibraryW(dll.c_str());
        expect(module != nullptr, "LoadLibraryW of adjacent DLL failed");
        constexpr const char* names[] = {"sa2_abi_version", "sa2_last_error", "sa2_probe", "sa2_attach", "sa2_create_texture", "sa2_release_texture",
            "sa2_register_slot", "sa2_unregister_slot", "sa2_signal_godot_free", "sa2_produce", "sa2_godot_wait_ready", "sa2_mark_shown",
            "sa2_verify_slot", "sa2_drain", "sa2_debug_counts", "sa2_debug_messages", "sa2_remove_device", "sa2_device_removed_reason", "sa2_detach",
            "sa2_scene_load", "sa2_scene_produce", "sa2_scene_trace_begin", "sa2_scene_trace_end", "sa2_scene_write_run",
            "sa2_scene_geometry_check", "sa2_identity", "sa2_scene_unload", "sa2_scene_load_wj", "sa2_scene_lattice_check",
            "sa2_describe", "sa2_engine_adopt", "sa2_prepare", "sa2_poll", "sa2_activate", "sa2_cancel", "sa2_retire",
            "sa2_frame_submit", "sa2_frame_reports", "sa2_presented", "sa2_pick_request", "sa2_pick_poll", "sa2_recover",
            "sa2_resize", "sa2_counters", "sa2_device_status", "sa2_create", "sa2_surface_attach", "sa2_surface_move",
            "sa2_surface_detach", "sa2_surface_swap_chain", "sa2_view_shape_set", "sa2_style_set", "sa2_view_style_set", "sa2_frame_submit_panes", "sa2_frame_panes"};
        // The header declares 19 functions (the packet's count of 21 is not
        // reflected in ABI v1). Test every declared name without inventing ABI.
        bool found = true;
        for (const auto* name : names) found = found && GetProcAddress(module, name) != nullptr;
        FreeLibrary(module);
        expect(found, "missing ABI export");
    });
    suite.check("invalid arguments", [] {
        sa2_device_info info{};
        invalid(sa2_probe(0, 0, 0, &info), "sa2_probe");
        info.struct_size = sizeof(info);
        invalid(sa2_probe(0, 0, 0, &info), "sa2_probe");
        sa2_config config{};
        sa2_context* ctx = nullptr;
        invalid(sa2_attach(0, 0, &config, &ctx), "sa2_attach");
        config.struct_size = sizeof(config);
        invalid(sa2_attach(0, 0, &config, &ctx), "sa2_attach");
        expect(!ctx, "failed attach returned a context");
        invalid(sa2_attach(0, 0, &config, nullptr), "sa2_attach");
        uint64_t resource = 1, mismatch = 0;
        uint32_t refcount = 0;
        int32_t hr = 0;
        char text[8]{};
        struct sa2_debug_counts counts{};
        invalid(sa2_create_texture(nullptr, 127, 128, SA2_STATE_COMMON, &resource), "sa2_create_texture");
        expect(!resource, "failed create returned a resource");
        invalid(sa2_create_texture(nullptr, 128, 128, 99, &resource), "sa2_create_texture");
        invalid(sa2_release_texture(nullptr, 0, &refcount), "sa2_release_texture");
        invalid(sa2_register_slot(nullptr, SA2_RING_SLOTS, 0, 128, 128, 0), "sa2_register_slot");
        invalid(sa2_unregister_slot(nullptr, SA2_RING_SLOTS), "sa2_unregister_slot");
        invalid(sa2_signal_godot_free(nullptr, 0), "sa2_signal_godot_free");
        invalid(sa2_produce(nullptr, SA2_RING_SLOTS, 1, 1, 0), "sa2_produce");
        invalid(sa2_godot_wait_ready(nullptr, 1), "sa2_godot_wait_ready");
        invalid(sa2_mark_shown(nullptr, SA2_RING_SLOTS, 1), "sa2_mark_shown");
        invalid(sa2_verify_slot(nullptr, SA2_RING_SLOTS, 1, 0, &mismatch), "sa2_verify_slot");
        invalid(sa2_drain(nullptr, 0), "sa2_drain");
        invalid(sa2_debug_counts(nullptr, &counts), "sa2_debug_counts");
        counts.struct_size = sizeof(counts);
        invalid(sa2_debug_counts(nullptr, &counts), "sa2_debug_counts");
        invalid(sa2_debug_messages(nullptr, text, sizeof(text)), "sa2_debug_messages");
        invalid(sa2_remove_device(nullptr), "sa2_remove_device");
        invalid(sa2_device_removed_reason(nullptr, &hr), "sa2_device_removed_reason");
        invalid(sa2_detach(nullptr), "sa2_detach");
        invalid(sa2_last_error(nullptr, nullptr, 0), "sa2_last_error");
    });
    suite.check("last error truncation", [] {
        invalid(sa2_detach(nullptr), "sa2_detach");
        std::array<char, 10> buffer;
        buffer.fill('#');
        expect(sa2_last_error(nullptr, buffer.data(), 1) == SA2_OK, "size-1 last_error failed");
        expect(buffer[0] == 0 && buffer[1] == '#', "size-1 last_error overrun or missing terminator");
        buffer.fill('#');
        expect(sa2_last_error(nullptr, buffer.data(), 8) == SA2_OK, "size-8 last_error failed");
        expect(std::string(buffer.data()) == "sa2_det" && buffer[7] == 0 && buffer[8] == '#', "size-8 last_error truncation incorrect");
    });
    suite.check("crc", [] {
        constexpr uint8_t input[] = {'1','2','3','4','5','6','7','8','9'};
        expect(sa2::crc16(input, sizeof(input)) == 0x29b1, "CCITT-FALSE check vector mismatch");
    });
    suite.check("vectors", [&] {
        for (const auto& vector : vectors(options.vectors)) expect(sa2::code(vector.fields) == vector.bits, "JSON code vector mismatch");
        expect(sa2::code({1, 16, 4096}) == sa2::code({1, 0, 0}), "field modulo rule mismatch");
    });
    suite.check("images", [&] {
        constexpr std::array<std::array<uint32_t, 2>, 3> sizes{{{128, 128}, {129, 131}, {1280, 720}}};
        for (const auto& size : sizes) for (const auto& vector : vectors(options.vectors)) {
            const auto [width, height] = size;
            const auto pixels = sa2::image(width, height, vector.fields);
            for (uint32_t y = 0; y < height; ++y) for (uint32_t x = 0; x < width; ++x) {
                const bool tl = x < 64 && y < 64, br = x >= width - 64 && y >= height - 64;
                sa2::Pixel expected{static_cast<uint8_t>(32 + 64 * vector.fields.slot), static_cast<uint8_t>(vector.fields.sequence % 251),
                    static_cast<uint8_t>(16 * (vector.fields.generation % 16)), 255};
                if (tl || br) {
                    const auto bit = ((y - (br ? height - 64 : 0)) / 8) * 8 + (x - (br ? width - 64 : 0)) / 8;
                    const auto k = static_cast<uint8_t>((vector.bits & (uint64_t(1) << bit)) ? 255 : 0);
                    expected = {k, k, k, 255};
                }
                expect(pixels[std::size_t(y) * width + x] == expected, "expected image does not match layout");
            }
            sa2::Fields decoded{};
            expect(sa2::decode(pixels.data(), std::size_t(width) * 4, width, height, decoded) && decoded == vector.fields, "both-corner decode mismatch");
        }
    });
    suite.check("decode rejection", [] {
        auto pixels = sa2::image(128, 128, {1, 0, 0});
        for (unsigned y = 0; y < 8; ++y) for (unsigned x = 0; x < 8; ++x) pixels[y * 128 + x] = {0, 0, 0, 255};
        sa2::Fields fields{};
        expect(!sa2::decode(pixels.data(), 512, 128, 128, fields), "one flipped block was accepted");
        for (unsigned y = 64; y < 72; ++y) for (unsigned x = 64; x < 72; ++x) pixels[y * 128 + x] = {0, 0, 0, 255};
        expect(!sa2::decode(pixels.data(), 512, 128, 128, fields), "matching corners with a bad CRC were accepted");
        expect(!sa2::decode(nullptr, 512, 128, 128, fields), "null image accepted");
        expect(!sa2::decode(pixels.data(), 4, 128, 128, fields), "short row pitch accepted");
    });
    suite.check("hex masking", [] {
        expect(sa2::mask_hex("SA2 0x123f / 0XAbC! 0x 0Xg 0x0tail 10xFF") == "SA2 0x? / 0x?! 0x 0Xg 0x?tail 10x?", "hex mask mismatch");
        expect(sa2::mask_hex("") == "", "empty mask mismatch");
    });
    suite.check("state tables", [] {
        constexpr D3D12_RESOURCE_STATES legacy[] = {D3D12_RESOURCE_STATE_COMMON, D3D12_RESOURCE_STATE_RENDER_TARGET,
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE | D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATE_COPY_SOURCE};
        constexpr D3D12_BARRIER_LAYOUT layouts[] = {D3D12_BARRIER_LAYOUT_COMMON, D3D12_BARRIER_LAYOUT_RENDER_TARGET,
            D3D12_BARRIER_LAYOUT_SHADER_RESOURCE, D3D12_BARRIER_LAYOUT_SHADER_RESOURCE, D3D12_BARRIER_LAYOUT_COPY_SOURCE};
        sa2::State state{};
        for (int32_t i = 0; i < 5; ++i) expect(sa2::state(i, state) && state.legacy == legacy[i] && state.layout == layouts[i], "state table mismatch");
        for (int32_t bad : {-1, 5, INT32_MAX, INT32_MIN}) expect(!sa2::state(bad, state), "invalid state accepted");
    });
    suite.check("json writer", [] {
        expect(sa2::json_string("a\"\\\n\t\x01") == "\"a\\\"\\\\\\u000a\\u0009\\u0001\"", "JSON string escaping mismatch");
        const auto text = sa2::json_report("cpu", {{"x", "unsupported", "reason"}});
        expect(text == "{\"format\":\"magic600-sa2-native-selftest-v1\",\"mode\":\"cpu\",\"checks\":[{\"name\":\"x\",\"status\":\"unsupported\",\"reason\":\"reason\"}]}\n", "JSON envelope mismatch");
    });
    suite.check("scene arguments", [] {
        sa2_scene_config config{sizeof(sa2_scene_config), 3, 190, nullptr, 0, 180000};
        invalid(sa2_scene_load(nullptr, nullptr), "sa2_scene_load");
        invalid(sa2_scene_load(nullptr, &config), "sa2_scene_load");
        auto refuses = [&](sa2_scene_config value) {
            invalid(sa2_scene_load(nullptr, &value), "sa2_scene_load");
            bool caught = false;
            try { sa2::validate_scene_config(&value); }
            catch (const sa2::Failure& error) { caught = error.status == SA2_E_INVALID_ARGUMENT; }
            expect(caught, "shared scene argument validator accepted invalid config");
        };
        auto bad = config; bad.struct_size = 0; refuses(bad);
        for (uint32_t scene : {0u, 5u}) { bad = config; bad.scene = scene; refuses(bad); }
        for (double ms : {0.0, -1.0, 10000.1, std::numeric_limits<double>::infinity(), std::numeric_limits<double>::quiet_NaN()}) {
            bad = config; bad.turn_ms = ms; refuses(bad);
        }
        bad = config; bad.inject = "unknown"; refuses(bad);
        bad = config; bad.inject = ""; refuses(bad);
        bad = config; bad.scene = 1; bad.inject = "corrupt-label"; refuses(bad);
        for (uint32_t ms : {0u, 3600001u}) { bad = config; bad.trace_ms = ms; refuses(bad); }
        bad = config; bad.turn_ms = 1e-9; refuses(bad); // readback beyond 64-bit sizes
        expect(sa2::label_copies(config) == 950, "S-B readback rule: ceil(180000 / 190) + 2 copies");
        { auto w4 = config; w4.scene = 4; w4.turn_ms = 0; expect(sa2::label_copies(w4) == 950, "W4 readback not sized at 190 ms"); }
        { auto one = config; one.trace_ms = 1; expect(sa2::label_copies(one) == 3, "one-millisecond trace must reserve three copies"); }
        bad = config; bad.flags = 2; refuses(bad);
        for (const char* fault : {"corrupt-label", "swap-same-colour", "delay-adoption", "stale-binding"}) {
            auto good = config; good.inject = fault; sa2::validate_scene_config(&good);
        }
        invalid(sa2_scene_produce(nullptr, 0, 1), "sa2_scene_produce");
        invalid(sa2_scene_produce(nullptr, 3, UINT64_MAX), "sa2_scene_produce");
        invalid(sa2_scene_trace_begin(nullptr), "sa2_scene_trace_begin");
        invalid(sa2_scene_trace_end(nullptr), "sa2_scene_trace_end");
        invalid(sa2_scene_write_run(nullptr, "existing"), "sa2_scene_write_run");
        invalid(sa2_scene_write_run(nullptr, nullptr), "sa2_scene_write_run");
        invalid(sa2_scene_write_run(nullptr, ""), "sa2_scene_write_run");
        invalid(sa2_scene_geometry_check(nullptr, "existing"), "sa2_scene_geometry_check");
        invalid(sa2_scene_geometry_check(nullptr, "\xff"), "sa2_scene_geometry_check");
        invalid(sa2_scene_unload(nullptr), "sa2_scene_unload");
        invalid(sa2_identity(nullptr, 1), "sa2_identity");
        char buffer[8]{}; invalid(sa2_identity(buffer, 0), "sa2_identity");
    });
    suite.check("scene state", [] {
        sa2_scene_config config{sizeof(sa2_scene_config), 3, 190, nullptr, 0, 180000};
        sa2::SceneRecord record(config, 10000000, 640, 360);
        auto wrong = [](const auto& call) {
            bool caught = false;
            try { call(); } catch (const sa2::Failure& error) { caught = error.status == SA2_E_WRONG_STATE; }
            expect(caught, "shared scene lifecycle validator did not return WRONG_STATE");
        };
        wrong([&] { record.end(200); }); wrong([&] { record.require_write(true); });
        record.require_geometry(); record.begin(100);
        wrong([&] { record.begin(101); }); wrong([&] { record.require_write(true); });
        wrong([&] { record.require_geometry(); }); record.end(200);
        wrong([&] { record.end(201); }); wrong([&] { record.begin(202); });
        wrong([&] { record.require_write(false); }); record.require_write(true); record.require_geometry();
        expect(record.start == 100 && record.stop == 200, "markers changed after rejected calls");
    });
    suite.check("scene assets", [] {
        const auto assets = sa2::scene_assets();
        expect(assets.vertices.size() == size_t(sb::Vertices) * 4 && assets.local.size() == sb::Vertices
            && assets.centers.size() == size_t(sb::Stickers) * 4 && assets.frames.size() == size_t(sb::Cells) * 16
            && assets.flags.size() == sb::Slots && assets.oracle[0].size() == sb::Slots
            && assets.src.size() == 4600 && assets.moving.size() == 4605, "S-B full model counts mismatch");
        const auto edges = sb::edgeMasks(assets);
        expect(edges.featureEdges == 3277 && edges.diagonals == 5546, "unchanged S-B edge reader mismatch");
        const auto turn = sb::turnAt(100 + 20 * sb::turnTicks(190, 10000000), 100, sb::turnTicks(190, 10000000), assets.angle);
        expect(turn.index == 20 && turn.phase == 0, "S-B QPC turn clock mismatch");
    });
    suite.check("scene native writer", [] {
        sa2_scene_config config{sizeof(sa2_scene_config), 3, 190, nullptr, 0, 180000};
        sa2::SceneRecord record(config, 10000000, 2560, 1600); record.begin(100); record.end(200);
        record.vram_peak = 123.5; record.vram_samples = 1; record.trace.push_back({0, 150, 0, true, 0, 0.25, 1});
        const Json labels = Json::Object{{"status", "pass"}, {"copies", 0}, {"revisions", 0}, {"mismatches", 0},
            {"late_adoptions", 0}, {"missing", 0}, {"oracle_sha256", Json::Object{{"even", "even"}, {"odd", "odd"}}}};
        const Json identity = Json::Object{{"dll", Json::Object{{"file", "sa2_interop.dll"}, {"sha256", "synthetic"}}}, {"shaders", Json::Array{}}};
        const auto json = Json::parse(record.native(labels, identity, SA2_QUEUE_OWN, SA2_BARRIERS_ENHANCED).dump());
        for (const char* field : {"format", "scene", "qpc_frequency", "markers", "frames", "turn_ms", "label_check", "injection_applied",
            "vram_peak_mb", "vram_samples", "target", "viewport", "identity", "queue_mode", "barrier_api"}) json.at(field);
        expect(json.at("format").string() == "magic600-sa2-scene-native-v1" && json.at("scene").string() == "w3"
            && json.at("frames").integer() == 1 && json.at("turn_ms").number() == 190 && json.at("vram_peak_mb").number() == 123.5,
            "native writer values mismatch");
        expect(json.at("target").dump() == json.at("viewport").dump() && json.at("target").at("width").integer() == 2560
            && json.at("target").at("height").integer() == 1600 && json.at("identity").dump() == identity.dump(), "native sizes/identity mismatch");
        const auto trace = Json::parse(record.trace_text());
        expect(trace.at("frame").integer() == 0 && trace.at("qpc").integer() == 150 && trace.at("camera").integer() == 1, "S-B trace writer mismatch");
        record.vram_samples = 0;
        expect(record.native(labels, identity, 0, 1).at("vram_peak_mb").null(), "VRAM peak without a sample was not null");
        record.vram_samples = 1; record.config.flags = SA2_SCENE_NO_VRAM;
        expect(record.native(labels, identity, 0, 1).at("vram_peak_mb").null(), "NO_VRAM did not write null");
        for (uint32_t scene : {1u, 2u, 4u}) {
            record.config.scene = scene; const auto j = record.native(labels, identity, 0, 1);
            expect(!j.object().contains("turn_ms") && j.object().contains("label_check") == (scene == 4), "optional native fields mismatch");
        }
    });
    suite.check("scene output IO", [] {
        const auto directory = executable_path().parent_path() / ("scene-cpu-" + std::to_string(GetCurrentProcessId()));
        expect(std::filesystem::create_directory(directory), "CPU IO fixture directory exists");
        struct Cleanup { std::filesystem::path path; ~Cleanup() { std::error_code error; std::filesystem::remove_all(path, error); } } cleanup{directory};
        const auto file = directory / "native.json";
        sa2::require_new_file(file); sa2::write_new_file(file, "sentinel\n");
        for (bool preflight : {true, false}) {
            bool caught = false;
            try { if (preflight) sa2::require_new_file(file); else sa2::write_new_file(file, "overwritten"); }
            catch (const sa2::Failure& error) { caught = error.status == SA2_E_IO; }
            expect(caught, "existing output did not return E_IO");
        }
        const auto raw = sa2::scene_bytes(file);
        expect(std::string(raw.begin(), raw.end()) == "sentinel\n", "existing output was changed");
        bool caught = false;
        try { sa2::scene_bytes(directory / "absent.dxil"); } catch (const sa2::Failure& error) { caught = error.status == SA2_E_IO; }
        expect(caught, "missing file did not return E_IO");
    });
    suite.check("identity", [] {
        std::array<char, 4096> text{};
        expect(sa2_identity(text.data(), uint32_t(text.size())) == SA2_OK, "sa2_identity failed");
        const auto json = Json::parse(text.data());
        expect(json.at("dll").at("file").string() == "sa2_interop.dll" && json.at("dll").at("sha256").string().size() == 64, "DLL identity incomplete");
        const auto& shaders = json.at("shaders").array(); expect(shaders.size() == sa2::shader_names().size(), "shader identity incomplete");
        for (size_t i = 0; i < shaders.size(); ++i)
            expect(shaders[i].at("file").string() == sa2::shader_names()[i] && shaders[i].at("sha256").string().size() == 64, "shader identity order/name mismatch");
        std::array<char, 10> truncated; truncated.fill('#');
        expect(sa2_identity(truncated.data(), 1) == SA2_OK && truncated[0] == 0 && truncated[1] == '#', "identity size-1 truncation overrun");
        truncated.fill('#');
        expect(sa2_identity(truncated.data(), 8) == SA2_OK && truncated[7] == 0 && truncated[8] == '#', "identity size-8 truncation overrun");
        expect(shaders.size() == 8 && shaders[5].at("file").string() == "lattice_cs.dxil" && shaders[4].at("file").string() == "id_ps.dxil"
            && shaders[7].at("file").string() == "product_vs.dxil", "ABI 4 identity does not list lattice_cs.dxil and the product shaders");
    });
    suite.check("wj arguments", [] {
        sa2_wj_config config{sizeof(sa2_wj_config), 0, "S4", "existing", 190, nullptr, 180000, 0};
        invalid(sa2_scene_load_wj(nullptr, nullptr), "sa2_scene_load_wj");
        invalid(sa2_scene_load_wj(nullptr, &config), "sa2_scene_load_wj");
        auto refuses = [&](sa2_wj_config value) {
            invalid(sa2_scene_load_wj(nullptr, &value), "sa2_scene_load_wj");
            bool caught = false;
            try { sa2::validate_wj_config(&value); }
            catch (const sa2::Failure& error) { caught = error.status == SA2_E_INVALID_ARGUMENT; }
            expect(caught, "shared W-J argument validator accepted invalid config");
        };
        auto bad = config; bad.struct_size = 0; refuses(bad);
        bad = config; bad.struct_size = sizeof(sa2_scene_config); refuses(bad);
        bad = config; bad.reserved = 1; refuses(bad);
        for (const char* menu : {static_cast<const char*>(nullptr), "", "s4", "I_c", "S4 ", "w3"}) { bad = config; bad.menu = menu; refuses(bad); }
        for (const char* data : {static_cast<const char*>(nullptr), "", "\xff"}) { bad = config; bad.data_utf8 = data; refuses(bad); }
        for (double ms : {0.0, -1.0, 10000.1, std::numeric_limits<double>::infinity(), std::numeric_limits<double>::quiet_NaN()}) {
            bad = config; bad.turn_ms = ms; refuses(bad);
        }
        bad = config; bad.flags = 2; refuses(bad);
        for (uint32_t ms : {0u, 3600001u}) { bad = config; bad.trace_ms = ms; refuses(bad); }
        bad = config; bad.turn_ms = 1e-9; refuses(bad); // bundle readback beyond 64-bit sizes
        for (const char* fault : {"unknown", "", "corrupt-label"}) { bad = config; bad.inject = fault; refuses(bad); }
        expect(sa2::wj_copies(config) == 950, "W-J readback rule: ceil(180000 / 190) + 2 bundles");
        { auto one = config; one.trace_ms = 1; expect(sa2::wj_copies(one) == 3, "one-millisecond W-J trace must reserve three bundles"); }
        for (const char* fault : {"corrupt-index", "swap-same-colour", "stale-pose", "delay-adoption", "stale-binding"}) {
            auto good = config; good.inject = fault; sa2::validate_wj_config(&good);
        }
        for (const char* menu : {"I_a", "I_b"}) { auto good = config; good.menu = menu; sa2::validate_wj_config(&good); }
        // The W-J scene number is internal: ABI 2's load still refuses it.
        sa2_scene_config abi2{sizeof(sa2_scene_config), sa2::SceneWj, 190, nullptr, 0, 180000};
        invalid(sa2_scene_load(nullptr, &abi2), "sa2_scene_load");
        invalid(sa2_scene_lattice_check(nullptr, "existing"), "sa2_scene_lattice_check");
        invalid(sa2_scene_lattice_check(nullptr, nullptr), "sa2_scene_lattice_check");
        invalid(sa2_scene_lattice_check(nullptr, ""), "sa2_scene_lattice_check");
        invalid(sa2_scene_lattice_check(nullptr, "\xff"), "sa2_scene_lattice_check");
    });
    suite.check("wj scene state", [] {
        sa2_wj_config config{sizeof(sa2_wj_config), SA2_SCENE_NO_VRAM, "I_a", "data", 125, "stale-pose", 180000, 0};
        sa2::SceneRecord record(config, 10000000, 640, 360);
        expect(record.jumbling() && record.config.scene == sa2::SceneWj && record.config.turn_ms == 125 && record.config.trace_ms == 180000
            && record.config.flags == SA2_SCENE_NO_VRAM && record.config.inject == nullptr && record.menu == "I_a"
            && record.injection == "stale-pose" && record.data == std::filesystem::path(L"data"), "W-J record fields mismatch");
        expect(sa2::label_copies(record.config) == sa2::wj_copies(config), "W-J readback not sized by its turn_ms");
        auto wrong = [](const auto& call) {
            bool caught = false;
            try { call(); } catch (const sa2::Failure& error) { caught = error.status == SA2_E_WRONG_STATE; }
            expect(caught, "shared W-J lifecycle validator did not return WRONG_STATE");
        };
        wrong([&] { record.end(200); }); wrong([&] { record.require_write(true); });
        record.require_geometry(); record.require_lattice(); record.begin(100);
        wrong([&] { record.begin(101); }); wrong([&] { record.require_geometry(); }); wrong([&] { record.require_lattice(); });
        record.end(200); wrong([&] { record.require_write(false); }); record.require_write(true);
        record.require_geometry(); record.require_lattice();
        expect(record.start == 100 && record.stop == 200, "markers changed after rejected calls");
        for (uint32_t scene : {1u, 2u, 3u, 4u}) {
            sa2_scene_config abi2{sizeof(sa2_scene_config), scene, 190, nullptr, 0, 180000};
            const sa2::SceneRecord other(abi2, 10000000, 640, 360);
            expect(!other.jumbling(), "an ABI 2 scene reports W-J");
            wrong([&] { other.require_lattice(); });
        }
    });
    suite.check("wj reader refusals", [] {
        const auto directory = executable_path().parent_path() / ("wj-cpu-" + std::to_string(GetCurrentProcessId()));
        expect(std::filesystem::create_directory(directory), "CPU W-J fixture directory exists");
        struct Cleanup { std::filesystem::path path; ~Cleanup() { std::error_code error; std::filesystem::remove_all(path, error); } } cleanup{directory};
        auto refused = [](const std::filesystem::path& data, const std::string& reason) {
            bool caught = false; std::string text;
            try { sa2::wj_assets("S4", data); } catch (const sa2::Failure& error) { caught = error.status == SA2_E_IO; text = error.what(); }
            expect(caught && text.find(reason) != std::string::npos, ("W-J reader did not refuse with E_IO: " + reason + " (got: " + text + ")").c_str());
        };
        refused(directory / "absent", "cannot read motion.json");
        const auto menu = directory / "S4"; expect(std::filesystem::create_directory(menu), "cannot create the W-J menu fixture");
        const auto fixture = wj::sha256(wj::bytes(wj::repoRoot() / "research/jumbling/fixtures/wj-S4.json"));
        auto write = [](const std::filesystem::path& file, const std::string& text) {
            std::ofstream out(file, std::ios::binary | std::ios::trunc); out << text; expect(static_cast<bool>(out), "cannot write W-J fixture file");
        };
        write(menu / "motion.json", "{\"dimension\":3,\"fixture_sha256\":\"" + fixture + "\"}");
        refused(directory, "unsupported motion or stale fixture export");
        write(menu / "motion.json", "{\"dimension\":4,\"fixture_sha256\":\"" + std::string(64, '0') + "\"}");
        refused(directory, "unsupported motion or stale fixture export");
        write(menu / "motion.json", "{\"dimension\":4,\"fixture_sha256\":\"" + fixture + "\"}");
        refused(directory, "cannot read moving.i32");
        write(menu / "moving.i32", std::string("\x01\x02\x03\x04", 4));
        refused(directory, "moving set hash mismatch");
        // W-J's injector refuses faults the poses cannot carry (one identity pose here): INVALID_ARGUMENT at load.
        wj::Options options; options.scene = "w3";
        const wj::Assets identity_poses(options);
        for (const char* fault : {"swap-same-colour", "stale-pose"}) {
            bool caught = false;
            try { sa2::wj_injected(identity_poses, fault); } catch (const sa2::Failure& error) { caught = error.status == SA2_E_INVALID_ARGUMENT; }
            expect(caught, "a W-J injection the poses cannot carry was accepted");
        }
        for (const char* fault : {"", "corrupt-index", "delay-adoption", "stale-binding"}) sa2::wj_injected(identity_poses, fault);
        // W1..W4 bind the same slot -> piece map W-J's reader reads.
        expect(sa2::scene_slot_pieces() == identity_poses.slotPiece, "W1..W4 slot -> piece map differs from W-J's reader");
    });
    suite.check("wj protocol controls", [] {
        // W-J's revision protocol against W-J's exact label and pose checks,
        // with two synthetic two-entry pose tables standing in for the GPU's
        // ring resources and preserved bundles.
        wj::Options options; options.scene = "w3";
        wj::Assets assets(options);
        auto synthetic = [](float first_scale, float second_scale, int32_t shift) {
            wj::PoseState state; state.index.resize(wj::Pieces);
            for (uint32_t piece = 0; piece < wj::Pieces; ++piece) state.index[piece] = int32_t((piece + uint32_t(shift)) % 2);
            for (float scale : {first_scale, second_scale}) { auto m = wj::identity(); for (auto& x : m) x *= scale; state.rows.insert(state.rows.end(), m.begin(), m.end()); }
            state.lattice = {shift, 1 - shift};
            const auto raw = state.raw(); for (size_t i = 0; i < 3; ++i) state.hashes[i] = wj::sha256(raw[i]);
            return state;
        };
        assets.poses = {synthetic(1, 2, 0), synthetic(3, 4, 1)}; assets.maxPoses = 2;
        assets.oracle[1] = assets.oracle[0]; assets.oracleHash[1] = assets.oracleHash[0];
        auto run = [&](const std::string& injection, uint64_t turns) {
            sa2::WjProtocol protocol; protocol.injection = injection;
            const auto injected = sa2::wj_injected(assets, injection);
            std::array<wj::PoseState, 3> resource{assets.poses[0], assets.poses[0], assets.poses[0]};
            std::vector<wj::PoseState> preserved; uint64_t frame = 0;
            for (uint64_t turn = 0; turn < turns; ++turn) for (int k = 0; k < 3; ++k, ++frame) {
                const auto step = protocol.timed(frame, turn);
                if (step.upload) resource[protocol.expected] = step.fault ? injected : assets.poses[protocol.authoritative % 2];
                if (step.copy) { protocol.copied(frame, preserved.size(), resource[protocol.bound].count()); preserved.push_back(resource[protocol.bound]); }
            }
            auto labels = wj::checkLabels(assets, protocol.records, [&](const wj::Copy&) { return std::span<const uint32_t>(assets.oracle[0]); });
            auto poses = wj::checkPoses(assets, protocol.records, [&](const wj::Copy& c) { return preserved.at(c.slot).raw(); });
            protocol.finish(labels, poses);
            expect(protocol.injection_applied == (!injection.empty() && turns > 20), "W-J injection applied at the wrong turn");
            return std::pair{labels, poses};
        };
        const auto [labels, poses] = run("", 24);
        Json::Array revisions; for (uint64_t r = 0; r < 24; ++r) revisions.emplace_back(r);
        expect(labels.at("status").string() == "pass" && poses.at("status").string() == "pass", "clean W-J protocol failed its exact checks");
        expect(poses.at("revisions").dump() == Json(revisions).dump() && labels.at("copies").integer() == 24, "clean W-J protocol revisions mismatch");
        for (const char* fault : {"corrupt-index", "swap-same-colour", "stale-pose"}) {
            const auto [l, p] = run(fault, 24);
            expect(l.at("status").string() == "pass" && p.at("status").string() == "fail", ("W-J pose fault not caught: " + std::string(fault)).c_str());
        }
        for (const char* fault : {"delay-adoption", "stale-binding"}) {
            const auto [l, p] = run(fault, 24);
            expect(l.at("status").string() == "fail" && p.at("status").string() == "fail", ("W-J binding fault not caught: " + std::string(fault)).c_str());
        }
        const auto [l, p] = run("stale-pose", 15);
        expect(l.at("status").string() == "fail" && p.at("status").string() == "fail" && p.at("injection_not_reached").dump() == "true",
            "an injection not reached did not fail both W-J checks");
    });
    suite.check("wj native writer", [] {
        sa2_wj_config config{sizeof(sa2_wj_config), 0, "I_b", "data", 190, nullptr, 180000, 0};
        sa2::SceneRecord record(config, 10000000, 2560, 1600); record.begin(100); record.end(200);
        record.vram_peak = 99.5; record.vram_samples = 1; record.trace.push_back({0, 150, 0, true, 0, 0.25, 1});
        const Json labels = Json::Object{{"status", "pass"}, {"copies", 1}};
        const Json poses = Json::Object{{"status", "pass"}, {"revisions", Json::Array{0}}, {"checks", Json::Array{}}};
        const Json identity = Json::Object{{"dll", Json::Object{{"file", "sa2_interop.dll"}, {"sha256", "synthetic"}}}, {"shaders", Json::Array{}}};
        const auto json = Json::parse(record.native(labels, identity, SA2_QUEUE_OWN, SA2_BARRIERS_ENHANCED, poses).dump());
        for (const char* field : {"format", "scene", "qpc_frequency", "markers", "frames", "turn_ms", "label_check", "injection_applied",
            "vram_peak_mb", "vram_samples", "target", "viewport", "identity", "queue_mode", "barrier_api", "fixture", "overlay", "pose_check", "pose_upload"}) json.at(field);
        expect(json.at("format").string() == "magic600-sa2-scene-native-v1" && json.at("scene").string() == "wj" && json.at("fixture").string() == "wj-i-b"
            && json.at("overlay").string() == "none" && json.at("turn_ms").number() == 190 && json.at("label_check").dump() == labels.dump(),
            "W-J native writer values mismatch");
        const auto& pose_check = json.at("pose_check");
        expect(pose_check.object().size() == 2 && pose_check.at("status").string() == "pass" && pose_check.at("revisions").dump() == "[0]",
            "W-J native pose_check is not {status, revisions}");
        const auto trace = Json::parse(record.trace_text());
        expect(trace.at("turn").integer() == 0 && trace.at("phase").number() == 0.25, "W-J trace records lack turn and phase");
        for (const auto& [menu, fixture] : std::array<std::pair<const char*, const char*>, 2>{{{"S4", "wj-s4"}, {"I_a", "wj-i-a"}}}) {
            auto other = config; other.menu = menu;
            expect(sa2::SceneRecord(other, 10000000, 64, 64).native(labels, identity, 0, 1, poses).at("fixture").string() == fixture, "W-J fixture name mismatch");
        }
    });
    // ABI 4: the product API's device-free logic (product.cpp) and its argument checks.
    suite.check("product arguments", [] {
        char text[64]{};
        uint32_t size = 0;
        uint64_t value = 0;
        int32_t hr = 0;
        const uint8_t digest[32]{};
        sa2_engine_image image{sizeof(sa2_engine_image)};
        sa2_preparation preparation{sizeof(sa2_preparation)};
        sa2_poll_row poll{sizeof(sa2_poll_row)};
        sa2_frame_input input{sizeof(sa2_frame_input)};
        sa2_frame_report report{sizeof(sa2_frame_report)};
        sa2_presented_row presented{sizeof(sa2_presented_row)};
        sa2_pick_row pick{sizeof(sa2_pick_row)};
        sa2_counter_values counters{sizeof(sa2_counter_values)};
        invalid(sa2_describe(nullptr, text, sizeof(text), &size), "sa2_describe");
        invalid(sa2_engine_adopt(nullptr, &image, &value), "sa2_engine_adopt");
        invalid(sa2_prepare(nullptr, &preparation, &value), "sa2_prepare");
        invalid(sa2_poll(nullptr, 1, &poll), "sa2_poll");
        invalid(sa2_activate(nullptr, 1, digest), "sa2_activate");
        invalid(sa2_cancel(nullptr, 1, SA2_CANCEL_BY_USER, 0), "sa2_cancel");
        invalid(sa2_retire(nullptr, 1), "sa2_retire");
        invalid(sa2_frame_submit(nullptr, &input, &value), "sa2_frame_submit");
        invalid(sa2_frame_reports(nullptr, &report, 1, &size), "sa2_frame_reports");
        invalid(sa2_presented(nullptr, &presented, 1, &size), "sa2_presented");
        invalid(sa2_pick_request(nullptr, 1, 0, 0, &value), "sa2_pick_request");
        invalid(sa2_pick_poll(nullptr, 1, &pick), "sa2_pick_poll");
        invalid(sa2_recover(nullptr, text, sizeof(text), &size), "sa2_recover");
        invalid(sa2_resize(nullptr, 256, 256), "sa2_resize");
        invalid(sa2_counters(nullptr, &counters), "sa2_counters");
        invalid(sa2_device_status(nullptr, &hr), "sa2_device_status");
        expect(size == 0 && value == 0 && hr == 0, "a refused call left an output set");
        // The engine image.
        EngineArrays engine(1, 0);
        expect(sa2::engine_image(&engine.image).labels.size() == SA2_SLOTS, "a valid engine image was refused");
        expect(rejects([&] { auto bad = engine.image; bad.struct_size = 0; sa2::engine_image(&bad); }), "engine_image struct_size was accepted");
        expect(rejects([&] { auto bad = engine.image; bad.labels = nullptr; sa2::engine_image(&bad); }), "NULL labels were accepted");
        expect(rejects([&] { auto bad = engine.image; bad.pose_count = 1; sa2::engine_image(&bad); }), "pose_count without a pose index was accepted");
        engine.labels[5] = SA2_SLOTS;
        expect(rejects([&] { sa2::engine_image(&engine.image); }), "a label at SA2_SLOTS was accepted");
        engine.labels[5] = 5;
        EngineArrays posed(2, 3);
        expect(sa2::engine_image(&posed.image).posed(), "a valid posed image was refused");
        expect(rejects([&] { auto bad = posed.image; bad.pose_count = SA2_MAX_POSES + 1; sa2::engine_image(&bad); }), "pose_count above SA2_MAX_POSES was accepted");
        expect(rejects([&] { auto bad = posed.image; bad.poses = nullptr; sa2::engine_image(&bad); }), "a pose index without poses was accepted");
        posed.pose_index[9] = 3;
        expect(rejects([&] { sa2::engine_image(&posed.image); }), "a pose index at pose_count was accepted");
        posed.pose_index[9] = 0;
        posed.poses[0] = 2.0f;
        expect(rejects([&] { sa2::engine_image(&posed.image); }), "a pose 0 other than the identity was accepted");
        posed.poses[0] = 1.0f;
        posed.poses[20] = std::numeric_limits<float>::quiet_NaN();
        expect(rejects([&] { sa2::engine_image(&posed.image); }), "a non-finite pose was accepted");
        posed.poses[20] = 0.0f;
        posed.lattice[1] = SA2_POSE_GROUP_ORDER;
        expect(rejects([&] { sa2::engine_image(&posed.image); }), "a pose_lattice entry at SA2_POSE_GROUP_ORDER was accepted");
        posed.lattice[1] = -1;
        posed.lattice[0] = 1;
        expect(rejects([&] { sa2::engine_image(&posed.image); }), "pose_lattice entry 0 other than 0 was accepted");
        posed.lattice[0] = 0;
        expect(sa2::engine_image(&posed.image).lattice[1] == -1, "the restored posed image was refused");
        // The preparation (F02: a slot that is not drawn or not interactive writes no instance).
        PreparationArrays arrays(1, 1, 1);
        arrays.interaction[10] = 0;
        arrays.visibility[11] = 0;
        const auto data = sa2::preparation(&arrays.value);
        expect(data.visible.size() == SA2_SLOTS && data.instances.size() == SA2_SLOTS && data.instances[12] == 13 && data.visible[10] == 1
            && data.instances[10] == 0 && data.visible[11] == 0 && data.instances[11] == 0 && data.stamp.command == 7, "the preparation copy is wrong");
        arrays.visibility[12] = 2;
        expect(rejects([&] { sa2::preparation(&arrays.value); }), "a visibility byte of 2 was accepted");
        arrays.visibility[12] = 1;
        expect(rejects([&] { auto bad = arrays.value; bad.reserved = 1; sa2::preparation(&bad); }), "a nonzero preparation reserved was accepted");
        expect(rejects([&] { auto bad = arrays.value; bad.instances = nullptr; sa2::preparation(&bad); }), "NULL instances were accepted");
        expect(rejects([&] { auto bad = arrays.value; bad.struct_size -= 8; sa2::preparation(&bad); }), "preparation struct_size was accepted");
        // The frame input.
        const auto frame = frame_input(1, 1);
        sa2::validate_frame_input(&frame);
        expect(rejects([&] { auto bad = frame; bad.flags = 2; sa2::validate_frame_input(&bad); }), "an unknown frame flag was accepted");
        expect(rejects([&] { auto bad = frame; bad.reserved = 1; sa2::validate_frame_input(&bad); }), "a nonzero frame reserved was accepted");
        expect(rejects([&] { auto bad = frame; bad.target = SA2_RING_SLOTS; sa2::validate_frame_input(&bad); }), "a target outside the ring was accepted");
        expect(rejects([&] { auto bad = frame; bad.viewport_height = SA2_MIN_TEXTURE_PX - 1; sa2::validate_frame_input(&bad); }), "a viewport below the minimum was accepted");
        expect(rejects([&] { auto bad = frame; bad.preview = 3; sa2::validate_frame_input(&bad); }), "a preview without SA2_FRAME_PREVIEW was accepted");
        expect(rejects([&] { auto bad = frame; bad.camera_matrix[3] = std::numeric_limits<float>::infinity(); sa2::validate_frame_input(&bad); }),
            "a non-finite camera matrix was accepted");
        auto preview = frame;
        preview.flags = SA2_FRAME_PREVIEW;
        preview.preview = 3;
        preview.preview_base = 2;
        preview.preview_literal_digest[0] = 1;
        sa2::validate_frame_input(&preview);
        // Cancel reasons and row arrays.
        sa2::validate_cancel(SA2_CANCEL_SUPERSEDED, 9);
        sa2::validate_cancel(SA2_CANCEL_BUDGET, 0);
        expect(rejects([] { sa2::validate_cancel(SA2_CANCEL_BY_USER, 9); }), "superseded_by without SA2_CANCEL_SUPERSEDED was accepted");
        expect(rejects([] { sa2::validate_cancel(SA2_CANCEL_BUDGET + 1, 0); }) && rejects([] { sa2::validate_cancel(-1, 0); }), "an unknown cancel reason was accepted");
        sa2::validate_rows<sa2_frame_report>(nullptr, 0);
        expect(rejects([] { sa2::validate_rows<sa2_frame_report>(nullptr, 1); }), "NULL rows with a capacity were accepted");
        sa2_frame_report rows[2]{};
        rows[0].struct_size = sizeof(sa2_frame_report);
        expect(rejects([&] { sa2::validate_rows(rows, 2); }), "a row without struct_size was accepted");
        rows[1].struct_size = sizeof(sa2_frame_report);
        sa2::validate_rows(rows, 2);
    });
    suite.check("product adopt", [] {
        sa2::Product product;
        expect(product.image() == nullptr, "a fresh product holds an image");
        EngineArrays first(5, 0);
        expect(product.adopt(sa2::engine_image(&first.image)) == 5, "adopt did not return the revision");
        first.labels[0] = 7; // the caller's arrays may change once adopt returned
        expect(product.image()->labels[0] == 0 && product.image()->digest[0] == 5 && !product.image()->posed(), "adopt did not copy the image");
        expect(wrong_state([&] { product.adopt(sa2::engine_image(&first.image)); }), "an equal revision was adopted");
        EngineArrays older(4, 0);
        expect(wrong_state([&] { product.adopt(sa2::engine_image(&older.image)); }), "an older revision was adopted");
        EngineArrays posed(6, 2);
        expect(product.adopt(sa2::engine_image(&posed.image)) == 6 && product.image()->posed() && product.image()->pose_count == 2, "a posed revision was not adopted");
        expect(product.image()->bytes() == uint64_t(SA2_SLOTS) * 4 + uint64_t(SA2_PIECES) * 4 + 2 * (64 + 4), "the posed upload size is wrong");
        const auto counters = product.counter_values();
        expect(counters.bytes_uploaded == 0 && counters.frames_submitted == 0, "adopt counted GPU work (RM.16: CPU copy only)");
    });
    suite.check("product tickets", [] {
        sa2::Product product;
        PreparationArrays arrays(1, 1, 1);
        const auto data = sa2::preparation(&arrays.value);
        const uint8_t stamp[32]{1};
        // Pending until the upload's fence, then ready with a resource set.
        product.check_prepare(data);
        const auto first = product.reserve_ticket();
        product.prepared(first, data, 10);
        expect(product.counters().bytes_uploaded == uint64_t(SA2_SLOTS) * 8, "the preparation upload size is wrong");
        auto row = product.poll(first, 9);
        expect(row.struct_size == sizeof(row) && row.state == SA2_TICKET_PENDING && row.resource_set == 0 && row.reason == 0, "an incomplete upload was not pending");
        row = product.poll(first, 10);
        const auto set = row.resource_set;
        expect(row.state == SA2_TICKET_READY && set != 0 && product.poll(first, 10).resource_set == set, "a completed upload was not ready with one set");
        expect(wrong_state([&] { product.activate(set + 100, stamp); }), "an unknown set was activated");
        product.activate(set, stamp);
        expect(wrong_state([&] { product.activate(set, stamp); }), "a set was activated twice");
        expect(wrong_state([&] { product.cancel(first, SA2_CANCEL_BY_USER, 0); }), "the ticket of an active set was cancelled");
        product.retire(set);
        expect(wrong_state([&] { product.retire(set); }) && wrong_state([&] { product.activate(set, stamp); }), "a retired set was retired or activated");
        expect(wrong_state([&] { product.poll(first, 10); }), "the ticket of a retired set is still known");
        expect(product.releasable(0, 10) == std::vector<uint64_t>{first}, "a retired, unused set was not released");
        expect(product.releasable(0, 10).empty() && product.live_tickets() == 0, "a released ticket was returned twice or kept");
        // RM.17: cancelled while pending, the late completion yields nothing.
        const auto second = product.reserve_ticket();
        product.prepared(second, data, 20);
        product.cancel(second, SA2_CANCEL_SUPERSEDED, 44);
        expect(product.releasable(0, 19).empty(), "a cancelled set was released before its upload completed");
        expect(product.releasable(0, 20) == std::vector<uint64_t>{second}, "a cancelled, uploaded set was not released");
        row = product.poll(second, 20);
        expect(row.state == SA2_TICKET_CANCELLED && row.reason == SA2_CANCEL_SUPERSEDED && row.superseded_by == 44 && row.resource_set == 0,
            "a late completion after cancel changed the reply (RM.17)");
        row = product.poll(second, 20);
        expect(row.state == SA2_TICKET_CANCELLED && row.superseded_by == 44, "polling again did not return the cancellation (RM.17)");
        product.releasable(0, 20);
        expect(product.live_tickets() == 1 && product.held_tickets() == 0, "a released cancellation holds GPU memory or was forgotten");
        // Cancelled when ready but not active: the set is gone; a second cancel does nothing.
        const auto third = product.reserve_ticket();
        product.prepared(third, data, 30);
        const auto third_set = product.poll(third, 30).resource_set;
        product.cancel(third, SA2_CANCEL_BUDGET, 0);
        product.cancel(third, SA2_CANCEL_BY_USER, 0);
        expect(wrong_state([&] { product.activate(third_set, stamp); }) && wrong_state([&] { product.retire(third_set); }), "a cancelled set was activated or retired");
        row = product.poll(third, 30);
        expect(row.state == SA2_TICKET_CANCELLED && row.reason == SA2_CANCEL_BUDGET && row.superseded_by == 0, "a second cancel changed the reply");
        // Another device epoch, unknown tickets and the ticket bound.
        auto other = data;
        other.stamp.device_epoch = sa2::DeviceEpoch + 1;
        expect(wrong_state([&] { product.check_prepare(other); }), "another device epoch was prepared");
        expect(wrong_state([&] { product.poll(999, 0); }) && wrong_state([&] { product.cancel(999, SA2_CANCEL_BY_USER, 0); }), "an unknown ticket was accepted");
        std::vector<uint64_t> held;
        while (product.held_tickets() < sa2::MaxTickets) {
            held.push_back(product.reserve_ticket());
            product.prepared(held.back(), data, 40);
        }
        expect(wrong_state([&] { product.check_prepare(data); }), "a ticket beyond the GPU bound was accepted");
        // The host never polls a ticket it cancelled: releasing it frees the bound.
        product.cancel(held.front(), SA2_CANCEL_BY_USER, 0);
        expect(product.releasable(0, 40).size() == 2, "the cancelled tickets were not released");
        product.check_prepare(data);
        // Ended replies are bounded: three are kept (second, third and the
        // cancelled held ticket); one more than the bound forgets the oldest.
        for (size_t i = 0; i + 2 < sa2::MaxEnded; ++i) {
            const auto ticket = product.reserve_ticket();
            product.prepared(ticket, data, 50);
            product.cancel(ticket, SA2_CANCEL_BY_USER, 0);
            product.releasable(0, 50);
        }
        expect(wrong_state([&] { product.poll(second, 50); }), "the oldest ended reply was kept beyond the bound");
        expect(product.poll(third, 50).state == SA2_TICKET_CANCELLED, "a newer ended reply was forgotten");
        expect(product.held_tickets() == sa2::MaxTickets - 2, "an ended ticket still holds GPU memory");
    });
    suite.check("product frames", [] {
        sa2::Product product;
        auto input = frame_input(3, 1);
        expect(wrong_state([&] { product.plan(input, 256, 256); }), "a frame was planned without an adopted image");
        EngineArrays engine(3, 0);
        product.adopt(sa2::engine_image(&engine.image));
        auto plan = product.plan(input, 256, 256);
        expect(plan.upload && plan.copy == 0 && plan.upload_wait == 0 && plan.pose_count == 0 && plan.set_ticket == 0 && plan.image == product.image(),
            "the first frame does not upload into ring copy 0");
        product.commit(input, plan, 1, 0);
        product.produced(1, 100, 10);
        expect(product.counters().bytes_uploaded == uint64_t(SA2_SLOTS) * 4 && product.counters().frames_submitted == 1, "the frame upload was not counted");
        input.sequence = 2;
        plan = product.plan(input, 256, 256);
        expect(!plan.upload && plan.copy == 0, "an uploaded revision was uploaded again");
        product.commit(input, plan, 2, 1);
        product.produced(2, 100, 10);
        // Stamp rules.
        expect(wrong_state([&] { product.plan(input, 256, 256); }), "a repeated sequence was accepted");
        input.sequence = 3;
        expect(wrong_state([&] { auto bad = input; bad.surface_generation = 0; product.plan(bad, 256, 256); }), "a decreasing surface_generation was accepted");
        expect(wrong_state([&] { auto bad = input; bad.view_generation = 0; product.plan(bad, 256, 256); }), "a decreasing view_generation was accepted");
        expect(wrong_state([&] { auto bad = input; bad.engine_revision = 2; product.plan(bad, 256, 256); }), "an engine revision not adopted was accepted");
        expect(wrong_state([&] { auto bad = input; bad.device_epoch = sa2::DeviceEpoch + 1; product.plan(bad, 256, 256); }), "another device epoch was accepted");
        expect(wrong_state([&] { product.plan(input, 512, 256); }), "a viewport other than the target's size was accepted");
        expect(wrong_state([&] { auto bad = input; bad.resource_set = 77; product.plan(bad, 256, 256); }), "an unknown resource set was accepted");
        auto other = input;
        other.surface = 2;
        other.sequence = 1;
        product.plan(other, 256, 256); // sequences count per surface
        // New revisions take the oldest ring copy; reusing one waits for the frame that last uploaded into it.
        uint64_t frame = 2;
        const uint32_t copies[] = {1, 2, 0};
        const uint64_t waits[] = {0, 0, 1};
        for (uint64_t revision = 4; revision <= 6; ++revision) {
            EngineArrays next(revision, revision == 5 ? 2 : 0);
            product.adopt(sa2::engine_image(&next.image));
            input.engine_revision = revision;
            input.sequence = ++frame;
            plan = product.plan(input, 256, 256);
            const auto i = size_t(revision - 4);
            expect(plan.upload && plan.copy == copies[i] && plan.upload_wait == waits[i] && plan.pose_count == (revision == 5 ? 2u : 0u),
                "the engine ring did not take the oldest copy");
            product.commit(input, plan, frame, uint32_t((frame - 1) % SA2_RING_SLOTS));
            product.produced(frame, 100, 10);
        }
        // A resource set draws only an active set's frames of its view, view generation and publication generation.
        PreparationArrays arrays(1, 1, 1);
        const auto readied = ready_set(product, sa2::preparation(&arrays.value), 0);
        input.sequence = ++frame;
        input.resource_set = readied.second;
        expect(wrong_state([&] { product.plan(input, 256, 256); }), "a set that is not active was drawn");
        const uint8_t stamp[32]{};
        product.activate(readied.second, stamp);
        plan = product.plan(input, 256, 256);
        expect(plan.set_ticket == readied.first && !plan.upload && plan.copy == 0, "the active set was not planned");
        expect(wrong_state([&] { auto bad = input; bad.view_generation = 2; product.plan(bad, 256, 256); }), "a set was drawn for another view generation");
        expect(wrong_state([&] { auto bad = input; bad.publication_generation = 2; product.plan(bad, 256, 256); }), "a set was drawn for another publication generation");
        expect(wrong_state([&] { auto bad = input; bad.view = 2; product.plan(bad, 256, 256); }), "a set was drawn for another view");
        product.commit(input, plan, frame, uint32_t((frame - 1) % SA2_RING_SLOTS));
        // A retired set is released once the frames that used it completed.
        product.retire(readied.second);
        expect(product.releasable(frame - 1, 0).empty(), "a retired set was released before its last frame completed");
        expect(product.releasable(frame, 0) == std::vector<uint64_t>{readied.first}, "a retired set was not released after its last frame");
        expect(product.counters().frames_submitted == frame, "frames_submitted is wrong");
    });
    suite.check("product reports", [] {
        sa2::Product product;
        EngineArrays engine(1, 0);
        product.adopt(sa2::engine_image(&engine.image));
        const auto submit = [&](uint64_t frame, uint32_t ring) {
            const auto input = frame_input(1, frame);
            product.commit(input, product.plan(input, 256, 256), frame, ring);
        };
        for (uint64_t frame = 1; frame <= 3; ++frame) submit(frame, uint32_t(frame - 1));
        sa2_frame_report rows[4]{};
        expect(product.reports(rows, 4) == 0, "an incomplete frame was reported");
        product.produced(2, 500, 7);
        expect(product.reports(rows, 4) == 0, "a frame was reported before an earlier incomplete one");
        product.produced(1, 400, 5);
        expect(product.reports(rows, 1) == 1 && rows[0].struct_size == sizeof(rows[0]) && rows[0].frame_id == 1 && rows[0].error == SA2_OK
            && rows[0].gpu_time_ns == 400 && rows[0].view_vertex_evals == 5 && rows[0].sequence == 1 && rows[0].engine_revision == 1
            && rows[0].device_epoch == sa2::DeviceEpoch && rows[0].surface == 1 && rows[0].fallback == 0, "capacity 1 did not report frame 1");
        expect(product.reports(rows, 4) == 1 && rows[0].frame_id == 2 && rows[0].view_vertex_evals == 7, "frame 2 was not reported once, in order");
        // A frame whose ring slot was reused before its results were read drew without results.
        submit(4, 2);
        product.produced(4, 600, 9);
        expect(product.reports(rows, 4) == 2 && rows[0].frame_id == 3 && rows[0].error == SA2_E_D3D12 && rows[0].view_vertex_evals == 0
            && rows[1].frame_id == 4 && rows[1].error == SA2_OK, "an unread frame was not reported with an error");
        auto counters = product.counter_values();
        expect(counters.struct_size == sizeof(counters) && counters.frames_submitted == 4 && counters.frames_reported == 4
            && counters.view_vertex_evals_last == 9 && counters.view_vertex_evals_total == 21 && counters.reports_dropped == 0, "the report counters are wrong");
        expect(product.reports(nullptr, 0) == 0, "an empty read reported a frame");
        // Unread reports are bounded: the oldest completed one is dropped and counted.
        const uint64_t last = 5 + sa2::MaxUnreported;
        for (uint64_t frame = 5; frame <= last; ++frame) {
            submit(frame, uint32_t(frame % SA2_RING_SLOTS));
            product.produced(frame, 1, 1);
        }
        counters = product.counter_values();
        expect(counters.reports_dropped == 1 && counters.frames_submitted == last, "the report beyond the bound did not drop the oldest");
        expect(product.reports(rows, 4) == 4 && rows[0].frame_id == 6 && rows[3].frame_id == 9, "reading resumed at the wrong frame");
        expect(product.retained_frames() <= sa2::MaxUnreported + sa2::RetainedFrames, "the frame records are unbounded");
    });
    suite.check("product picks", [] {
        sa2::Product product;
        EngineArrays engine(1, 0);
        product.adopt(sa2::engine_image(&engine.image));
        const auto submit = [&](sa2_frame_input input, uint64_t frame) {
            input.sequence = frame;
            product.commit(input, product.plan(input, 256, 256), frame, uint32_t((frame - 1) % SA2_RING_SLOTS));
            product.produced(frame, 100, 1);
        };
        const auto request = [&](uint64_t frame, uint64_t fence) {
            const auto plan = product.pick_plan(frame, 10, 20);
            const auto ticket = product.reserve_pick();
            product.pick_commit(ticket, plan, plan.pass ? fence : 0);
            return std::make_pair(ticket, plan);
        };
        const auto poll = [&](uint64_t ticket, uint64_t completed, bool& release) {
            return product.pick_poll(ticket, completed, [] { return 77u; }, release);
        };
        const uint8_t stamp[32]{};
        PreparationArrays first_arrays(1, 1, 1);
        const auto first = ready_set(product, sa2::preparation(&first_arrays.value), 0);
        product.activate(first.second, stamp);
        auto input = frame_input(1, 1);
        input.resource_set = first.second;
        input.camera = 5;
        input.camera_matrix[12] = 0.5f;
        submit(input, 1);
        expect(wrong_state([&] { product.pick_plan(99, 0, 0); }), "a pick on an unknown frame was accepted");
        expect(failure([&] { product.pick_plan(1, 256, 0); }) == SA2_E_INVALID_ARGUMENT, "a pick outside the viewport was accepted");
        // Pending until the ID pass completes, then the instance with the frame's binding.
        const auto ready = request(1, 50);
        const auto& pass = ready.second.input;
        expect(ready.second.pass && pass.set_ticket == first.first && pass.copy == 0 && pass.width == 256 && pass.height == 256 && pass.x == 10
            && pass.y == 20 && pass.camera[12] == 0.5f && pass.pose_count == 0, "the ID pass does not use the frame's binding");
        bool release = true;
        auto row = poll(ready.first, 49, release);
        expect(row.struct_size == sizeof(row) && row.state == SA2_PICK_PENDING && row.instance == 0 && !release, "an incomplete ID pass was not pending");
        row = poll(ready.first, 50, release);
        expect(row.state == SA2_PICK_READY && row.instance == 77 && row.stale == 0 && release && row.frame_id == 1 && row.camera == 5 && row.surface == 1
            && row.view == 1 && row.publication_generation == 1 && row.viewport_width == 256 && row.x == 10 && row.y == 20,
            "a completed ID pass was not ready with the frame's binding");
        expect(wrong_state([&] { poll(ready.first, 50, release); }), "a reported pick is still known");
        // RPI.23: a newer surface generation makes a pending reply stale, and a later request stale at once.
        const auto surface = request(1, 60);
        input.surface_generation = 2;
        submit(input, 2);
        row = poll(surface.first, 60, release);
        expect(row.state == SA2_PICK_STALE && row.stale == SA2_PICK_STALE_SURFACE && row.instance == 0 && release, "a newer surface generation did not stale the reply");
        const auto at_once = request(1, 0);
        expect(!at_once.second.pass && at_once.second.state == SA2_PICK_STALE && at_once.second.stale == SA2_PICK_STALE_SURFACE, "a stale request ran an ID pass");
        row = poll(at_once.first, 0, release);
        expect(row.state == SA2_PICK_STALE && row.stale == SA2_PICK_STALE_SURFACE && release, "a stale request did not answer at once");
        // RPI.24: a newer view generation.
        const auto view = request(2, 70);
        input.view_generation = 2;
        input.resource_set = 0;
        submit(input, 3);
        row = poll(view.first, 70, release);
        expect(row.state == SA2_PICK_STALE && row.stale == SA2_PICK_STALE_VIEW && row.instance == 0, "a newer view generation did not stale the reply");
        // A frame without a resource set has nothing pickable: ready, instance 0, no ID pass.
        const auto nothing = request(3, 0);
        bool read = false;
        row = product.pick_poll(nothing.first, 0, [&] { read = true; return 77u; }, release);
        expect(!nothing.second.pass && row.state == SA2_PICK_READY && row.instance == 0 && !read && release, "a frame without a set ran an ID pass");
        // A retired set's resources outlive the ID passes that read it.
        product.retire(first.second);
        expect(product.releasable(3, 69).empty(), "a set was released before its last ID pass completed");
        expect(product.releasable(3, 70) == std::vector<uint64_t>{first.first}, "a set was not released after its last ID pass");
        // RPI.25: the frame's set was retired before the request.
        PreparationArrays second_arrays(1, 2, 1);
        const auto second = ready_set(product, sa2::preparation(&second_arrays.value), 0);
        product.activate(second.second, stamp);
        input.resource_set = second.second;
        submit(input, 4);
        product.retire(second.second);
        const auto publication = request(4, 0);
        expect(publication.second.state == SA2_PICK_STALE && publication.second.stale == SA2_PICK_STALE_PUBLICATION, "a retired set did not stale the request");
        // The frame's engine revision left the GPU ring.
        PreparationArrays third_arrays(1, 2, 1);
        const auto third = ready_set(product, sa2::preparation(&third_arrays.value), 0);
        product.activate(third.second, stamp);
        input.resource_set = third.second;
        submit(input, 5);
        input.resource_set = 0;
        for (uint64_t revision = 2; revision <= 4; ++revision) {
            EngineArrays next(revision, 0);
            product.adopt(sa2::engine_image(&next.image));
            input.engine_revision = revision;
            submit(input, 4 + revision);
        }
        const auto engine_gone = request(5, 0);
        expect(engine_gone.second.state == SA2_PICK_STALE && engine_gone.second.stale == SA2_PICK_STALE_ENGINE, "a replaced ring copy did not stale the request");
        // A frame that drew nothing cannot be picked.
        input.resource_set = third.second;
        input.sequence = 9;
        product.commit(input, product.plan(input, 256, 256), 9, 2);
        product.produced(9, 0, 0, SA2_E_D3D12);
        expect(wrong_state([&] { product.pick_plan(9, 0, 0); }), "a frame that drew nothing was picked");
        // A resize makes a pending reply stale, and a later request stale at once.
        submit(input, 10);
        const auto target = request(10, 90);
        expect(target.second.pass && target.second.input.copy == 0, "the ID pass of the newest frame was not planned");
        product.resized();
        row = poll(target.first, 90, release);
        expect(row.state == SA2_PICK_STALE && row.stale == SA2_PICK_STALE_TARGET && row.instance == 0, "a resize did not stale the reply");
        expect(request(10, 0).second.stale == SA2_PICK_STALE_TARGET, "a resize did not stale the request");
        expect(product.counters().id_passes == 4, "the ID passes were not counted");
        // The live pick bound.
        while (product.live_picks() < sa2::MaxPicks) request(10, 0);
        expect(wrong_state([&] { product.pick_plan(10, 0, 0); }), "a pick beyond the bound was accepted");
    });
    suite.check("product device lost", [] {
        sa2::Product product;
        EngineArrays engine(1, 0);
        product.adopt(sa2::engine_image(&engine.image));
        PreparationArrays arrays(1, 1, 1);
        const auto data = sa2::preparation(&arrays.value);
        const auto ready = ready_set(product, data, 0);
        const auto pending = product.reserve_ticket();
        product.prepared(pending, data, 10);
        auto input = frame_input(1, 1);
        product.commit(input, product.plan(input, 256, 256), 1, 0);
        product.produced(1, 100, 3);
        input.sequence = 2;
        product.commit(input, product.plan(input, 256, 256), 2, 1);
        product.observe_removal(0);
        expect(!product.removed(), "S_OK was taken as a removal");
        product.observe_removal(int32_t(0x887A0005)); // DXGI_ERROR_DEVICE_REMOVED
        product.observe_removal(int32_t(0x887A0006)); // the first reason latches
        expect(product.removed() && product.removed_reason() == int32_t(0x887A0005), "the first removal reason did not latch");
        const uint8_t stamp[32]{};
        EngineArrays next(2, 0);
        expect(product.adopt(sa2::engine_image(&next.image)) == 2, "adopt stopped after removal (RM.16)");
        expect(failure([&] { product.check_prepare(data); }) == SA2_E_DEVICE_REMOVED, "prepare worked after removal");
        expect(failure([&] { product.activate(ready.second, stamp); }) == SA2_E_DEVICE_REMOVED, "activate worked after removal");
        input.sequence = 3;
        input.engine_revision = 2;
        expect(failure([&] { product.plan(input, 256, 256); }) == SA2_E_DEVICE_REMOVED, "a frame was planned after removal");
        expect(failure([&] { product.pick_plan(1, 0, 0); }) == SA2_E_DEVICE_REMOVED, "a pick was planned after removal");
        bool release = true;
        expect(failure([&] { product.pick_poll(1, 0, [] { return 0u; }, release); }) == SA2_E_DEVICE_REMOVED && !release, "pick_poll worked after removal");
        // Pending and ready tickets fail; cancel and retire keep working; every set is released.
        auto row = product.poll(pending, 0);
        expect(row.state == SA2_TICKET_FAILED && row.reason == SA2_E_DEVICE_REMOVED && row.resource_set == 0, "a pending ticket did not fail");
        row = product.poll(ready.first, 0);
        expect(row.state == SA2_TICKET_FAILED && row.reason == SA2_E_DEVICE_REMOVED && row.resource_set == 0, "a ready ticket did not fail");
        product.retire(ready.second);
        expect(product.releasable(0, 0).size() == 2 && product.held_tickets() == 0 && product.live_tickets() == 1, "device loss did not release every set");
        expect(product.poll(pending, 0).state == SA2_TICKET_FAILED, "a failed ticket stopped answering");
        // The completed frame reports normally; the unread one reports the removal.
        sa2_frame_report rows[2]{};
        expect(product.reports(rows, 2) == 2 && rows[0].frame_id == 1 && rows[0].error == SA2_OK && rows[0].view_vertex_evals == 3
            && rows[1].frame_id == 2 && rows[1].error == SA2_E_DEVICE_REMOVED && rows[1].view_vertex_evals == 0, "an unread frame did not report the removal");
        expect(product.counter_values().view_vertex_evals_total == 3, "a lost frame counted vertex evaluations");
        expect(sa2::describe("build", product.removed_reason()).at("device_removed").string() == "0x887a0005"
            && sa2::recover_report(product.removed_reason()).at("device_removed").string() == "0x887a0005", "describe or recover does not name the removal");
    });
    suite.check("product describe", [] {
        const auto json = Json::parse(sa2::describe("abc", 0).dump());
        expect(json.at("interface_version").integer() == 8 && json.at("backend_build").string() == "abc" && json.at("device_epoch").integer() == 1
            && json.at("device_removed").null() && json.at("evidence").array().empty(), "the SupportReport header is wrong");
        const auto& features = json.at("features");
        expect(features.object().size() == 8, "the SupportReport does not list eight rows");
        for (const char* row : {"F01", "F02", "F18", "J01", "J02"})
            expect(features.at(row).at("status").string() == "supported" && features.at(row).at("layout_version").integer() == 1, "a supported row is missing");
        for (const char* row : {"F22", "F23", "J13"})
            expect(features.at(row).at("status").string() == "unsupported" && !features.at(row).at("reason").string().empty(), "an unsupported row lacks its reason");
        const auto recover = Json::parse(sa2::recover_report(0).dump());
        expect(recover.at("status").string() == "unavailable" && !recover.at("reason").string().empty() && recover.at("device_removed").null()
            && recover.at("device_epoch").integer() == 1, "the recover report is wrong");
        expect(sa2::removed_text(int32_t(0x887A0005)) == "0x887a0005", "the removal text is not 0x%08x");
        // ABI 5, route B: F22 and F23 on a sa2_create context with a surface.
        const auto route_b = Json::parse(sa2::describe("abc", 0, 3, true).dump());
        expect(route_b.at("interface_version").integer() == 8 && route_b.at("device_epoch").integer() == 3, "the route B header is wrong");
        for (const char* row : {"F22", "F23"})
            expect(route_b.at("features").at(row).at("status").string() == "supported"
                && route_b.at("features").at(row).at("layout_version").integer() == 1, "route B does not support F22 and F23");
        expect(route_b.at("features").at("J13").at("status").string() == "unsupported", "route B changed J13");
    });
    // ABI 6: a view's shape. Its checks, its defaults (ABI 5's constants), per view, recorded with each frame for its
    // picks, and kept across a recovery.
    suite.check("view shape", [] {
        const auto defaults = sa2::view_shape(nullptr);
        expect(defaults.cell_shrink == 0.76f && defaults.sticker_shrink == 0.82f && defaults.d4 == 1.18f && defaults.zoom == 1.15f,
            "the default shape is not ABI 5's constants");
        sa2_view_shape shape{sizeof(sa2_view_shape), 0.7f, 0.55f, 1.18f, 1.15f, 0};
        expect(sa2::view_shape(&shape) == sa2::ViewShape{0.7f, 0.55f, 1.18f, 1.15f}, "a valid shape was not copied");
        const float nan = std::numeric_limits<float>::quiet_NaN(), inf = std::numeric_limits<float>::infinity();
        const auto refused = [&](auto change) { auto bad = shape; change(bad); return rejects([&] { sa2::view_shape(&bad); }); };
        expect(refused([](sa2_view_shape& s) { s.struct_size = 20; }) && refused([](sa2_view_shape& s) { s.reserved = 1; }),
            "a wrong struct_size or a nonzero reserved was accepted");
        expect(refused([&](sa2_view_shape& s) { s.cell_shrink = nan; }) && refused([&](sa2_view_shape& s) { s.sticker_shrink = inf; })
            && refused([&](sa2_view_shape& s) { s.d4 = nan; }) && refused([&](sa2_view_shape& s) { s.zoom = -inf; }),
            "a nonfinite value was accepted");
        expect(refused([](sa2_view_shape& s) { s.cell_shrink = 0; }) && refused([](sa2_view_shape& s) { s.cell_shrink = 1.01f; })
            && refused([](sa2_view_shape& s) { s.sticker_shrink = -0.5f; }) && refused([](sa2_view_shape& s) { s.sticker_shrink = 1.5f; })
            && refused([](sa2_view_shape& s) { s.d4 = 1.0f; }) && refused([](sa2_view_shape& s) { s.d4 = SA2_SHAPE_D4_MAX * 2; })
            && refused([](sa2_view_shape& s) { s.zoom = 0; }) && refused([](sa2_view_shape& s) { s.zoom = SA2_SHAPE_ZOOM_MAX * 2; }),
            "a value out of its range was accepted");
        shape.cell_shrink = 1; shape.sticker_shrink = 1; shape.d4 = SA2_SHAPE_D4_MAX; shape.zoom = SA2_SHAPE_ZOOM_MAX;
        sa2::view_shape(&shape); // every upper bound is inside its range
        sa2::Product product;
        EngineArrays engine(1, 0);
        product.adopt(sa2::engine_image(&engine.image));
        const sa2::ViewShape reduced{0.7f, 0.55f, 1.18f, 1.15f};
        product.set_shape(2, reduced);
        expect(product.shape(1) == sa2::ViewShape{} && product.shape(2) == reduced, "a shape is not per view");
        auto input = frame_input(1, 1);
        input.view = 2;
        auto plan = product.plan(input, 256, 256);
        expect(plan.shape == reduced, "a frame does not draw with its view's shape");
        product.commit(input, plan, 1, 0);
        product.produced(1, 100, 1);
        product.set_shape(2, sa2::ViewShape{});
        expect(product.shape(2) == sa2::ViewShape{}, "the defaults were not restored");
        const auto pick = product.pick_plan(1, 10, 20);
        expect(pick.state == SA2_PICK_READY && !pick.pass, "a frame without a set needs no ID pass");
        input.sequence = 2;
        expect(product.plan(input, 256, 256).shape == sa2::ViewShape{}, "a restored shape was not drawn");
        // A pick draws with the shape of its frame, not the view's current one.
        PreparationArrays arrays(2, 1, 1);
        arrays.value.view = 2;
        const auto readied = ready_set(product, sa2::preparation(&arrays.value), 0);
        const uint8_t stamp[32]{};
        product.activate(readied.second, stamp);
        product.set_shape(2, reduced);
        input.resource_set = readied.second;
        plan = product.plan(input, 256, 256);
        product.commit(input, plan, 2, 1);
        product.produced(2, 100, 1);
        product.set_shape(2, sa2::ViewShape{});
        const auto pass = product.pick_plan(2, 10, 20);
        expect(pass.pass && pass.input.shape == reduced, "an ID pass does not use its frame's shape");
        // Device loss and recovery keep it.
        product.set_shape(3, reduced);
        product.observe_removal(int32_t(0x887A0005));
        product.set_shape(4, reduced);
        product.recovered(sa2::DeviceEpoch + 1);
        expect(product.shape(3) == reduced && product.shape(4) == reduced, "a shape was lost across device loss and recovery");
    });
    // ABI 7 (ADR-D02 B14): the piece style and the style slots, without a device.
    suite.check("piece style", [] {
        static_assert(SA2_STYLE_SLOTS == 4u && SA2_STYLE_PALETTE == 16u && SA2_STYLE_INDEX == 0x0Fu && SA2_STYLE_DIM == 0x80u,
            "the ABI 7 defines changed");
        sa2_view_style style{};
        style.struct_size = sizeof(style);
        style.tint_mode = SA2_STYLE_TINT_FILL | SA2_STYLE_TINT_OUTLINE;
        style.outline_width = 0.1f;
        for (int i = 0; i < 64; ++i) style.palette[i] = (i % 4 == 3) ? 1.0f : 0.5f;
        const float identity[9] = {1, 0, 0, 0, 1, 0, 0, 0, 1};
        std::copy(std::begin(identity), std::end(identity), std::begin(style.matrix));
        const auto copied = sa2::view_style(&style);
        expect(copied.matrix_on == 0 && copied.rows[0] == 1 && copied.rows[5] == 1 && copied.rows[10] == 1 && copied.tint_mode == 3,
            "a valid style was not copied");
        const float nan = std::numeric_limits<float>::quiet_NaN();
        const auto refused = [&](auto change) { auto bad = style; change(bad); return rejects([&] { sa2::view_style(&bad); }); };
        expect(refused([](sa2_view_style& s) { s.struct_size = 312; }) && refused([](sa2_view_style& s) { s.reserved = 1; })
            && refused([](sa2_view_style& s) { s.tint_mode = 4; }) && refused([](sa2_view_style& s) { s.dim_mode = 3; })
            && refused([](sa2_view_style& s) { s.dim_amount = 1.5f; }) && refused([&](sa2_view_style& s) { s.dim_amount = nan; })
            && refused([](sa2_view_style& s) { s.outline_width = 0.6f; }) && refused([](sa2_view_style& s) { s.palette[7] = 1.5f; })
            && refused([&](sa2_view_style& s) { s.palette[0] = nan; }) && refused([](sa2_view_style& s) { s.matrix[4] = 9.0f; })
            && rejects([] { sa2::view_style(nullptr); }), "an invalid style was accepted");
        std::vector<uint8_t> bytes(SA2_PIECES, 0);
        expect(rejects([&] { sa2::validate_piece_style(bytes.data(), 2); }), "a third channel was accepted");
        bytes[5] = 0x10;
        expect(rejects([&] { sa2::validate_piece_style(bytes.data(), SA2_STYLE_CHANNEL_FILL); }), "an unknown fill bit was accepted");
        bytes[5] = 0x80;
        expect(rejects([&] { sa2::validate_piece_style(bytes.data(), SA2_STYLE_CHANNEL_OUTLINE); }), "a dim bit on the outline was accepted");
        sa2::validate_piece_style(bytes.data(), SA2_STYLE_CHANNEL_FILL);
        sa2::Product product;
        expect(wrong_state([&] { product.set_style(1, bytes.data(), 0); }), "a style was set without an adopted revision");
        EngineArrays engine(1, 0);
        product.adopt(sa2::engine_image(&engine.image));
        auto input = frame_input(1, 1);
        auto plan = product.plan(input, 256, 256);
        expect(!plan.style && plan.style_version == 0 && !plan.styled, "an unstyled frame plans a style");
        // A slot alone, with the identity and nothing to tint, changes nothing: the frame stays on the product pipeline.
        product.set_view_style(1, &style);
        expect(!product.plan(input, 256, 256).styled, "a style that changes nothing draws styled");
        expect(rejects([&] { product.set_view_style(0, &style); }) && rejects([&] { product.set_view_style(5, &style); }),
            "a style slot out of 1..4 was accepted");
        // Another revision is refused; the adopted one is kept per channel.
        std::fill(bytes.begin(), bytes.end(), uint8_t(0));
        bytes[7] = 3; bytes[9] = 0x81;
        expect(rejects([&] { product.set_style(2, bytes.data(), 0); }), "a style for another revision was accepted");
        product.set_style(1, bytes.data(), SA2_STYLE_CHANNEL_FILL);
        const auto* piece = product.piece_style();
        expect(piece && piece->revision == 1 && piece->version != 0 && piece->packed[7] == 3 && piece->packed[9] == 0x81
            && piece->any[0] && !piece->any[1] && piece->dim, "the fill channel was not kept");
        std::vector<uint8_t> lines(SA2_PIECES, 0);
        lines[9] = 2;
        product.set_style(1, lines.data(), SA2_STYLE_CHANNEL_OUTLINE);
        expect(piece->packed[9] == (0x81u | (2u << 8)) && piece->any[1], "the outline channel did not join the fill channel");
        input.sequence = 2;
        plan = product.plan(input, 256, 256);
        expect(plan.style && plan.style_version == piece->version && plan.styled && plan.upload,
            "a styled frame does not upload its style with the copy");
        product.commit(input, plan, 1, 0);
        input.sequence = 3;
        expect(!product.plan(input, 256, 256).upload, "an unchanged style uploaded again");
        // Clearing both channels leaves no style; an empty mask is no style.
        product.set_style(1, nullptr, SA2_STYLE_CHANNEL_FILL);
        product.set_style(1, nullptr, SA2_STYLE_CHANNEL_OUTLINE);
        expect(product.piece_style() == nullptr && !product.plan(input, 256, 256).styled, "cleared channels left a style");
        std::fill(bytes.begin(), bytes.end(), uint8_t(0));
        product.set_style(1, bytes.data(), SA2_STYLE_CHANNEL_FILL);
        expect(product.piece_style() == nullptr, "an empty mask was kept as a style");
        // A matrix other than the identity, or a dim of every unmarked piece, draws styled without a piece style.
        style.matrix[1] = 0.25f;
        product.set_view_style(1, &style);
        expect(product.plan(input, 256, 256).styled, "a colour matrix did not draw styled");
        style.matrix[1] = 0;
        style.dim_mode = SA2_STYLE_DIM_UNMARKED; style.dim_amount = 0.5f;
        product.set_view_style(1, &style);
        expect(product.plan(input, 256, 256).styled, "a focus dim did not draw styled");
        product.set_view_style(1, nullptr);
        expect(!product.plan(input, 256, 256).styled, "a cleared slot draws styled");
        // Device loss and recovery keep both; an adoption drops the piece style.
        product.set_view_style(1, &style);
        bytes[11] = 4;
        product.set_style(1, bytes.data(), SA2_STYLE_CHANNEL_FILL);
        product.observe_removal(int32_t(0x887A0005));
        product.recovered(sa2::DeviceEpoch + 1);
        input.device_epoch = sa2::DeviceEpoch + 1;
        expect(product.piece_style() != nullptr && product.plan(input, 256, 256).styled, "a style was lost across device loss and recovery");
        EngineArrays next(2, 0);
        product.adopt(sa2::engine_image(&next.image));
        expect(product.piece_style() == nullptr, "an adoption kept the previous revision's piece style");
    });
    // ABI 8 (ADR-D02 B18a): the composite frame's checks, its pane rows and a pick that resolves its pane.
    suite.check("pane arguments", [] {
        static_assert(SA2_PANES_MAX == 4u, "the ABI 8 defines changed");
        sa2::Product product;
        EngineArrays engine(1, 0);
        product.adopt(sa2::engine_image(&engine.image));
        auto input = frame_input(1, 1);
        const auto pane = [&](uint64_t view, int32_t x, int32_t y, uint32_t w, uint32_t h) {
            sa2_pane out{};
            out.view = view; out.view_generation = 1; out.x = x; out.y = y; out.width = w; out.height = h;
            std::copy(std::begin(input.camera_matrix), std::end(input.camera_matrix), std::begin(out.camera_matrix));
            return out;
        };
        sa2_pane two[2] = {pane(1, 0, 0, 128, 256), pane(2, 128, 0, 128, 256)};
        auto plan = product.plan_panes(input, two, 2, 256, 256);
        expect(plan.panes.size() == 2 && plan.panes[1].pane.view == 2 && !plan.panes[0].styled && !plan.panes[1].set_ticket,
            "two panes were not planned");
        const auto refused = [&](auto change, uint32_t count = 2) {
            sa2_pane copy[2] = {two[0], two[1]};
            auto frame = input;
            change(copy, frame);
            return rejects([&] { product.plan_panes(frame, copy, count, 256, 256); });
        };
        expect(refused([](sa2_pane*, sa2_frame_input&) {}, 0) && rejects([&] { product.plan_panes(input, nullptr, 1, 256, 256); }),
            "a composite frame without panes was accepted");
        sa2_pane five[5] = {pane(1, 0, 0, 32, 32), pane(2, 32, 0, 32, 32), pane(3, 64, 0, 32, 32), pane(4, 96, 0, 32, 32), pane(5, 128, 0, 32, 32)};
        expect(rejects([&] { product.plan_panes(input, five, 5, 256, 256); }), "a fifth pane was accepted");
        expect(refused([](sa2_pane* p, sa2_frame_input&) { p[1].x = 127; }), "overlapping panes were accepted");
        expect(refused([](sa2_pane* p, sa2_frame_input&) { p[1].x = 200; }) && refused([](sa2_pane* p, sa2_frame_input&) { p[0].y = -1; })
            && refused([](sa2_pane* p, sa2_frame_input&) { p[1].width = 0; }), "a pane outside the target or empty was accepted");
        expect(refused([](sa2_pane* p, sa2_frame_input&) { p[1].flags = 1; }) && refused([](sa2_pane* p, sa2_frame_input&) { p[1].style = 5; })
            && refused([](sa2_pane* p, sa2_frame_input&) { p[1].camera_matrix[3] = std::numeric_limits<float>::infinity(); }),
            "a pane's flags, style or camera was accepted out of range");
        expect(refused([](sa2_pane*, sa2_frame_input& f) { f.view = 2; }) && refused([](sa2_pane*, sa2_frame_input& f) { f.camera_matrix[1] = 0.5f; }),
            "a frame input that is not the first pane's was accepted");
        // A pane's set prepared at another engine revision refuses the whole frame.
        PreparationArrays arrays(2, 1, 1);
        arrays.value.base_revision = 2;
        const auto readied = ready_set(product, sa2::preparation(&arrays.value), 0);
        const uint8_t stamp[32]{};
        product.activate(readied.second, stamp);
        two[1].resource_set = readied.second;
        expect(wrong_state([&] { product.plan_panes(input, two, 2, 256, 256); }), "a pane of another engine revision was accepted");
        PreparationArrays current(2, 1, 1);
        current.value.base_revision = 1;
        const auto good = ready_set(product, sa2::preparation(&current.value), 0);
        product.activate(good.second, stamp);
        two[1].resource_set = good.second;
        plan = product.plan_panes(input, two, 2, 256, 256);
        expect(plan.panes[1].set_ticket == good.first, "a pane's set was not planned");
        product.commit(input, plan, 1, 0);
        expect(wrong_state([&] { product.pane_rows(1, nullptr, 0); }), "pane rows were answered before the frame produced");
        product.produced(1, 100, 7);
        product.pane_evals(1, {3, 4});
        sa2_pane_row rows[4]{};
        for (auto& row : rows) row.struct_size = sizeof(row);
        expect(product.pane_rows(1, rows, 4) == 2 && rows[0].view == 1 && rows[1].view == 2 && rows[0].vertex_evals == 3
            && rows[1].vertex_evals == 4 && rows[1].pane == 1, "the pane rows are wrong");
        // A pick resolves its pane: the right pane has a set (an ID pass), the left none, a point outside is refused.
        auto pick = product.pick_plan(1, 200, 10);
        expect(pick.pane == 1 && pick.pass && pick.input.origin_x == 128 && pick.input.view_width == 128 && pick.input.width == 256,
            "a pick in the right pane did not plan that pane's ID pass");
        pick = product.pick_plan(1, 10, 10);
        expect(pick.pane == 0 && !pick.pass && pick.state == SA2_PICK_READY, "a pick in the left pane did not resolve it");
        product.pick_commit(9, pick, 0);
        bool release = false;
        const auto row = product.pick_poll(9, 0, [] { return 0u; }, release);
        expect(row.view == 1 && row.state == SA2_PICK_READY, "a pane pick did not name its view");
        input.sequence = 2;
        auto one = product.plan(input, 256, 256);
        product.commit(input, one, 2, 1);
        product.produced(2, 100, 7);
        expect(wrong_state([&] { product.pane_rows(2, rows, 4); }), "a one-view frame answered pane rows");
    });
    // ABI 5, route B (accepted for now by the owner on 18 Nov 2026, pending
    // three owner-machine checks): the argument checks, the present ledger's
    // exact rule and the recovery bookkeeping, without a device or a window.
    suite.check("route b arguments", [] {
        static_assert(SA2_TARGET_SURFACE == 0xFFFFFFFFu && SA2_SURFACE_SYNC_INTERVAL == 1u && SA2_SURFACE_BUFFERS == SA2_RING_SLOTS
            && SA2_SURFACE_MAX_PX == 16384u, "the ABI 5 defines changed");
        sa2_config config{};
        auto* ctx = reinterpret_cast<sa2_context*>(uintptr_t(1));
        invalid(sa2_create(0, nullptr, &ctx), "sa2_create");
        expect(ctx == nullptr, "a refused sa2_create left its output set");
        invalid(sa2_create(0, &config, &ctx), "sa2_create");
        config.struct_size = sizeof(config);
        config.queue_mode = 7;
        invalid(sa2_create(0, &config, &ctx), "sa2_create");
        config.queue_mode = SA2_QUEUE_SAME;
        config.debug_callback = 2;
        invalid(sa2_create(0, &config, &ctx), "sa2_create");
        config.debug_callback = 0;
        invalid(sa2_create(0, &config, nullptr), "sa2_create");
        invalid(sa2_surface_attach(nullptr, 1, 0, 0, 256, 256), "sa2_surface_attach");
        invalid(sa2_surface_move(nullptr, 0, 0), "sa2_surface_move");
        invalid(sa2_surface_detach(nullptr), "sa2_surface_detach");
        invalid(sa2_scene_produce(nullptr, SA2_TARGET_SURFACE, 1), "sa2_scene_produce"); // C9 U4: the surface scene path
        uint64_t chain = 1;
        invalid(sa2_surface_swap_chain(nullptr, &chain), "sa2_surface_swap_chain");
        expect(chain == 0, "sa2_surface_swap_chain left its output set on failure");
        auto frame = frame_input(1, 1);
        frame.target = SA2_TARGET_SURFACE;
        sa2::validate_frame_input(&frame);
        expect(rejects([&] { auto bad = frame; bad.target = SA2_TARGET_SURFACE - 1; sa2::validate_frame_input(&bad); }), "a target between the ring and the surface was accepted");
    });
    suite.check("present matching", [] {
        using sa2::PresentMatch;
        constexpr int32_t disjoint = int32_t(0x887A000B); // DXGI_ERROR_FRAME_STATISTICS_DISJOINT
        const auto match = [](uint64_t generation, uint32_t count, sa2::PresentSample sample) { return sa2::present_match(generation, count, sample); };
        expect(match(1, 5, {0, 1, 5, 5}) == PresentMatch::Exact && match(1, 5, {0, 1, 5, 9}) == PresentMatch::Exact, "an equal count was not exact");
        expect(match(1, 5, {0, 1, 6, 9}) == PresentMatch::Superseded, "a row past the frame's count did not drop it");
        expect(match(1, 5, {0, 1, 4, 9}) == PresentMatch::Wait, "a row before the frame's count did not wait");
        expect(match(1, 5, {0, 2, 1, 1}) == PresentMatch::Superseded, "a newer swap-chain generation did not drop the frame");
        expect(match(2, 5, {0, 1, 5, 5}) == PresentMatch::Ignore, "an older generation's row was used");
        expect(match(1, 5, {disjoint, 1, 5, 5}) == PresentMatch::Ignore, "a disjoint row was used");
        expect(match(1, 5, {0, 1, 5, 4}) == PresentMatch::Ignore && match(1, 5, {0, 1, 6, 5}) == PresentMatch::Ignore,
            "a row above GetLastPresentCount was used");
        sa2::PresentLedger ledger;
        auto input = frame_input(1, 1);
        ledger.presented(input, 1, 1, 10);
        input.sequence = 2;
        input.flags = SA2_FRAME_PREVIEW;
        input.preview = 4;
        ledger.presented(input, 2, 1, 11);
        input.sequence = 3;
        input.flags = 0;
        input.preview = 0;
        ledger.presented(input, 3, 1, 12);
        sa2_presented_row rows[4]{};
        ledger.sample({0, 1, 9, 12});
        ledger.sample({disjoint, 1, 11, 12});
        expect(ledger.awaiting() == 3 && ledger.rows(rows, 4) == 0, "an earlier or disjoint row changed the ledger");
        ledger.sample({0, 1, 11, 12, 7000});
        ledger.sample({0, 1, 11, 12, 7100});
        expect(ledger.awaiting() == 1 && ledger.confirmed() == 1, "a row did not confirm its frame once and drop the passed one");
        expect(ledger.rows(rows, 4) == 1 && rows[0].struct_size == sizeof(rows[0]) && rows[0].frame_id == 2 && rows[0].previews == 1
            && rows[0].preview == 4 && rows[0].sequence == 2 && rows[0].surface == 1 && rows[0].device_epoch == sa2::DeviceEpoch,
            "the confirmed row does not carry its frame's input");
        expect(rows[0].display_qpc == 7000, "the confirmed row does not carry the confirming row's SyncQPCTime");
        expect(ledger.rows(rows, 4) == 0, "a confirmed frame was reported twice");
        ledger.sample({0, 1, 12, 12, 7200});
        expect(ledger.rows(rows, 4) == 1 && rows[0].frame_id == 3 && rows[0].previews == 0 && rows[0].preview == 0
            && rows[0].display_qpc == 7200, "frame 3 was not confirmed");
        // A resize or recovery ends the generation: its waiting frames are never reported.
        ledger.presented(input, 4, 1, 13);
        ledger.sample({0, 2, 1, 1});
        expect(ledger.awaiting() == 0 && ledger.confirmed() == 0, "an ended generation's frame was kept");
        // A present count that does not increase makes both frames ambiguous.
        ledger.presented(input, 5, 2, 3);
        ledger.presented(input, 6, 2, 3);
        ledger.sample({0, 2, 3, 3});
        expect(ledger.awaiting() == 0 && ledger.confirmed() == 0, "frames with one present count were confirmed");
        // Frame order, capacity 1.
        ledger.presented(input, 7, 3, 1);
        ledger.sample({0, 3, 1, 1});
        ledger.presented(input, 8, 3, 2);
        ledger.sample({0, 3, 2, 2});
        expect(ledger.rows(rows, 1) == 1 && rows[0].frame_id == 7 && ledger.rows(rows, 1) == 1 && rows[0].frame_id == 8, "confirmed frames left frame order");
        // Both lists are bounded; the oldest entries go first and are never reported.
        for (uint32_t i = 0; i < sa2::MaxPresents + 5; ++i) ledger.presented(input, 100 + i, 4, 1 + i);
        expect(ledger.awaiting() == sa2::MaxPresents, "the awaiting frames are unbounded");
        for (uint32_t i = 0; i < sa2::MaxPresents + 5; ++i) {
            ledger.presented(input, 5000 + i, 5, 1 + i);
            ledger.sample({0, 5, 1 + i, 1 + i});
        }
        expect(ledger.confirmed() == sa2::MaxPresents && ledger.awaiting() == 0, "the confirmed frames are unbounded");
        expect(ledger.rows(rows, 1) == 1 && rows[0].frame_id == 5005, "the oldest confirmed frame was not dropped first");
        ledger.clear();
        expect(ledger.awaiting() == 0 && ledger.confirmed() == 0 && ledger.rows(rows, 4) == 0, "clear left an entry");
        // C9, owner item 9: route B's traced scene frames, by trace index, under the same rule; presented.jsonl.
        sa2::TracePresents trace;
        trace.presented(0, 1, 20);
        trace.presented(1, 1, 21);
        trace.presented(2, 1, 22);
        trace.sample({0, 1, 19, 22, 100});
        trace.sample({disjoint, 1, 20, 22, 150});
        expect(trace.awaiting() == 3 && trace.confirmed() == 0 && trace.text().empty(), "an earlier or disjoint row confirmed a traced frame");
        trace.sample({0, 1, 20, 22, 200});
        trace.sample({0, 1, 22, 22, 300});
        expect(trace.awaiting() == 0 && trace.confirmed() == 2, "a row did not confirm its traced frame or drop the passed one");
        expect(trace.text() == "{\"display_qpc\":200,\"frame\":0,\"present_count\":20}\n{\"display_qpc\":300,\"frame\":2,\"present_count\":22}\n",
            "presented.jsonl rows differ");
    });
    suite.check("preparation reuse", [] {
        // C9 (26 Nov 2026): sa2::Spares, the released preparation resources a twist's preparation takes before it
        // creates any. Which tickets are released, and when, is Product::releasable's ("product tickets").
        sa2::Spares<std::unique_ptr<int>> spares;
        expect(!spares.take() && spares.size() == 0, "an empty pool gave a set");
        expect(spares.keep(std::make_unique<int>(1), false) && spares.keep(std::make_unique<int>(2), false)
            && spares.size() == sa2::MaxSpareSets, "a released set of a live device was not kept");
        auto third = std::make_unique<int>(3);
        expect(!spares.keep(std::move(third), false) && third && *third == 3 && spares.size() == sa2::MaxSpareSets,
            "more than MaxSpareSets were kept, or a refused set was taken from its owner");
        auto taken = spares.take();
        expect(taken && *taken && **taken == 2 && spares.size() == 1, "the newest kept set was not taken first");
        auto lost = std::make_unique<int>(4);
        expect(!spares.keep(std::move(lost), true) && lost && spares.size() == 1, "a set of a removed device was kept");
        expect(spares.keep(std::move(*taken), false) && spares.size() == 2, "a set taken and released again was not kept");
        spares.clear();
        expect(spares.size() == 0 && !spares.take(), "a removal left a kept set");
    });
    suite.check("latency wait", [] {
        // C9 (26 Nov 2026): sa2::latency_wait over a scripted wait that records each part's milliseconds.
        static constexpr uint32_t timed_out = 0x102, failed = 0xFFFFFFFFu, signalled = 0;
        struct Script { std::vector<uint32_t> asked; int samples = 0; };
        const auto run = [](uint32_t timeout_ms, uint32_t after, std::vector<uint32_t> answers, Script& s) {
            size_t next = 0;
            return sa2::latency_wait(timeout_ms, after, timed_out,
                [&](uint32_t ms) { s.asked.push_back(ms); return answers.at(next++); }, [&] { ++s.samples; });
        };
        using Parts = std::vector<uint32_t>;
        Script a, b, c, d, e, f, g;
        expect(run(1000, 5, {signalled}, a) == signalled && a.asked == Parts{5} && a.samples == 0,
            "a wait that ended in its first part took a sample");
        expect(run(1000, 5, {timed_out, signalled}, b) == signalled && b.asked == Parts{5, 995} && b.samples == 1,
            "a long wait did not take one sample and wait for the rest");
        expect(run(1000, 5, {timed_out, timed_out}, c) == timed_out && c.asked == Parts{5, 995} && c.samples == 1,
            "a timed-out wait did not keep its whole timeout");
        expect(run(1000, 5, {failed}, d) == failed && d.asked == Parts{5} && d.samples == 0,
            "a failed wait took a sample or waited again");
        expect(run(5, 5, {timed_out}, e) == timed_out && e.asked == Parts{5} && e.samples == 0,
            "a timeout not above the sample point was split");
        expect(run(UINT32_MAX, 5, {timed_out, signalled}, f) == signalled && f.asked == Parts{5, UINT32_MAX} && f.samples == 1,
            "an infinite timeout became finite");
        expect(run(1000, 0, {signalled}, g) == signalled && g.asked == Parts{1000} && g.samples == 0,
            "a zero sample point split the wait");
        // The race the sample answers (owner-machine sitting of 25 Nov 2026): a 60 Hz loop presents frame k (count
        // k) 0.3 ms after vblank k, which shows it from vblank k + 2, and samples 0.5 ms after the vblank, when DXGI
        // has posted that vblank's statistics for one vblank in three (at 0.2 ms) and not yet for the others (at
        // 0.9 ms). Alone, that sample passes every third count and the exact rule drops a frame that was shown; with
        // the sample inside the next frame's wait, 6.5 ms after the vblank, every shown frame is confirmed.
        const auto confirmed = [](bool in_wait) {
            constexpr int64_t period = 16667, presents_at = 300, sampled_at = 500, waited_at = 6500; // microseconds
            constexpr uint32_t frames = 600;
            const auto posted = [](uint32_t vblank) -> int64_t { return vblank % 3 == 0 ? 200 : 900; };
            const auto sample_at = [&](int64_t t) {
                uint32_t shown = 0, last = 0;
                for (uint32_t count = 1; count <= frames; ++count) {
                    if (int64_t(count + 2) * period + posted(count + 2) <= t) shown = count;
                    if (int64_t(count) * period + presents_at <= t) last = count;
                }
                return sa2::PresentSample{0, 1, shown, last, int64_t(shown + 2) * period};
            };
            sa2::PresentLedger ledger;
            const auto input = frame_input(1, 1);
            sa2_presented_row rows[8]{};
            uint32_t total = 0, previous = 0;
            bool ordered = true;
            for (uint32_t k = 1; k <= frames; ++k) {
                if (in_wait && k > 1) ledger.sample(sample_at(int64_t(k - 1) * period + waited_at));
                ledger.presented(input, k, 1, k);
                ledger.sample(sample_at(int64_t(k) * period + sampled_at));
                for (uint32_t n; (n = ledger.rows(rows, 8)) != 0; total += n)
                    for (uint32_t i = 0; i < n; ++i) {
                        ordered = ordered && rows[i].frame_id > previous
                            && rows[i].display_qpc == uint64_t(rows[i].frame_id + 2) * period;
                        previous = uint32_t(rows[i].frame_id);
                    }
            }
            return ordered ? total : 0;
        };
        expect(confirmed(false) == 399, "the simulated race did not drop every third shown frame");
        expect(confirmed(true) == 598, "with the sample inside the wait, a shown frame was not confirmed at its vblank");
    });
    suite.check("recovery bookkeeping", [] {
        sa2::Product product;
        EngineArrays engine(1, 0);
        product.adopt(sa2::engine_image(&engine.image));
        expect(wrong_state([&] { product.recovered(sa2::DeviceEpoch + 1); }), "a live device was recovered");
        PreparationArrays arrays(1, 1, 1);
        const auto data = sa2::preparation(&arrays.value);
        const auto ready = ready_set(product, data, 0);
        const uint8_t stamp[32]{};
        product.activate(ready.second, stamp);
        auto input = frame_input(1, 1);
        input.resource_set = ready.second;
        product.commit(input, product.plan(input, 256, 256), 1, 0);
        product.produced(1, 100, 3);
        input.sequence = 2;
        product.commit(input, product.plan(input, 256, 256), 2, 1);
        const auto plan = product.pick_plan(1, 0, 0);
        const auto pick = product.reserve_pick();
        product.pick_commit(pick, plan, plan.pass ? 5 : 0);
        product.presents().presented(input, 2, 1, 7);
        const auto generation = product.target_generation();
        product.observe_removal(int32_t(0x887A0005));
        expect(wrong_state([&] { product.recovered(sa2::DeviceEpoch); }), "a recovery kept the device epoch");
        product.recovered(sa2::DeviceEpoch + 1);
        expect(!product.removed() && product.removed_reason() == 0 && product.device_epoch() == sa2::DeviceEpoch + 1, "the recovery did not clear the removal");
        expect(product.image() != nullptr && product.image()->revision == 1, "the adopted CPU image was not kept (RM.16)");
        expect(product.live_tickets() == 0 && product.retained_frames() == 0 && product.live_picks() == 0 && product.presents().awaiting() == 0,
            "the old epoch's tickets, frames, picks or presents were kept");
        expect(product.target_generation() == generation + 1, "the recovery did not start a target generation");
        sa2_frame_report reports[2]{};
        expect(product.counter_values().reports_dropped == 2 && product.reports(reports, 2) == 0, "the old epoch's unread reports were not dropped");
        // The old epoch's tickets, sets, frames and picks answer SA2_E_WRONG_STATE.
        bool release = true;
        expect(wrong_state([&] { product.poll(ready.first, 0); }), "an old ticket was polled");
        expect(wrong_state([&] { product.cancel(ready.first, SA2_CANCEL_BY_USER, 0); }), "an old ticket was cancelled");
        expect(wrong_state([&] { product.activate(ready.second, stamp); }), "an old set was activated");
        expect(wrong_state([&] { product.retire(ready.second); }), "an old set was retired");
        expect(wrong_state([&] { product.pick_plan(1, 0, 0); }), "an old frame was picked");
        expect(wrong_state([&] { product.pick_poll(pick, 5, [] { return 1u; }, release); }), "an old pick was polled");
        // Work naming the old epoch is refused; the new epoch's work runs and its numbers keep counting.
        input.sequence = 3;
        input.resource_set = 0;
        expect(wrong_state([&] { product.plan(input, 256, 256); }), "a frame of the old epoch was planned");
        expect(wrong_state([&] { product.check_prepare(data); }), "a preparation of the old epoch was accepted");
        input.device_epoch = sa2::DeviceEpoch + 1;
        expect(product.plan(input, 256, 256).upload, "the kept image was not uploaded to the new device");
        arrays.value.device_epoch = sa2::DeviceEpoch + 1;
        const auto again = ready_set(product, sa2::preparation(&arrays.value), 0);
        expect(again.first > ready.first && again.second > ready.second && product.reserve_pick() > pick, "a ticket, set or pick number was reused");
        // The answers.
        const auto recovered = Json::parse(sa2::recover_result("recovered", "r", 2, int32_t(0x887A0005)).dump());
        expect(recovered.at("status").string() == "recovered" && recovered.at("reason").string() == "r" && recovered.at("device_epoch").integer() == 2
            && recovered.at("device_removed").string() == "0x887a0005", "the recovered answer is wrong");
        const auto live = Json::parse(sa2::recover_result("not_removed", "n", 1, 0).dump());
        expect(live.at("status").string() == "not_removed" && live.at("device_removed").null() && live.at("device_epoch").integer() == 1,
            "the not_removed answer is wrong");
    });
    // C9 U5: the frozen compiler profile, the composed pass source, the stage text's rules and the compile record.
    suite.check("stage arguments", [] {
        namespace st = sa2::stages;
        const auto names = [](uint32_t kind) {
            std::vector<std::string> out;
            for (const auto& pass : st::passes(kind)) out.push_back(std::string(pass.entry) + " " + pass.profile);
            return out;
        };
        expect(names(SA2_STAGE_VERTEX) == std::vector<std::string>{"m600_eval_stage cs_6_2"}, "the vertex stage's passes changed");
        expect(names(SA2_STAGE_FRAGMENT) == std::vector<std::string>{"m600_fragment_ps ps_6_2"}, "the fragment stage's passes changed");
        expect(names(st::CoreKind) == std::vector<std::string>{"m600_eval_builtin cs_6_2", "m600_bound cs_6_2", "m600_decide cs_6_2",
            "m600_pass_vs vs_6_2", "psMain ps_6_2"}, "the core's passes changed");
        bool unknown = false;
        try { st::passes(3); } catch (const std::invalid_argument&) { unknown = true; }
        expect(unknown, "an unknown stage kind has passes");
        for (const uint32_t kind : {uint32_t(SA2_STAGE_VERTEX), uint32_t(SA2_STAGE_FRAGMENT), st::CoreKind})
            for (const auto& pass : st::passes(kind))
                expect(st::arguments(pass) == std::vector<std::string>{"-T", pass.profile, "-E", pass.entry, "-HV", "2021", "-Gis",
                    "-denorm", "preserve", "-O3", "-Ges", "-WX"}, "the compiler arguments are not the frozen profile");
        expect(std::string(st::kind_name(SA2_STAGE_VERTEX)) == "vertex" && std::string(st::kind_name(SA2_STAGE_FRAGMENT)) == "fragment"
            && std::string(st::kind_name(st::CoreKind)) == "core", "the kind names changed");
        // The stage text's rules.
        const std::string good = "float3 m600_fragment(float c, float s, float x, float y, float z, float t) { return x; } // \xC3\xA9";
        expect(st::text_refusal(good.data(), uint32_t(good.size())).empty(), "UTF-8 stage text was refused");
        expect(!st::text_refusal(nullptr, 1).empty() && !st::text_refusal(good.data(), 0).empty(), "null or empty text was accepted");
        expect(!st::text_refusal(good.data(), SA2_STAGE_HLSL_MAX_BYTES + 1).empty(), "text past SA2_STAGE_HLSL_MAX_BYTES was accepted");
        for (const std::string bad : {std::string("a\0b", 3), std::string("\x80"), std::string("\xC0\x80"), std::string("\xED\xA0\x80"),
                                      std::string("\xF4\x90\x80\x80"), std::string("\xE2\x82")})
            expect(!st::text_refusal(bad.data(), uint32_t(bad.size())).empty(), "a NUL or ill-formed UTF-8 was accepted");
        expect(st::utf8_clean("a\xC0" "b\xE2\x82\xAC") == "a?b\xE2\x82\xAC", "utf8_clean kept an ill-formed byte");
        expect(st::names_identifier("StructuredBuffer<float> m600_vertex_arrays : register(t20);", "m600_vertex_arrays")
            && !st::names_identifier("m600_vertex_arrays2 + xm600_vertex_arrays", "m600_vertex_arrays"), "whole identifiers are misread");
        // The composed source: the define, the stage, its guards, then the core's files after #line markers.
        const st::CoreSources core{"G\n", "// draw\n#include \"geometry.hlsl\"\nD", "S"};
        const std::string stage = "uint m600_vertex() { return m600_vertex_arrays[0]; }";
        expect(st::compose(SA2_STAGE_VERTEX, stage, core) == "#define M600_STAGE_VERTEX 1\n#line 1 \"stage.hlsl\"\n" + stage
            + "\n#ifndef M600_VERTEX_ARRAYS_DECLARED\n#define M600_VERTEX_ARRAYS_DECLARED 1\n#endif\n#line 1 \"geometry.hlsl\"\nG\n"
              "#line 1 \"draw.hlsl\"\n// draw\n\nD\n#line 1 \"stages.hlsl\"\nS\n", "the composed vertex source is wrong");
        expect(st::compose(SA2_STAGE_FRAGMENT, "x\n", core) == "#define M600_STAGE_FRAGMENT 1\n#line 1 \"stage.hlsl\"\nx\n"
            "#line 1 \"geometry.hlsl\"\nG\n#line 1 \"draw.hlsl\"\n// draw\n\nD\n#line 1 \"stages.hlsl\"\nS\n", "the composed fragment source is wrong");
        expect(st::compose(st::CoreKind, "ignored", core).starts_with("#line 1 \"geometry.hlsl\"\n"), "the core source carries stage text");
        bool missing = false;
        try { st::compose(st::CoreKind, {}, {"G", "D", "S"}); } catch (const std::runtime_error&) { missing = true; }
        expect(missing, "draw.hlsl without its include line was composed");
        // The record fits its buffer: the error is cut at a character boundary.
        Json record(Json::Object{});
        record["format"] = st::RecordFormat;
        record["error"] = std::string(300, 'e') + "\xC3\xA9" + std::string(300, 'f');
        const auto truncated = [](const Json& parsed) { return std::get<bool>(parsed.at("error_truncated").value); };
        const auto whole = Json::parse(st::fit_record(record, 4096));
        expect(!truncated(whole) && whole.at("error").string() == record.at("error").string(), "a fitting record was truncated");
        for (uint32_t capacity = 300; capacity < 400; ++capacity) {
            const auto cut = st::fit_record(record, capacity);
            const auto parsed = Json::parse(cut);
            const auto error = parsed.at("error").string();
            expect(cut.size() <= capacity && truncated(parsed), "a long record was not truncated to its buffer");
            expect(st::text_refusal(error.data(), uint32_t(error.size())).empty() || error.empty(), "the record's error was cut inside a character");
        }
    });
    // C9 U5: RPI.72 step 3 exactly as rust/c600-display/src/raster.rs, with CT-RM-09's vectors and the core's clip classification.
    suite.check("raster bound", [] {
        namespace st = sa2::stages;
        using Shape = st::Shape;
        const auto placed = [](std::array<float, 2> a, std::array<float, 2> b, std::array<float, 2> c) {
            st::Triangle t;
            t.shape = Shape::Placed;
            t.corners = {a, b, c};
            return t;
        };
        const auto shaped = [](Shape shape) {
            st::Triangle t;
            t.shape = shape;
            return t;
        };
        const float nan = std::numeric_limits<float>::quiet_NaN();
        const struct { st::Triangle triangle; uint64_t count; } singles[] = {
            {placed({2.5f, 2.5f}, {5.5f, 2.5f}, {2.5f, 5.5f}), 16}, {placed({3, 3}, {5, 3}, {3, 5}), 16},
            {placed({-10, -10}, {3, -10}, {-10, 3}), 16}, {placed({20, 20}, {30, 20}, {20, 30}), 0},
            {placed({8, 2}, {8, 6}, {8, 10}), 0}, {placed({2, 8}, {6, 8}, {10, 8}), 0}, {placed({nan, 0}, {1, 0}, {0, 1}), 0},
            {shaped(Shape::CrossesNear), 256}, {shaped(Shape::Outside), 0}, {shaped(Shape::Undefined), 0}};
        for (const auto& single : singles) expect(st::count(16, 16, single.triangle) == single.count, "a single triangle's count differs from raster.rs");
        expect(st::count(15, 15, placed({14.2f, 0.2f}, {14.8f, 0.2f}, {14.2f, 0.8f})) == 4, "the odd-surface quad differs from raster.rs");
        expect(st::count(15, 15, shaped(Shape::CrossesNear)) == 225, "the near-plane count differs from raster.rs");
        const auto speck = placed({5.2f, 5.2f}, {5.8f, 5.2f}, {5.2f, 5.8f});
        const st::Triangle unbounded[] = {shaped(Shape::CrossesNear), shaped(Shape::Outside), shaped(Shape::Undefined)};
        const st::Triangle near_speck[] = {shaped(Shape::CrossesNear), speck};
        const st::RasterPass first[] = {{unbounded, 1}}, second[] = {{near_speck, 1}};
        expect(st::bound(16, 16, first) == 256 && st::bound(16, 16, second) == 260, "the clip classes' bound differs from raster.rs");
        const std::array<std::array<float, 2>, 6> v{{{0, 0}, {16, 0}, {16, 8}, {16, 16}, {8, 16}, {0, 16}}};
        const auto fan = [&](std::initializer_list<std::array<int, 3>> indices) {
            std::vector<st::Triangle> out;
            for (const auto& i : indices) out.push_back(placed(v[i[0]], v[i[1]], v[i[2]]));
            return out;
        };
        const auto spread = fan({{0, 1, 2}, {0, 2, 3}, {0, 3, 4}, {0, 4, 5}, {1, 2, 3}});
        const auto overlap = fan({{0, 1, 3}, {0, 3, 5}, {1, 5, 2}, {0, 2, 4}, {1, 4, 0}});
        const st::RasterPass spread_pass[] = {{spread, 1}}, overlap_pass[] = {{overlap, 1}}, two_passes[] = {{spread, 2}, {spread, 2}};
        expect(st::bound(16, 16, spread_pass) == 768 && st::bound(16, 16, overlap_pass) == 1280, "the fan bounds differ from raster.rs");
        expect(st::allowance(16, 16, 4) == 1024 && st::bound(16, 16, two_passes) == 3072, "the allowance or the pass sum differs from raster.rs");
        const st::Triangle whole[] = {shaped(Shape::CrossesNear)};
        const st::RasterPass huge[] = {{whole, UINT32_MAX}, {whole, UINT32_MAX}};
        bool overflow = false;
        try { st::bound(65535, 65535, huge); } catch (const std::overflow_error&) { overflow = true; }
        expect(overflow, "a bound past 64 bits was not refused");
        // The core's classification and viewport transform (stages.hlsl m600_count).
        using C = st::Clip;
        const auto px = st::pixel(C{0, 0, 0.5f, 1}, 16, 16), corner = st::pixel(C{-2, 2, 1, 2}, 16, 16);
        expect(px == std::array<float, 2>{8, 8} && corner == std::array<float, 2>{0, 0}, "the viewport transform changed");
        const C in{0, 0, 0.5f, 1}, right{2, 0, 0.5f, 1}, behind{0, 0, -0.5f, 1};
        expect(st::classify({in, in, C{nan, 0, 0, 1}}, 16, 16).shape == Shape::Undefined, "a nonfinite vertex is not undefined");
        expect(st::classify({right, right, right}, 16, 16).shape == Shape::Outside, "a triangle beyond x = w is not outside");
        expect(st::classify({behind, behind, behind}, 16, 16).shape == Shape::Outside, "a triangle behind z = 0 is not outside");
        expect(st::classify({in, right, behind}, 16, 16).shape == Shape::CrossesNear, "a vertex behind z = 0 does not cross the near plane");
        expect(st::classify({in, right, C{0, 0, 0, -1}}, 16, 16).shape == Shape::CrossesNear, "w <= 0 does not cross the near plane");
        const auto inside = st::classify({in, C{0.5f, 0, 0.5f, 1}, C{0, 0.5f, 0.5f, 1}}, 16, 16);
        expect(inside.shape == Shape::Placed && st::count(16, 16, inside) == 16, "a placed triangle's count is wrong");
        // eval_bound over device records: a placed triangle and one with an undefined vertex (code 1).
        std::vector<uint32_t> words(24);
        const C records[] = {in, C{0.5f, 0, 0.5f, 1}, C{0, 0.5f, 0.5f, 1}, in, in};
        for (size_t i = 0; i < 5; ++i) std::memcpy(&words[i * 4], records[i].data(), 16);
        words[20] = words[21] = words[22] = 0;
        words[23] = 0x7FC00001u;
        expect(st::coverage_code(&words[20]) == 1 && st::coverage_code(&words[0]) == 0, "the coverage code is misread");
        expect(st::eval_bound(words, 16, 16) == 16, "eval_bound differs from the formula");
        bool partial = false;
        try { st::eval_bound(std::span<const uint32_t>(words.data(), 20), 16, 16); } catch (const std::invalid_argument&) { partial = true; }
        expect(partial, "a partial triangle was bounded");
    });
}
Options parse(int argc, char** argv) {
    Options options;
    for (int i = 1; i < argc; ++i) {
        const std::string arg = argv[i];
        const auto value = [&]() -> std::string { expect(i + 1 < argc, "missing option value"); return argv[++i]; };
        if (arg == "--cpu" || arg == "--warp" || arg == "--hardware") {
            expect(options.mode.empty(), "choose exactly one mode");
            options.mode = arg.substr(2);
        } else if (arg == "--debug") options.debug = true;
        else if (arg == "--stages") options.stages = true;
        else if (arg == "--poses") options.poses = true;
        else if (arg == "--shape") options.shape = true;
        else if (arg == "--style") options.style = true;
        else if (arg == "--panes") options.panes = true;
        else if (arg == "--measure-stages") options.measure = value();
        else if (arg == "--measure-prepare") {
            const auto text = value();
            expect(!text.empty() && text.size() <= 5 && text.find_first_not_of("0123456789") == std::string::npos
                && std::stoul(text) > 0, "--measure-prepare takes 1..99999");
            options.prepares = uint32_t(std::stoul(text));
        }
        else if (arg == "--size") {
            const auto text = value();
            const auto x = text.find('x');
            expect(x != std::string::npos && x > 0 && x + 1 < text.size()
                && text.find_first_not_of("0123456789x") == std::string::npos && text.find('x', x + 1) == std::string::npos,
                "--size must be WIDTHxHEIGHT");
            const auto w = std::stoul(text.substr(0, x)), h = std::stoul(text.substr(x + 1));
            expect(w >= 16 && h >= 16 && w <= 16384 && h <= 16384, "--size must be within 16..16384 on each side");
            options.width = uint32_t(w), options.height = uint32_t(h);
        } else if (arg == "--frames" || arg == "--traced") {
            const auto text = value();
            expect(!text.empty() && text.size() <= 6 && text.find_first_not_of("0123456789") == std::string::npos,
                "--frames and --traced take 0..999999");
            (arg == "--frames" ? options.frames : options.traced) = uint32_t(std::stoul(text));
        } else if (arg == "--turn-ms") {
            size_t end = 0;
            const auto text = value();
            options.turn_ms = std::stod(text, &end);
            expect(end == text.size() && std::isfinite(options.turn_ms) && options.turn_ms > 0 && options.turn_ms <= 10000,
                "--turn-ms must be in (0, 10000]");
        }
        else if (arg == "--out") options.out = value();
        else if (arg == "--vectors") options.vectors = value();
        else if (arg == "--child") options.child = value();
        else throw std::invalid_argument("unknown selftest option");
    }
    expect(!options.mode.empty(), "choose --cpu, --warp or --hardware");
    expect(options.child.empty() || options.mode != "cpu", "child checks require a device mode");
    // C9 U5: the user-stage check runs on WARP only, never on a hardware adapter.
    expect(!options.stages || (options.mode == "warp" && options.child.empty()), "--stages requires --warp");
    // C9 jumble-on: the product pose check runs on WARP only, alone.
    expect(!options.poses || (options.mode == "warp" && options.child.empty() && !options.stages && options.measure.empty()),
        "--poses requires --warp, without --stages or --measure-stages");
    // ABI 6: the view shape check runs on WARP only, alone.
    expect(!options.shape || (options.mode == "warp" && options.child.empty() && !options.stages && !options.poses
        && options.measure.empty()), "--shape requires --warp, without --stages, --poses or --measure-stages");
    // ABI 7: the piece style check runs on WARP only, alone.
    expect(!options.style || (options.mode == "warp" && options.child.empty() && !options.stages && !options.poses && !options.shape
        && options.measure.empty()), "--style requires --warp, without --stages, --poses, --shape or --measure-stages");
    // ABI 8: the composite frame check runs on WARP only, alone.
    expect(!options.panes || (options.mode == "warp" && options.child.empty() && !options.stages && !options.poses && !options.shape
        && !options.style && options.measure.empty()), "--panes requires --warp, alone");
    // C9 U5c: the overdraw measurement also runs on WARP only, alone.
    expect(options.measure.empty() || (options.mode == "warp" && options.child.empty() && !options.stages && !options.out.empty()
        && options.frames + uint64_t(options.traced) > 0), "--measure-stages requires --warp, --out and frames, without --stages");
    // C9 (26 Nov 2026): the preparation timing needs a device and runs alone.
    expect(!options.prepares || (options.mode != "cpu" && options.child.empty() && !options.stages && !options.poses
        && !options.shape && !options.style && !options.panes && options.measure.empty() && !options.out.empty()), "--measure-prepare requires --warp or --hardware and --out, alone");
    return options;
}
}
}

int main(int argc, char** argv) {
    try {
        const auto options = sa2test::parse(argc, argv);
        if (!options.child.empty()) return sa2test::run_child(options);
        if (!options.measure.empty()) {
            sa2test::run_measure(options);
            std::cout << "measure: ok\n";
            return 0;
        }
        if (options.prepares) {
            sa2test::run_measure_prepare(options);
            std::cout << "measure: ok\n";
            return 0;
        }
        sa2test::Suite suite;
        if (options.mode == "cpu") sa2test::run_cpu(suite, options);
        else if (options.stages) sa2test::run_stages(suite, options);
        else if (options.poses) sa2test::run_poses(suite, options);
        else if (options.shape) sa2test::run_shape(suite, options);
        else if (options.style) sa2test::run_style(suite, options);
        else if (options.panes) sa2test::run_panes(suite, options);
        else sa2test::run_gpu(suite, options);
        auto json = sa2::json_report(options.mode, suite.checks);
        if (options.mode == "cpu" && suite.passed()) {
            char identity[4096]{};
            sa2test::expect(sa2_identity(identity, sizeof(identity)) == SA2_OK, "could not retrieve build identity");
            auto report = Json::parse(json); report["identity"] = Json::parse(identity); json = report.dump() + '\n';
        }
        if (options.out.empty()) std::cout << json;
        else {
            std::ofstream out(options.out, std::ios::binary);
            out << json;
            sa2test::expect(static_cast<bool>(out), "could not write selftest JSON");
        }
        for (const auto& check : suite.checks)
            if (check.status == "fail") std::cerr << check.name << ": " << check.reason << '\n';
        std::cout << (suite.passed() ? "selftest: ok\n" : "selftest: failed\n");
        return suite.passed() ? 0 : 1;
    } catch (const std::exception& error) {
        std::cerr << "selftest: failed: " << error.what() << '\n';
        return 1;
    }
}
