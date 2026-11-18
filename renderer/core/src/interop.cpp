#include "pure.h"
#include "scene.h"
#include "frame_target.h"
#include <d3d12sdklayers.h>
#include <dxgi1_6.h>
#include <wrl/client.h>
#include <algorithm>
#include <atomic>
#include <chrono>
#include <cstring>
#include <iomanip>
#include <memory>
#include <mutex>
#include <sstream>
#include <stdexcept>
#include <unordered_map>
#include <utility>

using Microsoft::WRL::ComPtr;
using Clock = std::chrono::steady_clock;

namespace {
using sa2::Failure;
std::string global_error;
std::string hr_text(const char* call, HRESULT hr) {
    std::ostringstream text;
    text << call << " failed: HRESULT 0x" << std::hex << std::setw(8) << std::setfill('0') << static_cast<unsigned long>(hr);
    return text.str();
}
void require(bool condition, const char* text, int32_t status = SA2_E_INVALID_ARGUMENT) {
    if (!condition) throw Failure(status, text);
}
void check_hr(ID3D12Device* device, HRESULT hr, const char* call) {
    if (SUCCEEDED(hr)) return;
    const auto removed = device ? device->GetDeviceRemovedReason() : S_OK;
    throw Failure(FAILED(removed) ? SA2_E_DEVICE_REMOVED : SA2_E_D3D12, hr_text(call, hr));
}
template<class T> ComPtr<T> interface_handle(uint64_t handle, ID3D12Device* device = nullptr) {
    require(handle != 0, "zero handle");
    ComPtr<T> object;
    const auto hr = reinterpret_cast<IUnknown*>(static_cast<uintptr_t>(handle))->QueryInterface(IID_PPV_ARGS(&object));
    if (FAILED(hr)) {
        if (device && FAILED(device->GetDeviceRemovedReason())) check_hr(device, hr, "QueryInterface");
        throw Failure(SA2_E_INVALID_ARGUMENT, hr_text("QueryInterface", hr));
    }
    return object;
}
bool same_identity(IUnknown* a, IUnknown* b, ID3D12Device* device) {
    ComPtr<IUnknown> first, second;
    check_hr(device, a->QueryInterface(IID_PPV_ARGS(&first)), "QueryInterface(IUnknown)");
    check_hr(device, b->QueryInterface(IID_PPV_ARGS(&second)), "QueryInterface(IUnknown)");
    return first.Get() == second.Get();
}
bool enhanced_supported(ID3D12Device* device) {
    D3D12_FEATURE_DATA_D3D12_OPTIONS12 options{};
    const auto hr = device->CheckFeatureSupport(D3D12_FEATURE_D3D12_OPTIONS12, &options, sizeof(options));
    if (FAILED(hr)) {
        if (FAILED(device->GetDeviceRemovedReason()) || hr != E_INVALIDARG) check_hr(device, hr, "CheckFeatureSupport(OPTIONS12)");
        return false;
    }
    return options.EnhancedBarriersSupported != FALSE;
}
sa2::State state(int32_t value) {
    sa2::State result{};
    require(sa2::state(value, result), "invalid SA2_STATE value");
    return result;
}
void copy_text(const std::string& text, char* buffer, uint32_t size) {
    require(buffer && size, "buffer and buffer_size must be nonzero");
    const auto count = std::min<std::size_t>(text.size(), size - 1);
    std::memcpy(buffer, text.data(), count);
    buffer[count] = 0;
}
struct Slot {
    ComPtr<ID3D12CommandAllocator> allocator;
    ComPtr<ID3D12GraphicsCommandList> list;
    ComPtr<ID3D12GraphicsCommandList7> list7;
    ComPtr<ID3D12Resource> external_reference, readback;
    ID3D12Resource* resource = nullptr;
    ID3D12Fence* pending_fence = nullptr;
    uint64_t pending_value = 0, last_shown = 0;
    uint32_t width = 0, height = 0;
};
struct Debug {
    std::atomic<uint64_t> corruption{}, error{}, warning{}, info{}, message{}, mismatch{}, mentioning{};
    std::mutex mutex;
    std::vector<int32_t> ids;
    std::vector<std::string> descriptions;
};
}

struct sa2_context {
    sa2_config config{};
    ComPtr<ID3D12Device> device;
    ComPtr<ID3D12Device10> device10;
    ComPtr<ID3D12CommandQueue> godot_queue, own_queue;
    ComPtr<ID3D12Fence> ready, free, godot_drain, own_drain;
    ComPtr<ID3D12DescriptorHeap> rtv_heap;
    ComPtr<ID3D12InfoQueue1> info_queue;
    std::array<Slot, SA2_RING_SLOTS> slots;
    std::unordered_map<ID3D12Resource*, ComPtr<ID3D12Resource>> textures;
    std::unique_ptr<sa2::Scene> scene;
    Debug debug;
    std::string last_error;
    HANDLE completion = nullptr;
    UINT rtv_step = 0;
    DWORD callback_cookie = 0;
    bool callback_registered = false, drained = false;
    uint64_t godot_drain_value = 0, own_drain_value = 0, last_frame = 0, last_ready = 0, last_free = 0;
    ~sa2_context() { if (completion) CloseHandle(completion); }
    ID3D12CommandQueue* producer() const { return own_queue ? own_queue.Get() : godot_queue.Get(); }
};

