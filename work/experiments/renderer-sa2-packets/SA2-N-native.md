# Packet SA2-N: the native D3D12 producer DLL for the Godot smoke test (E-2.4-02 level 1)

Run from the `claude/renderer-sb` checkout:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-sa2-packets/SA2-N-native.md`

The parallel packet SA2-G (`SA2-G-godot.md`) owns the Godot project, the run script and the project checks. This packet owns only `native/` (the DLL, its self-test, its build and its check). Both packets implement the same committed interface, `work/experiments/renderer-sa2/native/include/sa2_interop.h` (ABI version 1), and the code layout in `work/experiments/renderer-sa2/code_layout.json`. Neither file may change.

## 1. Goal and acceptance
- Goal: `sa2_interop.dll`, a small native Direct3D 12 producer that the Godot C# harness loads into Godot's process. It writes a sequence-numbered code image into textures on Godot's own `ID3D12Device`, on Godot's queue or on its own queue with fences, and hands the textures over in the state Godot tracks. Plus `sa2_selftest.exe`, which proves the DLL without Godot: on the CPU in the sandbox, and on WARP and the owner's GPU afterwards.
- Acceptance check: `python work/experiments/renderer-sa2/native/check_native.py` exits 0. It builds out of tree and runs `sa2_selftest.exe --cpu` (no device).
- Done when: every function in the header is implemented with the documented semantics; the self-test covers the CPU items in section 6 and the GPU items behind `--warp` and `--hardware`; `README.md` gives the owner's commands.
- Non-goals: the Godot project, PresentMon, geometry, NVIDIA features, the Agility SDK, a second device, shared handles.

## 2. Actual problem and reproduction
- E-2.4-02 level 1 needs a changing, sequence-numbered texture produced by D3D12 work and displayed through Godot's own D3D12 backend, with device ownership, synchronisation, resize and teardown working. The S-B handoff test (`work/experiments/renderer-sb/handoff/`, result in its `RESULT.md`) proved the fence protocol between our own devices and queues. Inside Godot, the device and the main queue belong to Godot, and Godot tracks every texture's state itself.
- The open question (PR #43, section 7): does an imported `ID3D12Resource` need to arrive in RENDER_TARGET state? The source facts below say that it must arrive in the state Godot's tracker holds. The smoke test measures this.
- Godot 4.7.2 source facts (read from tag `4.7.2-stable`; runtime confirmation is what the smoke test is for):
  - `RenderingDevice.get_driver_resource()` on the D3D12 driver returns `ID3D12Device*` (LOGICAL_DEVICE), `IDXGIAdapter*` (PHYSICAL_DEVICE), the main `ID3D12CommandQueue*` (COMMAND_QUEUE, a DIRECT queue) and `ID3D12Resource*` (TEXTURE). It adds no reference.
  - `texture_create_from_extension` keeps the raw pointer without AddRef. Its legacy-barrier tracking starts at RENDER_TARGET, and freeing the RID never releases the resource.
  - Godot uses enhanced barriers when `D3D12_FEATURE_DATA_D3D12_OPTIONS12.EnhancedBarriersSupported` is true, with the generic layouts (`SHADER_RESOURCE`, `COPY_SOURCE`, `RENDER_TARGET`, …), never the `DIRECT_QUEUE_*` ones. Its render-graph tracker starts every texture at layout UNDEFINED and keeps the last usage across frames.
  - Textures Godot creates with `TEXTURE_USAGE_COLOR_ATTACHMENT_BIT` have the typeless family format (`DXGI_FORMAT_R8G8B8A8_TYPELESS` for `R8G8B8A8_UNORM`), `ALLOW_RENDER_TARGET`, and an optimized clear value of black (0, 0, 0, 0) in `R8G8B8A8_UNORM`. With enhanced barriers they start at layout UNDEFINED, otherwise at `COPY_DEST`.
  - Godot's info-queue callback is registered with `D3D12_MESSAGE_CALLBACK_IGNORE_FILTERS` when the engine runs with `--gpu-validation`.

## 3. Environment and versions
- Base: branch `claude/renderer-sb` (this worktree's HEAD).
- Owner's machine: Windows 11 (build 26200), NVIDIA GeForce RTX 4070 Laptop GPU (driver 616.92) plus an integrated AMD Radeon 610M. Visual Studio Community 2026 (MSVC 14.51), Windows SDK 10.0.26100.0. CMake 4.4.3 and Ninja 1.13.2 in `tools/.venv/renderer-spike/Scripts/` (not present in this worktree). Godot 4.7.2 .NET runs on the owner's machine only.
- C++20, Windows SDK only: `d3d12`, `dxgi`, `dxguid`. Link the static CRT (`/MT`) so that the DLL needs no runtime DLL inside Godot's process.
- Evidence kind in the sandbox: source/fixture only.

## 4. Necessary source and evidence
- The interface: `work/experiments/renderer-sa2/native/include/sa2_interop.h`. Read all of it; its comments are the specification.
- The code layout: `work/experiments/renderer-sa2/code_layout.json` (bit order, CRC, colours, decode rule and test vectors).
- Working patterns on this machine: `work/experiments/renderer-sb/handoff/` (`build.cmd` with its NMake fallback for the sandbox, `CMakeLists.txt`, `check_handoff.py` with its temp-directory and timeout rules, `src/handoff.cpp` for the fence waits and the drain that ends the process with exit 3).
- Packet E-2.4-02: `docs/progress/1.0/packets/renderer/E-2.4-02-sa2-godot.md` (level 1).

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| — | none | — | — | — |

## 6. Constraints and owned files

Owned files (all under `work/experiments/renderer-sa2/native/`): `CMakeLists.txt`, `build.cmd`, `check_native.py`, `README.md`, `src/**`. Change nothing else. In particular, `include/sa2_interop.h` and `../code_layout.json` stay byte-identical. If the header cannot be implemented as written, stop and say why in the final message instead of changing it.

Build:
- `build.cmd [build-dir]`:
  - Works like `work/experiments/renderer-sb/handoff/build.cmd`: `vswhere` with `call "%VSWHERE%"` inside `for /f`, `vcvars64.bat`, the pinned CMake and Ninja from `tools\.venv\renderer-spike\Scripts\` when present, otherwise the Visual Studio copies.
  - When `M600_SA2_CPU_CHECK=1`, it uses the NMake fallback (Ninja stalls in the sandbox).
  - It prints the tool versions and must have CRLF line endings: cmd's label lookup fails on LF-only batch files.
  - The SA2-G run script calls it with a build directory and expects `<build-dir>\sa2_interop.dll` and `<build-dir>\sa2_selftest.exe` side by side in that directory's root, built Release with `/MT`.
- `CMakeLists.txt`:
  - A static library for the pure parts: code, expected image, text masking and state tables.
  - The `SHARED` library `sa2_interop`, compiled with `SA2_INTEROP_EXPORTS`.
  - The executable `sa2_selftest`, linked to the DLL's import library.
  - `/W4 /permissive- /utf-8 /EHsc`; definitions `UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX`.

DLL behaviour (beyond the header comments):
- **Struct sizes.** `static_assert`: `sizeof(sa2_device_info) == 48`, `sizeof(sa2_config) == 28`, `sizeof(sa2_debug_counts) == 128`. Every function that takes one of these structs checks `struct_size` and returns `SA2_E_INVALID_ARGUMENT` on a mismatch.
- **Errors.**
  - Every failure sets the last error: one line naming the failed call and, for D3D12 or DXGI calls, its HRESULT. A failure without a context sets the DLL-wide last error.
  - Never print pointer values or LUIDs.
  - Whenever a D3D12 call fails, check `GetDeviceRemovedReason()`. If the device is removed, return `SA2_E_DEVICE_REMOVED`, otherwise `SA2_E_D3D12`.
- **Names.** Name every object the DLL creates with `SetName`, starting with "SA2" (for example "SA2 ready fence", "SA2 own queue", "SA2 slot 1 command list", "SA2 imported texture").
- **`sa2_attach`.**
  - The queue must be a DIRECT queue whose `GetDevice` is the given device (compare the `IUnknown` identity); otherwise return `SA2_E_INVALID_ARGUMENT`.
  - Enhanced barriers need `ID3D12GraphicsCommandList7`.
  - `SA2_BARRIERS_MATCH_GODOT` resolves once, at attach, from OPTIONS12.
  - Use two drain fences, one per queue, so that every fence value only increases.
- **Barriers and states.**
  - Legacy barriers map `SA2_STATE_*` to `COMMON`, `RENDER_TARGET`, `PIXEL_SHADER_RESOURCE`, `PIXEL_SHADER_RESOURCE | NON_PIXEL_SHADER_RESOURCE` and `COPY_SOURCE`.
  - Enhanced barriers map them to the layouts `COMMON`, `RENDER_TARGET`, `SHADER_RESOURCE`, `SHADER_RESOURCE` and `COPY_SOURCE`, with the Sync and Access values the header gives.
  - The copy in `sa2_verify_slot` uses `SyncBefore NONE / AccessBefore NO_ACCESS` into `COPY / COPY_SOURCE` and back.
  - Reject any other state value with `SA2_E_INVALID_ARGUMENT`.
- **Image.** The fill clear and the code-block clears follow `code_layout.json` exactly:
  - colours are `k / 255.0f`, so each converts back to `k`;
  - code areas are 64 × 64 texels at both corners;
  - bit i is the 8 × 8 block at column `i % 8`, row `i / 8`.
  Batch the rectangles by colour.
- **Fail-closed checks.**
  - `frame` in `sa2_produce` must increase strictly from call to call. `sa2_signal_godot_free` values must not decrease.
  - `sa2_mark_shown` takes a frame no smaller than the slot's last shown frame.
  - In `SA2_QUEUE_OWN`, `sa2_produce` returns `SA2_E_WRONG_STATE`, before recording anything, when the slot's last shown frame is greater than the last free value signalled. A GPU wait on a value that is never signalled would hang the queue.
  - For the same reason, `sa2_godot_wait_ready` in `SA2_QUEUE_OWN` returns `SA2_E_WRONG_STATE` when `frame` is greater than the last frame passed to `sa2_produce`. A wait on Godot's queue for a value that is never signalled would hang Godot.
  - `sa2_unregister_slot`, `sa2_release_texture` and `sa2_detach` return `SA2_E_WRONG_STATE` unless a confirmed `sa2_drain` covers every command list submitted for the slot or texture. After the device is removed, they skip this requirement and only release.
  - `sa2_register_slot` returns `SA2_E_INVALID_ARGUMENT` unless all of these hold:
    - the resource is a 2D texture on the context's device;
    - it has the given size, at least `SA2_MIN_TEXTURE_PX`;
    - it has 1 mip, 1 array slice and 1 sample;
    - it has `ALLOW_RENDER_TARGET` and format `R8G8B8A8_UNORM` or `R8G8B8A8_TYPELESS`;
    - with `godot_owned` 0, it is a texture made by `sa2_create_texture` in this context.

    The render-target view is always `R8G8B8A8_UNORM`.
- **Callback.**
  - Register `ID3D12InfoQueue1::RegisterMessageCallback` with `D3D12_MESSAGE_CALLBACK_IGNORE_FILTERS`. It may run on any thread: use atomic counters, and a mutex for the distinct IDs and descriptions.
  - Unregister it in `sa2_detach`.
  - Masking replaces every `0x` or `0X` followed by one or more hexadecimal digits with `0x?`.
- **Drain.** `sa2_drain` follows the header exactly. On an unconfirmed drain, write the single line `sa2: drain not confirmed; ending the process with exit code 3` to standard error with `WriteFile` (unbuffered), then call `TerminateProcess(GetCurrentProcess(), 3)`. A drain confirmed while the device is removed is not a confirmation: it returns `SA2_E_DEVICE_REMOVED`.

`sa2_selftest.exe` (one JSON result with `--out <file>`, format `magic600-sa2-native-selftest-v1`, per-check `status` `pass`|`fail`|`unsupported` with a reason; exit 0 only if every check passed or was reported unsupported with a reason):

CPU checks (`--cpu`, no device, under 30 s, prints `selftest: ok` as its last line on success):
- **DLL surface.**
  - `sa2_abi_version() == 1`.
  - `LoadLibrary` of the DLL next to the executable finds all 21 exports by name.
  - Calls without a device fail cleanly: zero handles, a null context, a wrong `struct_size`, a slot out of range and a texture smaller than 128 return `SA2_E_INVALID_ARGUMENT`, and `sa2_last_error(NULL, …)` names the call. Also check truncation to `buffer_size` 1 and 8.
- **Code and image.**
  - CRC-16/CCITT-FALSE of "123456789" is 0x29B1.
  - The code values match the three test vectors in `code_layout.json`. `check_native.py` passes them in, from the JSON, with a `--vectors` option, so the C++ and the JSON cannot drift apart.
  - The expected image for 128 × 128, 129 × 131 and 1280 × 720 has the fill formula outside the code areas, and both corners decode back to the input.
  - The decode rejects an image with one flipped block.
- **Text and tables.**
  - Hex masking.
  - The `SA2_STATE_*` to legacy-state and layout tables, with invalid values rejected.
  - The JSON writer.

GPU checks (`--warp` or `--hardware`, optional `--debug` to enable the debug layer before the device is created; `--hardware` takes the high-performance hardware adapter and skips software adapters):
- **Stand-in Godot.** The self-test creates the device and a DIRECT queue that stands in for Godot's main queue.
- **Configurations.** For each queue mode (`same`, `own`), each barrier API (legacy; enhanced, reported `unsupported` when OPTIONS12 says so) and each texture source:
  - `created`: made by `sa2_create_texture`, with `initial_state` set to the handover state;
  - `external`: created by the self-test like Godot creates them (typeless, `ALLOW_RENDER_TARGET`, black optimized clear value, created in the handover state) and registered with `godot_owned` 1.
- **Each configuration** runs 300 frames on a ring of 3 slots at 256 × 192. The handover state is `SA2_STATE_ALL_SHADER_RESOURCE`, both before and after the write. At frame 150 the ring is rebuilt at 320 × 240: drain, unregister, release, register, and the generation increases.
- **Each frame f**, in the order the Godot harness uses:
  1. `sa2_signal_godot_free(f - 1)`;
  2. `sa2_produce(slot f % 3, f, sequence f, generation)`;
  3. `sa2_godot_wait_ready(f)`;
  4. `sa2_mark_shown`;
  5. the stand-in consumer, on the stand-in queue: a barrier from the handover state to COPY_SOURCE, a copy to a readback buffer, a barrier back (legacy or enhanced, matching the configuration), and a signal;
  6. the CPU checks the consumer's copy: both corners decode to (f, f % 3, generation).

  Every 50 frames, `sa2_verify_slot` must report 0 mismatched texels. One deliberate verify with sequence + 1 must return `SA2_E_VERIFY` with mismatches greater than 0.
- **Negative checks**, each returning the documented status:
  - a non-increasing frame;
  - in own mode, a produce whose slot was marked shown beyond the last free value;
  - in own mode, `sa2_godot_wait_ready` for a frame that was never produced;
  - a texture without `ALLOW_RENDER_TARGET`, and a texture of the wrong size;
  - detach while slots are registered;
  - release of an unknown handle;
  - unregister without a confirmed drain.
- **Reference counts.**
  - The stand-in device's and queue's counts (AddRef/Release probe) are the same before attach and after detach.
  - `sa2_release_texture` reports 0.
  - An external texture's count returns to its value before `sa2_register_slot`.
- **Debug layer** (with `--debug`): error and corruption counts are 0, and `sa2_debug_messages` is empty. The mismatching-clear-value warnings that external textures cause are allowed; they are counted in `mismatching_clear_value`.
- **Child processes** (the self-test starts itself with `--child <name>`):
  - `inject-drain`: with `M600_SA2_INJECT_UNCONFIRMED_DRAIN=1`, a context produces 5 frames and drains. The parent requires exit code 3, the exact stderr line, and no output the child prints only after the drain.
  - `device-loss`: on its own device, the child produces, calls `sa2_remove_device`, and then `sa2_drain` must return `SA2_E_DEVICE_REMOVED`, not exit 3. `sa2_device_removed_reason` gives a failure HRESULT, and unregister, release and detach still succeed. Exit 0.

`check_native.py`:
- Builds into `Path(tempfile.gettempdir()) / "m600-sa2-native-<pid>"`, created with plain `mkdir`. Never use `mkdtemp` or `TemporaryDirectory`: the sandbox's temp ACL breaks them.
- Sets `M600_SA2_CPU_CHECK=1` and runs `build.cmd` with a timeout. On a timeout, it kills only that build's process tree.
- Runs `sa2_selftest.exe --cpu --vectors <from code_layout.json> --out <file>` with a timeout and checks the success marker and the JSON.
- Checks that `build.cmd` is CRLF-only.
- Deletes only its own temp directory and leaves nothing inside the repository.
- `--gpu` (not part of the acceptance) also runs `--warp --debug`, then `--hardware --debug`, on the owner's machine.

`README.md`: build and run commands for the owner, what each GPU check proves, and the state rules the Godot harness relies on: which state the handover uses and why, who owns which reference, and the fence protocol per queue mode.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sa2/native/CMakeLists.txt", "work/experiments/renderer-sa2/native/build.cmd", "work/experiments/renderer-sa2/native/check_native.py", "work/experiments/renderer-sa2/native/README.md", "work/experiments/renderer-sa2/native/src/*"], "acceptance_check": ["python", "work/experiments/renderer-sa2/native/check_native.py"], "stop_condition": "sa2_interop.dll and sa2_selftest.exe build with build.cmd, check_native.py passes (sa2_selftest --cpu), every header function is implemented as documented, the --warp/--hardware checks and both child checks are implemented, and README.md gives the owner's commands"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the acceptance result;
  - what was not verified in the sandbox (every GPU check);
  - any place where the header's wording left a choice, and the choice made;
  - open points.
