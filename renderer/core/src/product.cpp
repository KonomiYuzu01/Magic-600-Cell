#include "product.h"
#include <algorithm>
#include <bit>
#include <string>
#include <cmath>
#include <cstdio>
#include <cstring>

namespace sa2 {
namespace {
// C9 (26 Nov 2026): the text becomes a std::string only for a failure. A preparation checks every one of its 259,800
// slots, and building the message on every pass cost about 9 ms per twist.
void argument(bool condition, const char* text) { if (!condition) scene_require(false, text, SA2_E_INVALID_ARGUMENT); }
void state(bool condition, const char* text) { if (!condition) scene_require(false, text, SA2_E_WRONG_STATE); }
Digest digest(const uint8_t* bytes) {
    Digest out{};
    std::memcpy(out.data(), bytes, out.size());
    return out;
}
bool zero(const uint8_t* bytes, size_t count) {
    return std::all_of(bytes, bytes + count, [](uint8_t b) { return b == 0; });
}
bool unit_colour(float value) { return std::isfinite(value) && value >= 0.0f && value <= 1.0f; }
}

void Product::set_sticker_colours(const float* rgb, uint32_t count) {
    StickerColours next{};
    if (rgb && count) {
        argument(count == SA2_STICKER_CLASSES, "a sticker colour table needs 600 RGB triples");
        for (uint32_t i = 0; i < count; ++i) {
            const float* colour = rgb + 3 * i;
            argument(std::all_of(colour, colour + 3, unit_colour), "sticker RGB must be finite and within 0..1");
            next[i] = {colour[0], colour[1], colour[2], 1.0f};
        }
    }
    sticker_colours_ = next;
}

void Product::set_scene_look(const sa2_scene_look* look) {
    if (look) {
        argument(look->struct_size == sizeof(*look) && look->reserved == 0, "invalid scene look size or reserved field");
        argument(std::all_of(look->background, look->background + 4, unit_colour) && unit_colour(look->fog)
                 && unit_colour(look->glow) && unit_colour(look->edge_brightness) && unit_colour(look->edge_weight),
                 "scene look values must be finite and within 0..1");
    }
    scene_look_ = look ? *look : DefaultSceneLook;
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

ViewShape view_shape(const sa2_view_shape* shape) {
    if (!shape) return ViewShape{};
    argument(shape->struct_size == sizeof(*shape), "invalid view_shape struct_size");
    argument(shape->reserved == 0, "view_shape reserved must be 0");
    const auto within = [](float value, float low, float high) { return std::isfinite(value) && value > low && value <= high; };
    argument(within(shape->cell_shrink, 0.0f, 1.0f), "cell_shrink must be finite, above 0 and at most 1");
    argument(within(shape->sticker_shrink, 0.0f, 1.0f), "sticker_shrink must be finite, above 0 and at most 1");
    argument(within(shape->d4, 1.0f, SA2_SHAPE_D4_MAX), "d4 must be finite, above 1 and at most SA2_SHAPE_D4_MAX");
    argument(within(shape->zoom, 0.0f, SA2_SHAPE_ZOOM_MAX), "zoom must be finite, above 0 and at most SA2_SHAPE_ZOOM_MAX");
    return ViewShape{shape->cell_shrink, shape->sticker_shrink, shape->d4, shape->zoom};
}

// ABI 9.
void view_focus(const sa2_view_focus* focus, ViewShape& shape) {
    const ViewShape defaults;
    if (!focus) {
        shape.projection = defaults.projection; shape.focus_cell = defaults.focus_cell;
        shape.focus_cos = defaults.focus_cos; shape.spread = defaults.spread;
        return;
    }
    argument(focus->struct_size == sizeof(*focus), "invalid view_focus struct_size");
    argument(focus->reserved == 0, "view_focus reserved must be 0");
    argument(focus->projection == SA2_PROJECTION_PERSPECTIVE || focus->projection == SA2_PROJECTION_ORTHOGRAPHIC,
             "projection must be SA2_PROJECTION_PERSPECTIVE or SA2_PROJECTION_ORTHOGRAPHIC");
    argument(focus->focus_cell == SA2_FOCUS_NONE || focus->focus_cell < 600, "focus_cell must be a cell 0..599 or SA2_FOCUS_NONE");
    argument(std::isfinite(focus->focus_cos) && focus->focus_cos >= -1.0f && focus->focus_cos <= 1.0f, "focus_cos must be finite and within -1..1");
    argument(std::isfinite(focus->spread) && focus->spread >= 1.0f && focus->spread <= 3.0f, "spread must be finite and within 1..3");
    shape.projection = focus->projection; shape.focus_cell = focus->focus_cell;
    shape.focus_cos = focus->focus_cell == SA2_FOCUS_NONE ? defaults.focus_cos : focus->focus_cos; shape.spread = focus->spread;
}

// ABI 10.
uint32_t view_layout_kind(const sa2_view_layout* layout) {
    if (!layout) return SA2_LAYOUT_CHAIN;
    argument(layout->struct_size == sizeof(*layout), "invalid view_layout struct_size");
    argument(layout->kind == SA2_LAYOUT_CHAIN || layout->kind == SA2_LAYOUT_HOPF_RINGS || layout->kind == SA2_LAYOUT_RING_SHEET,
             "kind must be SA2_LAYOUT_CHAIN, SA2_LAYOUT_HOPF_RINGS or SA2_LAYOUT_RING_SHEET");
    return layout->kind;
}

namespace {
using Q4 = std::array<double, 4>;
Q4 qmul(const Q4& a, const Q4& b) {
    return {a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3], a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
            a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1], a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0]};
}
Q4 conj(const Q4& a) { return {a[0], -a[1], -a[2], -a[3]}; }
double dot4(const Q4& a, const Q4& b) { return a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]; }
double dot3(const std::array<double, 3>& a, const std::array<double, 3>& b) { return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]; }
std::array<double, 3> im(const Q4& a) { return {a[1], a[2], a[3]}; }
// Im(o x o*), the image of the pure quaternion x under conjugation by o.
std::array<double, 3> turn(const Q4& o, const Q4& x) { return im(qmul(qmul(o, x), conj(o))); }
Q4 load(const float* v) { return {v[0], v[1], v[2], v[3]}; }
}

