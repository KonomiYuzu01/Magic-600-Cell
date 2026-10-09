#include "handoff.h"

#include <windows.h>
#include <d3d12.h>
#include <d3d12sdklayers.h>
#include <d3d11_4.h>
#include <dxgi1_6.h>
#include <wrl/client.h>
#include <algorithm>
#include <chrono>
#include <cstring>
#include <iomanip>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <utility>

namespace sb {
namespace {
using Microsoft::WRL::ComPtr;
using Clock = std::chrono::steady_clock;
constexpr DWORD wait_ms = 30000;
constexpr DWORD report_wait_ms = 10000;

struct Unsupported : std::runtime_error { using std::runtime_error::runtime_error; };
std::function<void(const Result&)> abandon_report;
HANDLE abandon_cancel = nullptr; // cancel event of the running child, if any
std::string inject_unconfirmed_drain; // role whose drains fail (test hook)
std::string hr_text(const char* operation, HRESULT hr) {
    std::ostringstream out;
    out << operation << " failed: HRESULT 0x" << std::hex << std::setw(8) << std::setfill('0') << static_cast<unsigned long>(hr);
    return out.str();
}
void check(HRESULT hr, const char* operation) {
    if (FAILED(hr)) throw std::runtime_error(hr_text(operation, hr));
}
void capability(HRESULT hr, const char* operation) {
    if (hr == E_NOINTERFACE || hr == E_NOTIMPL || hr == DXGI_ERROR_UNSUPPORTED)
        throw Unsupported(hr_text(operation, hr));
    check(hr, operation);
}
void win_check(BOOL ok, const char* operation) {
    if (!ok) check(HRESULT_FROM_WIN32(GetLastError()), operation);
}
void fail(Result& result, const std::string& error) {
    result.status = "fail";
    if (result.reason.empty()) result.reason = error;
    result.errors.push_back(error);
}
void failing_iteration(Result& result, std::uint64_t n) {
    if (n && (!result.first_failure || n < *result.first_failure)) result.first_failure = n;
}
DWORD WINAPI send_abandon_report(void* failed) {
    try { abandon_report(*static_cast<const Result*>(failed)); } catch (...) {}
    return 0;
}
// A drain that fails without device removal does not show that the GPU has
// finished with this process's objects, so releasing any of them could free
// memory still in use. The drain calls this before any unwinding: it records the
// failure and ends the process without running destructors. Windows reclaims
// GPU objects only after the GPU stops using them. Recording is best-effort and
// nothing here throws, so that a failure such as std::bad_alloc cannot unwind
// and release objects; the message stays a const char* until then.
[[noreturn]] void abandon(Result& result, const char* error) noexcept {
    try {
        result.status = "fail";
        result.teardown = "fail";
        fail(result, std::string(error) + "; drain unconfirmed, so the process ended without releasing GPU objects");
    } catch (...) {}
    if (abandon_cancel) SetEvent(abandon_cancel);
    // Report on another thread and wait a bounded time, so that a reader that
    // stops reading a pipe cannot keep this process alive.
    if (abandon_report) {
        if (HANDLE thread = CreateThread(nullptr, 0, send_abandon_report, &result, 0, nullptr))
            WaitForSingleObject(thread, report_wait_ms);
    }
    TerminateProcess(GetCurrentProcess(), abandoned_exit);
    ExitProcess(abandoned_exit);
}
class Handle {
    HANDLE value_ = nullptr;
public:
    Handle() = default;
    explicit Handle(HANDLE value) : value_(value) {}
    ~Handle() { reset(); }
    Handle(const Handle&) = delete;
    Handle& operator=(const Handle&) = delete;
    Handle(Handle&& other) noexcept : value_(other.release()) {}
    Handle& operator=(Handle&& other) noexcept { reset(other.release()); return *this; }
    HANDLE get() const { return value_; }
    HANDLE release() { return std::exchange(value_, nullptr); }
    void reset(HANDLE value = nullptr) {
        if (value_ && value_ != INVALID_HANDLE_VALUE) CloseHandle(value_);
        value_ = value;
    }
};
Handle event(bool manual = false) {
    Handle result(CreateEventW(nullptr, manual, FALSE, nullptr));
    win_check(result.get() != nullptr, "CreateEvent");
    return result;
}
template<class Fence> void wait_fence(Fence* fence, std::uint64_t value, HANDLE completion,
                                    HANDLE child = nullptr, HANDLE cancel = nullptr) {
    const auto completed = fence->GetCompletedValue();
    if (completed == UINT64_MAX) throw std::runtime_error("fence reports device removal");
    if (completed >= value) return;
    check(fence->SetEventOnCompletion(value, completion), "SetEventOnCompletion");
    HANDLE handles[3] = {completion};
    DWORD count = 1;
    if (child) handles[count++] = child;
    if (cancel) handles[count++] = cancel;
    const auto status = WaitForMultipleObjects(count, handles, FALSE, wait_ms);
    if (status == WAIT_TIMEOUT) throw std::runtime_error("fence wait timed out after 30 s");
    if (status == WAIT_FAILED) win_check(FALSE, "WaitForMultipleObjects");
    if (status != WAIT_OBJECT_0) throw std::runtime_error(child ? "consumer child exited before the free fence" : "handoff cancelled");
    if (fence->GetCompletedValue() == UINT64_MAX || fence->GetCompletedValue() < value)
        throw std::runtime_error("fence completion did not reach the requested value");
}
std::uint64_t luid_bits(LUID luid) {
    return (std::uint64_t(static_cast<std::uint32_t>(luid.HighPart)) << 32) | luid.LowPart;
}
LUID make_luid(std::uint64_t bits) {
    return {static_cast<DWORD>(bits), static_cast<LONG>(bits >> 32)};
}
std::string luid_string(LUID luid) {
    std::ostringstream out;
    out << std::hex << std::setfill('0') << std::setw(8) << static_cast<std::uint32_t>(luid.HighPart)
        << ':' << std::setw(8) << luid.LowPart;
    return out.str();
}
double elapsed(Clock::time_point start) {
    return std::chrono::duration<double, std::milli>(Clock::now() - start).count();
}

struct Device {
    ComPtr<IDXGIAdapter1> adapter;
    ComPtr<ID3D12Device> device;
    ComPtr<ID3D12InfoQueue> info;
    explicit Device(const Options& options, Result& result) {
        if (options.debug) {
            ComPtr<ID3D12Debug> debug;
            check(D3D12GetDebugInterface(IID_PPV_ARGS(&debug)), "D3D12GetDebugInterface (debug layer requested)");
            debug->EnableDebugLayer();
        }
        ComPtr<IDXGIFactory6> factory;
        capability(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)), "CreateDXGIFactory2");
        if (options.luid) {
            capability(factory->EnumAdapterByLuid(make_luid(*options.luid), IID_PPV_ARGS(&adapter)), "EnumAdapterByLuid");
        } else if (options.warp) {
            capability(factory->EnumWarpAdapter(IID_PPV_ARGS(&adapter)), "EnumWarpAdapter");
        } else {
            for (UINT i = 0;; ++i) {
                const auto hr = factory->EnumAdapterByGpuPreference(i, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE, IID_PPV_ARGS(&adapter));
                if (hr == DXGI_ERROR_NOT_FOUND) throw Unsupported("no high-performance hardware adapter available");
                capability(hr, "EnumAdapterByGpuPreference");
                DXGI_ADAPTER_DESC1 desc{};
                check(adapter->GetDesc1(&desc), "GetDesc1");
                if (!(desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE)) break;
                adapter.Reset();
            }
        }
        DXGI_ADAPTER_DESC1 desc{};
        check(adapter->GetDesc1(&desc), "GetDesc1");
        result.producer_luid = luid_string(desc.AdapterLuid);
        capability(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device)), "D3D12CreateDevice");
        if (luid_bits(device->GetAdapterLuid()) != luid_bits(desc.AdapterLuid))
            throw std::runtime_error("D3D12 device adapter LUID differs from the selected adapter");
        ComPtr<ID3D12Device> again;
        check(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&again)), "D3D12CreateDevice singleton check");
        result.singleton = again.Get() == device.Get();
        if (options.debug) check(device.As(&info), "ID3D12InfoQueue");
    }
    void report(Result& result) {
        if (!info) return;
        // Capture runtime errors before clearing the queue for the live-object report.
        for (UINT64 i = 0; i < info->GetNumStoredMessages(); ++i) {
            SIZE_T length = 0;
            check(info->GetMessage(i, nullptr, &length), "GetMessage size");
            std::vector<unsigned char> storage(length);
            auto* message = reinterpret_cast<D3D12_MESSAGE*>(storage.data());
            check(info->GetMessage(i, message, &length), "GetMessage");
            if (message->Severity == D3D12_MESSAGE_SEVERITY_ERROR || message->Severity == D3D12_MESSAGE_SEVERITY_CORRUPTION)
                fail(result, std::string("D3D12 debug: ") + message->pDescription);
        }
        info->ClearStoredMessages();
        info->ClearStorageFilter();
        ComPtr<ID3D12DebugDevice> debug;
        check(device.As(&debug), "ID3D12DebugDevice");
        check(debug->ReportLiveDeviceObjects(static_cast<D3D12_RLDO_FLAGS>(D3D12_RLDO_DETAIL | D3D12_RLDO_IGNORE_INTERNAL)),
              "ReportLiveDeviceObjects");
        const auto report_messages = info->GetNumStoredMessages();
        result.live_report_messages += report_messages;
        if (!report_messages) fail(result, "live-object report emitted no queryable diagnostics; cleanliness is unverified");
        for (UINT64 i = 0; i < info->GetNumStoredMessages(); ++i) {
            SIZE_T length = 0;
            check(info->GetMessage(i, nullptr, &length), "GetMessage size");
            std::vector<unsigned char> storage(length);
            auto* message = reinterpret_cast<D3D12_MESSAGE*>(storage.data());
            check(info->GetMessage(i, message, &length), "GetMessage");
            // Only the device and its debug/query interfaces are deliberately alive here.
            if (message->ID != D3D12_MESSAGE_ID_LIVE_DEVICE && message->ID != D3D12_MESSAGE_ID_LIVE_OBJECT_SUMMARY) {
                ++result.live_objects;
                fail(result, std::string("D3D12 teardown: ") + message->pDescription);
            }
        }
        result.teardown = result.live_objects || !report_messages || result.teardown == "fail" ? "fail" : "pass";
    }
};

