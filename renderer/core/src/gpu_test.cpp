#include "test.h"
#include "scene_record.h"
#include "product.h"
#include "stage_rules.h"
#include "stage_sources.h"
#include <d3d12sdklayers.h>
#include <dxgi1_6.h>
#include <wrl/client.h>
#include <algorithm>
#include <bit>
#include <chrono>
#include <cstring>
#include <iomanip>
#include <iostream>
#include <fstream>
#include <limits>
#include <map>
#include <set>
#include <sstream>
#include <thread>
#include <utility>

namespace sa2test {
namespace {
using Microsoft::WRL::ComPtr;
constexpr DWORD timeout_ms = 10000;
std::string hr_text(const char* call, HRESULT hr) {
    std::ostringstream out;
    out << call << " failed: HRESULT 0x" << std::hex << std::setw(8) << std::setfill('0') << static_cast<unsigned long>(hr);
    return out.str();
}
void check(HRESULT hr, const char* call) { if (FAILED(hr)) throw std::runtime_error(hr_text(call, hr)); }
void capability(HRESULT hr, const char* call) {
    if (hr == E_NOINTERFACE || hr == E_NOTIMPL || hr == DXGI_ERROR_UNSUPPORTED)
        throw Unsupported(hr_text(call, hr));
    check(hr, call);
}
void api(sa2_context* ctx, int32_t status) {
    if (status == SA2_OK) return;
    char error[2048]{};
    sa2_last_error(ctx, error, sizeof(error));
    if (status == SA2_E_UNSUPPORTED) throw Unsupported(error);
    throw std::runtime_error(error);
}
template<class T> uint64_t handle(T* object) { return reinterpret_cast<uint64_t>(object); }
ULONG references(IUnknown* object) { object->AddRef(); return object->Release(); }
class Handle {
    HANDLE value_ = nullptr;
public:
    Handle() = default;
    explicit Handle(HANDLE value) : value_(value) {}
    ~Handle() { if (value_ && value_ != INVALID_HANDLE_VALUE) CloseHandle(value_); }
    Handle(const Handle&) = delete;
    Handle& operator=(const Handle&) = delete;
    Handle(Handle&& other) noexcept : value_(std::exchange(other.value_, nullptr)) {}
    Handle& operator=(Handle&& other) noexcept {
        if (value_ && value_ != INVALID_HANDLE_VALUE) CloseHandle(value_);
        value_ = std::exchange(other.value_, nullptr);
        return *this;
    }
    HANDLE get() const { return value_; }
};
void win_check(BOOL ok, const char* call) { if (!ok) check(HRESULT_FROM_WIN32(GetLastError()), call); }
template<class T> void name(T* object, const wchar_t* text) { check(object->SetName(text), "SetName(selftest)"); }
struct Device {
    ComPtr<IDXGIAdapter1> adapter;
    ComPtr<ID3D12Device> device;
    ComPtr<ID3D12CommandQueue> queue;
    explicit Device(const Options& options) {
        if (options.debug) {
            ComPtr<ID3D12Debug> debug;
            capability(D3D12GetDebugInterface(IID_PPV_ARGS(&debug)), "D3D12GetDebugInterface");
            debug->EnableDebugLayer();
        }
        ComPtr<IDXGIFactory6> factory;
        capability(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)), "CreateDXGIFactory2");
        if (options.mode == "warp") capability(factory->EnumWarpAdapter(IID_PPV_ARGS(&adapter)), "EnumWarpAdapter");
        else {
            for (UINT i = 0;; ++i) {
                const auto hr = factory->EnumAdapterByGpuPreference(i, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE, IID_PPV_ARGS(&adapter));
                if (hr == DXGI_ERROR_NOT_FOUND) throw Unsupported("no high-performance hardware adapter");
                capability(hr, "EnumAdapterByGpuPreference");
                DXGI_ADAPTER_DESC1 desc{};
                check(adapter->GetDesc1(&desc), "GetDesc1");
                if (!(desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE)) break;
                adapter.Reset();
            }
        }
        capability(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device)), "D3D12CreateDevice");
        D3D12_COMMAND_QUEUE_DESC desc{};
        desc.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
        check(device->CreateCommandQueue(&desc, IID_PPV_ARGS(&queue)), "CreateCommandQueue(stand-in Godot)");
        name(queue.Get(), L"SA2 selftest stand-in Godot queue");
    }
};
ComPtr<ID3D12Resource> external_texture(ID3D12Device* device, uint32_t width, uint32_t height, bool enhanced, bool render_target = true) {
    D3D12_HEAP_PROPERTIES heap{};
    heap.Type = D3D12_HEAP_TYPE_DEFAULT;
    heap.CreationNodeMask = heap.VisibleNodeMask = 1;
    D3D12_RESOURCE_DESC1 desc{};
    desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    desc.Width = width; desc.Height = height;
    desc.DepthOrArraySize = desc.MipLevels = 1;
    desc.Format = DXGI_FORMAT_R8G8B8A8_TYPELESS;
    desc.SampleDesc.Count = 1;
    desc.Flags = render_target ? D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET : D3D12_RESOURCE_FLAG_NONE;
    D3D12_CLEAR_VALUE black{};
    black.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    ComPtr<ID3D12Resource> result;
    if (enhanced) {
        ComPtr<ID3D12Device10> device10;
        capability(device->QueryInterface(IID_PPV_ARGS(&device10)), "QueryInterface(ID3D12Device10)");
        check(device10->CreateCommittedResource3(&heap, D3D12_HEAP_FLAG_NONE, &desc, D3D12_BARRIER_LAYOUT_SHADER_RESOURCE,
            render_target ? &black : nullptr, nullptr, 0, nullptr, IID_PPV_ARGS(&result)), "CreateCommittedResource3(external)");
    } else {
        D3D12_RESOURCE_DESC legacy{};
        legacy.Dimension = desc.Dimension; legacy.Width = desc.Width; legacy.Height = desc.Height;
        legacy.DepthOrArraySize = desc.DepthOrArraySize; legacy.MipLevels = desc.MipLevels;
        legacy.Format = desc.Format; legacy.SampleDesc = desc.SampleDesc; legacy.Flags = desc.Flags;
        check(device->CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &legacy,
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE | D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE,
            render_target ? &black : nullptr, IID_PPV_ARGS(&result)), "CreateCommittedResource(external)");
    }
    name(result.Get(), L"SA2 selftest external texture");
    return result;
}

struct Harness {
    Device env;
    sa2_context* ctx = nullptr;
    bool enhanced, external;
    std::array<uint64_t, SA2_RING_SLOTS> resources{};
    std::array<bool, SA2_RING_SLOTS> registered{};
    std::array<ComPtr<ID3D12Resource>, SA2_RING_SLOTS> external_resources;
    std::array<ULONG, SA2_RING_SLOTS> external_counts{};
    ComPtr<ID3D12CommandAllocator> allocator;
    ComPtr<ID3D12GraphicsCommandList> list;
    ComPtr<ID3D12GraphicsCommandList7> list7;
    ComPtr<ID3D12Resource> readback;
    ComPtr<ID3D12Fence> consumer_done;
    Handle completion;
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT footprint{};
    UINT64 readback_bytes = 0, consumer_value = 0;
    ULONG device_before = 0, queue_before = 0;
    uint32_t width = 0, height = 0;
    DWORD wait = timeout_ms; // the core's wait timeout and the consumer's; the U5c measurement raises it for WARP

    Harness(const Options& options, int32_t queue_mode, int32_t barriers, bool is_external, const Device* shared = nullptr,
            DWORD wait_ms = timeout_ms)
        : env(shared ? *shared : Device(options)), enhanced(barriers == SA2_BARRIERS_ENHANCED), external(is_external), wait(wait_ms) {
        sa2_device_info info{};
        info.struct_size = sizeof(info);
        api(nullptr, sa2_probe(handle(env.device.Get()), handle(env.queue.Get()), handle(env.adapter.Get()), &info));
        expect(info.queue_type == D3D12_COMMAND_LIST_TYPE_DIRECT && info.queue_device_matches == 1 && info.adapter_matches_device == 1
            && info.node_count > 0, "device probe did not confirm stand-in ownership");
        if (options.debug) expect(info.debug_layer == 1, "debug device has no ID3D12InfoQueue1");
        if (enhanced && !info.enhanced_barriers) throw Unsupported("OPTIONS12 reports EnhancedBarriersSupported=false");
        check(env.device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&allocator)), "CreateCommandAllocator(consumer)");
        name(allocator.Get(), L"SA2 selftest consumer allocator");
        check(env.device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, allocator.Get(), nullptr, IID_PPV_ARGS(&list)), "CreateCommandList(consumer)");
        name(list.Get(), L"SA2 selftest consumer list");
        check(list->Close(), "Close new consumer list");
        if (enhanced) capability(list.As(&list7), "QueryInterface(ID3D12GraphicsCommandList7)");
        check(env.device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&consumer_done)), "CreateFence(consumer)");
        name(consumer_done.Get(), L"SA2 selftest consumer fence");
        completion = Handle(CreateEventW(nullptr, FALSE, FALSE, nullptr));
        win_check(completion.get() != nullptr, "CreateEventW(consumer)");
        device_before = references(env.device.Get()); queue_before = references(env.queue.Get());
        sa2_config config{};
        config.struct_size = sizeof(config);
        config.queue_mode = queue_mode;
        config.barrier_api = barriers;
        config.state_before_write = config.state_after_write = SA2_STATE_ALL_SHADER_RESOURCE;
        config.wait_timeout_ms = wait;
        config.debug_callback = 1;
        api(nullptr, sa2_attach(handle(env.device.Get()), handle(env.queue.Get()), &config, &ctx));
    }
    ~Harness() {
        // Even failed tests must finish queued consumer and producer work before
        // their owning COM pointers unwind. An unconfirmed drain exits in DLL.
        if (!ctx) return;
        sa2_drain(ctx, wait);
        sa2_scene_unload(ctx); // WRONG_STATE is harmless for ABI 1 configurations.
        for (uint32_t i = 0; i < SA2_RING_SLOTS; ++i) {
            if (registered[i]) sa2_unregister_slot(ctx, i);
            if (!external && resources[i]) {
                uint32_t count = 0;
                sa2_release_texture(ctx, resources[i], &count);
            }
        }
        sa2_detach(ctx);
    }
    void invalid_resources() {
        auto no_rt = external_texture(env.device.Get(), 256, 192, enhanced, false);
        expect(sa2_register_slot(ctx, 0, handle(no_rt.Get()), 256, 192, 1) == SA2_E_INVALID_ARGUMENT, "texture without ALLOW_RENDER_TARGET accepted");
        auto wrong_size = external_texture(env.device.Get(), 256, 192, enhanced);
        expect(sa2_register_slot(ctx, 0, handle(wrong_size.Get()), 257, 192, 1) == SA2_E_INVALID_ARGUMENT, "wrong texture size accepted");
        expect(sa2_register_slot(ctx, 0, handle(wrong_size.Get()), 256, 192, 0) == SA2_E_INVALID_ARGUMENT, "unknown imported texture accepted");
        expect(sa2_register_slot(ctx, SA2_RING_SLOTS, handle(wrong_size.Get()), 256, 192, 1) == SA2_E_INVALID_ARGUMENT, "out-of-range slot accepted");
        uint64_t resource = 0;
        expect(sa2_create_texture(ctx, 127, 192, SA2_STATE_ALL_SHADER_RESOURCE, &resource) == SA2_E_INVALID_ARGUMENT, "too-small texture accepted");
        expect(sa2_create_texture(ctx, 256, 192, 99, &resource) == SA2_E_INVALID_ARGUMENT, "invalid initial state accepted");
        struct sa2_debug_counts counts{};
        expect(sa2_debug_counts(ctx, &counts) == SA2_E_INVALID_ARGUMENT, "wrong debug_counts struct_size accepted");
    }
    void make_ring(uint32_t w, uint32_t h) {
        width = w; height = h;
        for (uint32_t i = 0; i < SA2_RING_SLOTS; ++i) {
            if (external) {
                external_resources[i] = external_texture(env.device.Get(), w, h, enhanced);
                external_counts[i] = references(external_resources[i].Get());
                resources[i] = handle(external_resources[i].Get());
            } else api(ctx, sa2_create_texture(ctx, w, h, SA2_STATE_ALL_SHADER_RESOURCE, &resources[i]));
            if (i == 1) expect(sa2_register_slot(ctx, i, resources[0], w, h, external ? 1 : 0) == SA2_E_WRONG_STATE,
                "one resource registered in two slots");
            api(ctx, sa2_register_slot(ctx, i, resources[i], w, h, external ? 1 : 0));
            registered[i] = true;
        }
        auto* texture = reinterpret_cast<ID3D12Resource*>(resources[0]);
        const auto desc = texture->GetDesc();
        env.device->GetCopyableFootprints(&desc, 0, 1, 0, &footprint, nullptr, nullptr, &readback_bytes);
        expect(readback_bytes && readback_bytes != UINT64_MAX, "invalid consumer footprint");
        D3D12_HEAP_PROPERTIES heap{};
        heap.Type = D3D12_HEAP_TYPE_READBACK;
        heap.CreationNodeMask = heap.VisibleNodeMask = 1;
        D3D12_RESOURCE_DESC buffer{};
        buffer.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
        buffer.Width = readback_bytes; buffer.Height = 1;
        buffer.DepthOrArraySize = buffer.MipLevels = 1;
        buffer.SampleDesc.Count = 1;
        buffer.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
        check(env.device->CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &buffer,
            D3D12_RESOURCE_STATE_COPY_DEST, nullptr, IID_PPV_ARGS(&readback)), "CreateCommittedResource(consumer readback)");
        name(readback.Get(), L"SA2 selftest consumer readback");
    }
    void clear_ring() {
        for (uint32_t i = 0; i < SA2_RING_SLOTS; ++i) {
            if (registered[i]) {
                api(ctx, sa2_unregister_slot(ctx, i));
                registered[i] = false;
            }
            if (!resources[i]) continue;
            if (external) {
                expect(references(external_resources[i].Get()) == external_counts[i], "external texture reference count did not return to baseline");
                external_resources[i].Reset();
            } else {
                uint32_t count = UINT32_MAX;
                api(ctx, sa2_release_texture(ctx, resources[i], &count));
                expect(count == 0, "created texture retained a reference after release");
            }
            resources[i] = 0;
        }
        readback.Reset();
    }
    void consumer_transition(ID3D12Resource* texture, bool into_copy) {
        if (!enhanced) {
            const auto shader = D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE | D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
            D3D12_RESOURCE_BARRIER barrier{};
            barrier.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
            barrier.Transition = {texture, D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                into_copy ? shader : D3D12_RESOURCE_STATE_COPY_SOURCE, into_copy ? D3D12_RESOURCE_STATE_COPY_SOURCE : shader};
            list->ResourceBarrier(1, &barrier);
        } else {
            D3D12_TEXTURE_BARRIER barrier{};
            barrier.SyncBefore = into_copy ? D3D12_BARRIER_SYNC_NONE : D3D12_BARRIER_SYNC_COPY;
            barrier.SyncAfter = into_copy ? D3D12_BARRIER_SYNC_COPY : D3D12_BARRIER_SYNC_NONE;
            barrier.AccessBefore = into_copy ? D3D12_BARRIER_ACCESS_NO_ACCESS : D3D12_BARRIER_ACCESS_COPY_SOURCE;
            barrier.AccessAfter = into_copy ? D3D12_BARRIER_ACCESS_COPY_SOURCE : D3D12_BARRIER_ACCESS_NO_ACCESS;
            barrier.LayoutBefore = into_copy ? D3D12_BARRIER_LAYOUT_SHADER_RESOURCE : D3D12_BARRIER_LAYOUT_COPY_SOURCE;
            barrier.LayoutAfter = into_copy ? D3D12_BARRIER_LAYOUT_COPY_SOURCE : D3D12_BARRIER_LAYOUT_SHADER_RESOURCE;
            barrier.pResource = texture;
            barrier.Subresources.NumMipLevels = barrier.Subresources.NumArraySlices = barrier.Subresources.NumPlanes = 1;
            D3D12_BARRIER_GROUP group{};
            group.Type = D3D12_BARRIER_TYPE_TEXTURE;
            group.NumBarriers = 1;
            group.pTextureBarriers = &barrier;
            list7->Barrier(1, &group);
        }
    }
    void consume(uint32_t frame, uint32_t generation, bool code_image = true) {
        const auto index = frame % SA2_RING_SLOTS;
        auto* texture = reinterpret_cast<ID3D12Resource*>(resources[index]);
        check(allocator->Reset(), "Reset consumer allocator");
        check(list->Reset(allocator.Get(), nullptr), "Reset consumer list");
        consumer_transition(texture, true);
        D3D12_TEXTURE_COPY_LOCATION source{};
        source.pResource = texture;
        source.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
        D3D12_TEXTURE_COPY_LOCATION destination{};
        destination.pResource = readback.Get();
        destination.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
        destination.PlacedFootprint = footprint;
        list->CopyTextureRegion(&destination, 0, 0, 0, &source, nullptr);
        consumer_transition(texture, false);
        check(list->Close(), "Close consumer list");
        ID3D12CommandList* commands[] = {list.Get()};
        env.queue->ExecuteCommandLists(1, commands);
        check(env.queue->Signal(consumer_done.Get(), ++consumer_value), "Signal consumer completion");
        check(consumer_done->SetEventOnCompletion(consumer_value, completion.get()), "SetEventOnCompletion(consumer)");
        expect(WaitForSingleObject(completion.get(), wait) == WAIT_OBJECT_0, "consumer fence wait failed or timed out");
        const auto completed = consumer_done->GetCompletedValue();
        expect(completed != UINT64_MAX && completed >= consumer_value && SUCCEEDED(env.device->GetDeviceRemovedReason()), "consumer completion was not confirmed");
        if (!code_image) return;
        void* bytes = nullptr;
        const D3D12_RANGE read_range{static_cast<SIZE_T>(footprint.Offset), static_cast<SIZE_T>(readback_bytes)};
        check(readback->Map(0, &read_range, &bytes), "Map consumer readback");
        sa2::Fields decoded{};
        const bool valid = sa2::decode(static_cast<const uint8_t*>(bytes) + footprint.Offset, footprint.Footprint.RowPitch, width, height, decoded);
        const D3D12_RANGE written{0, 0};
        readback->Unmap(0, &written);
        expect(valid && decoded == sa2::Fields{frame, index, generation}, "consumer corners do not decode to this frame, slot and generation");
    }
    void finish(bool debug) {
        api(ctx, sa2_drain(ctx, wait));
        clear_ring();
        if (debug) {
            struct sa2_debug_counts counts{};
            counts.struct_size = sizeof(counts);
            api(ctx, sa2_debug_counts(ctx, &counts));
            char messages[8192]{};
            api(ctx, sa2_debug_messages(ctx, messages, sizeof(messages)));
            expect(counts.error == 0 && counts.corruption == 0, "debug layer recorded errors or corruption");
            expect(messages[0] == 0, ("debug layer recorded a warning other than mismatching clear value: "
                + std::string(messages)).c_str());
            expect(counts.warning == counts.mismatching_clear_value, "unclassified debug warnings");
        }
        api(ctx, sa2_detach(ctx));
        ctx = nullptr;
        expect(references(env.device.Get()) == device_before && references(env.queue.Get()) == queue_before, "device or Godot queue reference count changed across attach/detach");
    }
};

void configuration(const Options& options, int32_t queue_mode, int32_t barriers, bool external) {
    Harness harness(options, queue_mode, barriers, external);
    auto* ctx = harness.ctx;
    harness.invalid_resources();
    harness.make_ring(256, 192);
    expect(sa2_detach(ctx) == SA2_E_WRONG_STATE, "detach with registered slots succeeded");
    uint32_t count = 0;
    expect(sa2_release_texture(ctx, handle(harness.readback.Get()), &count) == SA2_E_INVALID_ARGUMENT, "unknown resource release succeeded");
    uint32_t generation = 1;
    for (uint32_t frame = 1; frame <= 300; ++frame) {
        if (frame == 150) {
            api(ctx, sa2_drain(ctx, timeout_ms));
            harness.clear_ring();
            ++generation;
            harness.make_ring(320, 240);
        }
        const auto index = frame % SA2_RING_SLOTS;
        api(ctx, sa2_signal_godot_free(ctx, frame - 1));
        api(ctx, sa2_produce(ctx, index, frame, frame, generation));
        api(ctx, sa2_godot_wait_ready(ctx, frame));
        api(ctx, sa2_mark_shown(ctx, index, frame));
        harness.consume(frame, generation);
        if (frame == 1) {
            expect(sa2_produce(ctx, index, frame, frame, generation) == SA2_E_WRONG_STATE, "non-increasing frame accepted");
            expect(sa2_unregister_slot(ctx, index) == SA2_E_WRONG_STATE, "unregister without a confirmed drain succeeded");
            expect(sa2_mark_shown(ctx, index, 0) == SA2_E_WRONG_STATE, "decreasing shown frame accepted");
            if (queue_mode == SA2_QUEUE_OWN) {
                expect(sa2_produce(ctx, index, frame + 1, frame + 1, generation) == SA2_E_WRONG_STATE, "own producer enqueued an unsignalled free wait");
                expect(sa2_godot_wait_ready(ctx, frame + 1) == SA2_E_WRONG_STATE, "Godot enqueued an unproduced ready wait");
            }
        }
        if (frame == 2) expect(sa2_signal_godot_free(ctx, 0) == SA2_E_WRONG_STATE, "decreasing free frame accepted");
        if (frame % 50 == 0) {
            uint64_t mismatch = UINT64_MAX;
            api(ctx, sa2_verify_slot(ctx, index, frame, generation, &mismatch));
            expect(mismatch == 0, "correct image verification mismatched texels");
            if (frame == 50) {
                expect(sa2_verify_slot(ctx, index, frame + 1, generation, &mismatch) == SA2_E_VERIFY && mismatch > 0, "deliberate verify mismatch was not detected");
            }
        }
    }
    harness.finish(options.debug);
}

