# E-2.4-02 S-A2 result card

This is the experiment card ([protocol section 2](../../../docs/progress/1.0/stage-2-experiment-protocol.md)) for packet [E-2.4-02](../../../docs/progress/1.0/packets/renderer/E-2.4-02-sa2-godot.md). It is filled in as results arrive: an item that has not run is listed as open, never estimated. Raw logs and run records stay private; only sanitised summaries are published.

| Field | Content |
|---|---|
| ID | E-2.4-02 |
| Question | Can Godot 4.7 (.NET) host the S-B drawing method through its own Direct3D 12 RenderingDevice inside the selection gate, and what does it constrain (H-09)? |
| Decision it feeds | The day-7 go/no-go (E-2.4-04), the selection (E-2.4-05), and PR #43 section 7 (the framework paper's interop questions). |
| Hypothesis | Level 1: a texture written by our own D3D12 code on Godot's device, imported with `texture_create_from_extension`, displays correctly if it is handed back in the state Godot's tracker holds; RENDER_TARGET is required only where that tracker expects it. |
| Method | Level 1 smoke test: the producer DLL in [native/](native/README.md) (packet [SA2-N](../renderer-sa2-packets/SA2-N-native.md)), loaded by the Godot C# harness (packet [SA2-G](../renderer-sa2-packets/SA2-G-godot.md)). |
| Time box | Window days 3–6 (stage days 5–8), shared with E-2.4-03. |
| Kill criteria | Level 1 not passed by the end of window day 6. |
| Evidence class | Actual Windows/DirectX (native producer self-test, below), valid only for the source and build stated. No performance evidence. |
| Result | Level 1 in progress: the producer DLL passes its self-test on WARP and on the owner's GPU; the Godot smoke test has not run. |
| Decision | Open. |

## Native producer self-test (3 October 2026)

The DLL writes a sequence-numbered code image (layout: [code_layout.json](code_layout.json)) into textures on a device and queue it is given, on that queue or on its own queue with fences, and hands each texture back in a stated state. `sa2_selftest.exe` stands in for Godot: it creates the device and a consumer queue, reads every produced frame back and decodes both code corners.

| Mode | Result |
|---|---|
| `--cpu` (no device) | pass, 11 of 11 checks: ABI, all 19 exports, invalid arguments, error-text truncation, CRC and code vectors, exact images at three sizes, decode rejection, text masking, state tables, JSON writer |
| `--warp --debug` | pass, 10 of 10 |
| `--hardware --debug` (RTX 4070 Laptop GPU) | pass, 10 of 10 |

- Each GPU mode runs all eight combinations of same or own queue, legacy or enhanced barriers, and created or external (Godot-like typeless, render-target, black clear value) textures. Each runs 300 frames on three slots with a rebuild from 256 × 192 to 320 × 240 before frame 150, decodes both corners on every consumer readback, verifies every texel every 50 frames, and detects one deliberate mismatch. With the debug layer, error and corruption counts are 0 and the only warnings are the expected mismatching-clear-value warnings of external textures.
- Two child probes per GPU mode: a drain that cannot be confirmed ends the child with exit code 3 and the exact one-line report, running no later code; `ID3D12Device5::RemoveDevice` makes the drain return `SA2_E_DEVICE_REMOVED` with a failing removed-reason HRESULT, after which releases and detach succeed.
- The first owner-machine run found three defects in the implement call's version (call `20261003T143409Z-24c43648`), fixed before these results:
  1. On the NVIDIA adapter, `ClearRenderTargetView` with more than 32 rectangles in one call ended the process with `0xC0000409` inside the call. Bisected with an instrumented copy: 33 rectangles fail at the first call, 32 run; WARP accepts 100. The producer now clears the code blocks in batches of at most 32 rectangles.
  2. With its own queue, the producer enqueued a GPU wait for fence value 0 for a slot never shown, which the debug layer warns about; the wait is now skipped.
  3. Application Control on this machine refuses unsigned executables in the system temp folder, so the build and its check moved under the repository's ignored `work/` tree.
- The Codex review of the fixed candidate (review `20261003T154728Z-36c0c10e`, finding SA2-N-001, major) found that one resource could be registered in two slots. Fence history is kept per slot, so with its own queue the producer could write the resource through one slot while Godot still read it through the other. Registration now refuses a resource already registered in another slot, and the self-test checks the refusal in every configuration.
- Build: MSVC 19.51.36260 (Visual Studio Community 2026), Windows SDK 10.0.26100.0, CMake 4.4.3, Ninja 1.13.2; Release, static CRT. GPU: RTX 4070 Laptop GPU, NVIDIA driver 616.92. Commit: recorded with the integration commit.

## Godot smoke test (level 1)

Not run yet. It needs the reviewed SA2-G harness. Its answers (device and queue identity, the import state Godot expects, synchronisation, resize, teardown and the sequence numbers shown) go here and to PR #43 section 7.

## Acceptance items (packet section 1)

| # | Item | Status |
|---|---|---|
| 1 | Interop smoke test | In progress: producer DLL done and self-tested on the owner's GPU; Godot run not done. |
| 2 | Geometry port | Not started (needs item 1). |
| 3 | Three cold W3 runs | Not started. |
| 4 | Layout specification and feature list | Not started. |
| 5 | Renderer constraints (H-09) | Not started. |

## Not claimed

- The self-test is not a Godot result: Godot's own state tracker, display, window resize and frame composition are untested until the smoke test runs.
- Nothing here holds for another build, driver or machine. There are no performance claims.