HopfTable hopf_table(const sa2_view_layout& layout, const std::function<std::array<float, 4>(uint32_t)>& centre) {
    constexpr double RingRadius = 2.62, BaseRadius = 8.063, RingScale = 0.28, Pi = 3.14159265358979323846;
    constexpr double SheetDx = 0.68, SheetDy = 0.57, SheetScale = 0.6; // GlobalLayout's ring sheet (B22)
    const auto finite = [](const float* v, size_t n) { for (size_t i = 0; i < n; ++i) if (!std::isfinite(v[i])) return false; return true; };
    argument(finite(layout.j, 4) && finite(layout.k, 4) && finite(layout.origins, 80), "a Hopf ring value is not finite");
    const Q4 j = load(layout.j), k = load(layout.k);
    argument(std::abs(j[0]) < 1e-4 && std::abs(dot4(j, j) - 1) < 1e-4, "J must be a unit pure quaternion");
    argument(std::abs(k[0]) < 1e-4 && std::abs(dot4(k, k) - 1) < 1e-4 && std::abs(dot4(j, k)) < 1e-4, "K must be a unit pure quaternion orthogonal to J");
    const Q4 jk = qmul(j, k);
    std::array<Q4, 20> origin{};
    std::array<std::array<double, 3>, 20> base{}, e1{}, e2{};
    for (size_t r = 0; r < 20; ++r) {
        origin[r] = load(layout.origins + 4 * r);
        argument(std::abs(dot4(origin[r], origin[r]) - 1) < 1e-4, "a ring origin is not a unit point");
        base[r] = turn(origin[r], j); e1[r] = turn(origin[r], k); e2[r] = turn(origin[r], jk);
    }
    // Each ring holds 30 cells with distinct half-steps of one parity; its cells in half-step order.
    std::array<std::array<int, 60>, 20> at{};
    for (auto& ring : at) ring.fill(-1);
    for (uint32_t cell = 0; cell < 600; ++cell) {
        argument(layout.ring_of[cell] < 20 && layout.half_step[cell] < 60, "a cell's ring or half-step is out of range");
        auto& slot = at[layout.ring_of[cell]][layout.half_step[cell]];
        argument(slot < 0, "two cells share a ring and a half-step");
        slot = int(cell);
    }
    std::array<uint32_t, 20> parity{};
    std::array<uint32_t, 600> cells{};
    for (size_t r = 0; r < 20; ++r) {
        int count[2] = {0, 0};
        for (int h = 0; h < 60; ++h) if (at[r][h] >= 0) ++count[h % 2];
        argument((count[0] == 30 && count[1] == 0) || (count[0] == 0 && count[1] == 30), "a ring does not hold 30 cells of one parity");
        parity[r] = count[1] ? 1u : 0u;
        for (int i = 0; i < 30; ++i) cells[r * 30 + i] = uint32_t(at[r][2 * i + parity[r]]);
    }
    HopfTable table(HopfTableRows, std::array<float, 4>{});
    const std::array<std::array<double, 3>, 3> orientation = {e1[0], e2[0], base[0]};
    for (uint32_t cell = 0; cell < 600; ++cell) {
        const auto raw = centre(cell);
        const Q4 c = {raw[0], raw[1], raw[2], raw[3]};
        const size_t r = layout.ring_of[cell];
        const int h = layout.half_step[cell];
        // The core's centre projects onto its ring's plane (origin, origin J) at the phase of its half-step, 6h degrees,
        // and lies near that plane (a ring's cells wind around its great circle at one distance): the rings fit the
        // core's cells.
        const Q4 oj = qmul(origin[r], j);
        const double along = dot4(c, origin[r]), across = dot4(c, oj);
        const double off = std::fmod(std::fmod(std::atan2(across, along) * 30 / Pi - h, 60.0) + 90.0, 60.0) - 30.0;
        argument(std::abs(off) < 0.01 && std::hypot(along, across) > 0.99,
                 ("the Hopf rings do not fit the core's cell centres (cell " + std::to_string(cell) + ")").c_str());
        // Look Lab's HopfRings.Chart: the frame (c, cJ, eK, eJK) with e = c (cos a + sin a J), a = 18 - 72h degrees.
        const double a = (18.0 - 72.0 * h) * Pi / 180;
        const Q4 e = qmul(c, Q4{std::cos(a), std::sin(a) * j[1], std::sin(a) * j[2], std::sin(a) * j[3]});
        const std::array<Q4, 3> frame = {qmul(c, j), qmul(e, k), qmul(e, jk)};
        // GlobalLayout: the cell's place on its ring and the chart turned into the ring's frame, all scaled.
        const double t = h * Pi / 30;
        std::array<double, 3> n{}, tangent{}, position{}, back{};
        for (int i = 0; i < 3; ++i) {
            n[i] = std::cos(t) * e1[r][i] + std::sin(t) * e2[r][i];
            tangent[i] = -std::sin(t) * e1[r][i] + std::cos(t) * e2[r][i];
            position[i] = BaseRadius * base[r][i] + RingRadius * n[i];
            back[i] = -base[r][i];
        }
        const std::array<std::array<double, 3>, 3> columns = {tangent, n, back};
        for (int row = 0; row < 3; ++row) {
            for (int col = 0; col < 4; ++col) {
                double value = 0;
                for (int m = 0; m < 3; ++m) value += RingScale * dot3(orientation[row], columns[m]) * frame[m][col];
                table[cell * 4 + row][col] = float(value);
            }
            table[cell * 4 + 3][row] = float(RingScale * dot3(orientation[row], position));
            // B22: the ring sheet lays the chart flat at its row and column.
            for (int col = 0; col < 4; ++col) table[HopfSheetRow + cell * 4 + row][col] = float(SheetScale * frame[row][col]);
        }
        table[HopfSheetRow + cell * 4 + 3] = {float(SheetScale * SheetDx * (h / 2.0 - 14.75)), float(-SheetScale * SheetDy * (double(r) - 9.5)), 0, 0};
    }
    for (size_t r = 0; r < 20; ++r) {
        const Q4 oj = qmul(origin[r], j);
        for (int i = 0; i < 4; ++i) { table[2400 + r][i] = float(origin[r][i]); table[2420 + r][i] = float(oj[i]); }
        table[2440 + r] = {float(base[r][0]), float(base[r][1]), float(base[r][2]), std::bit_cast<float>(parity[r])};
    }
    table[2460] = {float(j[0]), float(j[1]), float(j[2]), float(j[3])};
    for (size_t e = 0; e < 600; ++e) table[2461 + e / 4][e % 4] = std::bit_cast<float>(cells[e]);
    return table;
}

