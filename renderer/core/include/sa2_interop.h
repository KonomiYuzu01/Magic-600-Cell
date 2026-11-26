/*
 * sa2_interop.h: the C ABI between the Godot C# host and the native Direct3D
 * 12 producer DLL sa2_interop.dll (renderer/core/, the product copy of the
 * selected S-A2 core).
 * ABI version 8; the ABI 1 to ABI 7 declarations, layouts and behaviour are
 * retained, except that ABI 5 extends sa2_presented_row with display_qpc (no
 * ABI 4 core wrote that row: without a surface sa2_presented answers
 * SA2_E_UNAVAILABLE). ABI 3 adds W-J's per-piece pose path (sa2_scene_load_wj and
 * sa2_scene_lattice_check). ABI 4 adds the product API, one call per RM.14
 * operation. ABI 5 adds route B: the core's own device, a DirectComposition
 * layer and a composition swap chain (both at the end of this file). ABI 6
 * adds a view's shape (sa2_view_shape_set), ABI 7 per-piece style
 * (sa2_style_set, sa2_view_style_set) and ABI 8 the composite frame
 * (sa2_frame_submit_panes, sa2_frame_panes), all at the end of this file.
 *
 * The integrator owns this file. An implementation must not change it; if a
 * change is unavoidable, stop and report the reason instead.
 *
 * Threading: on a sa2_attach context every function except sa2_abi_version is
 * called from one thread, Godot's render thread (inside
 * RenderingServer.call_on_render_thread). A sa2_create context (ABI 5) takes
 * no lock: its calls may come from any thread, one at a time, serialised by
 * the caller. The info-queue callback the DLL registers may run on any thread.
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

#define SA2_ABI_VERSION 8u

/* Code layout; must match code_layout.json and the host's CodeLayout.cs. */
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

/* Device-loss probe: calls ID3D12Device5::RemoveDevice on Godot's device; on
 * a sa2_create context (ABI 5), on the core's own device only. */
SA2_API int32_t sa2_remove_device(sa2_context* ctx);

/* GetDeviceRemovedReason of Godot's device (S_OK while the device is alive). */
SA2_API int32_t sa2_device_removed_reason(sa2_context* ctx, int32_t* out_hresult);

/*
 * Ends the context after a confirmed sa2_drain. Every slot must be
 * unregistered and every sa2_create_texture texture released first (else
 * SA2_E_WRONG_STATE). ABI 2 also requires any scene to be unloaded first.
 * Then it releases what sa2_attach created, unregisters
 * the callback and releases its references to Godot's device and queue.
 * ABI 5: on a sa2_create context, detach the surface first (else
 * SA2_E_WRONG_STATE); the core's own device and queue are released.
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
    uint32_t trace_ms; /* planned trace length, 1..3600000 ms; sizes the W3/W4
                        * preserved label readback (see sa2_scene_load) */
} sa2_scene_config;
#ifdef __cplusplus
static_assert(sizeof(sa2_scene_config) == 32, "sa2_scene_config requires the x64 ABI");
#endif

/* Load assets with S-B's readers and the four draw/count/geometry shader
 * blobs from the DLL's directory; create scene resources on the attached device.
 * ABI 3: the blobs are W-J's pose-path builds; W1..W4 bind identity poses and
 * draw through the pose path, as W-J's W3 run did (see ABI 3 below).
 * Register all three equally sized slots before load; keep that size until
 * unload. Missing slots or a second load: SA2_E_WRONG_STATE; unequal sizes:
 * SA2_E_INVALID_ARGUMENT. Missing/unreadable assets or blobs: SA2_E_IO (file
 * named in last error). D3D12 failures: SA2_E_D3D12 / SA2_E_DEVICE_REMOVED.
 * Load submits no work; static uploads execute with the first scene command.
 * W3/W4: load also allocates S-B's preserved label readback for
 * ceil(trace_ms / turn_ms) + 2 copies (W4: turn_ms 190) as one buffer, so
 * nothing is allocated inside the trace. A failed allocation fails the load.
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
 * A trace that needs more label copies than load reserved fails closed
 * (SA2_E_WRONG_STATE, the scene is poisoned): end the trace within trace_ms.
 *
 * C9 U4 (no ABI change): slot SA2_TARGET_SURFACE on a sa2_create context
 * draws into the attached surface's current back buffer instead and presents
 * it, as sa2_frame_submit does: it needs the surface at the scene's load-time
 * size, waits up to wait_timeout_ms for that buffer's previous frame and for
 * the frame-latency waitable (SA2_E_TIMEOUT), reads the record's QPC after
 * those waits, and presents with SA2_SURFACE_SYNC_INTERVAL. On a sa2_attach
 * context it answers SA2_E_WRONG_STATE. A core built before this extension
 * answers SA2_E_INVALID_ARGUMENT.
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
 * native.json into an existing UTF-8 directory. On a sa2_create context it
 * also writes presented.jsonl: one {"display_qpc","frame","present_count"}
 * row per traced frame that the surface's own frame statistics confirmed
 * displayed (C9, owner item 9). Never writes run.json or overwrites any
 * output. NULL/empty/malformed UTF-8: INVALID_ARGUMENT;
 * wrong lifecycle/drain: WRONG_STATE; file failures/existing output: E_IO.
 * Label mismatch (including an injection not reached): E_CHECK_FAILED with
 * every output written. Success: SA2_OK. Repeated writes need fresh outputs.
 * native.json's vram_peak_mb (MiB) is null with SA2_SCENE_NO_VRAM or when no
 * sample was taken; a harness refuses a gate record without it.
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

/* ABI 3 W-J scene API: W-J's per-piece pose path (overlays are not part of
 * ABI 3). W1..W4 keep their ABI 2 behaviour and outputs. The ABI 2 calls
 * sa2_scene_produce, sa2_scene_trace_begin, sa2_scene_trace_end,
 * sa2_scene_write_run, sa2_scene_geometry_check, sa2_identity and
 * sa2_scene_unload also serve a W-J scene, with these differences:
 *   - produce: W-J's revision protocol. During the trace, the first frame and
 *     every later turn upload the whole bundle (labels, piece pose index, pose
 *     matrix table and lattice table; revision = turn, sweep-before pose on even
 *     and sweep-after pose on odd revisions) into the next ring resource, bind
 *     it at once and, after the draw, copy exactly the bound bundle into the
 *     preserved readback. The injection, if any, applies at turn 20. The camera
 *     turns every frame (plane 0-3, 0.002 rad); the sweep animates on W-J's
 *     clock from trace begin and only inside the trace. Trace records always
 *     carry turn and phase.
 *   - write_run: also runs W-J's exact pose check and writes pose_check.json
 *     (W-J's complete result) beside trace.jsonl and native.json, none of them
 *     overwritten. native.json has scene "wj", fixture (wj-s4, wj-i-a, wj-i-b),
 *     overlay "none", turn_ms, label_check (W-J's), pose_check {status,
 *     revisions} and pose_upload. E_CHECK_FAILED when the label or pose check
 *     fails (an injection not reached fails both), with all outputs written.
 *   - geometry_check: W-J's check against
 *     work/experiments/renderer-wj/ref_<menu>_* (every reference digest, the
 *     sample rule, the per-cell count and 18 projections with stage poses).
 *     geometry.json has W-J's format, magic600-wj-geometry-check/1, plus
 *     "adapter"; build_identity is the SHA-256 of the sa2_identity JSON, as for
 *     ABI 2. A refused reference or stage export: E_CHECK_FAILED with output.
 *   - identity: lists lattice_cs.dxil too (every blob the DLL loads).
 * Invalid arguments (including struct_size and a nonzero reserved field)
 * return SA2_E_INVALID_ARGUMENT before state checks, as in ABI 2.
 */
