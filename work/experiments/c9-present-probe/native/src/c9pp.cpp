// c9pp.dll: route-B confirmed-presentation probe core. See ../c9pp.h for the ABI.
#include "../c9pp.h"

#include <windows.h>
#include <d3d12.h>
#include <dcomp.h>
#include <dxgi1_6.h>
#include <wrl/client.h>

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <new>
#include <string>
#include <string_view>
#include <vector>

#include "code.h"

using Microsoft::WRL::ComPtr;

static_assert(sizeof(c9pp_attach_desc) == 72, "c9pp_attach_desc layout");
static_assert(sizeof(c9pp_attach_info) == 48, "c9pp_attach_info layout");
static_assert(sizeof(c9pp_frame_record) == 40, "c9pp_frame_record layout");
static_assert(sizeof(c9pp_stats_row) == 48, "c9pp_stats_row layout");
static_assert(sizeof(c9pp_sample_desc) == 36, "c9pp_sample_desc layout");
static_assert(sizeof(c9pp_screen_sample) == 88, "c9pp_screen_sample layout");
static_assert(sizeof(c9pp_decode_pair) == 48, "c9pp_decode_pair layout");

namespace {

constexpr UINT kBuffers = 3;
constexpr DXGI_FORMAT kFormat = DXGI_FORMAT_B8G8R8A8_UNORM;
constexpr UINT kSwapFlags = DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT;
// Copied from d3d12.h so that dxguid.lib is not needed.
constexpr GUID kClsidDeviceFactory = {0x114863bf, 0xc386, 0x4aee, {0xb3, 0x9d, 0x8f, 0x0b, 0xbb, 0x06, 0x29, 0x55}};
constexpr GUID kClsidD3D12Debug = {0xf2352aeb, 0xdd84, 0x49fe, {0xb9, 0x7b, 0xa9, 0xdc, 0xfd, 0xcc, 0x1b, 0x4f}};

struct Failure {
    int32_t code;
    std::string text;
};

[[noreturn]] void fail(int32_t code, std::string text) { throw Failure{code, std::move(text)}; }

std::string hex(HRESULT hr) {
    char buffer[16];
    std::snprintf(buffer, sizeof(buffer), "0x%08lX", static_cast<unsigned long>(hr));
    return buffer;
}

bool removed_hr(HRESULT hr) {
    return hr == DXGI_ERROR_DEVICE_REMOVED || hr == DXGI_ERROR_DEVICE_RESET || hr == DXGI_ERROR_DEVICE_HUNG;
}

void check(HRESULT hr, const char* what) {
    if (FAILED(hr))
        fail(removed_hr(hr) ? C9PP_E_DEVICE_REMOVED : C9PP_E_WIN32, std::string(what) + " failed: " + hex(hr));
}

void require(bool ok, const char* text, int32_t code = C9PP_E_INVALID_ARGUMENT) {
    if (!ok) fail(code, text);
}

uint64_t qpc() {
    LARGE_INTEGER value;
    QueryPerformanceCounter(&value);
    return static_cast<uint64_t>(value.QuadPart);
}

int chroma(uint32_t rgb) {
    const int r = static_cast<int>(rgb >> 16 & 255), g = static_cast<int>(rgb >> 8 & 255), b = static_cast<int>(rgb & 255);
    return std::max({r, g, b}) - std::min({r, g, b});
}

// One JSON object per line. Keys are fixed ASCII; string values are escaped.
class Row {
public:
    explicit Row(std::string_view type) {
        text_.reserve(384);
        text_ += '{';
        str("t", type);
    }
    Row& str(const char* key, std::string_view value) {
        name(key);
        text_ += '"';
        for (char ch : value) {
            const auto u = static_cast<unsigned char>(ch);
            if (ch == '"' || ch == '\\') {
                text_ += '\\';
                text_ += ch;
            } else if (u < 0x20) {
                char esc[8];
                std::snprintf(esc, sizeof(esc), "\\u%04x", u);
                text_ += esc;
            } else {
                text_ += ch;
            }
        }
        text_ += '"';
        return *this;
    }
    Row& num(const char* key, int64_t value) {
        name(key);
        text_ += std::to_string(value);
        return *this;
    }
    Row& unum(const char* key, uint64_t value) {
        name(key);
        text_ += std::to_string(value);
        return *this;
    }
    Row& hr(const char* key, HRESULT value) { return num(key, static_cast<int32_t>(value)); }
    Row& real(const char* key, double value) {
        name(key);
        if (!std::isfinite(value)) {
            text_ += "null";
        } else {
            char buffer[40];
            std::snprintf(buffer, sizeof(buffer), "%.9g", value);
            text_ += buffer;
        }
        return *this;
    }
    Row& flag(const char* key, bool value) {
        name(key);
        text_ += value ? "true" : "false";
        return *this;
    }
    Row& raw(const char* key, std::string_view json) {
        name(key);
        text_ += json;
        return *this;
    }
    Row& rgb(const char* key, uint32_t value) {
        name(key);
        text_ += '[' + std::to_string(value >> 16 & 255) + ',' + std::to_string(value >> 8 & 255) + ',' +
                 std::to_string(value & 255) + ']';
        return *this;
    }
    std::string finish() {
        text_ += "}\n";
        return std::move(text_);
    }

private:
    void name(const char* key) {
        if (text_.size() > 1) text_ += ',';
        text_ += '"';
        text_ += key;
        text_ += "\":";
    }
    std::string text_;
};

struct Context {
    DWORD thread = 0;
    HWND hwnd = nullptr;
    uint32_t flags = 0;
    uint64_t host_device = 0;
    LUID luid{};
    int32_t x = 0, y = 0;
    uint32_t w = 0, h = 0;
    uint32_t cell = 24, max_latency = 2, wait_ms = 100, base_rgb = 0;
    uint32_t epoch = 1, sc_gen = 1;
    std::string phase = "attach";
    std::string last_error;
    FILE* log = nullptr;
    uint64_t qpc_frequency = 0;

    ComPtr<IDXGIFactory4> factory;
    ComPtr<IDXGIAdapter1> adapter;
    ComPtr<ID3D12DeviceFactory> device_factory;
    ComPtr<ID3D12Device> device;
    int32_t device_source = 0, device_independent = -1;
    HRESULT factory_hr = S_OK;
    ComPtr<ID3D12CommandQueue> queue;
    ComPtr<ID3D12CommandAllocator> allocators[kBuffers];
    ComPtr<ID3D12GraphicsCommandList> list;
    ComPtr<ID3D12Fence> fence;
    HANDLE fence_event = nullptr;
    uint64_t fence_value = 0;
    uint64_t buffer_fence[kBuffers] = {};
    uint32_t frames_traced = 0;  // frames whose steps were logged
    ComPtr<ID3D12DescriptorHeap> rtv_heap;
    UINT rtv_stride = 0;
    ComPtr<IDXGISwapChain3> swapchain;
    HANDLE waitable = nullptr;
    ComPtr<ID3D12Resource> buffers[kBuffers];

    ComPtr<IDCompositionDesktopDevice> dcomp;
    ComPtr<IDCompositionTarget> target;
    ComPtr<IDCompositionVisual2> visual;
    HRESULT target_hr = S_OK;
    bool removal_requested = false;

    std::vector<c9::Rect> white, black;
    std::vector<D3D12_RECT> rects;

    ~Context() {
        release_d3d();
        visual.Reset();
        target.Reset();
        dcomp.Reset();
        if (fence_event) CloseHandle(fence_event);
        if (log) std::fclose(log);
    }

