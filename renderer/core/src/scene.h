#pragma once
#include "scene_record.h"
#include "product.h"
#include "stages.h"
#include <d3d12.h>
#include <memory>
#include <functional>

namespace sa2 {
class Scene {
    struct Impl;
    std::unique_ptr<Impl> gpu;
public:
    SceneRecord record;
    Product product; // ABI 4's device-free state; it lives and dies with the scene (an ABI 5 recovery moves it to the new scene)
    TracePresents surface_trace; // C9, owner item 9: route B's traced frames and their confirmations (presented.jsonl)
    Scene(ID3D12Device* device, const sa2_scene_config& config, uint32_t width, uint32_t height);
    Scene(ID3D12Device* device, const sa2_wj_config& config, uint32_t width, uint32_t height);
    ~Scene();
    // The caller has waited for this slot's prior submission before record_draw. format is the target's:
    // R8G8B8A8_UNORM (ring slot) or, after surface_ready, B8G8R8A8_UNORM (C9 U4: the route-B surface; slot is then
    // its back buffer's index, below three, so the per-slot buffers keep their one-frame-in-flight rule).
    void record_draw(ID3D12GraphicsCommandList* list, uint32_t slot, D3D12_CPU_DESCRIPTOR_HANDLE target, int64_t qpc,
                     DXGI_FORMAT format = DXGI_FORMAT_R8G8B8A8_UNORM);
    // C9 U4: builds the draw pipeline for the route-B surface once; a failure leaves the scene usable.
    void surface_ready();
    void sample_vram();
    Json labels(Json& poses); // caller confirmed all scene submissions complete; poses stays null outside W-J
    Json geometry_check(ID3D12CommandQueue* queue, uint32_t timeout, const std::function<void()>& submitting);
    Json geometry_error(const std::string& error) const;
    Json lattice_check(ID3D12CommandQueue* queue, uint32_t timeout, const std::function<void()>& submitting);
    Json lattice_error(const std::string& error) const;
    void wait_geometry(uint32_t timeout);
    void poison();
    void require_healthy() const;

    // ABI 4's GPU side. The product resources are created on the first
    // product call that needs them (product_ready), so W1..W4 and W-J runs
    // allocate nothing more. Uploads, frames and ID passes run on the
    // producer queue. A failure while creating resources leaves the scene
    // usable; a failure while recording or submitting poisons it.
    // ABI 5: a B8G8R8A8_UNORM format also builds the route-B surface pipeline.
    void product_ready(ID3D12CommandQueue* queue, DXGI_FORMAT format = DXGI_FORMAT_R8G8B8A8_UNORM, bool styled = false,
                       bool panes = false); // ABI 7: styled; ABI 8: panes
    // Records the preparation's upload with its own list and signals the
    // product fence; returns the fence value.
    uint64_t product_prepare(uint64_t ticket, const PreparedData& data, ID3D12CommandQueue* queue, const std::function<void()>& submitting);
    // Records one frame into the open list: the engine upload the plan asks
    // for, the draw with the frame's camera and the set's visibility, and the
    // ring's timestamp and pipeline-statistics queries. format selects the
    // pipeline: R8G8B8A8_UNORM (ring slot) or B8G8R8A8_UNORM (ABI 5 surface).
    void product_frame(ID3D12GraphicsCommandList* list, const sa2_frame_input& input, const FramePlan& plan,
                       D3D12_CPU_DESCRIPTOR_HANDLE target, uint32_t ring, DXGI_FORMAT format = DXGI_FORMAT_R8G8B8A8_UNORM);
    // The ring's last frame results, read after that frame completed: GPU nanoseconds and VSInvocations.
    std::pair<uint64_t, uint64_t> product_results(uint32_t ring);
    // ABI 8: the composite frame (sa2_frame_submit_panes) and its per-pane vertex evaluations.
    void product_frame_panes(ID3D12GraphicsCommandList* list, const sa2_frame_input& input, const FramePlan& plan,
                             D3D12_CPU_DESCRIPTOR_HANDLE target, uint32_t ring, DXGI_FORMAT format);
    std::vector<uint64_t> product_pane_evals(uint32_t ring);
    // Records an ID pass and the texel copy with its own list and signals the product fence.
    uint64_t product_pick(uint64_t ticket, const PassInput& input, ID3D12CommandQueue* queue, const std::function<void()>& submitting);
    uint32_t product_pick_value(uint64_t ticket); // after the pick's fence completed
    void product_release_pick(uint64_t ticket);
    void product_release_picks(); // the device was removed: no ID pass can still run
    // C9 (26 Nov 2026): with `reuse` (a live device) the set's resources are kept for the next preparation.
    void product_release_set(uint64_t ticket, bool reuse);
    void product_release_spares(); // the device was removed: no kept set may be reused
    void product_collect(); // frees the staging of completed preparation uploads
    uint64_t product_completed() const; // the product fence's completed value; 0 before any product GPU work
    // Recreates the size-bound resources (depth and the ID target) at the new
    // size after a confirmed drain.
    void resize(uint32_t width, uint32_t height);

    // C9 U5: a user chain on W1..W4's record_draw (sa2_scene_stages). Attaching creates the stage resources on the
    // first attach (the eval buffer, the frame row, its readback and the statistics queries) and the chain's arrays;
    // a failure leaves the chain as it was. A null chain detaches. qpc is the attach time, t = 0.
    void stages_attach(std::shared_ptr<const stages::Chain> chain, int64_t qpc);
    bool stages_use(uint64_t handle) const; // the attached chain names the handle
    // After record_draw drew ring's frame with the chain: the frame's number, for its report row.
    void stage_frame(uint32_t ring, uint64_t frame);
    // The newest completed frame drawn with a chain, from every row whose frame is at most completed; zeros (frame 0)
    // before the first.
    void stage_report(uint64_t completed, sa2_stage_frame& out);
};
}
