#include "native_loader.h"
#include <stdexcept>

namespace sd {
void Native::load(const wchar_t* absolutePath) {
    module = LoadLibraryW(absolutePath);
    if (!module) throw std::runtime_error("LoadLibraryW failed");
#define SD_BIND(name) \
    fn_##name = reinterpret_cast<decltype(&name)>(GetProcAddress(module, #name)); \
    if (!fn_##name) throw std::runtime_error("missing DLL export " #name);
    SD_BIND(sa2_abi_version)
    // ABI and struct sizes are checked before binding/calling the rest of the ABI.
    if (fn_sa2_abi_version() != SA2_ABI_VERSION || sizeof(sa2_device_info) != 48
        || sizeof(sa2_config) != 28 || sizeof(struct sa2_debug_counts) != 128 || sizeof(sa2_scene_config) != 32)
        throw std::runtime_error("SA2 ABI mismatch");
    SD_BIND(sa2_last_error)
    SD_BIND(sa2_probe)
    SD_BIND(sa2_attach)
    SD_BIND(sa2_create_texture)
    SD_BIND(sa2_release_texture)
    SD_BIND(sa2_register_slot)
    SD_BIND(sa2_unregister_slot)
    SD_BIND(sa2_signal_godot_free)
    SD_BIND(sa2_produce)
    SD_BIND(sa2_godot_wait_ready)
    SD_BIND(sa2_mark_shown)
    SD_BIND(sa2_verify_slot)
    SD_BIND(sa2_drain)
    SD_BIND(sa2_debug_counts)
    SD_BIND(sa2_debug_messages)
    SD_BIND(sa2_remove_device)
    SD_BIND(sa2_device_removed_reason)
    SD_BIND(sa2_detach)
    SD_BIND(sa2_scene_load)
    SD_BIND(sa2_scene_produce)
    SD_BIND(sa2_scene_trace_begin)
    SD_BIND(sa2_scene_trace_end)
    SD_BIND(sa2_scene_write_run)
    SD_BIND(sa2_scene_geometry_check)
    SD_BIND(sa2_identity)
    SD_BIND(sa2_scene_unload)
#undef SD_BIND
}
Native::~Native() { if (module) FreeLibrary(module); }
}