ComPtr<ID3D12Fence> fence(ID3D12Device* device, bool shared = false) {
    ComPtr<ID3D12Fence> result;
    capability(device->CreateFence(0, shared ? D3D12_FENCE_FLAG_SHARED : D3D12_FENCE_FLAG_NONE,
                                  IID_PPV_ARGS(&result)), "CreateFence");
    return result;
}
Handle shared_handle(ID3D12Device* device, ID3D12DeviceChild* object) {
    HANDLE handle = nullptr;
    capability(device->CreateSharedHandle(object, nullptr, GENERIC_ALL, nullptr, &handle), "CreateSharedHandle");
    return Handle(handle);
}
Handle inheritable_copy(HANDLE original) {
    HANDLE copy = nullptr;
    win_check(DuplicateHandle(GetCurrentProcess(), original, GetCurrentProcess(), &copy, 0, TRUE, DUPLICATE_SAME_ACCESS),
              "DuplicateHandle");
    return Handle(copy);
}
struct Command {
    ComPtr<ID3D12CommandAllocator> allocator;
    ComPtr<ID3D12GraphicsCommandList> list;
    void create(ID3D12Device* device, D3D12_COMMAND_LIST_TYPE type) {
        check(device->CreateCommandAllocator(type, IID_PPV_ARGS(&allocator)), "CreateCommandAllocator");
        check(device->CreateCommandList(0, type, allocator.Get(), nullptr, IID_PPV_ARGS(&list)), "CreateCommandList");
        check(list->Close(), "Close new command list");
    }
    void reset() {
        check(allocator->Reset(), "Reset command allocator");
        check(list->Reset(allocator.Get(), nullptr), "Reset command list");
    }
};
struct Queue {
    ComPtr<ID3D12CommandQueue> queue;
    ComPtr<ID3D12Fence> drained;
    Handle completion = event();
    Result& result;
    const bool producer; // the producer is always the compute queue
    std::uint64_t drain_value = 0;
    bool busy = false;
    Queue(ID3D12Device* device, D3D12_COMMAND_LIST_TYPE type, Result& r)
        : result(r), producer(type == D3D12_COMMAND_LIST_TYPE_COMPUTE) {
        D3D12_COMMAND_QUEUE_DESC desc{}; desc.Type = type;
        check(device->CreateCommandQueue(&desc, IID_PPV_ARGS(&queue)), "CreateCommandQueue");
        drained = fence(device);
    }
    void execute(Command& command) {
        check(command.list->Close(), "Close command list");
        ID3D12CommandList* commands[] = {command.list.Get()};
        busy = true;
        queue->ExecuteCommandLists(1, commands);
    }
    void wait(ID3D12Fence* target, std::uint64_t value) {
        busy = true;
        check(queue->Wait(target, value), "command queue Wait");
    }
    void signal(ID3D12Fence* target, std::uint64_t value) {
        busy = true;
        check(queue->Signal(target, value), "command queue Signal");
    }
    // Returns or throws only when release is safe: after device removal all GPU
    // work has ended. Any other failure ends the process here, before unwinding.
    void drain() {
        if (!busy) return;
        try {
            if (inject_unconfirmed_drain == (producer ? "producer" : "consumer"))
                throw std::runtime_error("injected unconfirmed drain (test)");
            check(queue->Signal(drained.Get(), ++drain_value), "queue drain Signal");
            wait_fence(drained.Get(), drain_value, completion.get());
        } catch (const std::exception& error) {
            if (drained->GetCompletedValue() != UINT64_MAX) abandon(result, error.what());
            throw;
        }
        busy = false;
    }
    ~Queue() {
        try { drain(); } catch (const std::exception& error) { fail(result, error.what()); }
    }
};
D3D12_RESOURCE_DESC texture_desc(unsigned width, unsigned height) {
    D3D12_RESOURCE_DESC desc{};
    desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    desc.Width = width; desc.Height = height; desc.DepthOrArraySize = 1; desc.MipLevels = 1;
    desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM; desc.SampleDesc.Count = 1;
    desc.Layout = D3D12_TEXTURE_LAYOUT_UNKNOWN;
    desc.Flags = D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS;
    return desc;
}
ComPtr<ID3D12Resource> readback_buffer(ID3D12Device* device, UINT64 size) {
    D3D12_HEAP_PROPERTIES properties{};
    properties.Type = D3D12_HEAP_TYPE_READBACK;
    properties.CreationNodeMask = properties.VisibleNodeMask = 1;
    D3D12_RESOURCE_DESC desc{};
    desc.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER; desc.Width = size; desc.Height = 1;
    desc.DepthOrArraySize = 1; desc.MipLevels = 1; desc.SampleDesc.Count = 1;
    desc.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
    ComPtr<ID3D12Resource> result;
    check(device->CreateCommittedResource(&properties, D3D12_HEAP_FLAG_NONE, &desc,
        D3D12_RESOURCE_STATE_COPY_DEST, nullptr, IID_PPV_ARGS(&result)), "Create readback buffer");
    return result;
}
void transition(ID3D12GraphicsCommandList* list, ID3D12Resource* resource,
                D3D12_RESOURCE_STATES before, D3D12_RESOURCE_STATES after) {
    D3D12_RESOURCE_BARRIER barrier{};
    barrier.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    barrier.Transition = {resource, D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, before, after};
    list->ResourceBarrier(1, &barrier);
}
struct Slot {
    ComPtr<ID3D12Resource> texture, clear_source, readback;
    Command producer, consumer;
    std::uint64_t pending = 0;
    Clock::time_point started;
};
struct Ring {
    std::array<Slot, 3> slots;
    ComPtr<ID3D12Heap> heap;
    ComPtr<ID3D12DescriptorHeap> clear_cpu, clear_gpu;
    UINT clear_step = 0;
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT footprint{};
    UINT64 bytes = 0;
    unsigned width = 0, height = 0;
    void create(ID3D12Device* device, unsigned w, unsigned h, bool producer, bool consumer,
                bool shared_heap = false, ID3D12Heap* opened_heap = nullptr, bool shared_resources = false) {
        width = w; height = h;
        auto desc = texture_desc(w, h);
        // D3D11 OpenSharedResource1 needs render-target-capable committed shared
        // textures. The shared heap uses ALLOW_ONLY_NON_RT_DS_TEXTURES.
        if (shared_resources && !shared_heap && !opened_heap) desc.Flags |= D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
        device->GetCopyableFootprints(&desc, 0, 1, 0, &footprint, nullptr, nullptr, &bytes);
        const auto allocation = device->GetResourceAllocationInfo(0, 1, &desc);
        if (allocation.SizeInBytes == UINT64_MAX || !allocation.Alignment)
            throw std::runtime_error("invalid texture allocation info");
        const auto stride = (allocation.SizeInBytes + allocation.Alignment - 1) / allocation.Alignment * allocation.Alignment;
        D3D12_HEAP_PROPERTIES properties{};
        properties.Type = D3D12_HEAP_TYPE_DEFAULT;
        properties.CreationNodeMask = properties.VisibleNodeMask = 1;
        if (producer) {
            D3D12_DESCRIPTOR_HEAP_DESC hd{};
            hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
            hd.NumDescriptors = static_cast<UINT>(slots.size());
            check(device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&clear_cpu)), "Create CPU clear descriptors");
            hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
            check(device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&clear_gpu)), "Create GPU clear descriptors");
            clear_step = device->GetDescriptorHandleIncrementSize(hd.Type);
        }
        if (opened_heap) {
            heap = opened_heap;
            if (heap->GetDesc().SizeInBytes < stride * ring_size)
                throw std::runtime_error("opened shared heap is too small for the ring");
        } else if (shared_heap) {
            D3D12_HEAP_DESC hd{};
            hd.SizeInBytes = stride * ring_size; hd.Alignment = allocation.Alignment; hd.Properties = properties;
            hd.Flags = static_cast<D3D12_HEAP_FLAGS>(D3D12_HEAP_FLAG_SHARED | D3D12_HEAP_FLAG_ALLOW_ONLY_NON_RT_DS_TEXTURES);
            capability(device->CreateHeap(&hd, IID_PPV_ARGS(&heap)), "Create shared heap");
        }
        for (std::size_t i = 0; i < slots.size(); ++i) {
            auto& slot = slots[i];
            if (heap) {
                capability(device->CreatePlacedResource(heap.Get(), stride * i, &desc, D3D12_RESOURCE_STATE_COMMON,
                           nullptr, IID_PPV_ARGS(&slot.texture)), "CreatePlacedResource in shared heap");
            } else {
                capability(device->CreateCommittedResource(&properties, shared_resources ? D3D12_HEAP_FLAG_SHARED : D3D12_HEAP_FLAG_NONE,
                    &desc, D3D12_RESOURCE_STATE_COMMON, nullptr, IID_PPV_ARGS(&slot.texture)), "Create ring texture");
            }
            if (producer) {
                auto source_desc = desc;
                source_desc.Flags = D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS;
                check(device->CreateCommittedResource(&properties, D3D12_HEAP_FLAG_NONE, &source_desc,
                    D3D12_RESOURCE_STATE_COMMON, nullptr, IID_PPV_ARGS(&slot.clear_source)), "Create GPU clear source");
                D3D12_UNORDERED_ACCESS_VIEW_DESC uav{};
                uav.Format = desc.Format; uav.ViewDimension = D3D12_UAV_DIMENSION_TEXTURE2D;
                auto cpu = clear_cpu->GetCPUDescriptorHandleForHeapStart();
                cpu.ptr += i * clear_step;
                device->CreateUnorderedAccessView(slot.clear_source.Get(), nullptr, &uav, cpu);
                auto gpu_cpu = clear_gpu->GetCPUDescriptorHandleForHeapStart();
                gpu_cpu.ptr += i * clear_step;
                device->CopyDescriptorsSimple(1, gpu_cpu, cpu, D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
                slot.producer.create(device, D3D12_COMMAND_LIST_TYPE_COMPUTE);
            }
            if (consumer) {
                slot.readback = readback_buffer(device, bytes);
                slot.consumer.create(device, D3D12_COMMAND_LIST_TYPE_DIRECT);
            }
        }
    }
    void produce(Queue& queue, ID3D12Fence* ready, ID3D12Fence* free, std::uint64_t n) {
        const auto index = ring_slot(n);
        auto& slot = slots[index];
        slot.started = Clock::now();
        // CPU has also retired this slot, protecting its allocator and clear source.
        queue.wait(free, previous_use(n));
        slot.producer.reset();
        ID3D12DescriptorHeap* heaps[] = {clear_gpu.Get()};
        slot.producer.list->SetDescriptorHeaps(1, heaps);
        auto cpu = clear_cpu->GetCPUDescriptorHandleForHeapStart(); cpu.ptr += index * clear_step;
        auto gpu = clear_gpu->GetGPUDescriptorHandleForHeapStart(); gpu.ptr += index * clear_step;
        const auto rgba = encode_sequence(n);
        const float color[] = {rgba[0] / 255.0f, rgba[1] / 255.0f, rgba[2] / 255.0f, rgba[3] / 255.0f};
        transition(slot.producer.list.Get(), slot.clear_source.Get(), D3D12_RESOURCE_STATE_COMMON, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
        slot.producer.list->ClearUnorderedAccessViewFloat(gpu, cpu, slot.clear_source.Get(), color, 0, nullptr);
        // The transition orders the UAV clear before its copy; the scratch texture
        // is private to this producer and is never shared with a consumer.
        transition(slot.producer.list.Get(), slot.clear_source.Get(), D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_COPY_SOURCE);
        transition(slot.producer.list.Get(), slot.texture.Get(), D3D12_RESOURCE_STATE_COMMON, D3D12_RESOURCE_STATE_COPY_DEST);
        D3D12_TEXTURE_COPY_LOCATION src{}, dst{};
        src.pResource = slot.clear_source.Get(); src.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
        dst.pResource = slot.texture.Get(); dst.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
        slot.producer.list->CopyTextureRegion(&dst, 0, 0, 0, &src, nullptr);
        transition(slot.producer.list.Get(), slot.clear_source.Get(), D3D12_RESOURCE_STATE_COPY_SOURCE, D3D12_RESOURCE_STATE_COMMON);
        transition(slot.producer.list.Get(), slot.texture.Get(), D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_COMMON);
        queue.execute(slot.producer);
        queue.signal(ready, n);
        slot.pending = n;
    }
    void consume(Queue& queue, ID3D12Fence* ready, ID3D12Fence* copied, std::uint64_t n) {
        auto& slot = slots[ring_slot(n)];
        queue.wait(ready, n);
        slot.consumer.reset();
        transition(slot.consumer.list.Get(), slot.texture.Get(), D3D12_RESOURCE_STATE_COMMON, D3D12_RESOURCE_STATE_COPY_SOURCE);
        D3D12_TEXTURE_COPY_LOCATION src{}, dst{};
        src.pResource = slot.texture.Get(); src.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
        dst.pResource = slot.readback.Get(); dst.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT; dst.PlacedFootprint = footprint;
        slot.consumer.list->CopyTextureRegion(&dst, 0, 0, 0, &src, nullptr);
        transition(slot.consumer.list.Get(), slot.texture.Get(), D3D12_RESOURCE_STATE_COPY_SOURCE, D3D12_RESOURCE_STATE_COMMON);
        queue.execute(slot.consumer);
        queue.signal(copied, n);
    }
    void verify(std::uint64_t n) {
        auto& slot = slots[ring_slot(n)];
        void* mapped = nullptr;
        const D3D12_RANGE read{0, static_cast<SIZE_T>(bytes)};
        check(slot.readback->Map(0, &read, &mapped), "Map readback");
        std::string reason;
        const auto ok = verify_pattern(static_cast<unsigned char*>(mapped) + footprint.Offset,
                                      footprint.Footprint.RowPitch, width, height, n, reason);
        const D3D12_RANGE no_write{0, 0};
        slot.readback->Unmap(0, &no_write);
        if (!ok) throw std::runtime_error(reason);
    }
};

