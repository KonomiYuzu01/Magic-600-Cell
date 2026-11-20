#pragma once
// ABI 4's product state without a device: argument validation, the adopted
// engine image, the prepare/poll/activate/cancel/retire tickets, frame stamps
// and reports, picks and their staleness, counters and device loss; ABI 5
// adds the present ledger and the recovery bookkeeping (route B). The GPU
// work lives in scene.cpp; interop.cpp joins the two. Everything here is
// covered by the CPU self-test.
#include "scene_record.h"
#include <array>
#include <cstdint>
#include <deque>
#include <functional>
#include <map>
#include <optional>
#include <vector>

static_assert(sizeof(sa2_engine_image) == 80);
static_assert(sizeof(sa2_preparation) == 112);
static_assert(sizeof(sa2_poll_row) == 32);
static_assert(sizeof(sa2_frame_input) == 264);
static_assert(sizeof(sa2_frame_report) == 104);
static_assert(sizeof(sa2_presented_row) == 56);
static_assert(sizeof(sa2_pick_row) == 112);
static_assert(sizeof(sa2_counter_values) == 64);

namespace sa2 {
using Digest = std::array<uint8_t, 32>;
constexpr uint64_t DeviceEpoch = 1; // a context's first device epoch; only an ABI 5 recovery increases it
constexpr size_t MaxPresents = 1024; // ABI 5: frames awaiting a statistics row, and confirmed rows not yet read
constexpr uint32_t EngineCopies = 3; // GPU ring copies of the adopted engine image
constexpr size_t MaxTickets = 256; // tickets holding GPU memory (2 MiB each) until they are released
constexpr size_t MaxEnded = 1024; // released cancelled or failed tickets still answering poll (RM.17)
constexpr size_t MaxPicks = 1024; // live pick tickets
constexpr size_t RetainedFrames = 64; // reported frames kept for picks
constexpr size_t MaxUnreported = 1024; // unread frame reports kept

// One adopted engine state (sa2_engine_image), copied.
struct EngineImage {
    uint64_t revision = 0;
    uint32_t pose_count = 0; // 0: no pose path
    std::vector<uint32_t> labels; // SA2_SLOTS
    std::vector<int32_t> pose_index; // SA2_PIECES, or empty
    std::vector<float> poses; // pose_count * 16, rows contiguous
    std::vector<int32_t> lattice; // pose_count
    Digest digest{};
    bool posed() const { return pose_count != 0; }
    uint64_t bytes() const; // the bytes one GPU upload copies
};
EngineImage engine_image(const sa2_engine_image* image); // validates (SA2_E_INVALID_ARGUMENT) and copies

struct PublicationStamp {
    uint64_t command = 0, view = 0, view_generation = 0, publication_generation = 0, device_epoch = 0, base_revision = 0;
    Digest input_digest{};
};
// One preparation as the GPU uploads it: per slot, 1 = drawn, and the
// instance ID the ID pass writes (0 where the slot is not interactive).
struct PreparedData {
    PublicationStamp stamp;
    std::vector<uint32_t> visible, instances; // SA2_SLOTS each
    uint64_t bytes() const { return uint64_t(visible.size() + instances.size()) * 4; }
};
PreparedData preparation(const sa2_preparation* preparation); // validates and copies
void validate_frame_input(const sa2_frame_input* input);
void validate_cancel(int32_t reason, uint64_t superseded_by);
template<class Row> void validate_rows(const Row* rows, uint32_t capacity) {
    scene_require(capacity == 0 || rows != nullptr, "rows is NULL with a nonzero capacity", SA2_E_INVALID_ARGUMENT);
    for (uint32_t i = 0; i < capacity; ++i)
        scene_require(rows[i].struct_size == sizeof(Row), "a row's struct_size is not sizeof the row", SA2_E_INVALID_ARGUMENT);
}

// What a frame draws: the engine ring copy (uploaded first when upload is
// set, after a CPU wait for upload_wait, the frame that last read the copy's
// staging), the pose path and the resource set (0: every slot drawn, no
// instance IDs).
struct FramePlan {
    uint32_t copy = 0;
    bool upload = false;
    uint64_t upload_wait = 0;
    const EngineImage* image = nullptr;
    uint32_t pose_count = 0;
    uint64_t set_ticket = 0;
};
// An ID pass: the frame's camera, viewport, engine copy and resource set.
struct PassInput {
    std::array<float, 16> camera{};
    uint32_t width = 0, height = 0, x = 0, y = 0, copy = 0, pose_count = 0;
    uint64_t set_ticket = 0;
};
struct PickPlan {
    uint64_t frame = 0;
    int32_t state = SA2_PICK_PENDING; // READY or STALE: answered without a pass
    uint32_t stale = 0;
    bool pass = false;
    PassInput input;
};
struct Counters {
    uint64_t evals_last = 0, evals_total = 0, bytes_uploaded = 0, frames_submitted = 0, frames_reported = 0, id_passes = 0, reports_dropped = 0;
};

// ABI 5, route B: presentation confirmation. Route B is accepted for now by
// the owner on 18 Nov 2026, pending three owner-machine checks. The rule is
// the C9 present probe's "exact" rule (work/experiments/c9-present-probe/
// analyze.py): a frame is confirmed only by a statistics row of its own
// swap-chain generation whose PresentCount equals the frame's own present
// count (GetLastPresentCount right after its Present).
enum class PresentMatch {
    Ignore, // the sample says nothing about the frame
    Wait, // the sample has not reached the frame's present yet
    Exact, // the frame was presented: report it
    Superseded, // the sample passed the frame, or the frame's swap chain ended: never report it
};
struct PresentSample {
    int32_t hr = 0; // GetFrameStatistics; DXGI_ERROR_FRAME_STATISTICS_DISJOINT or any failure gives no row
    uint64_t generation = 0; // the swap-chain generation the sample was read from
    uint32_t present_count = 0; // DXGI_FRAME_STATISTICS.PresentCount
    uint32_t last_present_count = 0; // GetLastPresentCount read with the sample
    int64_t sync_qpc = 0; // DXGI_FRAME_STATISTICS.SyncQPCTime: the confirmed frame's display_qpc
};
// Pure: what one sample says about a frame of the generation with that present count.
PresentMatch present_match(uint64_t generation, uint32_t present_count, const PresentSample& sample);
class PresentLedger {
public:
    // A frame presented in the generation with its own present count. A count
    // that does not increase within a generation makes the earlier frames at
    // or above it ambiguous: they and the new frame are dropped, never reported.
    void presented(const sa2_frame_input& input, uint64_t frame, uint64_t generation, uint32_t present_count);
    void sample(const PresentSample& sample); // confirms or drops awaiting frames
    uint32_t rows(sa2_presented_row* rows, uint32_t capacity); // confirmed frames in frame order, each once
    void clear() { awaiting_.clear(); confirmed_.clear(); }
    size_t awaiting() const { return awaiting_.size(); }
    size_t confirmed() const { return confirmed_.size(); }
private:
    struct Entry { sa2_frame_input input{}; uint64_t frame = 0, generation = 0, display_qpc = 0; uint32_t count = 0; };
    std::deque<Entry> awaiting_, confirmed_;
};
// C9, owner item 9 (20 Nov 2026: docs/wiki/decisions/owner-decisions-2026-11-20-lp3-exit.md): route B's blind
// seconds come from the core's own frame statistics, cross-checked against PresentMon, which marks every present
// of the core's composition chain Dropped. The traced frames of a scene drawn on the surface
// (sa2_scene_produce with SA2_TARGET_SURFACE) enter a ledger of their own, by trace index and own present count,
// and are confirmed by the product's exact rule; presented.jsonl lists the confirmed frames in trace order.
class TracePresents {
public:
    void presented(uint64_t index, uint64_t generation, uint32_t present_count);
    void sample(const PresentSample& sample);
    size_t awaiting() const { return ledger_.awaiting(); }
    size_t confirmed() const { return confirmed_.size(); }
    // One {"display_qpc", "frame", "present_count"} row per confirmed frame: frame is the trace index.
    std::string text() const;
private:
    PresentLedger ledger_;
    std::vector<sa2_presented_row> confirmed_;
};

class Product {
public:
    // Device loss: the first failed GetDeviceRemovedReason latches.
    void observe_removal(int32_t hr);
    bool removed() const { return removed_; }
    int32_t removed_reason() const { return reason_; }
    // ABI 5 recovery: the device epoch frames and preparations must name.
    uint64_t device_epoch() const { return device_epoch_; }
    void set_device_epoch(uint64_t epoch) { device_epoch_ = epoch; } // a scene loaded after a recovery
    // After a recovery on a new device (epoch above the current one, device
    // removed): forgets every ticket, resource set, frame and pick of the old
    // epoch (unread reports count as dropped) and the engine ring copies, so
    // the next frame uploads the kept CPU image (RM.16); clears the removal
    // and the present ledger. Ticket, set and pick numbers keep counting.
    void recovered(uint64_t epoch);
    PresentLedger& presents() { return presents_; }