typedef struct sa2_wj_config {
    uint32_t struct_size; /* sizeof(sa2_wj_config); 48 on x64 */
    uint32_t flags; /* SA2_SCENE_NO_VRAM only; unknown bits are invalid */
    const char* menu; /* "S4", "I_a" or "I_b"; copied */
    const char* data_utf8; /* UTF-8 directory written by W-J's prepare_wj.py:
                            * <menu>/ exports and lattice-* controls; copied */
    double turn_ms; /* finite, 0 < turn_ms <= 10000 */
    const char* inject; /* NULL, or corrupt-index, swap-same-colour, stale-pose,
                         * delay-adoption, stale-binding; copied */
    uint32_t trace_ms; /* planned trace length, 1..3600000 ms; sizes the
                        * preserved readback (see sa2_scene_load_wj) */
    uint32_t reserved; /* must be 0 */
} sa2_wj_config;
#ifdef __cplusplus
static_assert(sizeof(sa2_wj_config) == 48, "sa2_wj_config requires the x64 ABI");
#endif

/* Load a W-J scene with W-J's readers: the S-B model, turn and camera assets,
 * research/jumbling/fixtures/wj-<menu>.json checked against W-J's immutable
 * reference index, and <data>/<menu>/ motion, moving set and stage pose
 * exports, with every digest and refusal of W-J's reader; and all five shader
 * blobs. Slot, lifecycle and D3D12 error rules as sa2_scene_load. Missing,
 * unreadable or refused inputs: SA2_E_IO (reason in last error). An injection
 * the fixture cannot carry (W-J's injector refuses it): SA2_E_INVALID_ARGUMENT.
 * Load allocates the preserved readback for ceil(trace_ms / turn_ms) + 2
 * bundle copies as one buffer, so nothing is allocated inside the trace; a
 * failed allocation fails the load.
 */
SA2_API int32_t sa2_scene_load_wj(sa2_context* ctx, const sa2_wj_config* config);

/* W-J's lattice control check: with a loaded W-J scene, outside the trace
 * window, check the lattice-start and lattice-retained controls under <data>/
 * (label and destination lengths and digests, pose export) on the producer
 * queue with owned buffers, never a slot or a ring resource. Writes
 * lattice_check.json (W-J format, magic600-wj-lattice-check/1, plus "adapter")
 * in an existing UTF-8 directory without overwrite. Argument, lifecycle, wait,
 * error and drain rules as sa2_scene_geometry_check; a W1..W4 scene:
 * SA2_E_WRONG_STATE. A missing or refused control or any failing slot:
 * E_CHECK_FAILED with output written.
 */
SA2_API int32_t sa2_scene_lattice_check(sa2_context* ctx, const char* directory_utf8);

/* ABI 4 product API: one call per RM.14 operation, for the Rust display host
 * (rust/c600-display/src/host.rs, the Backend trait). Field names follow the
 * Rust types: every ID (view, command, surface, preview, ticket, resource set)
 * is the u64 inside its Rust newtype.
 *
 * Scope: the product calls draw the loaded scene's model (sa2_scene_load or
 * sa2_scene_load_wj), so every stateful call needs a loaded scene
 * (SA2_E_WRONG_STATE otherwise), and sa2_scene_unload discards the adopted
 * image, every ticket, resource set, frame record and pick. No product call
 * is allowed inside a scene trace (between sa2_scene_trace_begin and
 * sa2_scene_trace_end): SA2_E_WRONG_STATE. Product frames use their own label,
 * pose, constant and query resources, so W1..W4 and W-J outputs are unchanged.
 *
 * Arguments: invalid arguments (NULL pointers, struct_size, reserved fields,
 * values out of range) return SA2_E_INVALID_ARGUMENT before any state check,
 * as in ABI 2. Every new struct starts with struct_size, which the caller
 * sets to sizeof the struct; for an array of rows, in every row it passes.
 *
 * Device loss: after the device is removed, sa2_prepare, sa2_activate,
 * sa2_frame_submit and sa2_pick_request return SA2_E_DEVICE_REMOVED;
 * sa2_poll reports a pending or ready ticket as SA2_TICKET_FAILED with reason
 * SA2_E_DEVICE_REMOVED; sa2_frame_reports returns SA2_OK and writes the
 * frames whose GPU results were not read before the removal with error
 * SA2_E_DEVICE_REMOVED; sa2_pick_poll returns SA2_E_DEVICE_REMOVED.
 * sa2_engine_adopt keeps updating the CPU image (RM.16); sa2_cancel,
 * sa2_retire, sa2_describe, sa2_recover, sa2_counters and sa2_device_status
 * keep working.
 */
#define SA2_E_UNAVAILABLE 10 /* the backend cannot perform the operation; nothing was done */

#define SA2_SLOTS 259800u /* labelled sticker slots of 600-cell-Full */
#define SA2_PIECES 177120u /* pieces, the length of a pose index */
#define SA2_MAX_POSES 177120u /* pose table capacity: one pose per piece */
#define SA2_POSE_GROUP_ORDER 7200 /* |K+|: a pose_lattice entry is -1 or below this */

/* Ticket states (sa2_poll_row.state), the Rust Poll variants. */
#define SA2_TICKET_PENDING 0
#define SA2_TICKET_READY 1
#define SA2_TICKET_CANCELLED 2
#define SA2_TICKET_FAILED 3

/* Cancel reasons, the Rust CancelReason variants. */
#define SA2_CANCEL_BY_USER 0
#define SA2_CANCEL_SUPERSEDED 1 /* with superseded_by, the superseding command */
#define SA2_CANCEL_REPLACED_REVISION 2
#define SA2_CANCEL_BUDGET 3

/* Pick states and the binding field that made a reply stale. */
#define SA2_PICK_PENDING 0
#define SA2_PICK_READY 1
#define SA2_PICK_STALE 2
#define SA2_PICK_STALE_SURFACE 1 /* a newer surface_generation of the frame's surface was submitted */
#define SA2_PICK_STALE_VIEW 2 /* a newer view_generation of the frame's view was submitted */
#define SA2_PICK_STALE_TARGET 3 /* sa2_resize ran after the frame */
#define SA2_PICK_STALE_ENGINE 4 /* the frame's engine revision left the GPU ring before the ID pass */
#define SA2_PICK_STALE_PUBLICATION 5 /* the frame's resource set was retired before the request */

/* sa2_frame_input.flags */
#define SA2_FRAME_PREVIEW 1u /* the frame shows the preview named by preview, preview_base and its literal digest */

/* adopt_engine. One engine state, copied on the CPU: the labels in the layout
 * W1..W4 upload and, for a posed state, W-J's pose path arrays.
 */
