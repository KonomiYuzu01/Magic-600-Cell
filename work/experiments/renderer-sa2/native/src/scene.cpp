#include "scene.h"
#include <dxgi1_6.h>
#include <wrl/client.h>
#include <algorithm>
#include <chrono>
#include <cstring>
#include <optional>
#include <limits>

namespace sa2 {
namespace {
using Microsoft::WRL::ComPtr;
using Clock = std::chrono::steady_clock;
void hr(ID3D12Device* device, HRESULT result, const char* call) {
    if (SUCCEEDED(result)) return;
    std::ostringstream text;
    text << call << " failed: HRESULT 0x" << std::hex << std::setw(8) << std::setfill('0') << uint32_t(result);
    throw Failure(FAILED(device->GetDeviceRemovedReason()) ? SA2_E_DEVICE_REMOVED : SA2_E_D3D12, text.str());
}
int64_t frequency() {
    LARGE_INTEGER hz{};
    scene_require(QueryPerformanceFrequency(&hz) != 0 && hz.QuadPart > 0, "QueryPerformanceFrequency failed", SA2_E_D3D12);
    return hz.QuadPart;
}
struct Constants {
    sb::Matrix q;
    std::array<float, 4> normal;
    float radius, cs, ss, d4, zoom, aspect, theta;
    uint32_t sampleCount;
    std::array<float, 4> planeU, planeV;
};
static_assert(sizeof(Constants) == 144);
Constants constants(const sb::Assets& a, const sb::Matrix& q, float aspect, float theta, uint32_t count = 0) {
    return {q, a.normal, a.radius, 0.76f, 0.82f, 1.18f, 1.15f, aspect, theta, count, a.planeU, a.planeV};
}
struct Buffer {
    ComPtr<ID3D12Resource> resource;
    D3D12_RESOURCE_STATES state = D3D12_RESOURCE_STATE_COMMON;
    uint64_t size = 0;
};
}

struct Scene::Impl {
    ID3D12Device* device;
    sb::Assets assets;
    std::vector<uint32_t> sample, authoritative_labels;
    std::array<Buffer, 6> geometry;
    std::array<Buffer, 3> label_buffers, cb, upload;
    Buffer solved, geometry_cb, result, result_read, counter, counter_read, zero;
    std::vector<Buffer> static_uploads, readbacks;
    struct InitialCopy { ID3D12Resource* target; ID3D12Resource* source; uint64_t size; };
    std::vector<InitialCopy> initial_copies;
    std::array<uint8_t*, 3> cb_data{}, upload_data{};
    uint8_t* geometry_cb_data = nullptr;
    ComPtr<ID3D12RootSignature> root;
    ComPtr<ID3D12PipelineState> draw_pso, count_pso, compute_pso;
    ComPtr<ID3D12Resource> depth, check_target, check_depth;
    ComPtr<ID3D12DescriptorHeap> dsv_heap, check_rtv_heap;
    ComPtr<ID3D12CommandAllocator> check_allocator;
    ComPtr<ID3D12GraphicsCommandList> check_list;
    ComPtr<ID3D12Fence> check_fence;
    ComPtr<IDXGIAdapter3> memory_adapter;
    std::string adapter_name;
    HANDLE event = nullptr;
    uint64_t check_value = 0, authoritative = 0, camera = 0;
    std::array<uint64_t, 3> label_revision{};
    unsigned expected = 0, bound = 0;
    std::optional<unsigned> pending;
    sb::LabelRecords records;
    sb::Matrix q = sb::identity();
    bool initialized = false, healthy = true;
    static constexpr uint64_t readback_slots = 64;

