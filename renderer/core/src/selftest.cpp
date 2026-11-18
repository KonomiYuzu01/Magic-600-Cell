#include "test.h"
#include "scene_record.h"
#include <algorithm>
#include <cstring>
#include <fstream>
#include <iostream>
#include <limits>
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
void run_cpu(Suite& suite, const Options& options) {
    suite.check("abi", [] { expect(sa2_abi_version() == 3, "ABI version mismatch"); });
    suite.check("exports", [] {
        const auto dll = executable_path().parent_path() / "sa2_interop.dll";
        HMODULE module = LoadLibraryW(dll.c_str());
        expect(module != nullptr, "LoadLibraryW of adjacent DLL failed");
        constexpr const char* names[] = {"sa2_abi_version", "sa2_last_error", "sa2_probe", "sa2_attach", "sa2_create_texture", "sa2_release_texture",
            "sa2_register_slot", "sa2_unregister_slot", "sa2_signal_godot_free", "sa2_produce", "sa2_godot_wait_ready", "sa2_mark_shown",
            "sa2_verify_slot", "sa2_drain", "sa2_debug_counts", "sa2_debug_messages", "sa2_remove_device", "sa2_device_removed_reason", "sa2_detach",
            "sa2_scene_load", "sa2_scene_produce", "sa2_scene_trace_begin", "sa2_scene_trace_end", "sa2_scene_write_run",
            "sa2_scene_geometry_check", "sa2_identity", "sa2_scene_unload", "sa2_scene_load_wj", "sa2_scene_lattice_check"};
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
        expect(shaders.size() == 5 && shaders[4].at("file").string() == "lattice_cs.dxil", "ABI 3 identity does not list lattice_cs.dxil");
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
        else if (arg == "--out") options.out = value();
        else if (arg == "--vectors") options.vectors = value();
        else if (arg == "--child") options.child = value();
        else throw std::invalid_argument("unknown selftest option");
    }
    expect(!options.mode.empty(), "choose --cpu, --warp or --hardware");
    expect(options.child.empty() || options.mode != "cpu", "child checks require a device mode");
    return options;
}
}
}

int main(int argc, char** argv) {
    try {
        const auto options = sa2test::parse(argc, argv);
        if (!options.child.empty()) return sa2test::run_child(options);
        sa2test::Suite suite;
        if (options.mode == "cpu") sa2test::run_cpu(suite, options);
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
