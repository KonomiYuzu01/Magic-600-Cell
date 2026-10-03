#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace sb {
constexpr std::uint64_t ring_size = 3;
std::size_t ring_slot(std::uint64_t sequence);
std::uint64_t previous_use(std::uint64_t sequence);
std::array<std::uint8_t, 4> encode_sequence(std::uint64_t sequence);
void fill_pattern(void* data, std::size_t pitch, unsigned width, unsigned height, std::uint64_t sequence);
bool verify_pattern(const void* data, std::size_t pitch, unsigned width, unsigned height,
                    std::uint64_t sequence, std::string& reason);

struct Options {
    std::vector<std::string> modes{"all"};
    std::uint64_t iterations = 1000;
    bool warp = false;
    bool debug = false;
    bool selftest = false;
    bool help = false;
    // Test hook: every drain of this role ("producer" or "consumer") fails as an
    // unconfirmed wait would. Empty: no injection.
    std::string inject_unconfirmed_drain;
    std::string out;
    // Child: heap, producer-ready fence, consumer-free fence, report pipe, cancel event.
    std::optional<std::array<std::uintptr_t, 5>> handles;
    std::optional<std::uint64_t> luid;
    unsigned width = 1024;
    unsigned height = 1024;
};
Options parse_options(const std::vector<std::string>& args);

struct Result {
    std::string mode;
    std::string status = "fail";
    std::string reason;
    std::uint64_t iterations = 0;
    std::uint64_t submitted = 0;
    std::uint64_t verified = 0;
    std::optional<std::uint64_t> first_failure;
    std::optional<bool> singleton;
    std::optional<bool> consumer_singleton;
    std::string producer_luid;
    std::string consumer_luid;
    std::string teardown = "not_requested";
    unsigned live_objects = 0;
    std::uint64_t live_report_messages = 0;
    unsigned resize_count = 0;
    std::vector<double> timings;
    std::vector<std::string> errors;
};
double mean(const std::vector<double>& samples);
double p99(std::vector<double> samples);
std::string json_string(const std::string& value);
std::string result_json(const std::vector<Result>& results, bool warp, bool debug);
bool report_succeeded(const std::vector<Result>& results);
void selftest();
}
