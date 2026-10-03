#include "test.h"
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
    suite.check("abi", [] { expect(sa2_abi_version() == 1, "ABI version mismatch"); });
    suite.check("exports", [] {
        const auto dll = executable_path().parent_path() / "sa2_interop.dll";
        HMODULE module = LoadLibraryW(dll.c_str());
        expect(module != nullptr, "LoadLibraryW of adjacent DLL failed");
        constexpr const char* names[] = {"sa2_abi_version", "sa2_last_error", "sa2_probe", "sa2_attach", "sa2_create_texture", "sa2_release_texture",
            "sa2_register_slot", "sa2_unregister_slot", "sa2_signal_godot_free", "sa2_produce", "sa2_godot_wait_ready", "sa2_mark_shown",
            "sa2_verify_slot", "sa2_drain", "sa2_debug_counts", "sa2_debug_messages", "sa2_remove_device", "sa2_device_removed_reason", "sa2_detach"};
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
        const auto json = sa2::json_report(options.mode, suite.checks);
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
