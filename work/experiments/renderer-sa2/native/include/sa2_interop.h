/*
 * sa2_interop.h: the C ABI between the Godot C# smoke-test harness (project/)
 * and the native Direct3D 12 producer DLL sa2_interop.dll (native/).
 * ABI version 2; the ABI 1 declarations and layouts are retained.
 *
 * Threading: every function except sa2_abi_version is called from one thread,
 * Godot's render thread (inside RenderingServer.call_on_render_thread). The
 * info-queue callback the DLL registers may run on any thread.
 *
 * Handles: device, queue, adapter and resource arguments are the uint64 values
 * returned by RenderingDevice.get_driver_resource() on Godot's D3D12 driver:
 * ID3D12Device*, ID3D12CommandQueue*, IDXGIAdapter* and ID3D12Resource*. The
 * DLL never releases a reference it did not add.
 *
 * Return values: every int32_t function returns an SA2_* status code. After a
 * failure, sa2_last_error() gives a one-line description that names the failed
 * call and its HRESULT. Descriptions never contain pointer values or LUIDs.
 */
#ifndef SA2_INTEROP_H
#define SA2_INTEROP_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifdef SA2_INTEROP_EXPORTS
#define SA2_API __declspec(dllexport)
#else
#define SA2_API __declspec(dllimport)
#endif

#define SA2_ABI_VERSION 2u

/* Code layout; must match code_layout.json and project/CodeLayout.cs. */
#define SA2_CODE_BLOCK_PX 8
#define SA2_CODE_GRID_COLS 8
#define SA2_CODE_GRID_ROWS 8
#define SA2_CODE_SEQUENCE_BITS 32
#define SA2_CODE_SLOT_BITS 4
#define SA2_CODE_GENERATION_BITS 12
#define SA2_CODE_CHECK_BITS 16
#define SA2_MIN_TEXTURE_PX 128
#define SA2_RING_SLOTS 3

/* Status codes. */
#define SA2_OK 0
#define SA2_E_INVALID_ARGUMENT 1
#define SA2_E_WRONG_STATE 2
#define SA2_E_D3D12 3 /* a D3D12 or DXGI call failed */
#define SA2_E_TIMEOUT 4 /* a bounded CPU fence wait expired */
#define SA2_E_DEVICE_REMOVED 5
#define SA2_E_VERIFY 6 /* a slot's texels differ from the expected image */
#define SA2_E_UNSUPPORTED 7 /* e.g. enhanced barriers requested but not supported */
#define SA2_E_CHECK_FAILED 8 /* a geometry or label check failed; output is written */
#define SA2_E_IO 9 /* file read/write failure, or an output already exists */

/* Queue modes. */
#define SA2_QUEUE_SAME 0 /* record on Godot's queue (DRIVER_RESOURCE_COMMAND_QUEUE) */
#define SA2_QUEUE_OWN 1 /* record on the DLL's own direct queue on Godot's device */

/* Barrier APIs. */
#define SA2_BARRIERS_MATCH_GODOT 0 /* enhanced if OPTIONS12.EnhancedBarriersSupported, else legacy */
#define SA2_BARRIERS_LEGACY 1
#define SA2_BARRIERS_ENHANCED 2

/*
 * Slot states. Legacy barriers use the D3D12_RESOURCE_STATES value; enhanced
 * barriers use the D3D12_BARRIER_LAYOUT value.
 */
#define SA2_STATE_COMMON 0 /* COMMON / LAYOUT_COMMON */
#define SA2_STATE_RENDER_TARGET 1 /* RENDER_TARGET / LAYOUT_RENDER_TARGET */
#define SA2_STATE_PIXEL_SHADER_RESOURCE 2 /* PIXEL_SHADER_RESOURCE / LAYOUT_SHADER_RESOURCE */
#define SA2_STATE_ALL_SHADER_RESOURCE 3 /* PIXEL_ | NON_PIXEL_SHADER_RESOURCE / LAYOUT_SHADER_RESOURCE */
#define SA2_STATE_COPY_SOURCE 4 /* COPY_SOURCE / LAYOUT_COPY_SOURCE */

