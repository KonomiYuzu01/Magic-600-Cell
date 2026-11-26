#include "pure.h"
#include "scene.h"
#include "frame_target.h"
#include <d3d12sdklayers.h>
#include <dxgi1_6.h>
#include <dcomp.h>
#include <wrl/client.h>
#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <cstring>
#include <iomanip>
#include <memory>
#include <mutex>
#include <optional>
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

// ABI 5, route B. Route B is accepted for now by the owner on 18 Nov 2026,
// pending three owner-machine checks (docs/wiki/decisions/
// owner-decisions-2026-11-18-route-b.md). The device factory, the composition
// swap chain, the DirectComposition layer and the recovery order are adapted
// from the C9 present probe (work/experiments/c9-present-probe/native/src/
// c9pp.cpp, PR 245; see PROVENANCE.md).
constexpr DXGI_FORMAT SurfaceFormat = DXGI_FORMAT_B8G8R8A8_UNORM;
constexpr UINT SurfaceFlags = DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT;
constexpr UINT SurfaceLatency = 2;
// C9 (26 Nov 2026): a product frame whose frame-latency wait has not ended
// after this many milliseconds takes one statistics sample for the present
// ledger, then waits on (sa2::latency_wait). Five milliseconds after the wait
// starts, which is a millisecond or two after a vblank, the last vblank's
// statistics are posted and the next vblank is still ahead at 60 to 144 Hz.
constexpr uint32_t SurfaceSampleMs = 5;
constexpr GUID ClsidDeviceFactory = {0x114863bf, 0xc386, 0x4aee, {0xb3, 0x9d, 0x8f, 0x0b, 0xbb, 0x06, 0x29, 0x55}};
static_assert(SA2_SURFACE_BUFFERS == SA2_RING_SLOTS, "a surface buffer uses its ring slot's RTV and query results");
// A composition swap chain and its back buffers; each buffer records with an
// allocator and a list of its own and keeps its last frame's fence value.
struct Chain {
    ComPtr<IDXGISwapChain3> swapchain;
    HANDLE waitable = nullptr;
    std::array<Slot, SA2_SURFACE_BUFFERS> buffers;
    Chain() = default;
    Chain(const Chain&) = delete;
    Chain& operator=(const Chain&) = delete;
    ~Chain() { if (waitable) CloseHandle(waitable); }
};
// The attached surface: the window, the visual's rectangle in client pixels,
// the DirectComposition objects and the current generation's swap chain.
struct Surface {
    HWND hwnd = nullptr;
    int32_t x = 0, y = 0;
    uint32_t width = 0, height = 0;
    uint64_t generation = 0;
    ComPtr<IDCompositionDesktopDevice> dcomp;
    ComPtr<IDCompositionTarget> target;
    ComPtr<IDCompositionVisual2> visual;
    std::unique_ptr<Chain> chain;
};
// A sa2_create context's scene config, kept so a recovery can load the scene
// again on the new device. The strings are copies; load points a copy of the
// config at them.
struct SceneSource {
    bool wj = false, has_inject = false;
    sa2_scene_config scene{};
    sa2_wj_config jumble{};
    std::string inject, menu, data;
    void keep(const sa2_scene_config& config) {
        *this = SceneSource{};
        scene = config;
        scene.inject = nullptr;
        has_inject = config.inject != nullptr;
        if (has_inject) inject = config.inject;
    }
    void keep(const sa2_wj_config& config) {
        *this = SceneSource{};
        wj = true;
        jumble = config;
        jumble.menu = jumble.data_utf8 = jumble.inject = nullptr;
        has_inject = config.inject != nullptr;
        if (has_inject) inject = config.inject;
        menu = config.menu;
        data = config.data_utf8;
    }
    std::unique_ptr<sa2::Scene> load(ID3D12Device* device, uint32_t width, uint32_t height) const {
        if (wj) {
            auto config = jumble;
            config.menu = menu.c_str();
            config.data_utf8 = data.c_str();
            config.inject = has_inject ? inject.c_str() : nullptr;
            return std::make_unique<sa2::Scene>(device, config, width, height);
        }
        auto config = scene;
        config.inject = has_inject ? inject.c_str() : nullptr;
        return std::make_unique<sa2::Scene>(device, config, width, height);
    }
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
    // ABI 5, route B (accepted for now by the owner on 18 Nov 2026, pending
    // three owner-machine checks): a sa2_create context owns its device, and
    // godot_queue is then the core's own DIRECT queue. requested is the config
    // as given, for a recovery's setup.
    bool created = false;
    ComPtr<IDXGIFactory4> factory;
    LUID adapter{};
    sa2_config requested{};
    uint64_t device_epoch = sa2::DeviceEpoch, surface_generation = 0;
    std::unique_ptr<Surface> surface;
    SceneSource source;
    // C9 U5: the device's compiled user stages, made by the first sa2_stage_compile; a recovery drops them. Handles
    // count up from 1 and are never reused, not even after a recovery.
    std::unique_ptr<sa2::stages::Registry> stages;
    uint64_t stage_handles = 0;
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
    DXGI_FORMAT format() const override { return DXGI_FORMAT_R8G8B8A8_UNORM; }
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

// ABI 5's frame target (route B; accepted for now by the owner on 18 Nov
// 2026, pending three owner-machine checks): the attached surface's current
// back buffer, presented with SA2_SURFACE_SYNC_INTERVAL (c9pp.cpp's frame).
// The buffer moves between COMMON (PRESENT) and RENDER_TARGET.
Surface& attached(sa2_context* ctx) {
    require(ctx->surface && ctx->surface->chain, "no surface is attached", SA2_E_WRONG_STATE);
    return *ctx->surface;
}
class SurfaceTarget final : public sa2::FrameTarget {
public:
    explicit SurfaceTarget(sa2_context* ctx) : ctx_(ctx), surface_(attached(ctx)), chain_(*surface_.chain),
        index_(chain_.swapchain->GetCurrentBackBufferIndex()), buffer_(chain_.buffers.at(index_)) {
        require(buffer_.resource != nullptr, "the surface's back buffer is not acquired", SA2_E_WRONG_STATE);
    }
    uint32_t width() const override { return surface_.width; }
    uint32_t height() const override { return surface_.height; }
    uint32_t ring() const override { return index_; }
    DXGI_FORMAT format() const override { return SurfaceFormat; }
    void wait(uint64_t frame) override {
        if (frame) wait_fence(ctx_, ctx_->ready.Get(), frame, deadline(ctx_->config.wait_timeout_ms));
    }
    // C9 (26 Nov 2026): a product frame's statistics sample, taken once when
    // the frame-latency wait lasts longer than SurfaceSampleMs. A scene
    // frame's target has none: its wait and its trace samples are unchanged.
    void sample_while_waiting(std::function<void()> sample) { sample_ = std::move(sample); }
    ID3D12GraphicsCommandList* open() override {
        // The buffer's previous frame first: a timeout there leaves the
        // frame-latency waitable untouched.
        wait_previous(ctx_, buffer_);
        const auto wait = [this](uint32_t ms) { return WaitForSingleObjectEx(chain_.waitable, ms, FALSE); };
        const DWORD status = sample_ ? sa2::latency_wait(ctx_->config.wait_timeout_ms, SurfaceSampleMs, WAIT_TIMEOUT, wait, sample_)
                                     : wait(ctx_->config.wait_timeout_ms);
        if (status == WAIT_TIMEOUT) {
            live(ctx_);
            throw Failure(SA2_E_TIMEOUT, "the surface's frame-latency wait timed out");
        }
        if (status != WAIT_OBJECT_0) {
            const auto error = GetLastError();
            check_hr(ctx_->device.Get(), error ? HRESULT_FROM_WIN32(error) : E_FAIL, "WaitForSingleObjectEx(frame latency)");
        }
        reset(ctx_, buffer_);
        transition(ctx_, buffer_, SA2_STATE_COMMON, SA2_STATE_RENDER_TARGET, true);
        return buffer_.list.Get();
    }
    D3D12_CPU_DESCRIPTOR_HANDLE view() const override { return rtv(ctx_, index_); }
    void close(uint64_t frame) override {
        transition(ctx_, buffer_, SA2_STATE_RENDER_TARGET, SA2_STATE_COMMON, false);
        ctx_->last_frame = frame;
        execute(ctx_, buffer_, ctx_->ready.Get(), frame);
        ctx_->last_ready = frame;
        check_hr(ctx_->device.Get(), chain_.swapchain->Present(SA2_SURFACE_SYNC_INTERVAL, 0), "IDXGISwapChain::Present");
        // The frame's own present count; a failed read leaves the frame
        // unconfirmable, never confirmed by another frame's row.
        UINT count = 0;
        if (SUCCEEDED(chain_.swapchain->GetLastPresentCount(&count)) && count) count_ = count;
    }
    std::optional<uint32_t> present_count() const { return count_; }
    uint64_t generation() const { return surface_.generation; }
private:
    sa2_context* ctx_;
    Surface& surface_;
    Chain& chain_;
    uint32_t index_;
    Slot& buffer_;
    std::optional<uint32_t> count_;
    std::function<void()> sample_;
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
// answers SA2_E_DEVICE_REMOVED and never reads a pick). C9 (26 Nov 2026): on a
// live device a released set's resources are kept for the next preparation
// (sa2::Spares); a removal drops them.
void release(sa2_context* ctx, sa2::Scene& scene) {
    const bool live = !scene.product.removed();
    for (const auto ticket : scene.product.releasable(completed(ctx->ready.Get()), scene.product_completed()))
        scene.product_release_set(ticket, live);
    if (scene.product.removed()) {
        scene.product_release_picks();
        scene.product_release_spares();
    }
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

// sa2_attach's config checks and context setup, shared with sa2_create and an
// ABI 5 recovery.
void validate_config(sa2_context** out, const sa2_config* config) {
    require(out && config && config->struct_size == sizeof(*config), "invalid config struct_size or output");
    require(config->queue_mode == SA2_QUEUE_SAME || config->queue_mode == SA2_QUEUE_OWN, "invalid queue_mode");
    require(config->barrier_api >= SA2_BARRIERS_MATCH_GODOT && config->barrier_api <= SA2_BARRIERS_ENHANCED, "invalid barrier_api");
    state(config->state_before_write); state(config->state_after_write);
    require(config->debug_callback == 0 || config->debug_callback == 1, "invalid debug_callback");
}
void command_list(sa2_context* ctx, Slot& slot, const std::wstring& label) {
    check_hr(ctx->device.Get(), ctx->device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&slot.allocator)), "CreateCommandAllocator");
    name(ctx, slot.allocator.Get(), (label + L" command allocator").c_str());
    check_hr(ctx->device.Get(), ctx->device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, slot.allocator.Get(), nullptr, IID_PPV_ARGS(&slot.list)), "CreateCommandList");
    name(ctx, slot.list.Get(), (label + L" command list").c_str());
    check_hr(ctx->device.Get(), slot.list->Close(), "Close new command list");
    if (ctx->config.barrier_api == SA2_BARRIERS_ENHANCED) {
        const auto hr = slot.list.As(&slot.list7);
        if (FAILED(hr)) {
            if (FAILED(ctx->device->GetDeviceRemovedReason()) || hr != E_NOINTERFACE) check_hr(ctx->device.Get(), hr, "QueryInterface(ID3D12GraphicsCommandList7)");
            throw Failure(SA2_E_UNSUPPORTED, hr_text("QueryInterface(ID3D12GraphicsCommandList7)", hr));
        }
    }
}
// On ctx->device and ctx->godot_queue: the config with its barrier API
// resolved, the fences, the optional own queue, the RTV heap and the slots'
// command lists.
void setup(sa2_context* ctx, const sa2_config& config) {
    ctx->config = config;
    const bool enhanced = enhanced_supported(ctx->device.Get());
    if (config.barrier_api == SA2_BARRIERS_MATCH_GODOT) ctx->config.barrier_api = enhanced ? SA2_BARRIERS_ENHANCED : SA2_BARRIERS_LEGACY;
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
    ctx->ready = fence(ctx, L"SA2 ready fence");
    ctx->free = fence(ctx, L"SA2 free fence");
    ctx->godot_drain = fence(ctx, L"SA2 Godot drain fence");
    if (config.queue_mode == SA2_QUEUE_OWN) {
        D3D12_COMMAND_QUEUE_DESC desc{};
        desc.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
        check_hr(ctx->device.Get(), ctx->device->CreateCommandQueue(&desc, IID_PPV_ARGS(&ctx->own_queue)), "CreateCommandQueue");
        name(ctx, ctx->own_queue.Get(), L"SA2 own queue");
        ctx->own_drain = fence(ctx, L"SA2 own drain fence");
    }
    D3D12_DESCRIPTOR_HEAP_DESC heap{};
    heap.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
    heap.NumDescriptors = SA2_RING_SLOTS;
    check_hr(ctx->device.Get(), ctx->device->CreateDescriptorHeap(&heap, IID_PPV_ARGS(&ctx->rtv_heap)), "CreateDescriptorHeap");
    name(ctx, ctx->rtv_heap.Get(), L"SA2 RTV heap");
    ctx->rtv_step = ctx->device->GetDescriptorHandleIncrementSize(heap.Type);
    for (uint32_t i = 0; i < SA2_RING_SLOTS; ++i) command_list(ctx, ctx->slots[i], L"SA2 slot " + std::to_wstring(i));
}
// The debug callback counts into user's Debug (a recovery's new device counts
// into the context it recovers).
void register_callback(sa2_context* ctx, sa2_context* user) {
    if (!ctx->config.debug_callback) return;
    const auto hr = ctx->device.As(&ctx->info_queue);
    if (FAILED(hr) && (hr != E_NOINTERFACE || removed(ctx))) check_hr(ctx->device.Get(), hr, "QueryInterface(ID3D12InfoQueue1)");
    if (ctx->info_queue) {
        check_hr(ctx->device.Get(), ctx->info_queue->RegisterMessageCallback(debug_callback, D3D12_MESSAGE_CALLBACK_IGNORE_FILTERS, user, &ctx->callback_cookie), "RegisterMessageCallback");
        ctx->callback_registered = true;
    }
}
void unregister_callback(sa2_context* ctx) noexcept {
    if (ctx->callback_registered) ctx->info_queue->UnregisterMessageCallback(ctx->callback_cookie); // the result is not needed: the queue goes with its device
    ctx->callback_registered = false;
}

