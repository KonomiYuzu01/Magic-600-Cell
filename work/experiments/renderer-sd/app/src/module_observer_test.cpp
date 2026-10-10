#include "module_observer.h"
#include <windows.h>
#include <algorithm>
#include <csignal>
#include <cstdlib>
#include <cwchar>
#include <iostream>
#include <iterator>
#include <stdexcept>
#include <string>

namespace {
void check(bool ok, const char* message) {
    if (!ok) throw std::runtime_error(message);
}
std::wstring executable() {
    wchar_t path[32768]{};
    const auto length = GetModuleFileNameW(nullptr, path, DWORD(std::size(path)));
    check(length && length < std::size(path), "test executable path unavailable");
    return {path, length};
}
std::wstring directory() {
    const auto path = executable();
    return path.substr(0, path.find_last_of(L"\\/") + 1);
}
std::wstring probePath() { return directory() + L"sd_module_probe.dll"; }
HMODULE loadProbe(const std::wstring& path) {
    const auto module = LoadLibraryW(path.c_str());
    check(module != nullptr, "probe load failed");
    const auto function = reinterpret_cast<int (*)(void)>(GetProcAddress(module, "sd_module_probe"));
    if (!function || function() != 600) {
        FreeLibrary(module);
        throw std::runtime_error("probe export failed");
    }
    return module;
}
bool samePath(const std::wstring& a, const std::wstring& b) {
    return CompareStringOrdinal(a.c_str(), -1, b.c_str(), -1, TRUE) == CSTR_EQUAL;
}
bool acceptable(const sd::modules::Record& record) {
    bool callbacks = false;
    for (const auto& event : record.events) {
        if (event.kind == sd::modules::Kind::Snapshot) { if (callbacks) return false; }
        else callbacks = true;
    }
    return record.sealed && record.failure == sd::modules::Failure::None && record.unloadHistory
        && record.unloadHistory->empty() && !record.overflow;
}
std::size_t unionCount(const sd::modules::Record& record, const std::wstring& path) {
    const auto paths = sd::modules::loadedPaths(record);
    return std::count_if(paths.begin(), paths.end(), [&](const auto& value) { return samePath(value, path); });
}
std::size_t eventCount(const sd::modules::Record& record, sd::modules::Kind kind, const std::wstring& path) {
    return std::count_if(record.events.begin(), record.events.end(), [&](const auto& event) {
        return event.kind == kind && samePath(event.path, path);
    });
}
bool eventOrder(const sd::modules::Record& record, const std::wstring& path, std::initializer_list<sd::modules::Kind> expected) {
    std::vector<sd::modules::Kind> kinds;
    for (const auto& event : record.events) if (samePath(event.path, path)) kinds.push_back(event.kind);
    return std::equal(kinds.begin(), kinds.end(), expected.begin(), expected.end());
}

struct BeforeMain {
    std::wstring copy;
    bool succeeded = false, deleted = false;
    BeforeMain() {
        const auto* command = GetCommandLineW();
        if (!std::wcsstr(command, L" --case 1 ")) return;
        try {
            // The parent owns this fresh GetTempFileNameW placeholder; its basename has no spaces.
            const auto* name = std::wcsrchr(command, L' ');
            check(name && !std::wcspbrk(name + 1, L"\\/:\""), "invalid copy basename");
            copy = directory() + (name + 1);
            check(CopyFileW(probePath().c_str(), copy.c_str(), FALSE) != 0, "pre-main probe copy failed");
            const auto module = loadProbe(copy);
            check(FreeLibrary(module) != 0, "pre-main probe unload failed");
            if (std::wcsstr(command, L" --case 1 delete ")) {
                check(DeleteFileW(copy.c_str()) != 0, "pre-main probe delete failed");
                deleted = true;
            }
            succeeded = true;
        } catch (const std::exception&) { succeeded = false; }
    }
};
BeforeMain beforeMain;

struct Race {
    std::wstring path;
    HANDLE raceReady = nullptr, raceGo = nullptr;
};
DWORD WINAPI racingLoad(void* context) {
    auto& race = *static_cast<Race*>(context);
    if (!SetEvent(race.raceReady) || WaitForSingleObject(race.raceGo, 10000) != WAIT_OBJECT_0) return 1;
    try { loadProbe(race.path); return 0; } // Kept until exit; a post-seal callback exits the whole child with 3.
    catch (const std::exception&) { return 1; }
}

int runCase(int number, int repetition) {
    sd::modules::start();
    const auto path = probePath();
    using sd::modules::Kind;
    switch (number) {
    case 1: {
        check(beforeMain.succeeded, "global-constructor probe did not run");
        const auto record = sd::modules::seal();
        check(record.sealed && !record.overflow && record.unloadHistory, "unload history could not be read");
        const auto name = beforeMain.copy.substr(beforeMain.copy.find_last_of(L"\\/") + 1);
        check(std::any_of(record.unloadHistory->begin(), record.unloadHistory->end(),
            [&](const auto& entry) { return samePath(entry, name); }), "pre-main unload absent from history");
        check(!acceptable(record) && unionCount(record, beforeMain.copy) == 0, "pre-main transient load was accepted");
        if (beforeMain.deleted) check(GetFileAttributesW(beforeMain.copy.c_str()) == INVALID_FILE_ATTRIBUTES, "deleted probe reappeared");
        break;
    }
    case 2: {
        auto uppercase = path;
        for (auto& c : uppercase) if (c >= L'a' && c <= L'z') c -= L'a' - L'A';
        loadProbe(uppercase); // Exercise case-insensitive scope matching, including the directory prefix.
        const auto record = sd::modules::seal();
        check(acceptable(record) && unionCount(record, path) == 1 && eventCount(record, Kind::Load, path) == 1,
            "persistent load missing from union");
        check(unionCount(record, executable()) == 1 && eventCount(record, Kind::Snapshot, executable()) == 1,
            "executable missing from startup snapshot");
        break;
    }
    case 3: {
        check(FreeLibrary(loadProbe(path)) != 0, "transient unload failed");
        const auto record = sd::modules::seal();
        check(acceptable(record) && unionCount(record, path) == 1 && eventCount(record, Kind::Load, path) == 1
            && eventCount(record, Kind::Unload, path) == 1 && eventOrder(record, path, {Kind::Load, Kind::Unload}),
            "transient load missing from union or event order");
        break;
    }
    case 4: {
        check(FreeLibrary(loadProbe(path)) != 0, "reload unload failed");
        loadProbe(path);
        const auto record = sd::modules::seal();
        check(acceptable(record) && unionCount(record, path) == 1 && eventCount(record, Kind::Load, path) == 2
            && eventCount(record, Kind::Unload, path) == 1 && eventOrder(record, path, {Kind::Load, Kind::Unload, Kind::Load}),
            "reload was lost, reordered or counted twice in union");
        break;
    }
    case 5:
        check(acceptable(sd::modules::seal()), "post-seal positive control refused");
        loadProbe(path);
        throw std::runtime_error("post-seal load survived");
    case 6: {
        Race race{path, CreateEventW(nullptr, TRUE, FALSE, nullptr), CreateEventW(nullptr, TRUE, FALSE, nullptr)};
        check(race.raceReady && race.raceGo, "race events unavailable");
        const auto thread = CreateThread(nullptr, 0, racingLoad, &race, 0, nullptr);
        check(thread && WaitForSingleObject(race.raceReady, 10000) == WAIT_OBJECT_0, "race worker unavailable");
        check(SetEvent(race.raceGo) != 0, "race start failed");
        // Delay the seal by 0 to 3.98 ms in 20 us steps, so the race lands on both sides of the load.
        LARGE_INTEGER frequency{}, start{}, now{};
        QueryPerformanceFrequency(&frequency);
        QueryPerformanceCounter(&start);
        const auto delay = frequency.QuadPart * 20 * repetition / 1000000;
        do QueryPerformanceCounter(&now); while (now.QuadPart - start.QuadPart < delay);
        const auto record = sd::modules::seal();
        check(WaitForSingleObject(thread, 10000) == WAIT_OBJECT_0, "race worker timeout");
        DWORD code = 1;
        check(GetExitCodeThread(thread, &code) && code == 0, "racing probe failed");
        CloseHandle(thread); CloseHandle(race.raceReady); CloseHandle(race.raceGo);
        check(acceptable(record) && unionCount(record, path) == 1, "racing load survived without a union entry");
        break;
    }
    case 7: {
        for (std::size_t i = 0; i <= sd::modules::SlotCount / 2; ++i)
            check(FreeLibrary(loadProbe(path)) != 0, "overflow probe unload failed");
        const auto record = sd::modules::seal();
        check(record.sealed && record.unloadHistory && record.unloadHistory->empty() && record.overflow && !acceptable(record),
            "event overflow was accepted");
        break;
    }
    case 8: {
        const auto extended = L"\\\\?\\" + path; // The loader keeps this spelling.
        loadProbe(extended);
        const auto record = sd::modules::seal();
        check(acceptable(record) && unionCount(record, extended) == 1 && eventCount(record, Kind::Load, extended) == 1,
            "extended-length load missing from union");
        break;
    }
    case 9:
        check(acceptable(sd::modules::seal()), "post-seal extended-length control refused");
        loadProbe(L"\\\\?\\" + path);
        throw std::runtime_error("post-seal extended-length load survived");
    case 10: {
        wchar_t shortDirectory[32768]{};
        const auto length = GetShortPathNameW(directory().c_str(), shortDirectory, DWORD(std::size(shortDirectory)));
        check(length && length < std::size(shortDirectory), "short path unavailable");
        const std::wstring shortPath = std::wstring(shortDirectory, length) + L"sd_module_probe.dll";
        if (samePath(shortPath, path)) return 4; // No 8.3 name on this path: not run.
        loadProbe(shortPath);
        const auto record = sd::modules::seal();
        check(acceptable(record) && unionCount(record, shortPath) + unionCount(record, path) == 1,
            "8.3 load missing from union");
        break;
    }
    default: throw std::runtime_error("unknown observer case");
    }
    return 0;
}

unsigned raceSealed = 0, notRun = 0;

// MSVC abort() exits 3, the seal's code; a test child crash must not look like it.
void abortExit(int) { _exit(2); }

bool child(int number, int repetition, const wchar_t* variant = L"keep") {
    wchar_t copy[32768]{};
    if (number == 1) check(GetTempFileNameW(directory().c_str(), L"spo", 0, copy) != 0, "fresh probe copy unavailable");
    bool passed = false;
    try {
        std::wstring command = L"\"" + executable() + L"\" --case " + std::to_wstring(number) + L" ";
        if (number == 1) command += std::wstring(variant) + L" " + (std::wcsrchr(copy, L'\\') + 1);
        else command += std::to_wstring(repetition);
        STARTUPINFOW startup{}; startup.cb = sizeof(startup);
        PROCESS_INFORMATION process{};
        if (CreateProcessW(executable().c_str(), command.data(), nullptr, nullptr, FALSE, CREATE_NO_WINDOW,
                nullptr, directory().c_str(), &startup, &process)) {
            const auto wait = WaitForSingleObject(process.hProcess, 30000);
            if (wait != WAIT_OBJECT_0) {
                TerminateProcess(process.hProcess, 1);
                WaitForSingleObject(process.hProcess, 5000);
            }
            DWORD code = 1;
            if (wait == WAIT_OBJECT_0 && GetExitCodeProcess(process.hProcess, &code))
                passed = number == 5 || number == 9 ? code == 3 : number == 6 ? code == 0 || code == 3
                    : number == 10 ? code == 0 || code == 4 : code == 0;
            raceSealed += number == 6 && passed && code == 3;
            notRun += number == 10 && passed && code == 4;
            CloseHandle(process.hThread); CloseHandle(process.hProcess);
        }
    } catch (...) {
        if (number == 1) DeleteFileW(copy);
        throw;
    }
    if (number == 1 && GetFileAttributesW(copy) != INVALID_FILE_ATTRIBUTES)
        passed = DeleteFileW(copy) != 0 && passed;
    return passed;
}
}

