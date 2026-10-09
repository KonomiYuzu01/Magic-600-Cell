# SA2 native producer (ABI 2)

This DLL retains the ABI 1 code-image producer and adds S-B's W1 to W4 drawing
method for the Godot and Qt level-2 hosts. It uses the device and DIRECT queue
supplied by the framework. The scene path uses S-B feature `none`, MSAA 1, full
259,800 labelled slots and 600 instances of 30,480 vertices. The interface is
[include/sa2_interop.h](include/sa2_interop.h); the ABI 1 image contract remains
[../code_layout.json](../code_layout.json).

## Build and CPU acceptance

Run from the repository root on Windows with Visual Studio's x64 C++ tools and
the Windows SDK installed:

```powershell
python work/experiments/renderer-sa2/native/check_native.py
```

The check builds Release with NMake into a plain-mkdir directory
`native/build-check-<pid>` inside the repository's ignored work tree, because
Application Control can refuse unsigned executables in system temp.
It runs only `--cpu`, validates the JSON and final `selftest: ok` line, and removes
that directory. It passes the three code vectors directly from `code_layout.json`.
It creates no tracked repository output and starts no D3D12 device. CPU checks cover
every ABI export, invalid arguments and error-buffer truncation, CRC and code
vectors, exact images at three sizes, both-corner decode and corrupted-block
rejection, text masking, state mappings and JSON escaping. The six ABI 2 CPU groups check
scene arguments, the shared lifecycle validators, S-B's readers and asset counts,
synthetic native/trace records, exclusive file creation and identity/truncation.
Python independently recomputes the DLL and all four shader SHA-256 values.
The CPU self-test must finish within 60 seconds.

For binaries that the Godot run script can keep and load:

```powershell
$build = 'work/experiments/renderer-sa2/native/build'
& work/experiments/renderer-sa2/native/build.cmd $build
& "$build/sa2_selftest.exe" --cpu --out "$build/cpu.json"
```

`build.cmd [build-dir]` locates Visual Studio with `vswhere`, calls `vcvars64.bat`,
and uses the pinned CMake/Ninja under `tools/.venv/renderer-spike/Scripts` when
available, otherwise Visual Studio's copies. It prints the actual tool versions, including the Windows SDK DXC.
The default generator is Ninja; setting `$env:M600_SA2_CPU_CHECK = '1'` selects
NMake for environments where Ninja stalls. Use a fresh build directory when
switching generators. Both `sa2_interop.dll` and `sa2_selftest.exe` are placed
directly in the build directory. All native targets use the static CRT (`/MT`) and C++20, with Windows SDK
libraries only. DXC compiles S-B's unmodified `draw.hlsl` and `check.hlsl` (which
include `geometry.hlsl`) with S-B's entries/profiles and `-nologo -O3 -Ges`.
The DLL loads only `draw_vs.dxil`, `draw_ps.dxil`, `count_vs.dxil` and
`geometry_cs.dxil`, all next to the DLL. Keep those four files with the DLL;
there is no runtime shader compiler.

## Owner GPU checks

After building, run each device mode, optionally omitting `--debug` for a run
without the debug layer:

```powershell
& "$build/sa2_selftest.exe" --warp --debug --out "$build/warp.json"
& "$build/sa2_selftest.exe" --hardware --debug --out "$build/hardware.json"
```

Or build, run the CPU check, then run both GPU modes in an automatically cleaned
build directory:

```powershell
python work/experiments/renderer-sa2/native/check_native.py --gpu
```

Every report uses `magic600-sa2-native-selftest-v1`, with a name, status
(`pass`, `fail`, `unsupported`) and reason for each check. The executable exits 0
only when all checks pass or have a reason for being unsupported. Keep the
explicit-build JSON files to retain owner GPU evidence; `check_native.py`
validates and deletes its reports with its build directory. `--vectors` accepts
three semicolon-separated rows of `sequence,slot,generation,code`; the acceptance
script supplies these from the immutable JSON, whereas direct invocations use
the matching ABI-1 defaults.