    void release_d3d() {
        for (auto& b : buffers) b.Reset();
        if (waitable) {
            CloseHandle(waitable);
            waitable = nullptr;
        }
        swapchain.Reset();
        rtv_heap.Reset();
        list.Reset();
        for (auto& a : allocators) a.Reset();
        fence.Reset();
        queue.Reset();
        device.Reset();
        device_factory.Reset();
        for (auto& v : buffer_fence) v = 0;
        fence_value = 0;
    }

    Row row(std::string_view type) const {
        Row r(type);
        r.unum("qpc", qpc()).str("phase", phase).unum("epoch", epoch);
        return r;
    }

    void emit(Row& r) {
        if (!log) return;
        const std::string line = r.finish();
        std::fwrite(line.data(), 1, line.size(), log);
        std::fflush(log);
    }
};

thread_local std::string g_error;

void copy_text(const std::string& text, char* buffer, uint32_t size) {
    if (!buffer || size == 0) return;
    const size_t n = std::min<size_t>(text.size(), size - 1);
    std::memcpy(buffer, text.data(), n);
    buffer[n] = 0;
}

template <class F>
int32_t global_call(const char* name, F&& body) {
    try {
        return body();
    } catch (const Failure& e) {
        g_error = std::string(name) + ": " + e.text;
        return e.code;
    } catch (const std::bad_alloc&) {
        g_error = std::string(name) + ": out of memory";
        return C9PP_E_WIN32;
    } catch (...) {
        g_error = std::string(name) + ": unexpected exception";
        return C9PP_E_WIN32;
    }
}

template <class F>
int32_t context_call(void* handle, const char* name, F&& body) {
    auto* c = static_cast<Context*>(handle);
    if (!c) {
        g_error = std::string(name) + ": context is NULL";
        return C9PP_E_INVALID_ARGUMENT;
    }
    if (c->thread != GetCurrentThreadId()) {
        g_error = std::string(name) + ": called from a thread other than the attaching thread";
        return C9PP_E_WRONG_THREAD;
    }
    std::string text;
    int32_t code = C9PP_E_WIN32;
    try {
        return body(*c);
    } catch (const Failure& e) {
        text = e.text;
        code = e.code;
    } catch (const std::bad_alloc&) {
        text = "out of memory";
    } catch (...) {
        text = "unexpected exception";
    }
    c->last_error = std::string(name) + ": " + text;
    try {
        Row r = c->row("error");
        r.str("function", name).num("status", code).str("text", text);
        c->emit(r);
    } catch (...) {
    }
    return code;
}

void validate_rect(int32_t x, int32_t y, uint32_t w, uint32_t h, uint32_t cell) {
    require(x >= 0 && y >= 0, "rectangle origin must not be negative");
    require(w <= C9PP_MAX_EXTENT && h <= C9PP_MAX_EXTENT, "rectangle larger than C9PP_MAX_EXTENT");
    require(w >= c9::min_rect_extent(cell) && h >= c9::min_rect_extent(cell),
            "rectangle too small for two frame-code copies (2 * 9 * cell_px per side)");
}

uint32_t effective_cell(const c9pp_attach_desc& d) { return d.cell_px ? d.cell_px : 24u; }

void validate(const c9pp_attach_desc* d) {
    require(d != nullptr, "desc is NULL");
    require(d->struct_size == sizeof(c9pp_attach_desc), "desc.struct_size mismatch");
    require((d->flags & ~C9PP_ATTACH_KNOWN_FLAGS) == 0, "unknown attach flags");
    require(d->hwnd != 0, "hwnd is 0");
    require(IsWindow(reinterpret_cast<HWND>(d->hwnd)) != FALSE, "hwnd is not a window");
    const bool by_luid = (d->flags & C9PP_ATTACH_USE_LUID) != 0;
    require(by_luid ? d->adapter == 0 : d->adapter != 0, "give exactly one of adapter or C9PP_ATTACH_USE_LUID");
    require(!((d->flags & C9PP_ATTACH_DEBUG_LAYER) && (d->flags & C9PP_ATTACH_GLOBAL_DEVICE)),
            "the debug layer needs the private device factory; it cannot combine with C9PP_ATTACH_GLOBAL_DEVICE");
    const uint32_t cell = effective_cell(*d);
    require(cell >= C9PP_MIN_CELL_PX && cell <= C9PP_MAX_CELL_PX, "cell_px out of range");
    require(d->max_latency <= 3, "max_latency out of range (0..3)");
    require(d->wait_timeout_ms <= 1000, "wait_timeout_ms out of range (0..1000)");
    require(d->base_rgb <= 0xFFFFFFu, "base_rgb has bits above 0xFFFFFF");
    require(chroma(d->base_rgb) > 48, "base_rgb must be chromatic so that a missing code never decodes");
    validate_rect(d->x, d->y, d->width, d->height, cell);
}

int32_t independence(Context& c) {
    if (!c.host_device) return -1;
    ComPtr<IUnknown> host, mine;
    check(reinterpret_cast<IUnknown*>(c.host_device)->QueryInterface(IID_PPV_ARGS(&host)), "host device QueryInterface");
    check(c.device.As(&mine), "core device QueryInterface");
    return host.Get() == mine.Get() ? 0 : 1;
}

void create_device(Context& c) {
    c.device.Reset();
    c.device_factory.Reset();
    c.factory_hr = S_OK;
    c.device_source = 0;
    if (!(c.flags & C9PP_ATTACH_GLOBAL_DEVICE)) {
        HRESULT hr = HRESULT_FROM_WIN32(ERROR_PROC_NOT_FOUND);
        const HMODULE module = GetModuleHandleW(L"d3d12.dll");
        const auto get = module ? reinterpret_cast<PFN_D3D12_GET_INTERFACE>(
                                      reinterpret_cast<void*>(GetProcAddress(module, "D3D12GetInterface")))
                                : nullptr;
        if (get) hr = get(kClsidDeviceFactory, IID_PPV_ARGS(&c.device_factory));
        if (SUCCEEDED(hr) && (c.flags & C9PP_ATTACH_DEBUG_LAYER)) {
            ComPtr<ID3D12Debug> debug;
            hr = c.device_factory->GetConfigurationInterface(kClsidD3D12Debug, IID_PPV_ARGS(&debug));
            if (SUCCEEDED(hr)) debug->EnableDebugLayer();
        }
        if (SUCCEEDED(hr)) hr = c.device_factory->SetFlags(D3D12_DEVICE_FACTORY_FLAG_DISALLOW_STORING_NEW_DEVICE_AS_SINGLETON);
        if (SUCCEEDED(hr)) hr = c.device_factory->CreateDevice(c.adapter.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&c.device));
        c.factory_hr = hr;
        if (SUCCEEDED(hr)) {
            c.device_source = C9PP_DEVICE_FACTORY;
        } else {
            c.device.Reset();
            c.device_factory.Reset();
        }
    }
    if (!c.device) {
        require(!(c.flags & C9PP_ATTACH_DEBUG_LAYER), "the debug layer needs the device factory, which is unavailable",
                C9PP_E_UNSUPPORTED);
        check(D3D12CreateDevice(c.adapter.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&c.device)), "D3D12CreateDevice");
        c.device_source = C9PP_DEVICE_GLOBAL;
    }
    c.device_independent = independence(c);
}