namespace {
template<class F> int32_t invoke(sa2_context* ctx, const char* call, F&& body) noexcept {
    try { return body(); }
    catch (const Failure& error) {
        (ctx ? ctx->last_error : global_error) = std::string(call) + ": " + error.what();
        return error.status;
    } catch (const std::exception& error) {
        (ctx ? ctx->last_error : global_error) = std::string(call) + ": " + error.what();
        return SA2_E_D3D12;
    } catch (...) {
        (ctx ? ctx->last_error : global_error) = std::string(call) + ": unexpected native failure";
        return SA2_E_D3D12;
    }
}
void context(sa2_context* ctx) { require(ctx != nullptr, "null context"); }
bool removed(sa2_context* ctx) { return FAILED(ctx->device->GetDeviceRemovedReason()); }
void live(sa2_context* ctx) {
    context(ctx);
    const auto hr = ctx->device->GetDeviceRemovedReason();
    if (FAILED(hr)) throw Failure(SA2_E_DEVICE_REMOVED, hr_text("GetDeviceRemovedReason", hr));
}
Slot& slot_at(sa2_context* ctx, uint32_t index, bool registered = true) {
    context(ctx);
    require(index < SA2_RING_SLOTS, "slot out of range");
    auto& slot = ctx->slots[index];
    if (registered) require(slot.resource != nullptr, "slot is not registered", SA2_E_WRONG_STATE);
    return slot;
}
template<class T> void name(sa2_context* ctx, T* object, const wchar_t* text) {
    check_hr(ctx->device.Get(), object->SetName(text), "SetName");
}
ComPtr<ID3D12Fence> fence(sa2_context* ctx, const wchar_t* text) {
    ComPtr<ID3D12Fence> result;
    check_hr(ctx->device.Get(), ctx->device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&result)), "CreateFence");
    name(ctx, result.Get(), text);
    return result;
}
void wait_fence(sa2_context* ctx, ID3D12Fence* target, uint64_t value, Clock::time_point deadline) {
    for (;;) {
        const auto completed = target->GetCompletedValue();
        if (completed == UINT64_MAX) {
            const auto hr = ctx->device->GetDeviceRemovedReason();
            throw Failure(SA2_E_DEVICE_REMOVED, hr_text("GetCompletedValue/GetDeviceRemovedReason", hr));
        }
        if (completed >= value) { live(ctx); return; }
        if (Clock::now() >= deadline) {
            live(ctx);
            throw Failure(SA2_E_TIMEOUT, "CPU fence wait timed out");
        }
        check_hr(ctx->device.Get(), target->SetEventOnCompletion(value, ctx->completion), "SetEventOnCompletion");
        const auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(deadline - Clock::now()).count();
        const auto timeout = static_cast<DWORD>(std::max<int64_t>(0, std::min<int64_t>(remaining, MAXDWORD - 1)));
        const auto status = WaitForSingleObject(ctx->completion, timeout);
        if (status == WAIT_FAILED) check_hr(ctx->device.Get(), HRESULT_FROM_WIN32(GetLastError()), "WaitForSingleObject");
        // Recheck the fence after both a wake and a timeout. A prior bounded
        // wait may have registered this same event for a different fence.
    }
}
Clock::time_point deadline(uint32_t timeout) { return Clock::now() + std::chrono::milliseconds(timeout); }
void wait_previous(sa2_context* ctx, Slot& slot) {
    if (slot.pending_fence) wait_fence(ctx, slot.pending_fence, slot.pending_value, deadline(ctx->config.wait_timeout_ms));
}
void signal(sa2_context* ctx, ID3D12CommandQueue* queue, ID3D12Fence* target, uint64_t value) {
    ctx->drained = false;
    check_hr(ctx->device.Get(), queue->Signal(target, value), "ID3D12CommandQueue::Signal");
}
void queue_wait(sa2_context* ctx, ID3D12CommandQueue* queue, ID3D12Fence* target, uint64_t value) {
    ctx->drained = false;
    check_hr(ctx->device.Get(), queue->Wait(target, value), "ID3D12CommandQueue::Wait");
}
void transition(sa2_context* ctx, Slot& slot, int32_t before, int32_t after, bool into_operation, bool copy = false) {
    if (before == after) return;
    const auto from = state(before), to = state(after);
    if (ctx->config.barrier_api == SA2_BARRIERS_LEGACY) {
        D3D12_RESOURCE_BARRIER barrier{};
        barrier.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
        barrier.Transition = {slot.resource, D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, from.legacy, to.legacy};
        slot.list->ResourceBarrier(1, &barrier);
    } else {
        const auto sync = copy ? D3D12_BARRIER_SYNC_COPY : D3D12_BARRIER_SYNC_RENDER_TARGET;
        const auto access = copy ? D3D12_BARRIER_ACCESS_COPY_SOURCE : D3D12_BARRIER_ACCESS_RENDER_TARGET;
        D3D12_TEXTURE_BARRIER barrier{};
        barrier.SyncBefore = into_operation ? D3D12_BARRIER_SYNC_NONE : sync;
        barrier.SyncAfter = into_operation ? sync : D3D12_BARRIER_SYNC_NONE;
        barrier.AccessBefore = into_operation ? D3D12_BARRIER_ACCESS_NO_ACCESS : access;
        barrier.AccessAfter = into_operation ? access : D3D12_BARRIER_ACCESS_NO_ACCESS;
        barrier.LayoutBefore = from.layout;
        barrier.LayoutAfter = to.layout;
        barrier.pResource = slot.resource;
        barrier.Subresources.IndexOrFirstMipLevel = 0;
        barrier.Subresources.NumMipLevels = 1;
        barrier.Subresources.NumArraySlices = 1;
        barrier.Subresources.NumPlanes = 1;
        D3D12_BARRIER_GROUP group{};
        group.Type = D3D12_BARRIER_TYPE_TEXTURE;
        group.NumBarriers = 1;
        group.pTextureBarriers = &barrier;
        slot.list7->Barrier(1, &group);
    }
}
void reset(sa2_context* ctx, Slot& slot) {
    wait_previous(ctx, slot);
    check_hr(ctx->device.Get(), slot.allocator->Reset(), "ID3D12CommandAllocator::Reset");
    check_hr(ctx->device.Get(), slot.list->Reset(slot.allocator.Get(), nullptr), "ID3D12GraphicsCommandList::Reset");
    slot.readback.Reset();
}
void execute(sa2_context* ctx, Slot& slot, ID3D12Fence* target, uint64_t value) {
    check_hr(ctx->device.Get(), slot.list->Close(), "ID3D12GraphicsCommandList::Close");
    ID3D12CommandList* lists[] = {slot.list.Get()};
    ctx->drained = false;
    slot.pending_fence = target;
    slot.pending_value = value;
    ctx->producer()->ExecuteCommandLists(1, lists);
    signal(ctx, ctx->producer(), target, value);
}
D3D12_CPU_DESCRIPTOR_HANDLE rtv(sa2_context* ctx, uint32_t index) {
    auto handle = ctx->rtv_heap->GetCPUDescriptorHandleForHeapStart();
    handle.ptr += std::size_t(index) * ctx->rtv_step;
    return handle;
}
std::array<float, 4> color(sa2::Pixel pixel) {
    return {pixel[0] / 255.0f, pixel[1] / 255.0f, pixel[2] / 255.0f, pixel[3] / 255.0f};
}
// The owner's NVIDIA driver ends the process with 0xC0000409 when one
// ClearRenderTargetView call carries more than 32 rectangles (33 fails at the
// first call, 32 runs; WARP accepts 100), so code blocks clear in batches.
constexpr std::size_t max_clear_rects = 32;
void clear_code(sa2_context* ctx, Slot& slot, uint32_t index, uint32_t sequence, uint32_t generation) {
    const auto fill = color(sa2::fill({sequence, index, generation}));
    slot.list->ClearRenderTargetView(rtv(ctx, index), fill.data(), 0, nullptr);
    std::array<std::vector<D3D12_RECT>, 2> rectangles;
    const auto code = sa2::code({sequence, index, generation});
    for (unsigned corner = 0; corner < 2; ++corner) {
        const LONG x0 = corner ? static_cast<LONG>(slot.width - 64) : 0;
        const LONG y0 = corner ? static_cast<LONG>(slot.height - 64) : 0;
        for (unsigned bit = 0; bit < 64; ++bit) {
            const LONG x = x0 + (bit % 8) * 8, y = y0 + (bit / 8) * 8;
            rectangles[(code >> bit) & 1].push_back({x, y, x + 8, y + 8});
        }
    }
    for (unsigned bit_value = 0; bit_value < 2; ++bit_value) {
        const auto k = static_cast<std::uint8_t>(bit_value ? 255 : 0);
        const auto rgba = color({k, k, k, 255});
        const auto& rects = rectangles[bit_value];
        for (std::size_t first = 0; first < rects.size(); first += max_clear_rects) {
            const auto count = std::min(max_clear_rects, rects.size() - first);
            slot.list->ClearRenderTargetView(rtv(ctx, index), rgba.data(), static_cast<UINT>(count), rects.data() + first);
        }
    }
}
void CALLBACK debug_callback(D3D12_MESSAGE_CATEGORY, D3D12_MESSAGE_SEVERITY severity, D3D12_MESSAGE_ID id,
                             LPCSTR description, void* user) noexcept {
    auto& debug = static_cast<sa2_context*>(user)->debug;
    switch (severity) {
    case D3D12_MESSAGE_SEVERITY_CORRUPTION: ++debug.corruption; break;
    case D3D12_MESSAGE_SEVERITY_ERROR: ++debug.error; break;
    case D3D12_MESSAGE_SEVERITY_WARNING: ++debug.warning; break;
    case D3D12_MESSAGE_SEVERITY_INFO: ++debug.info; break;
    case D3D12_MESSAGE_SEVERITY_MESSAGE: ++debug.message; break;
    }
    const bool mismatch = id == D3D12_MESSAGE_ID_CLEARRENDERTARGETVIEW_MISMATCHINGCLEARVALUE;
    if (mismatch) ++debug.mismatch;
    if (severity > D3D12_MESSAGE_SEVERITY_WARNING) return;
    if (description && std::strstr(description, "SA2")) ++debug.mentioning;
    try {
        std::lock_guard lock(debug.mutex);
        if (debug.ids.size() < 16 && std::find(debug.ids.begin(), debug.ids.end(), static_cast<int32_t>(id)) == debug.ids.end())
            debug.ids.push_back(static_cast<int32_t>(id));
        if (!mismatch && debug.descriptions.size() < 8) {
            auto text = sa2::mask_hex(description ? description : "");
            std::replace(text.begin(), text.end(), '\n', ' ');
            std::replace(text.begin(), text.end(), '\r', ' ');
            if (std::find(debug.descriptions.begin(), debug.descriptions.end(), text) == debug.descriptions.end())
                debug.descriptions.push_back(std::move(text));
        }
    } catch (...) {} // No exception can escape a runtime callback.
}
[[noreturn]] void abandon() noexcept {
    constexpr char line[] = "sa2: drain not confirmed; ending the process with exit code 3\n";
    DWORD written = 0;
    WriteFile(GetStdHandle(STD_ERROR_HANDLE), line, sizeof(line) - 1, &written, nullptr);
    TerminateProcess(GetCurrentProcess(), 3);
    ExitProcess(3);
}
void require_drained(sa2_context* ctx) {
    require(removed(ctx) || ctx->drained, "a confirmed drain is required", SA2_E_WRONG_STATE);
}