struct SceneDirectory {
    std::filesystem::path path;
    std::string utf8;
    explicit SceneDirectory(const std::string& name) {
        path = executable_path().parent_path() / ("scene-" + std::to_string(GetCurrentProcessId()) + '-' + name);
        expect(std::filesystem::create_directory(path), "scene run directory already exists");
        const auto text = path.u8string(); utf8.assign(text.begin(), text.end());
    }
    ~SceneDirectory() { std::error_code error; std::filesystem::remove_all(path, error); }
    Json read(const char* file) const { return sb::readJson(path / file); }
};
void scene_geometry(const Options& options, const Device& device) {
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false, &device);
    harness.make_ring(options.mode == "warp" ? 640 : 2560, options.mode == "warp" ? 360 : 1600);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    SceneDirectory directory("geometry");
    api(harness.ctx, sa2_scene_geometry_check(harness.ctx, directory.utf8.c_str()));
    const auto json = directory.read("geometry.json");
    expect(json.at("format").string() == "magic600-sb-geometry-check-v1" && json.at("status").string() == "pass"
        && json.at("results").array().size() == 9 && json.at("per_cell_count").at("failures").integer() == 0,
        "scene geometry/count check did not pass all nine camera/pose combinations");
    expect(sa2_scene_geometry_check(harness.ctx, directory.utf8.c_str()) == SA2_E_IO, "geometry output overwritten");
    expect(sa2_scene_unload(harness.ctx) == SA2_E_WRONG_STATE, "geometry work unloaded before explicit drain");
    api(harness.ctx, sa2_drain(harness.ctx, timeout_ms)); api(harness.ctx, sa2_scene_unload(harness.ctx));
    harness.finish(options.debug);
}
void scene_configuration(const Options& options, const Device& device, uint32_t scene, int32_t queue, int32_t barriers,
                         bool inject = false, bool no_vram = false) {
    Harness harness(options, queue, barriers, false, &device);
    auto* ctx = harness.ctx;
    const uint32_t width = options.mode == "warp" ? 640 : 2560, height = options.mode == "warp" ? 360 : 1600;
    const uint32_t frames = options.mode == "warp" ? 40 : 600, preroll = 3, trace_stop_frame = frames - 2;
    harness.make_ring(width, height);
    SceneDirectory directory("w" + std::to_string(scene) + '-' + std::to_string(queue) + '-' + std::to_string(barriers)
        + (inject ? "-fault" : no_vram ? "-no-vram" : ""));
    expect(sa2_scene_trace_begin(ctx) == SA2_E_WRONG_STATE && sa2_scene_trace_end(ctx) == SA2_E_WRONG_STATE
        && sa2_scene_unload(ctx) == SA2_E_WRONG_STATE, "scene API accepted absent scene");
    // WARP draws a frame in about a second at 640 x 360, so its W3 turns are
    // long enough for every turn to be shown; hardware keeps S-B's 190 ms.
    const bool warp = options.mode == "warp";
    const double turn_ms = warp ? 3000.0 : inject ? 1000.0 : 190.0;
    const double label_clock_ms = scene == 3 ? turn_ms : 190.0; // W4 keeps S-B's 190 ms label clock
    // At most one label copy per frame, so frames times the label clock always reserves enough.
    sa2_scene_config config{sizeof(sa2_scene_config), scene, turn_ms,
        inject ? "corrupt-label" : nullptr, no_vram ? SA2_SCENE_NO_VRAM : 0u, frames * static_cast<uint32_t>(label_clock_ms)};
    auto bad = config; bad.struct_size = 0;
    expect(sa2_scene_load(ctx, &bad) == SA2_E_INVALID_ARGUMENT, "scene struct_size accepted on live context");
    api(ctx, sa2_scene_load(ctx, &config));
    expect(sa2_scene_load(ctx, &config) == SA2_E_WRONG_STATE, "second scene load succeeded");
    expect(sa2_detach(ctx) == SA2_E_WRONG_STATE && sa2_scene_write_run(ctx, directory.utf8.c_str()) == SA2_E_WRONG_STATE,
        "scene detach/write before trace succeeded");
    expect(sa2_scene_trace_end(ctx) == SA2_E_WRONG_STATE, "trace_end before begin succeeded");
    int64_t start = 0;
    LARGE_INTEGER hz{}; expect(QueryPerformanceFrequency(&hz) != 0, "QPC frequency unavailable");
    for (uint32_t frame = 1; frame <= frames; ++frame) {
        if (frame == preroll + 1) {
            api(ctx, sa2_scene_trace_begin(ctx)); start = sa2::scene_qpc();
            expect(sa2_scene_trace_begin(ctx) == SA2_E_WRONG_STATE, "second trace_begin succeeded");
            expect(sa2_scene_geometry_check(ctx, directory.utf8.c_str()) == SA2_E_WRONG_STATE, "geometry ran inside trace");
        }
        if (frame == trace_stop_frame) {
            api(ctx, sa2_scene_trace_end(ctx));
            expect(sa2_scene_trace_end(ctx) == SA2_E_WRONG_STATE, "second trace_end succeeded");
            expect(sa2_scene_write_run(ctx, directory.utf8.c_str()) == SA2_E_WRONG_STATE, "write_run without drain succeeded");
        }
        if (inject && frame > preroll && frame < trace_stop_frame && frame <= preroll + 21) {
            // Pace the first 21 revisions so the fixed S-B turn-20 corruption
            // is actually uploaded and read back, rather than merely unreached.
            const auto target = start + int64_t(frame - preroll - 1) * sb::turnTicks(config.turn_ms, hz.QuadPart);
            while (sa2::scene_qpc() < target) Sleep(1);
        }
        const auto slot = frame % SA2_RING_SLOTS;
        api(ctx, sa2_signal_godot_free(ctx, frame - 1));
        api(ctx, sa2_scene_produce(ctx, slot, frame));
        api(ctx, sa2_godot_wait_ready(ctx, frame));
        harness.consume(frame, 0, false); // framework work on its own queue
        api(ctx, sa2_mark_shown(ctx, slot, frame));
        if (frame == 1) {
            expect(sa2_scene_produce(ctx, slot, frame) == SA2_E_WRONG_STATE, "scene frame did not increase");
            expect(sa2_scene_produce(ctx, SA2_RING_SLOTS, frame + 1) == SA2_E_INVALID_ARGUMENT, "scene slot out of range accepted");
            expect(sa2_scene_produce(ctx, slot, UINT64_MAX) == SA2_E_INVALID_ARGUMENT, "scene reserved fence accepted");
            expect(sa2_scene_unload(ctx) == SA2_E_WRONG_STATE, "scene unloaded with unconfirmed work");
            if (queue == SA2_QUEUE_OWN) {
                expect(sa2_scene_produce(ctx, slot, frame + 1) == SA2_E_WRONG_STATE, "scene own queue enqueued unsafe free wait");
                expect(sa2_godot_wait_ready(ctx, frame + 1) == SA2_E_WRONG_STATE, "scene enqueued unsafe ready wait");
            }
        }
    }
    api(ctx, sa2_drain(ctx, timeout_ms));
    const auto status = sa2_scene_write_run(ctx, directory.utf8.c_str());
    char error[1024]{};
    if (status != SA2_OK) sa2_last_error(ctx, error, sizeof(error));
    const auto native = directory.read("native.json");
    // A frame slower than the label clock skips revisions, which S-B's unchanged
    // checker counts as missing. On WARP that is a property of the software
    // device, so it is reported as unsupported when skipped revisions are the only
    // failure and a traced step is longer than the clock. Any mismatch, binding
    // error or late adoption still fails, and hardware never takes this path.
    std::string skipped;
    if (warp && scene >= 3) {
        const auto& labels = native.at("label_check");
        const bool only_missing = labels.at("status").string() == "fail" && labels.at("mismatches").integer() == 0
            && labels.at("binding_mismatches").integer() == 0 && labels.at("late_adoptions").integer() == 0
            && labels.at("missing").integer() > 0;
        const bool unreached = inject && !std::get<bool>(native.at("injection_applied").value);
        if (only_missing && (!inject || unreached)) {
            int64_t previous = native.at("markers").at("trace_start_qpc").integer(), longest = 0;
            std::ifstream steps(directory.path / "trace.jsonl", std::ios::binary);
            for (std::string line; std::getline(steps, line);) {
                const auto now = Json::parse(line).at("qpc").integer();
                longest = std::max(longest, now - previous); previous = now;
            }
            const double longest_ms = double(longest) * 1000.0 / double(hz.QuadPart);
            if (longest_ms > label_clock_ms)
                skipped = "WARP frame step " + std::to_string(int64_t(longest_ms)) + " ms exceeds the "
                    + std::to_string(int64_t(label_clock_ms)) + " ms label clock; " + std::to_string(labels.at("missing").integer())
                    + " revisions skipped, 0 mismatches; the exact check runs on hardware";
        }
    }
    if (skipped.empty() && status != (inject ? SA2_E_CHECK_FAILED : SA2_OK)) {
        std::string detail = "scene run/label check returned unexpected status " + std::to_string(status)
            + " (" + error + ")";
        if (scene >= 3) detail += ", label_check " + native.at("label_check").dump();
        expect(false, detail.c_str());
    }
    const auto traced_frames = trace_stop_frame - preroll - 1;
    expect(native.at("format").string() == "magic600-sa2-scene-native-v1" && native.at("scene").string() == "w" + std::to_string(scene)
        && native.at("frames").integer() == traced_frames, "native scene/frame count mismatch");
    expect(native.at("target").at("width").integer() == width && native.at("target").at("height").integer() == height
        && native.at("target").dump() == native.at("viewport").dump(), "native resolution/viewport mismatch");
    expect(native.at("queue_mode").string() == (queue == SA2_QUEUE_OWN ? "own" : "same")
        && native.at("barrier_api").string() == (barriers == SA2_BARRIERS_ENHANCED ? "enhanced" : "legacy"), "native queue/barrier mismatch");
    if (no_vram) expect(native.at("vram_peak_mb").null() && native.at("vram_samples").integer() == 0, "NO_VRAM metadata mismatch");
    else expect(native.at("vram_peak_mb").number() > 0 && native.at("vram_samples").integer() == traced_frames, "missing process local VRAM samples");
    if (scene >= 3 && skipped.empty()) expect(native.at("label_check").at("status").string() == (inject ? "fail" : "pass"), "label status mismatch");
    if (inject && skipped.empty()) expect(std::get<bool>(native.at("injection_applied").value) && native.at("label_check").at("mismatches").integer() > 0,
        "injected GPU label corruption was not copied/detected");
    const auto begin = native.at("markers").at("trace_start_qpc").integer(), end = native.at("markers").at("trace_stop_qpc").integer();
    expect(begin <= end, "trace markers reversed");
    std::ifstream trace(directory.path / "trace.jsonl", std::ios::binary); expect(bool(trace), "trace.jsonl missing");
    uint32_t count = 0; int64_t last_qpc = begin;
    for (std::string line; std::getline(trace, line); ++count) {
        const auto row = Json::parse(line); const auto now = row.at("qpc").integer();
        expect(row.at("frame").integer() == count && now >= last_qpc && now >= begin && now <= end, "trace frame/QPC sequence mismatch");
        last_qpc = now; row.at("camera"); row.at("revision");
        expect(row.at("turn").null() == (scene != 3) && row.at("phase").null() == (scene != 3), "S-B trace turn/phase shape mismatch");
        if (scene == 3) {
            const auto turn = sb::turnAt(now, begin, sb::turnTicks(config.turn_ms, hz.QuadPart), 1);
            expect(row.at("turn").integer() == int64_t(turn.index) && std::abs(row.at("phase").number() - turn.phase) < 1e-12,
                "trace does not use the trace-start turn clock");
        }
    }
    expect(count == traced_frames, "trace record count differs from traced frames");
    expect(sa2_scene_write_run(ctx, directory.utf8.c_str()) == SA2_E_IO, "existing run outputs overwritten");
    api(ctx, sa2_scene_unload(ctx)); expect(sa2_scene_unload(ctx) == SA2_E_WRONG_STATE, "second unload succeeded");
    harness.finish(options.debug);
    if (!skipped.empty()) throw Unsupported(skipped);
}

// C9 U5's WARP check (--warp --stages). The fixtures are written by hand: the vertex stage is the built-in projection
// (d4 from its arrays) with a cut around x = 0 (code 1, singular_neighbourhood) and a turn about the z axis by t; the
// fragment stage tints the class colour. Each declares its own array buffer; the other kind's buffer comes from the
// core's guarded declaration.
constexpr char FixtureVertex[] = R"hlsl(StructuredBuffer<float> m600_vertex_arrays : register(t20);
uint m600_vertex(float x, float y, float z, float w, float t, out float3 position) {
    float d4 = m600_vertex_arrays[0], cut = m600_vertex_arrays[1], angle = m600_vertex_arrays[2] * t;
    position = float3(0, 0, 0);
    if (abs(x) <= cut) return 1u;
    float3 p = d4 * float3(x, y, z) / (d4 - w);
    float c = cos(angle), s = sin(angle);
    position = float3(c * p.x - s * p.y, s * p.x + c * p.y, p.z);
    return 0u;
}
)hlsl";
constexpr char FixtureFragment[] = R"hlsl(StructuredBuffer<float> m600_fragment_arrays : register(t21);
float3 m600_fragment(float class_, float shade, float x, float y, float z, float t) {
    float h = frac(class_ * m600_fragment_arrays[0] + 0.1 * t);
    return shade * float3(h, 1 - h, 0.5 + 0.25 * sin(x + y + z));
}
)hlsl";
constexpr char FixtureBroken[] = "uint m600_vertex(float x, float y, float z, float w, float t, out float3 position) {\n"
                                 "    return fixture_undeclared_name;\n}\n";