typedef struct sa2_engine_image {
    uint32_t struct_size; /* sizeof(sa2_engine_image); 80 on x64 */
    uint32_t pose_count; /* entries in poses and pose_lattice, 1..SA2_MAX_POSES; 0 without a pose index */
    uint64_t revision; /* the engine state revision (FrameStamp.engine_revision); increases with every adoption */
    const uint32_t* labels; /* SA2_SLOTS labels by slot, each below SA2_SLOTS */
    const int32_t* pose_index; /* NULL, or SA2_PIECES pose indices by piece, each below pose_count */
    const float* poses; /* pose_count 4x4 matrices, rows contiguous, x' = M x; pose 0 is the identity; finite */
    const int32_t* pose_lattice; /* pose_count entries, -1 or 0..SA2_POSE_GROUP_ORDER - 1; entry 0 is 0 */
    uint8_t digest[32]; /* the caller's SHA-256 of the state (RevisionRef); stored, not recomputed */
} sa2_engine_image;
#ifdef __cplusplus
static_assert(sizeof(sa2_engine_image) == 80, "sa2_engine_image requires the x64 ABI");
#endif

/* prepare. One publication's per-view resources (F02): visibility and
 * interaction by slot, and the instance IDs the ID pass writes.
 */
typedef struct sa2_preparation {
    uint32_t struct_size; /* sizeof(sa2_preparation); 112 on x64 */
    uint32_t reserved; /* must be 0 */
    uint64_t command; /* PublicationStamp.command */
    uint64_t view; /* PublicationStamp.view */
    uint64_t view_generation;
    uint64_t publication_generation;
    uint64_t device_epoch; /* must equal the backend's (sa2_describe) */
    uint64_t base_revision; /* PublicationStamp.base.state */
    const uint8_t* visibility; /* SA2_SLOTS bytes, 0 or 1: the slot is drawn */
    const uint8_t* interaction; /* SA2_SLOTS bytes, 0 or 1: the slot answers picks (RPI.75) */
    const uint32_t* instances; /* SA2_SLOTS instance IDs by slot; 0 = none */
    uint8_t input_digest[32]; /* PublicationStamp.input_digest; stored */
} sa2_preparation;
#ifdef __cplusplus
static_assert(sizeof(sa2_preparation) == 112, "sa2_preparation requires the x64 ABI");
#endif

/* poll. The Rust Poll: Pending, Ready(ResourceSet), Cancelled(CancelReason), Failed(Error). */
typedef struct sa2_poll_row {
    uint32_t struct_size; /* sizeof(sa2_poll_row); 32 on x64 */
    int32_t state; /* SA2_TICKET_* */
    int32_t reason; /* CANCELLED: SA2_CANCEL_*; FAILED: the SA2_E_* status; else 0 */
    uint32_t reserved; /* written as 0 */
    uint64_t resource_set; /* READY: the resource set; else 0 */
    uint64_t superseded_by; /* CANCELLED with SA2_CANCEL_SUPERSEDED: the command; else 0 */
} sa2_poll_row;
#ifdef __cplusplus
static_assert(sizeof(sa2_poll_row) == 32, "sa2_poll_row requires the x64 ABI");
#endif

/* submit_frame. The FrameStamp, the camera matrix and the preview shown. */
typedef struct sa2_frame_input {
    uint32_t struct_size; /* sizeof(sa2_frame_input); 264 on x64 */
    uint32_t target; /* the frame target: on the Godot-texture route, the registered ring slot; ABI 5: SA2_TARGET_SURFACE */
    uint32_t flags; /* SA2_FRAME_PREVIEW or 0 */
    uint32_t reserved; /* must be 0 */
    uint64_t device_epoch; /* must equal the backend's */
    uint64_t surface;
    uint64_t surface_generation; /* never decreases for a surface */
    uint64_t sequence; /* increases within the surface */
    uint64_t engine_revision; /* the adopted revision */
    uint64_t view;
    uint64_t view_generation; /* never decreases for a view */
    uint64_t publication_generation; /* with a resource set, the set's */
    uint64_t camera; /* the camera version */
    uint32_t viewport_width; /* equal to the target's size */
    uint32_t viewport_height;
    uint64_t visibility; /* the visibility version */
    uint64_t interaction; /* the interaction version */
    uint64_t resource_set; /* an activated set prepared for this view and view generation, or 0: all slots drawn, nothing pickable */
    uint64_t preview; /* PreviewShown.preview; 0 without SA2_FRAME_PREVIEW */
    uint64_t preview_base; /* PreviewShown.base; 0 without SA2_FRAME_PREVIEW */
    uint8_t subjects_digest[32]; /* the caller's digest of FrameStamp.subjects; stored */
    uint8_t preview_literal_digest[32]; /* SHA-256 of PreviewShown.literal; zero without SA2_FRAME_PREVIEW */
    float camera_matrix[16]; /* the 4D rotation Q in the layout W1..W4 use (column-major float4x4); finite */
} sa2_frame_input;
#ifdef __cplusplus
static_assert(sizeof(sa2_frame_input) == 264, "sa2_frame_input requires the x64 ABI");
#endif

/* read_frame_reports. One row per frame the GPU completed (RM.41, RM.39). */
typedef struct sa2_frame_report {
    uint32_t struct_size; /* sizeof(sa2_frame_report); 104 on x64 */
    int32_t error; /* SA2_OK, or the status that stopped the frame; such a frame drew nothing */
    uint32_t fallback; /* RM.41's fallback; always 0 in ABI 4 */
    uint32_t reserved; /* written as 0 */
    uint64_t frame_id; /* the value sa2_frame_submit returned */
    uint64_t device_epoch;
    uint64_t surface;
    uint64_t surface_generation;
    uint64_t sequence;
    uint64_t engine_revision;
    uint64_t view;
    uint64_t view_generation;
    uint64_t publication_generation;
    uint64_t gpu_time_ns; /* GPU timestamps around the frame's draw */
    uint64_t view_vertex_evals; /* D3D12 pipeline statistics VSInvocations of the frame's draw (RM.39) */
} sa2_frame_report;
#ifdef __cplusplus
static_assert(sizeof(sa2_frame_report) == 104, "sa2_frame_report requires the x64 ABI");
#endif

/* read_presented. Written only on an ABI 5 surface context (see sa2_presented). */
typedef struct sa2_presented_row {
    uint32_t struct_size; /* sizeof(sa2_presented_row); 56 on x64 */
    uint32_t previews; /* 0: no_preview; 1: the frame showed its preview */
    uint64_t frame_id;
    uint64_t device_epoch;
    uint64_t surface;
    uint64_t sequence;
    uint64_t preview;
    uint64_t display_qpc; /* SyncQPCTime of the statistics row that confirmed the frame (QPC ticks) */
} sa2_presented_row;
#ifdef __cplusplus
static_assert(sizeof(sa2_presented_row) == 56, "sa2_presented_row requires the x64 ABI");
#endif

/* poll_pick. The Rust PickReply: the frame's PickBinding and the instance. */
typedef struct sa2_pick_row {
    uint32_t struct_size; /* sizeof(sa2_pick_row); 112 on x64 */
    int32_t state; /* SA2_PICK_PENDING, SA2_PICK_READY or SA2_PICK_STALE */
    uint32_t stale; /* SA2_PICK_STALE_* when state is SA2_PICK_STALE; else 0 */
    uint32_t instance; /* the ID-pass value at (x, y); 0 = no instance */
    uint64_t frame_id;
    uint64_t device_epoch;
    uint64_t surface;
    uint64_t surface_generation;
    uint64_t view;
    uint64_t view_generation;
    uint64_t publication_generation;
    uint64_t camera;
    uint64_t visibility;
    uint64_t interaction;
    uint32_t viewport_width;
    uint32_t viewport_height;
    uint32_t x;
    uint32_t y;
} sa2_pick_row;
#ifdef __cplusplus
static_assert(sizeof(sa2_pick_row) == 112, "sa2_pick_row requires the x64 ABI");
#endif