    explicit Impl(ID3D12Device* attached) : device(attached), assets(scene_assets()), sample(assets.samples()), authoritative_labels(assets.oracle[0]) {}
    ~Impl() {
        for (size_t i = 0; i < cb.size(); ++i) {
            if (cb_data[i]) cb[i].resource->Unmap(0, nullptr);
            if (upload_data[i]) upload[i].resource->Unmap(0, nullptr);
        }
        if (geometry_cb_data) geometry_cb.resource->Unmap(0, nullptr);
        if (event) CloseHandle(event);
    }
    template<class T> void name(T* object, const wchar_t* text) { hr(device, object->SetName(text), "SetName(scene)"); }
    Buffer buffer(uint64_t size, D3D12_HEAP_TYPE type, D3D12_RESOURCE_STATES state,
                  D3D12_RESOURCE_FLAGS flags = D3D12_RESOURCE_FLAG_NONE) {
        Buffer out; out.size = size; out.state = state;
        D3D12_HEAP_PROPERTIES heap{}; heap.Type = type; heap.CreationNodeMask = heap.VisibleNodeMask = 1;
        D3D12_RESOURCE_DESC d{}; d.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER; d.Width = size; d.Height = 1;
        d.DepthOrArraySize = d.MipLevels = 1; d.SampleDesc.Count = 1; d.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR; d.Flags = flags;
        hr(device, device->CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &d, state, nullptr, IID_PPV_ARGS(&out.resource)), "CreateCommittedResource(scene buffer)");
        name(out.resource.Get(), L"SA2 scene buffer");
        return out;
    }
    uint8_t* map(Buffer& b) {
        void* data = nullptr; D3D12_RANGE none{0, 0};
        hr(device, b.resource->Map(0, &none, &data), "Map(scene upload)");
        return static_cast<uint8_t*>(data);
    }
    Buffer static_buffer(const void* data, uint64_t size) {
        auto target = buffer(size, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_COPY_DEST);
        auto staging = buffer(size, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
        std::memcpy(map(staging), data, static_cast<size_t>(size)); staging.resource->Unmap(0, nullptr);
        initial_copies.push_back({target.resource.Get(), staging.resource.Get(), size});
        static_uploads.push_back(std::move(staging));
        return target;
    }
    void transition(ID3D12GraphicsCommandList* list, Buffer& b, D3D12_RESOURCE_STATES after) {
        if (b.state == after) return;
        D3D12_RESOURCE_BARRIER barrier{}; barrier.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
        barrier.Transition = {b.resource.Get(), D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, b.state, after};
        list->ResourceBarrier(1, &barrier); b.state = after;
    }
    void initialize(ID3D12GraphicsCommandList* list) {
        if (initialized) return;
        for (const auto& copy : initial_copies) list->CopyBufferRegion(copy.target, 0, copy.source, 0, copy.size);
        for (auto& b : geometry) transition(list, b, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
        for (auto& b : label_buffers) transition(list, b, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
        transition(list, solved, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
        initialized = true;
    }
    ComPtr<ID3D12Resource> texture(UINT width, UINT height, bool is_depth) {
        D3D12_HEAP_PROPERTIES heap{}; heap.Type = D3D12_HEAP_TYPE_DEFAULT; heap.CreationNodeMask = heap.VisibleNodeMask = 1;
        D3D12_RESOURCE_DESC d{}; d.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D; d.Width = width; d.Height = height;
        d.DepthOrArraySize = d.MipLevels = 1; d.SampleDesc.Count = 1;
        d.Format = is_depth ? DXGI_FORMAT_D32_FLOAT : DXGI_FORMAT_R8G8B8A8_UNORM;
        d.Flags = is_depth ? D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL : D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
        D3D12_CLEAR_VALUE clear{}; clear.Format = DXGI_FORMAT_D32_FLOAT; clear.DepthStencil.Depth = 1;
        ComPtr<ID3D12Resource> out;
        hr(device, device->CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &d,
            is_depth ? D3D12_RESOURCE_STATE_DEPTH_WRITE : D3D12_RESOURCE_STATE_RENDER_TARGET,
            is_depth ? &clear : nullptr, IID_PPV_ARGS(&out)), "CreateCommittedResource(scene target)");
        name(out.Get(), is_depth ? L"SA2 scene depth" : L"SA2 geometry target");
        return out;
    }
    void make_root() {
        D3D12_ROOT_PARAMETER params[10]{}; params[0].ParameterType = D3D12_ROOT_PARAMETER_TYPE_CBV;
        params[0].Descriptor.ShaderRegister = 0;
        for (UINT i = 1; i <= 7; ++i) { params[i].ParameterType = D3D12_ROOT_PARAMETER_TYPE_SRV; params[i].Descriptor.ShaderRegister = i - 1; }
        for (UINT i = 8; i <= 9; ++i) { params[i].ParameterType = D3D12_ROOT_PARAMETER_TYPE_UAV; params[i].Descriptor.ShaderRegister = i - 8; }
        D3D12_ROOT_SIGNATURE_DESC d{}; d.NumParameters = 10; d.pParameters = params;
        d.Flags = D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;
        ComPtr<ID3DBlob> blob, error;
        hr(device, D3D12SerializeRootSignature(&d, D3D_ROOT_SIGNATURE_VERSION_1, &blob, &error), "D3D12SerializeRootSignature(scene)");
        hr(device, device->CreateRootSignature(0, blob->GetBufferPointer(), blob->GetBufferSize(), IID_PPV_ARGS(&root)), "CreateRootSignature(scene)");
        name(root.Get(), L"SA2 scene root signature");
    }
    ComPtr<ID3D12PipelineState> graphics(const std::vector<uint8_t>& vs, const std::vector<uint8_t>& ps) {
        D3D12_GRAPHICS_PIPELINE_STATE_DESC d{}; d.pRootSignature = root.Get(); d.VS = {vs.data(), vs.size()}; d.PS = {ps.data(), ps.size()};
        auto& blend = d.BlendState.RenderTarget[0]; blend.SrcBlend = blend.SrcBlendAlpha = D3D12_BLEND_ONE;
        blend.DestBlend = blend.DestBlendAlpha = D3D12_BLEND_ZERO; blend.BlendOp = blend.BlendOpAlpha = D3D12_BLEND_OP_ADD;
        blend.LogicOp = D3D12_LOGIC_OP_NOOP; blend.RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL;
        d.SampleMask = UINT_MAX; d.RasterizerState.FillMode = D3D12_FILL_MODE_SOLID;
        d.RasterizerState.CullMode = D3D12_CULL_MODE_NONE; d.RasterizerState.DepthClipEnable = TRUE;
        d.DepthStencilState.DepthEnable = TRUE; d.DepthStencilState.DepthWriteMask = D3D12_DEPTH_WRITE_MASK_ALL;
        d.DepthStencilState.DepthFunc = D3D12_COMPARISON_FUNC_LESS_EQUAL;
        d.DepthStencilState.StencilReadMask = D3D12_DEFAULT_STENCIL_READ_MASK; d.DepthStencilState.StencilWriteMask = D3D12_DEFAULT_STENCIL_WRITE_MASK;
        D3D12_DEPTH_STENCILOP_DESC stencil{}; stencil.StencilFailOp = stencil.StencilDepthFailOp = stencil.StencilPassOp = D3D12_STENCIL_OP_KEEP;
        stencil.StencilFunc = D3D12_COMPARISON_FUNC_ALWAYS; d.DepthStencilState.FrontFace = d.DepthStencilState.BackFace = stencil;
        d.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE; d.NumRenderTargets = 1;
        d.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM; d.DSVFormat = DXGI_FORMAT_D32_FLOAT; d.SampleDesc.Count = 1;
        ComPtr<ID3D12PipelineState> out;
        hr(device, device->CreateGraphicsPipelineState(&d, IID_PPV_ARGS(&out)), "CreateGraphicsPipelineState(scene)");
        name(out.Get(), L"SA2 scene graphics pipeline");
        return out;
    }
    void create(const SceneRecord& record) {
        const auto directory = module_path().parent_path();
        std::array<std::vector<uint8_t>, 4> shaders;
        for (size_t i = 0; i < shaders.size(); ++i) shaders[i] = scene_bytes(directory / shader_names()[i]);
        make_root(); draw_pso = graphics(shaders[2], shaders[1]); count_pso = graphics(shaders[0], shaders[1]);
        D3D12_COMPUTE_PIPELINE_STATE_DESC p{}; p.pRootSignature = root.Get(); p.CS = {shaders[3].data(), shaders[3].size()};
        hr(device, device->CreateComputePipelineState(&p, IID_PPV_ARGS(&compute_pso)), "CreateComputePipelineState(scene)");
        name(compute_pso.Get(), L"SA2 geometry compute pipeline");
        auto put = [&](const auto& values) { return static_buffer(values.data(), values.size() * sizeof(values[0])); };
        geometry[0] = put(assets.vertices); geometry[1] = put(assets.local); geometry[2] = put(assets.centers);
        geometry[3] = put(assets.frames); geometry[4] = put(assets.flags); geometry[5] = put(sample);
        for (auto& label : label_buffers) label = put(assets.oracle[0]);
        solved = put(assets.oracle[0]);
        for (size_t i = 0; i < cb.size(); ++i) {
            cb[i] = buffer(256, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
            upload[i] = buffer(sb::LabelBytes, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
            cb_data[i] = map(cb[i]); upload_data[i] = map(upload[i]);
        }
        geometry_cb = buffer(256, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ); geometry_cb_data = map(geometry_cb);
        result = buffer(sample.size() * 12, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
        result_read = buffer(result.size, D3D12_HEAP_TYPE_READBACK, D3D12_RESOURCE_STATE_COPY_DEST);
        counter = buffer(sb::Cells * 4, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
        counter_read = buffer(counter.size, D3D12_HEAP_TYPE_READBACK, D3D12_RESOURCE_STATE_COPY_DEST);
        zero = buffer(counter.size, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
        std::memset(map(zero), 0, static_cast<size_t>(zero.size)); zero.resource->Unmap(0, nullptr);
        depth = texture(record.width, record.height, true); check_target = texture(2560, 1600, false); check_depth = texture(2560, 1600, true);
        D3D12_DESCRIPTOR_HEAP_DESC heap{}; heap.Type = D3D12_DESCRIPTOR_HEAP_TYPE_DSV; heap.NumDescriptors = 2;
        hr(device, device->CreateDescriptorHeap(&heap, IID_PPV_ARGS(&dsv_heap)), "CreateDescriptorHeap(scene DSV)");
        name(dsv_heap.Get(), L"SA2 scene DSV heap");
        auto dsv = dsv_heap->GetCPUDescriptorHandleForHeapStart(); device->CreateDepthStencilView(depth.Get(), nullptr, dsv);
        dsv.ptr += device->GetDescriptorHandleIncrementSize(heap.Type); device->CreateDepthStencilView(check_depth.Get(), nullptr, dsv);
        heap.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV; heap.NumDescriptors = 1;
        hr(device, device->CreateDescriptorHeap(&heap, IID_PPV_ARGS(&check_rtv_heap)), "CreateDescriptorHeap(geometry RTV)");
        name(check_rtv_heap.Get(), L"SA2 geometry RTV heap");
        device->CreateRenderTargetView(check_target.Get(), nullptr, check_rtv_heap->GetCPUDescriptorHandleForHeapStart());
        hr(device, device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&check_allocator)), "CreateCommandAllocator(geometry)");
        name(check_allocator.Get(), L"SA2 geometry command allocator");
        hr(device, device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, check_allocator.Get(), nullptr, IID_PPV_ARGS(&check_list)), "CreateCommandList(geometry)");
        name(check_list.Get(), L"SA2 geometry command list"); hr(device, check_list->Close(), "Close(geometry initial)");
        hr(device, device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&check_fence)), "CreateFence(geometry)");
        name(check_fence.Get(), L"SA2 geometry fence");
        event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        if (!event) hr(device, HRESULT_FROM_WIN32(GetLastError()), "CreateEventW(geometry)");
        ComPtr<IDXGIFactory4> factory;
        hr(device, CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)), "CreateDXGIFactory2(scene adapter)");
        ComPtr<IDXGIAdapter1> adapter;
        hr(device, factory->EnumAdapterByLuid(device->GetAdapterLuid(), IID_PPV_ARGS(&adapter)), "EnumAdapterByLuid(scene device)");
        DXGI_ADAPTER_DESC1 desc{}; hr(device, adapter->GetDesc1(&desc), "GetDesc1(scene adapter)");
        const int n = WideCharToMultiByte(CP_UTF8, 0, desc.Description, -1, nullptr, 0, nullptr, nullptr);
        scene_require(n > 0, "adapter description UTF-8 conversion failed", SA2_E_D3D12);
        adapter_name.resize(static_cast<size_t>(n));
        WideCharToMultiByte(CP_UTF8, 0, desc.Description, -1, adapter_name.data(), n, nullptr, nullptr); adapter_name.pop_back();
        if (!(record.config.flags & SA2_SCENE_NO_VRAM)) hr(device, adapter.As(&memory_adapter), "QueryInterface(IDXGIAdapter3)");
        if (record.config.scene >= 3) readbacks.push_back(buffer(readback_slots * sb::LabelBytes, D3D12_HEAP_TYPE_READBACK, D3D12_RESOURCE_STATE_COPY_DEST));
    }
    void bind(ID3D12GraphicsCommandList* list, const Constants& c, Buffer& constants_buffer, uint8_t* data,
              Buffer& label, bool compute) {
        std::memcpy(data, &c, sizeof(c));
        if (compute) { list->SetComputeRootSignature(root.Get()); list->SetComputeRootConstantBufferView(0, constants_buffer.resource->GetGPUVirtualAddress()); }
        else { list->SetGraphicsRootSignature(root.Get()); list->SetGraphicsRootConstantBufferView(0, constants_buffer.resource->GetGPUVirtualAddress()); }
        for (UINT i = 0; i < 7; ++i) {
            auto address = (i < 5 ? geometry[i] : i == 5 ? label : geometry[5]).resource->GetGPUVirtualAddress();
            if (compute) list->SetComputeRootShaderResourceView(i + 1, address); else list->SetGraphicsRootShaderResourceView(i + 1, address);
        }
    }
    void draw(ID3D12GraphicsCommandList* list, const Constants& c, Buffer& constants_buffer, uint8_t* data,
              Buffer& label, ID3D12PipelineState* pipeline, D3D12_CPU_DESCRIPTOR_HANDLE rtv,
              UINT width, UINT height, bool count) {
        list->SetPipelineState(pipeline); bind(list, c, constants_buffer, data, label, false);
        if (count) list->SetGraphicsRootUnorderedAccessView(8, counter.resource->GetGPUVirtualAddress());
        D3D12_VIEWPORT viewport{0, 0, float(width), float(height), 0, 1}; D3D12_RECT rect{0, 0, LONG(width), LONG(height)};
        list->RSSetViewports(1, &viewport); list->RSSetScissorRects(1, &rect);
        auto dsv = dsv_heap->GetCPUDescriptorHandleForHeapStart();
        if (count) dsv.ptr += device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_DSV);
        list->OMSetRenderTargets(1, &rtv, FALSE, &dsv);
        const float colour[4] = {0.13f, 0.145f, 0.16f, 1}; list->ClearRenderTargetView(rtv, colour, 0, nullptr);
        list->ClearDepthStencilView(dsv, D3D12_CLEAR_FLAG_DEPTH, 1, 0, 0, nullptr);
        list->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST); list->DrawInstanced(sb::Vertices, sb::Cells, 0, 0);
    }
    void wait(uint32_t timeout) {
        const auto until = Clock::now() + std::chrono::milliseconds(timeout);
        for (;;) {
            const auto completed = check_fence->GetCompletedValue();
            hr(device, device->GetDeviceRemovedReason(), "GetDeviceRemovedReason(geometry wait)");
            scene_require(completed != UINT64_MAX, "geometry fence reports device removal", SA2_E_DEVICE_REMOVED);
            if (completed >= check_value) return;
            scene_require(Clock::now() < until, "geometry CPU fence wait timed out", SA2_E_TIMEOUT);
            hr(device, check_fence->SetEventOnCompletion(check_value, event), "SetEventOnCompletion(geometry)");
            const auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(until - Clock::now()).count();
            const auto waited = WaitForSingleObject(event, static_cast<DWORD>(std::max<int64_t>(0, std::min<int64_t>(remaining, MAXDWORD - 1))));
            if (waited == WAIT_FAILED) hr(device, HRESULT_FROM_WIN32(GetLastError()), "WaitForSingleObject(geometry)");
        }
    }
    void begin(uint32_t timeout) {
        wait(timeout);
        hr(device, check_allocator->Reset(), "Reset(geometry allocator)");
        hr(device, check_list->Reset(check_allocator.Get(), nullptr), "Reset(geometry list)");
        initialize(check_list.Get());
    }
    void submit(ID3D12CommandQueue* queue, uint32_t timeout, const std::function<void()>& submitting) {
        hr(device, check_list->Close(), "Close(geometry list)");
        ID3D12CommandList* lists[] = {check_list.Get()}; submitting(); queue->ExecuteCommandLists(1, lists);
        hr(device, queue->Signal(check_fence.Get(), ++check_value), "Signal(geometry fence)");
        wait(timeout);
    }
};