// Route B (accepted for now by the owner on 18 Nov 2026, pending three
// owner-machine checks). The core's own device, made as c9pp.cpp makes it:
// the D3D12 device factory, told not to store the device as the process's
// singleton, so it is never Godot's device; without the factory the call is
// unsupported (no D3D12CreateDevice fallback, which could return Godot's).
// luid 0: the first hardware adapter. An adapter the LUID does not name
// answers missing.
void open_device(sa2_context* ctx, uint64_t luid, int32_t missing) {
    if (!ctx->factory || !ctx->factory->IsCurrent()) {
        ctx->factory.Reset();
        check_hr(nullptr, CreateDXGIFactory2(0, IID_PPV_ARGS(&ctx->factory)), "CreateDXGIFactory2");
    }
    ComPtr<IDXGIAdapter1> adapter;
    if (luid) {
        const LUID id{static_cast<DWORD>(luid & 0xFFFFFFFFu), static_cast<LONG>(static_cast<uint32_t>(luid >> 32))};
        const auto hr = ctx->factory->EnumAdapterByLuid(id, IID_PPV_ARGS(&adapter));
        if (hr == DXGI_ERROR_NOT_FOUND || hr == E_INVALIDARG) throw Failure(missing, hr_text("EnumAdapterByLuid", hr));
        check_hr(nullptr, hr, "EnumAdapterByLuid");
    } else {
        for (UINT i = 0; !adapter; ++i) {
            ComPtr<IDXGIAdapter1> candidate;
            const auto hr = ctx->factory->EnumAdapters1(i, &candidate);
            if (hr == DXGI_ERROR_NOT_FOUND) break;
            check_hr(nullptr, hr, "EnumAdapters1");
            DXGI_ADAPTER_DESC1 desc{};
            check_hr(nullptr, candidate->GetDesc1(&desc), "IDXGIAdapter1::GetDesc1");
            if (!(desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE)) adapter = candidate;
        }
        require(adapter != nullptr, "no hardware adapter", SA2_E_UNSUPPORTED);
    }
    DXGI_ADAPTER_DESC1 desc{};
    check_hr(nullptr, adapter->GetDesc1(&desc), "IDXGIAdapter1::GetDesc1");
    ctx->adapter = desc.AdapterLuid;
    const HMODULE module = GetModuleHandleW(L"d3d12.dll");
    const auto get = module ? reinterpret_cast<PFN_D3D12_GET_INTERFACE>(reinterpret_cast<void*>(GetProcAddress(module, "D3D12GetInterface"))) : nullptr;
    ComPtr<ID3D12DeviceFactory> factory;
    const HRESULT got = get ? get(ClsidDeviceFactory, IID_PPV_ARGS(&factory)) : HRESULT_FROM_WIN32(ERROR_PROC_NOT_FOUND);
    if (FAILED(got)) throw Failure(SA2_E_UNSUPPORTED, hr_text("D3D12GetInterface(CLSID_D3D12DeviceFactory)", got));
    check_hr(nullptr, factory->SetFlags(D3D12_DEVICE_FACTORY_FLAG_DISALLOW_STORING_NEW_DEVICE_AS_SINGLETON), "ID3D12DeviceFactory::SetFlags");
    check_hr(nullptr, factory->CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&ctx->device)), "ID3D12DeviceFactory::CreateDevice");
    D3D12_COMMAND_QUEUE_DESC queue{};
    queue.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    check_hr(ctx->device.Get(), ctx->device->CreateCommandQueue(&queue, IID_PPV_ARGS(&ctx->godot_queue)), "CreateCommandQueue");
    name(ctx, ctx->godot_queue.Get(), L"SA2 core queue");
}
uint64_t luid_value(LUID luid) {
    return (uint64_t(static_cast<uint32_t>(luid.HighPart)) << 32) | luid.LowPart;
}
// The ring-slot calls on a sa2_create context, after their argument checks.
void slot_route(sa2_context* ctx) {
    require(!ctx->created, "a sa2_create context draws on its surface and has no ring slots (route B)", SA2_E_WRONG_STATE);
}
// The swap chain's back buffers as the surface's render targets, with the
// ring slots' RTVs (c9pp.cpp create_rtvs).
void acquire_buffers(sa2_context* ctx, Chain& chain, uint32_t width, uint32_t height) {
    for (uint32_t i = 0; i < SA2_SURFACE_BUFFERS; ++i) {
        auto& buffer = chain.buffers[i];
        ComPtr<ID3D12Resource> resource;
        check_hr(ctx->device.Get(), chain.swapchain->GetBuffer(i, IID_PPV_ARGS(&resource)), "IDXGISwapChain::GetBuffer");
        name(ctx, resource.Get(), (L"SA2 surface buffer " + std::to_wstring(i)).c_str());
        ctx->device->CreateRenderTargetView(resource.Get(), nullptr, rtv(ctx, i));
        buffer.resource = resource.Get();
        buffer.external_reference = std::move(resource);
        buffer.width = width; buffer.height = height;
        buffer.pending_fence = nullptr;
        buffer.pending_value = buffer.last_shown = 0;
    }
}
void release_buffers(Chain& chain) {
    for (auto& buffer : chain.buffers) {
        buffer.resource = nullptr;
        buffer.external_reference.Reset();
        buffer.readback.Reset();
        buffer.pending_fence = nullptr;
        buffer.pending_value = 0;
    }
}
// c9pp.cpp create_swapchain, on the producer queue.
std::unique_ptr<Chain> make_chain(sa2_context* ctx, uint32_t width, uint32_t height) {
    auto chain = std::make_unique<Chain>();
    for (uint32_t i = 0; i < SA2_SURFACE_BUFFERS; ++i) command_list(ctx, chain->buffers[i], L"SA2 surface buffer " + std::to_wstring(i));
    DXGI_SWAP_CHAIN_DESC1 desc{};
    desc.Width = width;
    desc.Height = height;
    desc.Format = SurfaceFormat;
    desc.SampleDesc.Count = 1;
    desc.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    desc.BufferCount = SA2_SURFACE_BUFFERS;
    desc.Scaling = DXGI_SCALING_STRETCH;
    desc.SwapEffect = DXGI_SWAP_EFFECT_FLIP_DISCARD;
    desc.AlphaMode = DXGI_ALPHA_MODE_IGNORE;
    desc.Flags = SurfaceFlags;
    ComPtr<IDXGISwapChain1> first;
    check_hr(ctx->device.Get(), ctx->factory->CreateSwapChainForComposition(ctx->producer(), &desc, nullptr, &first), "CreateSwapChainForComposition");
    check_hr(ctx->device.Get(), first.As(&chain->swapchain), "QueryInterface(IDXGISwapChain3)");
    check_hr(ctx->device.Get(), chain->swapchain->SetMaximumFrameLatency(SurfaceLatency), "SetMaximumFrameLatency");
    chain->waitable = chain->swapchain->GetFrameLatencyWaitableObject();
    require(chain->waitable != nullptr, "GetFrameLatencyWaitableObject returned NULL", SA2_E_D3D12);
    acquire_buffers(ctx, *chain, width, height);
    return chain;
}
// c9pp.cpp create_dcomp: a DirectComposition device, a target for the window
// beneath its topmost content, and one visual at the surface's offset showing
// the swap chain. The surface takes the objects only when all succeeded.
void compose(sa2_context* ctx, Surface& surface, IDXGISwapChain3* swapchain) {
    ComPtr<IDCompositionDesktopDevice> dcomp;
    ComPtr<IDCompositionTarget> target;
    ComPtr<IDCompositionVisual2> visual;
    auto* device = ctx->device.Get();
    check_hr(device, DCompositionCreateDevice3(nullptr, IID_PPV_ARGS(&dcomp)), "DCompositionCreateDevice3");
    check_hr(device, dcomp->CreateTargetForHwnd(surface.hwnd, FALSE, &target), "CreateTargetForHwnd(hwnd, FALSE)");
    check_hr(device, dcomp->CreateVisual(&visual), "CreateVisual");
    check_hr(device, visual->SetContent(swapchain), "IDCompositionVisual::SetContent");
    check_hr(device, visual->SetOffsetX(static_cast<float>(surface.x)), "IDCompositionVisual::SetOffsetX");
    check_hr(device, visual->SetOffsetY(static_cast<float>(surface.y)), "IDCompositionVisual::SetOffsetY");
    check_hr(device, target->SetRoot(visual.Get()), "IDCompositionTarget::SetRoot");
    check_hr(device, dcomp->Commit(), "IDCompositionDevice::Commit");
    surface.dcomp = std::move(dcomp);
    surface.target = std::move(target);
    surface.visual = std::move(visual);
}
// The size a scene loads at: the attached surface's on a sa2_create context,
// else the complete three-slot ring's.
std::pair<uint32_t, uint32_t> load_size(sa2_context* ctx) {
    if (ctx->created) {
        const auto& surface = attached(ctx);
        return {surface.width, surface.height};
    }
    const auto& first = slot_at(ctx, 0);
    for (const auto& slot : ctx->slots) {
        require(slot.resource != nullptr, "register the complete three-slot ring before load", SA2_E_WRONG_STATE);
        require(slot.width == first.width && slot.height == first.height, "scene slots must be equally sized");
    }
    return {first.width, first.height};
}
template<class Config> void load_scene(sa2_context* ctx, const Config& config) {
    require(!ctx->scene, "a scene is already loaded", SA2_E_WRONG_STATE);
    const auto [width, height] = load_size(ctx);
    live(ctx);
    if (!ctx->created) {
        ctx->scene = std::make_unique<sa2::Scene>(ctx->device.Get(), config, width, height);
        return;
    }
    SceneSource source;
    source.keep(config);
    auto scene = source.load(ctx->device.Get(), width, height);
    scene->product.set_device_epoch(ctx->device_epoch);
    ctx->scene = std::move(scene);
    ctx->source = std::move(source);
}
// sa2_resize on a sa2_create context (c9pp.cpp c9pp_resize): the swap chain's
// buffers, then, with a loaded scene, its size-bound resources.
void resize_surface(sa2_context* ctx, uint32_t width, uint32_t height) {
    require(width <= SA2_SURFACE_MAX_PX && height <= SA2_SURFACE_MAX_PX, "width and height must be at most SA2_SURFACE_MAX_PX");
    auto& surface = attached(ctx);
    sa2::Scene* scene = ctx->scene ? &product_scene(ctx) : nullptr;
    require_drained(ctx);
    live(ctx);
    auto& chain = *surface.chain;
    release_buffers(chain);
    const auto hr = chain.swapchain->ResizeBuffers(SA2_SURFACE_BUFFERS, width, height, SurfaceFormat, SurfaceFlags);
    if (FAILED(hr)) {
        try { acquire_buffers(ctx, chain, surface.width, surface.height); } catch (...) {} // a frame then answers SA2_E_WRONG_STATE
        check_hr(ctx->device.Get(), hr, "IDXGISwapChain::ResizeBuffers");
    }
    surface.width = width;
    surface.height = height;
    surface.generation = ++ctx->surface_generation;
    acquire_buffers(ctx, chain, width, height);
    if (scene) {
        scene->resize(width, height);
        scene->product.resized();
    }
}
// sa2_recover on a sa2_create context (route B; accepted for now by the owner
// on 18 Nov 2026, pending three owner-machine checks). c9pp.cpp c9pp_recover's
// order, made transactional: the new device, queue, swap chain and scene are
// built first, and the old device's objects are released only when all of
// them exist, so a failed recovery leaves the context removed and retryable.
int32_t recover(sa2_context* ctx, char* buffer, uint32_t buffer_size, uint32_t* out_size) {
    const int32_t reason = removed_reason(ctx);
    if (reason >= 0) {
        write_json(sa2::recover_result("not_removed", "the device is not removed; nothing to recover", ctx->device_epoch, 0).dump(), buffer, buffer_size, out_size);
        throw Failure(SA2_E_WRONG_STATE, "the device is not removed; nothing to recover");
    }
    const uint64_t epoch = ctx->device_epoch + 1;
    const auto success = sa2::recover_result("recovered", "recovered on a new device of the same adapter (RPI.46)", epoch, reason).dump();
    if (buffer_size < success.size() + 1) write_json(success, buffer, buffer_size, out_size); // SA2_E_INVALID_ARGUMENT before any change
    if (ctx->scene) require(ctx->scene->product.removed() && ctx->scene->product.device_epoch() == ctx->device_epoch,
        "the scene's device epoch differs from the context's", SA2_E_WRONG_STATE);
    auto fresh = std::make_unique<sa2_context>();
    std::unique_ptr<Chain> chain;
    std::unique_ptr<sa2::Scene> scene;
    try {
        auto* surface = ctx->surface.get();
        if (surface && surface->visual && surface->dcomp) {
            surface->visual->SetContent(nullptr); // the old swap chain's device is gone (c9pp.cpp); results are not needed
            surface->dcomp->Commit();
        }
        fresh->created = true;
        fresh->factory = ctx->factory;
        open_device(fresh.get(), luid_value(ctx->adapter), SA2_E_D3D12);
        setup(fresh.get(), ctx->requested);
        register_callback(fresh.get(), ctx);
        if (surface) chain = make_chain(fresh.get(), surface->width, surface->height);
        if (ctx->scene) scene = ctx->source.load(fresh->device.Get(), ctx->scene->record.width, ctx->scene->record.height);
        if (surface) {
            HRESULT hr = surface->visual && surface->dcomp ? surface->visual->SetContent(chain->swapchain.Get()) : E_POINTER;
            if (SUCCEEDED(hr)) hr = surface->dcomp->Commit();
            if (FAILED(hr)) {
                // The kept DirectComposition objects are unusable: rebuild them
                // on the same window and rectangle (c9pp.cpp).
                surface->visual.Reset();
                surface->target.Reset();
                surface->dcomp.Reset();
                compose(fresh.get(), *surface, chain->swapchain.Get());
            }
        }
    } catch (const std::exception& error) {
        unregister_callback(fresh.get());
        try {
            write_json(sa2::recover_result("failed", error.what(), ctx->device_epoch, reason).dump(), buffer, buffer_size, out_size);
        } catch (const std::exception&) {} // a buffer too small for the failure: its size is in *out_size
        throw;
    }
    // Commit; nothing below throws. The old device's objects move into fresh
    // and are released with it.
    unregister_callback(ctx);
    std::swap(ctx->config, fresh->config);
    std::swap(ctx->device, fresh->device);
    std::swap(ctx->device10, fresh->device10);
    std::swap(ctx->godot_queue, fresh->godot_queue);
    std::swap(ctx->own_queue, fresh->own_queue);
    std::swap(ctx->ready, fresh->ready);
    std::swap(ctx->free, fresh->free);
    std::swap(ctx->godot_drain, fresh->godot_drain);
    std::swap(ctx->own_drain, fresh->own_drain);
    std::swap(ctx->rtv_heap, fresh->rtv_heap);
    std::swap(ctx->info_queue, fresh->info_queue);
    std::swap(ctx->slots, fresh->slots);
    std::swap(ctx->completion, fresh->completion);
    std::swap(ctx->rtv_step, fresh->rtv_step);
    std::swap(ctx->callback_cookie, fresh->callback_cookie);
    std::swap(ctx->callback_registered, fresh->callback_registered);
    std::swap(ctx->factory, fresh->factory);
    ctx->adapter = fresh->adapter;
    ctx->drained = false;
    std::swap(ctx->stages, fresh->stages); // C9 U5: the old device's stages leave with it; the new scene has no chain
    if (ctx->surface) {
        ctx->surface->chain = std::move(chain);
        ctx->surface->generation = ++ctx->surface_generation;
    }
    if (ctx->scene) {
        scene->product = std::move(ctx->scene->product);
        scene->product.recovered(epoch);
        ctx->scene = std::move(scene);
    }
    ctx->device_epoch = epoch;
    write_json(success, buffer, buffer_size, out_size);
    return SA2_OK;
}
// C9, owner item 9 (20 Nov 2026, no ABI change): one sample of the surface's
// frame statistics for the traced scene frames, read as sa2_presented reads
// them, while the trace runs or a traced frame still awaits its row. A failed
// or disjoint call adds no row; it never fails the frame.
void surface_statistics(sa2_context* ctx, sa2::Scene& scene) {
    if (!ctx->surface || !ctx->surface->chain || (!scene.record.tracing() && !scene.surface_trace.awaiting())) return;
    const auto& surface = *ctx->surface;
    DXGI_FRAME_STATISTICS statistics{};
    const HRESULT hr = surface.chain->swapchain->GetFrameStatistics(&statistics);
    UINT last = 0;
    const HRESULT last_hr = surface.chain->swapchain->GetLastPresentCount(&last);
    if (FAILED(hr) || FAILED(last_hr)) return;
    scene.surface_trace.sample({static_cast<int32_t>(hr), surface.generation, statistics.PresentCount, last, statistics.SyncQPCTime.QuadPart});
}
// C9 (26 Nov 2026, no ABI change): one sample of the surface's frame
// statistics for the product's present ledger, read as sa2_presented reads
// them, from inside a product frame's frame-latency wait (sa2::latency_wait).
// A failed or disjoint call adds no row and never fails the frame; a device
// removal is left to the calls that latch it.
void product_statistics(sa2_context* ctx, sa2::Scene& scene) {
    if (!ctx->surface || !ctx->surface->chain) return;
    const auto& surface = *ctx->surface;
    DXGI_FRAME_STATISTICS statistics{};
    const HRESULT hr = surface.chain->swapchain->GetFrameStatistics(&statistics);
    UINT last = 0;
    const HRESULT last_hr = surface.chain->swapchain->GetLastPresentCount(&last);
    if (FAILED(hr) || FAILED(last_hr)) return;
    scene.product.presents().sample({static_cast<int32_t>(hr), surface.generation, statistics.PresentCount, last, statistics.SyncQPCTime.QuadPart});
}
// C9 U4 (no ABI change): sa2_scene_produce with SA2_TARGET_SURFACE on a
// sa2_create context draws the loaded W1..W4 or W-J scene into the attached
// surface's current back buffer and presents it (SurfaceTarget), so the
// renderer gate's scenes run on route B. The frame's QPC is read after the
// target's waits, as the slot route reads it after its own checks.
void scene_surface(sa2_context* ctx, uint64_t frame) {
    require(ctx->created, "only a sa2_create context has a surface", SA2_E_WRONG_STATE);
    auto& scene = *ctx->scene;
    auto& surface = attached(ctx);
    require(surface.width == scene.record.width && surface.height == scene.record.height,
        "the surface is not at the scene's load-time size", SA2_E_WRONG_STATE);
    require(frame > ctx->last_frame, "frame must increase strictly from 1", SA2_E_WRONG_STATE);
    scene.require_healthy();
    live(ctx);
    scene.wait_geometry(ctx->config.wait_timeout_ms);
    scene.surface_ready();
    SurfaceTarget target(ctx);
    auto* list = target.open();
    const size_t traced = scene.record.trace.size();
    try {
        const auto now = sa2::scene_qpc();
        scene.record_draw(list, target.ring(), target.view(), now, target.format());
        scene.stage_frame(target.ring(), frame);
        scene.sample_vram();
        target.close(frame);
    } catch (...) { scene.poison(); throw; }
    // C9, owner item 9: a traced frame whose own present count was read awaits
    // the core's frame statistics; a frame without a count is never confirmed.
    if (scene.record.trace.size() > traced && target.present_count())
        scene.surface_trace.presented(traced, target.generation(), *target.present_count());
    surface_statistics(ctx, scene);
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
        validate_config(out, config);
        auto ctx = std::make_unique<sa2_context>();
        ctx->device = interface_handle<ID3D12Device>(device);
        ctx->godot_queue = interface_handle<ID3D12CommandQueue>(queue, ctx->device.Get());
        require(ctx->godot_queue->GetDesc().Type == D3D12_COMMAND_LIST_TYPE_DIRECT, "queue must be DIRECT");
        ComPtr<ID3D12Device> queue_device;
        check_hr(ctx->device.Get(), ctx->godot_queue->GetDevice(IID_PPV_ARGS(&queue_device)), "ID3D12CommandQueue::GetDevice");
        require(same_identity(ctx->device.Get(), queue_device.Get(), ctx->device.Get()), "queue device differs from given device");
        setup(ctx.get(), *config);
        register_callback(ctx.get(), ctx.get());
        *out = ctx.release();
        return SA2_OK;
    });
}