WARP proves the native queue, texture, barrier, fence and teardown protocol on a
software D3D12 adapter. Hardware selects the high-performance adapter and skips
software adapters. Each mode exercises all eight combinations of same/own queue,
legacy/enhanced barriers and created/external textures. Enhanced configurations
are reported unsupported when OPTIONS12 does not support them. Each supported
configuration checks 300 frames, three slots, both decoded corners on every
consumer readback, a rebuild from 256 by 192 to 320 by 240 before frame 150,
full-texel verification every 50 frames and one deliberate verification failure.
It also checks invalid resources, monotonic counters, unsafe waits, releases,
unregister/detach guards, and reference counts across attach/detach. With
`--debug`, error and corruption counts and non-clear-value warning descriptions
must be empty; mismatching-clear-value warnings from external textures are
permitted and counted.

Both GPU modes launch two bounded child probes on their own device:

- `inject-drain` produces five frames with its own queue and sets
  `M600_SA2_INJECT_UNCONFIRMED_DRAIN=1` inside the child. The parent requires exit
  3, exactly `sa2: drain not confirmed; ending the process with exit code 3`
  followed by one LF on stderr, and no output from the code after the drain.
- `device-loss` produces five frames, calls `ID3D12Device5::RemoveDevice`,
  requires `SA2_E_DEVICE_REMOVED` from drain and a failing removed-reason HRESULT,
  then unregisters, releases and detaches successfully. Removal affects only
  the child's self-test device.

The implementation sandbox did not run these GPU checks; the owner-machine
results are in [../RESULT.md](../RESULT.md). Even an owner GPU pass establishes
native interop only: Godot display,
its resource-state tracker, window resize and frame composition still require
the separate Godot smoke test. There are no performance claims here.

ABI 2 adds 16 scene runs per device: W1 to W4, same/own queue and
legacy/enhanced slot barriers. These use a common stand-in device and an
imported three-slot ring. WARP uses 640 x 360 and 40 total frames; hardware uses
2560 x 1600 and 600 total frames. Three preroll and three postroll frames put
trace begin/end inside each run (34 or 594 traced frames). Every traced frame
must have one consecutive zero-based S-B record, the actual full target and
viewport sizes, and a positive process-local VRAM sample unless sampling is
explicitly disabled. W3/W4 require S-B's exact label check to pass; the checker
is unchanged. A frame slower than the label clock skips revisions, which the
checker counts as missing. WARP draws a frame in about a second, so its W3 runs
use 3000 ms turns. W4's label clock is fixed at S-B's 190 ms, so a WARP W4 run
is reported unsupported when skipped revisions are its only failure and a
traced step is longer than the clock; any mismatch, binding error or late
adoption still fails. Hardware runs use S-B's 190 ms and must pass.

The same device also gets one geometry check: all nine reference camera/pose
comparisons at aspect 1.6 and the per-cell invocation counts, using the DLL's
own 2560 x 1600 targets. A negative W3 run uses a 1000 ms turn (3000 ms on WARP) and paces its first
21 traced frames so S-B's fixed turn-20 `corrupt-label` fault is actually applied;
it requires CHECK_FAILED, label status `fail`, `injection_applied: true` and
nonzero mismatches. A separate W1 run with NO_VRAM requires a null peak and zero
samples. The scene tests check state/order guards, exclusive outputs, debug
errors/corruption and restored references too. These offscreen checks establish
neither framework display nor PresentMon gate performance.

## States, references and fence protocol

The self-test uses `SA2_STATE_ALL_SHADER_RESOURCE` before and after each write:
legacy `PIXEL_SHADER_RESOURCE | NON_PIXEL_SHADER_RESOURCE`, or the enhanced
generic `SHADER_RESOURCE` layout. This keeps the texture available to either
shader stage. The stand-in consumer transitions to `COPY_SOURCE` for readback
and restores that same handover state. External textures match Godot's format
family and clear value: RGBA8 typeless, `ALLOW_RENDER_TARGET`, a UNORM RTV and an
optimized clear value of (0, 0, 0, 0). Imported textures are RGBA8 UNORM without
an optimized clear value.

