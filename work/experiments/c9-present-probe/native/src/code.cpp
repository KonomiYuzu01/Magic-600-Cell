#include "code.h"

#include <algorithm>
#include <cmath>
#include <cstdlib>

#include "../c9pp.h"

namespace c9 {

uint16_t crc16(const uint8_t* data, size_t size) {
    uint16_t crc = 0xFFFF;
    for (size_t i = 0; i < size; ++i) {
        crc = static_cast<uint16_t>(crc ^ (static_cast<uint16_t>(data[i]) << 8));
        for (int bit = 0; bit < 8; ++bit)
            crc = (crc & 0x8000) ? static_cast<uint16_t>((crc << 1) ^ 0x1021) : static_cast<uint16_t>(crc << 1);
    }
    return crc;
}

uint16_t code_check(uint32_t frame_id, uint32_t epoch) {
    const uint8_t bytes[5] = {static_cast<uint8_t>(frame_id), static_cast<uint8_t>(frame_id >> 8),
                              static_cast<uint8_t>(frame_id >> 16), static_cast<uint8_t>(frame_id >> 24),
                              static_cast<uint8_t>(epoch)};
    return crc16(bytes, sizeof(bytes));
}

void code_bits(uint32_t frame_id, uint32_t epoch, uint8_t bits[kCells]) {
    const uint16_t check = code_check(frame_id, epoch);
    for (uint32_t i = 0; i < 32; ++i) bits[i] = static_cast<uint8_t>((frame_id >> (31 - i)) & 1u);
    for (uint32_t i = 0; i < 8; ++i) bits[32 + i] = static_cast<uint8_t>((epoch >> (7 - i)) & 1u);
    for (uint32_t i = 0; i < 16; ++i) bits[40 + i] = static_cast<uint8_t>((check >> (15 - i)) & 1u);
    for (uint32_t i = 0; i < 8; ++i) bits[56 + i] = kSync[i];
}

void code_origins(uint32_t width, uint32_t height, uint32_t cell, int32_t ox[2], int32_t oy[2]) {
    const int32_t extent = static_cast<int32_t>(code_extent(cell));
    const int32_t margin = static_cast<int32_t>(code_margin(cell));
    ox[0] = margin;
    oy[0] = margin;
    ox[1] = static_cast<int32_t>(width) - margin - extent;
    oy[1] = static_cast<int32_t>(height) - margin - extent;
}

void code_rects(uint32_t frame_id, uint32_t epoch, uint32_t width, uint32_t height, uint32_t cell,
                std::vector<Rect>& white, std::vector<Rect>& black) {
    uint8_t bits[kCells];
    code_bits(frame_id, epoch, bits);
    int32_t ox[2], oy[2];
    code_origins(width, height, cell, ox, oy);
    white.clear();
    black.clear();
    const int32_t c = static_cast<int32_t>(cell);
    for (int copy = 0; copy < 2; ++copy) {
        for (uint32_t i = 0; i < kCells; ++i) {
            const int32_t col = static_cast<int32_t>(i % kGrid), row = static_cast<int32_t>(i / kGrid);
            const Rect r{ox[copy] + col * c, oy[copy] + row * c, ox[copy] + (col + 1) * c, oy[copy] + (row + 1) * c};
            (bits[i] ? white : black).push_back(r);
        }
    }
}

namespace {
void fill(uint8_t* bgra, uint32_t width, uint32_t height, uint32_t stride, const Rect& r, uint32_t rgb) {
    const int32_t x0 = std::max(r.left, 0), y0 = std::max(r.top, 0);
    const int32_t x1 = std::min(r.right, static_cast<int32_t>(width)), y1 = std::min(r.bottom, static_cast<int32_t>(height));
    for (int32_t y = y0; y < y1; ++y) {
        uint8_t* p = bgra + static_cast<size_t>(y) * stride + static_cast<size_t>(x0) * 4;
        for (int32_t x = x0; x < x1; ++x, p += 4) {
            p[0] = static_cast<uint8_t>(rgb);
            p[1] = static_cast<uint8_t>(rgb >> 8);
            p[2] = static_cast<uint8_t>(rgb >> 16);
            p[3] = 255;
        }
    }
}
}  // namespace

void render_bgra(uint32_t frame_id, uint32_t epoch, uint32_t base_rgb, uint32_t cell, uint8_t* bgra,
                 uint32_t width, uint32_t height, uint32_t stride) {
    fill(bgra, width, height, stride, Rect{0, 0, static_cast<int32_t>(width), static_cast<int32_t>(height)}, base_rgb);
    std::vector<Rect> white, black;
    code_rects(frame_id, epoch, width, height, cell, white, black);
    for (const Rect& r : white) fill(bgra, width, height, stride, r, kWhite);
    for (const Rect& r : black) fill(bgra, width, height, stride, r, kBlack);
}

bool mean_rgb(const uint8_t* bgra, uint32_t bw, uint32_t bh, uint32_t stride, int32_t x0, int32_t y0,
              int32_t x1, int32_t y1, uint32_t* rgb) {
    x0 = std::max(x0, 0);
    y0 = std::max(y0, 0);
    x1 = std::min(x1, static_cast<int32_t>(bw));
    y1 = std::min(y1, static_cast<int32_t>(bh));
    if (x1 <= x0 || y1 <= y0) return false;
    uint64_t sum[3] = {0, 0, 0};
    for (int32_t y = y0; y < y1; ++y) {
        const uint8_t* p = bgra + static_cast<size_t>(y) * stride + static_cast<size_t>(x0) * 4;
        for (int32_t x = x0; x < x1; ++x, p += 4) {
            sum[0] += p[2];
            sum[1] += p[1];
            sum[2] += p[0];
        }
    }
    const uint64_t n = static_cast<uint64_t>(x1 - x0) * static_cast<uint64_t>(y1 - y0);
    *rgb = static_cast<uint32_t>(((sum[0] + n / 2) / n) << 16 | ((sum[1] + n / 2) / n) << 8 | ((sum[2] + n / 2) / n));
    return true;
}

Corner decode_corner(const uint8_t* bgra, uint32_t bw, uint32_t bh, uint32_t stride, double origin_x,
                     double origin_y, double scale_x, double scale_y, int32_t cx, int32_t cy, uint32_t cell) {
    Corner out;
    uint8_t bits[kCells] = {};
    const double c = static_cast<double>(cell);
    for (uint32_t i = 0; i < kCells; ++i) {
        const double col = static_cast<double>(i % kGrid), row = static_cast<double>(i / kGrid);
        // Inner 40 % of the cell, mapped to buffer pixels and rounded inward.
        const int32_t x0 = static_cast<int32_t>(std::ceil(origin_x + (cx + col * c + 0.3 * c) * scale_x));
        const int32_t x1 = static_cast<int32_t>(std::floor(origin_x + (cx + col * c + 0.7 * c) * scale_x));
        const int32_t y0 = static_cast<int32_t>(std::ceil(origin_y + (cy + row * c + 0.3 * c) * scale_y));
        const int32_t y1 = static_cast<int32_t>(std::floor(origin_y + (cy + row * c + 0.7 * c) * scale_y));
        if (x0 < 0 || y0 < 0 || x1 > static_cast<int32_t>(bw) || y1 > static_cast<int32_t>(bh) || x1 <= x0 || y1 <= y0) {
            out.status = C9PP_CORNER_OUT_OF_BOUNDS;
            return out;
        }
        uint32_t rgb = 0;
        mean_rgb(bgra, bw, bh, stride, x0, y0, x1, y1, &rgb);
        const int mr = static_cast<int>(rgb >> 16 & 255), mg = static_cast<int>(rgb >> 8 & 255), mb = static_cast<int>(rgb & 255);
        int deviation = 0;
        for (int32_t y = y0; y < y1; ++y) {
            const uint8_t* p = bgra + static_cast<size_t>(y) * stride + static_cast<size_t>(x0) * 4;
            for (int32_t x = x0; x < x1; ++x, p += 4)
                deviation = std::max({deviation, std::abs(p[2] - mr), std::abs(p[1] - mg), std::abs(p[0] - mb)});
        }
        const int chroma = std::max({mr, mg, mb}) - std::min({mr, mg, mb});
        const int luma = (mr + mg + mb) / 3;
        if (chroma > 48 || deviation > 48 || (luma > 96 && luma < 160)) {
            ++out.ambiguous;
            continue;
        }
        bits[i] = luma >= 160 ? 1 : 0;
    }
    if (out.ambiguous) {
        out.status = C9PP_CORNER_AMBIGUOUS;
        return out;
    }
    for (uint32_t i = 0; i < 8; ++i) {
        if (bits[56 + i] != kSync[i]) {
            out.status = C9PP_CORNER_BAD_SYNC;
            return out;
        }
    }
    uint32_t frame_id = 0, epoch = 0, check = 0;
    for (uint32_t i = 0; i < 32; ++i) frame_id = frame_id << 1 | bits[i];
    for (uint32_t i = 0; i < 8; ++i) epoch = epoch << 1 | bits[32 + i];
    for (uint32_t i = 0; i < 16; ++i) check = check << 1 | bits[40 + i];
    if (check != code_check(frame_id, epoch)) {
        out.status = C9PP_CORNER_BAD_CHECK;
        return out;
    }
    out.status = C9PP_CORNER_OK;
    out.frame_id = frame_id;
    out.epoch = epoch;
    return out;
}

int32_t combine(const Corner& a, const Corner& b, uint32_t* frame_id, uint32_t* epoch) {
    const bool oka = a.status == C9PP_CORNER_OK, okb = b.status == C9PP_CORNER_OK;
    *frame_id = 0;
    *epoch = 0;
    if (oka && okb) {
        if (a.frame_id != b.frame_id || a.epoch != b.epoch) return C9PP_SAMPLE_MISMATCH;
        *frame_id = a.frame_id;
        *epoch = a.epoch;
        return C9PP_SAMPLE_MATCH;
    }
    if (oka || okb) {
        const Corner& k = oka ? a : b;
        *frame_id = k.frame_id;
        *epoch = k.epoch;
        return C9PP_SAMPLE_ONE_CORNER;
    }
    return C9PP_SAMPLE_UNDECODABLE;
}

}  // namespace c9
