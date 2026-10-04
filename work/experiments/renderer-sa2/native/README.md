# SA2 native producer (ABI 1)

This directory implements the D3D12 producer for the E-2.4-02 level-1 Godot
smoke test. It uses the device and DIRECT queue supplied by Godot. It creates
neither a second device nor shared handles. No geometry, window or shaders are
involved. The fixed interface is [include/sa2_interop.h](include/sa2_interop.h);
the image contract is [../code_layout.json](../code_layout.json).

## Build and CPU acceptance

Run from the repository root on Windows with Visual Studio's x64 C++ tools and
the Windows SDK installed:

```powershell
python work/experiments/renderer-sa2/native/check_native.py
```

The check builds Release with NMake into a freshly created `build-check-<pid>`
directory beside this README, inside the repository's ignored `work/` tree
(Application Control can refuse unsigned executables in the system temp folder).
It runs only `--cpu`, validates the JSON and final `selftest: ok` line, and removes
that directory. It passes the three code vectors directly from `code_layout.json`.
It creates no tracked repository output and starts no D3D12 device. CPU checks cover
every ABI export, invalid arguments and error-buffer truncation, CRC and code
vectors, exact images at three sizes, both-corner decode and corrupted-block
rejection, text masking, state mappings and JSON escaping.

For binaries that the Godot run script can keep and load:

```powershell
$build = 'work/experiments/renderer-sa2/native/build'
& work/experiments/renderer-sa2/native/build.cmd $build
& "$build/sa2_selftest.exe" --cpu --out "$build/cpu.json"
```

`build.cmd [build-dir]` locates Visual Studio with `vswhere`, calls `vcvars64.bat`,
and uses the pinned CMake/Ninja under `tools/.venv/renderer-spike/Scripts` when
available, otherwise Visual Studio's copies. It prints the actual tool versions.
The default generator is Ninja; setting `$env:M600_SA2_CPU_CHECK = '1'` selects
NMake for environments where Ninja stalls. Use a fresh build directory when
switching generators. Both `sa2_interop.dll` and `sa2_selftest.exe` are placed
directly in the build directory. All three targets use the static CRT (`/MT`)
and C++20, with Windows SDK libraries only.

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

The fixed header declares **19** exported functions, despite the packet's count
of 21. The surface test checks every declared export by its ABI name; no extra
functions are invented. MSVC accepts the header's `sa2_debug_counts` type/function
name overlap; implementation type references use `struct sa2_debug_counts`.
Both the header and `code_layout.json` remain unchanged.