void same_device(Device& env, const Options& options, Result& result, std::uint64_t& current) {
    result.consumer_luid = result.producer_luid;
    result.consumer_singleton = result.singleton;
    auto ready = fence(env.device.Get()), free = fence(env.device.Get());
    auto completion = event();
    Ring ring;
    ring.create(env.device.Get(), 1024, 1024, true, true);
    // Queues die (and drain) before the ring, including on the exception path.
    Queue producer(env.device.Get(), D3D12_COMMAND_LIST_TYPE_COMPUTE, result);
    Queue consumer(env.device.Get(), D3D12_COMMAND_LIST_TYPE_DIRECT, result);
    auto retire = [&](std::uint64_t n) {
        if (!n || ring.slots[ring_slot(n)].pending != n) return;
        current = n;
        wait_fence(free.Get(), n, completion.get());
        ring.verify(n);
        ++result.verified;
        auto& slot = ring.slots[ring_slot(n)];
        result.timings.push_back(elapsed(slot.started));
        slot.pending = 0;
    };
    auto retire_remaining = [&](std::uint64_t last) {
        for (auto n = last > 2 ? last - 2 : 1; n <= last; ++n) retire(n);
    };
    for (std::uint64_t n = 1; n <= options.iterations; ++n) {
        if (result.mode == "resize" && n > 1 && (n - 1) % 100 == 0) {
            retire_remaining(n - 1);
            producer.drain(); consumer.drain();
            current = n;
            ring = Ring{};
            const unsigned phase = (++result.resize_count) % 3;
            const unsigned widths[] = {1024, 640, 1280}, heights[] = {1024, 480, 720};
            ring.create(env.device.Get(), widths[phase], heights[phase], true, true);
        }
        retire(previous_use(n));
        current = n;
        ring.produce(producer, ready.Get(), free.Get(), n);
        ++result.submitted;
        ring.consume(consumer, ready.Get(), free.Get(), n);
    }
    // The last up-to-three frames are still outstanding when teardown begins.
    retire_remaining(options.iterations);
    producer.drain(); consumer.drain();
}

