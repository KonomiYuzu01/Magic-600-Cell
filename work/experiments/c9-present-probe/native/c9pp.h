/*
 * c9pp.h - C ABI of the C9 confirmed-presentation probe (route B).
 *
 * Probe evidence for a PHS.52 design proposal only; never product or gate evidence.
 *
 * The core owns its own D3D12 device, a direct queue, a composition swap chain and a
 * DirectComposition target created with CreateTargetForHwnd(hwnd, FALSE) on the host's
 * window, beneath the host's own topmost target. A frame is confirmed only when a
 * IDXGISwapChain::GetFrameStatistics row reports its present count; poll() returns the
 * raw rows and the analyzer does the matching.
 *
 * Rules
 * - Every call that takes a context must come from the thread that called c9pp_attach
 *   (C9PP_E_WRONG_THREAD otherwise). The pure functions may run on any thread.
 * - All functions return a C9PP_* status; c9pp_last_error gives the text of the last
 *   failure (per context, or per thread when ctx is NULL).
 * - Every struct starts with struct_size, which the caller sets to sizeof(struct).
 * - Colours are 0x00RRGGBB. Rectangles are in the host window's client pixels (the
 *   window's own DPI space, which is also DirectComposition's space for that window).
 * - With a log path, every call appends one JSON object per line (JSONL) to that file;
 *   analyze.py reads it. Each row has "t" (type), "qpc", "phase" and "epoch".
 * - Never set C9PP_ATTACH_DEBUG_LAYER through D3D12GetDebugInterface in a process that
 *   already has a D3D12 device: EnableDebugLayer would remove every existing device. The
 *   flag is honoured only through a private ID3D12DeviceFactory configuration and is
 *   refused when the factory is unavailable.
 */
#ifndef C9PP_H
#define C9PP_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(C9PP_EXPORTS)
#define C9PP_API __declspec(dllexport)
#else
#define C9PP_API __declspec(dllimport)
#endif

#define C9PP_ABI_VERSION 1u

/* Status codes. */
#define C9PP_OK 0
#define C9PP_E_INVALID_ARGUMENT 1
#define C9PP_E_WRONG_STATE 2
#define C9PP_E_WIN32 3          /* an HRESULT or Win32 failure; see c9pp_last_error */
#define C9PP_E_TIMEOUT 4
#define C9PP_E_DEVICE_REMOVED 5 /* the core's own device is removed; call c9pp_recover */
#define C9PP_E_UNSUPPORTED 6
#define C9PP_E_WRONG_THREAD 7
#define C9PP_E_IO 8

/* c9pp_attach_desc.flags */
#define C9PP_ATTACH_DEBUG_LAYER 0x1u     /* debug layer on the core's private device factory only */
#define C9PP_ATTACH_FLIP_SEQUENTIAL 0x2u /* default FLIP_DISCARD */
#define C9PP_ATTACH_USE_LUID 0x4u        /* pick the adapter by luid_low/luid_high instead of adapter */
#define C9PP_ATTACH_GLOBAL_DEVICE 0x8u   /* skip the device factory; D3D12CreateDevice may return the host's singleton */
#define C9PP_ATTACH_DCOMP_DEVICE2 0x10u  /* DCompositionCreateDevice2 instead of DCompositionCreateDevice3 */
#define C9PP_ATTACH_KNOWN_FLAGS 0x1Fu

/* c9pp_recover flags */
#define C9PP_RECOVER_RECREATE_TARGET 0x1u /* release and recreate the DComp target instead of keeping it */

/* Limits checked by c9pp_validate_attach and c9pp_resize. */
#define C9PP_MIN_CELL_PX 24u
#define C9PP_MAX_CELL_PX 64u
#define C9PP_MAX_EXTENT 16384u

/* c9pp_attach_info.device_source */
#define C9PP_DEVICE_FACTORY 1 /* ID3D12DeviceFactory, DISALLOW_STORING_NEW_DEVICE_AS_SINGLETON */
#define C9PP_DEVICE_GLOBAL 2  /* D3D12CreateDevice */

/* c9pp_frame_record.wait_result */
#define C9PP_WAIT_SIGNALLED 0
#define C9PP_WAIT_TIMEOUT 1
#define C9PP_WAIT_FAILED 2

