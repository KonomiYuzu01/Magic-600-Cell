#pragma once
// ABI 4's one seam between a product frame and where it is drawn and
// presented. sa2_frame_submit and sa2_resize see only this interface; the
// Godot-texture route (interop.cpp's SlotTarget, a registered ring slot that
// Godot composites) is its only implementation in ABI 4. A swap chain or a
// composition target would be another implementation, not a change to the
// product calls.
#include <d3d12.h>
#include <cstdint>

namespace sa2 {
class FrameTarget {
public:
    virtual ~FrameTarget() = default;
    virtual uint32_t width() const = 0;
    virtual uint32_t height() const = 0;
    // Selects the per-target constant buffer and query results; a target's
    // next frame is recorded only after its previous one completed (open).
    virtual uint32_t ring() const = 0;
    // Bounded CPU wait until an earlier frame (a value close signalled) completed.
    virtual void wait(uint64_t frame) = 0;
    // After the target's previous frame completed: a reset command list with
    // the target writable as a render target.
    virtual ID3D12GraphicsCommandList* open() = 0;
    virtual D3D12_CPU_DESCRIPTOR_HANDLE view() const = 0;
    // Hands the target over to its consumer, executes the list and signals frame.
    virtual void close(uint64_t frame) = 0;
};
}