    uint64_t adopt(EngineImage image); // revisions increase strictly; no GPU work; works after device loss (RM.16)
    const EngineImage* image() const { return image_ ? &*image_ : nullptr; }

    void check_prepare(const PreparedData& data) const; // removal, epoch, GPU capacity
    uint64_t reserve_ticket() { return next_ticket_++; }
    void prepared(uint64_t ticket, const PreparedData& data, uint64_t fence); // the upload signals fence
    sa2_poll_row poll(uint64_t ticket, uint64_t completed); // completed: the product fence's value
    void activate(uint64_t set, const uint8_t* stamp_digest);
    void cancel(uint64_t ticket, int32_t reason, uint64_t superseded_by);
    void retire(uint64_t set);
    // Tickets whose GPU resources may be released now; each is returned once.
    // A retired set's ticket is forgotten once released; a cancelled or failed
    // ticket keeps answering until MaxEnded newer ended tickets were released.
    std::vector<uint64_t> releasable(uint64_t frames_completed, uint64_t product_completed);

    FramePlan plan(const sa2_frame_input& input, uint32_t width, uint32_t height) const;
    void commit(const sa2_frame_input& input, const FramePlan& plan, uint64_t frame, uint32_t ring);
    uint64_t unharvested(uint32_t ring) const { return ring < unharvested_.size() ? unharvested_[ring] : 0; }
    void produced(uint64_t frame, uint64_t gpu_ns, uint64_t evals, int32_t error = SA2_OK);
    uint32_t reports(sa2_frame_report* rows, uint32_t capacity);