void create_d3d_objects(Context& c) {
    D3D12_COMMAND_QUEUE_DESC qd{};
    qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    check(c.device->CreateCommandQueue(&qd, IID_PPV_ARGS(&c.queue)), "CreateCommandQueue");
    for (auto& a : c.allocators)
        check(c.device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&a)), "CreateCommandAllocator");
    check(c.device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, c.allocators[0].Get(), nullptr, IID_PPV_ARGS(&c.list)),
          "CreateCommandList");
    check(c.list->Close(), "ID3D12GraphicsCommandList::Close");
    check(c.device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&c.fence)), "CreateFence");
    if (!c.fence_event) {
        c.fence_event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        require(c.fence_event != nullptr, "CreateEvent failed", C9PP_E_WIN32);
    }
    D3D12_DESCRIPTOR_HEAP_DESC hd{};
    hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
    hd.NumDescriptors = kBuffers;
    check(c.device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&c.rtv_heap)), "CreateDescriptorHeap");
    c.rtv_stride = c.device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
}

void create_rtvs(Context& c) {
    D3D12_CPU_DESCRIPTOR_HANDLE handle = c.rtv_heap->GetCPUDescriptorHandleForHeapStart();
    for (UINT i = 0; i < kBuffers; ++i) {
        check(c.swapchain->GetBuffer(i, IID_PPV_ARGS(&c.buffers[i])), "IDXGISwapChain::GetBuffer");
        c.device->CreateRenderTargetView(c.buffers[i].Get(), nullptr, handle);
        handle.ptr += c.rtv_stride;
    }
}

void create_swapchain(Context& c) {
    DXGI_SWAP_CHAIN_DESC1 d{};
    d.Width = c.w;
    d.Height = c.h;
    d.Format = kFormat;
    d.SampleDesc.Count = 1;
    d.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    d.BufferCount = kBuffers;
    d.Scaling = DXGI_SCALING_STRETCH;
    d.SwapEffect = (c.flags & C9PP_ATTACH_FLIP_SEQUENTIAL) ? DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL : DXGI_SWAP_EFFECT_FLIP_DISCARD;
    d.AlphaMode = DXGI_ALPHA_MODE_IGNORE;
    d.Flags = kSwapFlags;
    ComPtr<IDXGISwapChain1> sc1;
    check(c.factory->CreateSwapChainForComposition(c.queue.Get(), &d, nullptr, &sc1), "CreateSwapChainForComposition");
    check(sc1.As(&c.swapchain), "IDXGISwapChain3 QueryInterface");
    check(c.swapchain->SetMaximumFrameLatency(c.max_latency), "SetMaximumFrameLatency");
    c.waitable = c.swapchain->GetFrameLatencyWaitableObject();
    require(c.waitable != nullptr, "GetFrameLatencyWaitableObject returned NULL", C9PP_E_WIN32);
    create_rtvs(c);
}

void create_dcomp(Context& c) {
    HRESULT hr = (c.flags & C9PP_ATTACH_DCOMP_DEVICE2) ? DCompositionCreateDevice2(nullptr, IID_PPV_ARGS(&c.dcomp))
                                                       : DCompositionCreateDevice3(nullptr, IID_PPV_ARGS(&c.dcomp));
    check(hr, "DCompositionCreateDevice");
    c.target_hr = c.dcomp->CreateTargetForHwnd(c.hwnd, FALSE, &c.target);
    check(c.target_hr, "CreateTargetForHwnd(hwnd, FALSE)");
    check(c.dcomp->CreateVisual(&c.visual), "CreateVisual");
    check(c.visual->SetContent(c.swapchain.Get()), "IDCompositionVisual::SetContent");
    check(c.visual->SetOffsetX(static_cast<float>(c.x)), "SetOffsetX");
    check(c.visual->SetOffsetY(static_cast<float>(c.y)), "SetOffsetY");
    check(c.target->SetRoot(c.visual.Get()), "IDCompositionTarget::SetRoot");
    check(c.dcomp->Commit(), "IDCompositionDevice::Commit");
}

void open_adapter(Context& c) {
    c.adapter.Reset();
    check(c.factory->EnumAdapterByLuid(c.luid, IID_PPV_ARGS(&c.adapter)), "EnumAdapterByLuid");
}

void wait_fence(Context& c, uint64_t value, DWORD timeout_ms) {
    if (!value || c.fence->GetCompletedValue() >= value) return;
    check(c.fence->SetEventOnCompletion(value, c.fence_event), "SetEventOnCompletion");
    const DWORD r = WaitForSingleObject(c.fence_event, timeout_ms);
    require(r == WAIT_OBJECT_0, "GPU fence wait timed out", C9PP_E_TIMEOUT);
}

void wait_idle(Context& c, DWORD timeout_ms) {
    if (!c.queue || !c.fence) return;
    const HRESULT hr = c.queue->Signal(c.fence.Get(), ++c.fence_value);
    if (removed_hr(hr)) return;  // A removed device completes nothing more.
    check(hr, "ID3D12CommandQueue::Signal");
    wait_fence(c, c.fence_value, timeout_ms);
}

void transition(ID3D12GraphicsCommandList* list, ID3D12Resource* resource, D3D12_RESOURCE_STATES before,
                D3D12_RESOURCE_STATES after) {
    D3D12_RESOURCE_BARRIER b{};
    b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    b.Transition.pResource = resource;
    b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    b.Transition.StateBefore = before;
    b.Transition.StateAfter = after;
    list->ResourceBarrier(1, &b);
}

void colour(uint32_t rgb, float out[4]) {
    out[0] = static_cast<float>(rgb >> 16 & 255) / 255.0f;
    out[1] = static_cast<float>(rgb >> 8 & 255) / 255.0f;
    out[2] = static_cast<float>(rgb & 255) / 255.0f;
    out[3] = 1.0f;
}

void clear_rects(Context& c, D3D12_CPU_DESCRIPTOR_HANDLE rtv, const std::vector<c9::Rect>& rects, uint32_t rgb) {
    if (rects.empty()) return;
    c.rects.clear();
    for (const c9::Rect& r : rects) c.rects.push_back(D3D12_RECT{r.left, r.top, r.right, r.bottom});
    float value[4];
    colour(rgb, value);
    // One rect per call: the first gated run (18 Nov, RTX 4070 Laptop) died with a fail-fast (0xC0000409) inside a
    // single ClearRenderTargetView call carrying about 64 rects, in both hosts; the step markers located it.
    for (const D3D12_RECT& r : c.rects) c.list->ClearRenderTargetView(rtv, value, 1, &r);
}

std::wstring widen(const char* utf8) {
    const int n = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, utf8, -1, nullptr, 0);
    require(n > 0, "log path is not valid UTF-8");
    std::wstring out(static_cast<size_t>(n), L'\0');
    MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, utf8, -1, out.data(), n);
    out.resize(static_cast<size_t>(n - 1));
    return out;
}

bool simple_token(const char* text, size_t max_length) {
    if (!text || !*text) return false;
    size_t n = 0;
    for (const char* p = text; *p; ++p, ++n) {
        const char ch = *p;
        if (n >= max_length) return false;
        if (!((ch >= 'a' && ch <= 'z') || (ch >= '0' && ch <= '9') || ch == '_' || ch == '-')) return false;
    }
    return true;
}

std::string awareness_name(DPI_AWARENESS_CONTEXT context) {
    if (!context) return "unknown";
    if (AreDpiAwarenessContextsEqual(context, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)) return "per-monitor-v2";
    switch (GetAwarenessFromDpiAwarenessContext(context)) {
        case DPI_AWARENESS_UNAWARE: return "unaware";
        case DPI_AWARENESS_SYSTEM_AWARE: return "system";
        case DPI_AWARENESS_PER_MONITOR_AWARE: return "per-monitor";
        default: return "unknown";
    }
}

