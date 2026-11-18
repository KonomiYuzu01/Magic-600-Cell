using System;
using System.Runtime.InteropServices;
using System.Text;

// Mirrors native/c9pp.h (ABI version 1). Sizes are checked against the header's comments.
[StructLayout(LayoutKind.Sequential)]
public struct C9AttachDesc
{
    public uint struct_size, flags;
    public ulong hwnd, adapter, host_device;
    public uint luid_low;
    public int luid_high;
    public int x, y;
    public uint width, height, cell_px, max_latency, wait_timeout_ms, base_rgb;
}

[StructLayout(LayoutKind.Sequential)]
public struct C9AttachInfo
{
    public uint struct_size, epoch;
    public int device_source, device_independent, factory_hr, target_hr;
    public uint vendor_id, device_id, luid_low;
    public int luid_high;
    public ulong qpc_frequency;
}

[StructLayout(LayoutKind.Sequential)]
public struct C9FrameRecord
{
    public uint struct_size, frame_id, epoch, sync_interval;
    public int wait_result, present_hr;
    public uint present_count, wait_us;
    public ulong qpc_present;
}

[StructLayout(LayoutKind.Sequential)]
public struct C9StatsRow
{
    public uint struct_size;
    public int hr;
    public uint epoch, present_count, present_refresh_count, sync_refresh_count;
    public ulong sync_qpc, poll_qpc;
    public uint last_present_count, reserved;
}

[StructLayout(LayoutKind.Sequential)]
public struct C9SampleDesc
{
    public uint struct_size;
    public int ui_x, ui_y;
    public uint ui_w, ui_h;
    public int bg_x, bg_y;
    public uint bg_w, bg_h;
}

[StructLayout(LayoutKind.Sequential)]
public unsafe struct C9ScreenSample
{
    public uint struct_size;
    public int status;
    public fixed int corner_status[2];
    public fixed uint corner_frame[2];
    public fixed uint corner_epoch[2];
    public int decoded;
    public uint decoded_frame, decoded_epoch, ui_rgb, bg_rgb;
    public int foreground, iconic, center_is_host, dpi_consistent;
    public uint reserved;
    public ulong qpc_before, qpc_after;
}

public unsafe sealed class Native : IDisposable
{
    public const uint ABI_VERSION = 1;
    public const int OK = 0, E_INVALID_ARGUMENT = 1, E_WRONG_STATE = 2, E_WIN32 = 3, E_TIMEOUT = 4,
        E_DEVICE_REMOVED = 5, E_UNSUPPORTED = 6, E_WRONG_THREAD = 7, E_IO = 8;
    public const uint ATTACH_FLIP_SEQUENTIAL = 0x2, RECOVER_RECREATE_TARGET = 0x1;

    public readonly delegate* unmanaged[Cdecl]<uint> abi_version;
    public readonly delegate* unmanaged[Cdecl]<nint, byte*, uint, int> last_error;
    public readonly delegate* unmanaged[Cdecl]<C9AttachDesc*, byte*, C9AttachInfo*, nint*, int> attach;
    public readonly delegate* unmanaged[Cdecl]<nint, byte*, int> set_phase;
    public readonly delegate* unmanaged[Cdecl]<nint, byte*, byte*, int> note;
    public readonly delegate* unmanaged[Cdecl]<nint, uint, uint, C9FrameRecord*, int> frame;
    public readonly delegate* unmanaged[Cdecl]<nint, C9StatsRow*, int> poll;
    public readonly delegate* unmanaged[Cdecl]<nint, int, int, uint, uint, int> resize;
    public readonly delegate* unmanaged[Cdecl]<nint, int> remove_device;
    public readonly delegate* unmanaged[Cdecl]<nint, int*, int> device_status;
    public readonly delegate* unmanaged[Cdecl]<nint, uint, uint*, int> recover;
    public readonly delegate* unmanaged[Cdecl]<nint, C9SampleDesc*, C9ScreenSample*, int> sample_screen;
    public readonly delegate* unmanaged[Cdecl]<nint, ulong, int*, int> godot_device_status;
    public readonly delegate* unmanaged[Cdecl]<nint, int> detach;
    private readonly nint _library;