// ABI 4's frame target on the Godot-texture route: a registered ring slot
// that Godot composites. It keeps sa2_scene_produce's protocol: a bounded
// wait for the slot's previous frame, the fail-closed free check, the own
// queue's GPU wait and the configured handover states.
class SlotTarget final : public sa2::FrameTarget {
public:
    SlotTarget(sa2_context* ctx, uint32_t index) : ctx_(ctx), index_(index), slot_(slot_at(ctx, index)) {}
    uint32_t width() const override { return slot_.width; }
    uint32_t height() const override { return slot_.height; }
    uint32_t ring() const override { return index_; }
    void wait(uint64_t frame) override {
        if (frame) wait_fence(ctx_, ctx_->ready.Get(), frame, deadline(ctx_->config.wait_timeout_ms));
    }
    ID3D12GraphicsCommandList* open() override {
        if (ctx_->own_queue) require(slot_.last_shown <= ctx_->last_free, "slot's shown frame has not been signalled free", SA2_E_WRONG_STATE);
        reset(ctx_, slot_);
        if (ctx_->own_queue && slot_.last_shown) queue_wait(ctx_, ctx_->own_queue.Get(), ctx_->free.Get(), slot_.last_shown);
        transition(ctx_, slot_, ctx_->config.state_before_write, SA2_STATE_RENDER_TARGET, true);
        return slot_.list.Get();
    }
    D3D12_CPU_DESCRIPTOR_HANDLE view() const override { return rtv(ctx_, index_); }
    void close(uint64_t frame) override {
        transition(ctx_, slot_, SA2_STATE_RENDER_TARGET, ctx_->config.state_after_write, false);
        ctx_->last_frame = frame;
        execute(ctx_, slot_, ctx_->ready.Get(), frame);
        ctx_->last_ready = frame;
    }
private:
    sa2_context* ctx_;
    uint32_t index_;
    Slot& slot_;
};

// The loaded scene for a stateful product call, outside a trace; latches a device removal.
sa2::Scene& product_scene(sa2_context* ctx) {
    context(ctx);
    require(ctx->scene != nullptr, "no loaded scene", SA2_E_WRONG_STATE);
    auto& scene = *ctx->scene;
    require(!scene.record.tracing(), "product calls are not allowed inside a scene trace", SA2_E_WRONG_STATE);
    scene.product.observe_removal(ctx->device->GetDeviceRemovedReason());
    return scene;
}
int32_t removed_reason(sa2_context* ctx) {
    const int32_t hr = ctx->device->GetDeviceRemovedReason();
    if (!ctx->scene) return hr < 0 ? hr : 0;
    ctx->scene->product.observe_removal(hr);
    return ctx->scene->product.removed_reason();
}
uint64_t completed(ID3D12Fence* target) {
    const auto value = target->GetCompletedValue();
    return value == UINT64_MAX ? 0 : value; // UINT64_MAX reports device removal, never a completion
}
// Reads the query results of every ring slot whose last frame completed.
void harvest(sa2_context* ctx, sa2::Scene& scene) {
    if (scene.product.removed()) return;
    const auto done = completed(ctx->ready.Get());
    for (uint32_t ring = 0; ring < SA2_RING_SLOTS; ++ring) {
        const auto frame = scene.product.unharvested(ring);
        if (!frame || frame > done) continue;
        const auto [gpu_ns, evals] = scene.product_results(ring);
        scene.product.produced(frame, gpu_ns, evals);
    }
}
// Releases the GPU resources of finished tickets and the staging of completed
// uploads; after device loss, every set's and every pick's (sa2_pick_poll then
// answers SA2_E_DEVICE_REMOVED and never reads a pick).
void release(sa2_context* ctx, sa2::Scene& scene) {
    for (const auto ticket : scene.product.releasable(completed(ctx->ready.Get()), scene.product_completed()))
        scene.product_release_set(ticket);
    if (scene.product.removed()) scene.product_release_picks();
    scene.product_collect();
}
void write_json(const std::string& text, char* buffer, uint32_t buffer_size, uint32_t* out_size) {
    require(text.size() < UINT32_MAX, "JSON exceeds the size output", SA2_E_D3D12);
    *out_size = static_cast<uint32_t>(text.size() + 1);
    if (buffer_size < text.size() + 1) {
        if (buffer && buffer_size) buffer[0] = 0;
        throw Failure(SA2_E_INVALID_ARGUMENT, "buffer_size is smaller than *out_size");
    }
    std::memcpy(buffer, text.c_str(), text.size() + 1);
}
void require_text_output(char* buffer, uint32_t buffer_size, uint32_t* out_size) {
    require(out_size != nullptr && (buffer != nullptr || buffer_size == 0), "null size output, or a NULL buffer with a nonzero buffer_size");
}
}

