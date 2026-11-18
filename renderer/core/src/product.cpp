#include "product.h"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>

namespace sa2 {
namespace {
void argument(bool condition, const char* text) { scene_require(condition, text, SA2_E_INVALID_ARGUMENT); }
void state(bool condition, const char* text) { scene_require(condition, text, SA2_E_WRONG_STATE); }
Digest digest(const uint8_t* bytes) {
    Digest out{};
    std::memcpy(out.data(), bytes, out.size());
    return out;
}
bool zero(const uint8_t* bytes, size_t count) {
    return std::all_of(bytes, bytes + count, [](uint8_t b) { return b == 0; });
}
}

uint64_t EngineImage::bytes() const {
    uint64_t total = uint64_t(labels.size()) * 4;
    if (posed()) total += uint64_t(SA2_PIECES) * 4 + uint64_t(pose_count) * (64 + 4);
    return total;
}

EngineImage engine_image(const sa2_engine_image* image) {
    argument(image && image->struct_size == sizeof(*image), "invalid engine_image struct_size");
    argument(image->labels != nullptr, "engine_image labels is NULL");
    const bool posed = image->pose_index != nullptr;
    if (posed) {
        argument(image->pose_count >= 1 && image->pose_count <= SA2_MAX_POSES, "pose_count must be 1..SA2_MAX_POSES with a pose index");
        argument(image->poses && image->pose_lattice, "poses and pose_lattice are required with a pose index");
    } else {
        argument(image->pose_count == 0 && !image->poses && !image->pose_lattice,
            "without a pose index, pose_count is 0 and poses and pose_lattice are NULL");
    }
    EngineImage out;
    out.revision = image->revision;
    out.pose_count = image->pose_count;
    out.labels.assign(image->labels, image->labels + SA2_SLOTS);
    argument(std::all_of(out.labels.begin(), out.labels.end(), [](uint32_t label) { return label < SA2_SLOTS; }),
        "a label is not below SA2_SLOTS");
    if (posed) {
        const uint32_t count = image->pose_count;
        out.pose_index.assign(image->pose_index, image->pose_index + SA2_PIECES);
        argument(std::all_of(out.pose_index.begin(), out.pose_index.end(),
            [count](int32_t pose) { return pose >= 0 && uint32_t(pose) < count; }), "a pose index is not below pose_count");
        out.poses.assign(image->poses, image->poses + size_t(count) * 16);
        argument(std::all_of(out.poses.begin(), out.poses.end(), [](float value) { return std::isfinite(value); }),
            "a pose entry is not finite");
        for (size_t i = 0; i < 16; ++i)
            argument(out.poses[i] == (i % 5 == 0 ? 1.0f : 0.0f), "pose 0 is not the identity");
        out.lattice.assign(image->pose_lattice, image->pose_lattice + count);
        argument(std::all_of(out.lattice.begin(), out.lattice.end(),
            [](int32_t entry) { return entry >= -1 && entry < SA2_POSE_GROUP_ORDER; }), "a pose_lattice entry is not -1 or below SA2_POSE_GROUP_ORDER");
        argument(out.lattice[0] == 0, "pose_lattice entry 0 is not 0");
    }
    out.digest = digest(image->digest);
    return out;
}

PreparedData preparation(const sa2_preparation* value) {
    argument(value && value->struct_size == sizeof(*value), "invalid preparation struct_size");
    argument(value->reserved == 0, "preparation reserved must be 0");
    argument(value->visibility && value->interaction && value->instances, "preparation visibility, interaction and instances are required");
    PreparedData out;
    out.stamp = {value->command, value->view, value->view_generation, value->publication_generation, value->device_epoch,
        value->base_revision, digest(value->input_digest)};
    out.visible.resize(SA2_SLOTS);
    out.instances.resize(SA2_SLOTS);
    for (size_t slot = 0; slot < SA2_SLOTS; ++slot) {
        const uint8_t visible = value->visibility[slot], interactive = value->interaction[slot];
        argument(visible <= 1 && interactive <= 1, "visibility and interaction bytes must be 0 or 1");
        out.visible[slot] = visible;
        // F02: a slot that is not drawn or not interactive writes no instance.
        out.instances[slot] = visible && interactive ? value->instances[slot] : 0;
    }
    return out;
}

void validate_frame_input(const sa2_frame_input* input) {
    argument(input && input->struct_size == sizeof(*input), "invalid frame_input struct_size");
    argument(input->reserved == 0, "frame_input reserved must be 0");
    argument(!(input->flags & ~SA2_FRAME_PREVIEW), "unknown frame_input flags");
    argument(input->target < SA2_RING_SLOTS || input->target == SA2_TARGET_SURFACE, "target must be a ring slot 0..2 or SA2_TARGET_SURFACE");
    argument(input->viewport_width >= SA2_MIN_TEXTURE_PX && input->viewport_height >= SA2_MIN_TEXTURE_PX,
        "viewport is below SA2_MIN_TEXTURE_PX");
    if (!(input->flags & SA2_FRAME_PREVIEW))
        argument(input->preview == 0 && input->preview_base == 0 && zero(input->preview_literal_digest, 32),
            "preview, preview_base and preview_literal_digest are 0 without SA2_FRAME_PREVIEW");
    argument(std::all_of(std::begin(input->camera_matrix), std::end(input->camera_matrix), [](float v) { return std::isfinite(v); }),
        "camera_matrix is not finite");
}

std::string removed_text(int32_t removed_reason) {
    char text[16];
    std::snprintf(text, sizeof(text), "0x%08x", static_cast<unsigned>(removed_reason));
    return text;
}

void Product::observe_removal(int32_t hr) {
    if (!removed_ && hr < 0) {
        removed_ = true;
        reason_ = hr;
    }
}
void Product::require_live() const {
    if (removed_) throw Failure(SA2_E_DEVICE_REMOVED, "device removed (" + removed_text(reason_) + ")");
}

// RM.16: adoption updates the authoritative CPU image even during device
// loss; it claims nothing about a frame.
uint64_t Product::adopt(EngineImage image) {
    state(!image_ || image.revision > image_->revision, "engine revision must increase");
    image_ = std::move(image);
    return image_->revision;
}

void Product::check_prepare(const PreparedData& data) const {
    require_live();
    state(data.stamp.device_epoch == device_epoch_, "device_epoch is not the backend's");
    state(held_tickets() < MaxTickets, "256 tickets hold GPU memory");
}
size_t Product::held_tickets() const {
    return size_t(std::count_if(entries_.begin(), entries_.end(), [](const auto& entry) { return !entry.second.released; }));
}
void Product::prepared(uint64_t ticket, const PreparedData& data, uint64_t fence) {
    Entry entry;
    entry.stamp = data.stamp;
    entry.fence = fence;
    entries_[ticket] = entry;
    counters_.bytes_uploaded += data.bytes();
}

Product::Entry& Product::visible_entry(uint64_t ticket) {
    auto it = entries_.find(ticket);
    state(it != entries_.end() && it->second.visible(), "unknown ticket");
    return it->second;
}
Product::Entry* Product::set_entry(uint64_t set) {
    auto mapped = sets_.find(set);
    if (mapped == sets_.end()) return nullptr;
    auto it = entries_.find(mapped->second);
    return it == entries_.end() ? nullptr : &it->second;
}
const Product::Entry* Product::set_entry(uint64_t set) const {
    return const_cast<Product*>(this)->set_entry(set);
}

sa2_poll_row Product::poll(uint64_t ticket, uint64_t completed) {
    Entry& entry = visible_entry(ticket);
    if (removed_ && (entry.state == SA2_TICKET_PENDING || entry.state == SA2_TICKET_READY)) {
        entry.state = SA2_TICKET_FAILED;
        entry.reason = SA2_E_DEVICE_REMOVED;
    }
    if (entry.state == SA2_TICKET_PENDING && completed >= entry.fence) {
        entry.state = SA2_TICKET_READY;
        entry.set = next_set_++;
        sets_[entry.set] = ticket;
    }
    sa2_poll_row row{};
    row.struct_size = sizeof(row);
    row.state = entry.state;
    if (entry.state == SA2_TICKET_READY) row.resource_set = entry.set;
    if (entry.state == SA2_TICKET_CANCELLED) {
        row.reason = entry.reason;
        row.superseded_by = entry.superseded_by;
    }
    if (entry.state == SA2_TICKET_FAILED) row.reason = entry.reason;
    return row;
}

void Product::activate(uint64_t set, const uint8_t* stamp_digest) {
    argument(stamp_digest != nullptr, "stamp_digest is NULL");
    require_live();
    Entry* entry = set_entry(set);
    state(entry && entry->state == SA2_TICKET_READY && !entry->active && !entry->retired, "resource set is not ready or is already active");
    entry->active = true;
    entry->stamp_digest = digest(stamp_digest);
}

void validate_cancel(int32_t reason, uint64_t superseded_by) {
    argument(reason >= SA2_CANCEL_BY_USER && reason <= SA2_CANCEL_BUDGET, "unknown cancel reason");
    argument(reason == SA2_CANCEL_SUPERSEDED || superseded_by == 0, "superseded_by must be 0 unless the reason is SA2_CANCEL_SUPERSEDED");
}

void Product::cancel(uint64_t ticket, int32_t reason, uint64_t superseded_by) {
    validate_cancel(reason, superseded_by);
    Entry& entry = visible_entry(ticket);
    if (entry.state == SA2_TICKET_CANCELLED || entry.state == SA2_TICKET_FAILED) return;
    state(!entry.active, "the ticket's resource set is active; retire it");
    if (entry.state == SA2_TICKET_READY) sets_.erase(entry.set);
    entry.state = SA2_TICKET_CANCELLED;
    entry.reason = reason;
    entry.superseded_by = superseded_by;
}

void Product::retire(uint64_t set) {
    auto mapped = sets_.find(set);
    state(mapped != sets_.end(), "unknown or retired resource set");
    Entry& entry = entries_.at(mapped->second);
    entry.retired = true;
    entry.active = false;
    sets_.erase(mapped);
}

std::vector<uint64_t> Product::releasable(uint64_t frames_completed, uint64_t product_completed) {
    std::vector<uint64_t> out;
    size_t ended = 0;
    for (auto it = entries_.begin(); it != entries_.end();) {
        const uint64_t ticket = it->first;
        Entry& entry = it->second;
        if (!entry.released) {
            const bool uploaded = product_completed >= entry.fence;
            const bool unused = uploaded && product_completed >= entry.last_pick && frames_completed >= entry.last_frame;
            if (removed_ || (entry.state == SA2_TICKET_CANCELLED && uploaded) || (entry.retired && unused)) {
                entry.released = true;
                out.push_back(ticket);
            }
        }
        const auto mapped = entry.set ? sets_.find(entry.set) : sets_.end();
        const bool bound = mapped != sets_.end() && mapped->second == ticket;
        if (entry.released && entry.retired && !bound) it = entries_.erase(it);
        else {
            if (entry.released && entry.ended() && !bound) ++ended;
            ++it;
        }
    }
    // The host may never poll a ticket it cancelled: keep the newest ended
    // replies only, oldest ticket first out.
    for (auto it = entries_.begin(); ended > MaxEnded && it != entries_.end();) {
        const Entry& entry = it->second;
        const bool bound = entry.set && sets_.count(entry.set) && sets_.at(entry.set) == it->first;
        if (entry.released && entry.ended() && !bound) {
            it = entries_.erase(it);
            --ended;
        } else ++it;
    }
    return out;
}

FramePlan Product::plan(const sa2_frame_input& input, uint32_t width, uint32_t height) const {
    require_live();
    state(input.device_epoch == device_epoch_, "device_epoch is not the backend's");
    state(image_.has_value(), "no engine image adopted");
    state(input.engine_revision == image_->revision, "engine_revision is not the adopted revision");
    state(input.viewport_width == width && input.viewport_height == height, "viewport is not the target's size");
    if (auto surface = surfaces_.find(input.surface); surface != surfaces_.end()) {
        state(input.sequence > surface->second.first, "sequence must increase within the surface");
        state(input.surface_generation >= surface->second.second, "surface_generation must not decrease");
    }
    if (auto view = views_.find(input.view); view != views_.end())
        state(input.view_generation >= view->second, "view_generation must not decrease");
    FramePlan out;
    out.image = &*image_;
    out.pose_count = image_->pose_count;
    if (input.resource_set) {
        const Entry* entry = set_entry(input.resource_set);
        state(entry && entry->active, "resource_set is not an active set");
        state(entry->stamp.view == input.view && entry->stamp.view_generation == input.view_generation
            && entry->stamp.publication_generation == input.publication_generation,
            "resource_set was prepared for another view, view generation or publication generation");
        out.set_ticket = sets_.at(input.resource_set);
    }
    for (uint32_t copy = 0; copy < EngineCopies; ++copy) {
        if (copies_[copy].valid && copies_[copy].revision == image_->revision) {
            out.copy = copy;
            return out;
        }
    }
    uint32_t oldest = 0;
    for (uint32_t copy = 1; copy < EngineCopies; ++copy)
        if (copies_[copy].sequence < copies_[oldest].sequence) oldest = copy;
    out.copy = oldest;
    out.upload = true;
    out.upload_wait = copies_[oldest].upload_frame;
    return out;
}

void Product::commit(const sa2_frame_input& input, const FramePlan& plan, uint64_t frame, uint32_t ring) {
    auto& surface = surfaces_[input.surface];
    surface.first = input.sequence;
    surface.second = std::max(surface.second, input.surface_generation);
    auto& view = views_[input.view];
    view = std::max(view, input.view_generation);
    if (plan.upload) {
        copies_[plan.copy] = {plan.image->revision, frame, ++copy_sequence_, plan.image->pose_count, true};
        counters_.bytes_uploaded += plan.image->bytes();
    }
    if (plan.set_ticket) entries_.at(plan.set_ticket).last_frame = frame;
    // The target's previous frame completed before this one was recorded, so
    // the ring's query results were read; a frame left unread drew without
    // results.
    if (ring < unharvested_.size() && unharvested_[ring]) produced(unharvested_[ring], 0, 0, SA2_E_D3D12);
    Frame record;
    record.input = input;
    record.id = frame;
    record.target_generation = target_generation_;
    record.set_ticket = plan.set_ticket;
    record.ring = ring;
    record.copy = plan.copy;
    frames_.push_back(record);
    if (ring < unharvested_.size()) unharvested_[ring] = frame;
    ++counters_.frames_submitted;
    trim();
}

void Product::produced(uint64_t frame, uint64_t gpu_ns, uint64_t evals, int32_t error) {
    for (auto& record : frames_) {
        if (record.id == frame && !record.produced) {
            record.produced = true;
            record.gpu_ns = gpu_ns;
            record.evals = evals;
            record.error = error;
            break;
        }
    }
    for (auto& pending : unharvested_)
        if (pending == frame) pending = 0;
}

uint32_t Product::reports(sa2_frame_report* rows, uint32_t capacity) {
    uint32_t count = 0;
    for (auto& record : frames_) {
        if (record.reported) continue;
        if (!record.produced) {
            if (!removed_) break;
            produced(record.id, 0, 0, SA2_E_DEVICE_REMOVED);
        }
        if (count == capacity) break;
        sa2_frame_report& row = rows[count++];
        row = {};
        row.struct_size = sizeof(row);
        row.error = record.error;
        row.frame_id = record.id;
        row.device_epoch = record.input.device_epoch;
        row.surface = record.input.surface;
        row.surface_generation = record.input.surface_generation;
        row.sequence = record.input.sequence;
        row.engine_revision = record.input.engine_revision;
        row.view = record.input.view;
        row.view_generation = record.input.view_generation;
        row.publication_generation = record.input.publication_generation;
        row.gpu_time_ns = record.gpu_ns;
        row.view_vertex_evals = record.evals;
        record.reported = true;
        ++counters_.frames_reported;
        if (record.error == SA2_OK) {
            counters_.evals_last = record.evals;
            counters_.evals_total += record.evals;
        }
    }
    trim();
    return count;
}

void Product::trim() {
    size_t unreported = 0;
    for (const auto& record : frames_)
        if (!record.reported) ++unreported;
    for (auto& record : frames_) {
        if (unreported <= MaxUnreported) break;
        if (!record.reported && record.produced) {
            record.reported = true;
            ++counters_.reports_dropped;
            --unreported;
        }
    }
    while (frames_.size() > RetainedFrames && frames_.front().reported) frames_.pop_front();
}

const Product::Frame* Product::frame(uint64_t id) const {
    for (const auto& record : frames_)
        if (record.id == id) return &record;
    return nullptr;
}

uint32_t Product::stale(const sa2_frame_input& binding, uint64_t target_generation) const {
    if (target_generation != target_generation_) return SA2_PICK_STALE_TARGET;
    if (auto surface = surfaces_.find(binding.surface); surface != surfaces_.end() && surface->second.second > binding.surface_generation)
        return SA2_PICK_STALE_SURFACE;
    if (auto view = views_.find(binding.view); view != views_.end() && view->second > binding.view_generation)
        return SA2_PICK_STALE_VIEW;
    return 0;
}

PickPlan Product::pick_plan(uint64_t id, uint32_t x, uint32_t y) const {
    require_live();
    const Frame* record = frame(id);
    state(record != nullptr, "frame is not retained");
    argument(x < record->input.viewport_width && y < record->input.viewport_height, "x and y must lie inside the frame's viewport");
    state(!record->produced || record->error == SA2_OK, "the frame drew nothing");
    state(picks_.size() < MaxPicks, "1024 live picks");
    PickPlan out;
    out.frame = id;
    out.input.x = x;
    out.input.y = y;
    if (const uint32_t reason = stale(record->input, record->target_generation)) {
        out.state = SA2_PICK_STALE;
        out.stale = reason;
        return out;
    }
    if (!record->input.resource_set) {
        out.state = SA2_PICK_READY; // nothing was pickable in this frame
        return out;
    }
    const auto mapped = sets_.find(record->input.resource_set);
    if (mapped == sets_.end() || mapped->second != record->set_ticket) {
        out.state = SA2_PICK_STALE;
        out.stale = SA2_PICK_STALE_PUBLICATION;
        return out;
    }
    const Copy& copy = copies_[record->copy];
    if (!copy.valid || copy.revision != record->input.engine_revision) {
        out.state = SA2_PICK_STALE;
        out.stale = SA2_PICK_STALE_ENGINE;
        return out;
    }
    out.pass = true;
    std::copy(std::begin(record->input.camera_matrix), std::end(record->input.camera_matrix), out.input.camera.begin());
    out.input.width = record->input.viewport_width;
    out.input.height = record->input.viewport_height;
    out.input.copy = record->copy;
    out.input.pose_count = copy.pose_count;
    out.input.set_ticket = record->set_ticket;
    return out;
}

void Product::pick_commit(uint64_t ticket, const PickPlan& plan, uint64_t fence) {
    const Frame* record = frame(plan.frame);
    state(record != nullptr, "frame is not retained");
    Pick pick;
    pick.binding = record->input;
    pick.frame = plan.frame;
    pick.fence = fence;
    pick.target_generation = record->target_generation;
    pick.x = plan.input.x;
    pick.y = plan.input.y;
    pick.state = plan.state;
    pick.stale = plan.stale;
    if (plan.pass) {
        pick.state = SA2_PICK_PENDING;
        entries_.at(plan.input.set_ticket).last_pick = fence;
        ++counters_.id_passes;
    }
    picks_[ticket] = pick;
}

sa2_pick_row Product::pick_poll(uint64_t ticket, uint64_t completed, const std::function<uint32_t()>& read, bool& release) {
    release = false;
    require_live();
    auto it = picks_.find(ticket);
    state(it != picks_.end(), "unknown pick ticket");
    Pick& pick = it->second;
    if (pick.state == SA2_PICK_PENDING && completed >= pick.fence) {
        pick.instance = read();
        pick.state = SA2_PICK_READY;
    }
    if (pick.state == SA2_PICK_READY) {
        if (const uint32_t reason = stale(pick.binding, pick.target_generation)) {
            pick.state = SA2_PICK_STALE;
            pick.stale = reason;
        }
    }
    if (pick.state == SA2_PICK_STALE) pick.instance = 0;
    sa2_pick_row row{};
    row.struct_size = sizeof(row);
    row.state = pick.state;
    row.stale = pick.state == SA2_PICK_STALE ? pick.stale : 0;
    row.instance = pick.state == SA2_PICK_READY ? pick.instance : 0;
    row.frame_id = pick.frame;
    row.device_epoch = pick.binding.device_epoch;
    row.surface = pick.binding.surface;
    row.surface_generation = pick.binding.surface_generation;
    row.view = pick.binding.view;
    row.view_generation = pick.binding.view_generation;
    row.publication_generation = pick.binding.publication_generation;
    row.camera = pick.binding.camera;
    row.visibility = pick.binding.visibility;
    row.interaction = pick.binding.interaction;
    row.viewport_width = pick.binding.viewport_width;
    row.viewport_height = pick.binding.viewport_height;
    row.x = pick.x;
    row.y = pick.y;
    if (pick.state != SA2_PICK_PENDING) {
        release = true;
        picks_.erase(it);
    }
    return row;
}

sa2_counter_values Product::counter_values() const {
    sa2_counter_values out{};
    out.struct_size = sizeof(out);
    out.view_vertex_evals_last = counters_.evals_last;
    out.view_vertex_evals_total = counters_.evals_total;
    out.bytes_uploaded = counters_.bytes_uploaded;
    out.frames_submitted = counters_.frames_submitted;
    out.frames_reported = counters_.frames_reported;
    out.id_passes = counters_.id_passes;
    out.reports_dropped = counters_.reports_dropped;
    return out;
}

// ABI 5 (route B; accepted for now by the owner on 18 Nov 2026, pending three
// owner-machine checks): recovery on a new device of the same adapter.
void Product::recovered(uint64_t epoch) {
    state(removed_, "the device is not removed");
    state(epoch > device_epoch_, "the device epoch must increase");
    for (const auto& record : frames_)
        if (!record.reported) ++counters_.reports_dropped;
    entries_.clear();
    sets_.clear();
    picks_.clear();
    frames_.clear();
    copies_ = {}; // the ring copies were on the old device: the next frame uploads the kept image
    unharvested_ = {};
    presents_.clear();
    ++target_generation_;
    removed_ = false;
    reason_ = 0;
    device_epoch_ = epoch;
}

// ABI 5 (route B; accepted for now by the owner on 18 Nov 2026, pending three
// owner-machine checks): the C9 present probe's exact rule (analyze.py, pass
// 2), for one frame and one statistics sample.
PresentMatch present_match(uint64_t generation, uint32_t present_count, const PresentSample& sample) {
    if (sample.generation > generation) return PresentMatch::Superseded; // the frame's swap chain ended: its row never comes
    if (sample.generation < generation) return PresentMatch::Ignore;
    // A failed or disjoint call, or a count ahead of the swap chain's own last
    // present count, is no row (the probe's "usable" rows only).
    if (sample.hr != 0 || sample.present_count > sample.last_present_count) return PresentMatch::Ignore;
    if (sample.present_count == present_count) return PresentMatch::Exact;
    return sample.present_count > present_count ? PresentMatch::Superseded : PresentMatch::Wait;
}

void PresentLedger::presented(const sa2_frame_input& input, uint64_t frame, uint64_t generation, uint32_t present_count) {
    bool ambiguous = false;
    for (auto it = awaiting_.begin(); it != awaiting_.end();) {
        if (it->generation == generation && it->count >= present_count) {
            it = awaiting_.erase(it);
            ambiguous = true;
        } else ++it;
    }
    if (ambiguous) return;
    Entry entry;
    entry.input = input;
    entry.frame = frame;
    entry.generation = generation;
    entry.count = present_count;
    awaiting_.push_back(entry);
    while (awaiting_.size() > MaxPresents) awaiting_.pop_front();
}

void PresentLedger::sample(const PresentSample& sample) {
    for (auto it = awaiting_.begin(); it != awaiting_.end();) {
        const auto match = present_match(it->generation, it->count, sample);
        if (match == PresentMatch::Exact) {
            confirmed_.push_back(*it);
            confirmed_.back().display_qpc = static_cast<uint64_t>(sample.sync_qpc);
            it = awaiting_.erase(it);
        } else if (match == PresentMatch::Superseded) it = awaiting_.erase(it);
        else ++it;
    }
    while (confirmed_.size() > MaxPresents) confirmed_.pop_front();
}

uint32_t PresentLedger::rows(sa2_presented_row* rows, uint32_t capacity) {
    uint32_t count = 0;
    while (count < capacity && !confirmed_.empty()) {
        const Entry& entry = confirmed_.front();
        sa2_presented_row& row = rows[count++];
        row = {};
        row.struct_size = sizeof(row);
        row.previews = (entry.input.flags & SA2_FRAME_PREVIEW) ? 1u : 0u;
        row.frame_id = entry.frame;
        row.device_epoch = entry.input.device_epoch;
        row.surface = entry.input.surface;
        row.sequence = entry.input.sequence;
        row.preview = entry.input.preview;
        row.display_qpc = entry.display_qpc;
        confirmed_.pop_front();
    }
    return count;
}

namespace {
Json removal(int32_t removed_reason) { return removed_reason < 0 ? Json(removed_text(removed_reason)) : Json(); }
}

Json describe(const std::string& backend_build, int32_t removed_reason, uint64_t device_epoch, bool route_b) {
    const auto supported = [] { return Json(Json::Object{{"status", "supported"}, {"layout_version", 1}}); };
    const auto unsupported = [](const char* reason) { return Json(Json::Object{{"status", "unsupported"}, {"reason", reason}}); };
    Json::Object features{
        {"F01", supported()},
        {"F02", supported()},
        {"F18", supported()},
        {"J01", supported()},
        {"J02", supported()},
        // ABI 5, route B (accepted for now by the owner on 18 Nov 2026, pending
        // three owner-machine checks): supported only with the core's own device and surface.
        {"F22", route_b ? supported() : unsupported("recover: ABI 4 cannot recover a removed device (RPI.46)")},
        {"F23", route_b ? supported() : unsupported("read_presented: the core owns no swap chain and cannot confirm a presentation (RM.26)")},
        {"J13", unsupported("grip, piece and certificate-point IDs: the ID pass writes per-slot instance IDs only")},
    };
    return Json::Object{
        {"interface_version", SA2_ABI_VERSION},
        {"backend_build", backend_build},
        {"device_epoch", device_epoch},
        {"device_removed", removal(removed_reason)},
        {"features", features},
        {"evidence", Json::Array{}},
    };
}

Json recover_result(const char* status, const std::string& reason, uint64_t device_epoch, int32_t removed_reason) {
    return Json::Object{
        {"status", status},
        {"reason", reason},
        {"device_epoch", device_epoch},
        {"device_removed", removal(removed_reason)},
    };
}

Json recover_report(int32_t removed_reason) {
    return Json::Object{
        {"status", "unavailable"},
        {"reason", "ABI 4 cannot recover a device: it owns no swap chain and no device and has no recreation path (RPI.46)"},
        {"device_epoch", DeviceEpoch},
        {"device_removed", removal(removed_reason)},
    };
}
}
