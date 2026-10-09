// Minimal Direct3D 12 program for the stage 2.4 readiness check (packet E-2.4-00).
// Clears and presents a borderless window at the display's native size on the
// high-performance adapter, and writes a run.json skeleton next to the capture so that
// tools/perf/renderer_gate.py can parse a short PresentMon capture of it.
// Usage: minimal_d3d12.exe <seconds> <run-dir>
#include <windows.h>
#include <d3d12.h>
#include <dxgi1_6.h>
#include <wrl/client.h>
#include <cstdio>
#include <cstdlib>
#include <string>

using Microsoft::WRL::ComPtr;

#define CHECK(x) do { HRESULT hr_ = (x); if (FAILED(hr_)) { std::fprintf(stderr, "%s failed: 0x%08lx\n", #x, (unsigned long)hr_); std::exit(2); } } while (0)

static LRESULT CALLBACK WndProc(HWND h, UINT m, WPARAM w, LPARAM l) {
    if (m == WM_DESTROY) { PostQuitMessage(0); return 0; }
    if (m == WM_KEYDOWN && w == VK_ESCAPE) { DestroyWindow(h); return 0; }
    return DefWindowProcW(h, m, w, l);
}

static long long Qpc() { LARGE_INTEGER t; QueryPerformanceCounter(&t); return t.QuadPart; }

