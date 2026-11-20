#pragma once
#include "stage_rules.h"
#include <d3d12.h>
#include <wrl/client.h>
#include <array>
#include <map>
#include <memory>
#include <optional>

// C9 U5's device half: DXC at run time, the compiled stages of one device and the chain a scene draws with.
namespace sa2::stages {
using Microsoft::WRL::ComPtr;
// The stage root signature's parameters after the scene's eighteen (b0, t0..t6, u0..u1, t7..t14).
constexpr UINT FrameConstants = 18; // b1: eight 32-bit values (stages.hlsl M600StageFrame)
constexpr UINT FrameConstantCount = 8;
constexpr UINT ArrayTable = 19; // t20..t21: the vertex and fragment arrays
constexpr UINT EvalRead = 20; // t22
constexpr UINT EvalWrite = 21; // u2
constexpr UINT FrameWrite = 22; // u3
constexpr uint32_t Vertices = 30480, Triangles = Vertices / 3, Cells = 600; // W1..W4's instanced list
constexpr uint64_t EvalRecords = uint64_t(Vertices) * Cells, EvalBytes = EvalRecords * 16;
constexpr uint32_t EvalGroup = 64; // numthreads of m600_eval_* and m600_bound

// What a scene draws a user chain with: the device's stage root signature and passes, and the attached stages.
struct Chain {
    ComPtr<ID3D12RootSignature> root;
    ComPtr<ID3D12PipelineState> eval, bound, decide;
    std::array<ComPtr<ID3D12PipelineState>, 2> draw; // the user draw: [0] R8G8B8A8_UNORM, [1] B8G8R8A8_UNORM
    uint64_t vertex = 0, fragment = 0;
    uint32_t overdraw = 0;
    std::vector<uint32_t> vertex_arrays, fragment_arrays; // binary32 bit patterns
};

// The compiled stages of one device. Handles are numbered by the caller and never reused.
class Registry {
public:
    explicit Registry(ID3D12Device* device) : device(device) {}
    // Compiles one stage. Returns the status and fills the record; a failure keeps nothing.
    int32_t compile(uint32_t kind, std::string_view text, uint64_t handle, Json& record);
    // The chain for vertex and fragment (0: built-in); both handles must name stages of their kinds.
    std::shared_ptr<const Chain> chain(uint64_t vertex, uint64_t fragment, uint32_t overdraw,
                                       std::vector<uint32_t> vertex_arrays, std::vector<uint32_t> fragment_arrays) const;
    bool holds(uint64_t handle, uint32_t kind) const; // kind 0: any kind
    void release(uint64_t handle);
private:
    struct Stage {
        uint32_t kind = 0;
        ComPtr<ID3D12PipelineState> eval;
        std::array<ComPtr<ID3D12PipelineState>, 2> draw;
    };
    struct Core {
        ComPtr<ID3D12RootSignature> root;
        ComPtr<ID3D12PipelineState> eval, bound, decide;
        std::array<ComPtr<ID3D12PipelineState>, 2> draw; // m600_pass_vs with psMain
        std::vector<uint8_t> pass_vs;
    };
    ID3D12Device* device;
    std::map<uint64_t, Stage> stages;
    std::optional<Core> core;
    Json core_passes; // the core compile's pass records, after it succeeded
};
}
