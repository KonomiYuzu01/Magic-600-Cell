# Scoped verification: SA2 native producer, finding SA2-N-001

Review only; do not perform follow-up work.

Run from the `claude/renderer-sb` checkout:
`python tools/agents/codex_review.py --kind review --model gpt-6.1-sol --effort max --speed fast --packet work/experiments/renderer-sa2-packets/review-1-native-verify.md`

## 1. Goal and acceptance
- Goal: a scoped verification round of the SA2 native producer candidate after review `20261003T154728Z-36c0c10e`. Check only (a) whether finding SA2-N-001 is fixed, and (b) whether the fix introduces a new `blocker` or `major`. Do not repeat the full review.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Verdict `pass` if SA2-N-001 is fixed and the fix introduces no new `blocker` or `major`; otherwise one finding per remaining problem, with a concrete counterexample.

## 2. Actual problem and reproduction
- SA2-N-001 (major): `sa2_register_slot` accepted a resource already registered in another slot. Fence history is kept per slot and registration sets `last_shown` to 0, so with its own queue the producer could write the resource through the second slot while Godot still read it through the first. The full finding is in `work/reviews/20261003T154728Z-36c0c10e/review.json`; disposition `adopt`.
- The fix:
  - `work/experiments/renderer-sa2/native/src/interop.cpp`, `sa2_register_slot` (after the slot-occupied check, around line 491): every occupied slot is compared with the new resource by COM identity (`same_identity`, `QueryInterface(IUnknown)` on both); a match is refused with `SA2_E_WRONG_STATE` before the render-target view is created and before any state changes.
  - `work/experiments/renderer-sa2/native/include/sa2_interop.h`, the `sa2_register_slot` comment: the refusal is documented. Comment only; no ABI change.
  - `work/experiments/renderer-sa2/native/src/gpu_test.cpp`, `make_ring`: before slot 1 is registered, registering slot 0's resource into slot 1 must return `SA2_E_WRONG_STATE`. This runs in all eight configurations (created and external textures), at both ring sizes and in the child probes. The existing reference-count checks across attach and detach would catch a reference leaked by the refused registration.
- Re-registration after `sa2_unregister_slot` is not affected: unregister already requires a confirmed `sa2_drain`, which waits for both queues.
- All of Claude's changes over the implement call's version are in `work/experiments/renderer-sa2-packets/review-1-claude-fixes.diff` (Codex's version on the left).

## 3. Environment and versions
- Windows 11 (build 26200); NVIDIA GeForce RTX 4070 Laptop GPU, driver 616.92; MSVC 19.51.36260, Windows SDK 10.0.26100.0.
- The review sandbox is read-only and CPU only.

## 4. Necessary source and evidence
- Files: `work/experiments/renderer-sa2/native/src/interop.cpp`, `src/gpu_test.cpp`, `include/sa2_interop.h` (all read directly; the native folder is untracked under the ignored `work/` tree, except the header, which `git diff HEAD --` shows).
- Integrator's checks on this candidate (owner's machine):
  - `python work/experiments/renderer-sa2/native/check_native.py --gpu`: `check_native: ok (CPU, WARP and hardware)`; no leftover build directory.
  - Explicit build, `sa2_selftest.exe --cpu`, `--warp --debug`, `--hardware --debug`: exit 0 each, every check `pass`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | full review of the integrated candidate | none | review `20261003T154728Z-36c0c10e` | 1 major (SA2-N-001) |
| 2 | refusing duplicate registrations closes SA2-N-001 | the fix above | checks in section 4 | all pass |

## 6. Constraints and owned files
- Read-only review.
- Out of scope: everything except SA2-N-001 and new `blocker` or `major` problems caused by its fix; `minor` and `nit` findings.
- Questions:
  1. Can a resource still end up in two slots at once (for example through another interface pointer to the same resource, or a godot_owned and an imported registration of one resource)?
  2. Can the refused registration leave any state changed (a render-target view, a reference, slot fields)?
  3. Does the new self-test check fail if the refusal is removed?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