Scene::Scene(ID3D12Device* device, const sa2_scene_config& config, uint32_t width, uint32_t height)
    : record(config, frequency(), width, height) {
    const auto ticks = std::llround((config.scene == 3 ? config.turn_ms : 190) * double(record.frequency) / 1000);
    scene_require(ticks > 0, "turn_ms is below QPC resolution", SA2_E_INVALID_ARGUMENT);
    gpu = std::make_unique<Impl>(device); gpu->create(record);
}
Scene::~Scene() = default;
void Scene::poison() { gpu->healthy = false; }
void Scene::require_healthy() const { scene_require(gpu->healthy, "scene recording failed; drain and unload before retrying"); }
void Scene::wait_geometry(uint32_t timeout) { gpu->wait(timeout); }
void Scene::record_draw(ID3D12GraphicsCommandList* list, uint32_t slot, D3D12_CPU_DESCRIPTOR_HANDLE target, int64_t now) {
    auto& g = *gpu; g.initialize(list);
    const bool timed = record.tracing(), label_scene = record.config.scene >= 3;
    const auto frame = uint64_t(record.trace.size()); sb::Turn turn{0, 0, 0};
    if (timed) turn = sb::turnAt(now, record.start, sb::turnTicks(record.config.scene == 3 ? record.config.turn_ms : 190, record.frequency), g.assets.angle);
    bool copy = false;
    if (label_scene && timed) {
        if (turn.index > g.authoritative) {
            g.pending.reset(); const auto old = g.expected; g.expected = (g.expected + 1) % 3;
            while (g.authoritative < turn.index) { g.assets.move(g.authoritative_labels, g.authoritative); ++g.authoritative; }
            auto labels = g.authoritative_labels;
            const bool fault = turn.index == 20 && !record.injection_applied && !record.injection.empty();
            if (fault) { record.injection_applied = true; sb::injectLabels(labels, record.injection); }
            std::memcpy(g.upload_data[slot], labels.data(), size_t(sb::LabelBytes));
            auto& label = g.label_buffers[g.expected]; g.transition(list, label, D3D12_RESOURCE_STATE_COPY_DEST);
            list->CopyBufferRegion(label.resource.Get(), 0, g.upload[slot].resource.Get(), 0, sb::LabelBytes);
            g.transition(list, label, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE); g.label_revision[g.expected] = g.authoritative;
            g.records.uploads.push_back({frame, g.authoritative, uint64_t(g.expected + 1)});
            if (fault && (record.injection == "delay-adoption" || record.injection == "stale-binding")) {
                g.bound = old; g.pending = g.expected; copy = record.injection == "stale-binding";
            } else { g.bound = g.expected; copy = true; }
        } else if (g.pending) { g.bound = *g.pending; g.pending.reset(); copy = true; }
        g.records.uses.push_back({frame, turn.index, g.label_revision[g.bound], uint64_t(g.bound + 1), uint64_t(g.expected + 1)});
    }
    if (record.config.scene == 2 || record.config.scene == 3) { sb::rotate(g.q, 0, 3, 0.002); ++g.camera; }
    const auto c = constants(g.assets, g.q, float(record.width) / float(record.height), record.config.scene == 3 && timed ? float(turn.theta) : 0);
    g.draw(list, c, g.cb[slot], g.cb_data[slot], g.label_buffers[g.bound], g.draw_pso.Get(), target, record.width, record.height, false);
    if (copy) {
        const auto index = uint64_t(g.records.copies.size()), chunk = index / Impl::readback_slots;
        while (chunk >= g.readbacks.size()) g.readbacks.push_back(g.buffer(Impl::readback_slots * sb::LabelBytes, D3D12_HEAP_TYPE_READBACK, D3D12_RESOURCE_STATE_COPY_DEST));
        auto& label = g.label_buffers[g.bound]; g.transition(list, label, D3D12_RESOURCE_STATE_COPY_SOURCE);
        list->CopyBufferRegion(g.readbacks[size_t(chunk)].resource.Get(), index % Impl::readback_slots * sb::LabelBytes, label.resource.Get(), 0, sb::LabelBytes);
        g.transition(list, label, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
        g.records.copies.push_back({frame, g.label_revision[g.bound], uint64_t(g.bound + 1), uint64_t(g.expected + 1), index});
    }
    if (timed) record.trace.push_back({frame, now, label_scene ? g.label_revision[g.bound] : 0, record.config.scene == 3, turn.index, turn.phase, g.camera});
}
void Scene::sample_vram() {
    if (!record.tracing() || (record.config.flags & SA2_SCENE_NO_VRAM)) return;
    DXGI_QUERY_VIDEO_MEMORY_INFO info{};
    hr(gpu->device, gpu->memory_adapter->QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &info), "QueryVideoMemoryInfo(scene)");
    record.vram_peak = std::max(record.vram_peak, double(info.CurrentUsage) / 1048576); ++record.vram_samples;
}
Json Scene::labels() {
    if (record.config.scene < 3) return Json();
    auto& g = *gpu;
    // Preserve S-B's mapped readback protocol without duplicating the run's
    // entire (potentially gigabyte-sized) readback in CPU memory.
    std::vector<const uint32_t*> copied; copied.reserve(g.readbacks.size());
    struct Unmap {
        Impl& gpu; std::vector<const uint32_t*>& mapped;
        ~Unmap() { D3D12_RANGE none{0, 0}; for (size_t i = 0; i < mapped.size(); ++i) gpu.readbacks[i].resource->Unmap(0, &none); }
    } unmap{g, copied};
    for (auto& b : g.readbacks) {
        void* data = nullptr; D3D12_RANGE range{0, SIZE_T(b.size)};
        hr(g.device, b.resource->Map(0, &range, &data), "Map(scene preserved labels)");
        copied.push_back(static_cast<const uint32_t*>(data));
    }
    auto check = sb::checkLabels(g.assets, g.records, [&](const sb::Copy& c) {
        return std::span<const uint32_t>(copied.at(size_t(c.slot / Impl::readback_slots)) + c.slot % Impl::readback_slots * sb::Slots, sb::Slots);
    });
    if (!record.injection.empty() && !record.injection_applied) { check["status"] = "fail"; check["injection_not_reached"] = true; }
    return check;
}
Json Scene::geometry_check(ID3D12CommandQueue* queue, uint32_t timeout, const std::function<void()>& submitting) {
    auto& g = *gpu; const auto reference = sb::repoRoot() / "work/experiments/renderer-sb/reference";
    const auto raw_index = scene_bytes(reference / "index.json");
    const auto index = Json::parse(std::string(raw_index.begin(), raw_index.end()));
    for (const auto& [file, entry] : index.at("files").object()) {
        const auto path = reference / std::filesystem::path(file).filename();
        const auto& digest = std::holds_alternative<std::string>(entry.value) ? entry : entry.at("sha256");
        scene_require(sb::sha256(scene_bytes(path)) == digest.string(), "geometry reference SHA-256 mismatch: " + path.filename().string(), SA2_E_CHECK_FAILED);
    }
    const auto supplied = scene_bytes(reference / "sample.u32");
    scene_require(supplied.size() == g.sample.size() * 4 && std::memcmp(supplied.data(), g.sample.data(), supplied.size()) == 0,
        "geometry sample differs from SPEC rule", SA2_E_CHECK_FAILED);
    g.begin(timeout); auto* list = g.check_list.Get();
    g.transition(list, g.counter, D3D12_RESOURCE_STATE_COPY_DEST);
    list->CopyBufferRegion(g.counter.resource.Get(), 0, g.zero.resource.Get(), 0, g.counter.size);
    g.transition(list, g.counter, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
    g.draw(list, constants(g.assets, sb::identity(), 1.6f, 0), g.geometry_cb, g.geometry_cb_data, g.solved,
        g.count_pso.Get(), g.check_rtv_heap->GetCPUDescriptorHandleForHeapStart(), 2560, 1600, true);
    g.transition(list, g.counter, D3D12_RESOURCE_STATE_COPY_SOURCE);
    list->CopyBufferRegion(g.counter_read.resource.Get(), 0, g.counter.resource.Get(), 0, g.counter.size);
    g.submit(queue, timeout, submitting);
    void* data = nullptr; D3D12_RANGE count_range{0, SIZE_T(g.counter.size)};
    hr(g.device, g.counter_read.resource->Map(0, &count_range, &data), "Map(geometry counts)");
    std::array<uint32_t, sb::Cells> counts{}; std::memcpy(counts.data(), data, counts.size() * 4);
    D3D12_RANGE none{0, 0}; g.counter_read.resource->Unmap(0, &none);
    Json::Array count_array; uint64_t count_failures = 0;
    for (auto count : counts) { count_array.emplace_back(count); if (count != sb::Vertices) ++count_failures; }
    bool failed = count_failures > 0; Json::Array results;
    for (const auto& camera : g.assets.cameras.at("cameras").array()) {
        auto q = sb::identity();
        for (const auto& r : camera.at("rotations").array()) sb::rotate(q, int(r.array()[0].integer()), int(r.array()[1].integer()), r.array()[2].number());
        const std::array<std::pair<const char*, double>, 3> poses{{{"start", 0}, {"mid", g.assets.angle * 0.5}, {"end", g.assets.angle}}};
        for (const auto& [pose, theta] : poses) {
            const auto path = reference / (camera.at("name").string() + '_' + pose + ".f32");
            const auto raw = scene_bytes(path);
            scene_require(raw.size() == g.result.size, "reference projection length mismatch: " + path.filename().string(), SA2_E_CHECK_FAILED);
            std::vector<float> ref(raw.size() / 4); std::memcpy(ref.data(), raw.data(), raw.size());
            g.begin(timeout); list->SetPipelineState(g.compute_pso.Get());
            g.bind(list, constants(g.assets, q, 1.6f, float(theta), uint32_t(g.sample.size())), g.geometry_cb, g.geometry_cb_data, g.solved, true);
            list->SetComputeRootUnorderedAccessView(9, g.result.resource->GetGPUVirtualAddress()); list->Dispatch(UINT((g.sample.size() + 63) / 64), 1, 1);
            g.transition(list, g.result, D3D12_RESOURCE_STATE_COPY_SOURCE);
            list->CopyBufferRegion(g.result_read.resource.Get(), 0, g.result.resource.Get(), 0, g.result.size);
            g.transition(list, g.result, D3D12_RESOURCE_STATE_UNORDERED_ACCESS); g.submit(queue, timeout, submitting);
            D3D12_RANGE range{0, SIZE_T(g.result.size)};
            std::vector<float> values(ref.size());
            hr(g.device, g.result_read.resource->Map(0, &range, &data), "Map(geometry projections)");
            std::memcpy(values.data(), data, raw.size()); g.result_read.resource->Unmap(0, &none);
            double max_error = 0; uint64_t failures = 0;
            for (size_t i = 0; i < values.size(); ++i) {
                if (!std::isfinite(values[i]) || !std::isfinite(ref[i])) { ++failures; continue; }
                const double error = std::abs(double(values[i]) - ref[i]);
                if (error > 1e-4 + 1e-4 * std::abs(double(ref[i]))) ++failures;
                max_error = std::max(max_error, error);
            }
            failed |= failures > 0;
            results.emplace_back(Json::Object{{"camera", camera.at("name")}, {"pose", pose}, {"samples", uint64_t(g.sample.size())},
                {"max_abs_error", max_error}, {"failures", failures}, {"status", failures ? "fail" : "pass"}});
        }
    }
    const auto identity = scene_identity().dump();
    return Json::Object{{"format", "magic600-sb-geometry-check-v1"}, {"status", failed ? "fail" : "pass"},
        {"build_identity", sb::sha256(std::span(reinterpret_cast<const uint8_t*>(identity.data()), identity.size()))},
        {"adapter", g.adapter_name}, {"results", results},
        {"per_cell_count", Json::Object{{"cells", sb::Cells}, {"expected", sb::Vertices}, {"failures", count_failures}, {"counts", count_array}, {"status", count_failures ? "fail" : "pass"}}}};
}
Json Scene::geometry_error(const std::string& error) const {
    const auto identity = scene_identity().dump();
    return Json::Object{{"format", "magic600-sb-geometry-check-v1"}, {"status", "fail"},
        {"build_identity", sb::sha256(std::span(reinterpret_cast<const uint8_t*>(identity.data()), identity.size()))},
        {"adapter", gpu->adapter_name}, {"error", error}, {"results", Json::Array{}},
        {"per_cell_count", Json::Object{{"cells", sb::Cells}, {"expected", sb::Vertices}, {"status", "not-checked"}, {"counts", Json()}, {"failures", Json()}}}};
}
}