    public Native(string absolutePath)
    {
        if (sizeof(C9AttachDesc) != 72 || sizeof(C9AttachInfo) != 48 || sizeof(C9FrameRecord) != 40
            || sizeof(C9StatsRow) != 48 || sizeof(C9SampleDesc) != 36 || sizeof(C9ScreenSample) != 88)
            throw new InvalidOperationException("ABI struct size mismatch");
        _library = NativeLibrary.Load(absolutePath);
        try
        {
            abi_version = (delegate* unmanaged[Cdecl]<uint>)Export("c9pp_abi_version");
            if (abi_version() != ABI_VERSION) throw new InvalidOperationException("ABI version mismatch");
            last_error = (delegate* unmanaged[Cdecl]<nint, byte*, uint, int>)Export("c9pp_last_error");
            attach = (delegate* unmanaged[Cdecl]<C9AttachDesc*, byte*, C9AttachInfo*, nint*, int>)Export("c9pp_attach");
            set_phase = (delegate* unmanaged[Cdecl]<nint, byte*, int>)Export("c9pp_set_phase");
            note = (delegate* unmanaged[Cdecl]<nint, byte*, byte*, int>)Export("c9pp_note");
            frame = (delegate* unmanaged[Cdecl]<nint, uint, uint, C9FrameRecord*, int>)Export("c9pp_frame");
            poll = (delegate* unmanaged[Cdecl]<nint, C9StatsRow*, int>)Export("c9pp_poll");
            resize = (delegate* unmanaged[Cdecl]<nint, int, int, uint, uint, int>)Export("c9pp_resize");
            remove_device = (delegate* unmanaged[Cdecl]<nint, int>)Export("c9pp_remove_device");
            device_status = (delegate* unmanaged[Cdecl]<nint, int*, int>)Export("c9pp_device_status");
            recover = (delegate* unmanaged[Cdecl]<nint, uint, uint*, int>)Export("c9pp_recover");
            sample_screen = (delegate* unmanaged[Cdecl]<nint, C9SampleDesc*, C9ScreenSample*, int>)Export("c9pp_sample_screen");
            godot_device_status = (delegate* unmanaged[Cdecl]<nint, ulong, int*, int>)Export("c9pp_godot_device_status");
            detach = (delegate* unmanaged[Cdecl]<nint, int>)Export("c9pp_detach");
        }
        catch { NativeLibrary.Free(_library); throw; }
    }

    private nint Export(string name) => NativeLibrary.GetExport(_library, name);

    public static byte[] Utf8Z(string value)
    {
        byte[] bytes = new byte[Encoding.UTF8.GetByteCount(value) + 1];
        Encoding.UTF8.GetBytes(value, 0, value.Length, bytes, 0);
        return bytes;
    }

    public string LastError(nint ctx)
    {
        byte* buffer = stackalloc byte[1024];
        buffer[0] = 0;
        last_error(ctx, buffer, 1024);
        return Marshal.PtrToStringUTF8((nint)buffer) ?? "";
    }

    public int SetPhase(nint ctx, string phase)
    {
        fixed (byte* p = Utf8Z(phase)) return set_phase(ctx, p);
    }

    public int Note(nint ctx, string kind, string jsonObject)
    {
        fixed (byte* k = Utf8Z(kind))
        fixed (byte* j = Utf8Z(jsonObject))
            return note(ctx, k, j);
    }

    public int Resize(nint ctx, int x, int y, uint width, uint height) => resize(ctx, x, y, width, height);
    public int RemoveDevice(nint ctx) => remove_device(ctx);
    public int Detach(nint ctx) => detach(ctx);

    public void Dispose() => NativeLibrary.Free(_library);
}
