#pragma once

#include "pure.h"
#include <filesystem>
#include <functional>
#include <stdexcept>

namespace sa2test {
struct Unsupported : std::runtime_error { using std::runtime_error::runtime_error; };
struct Options {
    std::string mode, child, vectors;
    std::filesystem::path out;
    bool debug = false;
    bool stages = false; // C9 U5: --warp --stages runs only the user-stage check (run_stages)
    // C9 U5c: --warp --measure-stages DIR [--size WxH] [--frames N] [--traced N] [--turn-ms X] --out FILE measures U
    // for W3 with the stage pair in DIR (run_measure); the default size is the owner's native 2560 x 1600.
    std::filesystem::path measure;
    uint32_t width = 2560, height = 1600, frames = 0, traced = 0;
    double turn_ms = 190;
};
void expect(bool condition, const char* reason);
struct Suite {
    std::vector<sa2::Check> checks;
    void check(const std::string& name, const std::function<void()>& body);
    bool passed() const;
};
std::filesystem::path executable_path();
void run_gpu(Suite& suite, const Options& options);
void run_stages(Suite& suite, const Options& options);
void run_measure(const Options& options);
int run_child(const Options& options);
}