// Same executable/toolchain on both sides: this small binary report is private IPC,
// never persisted. The public result is the versioned JSON written by the parent.
struct ChildReport {
    std::uint32_t magic = 0x53424831;
    std::uint32_t phase = 0; // 1 = setup ready, 2 = final (including setup failure)
    std::uint32_t status = 0; // 0 = pass, 1 = fail, 2 = unsupported
    std::uint32_t singleton = 2; // 0/1 or 2 = not checked
    std::uint64_t luid = 0, verified = 0, first_failure = 0, report_messages = 0;
    std::uint32_t live_objects = 0;
    std::uint32_t teardown_failed = 0;
    char reason[768]{};
};
ChildReport child_report(const Result& result, unsigned phase) {
    ChildReport report;
    report.phase = phase;
    report.status = result.status == "pass" ? 0 : result.status == "unsupported" ? 2 : 1;
    report.singleton = result.singleton ? (*result.singleton ? 1 : 0) : 2;
    report.verified = result.verified; report.first_failure = result.first_failure.value_or(0);
    report.live_objects = result.live_objects;
    report.report_messages = result.live_report_messages;
    report.teardown_failed = result.teardown == "fail" ? 1 : 0;
    const auto length = std::min(result.reason.size(), sizeof report.reason - 1);
    std::memcpy(report.reason, result.reason.data(), length);
    return report;
}
void write_report(HANDLE pipe, const ChildReport& report) {
    DWORD written = 0;
    win_check(WriteFile(pipe, &report, sizeof report, &written, nullptr), "Write child report");
    if (written != sizeof report) throw std::runtime_error("short child report write");
}
struct ChildProcess {
    Handle process, pipe, cancel = event(true);
    Result& result;
    bool final_read = false, stopped = false;
    explicit ChildProcess(Result& r) : result(r) {}
    void apply(const ChildReport& report) {
        result.consumer_luid = luid_string(make_luid(report.luid));
        if (report.singleton != 2) result.consumer_singleton = report.singleton == 1;
        if (report.phase == 2) {
            final_read = true;
            result.verified = report.verified;
            failing_iteration(result, report.first_failure);
            result.live_objects += report.live_objects;
            result.live_report_messages += report.report_messages;
            if (report.teardown_failed) result.teardown = "fail";
            if (report.status == 1) fail(result, std::string("child: ") + report.reason);
            if (report.status == 2) {
                if (result.submitted) fail(result, std::string("child reported unsupported after submission: ") + report.reason);
                else { result.status = "unsupported"; result.reason = report.reason; }
            }
        }
    }
    ChildReport read() {
        const auto start = Clock::now();
        for (;;) {
            DWORD available = 0;
            win_check(PeekNamedPipe(pipe.get(), nullptr, 0, nullptr, &available, nullptr), "Peek child report");
            if (available >= sizeof(ChildReport)) break;
            if (WaitForSingleObject(process.get(), 5) == WAIT_OBJECT_0) {
                // The child may have written its final report during that wait.
                win_check(PeekNamedPipe(pipe.get(), nullptr, 0, nullptr, &available, nullptr), "Peek final child report");
                if (available >= sizeof(ChildReport)) break;
                throw std::runtime_error("child exited without a complete report");
            }
            if (elapsed(start) >= wait_ms) throw std::runtime_error("child report timed out after 30 s");
        }
        ChildReport report;
        DWORD read_bytes = 0;
        win_check(ReadFile(pipe.get(), &report, sizeof report, &read_bytes, nullptr), "Read child report");
        if (read_bytes != sizeof report || report.magic != 0x53424831 ||
            (report.phase != 1 && report.phase != 2) || report.status > 2 || report.singleton > 2 || report.teardown_failed > 1 ||
            report.reason[sizeof report.reason - 1] != 0)
            throw std::runtime_error("invalid child report");
        apply(report);
        return report;
    }
    void start(Device& env, Ring& ring, ID3D12Fence* ready, ID3D12Fence* free, const Options& options) {
        auto heap_export = shared_handle(env.device.Get(), ring.heap.Get());
        auto ready_export = shared_handle(env.device.Get(), ready);
        auto free_export = shared_handle(env.device.Get(), free);
        HANDLE read_end = nullptr, write_end = nullptr;
        win_check(CreatePipe(&read_end, &write_end, nullptr, 4096), "Create report pipe");
        pipe.reset(read_end);
        Handle writer(write_end);
        // DuplicateHandle makes only these five copies inheritable; the handle list
        // prevents accidental inheritance of unrelated parent handles.
        std::array<Handle, 5> inherited{inheritable_copy(heap_export.get()), inheritable_copy(ready_export.get()),
            inheritable_copy(free_export.get()), inheritable_copy(writer.get()), inheritable_copy(cancel.get())};
        std::array<HANDLE, 5> handle_list{};
        std::wstring handle_args;
        for (std::size_t i = 0; i < inherited.size(); ++i) {
            handle_list[i] = inherited[i].get();
            if (i) handle_args += L',';
            handle_args += std::to_wstring(reinterpret_cast<std::uintptr_t>(handle_list[i]));
        }
        std::wstring executable(32768, L'\0');
        const auto length = GetModuleFileNameW(nullptr, executable.data(), static_cast<DWORD>(executable.size()));
        win_check(length && length < executable.size(), "GetModuleFileName");
        executable.resize(length);
        std::wstring command = L"\"" + executable + L"\" --child " + handle_args + L" --luid " +
            std::to_wstring(luid_bits(env.device->GetAdapterLuid())) + L" --iterations " + std::to_wstring(options.iterations) +
            L" --width " + std::to_wstring(ring.width) + L" --height " + std::to_wstring(ring.height);
        if (options.debug) command += L" --debug-layer";
        if (options.inject_unconfirmed_drain == "consumer") command += L" --inject-unconfirmed-drain consumer";
        SIZE_T bytes = 0;
        InitializeProcThreadAttributeList(nullptr, 1, 0, &bytes);
        std::vector<unsigned char> storage(bytes);
        STARTUPINFOEXW startup{};
        startup.StartupInfo.cb = sizeof startup;
        startup.lpAttributeList = reinterpret_cast<LPPROC_THREAD_ATTRIBUTE_LIST>(storage.data());
        win_check(InitializeProcThreadAttributeList(startup.lpAttributeList, 1, 0, &bytes), "Initialize process attributes");
        const auto updated = UpdateProcThreadAttribute(startup.lpAttributeList, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            handle_list.data(), sizeof handle_list, nullptr, nullptr);
        if (!updated) {
            const auto error = GetLastError();
            DeleteProcThreadAttributeList(startup.lpAttributeList);
            check(HRESULT_FROM_WIN32(error), "Set inherited handle list");
        }
        PROCESS_INFORMATION child{};
        const auto created = CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, TRUE,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_NO_WINDOW, nullptr, nullptr, &startup.StartupInfo, &child);
        const auto creation_error = GetLastError();
        DeleteProcThreadAttributeList(startup.lpAttributeList);
        if (!created) check(HRESULT_FROM_WIN32(creation_error), "Create consumer child");
        process.reset(child.hProcess); Handle thread(child.hThread);
        abandon_cancel = cancel.get();
        // Close all parent writer copies before reading so an exited child is observable.
        writer.reset();
        for (auto& handle : inherited) handle.reset();
        const auto report = read();
        if (report.phase != 1 || report.status != 0) {
            if (report.status == 2) throw Unsupported(report.reason);
            throw std::runtime_error(std::string("child setup failed: ") + report.reason);
        }
        if (report.luid != luid_bits(env.device->GetAdapterLuid()))
            throw std::runtime_error("consumer child opened a different adapter");
    }
    void finish() {
        if (!final_read) {
            const auto report = read();
            if (report.phase != 2) throw std::runtime_error("expected final child report");
        }
        const auto wait = WaitForSingleObject(process.get(), wait_ms);
        if (wait != WAIT_OBJECT_0) throw std::runtime_error("consumer child did not exit after its final report");
        DWORD exit_code = 0;
        win_check(GetExitCodeProcess(process.get(), &exit_code), "Get child exit code");
        if (exit_code && result.status != "unsupported" && result.errors.empty()) fail(result, "consumer child returned a failure exit code");
        stopped = true;
    }
    ~ChildProcess() {
        if (abandon_cancel == cancel.get()) abandon_cancel = nullptr;
        if (!process.get() || stopped) return;
        SetEvent(cancel.get());
        try { finish(); }
        catch (const std::exception& error) {
            fail(result, error.what());
            if (WaitForSingleObject(process.get(), 0) != WAIT_OBJECT_0) {
                TerminateProcess(process.get(), 2);
                WaitForSingleObject(process.get(), 5000);
                fail(result, "consumer child required termination after cancellation");
            }
        }
    }
};

