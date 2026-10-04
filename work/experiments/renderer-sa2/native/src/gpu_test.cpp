#include "test.h"
#include <d3d12sdklayers.h>
#include <dxgi1_6.h>
#include <wrl/client.h>
#include <algorithm>
#include <cstring>
#include <iomanip>
#include <iostream>
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

    Harness(const Options& options, int32_t queue_mode, int32_t barriers, bool is_external)
        : env(options), enhanced(barriers == SA2_BARRIERS_ENHANCED), external(is_external) {
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
    void consume(uint32_t frame, uint32_t generation) {
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
            expect(messages[0] == 0, "debug layer recorded a warning other than mismatching clear value");
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