/* Corner and sample status (c9pp_decode_pair, c9pp_screen_sample). */
#define C9PP_CORNER_OK 0
#define C9PP_CORNER_AMBIGUOUS 1 /* at least one cell is neither clearly white nor clearly black */
#define C9PP_CORNER_BAD_SYNC 2
#define C9PP_CORNER_BAD_CHECK 3
#define C9PP_CORNER_OUT_OF_BOUNDS 4
#define C9PP_SAMPLE_MATCH 0       /* both corners decode the same code */
#define C9PP_SAMPLE_ONE_CORNER 1  /* exactly one corner decodes */
#define C9PP_SAMPLE_MISMATCH 2    /* both decode, to different codes */
#define C9PP_SAMPLE_UNDECODABLE 3 /* neither corner decodes */
#define C9PP_SAMPLE_CAPTURE_FAILED 4

typedef struct c9pp_attach_desc {
    uint32_t struct_size;     /* 72 */
    uint32_t flags;           /* C9PP_ATTACH_* */
    uint64_t hwnd;            /* host window */
    uint64_t adapter;         /* IDXGIAdapter* (Godot: RenderingDevice DriverResource.PhysicalDevice), or 0 with USE_LUID */
    uint64_t host_device;     /* host's ID3D12Device* (Godot: DriverResource.LogicalDevice); proves independence; required for removal */
    uint32_t luid_low;        /* with C9PP_ATTACH_USE_LUID */
    int32_t luid_high;
    int32_t x, y;             /* core rectangle in client pixels */
    uint32_t width, height;
    uint32_t cell_px;         /* frame-code cell size, C9PP_MIN_CELL_PX..C9PP_MAX_CELL_PX; 0 = 24 */
    uint32_t max_latency;     /* SetMaximumFrameLatency, 1..3; 0 = 2 */
    uint32_t wait_timeout_ms; /* bounded wait on the frame-latency object, 1..1000; 0 = 100 */
    uint32_t base_rgb;        /* background colour of the core layer */
} c9pp_attach_desc;

typedef struct c9pp_attach_info {
    uint32_t struct_size; /* 48 */
    uint32_t epoch;       /* device epoch, starts at 1 */
    int32_t device_source;      /* C9PP_DEVICE_FACTORY or C9PP_DEVICE_GLOBAL */
    int32_t device_independent; /* 1 differs from host_device, 0 same object, -1 no host_device given */
    int32_t factory_hr;         /* HRESULT of the device-factory path (0 when it was used) */
    int32_t target_hr;          /* HRESULT of CreateTargetForHwnd(hwnd, FALSE) */
    uint32_t vendor_id, device_id;
    uint32_t luid_low;
    int32_t luid_high;
    uint64_t qpc_frequency;
} c9pp_attach_info;

typedef struct c9pp_frame_record {
    uint32_t struct_size;   /* 40 */
    uint32_t frame_id;
    uint32_t epoch;
    uint32_t sync_interval;
    int32_t wait_result;    /* C9PP_WAIT_* */
    int32_t present_hr;
    uint32_t present_count; /* GetLastPresentCount right after Present */
    uint32_t wait_us;
    uint64_t qpc_present;   /* QPC right after Present returned */
} c9pp_frame_record;

typedef struct c9pp_stats_row {
    uint32_t struct_size; /* 48 */
    int32_t hr;           /* GetFrameStatistics result, e.g. DXGI_ERROR_FRAME_STATISTICS_DISJOINT */
    uint32_t epoch;
    uint32_t present_count;
    uint32_t present_refresh_count;
    uint32_t sync_refresh_count;
    uint64_t sync_qpc;
    uint64_t poll_qpc;
    uint32_t last_present_count;
    uint32_t reserved;
} c9pp_stats_row;

typedef struct c9pp_sample_desc {
    uint32_t struct_size; /* 36 */
    int32_t ui_x, ui_y;   /* a region of the host's opaque UI panel, client pixels */
    uint32_t ui_w, ui_h;
    int32_t bg_x, bg_y;   /* a region inside the core rectangle where the host is transparent */
    uint32_t bg_w, bg_h;
} c9pp_sample_desc;

typedef struct c9pp_screen_sample {
    uint32_t struct_size; /* 88 */
    int32_t status;       /* C9PP_SAMPLE_* */
    int32_t corner_status[2];
    uint32_t corner_frame[2];
    uint32_t corner_epoch[2];
    int32_t decoded;      /* 1 when status is MATCH or ONE_CORNER */
    uint32_t decoded_frame;
    uint32_t decoded_epoch;
    uint32_t ui_rgb;      /* mean colour of the UI region on screen */
    uint32_t bg_rgb;      /* mean colour of the transparent region on screen */
    int32_t foreground;   /* GetForegroundWindow() == hwnd */
    int32_t iconic;
    int32_t center_is_host; /* WindowFromPoint at the core rectangle centre belongs to hwnd */
    int32_t dpi_consistent; /* two independent client-to-physical mappings agree within 1 px */
    uint32_t reserved;
    uint64_t qpc_before;  /* QPC right before BitBlt */
    uint64_t qpc_after;
} c9pp_screen_sample;