typedef struct sa2_context sa2_context;

typedef struct sa2_device_info {
    uint32_t struct_size; /* caller sets sizeof(sa2_device_info) */
    int32_t queue_type; /* D3D12_COMMAND_LIST_TYPE of the queue */
    int32_t queue_device_matches; /* 1 if the queue's GetDevice is the given device (IUnknown identity) */
    int32_t adapter_matches_device; /* 1 if the adapter LUID equals the device's; the LUID is never returned */
    int32_t enhanced_barriers; /* D3D12_FEATURE_DATA_D3D12_OPTIONS12.EnhancedBarriersSupported */
    int32_t debug_layer; /* 1 if the device exposes ID3D12InfoQueue1 */
    int32_t max_feature_level; /* highest supported D3D_FEATURE_LEVEL value */
    uint32_t node_count;
    uint32_t vendor_id;
    uint32_t device_id;
    uint64_t umd_version; /* IDXGIAdapter::CheckInterfaceSupport(IDXGIDevice) result; 0 if unavailable */
} sa2_device_info;

typedef struct sa2_config {
    uint32_t struct_size; /* caller sets sizeof(sa2_config) */
    int32_t queue_mode; /* SA2_QUEUE_* */
    int32_t barrier_api; /* SA2_BARRIERS_* */
    int32_t state_before_write; /* SA2_STATE_* the producer assumes a slot is in when a write starts */
    int32_t state_after_write; /* SA2_STATE_* the producer leaves a slot in: the handover state */
    uint32_t wait_timeout_ms; /* bound for the CPU waits in sa2_produce and sa2_verify_slot */
    int32_t debug_callback; /* 1: register an ID3D12InfoQueue1 message callback when the device has one */
} sa2_config;

typedef struct sa2_debug_counts {
    uint32_t struct_size; /* caller sets sizeof(sa2_debug_counts) */
    uint32_t distinct_id_count; /* entries used in ids */
    uint64_t corruption;
    uint64_t error;
    uint64_t warning;
    uint64_t info;
    uint64_t message;
    uint64_t mismatching_clear_value; /* D3D12_MESSAGE_ID_CLEARRENDERTARGETVIEW_MISMATCHINGCLEARVALUE; also counted by severity */
    uint64_t mentioning_sa2; /* WARNING, ERROR and CORRUPTION messages whose text contains "SA2" */
    int32_t ids[16]; /* first distinct WARNING, ERROR and CORRUPTION message IDs, in the order seen */
} sa2_debug_counts;

/* SA2_ABI_VERSION of the loaded DLL. */
SA2_API uint32_t sa2_abi_version(void);

/*
 * Copies the last error description, NUL-terminated and truncated to
 * buffer_size, for ctx, or the DLL's last error outside a context when ctx is
 * NULL (for example after a failed sa2_attach).
 */
SA2_API int32_t sa2_last_error(sa2_context* ctx, char* buffer, uint32_t buffer_size);

/*
 * Inspects Godot's device, queue and adapter without keeping a reference and
 * fills *out. Fails with SA2_E_INVALID_ARGUMENT if a handle is zero or does not
 * answer QueryInterface for its interface.
 */
SA2_API int32_t sa2_probe(uint64_t device, uint64_t queue, uint64_t adapter, sa2_device_info* out);

/*
 * Starts a context on Godot's device and queue. Adds one reference to each and
 * creates, all named "SA2 ...": the ready, free and drain fences, one command
 * allocator and command list per ring slot, a CPU-only RTV heap with one
 * descriptor per slot, the DLL's own direct queue when queue_mode is
 * SA2_QUEUE_OWN, and the message callback when requested and available.
 * Fails with SA2_E_UNSUPPORTED if barrier_api is SA2_BARRIERS_ENHANCED and the
 * device does not support enhanced barriers.
 */
SA2_API int32_t sa2_attach(uint64_t device, uint64_t queue, const sa2_config* config, sa2_context** out);