extern "C" {
uint32_t sa2_abi_version(void) { return SA2_ABI_VERSION; }

int32_t sa2_last_error(sa2_context* ctx, char* buffer, uint32_t buffer_size) {
    return invoke(ctx, "sa2_last_error", [&] {
        copy_text(ctx ? ctx->last_error : global_error, buffer, buffer_size);
        return SA2_OK;
    });
}

int32_t sa2_probe(uint64_t device, uint64_t queue, uint64_t adapter, sa2_device_info* out) {
    return invoke(nullptr, "sa2_probe", [&] {
        require(out && out->struct_size == sizeof(*out), "invalid device_info struct_size");
        auto d = interface_handle<ID3D12Device>(device);
        auto q = interface_handle<ID3D12CommandQueue>(queue, d.Get());
        auto a = interface_handle<IDXGIAdapter>(adapter, d.Get());
        sa2_device_info result{};
        result.struct_size = sizeof(result);
        result.queue_type = q->GetDesc().Type;
        ComPtr<ID3D12Device> queue_device;
        check_hr(d.Get(), q->GetDevice(IID_PPV_ARGS(&queue_device)), "ID3D12CommandQueue::GetDevice");
        result.queue_device_matches = same_identity(d.Get(), queue_device.Get(), d.Get());
        DXGI_ADAPTER_DESC desc{};
        check_hr(d.Get(), a->GetDesc(&desc), "IDXGIAdapter::GetDesc");
        const auto luid = d->GetAdapterLuid();
        result.adapter_matches_device = luid.HighPart == desc.AdapterLuid.HighPart && luid.LowPart == desc.AdapterLuid.LowPart;
        result.vendor_id = desc.VendorId;
        result.device_id = desc.DeviceId;
        result.node_count = d->GetNodeCount();
        result.enhanced_barriers = enhanced_supported(d.Get());
        ComPtr<ID3D12InfoQueue1> info;
        const auto info_hr = d.As(&info);
        if (FAILED(info_hr) && (info_hr != E_NOINTERFACE || FAILED(d->GetDeviceRemovedReason()))) check_hr(d.Get(), info_hr, "QueryInterface(ID3D12InfoQueue1)");
        result.debug_layer = info != nullptr;
        const D3D_FEATURE_LEVEL levels[] = {D3D_FEATURE_LEVEL_12_2, D3D_FEATURE_LEVEL_12_1, D3D_FEATURE_LEVEL_12_0, D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0};
        D3D12_FEATURE_DATA_FEATURE_LEVELS features{static_cast<UINT>(std::size(levels)), levels, D3D_FEATURE_LEVEL_11_0};
        check_hr(d.Get(), d->CheckFeatureSupport(D3D12_FEATURE_FEATURE_LEVELS, &features, sizeof(features)), "CheckFeatureSupport(FEATURE_LEVELS)");
        result.max_feature_level = features.MaxSupportedFeatureLevel;
        LARGE_INTEGER version{};
        const auto version_hr = a->CheckInterfaceSupport(__uuidof(IDXGIDevice), &version);
        if (SUCCEEDED(version_hr)) result.umd_version = static_cast<uint64_t>(version.QuadPart);
        else if (FAILED(d->GetDeviceRemovedReason())) check_hr(d.Get(), version_hr, "CheckInterfaceSupport(IDXGIDevice)");
        *out = result;
        return SA2_OK;
    });
}

int32_t sa2_attach(uint64_t device, uint64_t queue, const sa2_config* config, sa2_context** out) {
    if (out) *out = nullptr;
    return invoke(nullptr, "sa2_attach", [&] {
        require(out && config && config->struct_size == sizeof(*config), "invalid config struct_size or output");
        require(config->queue_mode == SA2_QUEUE_SAME || config->queue_mode == SA2_QUEUE_OWN, "invalid queue_mode");
        require(config->barrier_api >= SA2_BARRIERS_MATCH_GODOT && config->barrier_api <= SA2_BARRIERS_ENHANCED, "invalid barrier_api");
        state(config->state_before_write); state(config->state_after_write);
        require(config->debug_callback == 0 || config->debug_callback == 1, "invalid debug_callback");
        auto ctx = std::make_unique<sa2_context>();
        ctx->device = interface_handle<ID3D12Device>(device);
        ctx->godot_queue = interface_handle<ID3D12CommandQueue>(queue, ctx->device.Get());
        require(ctx->godot_queue->GetDesc().Type == D3D12_COMMAND_LIST_TYPE_DIRECT, "queue must be DIRECT");
        ComPtr<ID3D12Device> queue_device;
        check_hr(ctx->device.Get(), ctx->godot_queue->GetDevice(IID_PPV_ARGS(&queue_device)), "ID3D12CommandQueue::GetDevice");
        require(same_identity(ctx->device.Get(), queue_device.Get(), ctx->device.Get()), "queue device differs from given device");
        ctx->config = *config;
        const bool enhanced = enhanced_supported(ctx->device.Get());
        if (config->barrier_api == SA2_BARRIERS_MATCH_GODOT) ctx->config.barrier_api = enhanced ? SA2_BARRIERS_ENHANCED : SA2_BARRIERS_LEGACY;
        if (ctx->config.barrier_api == SA2_BARRIERS_ENHANCED) {
            require(enhanced, "enhanced barriers are not supported", SA2_E_UNSUPPORTED);
            const auto hr = ctx->device.As(&ctx->device10);
            if (FAILED(hr)) {
                if (FAILED(ctx->device->GetDeviceRemovedReason()) || hr != E_NOINTERFACE) check_hr(ctx->device.Get(), hr, "QueryInterface(ID3D12Device10)");
                throw Failure(SA2_E_UNSUPPORTED, hr_text("QueryInterface(ID3D12Device10)", hr));
            }
        }
        ctx->completion = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        if (!ctx->completion) check_hr(ctx->device.Get(), HRESULT_FROM_WIN32(GetLastError()), "CreateEventW");
        ctx->ready = fence(ctx.get(), L"SA2 ready fence");
        ctx->free = fence(ctx.get(), L"SA2 free fence");
        ctx->godot_drain = fence(ctx.get(), L"SA2 Godot drain fence");
        if (config->queue_mode == SA2_QUEUE_OWN) {
            D3D12_COMMAND_QUEUE_DESC desc{};
            desc.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
            check_hr(ctx->device.Get(), ctx->device->CreateCommandQueue(&desc, IID_PPV_ARGS(&ctx->own_queue)), "CreateCommandQueue");
            name(ctx.get(), ctx->own_queue.Get(), L"SA2 own queue");
            ctx->own_drain = fence(ctx.get(), L"SA2 own drain fence");
        }
        D3D12_DESCRIPTOR_HEAP_DESC heap{};
        heap.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
        heap.NumDescriptors = SA2_RING_SLOTS;
        check_hr(ctx->device.Get(), ctx->device->CreateDescriptorHeap(&heap, IID_PPV_ARGS(&ctx->rtv_heap)), "CreateDescriptorHeap");
        name(ctx.get(), ctx->rtv_heap.Get(), L"SA2 RTV heap");
        ctx->rtv_step = ctx->device->GetDescriptorHandleIncrementSize(heap.Type);
        for (uint32_t i = 0; i < SA2_RING_SLOTS; ++i) {
            auto& slot = ctx->slots[i];
            check_hr(ctx->device.Get(), ctx->device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&slot.allocator)), "CreateCommandAllocator");
            name(ctx.get(), slot.allocator.Get(), (L"SA2 slot " + std::to_wstring(i) + L" command allocator").c_str());
            check_hr(ctx->device.Get(), ctx->device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, slot.allocator.Get(), nullptr, IID_PPV_ARGS(&slot.list)), "CreateCommandList");
            name(ctx.get(), slot.list.Get(), (L"SA2 slot " + std::to_wstring(i) + L" command list").c_str());
            check_hr(ctx->device.Get(), slot.list->Close(), "Close new command list");
            if (ctx->config.barrier_api == SA2_BARRIERS_ENHANCED) {
                const auto hr = slot.list.As(&slot.list7);
                if (FAILED(hr)) {
                    if (FAILED(ctx->device->GetDeviceRemovedReason()) || hr != E_NOINTERFACE) check_hr(ctx->device.Get(), hr, "QueryInterface(ID3D12GraphicsCommandList7)");
                    throw Failure(SA2_E_UNSUPPORTED, hr_text("QueryInterface(ID3D12GraphicsCommandList7)", hr));
                }
            }
        }
        if (config->debug_callback) {
            const auto hr = ctx->device.As(&ctx->info_queue);
            if (FAILED(hr) && (hr != E_NOINTERFACE || removed(ctx.get()))) check_hr(ctx->device.Get(), hr, "QueryInterface(ID3D12InfoQueue1)");
            if (ctx->info_queue) {
                check_hr(ctx->device.Get(), ctx->info_queue->RegisterMessageCallback(debug_callback, D3D12_MESSAGE_CALLBACK_IGNORE_FILTERS, ctx.get(), &ctx->callback_cookie), "RegisterMessageCallback");
                ctx->callback_registered = true;
            }
        }
        *out = ctx.release();
        return SA2_OK;
    });
}

int32_t sa2_create_texture(sa2_context* ctx, uint32_t width, uint32_t height, int32_t initial_state, uint64_t* out_resource) {
    if (out_resource) *out_resource = 0;
    return invoke(ctx, "sa2_create_texture", [&] {
        context(ctx);
        require(out_resource && width >= 128 && height >= 128, "output is null or texture smaller than 128");
        const auto initial = state(initial_state);
        live(ctx);
        D3D12_HEAP_PROPERTIES heap{};
        heap.Type = D3D12_HEAP_TYPE_DEFAULT;
        heap.CreationNodeMask = heap.VisibleNodeMask = 1;
        D3D12_RESOURCE_DESC1 desc{};
        desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
        desc.Width = width; desc.Height = height;
        desc.DepthOrArraySize = desc.MipLevels = 1;
        desc.SampleDesc.Count = 1;
        desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
        desc.Flags = D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
        ComPtr<ID3D12Resource> texture;
        if (ctx->device10) {
            check_hr(ctx->device.Get(), ctx->device10->CreateCommittedResource3(&heap, D3D12_HEAP_FLAG_NONE, &desc, initial.layout,
                nullptr, nullptr, 0, nullptr, IID_PPV_ARGS(&texture)), "CreateCommittedResource3(texture)");
        } else {
            D3D12_RESOURCE_DESC legacy{};
            legacy.Dimension = desc.Dimension; legacy.Width = desc.Width; legacy.Height = desc.Height;
            legacy.DepthOrArraySize = desc.DepthOrArraySize; legacy.MipLevels = desc.MipLevels;
            legacy.Format = desc.Format; legacy.SampleDesc = desc.SampleDesc; legacy.Flags = desc.Flags;
            check_hr(ctx->device.Get(), ctx->device->CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &legacy, initial.legacy,
                nullptr, IID_PPV_ARGS(&texture)), "CreateCommittedResource(texture)");
        }
        name(ctx, texture.Get(), L"SA2 imported texture");
        auto* resource = texture.Get();
        ctx->textures.emplace(resource, std::move(texture));
        *out_resource = reinterpret_cast<uint64_t>(resource);
        return SA2_OK;
    });
}

int32_t sa2_release_texture(sa2_context* ctx, uint64_t resource, uint32_t* out_refcount_after) {
    return invoke(ctx, "sa2_release_texture", [&] {
        context(ctx);
        require(resource && out_refcount_after, "zero resource or null output");
        auto* pointer = reinterpret_cast<ID3D12Resource*>(static_cast<uintptr_t>(resource));
        const auto found = ctx->textures.find(pointer);
        require(found != ctx->textures.end(), "texture was not created by this context");
        for (const auto& slot : ctx->slots) require(slot.resource != pointer, "texture still registered", SA2_E_WRONG_STATE);
        require_drained(ctx);
        auto* owned = found->second.Detach();
        ctx->textures.erase(found);
        *out_refcount_after = owned->Release();
        return SA2_OK;
    });
}

