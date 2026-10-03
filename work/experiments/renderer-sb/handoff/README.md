# S-B D3D12 resource handoff probe

Standalone item 4 of E-2.4-01. C++20 and the Windows SDK only; no framework,
Agility SDK, shader compiler, model data, window or swap chain. RGBA8 texels
store the low 32 bits of the frame sequence in little-endian R/G/B/A bytes.
Sequences and fence values start at 1 and remain 64-bit. Default: 1,000 frames,
three 1024 x 1024 textures, the first hardware adapter in DXGI's high-performance
ordering. `--warp` explicitly selects WARP. The same adapter LUID is checked
on both sides of every exchange.

## Build and CPU acceptance

From the repository root in PowerShell (no administrator rights):

```powershell
python work/experiments/renderer-sb/handoff/check_handoff.py

$build = "$env:TEMP\m600-sb-handoff-build"
& .\work\experiments\renderer-sb\handoff\build.cmd $build
& "$build\sb_handoff.exe" --selftest
```

`build.cmd` locates Visual Studio with `vswhere`, calls `vcvars64.bat`, prints
compiler/CMake/Ninja versions, and builds with Ninja in the optional directory.
Its default directory is `%TEMP%\m600-sb-handoff-build`. It prefers the pinned
`tools/.venv/renderer-spike/Scripts/cmake.exe` and `ninja.exe` when present;
otherwise it puts the installed Visual Studio CMake/Ninja copies on `PATH`.
It installs and downloads nothing.

`check_handoff.py` uses a plain `mkdir` under the system temp directory, builds
the entire native source, runs `--selftest` with a 30-second limit, parses its
JSON, then removes that invocation's build directory. It leaves no build or
fixture files in the repository. The fixtures test RGBA byte order and wraparound,
every-texel verification with padded rows, corrupt and stale frames, three-slot
ownership and 64-bit fence boundaries, strict child handle parsing, nearest-rank
p99, JSON escaping and exit-status rules. `--selftest` creates no device.

The check sets `M600_HANDOFF_CPU_CHECK=1` for its `build.cmd` subprocess and uses
MSVC's installed NMake generator. This is a build-only fallback: both installed
Ninja copies stalled on a trivial `cmd /c echo` job in the implementation
sandbox. The regular `build.cmd` invocation uses Ninja. Both generators compile
the same CMake target and all four GPU paths; this fallback is printed in the
acceptance output and does not substitute a fixture executable for native source.

An earlier generated build was blocked by Windows Code Integrity at launch with
error 4551 (Enterprise signing requirements). A later build launched successfully
and passed the CPU check. A blocked launch is never counted as a passing fixture
check. No security setting or signing policy is changed by the build or check
scripts.

**The acceptance result is source/fixture evidence only.** It does not establish
GPU handoff, debug-layer cleanliness, driver compatibility, framework interop
or performance. Those require the following owner-machine runs.

## Owner-machine runs

After building as above:

```powershell
& "$build\sb_handoff.exe" --mode all --iterations 1000 --debug-layer --out "$env:TEMP\sb-handoff-hardware.json"
$LASTEXITCODE

# Optional software-adapter comparison; separately identified in the JSON.
& "$build\sb_handoff.exe" --mode all --warp --debug-layer --out "$env:TEMP\sb-handoff-warp.json"

# A focused rerun, or repeat --mode to request a subset.
& "$build\sb_handoff.exe" --mode second-device --out "$env:TEMP\sb-handoff-second-device.json"
```

The debug layers must already be installed for `--debug-layer`; their absence
is a failure with the HRESULT, and this program does not install Windows
features. Runs without the flag still drain and release resources, but report
the live-object check as `not_requested`.

| Mode | What an owner-machine pass proves |
| --- | --- |
| `same-device` | One D3D12 device, a producer compute queue and consumer direct queue exchange three textures. Every readback texel matches its frame sequence. |
| `second-device` | A child process creates its own D3D12 device on the parent's exact adapter LUID, opens one shared heap and two shared fences, recreates matching placed textures and verifies every frame. |
| `d3d11-consumer` | A distinct D3D11 device on that adapter opens three D3D12 committed shared textures with `ID3D11Device1::OpenSharedResource1` and both fences with `ID3D11Device5::OpenSharedFence`. Its context4 waits, copies to staging textures, signals and flushes; the CPU checks every texel. |
| `resize` | The same-device protocol drains both queues and verifies all pending frames at each 100-frame boundary that has more work, releases the ring and cycles sizes 1024 x 1024, 640 x 480 and 1280 x 720. Fence numbering continues across recreation. The final outstanding frames are drained and verified before teardown. |

Each mode records a second `D3D12CreateDevice` call on the same adapter in that
process, comparing the returned `ID3D12Device` pointers directly. The child
records its own comparison too. This **measures** the documented per-process,
per-adapter singleton behaviour; it never treats a second in-process call as
an independent D3D12 device. The D3D11 consumer's D3D12 singleton field is null.

## Protocol and framework reuse

For sequence `n`, the slot is `(n - 1) % 3`, its previous use is `max(n - 3, 0)`,
and both fences use `n` without packing the slot into the value:

1. Before reusing a slot, the CPU observes `free >= n - 3` and verifies its old
   readback. This also protects the command allocator, clear source and readback
   storage. The producer queue issues `Wait(free, max(n - 3, 0))`.
