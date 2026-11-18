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
// W-J's constants (renderer-wj/gpu.cpp): S-B's 144 bytes, then the pose and overlay fields.
struct Constants {
    sb::Matrix q;
    std::array<float, 4> normal;
    float radius, cs, ss, d4, zoom, aspect, theta;
    uint32_t sampleCount;
    std::array<float, 4> planeU, planeV;
    uint32_t poseEnabled, overlayMode, overlayCount, poseCount;
    float previewAngle, maxAngle, pad0 = 0, pad1 = 0;
};
static_assert(sizeof(Constants) == 176);
// The model fields the constants read, from S-B's reader (W1..W4) or W-J's.
struct Shape {
    std::array<float, 4> normal{}, plane_u{}, plane_v{};
    float radius = 0;
    double angle = 0;
    uint32_t pose_enabled = 0, max_poses = 1;
};
// W-J's constants() with overlay "none"; the pose path is enabled for W-J only.
Constants constants(const Shape& s, const sb::Matrix& q, float aspect, float theta, uint32_t count = 0) {
    return {q, s.normal, s.radius, 0.76f, 0.82f, 1.18f, 1.15f, aspect, theta, count, s.plane_u, s.plane_v,
            s.pose_enabled, 0u, 0u, s.max_poses, std::abs(theta), float(s.angle)};
}
struct Buffer {
    ComPtr<ID3D12Resource> resource;
    D3D12_RESOURCE_STATES state = D3D12_RESOURCE_STATE_COMMON;
    uint64_t size = 0;
};
std::string build_identity() {
    const auto identity = scene_identity().dump();
    return sb::sha256(std::span(reinterpret_cast<const uint8_t*>(identity.data()), identity.size()));
}
// A W-J reader or reference refusal inside a check fails the check (output written), never the scene.
template<class F> void refused(F&& body) {
    try { body(); }
    catch (const Failure&) { throw; }
    catch (const std::exception& error) { throw Failure(SA2_E_CHECK_FAILED, error.what()); }
}
}

struct Scene::Impl {
    ID3D12Device* device;
    std::optional<sb::Assets> assets; // W1..W4: S-B's readers, unchanged
    std::optional<wj::Assets> jumble; // W-J: W-J's readers
    Shape shape;
    std::vector<uint32_t> sample, authoritative_labels;
    std::array<Buffer, 6> geometry;
    std::array<Buffer, 3> label_buffers, cb, upload, check_pose;
    std::array<std::array<Buffer, 3>, 3> pose_buffers; // per ring resource: piece pose index, matrix rows, lattice table
    std::array<uint32_t, 3> pose_counts{};
    std::array<Buffer, 5> extra; // W-J's order: slot -> piece, destination, overlay points, straddles, offsets
    Buffer solved, geometry_cb, result, result_read, counter, counter_read, zero;
    Buffer check_label, check_destination, check_upload, lattice_result, lattice_read; // W-J checks only
    std::vector<Buffer> static_uploads, readbacks;
    struct InitialCopy { ID3D12Resource* target; ID3D12Resource* source; uint64_t size; };
    std::vector<InitialCopy> initial_copies;
    std::array<uint8_t*, 3> cb_data{}, upload_data{};
    uint8_t* geometry_cb_data = nullptr;
    uint8_t* check_upload_data = nullptr;
    ComPtr<ID3D12RootSignature> root;
    ComPtr<ID3D12PipelineState> draw_pso, count_pso, compute_pso, lattice_pso;
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
    WjProtocol protocol; // W-J's revision state and records
    wj::PoseState injected; // W-J: poses[0] with the injection applied, the turn-20 upload
    sb::Matrix q = sb::identity();
    bool initialized = false, healthy = true;
    uint64_t readback_capacity = 0, row_bytes = 64, lattice_bytes = 4, stride = sb::LabelBytes;