Godot's actual tracker is authoritative for the Godot harness's configuration.
`state_before_write` must describe the slot at the write point;
`state_after_write` must equal the state Godot expects when control returns.
The DLL does not alter Godot's tracker. The supplied Godot 4.7.2 source facts
give a legacy imported texture an initial tracked `RENDER_TARGET` state; the
enhanced render-graph tracker starts at `UNDEFINED` and retains the last usage.
An imported resource must be brought into the appropriate tracked state before
Godot uses it. Do not assume that an import always expects `COMMON`, or that the
self-test's shader-resource handover establishes Godot's initial import state.
ABI 1 accepts only the five declared `SA2_STATE_*` states, not `UNDEFINED`.
The harness must establish the initial usage before using this steady-state
producer interface, and confirm that setup with the Godot smoke test.

`SA2_BARRIERS_MATCH_GODOT` resolves once at attach using OPTIONS12. Enhanced
mode requires `ID3D12GraphicsCommandList7` and uses the generic layouts, never
`DIRECT_QUEUE_*` layouts. Enhanced texture creation uses
`ID3D12Device10::CreateCommittedResource3` to establish the requested layout.
Production transitions use NONE/NO_ACCESS into RENDER_TARGET/RENDER_TARGET and
back to NONE/NO_ACCESS; diagnostic copies use NONE/NO_ACCESS into COPY/COPY_SOURCE
and back. Equal states skip the barrier. Invalid state values are rejected.

All ABI calls run on the render thread except the runtime's message callback.
Attach takes its own references to the supplied device and queue. Probe keeps
none. A texture from `sa2_create_texture` has one reference owned by the context;
Godot's `texture_create_from_extension` neither adds nor releases a reference.
Registering it with `godot_owned=0` adds no reference. An external texture from
Godot is registered with `godot_owned=1`; registration adds a reference and
unregistration releases exactly that reference. The DLL never releases a
reference supplied by Godot.

For frame `f`, the harness calls:

1. `sa2_signal_godot_free(f - 1)` after the previous Godot submission.
2. `sa2_produce(f % 3, f, sequence, generation)`; the slot's prior command list
   is CPU-waited with `wait_timeout_ms` before allocator reuse.
3. `sa2_godot_wait_ready(f)`, then `sa2_mark_shown(slot, f)` before consumer work.

In same-queue mode, queue order serializes all work and wait-ready is a no-op.
In own-queue mode, the producer waits for the slot's last shown frame on the free
fence; it then writes and signals ready to `f`. Godot's queue waits for that ready
value before consuming. Produce fails before recording when the shown frame
has not been signalled free. Wait-ready fails when its requested frame has never
been successfully signalled ready. Produce frames strictly increase, free values
do not decrease, and shown values do not decrease per slot. Fence value
`UINT64_MAX` is rejected because D3D12 reserves its completion meaning for
device removal.

Verification stalls. In own-queue mode it first signals and CPU-waits a marker on
Godot's queue, so it can safely inspect a just-consumed slot even before the next
frame signals free. It then copies on the producer queue and checks every texel.
Its readback buffer remains context-owned if a bounded wait fails. Verify never
queues a wait for a free value that has not been signalled.

Resize and teardown require a successful explicit `sa2_drain` after the last
Godot use. The context uses a separate increasing drain fence per queue. Any new
queue operation or shown-frame mark invalidates the drain confirmation;
unregister, release and detach conservatively require a current context-wide
confirmation. For imports, also free every Godot RID before releasing its native
texture. Unregister every slot, release every created texture (the returned
reference count should be 0), then detach. Detach unregisters the callback and
releases the DLL's objects and device/queue references. Device removal allows
release without a confirmed drain, but drain returns `SA2_E_DEVICE_REMOVED` and
does not report a false confirmation. A live-device unconfirmed drain writes
the fixed stderr line with `WriteFile` and calls `TerminateProcess` with code 3.

ABI 1 has 19 declared exports (the level-1 packet counted 21); ABI 2 adds eight,
for 27. The surface test checks every declared name. MSVC accepts the header's
`sa2_debug_counts` type/function name overlap; implementation type references
use `struct sa2_debug_counts`. `code_layout.json` remains byte-identical.

## Scene host call order

The same API serves either framework; `godot` in the ABI 1 fence names denotes
the framework's queue. Call from its render/context thread:

