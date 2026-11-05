// LX-7 experiment only. No window, swap chain, renderer, session or timing claim.
// Arguments: shader.dxil inputs.bin outputs.bin adapter.json exact-adapter-name
#define NOMINMAX
#include <windows.h>
#include <d3d12.h>
#include <dxgi1_6.h>
#include <wrl/client.h>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

using Microsoft::WRL::ComPtr;

static void require(bool good, const char* message) {
    if (!good) throw std::runtime_error(message);
}
static void check(HRESULT hr, const char* operation) {
    if (FAILED(hr)) throw std::runtime_error(std::string(operation) + " failed (HRESULT " + std::to_string(uint32_t(hr)) + ")");
}
static std::vector<char> read(const char* path) {
    std::ifstream f(path, std::ios::binary | std::ios::ate);
    require(bool(f), "input open failed");
    auto size = f.tellg();
    require(size > 0 && size <= 64 * 1024 * 1024, "input size outside harness bound");
    std::vector<char> bytes(static_cast<size_t>(size));
    f.seekg(0); f.read(bytes.data(), std::streamsize(bytes.size()));
    require(bool(f), "input read failed");
    return bytes;
}
static std::string utf8(const wchar_t* s) {
    int n = WideCharToMultiByte(CP_UTF8, 0, s, -1, nullptr, 0, nullptr, nullptr);
    require(n > 0, "adapter name conversion failed");
    std::string result(size_t(n), '\0');
    require(WideCharToMultiByte(CP_UTF8, 0, s, -1, result.data(), n, nullptr, nullptr) == n, "adapter name conversion failed");
    result.pop_back(); return result;
}
static std::string quote(const std::string& s) {
    std::string out = "\"";
    for (unsigned char c : s) {
        if (c == '\\' || c == '"') out += '\\';
        require(c >= 32, "control character in adapter description");
        out += char(c);
    }
    return out + "\"";
}
static ComPtr<ID3D12Resource> buffer(ID3D12Device* dev, UINT64 size, D3D12_HEAP_TYPE type,
                                    D3D12_RESOURCE_STATES state, D3D12_RESOURCE_FLAGS flags = D3D12_RESOURCE_FLAG_NONE) {
    D3D12_HEAP_PROPERTIES heap{}; heap.Type = type; heap.CreationNodeMask = heap.VisibleNodeMask = 1;
    D3D12_RESOURCE_DESC desc{}; desc.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
    desc.Width = size; desc.Height = 1; desc.DepthOrArraySize = 1; desc.MipLevels = 1;
    desc.SampleDesc.Count = 1; desc.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR; desc.Flags = flags;
    ComPtr<ID3D12Resource> result;
    check(dev->CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &desc, state, nullptr, IID_PPV_ARGS(&result)), "buffer creation");
    return result;
}
static void transition(ID3D12GraphicsCommandList* list, ID3D12Resource* resource,
                       D3D12_RESOURCE_STATES from, D3D12_RESOURCE_STATES to) {
    D3D12_RESOURCE_BARRIER b{}; b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    b.Transition.pResource = resource; b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    b.Transition.StateBefore = from; b.Transition.StateAfter = to; list->ResourceBarrier(1, &b);
}
static void upload(ID3D12Resource* resource, const void* data, size_t size) {
    void* mapped = nullptr; D3D12_RANGE none{0, 0};
    check(resource->Map(0, &none, &mapped), "upload map");
    std::memcpy(mapped, data, size); resource->Unmap(0, nullptr);
}

