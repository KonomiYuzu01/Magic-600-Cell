#pragma once
#include "scene_record.h"
#include <d3d12.h>
#include <memory>
#include <functional>

namespace sa2 {
class Scene {
    struct Impl;
    std::unique_ptr<Impl> gpu;
public:
    SceneRecord record;
    Scene(ID3D12Device* device, const sa2_scene_config& config, uint32_t width, uint32_t height);
    ~Scene();
    // The caller has waited for this slot's prior submission before record_draw.
    void record_draw(ID3D12GraphicsCommandList* list, uint32_t slot, D3D12_CPU_DESCRIPTOR_HANDLE target, int64_t qpc);
    void sample_vram();
    Json labels(); // caller confirmed all scene submissions complete
    Json geometry_check(ID3D12CommandQueue* queue, uint32_t timeout, const std::function<void()>& submitting);
    Json geometry_error(const std::string& error) const;
    void wait_geometry(uint32_t timeout);
    void poison();
    void require_healthy() const;
};
}
