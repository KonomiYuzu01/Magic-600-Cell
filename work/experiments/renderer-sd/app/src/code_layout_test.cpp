#include "code_layout.h"
#include <algorithm>
#include <iostream>
#include <stdexcept>

int main() {
    try {
        const auto check = [](bool ok) { if (!ok) throw std::runtime_error("code layout fixture failed"); };
        constexpr char input[] = "123456789";
        check(sd::crc16(reinterpret_cast<const std::uint8_t*>(input), 9) == 0x29B1);
        check(sd::code({1, 0, 0}) == 0x4BB0000000000001ULL);
        check(sd::code({3735928559u, 2, 2748}) == 0x786FABC2DEADBEEFULL);
        check(sd::code({1000, 1, 7}) == 0x5DD00071000003E8ULL);
        for (const auto size : {std::array<unsigned, 2>{128, 128}, {160, 144}}) {
            const sd::Fields wanted{3735928559u, 2, 2748};
            auto pixels = sd::expectedImage(size[0], size[1], wanted);
            sd::Fields decoded{};
            check(sd::decode(pixels.data(), pixels.size() * 4, size[0], size[1], size[0], size[1], false, decoded));
            check(decoded == wanted);
            pixels[4 * size[0] + 4][0] ^= 255;
            check(!sd::decode(pixels.data(), pixels.size() * 4, size[0], size[1], size[0], size[1], false, decoded));
            pixels = sd::expectedImage(size[0], size[1], wanted);
            const unsigned width = size[0] + 31, height = size[1] + 17;
            std::vector<sd::Pixel> composite(std::size_t(width) * height, {19, 37, 113, 255});
            for (unsigned y = 0; y < size[1]; ++y)
                std::copy_n(pixels.begin() + y * size[0], size[0], composite.begin() + y * width);
            for (unsigned corner = 0; corner < 2; ++corner) {
                const auto x0 = corner ? size[0] - 64 : 0;
                const auto y0 = corner ? size[1] - 64 : 0;
                for (unsigned bit = 0; bit < 64; ++bit) {
                    auto& center = composite[std::size_t(y0 + (bit / 8) * 8 + 4) * width + x0 + (bit % 8) * 8 + 4];
                    center[2] = 255 - center[0];
                }
            }
            for (const bool bgra : {false, true}) {
                if (bgra) for (auto& pixel : composite) std::swap(pixel[0], pixel[2]);
                check(sd::decode(composite.data(), composite.size() * 4, width, height, size[0], size[1], bgra, decoded));
                check(decoded == wanted);
            }
            check(!sd::decode(composite.data(), composite.size() * 4, width, height, size[0], size[1], false, decoded));
        }
        std::cout << "sd_code_layout_test: ok\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
