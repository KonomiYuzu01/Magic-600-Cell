using System;
using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential)]
public struct Sa2DeviceInfo
{
    public uint struct_size;
    public int queue_type, queue_device_matches, adapter_matches_device;
    public int enhanced_barriers, debug_layer, max_feature_level;
    public uint node_count, vendor_id, device_id;
    public ulong umd_version;
}

[StructLayout(LayoutKind.Sequential)]
public struct Sa2Config
{
    public uint struct_size;
    public int queue_mode, barrier_api, state_before_write, state_after_write;
    public uint wait_timeout_ms;
    public int debug_callback;
}

[StructLayout(LayoutKind.Sequential)]
public unsafe struct Sa2DebugCounts
{
    public uint struct_size, distinct_id_count;
    public ulong corruption, error, warning, info, message;
    public ulong mismatching_clear_value, mentioning_sa2;
    public fixed int Ids[16];
}

[StructLayout(LayoutKind.Sequential)]
public unsafe struct Sa2SceneConfig
{
    public uint struct_size, scene;
    public double turn_ms;
    public byte* inject;
    public uint flags, trace_ms;
}

public unsafe sealed class Native : IDisposable
{
    public const uint SA2_ABI_VERSION = 2;
    public const int SA2_OK = 0;
    public const int SA2_E_INVALID_ARGUMENT = 1;
    public const int SA2_E_WRONG_STATE = 2;
    public const int SA2_E_D3D12 = 3;
    public const int SA2_E_TIMEOUT = 4;
    public const int SA2_E_DEVICE_REMOVED = 5;
    public const int SA2_E_VERIFY = 6;
    public const int SA2_E_UNSUPPORTED = 7;
    public const int SA2_E_CHECK_FAILED = 8;
    public const int SA2_E_IO = 9;
    public const uint SA2_SCENE_NO_VRAM = 1;
    public const int SA2_QUEUE_SAME = 0;
    public const int SA2_QUEUE_OWN = 1;
    public const int SA2_BARRIERS_MATCH_GODOT = 0;
    public const int SA2_BARRIERS_LEGACY = 1;
    public const int SA2_BARRIERS_ENHANCED = 2;
    public const int SA2_STATE_COMMON = 0;
    public const int SA2_STATE_RENDER_TARGET = 1;
    public const int SA2_STATE_PIXEL_SHADER_RESOURCE = 2;
    public const int SA2_STATE_ALL_SHADER_RESOURCE = 3;
    public const int SA2_STATE_COPY_SOURCE = 4;
    public const int DeviceInfoSize = 48;
    public const int ConfigSize = 28;
    public const int DebugCountsSize = 128;
    public const int SceneConfigSize = 32;

    public readonly delegate* unmanaged[Cdecl]<uint> sa2_abi_version;
    public readonly delegate* unmanaged[Cdecl]<nint, byte*, uint, int> sa2_last_error;
    public readonly delegate* unmanaged[Cdecl]<ulong, ulong, ulong, Sa2DeviceInfo*, int> sa2_probe;
    public readonly delegate* unmanaged[Cdecl]<ulong, ulong, Sa2Config*, nint*, int> sa2_attach;
    public readonly delegate* unmanaged[Cdecl]<nint, uint, uint, int, ulong*, int> sa2_create_texture;
    public readonly delegate* unmanaged[Cdecl]<nint, ulong, uint*, int> sa2_release_texture;
    public readonly delegate* unmanaged[Cdecl]<nint, uint, ulong, uint, uint, int, int> sa2_register_slot;
    public readonly delegate* unmanaged[Cdecl]<nint, uint, int> sa2_unregister_slot;
    public readonly delegate* unmanaged[Cdecl]<nint, ulong, int> sa2_signal_godot_free;
    public readonly delegate* unmanaged[Cdecl]<nint, uint, ulong, uint, uint, int> sa2_produce;
    public readonly delegate* unmanaged[Cdecl]<nint, ulong, int> sa2_godot_wait_ready;
    public readonly delegate* unmanaged[Cdecl]<nint, uint, ulong, int> sa2_mark_shown;
    public readonly delegate* unmanaged[Cdecl]<nint, uint, uint, uint, ulong*, int> sa2_verify_slot;
    public readonly delegate* unmanaged[Cdecl]<nint, uint, int> sa2_drain;
    public readonly delegate* unmanaged[Cdecl]<nint, Sa2DebugCounts*, int> sa2_debug_counts;
    public readonly delegate* unmanaged[Cdecl]<nint, byte*, uint, int> sa2_debug_messages;
    public readonly delegate* unmanaged[Cdecl]<nint, int> sa2_remove_device;
    public readonly delegate* unmanaged[Cdecl]<nint, int*, int> sa2_device_removed_reason;
    public readonly delegate* unmanaged[Cdecl]<nint, int> sa2_detach;
    public readonly delegate* unmanaged[Cdecl]<nint, Sa2SceneConfig*, int> sa2_scene_load;
    public readonly delegate* unmanaged[Cdecl]<nint, uint, ulong, int> sa2_scene_produce;
    public readonly delegate* unmanaged[Cdecl]<nint, int> sa2_scene_trace_begin;
    public readonly delegate* unmanaged[Cdecl]<nint, int> sa2_scene_trace_end;
    public readonly delegate* unmanaged[Cdecl]<nint, byte*, int> sa2_scene_write_run;
    public readonly delegate* unmanaged[Cdecl]<nint, byte*, int> sa2_scene_geometry_check;
    public readonly delegate* unmanaged[Cdecl]<byte*, uint, int> sa2_identity;
    public readonly delegate* unmanaged[Cdecl]<nint, int> sa2_scene_unload;
    private readonly nint _library;

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
    private static extern uint GetModuleFileNameW(nint module, char* path, uint size);