namespace st = sa2::stages;
std::string digest(std::string_view text) {
    return sb::sha256(std::span(reinterpret_cast<const uint8_t*>(text.data()), text.size()));
}
bool hex64(const std::string& text) {
    return text.size() == 64 && std::all_of(text.begin(), text.end(), [](char c) { return (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'); });
}
std::vector<uint32_t> bits(std::initializer_list<float> values) {
    std::vector<uint32_t> out;
    for (float value : values) out.push_back(std::bit_cast<uint32_t>(value));
    return out;
}
struct StageCompile {
    int32_t status = SA2_OK;
    uint64_t handle = 0;
    Json record;
    std::string error;
};
StageCompile compile_stage(sa2_context* ctx, uint32_t kind, std::string_view text) {
    std::vector<char> record(SA2_STAGE_RECORD_BYTES);
    uint32_t size = 0;
    StageCompile out;
    out.status = sa2_stage_compile(ctx, kind, text.data(), uint32_t(text.size()), &out.handle, record.data(), uint32_t(record.size()), &size);
    if (out.status != SA2_OK) {
        char error[8192]{};
        sa2_last_error(ctx, error, sizeof(error));
        out.error = error;
    }
    expect(size > 1 && size <= record.size() && record[size - 1] == 0 && std::strlen(record.data()) == size - 1,
        ("the compile record is missing or not NUL-terminated within its size: " + out.error).c_str());
    out.record = Json::parse(std::string(record.data(), size - 1));
    return out;
}
// The record of a compiled stage: its fields, the frozen arguments of every pass, and the SHA-256 of the composed
// source recomputed here from the same embedded core text.
void check_record(const StageCompile& compiled, uint32_t kind, std::string_view text) {
    expect(compiled.status == SA2_OK, ("a fixture stage did not compile: " + compiled.error).c_str());
    const auto& record = compiled.record;
    expect(record.at("format").string() == st::RecordFormat && record.at("kind").string() == st::kind_name(kind)
        && record.at("status").string() == "compiled" && record.at("code").null()
        && record.at("handle").integer() == int64_t(compiled.handle) && compiled.handle != 0
        && record.at("error").string().empty() && !std::get<bool>(record.at("error_truncated").value),
        "the compile record does not report a compiled stage");
    expect(record.at("stage_sha256").string() == digest(text), "stage_sha256 is not the stage text's digest");
    const auto& compiler = record.at("compiler");
    expect(compiler.at("library").string() == "dxcompiler.dll" && !compiler.at("version").string().empty()
        && compiler.at("validator").at("library").string() == "dxil.dll" && !compiler.at("validator").at("version").string().empty(),
        "the compile record does not name the compiler's and validator's versions");
    const st::CoreSources core{
        std::string_view(reinterpret_cast<const char*>(st::embedded::geometry), st::embedded::geometry_size),
        std::string_view(reinterpret_cast<const char*>(st::embedded::draw), st::embedded::draw_size),
        std::string_view(reinterpret_cast<const char*>(st::embedded::stages), st::embedded::stages_size)};
    for (const auto& [field, pass_kind] : {std::pair{"passes", kind}, std::pair{"core_passes", st::CoreKind}}) {
        const auto& rows = record.at(field).array();
        const auto expected = st::passes(pass_kind);
        expect(rows.size() == expected.size(), "the compile record lists a different set of passes");
        const auto source = digest(st::compose(pass_kind, pass_kind == st::CoreKind ? std::string_view() : text, core));
        for (size_t i = 0; i < rows.size(); ++i) {
            const auto& row = rows[i];
            std::vector<std::string> arguments;
            for (const auto& argument : row.at("arguments").array()) arguments.push_back(argument.string());
            expect(row.at("entry").string() == expected[i].entry && row.at("profile").string() == expected[i].profile
                && row.at("status").string() == "compiled" && arguments == st::arguments(expected[i]),
                "a pass record differs from the frozen profile");
            expect(row.at("source_sha256").string() == source, "source_sha256 is not the composed source's digest");
            expect(hex64(row.at("dxil_sha256").string()), "a compiled pass has no DXIL digest");
        }
    }
}
// One ring frame through the stand-in Godot's handshake; the frame has completed when consume returns.
sa2_stage_frame stage_produce(Harness& harness, uint32_t frame) {
    auto* ctx = harness.ctx;
    const auto slot = frame % SA2_RING_SLOTS;
    api(ctx, sa2_signal_godot_free(ctx, frame - 1));
    api(ctx, sa2_scene_produce(ctx, slot, frame));
    api(ctx, sa2_godot_wait_ready(ctx, frame));
    harness.consume(frame, 0, false);
    api(ctx, sa2_mark_shown(ctx, slot, frame));
    sa2_stage_frame report{};
    report.struct_size = sizeof(report);
    api(ctx, sa2_stage_report(ctx, &report));
    return report;
}
// U recomputed on the CPU over the dumped eval buffer, with its undefined vertices by coverage code.
struct DumpBound {
    uint64_t bound = 0, singular = 0, nonfinite = 0;
};
DumpBound dump_bound(const std::filesystem::path& file, uint32_t width, uint32_t height) {
    // sa2_interop.h's test hook: 16 bytes per (cell, vertex) of W1..W4's instanced list.
    std::vector<uint32_t> words(size_t(sb::Vertices) * sb::Cells * 4);
    std::ifstream in(file, std::ios::binary);
    expect(bool(in), "the eval dump was not written");
    in.read(reinterpret_cast<char*>(words.data()), std::streamsize(words.size() * 4));
    expect(size_t(in.gcount()) == words.size() * 4 && in.peek() == std::char_traits<char>::eof(), "the eval dump has the wrong size");
    DumpBound out;
    for (size_t at = 0; at < words.size(); at += 4) {
        const auto code = st::coverage_code(&words[at]);
        out.singular += code == 1;
        out.nonfinite += code == 2;
        expect(code <= 2, "an eval record holds an unknown coverage code");
    }
    out.bound = st::eval_bound(words, width, height);
    return out;
}
void scene_stages(const Options& options) {
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    auto* ctx = harness.ctx;
    constexpr uint32_t width = 640, height = 360;
    harness.make_ring(width, height);
    SceneDirectory directory("stages");
    const auto dump = directory.path / "eval.bin";
    std::vector<char> record(SA2_STAGE_RECORD_BYTES);
    uint32_t record_size = 0;
    uint64_t handle = 7;
    sa2_stage_frame report{};
    report.struct_size = sizeof(report);
    // Arguments and states before any compile.
    expect(sa2_stage_compile(ctx, SA2_STAGE_VERTEX, "", 0, &handle, record.data(), SA2_STAGE_RECORD_BYTES, &record_size) == SA2_E_INVALID_ARGUMENT
        && handle == 0 && record_size == 0, "an empty stage text was accepted");
    expect(sa2_stage_compile(ctx, 3, FixtureVertex, uint32_t(std::strlen(FixtureVertex)), &handle, record.data(), SA2_STAGE_RECORD_BYTES,
        &record_size) == SA2_E_INVALID_ARGUMENT, "an unknown stage kind was accepted");
    expect(sa2_stage_compile(ctx, SA2_STAGE_VERTEX, FixtureVertex, uint32_t(std::strlen(FixtureVertex)), &handle, record.data(),
        SA2_STAGE_RECORD_BYTES - 1, &record_size) == SA2_E_INVALID_ARGUMENT, "a record buffer below SA2_STAGE_RECORD_BYTES was accepted");
    expect(sa2_scene_stages(ctx, 0, 0, 1, nullptr, 0, nullptr, 0) == SA2_E_WRONG_STATE, "stages were attached without a scene");
    expect(sa2_stage_release(ctx, 1) == SA2_E_INVALID_ARGUMENT, "an unknown handle was released");
    api(ctx, sa2_stage_report(ctx, &report));
    expect(report.frame == 0 && report.bound == 0, "a report before any chain frame is not zero");
    // DXC at run time: the fixture pair and a broken stage.
    const auto vertex = compile_stage(ctx, SA2_STAGE_VERTEX, FixtureVertex);
    check_record(vertex, SA2_STAGE_VERTEX, FixtureVertex);
    const auto fragment = compile_stage(ctx, SA2_STAGE_FRAGMENT, FixtureFragment);
    check_record(fragment, SA2_STAGE_FRAGMENT, FixtureFragment);
    expect(vertex.handle == 1 && fragment.handle == 2, "stage handles are not numbered from 1");
    const auto broken = compile_stage(ctx, SA2_STAGE_VERTEX, FixtureBroken);
    expect(broken.status == SA2_E_CHECK_FAILED && broken.handle == 0 && broken.record.at("status").string() == "compile_failed"
        && broken.record.at("code").string() == st::CompileFailed
        && broken.record.at("error").string().find("fixture_undeclared_name") != std::string::npos
        && broken.record.at("passes").array().at(0).at("status").string() == "failed"
        && broken.error.find(st::CompileFailed) != std::string::npos
        && broken.error.find("fixture_undeclared_name") != std::string::npos,
        "a broken stage did not fail with EV3:input.stage_compile_failed and DXC's error text");
    std::cout << "stages: compiler " << vertex.record.at("compiler").at("version").string() << " (validator "
              << vertex.record.at("compiler").at("validator").at("version").string() << "); broken stage: SA2_E_CHECK_FAILED\n";
    // W3 at 640 x 360 without a chain first: the built-in path, no report row.
    sa2_scene_config config{sizeof(sa2_scene_config), 3, 3000.0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(ctx, sa2_scene_load(ctx, &config));
    const auto vertex_arrays = bits({1.18f, 0.05f, 0.1f}), fragment_arrays = bits({0.618034f});
    expect(sa2_scene_stages(ctx, fragment.handle, 0, 1, nullptr, 0, nullptr, 0) == SA2_E_INVALID_ARGUMENT
        && sa2_scene_stages(ctx, 0, vertex.handle, 1, nullptr, 0, nullptr, 0) == SA2_E_INVALID_ARGUMENT
        && sa2_scene_stages(ctx, 99, 0, 1, nullptr, 0, nullptr, 0) == SA2_E_INVALID_ARGUMENT
        && sa2_scene_stages(ctx, vertex.handle, 0, 1, nullptr, 3, nullptr, 0) == SA2_E_INVALID_ARGUMENT
        && sa2_scene_stages(ctx, vertex.handle, 0, 1, vertex_arrays.data(), SA2_STAGE_ARRAY_MAX + 1, nullptr, 0) == SA2_E_INVALID_ARGUMENT,
        "a wrong handle or array was attached");
    uint32_t frame = 1;
    expect(stage_produce(harness, frame).frame == 0, "a frame without a chain wrote a stage report");
    // The fixture chain with an allowance it cannot exceed: the user draw runs and the fallback is skipped.
    struct DumpVariable {
        ~DumpVariable() { SetEnvironmentVariableW(L"M600_SA2_STAGE_EVAL_DUMP", nullptr); }
    } dump_variable;
    win_check(SetEnvironmentVariableW(L"M600_SA2_STAGE_EVAL_DUMP", dump.c_str()), "SetEnvironmentVariableW(stage eval dump)");
    api(ctx, sa2_scene_stages(ctx, vertex.handle, fragment.handle, UINT32_MAX, vertex_arrays.data(), 3, fragment_arrays.data(), 1));
    SetEnvironmentVariableW(L"M600_SA2_STAGE_EVAL_DUMP", nullptr);
    const uint64_t pixels = uint64_t(width) * height;
    auto check_frame = [&](const sa2_stage_frame& row, uint64_t vertex_handle, uint64_t fragment_handle, uint32_t overdraw, bool stopped) {
        const auto cpu = dump_bound(dump, width, height);
        std::cout << "stages: frame " << row.frame << " U " << row.bound << " CPU " << cpu.bound << " allowance " << row.allowance
                  << " stopped " << row.stopped << " user PS " << row.user_pixel_invocations << " built-in PS "
                  << row.builtin_pixel_invocations << " singular " << cpu.singular << " nonfinite " << cpu.nonfinite << '\n';
        expect(row.frame == frame && row.vertex_stage == vertex_handle && row.fragment_stage == fragment_handle
            && row.allowance == pixels * overdraw, "the stage report names another frame, chain or allowance");
        expect(row.bound == cpu.bound && row.bound > 0, "the device's U differs from the C++ bound over the eval buffer");
        expect(row.stopped == (stopped ? 1u : 0u) && (row.bound > row.allowance) == stopped, "the frame's stop decision is wrong");
        if (stopped) expect(row.user_pixel_invocations == 0 && row.builtin_pixel_invocations > 0,
            "a stopped frame ran the user draw or skipped the fallback");
        else expect(row.user_pixel_invocations > 0 && row.builtin_pixel_invocations == 0,
            "a running frame skipped the user draw or ran the fallback");
        return cpu;
    };
    for (int i = 0; i < 3; ++i) {
        ++frame;
        const auto cpu = check_frame(stage_produce(harness, frame), vertex.handle, fragment.handle, UINT32_MAX, false);
        expect(cpu.singular > 0, "the fixture's cut left no undefined vertex");
    }
    // A forced stop (overdraw 0): predication skips the user draw and the built-in draw runs.
    api(ctx, sa2_scene_stages(ctx, vertex.handle, fragment.handle, 0, vertex_arrays.data(), 3, fragment_arrays.data(), 1));
    ++frame;
    check_frame(stage_produce(harness, frame), vertex.handle, fragment.handle, 0, true);
    // The built-in vertex stage with the user fragment stage: no cut, so no singular vertex.
    api(ctx, sa2_scene_stages(ctx, 0, fragment.handle, UINT32_MAX, nullptr, 0, fragment_arrays.data(), 1));
    ++frame;
    expect(check_frame(stage_produce(harness, frame), 0, fragment.handle, UINT32_MAX, false).singular == 0,
        "the built-in vertex stage reported a singular vertex");
    // Releases: an attached handle is refused, a free one is released once.
    expect(sa2_stage_release(ctx, fragment.handle) == SA2_E_WRONG_STATE, "an attached stage was released");
    api(ctx, sa2_stage_release(ctx, vertex.handle));
    expect(sa2_stage_release(ctx, vertex.handle) == SA2_E_INVALID_ARGUMENT
        && sa2_scene_stages(ctx, vertex.handle, fragment.handle, 1, nullptr, 0, nullptr, 0) == SA2_E_INVALID_ARGUMENT,
        "a released handle was released or attached again");
    // U5c: a traced frame draws with the chain (W3's turn included) and the chain cannot change inside the trace.
    api(ctx, sa2_scene_trace_begin(ctx));
    expect(sa2_scene_stages(ctx, 0, 0, 0, nullptr, 0, nullptr, 0) == SA2_E_WRONG_STATE, "the chain changed inside a trace");
    ++frame;
    check_frame(stage_produce(harness, frame), 0, fragment.handle, UINT32_MAX, false);
    api(ctx, sa2_scene_trace_end(ctx));
    const auto last = frame;
    // U5c: sa2_stage_report writes every chain frame once, in frame order, when several complete between two calls.
    for (uint32_t i = 0; i < 2; ++i) {
        ++frame;
        const auto slot = frame % SA2_RING_SLOTS;
        api(ctx, sa2_signal_godot_free(ctx, frame - 1));
        api(ctx, sa2_scene_produce(ctx, slot, frame));
        api(ctx, sa2_godot_wait_ready(ctx, frame));
        harness.consume(frame, 0, false);
        api(ctx, sa2_mark_shown(ctx, slot, frame));
    }
    std::vector<uint64_t> reported;
    for (int i = 0; i < 4; ++i) {
        api(ctx, sa2_stage_report(ctx, &report));
        reported.push_back(report.frame);
    }
    expect(reported == std::vector<uint64_t>{frame - 1, frame, frame, frame},
        "two completed chain frames were not reported once each in frame order");
    const auto queued = frame;
    expect(queued == last + 2, "the queued frames are not the two after the traced frame");
    // Detached: the built-in path again; then the last handle is released.
    api(ctx, sa2_scene_stages(ctx, 0, 0, 0, nullptr, 0, nullptr, 0));
    ++frame;
    expect(stage_produce(harness, frame).frame == queued, "a frame after the detach drew with the chain");
    api(ctx, sa2_stage_release(ctx, fragment.handle));
    expect(sa2_stage_release(ctx, fragment.handle) == SA2_E_INVALID_ARGUMENT, "a released fragment stage was released again");
    api(ctx, sa2_drain(ctx, timeout_ms));
    api(ctx, sa2_scene_unload(ctx));
    harness.finish(options.debug);
}

// C9 U5c's overdraw measurement (--warp --measure-stages DIR, WARP only). DIR is c600-stage-hlsl's output for the two
// stages (vertex.hlsl, vertex.json, fragment.hlsl, fragment.json) with the arrays file beside them (arrays.json:
// {"format": "magic600-stage-arrays/1", "vertex": {name: ["0x<binary32 bits>", ...]}, "fragment": {...}}). It loads W3
// at --size, attaches the pair with overdraw UINT32_MAX so that no frame stops, produces --frames untraced frames
// (the camera turns 0.002 rad in the x-w plane per frame) and then --traced frames inside a trace (W3's turns at
// --turn-ms), and writes one JSON line per frame with the device's U to --out: a measurement, never a check.
std::string file_text(const std::filesystem::path& file) {
    std::ifstream in(file, std::ios::binary);
    expect(bool(in), ("cannot read " + file.string()).c_str());
    std::ostringstream text;
    text << in.rdbuf();
    return text.str();
}
// One stage's arrays as binary32 bit patterns in the stage record's offset order; arrays.json must give every
// declared array exactly its length and name no other array.
std::vector<uint32_t> stage_arrays(const Json& stage, const Json& values) {
    std::vector<uint32_t> out;
    const auto& declared = stage.at("arrays").array();
    expect(declared.size() == values.object().size(), "arrays.json names a different set of arrays than the stage");
    for (const auto& array : declared) {
        expect(array.at("offset").integer() == int64_t(out.size()), "a stage array is not at the sum of the earlier lengths");
        const auto& given = values.at(array.at("name").string()).array();
        expect(int64_t(given.size()) == array.at("length").integer(), "an array's value count differs from its length");
        for (const auto& value : given) {
            const auto& text = value.string();
            expect(text.size() == 10 && text.rfind("0x", 0) == 0
                && text.find_first_not_of("0123456789abcdef", 2) == std::string::npos,
                "an array value is not a lowercase 0x-prefixed binary32 bit pattern");
            out.push_back(uint32_t(std::stoul(text.substr(2), nullptr, 16)));
        }
    }
    return out;
}
void measure_stages(const Options& options) {
    expect(options.mode == "warp" && !options.out.empty(), "the stage measurement runs on WARP with --out");
    constexpr DWORD measure_wait = 600000; // WARP draws W3's 6.1 million triangles at the owner's resolution
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false, nullptr, measure_wait);
    auto* ctx = harness.ctx;
    const uint32_t width = options.width, height = options.height;
    harness.make_ring(width, height);
    const auto& folder = options.measure;
    const auto values = Json::parse(file_text(folder / "arrays.json"));
    expect(values.at("format").string() == "magic600-stage-arrays/1", "arrays.json is not magic600-stage-arrays/1");
    struct Loaded {
        Json record;
        StageCompile compiled;
        std::vector<uint32_t> arrays;
    };
    auto load = [&](const char* kind, uint32_t code) {
        Loaded out;
        const auto text = file_text(folder / (std::string(kind) + ".hlsl"));
        out.record = Json::parse(file_text(folder / (std::string(kind) + ".json")));
        expect(out.record.at("kind").string() == kind && out.record.at("hlsl_sha256").string() == digest(text),
            "a stage record does not describe its HLSL file");
        out.compiled = compile_stage(ctx, code, text);
        expect(out.compiled.status == SA2_OK, ("a stage did not compile: " + out.compiled.error).c_str());
        out.arrays = stage_arrays(out.record, values.at(kind));
        return out;
    };
    const auto vertex = load("vertex", SA2_STAGE_VERTEX), fragment = load("fragment", SA2_STAGE_FRAGMENT);
    // The trace's label readback holds one copy per traced frame at most (one turn change per frame).
    const auto trace_ms = uint32_t(std::min(3600000.0, std::ceil(options.turn_ms * (options.traced + 2))));
    sa2_scene_config config{sizeof(sa2_scene_config), 3, options.turn_ms, nullptr, SA2_SCENE_NO_VRAM, std::max(trace_ms, 1u)};
    api(ctx, sa2_scene_load(ctx, &config));
    api(ctx, sa2_scene_stages(ctx, vertex.compiled.handle, fragment.compiled.handle, UINT32_MAX, vertex.arrays.data(),
        uint32_t(vertex.arrays.size()), fragment.arrays.data(), uint32_t(fragment.arrays.size())));
    std::ofstream out(options.out, std::ios::binary);
    expect(bool(out), "cannot write the measurement");
    const uint64_t pixels = uint64_t(width) * height;
    auto stage = [](const Loaded& loaded) {
        return Json(Json::Object{{"digest", loaded.record.at("digest")}, {"hlsl_sha256", loaded.record.at("hlsl_sha256")},
                                 {"overdraw", loaded.record.at("overdraw")}, {"handle", loaded.compiled.handle}});
    };
    out << Json(Json::Object{{"format", "magic600-stage-overdraw-measure/1"}, {"adapter", "warp"}, {"scene", 3},
        {"width", width}, {"height", height}, {"pixels", pixels}, {"turn_ms", options.turn_ms}, {"trace_ms", config.trace_ms},
        {"untraced_frames", options.frames}, {"traced_frames", options.traced}, {"overdraw_attached", UINT32_MAX},
        {"camera_step_rad", 0.002}, {"compiler", vertex.compiled.record.at("compiler")},
        {"vertex", stage(vertex)}, {"fragment", stage(fragment)}}).dump() << '\n' << std::flush;
    const uint32_t total = options.frames + options.traced;
    uint64_t largest = 0, largest_frame = 0;
    for (uint32_t frame = 1; frame <= total; ++frame) {
        const bool traced = frame > options.frames;
        if (traced && frame == options.frames + 1) api(ctx, sa2_scene_trace_begin(ctx));
        const auto start = std::chrono::steady_clock::now();
        const auto row = stage_produce(harness, frame);
        const double seconds = std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
        expect(row.frame == frame && row.stopped == 0 && row.user_pixel_invocations > 0 && row.builtin_pixel_invocations == 0,
            "a measured frame has no report row of its own, stopped or skipped the user draw");
        if (row.bound > largest) largest = row.bound, largest_frame = frame;
        out << Json(Json::Object{{"frame", frame}, {"traced", traced}, {"camera_steps", frame}, {"bound", row.bound},
            {"per_pixel", double(row.bound) / double(pixels)}, {"user_ps", row.user_pixel_invocations},
            {"seconds", seconds}}).dump() << '\n' << std::flush;
        if (frame % 25 == 0 || frame == total)
            std::cout << "measure: frame " << frame << '/' << total << " U " << row.bound << " (" << double(row.bound) / double(pixels)
                      << " per pixel), largest " << largest << " at frame " << largest_frame << ", " << seconds << " s\n" << std::flush;
    }
    if (options.traced) api(ctx, sa2_scene_trace_end(ctx));
    expect(bool(out), "the measurement was not written completely");
    api(ctx, sa2_drain(ctx, measure_wait));
    api(ctx, sa2_scene_unload(ctx));
    harness.finish(options.debug);
}

// C9 jumble-on's WARP check (--warp --poses): the product draw applies an engine image's per-piece poses as W-J's
// path does. Row-major 4x4 matrices in double; R poses every even piece, Q is the camera.
using Rows = std::array<double, 16>;
Rows double_rotation(int a, int b, double theta, int c, int d, double phi) {
    Rows r{};
    for (int i = 0; i < 4; ++i) r[i * 5] = 1;
    const auto plane = [&](int p, int q, double t) {
        r[p * 4 + p] = std::cos(t); r[p * 4 + q] = -std::sin(t); r[q * 4 + p] = std::sin(t); r[q * 4 + q] = std::cos(t);
    };
    plane(a, b, theta); plane(c, d, phi);
    return r;
}
Rows multiply(const Rows& x, const Rows& y) {
    Rows out{};
    for (int r = 0; r < 4; ++r) for (int c = 0; c < 4; ++c) for (int k = 0; k < 4; ++k) out[r * 4 + c] += x[r * 4 + k] * y[k * 4 + c];
    return out;
}
struct PoseFrames {
    Harness& harness;
    uint64_t frame = 0, sequence = 0;
    // One product frame into the ring slot after the last, read back as R8G8B8A8 words by pixel.
    std::vector<uint32_t> draw(uint64_t revision, uint64_t view, uint64_t set, const Rows& camera) {
        auto* ctx = harness.ctx;
        const uint64_t expected = ++frame;
        const auto slot = uint32_t(expected % SA2_RING_SLOTS);
        sa2_frame_input input{};
        input.struct_size = sizeof(input);
        input.target = slot;
        input.device_epoch = 1;
        input.surface = 1; input.surface_generation = 1; input.sequence = ++sequence;
        input.engine_revision = revision;
        input.view = view; input.view_generation = 1; input.publication_generation = 1;
        input.camera = sequence;
        input.viewport_width = harness.width; input.viewport_height = harness.height;
        input.visibility = 1; input.interaction = 1; input.resource_set = set;
        for (int r = 0; r < 4; ++r) for (int c = 0; c < 4; ++c) input.camera_matrix[c * 4 + r] = float(camera[r * 4 + c]); // column-major Q
        api(ctx, sa2_signal_godot_free(ctx, expected - 1));
        uint64_t id = 0;
        api(ctx, sa2_frame_submit(ctx, &input, &id));
        expect(id == expected, "a pose-check frame did not get the next frame ID");
        return read(id, slot);
    }
    // ABI 8: a composite frame of the panes at the engine revision; the frame input names the first pane.
    std::vector<uint32_t> draw_panes(uint64_t revision, const std::vector<sa2_pane>& panes) {
        auto* ctx = harness.ctx;
        const uint64_t expected = ++frame;
        const auto slot = uint32_t(expected % SA2_RING_SLOTS);
        sa2_frame_input input{};
        input.struct_size = sizeof(input);
        input.target = slot;
        input.device_epoch = 1;
        input.surface = 1; input.surface_generation = 1; input.sequence = ++sequence;
        input.engine_revision = revision;
        input.view = panes[0].view; input.view_generation = panes[0].view_generation; input.publication_generation = 1;
        input.camera = sequence;
        input.viewport_width = harness.width; input.viewport_height = harness.height;
        input.visibility = 1; input.interaction = 1; input.resource_set = panes[0].resource_set;
        std::copy(std::begin(panes[0].camera_matrix), std::end(panes[0].camera_matrix), std::begin(input.camera_matrix));
        api(ctx, sa2_signal_godot_free(ctx, expected - 1));
        uint64_t id = 0;
        api(ctx, sa2_frame_submit_panes(ctx, &input, panes.data(), uint32_t(panes.size()), &id));
        expect(id == expected, "a composite frame did not get the next frame ID");
        return read(id, slot);
    }
    std::vector<uint32_t> read(uint64_t id, uint32_t slot) {
        auto* ctx = harness.ctx;
        api(ctx, sa2_godot_wait_ready(ctx, id));
        harness.consume(uint32_t(id), 0, false);
        std::vector<uint32_t> pixels(size_t(harness.width) * harness.height);
        void* bytes = nullptr;
        const D3D12_RANGE read_range{static_cast<SIZE_T>(harness.footprint.Offset), static_cast<SIZE_T>(harness.readback_bytes)};
        check(harness.readback->Map(0, &read_range, &bytes), "Map pose readback");
        const auto* base = static_cast<const uint8_t*>(bytes) + harness.footprint.Offset;
        for (uint32_t y = 0; y < harness.height; ++y)
            std::memcpy(pixels.data() + size_t(y) * harness.width, base + size_t(y) * harness.footprint.Footprint.RowPitch, size_t(harness.width) * 4);
        const D3D12_RANGE written{0, 0};
        harness.readback->Unmap(0, &written);
        api(ctx, sa2_mark_shown(ctx, slot, id));
        return pixels;
    }
};
// A resource set of one view that draws the slots whose visible byte is 1; nothing pickable.
uint64_t pose_set(Harness& harness, uint64_t view, const std::vector<uint8_t>& visible) {
    auto* ctx = harness.ctx;
    const std::vector<uint8_t> interaction(SA2_SLOTS, 0);
    const std::vector<uint32_t> instances(SA2_SLOTS, 0);
    sa2_preparation preparation{};
    preparation.struct_size = sizeof(preparation);
    preparation.command = view; preparation.view = view;
    preparation.view_generation = 1; preparation.publication_generation = 1;
    preparation.device_epoch = 1; preparation.base_revision = 1;
    preparation.visibility = visible.data(); preparation.interaction = interaction.data(); preparation.instances = instances.data();
    uint64_t ticket = 0;
    api(ctx, sa2_prepare(ctx, &preparation, &ticket));
    const auto until = std::chrono::steady_clock::now() + std::chrono::milliseconds(harness.wait);
    for (;;) {
        sa2_poll_row row{sizeof(sa2_poll_row)};
        api(ctx, sa2_poll(ctx, ticket, &row));
        if (row.state == SA2_TICKET_READY) {
            const uint8_t digest[32]{};
            api(ctx, sa2_activate(ctx, row.resource_set, digest));
            return row.resource_set;
        }
        expect(row.state == SA2_TICKET_PENDING, "a pose-check preparation was cancelled or failed");
        expect(std::chrono::steady_clock::now() < until, "a pose-check preparation did not become ready in time");
        Sleep(1);
    }
}
uint64_t adopt_poses(Harness& harness, uint64_t revision, const std::vector<int32_t>& index, const std::vector<float>& poses,
                     const std::vector<int32_t>& lattice) {
    std::vector<uint32_t> labels(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) labels[slot] = slot;
    sa2_engine_image image{};
    image.struct_size = sizeof(image);
    image.pose_count = uint32_t(lattice.size());
    image.revision = revision;
    image.labels = labels.data(); image.pose_index = index.data(); image.poses = poses.data(); image.pose_lattice = lattice.data();
    uint64_t adopted = 0;
    api(harness.ctx, sa2_engine_adopt(harness.ctx, &image, &adopted));
    expect(adopted == revision, "the adopted revision is not the image's");
    return adopted;
}
void product_poses(const Options& options) {
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    const auto pieces = sa2::scene_slot_pieces();
    expect(pieces.size() == SA2_SLOTS, "the slot -> piece map does not cover every slot");
    std::vector<uint8_t> even(SA2_SLOTS), odd(SA2_SLOTS), none(SA2_SLOTS, 0);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) {
        expect(pieces[slot] < SA2_PIECES, "a slot names a piece out of range");
        even[slot] = uint8_t(pieces[slot] % 2 == 0); odd[slot] = uint8_t(1 - even[slot]);
    }
    const auto set_even = pose_set(harness, 1, even), set_odd = pose_set(harness, 2, odd), set_none = pose_set(harness, 3, none);
    const Rows R = double_rotation(0, 1, 0.7, 2, 3, 0.4), Q = double_rotation(0, 3, 0.35, 1, 2, 0.2), QR = multiply(Q, R);
    // Revision 1: pose 1 = R for every even piece (lattice -1: off the lattice), pose 0 for the rest.
    std::vector<float> table(32, 0.0f);
    for (int i = 0; i < 4; ++i) table[i * 5] = 1.0f;
    for (int i = 0; i < 16; ++i) table[16 + i] = float(R[i]);
    std::vector<int32_t> posed(SA2_PIECES);
    for (uint32_t piece = 0; piece < SA2_PIECES; ++piece) posed[piece] = int32_t(piece % 2 == 0);
    PoseFrames frames{harness};
    adopt_poses(harness, 1, posed, table, {0, -1});
    const auto a_even = frames.draw(1, 1, set_even, Q), a_odd = frames.draw(1, 2, set_odd, Q);
    // Revision 2: every piece at the identity.
    adopt_poses(harness, 2, std::vector<int32_t>(SA2_PIECES, 0), std::vector<float>(table.begin(), table.begin() + 16), {0});
    const auto b_even = frames.draw(2, 1, set_even, QR), b_odd = frames.draw(2, 2, set_odd, Q);
    const auto c_even = frames.draw(2, 1, set_even, Q), clear = frames.draw(2, 3, set_none, Q);
    const auto background = clear[0];
    expect(std::all_of(clear.begin(), clear.end(), [&](uint32_t pixel) { return pixel == background; }),
        "a frame with no visible slot is not a uniform clear");
    const auto covered = [&](const std::vector<uint32_t>& image) {
        return uint64_t(std::count_if(image.begin(), image.end(), [&](uint32_t pixel) { return pixel != background; }));
    };
    const auto differing = [](const std::vector<uint32_t>& x, const std::vector<uint32_t>& y) {
        uint64_t count = 0;
        for (size_t i = 0; i < x.size(); ++i)
            for (int shift = 0; shift < 32; shift += 8)
                if (std::abs(int((x[i] >> shift) & 0xff) - int((y[i] >> shift) & 0xff)) > 2) { ++count; break; }
        return count;
    };
    const uint64_t total = uint64_t(harness.width) * harness.height;
    const auto cover_even = covered(a_even), cover_odd = covered(a_odd), cover_b = covered(b_even);
    const auto moved = differing(a_even, b_even), unposed = differing(a_even, c_even);
    std::cout << "product poses: " << total << " pixels; covered even " << cover_even << ", odd " << cover_odd
              << ", even under Q R " << cover_b << "; posed vs camera Q R differ at " << moved << ", posed vs unposed at "
              << unposed << '\n' << std::flush;
    expect(cover_even >= total / 20 && cover_odd >= total / 20 && cover_b >= total / 20, "a pose-check frame covers less than 5% of the target");
    expect(a_odd == b_odd, "pieces at pose 0 drew differently next to posed pieces than with an identity table");
    expect(moved * 100 <= std::max(cover_even, cover_b), "posed pieces do not draw as the unposed pieces under the camera Q R (over 1% differ)");
    expect(unposed * 5 >= cover_even, "posed pieces draw like unposed ones (under 20% of their pixels differ)");
    // C9 (26 Nov 2026): sets prepared after others were retired and their frames completed take the released resources
    // (a completed upload's allocator, list and staging, and a released set's buffers) and draw as new sets do.
    api(harness.ctx, sa2_retire(harness.ctx, set_odd));
    api(harness.ctx, sa2_retire(harness.ctx, set_none));
    api(harness.ctx, sa2_drain(harness.ctx, harness.wait));
    const auto again_odd = pose_set(harness, 4, odd), again_none = pose_set(harness, 5, none);
    const auto again_even = pose_set(harness, 6, even);
    expect(frames.draw(2, 4, again_odd, Q) == b_odd, "an odd set prepared from released resources draws differently");
    expect(frames.draw(2, 5, again_none, Q) == clear, "an empty set prepared from released resources draws differently");
    expect(frames.draw(2, 6, again_even, Q) == c_even, "an even set prepared from released resources draws differently");
    api(harness.ctx, sa2_drain(harness.ctx, harness.wait));
    api(harness.ctx, sa2_scene_unload(harness.ctx));
    harness.finish(options.debug);
}

// ADR-D02 B18's WARP check (--warp --sweep): a committed twist's transform. Revision 1 has every piece at the identity;
// revision 2 is entered by a twist that turns the even pieces by R, a rotation of the plane (e0, e1). With the
// transform of that twist on revision 2, phase 1 draws revision 2 byte-identically, phase 0 draws revision 1 (within
// rounding), a half phase draws neither, a cleared transform draws revision 2 again, and refused calls change nothing.
void product_sweep(const Options& options) {
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    const auto all = pose_set(harness, 1, std::vector<uint8_t>(SA2_SLOTS, 1));
    const double angle = 0.9;
    const Rows R = double_rotation(0, 1, angle, 2, 3, 0), Q = double_rotation(0, 3, 0.35, 1, 2, 0.2);
    std::vector<float> table(32, 0.0f);
    for (int i = 0; i < 4; ++i) table[i * 5] = 1.0f;
    for (int i = 0; i < 16; ++i) table[16 + i] = float(R[i]);
    std::vector<int32_t> posed(SA2_PIECES);
    std::vector<uint8_t> moving(SA2_PIECES);
    for (uint32_t piece = 0; piece < SA2_PIECES; ++piece) { posed[piece] = int32_t(piece % 2 == 0); moving[piece] = uint8_t(piece % 2 == 0); }
    PoseFrames frames{harness};
    adopt_poses(harness, 1, std::vector<int32_t>(SA2_PIECES, 0), std::vector<float>(table.begin(), table.begin() + 16), {0});
    const auto before = frames.draw(1, 1, all, Q);
    adopt_poses(harness, 2, posed, table, {0, -1});
    const auto after = frames.draw(2, 1, all, Q);
    const float u[4] = {1, 0, 0, 0}, v[4] = {0, 1, 0, 0};
    auto* ctx = harness.ctx;
    const auto refused = [&](int32_t status, const char* what) { expect(status == SA2_E_INVALID_ARGUMENT, what); };
    refused(sa2_transform_set(ctx, 1, u, v, float(angle), moving.data(), 0.0f), "a transform of another revision was not refused");
    refused(sa2_transform_set(ctx, 2, u, u, float(angle), moving.data(), 0.0f), "a degenerate plane was not refused");
    refused(sa2_transform_set(ctx, 2, u, v, float(angle), moving.data(), 1.5f), "a phase above 1 was not refused");
    refused(sa2_transform_set(ctx, 2, u, v, 4.0f, moving.data(), 0.0f), "an angle above pi was not refused");
    std::vector<uint8_t> bad(moving); bad[3] = 2;
    refused(sa2_transform_set(ctx, 2, u, v, float(angle), bad.data(), 0.0f), "a moving byte of 2 was not refused");
    expect(frames.draw(2, 1, all, Q) == after, "a refused transform changed the frame");
    api(ctx, sa2_transform_set(ctx, 2, u, v, float(angle), moving.data(), 1.0f));
    const auto one = frames.draw(2, 1, all, Q);
    api(ctx, sa2_transform_set(ctx, 2, u, v, float(angle), moving.data(), 0.0f));
    const auto zero = frames.draw(2, 1, all, Q);
    api(ctx, sa2_transform_set(ctx, 2, u, v, float(angle), moving.data(), 0.5f));
    const auto half = frames.draw(2, 1, all, Q);
    api(ctx, sa2_transform_set(ctx, 2, u, v, float(angle), nullptr, 0.0f));
    const auto cleared = frames.draw(2, 1, all, Q);
    const auto background = before[0];
    const auto covered = [&](const std::vector<uint32_t>& image) {
        return uint64_t(std::count_if(image.begin(), image.end(), [&](uint32_t pixel) { return pixel != background; }));
    };
    const auto differing = [](const std::vector<uint32_t>& x, const std::vector<uint32_t>& y) {
        uint64_t count = 0;
        for (size_t i = 0; i < x.size(); ++i)
            for (int shift = 0; shift < 32; shift += 8)
                if (std::abs(int((x[i] >> shift) & 0xff) - int((y[i] >> shift) & 0xff)) > 2) { ++count; break; }
        return count;
    };
    const auto cover = covered(before), apart = differing(before, after), back = differing(zero, before);
    const auto half_before = differing(half, before), half_after = differing(half, after);
    std::cout << "transform: covered " << cover << "; revisions differ at " << apart << ", phase 0 vs revision 1 at " << back
              << ", phase 0.5 vs revision 1 at " << half_before << " and vs revision 2 at " << half_after << '\n' << std::flush;
    expect(cover >= uint64_t(harness.width) * harness.height / 20, "a transform-check frame covers less than 5% of the target");
    expect(apart * 5 >= cover, "the twist moved too little to check (under 20% of the covered pixels differ)");
    expect(one == after, "phase 1 does not draw the revision byte-identically");
    expect(back * 100 <= cover, "phase 0 does not draw the revision before the twist (over 1% differ)");
    expect(half_before * 10 >= cover && half_after * 10 >= cover, "phase 0.5 draws like an endpoint (under 10% differ)");
    expect(cleared == after, "a cleared transform does not draw the revision byte-identically");
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    harness.finish(options.debug);
}

// ABI 6's WARP check (--warp --shape): a view's shape. The default shape draws byte-identically to no call, a
// smaller sticker shrink covers fewer pixels, the owner's opening values draw, and a refused shape changes nothing.
void product_shape(const Options& options) {
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    auto* ctx = harness.ctx;
    expect(sa2_view_shape_set(nullptr, 1, nullptr) == SA2_E_INVALID_ARGUMENT, "a shape was set without a context");
    std::vector<uint32_t> labels(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) labels[slot] = slot;
    sa2_engine_image image{};
    image.struct_size = sizeof(image);
    image.revision = 1;
    image.labels = labels.data();
    uint64_t adopted = 0;
    api(ctx, sa2_engine_adopt(ctx, &image, &adopted));
    PoseFrames frames{harness};
    const Rows Q = double_rotation(0, 3, 0.35, 1, 2, 0.2);
    const auto unset = frames.draw(1, 1, 0, Q);
    const sa2_view_shape defaults{sizeof(sa2_view_shape), SA2_SHAPE_DEFAULT_CELL_SHRINK, SA2_SHAPE_DEFAULT_STICKER_SHRINK,
                                  SA2_SHAPE_DEFAULT_D4, SA2_SHAPE_DEFAULT_ZOOM, 0};
    api(ctx, sa2_view_shape_set(ctx, 1, &defaults));
    expect(frames.draw(1, 1, 0, Q) == unset, "the default shape does not draw byte-identically to no call");
    const auto background = unset[0];
    const auto covered = [&](const std::vector<uint32_t>& image) {
        return uint64_t(std::count_if(image.begin(), image.end(), [&](uint32_t pixel) { return pixel != background; }));
    };
    auto smaller = defaults;
    smaller.sticker_shrink = 0.55f;
    api(ctx, sa2_view_shape_set(ctx, 1, &smaller));
    const auto reduced = frames.draw(1, 1, 0, Q);
    expect(frames.draw(1, 2, 0, Q) == unset, "another view drew with view 1's shape");
    auto opening = defaults; // the owner's opening values: gap 0.3, sticker shrink 0.55
    opening.cell_shrink = 1.0f - 0.3f;
    opening.sticker_shrink = 0.55f;
    api(ctx, sa2_view_shape_set(ctx, 2, &opening));
    const auto open = frames.draw(1, 2, 0, Q);
    const auto cover_unset = covered(unset), cover_small = covered(reduced), cover_open = covered(open);
    std::cout << "view shape: " << uint64_t(harness.width) * harness.height << " pixels; covered by default " << cover_unset
              << ", sticker shrink 0.55 " << cover_small << ", opening values " << cover_open << std::endl;
    expect(cover_unset >= uint64_t(harness.width) * harness.height / 20, "the default frame covers less than 5% of the target");
    expect(cover_small > 0 && cover_small < cover_unset, "a smaller sticker shrink does not cover fewer pixels");
    expect(cover_open > 0 && cover_open < cover_unset, "the opening values do not cover fewer pixels than the defaults");
    // Refusals change nothing.
    const float nan = std::numeric_limits<float>::quiet_NaN();
    for (int field = 0; field < 8; ++field) {
        auto bad = smaller;
        switch (field) {
        case 0: bad.cell_shrink = nan; break;
        case 1: bad.sticker_shrink = std::numeric_limits<float>::infinity(); break;
        case 2: bad.d4 = nan; break;
        case 3: bad.zoom = nan; break;
        case 4: bad.sticker_shrink = 0; break;
        case 5: bad.d4 = 1.0f; break;
        case 6: bad.zoom = SA2_SHAPE_ZOOM_MAX * 2; break;
        default: bad.struct_size = 20; break;
        }
        expect(sa2_view_shape_set(ctx, 1, &bad) == SA2_E_INVALID_ARGUMENT, "a non-finite or out-of-range shape was accepted");
    }
    expect(frames.draw(1, 1, 0, Q) == reduced, "a refused shape changed the view's frames");
    api(ctx, sa2_view_shape_set(ctx, 1, nullptr));
    expect(frames.draw(1, 1, 0, Q) == unset, "a NULL shape did not restore the defaults");
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    expect(sa2_view_shape_set(ctx, 1, &smaller) == SA2_E_WRONG_STATE, "a shape was set without a loaded scene");
    harness.finish(options.debug);
}

// ABI 9's WARP check (--warp --focus): a view's focus (ADR-D02 B18c, Local's own shape). The default focus draws
// byte-identically to no call; Local's shape with the pole of cell 150 turned to +w draws the whole view, the cap test
// (121/125) draws fewer pixels around the target's centre, the spread changes the picture, a later shape keeps the
// focus, a refused focus changes nothing and NULL restores the default. The three pictures go beside the report.
void product_focus(const Options& options) {
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    auto* ctx = harness.ctx;
    expect(sa2_view_focus_set(nullptr, 1, nullptr) == SA2_E_INVALID_ARGUMENT, "a focus was set without a context");
    std::vector<uint32_t> labels(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) labels[slot] = slot;
    sa2_engine_image image{};
    image.struct_size = sizeof(image);
    image.revision = 1;
    image.labels = labels.data();
    uint64_t adopted = 0;
    api(ctx, sa2_engine_adopt(ctx, &image, &adopted));
    PoseFrames frames{harness};
    const Rows Q = double_rotation(0, 3, 0.35, 1, 2, 0.2), I = double_rotation(0, 3, 0, 1, 2, 0);
    const auto unset = frames.draw(1, 1, 0, Q);
    const sa2_view_focus defaults{sizeof(sa2_view_focus), SA2_PROJECTION_PERSPECTIVE, SA2_FOCUS_NONE, -1.0f, 1.0f, 0};
    api(ctx, sa2_view_focus_set(ctx, 1, &defaults));
    expect(frames.draw(1, 1, 0, Q) == unset, "the default focus does not draw byte-identically to no call");
    const auto background = unset[0];
    const auto covered = [&](const std::vector<uint32_t>& picture) {
        return uint64_t(std::count_if(picture.begin(), picture.end(), [&](uint32_t pixel) { return pixel != background; }));
    };
    // Local's shape from local-view.json (gap 0.3, sticker shrink 0.6, d4 2) on view 2, zoomed to the cap.
    const sa2_view_shape local{sizeof(sa2_view_shape), 0.7f, 0.6f, 2.0f, 4.5f, 0};
    api(ctx, sa2_view_shape_set(ctx, 2, &local));
    const auto shaped = frames.draw(1, 2, 0, I);
    sa2_view_focus whole{sizeof(sa2_view_focus), SA2_PROJECTION_ORTHOGRAPHIC, 150, -1.0f, 1.0f, 0};
    api(ctx, sa2_view_focus_set(ctx, 2, &whole));
    const auto centred = frames.draw(1, 2, 0, I);
    auto cap = whole;
    cap.focus_cos = 121.0f / 125.0f;
    api(ctx, sa2_view_focus_set(ctx, 2, &cap));
    const auto only = frames.draw(1, 2, 0, I);
    auto apart = cap;
    apart.spread = 1.7f;
    api(ctx, sa2_view_focus_set(ctx, 2, &apart));
    const auto spread = frames.draw(1, 2, 0, I);
    expect(frames.draw(1, 1, 0, Q) == unset, "another view drew with view 2's focus");
    const auto cover_whole = covered(centred), cover_cap = covered(only), cover_spread = covered(spread);
    // The cap's covered pixels lie around the target's centre: their bounding box holds it.
    int left = int(harness.width), right = -1, top = int(harness.height), bottom = -1;
    for (uint32_t y = 0; y < harness.height; ++y)
        for (uint32_t x = 0; x < harness.width; ++x)
            if (only[size_t(y) * harness.width + x] != background) {
                left = std::min(left, int(x)); right = std::max(right, int(x)); top = std::min(top, int(y)); bottom = std::max(bottom, int(y));
            }
    std::cout << "view focus: " << uint64_t(harness.width) * harness.height << " pixels; covered whole " << cover_whole << ", cap "
              << cover_cap << ", cap spread " << cover_spread << "; the cap spans x " << left << ".." << right << ", y " << top
              << ".." << bottom << std::endl;
    expect(centred != shaped, "the pole turned to +w and the orthographic projection changed nothing");
    expect(cover_cap > 0 && cover_cap < cover_whole, "the cap test does not draw fewer pixels than the whole view");
    expect(left < int(harness.width) / 2 && right > int(harness.width) / 2 && top < int(harness.height) / 2 && bottom > int(harness.height) / 2,
           "the cap is not centred on the target");
    expect(cover_spread > 0 && spread != only, "the spread changed nothing");
    // A shape set later keeps the focus; a refused focus changes nothing; NULL restores the default and keeps the shape.
    api(ctx, sa2_view_shape_set(ctx, 2, &local));
    expect(frames.draw(1, 2, 0, I) == spread, "a shape set dropped the view's focus");
    const float nan = std::numeric_limits<float>::quiet_NaN();
    for (int field = 0; field < 8; ++field) {
        auto bad = apart;
        switch (field) {
        case 0: bad.struct_size = 20; break;
        case 1: bad.projection = 3; break; // ADR-D02 B21: 2 is the raw view's stereographic projection
        case 2: bad.focus_cell = 600; break;
        case 3: bad.focus_cos = nan; break;
        case 4: bad.focus_cos = 1.5f; break;
        case 5: bad.spread = 0.5f; break;
        case 6: bad.spread = 3.5f; break;
        default: bad.reserved = 1; break;
        }
        expect(sa2_view_focus_set(ctx, 2, &bad) == SA2_E_INVALID_ARGUMENT, "a malformed or out-of-range focus was accepted");
    }
    expect(frames.draw(1, 2, 0, I) == spread, "a refused focus changed the view's frames");
    api(ctx, sa2_view_focus_set(ctx, 2, nullptr));
    expect(frames.draw(1, 2, 0, I) == shaped, "a NULL focus did not restore the default");
    if (!options.out.empty()) {
        const auto write = [&](const std::vector<uint32_t>& picture, const char* name) {
            std::ofstream file(options.out.parent_path() / name, std::ios::binary);
            file << "P6\n" << harness.width << " " << harness.height << "\n255\n";
            for (uint32_t pixel : picture) {
                const char rgb[3] = {char(pixel & 0xff), char((pixel >> 8) & 0xff), char((pixel >> 16) & 0xff)};
                file.write(rgb, 3);
            }
        };
        write(centred, "warp-focus-whole.ppm"); write(only, "warp-focus-cap.ppm"); write(spread, "warp-focus-spread.ppm");
    }
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    expect(sa2_view_focus_set(ctx, 2, &apart) == SA2_E_WRONG_STATE, "a focus was set without a loaded scene");
    harness.finish(options.debug);
}

// ABI 7 (ADR-D02 B14): --warp --style. Over the unposed home labels at the scene's size: a fill tint and an outline on
// a quarter of the pieces change only pixels those pieces cover (a frame of only those pieces bounds them); an empty
// mask, and a slot whose style changes nothing, draw byte-identically to no style; a style for another revision is
// refused; a focus dim and a colour matrix change frames without a piece style; clearing restores the unstyled frame.
void product_style(const Options& options) {
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    auto* ctx = harness.ctx;
    std::vector<uint32_t> labels(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) labels[slot] = slot;
    sa2_engine_image image{};
    image.struct_size = sizeof(image);
    image.revision = 1;
    image.labels = labels.data();
    uint64_t adopted = 0;
    api(ctx, sa2_engine_adopt(ctx, &image, &adopted));
    const auto pieces = sa2::scene_slot_pieces();
    std::vector<uint8_t> masked(SA2_SLOTS, 0), all(SA2_SLOTS, 1);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) masked[slot] = uint8_t(pieces[slot] % 4 == 1);
    const auto set_all = pose_set(harness, 1, all), set_masked = pose_set(harness, 2, masked);
    PoseFrames frames{harness};
    const Rows Q = double_rotation(0, 3, 0.35, 1, 2, 0.2);
    const auto unstyled = frames.draw(1, 1, set_all, Q);
    const auto only = frames.draw(1, 2, set_masked, Q);
    const auto background = unstyled[0];
    const uint64_t total = uint64_t(harness.width) * harness.height;
    uint64_t cover_only = 0;
    for (const auto pixel : only) cover_only += pixel != background;
    expect(cover_only >= total / 100, "the masked pieces cover less than 1% of the target");
    // A slot whose style changes nothing, and an empty mask, keep the unstyled frame byte for byte.
    sa2_view_style style{};
    style.struct_size = sizeof(style);
    style.tint_mode = SA2_STYLE_TINT_FILL | SA2_STYLE_TINT_OUTLINE;
    style.outline_width = 0.12f;
    const float identity[9] = {1, 0, 0, 0, 1, 0, 0, 0, 1};
    std::copy(std::begin(identity), std::end(identity), std::begin(style.matrix));
    const float red[4] = {0.95f, 0.1f, 0.1f, 1}, black[4] = {0, 0, 0, 1};
    std::copy(std::begin(red), std::end(red), style.palette + 4 * 3);
    std::copy(std::begin(black), std::end(black), style.palette + 4 * 5);
    api(ctx, sa2_view_style_set(ctx, 1, &style));
    expect(frames.draw(1, 1, set_all, Q) == unstyled, "a slot that changes nothing does not draw byte-identically");
    std::vector<uint8_t> fill(SA2_PIECES, 0), outline(SA2_PIECES, 0);
    api(ctx, sa2_style_set(ctx, 1, fill.data(), SA2_STYLE_CHANNEL_FILL));
    expect(frames.draw(1, 1, set_all, Q) == unstyled, "an empty mask does not draw byte-identically to no style");
    expect(sa2_style_set(ctx, 2, fill.data(), SA2_STYLE_CHANNEL_FILL) == SA2_E_INVALID_ARGUMENT, "a style for another revision was accepted");
    // The fill tint and the outline on every piece = 1 (mod 4).
    for (uint32_t piece = 0; piece < SA2_PIECES; ++piece) {
        if (piece % 4 != 1) continue;
        fill[piece] = 3;
        outline[piece] = 5;
    }
    api(ctx, sa2_style_set(ctx, 1, fill.data(), SA2_STYLE_CHANNEL_FILL));
    const auto tinted = frames.draw(1, 1, set_all, Q);
    api(ctx, sa2_style_set(ctx, 1, outline.data(), SA2_STYLE_CHANNEL_OUTLINE));
    const auto outlined = frames.draw(1, 1, set_all, Q);
    uint64_t changed = 0, stray = 0, lined = 0;
    for (size_t i = 0; i < unstyled.size(); ++i) {
        const bool differs = tinted[i] != unstyled[i];
        changed += differs;
        stray += (differs || outlined[i] != unstyled[i]) && only[i] == background;
        lined += outlined[i] != tinted[i];
    }
    std::cout << "piece style: " << total << " pixels; masked pieces cover " << cover_only << ", the tint changed " << changed
              << ", the outline changed " << lined << ", outside the masked pieces " << stray << std::endl;
    expect(changed * 8 >= cover_only, "the fill tint changed under an eighth of the masked pieces' pixels");
    expect(lined > 0, "the outline changed no pixel");
    expect(stray == 0, "a styled frame changed a pixel no masked piece covers");
    // Another revision's style is gone after an adoption: the frame draws unstyled again.
    image.revision = 2; // the same labels, a new revision
    api(ctx, sa2_engine_adopt(ctx, &image, &adopted));
    expect(frames.draw(2, 1, set_all, Q) == unstyled, "an adoption kept the previous revision's style");
    expect(sa2_style_set(ctx, 1, fill.data(), SA2_STYLE_CHANNEL_FILL) == SA2_E_INVALID_ARGUMENT, "a style for a replaced revision was accepted");
    // Without a piece style, a focus dim and a colour matrix still change the frame; clearing the slot restores it.
    style.dim_mode = SA2_STYLE_DIM_UNMARKED;
    style.dim_amount = 0.5f;
    api(ctx, sa2_view_style_set(ctx, 1, &style));
    expect(frames.draw(2, 1, set_all, Q) != unstyled, "a focus dim did not change the frame");
    style.dim_mode = SA2_STYLE_DIM_NONE;
    style.dim_amount = 0;
    const float swap[9] = {0, 1, 0, 1, 0, 0, 0, 0, 1};
    std::copy(std::begin(swap), std::end(swap), std::begin(style.matrix));
    api(ctx, sa2_view_style_set(ctx, 1, &style));
    expect(frames.draw(2, 1, set_all, Q) != unstyled, "a colour matrix did not change the frame");
    style.matrix[0] = 9.0f;
    expect(sa2_view_style_set(ctx, 1, &style) == SA2_E_INVALID_ARGUMENT, "a matrix entry out of range was accepted");
    expect(sa2_view_style_set(ctx, 5, nullptr) == SA2_E_INVALID_ARGUMENT, "a fifth style slot was accepted");
    api(ctx, sa2_view_style_set(ctx, 1, nullptr));
    expect(frames.draw(2, 1, set_all, Q) == unstyled, "a cleared slot did not restore the unstyled frame");
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    expect(sa2_view_style_set(ctx, 1, nullptr) == SA2_E_WRONG_STATE, "a style was set without a loaded scene");
    harness.finish(options.debug);
}

// ABI 8 (ADR-D02 B18a): --warp --panes. Two one-view frames at 300 x 300 (harness A), then composite frames of the
// same two views at 640 x 360 (harness B): a single pane covering the target draws byte-identically to
// sa2_frame_submit; each of two panes matches its one-view frame (at most 1% of covered pixels differ by more than 2
// in a channel); pixels outside every pane hold the clear colour; overlap, a fifth pane and a mixed revision are
// refused; a pick in each pane names its view and the instance the one-view pick returns; the composite frame's
// vertex evaluations are the sum of the two one-view frames'.
namespace panes_check {
constexpr uint32_t Side = 300;
uint64_t pickable_set(Harness& harness, uint64_t view, uint64_t revision) {
    auto* ctx = harness.ctx;
    const std::vector<uint8_t> visible(SA2_SLOTS, 1), interaction(SA2_SLOTS, 1);
    std::vector<uint32_t> instances(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) instances[slot] = slot + 1;
    sa2_preparation preparation{};
    preparation.struct_size = sizeof(preparation);
    preparation.command = view; preparation.view = view;
    preparation.view_generation = 1; preparation.publication_generation = 1;
    preparation.device_epoch = 1; preparation.base_revision = revision;
    preparation.visibility = visible.data(); preparation.interaction = interaction.data(); preparation.instances = instances.data();
    uint64_t ticket = 0;
    api(ctx, sa2_prepare(ctx, &preparation, &ticket));
    const auto until = std::chrono::steady_clock::now() + std::chrono::milliseconds(harness.wait);
    for (;;) {
        sa2_poll_row row{sizeof(sa2_poll_row)};
        api(ctx, sa2_poll(ctx, ticket, &row));
        if (row.state == SA2_TICKET_READY) {
            const uint8_t digest[32]{};
            api(ctx, sa2_activate(ctx, row.resource_set, digest));
            return row.resource_set;
        }
        expect(row.state == SA2_TICKET_PENDING, "a pane-check preparation was cancelled or failed");
        expect(std::chrono::steady_clock::now() < until, "a pane-check preparation timed out");
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
}
sa2_pick_row pick(Harness& harness, uint64_t frame, uint32_t x, uint32_t y) {
    auto* ctx = harness.ctx;
    uint64_t ticket = 0;
    api(ctx, sa2_pick_request(ctx, frame, x, y, &ticket));
    const auto until = std::chrono::steady_clock::now() + std::chrono::milliseconds(harness.wait);
    for (;;) {
        sa2_pick_row row{};
        row.struct_size = sizeof(row);
        api(ctx, sa2_pick_poll(ctx, ticket, &row));
        if (row.state != SA2_PICK_PENDING) return row;
        expect(std::chrono::steady_clock::now() < until, "a pane-check pick timed out");
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
}
std::map<uint64_t, uint64_t> evals(Harness& harness) {
    std::map<uint64_t, uint64_t> out;
    sa2_frame_report rows[16]{};
    for (auto& row : rows) row.struct_size = sizeof(row);
    uint32_t count = 0;
    api(harness.ctx, sa2_frame_reports(harness.ctx, rows, 16, &count));
    for (uint32_t i = 0; i < count; ++i) out[rows[i].frame_id] = rows[i].view_vertex_evals;
    return out;
}
void load(Harness& harness, uint32_t width, uint32_t height) {
    harness.make_ring(width, height);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    std::vector<uint32_t> labels(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) labels[slot] = slot;
    sa2_engine_image image{};
    image.struct_size = sizeof(image);
    image.revision = 1;
    image.labels = labels.data();
    uint64_t adopted = 0;
    api(harness.ctx, sa2_engine_adopt(harness.ctx, &image, &adopted));
}
// The covered pixel of a one-view frame nearest its centre.
std::pair<uint32_t, uint32_t> covered_point(const std::vector<uint32_t>& image) {
    std::pair<uint32_t, uint32_t> best{Side / 2, Side / 2};
    uint64_t nearest = UINT64_MAX;
    for (uint32_t y = 0; y < Side; ++y)
        for (uint32_t x = 0; x < Side; ++x) {
            if (image[size_t(y) * Side + x] == image[0]) continue;
            const int64_t dx = int64_t(x) - Side / 2, dy = int64_t(y) - Side / 2;
            if (uint64_t(dx * dx + dy * dy) < nearest) { nearest = uint64_t(dx * dx + dy * dy); best = {x, y}; }
        }
    return best;
}
sa2_pane make(uint64_t view, uint64_t set, int32_t x, int32_t y, uint32_t w, uint32_t h, const Rows& camera) {
    sa2_pane pane{};
    pane.view = view; pane.view_generation = 1; pane.resource_set = set;
    pane.x = x; pane.y = y; pane.width = w; pane.height = h;
    for (int r = 0; r < 4; ++r) for (int c = 0; c < 4; ++c) pane.camera_matrix[c * 4 + r] = float(camera[r * 4 + c]);
    return pane;
}
}

// ADR-D02 B17 (ADR-D01.46, added to ABI 10): --warp --style also checks the focus layers drawn in the core. The pieces
// = 1 (mod 4) are marked (SA2_STYLE_DIM in their fill byte). Hiding them draws what a set without them draws (at most
// 1% of the covered pixels differ by more than 2 in a channel). A pick follows the style its frame drew with: where the
// unstyled frame shows a marked piece, the hiding frame's pick names an unmarked piece or none (at least once the one
// behind) and the dimming frame's names none; where it shows an unmarked piece, both answer as the unstyled pick does,
// as an outline-only style does everywhere. A single styled pane covering the target picks as the one-view frame, and
// a styled frame whose engine copy has taken another piece style answers SA2_PICK_STALE_ENGINE.
void product_focus_layers(const Options& options) {
    using panes_check::pick;
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    auto* ctx = harness.ctx;
    std::vector<uint32_t> labels(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) labels[slot] = slot;
    sa2_engine_image image{};
    image.struct_size = sizeof(image);
    image.revision = 1;
    image.labels = labels.data();
    uint64_t adopted = 0;
    api(ctx, sa2_engine_adopt(ctx, &image, &adopted));
    const auto pieces = sa2::scene_slot_pieces();
    const auto marked = [&](uint32_t instance) { return instance != 0 && pieces[instance - 1] % 4 == 1; };
    std::vector<uint8_t> rest(SA2_SLOTS, 0);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) rest[slot] = uint8_t(pieces[slot] % 4 != 1);
    const auto set_pick = panes_check::pickable_set(harness, 1, 1), set_rest = pose_set(harness, 2, rest);
    PoseFrames frames{harness};
    const Rows Q = double_rotation(0, 3, 0.35, 1, 2, 0.2);
    const auto unstyled = frames.draw(1, 1, set_pick, Q);
    const uint64_t unstyled_frame = frames.frame;
    const auto rest_only = frames.draw(1, 2, set_rest, Q); // drawn before any style, through the unchanged pipeline
    const auto background = unstyled[0];
    // Up to eight sample points where the unstyled pick names a marked piece and eight where it names another.
    std::vector<std::pair<uint32_t, uint32_t>> points;
    std::vector<uint32_t> answers;
    uint32_t on_marked = 0, on_rest = 0, drawn = 0, empty = 0;
    for (uint32_t y = 3; y < harness.height && (on_marked < 8 || on_rest < 8); y += 7)
        for (uint32_t x = 5; x < harness.width && (on_marked < 8 || on_rest < 8); x += 11) {
            if (unstyled[size_t(y) * harness.width + x] == background) continue;
            ++drawn;
            const auto row = pick(harness, unstyled_frame, x, y);
            expect(row.state == SA2_PICK_READY, "an unstyled pick was not ready");
            if (row.instance == 0) { ++empty; continue; }
            uint32_t& count = marked(row.instance) ? on_marked : on_rest;
            if (count >= 8) continue;
            ++count;
            points.push_back({x, y});
            answers.push_back(row.instance);
        }
    std::cout << "focus layers: " << drawn << " drawn sample points, " << empty << " without an instance; " << on_marked
              << " on marked pieces, " << on_rest << " on others" << std::endl;
    expect(on_marked == 8 && on_rest == 8, "the unstyled frame offers too few sample points");
    const auto picks = [&](uint64_t frame) {
        std::vector<uint32_t> out;
        for (const auto& [x, y] : points) {
            const auto row = pick(harness, frame, x, y);
            expect(row.state == SA2_PICK_READY, "a pick on a styled frame was not ready");
            out.push_back(row.instance);
        }
        return out;
    };
    std::vector<uint8_t> fill(SA2_PIECES, 0), outline(SA2_PIECES, 0);
    for (uint32_t piece = 0; piece < SA2_PIECES; ++piece) {
        if (piece % 4 != 1) continue;
        fill[piece] = SA2_STYLE_DIM;
        outline[piece] = 5;
    }
    api(ctx, sa2_style_set(ctx, 1, fill.data(), SA2_STYLE_CHANNEL_FILL));
    sa2_view_style style{};
    style.struct_size = sizeof(style);
    const float identity[9] = {1, 0, 0, 0, 1, 0, 0, 0, 1}, black[4] = {0, 0, 0, 1};
    std::copy(std::begin(identity), std::end(identity), std::begin(style.matrix));
    std::copy(std::begin(black), std::end(black), style.palette + 4 * 5);
    style.dim_mode = 5;
    expect(sa2_view_style_set(ctx, 1, &style) == SA2_E_INVALID_ARGUMENT, "an unknown dim mode was accepted");
    // Hidden: the picture of the set without the marked pieces, and no pick names one.
    style.dim_mode = SA2_STYLE_HIDE_MARKED;
    api(ctx, sa2_view_style_set(ctx, 1, &style));
    const auto hidden = frames.draw(1, 1, set_pick, Q);
    const uint64_t hidden_frame = frames.frame;
    const auto hidden_picks = picks(hidden_frame);
    uint64_t covered = 0, apart = 0;
    for (size_t i = 0; i < hidden.size(); ++i) {
        covered += rest_only[i] != background || hidden[i] != background;
        bool distant = false; // (windows.h defines far)
        for (int shift = 0; shift < 32; shift += 8) {
            const int a = int((hidden[i] >> shift) & 0xFF), b = int((rest_only[i] >> shift) & 0xFF);
            distant = distant || std::abs(a - b) > 2;
        }
        apart += distant;
    }
    uint32_t revealed = 0;
    for (size_t i = 0; i < points.size(); ++i) {
        expect(!marked(hidden_picks[i]), "a hidden piece was picked");
        if (!marked(answers[i])) expect(hidden_picks[i] == answers[i], "hiding changed the pick of a piece in front");
        else revealed += hidden_picks[i] != 0;
    }
    std::cout << "focus layers: hiding the marked pieces covers " << covered << " pixels, " << apart
              << " apart from the set without them; " << revealed << " of 8 hidden points pick the piece behind" << std::endl;
    expect(covered > 0 && apart * 100 <= covered, "hiding the marked pieces does not draw the set without them");
    expect(revealed > 0, "no hidden point picked the piece behind it");
    // A single styled pane covering the target picks as the one-view frame.
    auto pane = panes_check::make(1, set_pick, 0, 0, harness.width, harness.height, Q);
    pane.style = 1;
    frames.draw_panes(1, {pane});
    expect(picks(frames.frame) == hidden_picks, "a styled pane does not pick as its one-view frame");
    // Dimmed: drawn, and no pick names a marked piece in front.
    style.dim_mode = SA2_STYLE_DIM_MARKED;
    style.dim_amount = 0.75f;
    api(ctx, sa2_view_style_set(ctx, 1, &style));
    expect(frames.draw(1, 1, set_pick, Q) != unstyled, "the dim did not change the frame");
    const auto dimmed_picks = picks(frames.frame);
    for (size_t i = 0; i < points.size(); ++i)
        expect(dimmed_picks[i] == (marked(answers[i]) ? 0u : answers[i]), "a dimmed piece was picked or a shown one was not");
    // An outline alone leaves every pick as it was.
    style.dim_mode = SA2_STYLE_DIM_NONE;
    style.dim_amount = 0;
    style.tint_mode = SA2_STYLE_TINT_OUTLINE;
    style.outline_width = 0.12f;
    api(ctx, sa2_view_style_set(ctx, 1, &style));
    api(ctx, sa2_style_set(ctx, 1, outline.data(), SA2_STYLE_CHANNEL_OUTLINE));
    expect(frames.draw(1, 1, set_pick, Q) != unstyled, "the outline did not change the frame");
    expect(picks(frames.frame) == answers, "an outline changed a pick");
    // Three more piece styles take every engine copy; the hiding frame's copy no longer holds its style.
    for (uint8_t entry = 6; entry <= 8; ++entry) {
        outline[1] = entry;
        api(ctx, sa2_style_set(ctx, 1, outline.data(), SA2_STYLE_CHANNEL_OUTLINE));
        frames.draw(1, 1, set_pick, Q);
    }
    const auto stale = pick(harness, hidden_frame, points[0].first, points[0].second);
    expect(stale.state == SA2_PICK_STALE && stale.stale == SA2_PICK_STALE_ENGINE, "a pick on a styled frame whose copy changed style was not stale");
    expect(pick(harness, unstyled_frame, points[0].first, points[0].second).instance == answers[0], "an unstyled frame's pick changed");
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    harness.finish(options.debug);
}

// B18 review finding B18-03 (--warp --sweep): an unstyled turning frame's pick follows the moving mask it drew with.
// Four unstyled frames of one revision, each turning a different mask, take every engine copy, so the first frame's
// copy holds another frame's mask: a pick on it answers SA2_PICK_STALE_ENGINE instead of picking through that mask,
// and a pick on the newest frame is not stale.
void product_sweep_picks(const Options& options) {
    using panes_check::pick;
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    auto* ctx = harness.ctx;
    std::vector<float> identity(16, 0.0f);
    for (int i = 0; i < 4; ++i) identity[i * 5] = 1.0f;
    adopt_poses(harness, 1, std::vector<int32_t>(SA2_PIECES, 0), identity, {0});
    const auto set = panes_check::pickable_set(harness, 1, 1);
    PoseFrames frames{harness};
    const Rows Q = double_rotation(0, 3, 0.35, 1, 2, 0.2);
    const float u[4] = {1, 0, 0, 0}, v[4] = {0, 1, 0, 0};
    std::vector<uint64_t> ids;
    for (uint32_t k = 0; k < 4; ++k) {
        std::vector<uint8_t> mask(SA2_PIECES);
        for (uint32_t piece = 0; piece < SA2_PIECES; ++piece) mask[piece] = uint8_t(piece % 4 == k);
        api(ctx, sa2_transform_set(ctx, 1, u, v, 0.9f, mask.data(), 0.0f));
        frames.draw(1, 1, set, Q);
        ids.push_back(frames.frame);
    }
    const uint32_t x = harness.width / 2, y = harness.height / 2;
    const auto first = pick(harness, ids[0], x, y), last = pick(harness, ids[3], x, y);
    std::cout << "transform picks: the first turning frame's pick state " << first.state << " (stale " << first.stale
              << "), the newest's " << last.state << std::endl;
    expect(first.state == SA2_PICK_STALE && first.stale == SA2_PICK_STALE_ENGINE,
           "a pick on an unstyled turning frame whose copy took another moving mask was not stale");
    expect(last.state != SA2_PICK_STALE, "a pick on the newest turning frame was stale");
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    harness.finish(options.debug);
}

void product_panes(const Options& options) {
    using namespace panes_check;
    const Rows Q1 = double_rotation(0, 3, 0.35, 1, 2, 0.2), Q2 = double_rotation(0, 2, 0.9, 1, 3, 0.4);
    // Harness A: the two views as one-view frames at the pane size, their picks and their evaluations.
    std::vector<uint32_t> a1, a2;
    sa2_pick_row a1_pick{}, a2_pick{};
    std::pair<uint32_t, uint32_t> p1, p2;
    uint64_t a1_evals = 0, a2_evals = 0;
    {
        Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
        load(harness, Side, Side);
        const auto set1 = pickable_set(harness, 1, 1), set2 = pickable_set(harness, 2, 1);
        PoseFrames frames{harness};
        a1 = frames.draw(1, 1, set1, Q1);
        const auto f1 = frames.frame;
        a2 = frames.draw(1, 2, set2, Q2);
        const auto f2 = frames.frame;
        p1 = covered_point(a1); p2 = covered_point(a2);
        a1_pick = pick(harness, f1, p1.first, p1.second);
        a2_pick = pick(harness, f2, p2.first, p2.second);
        const auto reported = evals(harness);
        expect(reported.count(f1) && reported.count(f2), "the one-view frames were not reported");
        a1_evals = reported.at(f1); a2_evals = reported.at(f2);
        api(harness.ctx, sa2_drain(harness.ctx, harness.wait));
        api(harness.ctx, sa2_scene_unload(harness.ctx));
        harness.finish(options.debug);
    }
    expect(a1_pick.state == SA2_PICK_READY && a1_pick.instance != 0 && a2_pick.state == SA2_PICK_READY && a2_pick.instance != 0,
        "a one-view pick on a covered pixel found no instance");
    // Harness B: composite frames at 640 x 360.
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    load(harness, 640, 360);
    auto* ctx = harness.ctx;
    const auto set1 = pickable_set(harness, 1, 1), set2 = pickable_set(harness, 2, 1);
    PoseFrames frames{harness};
    // One pane covering the target draws byte-identically to sa2_frame_submit.
    const auto single = frames.draw(1, 1, set1, Q1);
    const auto whole = frames.draw_panes(1, {make(1, set1, 0, 0, 640, 360, Q1)});
    expect(whole == single, "one pane covering the target does not draw byte-identically to sa2_frame_submit");
    // Two panes, each matching its one-view frame; outside them the clear colour.
    const auto two = frames.draw_panes(1, {make(1, set1, 10, 30, Side, Side, Q1), make(2, set2, 330, 30, Side, Side, Q2)});
    const auto composite = frames.frame;
    const auto background = a1[0];
    uint64_t covered = 0, differing = 0, outside = 0;
    const auto compare = [&](const std::vector<uint32_t>& one, uint32_t ox, uint32_t oy) {
        for (uint32_t y = 0; y < Side; ++y)
            for (uint32_t x = 0; x < Side; ++x) {
                const auto a = one[size_t(y) * Side + x], b = two[size_t(y + oy) * 640 + x + ox];
                if (a == background && b == background) continue;
                ++covered;
                for (int shift = 0; shift < 32; shift += 8)
                    if (std::abs(int((a >> shift) & 255) - int((b >> shift) & 255)) > 2) { ++differing; break; }
            }
    };
    compare(a1, 10, 30);
    compare(a2, 330, 30);
    for (uint32_t y = 0; y < 360; ++y)
        for (uint32_t x = 0; x < 640; ++x) {
            const bool in = y >= 30 && y < 30 + Side && ((x >= 10 && x < 10 + Side) || (x >= 330 && x < 330 + Side));
            if (!in && two[size_t(y) * 640 + x] != background) ++outside;
        }
    std::cout << "panes: covered " << covered << ", differing from the one-view frames " << differing << ", outside the panes "
              << outside << std::endl;
    expect(covered >= uint64_t(Side) * Side / 20, "the panes cover less than 5% of their area");
    expect(differing * 100 <= covered, "a pane differs from its one-view frame (over 1% of covered pixels)");
    expect(outside == 0, "a pixel outside every pane is not the clear colour");
    // Refusals: overlap, a fifth pane, a mixed revision.
    const auto refused = [&](const std::vector<sa2_pane>& panes, int32_t status) {
        sa2_frame_input input{};
        input.struct_size = sizeof(input);
        input.target = uint32_t((frames.frame + 1) % SA2_RING_SLOTS);
        input.device_epoch = 1; input.surface = 1; input.surface_generation = 1; input.sequence = frames.sequence + 1;
        input.engine_revision = 1; input.view = panes[0].view; input.view_generation = 1; input.publication_generation = 1;
        input.viewport_width = 640; input.viewport_height = 360; input.resource_set = panes[0].resource_set;
        std::copy(std::begin(panes[0].camera_matrix), std::end(panes[0].camera_matrix), std::begin(input.camera_matrix));
        uint64_t id = 0;
        return sa2_frame_submit_panes(ctx, &input, panes.data(), uint32_t(panes.size()), &id) == status && id == 0;
    };
    expect(refused({make(1, set1, 10, 30, Side, Side, Q1), make(2, set2, 300, 30, Side, Side, Q2)}, SA2_E_INVALID_ARGUMENT),
        "overlapping panes were accepted");
    expect(refused({make(1, 0, 0, 0, 64, 64, Q1), make(2, 0, 64, 0, 64, 64, Q1), make(3, 0, 128, 0, 64, 64, Q1),
                    make(4, 0, 192, 0, 64, 64, Q1), make(5, 0, 256, 0, 64, 64, Q1)}, SA2_E_INVALID_ARGUMENT), "a fifth pane was accepted");
    const auto stale = pickable_set(harness, 3, 2);
    expect(refused({make(1, set1, 10, 30, Side, Side, Q1), make(3, stale, 330, 30, Side, Side, Q2)}, SA2_E_WRONG_STATE),
        "a pane of another engine revision was accepted");
    // A pick in each pane names its view and the one-view pick's instance; a point in no pane is refused.
    const auto left = pick(harness, composite, 10 + p1.first, 30 + p1.second), right = pick(harness, composite, 330 + p2.first, 30 + p2.second);
    expect(left.state == SA2_PICK_READY && left.view == 1 && left.instance == a1_pick.instance, "a pick in the left pane is not the one-view pick");
    expect(right.state == SA2_PICK_READY && right.view == 2 && right.instance == a2_pick.instance, "a pick in the right pane is not the one-view pick");
    uint64_t ticket = 0;
    expect(sa2_pick_request(ctx, composite, 5, 5, &ticket) == SA2_E_INVALID_ARGUMENT, "a pick in no pane was accepted");
    // The vertex evaluations: the frame's sum, and each pane's own.
    const auto reported = evals(harness);
    expect(reported.count(composite) != 0, "the composite frame was not reported");
    sa2_pane_row rows[4]{};
    for (auto& row : rows) row.struct_size = sizeof(row);
    uint32_t count = 0;
    api(ctx, sa2_frame_panes(ctx, composite, rows, 4, &count));
    std::cout << "panes: one-view evaluations " << a1_evals << " + " << a2_evals << ", composite " << reported.at(composite)
              << " (panes " << rows[0].vertex_evals << ", " << rows[1].vertex_evals << ")" << std::endl;
    expect(count == 2 && rows[0].view == 1 && rows[1].view == 2, "the pane rows do not name the two views");
    expect(reported.at(composite) == a1_evals + a2_evals && rows[0].vertex_evals == a1_evals && rows[1].vertex_evals == a2_evals,
        "the composite frame's vertex evaluations are not the sum of the one-view frames'");
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    harness.finish(options.debug);
}

struct ChildResult { DWORD code = 0; std::string output, error; };
std::string read_pipe(HANDLE pipe) {
    std::string text;
    char buffer[1024];
    DWORD count = 0;
    while (ReadFile(pipe, buffer, sizeof(buffer), &count, nullptr) && count) text.append(buffer, count);
    return text;
}
ChildResult child_process(const Options& options, const std::string& child) {
    SECURITY_ATTRIBUTES security{sizeof(security), nullptr, TRUE};
    HANDLE stdout_read = nullptr, stdout_write = nullptr, stderr_read = nullptr, stderr_write = nullptr;
    win_check(CreatePipe(&stdout_read, &stdout_write, &security, 0), "CreatePipe(stdout)");
    Handle out_read(stdout_read), out_write(stdout_write);
    win_check(CreatePipe(&stderr_read, &stderr_write, &security, 0), "CreatePipe(stderr)");
    Handle err_read(stderr_read), err_write(stderr_write);
    win_check(SetHandleInformation(out_read.get(), HANDLE_FLAG_INHERIT, 0), "SetHandleInformation(stdout)");
    win_check(SetHandleInformation(err_read.get(), HANDLE_FLAG_INHERIT, 0), "SetHandleInformation(stderr)");
    Handle input(CreateFileW(L"NUL", GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, &security, OPEN_EXISTING, 0, nullptr));
    win_check(input.get() != INVALID_HANDLE_VALUE, "CreateFileW(NUL)");
    SIZE_T bytes = 0;
    InitializeProcThreadAttributeList(nullptr, 1, 0, &bytes);
    std::vector<uint8_t> storage(bytes);
    auto* attributes = reinterpret_cast<LPPROC_THREAD_ATTRIBUTE_LIST>(storage.data());
    win_check(InitializeProcThreadAttributeList(attributes, 1, 0, &bytes), "InitializeProcThreadAttributeList");
    struct AttributeGuard {
        LPPROC_THREAD_ATTRIBUTE_LIST attributes;
        ~AttributeGuard() { DeleteProcThreadAttributeList(attributes); }
    } attribute_guard{attributes};
    HANDLE inherited[] = {out_write.get(), err_write.get(), input.get()};
    win_check(UpdateProcThreadAttribute(attributes, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST, inherited, sizeof(inherited), nullptr, nullptr), "UpdateProcThreadAttribute");
    STARTUPINFOEXW startup{};
    startup.StartupInfo.cb = sizeof(startup);
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdOutput = out_write.get();
    startup.StartupInfo.hStdError = err_write.get();
    startup.StartupInfo.hStdInput = input.get();
    startup.lpAttributeList = attributes;
    const auto exe = executable_path();
    const auto directory = exe.parent_path().wstring();
    const std::wstring child_name(child.begin(), child.end());
    std::wstring command = L"\"" + exe.wstring() + L"\" --child " + child_name
        + (options.mode == "warp" ? L" --warp" : L" --hardware") + (options.debug ? L" --debug" : L"");
    PROCESS_INFORMATION process{};
    win_check(CreateProcessW(exe.c_str(), command.data(), nullptr, nullptr, TRUE,
        EXTENDED_STARTUPINFO_PRESENT | CREATE_NO_WINDOW, nullptr, directory.c_str(), &startup.StartupInfo, &process), "CreateProcessW(selftest child)");
    Handle process_handle(process.hProcess), thread_handle(process.hThread);
    out_write = Handle(); err_write = Handle();
    const auto waited = WaitForSingleObject(process_handle.get(), 30000);
    if (waited != WAIT_OBJECT_0) {
        TerminateProcess(process_handle.get(), 124);
        WaitForSingleObject(process_handle.get(), timeout_ms);
        throw std::runtime_error("selftest child did not finish within 30 seconds");
    }
    ChildResult result;
    win_check(GetExitCodeProcess(process_handle.get(), &result.code), "GetExitCodeProcess");
    result.output = read_pipe(out_read.get()); result.error = read_pipe(err_read.get());
    return result;
}
}

void run_gpu(Suite& suite, const Options& options) {
    for (int32_t queue : {SA2_QUEUE_SAME, SA2_QUEUE_OWN}) for (int32_t barriers : {SA2_BARRIERS_LEGACY, SA2_BARRIERS_ENHANCED})
        for (bool external : {false, true}) {
            const auto label = std::string(queue == SA2_QUEUE_SAME ? "same" : "own") + '/'
                + (barriers == SA2_BARRIERS_LEGACY ? "legacy" : "enhanced") + '/' + (external ? "external" : "created")
                + " (300 frames, resize, negatives, references, debug)";
            suite.check(label, [&] { configuration(options, queue, barriers, external); });
        }
    // Reuse one stand-in device across the complete ABI 2 matrix; geometry runs
    // exactly once on that device, while each scene has its own drained context.
    try {
        Device scene_device(options);
        suite.check("scene geometry", [&] { scene_geometry(options, scene_device); });
        for (uint32_t scene = 1; scene <= 4; ++scene)
            for (int32_t queue : {SA2_QUEUE_SAME, SA2_QUEUE_OWN})
                for (int32_t barriers : {SA2_BARRIERS_LEGACY, SA2_BARRIERS_ENHANCED}) {
                    const auto label = "scene w" + std::to_string(scene) + '/' + (queue == SA2_QUEUE_SAME ? "same" : "own")
                        + '/' + (barriers == SA2_BARRIERS_LEGACY ? "legacy" : "enhanced");
                    suite.check(label, [&] { scene_configuration(options, scene_device, scene, queue, barriers); });
                }
        suite.check("scene injected W3", [&] { scene_configuration(options, scene_device, 3, SA2_QUEUE_OWN, SA2_BARRIERS_LEGACY, true); });
        suite.check("scene NO_VRAM", [&] { scene_configuration(options, scene_device, 1, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false, true); });
    } catch (const Unsupported& error) { suite.check("scene device", [&] { throw Unsupported(error.what()); }); }
      catch (const std::exception& error) { suite.check("scene device", [&] { throw std::runtime_error(error.what()); }); }
    suite.check("child inject-drain", [&] {
        Device available(options);
        const auto result = child_process(options, "inject-drain");
        expect(result.code == 3, "unconfirmed drain child did not exit 3");
        expect(result.error == "sa2: drain not confirmed; ending the process with exit code 3\n", "unconfirmed drain child stderr did not match the exact line");
        expect(result.output.empty(), "unconfirmed drain child ran code after the drain");
    });
    suite.check("child device-loss", [&] {
        Device available(options);
        const auto result = child_process(options, "device-loss");
        expect(result.code == 0 && result.output == "child: device-loss ok\r\n" && result.error.empty(), "device-loss child did not confirm removal and cleanup");
    });
}

// C9 U5: --warp --stages. Only WARP; parse refuses any other mode.
void run_stages(Suite& suite, const Options& options) {
    expect(options.mode == "warp", "the stage check runs on WARP only");
    suite.check("stages w3 warp", [&] { scene_stages(options); });
}

// C9 jumble-on: --warp --poses. Only WARP; parse refuses any other mode.
void run_poses(Suite& suite, const Options& options) {
    expect(options.mode == "warp", "the pose check runs on WARP only");
    suite.check("product poses warp", [&] { product_poses(options); });
}

// ABI 6: --warp --shape. Only WARP; parse refuses any other mode.
void run_shape(Suite& suite, const Options& options) {
    expect(options.mode == "warp", "the shape check runs on WARP only");
    suite.check("view shape warp", [&] { product_shape(options); });
}

// ABI 10's WARP check (--warp --hopf FILE): a view's layout, the Hopf-ring opening (ADR-D02 B23, bare). FILE is Look
// Lab's tools/looklab/fixtures/hopf-rings.json. The chain layout and NULL draw byte-identically to no call; malformed
// rings and rings that do not fit the core's cells are refused and change nothing; the Hopf rings change the picture
// and cover pixels around its centre; another view keeps the chain; the same rings on a third view draw the same
// picture; NULL restores the chain. On the CPU, the table built from the rings' own centres puts the 20 rings' centres
// at a dodecahedron's vertices (Look Lab's base angle multiset). The picture goes beside the report.
sa2_view_layout hopf_layout(const std::filesystem::path& file) {
    std::ifstream in(file, std::ios::binary);
    expect(bool(in), "the Hopf ring file cannot be read");
    std::stringstream text; text << in.rdbuf();
    const auto json = Json::parse(text.str());
    sa2_view_layout layout{};
    layout.struct_size = sizeof(layout);
    layout.kind = SA2_LAYOUT_HOPF_RINGS;
    for (size_t i = 0; i < 4; ++i) {
        layout.j[i] = float(json.at("J").array()[i].number());
        layout.k[i] = float(json.at("K").array()[i].number());
    }
    const auto& rings = json.at("rings").array();
    expect(rings.size() == SA2_HOPF_RINGS, "the Hopf ring file does not hold 20 rings");
    for (size_t r = 0; r < rings.size(); ++r)
        for (size_t i = 0; i < 4; ++i) layout.origins[4 * r + i] = float(rings[r].at("origin").array()[i].number());
    const auto& ring_of = json.at("ringOf").array();
    const auto& half_step = json.at("halfStep").array();
    expect(ring_of.size() == 600 && half_step.size() == 600, "the Hopf ring file does not cover 600 cells");
    for (size_t c = 0; c < 600; ++c) {
        layout.ring_of[c] = uint8_t(ring_of[c].integer());
        layout.half_step[c] = uint8_t(half_step[c].integer());
    }
    return layout;
}

void hopf_dodecahedron(const sa2_view_layout& layout) {
    // The rings' own centres: cos(6h) O + sin(6h) O J, so the table is Look Lab's for the fixture's cells.
    const auto centre = [&](uint32_t cell) {
        const float* o = layout.origins + 4 * layout.ring_of[cell];
        const double t = layout.half_step[cell] * 3.14159265358979323846 / 30, c = std::cos(t), s = std::sin(t);
        const double oj[4] = {o[0] * 0 - o[1] * layout.j[1] - o[2] * layout.j[2] - o[3] * layout.j[3],
                              o[0] * layout.j[1] + o[2] * layout.j[3] - o[3] * layout.j[2],
                              o[0] * layout.j[2] - o[1] * layout.j[3] + o[3] * layout.j[1],
                              o[0] * layout.j[3] + o[1] * layout.j[2] - o[2] * layout.j[1]};
        return std::array<float, 4>{float(c * o[0] + s * oj[0]), float(c * o[1] + s * oj[1]), float(c * o[2] + s * oj[2]), float(c * o[3] + s * oj[3])};
    };
    const auto table = sa2::hopf_table(layout, centre);
    std::array<std::array<double, 3>, SA2_HOPF_RINGS> mean{};
    for (uint32_t cell = 0; cell < 600; ++cell)
        for (int i = 0; i < 3; ++i) mean[layout.ring_of[cell]][i] += table[cell * 4 + 3][i] / 30.0;
    const auto length = [](const std::array<double, 3>& a) { return std::sqrt(a[0] * a[0] + a[1] * a[1] + a[2] * a[2]); };
    const std::multiset<double> expected = {0, 41.81, 41.81, 41.81, 70.53, 70.53, 70.53, 70.53, 70.53, 70.53, 109.47, 109.47, 109.47,
                                            109.47, 109.47, 109.47, 138.19, 138.19, 138.19, 180};
    for (size_t a = 0; a < SA2_HOPF_RINGS; ++a) {
        expect(std::abs(length(mean[a]) - 0.28 * 8.063) < 1e-3, "a ring's centre is not at the base radius");
        std::multiset<double> angles;
        for (size_t b = 0; b < SA2_HOPF_RINGS; ++b) {
            const double cosine = (mean[a][0] * mean[b][0] + mean[a][1] * mean[b][1] + mean[a][2] * mean[b][2]) / (length(mean[a]) * length(mean[b]));
            angles.insert(std::round(std::acos(std::clamp(cosine, -1.0, 1.0)) * 180 / 3.14159265358979323846 * 100) / 100);
        }
        expect(angles == expected, "the ring centres are not a dodecahedron's vertices");
    }
    // B22: the ring sheet puts each cell's own centre at its row and column, 20 rows of 30 distinct places.
    std::set<std::pair<long, long>> places;
    for (uint32_t cell = 0; cell < 600; ++cell) {
        const auto c = centre(cell);
        const size_t at = sa2::HopfSheetRow + cell * 4;
        double placed[3];
        for (int row = 0; row < 3; ++row)
            placed[row] = table[at + row][0] * c[0] + table[at + row][1] * c[1] + table[at + row][2] * c[2] + table[at + row][3] * c[3] + table[at + 3][row];
        const double column = layout.half_step[cell] / 2.0 - 14.75, row = double(layout.ring_of[cell]) - 9.5;
        expect(std::abs(placed[0] - 0.6 * 0.68 * column) < 1e-4 && std::abs(placed[1] + 0.6 * 0.57 * row) < 1e-4 && std::abs(placed[2]) < 1e-4,
               "a cell's centre is not at its place on the ring sheet");
        places.insert({std::lround(placed[0] * 1000), std::lround(placed[1] * 1000)});
    }
    expect(places.size() == 600, "two cells share a place on the ring sheet");
}

// B22: the ring sheet named before any rings, on a fresh context: the core builds the Hopf table for the sheet too.
// It once did not, and the shaders read t18 past another buffer (DXGI_ERROR_DEVICE_HUNG on the owner's GPU).
std::vector<uint32_t> product_sheet_first(const Options& options, const sa2_view_layout& layout) {
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    auto* ctx = harness.ctx;
    std::vector<uint32_t> labels(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) labels[slot] = slot;
    sa2_engine_image image{};
    image.struct_size = sizeof(image);
    image.revision = 1;
    image.labels = labels.data();
    uint64_t adopted = 0;
    api(ctx, sa2_engine_adopt(ctx, &image, &adopted));
    auto sheet = layout;
    sheet.kind = SA2_LAYOUT_RING_SHEET;
    api(ctx, sa2_view_layout_set(ctx, 1, &sheet));
    PoseFrames frames{harness};
    const auto flat = frames.draw(1, 1, 0, double_rotation(0, 3, 0, 1, 2, 0));
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    harness.finish(options.debug);
    return flat;
}

void product_hopf(const Options& options) {
    const auto layout = hopf_layout(options.hopf);
    const auto sheet_first = product_sheet_first(options, layout);
    hopf_dodecahedron(layout);
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    auto* ctx = harness.ctx;
    expect(sa2_view_layout_set(nullptr, 1, nullptr) == SA2_E_INVALID_ARGUMENT, "a layout was set without a context");
    std::vector<uint32_t> labels(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) labels[slot] = slot;
    sa2_engine_image image{};
    image.struct_size = sizeof(image);
    image.revision = 1;
    image.labels = labels.data();
    uint64_t adopted = 0;
    api(ctx, sa2_engine_adopt(ctx, &image, &adopted));
    PoseFrames frames{harness};
    const Rows I = double_rotation(0, 3, 0, 1, 2, 0), turned = double_rotation(0, 2, 0.6, 1, 3, 0);
    const auto unset = frames.draw(1, 1, 0, I);
    sa2_view_layout chain{};
    chain.struct_size = sizeof(chain);
    chain.kind = SA2_LAYOUT_CHAIN;
    api(ctx, sa2_view_layout_set(ctx, 1, &chain));
    expect(frames.draw(1, 1, 0, I) == unset, "the chain layout does not draw byte-identically to no call");
    for (int field = 0; field < 7; ++field) {
        auto bad = layout;
        switch (field) {
        case 0: bad.struct_size = 1500; break;
        case 1: bad.kind = 3; break;
        case 2: bad.ring_of[0] = 20; break;
        case 3: bad.half_step[0] = uint8_t((bad.half_step[0] + 1) % 60); break;
        case 4: for (int i = 0; i < 4; ++i) std::swap(bad.origins[i], bad.origins[4 + i]); break;
        case 5: bad.j[1] += 0.5f; break;
        default: bad.k[0] = std::numeric_limits<float>::quiet_NaN(); break;
        }
        expect(sa2_view_layout_set(ctx, 1, &bad) == SA2_E_INVALID_ARGUMENT, "malformed or unfitting Hopf rings were accepted");
    }
    expect(frames.draw(1, 1, 0, I) == unset, "a refused layout changed the frames");
    api(ctx, sa2_view_layout_set(ctx, 1, &layout));
    const auto rings = frames.draw(1, 1, 0, I);
    const auto background = unset[0];
    uint64_t covered = 0;
    int left = int(harness.width), right = -1, top = int(harness.height), bottom = -1;
    for (uint32_t y = 0; y < harness.height; ++y)
        for (uint32_t x = 0; x < harness.width; ++x)
            if (rings[size_t(y) * harness.width + x] != background) {
                ++covered;
                left = std::min(left, int(x)); right = std::max(right, int(x)); top = std::min(top, int(y)); bottom = std::max(bottom, int(y));
            }
    std::cout << "hopf rings: " << uint64_t(harness.width) * harness.height << " pixels; covered " << covered << "; x " << left << ".."
              << right << ", y " << top << ".." << bottom << std::endl;
    expect(rings != unset && covered > 0, "the Hopf rings changed nothing");
    expect(left < int(harness.width) / 2 && right > int(harness.width) / 2 && top < int(harness.height) / 2 && bottom > int(harness.height) / 2,
           "the Hopf rings are not around the picture's centre");
    expect(frames.draw(1, 1, 0, turned) != rings, "the camera did not turn the Hopf rings");
    expect(frames.draw(1, 2, 0, I) == unset, "another view drew with view 1's layout");
    api(ctx, sa2_view_layout_set(ctx, 3, &layout));
    expect(frames.draw(1, 3, 0, I) == rings, "the same rings on another view drew another picture");
    // B22: the ring sheet draws another picture, around the centre, which the camera does not turn.
    auto sheet = layout;
    sheet.kind = SA2_LAYOUT_RING_SHEET;
    api(ctx, sa2_view_layout_set(ctx, 1, &sheet));
    const auto flat = frames.draw(1, 1, 0, I);
    uint64_t sheet_covered = 0;
    int sheet_left = int(harness.width), sheet_right = -1, sheet_top = int(harness.height), sheet_bottom = -1;
    for (uint32_t y = 0; y < harness.height; ++y)
        for (uint32_t x = 0; x < harness.width; ++x)
            if (flat[size_t(y) * harness.width + x] != background) {
                ++sheet_covered;
                sheet_left = std::min(sheet_left, int(x)); sheet_right = std::max(sheet_right, int(x));
                sheet_top = std::min(sheet_top, int(y)); sheet_bottom = std::max(sheet_bottom, int(y));
            }
    std::cout << "ring sheet: covered " << sheet_covered << "; x " << sheet_left << ".." << sheet_right << ", y " << sheet_top << ".."
              << sheet_bottom << std::endl;
    expect(flat != rings && flat != unset && sheet_covered > 0, "the ring sheet drew the rings or nothing");
    expect(sheet_left < int(harness.width) / 2 && sheet_right > int(harness.width) / 2 && sheet_top < int(harness.height) / 2
               && sheet_bottom > int(harness.height) / 2, "the ring sheet is not around the picture's centre");
    expect(frames.draw(1, 1, 0, turned) == flat, "the camera turned the ring sheet");
    expect(frames.draw(1, 3, 0, I) == rings, "the ring sheet on view 1 changed view 3's rings");
    expect(flat == sheet_first, "the ring sheet named before any rings drew another picture");
    // B22 rest (ADR-D02.10): under a chart Global draws no sweep. A transform at phase 0 turns the chain's moving
    // pieces (view 2) but leaves the sheet (view 1) and the rings (view 3) as they were.
    {
        const float u[4] = {1, 0, 0, 0}, v[4] = {0, 1, 0, 0};
        std::vector<uint8_t> mask(SA2_PIECES);
        for (uint32_t piece = 0; piece < SA2_PIECES; ++piece) mask[piece] = uint8_t(piece % 2 == 0);
        api(ctx, sa2_transform_set(ctx, 1, u, v, 0.9f, mask.data(), 0.0f));
        expect(frames.draw(1, 1, 0, I) == flat, "the ring sheet drew a sweep");
        expect(frames.draw(1, 3, 0, I) == rings, "the Hopf rings drew a sweep");
        expect(frames.draw(1, 2, 0, I) != unset, "the chain did not draw the sweep");
        api(ctx, sa2_transform_set(ctx, 1, u, v, 0.9f, nullptr, 0.0f));
        expect(frames.draw(1, 2, 0, I) == unset, "a cleared transform still turned the chain");
        std::cout << "ring sheet and rings: no sweep under a transform; the chain turned" << std::endl;
    }
    if (!options.out.empty()) {
        std::ofstream file(options.out.parent_path() / "warp-sheet.ppm", std::ios::binary);
        file << "P6\n" << harness.width << " " << harness.height << "\n255\n";
        for (uint32_t pixel : flat) {
            const char rgb[3] = {char(pixel & 0xff), char((pixel >> 8) & 0xff), char((pixel >> 16) & 0xff)};
            file.write(rgb, 3);
        }
    }
    api(ctx, sa2_view_layout_set(ctx, 1, nullptr));
    expect(frames.draw(1, 1, 0, I) == unset, "a NULL layout did not restore the chain");
    if (!options.out.empty()) {
        std::ofstream file(options.out.parent_path() / "warp-hopf.ppm", std::ios::binary);
        file << "P6\n" << harness.width << " " << harness.height << "\n255\n";
        for (uint32_t pixel : rings) {
            const char rgb[3] = {char(pixel & 0xff), char((pixel >> 8) & 0xff), char((pixel >> 16) & 0xff)};
            file.write(rgb, 3);
        }
    }
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    expect(sa2_view_layout_set(ctx, 1, &layout) == SA2_E_WRONG_STATE, "a layout was set without a loaded scene");
    harness.finish(options.debug);
}

// ADR-D02 B20's WARP check (--warp --slice): the cut slice. The even pieces are the inside layer. With shrink 1 and the
// whole shell, the outside copy draws what a set of the odd pieces draws (at most 1% of the covered pixels differ by
// more than 2), the inside copy covers only part of what a set of the even pieces covers and nothing else, and the
// two copies together cover exactly the pixels the unsplit slice covers. A smaller shrink and a narrower shell cover
// less. A ghost slot claims the transform: in a composite frame its pane turns the inside pieces and the other pane
// draws exactly as without a transform.
void product_slice(const Options& options) {
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    auto* ctx = harness.ctx;
    std::vector<uint32_t> labels(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) labels[slot] = slot;
    sa2_engine_image image{};
    image.struct_size = sizeof(image);
    image.revision = 1;
    image.labels = labels.data();
    uint64_t adopted = 0;
    api(ctx, sa2_engine_adopt(ctx, &image, &adopted));
    const auto pieces = sa2::scene_slot_pieces();
    std::vector<uint8_t> all(SA2_SLOTS, 1), even(SA2_SLOTS, 0), odd(SA2_SLOTS, 0), inside(SA2_PIECES, 0);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) { even[slot] = uint8_t(pieces[slot] % 2 == 0); odd[slot] = uint8_t(pieces[slot] % 2 == 1); }
    for (uint32_t piece = 0; piece < SA2_PIECES; piece += 2) inside[piece] = 1;
    const auto set_all = pose_set(harness, 1, all), set_even = pose_set(harness, 2, even), set_odd = pose_set(harness, 3, odd);
    PoseFrames frames{harness};
    const Rows Q = double_rotation(0, 3, 0.35, 1, 2, 0.2);
    const auto plain = frames.draw(1, 1, set_all, Q);
    const auto only_odd = frames.draw(1, 3, set_odd, Q), only_even = frames.draw(1, 2, set_even, Q);
    const auto background = plain[0];
    const auto covered = [&](const std::vector<uint32_t>& image) {
        return uint64_t(std::count_if(image.begin(), image.end(), [&](uint32_t pixel) { return pixel != background; }));
    };
    const auto differing = [](const std::vector<uint32_t>& x, const std::vector<uint32_t>& y) {
        uint64_t count = 0;
        for (size_t i = 0; i < x.size(); ++i)
            for (int shift = 0; shift < 32; shift += 8)
                if (std::abs(int((x[i] >> shift) & 0xff) - int((y[i] >> shift) & 0xff)) > 2) { ++count; break; }
        return count;
    };
    api(ctx, sa2_style_set(ctx, 1, inside.data(), SA2_STYLE_CHANNEL_SLICE));
    expect(frames.draw(1, 1, set_all, Q) == plain, "a slice channel without a slice slot changed the frame");
    sa2_view_style style{};
    style.struct_size = sizeof(style);
    const float identity[9] = {1, 0, 0, 0, 1, 0, 0, 0, 1};
    std::copy(std::begin(identity), std::end(identity), std::begin(style.matrix));
    style.slice_mode = SA2_SLICE_BOTH; style.slice_shrink = 1.0f; style.shell_min = 0.0f; style.shell_max = 3.14159265f;
    const auto drawn = [&](uint32_t mode, float shrink, float shell_max, uint64_t view, uint64_t set) {
        auto s = style; s.slice_mode = mode; s.slice_shrink = shrink; s.shell_max = shell_max;
        api(ctx, sa2_view_style_set(ctx, 1, &s));
        return frames.draw(1, view, set, Q);
    };
    const auto both = drawn(SA2_SLICE_BOTH, 1.0f, 3.14159265f, 1, set_all);
    const auto outside = drawn(SA2_SLICE_OUTSIDE, 1.0f, 3.14159265f, 1, set_all);
    const auto inner = drawn(SA2_SLICE_INSIDE, 1.0f, 3.14159265f, 1, set_all);
    const auto shrunk = drawn(SA2_SLICE_BOTH, 0.6f, 3.14159265f, 1, set_all);
    uint64_t outside_band = 0, union_miss = 0, union_extra = 0;
    for (size_t i = 0; i < plain.size(); ++i) {
        const bool b = both[i] != background, o = outside[i] != background, n = inner[i] != background;
        outside_band += n && only_even[i] == background;
        union_miss += b && !o && !n;
        union_extra += !b && (o || n);
    }
    const auto c_both = covered(both), c_inner = covered(inner), c_even = covered(only_even), c_shrunk = covered(shrunk);
    const auto apart = differing(outside, only_odd);
    std::cout << "slice: covered " << c_both << " (inside outlines " << c_inner << " of " << c_even << ", shrink 0.6 " << c_shrunk
              << "); outside vs the odd set differ at " << apart << "; union misses " << union_miss << ", extra " << union_extra << '\n' << std::flush;
    expect(c_both >= uint64_t(harness.width) * harness.height / 20, "a slice-check frame covers less than 5% of the target");
    expect(apart * 100 <= covered(only_odd), "the outside layer is not drawn filled as the odd set (over 1% differ)");
    expect(outside_band == 0, "the inside layer covered a pixel the even set does not");
    // Small stickers keep most of their pixels in the band; what shows through the holes differs from the filled set.
    const auto holes = differing(inner, only_even);
    std::cout << "slice: the inside layer differs from the filled even set at " << holes << '\n' << std::flush;
    expect(c_inner * 10 >= c_even && holes * 10 >= c_even, "the inside layer is not drawn as outlines only (under 10% show through)");
    expect(union_miss == 0 && union_extra == 0, "the split copies together do not cover what the unsplit slice covers");
    expect(c_shrunk < c_both, "a smaller inside shrink did not cover less");
    // The radius shell about a focus pole (cell 150): a narrower shell draws fewer pixels, never none.
    sa2_view_focus focus{sizeof(sa2_view_focus), SA2_PROJECTION_ORTHOGRAPHIC, 150, -1.0f, 1.0f, 0};
    api(ctx, sa2_view_focus_set(ctx, 1, &focus));
    const auto wide = covered(drawn(SA2_SLICE_BOTH, 1.0f, 3.14159265f, 1, set_all));
    const auto narrow = covered(drawn(SA2_SLICE_BOTH, 1.0f, 0.6f, 1, set_all));
    std::cout << "slice shell: whole " << wide << ", within 0.6 rad " << narrow << '\n' << std::flush;
    expect(narrow > 0 && narrow < wide, "a narrower radius shell did not draw less");
    api(ctx, sa2_view_focus_set(ctx, 1, nullptr));
    // The hardware timeout's check (26 November 2026): the product's Local on view 2, local-view.json's shape at Local's
    // zoom for this target (ShellLocal.Zoom: 4.507 at 16:9) with the orthographic focus on the default cap 0 (cos
    // 121/125, spread 1.7), drawn by the cap test and by the slice's default (0..0.6 rad) and whole shells, then the
    // whole shell in perspective at the ABI's nearest eye (d4 1.02). Each frame's clipped triangles and pixel-shader
    // invocations (the newest harvested frame's pipeline statistics) are its raster work.
    {
        using Newest = void (*)(uint64_t*);
        const auto newest = reinterpret_cast<Newest>(reinterpret_cast<void*>(
            GetProcAddress(GetModuleHandleW(L"sa2_interop.dll"), "sa2_test_frame_statistics")));
        expect(newest != nullptr, "sa2_interop.dll has no sa2_test_frame_statistics");
        const auto reports = [&] {
            sa2_frame_report rows[16]{};
            for (auto& row : rows) row.struct_size = sizeof(row);
            uint32_t count = 0;
            api(ctx, sa2_frame_reports(ctx, rows, 16, &count));
        };
        const auto measure = [&](const char* what, const sa2_view_style* slot1) {
            api(ctx, sa2_view_style_set(ctx, 1, slot1));
            reports();
            frames.draw(1, 2, 0, Q); // no resource set: every slot drawn
            reports();
            uint64_t s[4]{}; // frame, vertex evaluations, clipped triangles, pixel-shader invocations
            newest(s);
            expect(s[0] == frames.frame, "the newest harvested frame is not the measured one");
            std::cout << "slice raster: " << what << ": " << s[2] << " triangles, " << s[3] << " pixel invocations ("
                      << double(s[3]) / (double(harness.width) * harness.height) << " targets)\n" << std::flush;
            return std::pair<uint64_t, uint64_t>{s[2], s[3]};
        };
        const sa2_view_shape local{sizeof(sa2_view_shape), 0.7f, 0.6f, 2.0f, 4.507f, 0};
        api(ctx, sa2_view_shape_set(ctx, 2, &local));
        const sa2_view_focus cap0{sizeof(sa2_view_focus), SA2_PROJECTION_ORTHOGRAPHIC, 0, 121.0f / 125.0f, 1.7f, 0};
        api(ctx, sa2_view_focus_set(ctx, 2, &cap0));
        auto slice = style; slice.slice_shrink = 0.6f; slice.shell_max = 0.6f;
        auto whole = slice; whole.shell_max = 3.14159265f;
        const auto by_cap = measure("Local, cap test", nullptr);
        const auto by_shell = measure("Local, slice 0..0.6 rad", &slice);
        const auto by_whole = measure("Local, slice whole shell", &whole);
        const sa2_view_shape near_eye{sizeof(sa2_view_shape), 0.7f, 0.6f, 1.02f, 4.507f, 0};
        api(ctx, sa2_view_shape_set(ctx, 2, &near_eye));
        auto perspective = cap0; perspective.projection = SA2_PROJECTION_PERSPECTIVE;
        api(ctx, sa2_view_focus_set(ctx, 2, &perspective));
        const auto by_near = measure("Local perspective d4 1.02, slice whole shell", &whole);
        api(ctx, sa2_view_focus_set(ctx, 2, nullptr));
        api(ctx, sa2_view_shape_set(ctx, 2, nullptr));
        api(ctx, sa2_view_style_set(ctx, 1, nullptr));
        // Measured on WARP on 26 November 2026 at 640 x 360: the cap test 105,216 triangles and 0.87 targets of pixel
        // work, the default shell 321,547 and 2.65, the whole shell 4,575,204 and 37.1, the near eye 3,590,931 and 46.3.
        // A slice's raster work is bounded as the whole puzzle's is (the 2-second limit lies orders of magnitude above).
        const uint64_t target = uint64_t(harness.width) * harness.height;
        expect(by_cap.first < by_shell.first && by_shell.first < by_whole.first, "a wider shell did not draw more triangles");
        expect(by_shell.second <= 8 * target, "the default shell's pixel work exceeds 8 targets");
        expect(by_whole.second <= 64 * target && by_near.second <= 64 * target, "a whole shell's pixel work exceeds 64 targets");
        expect(by_whole.first <= 5000000 && by_near.first <= 5000000, "a whole shell drew more than 5,000,000 triangles");
    }
    // A ghost slot claims the transform: pane 1 (slot 1, no slice) stays unturned; pane 2 (slot 2, a ghost) turns.
    sa2_view_style none{};
    none.struct_size = sizeof(none);
    std::copy(std::begin(identity), std::end(identity), std::begin(none.matrix));
    api(ctx, sa2_view_style_set(ctx, 1, &none));
    auto ghost = style; ghost.ghost[0] = 0.9f; ghost.ghost[1] = 0.9f; ghost.ghost[2] = 1.0f; ghost.ghost[3] = 0.5f;
    api(ctx, sa2_view_style_set(ctx, 2, &ghost));
    const auto paned = [&] {
        auto left = panes_check::make(1, set_all, 0, 0, 320, 360, Q), right = panes_check::make(1, set_all, 320, 0, 320, 360, Q);
        left.style = 1; right.style = 2;
        return frames.draw_panes(1, {left, right});
    };
    const auto still = paned();
    const float u[4] = {1, 0, 0, 0}, v[4] = {0, 1, 0, 0};
    api(ctx, sa2_transform_set(ctx, 1, u, v, 0.9f, inside.data(), 0.0f));
    const auto turned = paned();
    api(ctx, sa2_transform_set(ctx, 1, u, v, 0.9f, nullptr, 1.0f));
    uint64_t left_moved = 0, right_moved = 0, right_cover = 0;
    for (uint32_t y = 0; y < 360; ++y)
        for (uint32_t x = 0; x < 640; ++x) {
            const size_t i = size_t(y) * harness.width + x;
            (x < 320 ? left_moved : right_moved) += turned[i] != still[i];
            right_cover += x >= 320 && still[i] != background;
        }
    std::cout << "slice ghost: the plain pane changed at " << left_moved << ", the ghost pane at " << right_moved << " of " << right_cover << '\n' << std::flush;
    expect(left_moved == 0, "a pane without the ghost turned the moving pieces");
    expect(right_moved * 20 >= right_cover, "the ghost pane did not turn its inside pieces (under 5% of its covered pixels)");
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    harness.finish(options.debug);
}