// ABI 5, route B (accepted for now by the owner on 18 Nov 2026, pending three
// owner-machine checks).
int32_t sa2_create(uint64_t adapter_luid, const sa2_config* config, sa2_context** out) {
    if (out) *out = nullptr;
    return invoke(nullptr, "sa2_create", [&] {
        validate_config(out, config);
        auto ctx = std::make_unique<sa2_context>();
        ctx->created = true;
        ctx->requested = *config;
        open_device(ctx.get(), adapter_luid, SA2_E_INVALID_ARGUMENT);
        setup(ctx.get(), *config);
        register_callback(ctx.get(), ctx.get());
        *out = ctx.release();
        return SA2_OK;
    });
}

int32_t sa2_surface_attach(sa2_context* ctx, uint64_t hwnd, int32_t x, int32_t y, uint32_t width, uint32_t height) {
    return invoke(ctx, "sa2_surface_attach", [&] {
        context(ctx);
        const auto window = reinterpret_cast<HWND>(static_cast<uintptr_t>(hwnd));
        require(hwnd != 0 && IsWindow(window), "hwnd is not a window");
        require(width >= SA2_MIN_TEXTURE_PX && height >= SA2_MIN_TEXTURE_PX && width <= SA2_SURFACE_MAX_PX && height <= SA2_SURFACE_MAX_PX,
            "width and height must lie in SA2_MIN_TEXTURE_PX .. SA2_SURFACE_MAX_PX");
        require(ctx->created, "only a sa2_create context has a surface (route B)", SA2_E_WRONG_STATE);
        require(!ctx->surface, "a surface is already attached", SA2_E_WRONG_STATE);
        live(ctx);
        auto surface = std::make_unique<Surface>();
        surface->hwnd = window;
        surface->x = x; surface->y = y;
        surface->width = width; surface->height = height;
        surface->chain = make_chain(ctx, width, height);
        compose(ctx, *surface, surface->chain->swapchain.Get());
        surface->generation = ++ctx->surface_generation;
        ctx->surface = std::move(surface);
        return SA2_OK;
    });
}

