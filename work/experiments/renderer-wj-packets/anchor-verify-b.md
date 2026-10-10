# Scoped verification, shard B: area-centroid shrink anchors in W-J

Review only; do not perform follow-up work. Read only.

## 1. Goal and acceptance
- **Goal.** Check only whether the blocking finding of review `20261010T062811Z-a0f46b4c` is fixed on branch `claude/wj-area-centroid`, and whether the fixes in the W-J files introduce a new `blocker` or `major`. The finding was adopted (`work/reviews/20261010T062811Z-a0f46b4c/dispositions.json`).
  - Diff of the fixes: `git diff a301851 HEAD -- work/experiments/renderer-wj`. `a301851` is the reviewed candidate.
- **Acceptance.** One JSON result matching `schemas/review-result.schema.json`.
  - `pass` when the finding is fixed and nothing new is blocking;
  - otherwise one `blocker` or `major` finding per unresolved item or new defect, each with a concrete counterexample.
- **Out of scope.** Everything the full review covered and did not flag; the S-B files (shard A's verification covers them); the J2 viewer (unchanged since the review); wording; findings below `major`.

## 2. Actual problem and reproduction
- **B1 (major).** `lattice_wj.py` `controls()` checked neither the geometry against the manifest nor the anchor digest, so a doubled mesh or zero anchors passed both lattice controls.
  - Fix: a new `asset(name)` reads each asset and refuses bytes whose SHA-256 differs from `assets/manifest.json`. `check_frame()` and `controls()` read `mesh.json`, `model.npz`, `cell_frames.f32`, `slot_piece.u32`, `mesh_vertices.f32` and `mesh_sticker.u32` only through it. `controls()` packs the anchors as little-endian float32 and refuses a SHA-256 other than `ANCHOR_SHA256` before any comparison.
  - Experiments (engine Python, in memory, no file writes):
    - every float of `mesh_vertices.f32` doubled, manifest unchanged: `controls()` raises `assets/mesh_vertices.f32: SHA-256 digest mismatch with manifest.json`;
    - `sticker_anchors` replaced by 1,732 zeros: `controls()` raises `shrink anchors: SHA-256 mismatch with SPEC section 3`;
    - unchanged assets: `check_lattice.py` exits 0 with the same figures as before (retained bound error 8.16769804e-08, centre error 9.8010051e-08).
- **Same gap in the W-J native probe** (shard A's ANCHOR-A-002, applied here too). The W-J `probe.cpp` checked the anchor digest only in `--selftest`.
  - Fix: as in S-B, `shrinkAnchors(offsets, vertices)` holds the unchanged loop and refuses another digest; `Assets::Assets` calls it. `probe.h` gains the `AnchorSha256` constant. The self-test recomputes the anchors under `_RC_UP` and requires refusal or byte identity.
  - The file has mixed line endings from earlier edits; the fix keeps each existing line's ending.

## 3. Environment and versions
- Branch `claude/wj-area-centroid`, at the commit that adds this packet. Windows 11 (build 26200); engine Python CPython 3.14.7 with NumPy 2.3.5.
- Run by Claude on this head:
  - `work/experiments/renderer-wj/build.cmd <fresh directory>`: build ok;
  - `prepare_wj.py --out <fresh private folder>`: `W-J exports: pass`, both lattice controls pass;
  - `wj_probe.exe --selftest --scene wj --menu S4 --data <those fixtures>`: see section 5;
  - `tools/.venv/engine/Scripts/python.exe -B work/experiments/renderer-wj/check_lattice.py`: exit 0;
  - `tools/.venv/engine/Scripts/python.exe -B work/experiments/renderer-wj/check_wj.py`: see section 5.
- No GPU run has used this head.

## 4. Necessary source and evidence
- `work/reviews/20261010T062811Z-a0f46b4c/review.json` (the finding).
- `work/experiments/renderer-wj/lattice_wj.py`, `probe.cpp` (`shrinkAnchors`, `Assets::Assets`, `selftest`), `probe.h`, `README.md`, `RESULT.md`.
- Callers of `controls()`: `check_lattice.py`, `prepare_wj.py`, `check_wj.py`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | Candidate `a301851` | full review `20261010T062811Z-a0f46b4c` | shard B | B1 major; adopted |
| 2 | Manifest and anchor checks close B1 | `asset()`, digest check in `controls()` | the three experiments in section 2 | both faults refused; unchanged assets pass |
| 3 | The native pin closes the W-J self-test-only gap | `shrinkAnchors` | `wj_probe.exe --selftest` | pass: `shrink anchors SHA-256: ok`, `shrink anchors under upward rounding: refused`, the clean pose control passes and all five pose faults fail (CPU only) |
| 4 | Nothing else in the acceptance changed | — | `check_wj.py` | pass: all three fixtures replay with exact digests, 18 altered-fixture refusals, 60 reference files byte-identical |

## 6. Constraints and owned files
- Read only. No file changes.
- Invariants: synthetic labels only; claims only for the measured build identity; no asset and no model identity change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`, stating whether B1 is fixed and whether the W-J native change is correct.
- Review only; do not perform follow-up work.