// ADR-D01.49/.52: the colour and background exports, with the product fixture on WARP.
void product_colours(const Options& options) {
    using namespace panes_check;
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    load(harness, Side, Side);
    auto* ctx = harness.ctx;
    const auto set = pickable_set(harness, 1, 1);
    const Rows q = double_rotation(0, 3, 0.35, 1, 2, 0.2);
    PoseFrames frames{harness};
    const auto draw = [&] { return frames.draw(1, 1, set, q); };

    constexpr uint32_t default_background = 0xff292521, green_background = 0xff00ff00;
    const auto original = draw();
    expect(original[0] == default_background, "the no-call clear colour is not today's RGBA8 value");
    const auto [x, y] = covered_point(original);
    const auto ids = [&] {
        const auto sticker = pick(harness, frames.frame, x, y), background = pick(harness, frames.frame, 0, 0);
        expect(sticker.state == SA2_PICK_READY && background.state == SA2_PICK_READY, "a colour-check pick was not ready");
        return std::array{sticker.instance, background.instance};
    };
    const auto original_ids = ids();
    expect(original_ids[0] > 0 && original_ids[0] <= SA2_SLOTS && original_ids[1] == 0, "the fixture needs a sticker and background pick");
    const uint32_t home = (original_ids[0] - 1) / sb::Stickers;

    const std::vector<uint8_t> fill(SA2_PIECES, 1);
    sa2_view_style tint{};
    tint.struct_size = sizeof(tint);
    tint.tint_mode = SA2_STYLE_TINT_FILL;
    tint.matrix[0] = tint.matrix[4] = tint.matrix[8] = 1.0f;
    tint.palette[6] = 1.0f; // index 1 is blue at half strength
    tint.palette[7] = 0.5f;
    api(ctx, sa2_style_set(ctx, 1, fill.data(), SA2_STYLE_CHANNEL_FILL));
    api(ctx, sa2_view_style_set(ctx, 1, &tint));
    const auto original_styled = draw();
    expect(original_styled != original && ids() == original_ids, "the tint fixture did not draw or changed the IDs");
    api(ctx, sa2_view_style_set(ctx, 1, nullptr));

    api(ctx, sa2_sticker_colours_set(ctx, nullptr, 0));
    expect(draw() == original, "a NULL table changed the default frame");
    api(ctx, sa2_scene_look_set(ctx, nullptr));
    expect(draw() == original, "a NULL scene look changed the default frame");
    sa2_scene_look look{sizeof(sa2_scene_look), 0, {0.13f, 0.145f, 0.16f, 1.0f}, 0.2f, 0.4f, 0.6f, 0.8f};
    api(ctx, sa2_scene_look_set(ctx, &look));
    expect(draw() == original, "today's background with carried values changed the default frame");

    // Isolate the chosen home class in the left pane and put every other class in the right pane. Every changed
    // pixel then has an independent class/coverage check, including when the two sets would occlude each other.
    std::vector<uint8_t> chosen(SA2_SLOTS), rest(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) {
        chosen[slot] = uint8_t(slot / sb::Stickers == home);
        rest[slot] = uint8_t(!chosen[slot]);
    }
    const auto one = pose_set(harness, 2, chosen), other = pose_set(harness, 3, rest);
    const std::vector<sa2_pane> panes{
        make(2, one, 0, 0, Side / 2, Side, q), make(3, other, Side / 2, 0, Side / 2, Side, q),
    };
    std::array<float, SA2_STICKER_CLASSES * 3> table{};
    api(ctx, sa2_sticker_colours_set(ctx, table.data(), SA2_STICKER_CLASSES));
    const auto pane_base = frames.draw_panes(1, panes);
    table[3 * home] = 1.0f;
    api(ctx, sa2_sticker_colours_set(ctx, table.data(), SA2_STICKER_CLASSES));
    const auto pane_colours = frames.draw_panes(1, panes);
    size_t recoloured = 0, other_class = 0;
    for (size_t i = 0; i < pane_base.size(); ++i) {
        const bool left = i % Side < Side / 2;
        const bool covered = pane_base[i] != default_background;
        const bool changed = pane_base[i] != pane_colours[i];
        expect(changed == (left && covered), "the palette changed a pixel outside its class or missed one inside it");
        if (changed) expect((pane_colours[i] & 0xffffff) == (pane_colours[i] & 0xff) && (pane_colours[i] & 0xff) > 0,
                           "a recoloured sticker did not draw red");
        recoloured += changed;
        other_class += covered && !left;
    }
    expect(recoloured > 0 && other_class > 0, "both panes must show stickers");

    const auto colours = draw();
    look.background[0] = look.background[2] = 0.0f;
    look.background[1] = 1.0f;
    api(ctx, sa2_scene_look_set(ctx, &look));
    const auto backdrop = draw();
    expect(ids() == original_ids, "the table and scene look changed the product IDs");
    const auto compare_background = [&](const auto& from, const auto& to) {
        size_t changed = 0;
        for (size_t i = 0; i < from.size(); ++i) {
            const bool clear = from[i] == default_background;
            expect(to[i] == (clear ? green_background : from[i]), "only the old background pixels should change to the new clear");
            changed += clear;
        }
        expect(changed > 0 && changed < from.size(), "the fixture needs background and sticker pixels");
        return changed;
    };
    const auto cleared = compare_background(colours, backdrop);
    const auto pane_cleared = compare_background(pane_colours, frames.draw_panes(1, panes));

    const auto refused = [](int32_t code) { expect(code == SA2_E_INVALID_ARGUMENT, "an invalid colour or scene look was accepted"); };
    auto bad_table = table;
    bad_table[3 * home] = 0.25f;
    refused(sa2_sticker_colours_set(ctx, bad_table.data(), SA2_STICKER_CLASSES - 1));
    bad_table.back() = std::numeric_limits<float>::quiet_NaN();
    refused(sa2_sticker_colours_set(ctx, bad_table.data(), SA2_STICKER_CLASSES));
    bad_table.back() = 1.5f;
    refused(sa2_sticker_colours_set(ctx, bad_table.data(), SA2_STICKER_CLASSES));
    auto bad_look = look;
    bad_look.background[0] = 1.0f;
    bad_look.struct_size -= uint32_t(sizeof(float));
    refused(sa2_scene_look_set(ctx, &bad_look));
    bad_look.struct_size = sizeof(bad_look);
    bad_look.fog = std::numeric_limits<float>::quiet_NaN();
    refused(sa2_scene_look_set(ctx, &bad_look));
    bad_look.fog = 1.5f;
    refused(sa2_scene_look_set(ctx, &bad_look));
    expect(draw() == backdrop, "an invalid argument changed the installed table or background");

    api(ctx, sa2_view_style_set(ctx, 1, &tint));
    const auto styled = draw();
    expect(ids() == original_ids, "the table and scene look changed the styled IDs");
    size_t tinted = 0, tinted_red = 0;
    for (size_t i = 0; i < backdrop.size(); ++i) {
        if (backdrop[i] == green_background) {
            expect(styled[i] == green_background, "the tint changed the background");
            continue;
        }
        const int base_red = int(backdrop[i] & 0xff), red = int(styled[i] & 0xff), blue = int(styled[i] >> 16 & 0xff);
        expect(blue > 0 && !(styled[i] & 0xff00) && std::abs(2 * red - base_red) <= 1,
               "the half-strength blue tint did not mix with the shaded base colour");
        if (base_red > 0) expect(std::abs(red - blue) <= 1, "the tint and sticker colour are shaded differently");
        tinted_red += base_red > 0;
        tinted += styled[i] != backdrop[i];
    }
    expect(tinted > 0 && tinted_red > 0, "the styled frame needs tinted pixels in the recoloured class");
    api(ctx, sa2_sticker_colours_set(ctx, table.data(), 0));
    api(ctx, sa2_scene_look_set(ctx, nullptr));
    expect(draw() == original_styled, "clearing both exports did not restore the styled frame bytes");

    // ADR-D02 B33: the table and the scene look are scene state, not revision state. Every new head the shell adopts (a
    // twist, a fixture load, jumble.on/off, a state switch) reaches the core as sa2_engine_adopt, which drops only the
    // revision-bound piece style, transform and chart channel, so the next revision draws the same recoloured bytes on
    // the same background without a resend. The tint and its piece style are cleared first: those are revision-bound.
    api(ctx, sa2_view_style_set(ctx, 1, nullptr));
    api(ctx, sa2_style_set(ctx, 1, nullptr, SA2_STYLE_CHANNEL_FILL));
    api(ctx, sa2_sticker_colours_set(ctx, table.data(), SA2_STICKER_CLASSES));
    api(ctx, sa2_scene_look_set(ctx, &look));
    const auto before = draw();
    expect(before[0] == green_background && before != original, "the table and scene look did not draw before the adoption");
    std::vector<uint32_t> labels(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) labels[slot] = slot;
    sa2_engine_image next{};
    next.struct_size = sizeof(next);
    next.revision = 2; // the same labels, a new revision
    next.labels = labels.data();
    uint64_t adopted = 0;
    api(ctx, sa2_engine_adopt(ctx, &next, &adopted));
    expect(adopted == 2, "the next revision was not adopted");
    expect(frames.draw(2, 1, set, q) == before, "an adoption dropped the sticker table or the scene look");
    std::cout << "scene colours: kept across an adoption (revision 2 draws revision 1's recoloured, green-backed bytes)" << std::endl;
    std::cout << "scene colours: " << recoloured << " palette pixels (0 outside class " << home << "), " << cleared
              << " background pixels, " << pane_cleared << " pane background pixels, " << tinted << " tint pixels ("
              << tinted_red << " over the recoloured class)" << std::endl;
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    harness.finish(options.debug);
}

