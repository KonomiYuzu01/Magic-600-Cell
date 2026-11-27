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
static_assert(sizeof(sa2_view_shape) == 24);
static_assert(sizeof(sa2_view_style) == 348);
static_assert(sizeof(sa2_pane) == 112);
static_assert(sizeof(sa2_pane_row) == 32);
static_assert(sizeof(sa2_view_focus) == 24);

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
constexpr size_t MaxSpareSets = 2; // C9 (26 Nov 2026): released preparation resources kept for the next preparation

// C9 (26 Nov 2026): released preparation resources (a command allocator, a command list and the set's buffers) kept
// for the next preparation, so that a twist's preparation reuses them instead of creating them on the shell's frame
// thread. scene.cpp keeps a set only when Product::releasable released it on a live device, that is after its upload
// and every frame and ID pass that read it completed; at most MaxSpareSets are kept, the newest is taken first, and a
// device removal drops them all.
template<class T> class Spares {
public:
    // Keeps `value` for a later take. False, with `value` left to the caller, when `removed` or when MaxSpareSets are
    // kept already.
    bool keep(T&& value, bool removed) {
        if (removed || items_.size() >= MaxSpareSets) return false;
        items_.push_back(std::move(value));
        return true;
    }
    std::optional<T> take() {
        if (items_.empty()) return std::nullopt;
        std::optional<T> out(std::move(items_.back()));
        items_.pop_back();
        return out;
    }
    void clear() { items_.clear(); }
    size_t size() const { return items_.size(); }
private:
    std::vector<T> items_;
};

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
// ABI 6: the shape a view's frames draw with (sa2_view_shape_set). The defaults are ABI 5's constants, so a view whose
// shape was never set draws exactly as before.
struct ViewShape {
    float cell_shrink = SA2_SHAPE_DEFAULT_CELL_SHRINK, sticker_shrink = SA2_SHAPE_DEFAULT_STICKER_SHRINK;
    float d4 = SA2_SHAPE_DEFAULT_D4, zoom = SA2_SHAPE_DEFAULT_ZOOM;
    // ABI 9: the view's focus (sa2_view_focus_set); the defaults draw exactly as ABI 8.
    uint32_t projection = SA2_PROJECTION_PERSPECTIVE, focus_cell = SA2_FOCUS_NONE;
    float focus_cos = -1.0f, spread = 1.0f;
    uint32_t layout = SA2_LAYOUT_CHAIN; // ABI 10: the view's layout (sa2_view_layout_set)
    // ADR-D02 B18: the transform's turn of the moving pieces, written into a frame's or pane's shape when it is
    // planned (Product::turn); a view's stored shape never carries one. 0: no piece turns.
    std::array<float, 4> turn_u{}, turn_v{};
    float turn = 0.0f;
    uint32_t antipode = 0; // ADR-D02 B21: a pane's SA2_PANE_ANTIPODE, written into its shape when it is planned
    bool operator==(const ViewShape&) const = default;
};
ViewShape view_shape(const sa2_view_shape* shape); // validates (SA2_E_INVALID_ARGUMENT); NULL gives the defaults
// ABI 9: validates a focus (SA2_E_INVALID_ARGUMENT) and writes it into `shape`; NULL writes the defaults.
void view_focus(const sa2_view_focus* focus, ViewShape& shape);
// ABI 10: the Hopf-ring table the product shaders read (register t18), 5011 float4: the 600 cells' ring charts (rows 0
// to 2, then the translation), the rings' origins, their origins times J, their base points (w: the ring's half-step
// parity), J, the 600 cells by ring and place along it (as uint bits, four to an element), then from HopfSheetRow the
// 600 cells' ring-sheet charts (B22), laid out as the ring charts.
constexpr size_t HopfSheetRow = 2611;
constexpr size_t HopfTableRows = HopfSheetRow + 2400;
using HopfTable = std::vector<std::array<float, 4>>;
// Validates a layout's struct (SA2_E_INVALID_ARGUMENT); NULL is the chain.
uint32_t view_layout_kind(const sa2_view_layout* layout);
// Builds the Hopf table from the rings and the core's own cell centres; rings that do not fit them are refused.
HopfTable hopf_table(const sa2_view_layout& layout, const std::function<std::array<float, 4>(uint32_t)>& centre);
// ABI 7: a style slot (sa2_view_style_set), as the styled pass reads it (register b1): 16 palette entries, the three
// matrix rows (xyz), then the modes; ADR-D02 B20: the slice's mode, shrink and shell (as cosines), then the ghost.
// 368 bytes, inside one 512-byte constant buffer.
struct ViewStyle {
    std::array<float, 64> palette{};
    std::array<float, 12> rows{1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0};
    uint32_t tint_mode = 0, dim_mode = 0;
    float dim_amount = 0, outline_width = 0;
    uint32_t matrix_on = 0, slice_mode = 0;
    float slice_shrink = 1, shell_min_cos = 1;
    float shell_max_cos = -1, pad[3]{};
    std::array<float, 4> ghost{};
    bool operator==(const ViewStyle&) const = default;
};
static_assert(sizeof(ViewStyle) == 368);
ViewStyle view_style(const sa2_view_style* style); // validates (SA2_E_INVALID_ARGUMENT)
// ABI 7: one revision's piece style, packed per piece as the styled pass reads it (register t17): the fill byte, then
// the outline byte << 8; ADR-D02 B18: bit 16 marks a moving piece of the transform (t4 in the product passes).
struct PieceStyle {
    uint64_t revision = 0, version = 0; // version: the style's upload key, never 0
    std::vector<uint32_t> packed; // SA2_PIECES
    std::array<bool, 3> any{}; // per channel: a non-zero byte (ADR-D02 B20: channel 2, the slice's inside layer)
    bool dim = false; // a fill byte has SA2_STYLE_DIM
    bool moving = false; // ADR-D02 B18: a piece has the moving bit
};
void validate_piece_style(const uint8_t* bytes, uint32_t channel); // SA2_E_INVALID_ARGUMENT
void validate_frame_input(const sa2_frame_input* input);
void validate_cancel(int32_t reason, uint64_t superseded_by);
template<class Row> void validate_rows(const Row* rows, uint32_t capacity) {
    scene_require(capacity == 0 || rows != nullptr, "rows is NULL with a nonzero capacity", SA2_E_INVALID_ARGUMENT);
    for (uint32_t i = 0; i < capacity; ++i)
        scene_require(rows[i].struct_size == sizeof(Row), "a row's struct_size is not sizeof the row", SA2_E_INVALID_ARGUMENT);
}