void attach_row(Context& c, Row& r) {
    r.unum("hwnd", static_cast<uint64_t>(reinterpret_cast<uintptr_t>(c.hwnd)))
        .unum("flags", c.flags)
        .num("x", c.x).num("y", c.y).unum("w", c.w).unum("h", c.h)
        .unum("cell_px", c.cell).unum("max_latency", c.max_latency).unum("wait_timeout_ms", c.wait_ms)
        .rgb("base_rgb", c.base_rgb).rgb("code_white", c9::kWhite).rgb("code_black", c9::kBlack)
        .unum("qpc_frequency", c.qpc_frequency)
        .unum("luid_low", c.luid.LowPart).num("luid_high", c.luid.HighPart)
        .num("device_source", c.device_source).num("device_independent", c.device_independent)
        .hr("factory_hr", c.factory_hr).hr("target_hr", c.target_hr)
        .str("swap_effect", (c.flags & C9PP_ATTACH_FLIP_SEQUENTIAL) ? "flip_sequential" : "flip_discard")
        .str("dcomp_device", (c.flags & C9PP_ATTACH_DCOMP_DEVICE2) ? "DCompositionCreateDevice2" : "DCompositionCreateDevice3")
        .str("window_awareness", awareness_name(GetWindowDpiAwarenessContext(c.hwnd)))
        .unum("window_dpi", GetDpiForWindow(c.hwnd));
}

}  // namespace