void Product::set_hopf(HopfTable table) {
    argument(table.size() == HopfTableRows, "a Hopf table has the wrong size");
    argument(hopf_.empty() || hopf_ == table, "the core keeps one set of Hopf rings; a different set is refused");
    hopf_ = std::move(table);
}

// ABI 7.
ViewStyle view_style(const sa2_view_style* style) {
    argument(style != nullptr, "view_style is NULL");
    argument(style->struct_size == sizeof(*style), "invalid view_style struct_size");
    argument(style->reserved == 0, "view_style reserved must be 0");
    argument(!(style->tint_mode & ~(SA2_STYLE_TINT_FILL | SA2_STYLE_TINT_OUTLINE)), "unknown view_style tint_mode bits");
    argument(style->dim_mode <= SA2_STYLE_HIDE_UNMARKED, "view_style dim_mode must be a SA2_STYLE_DIM_* or SA2_STYLE_HIDE_* value");
    const auto within = [](float value, float low, float high) { return std::isfinite(value) && value >= low && value <= high; };
    argument(within(style->dim_amount, 0.0f, 1.0f), "dim_amount must be finite and within 0..1");
    argument(within(style->outline_width, 0.0f, SA2_STYLE_OUTLINE_MAX), "outline_width must be finite and within 0..SA2_STYLE_OUTLINE_MAX");
    argument(std::all_of(std::begin(style->palette), std::end(style->palette), [&](float v) { return within(v, 0.0f, 1.0f); }),
        "every palette channel must be finite and within 0..1");
    argument(std::all_of(std::begin(style->matrix), std::end(style->matrix),
        [&](float v) { return within(v, -SA2_STYLE_MATRIX_MAX, SA2_STYLE_MATRIX_MAX); }), "every matrix entry must be finite and within the bound");
    // ADR-D02 B20: the slice.
    constexpr float pi = 3.14159265f;
    argument(style->slice_mode <= SA2_SLICE_INSIDE, "view_style slice_mode must be a SA2_SLICE_* value");
    argument(within(style->slice_shrink, 0.0f, 1.0f), "slice_shrink must be finite and within 0..1");
    argument(within(style->shell_min, 0.0f, pi) && within(style->shell_max, 0.0f, pi), "the shell's angles must be finite and within 0..pi");
    argument(std::all_of(std::begin(style->ghost), std::end(style->ghost), [&](float v) { return within(v, 0.0f, 1.0f); }),
        "every ghost channel must be finite and within 0..1");
    argument(style->slice_mode == SA2_SLICE_OFF || (style->slice_shrink > 0.0f && style->shell_min <= style->shell_max),
        "a slice needs slice_shrink above 0 and shell_min <= shell_max");
    ViewStyle out;
    std::copy(std::begin(style->palette), std::end(style->palette), out.palette.begin());
    for (int r = 0; r < 3; ++r) for (int c = 0; c < 3; ++c) out.rows[size_t(r) * 4 + c] = style->matrix[r * 3 + c];
    out.tint_mode = style->tint_mode; out.dim_mode = style->dim_mode;
    out.dim_amount = style->dim_amount; out.outline_width = style->outline_width;
    const float identity[9] = {1, 0, 0, 0, 1, 0, 0, 0, 1};
    out.matrix_on = std::equal(std::begin(style->matrix), std::end(style->matrix), std::begin(identity)) ? 0u : 1u;
    out.slice_mode = style->slice_mode;
    if (out.slice_mode != SA2_SLICE_OFF) {
        out.slice_shrink = style->slice_shrink;
        out.shell_min_cos = std::cos(style->shell_min); out.shell_max_cos = std::cos(style->shell_max);
        std::copy(std::begin(style->ghost), std::end(style->ghost), out.ghost.begin());
    }
    return out;
}