int32_t sa2_register_slot(sa2_context* ctx, uint32_t index, uint64_t resource, uint32_t width, uint32_t height, int32_t godot_owned) {
    return invoke(ctx, "sa2_register_slot", [&] {
        auto& slot = slot_at(ctx, index, false);
        require(resource && width >= 128 && height >= 128 && (godot_owned == 0 || godot_owned == 1), "invalid resource, dimensions or ownership");
        live(ctx);
        auto texture = interface_handle<ID3D12Resource>(resource, ctx->device.Get());
        ComPtr<ID3D12Device> device;
        check_hr(ctx->device.Get(), texture->GetDevice(IID_PPV_ARGS(&device)), "ID3D12Resource::GetDevice");
        require(same_identity(ctx->device.Get(), device.Get(), ctx->device.Get()), "texture is on another device");
        const auto desc = texture->GetDesc();
        require(desc.Dimension == D3D12_RESOURCE_DIMENSION_TEXTURE2D && desc.Width == width && desc.Height == height
            && desc.MipLevels == 1 && desc.DepthOrArraySize == 1 && desc.SampleDesc.Count == 1
            && (desc.Flags & D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET)
            && (desc.Format == DXGI_FORMAT_R8G8B8A8_UNORM || desc.Format == DXGI_FORMAT_R8G8B8A8_TYPELESS), "incompatible texture descriptor");
        require(godot_owned || ctx->textures.contains(texture.Get()), "imported texture was not made by this context");
        require(!slot.resource, "slot is already registered", SA2_E_WRONG_STATE);
        // Fence history is kept per slot, so one resource in two slots could be
        // written through one while Godot still reads it through the other.
        for (const auto& other : ctx->slots)
            if (other.resource) require(!same_identity(other.resource, texture.Get(), ctx->device.Get()),
                "resource is already registered in another slot", SA2_E_WRONG_STATE);
        D3D12_RENDER_TARGET_VIEW_DESC view{};
        view.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
        view.ViewDimension = D3D12_RTV_DIMENSION_TEXTURE2D;
        ctx->device->CreateRenderTargetView(texture.Get(), &view, rtv(ctx, index));
        live(ctx);
        slot.resource = texture.Get();
        if (godot_owned) slot.external_reference = std::move(texture);
        slot.width = width; slot.height = height;
        slot.last_shown = slot.pending_value = 0;
        slot.pending_fence = nullptr;
        return SA2_OK;
    });
}

int32_t sa2_unregister_slot(sa2_context* ctx, uint32_t index) {
    return invoke(ctx, "sa2_unregister_slot", [&] {
        auto& slot = slot_at(ctx, index);
        require_drained(ctx);
        slot.resource = nullptr;
        slot.external_reference.Reset();
        slot.readback.Reset();
        slot.pending_fence = nullptr;
        slot.pending_value = slot.last_shown = 0;
        slot.width = slot.height = 0;
        return SA2_OK;
    });
}

int32_t sa2_signal_godot_free(sa2_context* ctx, uint64_t frame) {
    return invoke(ctx, "sa2_signal_godot_free", [&] {
        context(ctx);
        require(frame != UINT64_MAX, "reserved fence value");
        require(frame >= ctx->last_free, "free frame decreased", SA2_E_WRONG_STATE);
        live(ctx);
        signal(ctx, ctx->godot_queue.Get(), ctx->free.Get(), frame);
        ctx->last_free = frame;
        return SA2_OK;
    });
}

int32_t sa2_produce(sa2_context* ctx, uint32_t index, uint64_t frame, uint32_t sequence, uint32_t generation) {
    return invoke(ctx, "sa2_produce", [&] {
        auto& slot = slot_at(ctx, index);
        require(frame != UINT64_MAX, "reserved fence value");
        require(frame > ctx->last_frame, "frame must increase strictly from 1", SA2_E_WRONG_STATE);
        if (ctx->own_queue) require(slot.last_shown <= ctx->last_free, "slot's shown frame has not been signalled free", SA2_E_WRONG_STATE);
        live(ctx);
        reset(ctx, slot);
        // A slot not yet shown needs no wait; a Wait for fence value 0 is always
        // satisfied and draws a debug-layer warning.
        if (ctx->own_queue && slot.last_shown) queue_wait(ctx, ctx->own_queue.Get(), ctx->free.Get(), slot.last_shown);
        transition(ctx, slot, ctx->config.state_before_write, SA2_STATE_RENDER_TARGET, true);
        clear_code(ctx, slot, index, sequence, generation);
        transition(ctx, slot, SA2_STATE_RENDER_TARGET, ctx->config.state_after_write, false);
        ctx->last_frame = frame;
        execute(ctx, slot, ctx->ready.Get(), frame);
        ctx->last_ready = frame;
        return SA2_OK;
    });
}

int32_t sa2_godot_wait_ready(sa2_context* ctx, uint64_t frame) {
    return invoke(ctx, "sa2_godot_wait_ready", [&] {
        context(ctx);
        if (!ctx->own_queue) return SA2_OK;
        require(frame <= ctx->last_ready, "ready frame has never been produced", SA2_E_WRONG_STATE);
        live(ctx);
        queue_wait(ctx, ctx->godot_queue.Get(), ctx->ready.Get(), frame);
        return SA2_OK;
    });
}

int32_t sa2_mark_shown(sa2_context* ctx, uint32_t index, uint64_t frame) {
    return invoke(ctx, "sa2_mark_shown", [&] {
        auto& slot = slot_at(ctx, index);
        require(frame != UINT64_MAX, "reserved fence value");
        require(frame >= slot.last_shown, "shown frame decreased", SA2_E_WRONG_STATE);
        slot.last_shown = frame;
        ctx->drained = false;
        return SA2_OK;
    });
}

int32_t sa2_verify_slot(sa2_context* ctx, uint32_t index, uint32_t sequence, uint32_t generation, uint64_t* out_mismatched_texels) {
    if (out_mismatched_texels) *out_mismatched_texels = 0;
    return invoke(ctx, "sa2_verify_slot", [&] {
        auto& slot = slot_at(ctx, index);
        require(out_mismatched_texels != nullptr, "null mismatch output");
        live(ctx);
        // A diagnostic may follow the current Godot consumer, before the next
        // frame signals free. Bound this CPU wait rather than enqueueing a GPU
        // wait on an as-yet unsignalled free value.
        if (ctx->own_queue) {
            const auto value = ++ctx->godot_drain_value;
            signal(ctx, ctx->godot_queue.Get(), ctx->godot_drain.Get(), value);
            wait_fence(ctx, ctx->godot_drain.Get(), value, deadline(ctx->config.wait_timeout_ms));
        }
        reset(ctx, slot);
        const auto desc = slot.resource->GetDesc();
        D3D12_PLACED_SUBRESOURCE_FOOTPRINT footprint{};
        UINT64 bytes = 0;
        ctx->device->GetCopyableFootprints(&desc, 0, 1, 0, &footprint, nullptr, nullptr, &bytes);
        require(bytes != UINT64_MAX && bytes != 0, "GetCopyableFootprints returned an invalid size", SA2_E_D3D12);
        D3D12_HEAP_PROPERTIES heap{};
        heap.Type = D3D12_HEAP_TYPE_READBACK;
        heap.CreationNodeMask = heap.VisibleNodeMask = 1;
        D3D12_RESOURCE_DESC buffer{};
        buffer.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
        buffer.Width = bytes; buffer.Height = 1;
        buffer.DepthOrArraySize = buffer.MipLevels = 1;
        buffer.SampleDesc.Count = 1;
        buffer.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
        check_hr(ctx->device.Get(), ctx->device->CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &buffer,
            D3D12_RESOURCE_STATE_COPY_DEST, nullptr, IID_PPV_ARGS(&slot.readback)), "CreateCommittedResource(readback)");
        name(ctx, slot.readback.Get(), (L"SA2 slot " + std::to_wstring(index) + L" readback").c_str());
        transition(ctx, slot, ctx->config.state_after_write, SA2_STATE_COPY_SOURCE, true, true);
        D3D12_TEXTURE_COPY_LOCATION source{};
        source.pResource = slot.resource;
        source.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
        D3D12_TEXTURE_COPY_LOCATION destination{};
        destination.pResource = slot.readback.Get();
        destination.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
        destination.PlacedFootprint = footprint;
        slot.list->CopyTextureRegion(&destination, 0, 0, 0, &source, nullptr);
        transition(ctx, slot, SA2_STATE_COPY_SOURCE, ctx->config.state_after_write, false, true);
        auto* completion = ctx->own_queue ? ctx->own_drain.Get() : ctx->godot_drain.Get();
        auto& value = ctx->own_queue ? ctx->own_drain_value : ctx->godot_drain_value;
        execute(ctx, slot, completion, ++value);
        wait_fence(ctx, completion, value, deadline(ctx->config.wait_timeout_ms));
        const auto expected = sa2::image(slot.width, slot.height, {sequence, index, generation});
        void* mapped = nullptr;
        const D3D12_RANGE read_range{static_cast<SIZE_T>(footprint.Offset), static_cast<SIZE_T>(bytes)};
        check_hr(ctx->device.Get(), slot.readback->Map(0, &read_range, &mapped), "ID3D12Resource::Map");
        const auto* pixels = static_cast<const uint8_t*>(mapped) + footprint.Offset;
        uint64_t mismatch = 0;
        for (uint32_t y = 0; y < slot.height; ++y)
            for (uint32_t x = 0; x < slot.width; ++x)
                if (std::memcmp(pixels + std::size_t(y) * footprint.Footprint.RowPitch + x * 4,
                    expected[std::size_t(y) * slot.width + x].data(), 4) != 0) ++mismatch;
        const D3D12_RANGE written{0, 0};
        slot.readback->Unmap(0, &written);
        *out_mismatched_texels = mismatch;
        require(mismatch == 0, "image contains mismatched texels", SA2_E_VERIFY);
        return SA2_OK;
    });
}

