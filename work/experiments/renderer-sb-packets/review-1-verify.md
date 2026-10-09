# Scoped verification packet: S-B probe review findings SB1-R1 to SB1-R3 (round 1)

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: check only whether the blocking findings of review `20261003T025224Z-7c1d70c4` (shard 1, probe) are fixed, and whether the fixes introduce a new `blocker` or `major`. This is not a full review: do not report other issues unless they are a new `blocker` or `major` caused by these fixes.
- Findings under verification:
  - SB1-R1 (major): invisible flip-model presents could produce accepted gate evidence; `presents_occluded` cannot detect it because flip-model `Present` never returns `DXGI_STATUS_OCCLUDED`.
  - SB1-R2 (major): `power_source` was sampled once, after the trace, so a battery run could be recorded as mains.
  - SB1-R3 (major): an early probe failure could leave `run_scene.ps1` waiting for PresentMon indefinitely.
  - SB1-R4 (minor, fixed in the same pass): the gate summary dropped adapter, driver, refresh rate, sync interval and condition declarations.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`, with one finding per unresolved item (same ID) and any new `blocker`/`major`, each with a concrete counterexample. Verdict `pass` if all three are fixed and nothing new blocks.

## 2. Actual problem and reproduction
- Fixes, all uncommitted in the working tree on top of `claude/renderer-sb` HEAD:
  - SB1-R1:
    - `probe/src/gpu.cpp` drops the occlusion counter. Every 100 ms of the trace, `sampleConditions()` samples `IsWindowVisible`, `IsIconic`, `DWMWA_CLOAKED` and the foreground window.
    - `run.json` `window` records `samples`, `samples_not_visible`, `samples_not_foreground` and `visible_throughout`.
    - The probe exits 3 when any sample was not visible.
    - `probe/run_scene.ps1` refuses exit 3, and refuses a swap chain all of whose PresentMon rows have `Dropped` = 1. It prints the present count, the not-displayed count and the present modes.
  - SB1-R2:
    - The same 100 ms sampling records `GetSystemPowerStatus`. `power_source` is `mains` or `battery` only when every sample agrees, otherwise `changed` or `unknown`. The gate accepts only `mains`.
    - `power_mode` comes from `PowerRegisterForEffectivePowerModeNotifications`. It is `changed` if the mode changed during the trace.
  - SB1-R3: `run_scene.ps1` now:
    - waits for the probe;
    - checks its exit code first and throws on anything but 0 or 2, so the `finally` block kills PresentMon;
    - then waits at most 60 s for PresentMon.
  - SB1-R4: the environment uses the summary's public keys:
    - `presenting_adapter`, which holds the adapter, the driver, and whether this adapter drives the window's display;
    - `presentation_interval`, `power_mode` and `display.refresh_hz`;
    - owner declarations through `declared.vendor_mode` and `declared.overlays`. The README documents these keys.
- Reproduce: `python work/experiments/renderer-sb/probe/check_probe.py` (build plus `--selftest`); `git diff` shows the fixes.

## 3. Environment and versions
- As in the shard 1 packet (`work/experiments/renderer-sb-packets/review-1-probe.md` section 3).
- Evidence kind in the review sandbox: source/fixture only.

## 4. Necessary source and evidence
- Original findings: `work/reviews/20261003T025224Z-7c1d70c4/review.json` (if readable), otherwise section 1 above.
- Changed files: `work/experiments/renderer-sb/probe/src/gpu.cpp`, `src/probe.cpp` (self-test fixture environment), `CMakeLists.txt` (links `dwmapi`, `powrprof`), `run_scene.ps1`, `README.md`.
- Readers: `tools/perf/renderer_gate.py` (`condition_reasons`), `tools/perf/b412_summary.py` (`public_environment`, `PUBLIC_SCALARS`, `PUBLIC_OBJECTS`).
- Integrator's local results on the owner's machine (actual Windows/DirectX, functional, not performance evidence; probe launched without PresentMon):
  - Build ok; `check_probe.py` self-test ok.
  - W3, 8 s: exit 0; label check pass; 80 samples, 0 not visible, 0 not foreground.
    - power `mains`, mode `max_performance`;
    - `presenting_adapter`: `NVIDIA GeForce RTX 4070 Laptop GPU, driver 32.0.16.1692, drives the window's display`;
    - refresh 60, interval 0.
  - W3, 10 s, window minimised with `ShowWindow(SW_MINIMIZE)` after 7 s: exit 3; 53 of 100 samples not visible.
  - Four 8 s W3 fault injections (`corrupt-label`, `swap-same-colour`, `delay-adoption`, `stale-binding`): exit 2 each; label check fail.
  - `b412_summary.public_environment` of the normal run keeps `presenting_adapter`, `presentation_interval`, `power_source`, `power_mode`, `display` with `refresh_hz`, `backbuffer`, and `declared.frame_generation`.
  - `run_scene.ps1` parses. Its PresentMon path is not yet exercised; it needs an administrator shell.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | the fixes in section 2 close SB1-R1 to SB1-R3 | as listed | the local runs in section 4 | normal run accepted; minimised run exits 3; faults exit 2 |

## 6. Constraints and owned files
- Read-only review. Out of scope:
  - `tools/perf/renderer_gate.py` and `b412_summary.py`: their whitelist does not publish `declared.upscaling`, a gate-side limitation recorded as deferred;
  - the handoff test;
  - performance numbers;
  - style.
- Questions:
  1. Can a run whose window was hidden, minimised or cloaked for a material part of the measured interval still exit 0 and be admitted by `run_scene.ps1`?
  2. Can a run with any battery sample during the trace still record `power_source` `mains`?
  3. Can `run_scene.ps1` still wait without bound, or report a run as evidence after the probe failed?
  4. Do the fixes add a hazard: the power notification callback outliving `Gpu`, a data race on `powerMode`, or sampling cost or blocking inside the timed loop large enough to change the gate result?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