/*
 * Import route: creates a committed DXGI_FORMAT_R8G8B8A8_UNORM 2D texture with
 * one mip and ALLOW_RENDER_TARGET, without an optimized clear value, in
 * initial_state, named "SA2 imported texture". width and height are at least
 * SA2_MIN_TEXTURE_PX. The context holds the only reference until
 * sa2_release_texture; pass *out_resource to texture_create_from_extension.
 */
SA2_API int32_t sa2_create_texture(sa2_context* ctx, uint32_t width, uint32_t height, int32_t initial_state, uint64_t* out_resource);

/*
 * Releases a texture made by sa2_create_texture. Call it only after a
 * confirmed sa2_drain that followed the last Godot use and the free_rid of
 * every RID made from it, and after sa2_unregister_slot. *out_refcount_after
 * receives the count Release returned (0 means destroyed).
 */
SA2_API int32_t sa2_release_texture(sa2_context* ctx, uint64_t resource, uint32_t* out_refcount_after);

/*
 * Binds ring slot (0 to SA2_RING_SLOTS - 1) to a resource and creates its
 * R8G8B8A8_UNORM render-target view. godot_owned is 1 for the export route (a
 * texture Godot created, from get_driver_resource(DRIVER_RESOURCE_TEXTURE,
 * rid, 0)): the context adds a reference while the slot is registered. The
 * resource must be a 2D texture of the given size with ALLOW_RENDER_TARGET.
 * A resource already registered in another slot is refused with
 * SA2_E_WRONG_STATE, because fence history is kept per slot.
 */
SA2_API int32_t sa2_register_slot(sa2_context* ctx, uint32_t slot, uint64_t resource, uint32_t width, uint32_t height, int32_t godot_owned);

/* Unbinds a slot; releases the reference sa2_register_slot added, if any. Only after a confirmed sa2_drain. */
SA2_API int32_t sa2_unregister_slot(sa2_context* ctx, uint32_t slot);

/*
 * Signals the free fence to `frame` on Godot's queue. The harness calls it at
 * the start of frame f with frame = f - 1, after Godot submitted frame f - 1,
 * so the fence reaches f - 1 when the GPU has finished all of Godot's work up
 * to that frame.
 */
SA2_API int32_t sa2_signal_godot_free(sa2_context* ctx, uint64_t frame);

/*
 * Writes the code image for (sequence, slot, generation) into the slot and
 * signals the ready fence to `frame` on the queue that ran the write.
 * frame starts at 1 and increases with every call.
 * Steps:
 *   1. CPU wait, bounded by wait_timeout_ms, until this slot's previous command list has completed.
 *   2. SA2_QUEUE_OWN only: the own queue waits for free >= the frame last
 *      passed to sa2_mark_shown for this slot. SA2_QUEUE_SAME never waits on
 *      Godot's queue, because queue order already serialises the work.
 *   3. Record: a barrier from state_before_write to RENDER_TARGET, the fill
 *      clear, the code-block clears, and a barrier from RENDER_TARGET to state_after_write.
 *   4. Execute and signal.
 * Barriers are skipped when the two states are equal. Enhanced barriers use:
 *   - first barrier: SyncBefore NONE, AccessBefore NO_ACCESS, SyncAfter RENDER_TARGET, AccessAfter RENDER_TARGET;
 *   - second barrier: SyncBefore RENDER_TARGET, AccessBefore RENDER_TARGET, SyncAfter NONE, AccessAfter NO_ACCESS.
 */
SA2_API int32_t sa2_produce(sa2_context* ctx, uint32_t slot, uint64_t frame, uint32_t sequence, uint32_t generation);

/*
 * SA2_QUEUE_OWN: Godot's queue waits for ready >= frame, so Godot's next
 * submission starts after the write. SA2_QUEUE_SAME: does nothing and returns SA2_OK.
 */
SA2_API int32_t sa2_godot_wait_ready(sa2_context* ctx, uint64_t frame);

/* Records that Godot's frame `frame` samples or copies this slot. */
SA2_API int32_t sa2_mark_shown(sa2_context* ctx, uint32_t slot, uint64_t frame);