int32_t sa2_drain(sa2_context* ctx, uint32_t timeout_ms) {
    return invoke(ctx, "sa2_drain", [&] {
        context(ctx);
        live(ctx);
        try {
            char inject[2]{};
            if (GetEnvironmentVariableA("M600_SA2_INJECT_UNCONFIRMED_DRAIN", inject, sizeof(inject)) == 1 && inject[0] == '1') {
                live(ctx);
                abandon();
            }
            const auto until = deadline(timeout_ms);
            signal(ctx, ctx->godot_queue.Get(), ctx->godot_drain.Get(), ++ctx->godot_drain_value);
            if (ctx->own_queue) signal(ctx, ctx->own_queue.Get(), ctx->own_drain.Get(), ++ctx->own_drain_value);
            wait_fence(ctx, ctx->godot_drain.Get(), ctx->godot_drain_value, until);
            if (ctx->own_queue) wait_fence(ctx, ctx->own_drain.Get(), ctx->own_drain_value, until);
            live(ctx); // UINT64_MAX/removed completion is never a confirmation.
        } catch (...) {
            if (removed(ctx)) throw Failure(SA2_E_DEVICE_REMOVED, hr_text("GetDeviceRemovedReason", ctx->device->GetDeviceRemovedReason()));
            abandon();
        }
        ctx->drained = true;
        return SA2_OK;
    });
}

int32_t sa2_debug_counts(sa2_context* ctx, struct sa2_debug_counts* out) {
    return invoke(ctx, "sa2_debug_counts", [&] {
        context(ctx);
        require(out && out->struct_size == sizeof(*out), "invalid debug_counts struct_size");
        struct sa2_debug_counts result{};
        result.struct_size = sizeof(result);
        if (ctx->callback_registered) {
            auto& d = ctx->debug;
            result.corruption = d.corruption.load(); result.error = d.error.load(); result.warning = d.warning.load();
            result.info = d.info.load(); result.message = d.message.load(); result.mismatching_clear_value = d.mismatch.load();
            result.mentioning_sa2 = d.mentioning.load();
            std::lock_guard lock(d.mutex);
            result.distinct_id_count = static_cast<uint32_t>(d.ids.size());
            std::copy(d.ids.begin(), d.ids.end(), result.ids);
        }
        *out = result;
        return SA2_OK;
    });
}

int32_t sa2_debug_messages(sa2_context* ctx, char* buffer, uint32_t buffer_size) {
    return invoke(ctx, "sa2_debug_messages", [&] {
        context(ctx);
        std::string text;
        {
            std::lock_guard lock(ctx->debug.mutex);
            for (const auto& description : ctx->debug.descriptions) {
                if (!text.empty()) text += '\n';
                text += description;
            }
        }
        copy_text(text, buffer, buffer_size);
        return SA2_OK;
    });
}

int32_t sa2_remove_device(sa2_context* ctx) {
    return invoke(ctx, "sa2_remove_device", [&] {
        live(ctx);
        ComPtr<ID3D12Device5> device5;
        const auto hr = ctx->device.As(&device5);
        if (FAILED(hr)) {
            if (hr != E_NOINTERFACE || removed(ctx)) check_hr(ctx->device.Get(), hr, "QueryInterface(ID3D12Device5)");
            throw Failure(SA2_E_UNSUPPORTED, hr_text("QueryInterface(ID3D12Device5)", hr));
        }
        device5->RemoveDevice();
        return SA2_OK;
    });
}

int32_t sa2_device_removed_reason(sa2_context* ctx, int32_t* out_hresult) {
    return invoke(ctx, "sa2_device_removed_reason", [&] {
        context(ctx);
        require(out_hresult != nullptr, "null HRESULT output");
        *out_hresult = ctx->device->GetDeviceRemovedReason();
        return SA2_OK;
    });
}

int32_t sa2_detach(sa2_context* ctx) {
    return invoke(ctx, "sa2_detach", [&] {
        context(ctx);
        require(!ctx->scene, "scene remains loaded", SA2_E_WRONG_STATE);
        for (const auto& slot : ctx->slots) require(!slot.resource, "slots remain registered", SA2_E_WRONG_STATE);
        require(ctx->textures.empty(), "created textures remain unreleased", SA2_E_WRONG_STATE);
        require_drained(ctx);
        if (ctx->callback_registered) {
            const auto hr = ctx->info_queue->UnregisterMessageCallback(ctx->callback_cookie);
            if (FAILED(hr) && !removed(ctx)) check_hr(ctx->device.Get(), hr, "UnregisterMessageCallback");
            ctx->callback_registered = false;
        }
        delete ctx;
        return SA2_OK;
    });
}

int32_t sa2_scene_load(sa2_context* ctx, const sa2_scene_config* config) {
    return invoke(ctx, "sa2_scene_load", [&] {
        sa2::validate_scene_config(config);
        context(ctx);
        require(!ctx->scene, "a scene is already loaded", SA2_E_WRONG_STATE);
        const auto& first = slot_at(ctx, 0);
        for (const auto& slot : ctx->slots) {
            require(slot.resource != nullptr, "register the complete three-slot ring before load", SA2_E_WRONG_STATE);
            require(slot.width == first.width && slot.height == first.height, "scene slots must be equally sized");
        }
        live(ctx);
        ctx->scene = std::make_unique<sa2::Scene>(ctx->device.Get(), *config, first.width, first.height);
        return SA2_OK;
    });
}

int32_t sa2_scene_load_wj(sa2_context* ctx, const sa2_wj_config* config) {
    return invoke(ctx, "sa2_scene_load_wj", [&] {
        sa2::validate_wj_config(config);
        context(ctx);
        require(!ctx->scene, "a scene is already loaded", SA2_E_WRONG_STATE);
        const auto& first = slot_at(ctx, 0);
        for (const auto& slot : ctx->slots) {
            require(slot.resource != nullptr, "register the complete three-slot ring before load", SA2_E_WRONG_STATE);
            require(slot.width == first.width && slot.height == first.height, "scene slots must be equally sized");
        }
        live(ctx);
        ctx->scene = std::make_unique<sa2::Scene>(ctx->device.Get(), *config, first.width, first.height);
        return SA2_OK;
    });
}