void run_error(Suite& suite, const Options& options) {
    suite.check("error overlay warp", [&] {
        using namespace panes_check;
        Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
        constexpr uint32_t side = SA2_MIN_TEXTURE_PX;
        load(harness, side, side);
        auto* ctx = harness.ctx;
        const auto set = pickable_set(harness, 1, 1);
        const auto pieces = sa2::scene_slot_pieces();
        const auto q = double_rotation(0, 3, 0.35, 1, 2, 0.2);
        PoseFrames frames{harness};
        const auto draw = [&] { return frames.draw(1, 1, set, q); };
        const auto base = draw();
        const auto base_frame = frames.frame;
        const auto instance = [&](uint64_t frame, size_t i) {
            const auto row = pick(harness, frame, uint32_t(i % side), uint32_t(i / side));
            expect(row.state == SA2_PICK_READY && row.instance <= SA2_SLOTS, "the error overlay pick did not answer");
            return row.instance;
        };
        // SV_Position is at the pixel centre: (x + 0.5) + (y + 0.5).
        const auto hatch = [](size_t i) { return ((i % side + i / side + 1) % 8) < 4; };
        std::vector<uint8_t> outline(SA2_PIECES, 0);
        std::array<size_t, 2> sample{base.size(), base.size()};
        size_t named = 0;
        for (size_t i = 0; i < base.size() && named < sample.size(); ++i) {
            const size_t phase = hatch(i);
            if (base[i] == base[0] || base[i] == 0xffffffff || sample[phase] != base.size()) continue;
            const auto id = instance(base_frame, i);
            if (id == 0 || outline[pieces[id - 1]] != 0) continue;
            outline[pieces[id - 1]] = SA2_STYLE_ERROR;
            sample[phase] = i;
            ++named;
        }
        expect(named == sample.size(), "the fixture needs distinct visible pieces on both hatch phases");

        sa2_view_style style{};
        style.struct_size = sizeof(style);
        style.tint_mode = SA2_STYLE_TINT_OUTLINE; // zero band width isolates the hatch
        style.matrix[0] = style.matrix[4] = style.matrix[8] = 1;
        style.palette[0] = style.palette[1] = style.palette[2] = style.palette[3] = 1;
        api(ctx, sa2_view_style_set(ctx, 1, &style));
        expect(draw() == base, "entry 0 changed the frame with no error bit");
        style.palette[3] = 0;
        api(ctx, sa2_view_style_set(ctx, 1, &style));
        api(ctx, sa2_style_set(ctx, 1, outline.data(), SA2_STYLE_CHANNEL_OUTLINE));
        expect(draw() == base, "an error bit with entry 0's alpha zero changed the frame");
        style.palette[3] = 1;
        api(ctx, sa2_view_style_set(ctx, 1, &style));
        const auto painted = draw();
        const auto painted_frame = frames.frame;
        expect(painted[sample[0]] == base[sample[0]] && painted[sample[1]] == 0xffffffff,
            "the named pieces did not have both hatch-on and hatch-off pixels");
        size_t changed = 0;
        for (size_t i = 0; i < base.size(); ++i) {
            if (painted[i] == base[i]) continue;
            ++changed;
            const auto id = instance(painted_frame, i);
            expect(id != 0 && outline[pieces[id - 1]] != 0, "the hatch changed a pixel outside its named pieces");
            expect(id == instance(base_frame, i), "the hatch changed a pixel's ID");
            expect(hatch(i), "the hatch painted a pixel in the gap between stripes");
        }
        std::cout << "error overlay: " << changed << " changed pixels on " << named << " named pieces" << std::endl;
        api(ctx, sa2_drain(ctx, harness.wait));
        api(ctx, sa2_scene_unload(ctx));
        harness.finish(options.debug);
    });
}