int32_t sa2_surface_move(sa2_context* ctx, int32_t x, int32_t y) {
    return invoke(ctx, "sa2_surface_move", [&] {
        context(ctx);
        require(ctx->surface != nullptr, "no surface is attached", SA2_E_WRONG_STATE);
        auto& surface = *ctx->surface;
        require(surface.visual && surface.dcomp, "the surface's composition objects are gone; sa2_recover rebuilds them", SA2_E_WRONG_STATE);
        // No GPU work, so no device check: it works during device loss.
        check_hr(nullptr, surface.visual->SetOffsetX(static_cast<float>(x)), "IDCompositionVisual::SetOffsetX");
        check_hr(nullptr, surface.visual->SetOffsetY(static_cast<float>(y)), "IDCompositionVisual::SetOffsetY");
        check_hr(nullptr, surface.dcomp->Commit(), "IDCompositionDevice::Commit");
        surface.x = x;
        surface.y = y;
        return SA2_OK;
    });
}

int32_t sa2_surface_swap_chain(sa2_context* ctx, uint64_t* out) {
    if (out) *out = 0;
    return invoke(ctx, "sa2_surface_swap_chain", [&] {
        context(ctx);
        require(out != nullptr, "null swap-chain output");
        require(ctx->surface != nullptr && ctx->surface->chain && ctx->surface->chain->swapchain, "no surface is attached",
                SA2_E_WRONG_STATE);
        // IDXGISwapChain3 derives singly from IDXGISwapChain, so this is also the IDXGISwapChain pointer.
        *out = static_cast<uint64_t>(reinterpret_cast<uintptr_t>(ctx->surface->chain->swapchain.Get()));
        return SA2_OK;
    });
}