/*
 * Diagnostic, and it stalls: copies the slot to a readback buffer on the
 * producer's queue after its last write, waits up to wait_timeout_ms, and
 * compares every texel with the expected image. *out_mismatched_texels
 * receives the count; returns SA2_E_VERIFY when it is not 0. The copy
 * temporarily transitions the slot from state_after_write to COPY_SOURCE and back.
 */
SA2_API int32_t sa2_verify_slot(sa2_context* ctx, uint32_t slot, uint32_t sequence, uint32_t generation, uint64_t* out_mismatched_texels);

/*
 * Signals the drain fence on Godot's queue and, in SA2_QUEUE_OWN, on the own
 * queue, then waits up to timeout_ms for both. If either wait cannot be
 * confirmed (timeout, failed wait or failed signal) and the device is not
 * removed, it writes one line to stderr and ends the process with exit code 3
 * through TerminateProcess, without releasing anything. Test hook: the
 * environment variable M600_SA2_INJECT_UNCONFIRMED_DRAIN=1 makes every drain
 * take that path. Returns SA2_E_DEVICE_REMOVED if the device is removed.
 */
SA2_API int32_t sa2_drain(sa2_context* ctx, uint32_t timeout_ms);

/* Copies the message-callback counters; all zero when no callback is registered. */
SA2_API int32_t sa2_debug_counts(sa2_context* ctx, sa2_debug_counts* out);

/*
 * Copies up to eight distinct WARNING, ERROR and CORRUPTION descriptions seen
 * by the callback, one per line, with every hexadecimal number replaced by
 * "0x?", NUL-terminated and truncated to buffer_size.
 * CLEARRENDERTARGETVIEW_MISMATCHINGCLEARVALUE is excluded.
 */
SA2_API int32_t sa2_debug_messages(sa2_context* ctx, char* buffer, uint32_t buffer_size);

/* Device-loss probe: calls ID3D12Device5::RemoveDevice on Godot's device. */
SA2_API int32_t sa2_remove_device(sa2_context* ctx);

/* GetDeviceRemovedReason of Godot's device (S_OK while the device is alive). */
SA2_API int32_t sa2_device_removed_reason(sa2_context* ctx, int32_t* out_hresult);

/*
 * Ends the context after a confirmed sa2_drain. Every slot must be
 * unregistered and every sa2_create_texture texture released first (else
 * SA2_E_WRONG_STATE). ABI 2 also requires any scene to be unloaded first.
 * Then it releases what sa2_attach created, unregisters
 * the callback and releases its references to Godot's device and queue.
 */
SA2_API int32_t sa2_detach(sa2_context* ctx);

/* ABI 2 scene API. All context calls use the context's render thread.
 * Invalid arguments (including struct_size) return SA2_E_INVALID_ARGUMENT
 * before state checks. Every failure sets sa2_last_error as in ABI 1.
 */
#define SA2_SCENE_NO_VRAM 1u
typedef struct sa2_scene_config {
    uint32_t struct_size; /* sizeof(sa2_scene_config); 32 on x64 */
    uint32_t scene; /* 1..4 for S-B W1..W4, feature none, MSAA 1 */
    double turn_ms; /* W3: finite, 0 < turn_ms <= 10000; ignored otherwise */
    const char* inject; /* NULL, or corrupt-label, swap-same-colour,
                        * delay-adoption, stale-binding; W3/W4 only; copied */
    uint32_t flags; /* SA2_SCENE_NO_VRAM only; unknown bits are invalid */
    uint32_t reserved; /* must be zero */
} sa2_scene_config;
#ifdef __cplusplus
static_assert(sizeof(sa2_scene_config) == 32, "sa2_scene_config requires the x64 ABI");
#endif

/* Load assets with S-B's readers and all four baseline/check shader blobs
 * from the DLL's directory; create scene resources on the attached device.
 * Register all three equally sized slots before load; keep that size until
 * unload. Missing slots or a second load: SA2_E_WRONG_STATE; unequal sizes:
 * SA2_E_INVALID_ARGUMENT. Missing/unreadable assets or blobs: SA2_E_IO (file
 * named in last error). D3D12 failures: SA2_E_D3D12 / SA2_E_DEVICE_REMOVED.
 * Load submits no work; static uploads execute with the first scene command.
 */
