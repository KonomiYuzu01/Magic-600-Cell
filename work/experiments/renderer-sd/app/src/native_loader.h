#pragma once
#include "sa2_interop.h"
#include <windows.h>

namespace sd {
struct Native {
    HMODULE module = nullptr;
#define SD_FUNCTION(name) decltype(&name) fn_##name = nullptr;
    SD_FUNCTION(sa2_abi_version)
    SD_FUNCTION(sa2_last_error)
    SD_FUNCTION(sa2_probe)
    SD_FUNCTION(sa2_attach)
    SD_FUNCTION(sa2_create_texture)
    SD_FUNCTION(sa2_release_texture)
    SD_FUNCTION(sa2_register_slot)
    SD_FUNCTION(sa2_unregister_slot)
    SD_FUNCTION(sa2_signal_godot_free)
    SD_FUNCTION(sa2_produce)
    SD_FUNCTION(sa2_godot_wait_ready)
    SD_FUNCTION(sa2_mark_shown)
    SD_FUNCTION(sa2_verify_slot)
    SD_FUNCTION(sa2_drain)
    SD_FUNCTION(sa2_debug_counts)
    SD_FUNCTION(sa2_debug_messages)
    SD_FUNCTION(sa2_remove_device)
    SD_FUNCTION(sa2_device_removed_reason)
    SD_FUNCTION(sa2_detach)
#undef SD_FUNCTION
    void load(const wchar_t* absolutePath);
    ~Native();
};
static_assert(sizeof(sa2_device_info) == 48);
static_assert(sizeof(sa2_config) == 28);
static_assert(sizeof(struct sa2_debug_counts) == 128);
}