void second_device(Device& env, const Options& options, Result& result, std::uint64_t& current) {
    auto ready = fence(env.device.Get(), true), free = fence(env.device.Get(), true);
    auto completion = event();
    Ring ring;
    ring.create(env.device.Get(), 1024, 1024, true, false, true);
    ChildProcess child(result);
    child.start(env, ring, ready.Get(), free.Get(), options);
    Queue producer(env.device.Get(), D3D12_COMMAND_LIST_TYPE_COMPUTE, result);
    auto retire = [&](std::uint64_t n) {
        if (!n) return;
        current = n;
        wait_fence(free.Get(), n, completion.get(), child.process.get());
        auto& slot = ring.slots[ring_slot(n)];
        result.timings.push_back(elapsed(slot.started));
        slot.pending = 0;
        ++result.verified; // Child signals free only after successful CPU verification.
    };
    for (std::uint64_t n = 1; n <= options.iterations; ++n) {
        retire(previous_use(n));
        current = n;
        ring.produce(producer, ready.Get(), free.Get(), n);
        ++result.submitted;
    }
    for (auto n = options.iterations > 2 ? options.iterations - 2 : 1; n <= options.iterations; ++n) retire(n);
    producer.drain();
    child.finish();
}

struct D11Consumer {
    ComPtr<ID3D11Device5> device;
    ComPtr<ID3D11DeviceContext4> context;
    ComPtr<ID3D11Fence> ready, free, drained;
    std::array<ComPtr<ID3D11Texture2D>, 3> textures, staging;
    Handle completion = event();
    Result& result;
    std::uint64_t drain_value = 0;
    bool busy = false;
    D11Consumer(Device& env, Ring& ring, ID3D12Fence* ready12, ID3D12Fence* free12, const Options& options, Result& r) : result(r) {
        ComPtr<ID3D11Device> base;
        ComPtr<ID3D11DeviceContext> immediate;
        const D3D_FEATURE_LEVEL levels[] = {D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0};
        capability(D3D11CreateDevice(options.warp ? nullptr : env.adapter.Get(),
            options.warp ? D3D_DRIVER_TYPE_WARP : D3D_DRIVER_TYPE_UNKNOWN, nullptr,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT | (options.debug ? D3D11_CREATE_DEVICE_DEBUG : 0), levels, 2,
            D3D11_SDK_VERSION, &base, nullptr, &immediate), "D3D11CreateDevice");
        capability(base.As(&device), "ID3D11Device5 (shared fence support)");
        capability(immediate.As(&context), "ID3D11DeviceContext4 (Wait/Signal support)");
        ComPtr<IDXGIDevice> dxgi;
        check(base.As(&dxgi), "D3D11 IDXGIDevice");
        ComPtr<IDXGIAdapter> adapter;
        check(dxgi->GetAdapter(&adapter), "D3D11 GetAdapter");
        DXGI_ADAPTER_DESC desc{};
        check(adapter->GetDesc(&desc), "D3D11 GetDesc");
        result.consumer_luid = luid_string(desc.AdapterLuid);
        if (luid_bits(desc.AdapterLuid) != luid_bits(env.device->GetAdapterLuid()))
            throw std::runtime_error("D3D11 consumer uses a different adapter");
        auto ready_handle = shared_handle(env.device.Get(), ready12), free_handle = shared_handle(env.device.Get(), free12);
        capability(device->OpenSharedFence(ready_handle.get(), IID_PPV_ARGS(&ready)), "D3D11 OpenSharedFence ready");
        capability(device->OpenSharedFence(free_handle.get(), IID_PPV_ARGS(&free)), "D3D11 OpenSharedFence free");
        capability(device->CreateFence(0, D3D11_FENCE_FLAG_NONE, IID_PPV_ARGS(&drained)), "D3D11 Create drain fence");
        for (std::size_t i = 0; i < textures.size(); ++i) {
            auto handle = shared_handle(env.device.Get(), ring.slots[i].texture.Get());
            capability(device->OpenSharedResource1(handle.get(), IID_PPV_ARGS(&textures[i])), "D3D11 OpenSharedResource1");
            D3D11_TEXTURE2D_DESC td{};
            textures[i]->GetDesc(&td);
            if (td.Width != ring.width || td.Height != ring.height || td.Format != DXGI_FORMAT_R8G8B8A8_UNORM)
                throw std::runtime_error("D3D11 opened texture has an unexpected descriptor");
            td.Usage = D3D11_USAGE_STAGING; td.BindFlags = 0; td.CPUAccessFlags = D3D11_CPU_ACCESS_READ; td.MiscFlags = 0;
            check(device->CreateTexture2D(&td, nullptr, &staging[i]), "D3D11 Create staging texture");
        }
    }
    void consume(std::uint64_t n) {
        busy = true;
        check(context->Wait(ready.Get(), n), "D3D11 context Wait ready");
        context->CopyResource(staging[ring_slot(n)].Get(), textures[ring_slot(n)].Get());
        check(context->Signal(free.Get(), n), "D3D11 context Signal free");
        context->Flush();
    }
    void verify(Ring& ring, std::uint64_t n) {
        D3D11_MAPPED_SUBRESOURCE mapped{};
        auto* resource = staging[ring_slot(n)].Get();
        check(context->Map(resource, 0, D3D11_MAP_READ, 0, &mapped), "D3D11 Map staging texture");
        std::string reason;
        const auto ok = verify_pattern(mapped.pData, mapped.RowPitch, ring.width, ring.height, n, reason);
        context->Unmap(resource, 0);
        if (!ok) throw std::runtime_error(reason);
    }
    // As Queue::drain: an unconfirmed drain without device removal ends the process.
    void drain() {
        if (!busy) return;
        try {
            if (inject_unconfirmed_drain == "consumer") throw std::runtime_error("injected unconfirmed drain (test)");
            check(context->Signal(drained.Get(), ++drain_value), "D3D11 drain Signal");
            context->Flush();
            wait_fence(drained.Get(), drain_value, completion.get());
        } catch (const std::exception& error) {
            if (device->GetDeviceRemovedReason() == S_OK) abandon(result, error.what());
            throw;
        }
        context->ClearState();
        context->Flush();
        busy = false;
    }
    ~D11Consumer() {
        try { drain(); } catch (const std::exception& error) { fail(result, error.what()); }
    }
};
void d3d11_consumer(Device& env, const Options& options, Result& result, std::uint64_t& current) {
    auto ready = fence(env.device.Get(), true), free = fence(env.device.Get(), true);
    auto completion = event();
    Ring ring;
    // D3D11 opens committed resource handles; only second-device uses a shared heap handle.
    ring.create(env.device.Get(), 1024, 1024, true, false, false, nullptr, true);
    D11Consumer consumer(env, ring, ready.Get(), free.Get(), options, result);
    Queue producer(env.device.Get(), D3D12_COMMAND_LIST_TYPE_COMPUTE, result);
    auto retire = [&](std::uint64_t n) {
        if (!n) return;
        current = n;
        wait_fence(free.Get(), n, completion.get());
        consumer.verify(ring, n);
        ++result.verified;
        auto& slot = ring.slots[ring_slot(n)];
        result.timings.push_back(elapsed(slot.started));
        slot.pending = 0;
    };
    for (std::uint64_t n = 1; n <= options.iterations; ++n) {
        retire(previous_use(n));
        current = n;
        ring.produce(producer, ready.Get(), free.Get(), n);
        ++result.submitted;
        consumer.consume(n);
    }
    for (auto n = options.iterations > 2 ? options.iterations - 2 : 1; n <= options.iterations; ++n) retire(n);
    producer.drain(); consumer.drain();
}
}