/* Counters since the scene was loaded. */
typedef struct sa2_counter_values {
    uint32_t struct_size; /* sizeof(sa2_counter_values); 64 on x64 */
    uint32_t reserved; /* written as 0 */
    uint64_t view_vertex_evals_last; /* of the newest reported frame */
    uint64_t view_vertex_evals_total; /* over every reported frame */
    uint64_t bytes_uploaded; /* engine images and preparations recorded for upload */
    uint64_t frames_submitted;
    uint64_t frames_reported;
    uint64_t id_passes;
    uint64_t reports_dropped; /* unread reports dropped past the bound of 1024 */
} sa2_counter_values;
#ifdef __cplusplus
static_assert(sizeof(sa2_counter_values) == 64, "sa2_counter_values requires the x64 ABI");
#endif

/* describe_backend. Writes the SupportReport as NUL-terminated JSON:
 * {interface_version: SA2_ABI_VERSION, backend_build: SHA-256 of the sa2_identity JSON,
 * device_epoch, device_removed: null or the removal HRESULT as "0x%08x",
 * features: {row: {status: "supported", layout_version} or {status:
 * "unsupported", reason}}, evidence: []}. Supported rows: F01 full-detail
 * stickers (all SA2_SLOTS), F02 visibility and interaction sets, J01 and J02
 * the posed sticker path, F18 the stamped ID pass. F22 (recover) and F23
 * (read_presented) are supported on a sa2_create context with an attached
 * surface (ABI 5, route B) and listed as unsupported otherwise; J13 (grip,
 * piece and certificate-point IDs) is listed as unsupported; an omitted row
 * is unsupported (RM.15). device_epoch is the context's: 1, plus one per
 * successful sa2_recover. The evidence list is
 * empty: no measured run backs these rows yet. *out_size receives the JSON
 * length plus one; a smaller
 * buffer_size returns SA2_E_INVALID_ARGUMENT and writes an empty string.
 */
SA2_API int32_t sa2_describe(sa2_context* ctx, char* buffer, uint32_t buffer_size, uint32_t* out_size);

/* adopt_engine. Validates and copies the image on the CPU and returns its
 * revision; no GPU work and no GPU wait (RM.16). The next sa2_frame_submit
 * naming this revision uploads it into the next of three GPU ring copies
 * before it draws. A revision that does not increase: SA2_E_WRONG_STATE.
 * Adoption works during device loss as well (RM.16).
 */
SA2_API int32_t sa2_engine_adopt(sa2_context* ctx, const sa2_engine_image* image, uint64_t* out_revision);

/* prepare. Copies the preparation, records its upload on the producer queue
 * with a command list of its own, signals the product fence and returns the
 * ticket. The resource set is ready when that fence is reached (sa2_poll). A
 * device_epoch other than the backend's, or 256 tickets still holding GPU
 * memory (2 MiB each, until the set is released after retire or cancel):
 * SA2_E_WRONG_STATE.
 */
SA2_API int32_t sa2_prepare(sa2_context* ctx, const sa2_preparation* preparation, uint64_t* out_ticket);

/* poll. Never waits. A ticket is forgotten when its resource set is retired.
 * A cancelled or failed ticket keeps answering that state (RM.17) until 1024
 * newer cancelled or failed tickets have released their resources. An
 * unknown or forgotten ticket: SA2_E_WRONG_STATE. A ticket cancelled while
 * pending stays SA2_TICKET_CANCELLED after its upload completes (RM.17).
 */
SA2_API int32_t sa2_poll(sa2_context* ctx, uint64_t ticket, sa2_poll_row* out);

/* activate. Makes a ready resource set drawable by frames of its view and
 * view generation and stores the PublicationStamp digest. A set that is not
 * ready, already active or retired: SA2_E_WRONG_STATE.
 */
SA2_API int32_t sa2_activate(sa2_context* ctx, uint64_t resource_set, const uint8_t stamp_digest[32]);

/* cancel. reason is SA2_CANCEL_*; superseded_by is the command for
 * SA2_CANCEL_SUPERSEDED and 0 otherwise. Cancelling a pending or a ready but
 * unactivated ticket discards its resources once its upload completes;
 * cancelling a cancelled or failed ticket does nothing; a ticket whose set is
 * active: SA2_E_WRONG_STATE (retire the set instead).
 */
SA2_API int32_t sa2_cancel(sa2_context* ctx, uint64_t ticket, int32_t reason, uint64_t superseded_by);

/* retire. Later frames cannot name the set; picks on earlier frames that
 * used it answer SA2_PICK_STALE_PUBLICATION. Its GPU resources are released
 * once every frame and ID pass that used it has completed. An unknown or
 * retired set: SA2_E_WRONG_STATE.
 */
SA2_API int32_t sa2_retire(sa2_context* ctx, uint64_t resource_set);

/* submit_frame. Checks the stamp (device epoch, adopted engine revision,
 * viewport equal to the target's size, increasing sequence and
 * non-decreasing surface and view generations, the resource set), then draws
 * the loaded model into the frame target with the frame's camera and the set's
 * visibility, as sa2_scene_produce draws (bounded wait for the target's
 * previous frame, fail-closed free check, queue mode and handover states). One
 * ClearRenderTargetView without rectangles clears the target. *out_frame_id
 * receives the frame ID: the ready fence value the frame signals, one more
 * than the last frame any produce call signalled, for sa2_godot_wait_ready
 * and sa2_mark_shown. Stamp rule failures: SA2_E_WRONG_STATE.
 */
SA2_API int32_t sa2_frame_submit(sa2_context* ctx, const sa2_frame_input* input, uint64_t* out_frame_id);

/* read_frame_reports. Never waits. Writes up to capacity rows, in frame
 * order, for frames whose ready fence the GPU reached, each frame once; it
 * stops at the first frame not yet completed. *out_count receives the number
 * written. rows may be NULL when capacity is 0.
 */
SA2_API int32_t sa2_frame_reports(sa2_context* ctx, sa2_frame_report* rows, uint32_t capacity, uint32_t* out_count);

/* read_presented. On a context without a surface the core owns no swap
 * chain and cannot confirm a presentation (RM.26): after the argument checks
 * it sets *out_count to 0, writes no row and returns SA2_E_UNAVAILABLE. On an
 * ABI 5 surface context, see "ABI 5" below.
 */
SA2_API int32_t sa2_presented(sa2_context* ctx, sa2_presented_row* rows, uint32_t capacity, uint32_t* out_count);

/* request_pick. Records an ID pass with that frame's camera, viewport,
 * engine ring copy, visibility and instance IDs into an R32_UINT target of
 * the target's size on the producer queue (same vertex stage, no discard;
 * RPI.22), copies texel (x, y) to a readback buffer and signals the product
 * fence. A frame without a resource set answers SA2_PICK_READY with instance
 * 0 and no GPU work; the stale cases detectable at request time answer at
 * once with instance 0. The frame must be retained (the last 64 reported
 * frames and every unreported one) and drawn, and fewer than 1024 picks may be
 * live (else SA2_E_WRONG_STATE); x and y lie inside its viewport (else
 * SA2_E_INVALID_ARGUMENT). The x and y check needs the frame record, so a
 * removed device or a frame that is not retained answers first.
 */
