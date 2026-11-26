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
    bool poses = false; // C9 jumble-on: --warp --poses runs only the product pose check (run_poses)
    bool shape = false; // ABI 6: --warp --shape runs only the view shape check (run_shape)
    bool style = false; // ABI 7: --warp --style runs only the piece style check (run_style)
    bool panes = false; // ABI 8: --warp --panes runs only the composite frame check (run_panes)
    bool focus = false; // ABI 9: --warp --focus runs only the view focus check (run_focus)
    std::filesystem::path hopf; // ABI 10: --warp --hopf FILE runs only the view layout check (run_hopf)
    bool sweep = false; // ADR-D02 B18: --warp --sweep runs only the transform check (run_sweep)
    // C9 U5c: --warp --measure-stages DIR [--size WxH] [--frames N] [--traced N] [--turn-ms X] --out FILE measures U
    // for W3 with the stage pair in DIR (run_measure); the default size is the owner's native 2560 x 1600.
    std::filesystem::path measure;
    uint32_t width = 2560, height = 1600, frames = 0, traced = 0;
    double turn_ms = 190;
    // C9 (26 Nov 2026): --hardware or --warp --measure-prepare N --out FILE times N product preparations on one view,
    // each activated and its predecessor retired, as the shell does per twist (run_measure_prepare).
    uint32_t prepares = 0;
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
void run_poses(Suite& suite, const Options& options);
void run_shape(Suite& suite, const Options& options);
void run_style(Suite& suite, const Options& options);
void run_panes(Suite& suite, const Options& options);
void run_focus(Suite& suite, const Options& options);
void run_hopf(Suite& suite, const Options& options);
void run_sweep(Suite& suite, const Options& options);
void run_measure(const Options& options);
void run_measure_prepare(const Options& options);
int run_child(const Options& options);
}