typedef struct c9pp_decode_pair {
    uint32_t struct_size; /* 48 */
    int32_t status;       /* C9PP_SAMPLE_* (never CAPTURE_FAILED) */
    int32_t corner_status[2];
    uint32_t corner_frame[2];
    uint32_t corner_epoch[2];
    uint32_t corner_ambiguous[2]; /* number of ambiguous cells */
    uint32_t frame_id;    /* valid when status is MATCH or ONE_CORNER */
    uint32_t epoch;
} c9pp_decode_pair;

/* Version and errors. */
C9PP_API uint32_t c9pp_abi_version(void);
C9PP_API int32_t c9pp_last_error(void* ctx, char* buffer, uint32_t buffer_size);

/* Pure functions: no device, no window access beyond IsWindow. */
C9PP_API int32_t c9pp_validate_attach(const c9pp_attach_desc* desc);
C9PP_API uint32_t c9pp_crc16(const uint8_t* data, uint32_t size); /* CRC-16/CCITT-FALSE */
/* Renders the core layer image (base colour plus both code copies) into a BGRA buffer of width x height. */
C9PP_API int32_t c9pp_code_render(uint32_t frame_id, uint32_t epoch, uint32_t base_rgb, uint32_t cell_px,
                                  uint8_t* bgra, uint32_t width, uint32_t height, uint32_t stride);
/* Decodes both code copies of a width x height core rectangle whose client origin lies at
 * (origin_x, origin_y) in a BGRA buffer, with client-to-buffer scales scale_x, scale_y. */
C9PP_API int32_t c9pp_code_decode(const uint8_t* bgra, uint32_t buffer_width, uint32_t buffer_height,
                                  uint32_t stride, double origin_x, double origin_y, double scale_x,
                                  double scale_y, uint32_t width, uint32_t height, uint32_t cell_px,
                                  c9pp_decode_pair* out);

/* Lifecycle. log_path_utf8 may be NULL. On success *out_ctx is the context. */
C9PP_API int32_t c9pp_attach(const c9pp_attach_desc* desc, const char* log_path_utf8,
                             c9pp_attach_info* info, void** out_ctx);
C9PP_API int32_t c9pp_set_phase(void* ctx, const char* phase);
/* Appends {"t":"note","kind":kind,"data":json_object}; json_object must be one JSON object without newlines. */
C9PP_API int32_t c9pp_note(void* ctx, const char* kind, const char* json_object);
/* Waits (bounded) on the frame-latency object, draws the base colour and the frame code
 * (frame_id, epoch), presents with sync_interval (0..4) and records the present count. */
C9PP_API int32_t c9pp_frame(void* ctx, uint32_t frame_id, uint32_t sync_interval, c9pp_frame_record* out);
/* One IDXGISwapChain::GetFrameStatistics row. A failing GetFrameStatistics is data (row.hr), not an error. */
C9PP_API int32_t c9pp_poll(void* ctx, c9pp_stats_row* out);
C9PP_API int32_t c9pp_resize(void* ctx, int32_t x, int32_t y, uint32_t width, uint32_t height);
/* ID3D12Device5::RemoveDevice on the core's own device. Refused (C9PP_E_UNSUPPORTED) unless the
 * device is proven to be a different object from the host device. */
C9PP_API int32_t c9pp_remove_device(void* ctx);
/* GetDeviceRemovedReason of the core's own device. */
C9PP_API int32_t c9pp_device_status(void* ctx, int32_t* out_hr);
/* Recreates device, queue and swap chain after removal; epoch + 1. */
C9PP_API int32_t c9pp_recover(void* ctx, uint32_t flags, uint32_t* out_epoch);
/* Inline screen sample (BitBlt from the screen DC under a per-monitor-v2 thread context). */
C9PP_API int32_t c9pp_sample_screen(void* ctx, const c9pp_sample_desc* desc, c9pp_screen_sample* out);
/* GetDeviceRemovedReason on the host's device (Godot: DriverResource.LogicalDevice). ctx may be NULL;
 * with a context the row is also logged. */
C9PP_API int32_t c9pp_godot_device_status(void* ctx, uint64_t device, int32_t* out_hr);
C9PP_API int32_t c9pp_detach(void* ctx);

#ifdef __cplusplus
}
#endif

#endif /* C9PP_H */