int main(int argc, char** argv) {
    try {
        require(argc == 6, "usage: chart_device shader inputs outputs adapter-json exact-adapter-name");
        const auto shader = read(argv[1]), input = read(argv[2]);
        require(input.size() % 96 == 0 && input.size() / 96 <= 65535 * 64, "input record size/count invalid");
        const UINT count = UINT(input.size() / 96);
        const size_t outputSize = size_t(count) * 16;
        ComPtr<IDXGIFactory6> factory; check(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)), "DXGI factory");
        ComPtr<IDXGIAdapter1> adapter; DXGI_ADAPTER_DESC1 desc{};
        for (UINT i = 0; ; ++i) {
            ComPtr<IDXGIAdapter1> candidate;
            HRESULT hr = factory->EnumAdapterByGpuPreference(i, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE, IID_PPV_ARGS(&candidate));
            if (hr == DXGI_ERROR_NOT_FOUND) break;
            check(hr, "adapter enumeration");
            check(candidate->GetDesc1(&desc), "adapter description");
            if (!(desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) && utf8(desc.Description) == argv[5]) { adapter = candidate; break; }
        }
        require(bool(adapter), "requested hardware adapter not found; no fallback");
        check(adapter->GetDesc1(&desc), "selected adapter description");
        ComPtr<ID3D12Device> device;
        check(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_12_0, IID_PPV_ARGS(&device)), "D3D12 device");
        D3D12_FEATURE_DATA_SHADER_MODEL sm{D3D_SHADER_MODEL_6_2};
        check(device->CheckFeatureSupport(D3D12_FEATURE_SHADER_MODEL, &sm, sizeof sm), "shader model query");
        require(sm.HighestShaderModel >= D3D_SHADER_MODEL_6_2, "shader model 6.2 required");

        D3D12_ROOT_PARAMETER parameters[3]{};
        parameters[0].ParameterType = D3D12_ROOT_PARAMETER_TYPE_SRV; parameters[0].Descriptor.ShaderRegister = 0;
        parameters[1].ParameterType = D3D12_ROOT_PARAMETER_TYPE_UAV; parameters[1].Descriptor.ShaderRegister = 0;
        parameters[2].ParameterType = D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS;
        parameters[2].Constants.ShaderRegister = 0; parameters[2].Constants.Num32BitValues = 1;
        D3D12_ROOT_SIGNATURE_DESC signature{}; signature.NumParameters = 3; signature.pParameters = parameters;
        ComPtr<ID3DBlob> blob, errors;
        check(D3D12SerializeRootSignature(&signature, D3D_ROOT_SIGNATURE_VERSION_1, &blob, &errors), "root serialization");
        ComPtr<ID3D12RootSignature> root;
        check(device->CreateRootSignature(0, blob->GetBufferPointer(), blob->GetBufferSize(), IID_PPV_ARGS(&root)), "root signature");
        D3D12_COMPUTE_PIPELINE_STATE_DESC pd{}; pd.pRootSignature = root.Get(); pd.CS = {shader.data(), shader.size()};
        ComPtr<ID3D12PipelineState> pso;
        check(device->CreateComputePipelineState(&pd, IID_PPV_ARGS(&pso)), "compute pipeline");
        auto in = buffer(device.Get(), input.size(), D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
        auto init = buffer(device.Get(), outputSize, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
        auto out = buffer(device.Get(), outputSize, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
        auto readback = buffer(device.Get(), outputSize, D3D12_HEAP_TYPE_READBACK, D3D12_RESOURCE_STATE_COPY_DEST);
        upload(in.Get(), input.data(), input.size());
        std::vector<uint32_t> sentinel(size_t(count) * 4, 0xffffffffu); upload(init.Get(), sentinel.data(), outputSize);

        D3D12_COMMAND_QUEUE_DESC qd{}; qd.Type = D3D12_COMMAND_LIST_TYPE_COMPUTE;
        ComPtr<ID3D12CommandQueue> queue;
        check(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue)), "compute queue");
        ComPtr<ID3D12CommandAllocator> allocator;
        check(device->CreateCommandAllocator(qd.Type, IID_PPV_ARGS(&allocator)), "allocator");
        ComPtr<ID3D12GraphicsCommandList> list;
        check(device->CreateCommandList(0, qd.Type, allocator.Get(), pso.Get(), IID_PPV_ARGS(&list)), "command list");
        list->CopyBufferRegion(out.Get(), 0, init.Get(), 0, outputSize);
        transition(list.Get(), out.Get(), D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
        list->SetComputeRootSignature(root.Get());
        list->SetComputeRootShaderResourceView(0, in->GetGPUVirtualAddress());
        list->SetComputeRootUnorderedAccessView(1, out->GetGPUVirtualAddress());
        list->SetComputeRoot32BitConstant(2, count, 0); list->Dispatch((count + 63) / 64, 1, 1);
        transition(list.Get(), out.Get(), D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_COPY_SOURCE);
        list->CopyBufferRegion(readback.Get(), 0, out.Get(), 0, outputSize); check(list->Close(), "list close");
        ID3D12CommandList* submitted[] = {list.Get()}; queue->ExecuteCommandLists(1, submitted);
        ComPtr<ID3D12Fence> fence;
        check(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)), "fence");
        HANDLE event = CreateEventW(nullptr, FALSE, FALSE, nullptr); require(event != nullptr, "fence event failed");
        check(queue->Signal(fence.Get(), 1), "queue signal");
        check(fence->SetEventOnCompletion(1, event), "fence completion");
        DWORD wait = WaitForSingleObject(event, 30000); CloseHandle(event);
        require(wait == WAIT_OBJECT_0, "fence wait failed or timed out");
        check(device->GetDeviceRemovedReason(), "device status after readback");
        void* mapped = nullptr; D3D12_RANGE range{0, outputSize};
        check(readback->Map(0, &range, &mapped), "readback map");
        std::ofstream result(argv[3], std::ios::binary); result.write(static_cast<const char*>(mapped), std::streamsize(outputSize));
        result.close(); D3D12_RANGE none{0, 0}; readback->Unmap(0, &none); require(bool(result), "result write failed");
        LARGE_INTEGER version{};
        bool versionKnown = SUCCEEDED(adapter->CheckInterfaceSupport(__uuidof(IDXGIDevice), &version));
        std::string driver = "unavailable";
        if (versionKnown) {
            uint64_t v = uint64_t(version.QuadPart);
            driver = std::to_string((v >> 48) & 65535) + "." + std::to_string((v >> 32) & 65535) + "." +
                     std::to_string((v >> 16) & 65535) + "." + std::to_string(v & 65535);
        }
        std::ofstream metadata(argv[4], std::ios::binary);
        metadata << "{\n  \"adapter\": " << quote(utf8(desc.Description)) << ",\n  \"vendor_id\": " << desc.VendorId
                 << ",\n  \"device_id\": " << desc.DeviceId << ",\n  \"dedicated_video_memory\": " << desc.DedicatedVideoMemory
                 << ",\n  \"driver\": " << quote(driver) << ",\n  \"software\": false,\n  \"shader_model\": \"6.2\",\n  \"records\": " << count << "\n}\n";
        metadata.close(); require(bool(metadata), "adapter metadata write failed");
        std::cout << "Named hardware readback complete: " << count << " records. No performance result.\n";
        return 0;
    } catch (const std::exception& e) {
        std::cerr << e.what() << '\n'; return 1;
    }
}
