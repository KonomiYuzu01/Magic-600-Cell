// Frame code layout, encoder and decoder. Pure; shared by the GPU path, the screen
// sampler and the --cpu check so that all three use one definition.
#pragma once
#include <cstddef>
#include <cstdint>
#include <vector>

namespace c9 {

// An 8 x 8 grid of square cells, row-major:
//   cells  0..31  frame_id, most significant bit first
//   cells 32..39  epoch (low 8 bits)
//   cells 40..55  CRC-16/CCITT-FALSE over frame_id (little endian, 4 bytes) and epoch (1 byte)
//   cells 56..63  fixed sync pattern 1 0 1 1 0 0 1 0
// Bit 1 is drawn kWhite, bit 0 kBlack. One copy sits one cell in from the top-left corner of
// the core rectangle, the other one cell in from the bottom-right corner.
constexpr uint32_t kGrid = 8;
constexpr uint32_t kCells = kGrid * kGrid;
constexpr uint8_t kSync[8] = {1, 0, 1, 1, 0, 0, 1, 0};
constexpr uint32_t kWhite = 0xEBEBEBu;
constexpr uint32_t kBlack = 0x0C0C0Cu;

struct Rect {
    int32_t left, top, right, bottom;
};

struct Corner {
    int32_t status = 0;
    uint32_t frame_id = 0, epoch = 0, ambiguous = 0;
};

uint16_t crc16(const uint8_t* data, size_t size);
uint16_t code_check(uint32_t frame_id, uint32_t epoch);
void code_bits(uint32_t frame_id, uint32_t epoch, uint8_t bits[kCells]);
inline uint32_t code_extent(uint32_t cell) { return kGrid * cell; }
inline uint32_t code_margin(uint32_t cell) { return cell; }
// Smallest core width and height that keep both copies apart.
inline uint32_t min_rect_extent(uint32_t cell) { return 2 * (code_extent(cell) + code_margin(cell)); }
void code_origins(uint32_t width, uint32_t height, uint32_t cell, int32_t ox[2], int32_t oy[2]);
// Rectangles (core-rectangle coordinates) to clear white and black on top of the base colour.
void code_rects(uint32_t frame_id, uint32_t epoch, uint32_t width, uint32_t height, uint32_t cell,
                std::vector<Rect>& white, std::vector<Rect>& black);
void render_bgra(uint32_t frame_id, uint32_t epoch, uint32_t base_rgb, uint32_t cell, uint8_t* bgra,
                 uint32_t width, uint32_t height, uint32_t stride);
// Decodes one copy whose top-left cell corner is at client position (cx, cy) relative to the
// core rectangle; buffer position = origin + client * scale.
Corner decode_corner(const uint8_t* bgra, uint32_t bw, uint32_t bh, uint32_t stride, double origin_x,
                     double origin_y, double scale_x, double scale_y, int32_t cx, int32_t cy, uint32_t cell);
// Mean colour (0x00RRGGBB) of a buffer region; false when the region is empty or outside.
bool mean_rgb(const uint8_t* bgra, uint32_t bw, uint32_t bh, uint32_t stride, int32_t x0, int32_t y0,
              int32_t x1, int32_t y1, uint32_t* rgb);
int32_t combine(const Corner& a, const Corner& b, uint32_t* frame_id, uint32_t* epoch);

}  // namespace c9