void set_abandon_report(std::function<void(const Result&)> report) { abandon_report = std::move(report); }

Result run_mode(const Options& options, const std::string& mode) {
    inject_unconfirmed_drain = options.inject_unconfirmed_drain;
    Result result;
    result.mode = mode; result.iterations = options.iterations;
    if (options.debug) result.teardown = "not_run";
    std::unique_ptr<Device> env;
    std::uint64_t current = 0;
    try {
        env = std::make_unique<Device>(options, result);
        result.status = "pass";
        if (mode == "same-device" || mode == "resize") same_device(*env, options, result, current);
        else if (mode == "second-device") second_device(*env, options, result, current);
        else if (mode == "d3d11-consumer") d3d11_consumer(*env, options, result, current);
        else throw std::invalid_argument("unimplemented mode");
        if (result.status == "pass" && result.verified != result.iterations) fail(result, "not every submitted iteration was verified");
    } catch (const Unsupported& error) {
        if (result.submitted || !result.errors.empty()) {
            fail(result, error.what()); failing_iteration(result, current);
        } else { result.status = "unsupported"; result.reason = error.what(); }
    } catch (const std::exception& error) {
        fail(result, error.what()); failing_iteration(result, current);
    }
    if (env && options.debug) {
        try { env->report(result); }
        catch (const std::exception& error) { result.teardown = "fail"; fail(result, error.what()); }
    }
    if (result.status == "pass") result.reason = "every handed-over frame verified";
    return result;
}

