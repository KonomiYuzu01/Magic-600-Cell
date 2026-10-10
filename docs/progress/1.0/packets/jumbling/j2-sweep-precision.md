# Packet: review of the sweep-angle precision fix in the full-state viewer (review kind)

Run read-only as a review with the default reviewer model, effort `max` and speed tier `fast`. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: review one integrator change to the Codex-authored full-state page (`research/jumbling/viewer/full.js`, call `20261009T223844Z-3048ea6b`). The shader computed `cos(uAngle) - 1` and `sin(uAngle)` in single precision on the GPU; the uniform `uSweep` now carries (cos φ − 1, sin φ) computed on the CPU in double precision as (−2 sin²(φ/2), sin φ), and the shader only multiplies.
- Acceptance: a schema-valid result. A `blocker` or `major` names the input that fails and `path:line` evidence. Check only:
  1. the swept rotation is unchanged in exact arithmetic: same plane, same sign convention, same angle schedule (smoothstep over the scrub, twist then exact inverse), identity when not sweeping;
  2. no other use of the removed `uAngle` remains, and the "moving and sweeping" condition is equivalent;
  3. the change adds no new `blocker` or `major`.
- Out of scope: everything else in the page, look values, performance, `minor` and `nit` findings.

## 2. Actual problem and reproduction
- `check_full.mjs` compares 16 GPU sweep samples per menu, state and projection with the fixture's double-precision samples at relative tolerance 1e-4. Before the fix, two of 142 assertions failed: I_a at t = 1 (end of the twist), perspective and stereographic. A diagnosis of the I_a perspective case showed world-coordinate errors of 1.01–1.17e-4 on four pieces; projected and NDC errors stayed below 6e-5.
- After the fix the same diagnosis gives world errors of 2e-8 to 2.1e-7 on all 16 pieces, the float32 level. The full rerun of `check_full.mjs` is recorded in section 5.
- Reproduce: `python research/jumbling/viewer/export_full.py`, then `NODE_PATH=$(npm root -g) PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers node research/jumbling/viewer/check_full.mjs [--vendor DIR]`.

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, current head. Node 22, Playwright Chromium with SwiftShader (WebGL2), three.js 0.160.0. Evidence kind: synthetic geometry; headless, no Windows, Direct3D or performance evidence.

## 4. Necessary source and evidence
- `research/jumbling/viewer/full.js`: the vertex shader (`sweepPoint`, `main`), the uniforms block and `syncView` (the angle schedule).
- `research/jumbling/render-contract.md` section 4 (swept motion) and `research/jumbling/viewer/check_full.mjs` (`expectedProjection`, the sweep-sample loop).

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | Full-state page (Codex implementation `20261009T223844Z-3048ea6b`) | page check passed in its sandbox |
| 2 | All-menu export in the integrator's session, then `check_full.mjs` | 140/142; I_a sweep samples at t = 1 off by up to 1.2e-4 |
| 3 | Angle terms computed on the CPU | CHECK_RESULT |

## 6. Constraints and owned files
- Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Finding IDs use the prefix `J2P`.
- Review only; do not perform follow-up work.