    public string ModulePath
    {
        get
        {
            const int capacity = 32768;
            char* path = stackalloc char[capacity];
            uint length = GetModuleFileNameW(_library, path, capacity);
            if (length == 0 || length >= capacity)
                throw new InvalidOperationException("framework-modules");
            return new string(path, 0, (int)length);
        }
    }

    public Native(string absolutePath)
    {
        if (sizeof(Sa2DeviceInfo) != DeviceInfoSize || sizeof(Sa2Config) != ConfigSize
            || sizeof(Sa2DebugCounts) != DebugCountsSize || sizeof(Sa2SceneConfig) != SceneConfigSize)
            throw new InvalidOperationException("ABI struct size mismatch");
        _library = NativeLibrary.Load(absolutePath);
        try
        {
            sa2_abi_version = (delegate* unmanaged[Cdecl]<uint>)Export("sa2_abi_version");
            if (sa2_abi_version() != SA2_ABI_VERSION)
                throw new InvalidOperationException("ABI version mismatch");
            sa2_last_error = (delegate* unmanaged[Cdecl]<nint, byte*, uint, int>)Export("sa2_last_error");
            sa2_probe = (delegate* unmanaged[Cdecl]<ulong, ulong, ulong, Sa2DeviceInfo*, int>)Export("sa2_probe");
            sa2_attach = (delegate* unmanaged[Cdecl]<ulong, ulong, Sa2Config*, nint*, int>)Export("sa2_attach");
            sa2_create_texture = (delegate* unmanaged[Cdecl]<nint, uint, uint, int, ulong*, int>)Export("sa2_create_texture");
            sa2_release_texture = (delegate* unmanaged[Cdecl]<nint, ulong, uint*, int>)Export("sa2_release_texture");
            sa2_register_slot = (delegate* unmanaged[Cdecl]<nint, uint, ulong, uint, uint, int, int>)Export("sa2_register_slot");
            sa2_unregister_slot = (delegate* unmanaged[Cdecl]<nint, uint, int>)Export("sa2_unregister_slot");
            sa2_signal_godot_free = (delegate* unmanaged[Cdecl]<nint, ulong, int>)Export("sa2_signal_godot_free");
            sa2_produce = (delegate* unmanaged[Cdecl]<nint, uint, ulong, uint, uint, int>)Export("sa2_produce");
            sa2_godot_wait_ready = (delegate* unmanaged[Cdecl]<nint, ulong, int>)Export("sa2_godot_wait_ready");
            sa2_mark_shown = (delegate* unmanaged[Cdecl]<nint, uint, ulong, int>)Export("sa2_mark_shown");
            sa2_verify_slot = (delegate* unmanaged[Cdecl]<nint, uint, uint, uint, ulong*, int>)Export("sa2_verify_slot");
            sa2_drain = (delegate* unmanaged[Cdecl]<nint, uint, int>)Export("sa2_drain");
            sa2_debug_counts = (delegate* unmanaged[Cdecl]<nint, Sa2DebugCounts*, int>)Export("sa2_debug_counts");
            sa2_debug_messages = (delegate* unmanaged[Cdecl]<nint, byte*, uint, int>)Export("sa2_debug_messages");
            sa2_remove_device = (delegate* unmanaged[Cdecl]<nint, int>)Export("sa2_remove_device");
            sa2_device_removed_reason = (delegate* unmanaged[Cdecl]<nint, int*, int>)Export("sa2_device_removed_reason");
            sa2_detach = (delegate* unmanaged[Cdecl]<nint, int>)Export("sa2_detach");
            sa2_scene_load = (delegate* unmanaged[Cdecl]<nint, Sa2SceneConfig*, int>)Export("sa2_scene_load");
            sa2_scene_produce = (delegate* unmanaged[Cdecl]<nint, uint, ulong, int>)Export("sa2_scene_produce");
            sa2_scene_trace_begin = (delegate* unmanaged[Cdecl]<nint, int>)Export("sa2_scene_trace_begin");
            sa2_scene_trace_end = (delegate* unmanaged[Cdecl]<nint, int>)Export("sa2_scene_trace_end");
            sa2_scene_write_run = (delegate* unmanaged[Cdecl]<nint, byte*, int>)Export("sa2_scene_write_run");
            sa2_scene_geometry_check = (delegate* unmanaged[Cdecl]<nint, byte*, int>)Export("sa2_scene_geometry_check");
            sa2_identity = (delegate* unmanaged[Cdecl]<byte*, uint, int>)Export("sa2_identity");
            sa2_scene_unload = (delegate* unmanaged[Cdecl]<nint, int>)Export("sa2_scene_unload");
        }
        catch { NativeLibrary.Free(_library); throw; }
    }

    private nint Export(string name) => NativeLibrary.GetExport(_library, name);
    public void Dispose() => NativeLibrary.Free(_library);
}