int32_t sa2_scene_produce(sa2_context* ctx, uint32_t index, uint64_t frame) {
    return invoke(ctx, "sa2_scene_produce", [&] {
        context(ctx);
        require(index < SA2_RING_SLOTS && frame != UINT64_MAX, "invalid slot or reserved fence value");
        require(ctx->scene != nullptr, "no loaded scene", SA2_E_WRONG_STATE);
        auto& slot = slot_at(ctx, index);
        auto& scene = *ctx->scene;
        for (const auto& current : ctx->slots)
            require(current.resource && current.width == scene.record.width && current.height == scene.record.height,
                "scene needs its load-time three-slot size", SA2_E_WRONG_STATE);
        require(frame > ctx->last_frame, "frame must increase strictly from 1", SA2_E_WRONG_STATE);
        if (ctx->own_queue) require(slot.last_shown <= ctx->last_free, "slot's shown frame has not been signalled free", SA2_E_WRONG_STATE);
        scene.require_healthy();
        const auto now = sa2::scene_qpc();
        live(ctx);
        scene.wait_geometry(ctx->config.wait_timeout_ms);
        reset(ctx, slot);
        if (ctx->own_queue && slot.last_shown) queue_wait(ctx, ctx->own_queue.Get(), ctx->free.Get(), slot.last_shown);
        try {
            transition(ctx, slot, ctx->config.state_before_write, SA2_STATE_RENDER_TARGET, true);
            scene.record_draw(slot.list.Get(), index, rtv(ctx, index), now);
            scene.sample_vram();
            transition(ctx, slot, SA2_STATE_RENDER_TARGET, ctx->config.state_after_write, false);
            ctx->last_frame = frame;
            execute(ctx, slot, ctx->ready.Get(), frame);
            ctx->last_ready = frame;
        } catch (...) { scene.poison(); throw; }
        return SA2_OK;
    });
}

int32_t sa2_scene_trace_begin(sa2_context* ctx) {
    return invoke(ctx, "sa2_scene_trace_begin", [&] {
        context(ctx);
        require(ctx->scene != nullptr, "no loaded scene", SA2_E_WRONG_STATE);
        ctx->scene->require_healthy();
        ctx->scene->record.begin(sa2::scene_qpc());
        return SA2_OK;
    });
}

int32_t sa2_scene_trace_end(sa2_context* ctx) {
    return invoke(ctx, "sa2_scene_trace_end", [&] {
        context(ctx);
        require(ctx->scene != nullptr, "no loaded scene", SA2_E_WRONG_STATE);
        ctx->scene->record.end(sa2::scene_qpc());
        return SA2_OK;
    });
}

int32_t sa2_scene_write_run(sa2_context* ctx, const char* directory_utf8) {
    return invoke(ctx, "sa2_scene_write_run", [&] {
        const auto directory = sa2::output_directory(directory_utf8);
        context(ctx);
        require(ctx->scene != nullptr, "no loaded scene", SA2_E_WRONG_STATE);
        auto& scene = *ctx->scene;
        scene.record.require_write(ctx->drained);
        live(ctx); scene.require_healthy();
        sa2::require_new_file(directory / "trace.jsonl"); sa2::require_new_file(directory / "native.json");
        const bool jumbling = scene.record.jumbling();
        if (jumbling) sa2::require_new_file(directory / "pose_check.json");
        Json poses;
        const auto labels = scene.labels(poses);
        const auto native = scene.record.native(labels, sa2::scene_identity(), ctx->config.queue_mode, ctx->config.barrier_api, poses);
        sa2::write_new_file(directory / "trace.jsonl", scene.record.trace_text());
        sa2::write_new_file(directory / "native.json", native.dump() + '\n');
        if (jumbling) {
            sa2::write_new_file(directory / "pose_check.json", poses.dump() + '\n');
            require(labels.at("status").string() == "pass" && poses.at("status").string() == "pass",
                "W-J exact label or pose check failed", SA2_E_CHECK_FAILED);
        }
        require(labels.null() || labels.at("status").string() == "pass", "S-B exact label check failed", SA2_E_CHECK_FAILED);
        return SA2_OK;
    });
}

int32_t sa2_scene_geometry_check(sa2_context* ctx, const char* directory_utf8) {
    return invoke(ctx, "sa2_scene_geometry_check", [&] {
        const auto directory = sa2::output_directory(directory_utf8);
        context(ctx);
        require(ctx->scene != nullptr, "no loaded scene", SA2_E_WRONG_STATE);
        auto& scene = *ctx->scene;
        scene.record.require_geometry(); scene.require_healthy(); live(ctx);
        const auto file = directory / "geometry.json"; sa2::require_new_file(file);
        Json result;
        try {
            result = scene.geometry_check(ctx->producer(), ctx->config.wait_timeout_ms, [&] { ctx->drained = false; });
        } catch (const Failure& error) {
            if (error.status != SA2_E_CHECK_FAILED) {
                // A bounded timeout keeps every submitted resource alive; a
                // failed recording requires a drain/unload before reuse.
                if (error.status != SA2_E_TIMEOUT) scene.poison();
                throw;
            }
            result = scene.geometry_error(error.what());
        } catch (...) { scene.poison(); throw; }
        sa2::write_new_file(file, result.dump() + '\n');
        require(result.at("status").string() == "pass", scene.record.jumbling() ? "W-J geometry check failed" : "S-B geometry check failed", SA2_E_CHECK_FAILED);
        return SA2_OK;
    });
}

int32_t sa2_scene_lattice_check(sa2_context* ctx, const char* directory_utf8) {
    return invoke(ctx, "sa2_scene_lattice_check", [&] {
        const auto directory = sa2::output_directory(directory_utf8);
        context(ctx);
        require(ctx->scene != nullptr, "no loaded scene", SA2_E_WRONG_STATE);
        auto& scene = *ctx->scene;
        scene.record.require_lattice(); scene.require_healthy(); live(ctx);
        const auto file = directory / "lattice_check.json"; sa2::require_new_file(file);
        Json result;
        try {
            result = scene.lattice_check(ctx->producer(), ctx->config.wait_timeout_ms, [&] { ctx->drained = false; });
        } catch (const Failure& error) {
            if (error.status != SA2_E_CHECK_FAILED) {
                // As the geometry check: a bounded timeout keeps every
                // submitted resource alive; other failures poison the scene.
                if (error.status != SA2_E_TIMEOUT) scene.poison();
                throw;
            }
            result = scene.lattice_error(error.what());
        } catch (...) { scene.poison(); throw; }
        sa2::write_new_file(file, result.dump() + '\n');
        require(result.at("status").string() == "pass", "W-J lattice check failed", SA2_E_CHECK_FAILED);
        return SA2_OK;
    });
}

int32_t sa2_identity(char* buffer, uint32_t buffer_size) {
    return invoke(nullptr, "sa2_identity", [&] {
        require(buffer && buffer_size, "buffer and buffer_size must be nonzero");
        copy_text(sa2::scene_identity().dump(), buffer, buffer_size);
        return SA2_OK;
    });
}

int32_t sa2_scene_unload(sa2_context* ctx) {
    return invoke(ctx, "sa2_scene_unload", [&] {
        context(ctx);
        require(ctx->scene != nullptr, "no loaded scene", SA2_E_WRONG_STATE);
        require_drained(ctx);
        ctx->scene.reset();
        return SA2_OK;
    });
}

// ABI 4 product API.
int32_t sa2_describe(sa2_context* ctx, char* buffer, uint32_t buffer_size, uint32_t* out_size) {
    if (out_size) *out_size = 0;
    return invoke(ctx, "sa2_describe", [&] {
        context(ctx);
        require_text_output(buffer, buffer_size, out_size);
        const auto identity = sa2::scene_identity().dump();
        const auto build = sb::sha256(std::span(reinterpret_cast<const uint8_t*>(identity.data()), identity.size()));
        write_json(sa2::describe(build, removed_reason(ctx)).dump(), buffer, buffer_size, out_size);
        return SA2_OK;
    });
}

int32_t sa2_engine_adopt(sa2_context* ctx, const sa2_engine_image* image, uint64_t* out_revision) {
    if (out_revision) *out_revision = 0;
    return invoke(ctx, "sa2_engine_adopt", [&] {
        context(ctx);
        require(out_revision != nullptr, "null revision output");
        auto copy = sa2::engine_image(image);
        auto& scene = product_scene(ctx);
        *out_revision = scene.product.adopt(std::move(copy));
        return SA2_OK;
    });
}

int32_t sa2_prepare(sa2_context* ctx, const sa2_preparation* preparation, uint64_t* out_ticket) {
    if (out_ticket) *out_ticket = 0;
    return invoke(ctx, "sa2_prepare", [&] {
        context(ctx);
        require(out_ticket != nullptr, "null ticket output");
        const auto data = sa2::preparation(preparation);
        auto& scene = product_scene(ctx);
        scene.product.check_prepare(data);
        scene.require_healthy();
        live(ctx);
        scene.product_ready(ctx->producer());
        const auto ticket = scene.product.reserve_ticket();
        const auto fence = scene.product_prepare(ticket, data, ctx->producer(), [&] { ctx->drained = false; });
        scene.product.prepared(ticket, data, fence);
        *out_ticket = ticket;
        release(ctx, scene);
        return SA2_OK;
    });
}