// ABI 8: one pane of a composite frame as planned: its row, resource set ticket, shape and style.
struct PanePlan {
    sa2_pane pane{};
    uint64_t set_ticket = 0;
    uint64_t publication_generation = 0; // the pane's set's (the frame's for the first pane)
    ViewShape shape;
    bool styled = false;
    ViewStyle view_style;
};
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
    ViewShape shape; // ABI 6: the frame's view's shape
    // ABI 7: the revision's piece style (null: none; uploaded with the copy, whose key includes style_version) and,
    // when the frame draws styled, its slot's style.
    const std::vector<uint32_t>* style = nullptr;
    uint64_t style_version = 0;
    bool styled = false;
    ViewStyle view_style;
    std::vector<PanePlan> panes; // ABI 8: a composite frame's panes; empty for a one-view frame
};
// An ID pass: the frame's camera, viewport, engine copy, resource set and (ABI 6) shape.
struct PassInput {
    std::array<float, 16> camera{};
    uint32_t width = 0, height = 0, x = 0, y = 0, copy = 0, pose_count = 0;
    uint64_t set_ticket = 0;
    ViewShape shape;
    // ABI 8: a composite frame's pane: the viewport's origin and size in the scene-size ID target (0: the whole target).
    int32_t origin_x = 0, origin_y = 0;
    uint32_t view_width = 0, view_height = 0;
    // ADR-D02 B17 (ABI 10): the style slot the frame or pane drew with, so the ID pass follows visibility; piece_style
    // says whether its engine copy holds a piece style (else the zero buffer stands in).
    bool styled = false, piece_style = false;
    ViewStyle view_style;
};
struct PickPlan {
    uint64_t frame = 0;
    int32_t state = SA2_PICK_PENDING; // READY or STALE: answered without a pass
    uint32_t stale = 0;
    bool pass = false;
    PassInput input;
    int32_t pane = -1; // ABI 8: the composite frame's pane that holds the point
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
// C9 (26 Nov 2026, no ABI change): a product frame's wait for the surface's
// frame-latency waitable, in two parts. The shell's frame loop blocks in this
// wait for most of a refresh and calls sa2_presented right after it, within a
// millisecond of the vblank, when DXGI may or may not have posted that
// vblank's statistics yet. Where that race goes one way for one frame and the
// other way for the next, the one sample per frame passes a present without
// meeting its count, and the exact rule drops a frame that was on screen for
// a whole refresh. So when the wait has not ended after `sample_after_ms`,
// `sample` runs once (one more statistics sample for the ledger), and the
// wait goes on for the rest of `timeout_ms`; an infinite timeout (UINT32_MAX)
// stays infinite. `wait(ms)` waits up to ms and returns its status, and
// `timed_out` is the status of a wait that timed out. A wait that ends in its
// first part, or a timeout not above `sample_after_ms`, is one plain wait.
template <class Wait, class Sample>
auto latency_wait(uint32_t timeout_ms, uint32_t sample_after_ms, uint32_t timed_out, Wait&& wait, Sample&& sample)
    -> decltype(wait(timeout_ms)) {
    if (sample_after_ms == 0 || timeout_ms <= sample_after_ms) return wait(timeout_ms);
    const auto first = wait(sample_after_ms);
    if (first != timed_out) return first;
    sample();
    return wait(timeout_ms == UINT32_MAX ? timeout_ms : timeout_ms - sample_after_ms);
}
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