1. Probe and attach the framework's own device and DIRECT queue with the
   handover states that its tracker expects.
2. Create/import and register all three equally sized slots, then `scene_load`
   with `trace_ms` set to the planned trace length.
   The DLL creates all fixed scene resources during load. Static copies execute
   with the first scene submission; their staging buffers stay alive until
   unload. Resize requires drain, unload, ring rebuild and a new scene load.
3. Optionally run `scene_geometry_check` before the trace, with an existing
   output directory. It writes `geometry.json`; it never touches a slot. Its
   bounded wait uses `wait_timeout_ms`. Drain again after the check before
   teardown; a successful geometry wait is not an explicit drain confirmation.
4. Produce preroll frames. For each frame `f` starting at 1, call
   `sa2_signal_godot_free(ctx, f - 1)`,
   `sa2_scene_produce(ctx, f % 3, f)`, `sa2_godot_wait_ready(ctx, f)`;
   submit the framework's draws, then `sa2_mark_shown(ctx, f % 3, f)`.
5. After preroll, call `sa2_scene_trace_begin`. Keep the same per-frame order
   for the duration. W3's clock starts at that call; W4 uses the same 190 ms
   label clock without turn animation. W2/W3 rotate once per produced frame,
   including preroll, exactly as S-B does.
6. Call `sa2_scene_trace_end` within `trace_ms` of trace begin, then `sa2_drain` after the last framework use.
   `sa2_scene_write_run` writes `trace.jsonl` and `native.json` into an existing
   fresh directory. A failed label check still writes both and returns
   `SA2_E_CHECK_FAILED`. Existing outputs return `SA2_E_IO` and are preserved.
   The host merges `native.json` into its own `run.json`; the DLL never writes
   `run.json`. Postroll is permitted, but requires another drain before writing.
7. Unload the scene, unregister slots, free framework wrappers/RIDs, release
   DLL-created textures and detach. A loaded scene prevents detach. Removal
   permits scene unload without drain just as it permits ABI 1 releases.

New context calls retain the ABI 1 last-error convention. A failed command
recording poisons the scene to prevent reuse of speculative resource states;
drain and unload before loading again. Geometry timeout retains every resource
and permits a bounded wait/retry; it never unwinds in-flight resources.

## Reuse and record choices

`probe.cpp` and `edges.cpp` are compiled unchanged with S-B's repository-root
definition. Only their unused standalone `main` symbol is renamed at compilation
so the common reader library can link into the self-test. The unused `runGpu`
symbol is satisfied by a DLL-local stub that throws a clear unsupported message.
No DLL path calls S-B's `exeDir()` or `buildIdentity()`.

`gpu.cpp` supplies the ported baseline resources, ten-parameter root signature,
PSOs, bindings, camera/turn logic, label upload/use/copy records and geometry
checks. Its device creation, window, swap chain, presents, condition sampling,
features and MSAA variants are omitted for the framework-hosted path. Internal
resources retain S-B's legacy barriers; only slot handovers use the selected
legacy/enhanced API. Unlike S-B, default-heap buffers are created in COMMON:
the runtime ignores any other initial state for buffers and the debug layer
warns about it; the first use promotes them implicitly. No S-B source or
shader was edited.

The ring is fixed at three equally sized slots per load, with three constant and
upload entries protected by slot completion. W3/W4 allocate the whole run's
preserved label readback at load (S-B's rule: `ceil(trace_ms / turn_ms) + 2`
copies), so nothing is allocated inside the trace. Trace frames start at zero independently of ABI fence frames
starting at one. `native.json` uses string queue/barrier names (`same`/`own`,
`legacy`/`enhanced`). Identity uses sorted basenames and exact on-disk bytes,
located through `GetModuleHandleExW(FROM_ADDRESS)` in this DLL. The geometry
record's `build_identity` is SHA-256 of the canonical identity JSON. Reference
validation failures also write a failed geometry record; unrecorded counts are
null and marked `not-checked`, never presented as measured counts.

The implementation sandbox runs source/fixture checks only. Every ABI 2 GPU
check, framework capture and day-7 gate remains for the owner's machine.
