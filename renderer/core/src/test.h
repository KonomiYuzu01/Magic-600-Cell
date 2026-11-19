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
int run_child(const Options& options);
}