    // ABI 6: a view's shape; the defaults until it is set. Kept across a recovery.
    void set_shape(uint64_t view, const ViewShape& shape);
    // ABI 10: the shared Hopf table; a second, different table is refused (the GPU copy is written once).
    void set_hopf(HopfTable table);
    const HopfTable* hopf() const { return hopf_.empty() ? nullptr : &hopf_; }
    ViewShape shape(uint64_t view) const;
    // ABI 7: the adopted revision's piece style (a channel, or null to clear it) and the style slots. Kept across a
    // recovery; an adoption drops the piece style.
    void set_style(uint64_t revision, const uint8_t* bytes, uint32_t channel);
    void set_view_style(uint32_t slot, const sa2_view_style* style);
    const PieceStyle* piece_style() const { return style_ ? &*style_ : nullptr; }
    // Whether a frame with this slot draws styled, and with what (false: the unchanged product pipeline).
    bool styled(uint32_t slot, ViewStyle& out) const;
    // ADR-D02 B18 (ABI 10): the adopted revision's transform (a NULL mask clears it); an adoption drops it. turn()
    // writes its turn into a planned frame's or pane's shape.
    void set_transform(uint64_t revision, const float* u, const float* v, float angle, const uint8_t* mask, float phase);
    void turn(ViewShape& shape) const;

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
    // ABI 8: a composite frame's plan (sa2_frame_submit_panes): the one-view plan of the frame input, then each pane's
    // checks, set, shape and style. commit records the panes; pane_evals and pane_rows answer sa2_frame_panes.
    FramePlan plan_panes(const sa2_frame_input& input, const sa2_pane* panes, uint32_t count, uint32_t width, uint32_t height) const;
    void pane_evals(uint64_t frame, const std::vector<uint64_t>& evals);
    uint32_t pane_rows(uint64_t frame, sa2_pane_row* rows, uint32_t capacity) const;
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
    struct Copy { uint64_t revision = 0, upload_frame = 0, sequence = 0; uint32_t pose_count = 0; bool valid = false; uint64_t style = 0; };
    struct PaneRecord {
        sa2_pane pane{}; uint64_t set_ticket = 0, evals = 0, publication_generation = 0; ViewShape shape;
        bool styled = false; ViewStyle view_style; // ADR-D02 B17 (ABI 10): the style the pane drew with, for its picks
    };
    struct Frame {
        sa2_frame_input input{};
        std::vector<PaneRecord> panes; // ABI 8: a composite frame's panes
        uint64_t id = 0, target_generation = 0, set_ticket = 0;
        uint32_t ring = 0, copy = 0;
        ViewShape shape; // ABI 6: the shape the frame drew with, for its picks
        // ADR-D02 B17 (ABI 10): the style slot a one-view frame drew with, and the piece style version of its engine copy
        // (0: none), for its picks.
        bool styled = false; ViewStyle view_style; uint64_t style_version = 0;
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
    static sa2_frame_input pane_binding(const Frame& record, int32_t pane); // ABI 8
    PickPlan pick_pane(const Frame& record, PickPlan out) const; // ABI 8
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
    std::map<uint64_t, ViewShape> shapes_; // ABI 6: view -> shape, for views whose shape is not the default
    HopfTable hopf_; // ABI 10
    std::optional<PieceStyle> style_; // ABI 7: the adopted revision's piece style
    uint64_t style_version_ = 0;
    struct Turn { std::array<float, 4> u{}, v{}; float angle = 0.0f; };
    std::optional<Turn> transform_; // ADR-D02 B18: the plane and the drawn angle (phase - 1) * angle
    std::array<std::optional<ViewStyle>, SA2_STYLE_SLOTS> view_styles_; // ABI 7: slots 1..4
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