void validate_piece_style(const uint8_t* bytes, uint32_t channel) {
    argument(channel <= SA2_STYLE_CHANNEL_SLICE, "channel must be SA2_STYLE_CHANNEL_FILL, _OUTLINE or _SLICE");
    if (!bytes) return;
    const uint8_t allowed = uint8_t(channel == SA2_STYLE_CHANNEL_FILL ? (SA2_STYLE_INDEX | SA2_STYLE_DIM)
        : channel == SA2_STYLE_CHANNEL_SLICE ? 1u : SA2_STYLE_INDEX); // ADR-D02 B20: a slice byte is 0 or 1
    argument(std::all_of(bytes, bytes + SA2_PIECES, [&](uint8_t b) { return (b & ~allowed) == 0; }),
        "a piece style byte sets a bit its channel does not have");
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
    style_.reset(); // ABI 7: a piece style is bound to its revision
    transform_.reset(); // ADR-D02 B18: so is a transform
    return image_->revision;
}

// ABI 6: device-free, so it works during device loss; only views off the defaults are kept.
void Product::set_shape(uint64_t view, const ViewShape& shape) {
    // B22: a Hopf layout draws from the Hopf table, so no view takes one before the table exists.
    argument(shape.layout == SA2_LAYOUT_CHAIN || !hopf_.empty(), "a Hopf layout needs the Hopf table");
    if (shape == ViewShape{}) shapes_.erase(view);
    else shapes_[view] = shape;
}
ViewShape Product::shape(uint64_t view) const {
    const auto it = shapes_.find(view);
    return it == shapes_.end() ? ViewShape{} : it->second;
}