2. On the compute queue, `ClearUnorderedAccessViewFloat` fills a private RGBA8
   scratch texture with each sequence byte divided by 255. The scratch transitions
   `COMMON -> UNORDERED_ACCESS -> COPY_SOURCE -> COMMON`; the transition orders
   the clear before the copy into the ring texture. The ring texture transitions
   `COMMON -> COPY_DEST -> COMMON`, then the producer signals `ready = n`.
3. The consumer queue waits for `ready = n`, copies the texture into a readback
   buffer with `COMMON -> COPY_SOURCE -> COMMON`, then signals the free fence.
   D3D11 uses its implicit state management between the same `COMMON` boundaries.
4. Same-device and D3D11 readbacks are verified in chronological order before
   slot reuse, with the last up-to-three frames checked at the final drain.
   The child verifies each copy before signalling `free = n`; this lets the
   parent associate a free-fence completion with successful child verification.

Readback buffers stay `COPY_DEST`. All handed-over textures use
`ALLOW_SIMULTANEOUS_ACCESS`, one mip, one sample and
`R8G8B8A8_UNORM`. Textures are always `COMMON` at a cross-queue/API/device
handoff. A framework that samples them must transition to its appropriate
shader-resource state after waiting and return to `COMMON` before signalling
free. Keep both queues and fences alive until all their submitted work is drained.

On the same device, pass the existing texture and fence objects to the consumer;
the framework owns its device and this producer owns only its queue and ring.
This executable owns that device itself as a standalone test. For a second
device, this test shares a `D3D12_HEAP_FLAG_SHARED` heap with three nonoverlapping
placed textures, plus `D3D12_FENCE_FLAG_SHARED` ready/free fences. The D3D11
variant shares each committed resource instead, because D3D11 opens resources,
not D3D12 heaps. Nothing shares scratch textures, clear descriptors, readback
buffers or command allocators.

D3D11 needs a render-target-capable shared texture for `OpenSharedResource1`,
so this path sets `D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET` on the committed
shared ring textures. Without it, the packet's owner-machine run failed with
`E_INVALIDARG` (`0x80070057`). The shared-heap path uses
`D3D12_HEAP_FLAG_ALLOW_ONLY_NON_RT_DS_TEXTURES` and keeps its textures non-RT.

The child protocol duplicates NT handles with `DuplicateHandle`, passes only
five inheritable copies through `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, and uses
decimal arguments `--child heap,ready,free,report,cancel --luid <uint64>`,
plus the ring extent as `--width` and `--height`.
The last two handles are an anonymous report pipe and cancellation event.
These arguments are internal; let the parent launch the child. A setup report
must arrive before the producer submits work. The child never enqueues a GPU
wait before the ready value exists, permitting cancellation and drain on a
parent failure. All CPU waits are bounded at 30 seconds. If cancellation cannot
stop the owned child, the parent terminates it and records a failure.

Godot's RenderingDevice and Qt's QRhi smoke tests can reuse the sequence pattern,
ring arithmetic, two-fence ownership protocol and `COMMON` boundaries. They
must still exercise their actual import/native-handle APIs, display the texture,
and independently check resize and teardown; a pass here does not prove either
framework's smoke test.

## Result interpretation

`--out` writes `magic600-sb-handoff-v1` JSON. Without it, GPU runs print the JSON
to stdout. Each mode contains status/reason, requested/submitted/verified frame
counts, the first failing sequence (null for setup failures and passes), parent
and consumer adapter LUIDs, nullable singleton comparisons, errors, resize count,
and timings in milliseconds (samples, mean and nearest-rank p99). Timing samples
are wall time from just before recording/submitting the producer clear to CPU observation
and verification of that frame. They include ring pacing, CPU work and process
effects; they are information only and do not measure GPU execution time or
the renderer's frame-time gate.

Exit 0 means every requested mode passed or was explicitly unsupported **before
any frame submission**, with a reason. Missing adapter/API/interface capabilities
may be unsupported. Invalid arguments, access errors, bad resource parameters,
device removal, timeouts, texel mismatches and runtime failures remain failures.
Treat `unsupported` as an open interop requirement, never as a verified handoff.

A drain that fails without device removal does not show that the GPU has
finished, so the program then releases nothing. The failing drain itself, before
any unwinding, records the failure, writes the JSON up to and including that
mode, cancels a running child and ends its own process with exit code 3; later
modes do not run. If recording the failure fails, for example for lack of
memory, the process still ends this way. The report is written on a separate
thread, and the process ends after at most 10 s even if the report cannot be
delivered, for example to a pipe that nobody reads. Windows reclaims the
process's GPU objects only after the GPU has stopped using them. A child ends
the same way after sending its final report; the parent then records the
failure and exits 1. After device removal all GPU work has ended, so teardown
continues and the mode fails.

`--inject-unconfirmed-drain producer|consumer` makes every drain of that role
fail this way, to test that path. `consumer` covers the D3D12 consumer queue, the
D3D11 consumer and, in `second-device`, the child process. Such a run always
fails.

With `--debug-layer`, resources, command lists, allocators, queues and fences are
released after their drains succeed, then `ID3D12DebugDevice::ReportLiveDeviceObjects`
runs with `DETAIL | IGNORE_INTERNAL`. Only the device and its diagnostic interfaces
are deliberately retained for reporting. Unexpected live-object messages and
D3D12 debug errors fail the mode; the child reports these too. The JSON records
the number of queryable report messages; an empty report cannot establish
cleanliness and fails the check. No GPU result is
pre-populated by the CPU acceptance check.
