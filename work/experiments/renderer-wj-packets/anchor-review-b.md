# Review packet, shard B: area-centroid shrink anchors in W-J and the J2 viewer

Review only; do not perform follow-up work. Read only.

## 1. Goal and acceptance
- **Goal.** One full review of the W-J and jumbling side of the anchor candidate on branch `claude/wj-area-centroid`, which carries out the owner's decision `docs/wiki/decisions/owner-decisions-2026-10-10-anchor.md`. The plan is `work/experiments/renderer-wj-packets/anchor-plan.md` (plan check `20261010T053301Z-f2cab142`, both findings adopted; scoped re-check `20261010T053858Z-f5bbb789` passed). The parts:
  1. Written by Codex in implement call `20261010T053816Z-28d86823` (packet `anchor-wj.md`), reviewed and applied by Claude with no change: `work/experiments/renderer-wj/probe.cpp`, `probe.h`, `reference_wj.py`, `lattice_wj.py`, `check_lattice.py`, `README.md`, `PROTOCOL.md`, and the regenerated `ref_*` files and indexes.
  2. Written by Claude: `research/jumbling/render-contract.md` section 2, `research/jumbling/viewer/full.js` (`ANCHOR_SHA256`, `stickerAnchors`, the loader and the `uCenters` upload) and `viewer/README.md`, the notes in `docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md` sections 4 and 6, and `work/experiments/renderer-wj/RESULT.md`.
  - The rule itself is `work/experiments/renderer-sb/SPEC.md` section 3 and `sticker_anchors` in `work/experiments/renderer-sb/reference_geometry.py`. Shard A reviews them; read them here as the specification.
  - Diff: `git diff afe5f85 HEAD -- work/experiments/renderer-wj research/jumbling docs/progress/1.0/packets/renderer`. `afe5f85` is `main` before this branch.
- **Acceptance.** One JSON result matching `schemas/review-result.schema.json`.
  - Report only `blocker` or `major` findings, each with a concrete counterexample: an input, an asset byte, a sticker, a slot, a fixture state or a run that gives the wrong outcome.
  - The verdict is `pass` if there is no such finding.
- **Questions.**
  1. Does the W-J C++ `Assets::Assets` compute the same bytes as the SPEC rule, in the same order, with one float32 rounding, and does its self-test refuse another digest? Do `reference_wj.py` and `lattice_wj.py` take the anchors only from `sticker_anchors`, and does `reference_wj.py` refuse a digest other than the pin?
  2. Can `check_lattice.py` pass with wrong anchors? Consider the bound and centre comparisons, the new `center_error.max() > 2e-6` condition, the solved control and the retained state.
  3. Are the regenerated W-J references consistent with the rule and with `check_wj.py`'s byte-identity and refusal checks? Does any W-J file still read `mesh_centers.f32` for the shrink, or pin a digest that the change makes stale? The GPU path is `gpu.cpp`, `geometry.hlsl` and `lattice.hlsl`.
  4. J2 viewer: does `stickerAnchors` in `full.js` follow the SPEC order in JavaScript doubles with one `Float32Array` rounding? Does the loader refuse to draw on a digest mismatch, before any upload? Is `uCenters` now bound to the computed anchors and nothing else? Does `export_full.py`, or anything else the viewer loads, still feed `mesh_centers.f32` into the shrink?
  5. Does the render contract define the anchor so that a second implementation produces the same bytes? Does it still match `SPEC.md`?
  6. Do the records claim more than was measured? `RESULT.md` items 2 and 6 and the E-2.4-0J notes must leave the GPU lattice check, the W-J runs and the three cold W3 runs of the changed W-J executable as not run. `check_full.mjs` was not run (it needs network access and Chromium); `node --check` and a Node script that runs `full.js`'s `stickerAnchors` on the assets were run.
- **Out of scope.**
  - The owner's choice of the rule, the W-J pose method and the jumbling theory;
  - the S-B probe, its references and the S-A2/S-D consumers (shard A reviews them);
  - Look Lab (`tools/looklab/`, another session's area) and the 0.4 code;
  - performance claims and wording;
  - findings below `major`.

## 2. Actual problem and reproduction
- With `assets/mesh_centers.f32` as the anchor, `check_lattice.py` exited 1: for J1 generator 1, 4,125 of the 4,605 moved stickers disagreed with their destination slot by up to 0.0144 world units. The raw sticker meshes are congruent under the twist (bounds agree to 1.07e-7); the anchors are not.
- This candidate implements the approved fix. No failure of the candidate is known.

## 3. Environment and versions
- Branch `claude/wj-area-centroid`, at the commit that adds this packet. Windows 11 (build 26200).
- The engine environment `tools/.venv/engine` (CPython 3.14.7, NumPy 2.3.5); Node.js for the viewer checks; MSVC, the Windows SDK with DXC, CMake and Ninja for the probe.
- Source evidence, run by Claude on the candidate:
  - `tools/.venv/engine/Scripts/python.exe -B work/experiments/renderer-wj/check_lattice.py`: exit 0; lattice-start errors 0; lattice-retained maximum bound error 8.17e-8, maximum centre error 9.80e-8;
  - `tools/.venv/engine/Scripts/python.exe -B work/experiments/renderer-wj/check_wj.py`: see `RESULT.md` and the PR body for its outcome on this candidate; it also passed as the wrapper's acceptance check of the implement call;
  - `node --check research/jumbling/viewer/full.js`: ok; the Node script reproduces the pin and a changed vertex changes the digest.
- No GPU run of the W-J probe, no lattice check on the GPU and no capture has used this candidate.

## 4. Necessary source and evidence
- Rule: `work/experiments/renderer-sb/SPEC.md` section 3; contract: `research/jumbling/render-contract.md` section 2.
- Packets: `work/experiments/renderer-wj-packets/anchor-plan.md`, `anchor-wj.md`; renderer packet `docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md` (acceptance items 2 and 6).
- Viewer: `research/jumbling/viewer/full.js`, `export_full.py`, `README.md`, `check_full.mjs`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | The W-J probe with the fixed home shrink agrees with S-B slot geometry | call `20261010T024836Z-a9b1fb0b` | `check_lattice.py` | failed, 0.0144; queued to the owner |
| 2 | The anchor, not the twist, is at fault | integrator check of raw mesh bounds | NumPy | raw meshes agree to 1.07e-7 |
| 3 | `anchor-wj.md` as written | implement call `20261010T053816Z-28d86823` | wrapper acceptance check; Claude's review; the checks in section 3 | valid, applied with no change |

## 6. Constraints and owned files
- Read only. No file changes.
- Invariants: synthetic labels only; claims only for the measured build identity; no asset and no model identity change; the human-solve boundary; never commit raw PresentMon output or machine diagnostics.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`, with an answer to each of questions 1 to 6 in the summary.
- Review only; do not perform follow-up work.
