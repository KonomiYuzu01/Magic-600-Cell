# Scoped verification, shard A: area-centroid shrink anchors in S-B

Review only; do not perform follow-up work. Read only.

## 1. Goal and acceptance
- **Goal.** Check only whether the two blocking findings of review `20261010T062811Z-66599ccb` are fixed on branch `claude/wj-area-centroid`, and whether the fixes introduce a new `blocker` or `major`. Both findings were adopted (`work/reviews/20261010T062811Z-66599ccb/dispositions.json`).
  - Diff of the fixes: `git diff a301851 HEAD -- work/experiments/renderer-sb docs/wiki`. `a301851` is the reviewed candidate.
- **Acceptance.** One JSON result matching `schemas/review-result.schema.json`.
  - `pass` when both findings are fixed and nothing new is blocking;
  - otherwise one `blocker` or `major` finding per unresolved item or new defect, each with a concrete counterexample.
- **Out of scope.** Everything the full review covered and did not flag; the W-J and viewer files (shard B's verification covers them); wording; findings below `major`.

## 2. Actual problem and reproduction
- **ANCHOR-A-001 (major).** `check_assets.py` used `assets/mesh_centers.f32` as the continuity control without checking its manifest digest, so a corrupted control file still passed.
  - Fix: `check_anchors()` reads the file once and refuses bytes whose SHA-256 differs from `assets/manifest.json` before the control runs.
  - Experiment (in memory, no file writes): byte 0 of `mesh_centers.f32` XOR 1, manifest unchanged. `check_anchors()` raises `assets/mesh_centers.f32: SHA-256 digest mismatch with manifest.json`. The unchanged assets still pass.
- **ANCHOR-A-002 (major).** The native anchor digest was checked only in `--selftest`, so ordinary S-B runs and the S-A2/S-D consumers, which construct `sb::Assets` directly, could upload non-canonical anchors.
  - Fix: a new `shrinkAnchors(offsets, vertices)` in `work/experiments/renderer-sb/probe/src/probe.cpp` holds the unchanged SPEC loop and refuses any result whose SHA-256 differs from `AnchorSha256`. `Assets::Assets` calls it, so every consumer gets either the pinned bytes or an exception before any upload.
  - The self-test sets `_RC_UP` with `_controlfp_s`, recomputes the anchors, restores the rounding mode and requires them to be either refused with that message or byte-identical.
  - Experiment: on the owner's machine (MSVC x64, default `/fp:precise`), `check_probe.py` rebuilds the probe and its self-test prints `selftest: shrink anchors under upward rounding: refused`.

## 3. Environment and versions
- Branch `claude/wj-area-centroid`, at the commit that adds this packet. Windows 11 (build 26200).
- Run by Claude on this head:
  - `python -B work/experiments/renderer-sb/check_assets.py`: ok;
  - `python -B work/experiments/renderer-sb/probe/check_probe.py`: build exit 0, self-test ok, including both anchor lines;
  - `python -B work/experiments/renderer-sa2/native/check_native.py`: a fresh build of `sa2_interop.dll`, which compiles the S-B `probe.cpp`, and its CPU self-test: ok;
  - `python -B work/experiments/renderer-sa2/check_project.py`: PASS; `python -B work/experiments/renderer-sd/check_project.py`: ok;
  - the wiki lint and its test: ok (the `w3-label-check` claim digest was refreshed for the edited `RESULT.md`).
- No GPU run has used this head.

## 4. Necessary source and evidence
- `work/reviews/20261010T062811Z-66599ccb/review.json` (the findings).
- `work/experiments/renderer-sb/check_assets.py` `check_anchors()`; `work/experiments/renderer-sb/probe/src/probe.cpp` `shrinkAnchors`, `Assets::Assets` and `selftest`; `probe/README.md`; `RESULT.md` section "Shrink anchor change, 10 October 2026".
- Consumers: `work/experiments/renderer-sa2/native/CMakeLists.txt` (same `/W4 /permissive- /utf-8 /EHsc` options, default floating-point model), `src/scene.cpp`, `src/scene_record.cpp`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | Candidate `a301851` | full review `20261010T062811Z-66599ccb` | two shards | A-001, A-002 major; both adopted |
| 2 | Manifest check closes A-001 | `check_anchors()` | in-memory corrupted control | refused |
| 3 | Constructor pin closes A-002 | `shrinkAnchors` | upward-rounding self-test; S-A2 CPU self-test | refused; ok |

## 6. Constraints and owned files
- Read only. No file changes.
- Invariants: synthetic labels only; claims only for the measured build identity; no asset and no model identity change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`, stating for each of ANCHOR-A-001 and ANCHOR-A-002 whether it is fixed.
- Review only; do not perform follow-up work.
