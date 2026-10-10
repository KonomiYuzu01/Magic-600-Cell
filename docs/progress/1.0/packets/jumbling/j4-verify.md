# Packet: scoped verification of the J4 review fixes (senior reviewer, review kind)

Round 2 (the last allowed) checks only J4V001 from round 1 (`20261009T224938Z-57acbf5c`): `--table` labelled an exactly disproved closure as closed. Fix: `result_label()` in `explore.py` applies `verdict()`'s invalidation rule to the table's Result column, and `--selftest` checks the label of both controls (`invalid (exact check failed)` and `closed`). The README tables are unchanged because no stored run is invalid. Report whether J4V001 is fixed and whether the fix adds a new `blocker` or `major`; nothing else.

Run read-only as a review by the senior reviewer, with effort `max` and speed tier `fast`. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: the scoped verification round after the full review `20261009T221306Z-30522c58` of the J4 explorer (`research/jumbling/explorer/`). Check only:
  1. whether each blocking finding is fixed:
     - **J4R001** (failed exact confirmations did not invalidate a closed verdict);
     - **J4R002** (a late points response could replace the current selection's points);
     - **J4R003** (the page ignored the recorded threshold);
     - **J4R004** (the README promised byte preservation for any formatting);
  2. whether the fixes introduce a new `blocker` or `major`.
- Acceptance: a schema-valid result. A finding names the failing input and `path:line` evidence.
- Out of scope: everything the full review did not flag as blocking, `minor` and `nit` issues, the result numbers as such, performance.

## 2. Actual problem and reproduction
The fixes, by finding:
- **J4R001.** `explore.py`: a new `verdict()` returns `invalid: exact confirmation failed` whenever a sampled coincidence fails the exact check, and calls a closed run `closed in the ball model: lead, not a finiteness proof`. `--selftest` adds the reviewer's tiny exact plane rotation (Cayley parameter of denominator up to 10¹²) as the negative control (24/456 confirmed, verdict invalid) and A4 as the positive control (closed, 0/0). The stored verdicts in `explorer-results.json` were recomputed with the same function, without rerunning any search (two runs changed wording; none is invalid). `explorer.js` shows an invalid run as "Invalid: exact check failed" and a closed one as "Closed in the ball model (lead)".
- **J4R002.** `explorer.js` `refresh()` takes a token and the run key before awaiting the points, and returns without touching the scene or the panel when either changed. `check_page.mjs` delays the class-00 points by 1.5 s, switches class-00 then class-05, and requires the class-05 stage line. With the guard removed, this check fails (2,680 points shown under class-05 instead of 3,000).
- **J4R003.** `explorer.js` reads `parameters.threshold_deg` and scales the ball and its rim to tan(θ/2), and every caption marked `data-threshold` shows θ. `check_page.mjs` serves a results file with θ = 30° and requires every caption to show 30°.
- **J4R004.** The README now states that other run records keep their values and that the file is rewritten in the tool's own format: records the tool wrote stay byte for byte the same, and a hand-formatted record is reformatted.

Reproduce from the repository root:
- `python research/jumbling/explorer/explore.py --selftest` (all lines `ok`);
- `NODE_PATH=$(npm root -g) PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers node research/jumbling/explorer/check_page.mjs [--vendor DIR]` (passed in the integrator's Linux session).

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, commit `3be8ffb`. Python 3 with NumPy, Node 22. Evidence kind: synthetic geometry.

## 4. Necessary source and evidence
- `work/reviews/20261009T221306Z-30522c58/review.json` (the findings).
- `research/jumbling/explorer/explore.py` (`verdict`, `run_one`, `selftest`), `explorer.js` (`setThreshold`, `statusText`, `reasonText`, `refresh`), `index.html` (captions, `.pill.invalid`, `--danger`), `check_page.mjs`, `README.md`, `explorer-results.json`.

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | Full review `20261009T221306Z-30522c58` | four majors, all adopted |
| 2 | Fixes above | self-test and page check pass |
| 3 | Scoped verification round 1 `20261009T224938Z-57acbf5c` | J4R002–J4R004 fixed; J4V001 (table label) open, adopted |
| 4 | `result_label()` (commit `3be8ffb`) | self-test controls `ok` |

## 6. Constraints and owned files
- Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Finding IDs use the prefix `J4V2`.
- Review only; do not perform follow-up work.