int32_t sa2_surface_detach(sa2_context* ctx) {
    return invoke(ctx, "sa2_surface_detach", [&] {
        context(ctx);
        require(ctx->surface != nullptr, "no surface is attached", SA2_E_WRONG_STATE);
        require(!ctx->scene, "scene remains loaded", SA2_E_WRONG_STATE);
        require_drained(ctx);
        // As c9pp.cpp's detach: the results are not needed, since releasing
        // the target removes the visual tree in any case.
        auto& surface = *ctx->surface;
        if (surface.visual) surface.visual->SetContent(nullptr);
        if (surface.target) surface.target->SetRoot(nullptr);
        if (surface.dcomp) surface.dcomp->Commit();
        ctx->surface.reset();
        return SA2_OK;
    });
}

int32_t sa2_create_texture(sa2_context* ctx, uint32_t width, uint32_t height, int32_t initial_state, uint64_t* out_resource) {
    if (out_resource) *out_resource = 0;
    return invoke(ctx, "sa2_create_texture", [&] {
        context(ctx);
        require(out_resource && width >= 128 && height >= 128, "output is null or texture smaller than 128");
        const auto initial = state(initial_state);
        slot_route(ctx);
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
        slot_route(ctx);
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
        slot_route(ctx);
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
        slot_route(ctx);
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
        slot_route(ctx);
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
        require(!ctx->surface, "the surface remains attached", SA2_E_WRONG_STATE);
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
        load_scene(ctx, *config);
        return SA2_OK;
    });
}