int run_child(const Options& options) {
    // Adopt only the handles explicitly listed by our parent; no files are written.
    inject_unconfirmed_drain = options.inject_unconfirmed_drain;
    std::array<Handle, 5> handles;
    for (std::size_t i = 0; i < handles.size(); ++i) handles[i].reset(reinterpret_cast<HANDLE>((*options.handles)[i]));
    Result result;
    result.mode = "second-device-child"; result.iterations = options.iterations;
    if (options.debug) result.teardown = "not_run";
    std::unique_ptr<Device> env;
    std::uint64_t current = 0;
    abandon_report = [&](const Result& failed) {
        auto report = child_report(failed, 2);
        report.luid = env ? luid_bits(env->device->GetAdapterLuid()) : options.luid.value_or(0);
        write_report(handles[3].get(), report);
    };
    try {
        env = std::make_unique<Device>(options, result);
        ComPtr<ID3D12Heap> heap;
        capability(env->device->OpenSharedHandle(handles[0].get(), IID_PPV_ARGS(&heap)), "child OpenSharedHandle heap");
        ComPtr<ID3D12Fence> ready, free;
        capability(env->device->OpenSharedHandle(handles[1].get(), IID_PPV_ARGS(&ready)), "child OpenSharedHandle ready");
        capability(env->device->OpenSharedHandle(handles[2].get(), IID_PPV_ARGS(&free)), "child OpenSharedHandle free");
        handles[0].reset(); handles[1].reset(); handles[2].reset();
        auto copied = fence(env->device.Get());
        auto completion = event();
        Ring ring;
        ring.create(env->device.Get(), options.width, options.height, false, true, false, heap.Get());
        Queue consumer(env->device.Get(), D3D12_COMMAND_LIST_TYPE_DIRECT, result);
        result.status = "pass";
        auto setup = child_report(result, 1);
        setup.luid = luid_bits(env->device->GetAdapterLuid());
        write_report(handles[3].get(), setup);
        for (std::uint64_t n = 1; n <= options.iterations; ++n) {
            current = n;
            // Never enqueue a GPU wait on a value the parent has not signalled.
            // This permits bounded cancellation and draining after a parent failure.
            wait_fence(ready.Get(), n, completion.get(), nullptr, handles[4].get());
            ring.consume(consumer, ready.Get(), copied.Get(), n);
            ++result.submitted;
            wait_fence(copied.Get(), n, completion.get(), nullptr, handles[4].get());
            ring.verify(n);
            ++result.verified;
            consumer.signal(free.Get(), n);
        }
        consumer.drain();
    } catch (const Unsupported& error) {
        if (result.submitted || !result.errors.empty()) { fail(result, error.what()); failing_iteration(result, current); }
        else { result.status = "unsupported"; result.reason = error.what(); }
    } catch (const std::exception& error) {
        fail(result, error.what()); failing_iteration(result, current);
    }
    if (env && options.debug) {
        try { env->report(result); }
        catch (const std::exception& error) { result.teardown = "fail"; fail(result, error.what()); }
    }
    auto report = child_report(result, 2);
    report.luid = env ? luid_bits(env->device->GetAdapterLuid()) : options.luid.value_or(0);
    try { write_report(handles[3].get(), report); }
    catch (const std::exception&) { return 2; }
    return result.status == "pass" || result.status == "unsupported" ? 0 : 1;
}
}