int32_t sa2_poll(sa2_context* ctx, uint64_t ticket, sa2_poll_row* out) {
    return invoke(ctx, "sa2_poll", [&] {
        context(ctx);
        require(out && out->struct_size == sizeof(*out), "invalid poll_row struct_size");
        auto& scene = product_scene(ctx);
        *out = scene.product.poll(ticket, scene.product_completed());
        release(ctx, scene);
        return SA2_OK;
    });
}

int32_t sa2_activate(sa2_context* ctx, uint64_t resource_set, const uint8_t stamp_digest[32]) {
    return invoke(ctx, "sa2_activate", [&] {
        context(ctx);
        require(stamp_digest != nullptr, "stamp_digest is NULL");
        auto& scene = product_scene(ctx);
        scene.product.activate(resource_set, stamp_digest);
        return SA2_OK;
    });
}

int32_t sa2_cancel(sa2_context* ctx, uint64_t ticket, int32_t reason, uint64_t superseded_by) {
    return invoke(ctx, "sa2_cancel", [&] {
        context(ctx);
        sa2::validate_cancel(reason, superseded_by);
        auto& scene = product_scene(ctx);
        scene.product.cancel(ticket, reason, superseded_by);
        release(ctx, scene);
        return SA2_OK;
    });
}

int32_t sa2_retire(sa2_context* ctx, uint64_t resource_set) {
    return invoke(ctx, "sa2_retire", [&] {
        context(ctx);
        auto& scene = product_scene(ctx);
        scene.product.retire(resource_set);
        release(ctx, scene);
        return SA2_OK;
    });
}

int32_t sa2_frame_submit(sa2_context* ctx, const sa2_frame_input* input, uint64_t* out_frame_id) {
    if (out_frame_id) *out_frame_id = 0;
    return invoke(ctx, "sa2_frame_submit", [&] {
        context(ctx);
        require(out_frame_id != nullptr, "null frame ID output");
        sa2::validate_frame_input(input);
        auto& scene = product_scene(ctx);
        SlotTarget slot(ctx, input->target);
        sa2::FrameTarget& target = slot;
        require(target.width() == scene.record.width && target.height() == scene.record.height,
            "the frame target is not at the scene's size", SA2_E_WRONG_STATE);
        require(ctx->last_frame < UINT64_MAX - 1, "frame IDs are exhausted", SA2_E_WRONG_STATE);
        const uint64_t frame = ctx->last_frame + 1;
        const auto plan = scene.product.plan(*input, target.width(), target.height());
        scene.require_healthy();
        live(ctx);
        scene.wait_geometry(ctx->config.wait_timeout_ms);
        scene.product_ready(ctx->producer());
        if (plan.upload) target.wait(plan.upload_wait); // the copy's staging was last read by that frame
        auto* list = target.open();
        try {
            harvest(ctx, scene); // the target's previous frame completed: read its ring results before this frame reuses them
            scene.product_frame(list, *input, plan, target.view(), target.ring());
            target.close(frame);
        } catch (...) { scene.poison(); throw; }
        scene.product.commit(*input, plan, frame, target.ring());
        *out_frame_id = frame;
        release(ctx, scene);
        return SA2_OK;
    });
}

int32_t sa2_frame_reports(sa2_context* ctx, sa2_frame_report* rows, uint32_t capacity, uint32_t* out_count) {
    if (out_count) *out_count = 0;
    return invoke(ctx, "sa2_frame_reports", [&] {
        context(ctx);
        require(out_count != nullptr, "null count output");
        sa2::validate_rows(rows, capacity);
        auto& scene = product_scene(ctx);
        harvest(ctx, scene);
        *out_count = scene.product.reports(rows, capacity);
        release(ctx, scene);
        return SA2_OK;
    });
}

int32_t sa2_presented(sa2_context* ctx, sa2_presented_row* rows, uint32_t capacity, uint32_t* out_count) {
    if (out_count) *out_count = 0;
    return invoke(ctx, "sa2_presented", [&]() -> int32_t {
        context(ctx);
        require(out_count != nullptr, "null count output");
        sa2::validate_rows(rows, capacity);
        throw Failure(SA2_E_UNAVAILABLE, "the core owns no swap chain and cannot confirm a presentation (RM.26)");
    });
}

int32_t sa2_pick_request(sa2_context* ctx, uint64_t frame_id, uint32_t x, uint32_t y, uint64_t* out_ticket) {
    if (out_ticket) *out_ticket = 0;
    return invoke(ctx, "sa2_pick_request", [&] {
        context(ctx);
        require(out_ticket != nullptr, "null ticket output");
        auto& scene = product_scene(ctx);
        const auto plan = scene.product.pick_plan(frame_id, x, y);
        uint64_t fence = 0;
        if (plan.pass) {
            scene.require_healthy();
            live(ctx);
            scene.wait_geometry(ctx->config.wait_timeout_ms);
        }
        const auto ticket = scene.product.reserve_pick();
        if (plan.pass) fence = scene.product_pick(ticket, plan.input, ctx->producer(), [&] { ctx->drained = false; });
        scene.product.pick_commit(ticket, plan, fence);
        *out_ticket = ticket;
        release(ctx, scene);
        return SA2_OK;
    });
}

int32_t sa2_pick_poll(sa2_context* ctx, uint64_t ticket, sa2_pick_row* out) {
    return invoke(ctx, "sa2_pick_poll", [&] {
        context(ctx);
        require(out && out->struct_size == sizeof(*out), "invalid pick_row struct_size");
        auto& scene = product_scene(ctx);
        bool forget = false;
        *out = scene.product.pick_poll(ticket, scene.product_completed(), [&] { return scene.product_pick_value(ticket); }, forget);
        if (forget) scene.product_release_pick(ticket);
        release(ctx, scene);
        return SA2_OK;
    });
}

int32_t sa2_recover(sa2_context* ctx, char* buffer, uint32_t buffer_size, uint32_t* out_size) {
    if (out_size) *out_size = 0;
    return invoke(ctx, "sa2_recover", [&]() -> int32_t {
        context(ctx);
        require_text_output(buffer, buffer_size, out_size);
        write_json(sa2::recover_report(removed_reason(ctx)).dump(), buffer, buffer_size, out_size);
        throw Failure(SA2_E_UNAVAILABLE, "ABI 4 cannot recover a device: it owns no swap chain and no device (RPI.46)");
    });
}

int32_t sa2_resize(sa2_context* ctx, uint32_t width, uint32_t height) {
    return invoke(ctx, "sa2_resize", [&] {
        context(ctx);
        require(width >= SA2_MIN_TEXTURE_PX && height >= SA2_MIN_TEXTURE_PX, "width and height must be at least SA2_MIN_TEXTURE_PX");
        auto& scene = product_scene(ctx);
        require_drained(ctx);
        live(ctx);
        for (uint32_t index = 0; index < SA2_RING_SLOTS; ++index) {
            SlotTarget slot(ctx, index);
            const sa2::FrameTarget& target = slot;
            require(target.width() == width && target.height() == height,
                "register all three slots at the new size before sa2_resize", SA2_E_WRONG_STATE);
        }
        scene.resize(width, height);
        scene.product.resized();
        return SA2_OK;
    });
}

int32_t sa2_counters(sa2_context* ctx, sa2_counter_values* out) {
    return invoke(ctx, "sa2_counters", [&] {
        context(ctx);
        require(out && out->struct_size == sizeof(*out), "invalid counter_values struct_size");
        if (ctx->scene) *out = ctx->scene->product.counter_values();
        else {
            sa2_counter_values zero{};
            zero.struct_size = sizeof(zero);
            *out = zero;
        }
        return SA2_OK;
    });
}

int32_t sa2_device_status(sa2_context* ctx, int32_t* out_removed_reason) {
    if (out_removed_reason) *out_removed_reason = 0;
    return invoke(ctx, "sa2_device_status", [&] {
        context(ctx);
        require(out_removed_reason != nullptr, "null removed-reason output");
        const int32_t hr = ctx->device->GetDeviceRemovedReason();
        *out_removed_reason = hr;
        if (ctx->scene) ctx->scene->product.observe_removal(hr);
        if (hr < 0) throw Failure(SA2_E_DEVICE_REMOVED, "device removed (" + sa2::removed_text(hr) + ")");
        return SA2_OK;
    });
}
}