    Impl(ID3D12Device* attached, const SceneRecord& record) : device(attached) {
        if (record.jumbling()) {
            jumble.emplace(wj_assets(record.menu, record.data));
            const auto& a = *jumble;
            shape = {a.normal, a.planeU, a.planeV, a.radius, a.angle, 1u, a.maxPoses};
            sample = a.samples(); authoritative_labels = a.oracle[0];
            protocol.injection = record.injection; injected = wj_injected(a, record.injection);
            row_bytes = uint64_t(a.maxPoses) * 64; lattice_bytes = uint64_t(a.maxPoses) * 4;
            stride = wj::LabelBytes + wj::IndexBytes + row_bytes + lattice_bytes;
        } else {
            assets.emplace(scene_assets());
            const auto& a = *assets;
            shape = {a.normal, a.planeU, a.planeV, a.radius, a.angle, 0u, 1u};
            sample = a.samples(); authoritative_labels = a.oracle[0];
        }
    }
    ~Impl() {
        for (size_t i = 0; i < cb.size(); ++i) {
            if (cb_data[i]) cb[i].resource->Unmap(0, nullptr);
            if (upload_data[i]) upload[i].resource->Unmap(0, nullptr);
        }
        if (geometry_cb_data) geometry_cb.resource->Unmap(0, nullptr);
        if (check_upload_data) check_upload.resource->Unmap(0, nullptr);
        if (event) CloseHandle(event);
    }
    template<class T> void name(T* object, const wchar_t* text) { hr(device, object->SetName(text), "SetName(scene)"); }
    Buffer buffer(uint64_t size, D3D12_HEAP_TYPE type, D3D12_RESOURCE_STATES state,
                  D3D12_RESOURCE_FLAGS flags = D3D12_RESOURCE_FLAG_NONE) {
        Buffer out; out.size = size; out.state = state;
        D3D12_HEAP_PROPERTIES heap{}; heap.Type = type; heap.CreationNodeMask = heap.VisibleNodeMask = 1;
        D3D12_RESOURCE_DESC d{}; d.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER; d.Width = size; d.Height = 1;
        d.DepthOrArraySize = d.MipLevels = 1; d.SampleDesc.Count = 1; d.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR; d.Flags = flags;
        // The runtime creates every default-heap buffer in COMMON whatever the
        // initial state says, and the debug layer warns about any other value.
        // The first use promotes the buffer implicitly, so the tracked state
        // stays the state that use needs.
        const auto initial = type == D3D12_HEAP_TYPE_DEFAULT ? D3D12_RESOURCE_STATE_COMMON : state;
        hr(device, device->CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &d, initial, nullptr, IID_PPV_ARGS(&out.resource)), "CreateCommittedResource(scene buffer)");
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
        for (auto& ring : pose_buffers) for (auto& b : ring) transition(list, b, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
        for (auto& b : extra) transition(list, b, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
        initialized = true;
    }
    // W-J's uploadLabels/uploadPose step: zero the target's staging range, copy the raw bytes, copy the whole target.
    void stage(ID3D12GraphicsCommandList* list, Buffer& target, Buffer& staging, uint8_t* data, uint64_t offset, std::span<const uint8_t> raw) {
        scene_require(raw.size() <= target.size && offset + target.size <= staging.size, "upload exceeds its staging range");
        std::memset(data + offset, 0, size_t(target.size)); std::memcpy(data + offset, raw.data(), raw.size());
        transition(list, target, D3D12_RESOURCE_STATE_COPY_DEST);
        list->CopyBufferRegion(target.resource.Get(), 0, staging.resource.Get(), offset, target.size);
        transition(list, target, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
    }
    // Index, matrix rows and lattice table at offset, offset + IndexBytes and offset + IndexBytes + row_bytes.
    void upload_pose(ID3D12GraphicsCommandList* list, std::array<Buffer, 3>& target, Buffer& staging, uint8_t* data, uint64_t offset, const wj::PoseState& state) {
        scene_require(state.count() * uint64_t(64) <= row_bytes, "pose table exceeds allocated capacity");
        const auto raw = state.raw(); const uint64_t offsets[] = {offset, offset + wj::IndexBytes, offset + wj::IndexBytes + row_bytes};
        for (size_t j = 0; j < 3; ++j) stage(list, target[j], staging, data, offsets[j], raw[j]);
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
        // W-J's root signature: S-B's ten parameters, then root SRVs t7..t14 for the pose path.
        D3D12_ROOT_PARAMETER params[18]{}; params[0].ParameterType = D3D12_ROOT_PARAMETER_TYPE_CBV;
        params[0].Descriptor.ShaderRegister = 0;
        for (UINT i = 1; i <= 7; ++i) { params[i].ParameterType = D3D12_ROOT_PARAMETER_TYPE_SRV; params[i].Descriptor.ShaderRegister = i - 1; }
        for (UINT i = 8; i <= 9; ++i) { params[i].ParameterType = D3D12_ROOT_PARAMETER_TYPE_UAV; params[i].Descriptor.ShaderRegister = i - 8; }
        for (UINT i = 10; i < 18; ++i) { params[i].ParameterType = D3D12_ROOT_PARAMETER_TYPE_SRV; params[i].Descriptor.ShaderRegister = i - 3; }
        D3D12_ROOT_SIGNATURE_DESC d{}; d.NumParameters = 18; d.pParameters = params;
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
    ComPtr<ID3D12PipelineState> compute(const std::vector<uint8_t>& cs, const wchar_t* label) {
        D3D12_COMPUTE_PIPELINE_STATE_DESC p{}; p.pRootSignature = root.Get(); p.CS = {cs.data(), cs.size()};
        ComPtr<ID3D12PipelineState> out;
        hr(device, device->CreateComputePipelineState(&p, IID_PPV_ARGS(&out)), "CreateComputePipelineState(scene)");
        name(out.Get(), label);
        return out;
    }
    void create(const SceneRecord& record) {
        const auto directory = module_path().parent_path();
        // W1..W4 load the four draw/count/geometry blobs, as in ABI 2; W-J also loads lattice_cs.dxil.
        std::array<std::vector<uint8_t>, 5> shaders;
        for (size_t i = 0; i < (jumble ? shaders.size() : 4); ++i) shaders[i] = scene_bytes(directory / shader_names()[i]);
        make_root(); draw_pso = graphics(shaders[2], shaders[1]); count_pso = graphics(shaders[0], shaders[1]);
        compute_pso = compute(shaders[3], L"SA2 geometry compute pipeline");
        if (jumble) lattice_pso = compute(shaders[4], L"SA2 lattice compute pipeline");
        auto put = [&](const auto& values) { return static_buffer(values.data(), values.size() * sizeof(values[0])); };
        auto model = [&](const auto& a) {
            geometry[0] = put(a.vertices); geometry[1] = put(a.local); geometry[2] = put(a.centers);
            geometry[3] = put(a.frames); geometry[4] = put(a.flags); geometry[5] = put(sample);
            for (auto& label : label_buffers) label = put(a.oracle[0]);
            solved = put(a.oracle[0]);
            extra[4] = put(a.offsets);
        };
        if (jumble) model(*jumble); else model(*assets);
        // The pose path's inputs as W-J's constructor makes them; W1..W4 get W-J's W3
        // inputs: identity poses, no straddles, one zero overlay point.
        wj::PoseState start;
        if (jumble) start = jumble->poses[0];
        else { start.index.assign(wj::Pieces, 0); start.lattice = {0}; const auto id = wj::identity(); start.rows.assign(id.begin(), id.end()); }
        const auto raw = start.raw(); const uint64_t sizes[] = {wj::IndexBytes, row_bytes, lattice_bytes};
        for (auto& ring : pose_buffers) for (size_t j = 0; j < 3; ++j) {
            std::vector<uint8_t> padded(size_t(sizes[j]), 0); std::copy(raw[j].begin(), raw[j].end(), padded.begin()); ring[j] = put(padded);
        }
        pose_counts.fill(start.count());
        extra[0] = put(jumble ? jumble->slotPiece : scene_slot_pieces());
        std::vector<uint32_t> destination(wj::Slots); for (uint32_t i = 0; i < wj::Slots; ++i) destination[i] = i;
        extra[1] = put(destination);
        auto points = jumble ? jumble->certificates : std::vector<wj::OverlayPoint>{}; if (points.empty()) points.push_back({});
        extra[2] = put(points);
        extra[3] = put(jumble ? jumble->straddle : std::vector<uint32_t>(wj::Pieces));
        for (size_t i = 0; i < cb.size(); ++i) {
            cb[i] = buffer(256, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
            // A W-J ring slot stages a whole bundle: labels, index, matrix rows and lattice table.
            upload[i] = buffer(jumble ? stride : sb::LabelBytes, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
            cb_data[i] = map(cb[i]); upload_data[i] = map(upload[i]);
        }
        geometry_cb = buffer(256, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ); geometry_cb_data = map(geometry_cb);
        result = buffer(sample.size() * 12, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
        result_read = buffer(result.size, D3D12_HEAP_TYPE_READBACK, D3D12_RESOURCE_STATE_COPY_DEST);
        counter = buffer(sb::Cells * 4, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
        counter_read = buffer(counter.size, D3D12_HEAP_TYPE_READBACK, D3D12_RESOURCE_STATE_COPY_DEST);
        zero = buffer(counter.size, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
        std::memset(map(zero), 0, static_cast<size_t>(zero.size)); zero.resource->Unmap(0, nullptr);
        if (jumble) {
            // Owned check inputs, so the checks never touch a ring resource.
            check_label = buffer(wj::LabelBytes, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_COPY_DEST);
            check_destination = buffer(wj::LabelBytes, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_COPY_DEST);
            for (size_t j = 0; j < 3; ++j) check_pose[j] = buffer(sizes[j], D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_COPY_DEST);
            check_upload = buffer(stride + wj::LabelBytes, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ); check_upload_data = map(check_upload);
            lattice_result = buffer(uint64_t(wj::Slots) * 16, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
            lattice_read = buffer(lattice_result.size, D3D12_HEAP_TYPE_READBACK, D3D12_RESOURCE_STATE_COPY_DEST);
        }
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
        if (record.config.scene >= 3) {
            // S-B's rule: the run's whole preserved readback exists before the
            // trace, so nothing is allocated inside the measured window. W-J
            // preserves the whole bundle per copy (stride), W3/W4 the labels.
            readback_capacity = label_copies(record.config);
            try { readbacks.push_back(buffer(readback_capacity * stride, D3D12_HEAP_TYPE_READBACK, D3D12_RESOURCE_STATE_COPY_DEST)); }
            catch (const Failure& error) {
                throw Failure(error.status, std::string("cannot allocate the run's preserved ") + (jumble ? "bundle" : "label") + " readback buffer ("
                    + std::to_string(readback_capacity * stride / 1048576) + " MiB): " + error.what());
            }
        }
    }
    void bind(ID3D12GraphicsCommandList* list, const Constants& c, Buffer& constants_buffer, uint8_t* data,
              Buffer& label, std::array<Buffer, 3>& pose, bool compute, Buffer* destination = nullptr) {
        std::memcpy(data, &c, sizeof(c));
        if (compute) { list->SetComputeRootSignature(root.Get()); list->SetComputeRootConstantBufferView(0, constants_buffer.resource->GetGPUVirtualAddress()); }
        else { list->SetGraphicsRootSignature(root.Get()); list->SetGraphicsRootConstantBufferView(0, constants_buffer.resource->GetGPUVirtualAddress()); }
        for (UINT i = 0; i < 7; ++i) {
            auto address = (i < 5 ? geometry[i] : i == 5 ? label : geometry[5]).resource->GetGPUVirtualAddress();
            if (compute) list->SetComputeRootShaderResourceView(i + 1, address); else list->SetGraphicsRootShaderResourceView(i + 1, address);
        }
        // t7..t14 in W-J's order: slot -> piece, pose index, rows, lattice, destination, overlay points, straddles, offsets.
        const Buffer* additional[] = {&extra[0], &pose[0], &pose[1], &pose[2], destination ? destination : &extra[1], &extra[2], &extra[3], &extra[4]};
        for (UINT i = 0; i < 8; ++i) {
            auto address = additional[i]->resource->GetGPUVirtualAddress();
            if (compute) list->SetComputeRootShaderResourceView(i + 10, address); else list->SetGraphicsRootShaderResourceView(i + 10, address);
        }
    }
    void draw(ID3D12GraphicsCommandList* list, const Constants& c, Buffer& constants_buffer, uint8_t* data,
              Buffer& label, std::array<Buffer, 3>& pose, ID3D12PipelineState* pipeline, D3D12_CPU_DESCRIPTOR_HANDLE rtv,
              UINT width, UINT height, bool count) {
        list->SetPipelineState(pipeline); bind(list, c, constants_buffer, data, label, pose, false);
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
    // One frame of W-J's runGpu (renderer-wj/gpu.cpp) on the host's frame and trace window.
    void record_jumbling(SceneRecord& record, ID3D12GraphicsCommandList* list, uint32_t slot, D3D12_CPU_DESCRIPTOR_HANDLE target, int64_t now) {
        const auto& a = *jumble;
        const bool timed = record.tracing();
        const auto frame = uint64_t(record.trace.size()); wj::Turn turn{0, 0, 0}; WjStep step;
        if (timed) {
            turn = wj::turnAt(now, record.start, wj::turnTicks(record.config.turn_ms, record.frequency), a.angle);
            step = protocol.timed(frame, turn.index);
            if (step.upload) {
                // W-J's labels never move; the pose bundle alternates sweep-before/after by revision parity.
                const auto e = protocol.expected;
                stage(list, label_buffers[e], upload[slot], upload_data[slot], 0,
                    std::span(reinterpret_cast<const uint8_t*>(a.oracle[0].data()), size_t(wj::LabelBytes)));
                const auto& pose = step.fault ? injected : a.poses[protocol.authoritative % 2];
                upload_pose(list, pose_buffers[e], upload[slot], upload_data[slot], wj::LabelBytes, pose);
                pose_counts[e] = pose.count();
                record.injection_applied = protocol.injection_applied;
            }
        }
        sb::rotate(q, 0, 3, 0.002); ++camera;
        const auto c = constants(shape, q, float(record.width) / float(record.height), timed ? float(turn.theta) : 0);
        const auto b = protocol.bound;
        draw(list, c, cb[slot], cb_data[slot], label_buffers[b], pose_buffers[b], draw_pso.Get(), target, record.width, record.height, false);
        if (step.copy) {
            const auto index = uint64_t(protocol.records.copies.size());
            scene_require(index < readback_capacity, "preserved bundle readback capacity exceeded; end the trace within trace_ms");
            const auto count = pose_counts[b];
            auto copy = [&](Buffer& source, uint64_t offset, uint64_t size) {
                transition(list, source, D3D12_RESOURCE_STATE_COPY_SOURCE);
                list->CopyBufferRegion(readbacks[0].resource.Get(), index * stride + offset, source.resource.Get(), 0, size);
                transition(list, source, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
            };
            // Exactly the label and three pose root SRVs the draw bound (W-J's copyBound).
            copy(label_buffers[b], 0, wj::LabelBytes); copy(pose_buffers[b][0], wj::LabelBytes, wj::IndexBytes);
            copy(pose_buffers[b][1], wj::LabelBytes + wj::IndexBytes, uint64_t(count) * 64);
            copy(pose_buffers[b][2], wj::LabelBytes + wj::IndexBytes + row_bytes, uint64_t(count) * 4);
            protocol.copied(frame, index, count);
        }
        if (timed) record.trace.push_back({frame, now, protocol.revision[b], true, turn.index, turn.phase, camera});
    }
    // W-J's geometryCheck: every reference, sample and stage export is read and checked before GPU work.
    Json jumbling_geometry(ID3D12CommandQueue* queue, uint32_t timeout, const std::function<void()>& submitting) {
        const auto& a = *jumble; const auto reference = wj::repoRoot() / "work/experiments/renderer-wj";
        const auto prefix = "ref_" + a.opt.menu + "_";
        const auto raw_index = scene_bytes(reference / (prefix + "index.json"));
        Json index;
        refused([&] { index = Json::parse(std::string(raw_index.begin(), raw_index.end())); });
        refused([&] {
            for (const auto& [file, digest] : index.at("files").object())
                scene_require(wj::sha256(scene_bytes(reference / std::filesystem::path(file).filename())) == digest.string(),
                    "immutable geometry reference hash mismatch", SA2_E_CHECK_FAILED);
        });
        const auto supplied = scene_bytes(reference / (prefix + "sample.u32"));
        scene_require(supplied.size() == sample.size() * 4 && std::memcmp(supplied.data(), sample.data(), supplied.size()) == 0,
            "geometry sample differs from reference rule", SA2_E_CHECK_FAILED);
        struct Output { std::string file; sb::Matrix q; float theta; wj::PoseState pose; std::vector<float> expected; };
        std::vector<Output> outputs;
        refused([&] {
            for (const auto& output : index.at("outputs").array()) {
                Output o{output.at("file").string(), wj::identity(), float(output.at("theta").number()), {}, {}};
                bool found = false;
                for (const auto& camera_entry : a.cameras.at("cameras").array()) if (camera_entry.at("name").string() == output.at("camera").string()) {
                    for (const auto& r : camera_entry.at("rotations").array()) wj::rotate(o.q, int(r.array()[0].integer()), int(r.array()[1].integer()), r.array()[2].number());
                    found = true;
                }
                wj::require(found, "reference names unknown camera");
                o.pose = a.poseState(output.at("stage").string());
                wj::require(o.pose.count() <= a.maxPoses, "pose table exceeds allocated capacity");
                const auto raw = scene_bytes(reference / std::filesystem::path(o.file).filename());
                scene_require(raw.size() == result.size, "geometry reference length mismatch", SA2_E_CHECK_FAILED);
                o.expected.resize(raw.size() / 4); std::memcpy(o.expected.data(), raw.data(), raw.size());
                outputs.push_back(std::move(o));
            }
        });
        begin(timeout); auto* list = check_list.Get();
        transition(list, counter, D3D12_RESOURCE_STATE_COPY_DEST);
        list->CopyBufferRegion(counter.resource.Get(), 0, zero.resource.Get(), 0, counter.size);
        transition(list, counter, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
        // W-J's count draw binds its first ring bundle, poses[0].
        upload_pose(list, check_pose, check_upload, check_upload_data, wj::LabelBytes, a.poses[0]);
        draw(list, constants(shape, wj::identity(), 1.6f, 0), geometry_cb, geometry_cb_data, solved, check_pose,
            count_pso.Get(), check_rtv_heap->GetCPUDescriptorHandleForHeapStart(), 2560, 1600, true);
        transition(list, counter, D3D12_RESOURCE_STATE_COPY_SOURCE);
        list->CopyBufferRegion(counter_read.resource.Get(), 0, counter.resource.Get(), 0, counter.size);
        submit(queue, timeout, submitting);
        void* data = nullptr; D3D12_RANGE count_range{0, SIZE_T(counter.size)};
        hr(device, counter_read.resource->Map(0, &count_range, &data), "Map(geometry counts)");
        std::array<uint32_t, sb::Cells> counts{}; std::memcpy(counts.data(), data, counts.size() * 4);
        D3D12_RANGE none{0, 0}; counter_read.resource->Unmap(0, &none);
        uint64_t count_failures = 0;
        for (auto count : counts) if (count != sb::Vertices) ++count_failures;
        bool failed = count_failures != 0; Json::Array checks;
        for (const auto& o : outputs) {
            begin(timeout); upload_pose(list, check_pose, check_upload, check_upload_data, wj::LabelBytes, o.pose);
            list->SetPipelineState(compute_pso.Get());
            bind(list, constants(shape, o.q, 1.6f, o.theta, uint32_t(sample.size())), geometry_cb, geometry_cb_data, solved, check_pose, true);
            list->SetComputeRootUnorderedAccessView(9, result.resource->GetGPUVirtualAddress()); list->Dispatch(UINT((sample.size() + 63) / 64), 1, 1);
            transition(list, result, D3D12_RESOURCE_STATE_COPY_SOURCE);
            list->CopyBufferRegion(result_read.resource.Get(), 0, result.resource.Get(), 0, result.size);
            transition(list, result, D3D12_RESOURCE_STATE_UNORDERED_ACCESS); submit(queue, timeout, submitting);
            D3D12_RANGE range{0, SIZE_T(result.size)};
            std::vector<float> values(o.expected.size());
            hr(device, result_read.resource->Map(0, &range, &data), "Map(geometry projections)");
            std::memcpy(values.data(), data, values.size() * 4); result_read.resource->Unmap(0, &none);
            double max_error = 0; uint64_t failures = 0;
            for (size_t i = 0; i < values.size(); ++i) {
                const double error = std::abs(double(values[i]) - o.expected[i]);
                if (!std::isfinite(values[i]) || error > 1e-4 + 1e-4 * std::abs(double(o.expected[i]))) ++failures;
                max_error = std::max(max_error, error);
            }
            failed |= failures != 0;
            checks.emplace_back(Json::Object{{"file", o.file}, {"samples", uint64_t(sample.size())}, {"max_abs_error", max_error},
                {"failures", failures}, {"status", failures ? "fail" : "pass"}});
        }
        return Json::Object{{"format", "magic600-wj-geometry-check/1"}, {"status", failed ? "fail" : "pass"}, {"build_identity", build_identity()},
            {"adapter", adapter_name}, {"results", checks}, {"cells", sb::Cells}, {"vertices_per_cell", sb::Vertices},
            {"invocation_count_failures", count_failures}};
    }
    // W-J's latticeCheck: both controls are read and checked before GPU work.
    Json jumbling_lattice(ID3D12CommandQueue* queue, uint32_t timeout, const std::function<void()>& submitting) {
        const auto& a = *jumble;
        struct Control { std::string stage; std::vector<uint8_t> labels, destination; wj::PoseState pose; };
        std::vector<Control> controls;
        refused([&] {
            for (const char* stage_name : {"lattice-start", "lattice-retained"}) {
                const auto directory = a.opt.data / stage_name; const auto header = wj::readJson(directory / "header.json");
                Control control{stage_name, wj::bytes(directory / "labels.u32"), wj::bytes(directory / "destination.u32"), {}};
                wj::require(control.labels.size() == wj::LabelBytes && control.destination.size() == wj::LabelBytes, "lattice control length mismatch");
                wj::require(wj::sha256(control.labels) == header.at("files").at("labels.u32").at("sha256").string()
                    && wj::sha256(control.destination) == header.at("files").at("destination.u32").at("sha256").string(), "lattice control hash mismatch");
                control.pose = a.poseState(stage_name);
                wj::require(control.pose.count() <= a.maxPoses, "pose table exceeds allocated capacity");
                controls.push_back(std::move(control));
            }
        });
        Json::Array states; bool failed = false; auto* list = check_list.Get();
        for (const auto& control : controls) {
            begin(timeout);
            stage(list, check_label, check_upload, check_upload_data, 0, control.labels);
            upload_pose(list, check_pose, check_upload, check_upload_data, wj::LabelBytes, control.pose);
            stage(list, check_destination, check_upload, check_upload_data, stride, control.destination);
            bind(list, constants(shape, wj::identity(), 1.6f, 0), geometry_cb, geometry_cb_data, check_label, check_pose, true, &check_destination);
            list->SetPipelineState(lattice_pso.Get());
            list->SetComputeRootUnorderedAccessView(9, lattice_result.resource->GetGPUVirtualAddress()); list->Dispatch((wj::Slots + 63) / 64, 1, 1);
            transition(list, lattice_result, D3D12_RESOURCE_STATE_COPY_SOURCE);
            list->CopyBufferRegion(lattice_read.resource.Get(), 0, lattice_result.resource.Get(), 0, lattice_result.size);
            transition(list, lattice_result, D3D12_RESOURCE_STATE_UNORDERED_ACCESS); submit(queue, timeout, submitting);
            void* data = nullptr; D3D12_RANGE range{0, SIZE_T(lattice_result.size)};
            hr(device, lattice_read.resource->Map(0, &range, &data), "Map(lattice results)");
            const auto* values = static_cast<const uint32_t*>(data); std::array<uint64_t, 4> errors{};
            for (uint32_t slot = 0; slot < wj::Slots; ++slot) for (size_t j = 0; j < 4; ++j) errors[j] += values[size_t(slot) * 4 + j] != 0;
            D3D12_RANGE none{0, 0}; lattice_read.resource->Unmap(0, &none);
            const bool bad = std::any_of(errors.begin(), errors.end(), [](uint64_t e) { return e != 0; }); failed |= bad;
            states.emplace_back(Json::Object{{"state", control.stage}, {"status", bad ? "fail" : "pass"}, {"slots", wj::Slots},
                {"label_failures", errors[0]}, {"center_failures", errors[1]}, {"geometry_bounds_failures", errors[2]}, {"lattice_failures", errors[3]}});
        }
        return Json::Object{{"format", "magic600-wj-lattice-check/1"}, {"status", failed ? "fail" : "pass"}, {"build_identity", build_identity()},
            {"adapter", adapter_name}, {"states", states}};
    }
};

Scene::Scene(ID3D12Device* device, const sa2_scene_config& config, uint32_t width, uint32_t height)
    : record(config, frequency(), width, height) {
    const auto ticks = std::llround((config.scene == 3 ? config.turn_ms : 190) * double(record.frequency) / 1000);
    scene_require(ticks > 0, "turn_ms is below QPC resolution", SA2_E_INVALID_ARGUMENT);
    gpu = std::make_unique<Impl>(device, record); gpu->create(record);
}
Scene::Scene(ID3D12Device* device, const sa2_wj_config& config, uint32_t width, uint32_t height)
    : record(config, frequency(), width, height) {
    const auto ticks = std::llround(config.turn_ms * double(record.frequency) / 1000);
    scene_require(ticks > 0, "turn_ms is below QPC resolution", SA2_E_INVALID_ARGUMENT);
    gpu = std::make_unique<Impl>(device, record); gpu->create(record);
}
Scene::~Scene() = default;
void Scene::poison() { gpu->healthy = false; }
void Scene::require_healthy() const { scene_require(gpu->healthy, "scene recording failed; drain and unload before retrying"); }
void Scene::wait_geometry(uint32_t timeout) { gpu->wait(timeout); }
void Scene::record_draw(ID3D12GraphicsCommandList* list, uint32_t slot, D3D12_CPU_DESCRIPTOR_HANDLE target, int64_t now) {
    auto& g = *gpu; g.initialize(list);
    if (g.jumble) { g.record_jumbling(record, list, slot, target, now); return; }
    const bool timed = record.tracing(), label_scene = record.config.scene >= 3;
    const auto frame = uint64_t(record.trace.size()); sb::Turn turn{0, 0, 0};
    if (timed) turn = sb::turnAt(now, record.start, sb::turnTicks(record.config.scene == 3 ? record.config.turn_ms : 190, record.frequency), g.assets->angle);
    bool copy = false;
    if (label_scene && timed) {
        if (turn.index > g.authoritative) {
            g.pending.reset(); const auto old = g.expected; g.expected = (g.expected + 1) % 3;
            while (g.authoritative < turn.index) { g.assets->move(g.authoritative_labels, g.authoritative); ++g.authoritative; }
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
    const auto c = constants(g.shape, g.q, float(record.width) / float(record.height), record.config.scene == 3 && timed ? float(turn.theta) : 0);
    // W1..W4 bind identity poses (never re-uploaded) through W-J's pose path.
    g.draw(list, c, g.cb[slot], g.cb_data[slot], g.label_buffers[g.bound], g.pose_buffers[g.bound], g.draw_pso.Get(), target, record.width, record.height, false);
    if (copy) {
        const auto index = uint64_t(g.records.copies.size());
        scene_require(index < g.readback_capacity, "preserved label readback capacity exceeded; end the trace within trace_ms");
        auto& label = g.label_buffers[g.bound]; g.transition(list, label, D3D12_RESOURCE_STATE_COPY_SOURCE);
        list->CopyBufferRegion(g.readbacks[0].resource.Get(), index * sb::LabelBytes, label.resource.Get(), 0, sb::LabelBytes);
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
Json Scene::labels(Json& poses) {
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
    if (g.jumble) {
        // W-J's exact checks over the preserved bundles (renderer-wj/gpu.cpp runGpu).
        const auto* base = reinterpret_cast<const uint8_t*>(copied.at(0));
        auto check = wj::checkLabels(*g.jumble, g.protocol.records, [&](const wj::Copy& c) {
            return std::span<const uint32_t>(reinterpret_cast<const uint32_t*>(base + c.slot * g.stride), wj::Slots);
        });
        poses = wj::checkPoses(*g.jumble, g.protocol.records, [&](const wj::Copy& c) -> wj::PoseBytes {
            const auto* bundle = base + c.slot * g.stride + wj::LabelBytes;
            return {std::span(bundle, size_t(wj::IndexBytes)), std::span(bundle + wj::IndexBytes, size_t(c.poses) * 64),
                    std::span(bundle + wj::IndexBytes + g.row_bytes, size_t(c.poses) * 4)};
        });
        g.protocol.finish(check, poses);
        return check;
    }
    auto check = sb::checkLabels(*g.assets, g.records, [&](const sb::Copy& c) {
        return std::span<const uint32_t>(copied.at(0) + c.slot * sb::Slots, sb::Slots);
    });
    if (!record.injection.empty() && !record.injection_applied) { check["status"] = "fail"; check["injection_not_reached"] = true; }
    return check;
}
Json Scene::geometry_check(ID3D12CommandQueue* queue, uint32_t timeout, const std::function<void()>& submitting) {
    auto& g = *gpu;
    if (g.jumble) return g.jumbling_geometry(queue, timeout, submitting);
    const auto reference = sb::repoRoot() / "work/experiments/renderer-sb/reference";
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
    g.draw(list, constants(g.shape, sb::identity(), 1.6f, 0), g.geometry_cb, g.geometry_cb_data, g.solved, g.pose_buffers[0],
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
    for (const auto& camera : g.assets->cameras.at("cameras").array()) {
        auto q = sb::identity();
        for (const auto& r : camera.at("rotations").array()) sb::rotate(q, int(r.array()[0].integer()), int(r.array()[1].integer()), r.array()[2].number());
        const std::array<std::pair<const char*, double>, 3> poses{{{"start", 0}, {"mid", g.assets->angle * 0.5}, {"end", g.assets->angle}}};
        for (const auto& [pose, theta] : poses) {
            const auto path = reference / (camera.at("name").string() + '_' + pose + ".f32");
            const auto raw = scene_bytes(path);
            scene_require(raw.size() == g.result.size, "reference projection length mismatch: " + path.filename().string(), SA2_E_CHECK_FAILED);
            std::vector<float> ref(raw.size() / 4); std::memcpy(ref.data(), raw.data(), raw.size());
            g.begin(timeout); list->SetPipelineState(g.compute_pso.Get());
            g.bind(list, constants(g.shape, q, 1.6f, float(theta), uint32_t(g.sample.size())), g.geometry_cb, g.geometry_cb_data, g.solved, g.pose_buffers[0], true);
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
    return Json::Object{{"format", "magic600-sb-geometry-check-v1"}, {"status", failed ? "fail" : "pass"},
        {"build_identity", build_identity()},
        {"adapter", g.adapter_name}, {"results", results},
        {"per_cell_count", Json::Object{{"cells", sb::Cells}, {"expected", sb::Vertices}, {"failures", count_failures}, {"counts", count_array}, {"status", count_failures ? "fail" : "pass"}}}};
}
Json Scene::geometry_error(const std::string& error) const {
    if (gpu->jumble)
        return Json::Object{{"format", "magic600-wj-geometry-check/1"}, {"status", "fail"}, {"build_identity", build_identity()},
            {"adapter", gpu->adapter_name}, {"error", error}, {"results", Json::Array{}}, {"cells", sb::Cells},
            {"vertices_per_cell", sb::Vertices}, {"invocation_count_failures", Json()}};
    return Json::Object{{"format", "magic600-sb-geometry-check-v1"}, {"status", "fail"},
        {"build_identity", build_identity()},
        {"adapter", gpu->adapter_name}, {"error", error}, {"results", Json::Array{}},
        {"per_cell_count", Json::Object{{"cells", sb::Cells}, {"expected", sb::Vertices}, {"status", "not-checked"}, {"counts", Json()}, {"failures", Json()}}}};
}
Json Scene::lattice_check(ID3D12CommandQueue* queue, uint32_t timeout, const std::function<void()>& submitting) {
    record.require_lattice();
    return gpu->jumbling_lattice(queue, timeout, submitting);
}
Json Scene::lattice_error(const std::string& error) const {
    return Json::Object{{"format", "magic600-wj-lattice-check/1"}, {"status", "fail"}, {"build_identity", build_identity()},
        {"adapter", gpu->adapter_name}, {"error", error}, {"states", Json::Array{}}};
}
}