SA2_API int32_t sa2_pick_request(sa2_context* ctx, uint64_t frame_id, uint32_t x, uint32_t y, uint64_t* out_ticket);

/* poll_pick. Never waits. Pending until the ID pass completes; then reports
 * the instance with the frame's binding, SA2_PICK_STALE when a newer surface
 * or view generation was submitted, the target was resized or the request
 * found the binding gone (RPI.23 to RPI.25). A reported pick is forgotten: a
 * second poll returns SA2_E_WRONG_STATE.
 */
SA2_API int32_t sa2_pick_poll(sa2_context* ctx, uint64_t ticket, sa2_pick_row* out);

/* recover. ABI 4 cannot recover a device (RPI.46): on a sa2_attach context it
 * writes NUL-terminated JSON {status: "unavailable", reason, device_epoch,
 * device_removed: null or "0x%08x"}, sets *out_size as sa2_describe and
 * returns SA2_E_UNAVAILABLE. On an ABI 5 sa2_create context, see "ABI 5" below.
 */
SA2_API int32_t sa2_recover(sa2_context* ctx, char* buffer, uint32_t buffer_size, uint32_t* out_size);

/* The frame target changed size. After a confirmed sa2_drain, with a loaded
 * scene outside a trace and all three slots registered at width x height
 * (the host registers the new slots first), recreate the size-bound
 * resources (depth and ID-pass targets) and start a new target generation:
 * picks on earlier frames answer SA2_PICK_STALE_TARGET. W1..W4 and W-J draw
 * at the new size afterwards. Width and height are at least SA2_MIN_TEXTURE_PX.
 * On an ABI 5 sa2_create context the surface is resized instead (see "ABI 5").
 */
SA2_API int32_t sa2_resize(sa2_context* ctx, uint32_t width, uint32_t height);

/* Copies the counters; all zero without a loaded scene. */
SA2_API int32_t sa2_counters(sa2_context* ctx, sa2_counter_values* out);

/* *out_removed_reason receives GetDeviceRemovedReason (S_OK while alive).
 * Returns SA2_OK while the device is alive and SA2_E_DEVICE_REMOVED after
 * removal, which the product calls then observe.
 */
SA2_API int32_t sa2_device_status(sa2_context* ctx, int32_t* out_removed_reason);

/* ABI 5: route B. Route B is accepted for now by the owner on 18 Nov 2026,
 * pending three owner-machine checks (docs/wiki/decisions/
 * owner-decisions-2026-11-18-route-b.md). The core draws the product frames
 * on a Direct3D 12 device of its own and presents them in its own composition
 * swap chain, in a DirectComposition layer beneath the host window's content,
 * so it can confirm a presentation per frame (RM.26) and recover from the loss
 * of its own device (RPI.46). The Godot-texture route (sa2_attach, the ring
 * slots, W1..W4 and W-J) is unchanged. ABI 5 adds no struct; it extends
 * sa2_presented_row, which it is the first to write, with display_qpc.
 *
 * A sa2_create context runs every ABI 4 product call; its frames name
 * SA2_TARGET_SURFACE. The ring-slot calls (sa2_create_texture,
 * sa2_release_texture, sa2_register_slot, sa2_unregister_slot,
 * sa2_signal_godot_free, sa2_produce, sa2_godot_wait_ready, sa2_mark_shown,
 * sa2_verify_slot and sa2_scene_produce with a ring slot) answer
 * SA2_E_WRONG_STATE after their argument checks; sa2_scene_produce with
 * SA2_TARGET_SURFACE draws the scene on the surface (C9 U4, no ABI change).
 * sa2_scene_load and sa2_scene_load_wj need an attached
 * surface instead of the three slots and use its size; the context keeps a
 * copy of the scene config for sa2_recover. The surface calls on a sa2_attach
 * context answer SA2_E_WRONG_STATE. Invalid arguments return
 * SA2_E_INVALID_ARGUMENT before any state check.
 *
 * The surface's back buffers move between COMMON (PRESENT) and RENDER_TARGET
 * whatever the config's handover states. The swap chain presents on the
 * producer queue: the core's queue, or its second queue with SA2_QUEUE_OWN.
 */
#define SA2_TARGET_SURFACE 0xFFFFFFFFu /* sa2_frame_input.target: the attached surface's current back buffer */
#define SA2_SURFACE_SYNC_INTERVAL 1u /* the Present sync interval of every surface frame: one vertical blank */
#define SA2_SURFACE_BUFFERS 3u /* swap-chain buffers; FLIP_DISCARD, frame-latency waitable, maximum frame latency 2 */
#define SA2_SURFACE_MAX_PX 16384u /* the largest surface width and height */

/* Creates a context on a Direct3D 12 device of the core's own, made by the
 * D3D12 device factory without storing it as the process's singleton device,
 * so it is never Godot's device. The adapter is the one whose LUID is
 * adapter_luid ((HighPart << 32) | LowPart), or for 0 the first hardware
 * adapter DXGI lists. The context gets its own DIRECT queue and everything
 * sa2_attach creates; config is validated exactly as sa2_attach validates it.
 * An adapter_luid no adapter has: SA2_E_INVALID_ARGUMENT; no hardware adapter
 * or no device factory: SA2_E_UNSUPPORTED. sa2_detach ends the context.
 */
SA2_API int32_t sa2_create(uint64_t adapter_luid, const sa2_config* config, sa2_context** out);

/* Attaches the surface. It creates a DirectComposition device, a target for
 * hwnd with CreateTargetForHwnd(hwnd, FALSE), beneath the host's topmost
 * target, one visual at (x, y) in the window's client pixels, and a
 * composition swap chain of width x height on the producer queue
 * (B8G8R8A8_UNORM, ALPHA_MODE_IGNORE, SA2_SURFACE_BUFFERS buffers,
 * FLIP_DISCARD, the frame-latency waitable object), then commits. hwnd must be
 * a window; width and height lie in SA2_MIN_TEXTURE_PX .. SA2_SURFACE_MAX_PX.
 * Only on a sa2_create context with no surface (else SA2_E_WRONG_STATE) and
 * a live device (else SA2_E_DEVICE_REMOVED). A failed DirectComposition or
 * DXGI call: SA2_E_D3D12, and nothing is attached. Each new swap chain, resize and recovery starts a
 * new swap-chain generation.
 */
SA2_API int32_t sa2_surface_attach(sa2_context* ctx, uint64_t hwnd, int32_t x, int32_t y, uint32_t width, uint32_t height);

/* Moves the surface's visual to (x, y) and commits. No GPU work; it works
 * during device loss. No surface: SA2_E_WRONG_STATE.
 */
SA2_API int32_t sa2_surface_move(sa2_context* ctx, int32_t x, int32_t y);

/* Diagnostics for C9's measurements, no ABI change: writes the address of the
 * surface's current swap chain (its IDXGISwapChain interface pointer) to *out,
 * so that the shell can name the core's swap chain among the presents of a
 * PresentMon capture (DRAFT: the owner-machine run confirms that PresentMon
 * prints this address). A resize or a recovery makes a new swap chain with a
 * new address. No GPU work; it works during device loss. A NULL out:
 * SA2_E_INVALID_ARGUMENT; no surface: SA2_E_WRONG_STATE. A shell may treat the
 * export as optional, since an ABI 5 core built before it lacks it.
 */
SA2_API int32_t sa2_surface_swap_chain(sa2_context* ctx, uint64_t* out);