int wmain(int argc, wchar_t** argv) {
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    try {
        if (argc >= 3 && std::wcscmp(argv[1], L"--case") == 0) {
            _set_abort_behavior(0, _WRITE_ABORT_MSG | _CALL_REPORTFAULT);
            std::signal(SIGABRT, abortExit);
            return runCase(std::wcstol(argv[2], nullptr, 10), argc > 3 ? std::wcstol(argv[3], nullptr, 10) : 0);
        }
        check(argc == 1, "test takes no arguments except its internal --case");
        unsigned cases = 0, children = 0;
        for (int number = 1; number <= 10; ++number) {
            const int repetitions = number == 6 ? 200 : number == 1 ? 2 : 1;
            bool passed = true;
            for (int repetition = 0; repetition < repetitions; ++repetition) {
                passed = child(number, repetition, repetition ? L"delete" : L"keep") && passed;
                ++children;
            }
            cases += passed;
            std::cout << "case " << number << ": " << (passed ? "pass" : "FAIL") << " (" << repetitions << " children";
            if (number == 6) std::cout << "; " << raceSealed << " exited 3 at the seal";
            if (number == 10 && notRun) std::cout << "; not run: no 8.3 name on this path";
            std::cout << ")\n";
        }
        std::cout << "sd_module_observer_test: " << cases << "/10 cases, " << children << " children\n";
        return cases == 10 ? 0 : 1;
    } catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
}
