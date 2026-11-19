#include "test.h"
#include "scene_record.h"
#include "stage_rules.h"
#include "stage_sources.h"
#include <d3d12sdklayers.h>
#include <dxgi1_6.h>
#include <wrl/client.h>
#include <algorithm>
#include <bit>
#include <cstring>
#include <iomanip>
#include <iostream>
#include <fstream>
#include <sstream>
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

    Harness(const Options& options, int32_t queue_mode, int32_t barriers, bool is_external, const Device* shared = nullptr)
        : env(shared ? *shared : Device(options)), enhanced(barriers == SA2_BARRIERS_ENHANCED), external(is_external) {
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
        config.wait_timeout_ms = timeout_ms;
        config.debug_callback = 1;
        api(nullptr, sa2_attach(handle(env.device.Get()), handle(env.queue.Get()), &config, &ctx));
    }
    ~Harness() {
        // Even failed tests must finish queued consumer and producer work before
        // their owning COM pointers unwind. An unconfirmed drain exits in DLL.
        if (!ctx) return;
        sa2_drain(ctx, timeout_ms);
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
        expect(WaitForSingleObject(completion.get(), timeout_ms) == WAIT_OBJECT_0, "consumer fence wait failed or timed out");
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
        api(ctx, sa2_drain(ctx, timeout_ms));
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
    const auto last = frame;
    // Releases: an attached handle is refused, a free one is released once.
    expect(sa2_stage_release(ctx, fragment.handle) == SA2_E_WRONG_STATE, "an attached stage was released");
    api(ctx, sa2_stage_release(ctx, vertex.handle));
    expect(sa2_stage_release(ctx, vertex.handle) == SA2_E_INVALID_ARGUMENT
        && sa2_scene_stages(ctx, vertex.handle, fragment.handle, 1, nullptr, 0, nullptr, 0) == SA2_E_INVALID_ARGUMENT,
        "a released handle was released or attached again");
    // A traced frame takes the built-in path and the chain cannot change inside the trace.
    api(ctx, sa2_scene_trace_begin(ctx));
    expect(sa2_scene_stages(ctx, 0, 0, 0, nullptr, 0, nullptr, 0) == SA2_E_WRONG_STATE, "the chain changed inside a trace");
    ++frame;
    expect(stage_produce(harness, frame).frame == last, "a traced frame drew with the chain");
    api(ctx, sa2_scene_trace_end(ctx));
    // Detached: the built-in path again; then the last handle is released.
    api(ctx, sa2_scene_stages(ctx, 0, 0, 0, nullptr, 0, nullptr, 0));
    ++frame;
    expect(stage_produce(harness, frame).frame == last, "a frame after the detach drew with the chain");
    api(ctx, sa2_stage_release(ctx, fragment.handle));
    expect(sa2_stage_release(ctx, fragment.handle) == SA2_E_INVALID_ARGUMENT, "a released fragment stage was released again");
    api(ctx, sa2_drain(ctx, timeout_ms));
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