int32_t sa2_scene_load_wj(sa2_context* ctx, const sa2_wj_config* config) {
    return invoke(ctx, "sa2_scene_load_wj", [&] {
        sa2::validate_wj_config(config);
        context(ctx);
        load_scene(ctx, *config);
        return SA2_OK;
    });
}

int32_t sa2_scene_produce(sa2_context* ctx, uint32_t index, uint64_t frame) {
    return invoke(ctx, "sa2_scene_produce", [&] {
        context(ctx);
        require((index < SA2_RING_SLOTS || index == SA2_TARGET_SURFACE) && frame != UINT64_MAX, "invalid slot or reserved fence value");
        require(ctx->scene != nullptr, "no loaded scene", SA2_E_WRONG_STATE);
        if (index == SA2_TARGET_SURFACE) {
            scene_surface(ctx, frame);
            return SA2_OK;
        }
        slot_route(ctx);
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
            scene.stage_frame(index, frame);
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
        // C9, owner item 9: one more sample for the last traced frames on the surface.
        if (ctx->created) surface_statistics(ctx, *ctx->scene);
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
        // C9, owner item 9: a route-B run (a sa2_create context, whose scene draws on its surface) also writes the
        // traced frames its own frame statistics confirmed, which the acceptance finalizer's blind-seconds rule reads.
        if (ctx->created) sa2::require_new_file(directory / "presented.jsonl");
        const bool jumbling = scene.record.jumbling();
        if (jumbling) sa2::require_new_file(directory / "pose_check.json");
        Json poses;
        const auto labels = scene.labels(poses);
        const auto native = scene.record.native(labels, sa2::scene_identity(), ctx->config.queue_mode, ctx->config.barrier_api, poses);
        sa2::write_new_file(directory / "trace.jsonl", scene.record.trace_text());
        sa2::write_new_file(directory / "native.json", native.dump() + '\n');
        if (ctx->created) sa2::write_new_file(directory / "presented.jsonl", scene.surface_trace.text());
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
        write_json(sa2::describe(build, removed_reason(ctx), ctx->device_epoch, ctx->created && ctx->surface != nullptr).dump(), buffer, buffer_size, out_size);
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
        std::optional<SlotTarget> slot;
        std::optional<SurfaceTarget> surface;
        if (input->target == SA2_TARGET_SURFACE) {
            surface.emplace(ctx);
            surface->sample_while_waiting([ctx, &scene] { product_statistics(ctx, scene); });
        } else {
            slot_route(ctx);
            slot.emplace(ctx, input->target);
        }
        sa2::FrameTarget& target = surface ? static_cast<sa2::FrameTarget&>(*surface) : *slot;
        require(target.width() == scene.record.width && target.height() == scene.record.height,
            "the frame target is not at the scene's size", SA2_E_WRONG_STATE);
        require(ctx->last_frame < UINT64_MAX - 1, "frame IDs are exhausted", SA2_E_WRONG_STATE);
        const uint64_t frame = ctx->last_frame + 1;
        const auto plan = scene.product.plan(*input, target.width(), target.height());
        scene.require_healthy();
        live(ctx);
        scene.wait_geometry(ctx->config.wait_timeout_ms);
        scene.product_ready(ctx->producer(), target.format(), plan.styled);
        if (plan.upload) target.wait(plan.upload_wait); // the copy's staging was last read by that frame
        auto* list = target.open();
        try {
            harvest(ctx, scene); // the target's previous frame completed: read its ring results before this frame reuses them
            scene.product_frame(list, *input, plan, target.view(), target.ring(), target.format());
            target.close(frame);
        } catch (...) { scene.poison(); throw; }
        scene.product.commit(*input, plan, frame, target.ring());
        if (surface && surface->present_count())
            scene.product.presents().presented(*input, frame, surface->generation(), *surface->present_count());
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
        if (!ctx->surface) throw Failure(SA2_E_UNAVAILABLE, "the core owns no swap chain and cannot confirm a presentation (RM.26)");
        // ABI 5 (route B; accepted for now by the owner on 18 Nov 2026,
        // pending three owner-machine checks): one statistics sample per call.
        auto& scene = product_scene(ctx);
        if (scene.product.removed()) throw Failure(SA2_E_DEVICE_REMOVED, "device removed (" + sa2::removed_text(scene.product.removed_reason()) + ")");
        const auto& surface = *ctx->surface;
        DXGI_FRAME_STATISTICS statistics{};
        const HRESULT hr = surface.chain->swapchain->GetFrameStatistics(&statistics);
        UINT last = 0;
        const HRESULT last_hr = surface.chain->swapchain->GetLastPresentCount(&last);
        if (FAILED(hr) || FAILED(last_hr)) {
            scene.product.observe_removal(ctx->device->GetDeviceRemovedReason());
            if (scene.product.removed()) throw Failure(SA2_E_DEVICE_REMOVED, "device removed (" + sa2::removed_text(scene.product.removed_reason()) + ")");
            return SA2_OK; // no row, such as DXGI_ERROR_FRAME_STATISTICS_DISJOINT: *out_count stays 0
        }
        scene.product.presents().sample(
            {static_cast<int32_t>(hr), surface.generation, statistics.PresentCount, last, statistics.SyncQPCTime.QuadPart});
        *out_count = scene.product.presents().rows(rows, capacity);
        return SA2_OK;
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
        if (ctx->created) return recover(ctx, buffer, buffer_size, out_size);
        write_json(sa2::recover_report(removed_reason(ctx)).dump(), buffer, buffer_size, out_size);
        throw Failure(SA2_E_UNAVAILABLE, "ABI 4 cannot recover a device: it owns no swap chain and no device (RPI.46)");
    });
}

