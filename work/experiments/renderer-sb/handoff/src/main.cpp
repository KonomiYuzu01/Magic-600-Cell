#include "handoff.h"

#include <windows.h>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>

static void write_output(const sb::Options& options, const std::string& json) {
    if (!options.out.empty()) {
        std::ofstream out(std::filesystem::path(std::u8string(options.out.begin(), options.out.end())), std::ios::binary);
        out << json;
        out.close();
        if (!out) throw std::runtime_error("cannot write output JSON");
    } else if (!options.selftest) std::cout << json;
}
static std::string utf8(const wchar_t* text) {
    const int size = WideCharToMultiByte(CP_UTF8, 0, text, -1, nullptr, 0, nullptr, nullptr);
    if (!size) throw std::runtime_error("cannot encode command line");
    std::string result(static_cast<std::size_t>(size), '\0');
    WideCharToMultiByte(CP_UTF8, 0, text, -1, result.data(), size, nullptr, nullptr);
    result.pop_back();
    return result;
}
int wmain(int argc, wchar_t** argv) {
    try {
        std::vector<std::string> args;
        for (int i = 1; i < argc; ++i) args.push_back(utf8(argv[i]));
        const auto options = sb::parse_options(args);
        if (options.help) {
            std::cout << "sb_handoff [--mode all|same-device|second-device|d3d11-consumer|resize]\n"
                         "           [--iterations N] [--warp] [--debug-layer] [--out file.json]\n"
                         "           [--inject-unconfirmed-drain producer|consumer] (test: that role's drains fail)\n"
                         "sb_handoff --selftest [--out fixture.json] (CPU only)\n";
            return 0;
        }
        if (options.handles) return sb::run_child(options);
        std::vector<sb::Result> results;
        if (options.selftest) {
            sb::selftest();
            sb::Result fixture;
            fixture.mode = "selftest"; fixture.status = "pass";
            fixture.reason = "CPU fixtures only; no device created";
            fixture.iterations = fixture.submitted = fixture.verified = 1;
            results.push_back(fixture);
            std::cout << "selftest: ok\n";
        } else {
            // An unconfirmed drain ends the process; report everything up to it first.
            sb::set_abandon_report([&](const sb::Result& failed) {
                std::cout << failed.mode << ": " << failed.status << " (" << failed.verified << '/'
                          << failed.iterations << ") " << failed.reason << '\n';
                auto partial = results;
                partial.push_back(failed);
                write_output(options, sb::result_json(partial, options.warp, options.debug));
                std::cout.flush();
            });
            for (const auto& mode : options.modes) {
                results.push_back(sb::run_mode(options, mode));
                const auto& r = results.back();
                std::cout << mode << ": " << r.status << " (" << r.verified << '/' << r.iterations << ") " << r.reason << '\n';
            }
        }
        write_output(options, sb::result_json(results, options.warp, options.debug));
        return sb::report_succeeded(results) ? 0 : 1;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 2;
    }
}
