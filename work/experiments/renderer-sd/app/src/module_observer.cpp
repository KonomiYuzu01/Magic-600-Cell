#include "module_observer.h"
#include <windows.h>
#include <psapi.h>
#include <algorithm>
#include <atomic>
#include <iterator>

namespace sd::modules {
namespace {
constexpr ULONG loaded = 1, unloaded = 2;
struct CountedName {
    USHORT Length, MaximumLength;
    wchar_t* Buffer;
};
struct NotificationEntry {
    ULONG Flags;
    const CountedName* FullDllName;
    const CountedName* BaseDllName;
    void* DllBase;
    ULONG SizeOfImage;
};
union NotificationData {
    NotificationEntry Loaded, Unloaded;
};
using RegisterNotification = LONG (NTAPI*)(ULONG, void (CALLBACK*)(ULONG, const NotificationData*, void*), void*, void**);
using GetUnloadTrace = void (NTAPI*)(ULONG**, ULONG**, void**);
using Upcase = wchar_t (NTAPI*)(wchar_t);
using LockLoader = LONG (NTAPI*)(ULONG, ULONG*, ULONG_PTR*);
using UnlockLoader = LONG (NTAPI*)(ULONG, ULONG_PTR);
struct UnloadEntry {
    void* BaseAddress;
    SIZE_T SizeOfImage;
    ULONG Sequence, TimeDateStamp, CheckSum;
    wchar_t ImageName[32];
};
struct HistoryEntry {
    ULONG sequence = 0;
    wchar_t name[32]{};
};
struct Slot {
    std::atomic<bool> complete{false};
    Kind kind = Kind::Ignored;
    std::size_t length = 0;
    wchar_t path[PathCapacity]{};
};
struct State {
    // Only lock-free atomics and fixed storage are accessed under the loader lock.
    std::atomic<std::size_t> count{0};
    std::atomic<bool> sealed{false}, overflow{false}, overflowComplete{false};
    Failure failure = Failure::None;
    Upcase upcase = nullptr;
    void* cookie = nullptr;
    wchar_t scope[32768]{};
    std::size_t scopeLength = 0;
    Slot slots[SlotCount];
    Slot snapshot[SlotCount];
    std::size_t snapshotCount = 0, historyCount = 0;
    HistoryEntry history[SlotCount];
    bool historyReadable = false;
};
static_assert(std::atomic<std::size_t>::is_always_lock_free && std::atomic<bool>::is_always_lock_free);
// No destructor or dynamically owned buffer: callbacks remain valid during CRT/loader shutdown.
constinit State state;
void fail(Failure failure) { if (state.failure == Failure::None) state.failure = failure; }
wchar_t backslash(wchar_t value) noexcept { return value == L'/' ? L'\\' : value; }
bool inScope(const CountedName& name) noexcept {
    const auto length = name.Length / sizeof(wchar_t);
    if (length <= state.scopeLength) return false;
    for (std::size_t i = 0; i < state.scopeLength; ++i)
        if (state.upcase(backslash(name.Buffer[i])) != state.upcase(state.scope[i])) return false;
    return true; // scope includes its final backslash, so sibling directory prefixes never match.
}
void CALLBACK notify(ULONG reason, const NotificationData* data, void*) noexcept {
    const auto* name = data ? (reason == loaded ? data->Loaded.FullDllName : data->Unloaded.FullDllName) : nullptr;
    const bool valid = (reason == loaded || reason == unloaded) && name && name->Buffer && name->Length
        && name->Length % sizeof(wchar_t) == 0 && name->Length <= name->MaximumLength;
    if (valid && !inScope(*name)) return;
    const auto index = state.count.fetch_add(1, std::memory_order_seq_cst);
    const bool sealed = state.sealed.load(std::memory_order_seq_cst);
    if ((reason == loaded && sealed) || (!valid && sealed)) {
        TerminateProcess(GetCurrentProcess(), 3);
        return;
    }
    // A reserved unload may fall inside the sealer's count. Complete it even when it is ignored.
    if (valid && reason == unloaded && sealed) {
        if (index < SlotCount) state.slots[index].complete.store(true, std::memory_order_seq_cst);
        else state.overflowComplete.store(true, std::memory_order_seq_cst);
        return;
    }
    if (index >= SlotCount) {
        state.overflow.store(true, std::memory_order_seq_cst);
        state.overflowComplete.store(true, std::memory_order_seq_cst);
        return;
    }
    auto& slot = state.slots[index];
    const auto length = valid ? name->Length / sizeof(wchar_t) : 0;
    if (!valid || length >= PathCapacity) state.overflow.store(true, std::memory_order_seq_cst);
    else {
        slot.kind = reason == loaded ? Kind::Load : Kind::Unload;
        slot.length = length;
        for (std::size_t i = 0; i < length; ++i) slot.path[i] = name->Buffer[i];
        slot.path[length] = L'\0';
    }
    slot.complete.store(true, std::memory_order_seq_cst);
}
bool readUnloadHistory(GetUnloadTrace getUnloadTrace) noexcept {
    if (!getUnloadTrace) return false;
    // Keep SEH separate from functions that own C++ objects. Changed or unreadable NT data fails closed.
    __try {
        ULONG* elementSize = nullptr;
        ULONG* elementCount = nullptr;
        void* traceAddress = nullptr;
        getUnloadTrace(&elementSize, &elementCount, &traceAddress);
        if (!elementSize || !elementCount || !traceAddress || *elementSize < sizeof(UnloadEntry)
            || *elementSize > 4096 || !*elementCount || *elementCount > SlotCount) return false;
        // RtlGetUnloadEventTraceEx exposes the address of the trace-pointer variable, not the array.
        const auto* trace = *static_cast<const unsigned char* const*>(traceAddress);
        if (trace) {
            for (ULONG i = 0; i < *elementCount; ++i) {
                const auto* entry = reinterpret_cast<const UnloadEntry*>(trace + std::size_t(i) * *elementSize);
                if (!entry->BaseAddress) continue;
                auto& saved = state.history[state.historyCount++];
                saved.sequence = entry->Sequence;
                for (std::size_t c = 0; c < 32; ++c) saved.name[c] = entry->ImageName[c];
            }
        } // An unallocated trace is an empty history.
        state.historyReadable = true;
        return true;
    } __except (EXCEPTION_EXECUTE_HANDLER) { return false; }
}
bool takeSnapshot() noexcept {
    HMODULE handles[SlotCount]{};
    DWORD required = 0;
    if (!EnumProcessModules(GetCurrentProcess(), handles, DWORD(sizeof(handles)), &required)
        || !required || required > sizeof(handles) || required % sizeof(HMODULE)) return false;
    for (std::size_t i = 0; i < required / sizeof(HMODULE); ++i) {
        wchar_t path[32768]{};
        const auto length = GetModuleFileNameW(handles[i], path, DWORD(std::size(path)));
        if (!length || length >= std::size(path)) return false;
        // UNICODE_STRING's byte count fits any path accepted above (without its terminator).
        const CountedName name{USHORT(length * sizeof(wchar_t)), USHORT(length * sizeof(wchar_t)), path};
        if (!inScope(name)) continue;
        if (length >= PathCapacity) { state.overflow.store(true, std::memory_order_seq_cst); continue; }
        auto& slot = state.snapshot[state.snapshotCount++];
        slot.kind = Kind::Snapshot;
        slot.length = length;
        for (std::size_t c = 0; c <= length; ++c) slot.path[c] = path[c];
    }
    return true;
}
}

void start() noexcept {
    const auto ntdll = GetModuleHandleW(L"ntdll.dll");
    const auto registerNotification = reinterpret_cast<RegisterNotification>(GetProcAddress(ntdll, "LdrRegisterDllNotification"));
    const auto unregisterNotification = GetProcAddress(ntdll, "LdrUnregisterDllNotification");
    const auto getUnloadTrace = reinterpret_cast<GetUnloadTrace>(GetProcAddress(ntdll, "RtlGetUnloadEventTraceEx"));
    const auto lockLoader = reinterpret_cast<LockLoader>(GetProcAddress(ntdll, "LdrLockLoaderLock"));
    const auto unlockLoader = reinterpret_cast<UnlockLoader>(GetProcAddress(ntdll, "LdrUnlockLoaderLock"));
    state.upcase = reinterpret_cast<Upcase>(GetProcAddress(ntdll, "RtlUpcaseUnicodeChar"));
    const auto length = GetModuleFileNameW(nullptr, state.scope, DWORD(std::size(state.scope)));
    if (!registerNotification || !unregisterNotification || !lockLoader || !unlockLoader || !state.upcase
        || !length || length >= std::size(state.scope)) {
        fail(Failure::Register); return;
    }
    for (DWORD i = 0; i < length; ++i) {
        state.scope[i] = backslash(state.scope[i]);
        if (state.scope[i] == L'\\') state.scopeLength = i + 1;
    }
    if (!state.scopeLength) { fail(Failure::Register); return; }
    state.scope[state.scopeLength] = L'\0';
    // Register outside the loader lock, so the lock order is never loader lock then notification lock.
    // A load or unload between registration and the lock is an event, and an unload also enters the history.
    if (registerNotification(0, notify, nullptr, &state.cookie) < 0 || !state.cookie) {
        fail(Failure::Register); return;
    }
    // Without this lock a pre-main import could unload after the history read but before the snapshot.
    ULONG disposition = 0;
    ULONG_PTR loaderCookie = 0;
    if (lockLoader(0, &disposition, &loaderCookie) < 0 || disposition != 1 || !loaderCookie) {
        fail(Failure::Register); return;
    }
    if (!readUnloadHistory(getUnloadTrace)) fail(Failure::UnloadHistory);
    if (!takeSnapshot()) fail(Failure::Snapshot);
    if (unlockLoader(0, loaderCookie) < 0) fail(Failure::Snapshot);
}

Record seal() {
    state.sealed.store(true, std::memory_order_seq_cst);
    const auto count = state.count.load(std::memory_order_seq_cst);
    const auto began = GetTickCount64();
    if (!state.cookie) fail(Failure::Register);
    const auto bounded = std::min(count, SlotCount);
    for (std::size_t i = 0; i < bounded; ++i) {
        const auto& slot = state.slots[i];
        while (!slot.complete.load(std::memory_order_seq_cst)) {
            if (GetTickCount64() - began >= 5000) { fail(Failure::Seal); break; }
            SwitchToThread();
        }
        if (state.failure == Failure::Seal) break;
    }
    // The loader lock serializes callbacks. Completion of the first excess reservation
    // publishes either genuine overflow or an ignored post-seal unload, without a false refusal.
    while (count > SlotCount && !state.overflowComplete.load(std::memory_order_seq_cst)) {
        if (GetTickCount64() - began >= 5000) { fail(Failure::Seal); break; }
        SwitchToThread();
    }
    Record result;
    result.failure = state.failure;
    result.sealed = result.failure == Failure::None;
    const auto scopeLength = state.scopeLength > 3 ? state.scopeLength - 1 : state.scopeLength;
    result.scope.assign(state.scope, scopeLength);
    if (state.historyReadable) {
        result.unloadHistory.emplace();
        // Sequence restores chronological trace order when the loader's ring has wrapped.
        std::sort(state.history, state.history + state.historyCount, [](const auto& a, const auto& b) { return a.sequence < b.sequence; });
        for (std::size_t i = 0; i < state.historyCount; ++i) {
            const auto& name = state.history[i].name;
            const auto length = std::find(name, name + 32, L'\0') - name;
            result.unloadHistory->emplace_back(name, length);
        }
    }
    result.overflow = state.overflow.load(std::memory_order_seq_cst);
    // Unloads after sealing reserve slots but are ignored. They cannot invalidate the loaded union.
    for (std::size_t i = 0; i < state.snapshotCount; ++i) {
        const auto& slot = state.snapshot[i];
        result.events.push_back({Kind::Snapshot, {slot.path, slot.length}});
    }
    for (std::size_t i = 0; i < bounded; ++i) {
        const auto& slot = state.slots[i];
        if (slot.complete.load(std::memory_order_seq_cst) && slot.kind != Kind::Ignored)
            result.events.push_back({slot.kind, {slot.path, slot.length}});
    }
    return result;
}

std::vector<std::wstring> loadedPaths(const Record& record) {
    std::vector<std::wstring> paths;
    for (const auto& event : record.events) {
        if (event.kind == Kind::Snapshot || event.kind == Kind::Load) {
            if (std::none_of(paths.begin(), paths.end(), [&](const auto& path) {
                return CompareStringOrdinal(path.c_str(), -1, event.path.c_str(), -1, TRUE) == CSTR_EQUAL;
            })) paths.push_back(event.path);
        }
    }
    return paths;
}
}