int32_t sa2_resize(sa2_context* ctx, uint32_t width, uint32_t height) {
    return invoke(ctx, "sa2_resize", [&] {
        context(ctx);
        require(width >= SA2_MIN_TEXTURE_PX && height >= SA2_MIN_TEXTURE_PX, "width and height must be at least SA2_MIN_TEXTURE_PX");
        if (ctx->created) {
            resize_surface(ctx, width, height);
            return SA2_OK;
        }
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

// C9 U5 (additive in ABI 5): user shader stages (sa2_interop.h).
int32_t sa2_stage_compile(sa2_context* ctx, uint32_t kind, const char* hlsl_utf8, uint32_t hlsl_size, uint64_t* out_handle,
                          char* record, uint32_t record_size, uint32_t* out_record_size) {
    if (out_handle) *out_handle = 0;
    if (out_record_size) *out_record_size = 0;
    return invoke(ctx, "sa2_stage_compile", [&]() -> int32_t {
        context(ctx);
        require(out_handle && record && out_record_size, "null handle output, record buffer or record size output");
        require(record_size >= SA2_STAGE_RECORD_BYTES, "record_size must be at least SA2_STAGE_RECORD_BYTES");
        require(kind == SA2_STAGE_VERTEX || kind == SA2_STAGE_FRAGMENT, "kind must be SA2_STAGE_VERTEX or SA2_STAGE_FRAGMENT");
        const auto refusal = sa2::stages::text_refusal(hlsl_utf8, hlsl_size);
        require(refusal.empty(), refusal.c_str());
        live(ctx);
        if (!ctx->stages) ctx->stages = std::make_unique<sa2::stages::Registry>(ctx->device.Get());
        const uint64_t handle = ctx->stage_handles + 1;
        Json result;
        const int32_t status = ctx->stages->compile(kind, std::string_view(hlsl_utf8, hlsl_size), handle, result);
        const std::string error = result.at("error").string();
        write_json(sa2::stages::fit_record(std::move(result), record_size - 1), record, record_size, out_record_size);
        if (status != SA2_OK) throw Failure(status, std::string(sa2::stages::CompileFailed) + ": " + error);
        ctx->stage_handles = handle;
        *out_handle = handle;
        return SA2_OK;
    });
}

int32_t sa2_scene_stages(sa2_context* ctx, uint64_t vertex_stage, uint64_t fragment_stage, uint32_t overdraw,
                         const uint32_t* vertex_arrays, uint32_t vertex_count, const uint32_t* fragment_arrays, uint32_t fragment_count) {
    return invoke(ctx, "sa2_scene_stages", [&] {
        context(ctx);
        require(vertex_count <= SA2_STAGE_ARRAY_MAX && fragment_count <= SA2_STAGE_ARRAY_MAX, "an array count exceeds SA2_STAGE_ARRAY_MAX");
        require((vertex_arrays || !vertex_count) && (fragment_arrays || !fragment_count), "a NULL array with a nonzero count");
        require(ctx->scene != nullptr, "no loaded scene", SA2_E_WRONG_STATE);
        auto& scene = *ctx->scene;
        require(!scene.record.tracing(), "user stages cannot change inside a scene trace", SA2_E_WRONG_STATE);
        scene.require_healthy();
        live(ctx);
        if (!vertex_stage && !fragment_stage) {
            scene.stages_attach(nullptr, 0);
            return SA2_OK;
        }
        require(ctx->stages && (!vertex_stage || ctx->stages->holds(vertex_stage, SA2_STAGE_VERTEX)),
            "vertex_stage is neither 0 nor a compiled vertex stage of this device");
        require(ctx->stages && (!fragment_stage || ctx->stages->holds(fragment_stage, SA2_STAGE_FRAGMENT)),
            "fragment_stage is neither 0 nor a compiled fragment stage of this device");
        std::vector<uint32_t> vertex_values, fragment_values;
        if (vertex_count) vertex_values.assign(vertex_arrays, vertex_arrays + vertex_count);
        if (fragment_count) fragment_values.assign(fragment_arrays, fragment_arrays + fragment_count);
        auto chain = ctx->stages->chain(vertex_stage, fragment_stage, overdraw, std::move(vertex_values), std::move(fragment_values));
        scene.stages_attach(std::move(chain), sa2::scene_qpc());
        return SA2_OK;
    });
}

int32_t sa2_stage_report(sa2_context* ctx, sa2_stage_frame* out) {
    return invoke(ctx, "sa2_stage_report", [&] {
        context(ctx);
        require(out && out->struct_size == sizeof(*out), "invalid stage_frame struct_size");
        if (ctx->scene) ctx->scene->stage_report(completed(ctx->ready.Get()), *out);
        else {
            sa2_stage_frame zero{};
            zero.struct_size = sizeof(zero);
            *out = zero;
        }
        return SA2_OK;
    });
}

int32_t sa2_stage_release(sa2_context* ctx, uint64_t handle) {
    return invoke(ctx, "sa2_stage_release", [&] {
        context(ctx);
        require(ctx->stages && ctx->stages->holds(handle, 0), "handle is not a compiled stage of this device");
        require(!(ctx->scene && ctx->scene->stages_use(handle)), "the stage is attached to the scene; detach it first", SA2_E_WRONG_STATE);
        ctx->stages->release(handle);
        return SA2_OK;
    });
}

// ABI 6: a view's shape (sa2_interop.h). Device-free product state, so it works during device loss.
int32_t sa2_view_shape_set(sa2_context* ctx, uint64_t view, const sa2_view_shape* shape) {
    return invoke(ctx, "sa2_view_shape_set", [&] {
        context(ctx);
        const auto value = sa2::view_shape(shape);
        auto& scene = product_scene(ctx);
        scene.product.set_shape(view, value);
        return SA2_OK;
    });
}

// ABI 7: per-piece style (sa2_interop.h). Device-free product state, so both work during device loss.
int32_t sa2_style_set(sa2_context* ctx, uint64_t revision, const uint8_t* piece_style, uint32_t channel) {
    return invoke(ctx, "sa2_style_set", [&] {
        context(ctx);
        sa2::validate_piece_style(piece_style, channel);
        auto& scene = product_scene(ctx);
        scene.product.set_style(revision, piece_style, channel);
        return SA2_OK;
    });
}
int32_t sa2_view_style_set(sa2_context* ctx, uint32_t slot, const sa2_view_style* style) {
    return invoke(ctx, "sa2_view_style_set", [&] {
        context(ctx);
        require(slot >= 1 && slot <= SA2_STYLE_SLOTS, "a style slot is 1..SA2_STYLE_SLOTS");
        if (style) sa2::view_style(style);
        auto& scene = product_scene(ctx);
        scene.product.set_view_style(slot, style);
        return SA2_OK;
    });
}
}
