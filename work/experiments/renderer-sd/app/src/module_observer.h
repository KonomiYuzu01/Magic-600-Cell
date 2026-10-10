#pragma once
#include <cstddef>
#include <optional>
#include <string>
#include <vector>

namespace sd::modules {
inline constexpr std::size_t SlotCount = 4096, PathCapacity = 2048;
enum class Kind { Snapshot, Load, Unload, Ignored };
enum class Failure { None, Register, UnloadHistory, Snapshot, Seal };
struct Event {
    Kind kind;
    std::wstring path;
};
struct Record {
    bool sealed = false;
    Failure failure = Failure::None;
    std::wstring scope;
    std::optional<std::vector<std::wstring>> unloadHistory;
    bool overflow = false;
    std::vector<Event> events;
};
// Process lifetime storage: registration is deliberately never released, even after sealing.
void start() noexcept;
Record seal();
std::vector<std::wstring> loadedPaths(const Record& record);
}
