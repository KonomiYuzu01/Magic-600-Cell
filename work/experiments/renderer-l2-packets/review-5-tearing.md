# Review packet: `tearing` from displayed presents, and framework facts Q10 and P1

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: review one change on branch `claude/renderer-l2-followup`. It is not committed yet: review the working tree against `HEAD` (`git diff HEAD`). The working tree also holds the blind-seconds change, which is out of scope: `check_blind_seconds`, its two fixtures and the `blind-seconds` table row. Review `20261009T155227Z-a94ea84f` passed it.
- This change:
  - `work/experiments/renderer-l2/finalize_run.py`, at the `environment['tearing']` assignment (about line 490). `tearing` is now true only when at least one row of the chain in `[T0, stop)` is displayed (`Dropped` 0) and every displayed row has `AllowsTearing` 1. Before, every row counted.
  - `work/experiments/renderer-l2/check_l2.py`, in `extra_cases`:
    - the existing `presentmon-no-tearing` fixture keeps a displayed row with `AllowsTearing` 0, and `tearing` is false;
    - the new `presentmon-dropped-no-tearing` fixture has a dropped row with `AllowsTearing` 0, and `tearing` stays true.
  - `work/experiments/renderer-l2-packets/HARNESS.md`, the `run.json` `environment` rule (about line 234).
  - `work/experiments/renderer-l2-packets/FRAMEWORK-FACTS.md`:
    - new fact Q10: Qt 6.10.3 paces `requestUpdate` with the DXGI vblank service;
    - new section "PresentMon 2.6.0" with fact P1: when `AllowsTearing` is set.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. The verdict is `pass` if no `blocker` or `major` finding remains. Each finding needs a concrete counterexample.
- Questions:
  1. Can the new rule report `tearing` true for a run in which a displayed present did not allow tearing? Can it report false only because of dropped presents?
  2. P1: at PresentMon tag `v2.6.0`, is `AllowsTearing` (`SupportsTearing`) set only by the kernel flip and blit handlers named in P1, never from the DXGI present flags? Is a present that is dropped before its flip therefore always reported with `AllowsTearing` 0? Source: `PresentData/PresentMonTraceConsumer.cpp`; the CSV writer maps `SupportsTearing` to `AllowsTearing`.
  3. Q10: at Qt tags `v6.10.3`, is the following chain right?
     - `QSGThreadedRenderLoop::update` and `maybeUpdate` reach `QWindow::requestUpdate()` through `postUpdateRequest`.
     - `QWindowsWindow::requestUpdate` uses `QDxgiVSyncService` when `supportsWindow` is true, and delivers the update only after `IDXGIOutput::WaitForVBlank`.
     - `QT_D3D_NO_VBLANK_THREAD` set to a nonzero integer turns the service off.
     - The fallback `QPlatformWindow::requestUpdate` uses a 5 ms precise timer, which `QT_QPA_UPDATE_IDLE_TIME` replaces.

     Is the stated consequence right: the Qt app asks for each frame from `frameSwapped`, so its frames are paced at the display refresh although the swap interval is 0?
  4. Does the `HARNESS.md` text describe the new rule exactly?
- Out of scope:
  - the blind-seconds change;
  - L2-V-002 and its follow-up findings (escalation `20261009T155501Z-a2cb4438`);
  - every other part of level 2;
  - performance claims;
  - findings below `major`.

## 2. Actual problem and reproduction
- The owner's attended Godot W3 gate series of 4 October has three runs. Run 1 recorded `environment.tearing` false; runs 2 and 3 recorded true.
- Private analysis of run 1: exactly one row of the chain in `[T0, stop)` had `AllowsTearing` 0, out of 129,022.
  - It lies 6.809 s after trace start, in the preroll, before the gate interval.
  - Its `PresentFlags` is 512 (`DXGI_PRESENT_ALLOW_TEARING`), and it has `Dropped` 1, `SyncInterval` 0 and `Hardware: Independent Flip`.
- Every displayed row allowed tearing.
- `tearing` is metadata in `run.json` and in the gate summary's environment. `renderer_gate.py` does not use it for a verdict.
- Qt: every level 2 scene ran at 60.00 fps with p99 about 17.4 ms on a 60 Hz display, with `SyncInterval` 0, `AllowsTearing` 1 and `Hardware: Independent Flip`. Q10 is the source explanation. No runtime test has run yet; the owner decides whether to run the uncapped check named in Q10.

## 3. Environment and versions
- Branch `claude/renderer-l2-followup`, from `main` at `7057cb8`. Python 3.14 on Windows 11.
- PresentMon 2.6.0 (the owner's pinned capture tool). Qt 6.10.3 and Godot 4.7.2 .NET, as in the existing facts.

## 4. Necessary source and evidence
- `work/experiments/renderer-l2/finalize_run.py`:
  - the `tearing` assignment;
  - `read_presentmon` (line 333), where `dropped` and `tearing` are parsed.
- `work/experiments/renderer-l2/check_l2.py`: `fixture` (line 63) and the two tearing fixtures in `extra_cases`.
- `work/experiments/renderer-l2-packets/HARNESS.md`, the `run.json` description in section 7.
- `work/experiments/renderer-l2-packets/FRAMEWORK-FACTS.md`, Q10 and P1.
- Upstream sources at the pinned tags:
  - github.com/GameTechDev/PresentMon `v2.6.0`;
  - github.com/qt/qtbase and github.com/qt/qtdeclarative `v6.10.3`.
- `python -B work/experiments/renderer-l2/check_l2.py` passes all 173 finalizer cases.

## 5. Attempts so far
First change. Claude read the PresentMon and Qt sources on 2026-10-09.

## 6. Constraints and owned files
- Owned files: the four files of section 1 and this packet. No change to `tools/perf/renderer_gate.py`, S-B's runner or any app.
- Not a critical path. Risk tier: other code and tools, one Sol review on the fast tier.
- The 4 October records keep their recorded `tearing` values and are not rewritten.

## 7. Required return format
One JSON object matching `schemas/review-result.schema.json`. Each finding gives:
- ID;
- severity;
- evidence as `path:line` with the file digest;
- counterexample;
- suggested experiment;
- verification status.
