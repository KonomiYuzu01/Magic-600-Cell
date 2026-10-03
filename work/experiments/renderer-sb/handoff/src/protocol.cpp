#include "protocol.h"

#include <algorithm>
#include <charconv>
#include <cmath>
#include <cstring>
#include <iomanip>
#include <limits>
#include <numeric>
#include <sstream>
#include <stdexcept>

namespace sb {
std::size_t ring_slot(std::uint64_t sequence) {
    if (!sequence) throw std::invalid_argument("sequence starts at 1");
    return static_cast<std::size_t>((sequence - 1) % ring_size);
}
std::uint64_t previous_use(std::uint64_t sequence) {
    if (!sequence) throw std::invalid_argument("sequence starts at 1");
    return sequence > ring_size ? sequence - ring_size : 0;
}
std::array<std::uint8_t, 4> encode_sequence(std::uint64_t sequence) {
    // RGBA stores the low 32 bits, little endian; fence values remain full 64 bits.
    return {static_cast<std::uint8_t>(sequence), static_cast<std::uint8_t>(sequence >> 8),
            static_cast<std::uint8_t>(sequence >> 16), static_cast<std::uint8_t>(sequence >> 24)};
}
static void check_extent(std::size_t pitch, unsigned width, unsigned height) {
    if (!width || !height || pitch < std::size_t(width) * 4 ||
        pitch > std::numeric_limits<std::size_t>::max() / height)
        throw std::invalid_argument("invalid RGBA8 row pitch or extent");
}
void fill_pattern(void* data, std::size_t pitch, unsigned width, unsigned height, std::uint64_t sequence) {
    check_extent(pitch, width, height);
    const auto rgba = encode_sequence(sequence);
    auto* bytes = static_cast<std::uint8_t*>(data);
    for (unsigned y = 0; y < height; ++y)
        for (unsigned x = 0; x < width; ++x)
            std::memcpy(bytes + y * pitch + std::size_t(x) * 4, rgba.data(), 4);
}
bool verify_pattern(const void* data, std::size_t pitch, unsigned width, unsigned height,
                    std::uint64_t sequence, std::string& reason) {
    check_extent(pitch, width, height);
    const auto rgba = encode_sequence(sequence);
    const auto* bytes = static_cast<const std::uint8_t*>(data);
    for (unsigned y = 0; y < height; ++y) {
        for (unsigned x = 0; x < width; ++x) {
            const auto* texel = bytes + y * pitch + std::size_t(x) * 4;
            if (std::memcmp(texel, rgba.data(), 4) != 0) {
                std::ostringstream out;
                out << "sequence " << sequence << " mismatch at texel (" << x << ',' << y << "): RGBA [";
                for (unsigned c = 0; c < 4; ++c) out << (c ? "," : "") << unsigned(texel[c]);
                out << "] expected [";
                for (unsigned c = 0; c < 4; ++c) out << (c ? "," : "") << unsigned(rgba[c]);
                out << ']';
                reason = out.str();
                return false;
            }
        }
    }
    reason.clear();
    return true;
}

static std::uint64_t number(const std::string& text, std::uint64_t maximum, bool allow_zero = false) {
    std::uint64_t value = 0;
    const auto parsed = std::from_chars(text.data(), text.data() + text.size(), value);
    if (text.empty() || parsed.ec != std::errc{} || parsed.ptr != text.data() + text.size() ||
        value > maximum || (!allow_zero && !value))
        throw std::invalid_argument("invalid unsigned decimal argument: " + text);
    return value;
}
static std::array<std::uintptr_t, 5> parse_handles(const std::string& text) {
    std::array<std::uintptr_t, 5> handles{};
    std::size_t start = 0;
    for (std::size_t i = 0; i < handles.size(); ++i) {
        const auto end = text.find(',', start);
        if ((i + 1 == handles.size()) != (end == std::string::npos))
            throw std::invalid_argument("--child needs exactly five comma-separated handles");
        handles[i] = static_cast<std::uintptr_t>(number(text.substr(start, end - start),
            std::numeric_limits<std::uintptr_t>::max() - 1));
        if (std::find(handles.begin(), handles.begin() + i, handles[i]) != handles.begin() + i)
            throw std::invalid_argument("--child handles must be distinct");
        start = end == std::string::npos ? text.size() : end + 1;
    }
    return handles;
}
Options parse_options(const std::vector<std::string>& args) {
    Options options;
    bool mode_set = false, child_extent = false;
    for (std::size_t i = 0; i < args.size(); ++i) {
        const auto& key = args[i];
        auto value = [&]() -> const std::string& {
            if (++i == args.size()) throw std::invalid_argument("missing value for " + key);
            return args[i];
        };
        if (key == "--mode") {
            if (!mode_set) { options.modes.clear(); mode_set = true; }
            const auto& mode = value();
            if (mode != "all" && mode != "same-device" && mode != "second-device" &&
                mode != "d3d11-consumer" && mode != "resize")
                throw std::invalid_argument("unknown mode: " + mode);
            if (std::find(options.modes.begin(), options.modes.end(), mode) != options.modes.end())
                throw std::invalid_argument("duplicate mode: " + mode);
            options.modes.push_back(mode);
        } else if (key == "--iterations") {
            options.iterations = number(value(), std::numeric_limits<std::uint64_t>::max() - 1);
        } else if (key == "--child") {
            if (options.handles) throw std::invalid_argument("duplicate --child");
            options.handles = parse_handles(value());
        } else if (key == "--luid") {
            options.luid = number(value(), std::numeric_limits<std::uint64_t>::max(), true);
        } else if (key == "--width" || key == "--height") {
            const auto extent = static_cast<unsigned>(number(value(), 16384));
            if (key == "--width") options.width = extent; else options.height = extent;
            child_extent = true;
        } else if (key == "--out") options.out = value();
        else if (key == "--warp") options.warp = true;
        else if (key == "--debug-layer") options.debug = true;
        else if (key == "--selftest") options.selftest = true;
        else if (key == "--inject-unconfirmed-drain") {
            options.inject_unconfirmed_drain = value();
            if (options.inject_unconfirmed_drain != "producer" && options.inject_unconfirmed_drain != "consumer")
                throw std::invalid_argument("--inject-unconfirmed-drain needs producer or consumer");
        }
        else if (key == "--help") options.help = true;
        else throw std::invalid_argument("unknown argument: " + key);
    }
    // The child is a consumer, so it accepts only a consumer injection.
    if (options.handles && (!options.luid || mode_set || options.selftest || !options.out.empty() ||
                            options.inject_unconfirmed_drain == "producer"))
        throw std::invalid_argument("--child requires --luid and cannot select modes, output, selftest or a producer injection");
    if (!options.handles && (options.luid || child_extent))
        throw std::invalid_argument("--luid, --width and --height are child-only arguments");
    if (std::find(options.modes.begin(), options.modes.end(), "all") != options.modes.end()) {
        if (options.modes.size() != 1) throw std::invalid_argument("--mode all cannot be combined");
        options.modes = {"same-device", "second-device", "d3d11-consumer", "resize"};
    }
    return options;
}

double mean(const std::vector<double>& samples) {
    return samples.empty() ? 0 : std::accumulate(samples.begin(), samples.end(), 0.0) / static_cast<double>(samples.size());
}
double p99(std::vector<double> samples) {
    if (samples.empty()) return 0;
    std::sort(samples.begin(), samples.end());
    // ceil(0.99*N), expressed with integer arithmetic (also exact for small N).
    return samples[samples.size() - samples.size() / 100 - 1];
}
std::string json_string(const std::string& value) {
    std::ostringstream out;
    out << '"';
    for (unsigned char c : value) {
        switch (c) {
        case '"': out << "\\\""; break;
        case '\\': out << "\\\\"; break;
        case '\n': out << "\\n"; break;
        case '\r': out << "\\r"; break;
        case '\t': out << "\\t"; break;
        default:
            if (c < 32) out << "\\u" << std::hex << std::setw(4) << std::setfill('0') << unsigned(c) << std::dec;
            else out << c;
        }
    }
    out << '"';
    return out.str();
}
static const char* json_bool(const std::optional<bool>& value) {
    return value ? (*value ? "true" : "false") : "null";
}
std::string result_json(const std::vector<Result>& results, bool warp, bool debug) {
    std::ostringstream out;
    out << std::setprecision(9) << "{\n  \"format\": \"magic600-sb-handoff-v1\",\n"
        << "  \"warp\": " << (warp ? "true" : "false") << ",\n  \"debug_layer\": "
        << (debug ? "true" : "false") << ",\n  \"modes\": [\n";
    for (std::size_t i = 0; i < results.size(); ++i) {
        const auto& r = results[i];
        if (i) out << ",\n";
        out << "    {\"mode\": " << json_string(r.mode) << ", \"status\": " << json_string(r.status)
            << ", \"reason\": " << json_string(r.reason) << ",\n"
            << "     \"iterations\": " << r.iterations << ", \"submitted_iterations\": " << r.submitted
            << ", \"verified_iterations\": " << r.verified << ", \"first_failing_iteration\": ";
        if (r.first_failure) out << *r.first_failure; else out << "null";
        out << ",\n     \"singleton_result\": {\"producer_same_pointer\": " << json_bool(r.singleton)
            << ", \"consumer_same_pointer\": " << json_bool(r.consumer_singleton) << "},\n"
            << "     \"adapter_luids\": {\"producer\": " << (r.producer_luid.empty() ? "null" : json_string(r.producer_luid))
            << ", \"consumer\": " << (r.consumer_luid.empty() ? "null" : json_string(r.consumer_luid)) << "},\n"
            << "     \"teardown\": {\"status\": " << json_string(r.teardown)
            << ", \"unexpected_live_objects\": " << r.live_objects
            << ", \"report_messages\": " << r.live_report_messages << "}, \"resize_count\": " << r.resize_count << ",\n"
            << "     \"timings\": {\"unit\": \"ms\", \"measurement\": \"CPU submission to CPU verification observation\","
            << " \"mean\": " << mean(r.timings) << ", \"p99_nearest_rank\": " << p99(r.timings) << ", \"per_iteration\": [";
        for (std::size_t t = 0; t < r.timings.size(); ++t) out << (t ? ", " : "") << r.timings[t];
        out << "]},\n     \"errors\": [";
        for (std::size_t e = 0; e < r.errors.size(); ++e) out << (e ? ", " : "") << json_string(r.errors[e]);
        out << "]}";
    }
    out << "\n  ]\n}\n";
    return out.str();
}
bool report_succeeded(const std::vector<Result>& results) {
    if (results.empty()) return false;
    return std::all_of(results.begin(), results.end(), [](const Result& r) {
        return (r.status == "pass" && r.verified == r.iterations) ||
               (r.status == "unsupported" && !r.reason.empty() && !r.submitted);
    });
}

void selftest() {
    auto require = [](bool ok, const char* text) {
        if (!ok) throw std::runtime_error(std::string("selftest: ") + text);
    };
    const std::vector<std::uint64_t> sequences{0, 1, 2, 3, 255, 256, 65535, 65536,
        0xffffff, 0x1000000, 0xffffffff, 0x100000000, 0x100000001, 0xfffffffffffffffe};
    for (auto n : sequences) {
        std::vector<std::uint8_t> bytes(64 * 7, 0xa5);
        fill_pattern(bytes.data(), 64, 11, 7, n);
        std::string reason;
        require(verify_pattern(bytes.data(), 64, 11, 7, n, reason), "pattern round trip");
        require(bytes[44] == 0xa5 && bytes.back() == 0xa5, "row padding preserved");
        bytes[6 * 64 + 10 * 4 + 3] ^= 1;
        require(!verify_pattern(bytes.data(), 64, 11, 7, n, reason) && reason.find("(10,6)") != std::string::npos,
                "corrupt final texel rejected");
        fill_pattern(bytes.data(), 64, 11, 7, n + 1);
        require(!verify_pattern(bytes.data(), 64, 11, 7, n, reason), "stale/swapped sequence rejected");
    }
    require(encode_sequence(0x78563412) == std::array<std::uint8_t, 4>{0x12, 0x34, 0x56, 0x78}, "RGBA byte order");
    require(encode_sequence(0x100000001) == encode_sequence(1), "32-bit pattern wraparound");
    // Simulate queue ownership and readback retention for many ring revolutions.
    std::array<std::uint64_t, 3> in_flight{};
    for (std::uint64_t n = 1; n <= 10000; ++n) {
        const auto slot = ring_slot(n);
        require(in_flight[slot] == previous_use(n), "slot cannot be overwritten before its free fence");
        if (n > 3) require(ring_slot(n - 3) == slot, "ring recurrence");
        in_flight[slot] = n;
    }
    require(ring_slot(1) == 0 && ring_slot(3) == 2 && ring_slot(4) == 0, "ring boundaries");
    require(previous_use(3) == 0 && previous_use(4) == 1, "initial fence values");
    const auto last = std::numeric_limits<std::uint64_t>::max() - 1;
    require(previous_use(last) == last - 3 && ring_slot(last) == ring_slot(last - 3), "64-bit fence boundary");
    const auto child = parse_options({"--child", "11,12,13,14,15", "--luid", "18446744073709551615", "--iterations", "7"});
    require(child.handles && (*child.handles)[4] == 15 && child.luid == std::numeric_limits<std::uint64_t>::max(), "child argument round trip");
    for (const auto& handles : {"", "1,2,3,4", "1,2,3,4,5,6", "0,2,3,4,5", "1,1,3,4,5", "-1,2,3,4,5",
                               "+1,2,3,4,5", "1x,2,3,4,5", "18446744073709551616,2,3,4,5", "1,2,3,4,5,"}) {
        bool rejected = false;
        try { parse_options({"--child", handles, "--luid", "1"}); } catch (const std::invalid_argument&) { rejected = true; }
        require(rejected, "malformed child handles rejected");
    }
    for (const auto& args : std::vector<std::vector<std::string>>{
        {"--child", "1,2,3,4,5"}, {"--mode"}, {"--iterations", "0"}, {"--iterations", "-1"},
        {"--mode", "all", "--mode", "resize"}, {"--luid", "1"}, {"--unknown"},
        {"--inject-unconfirmed-drain"}, {"--inject-unconfirmed-drain", "both"},
        {"--child", "1,2,3,4,5", "--luid", "1", "--inject-unconfirmed-drain", "producer"}}) {
        bool rejected = false;
        try { parse_options(args); } catch (const std::invalid_argument&) { rejected = true; }
        require(rejected, "malformed command line rejected");
    }
    require(parse_options({}).modes.size() == 4, "default all modes");
    require(parse_options({"--inject-unconfirmed-drain", "producer"}).inject_unconfirmed_drain == "producer", "injection role");
    require(parse_options({"--child", "1,2,3,4,5", "--luid", "1", "--inject-unconfirmed-drain", "consumer"})
            .inject_unconfirmed_drain == "consumer", "child consumer injection");
    require(p99({}) == 0 && p99({3}) == 3 && mean({1, 2, 3}) == 2, "timing empty/single/mean");
    std::vector<double> samples(100);
    std::iota(samples.begin(), samples.end(), 1.0);
    require(p99(samples) == 99, "nearest rank at 100 samples");
    samples.push_back(101);
    require(p99(samples) == 100, "nearest rank at 101 samples");
    require(json_string("a\"\\\n\r\t\x01") == "\"a\\\"\\\\\\n\\r\\t\\u0001\"", "JSON escaping");
    Result pass; pass.mode = "fixture"; pass.status = "pass"; pass.iterations = pass.submitted = pass.verified = 2;
    pass.singleton = true; pass.timings = {1, 2};
    Result fail; fail.mode = "failed fixture"; fail.reason = "\"quoted\"\nerror"; fail.first_failure = 7;
    fail.errors.push_back(fail.reason);
    Result unsupported; unsupported.mode = "unsupported fixture"; unsupported.status = "unsupported";
    unsupported.reason = "missing capability";
    const auto json = result_json({pass, fail, unsupported}, false, true);
    require(json.find("\"format\": \"magic600-sb-handoff-v1\"") != std::string::npos &&
            json.find("\"first_failing_iteration\": 7") != std::string::npos &&
            json.find("\\\"quoted\\\"\\nerror") != std::string::npos &&
            json.find("\"consumer_same_pointer\": null") != std::string::npos, "JSON result fields");
    require(report_succeeded({pass, unsupported}) && !report_succeeded({pass, fail}) && !report_succeeded({}), "exit-status policy");
    unsupported.reason.clear();
    require(!report_succeeded({unsupported}), "unsupported needs a reason");
    pass.verified = 1;
    require(!report_succeeded({pass}), "missing verification cannot pass");
}
}