extern "C" {

uint32_t c9pp_abi_version(void) { return C9PP_ABI_VERSION; }

int32_t c9pp_last_error(void* ctx, char* buffer, uint32_t buffer_size) {
    if (!buffer || buffer_size == 0) {
        g_error = "c9pp_last_error: buffer is NULL or empty";
        return C9PP_E_INVALID_ARGUMENT;
    }
    auto* c = static_cast<Context*>(ctx);
    if (c && c->thread == GetCurrentThreadId())
        copy_text(c->last_error, buffer, buffer_size);
    else
        copy_text(g_error, buffer, buffer_size);
    return C9PP_OK;
}

int32_t c9pp_validate_attach(const c9pp_attach_desc* desc) {
    return global_call("c9pp_validate_attach", [&] {
        validate(desc);
        return C9PP_OK;
    });
}

uint32_t c9pp_crc16(const uint8_t* data, uint32_t size) { return data || size == 0 ? c9::crc16(data, size) : 0u; }

int32_t c9pp_code_render(uint32_t frame_id, uint32_t epoch, uint32_t base_rgb, uint32_t cell_px, uint8_t* bgra,
                         uint32_t width, uint32_t height, uint32_t stride) {
    return global_call("c9pp_code_render", [&] {
        require(bgra != nullptr, "bgra is NULL");
        require(cell_px >= C9PP_MIN_CELL_PX && cell_px <= C9PP_MAX_CELL_PX, "cell_px out of range");
        require(base_rgb <= 0xFFFFFFu, "base_rgb has bits above 0xFFFFFF");
        validate_rect(0, 0, width, height, cell_px);
        require(stride >= width * 4, "stride smaller than width * 4");
        c9::render_bgra(frame_id, epoch, base_rgb, cell_px, bgra, width, height, stride);
        return C9PP_OK;
    });
}

int32_t c9pp_code_decode(const uint8_t* bgra, uint32_t buffer_width, uint32_t buffer_height, uint32_t stride,
                         double origin_x, double origin_y, double scale_x, double scale_y, uint32_t width,
                         uint32_t height, uint32_t cell_px, c9pp_decode_pair* out) {
    return global_call("c9pp_code_decode", [&] {
        require(out != nullptr && out->struct_size == sizeof(*out), "out.struct_size mismatch");
        require(bgra != nullptr, "bgra is NULL");
        require(stride >= buffer_width * 4, "stride smaller than buffer_width * 4");
        require(scale_x > 0.0 && scale_y > 0.0 && std::isfinite(scale_x) && std::isfinite(scale_y), "scale must be positive");
        require(std::isfinite(origin_x) && std::isfinite(origin_y), "origin must be finite");
        require(cell_px >= C9PP_MIN_CELL_PX && cell_px <= C9PP_MAX_CELL_PX, "cell_px out of range");
        validate_rect(0, 0, width, height, cell_px);
        int32_t ox[2], oy[2];
        c9::code_origins(width, height, cell_px, ox, oy);
        c9::Corner k[2];
        for (int i = 0; i < 2; ++i)
            k[i] = c9::decode_corner(bgra, buffer_width, buffer_height, stride, origin_x, origin_y, scale_x, scale_y, ox[i],
                                     oy[i], cell_px);
        c9pp_decode_pair result{};
        result.struct_size = sizeof(result);
        for (int i = 0; i < 2; ++i) {
            result.corner_status[i] = k[i].status;
            result.corner_frame[i] = k[i].frame_id;
            result.corner_epoch[i] = k[i].epoch;
            result.corner_ambiguous[i] = k[i].ambiguous;
        }
        result.status = c9::combine(k[0], k[1], &result.frame_id, &result.epoch);
        *out = result;
        return C9PP_OK;
    });
}

int32_t c9pp_attach(const c9pp_attach_desc* desc, const char* log_path_utf8, c9pp_attach_info* info, void** out_ctx) {
    return global_call("c9pp_attach", [&] {
        require(out_ctx != nullptr, "out_ctx is NULL");
        *out_ctx = nullptr;
        require(info != nullptr && info->struct_size == sizeof(*info), "info.struct_size mismatch");
        validate(desc);
        auto c = std::make_unique<Context>();
        c->thread = GetCurrentThreadId();
        c->hwnd = reinterpret_cast<HWND>(desc->hwnd);
        c->flags = desc->flags;
        c->host_device = desc->host_device;
        c->x = desc->x;
        c->y = desc->y;
        c->w = desc->width;
        c->h = desc->height;
        c->cell = effective_cell(*desc);
        c->max_latency = desc->max_latency ? desc->max_latency : 2u;
        c->wait_ms = desc->wait_timeout_ms ? desc->wait_timeout_ms : 100u;
        c->base_rgb = desc->base_rgb;
        LARGE_INTEGER frequency;
        QueryPerformanceFrequency(&frequency);
        c->qpc_frequency = static_cast<uint64_t>(frequency.QuadPart);
        if (log_path_utf8 && *log_path_utf8) {
            const std::wstring path = widen(log_path_utf8);
            if (_wfopen_s(&c->log, path.c_str(), L"wb") != 0) c->log = nullptr;
            require(c->log != nullptr, "cannot open the log file", C9PP_E_IO);
        }
        const uint64_t t0 = qpc();
        try {
            check(CreateDXGIFactory2(0, IID_PPV_ARGS(&c->factory)), "CreateDXGIFactory2");
            if (desc->flags & C9PP_ATTACH_USE_LUID) {
                c->luid.LowPart = desc->luid_low;
                c->luid.HighPart = desc->luid_high;
            } else {
                ComPtr<IDXGIAdapter> given;
                check(reinterpret_cast<IUnknown*>(desc->adapter)->QueryInterface(IID_PPV_ARGS(&given)), "adapter QueryInterface");
                DXGI_ADAPTER_DESC ad{};
                check(given->GetDesc(&ad), "IDXGIAdapter::GetDesc");
                c->luid = ad.AdapterLuid;
            }
            open_adapter(*c);
            create_device(*c);
            create_d3d_objects(*c);
            create_swapchain(*c);
            create_dcomp(*c);
        } catch (const Failure& e) {
            Row r = c->row("attach");
            attach_row(*c, r);
            r.flag("ok", false).str("error", e.text).unum("duration_us", (qpc() - t0) * 1000000 / c->qpc_frequency);
            c->emit(r);
            throw;
        }
        DXGI_ADAPTER_DESC1 ad{};
        c->adapter->GetDesc1(&ad);
        bool host_same_adapter = false;
        if (c->host_device) {
            ComPtr<ID3D12Device> host;
            if (SUCCEEDED(reinterpret_cast<IUnknown*>(c->host_device)->QueryInterface(IID_PPV_ARGS(&host)))) {
                const LUID hl = host->GetAdapterLuid();
                host_same_adapter = hl.LowPart == c->luid.LowPart && hl.HighPart == c->luid.HighPart;
            }
        }
        Row r = c->row("attach");
        attach_row(*c, r);
        r.flag("ok", true)
            .unum("vendor_id", ad.VendorId)
            .unum("device_id", ad.DeviceId)
            .flag("host_same_adapter", host_same_adapter)
            .unum("duration_us", (qpc() - t0) * 1000000 / c->qpc_frequency);
        c->emit(r);
        c9pp_attach_info result{};
        result.struct_size = sizeof(result);
        result.epoch = c->epoch;
        result.device_source = c->device_source;
        result.device_independent = c->device_independent;
        result.factory_hr = c->factory_hr;
        result.target_hr = c->target_hr;
        result.vendor_id = ad.VendorId;
        result.device_id = ad.DeviceId;
        result.luid_low = c->luid.LowPart;
        result.luid_high = c->luid.HighPart;
        result.qpc_frequency = c->qpc_frequency;
        *info = result;
        *out_ctx = c.release();
        return C9PP_OK;
    });
}

int32_t c9pp_set_phase(void* ctx, const char* phase) {
    return context_call(ctx, "c9pp_set_phase", [&](Context& c) {
        require(simple_token(phase, 48), "phase must be 1..48 characters of [a-z0-9_-]");
        c.phase = phase;
        Row r = c.row("phase");
        r.str("name", c.phase);
        c.emit(r);
        return C9PP_OK;
    });
}

int32_t c9pp_note(void* ctx, const char* kind, const char* json_object) {
    return context_call(ctx, "c9pp_note", [&](Context& c) {
        require(simple_token(kind, 48), "kind must be 1..48 characters of [a-z0-9_-]");
        require(json_object != nullptr, "json_object is NULL");
        const std::string_view json(json_object);
        require(json.size() >= 2 && json.size() <= 8192 && json.front() == '{' && json.back() == '}',
                "json_object must be one JSON object of at most 8192 bytes");
        require(json.find('\n') == std::string_view::npos && json.find('\r') == std::string_view::npos,
                "json_object must not contain line breaks");
        Row r = c.row("note");
        r.str("kind", kind).raw("data", json);
        c.emit(r);
        return C9PP_OK;
    });
}

int32_t c9pp_frame(void* ctx, uint32_t frame_id, uint32_t sync_interval, c9pp_frame_record* out) {
    return context_call(ctx, "c9pp_frame", [&](Context& c) {
        require(out != nullptr && out->struct_size == sizeof(*out), "out.struct_size mismatch");
        require(sync_interval <= 4, "sync_interval out of range (0..4)");
        require(c.swapchain != nullptr, "no swap chain; recover the device first", C9PP_E_WRONG_STATE);
        // Step markers (flushed rows) locate a fail-fast inside the frame path: the last row names the step that died.
        const bool trace_steps = c.frames_traced < 3;
        auto step = [&](const char* at) {
            if (!trace_steps) return;
            Row s = c.row("step");
            s.unum("frame_id", frame_id).str("at", at);
            c.emit(s);
        };
        step("enter");
        c9pp_frame_record rec{};
        rec.struct_size = sizeof(rec);
        rec.frame_id = frame_id;
        rec.epoch = c.epoch;
        rec.sync_interval = sync_interval;
        const HRESULT reason = c.device->GetDeviceRemovedReason();
        if (FAILED(reason)) {
            Row r = c.row("frame");
            r.unum("frame_id", frame_id).unum("sync", sync_interval).str("status", "device_removed").hr("reason", reason);
            c.emit(r);
            *out = rec;
            fail(C9PP_E_DEVICE_REMOVED, "the core device is removed: " + hex(reason));
        }
        step("wait_latency");
        const uint64_t t0 = qpc();
        const DWORD w = WaitForSingleObjectEx(c.waitable, c.wait_ms, FALSE);
        const uint64_t t1 = qpc();
        rec.wait_result = w == WAIT_OBJECT_0 ? C9PP_WAIT_SIGNALLED : w == WAIT_TIMEOUT ? C9PP_WAIT_TIMEOUT : C9PP_WAIT_FAILED;
        rec.wait_us = static_cast<uint32_t>(std::min<uint64_t>((t1 - t0) * 1000000 / c.qpc_frequency, 0xFFFFFFFFu));
        step("back_buffer_index");
        const UINT index = c.swapchain->GetCurrentBackBufferIndex();
        step("wait_buffer_fence");
        wait_fence(c, c.buffer_fence[index], 1000);
        step("reset");
        check(c.allocators[index]->Reset(), "ID3D12CommandAllocator::Reset");
        check(c.list->Reset(c.allocators[index].Get(), nullptr), "ID3D12GraphicsCommandList::Reset");
        transition(c.list.Get(), c.buffers[index].Get(), D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET);
        D3D12_CPU_DESCRIPTOR_HANDLE rtv = c.rtv_heap->GetCPUDescriptorHandleForHeapStart();
        rtv.ptr += static_cast<SIZE_T>(index) * c.rtv_stride;
        float base[4];
        colour(c.base_rgb, base);
        c.list->ClearRenderTargetView(rtv, base, 0, nullptr);
        step("code_rects");
        c9::code_rects(frame_id, c.epoch & 0xFFu, c.w, c.h, c.cell, c.white, c.black);
        if (trace_steps) {
            Row s = c.row("step");
            s.unum("frame_id", frame_id).str("at", "clear_rects").unum("white", c.white.size()).unum("black", c.black.size());
            c.emit(s);
        }
        clear_rects(c, rtv, c.white, c9::kWhite);
        clear_rects(c, rtv, c.black, c9::kBlack);
        transition(c.list.Get(), c.buffers[index].Get(), D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_STATE_PRESENT);
        step("close");
        check(c.list->Close(), "ID3D12GraphicsCommandList::Close");
        ID3D12CommandList* lists[] = {c.list.Get()};
        step("execute");
        c.queue->ExecuteCommandLists(1, lists);
        check(c.queue->Signal(c.fence.Get(), ++c.fence_value), "ID3D12CommandQueue::Signal");
        c.buffer_fence[index] = c.fence_value;
        step("present");
        const uint64_t t2 = qpc();
        const HRESULT present_hr = c.swapchain->Present(sync_interval, 0);
        const uint64_t t3 = qpc();
        step("last_present_count");
        UINT count = 0;
        const HRESULT count_hr = c.swapchain->GetLastPresentCount(&count);
        rec.present_hr = present_hr;
        rec.present_count = count;
        rec.qpc_present = t3;
        Row r = c.row("frame");
        r.unum("frame_id", frame_id)
            .unum("sync", sync_interval)
            .str("status", SUCCEEDED(present_hr) ? "presented" : "present_failed")
            .num("wait_result", rec.wait_result)
            .unum("wait_us", rec.wait_us)
            .unum("buffer", index)
            .unum("sc_gen", c.sc_gen)
            .hr("present_hr", present_hr)
            .unum("present_count", count)
            .hr("count_hr", count_hr)
            .unum("qpc_before_present", t2)
            .unum("qpc_present", t3);
        c.emit(r);
        ++c.frames_traced;
        *out = rec;
        if (FAILED(present_hr))
            fail(removed_hr(present_hr) ? C9PP_E_DEVICE_REMOVED : C9PP_E_WIN32, "Present failed: " + hex(present_hr));
        return C9PP_OK;
    });
}

int32_t c9pp_poll(void* ctx, c9pp_stats_row* out) {
    return context_call(ctx, "c9pp_poll", [&](Context& c) {
        require(out != nullptr && out->struct_size == sizeof(*out), "out.struct_size mismatch");
        require(c.swapchain != nullptr, "no swap chain; recover the device first", C9PP_E_WRONG_STATE);
        DXGI_FRAME_STATISTICS st{};
        const HRESULT hr = c.swapchain->GetFrameStatistics(&st);
        const uint64_t t = qpc();
        UINT last = 0;
        const HRESULT last_hr = c.swapchain->GetLastPresentCount(&last);
        DCOMPOSITION_FRAME_STATISTICS ds{};
        const HRESULT dcomp_hr = c.dcomp->GetFrameStatistics(&ds);
        c9pp_stats_row row{};
        row.struct_size = sizeof(row);
        row.hr = hr;
        row.epoch = c.epoch;
        row.present_count = st.PresentCount;
        row.present_refresh_count = st.PresentRefreshCount;
        row.sync_refresh_count = st.SyncRefreshCount;
        row.sync_qpc = static_cast<uint64_t>(st.SyncQPCTime.QuadPart);
        row.poll_qpc = t;
        row.last_present_count = last;
        Row r = c.row("poll");
        r.hr("hr", hr)
            .flag("disjoint", hr == DXGI_ERROR_FRAME_STATISTICS_DISJOINT)
            .unum("present_count", st.PresentCount)
            .unum("present_refresh", st.PresentRefreshCount)
            .unum("sync_refresh", st.SyncRefreshCount)
            .unum("sync_qpc", row.sync_qpc)
            .unum("poll_qpc", t)
            .unum("last_present_count", last)
            .hr("last_hr", last_hr)
            .unum("sc_gen", c.sc_gen)
            .hr("dcomp_hr", dcomp_hr)
            .num("dcomp_last_frame", ds.lastFrameTime.QuadPart)
            .unum("dcomp_rate_num", ds.currentCompositionRate.Numerator)
            .unum("dcomp_rate_den", ds.currentCompositionRate.Denominator)
            .num("dcomp_time", ds.currentTime.QuadPart)
            .num("dcomp_frequency", ds.timeFrequency.QuadPart)
            .num("dcomp_next", ds.nextEstimatedFrameTime.QuadPart);
        c.emit(r);
        *out = row;
        return C9PP_OK;
    });
}

int32_t c9pp_resize(void* ctx, int32_t x, int32_t y, uint32_t width, uint32_t height) {
    return context_call(ctx, "c9pp_resize", [&](Context& c) {
        validate_rect(x, y, width, height, c.cell);
        require(c.swapchain != nullptr, "no swap chain; recover the device first", C9PP_E_WRONG_STATE);
        const uint64_t t0 = qpc();
        wait_idle(c, 2000);
        for (auto& b : c.buffers) b.Reset();
        const HRESULT resize_hr = c.swapchain->ResizeBuffers(kBuffers, width, height, kFormat, kSwapFlags);
        HRESULT commit_hr = E_FAIL;
        if (SUCCEEDED(resize_hr)) {
            c.w = width;
            c.h = height;
            c.x = x;
            c.y = y;
            for (auto& v : c.buffer_fence) v = 0;
            create_rtvs(c);
            check(c.visual->SetOffsetX(static_cast<float>(x)), "SetOffsetX");
            check(c.visual->SetOffsetY(static_cast<float>(y)), "SetOffsetY");
            commit_hr = c.dcomp->Commit();
            ++c.sc_gen;
        }
        Row r = c.row("resize");
        r.num("x", x).num("y", y).unum("w", width).unum("h", height)
            .hr("resize_hr", resize_hr).hr("commit_hr", commit_hr).unum("sc_gen", c.sc_gen)
            .unum("duration_us", (qpc() - t0) * 1000000 / c.qpc_frequency);
        c.emit(r);
        check(resize_hr, "ResizeBuffers");
        check(commit_hr, "IDCompositionDevice::Commit");
        return C9PP_OK;
    });
}

int32_t c9pp_remove_device(void* ctx) {
    return context_call(ctx, "c9pp_remove_device", [&](Context& c) {
        require(c.device != nullptr, "no device", C9PP_E_WRONG_STATE);
        // Re-check identity immediately before the removal: never remove the host's device.
        c.device_independent = independence(c);
        if (c.device_independent != 1) {
            Row r = c.row("remove_device");
            r.flag("refused", true).num("device_independent", c.device_independent).num("device_source", c.device_source);
            c.emit(r);
            fail(C9PP_E_UNSUPPORTED, "the core device is not proven independent of the host device; removal refused");
        }
        ComPtr<ID3D12Device5> d5;
        const HRESULT qhr = c.device.As(&d5);
        if (FAILED(qhr)) {
            Row r = c.row("remove_device");
            r.flag("refused", true).hr("query_hr", qhr);
            c.emit(r);
            fail(C9PP_E_UNSUPPORTED, "ID3D12Device5 unavailable: " + hex(qhr));
        }
        const uint64_t t = qpc();
        d5->RemoveDevice();
        const HRESULT reason = c.device->GetDeviceRemovedReason();
        c.removal_requested = true;
        Row r = c.row("remove_device");
        r.flag("refused", false).unum("qpc_remove", t).hr("reason", reason).num("device_independent", c.device_independent);
        c.emit(r);
        return C9PP_OK;
    });
}

int32_t c9pp_device_status(void* ctx, int32_t* out_hr) {
    return context_call(ctx, "c9pp_device_status", [&](Context& c) {
        require(out_hr != nullptr, "out_hr is NULL");
        const HRESULT hr = c.device ? c.device->GetDeviceRemovedReason() : DXGI_ERROR_DEVICE_REMOVED;
        *out_hr = hr;
        Row r = c.row("core_status");
        r.hr("hr", hr);
        c.emit(r);
        return C9PP_OK;
    });
}

int32_t c9pp_recover(void* ctx, uint32_t flags, uint32_t* out_epoch) {
    return context_call(ctx, "c9pp_recover", [&](Context& c) {
        require((flags & ~C9PP_RECOVER_RECREATE_TARGET) == 0, "unknown recover flags");
        require(out_epoch != nullptr, "out_epoch is NULL");
        const HRESULT reason = c.device ? c.device->GetDeviceRemovedReason() : DXGI_ERROR_DEVICE_REMOVED;
        require(FAILED(reason), "the core device is not removed", C9PP_E_WRONG_STATE);
        const uint64_t t0 = qpc();
        const uint32_t epoch_before = c.epoch;
        HRESULT detach_hr = S_OK, commit1_hr = S_OK, target_release_hr = S_OK, set_hr = E_FAIL, commit2_hr = E_FAIL;
        if (c.visual) {
            detach_hr = c.visual->SetContent(nullptr);
            commit1_hr = c.dcomp->Commit();
        }
        c.release_d3d();
        const bool factory_current = c.factory && c.factory->IsCurrent();
        if (!factory_current) {
            c.factory.Reset();
            check(CreateDXGIFactory2(0, IID_PPV_ARGS(&c.factory)), "CreateDXGIFactory2");
        }
        open_adapter(c);
        const bool recreate_target = (flags & C9PP_RECOVER_RECREATE_TARGET) != 0;
        if (recreate_target && c.target) {
            target_release_hr = c.target->SetRoot(nullptr);
            c.dcomp->Commit();
            c.target.Reset();
            c.dcomp->Commit();
            c.target_hr = c.dcomp->CreateTargetForHwnd(c.hwnd, FALSE, &c.target);
            check(c.target_hr, "CreateTargetForHwnd(hwnd, FALSE) on recovery");
            check(c.target->SetRoot(c.visual.Get()), "IDCompositionTarget::SetRoot");
        }
        create_device(c);
        create_d3d_objects(c);
        create_swapchain(c);
        bool dcomp_recreated = false;
        set_hr = c.visual ? c.visual->SetContent(c.swapchain.Get()) : E_POINTER;
        if (SUCCEEDED(set_hr)) commit2_hr = c.dcomp->Commit();
        if (FAILED(set_hr) || FAILED(commit2_hr)) {
            // The kept DComp chain is unusable: rebuild device, target and visual.
            c.visual.Reset();
            c.target.Reset();
            c.dcomp.Reset();
            create_dcomp(c);
            dcomp_recreated = true;
        }
        ++c.epoch;
        ++c.sc_gen;
        c.removal_requested = false;
        Row r = c.row("recover");
        r.hr("reason_before", reason)
            .unum("epoch_before", epoch_before)
            .unum("epoch_after", c.epoch)
            .hr("detach_hr", detach_hr)
            .hr("commit1_hr", commit1_hr)
            .flag("factory_current", factory_current)
            .flag("recreate_target", recreate_target)
            .hr("target_release_hr", target_release_hr)
            .hr("target_hr", c.target_hr)
            .hr("set_content_hr", set_hr)
            .hr("commit2_hr", commit2_hr)
            .flag("dcomp_recreated", dcomp_recreated)
            .num("device_source", c.device_source)
            .num("device_independent", c.device_independent)
            .hr("factory_hr", c.factory_hr)
            .unum("duration_us", (qpc() - t0) * 1000000 / c.qpc_frequency);
        c.emit(r);
        *out_epoch = c.epoch;
        return C9PP_OK;
    });
}

int32_t c9pp_sample_screen(void* ctx, const c9pp_sample_desc* desc, c9pp_screen_sample* out) {
    return context_call(ctx, "c9pp_sample_screen", [&](Context& c) {
        require(desc != nullptr && desc->struct_size == sizeof(*desc), "desc.struct_size mismatch");
        require(out != nullptr && out->struct_size == sizeof(*out), "out.struct_size mismatch");
        require(desc->ui_w > 0 && desc->ui_h > 0 && desc->bg_w > 0 && desc->bg_h > 0, "empty sample region");
        require(desc->ui_w <= C9PP_MAX_EXTENT && desc->ui_h <= C9PP_MAX_EXTENT && desc->bg_w <= C9PP_MAX_EXTENT &&
                    desc->bg_h <= C9PP_MAX_EXTENT,
                "sample region too large");
        c9pp_screen_sample s{};
        s.struct_size = sizeof(s);
        s.status = C9PP_SAMPLE_CAPTURE_FAILED;
        const HWND hwnd = c.hwnd;

        // 1. The window's own DPI space: client size and logical screen corners, then the
        //    documented logical-to-physical conversion.
        const DPI_AWARENESS_CONTEXT own = GetWindowDpiAwarenessContext(hwnd);
        DPI_AWARENESS_CONTEXT previous = SetThreadDpiAwarenessContext(own);
        RECT client{};
        GetClientRect(hwnd, &client);
        POINT p0{0, 0}, p1{client.right, client.bottom};
        ClientToScreen(hwnd, &p0);
        ClientToScreen(hwnd, &p1);
        if (previous) SetThreadDpiAwarenessContext(previous);
        POINT q0 = p0, q1 = p1;
        const BOOL l0 = LogicalToPhysicalPointForPerMonitorDPI(hwnd, &q0);
        const BOOL l1 = LogicalToPhysicalPointForPerMonitorDPI(hwnd, &q1);
        const double sx = client.right > 0 ? static_cast<double>(q1.x - q0.x) / client.right : 1.0;
        const double sy = client.bottom > 0 ? static_cast<double>(q1.y - q0.y) / client.bottom : 1.0;

        // 2. Per-monitor-v2 context: an independent physical mapping, the capture and the hit test.
        previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        const bool pmv2 = previous != nullptr;
        RECT physical{};
        GetClientRect(hwnd, &physical);
        POINT r0{0, 0};
        ClientToScreen(hwnd, &r0);
        s.dpi_consistent = pmv2 && l0 && l1 && std::abs(r0.x - q0.x) <= 1 && std::abs(r0.y - q0.y) <= 1 &&
                           std::abs(physical.right - (q1.x - q0.x)) <= 1 && std::abs(physical.bottom - (q1.y - q0.y)) <= 1;
        const int32_t bx0 = std::min({c.x, desc->ui_x, desc->bg_x});
        const int32_t by0 = std::min({c.y, desc->ui_y, desc->bg_y});
        const int64_t bx1 = std::max({static_cast<int64_t>(c.x) + c.w, static_cast<int64_t>(desc->ui_x) + desc->ui_w,
                                      static_cast<int64_t>(desc->bg_x) + desc->bg_w});
        const int64_t by1 = std::max({static_cast<int64_t>(c.y) + c.h, static_cast<int64_t>(desc->ui_y) + desc->ui_h,
                                      static_cast<int64_t>(desc->bg_y) + desc->bg_h});
        const int32_t X0 = static_cast<int32_t>(std::floor(q0.x + bx0 * sx));
        const int32_t Y0 = static_cast<int32_t>(std::floor(q0.y + by0 * sy));
        const int32_t X1 = static_cast<int32_t>(std::ceil(q0.x + static_cast<double>(bx1) * sx));
        const int32_t Y1 = static_cast<int32_t>(std::ceil(q0.y + static_cast<double>(by1) * sy));
        const int32_t W = X1 - X0, H = Y1 - Y0;
        BOOL blt = FALSE;
        uint8_t* pixels = nullptr;
        HDC screen = nullptr, memory = nullptr;
        HBITMAP bitmap = nullptr;
        HGDIOBJ old = nullptr;
        if (W > 0 && H > 0 && W <= 2 * static_cast<int32_t>(C9PP_MAX_EXTENT) && H <= 2 * static_cast<int32_t>(C9PP_MAX_EXTENT)) {
            screen = GetDC(nullptr);
            memory = screen ? CreateCompatibleDC(screen) : nullptr;
            BITMAPINFO bi{};
            bi.bmiHeader.biSize = sizeof(bi.bmiHeader);
            bi.bmiHeader.biWidth = W;
            bi.bmiHeader.biHeight = -H;
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB;
            void* bits = nullptr;
            bitmap = memory ? CreateDIBSection(memory, &bi, DIB_RGB_COLORS, &bits, nullptr, 0) : nullptr;
            if (bitmap) {
                old = SelectObject(memory, bitmap);
                s.qpc_before = qpc();
                blt = BitBlt(memory, 0, 0, W, H, screen, X0, Y0, SRCCOPY | CAPTUREBLT);
                s.qpc_after = qpc();
                GdiFlush();
                pixels = static_cast<uint8_t*>(bits);
            }
        }
        const POINT centre{static_cast<LONG>(std::lround(q0.x + (c.x + c.w / 2.0) * sx)),
                           static_cast<LONG>(std::lround(q0.y + (c.y + c.h / 2.0) * sy))};
        const HWND hit = WindowFromPoint(centre);
        s.center_is_host = hit != nullptr && GetAncestor(hit, GA_ROOT) == hwnd;
        if (pmv2) SetThreadDpiAwarenessContext(previous);
        s.foreground = GetForegroundWindow() == hwnd;
        s.iconic = IsIconic(hwnd) != FALSE;

        c9::Corner k[2];
        k[0].status = k[1].status = C9PP_CORNER_OUT_OF_BOUNDS;
        if (blt && pixels) {
            const uint32_t stride = static_cast<uint32_t>(W) * 4;
            const double ox = q0.x + c.x * sx - X0, oy = q0.y + c.y * sy - Y0;
            int32_t cx[2], cy[2];
            c9::code_origins(c.w, c.h, c.cell, cx, cy);
            for (int i = 0; i < 2; ++i)
                k[i] = c9::decode_corner(pixels, static_cast<uint32_t>(W), static_cast<uint32_t>(H), stride, ox, oy, sx, sy,
                                         cx[i], cy[i], c.cell);
            s.status = c9::combine(k[0], k[1], &s.decoded_frame, &s.decoded_epoch);
            s.decoded = s.status == C9PP_SAMPLE_MATCH || s.status == C9PP_SAMPLE_ONE_CORNER;
            for (int i = 0; i < 2; ++i) {
                s.corner_status[i] = k[i].status;
                s.corner_frame[i] = k[i].frame_id;
                s.corner_epoch[i] = k[i].epoch;
            }
            auto region = [&](int32_t x, int32_t y, uint32_t w, uint32_t h, uint32_t* rgb) {
                // Inset by a quarter on each side to stay clear of edges and filtering.
                const double ix0 = x + w * 0.25, ix1 = x + w * 0.75, iy0 = y + h * 0.25, iy1 = y + h * 0.75;
                return c9::mean_rgb(pixels, static_cast<uint32_t>(W), static_cast<uint32_t>(H), stride,
                                    static_cast<int32_t>(std::ceil(q0.x + ix0 * sx - X0)),
                                    static_cast<int32_t>(std::ceil(q0.y + iy0 * sy - Y0)),
                                    static_cast<int32_t>(std::floor(q0.x + ix1 * sx - X0)),
                                    static_cast<int32_t>(std::floor(q0.y + iy1 * sy - Y0)), rgb);
            };
            region(desc->ui_x, desc->ui_y, desc->ui_w, desc->ui_h, &s.ui_rgb);
            region(desc->bg_x, desc->bg_y, desc->bg_w, desc->bg_h, &s.bg_rgb);
        }
        if (old) SelectObject(memory, old);
        if (bitmap) DeleteObject(bitmap);
        if (memory) DeleteDC(memory);
        if (screen) ReleaseDC(nullptr, screen);

        static const char* const kSampleNames[] = {"match", "one_corner", "mismatch", "undecodable", "capture_failed"};
        static const char* const kCornerNames[] = {"ok", "ambiguous", "bad_sync", "bad_check", "out_of_bounds"};
        Row r = c.row("sample");
        r.str("status", kSampleNames[s.status])
            .flag("decoded", s.decoded != 0)
            .unum("frame_id", s.decoded_frame)
            .unum("code_epoch", s.decoded_epoch)
            .str("corner_a", kCornerNames[k[0].status])
            .str("corner_b", kCornerNames[k[1].status])
            .unum("corner_a_frame", k[0].frame_id).unum("corner_a_epoch", k[0].epoch).unum("corner_a_ambiguous", k[0].ambiguous)
            .unum("corner_b_frame", k[1].frame_id).unum("corner_b_epoch", k[1].epoch).unum("corner_b_ambiguous", k[1].ambiguous)
            .rgb("ui_rgb", s.ui_rgb)
            .rgb("bg_rgb", s.bg_rgb)
            .flag("foreground", s.foreground != 0)
            .flag("iconic", s.iconic != 0)
            .flag("center_is_host", s.center_is_host != 0)
            .flag("dpi_consistent", s.dpi_consistent != 0)
            .flag("pmv2_thread", pmv2)
            .str("window_awareness", awareness_name(own))
            .unum("window_dpi", GetDpiForWindow(hwnd))
            .real("scale_x", sx).real("scale_y", sy)
            .num("client_w", client.right).num("client_h", client.bottom)
            .num("physical_x", q0.x).num("physical_y", q0.y)
            .num("pmv2_x", r0.x).num("pmv2_y", r0.y).num("pmv2_w", physical.right).num("pmv2_h", physical.bottom)
            .num("capture_x", X0).num("capture_y", Y0).num("capture_w", W).num("capture_h", H)
            .flag("blt_ok", blt != FALSE)
            .unum("qpc_before", s.qpc_before)
            .unum("qpc_after", s.qpc_after)
            .unum("blt_us", s.qpc_after > s.qpc_before ? (s.qpc_after - s.qpc_before) * 1000000 / c.qpc_frequency : 0);
        c.emit(r);
        *out = s;
        return C9PP_OK;
    });
}

int32_t c9pp_godot_device_status(void* ctx, uint64_t device, int32_t* out_hr) {
    auto body = [&](Context* c) {
        require(device != 0, "device is 0");
        require(out_hr != nullptr, "out_hr is NULL");
        ComPtr<ID3D12Device> d;
        check(reinterpret_cast<IUnknown*>(device)->QueryInterface(IID_PPV_ARGS(&d)), "host device QueryInterface");
        const HRESULT hr = d->GetDeviceRemovedReason();
        *out_hr = hr;
        if (c) {
            Row r = c->row("host_status");
            r.hr("hr", hr);
            c->emit(r);
        }
        return C9PP_OK;
    };
    if (!ctx) return global_call("c9pp_godot_device_status", [&] { return body(nullptr); });
    return context_call(ctx, "c9pp_godot_device_status", [&](Context& c) { return body(&c); });
}

int32_t c9pp_detach(void* ctx) {
    auto* c = static_cast<Context*>(ctx);
    const int32_t status = context_call(ctx, "c9pp_detach", [&](Context& k) {
        const bool alive = k.device && SUCCEEDED(k.device->GetDeviceRemovedReason());
        HRESULT commit_hr = S_OK;
        if (alive) {
            try {
                wait_idle(k, 2000);
            } catch (const Failure&) {
            }
        }
        if (k.visual) k.visual->SetContent(nullptr);
        if (k.target) k.target->SetRoot(nullptr);
        if (k.dcomp) commit_hr = k.dcomp->Commit();
        Row r = k.row("detach");
        r.flag("device_alive", alive).hr("commit_hr", commit_hr);
        k.emit(r);
        return C9PP_OK;
    });
    if (status == C9PP_OK || status == C9PP_E_TIMEOUT) delete c;
    return status;
}

}  // extern "C"
