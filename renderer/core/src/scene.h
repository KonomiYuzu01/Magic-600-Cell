#pragma once
#include "scene_record.h"
#include "product.h"
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
    Scene(ID3D12Device* device, const sa2_scene_config& config, uint32_t width, uint32_t height);
    Scene(ID3D12Device* device, const sa2_wj_config& config, uint32_t width, uint32_t height);
    ~Scene();
    // The caller has waited for this slot's prior submission before record_draw.
    void record_draw(ID3D12GraphicsCommandList* list, uint32_t slot, D3D12_CPU_DESCRIPTOR_HANDLE target, int64_t qpc);
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
    void product_ready(ID3D12CommandQueue* queue, DXGI_FORMAT format = DXGI_FORMAT_R8G8B8A8_UNORM);
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
    // Records an ID pass and the texel copy with its own list and signals the product fence.
    uint64_t product_pick(uint64_t ticket, const PassInput& input, ID3D12CommandQueue* queue, const std::function<void()>& submitting);
    uint32_t product_pick_value(uint64_t ticket); // after the pick's fence completed
    void product_release_pick(uint64_t ticket);
    void product_release_picks(); // the device was removed: no ID pass can still run
    void product_release_set(uint64_t ticket);
    void product_collect(); // frees the staging of completed preparation uploads
    uint64_t product_completed() const; // the product fence's completed value; 0 before any product GPU work
    // Recreates the size-bound resources (depth and the ID target) at the new
    // size after a confirmed drain.
    void resize(uint32_t width, uint32_t height);
};
}