int main(int argc, char** argv) {
    double seconds = argc > 1 ? std::atof(argv[1]) : 20.0;
    std::string runDir = argc > 2 ? argv[2] : ".";
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    const int width = GetSystemMetrics(SM_CXSCREEN), height = GetSystemMetrics(SM_CYSCREEN);

    WNDCLASSW wc{}; wc.lpfnWndProc = WndProc; wc.hInstance = GetModuleHandleW(nullptr); wc.lpszClassName = L"m600ready";
    RegisterClassW(&wc);
    HWND hwnd = CreateWindowExW(0, wc.lpszClassName, L"Magic 600 Cell D3D12 readiness", WS_POPUP | WS_VISIBLE,
                                0, 0, width, height, nullptr, nullptr, wc.hInstance, nullptr);

    ComPtr<IDXGIFactory6> factory; CHECK(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)));
    ComPtr<IDXGIAdapter1> adapter;
    CHECK(factory->EnumAdapterByGpuPreference(0, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE, IID_PPV_ARGS(&adapter)));
    DXGI_ADAPTER_DESC1 desc; adapter->GetDesc1(&desc);
    ComPtr<ID3D12Device> device; CHECK(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_12_0, IID_PPV_ARGS(&device)));

    D3D12_COMMAND_QUEUE_DESC qd{}; qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    ComPtr<ID3D12CommandQueue> queue; CHECK(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue)));
    const UINT kFrames = 2;
    DXGI_SWAP_CHAIN_DESC1 sd{}; sd.Width = width; sd.Height = height; sd.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    sd.SampleDesc.Count = 1; sd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT; sd.BufferCount = kFrames;
    sd.SwapEffect = DXGI_SWAP_EFFECT_FLIP_DISCARD;
    ComPtr<IDXGISwapChain1> sc1; CHECK(factory->CreateSwapChainForHwnd(queue.Get(), hwnd, &sd, nullptr, nullptr, &sc1));
    ComPtr<IDXGISwapChain3> swap; CHECK(sc1.As(&swap));

    D3D12_DESCRIPTOR_HEAP_DESC hd{}; hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV; hd.NumDescriptors = kFrames;
    ComPtr<ID3D12DescriptorHeap> rtvHeap; CHECK(device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&rtvHeap)));
    const UINT rtvStep = device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
    ComPtr<ID3D12Resource> targets[kFrames]; ComPtr<ID3D12CommandAllocator> allocators[kFrames];
    for (UINT i = 0; i < kFrames; ++i) {
        CHECK(swap->GetBuffer(i, IID_PPV_ARGS(&targets[i])));
        D3D12_CPU_DESCRIPTOR_HANDLE h = rtvHeap->GetCPUDescriptorHandleForHeapStart(); h.ptr += SIZE_T(i) * rtvStep;
        device->CreateRenderTargetView(targets[i].Get(), nullptr, h);
        CHECK(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&allocators[i])));
    }
    ComPtr<ID3D12GraphicsCommandList> list;
    CHECK(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, allocators[0].Get(), nullptr, IID_PPV_ARGS(&list)));
    list->Close();
    ComPtr<ID3D12Fence> fence; CHECK(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)));
    HANDLE event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    UINT64 fenceValues[kFrames] = {}, nextValue = 1;

    LARGE_INTEGER freq; QueryPerformanceFrequency(&freq);
    const long long start = Qpc(), stopAt = start + (long long)(seconds * freq.QuadPart);
    unsigned long long frames = 0; MSG msg{}; bool running = true;
    while (running && Qpc() < stopAt) {
        while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) {
            if (msg.message == WM_QUIT) running = false;
            TranslateMessage(&msg); DispatchMessageW(&msg);
        }
        if (!running) break;
        const UINT i = swap->GetCurrentBackBufferIndex();
        if (fence->GetCompletedValue() < fenceValues[i]) { fence->SetEventOnCompletion(fenceValues[i], event); WaitForSingleObject(event, INFINITE); }
        CHECK(allocators[i]->Reset()); CHECK(list->Reset(allocators[i].Get(), nullptr));
        D3D12_RESOURCE_BARRIER b{}; b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION; b.Transition.pResource = targets[i].Get();
        b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        b.Transition.StateBefore = D3D12_RESOURCE_STATE_PRESENT; b.Transition.StateAfter = D3D12_RESOURCE_STATE_RENDER_TARGET;
        list->ResourceBarrier(1, &b);
        D3D12_CPU_DESCRIPTOR_HANDLE h = rtvHeap->GetCPUDescriptorHandleForHeapStart(); h.ptr += SIZE_T(i) * rtvStep;
        const float shade = float(frames % 120) / 120.0f, color[4] = {0.05f, 0.08f + 0.2f * shade, 0.12f, 1.0f};
        list->ClearRenderTargetView(h, color, 0, nullptr);
        std::swap(b.Transition.StateBefore, b.Transition.StateAfter);
        list->ResourceBarrier(1, &b);
        CHECK(list->Close());
        ID3D12CommandList* lists[] = {list.Get()}; queue->ExecuteCommandLists(1, lists);
        CHECK(swap->Present(1, 0));
        fenceValues[i] = nextValue; CHECK(queue->Signal(fence.Get(), nextValue++));
        ++frames;
    }
    const long long stop = Qpc();
    CHECK(queue->Signal(fence.Get(), nextValue)); fence->SetEventOnCompletion(nextValue, event); WaitForSingleObject(event, INFINITE);

    char adapterName[128]; WideCharToMultiByte(CP_UTF8, 0, desc.Description, -1, adapterName, sizeof adapterName, nullptr, nullptr);
    SYSTEM_POWER_STATUS power; GetSystemPowerStatus(&power);
    std::string path = runDir + "\\run.json";
    FILE* out = std::fopen(path.c_str(), "w");
    if (!out) { std::fprintf(stderr, "cannot write %s\n", path.c_str()); return 3; }
    // Hand-written in the sense of the packet: the swap chain address is filled in from the
    // capture's SwapChainAddress column before running renderer_gate.py.
    std::fprintf(out,
        "{\n  \"format\": \"magic600-renderer-run-v1\",\n  \"run_id\": \"readiness-%lld\",\n  \"scene\": \"w1\",\n"
        "  \"candidate\": \"readiness\",\n  \"qpc_frequency\": %lld,\n"
        "  \"markers\": {\"trace_start_qpc\": %lld, \"trace_stop_qpc\": %lld},\n"
        "  \"presentmon\": {\"process_id\": %lu, \"swap_chain\": \"FILL-FROM-CSV\"},\n"
        "  \"build\": {\"build_identity\": \"renderer-readiness-minimal-d3d12\"},\n"
        "  \"environment\": {\"power_source\": \"%s\", \"declared\": {\"frame_generation\": false, \"upscaling\": false},\n"
        "    \"display\": {\"width\": %d, \"height\": %d}, \"backbuffer\": {\"width\": %d, \"height\": %d}},\n"
        "  \"frames\": %llu,\n  \"adapter\": \"%s\"\n}\n",
        start, freq.QuadPart, start, stop, GetCurrentProcessId(), power.ACLineStatus == 1 ? "mains" : "battery",
        width, height, width, height, frames, adapterName);
    std::fclose(out);
    std::printf("presented %llu frames in %.1f s on %s; wrote %s\n", frames, double(stop - start) / freq.QuadPart, adapterName, path.c_str());
    return 0;
}