void run_colours(Suite& suite, const Options& options) {
    expect(options.mode == "warp", "the scene colours check runs on WARP only");
    suite.check("scene colours warp", [&] { product_colours(options); });
}

// ADR-D02 B18: --warp --sweep. Only WARP; parse refuses any other mode.
void run_sweep(Suite& suite, const Options& options) {
    expect(options.mode == "warp", "the transform check runs on WARP only");
    suite.check("transform warp", [&] { product_sweep(options); });
    suite.check("transform picks warp", [&] { product_sweep_picks(options); });
}
// ADR-D02 B20: --warp --slice. Only WARP; parse refuses any other mode.
void run_slice(Suite& suite, const Options& options) {
    expect(options.mode == "warp", "the slice check runs on WARP only");
    suite.check("slice warp", [&] { product_slice(options); });
}

// ADR-D02 B21's WARP check (--warp --raw): the raw view on view 2 at focus cell 150, each frame one pane. A frame's
// clipped triangles (the newest harvested frame's pipeline statistics) count its drawn stickers' triangles exactly,
// since every chart here fits the target and no triangle is cut by the frustum.
void product_raw(const Options& options) {
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    auto* ctx = harness.ctx;
    std::vector<uint32_t> labels(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) labels[slot] = slot;
    sa2_engine_image image{};
    image.struct_size = sizeof(image);
    image.revision = 1;
    image.labels = labels.data();
    uint64_t adopted = 0;
    api(ctx, sa2_engine_adopt(ctx, &image, &adopted));
    PoseFrames frames{harness};
    using Newest = void (*)(uint64_t*);
    const auto newest = reinterpret_cast<Newest>(reinterpret_cast<void*>(
        GetProcAddress(GetModuleHandleW(L"sa2_interop.dll"), "sa2_test_frame_statistics")));
    expect(newest != nullptr, "sa2_interop.dll has no sa2_test_frame_statistics");
    const auto reports = [&] {
        sa2_frame_report rows[16]{};
        for (auto& row : rows) row.struct_size = sizeof(row);
        uint32_t count = 0;
        api(ctx, sa2_frame_reports(ctx, rows, 16, &count));
    };
    std::vector<uint32_t> last;
    const auto triangles = [&](const Rows& camera, uint32_t flags) {
        sa2_pane pane{};
        pane.view = 2; pane.view_generation = 1; pane.resource_set = 0;
        pane.width = harness.width; pane.height = harness.height;
        for (int r = 0; r < 4; ++r) for (int c = 0; c < 4; ++c) pane.camera_matrix[c * 4 + r] = float(camera[r * 4 + c]);
        pane.flags = flags;
        reports();
        last = frames.draw_panes(1, {pane});
        reports();
        uint64_t s[4]{};
        newest(s);
        expect(s[0] == frames.frame, "the newest harvested frame is not the measured one");
        return s[2];
    };
    const auto focus = [&](uint32_t projection, uint32_t cell, float h) {
        const sa2_view_focus f{sizeof(sa2_view_focus), projection, cell, h, 1.0f, 0};
        return sa2_view_focus_set(ctx, 2, &f);
    };
    // Zoom 1.6: clip w is 5 - p.z, so even the h = 0 chart (radius about 2) stays inside the frustum.
    const sa2_view_shape shape{sizeof(sa2_view_shape), 0.76f, 0.82f, 1.18f, 1.6f, 0};
    api(ctx, sa2_view_shape_set(ctx, 2, &shape));
    expect(focus(SA2_PROJECTION_STEREOGRAPHIC, SA2_FOCUS_NONE, 0.5f) == SA2_E_INVALID_ARGUMENT
        && focus(SA2_PROJECTION_STEREOGRAPHIC, 150, -0.01f) == SA2_E_INVALID_ARGUMENT, "a raw view without a pole or past 90 degrees was taken");
    const Rows I = double_rotation(0, 1, 0, 2, 3, 0);
    api(ctx, focus(SA2_PROJECTION_ORTHOGRAPHIC, SA2_FOCUS_NONE, -1.0f));
    const auto whole = triangles(I, 0);
    expect(triangles(I, SA2_PANE_ANTIPODE) == whole, "the antipode flag changed a frame without the raw view");
    // At h = 0 every sticker is drawn in exactly one of the two charts.
    api(ctx, focus(SA2_PROJECTION_STEREOGRAPHIC, 150, 0.0f));
    const auto near0 = triangles(I, 0), far0 = triangles(I, SA2_PANE_ANTIPODE);
    std::cout << "raw: whole " << whole << " triangles; at h = 0 the chart draws " << near0 << " and the antipode chart " << far0 << '\n' << std::flush;
    expect(near0 > 0 && far0 > 0 && near0 + far0 == whole, "at h = 0 the two charts together do not draw every sticker exactly once");
    // Nothing draws past the clip: the raw chart draws exactly the stickers the cap test keeps at the same h.
    for (const float h : {121.0f / 125.0f, 0.5f}) {
        api(ctx, focus(SA2_PROJECTION_ORTHOGRAPHIC, 150, h));
        const auto cap = triangles(I, 0);
        api(ctx, focus(SA2_PROJECTION_STEREOGRAPHIC, 150, h));
        const auto raw = triangles(I, 0), antipode = triangles(I, SA2_PANE_ANTIPODE);
        std::cout << "raw: h " << h << ": the cap test draws " << cap << " triangles, the raw chart " << raw << ", the antipode chart "
                  << antipode << '\n' << std::flush;
        expect(cap > 0 && raw == cap, "the raw chart does not draw exactly the stickers inside its clip");
        // The cells are symmetric under -I but their sticker triangulations are not exactly (at h 0.968: 103,500 and 102,652).
        expect(antipode > 0 && std::max(antipode, raw) - std::min(antipode, raw) <= raw / 50, "the antipode chart does not draw about as many triangles as the chart");
    }
    // A rotation that keeps the pole (a twist of the named cap; here the camera's spin in the screen plane) turns each
    // chart rigidly about its centre: the turned frame's covered pixels, turned back, land on the first frame's.
    api(ctx, focus(SA2_PROJECTION_STEREOGRAPHIC, 150, 0.5f));
    const Rows spun = double_rotation(0, 1, 0.6, 2, 3, 0);
    for (const uint32_t flags : {0u, SA2_PANE_ANTIPODE}) {
        triangles(I, flags);
        const auto a = last;
        triangles(spun, flags);
        const auto b = last;
        const auto background = a[0];
        const int w = int(harness.width), h = int(harness.height);
        uint64_t cover = 0, best = 0;
        for (const double angle : {0.6, -0.6}) {
            uint64_t seen = 0, match = 0;
            for (int y = 0; y < h; ++y)
                for (int x = 0; x < w; ++x) {
                    if (b[size_t(y) * w + x] == background) continue;
                    ++seen;
                    const double dx = x + 0.5 - w / 2.0, dy = y + 0.5 - h / 2.0;
                    const int ax = int(std::floor(w / 2.0 + dx * std::cos(angle) + dy * std::sin(angle)));
                    const int ay = int(std::floor(h / 2.0 - dx * std::sin(angle) + dy * std::cos(angle)));
                    bool hit = false;
                    for (int oy = -1; oy <= 1 && !hit; ++oy)
                        for (int ox = -1; ox <= 1 && !hit; ++ox) {
                            const int px = ax + ox, py = ay + oy;
                            hit = px >= 0 && py >= 0 && px < w && py < h && a[size_t(py) * w + px] != background;
                        }
                    match += hit ? 1 : 0;
                }
            cover = seen;
            best = std::max(best, match);
        }
        std::cout << "raw: turned about the centre" << (flags ? " (antipode)" : "") << ": " << best << " of " << cover
                  << " covered pixels land on covered pixels\n" << std::flush;
        expect(cover > 1000 && best * 100 >= cover * 98, "a rotation that keeps the pole did not turn the chart rigidly about its centre");
    }
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    harness.finish(options.debug);
}