    PickPlan pick_plan(uint64_t frame, uint32_t x, uint32_t y) const;
    uint64_t reserve_pick() { return next_pick_++; }
    void pick_commit(uint64_t ticket, const PickPlan& plan, uint64_t fence); // fence 0: answered without a pass
    size_t live_picks() const { return picks_.size(); }
    // A final reply sets release (the pick's GPU resources may go) and forgets the ticket.
    sa2_pick_row pick_poll(uint64_t ticket, uint64_t completed, const std::function<uint32_t()>& read, bool& release);

    void resized() { ++target_generation_; }
    uint64_t target_generation() const { return target_generation_; }
    const Counters& counters() const { return counters_; }
    sa2_counter_values counter_values() const;
    size_t live_tickets() const { return entries_.size(); }
    size_t held_tickets() const; // tickets whose GPU resources are not yet released
    size_t retained_frames() const { return frames_.size(); }

private:
    struct Entry {
        PublicationStamp stamp;
        int32_t state = SA2_TICKET_PENDING, reason = 0;
        uint64_t superseded_by = 0, fence = 0, set = 0, last_frame = 0, last_pick = 0;
        bool active = false, retired = false, released = false;
        Digest stamp_digest{};
        bool visible() const { return !retired; }
        bool ended() const { return state == SA2_TICKET_CANCELLED || state == SA2_TICKET_FAILED; }
    };
    struct Copy { uint64_t revision = 0, upload_frame = 0, sequence = 0; uint32_t pose_count = 0; bool valid = false; };
    struct Frame {
        sa2_frame_input input{};
        uint64_t id = 0, target_generation = 0, set_ticket = 0;
        uint32_t ring = 0, copy = 0;
        bool produced = false, reported = false;
        int32_t error = SA2_OK;
        uint64_t gpu_ns = 0, evals = 0;
    };
    struct Pick {
        sa2_frame_input binding{};
        uint64_t frame = 0, fence = 0, target_generation = 0;
        uint32_t x = 0, y = 0, stale = 0, instance = 0;
        int32_t state = SA2_PICK_PENDING;
    };
    Entry& visible_entry(uint64_t ticket);
    Entry* set_entry(uint64_t set);
    const Entry* set_entry(uint64_t set) const;
    const Frame* frame(uint64_t id) const;
    uint32_t stale(const sa2_frame_input& binding, uint64_t target_generation) const;
    void require_live() const;
    void trim();

    bool removed_ = false;
    int32_t reason_ = 0;
    uint64_t device_epoch_ = DeviceEpoch;
    PresentLedger presents_;
    std::optional<EngineImage> image_;
    uint64_t next_ticket_ = 1, next_set_ = 1, next_pick_ = 1, target_generation_ = 1, copy_sequence_ = 0;
    std::map<uint64_t, Entry> entries_; // by ticket
    std::map<uint64_t, uint64_t> sets_; // resource set -> ticket
    std::array<Copy, EngineCopies> copies_{};
    std::deque<Frame> frames_; // in frame order: reported frames, then unreported
    std::array<uint64_t, SA2_RING_SLOTS> unharvested_{};
    std::map<uint64_t, std::pair<uint64_t, uint64_t>> surfaces_; // surface -> (last sequence, newest surface_generation)
    std::map<uint64_t, uint64_t> views_; // view -> newest view_generation
    std::map<uint64_t, Pick> picks_;
    Counters counters_;
};

// describe_backend's SupportReport and recover's refusal, as JSON. route_b:
// a sa2_create context with a surface (ABI 5), where F22 and F23 are supported.
Json describe(const std::string& backend_build, int32_t removed_reason, uint64_t device_epoch = DeviceEpoch, bool route_b = false);
Json recover_report(int32_t removed_reason);
// ABI 5 recover's answer on a sa2_create context: status "recovered",
// "not_removed" or "failed".
Json recover_result(const char* status, const std::string& reason, uint64_t device_epoch, int32_t removed_reason);
std::string removed_text(int32_t removed_reason); // "0x%08x"
}