// ABI 7: device-free, so both work during device loss.
void Product::set_style(uint64_t revision, const uint8_t* bytes, uint32_t channel) {
    validate_piece_style(bytes, channel);
    state(image_.has_value(), "no engine image adopted");
    argument(revision == image_->revision, "a piece style names a revision other than the adopted one");
    if (!bytes && !style_) return;
    if (!style_) style_ = PieceStyle{revision, 0, std::vector<uint32_t>(SA2_PIECES, 0u), {}, false};
    const uint32_t shift = channel == SA2_STYLE_CHANNEL_FILL ? 0 : channel == SA2_STYLE_CHANNEL_OUTLINE ? 8 : 17; // B20: bit 17
    bool any = false, dim = false;
    for (uint32_t piece = 0; piece < SA2_PIECES; ++piece) {
        const uint32_t value = bytes ? bytes[piece] : 0u;
        auto& packed = style_->packed[piece];
        packed = (packed & ~(0xFFu << shift)) | (value << shift);
        any = any || (value & SA2_STYLE_INDEX) != 0;
        dim = dim || (value & SA2_STYLE_DIM) != 0;
    }
    style_->any[channel] = any;
    if (channel == SA2_STYLE_CHANNEL_FILL) style_->dim = dim;
    if (!style_->any[0] && !style_->any[1] && !style_->any[2] && !style_->dim && !style_->moving) { style_.reset(); return; }
    style_->version = ++style_version_;
}
// ADR-D02 B18 (ABI 10): device-free, as the piece style. The moving bit shares the style's word per piece (bit 16), so
// a new moving set uploads with the copy; a changed phase, plane or angle changes only the frame's constants.
void Product::set_transform(uint64_t revision, const float* u, const float* v, float angle, const uint8_t* mask, float phase) {
    if (mask) {
        argument(u != nullptr && v != nullptr, "a transform's plane is NULL");
        double uu = 0, vv = 0, uv = 0;
        for (int i = 0; i < 4; ++i) {
            argument(std::isfinite(u[i]) && std::isfinite(v[i]), "a transform's plane is not finite");
            uu += double(u[i]) * u[i]; vv += double(v[i]) * v[i]; uv += double(u[i]) * v[i];
        }
        const double tolerance = SA2_TRANSFORM_PLANE_TOLERANCE;
        argument(std::abs(uu - 1) <= tolerance && std::abs(vv - 1) <= tolerance && std::abs(uv) <= tolerance,
                 "a transform's plane is not two orthogonal unit vectors");
        const auto within = [](float value, float low, float high) { return std::isfinite(value) && value >= low && value <= high; };
        argument(within(angle, -3.14159274f, 3.14159274f), "a transform's angle must be finite and within -pi..pi");
        argument(within(phase, 0.0f, 1.0f), "a transform's phase must be finite and within 0..1");
        for (uint32_t piece = 0; piece < SA2_PIECES; ++piece) argument(mask[piece] <= 1, "a moving byte is not 0 or 1");
    }
    state(image_.has_value(), "no engine image adopted");
    argument(revision == image_->revision, "a transform names a revision other than the adopted one");
    if (!mask) {
        transform_.reset();
        if (!style_ || !style_->moving) return;
    } else if (!style_) {
        style_ = PieceStyle{revision, 0, std::vector<uint32_t>(SA2_PIECES, 0u), {}, false};
    }
    bool any = false, changed = false;
    for (uint32_t piece = 0; piece < SA2_PIECES; ++piece) {
        const uint32_t bit = mask && mask[piece] ? 0x10000u : 0u;
        auto& packed = style_->packed[piece];
        changed = changed || (packed & 0x10000u) != bit;
        packed = (packed & ~0x10000u) | bit;
        any = any || bit != 0;
    }
    style_->moving = any;
    if (any) {
        Turn t; std::copy(u, u + 4, t.u.begin()); std::copy(v, v + 4, t.v.begin()); t.angle = (phase - 1.0f) * angle;
        transform_ = t;
    } else {
        transform_.reset();
    }
    if (!style_->any[0] && !style_->any[1] && !style_->any[2] && !style_->dim && !style_->moving) { style_.reset(); return; }
    if (changed || style_->version == 0) style_->version = ++style_version_;
}
void Product::turn(ViewShape& shape) const {
    if (!transform_ || transform_->angle == 0.0f) return;
    shape.turn_u = transform_->u; shape.turn_v = transform_->v; shape.turn = transform_->angle;
}
void Product::set_view_style(uint32_t slot, const sa2_view_style* style) {
    argument(slot >= 1 && slot <= SA2_STYLE_SLOTS, "a style slot is 1..SA2_STYLE_SLOTS");
    if (!style) { view_styles_[slot - 1].reset(); return; }
    view_styles_[slot - 1] = view_style(style);
}
bool Product::styled(uint32_t slot, ViewStyle& out) const {
    if (slot < 1 || slot > SA2_STYLE_SLOTS || !view_styles_[slot - 1]) return false;
    const auto& view = *view_styles_[slot - 1];
    const bool fill = (view.tint_mode & SA2_STYLE_TINT_FILL) && style_ && style_->any[0];
    const bool outline = (view.tint_mode & SA2_STYLE_TINT_OUTLINE) && style_ && style_->any[1] && view.outline_width > 0;
    const bool dim = view.dim_amount > 0
        && ((view.dim_mode == SA2_STYLE_DIM_MARKED && style_ && style_->dim) || view.dim_mode == SA2_STYLE_DIM_UNMARKED);
    // ADR-D02 B17 (ABI 10): a hide that applies.
    const bool hide = (view.dim_mode == SA2_STYLE_HIDE_MARKED && style_ && style_->dim) || view.dim_mode == SA2_STYLE_HIDE_UNMARKED;
    if (!fill && !outline && !dim && !hide && !view.matrix_on && view.slice_mode == SA2_SLICE_OFF) return false; // B20: a slice
    out = view;
    return true;
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
    out.shape = shape(input.view);
    turn(out.shape); // ADR-D02 B18
    if (style_) { out.style = &style_->packed; out.style_version = style_->version; }
    out.styled = styled(1, out.view_style); // ABI 7: a one-view frame uses style slot 1
    if (input.resource_set) {
        const Entry* entry = set_entry(input.resource_set);
        state(entry && entry->active, "resource_set is not an active set");
        state(entry->stamp.view == input.view && entry->stamp.view_generation == input.view_generation
            && entry->stamp.publication_generation == input.publication_generation,
            "resource_set was prepared for another view, view generation or publication generation");
        out.set_ticket = sets_.at(input.resource_set);
    }
    for (uint32_t copy = 0; copy < EngineCopies; ++copy) {
        if (copies_[copy].valid && copies_[copy].revision == image_->revision && copies_[copy].style == out.style_version) {
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

// ABI 8. The frame input is planned as a one-view frame (its view, generation, set and camera are the first pane's),
// then every pane is checked; any refusal refuses the whole frame.
FramePlan Product::plan_panes(const sa2_frame_input& input, const sa2_pane* panes, uint32_t count, uint32_t width, uint32_t height) const {
    argument(panes != nullptr && count >= 1 && count <= SA2_PANES_MAX, "a composite frame has 1..SA2_PANES_MAX panes");
    const auto& first = panes[0];
    argument(input.view == first.view && input.view_generation == first.view_generation && input.resource_set == first.resource_set
        && std::equal(std::begin(input.camera_matrix), std::end(input.camera_matrix), std::begin(first.camera_matrix)),
        "the frame input's view, view generation, resource set and camera must be the first pane's");
    for (uint32_t i = 0; i < count; ++i) {
        const auto& pane = panes[i];
        argument(pane.flags == 0, "a pane's flags must be 0");
        argument(pane.style <= SA2_STYLE_SLOTS, "a pane's style is 0 or a style slot 1..SA2_STYLE_SLOTS");
        argument(pane.width >= 1 && pane.height >= 1 && pane.x >= 0 && pane.y >= 0
            && uint64_t(pane.x) + pane.width <= width && uint64_t(pane.y) + pane.height <= height, "a pane's rectangle must lie inside the target");
        argument(std::all_of(std::begin(pane.camera_matrix), std::end(pane.camera_matrix), [](float v) { return std::isfinite(v); }),
            "a pane's camera must be finite");
        for (uint32_t j = 0; j < i; ++j) {
            const auto& other = panes[j];
            const bool apart = uint64_t(pane.x) + pane.width <= uint64_t(other.x) || uint64_t(other.x) + other.width <= uint64_t(pane.x)
                || uint64_t(pane.y) + pane.height <= uint64_t(other.y) || uint64_t(other.y) + other.height <= uint64_t(pane.y);
            argument(apart, "pane rectangles must not overlap");
        }
    }
    FramePlan out = plan(input, width, height);
    // ADR-D02 B20: a slot with a slice and a ghost claims the transform for its panes.
    const auto ghosts = [&](uint32_t slot) {
        if (slot < 1 || slot > SA2_STYLE_SLOTS || !view_styles_[slot - 1]) return false;
        const auto& view = *view_styles_[slot - 1];
        return view.slice_mode != SA2_SLICE_OFF && view.ghost[3] > 0.0f;
    };
    bool ghost_frame = false;
    for (uint32_t i = 0; i < count; ++i) ghost_frame = ghost_frame || ghosts(panes[i].style);
    for (uint32_t i = 0; i < count; ++i) {
        const auto& pane = panes[i];
        if (auto view = views_.find(pane.view); view != views_.end())
            state(pane.view_generation >= view->second, "a pane's view_generation must not decrease");
        PanePlan made;
        made.pane = pane;
        made.publication_generation = i == 0 ? input.publication_generation : 0;
        made.shape = shape(pane.view);
        // ADR-D02 B18; B20: a frame with a ghost pane turns the pieces only in its ghost panes.
        if (!ghost_frame || ghosts(pane.style)) turn(made.shape);
        made.styled = pane.style != 0 && styled(pane.style, made.view_style);
        if (pane.resource_set) {
            const Entry* entry = set_entry(pane.resource_set);
            state(entry && entry->active, "a pane's resource_set is not an active set");
            // Each view keeps its own publication generation; the first pane's set is the frame input's.
            state(entry->stamp.view == pane.view && entry->stamp.view_generation == pane.view_generation
                && (i != 0 || entry->stamp.publication_generation == input.publication_generation),
                "a pane's resource_set was prepared for another view or view generation");
            state(entry->stamp.base_revision == input.engine_revision, "a pane's resource_set was prepared at another engine revision");
            made.set_ticket = sets_.at(pane.resource_set);
            made.publication_generation = entry->stamp.publication_generation;
        }
        out.panes.push_back(made);
    }
    return out;
}

void Product::pane_evals(uint64_t frame, const std::vector<uint64_t>& evals) {
    for (auto& record : frames_) {
        if (record.id != frame) continue;
        for (size_t i = 0; i < record.panes.size() && i < evals.size(); ++i) record.panes[i].evals = evals[i];
        return;
    }
}

uint32_t Product::pane_rows(uint64_t id, sa2_pane_row* rows, uint32_t capacity) const {
    validate_rows(rows, capacity);
    const Frame* record = frame(id);
    state(record != nullptr && !record->panes.empty(), "the frame is not a retained composite frame");
    state(record->produced && record->error == SA2_OK, "the composite frame has not produced");
    const auto count = uint32_t(record->panes.size());
    for (uint32_t i = 0; i < count && i < capacity; ++i) {
        rows[i] = sa2_pane_row{sizeof(sa2_pane_row), i, record->panes[i].pane.view, record->panes[i].pane.view_generation, record->panes[i].evals};
    }
    return count;
}

void Product::commit(const sa2_frame_input& input, const FramePlan& plan, uint64_t frame, uint32_t ring) {
    auto& surface = surfaces_[input.surface];
    surface.first = input.sequence;
    surface.second = std::max(surface.second, input.surface_generation);
    auto& view = views_[input.view];
    view = std::max(view, input.view_generation);
    for (const auto& pane : plan.panes) { // ABI 8
        auto& generation = views_[pane.pane.view];
        generation = std::max(generation, pane.pane.view_generation);
        if (pane.set_ticket) entries_.at(pane.set_ticket).last_frame = frame;
    }
    if (plan.upload) {
        copies_[plan.copy] = {plan.image->revision, frame, ++copy_sequence_, plan.image->pose_count, true, plan.style_version};
        counters_.bytes_uploaded += plan.image->bytes() + (plan.style ? uint64_t(SA2_PIECES) * 4 : 0);
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
    record.shape = plan.shape;
    record.styled = plan.styled; // ADR-D02 B17 (ABI 10): its picks follow the style it drew with
    record.view_style = plan.view_style;
    record.style_version = plan.style_version;
    for (const auto& pane : plan.panes)
        record.panes.push_back(PaneRecord{pane.pane, pane.set_ticket, 0, pane.publication_generation, pane.shape, pane.styled, pane.view_style});
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
    if (!record->panes.empty()) return pick_pane(*record, out);
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
    // ADR-D02 B17 (ABI 10): a styled frame's copy must still hold the piece style it drew with; so must a turning
    // frame's, styled or not, since its moving mask lives in the same words (B18 review finding B18-03).
    const bool bound = record->styled || record->shape.turn != 0.0f;
    if (!copy.valid || copy.revision != record->input.engine_revision || (bound && copy.style != record->style_version)) {
        out.state = SA2_PICK_STALE;
        out.stale = SA2_PICK_STALE_ENGINE;
        return out;
    }
    out.pass = true;
    out.input.styled = record->styled;
    out.input.view_style = record->view_style;
    out.input.piece_style = record->style_version != 0;
    std::copy(std::begin(record->input.camera_matrix), std::end(record->input.camera_matrix), out.input.camera.begin());
    out.input.width = record->input.viewport_width;
    out.input.height = record->input.viewport_height;
    out.input.copy = record->copy;
    out.input.pose_count = copy.pose_count;
    out.input.set_ticket = record->set_ticket;
    out.input.shape = record->shape;
    return out;
}

// ABI 8: a pick on a composite frame: the pane that holds the point, bound to that pane's view and generation.
sa2_frame_input Product::pane_binding(const Frame& record, int32_t pane) {
    auto binding = record.input;
    if (pane < 0) return binding;
    const auto& row = record.panes.at(size_t(pane)).pane;
    binding.view = row.view;
    binding.view_generation = row.view_generation;
    binding.resource_set = row.resource_set;
    binding.publication_generation = record.panes.at(size_t(pane)).publication_generation;
    std::copy(std::begin(row.camera_matrix), std::end(row.camera_matrix), std::begin(binding.camera_matrix));
    return binding;
}
PickPlan Product::pick_pane(const Frame& record, PickPlan out) const {
    const auto x = out.input.x, y = out.input.y;
    for (size_t i = 0; i < record.panes.size(); ++i) {
        const auto& pane = record.panes[i].pane;
        if (x < uint32_t(pane.x) || y < uint32_t(pane.y) || x >= uint32_t(pane.x) + pane.width || y >= uint32_t(pane.y) + pane.height) continue;
        out.pane = int32_t(i);
        const auto binding = pane_binding(record, out.pane);
        if (const uint32_t reason = stale(binding, record.target_generation)) {
            out.state = SA2_PICK_STALE;
            out.stale = reason;
            return out;
        }
        if (!pane.resource_set) {
            out.state = SA2_PICK_READY; // nothing was pickable in this pane
            return out;
        }
        const auto mapped = sets_.find(pane.resource_set);
        if (mapped == sets_.end() || mapped->second != record.panes[i].set_ticket) {
            out.state = SA2_PICK_STALE;
            out.stale = SA2_PICK_STALE_PUBLICATION;
            return out;
        }
        const Copy& copy = copies_[record.copy];
        const bool styled = record.panes[i].styled; // ADR-D02 B17 (ABI 10)
        const bool bound = styled || record.panes[i].shape.turn != 0.0f; // B18-03: a turning pane's moving mask
        if (!copy.valid || copy.revision != record.input.engine_revision || (bound && copy.style != record.style_version)) {
            out.state = SA2_PICK_STALE;
            out.stale = SA2_PICK_STALE_ENGINE;
            return out;
        }
        out.pass = true;
        out.input.styled = styled;
        out.input.view_style = record.panes[i].view_style;
        out.input.piece_style = record.style_version != 0;
        std::copy(std::begin(pane.camera_matrix), std::end(pane.camera_matrix), out.input.camera.begin());
        out.input.width = record.input.viewport_width;
        out.input.height = record.input.viewport_height;
        out.input.origin_x = pane.x;
        out.input.origin_y = pane.y;
        out.input.view_width = pane.width;
        out.input.view_height = pane.height;
        out.input.copy = record.copy;
        out.input.pose_count = copy.pose_count;
        out.input.set_ticket = record.panes[i].set_ticket;
        out.input.shape = record.panes[i].shape;
        return out;
    }
    argument(false, "the point lies in no pane of the composite frame");
    return out;
}

void Product::pick_commit(uint64_t ticket, const PickPlan& plan, uint64_t fence) {
    const Frame* record = frame(plan.frame);
    state(record != nullptr, "frame is not retained");
    Pick pick;
    pick.binding = pane_binding(*record, plan.pane);
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

// C9, owner item 9: the ledger's frame is the trace index, and the input's
// sequence carries the frame's own present count into its confirmed row.
void TracePresents::presented(uint64_t index, uint64_t generation, uint32_t present_count) {
    sa2_frame_input input{};
    input.sequence = present_count;
    ledger_.presented(input, index, generation, present_count);
}

void TracePresents::sample(const PresentSample& sample) {
    ledger_.sample(sample);
    std::array<sa2_presented_row, 16> rows{};
    for (uint32_t count; (count = ledger_.rows(rows.data(), uint32_t(rows.size()))) != 0;)
        confirmed_.insert(confirmed_.end(), rows.begin(), rows.begin() + count);
}

std::string TracePresents::text() const {
    std::string text;
    for (const auto& row : confirmed_)
        text += Json(Json::Object{{"frame", row.frame_id}, {"present_count", row.sequence}, {"display_qpc", row.display_qpc}}).dump() + '\n';
    return text;
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