/* Detaches the surface: clears the visual's content and the target's root,
 * commits, and releases the swap chain and the DirectComposition objects.
 * Needs the scene unloaded and a confirmed sa2_drain (a removed device permits
 * it without one), else SA2_E_WRONG_STATE. As in the C9 probe, the
 * DirectComposition results are not checked: releasing the target removes the
 * layer in any case.
 */
SA2_API int32_t sa2_surface_detach(sa2_context* ctx);

/*
 * sa2_frame_submit with target SA2_TARGET_SURFACE waits up to
 * config.wait_timeout_ms for the back buffer's previous frame and then for the
 * frame-latency waitable (SA2_E_TIMEOUT, nothing recorded), draws as on a ring
 * slot, signals the ready fence and presents with
 * SA2_SURFACE_SYNC_INTERVAL. The frame's own present count is
 * GetLastPresentCount right after its Present. A failed Present returns its
 * status (SA2_E_D3D12 or SA2_E_DEVICE_REMOVED) and poisons the scene, as any
 * failed submission does.
 *
 * sa2_presented on a surface context reads GetFrameStatistics and
 * GetLastPresentCount once per call. A surface frame whose wait for the
 * frame-latency waitable lasts longer than 5 ms also reads them once during
 * that wait (C9, 26 November 2026), for the same ledger, so that a frame shown
 * for a whole refresh meets a statistics row of its own even when the call
 * after each frame runs before the vblank's statistics are posted; a failed
 * read there adds no row and never fails the frame. A frame is confirmed only by the C9
 * present probe's exact rule: a statistics row whose PresentCount equals the
 * frame's own present count, in the frame's swap-chain generation, with
 * PresentCount not above GetLastPresentCount. DXGI_ERROR_FRAME_STATISTICS_
 * DISJOINT or any other failed call is no row. A frame whose count a row
 * passed, or whose swap-chain generation ended (resize, recovery), is never
 * reported. Each confirmed frame is written once, in frame order, with
 * previews and preview from its input and display_qpc, the SyncQPCTime of
 * the statistics row that confirmed it; up to 1024 confirmed and 1024 awaiting
 * frames are kept, the oldest dropped first and never reported. A failed
 * statistics call writes no row and returns SA2_OK with *out_count 0, unless
 * the device is removed (SA2_E_DEVICE_REMOVED). Needs a loaded scene.
 *
 * sa2_resize on a sa2_create context needs an attached surface and a
 * confirmed sa2_drain; width and height are at most SA2_SURFACE_MAX_PX. It
 * resizes the swap chain's buffers and, with a loaded scene, the size-bound
 * resources with a new target generation, as in ABI 4. A scene is not required.
 * A sa2_resize that fails after ResizeBuffers succeeded leaves surface frames
 * answering SA2_E_WRONG_STATE until a sa2_resize to the same size succeeds;
 * the failed call queues no GPU work, so the drain still counts as confirmed.
 *
 * sa2_recover on a sa2_create context whose device is removed builds a new
 * device and queue on the same adapter with what sa2_create made, a new swap
 * chain at the surface's size set as the same visual's content (or a rebuilt
 * DirectComposition target and visual on the same hwnd and rectangle), and the
 * scene loaded again from its retained config. Only then are the old device's
 * objects released, so a failed recovery leaves the context removed and may be
 * retried; it returns the failure's status and writes {status: "failed",
 * reason, device_epoch, device_removed} when the buffer holds it. On success
 * the adopted CPU engine image is kept (RM.16) and the next frame uploads it,
 * device_epoch increases by one, and it writes {status: "recovered", reason,
 * device_epoch: the new epoch, device_removed: the removal reason as
 * "0x%08x"} and returns SA2_OK. The old epoch's tickets, resource sets,
 * frames and picks are gone: sa2_poll or sa2_cancel of an old ticket,
 * sa2_activate or sa2_retire of an old set, sa2_pick_request on an old frame
 * and sa2_pick_poll of an old pick answer SA2_E_WRONG_STATE, as for an unknown
 * one. Ticket, set, pick and frame numbers keep counting and are never reused.
 * Unread reports of old frames are dropped (reports_dropped), and old frames
 * are never confirmed as presented. On a live device it writes {status:
 * "not_removed", reason, device_epoch, device_removed: null} and returns
 * SA2_E_WRONG_STATE. A buffer too small for the success JSON:
 * SA2_E_INVALID_ARGUMENT before any change.
 */

/* C9 U5: user shader stages, additive in ABI 5 (the U5 design names ABI 6;
 * these calls stay in ABI 5 and change no earlier call). A stage is HLSL text
 * with one fixed entry:
 *   vertex:   uint m600_vertex(float x, float y, float z, float w, float t, out float3 position)
 *             x, y, z, w: the vertex's Q*world; position: the projected 3D
 *             position; the return value is the coverage code: 0 a point,
 *             1 singular_neighbourhood, 2 nonfinite (position is used only
 *             when the code is 0; a nonfinite clip position counts as 2).
 *   fragment: float3 m600_fragment(float class_, float shade, float x, float y, float z, float t)
 *             x, y, z: the projected 3D position; the result is the colour.
 * t is the seconds since the chain was attached. The stage reads its arrays
 * from StructuredBuffer<float> m600_vertex_arrays : register(t20) and
 * m600_fragment_arrays : register(t21). The core declares a buffer only when
 * the stage text does not: when the text names the buffer's identifier, the
 * composed source defines M600_VERTEX_ARRAYS_DECLARED (or
 * M600_FRAGMENT_ARRAYS_DECLARED) after the stage text, and the core's own
 * declaration sits under #ifndef of that guard. A stage that names the buffer
 * declares it itself, with the register above.
 *
 * sa2_stage_compile composes the pass source (the kind's define, the stage
 * text, the guards, then the core's geometry.hlsl, draw.hlsl and stages.hlsl,
 * embedded in the DLL at build time) and compiles every pass of the kind with
 * DXC at run time: exactly -T <vs|ps|cs>_6_2 -E <entry> -HV 2021 -Gis
 * -denorm preserve -O3 -Ges -WX. dxcompiler.dll and dxil.dll load on first use
 * from the core DLL's own folder by absolute path, never the search path
 * (SA2_E_UNAVAILABLE when either is missing; a later call retries). The first
 * compile on a device also builds the core's own passes. It writes a
 * NUL-terminated compile record into record (record_size at least
 * SA2_STAGE_RECORD_BYTES; *out_record_size includes the NUL): {format:
 * "magic600-stage-compile/1", kind, status: "compiled" | "compile_failed" |
 * "compiler_unavailable" | "pipeline_failed", code: null or
 * "EV3:input.stage_compile_failed", handle, stage_sha256, compiler: {library,
 * version, commit_count, commit_hash, custom_version, validator} from
 * IDxcVersionInfo, passes and core_passes: [{entry, profile, arguments,
 * source_sha256 of the composed source, dxil_sha256, status}], error: DXC's
 * error text, error_truncated}. On success *out_handle is the new handle
 * (nonzero, never reused, not even after sa2_recover). A failure keeps
 * nothing, sets *out_handle to 0, writes the record with code
 * "EV3:input.stage_compile_failed" and returns SA2_E_CHECK_FAILED for a
 * compile error (or a pass source that cannot be composed: its source_sha256
 * is null), SA2_E_UNAVAILABLE without the compiler (also when DXC cannot
 * create a compiler object for a pass), or the D3D12 status of a failed
 * pipeline; the built-in chain stays usable. Text that is empty,
 * longer than SA2_STAGE_HLSL_MAX_BYTES, not UTF-8 or holds a NUL:
 * SA2_E_INVALID_ARGUMENT, nothing compiled.
 *
 * sa2_scene_stages attaches a vertex and a fragment stage (0: the built-in
 * stage of that kind) with the overdraw allowance and the stages' arrays
 * (binary32 bit patterns, at most SA2_STAGE_ARRAY_MAX each) to the loaded W1..W4
 * scene; (0, 0) detaches. It needs a loaded scene outside a trace
 * (SA2_E_WRONG_STATE); W-J refuses an attach. A failure keeps the current
 * chain. Every later sa2_scene_produce frame, traced or not (C9 U5c: the
 * gate's measured frames are traced), on a ring slot or the surface, runs
 * RPI.72: eval (compute: the built-in chain up to Q*world, the W3 turn of a
 * traced frame included, the vertex stage, camera and clip, into one record
 * per vertex), bound (compute: U, raster.rs's exact 64-bit raster bound at one
 * sample and one pass: scene frames draw at MSAA 1 and run no ID pass),
 * decide (stopped when U > width * height * overdraw), then the user draw
 * predicated on not stopped and the built-in draw predicated on stopped. A
 * triangle with an undefined vertex is drawn outside the clip volume. A frame
 * without a chain draws exactly as before.
 *
 * sa2_stage_report writes the oldest completed frame drawn with a chain that
 * it has not written yet, so a caller that reports after every frame, and
 * again while the frame number still rises, sees every chain frame once in
 * frame order; with none waiting it writes the last written row again (frame
 * 0 before the first). At most 4096 completed rows wait; past that the oldest
 * are dropped. sa2_stage_release releases a handle; an attached one answers
 * SA2_E_WRONG_STATE.
 *
 * Test hook: with M600_SA2_STAGE_EVAL_DUMP set to a file path when a scene
 * first attaches a chain, every chain frame also copies its eval records
 * (sa2 stage records, 16 bytes per vertex: the clip position as four binary32
 * values, or (0, 0, 0, NaN) with the coverage code in the NaN's low payload
 * bits) and the sa2_stage_report call that first reports the newest such
 * frame writes them to that path.
 */