// ADR-D02 B21: --warp --raw. Only WARP; parse refuses any other mode.
void run_raw(Suite& suite, const Options& options) {
    expect(options.mode == "warp", "the raw view check runs on WARP only");
    suite.check("raw view warp", [&] { product_raw(options); });
}

// ABI 10: --warp --hopf FILE. Only WARP; parse refuses any other mode.
// ADR-D02 B22 item 1 (--warp --chart FILE): the chart channel on the ring sheet. Revision 2 poses every piece one place
// along its ring (x -> x u, u = cos 12 deg + J sin 12 deg, which takes each cell centre to the next place of its ring).
// Six sample home cells get their own colours. With the channel naming each slot's next cell, every sample sticker draws
// inside the box in which the unposed sheet draws that next cell's own stickers (its chart's rectangle); with a wrong
// channel (each slot's own cell) none stays inside; without a channel the Hopf-coordinate choice draws the same picture;
// a chain view ignores the channel; an adoption drops it; malformed channels are refused and change nothing.
void product_chart(const Options& options) {
    const auto layout = hopf_layout(options.hopf);
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    auto* ctx = harness.ctx;
    std::array<uint32_t, 1200> at{}; // ring * 60 + half step: 20 rings of 60 half steps
    std::array<uint32_t, 600> next{};
    for (uint32_t c = 0; c < 600; ++c) at[layout.ring_of[c] * 60u + layout.half_step[c]] = c;
    for (uint32_t c = 0; c < 600; ++c) next[c] = at[layout.ring_of[c] * 60u + (layout.half_step[c] + 2u) % 60u];
    const double t = 12.0 * 3.14159265358979323846 / 180.0;
    const double u[4] = {std::cos(t), std::sin(t) * layout.j[1], std::sin(t) * layout.j[2], std::sin(t) * layout.j[3]};
    const auto qmul = [](const double* a, const double* b, double* out) {
        out[0] = a[0]*b[0] - a[1]*b[1] - a[2]*b[2] - a[3]*b[3]; out[1] = a[0]*b[1] + a[1]*b[0] + a[2]*b[3] - a[3]*b[2];
        out[2] = a[0]*b[2] - a[1]*b[3] + a[2]*b[0] + a[3]*b[1]; out[3] = a[0]*b[3] + a[1]*b[2] - a[2]*b[1] + a[3]*b[0];
    };
    std::vector<float> identity(16, 0.0f), turn(16, 0.0f);
    for (int k = 0; k < 4; ++k) {
        identity[size_t(k) * 5] = 1.0f;
        double e[4] = {0, 0, 0, 0}, column[4];
        e[k] = 1.0;
        qmul(e, u, column);
        for (int r = 0; r < 4; ++r) turn[size_t(r) * 4 + k] = float(column[r]); // row-major, x' = M x
    }
    // Pose 0 is the identity on the lattice (index 0); revision 2 puts every piece at pose 1, the turn, off it.
    const std::vector<int32_t> home(SA2_PIECES, 0), index(SA2_PIECES, 1), lattice{0, -1};
    std::vector<float> both(identity);
    both.insert(both.end(), turn.begin(), turn.end());
    adopt_poses(harness, 1, home, identity, {0});
    auto sheet = layout;
    sheet.kind = SA2_LAYOUT_RING_SHEET;
    api(ctx, sa2_view_layout_set(ctx, 1, &sheet));
    const uint32_t samples[6] = {0, 101, 202, 303, 404, 505};
    const uint32_t pattern[6] = {1, 2, 4, 3, 6, 5}; // R, G, B bits
    const auto colours = [&](const std::array<uint32_t, 6>& cells) {
        std::vector<float> table(size_t(SA2_STICKER_CLASSES) * 3, 0.5f);
        for (int i = 0; i < 6; ++i)
            for (int ch = 0; ch < 3; ++ch) table[size_t(cells[size_t(i)]) * 3 + ch] = (pattern[i] >> ch) & 1u ? 1.0f : 0.0f;
        api(ctx, sa2_sticker_colours_set(ctx, table.data(), SA2_STICKER_CLASSES));
    };
    std::array<uint32_t, 6> own{}, target{}, beyond{};
    for (int i = 0; i < 6; ++i) { own[size_t(i)] = samples[i]; target[size_t(i)] = next[samples[i]]; beyond[size_t(i)] = next[next[samples[i]]]; }
    PoseFrames frames{harness};
    const Rows I = double_rotation(0, 3, 0, 1, 2, 0);
    struct Region { uint64_t count = 0; int left = 1 << 30, right = -1, top = 1 << 30, bottom = -1; };
    const auto regions = [&](const std::vector<uint32_t>& picture) {
        std::array<Region, 6> out{};
        for (uint32_t y = 0; y < harness.height; ++y)
            for (uint32_t x = 0; x < harness.width; ++x) {
                const uint32_t w = picture[size_t(y) * harness.width + x];
                uint32_t bits = 0;
                bool clear = true;
                for (int ch = 0; ch < 3; ++ch) {
                    const uint32_t v = (w >> (8 * ch)) & 0xFFu;
                    if (v > 60) bits |= 1u << ch; else if (v > 10) clear = false;
                }
                if (!clear || bits == 0 || bits == 7) continue;
                for (int i = 0; i < 6; ++i)
                    if (pattern[i] == bits) {
                        auto& r = out[size_t(i)];
                        ++r.count;
                        r.left = std::min(r.left, int(x)); r.right = std::max(r.right, int(x));
                        r.top = std::min(r.top, int(y)); r.bottom = std::max(r.bottom, int(y));
                    }
            }
        return out;
    };
    // A chart's rectangle on the sheet: one column (two half steps) wide and one ring high, centred on the chart's own
    // stickers; the pitch is measured from the next chart along the same ring.
    struct Tile { double cx = 0, cy = 0, half = 0; };
    std::array<Tile, 6> tiles{};
    // Outside counts each sample's pixels outside its chart's rectangle, with 2 pixels of slack: the measured centre and
    // pitch are each good to about half a pixel, and rasterization adds half a pixel.
    const auto outside = [&](const std::vector<uint32_t>& picture, const std::array<Tile, 6>& boxes, int i) {
        uint64_t n = 0;
        for (uint32_t y = 0; y < harness.height; ++y)
            for (uint32_t x = 0; x < harness.width; ++x) {
                const uint32_t w = picture[size_t(y) * harness.width + x];
                uint32_t bits = 0;
                bool clear = true;
                for (int ch = 0; ch < 3; ++ch) {
                    const uint32_t v = (w >> (8 * ch)) & 0xFFu;
                    if (v > 60) bits |= 1u << ch; else if (v > 10) clear = false;
                }
                if (!clear || bits != pattern[i]) continue;
                const auto& b = boxes[size_t(i)];
                n += std::fabs(double(x) - b.cx) > b.half + 2.0 || std::fabs(double(y) - b.cy) > b.half + 2.0;
            }
        return n;
    };
    colours(target);
    const auto reference = regions(frames.draw(1, 1, 0, I));
    colours(beyond);
    const auto further = regions(frames.draw(1, 1, 0, I));
    for (int i = 0; i < 6; ++i) {
        const auto& r = reference[size_t(i)];
        const auto& f = further[size_t(i)];
        expect(r.count > 0 && f.count > 0, "the unposed sheet drew no sticker of a sample's next cell");
        auto& tile = tiles[size_t(i)];
        tile.cx = (r.left + r.right) / 2.0;
        tile.cy = (r.top + r.bottom) / 2.0;
        tile.half = std::fabs((f.left + f.right) / 2.0 - tile.cx) / 2.0;
        expect(tile.half >= 2.0 && r.right - tile.cx <= tile.half + 1.0 && r.bottom - tile.cy <= tile.half + 1.0,
               "a chart's own stickers do not fit its measured rectangle");
    }
    adopt_poses(harness, 2, index, both, lattice);
    colours(own);
    const auto unset = frames.draw(2, 1, 0, I);
    std::vector<uint32_t> right(SA2_SLOTS), wrong(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) { wrong[slot] = slot / 433; right[slot] = next[slot / 433]; }
    const auto chain_unset = frames.draw(2, 2, 0, I);
    api(ctx, sa2_charts_set(ctx, 2, wrong.data(), SA2_SLOTS));
    const auto bad = frames.draw(2, 1, 0, I);
    expect(frames.draw(2, 2, 0, I) == chain_unset, "a chain view read the chart channel");
    api(ctx, sa2_charts_set(ctx, 2, right.data(), SA2_SLOTS));
    const auto good = frames.draw(2, 1, 0, I);
    const auto drawn = regions(good);
    uint64_t total_out_good = 0, moved_out = 0;
    for (int i = 0; i < 6; ++i) {
        const uint64_t out_good = outside(good, tiles, i), out_bad = outside(bad, tiles, i);
        std::cout << "chart channel: sample " << samples[i] << " -> chart " << next[samples[i]] << ": " << drawn[size_t(i)].count
                  << " pixels (unposed next cell " << reference[size_t(i)].count << ", rectangle " << 2 * tiles[size_t(i)].half
                  << " px), outside its chart's rectangle " << out_good
                  << "; with each slot's own cell " << out_bad << " outside" << std::endl;
        expect(drawn[size_t(i)].count > 0, "a sample drew nothing under the chart channel");
        total_out_good += out_good;
        moved_out += out_bad > 0 || regions(bad)[size_t(i)].count == 0;
    }
    expect(total_out_good == 0, "a sample sticker drew outside its assigned chart's rectangle");
    expect(moved_out == 6, "the wrong channel left a sample inside its next cell's chart");
    uint64_t differ = 0;
    for (size_t k = 0; k < good.size(); ++k) differ += good[k] != unset[k];
    std::cout << "chart channel: the Hopf-coordinate choice differs from the channel at " << differ << " pixels" << std::endl;
    expect(differ == 0, "the Hopf-coordinate choice drew another picture than the nearest-cell channel");
    std::vector<uint32_t> past(right);
    past[7] = 600;
    expect(sa2_charts_set(ctx, 2, past.data(), SA2_SLOTS) == SA2_E_INVALID_ARGUMENT, "a cell past 599 was accepted");
    expect(sa2_charts_set(ctx, 2, right.data(), SA2_SLOTS - 1) == SA2_E_INVALID_ARGUMENT, "a short chart channel was accepted");
    expect(sa2_charts_set(ctx, 1, right.data(), SA2_SLOTS) == SA2_E_INVALID_ARGUMENT, "a channel for another revision was accepted");
    expect(sa2_charts_set(ctx, 2, nullptr, 5) == SA2_E_INVALID_ARGUMENT, "a NULL channel with a count was accepted");
    expect(sa2_charts_set(nullptr, 2, right.data(), SA2_SLOTS) == SA2_E_INVALID_ARGUMENT, "a channel was set without a context");
    expect(frames.draw(2, 1, 0, I) == good, "a refused chart channel changed the picture");
    // C3/C4 finding F4 (27 November 2026): a pick on a composite frame reads the chart channel its pane drew with, as a
    // one-view pick does (pick_plan): once the channel changes, a pane's pick on the older frame is stale.
    {
        const auto set = panes_check::pickable_set(harness, 1, 2);
        sa2_pane pane{};
        pane.view = 1; pane.view_generation = 1; pane.resource_set = set;
        pane.width = 640; pane.height = 360;
        for (int r = 0; r < 4; ++r) for (int c = 0; c < 4; ++c) pane.camera_matrix[c * 4 + r] = float(I[r * 4 + c]);
        frames.draw(2, 1, set, I);
        const auto one_view = frames.frame;
        frames.draw_panes(2, {pane});
        const auto composite = frames.frame;
        const auto before = panes_check::pick(harness, composite, 320, 180);
        expect(before.state == SA2_PICK_READY, "a pane pick under an unchanged chart channel was not ready");
        // Each changed channel gets a fresh copy of the revision (EngineCopies = 3), so three changes, each drawn,
        // overwrite the copy both older frames drew with: their picks must then be stale, never read the new channel.
        for (int k = 0; k < 3; ++k) {
            api(ctx, sa2_charts_set(ctx, 2, (k % 2 ? right : wrong).data(), SA2_SLOTS));
            frames.draw(2, 1, 0, I);
        }
        const auto one = panes_check::pick(harness, one_view, 320, 180), panes = panes_check::pick(harness, composite, 320, 180);
        std::cout << "chart channel: after a change the one-view pick is " << one.state << "/" << one.stale
                  << " and the pane pick " << panes.state << "/" << panes.stale << " (state/stale)" << std::endl;
        expect(one.state == SA2_PICK_STALE && one.stale == SA2_PICK_STALE_ENGINE, "a one-view pick read a changed chart channel");
        expect(panes.state == SA2_PICK_STALE && panes.stale == SA2_PICK_STALE_ENGINE, "a pane pick read a changed chart channel");
    }
    adopt_poses(harness, 3, index, both, lattice);
    expect(frames.draw(3, 1, 0, I) == unset, "an adoption kept the previous revision's chart channel");
    api(ctx, sa2_charts_set(ctx, 3, nullptr, 0));
    std::cout << "chart channel: every sample draws inside its assigned chart's rectangle" << std::endl;
}