SA2_API int32_t sa2_scene_load(sa2_context* ctx, const sa2_scene_config* config);

/* Draw into the registered slot with the full-size viewport, using exactly
 * sa2_produce's monotonically increasing frame (starts at 1), bounded allocator
 * wait, fail-closed free/ready checks, queue mode and handover states. Needs a
 * loaded scene and the load-time slot size (SA2_E_WRONG_STATE). Each successful
 * call during the trace appends one S-B record with a zero-based trace frame,
 * QPC read at entry, camera, turn and actual bound revision; samples process
 * local CurrentUsage on the device's adapter unless NO_VRAM. W3 runs its turn
 * clock from trace begin; W4 uses S-B's 190 ms label clock without animation.
 * Outside the trace, preroll/postroll draws do not change label revisions.
 * Returns SA2_E_TIMEOUT, SA2_E_D3D12 / SA2_E_DEVICE_REMOVED on GPU failure.
 */
SA2_API int32_t sa2_scene_produce(sa2_context* ctx, uint32_t slot, uint64_t frame);

/* Record trace_start_qpc at the call. Once per load, before trace_end;
 * missing scene or repeated/out-of-order call: SA2_E_WRONG_STATE.
 */
SA2_API int32_t sa2_scene_trace_begin(sa2_context* ctx);

/* Record trace_stop_qpc at the call. Once per load, after trace_begin;
 * missing scene or repeated/out-of-order call: SA2_E_WRONG_STATE.
 */
SA2_API int32_t sa2_scene_trace_end(sa2_context* ctx);

/* After trace_end and a live-device confirmed sa2_drain covering every scene
 * command list, run S-B's exact label check (W3/W4) and write trace.jsonl and
 * native.json into an existing UTF-8 directory. Never writes run.json or
 * overwrites either output. NULL/empty/malformed UTF-8: INVALID_ARGUMENT;
 * wrong lifecycle/drain: WRONG_STATE; file failures/existing output: E_IO.
 * Label mismatch (including an injection not reached): E_CHECK_FAILED with
 * both outputs written. Success: SA2_OK. Repeated writes need fresh outputs.
 */
SA2_API int32_t sa2_scene_write_run(sa2_context* ctx, const char* directory_utf8);

/* With a loaded scene, outside the trace window, run S-B's geometry/count
 * check on the producer queue using owned 2560x1600 offscreen targets, never
 * a slot. Wait for its own work up to config.wait_timeout_ms. Writes
 * geometry.json (S-B format) in an existing UTF-8 directory without overwrite.
 * Invalid directory argument: INVALID_ARGUMENT; wrong lifecycle: WRONG_STATE;
 * file failure: E_IO; bounded wait: E_TIMEOUT; GPU failure: E_D3D12 or
 * E_DEVICE_REMOVED; mismatch: E_CHECK_FAILED with output written; pass: SA2_OK.
 * A geometry submission invalidates the drain; drain again before teardown.
 */
SA2_API int32_t sa2_scene_geometry_check(sa2_context* ctx, const char* directory_utf8);

/* No context. Copy NUL-terminated JSON, truncated to buffer_size (as
 * sa2_debug_messages): {dll:{file,sha256},shaders:[{file,sha256},...]}, every
 * blob loaded by the DLL, sorted by file name. File names are basenames in
 * the DLL's directory; digests cover exact disk bytes. NULL buffer/zero size:
 * INVALID_ARGUMENT; unreadable files: E_IO; sets the DLL-wide last error.
 */
SA2_API int32_t sa2_identity(char* buffer, uint32_t buffer_size);

/* Release scene resources after a confirmed drain covering all scene work;
 * removal permits release without confirmation, as sa2_unregister_slot.
 * Missing scene or unconfirmed drain: WRONG_STATE. A loaded scene prevents
 * sa2_detach (WRONG_STATE); unload before detaching. Slots remain registered.
 */
SA2_API int32_t sa2_scene_unload(sa2_context* ctx);

#ifdef __cplusplus
}
#endif

#endif /* SA2_INTEROP_H */