#define SA2_STAGE_VERTEX 1u
#define SA2_STAGE_FRAGMENT 2u
#define SA2_STAGE_RECORD_BYTES 65536u /* the smallest compile record buffer */
#define SA2_STAGE_HLSL_MAX_BYTES 1048576u /* the longest stage text */
#define SA2_STAGE_ARRAY_MAX 1048576u /* the most array values per stage kind */

typedef struct sa2_stage_frame {
    uint32_t struct_size; /* sizeof(sa2_stage_frame); 64 on x64 */
    uint32_t stopped; /* 1 when U exceeded the allowance and the built-in draw ran */
    uint64_t frame; /* the sa2_scene_produce frame; 0 before the first */
    uint64_t bound; /* U: the raster bound over every triangle of the frame */
    uint64_t allowance; /* width * height * overdraw */
    uint64_t vertex_stage; /* the attached handles; 0 for built-in */
    uint64_t fragment_stage;
    uint64_t user_pixel_invocations; /* pipeline statistics of the user draw */
    uint64_t builtin_pixel_invocations; /* and of the built-in fallback draw */
} sa2_stage_frame;
#ifdef __cplusplus
static_assert(sizeof(sa2_stage_frame) == 64, "sa2_stage_frame requires the x64 ABI");
#endif

SA2_API int32_t sa2_stage_compile(sa2_context* ctx, uint32_t kind, const char* hlsl_utf8, uint32_t hlsl_size, uint64_t* out_handle, char* record, uint32_t record_size, uint32_t* out_record_size);
SA2_API int32_t sa2_scene_stages(sa2_context* ctx, uint64_t vertex_stage, uint64_t fragment_stage, uint32_t overdraw, const uint32_t* vertex_arrays, uint32_t vertex_count, const uint32_t* fragment_arrays, uint32_t fragment_count);
SA2_API int32_t sa2_stage_report(sa2_context* ctx, sa2_stage_frame* out);
SA2_API int32_t sa2_stage_release(sa2_context* ctx, uint64_t handle);

/* ABI 6: a view's shape (ADR-D02's build, a candidate behind the shell's --d02; on 26 November 2026 the owner asked
 * for smaller stickers at Look Lab's opening values). Additive: every ABI 5 call, struct and behaviour is unchanged,
 * and a view whose shape was never set draws byte-identically to ABI 5.
 *
 * sa2_view_shape_set sets the shape that the product frames of `view` (sa2_frame_input.view) and their ID passes draw
 * with: the cell shrink (Look Lab's 1 - gap), the sticker shrink, the 4D eye distance d4 and the zoom (Look Lab's
 * 1 / tan(field of view / 2)). ABI 5 hard-codes them as the SA2_SHAPE_DEFAULT_* values; a NULL shape restores those.
 * Each value must be finite and inside its range below, struct_size must be sizeof(sa2_view_shape) and reserved 0,
 * else SA2_E_INVALID_ARGUMENT and nothing changes. The shape is device-free product state: it needs a loaded scene
 * outside a trace (SA2_E_WRONG_STATE), works during device loss, is kept by sa2_recover and is forgotten by
 * sa2_scene_unload. A frame keeps the shape it was drawn with, so a pick on it draws its ID pass with that shape.
 */
#define SA2_SHAPE_DEFAULT_CELL_SHRINK 0.76f /* ABI 5's constants: every view's shape until one is set */
#define SA2_SHAPE_DEFAULT_STICKER_SHRINK 0.82f
#define SA2_SHAPE_DEFAULT_D4 1.18f
#define SA2_SHAPE_DEFAULT_ZOOM 1.15f
#define SA2_SHAPE_D4_MAX 64.0f
#define SA2_SHAPE_ZOOM_MAX 64.0f

typedef struct sa2_view_shape {
    uint32_t struct_size; /* sizeof(sa2_view_shape); 24 on x64 */
    float cell_shrink; /* 0 < cell_shrink <= 1 */
    float sticker_shrink; /* 0 < sticker_shrink <= 1 */
    float d4; /* 1 < d4 <= SA2_SHAPE_D4_MAX: every drawn point lies in the unit ball, so w < d4 */
    float zoom; /* 0 < zoom <= SA2_SHAPE_ZOOM_MAX */
    uint32_t reserved; /* must be 0 */
} sa2_view_shape;
#ifdef __cplusplus
static_assert(sizeof(sa2_view_shape) == 24, "sa2_view_shape requires the x64 ABI");
#endif

SA2_API int32_t sa2_view_shape_set(sa2_context* ctx, uint64_t view, const sa2_view_shape* shape);