void run_hopf(Suite& suite, const Options& options) {
    expect(options.mode == "warp", "the Hopf layout check runs on WARP only");
    suite.check("view layout warp", [&] { product_hopf(options); });
}

// ADR-D02 B22 item 1: --warp --chart FILE. Only WARP, alone, as the Hopf check.
void run_chart(Suite& suite, const Options& options) {
    expect(options.mode == "warp", "the chart channel check runs on WARP only");
    suite.check("chart channel warp", [&] { product_chart(options); });
}

// ABI 9: --warp --focus. Only WARP; parse refuses any other mode.
void run_focus(Suite& suite, const Options& options) {
    expect(options.mode == "warp", "the focus check runs on WARP only");
    suite.check("view focus warp", [&] { product_focus(options); });
}

// ABI 8: --warp --panes. Only WARP; parse refuses any other mode.
void run_panes(Suite& suite, const Options& options) {
    expect(options.mode == "warp", "the composite frame check runs on WARP only");
    suite.check("panes warp", [&] { product_panes(options); });
}

// ABI 7: --warp --style. Only WARP; parse refuses any other mode.
void run_style(Suite& suite, const Options& options) {
    expect(options.mode == "warp", "the style check runs on WARP only");
    suite.check("piece style warp", [&] { product_style(options); });
    suite.check("focus layers warp", [&] { product_focus_layers(options); }); // ADR-D02 B17 (ABI 10)
}

// C9 U5c: --warp --measure-stages DIR. Only WARP; parse refuses any other mode.
void run_measure(const Options& options) {
    measure_stages(options);
}

// C9 (26 Nov 2026): the shell's per-twist preparation, timed. Each round prepares a full set on one view (every slot
// drawn and pickable), waits until it is ready, activates it and retires the previous set, as the shell's backend does
// for each twist; one JSON line per round gives the duration of the sa2_prepare call alone.
void run_measure_prepare(const Options& options) {
    Harness harness(options, SA2_QUEUE_SAME, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(640, 360);
    sa2_scene_config config{sizeof(sa2_scene_config), 1, 0, nullptr, SA2_SCENE_NO_VRAM, 1000};
    api(harness.ctx, sa2_scene_load(harness.ctx, &config));
    auto* ctx = harness.ctx;
    const std::vector<uint8_t> visible(SA2_SLOTS, 1), interaction(SA2_SLOTS, 1);
    std::vector<uint32_t> instances(SA2_SLOTS);
    for (uint32_t slot = 0; slot < SA2_SLOTS; ++slot) instances[slot] = slot + 1;
    std::ofstream out(options.out, std::ios::binary);
    expect(static_cast<bool>(out), "could not open the --out file");
    uint64_t previous = 0;
    for (uint32_t round = 1; round <= options.prepares; ++round) {
        sa2_preparation preparation{};
        preparation.struct_size = sizeof(preparation);
        preparation.command = round; preparation.view = 1;
        preparation.view_generation = 1; preparation.publication_generation = round;
        preparation.device_epoch = 1; preparation.base_revision = 1;
        preparation.visibility = visible.data(); preparation.interaction = interaction.data(); preparation.instances = instances.data();
        uint64_t ticket = 0;
        const auto started = std::chrono::steady_clock::now();
        api(ctx, sa2_prepare(ctx, &preparation, &ticket));
        const auto ended = std::chrono::steady_clock::now();
        const auto until = ended + std::chrono::milliseconds(harness.wait);
        for (;;) {
            sa2_poll_row row{sizeof(sa2_poll_row)};
            api(ctx, sa2_poll(ctx, ticket, &row));
            if (row.state == SA2_TICKET_READY) {
                const uint8_t digest[32]{};
                api(ctx, sa2_activate(ctx, row.resource_set, digest));
                if (previous) api(ctx, sa2_retire(ctx, previous));
                previous = row.resource_set;
                break;
            }
            expect(row.state == SA2_TICKET_PENDING, "a measured preparation was cancelled or failed");
            expect(std::chrono::steady_clock::now() < until, "a measured preparation did not become ready in time");
            Sleep(1);
        }
        const double ms = std::chrono::duration<double, std::milli>(ended - started).count();
        out << "{\"round\": " << round << ", \"prepare_ms\": " << ms << "}\n";
    }
    expect(static_cast<bool>(out), "could not write the --out file");
    api(ctx, sa2_drain(ctx, harness.wait));
    api(ctx, sa2_scene_unload(ctx));
    harness.finish(options.debug);
}

int run_child(const Options& options) {
    Harness harness(options, SA2_QUEUE_OWN, SA2_BARRIERS_LEGACY, false);
    harness.make_ring(256, 192);
    for (uint32_t frame = 1; frame <= 5; ++frame) {
        api(harness.ctx, sa2_signal_godot_free(harness.ctx, frame - 1));
        api(harness.ctx, sa2_produce(harness.ctx, frame % SA2_RING_SLOTS, frame, frame, 1));
    }
    if (options.child == "inject-drain") {
        win_check(SetEnvironmentVariableW(L"M600_SA2_INJECT_UNCONFIRMED_DRAIN", L"1"), "SetEnvironmentVariableW(inject)");
        sa2_drain(harness.ctx, timeout_ms);
        std::cout << "child: after drain\n" << std::flush;
        return 1;
    }
    expect(options.child == "device-loss", "unknown selftest child");
    api(harness.ctx, sa2_remove_device(harness.ctx));
    expect(sa2_drain(harness.ctx, timeout_ms) == SA2_E_DEVICE_REMOVED, "removed-device drain did not return SA2_E_DEVICE_REMOVED");
    int32_t reason = 0;
    api(harness.ctx, sa2_device_removed_reason(harness.ctx, &reason));
    expect(FAILED(static_cast<HRESULT>(reason)), "removed device reported a success HRESULT");
    harness.clear_ring(); // Removal permits releases without a drain confirmation.
    api(harness.ctx, sa2_detach(harness.ctx));
    harness.ctx = nullptr;
    std::cout << "child: device-loss ok\n" << std::flush;
    return 0;
}
}
