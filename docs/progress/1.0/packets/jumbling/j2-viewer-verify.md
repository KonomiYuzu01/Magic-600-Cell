# Packet: scoped verification of the J2 viewer fixes (senior reviewer, review kind)

Run read-only as a scoped verification review by the senior reviewer, with effort `max` and speed tier `fast`. The fixes were implemented by the routine reviewer model, so the senior reviewer checks them. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: a scoped verification round under `AGENTS.md` "Review rounds". Check only two things:
  - Are the blocking findings V1 and V2 of review `20261009T195031Z-2a42a035` fixed at the current head?
  - Does the fix commit `3487b02` introduce a new `blocker` or `major`?
- The findings:
  - **V1 (major).** Global shrink moved certificate points across their certified cut. Counterexample: `#s1-g1-p7`, vertex 7, exact h ≈ +0.003370; the Global marker sat at h ≈ −0.001076.
  - **V2 (major).** Retained-turn previews described moving pieces as on the lattice. Counterexample: `#s1-p1035` scrubbed to 1.5 said "On the lattice: the piece sits in a slot."
- The same commit also fixes three minor findings: V3 (digest check), V4 (signed certificate assertion) and V5 (deep-link grips). Check them only for new `blocker` or `major` issues.
- Acceptance: a schema-valid result. For V1 and V2, say fixed or not fixed, with `path:line` evidence. Report any new `blocker` or `major` with a counterexample.
- Out of scope: everything outside `git diff 0adcd61 3487b02 -- research/jumbling/viewer/`, and every `minor` or `nit`.

## 2. Actual problem and reproduction
- `git diff 0adcd61 3487b02 -- research/jumbling/viewer/`.
- Headless check: `node research/jumbling/viewer/check_viewer.mjs --vendor <dir>`. At `3487b02` it passes 43 of 43 assertions, including new ones for V1 (both projections), V2, V3 (both loading paths), V4 (reversed signs fail) and V5.

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, head at or after `3487b02`. Linux, Node 22, Playwright with headless Chromium on SwiftShader.

## 4. Necessary source and evidence
- `research/jumbling/viewer/viewer.js`: `loadScene`, `isOff`, `updateFocusLines`, `buildLocal`, `updatePanels`, `readHash`.
- `research/jumbling/viewer/check_viewer.mjs`, the new assertions.
- `research/jumbling/viewer/README.md`.

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | Review `20261009T195031Z-2a42a035` | V1 and V2 major; V3, V4 and V5 minor; all adopted |
| 2 | Implementation `20261009T201016Z-d7b09cf4`, integrated as `3487b02` | Headless check 43/43 |

## 6. Constraints and owned files
- Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Reuse the IDs V1 and V2; new findings use the prefix `VV`.
- Review only; do not perform follow-up work.