/* ABI 7: per-piece style (ADR-D02 build step B14, a candidate behind the shell's --d02; rows J07 and J09 of the
 * feature matrix). Additive: every ABI 6 call, struct and behaviour is unchanged, and a frame with no style draws
 * through the unchanged product pipeline, byte-identically to ABI 6.
 *
 * sa2_style_set sets one channel of the adopted engine revision's piece style: SA2_PIECES bytes, one per piece, bound
 * to that revision as its pose arrays are. A revision other than the adopted one is refused (SA2_E_INVALID_ARGUMENT),
 * and adopting a newer revision drops both channels. A NULL piece_style clears the channel. A fill byte holds a
 * palette entry in SA2_STYLE_INDEX (0: the piece keeps its label colour) and may set SA2_STYLE_DIM; an outline byte
 * holds a palette entry only (0: no outline). Any other bit is refused and changes nothing.
 *
 * sa2_view_style_set sets a style slot (1 to SA2_STYLE_SLOTS). A product frame drawn by sa2_frame_submit uses slot 1;
 * a composite frame's pane names its slot. A NULL style clears the slot. tint_mode says which channels draw;
 * dim_mode which pieces dim (SA2_STYLE_DIM_MARKED: those whose fill byte has SA2_STYLE_DIM; SA2_STYLE_DIM_UNMARKED:
 * every other piece, a focus); a dimmed colour is multiplied by 1 - dim_amount. A tint mixes the palette entry
 * (shaded as the label colour is) over the label colour by its alpha; an outline mixes the entry over the band of each
 * sticker triangle within outline_width of an edge (barycentric units). The row-major 3x3 matrix then maps every drawn
 * colour (a colour-vision or contrast simulation); with the identity it is skipped. Out-of-range values, non-finite
 * values or unknown mode bits are refused and change nothing.
 *
 * A frame draws styled only when its slot is set and the style changes something: a non-zero piece byte in a drawn
 * channel, a dim that applies, or a matrix other than the identity. Otherwise it draws exactly as ABI 6. Both calls are
 * device-free product state: they need a loaded scene outside a trace (SA2_E_WRONG_STATE), work during device loss,
 * are kept by sa2_recover and are forgotten by sa2_scene_unload. The ID pass (sa2_pick_request) ignores the style.
 */
#define SA2_STYLE_CHANNEL_FILL 0u
#define SA2_STYLE_CHANNEL_OUTLINE 1u
#define SA2_STYLE_INDEX 0x0Fu /* a byte's palette entry */
#define SA2_STYLE_DIM 0x80u /* fill bytes only: the piece belongs to the dim set */
#define SA2_STYLE_SLOTS 4u
#define SA2_STYLE_PALETTE 16u
#define SA2_STYLE_TINT_FILL 1u
#define SA2_STYLE_TINT_OUTLINE 2u
#define SA2_STYLE_DIM_NONE 0u
#define SA2_STYLE_DIM_MARKED 1u
#define SA2_STYLE_DIM_UNMARKED 2u
#define SA2_STYLE_OUTLINE_MAX 0.5f
#define SA2_STYLE_MATRIX_MAX 8.0f

typedef struct sa2_view_style {
    uint32_t struct_size; /* sizeof(sa2_view_style); 316 on x64 */
    uint32_t tint_mode; /* SA2_STYLE_TINT_* flags */
    uint32_t dim_mode; /* SA2_STYLE_DIM_* */
    float dim_amount; /* 0 <= dim_amount <= 1 */
    float outline_width; /* 0 <= outline_width <= SA2_STYLE_OUTLINE_MAX */
    float palette[64]; /* SA2_STYLE_PALETTE RGBA entries, each channel 0..1; entry 0 is never drawn */
    float matrix[9]; /* row-major 3x3 output colour matrix; |entry| <= SA2_STYLE_MATRIX_MAX */
    uint32_t reserved; /* must be 0 */
} sa2_view_style;
#ifdef __cplusplus
static_assert(sizeof(sa2_view_style) == 316, "sa2_view_style requires the x64 ABI");
#endif

SA2_API int32_t sa2_style_set(sa2_context* ctx, uint64_t revision, const uint8_t* piece_style, uint32_t channel);
SA2_API int32_t sa2_view_style_set(sa2_context* ctx, uint32_t slot, const sa2_view_style* style);

/* ABI 8: the composite frame (ADR-D02 build step B18a, a candidate behind the shell's --d02; row F27 of the feature
 * matrix). Additive: every ABI 7 call, struct and behaviour is unchanged.
 *
 * sa2_frame_submit_panes draws up to SA2_PANES_MAX panes of one engine revision into one target as one frame: one
 * clear of the whole target, one draw per pane with its viewport and scissor at the pane's rectangle, one present on
 * a surface, one frame ID, one report and one presented row. `frame` is validated as sa2_frame_submit validates it
 * and names the target, surface, sequence, engine revision, publication generation and stamps; its view,
 * view_generation, resource_set and camera_matrix must equal the first pane's, so its report and presented row read
 * as a one-view frame's. Each pane names its view, view generation, resource set (0: every slot drawn, nothing
 * pickable), rectangle in target pixels, camera matrix and style slot (0: no style, else 1 to SA2_STYLE_SLOTS); its
 * shape is its view's (ABI 6). Rectangles lie inside the target and do not overlap; a count of 0 or above
 * SA2_PANES_MAX, an empty rectangle, nonzero flags or a non-finite camera is SA2_E_INVALID_ARGUMENT. Two panes may
 * name one view (a held split with two styles). Every pane's resource set must be active, stamped for its view and
 * view generation, and prepared at the frame's engine revision; otherwise the whole frame is refused
 * (SA2_E_WRONG_STATE) and nothing is drawn. Each view keeps its own publication generation: the first pane's set is
 * the frame input's and carries the frame's, another pane's set may carry another.
 *
 * sa2_frame_panes returns a produced composite frame's panes in order: each one's view, view generation and vertex
 * evaluations; the frame report's view_vertex_evals is their sum. A frame that is not a retained composite frame, or
 * that has not produced yet, answers SA2_E_WRONG_STATE.
 *
 * sa2_pick_request on a composite frame takes target coordinates and draws the ID pass with the camera, resource set,
 * shape and viewport of the pane that contains the point; a point in no pane is SA2_E_INVALID_ARGUMENT. The reply
 * names that pane's view, view generation and its set's publication generation, and only a newer generation of
 * that view makes it stale.
 */
#define SA2_PANES_MAX 4u

typedef struct sa2_pane {
    uint64_t view;
    uint64_t view_generation;
    uint64_t resource_set;
    int32_t x; /* the rectangle's left edge, in target pixels */
    int32_t y; /* its top edge */
    uint32_t width;
    uint32_t height;
    float camera_matrix[16]; /* column-major, as sa2_frame_input's */
    uint32_t style; /* 0: no style; else a style slot 1..SA2_STYLE_SLOTS (ABI 7) */
    uint32_t flags; /* must be 0 */
} sa2_pane;
#ifdef __cplusplus
static_assert(sizeof(sa2_pane) == 112, "sa2_pane requires the x64 ABI");
#endif

typedef struct sa2_pane_row {
    uint32_t struct_size; /* sizeof(sa2_pane_row); 32 on x64 */
    uint32_t pane; /* the pane's index in the frame */
    uint64_t view;
    uint64_t view_generation;
    uint64_t vertex_evals;
} sa2_pane_row;
#ifdef __cplusplus
static_assert(sizeof(sa2_pane_row) == 32, "sa2_pane_row requires the x64 ABI");
#endif

SA2_API int32_t sa2_frame_submit_panes(sa2_context* ctx, const sa2_frame_input* frame, const sa2_pane* panes, uint32_t count, uint64_t* out_frame_id);
SA2_API int32_t sa2_frame_panes(sa2_context* ctx, uint64_t frame_id, sa2_pane_row* rows, uint32_t capacity, uint32_t* out_count);

#ifdef __cplusplus
}
#endif

#endif /* SA2_INTEROP_H */
