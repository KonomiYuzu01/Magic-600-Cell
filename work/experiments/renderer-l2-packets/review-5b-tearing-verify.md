# Scoped verification packet: L2-TEARING-001

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: a scoped verification round of review `20261009T160651Z-220450f6` (packet `review-5-tearing.md`). Check only:
  1. whether finding L2-TEARING-001 is fixed;
  2. whether the fix introduces a new `blocker` or `major`.
- The change is not committed: review the working tree against `HEAD` (`git diff HEAD`). The fix changes wording only, in five places:
  - `work/experiments/renderer-l2-packets/HARNESS.md`, the `tearing` rule (about line 234);
  - `work/experiments/renderer-l2/finalize_run.py`, the comment at the `environment['tearing']` assignment (about line 487);
  - `work/experiments/renderer-l2/check_l2.py`, the comment above `presentmon-dropped-no-tearing` (about line 522);
  - `work/experiments/renderer-l2-packets/FRAMEWORK-FACTS.md`, fact P1;
  - `work/experiments/renderer-l2/RESULT.md`, finding 3 (a new file; only the P1 sentence is in scope).
- The predicate is unchanged.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. The verdict is `pass` if no `blocker` or `major` finding remains.
- Out of scope:
  - everything else in the working tree, including the blind-seconds change and the rest of `RESULT.md`;
  - L2-V-002;
  - findings below `major`.

## 2. Actual problem and reproduction
L2-TEARING-001 (major): the text said that PresentMon reports `AllowsTearing` 0 for every dropped present. At `v2.6.0`, `Blit_Info` can set `SupportsTearing` and a later `BlitCancel_Info` can discard the present without clearing it, so a dropped row can show 1. The reviewer asked:
- to keep the predicate;
- to qualify the zero-value explanation to presents dropped before any tearing-setting handler;
- to document that at least one displayed row is required.

## 3. Environment and versions
Branch `claude/renderer-l2-followup` from `main` at `7057cb8`; PresentMon 2.6.0.

## 4. Necessary source and evidence
- The five places in section 1.
- PresentMon `v2.6.0`: `PresentData/PresentMonTraceConsumer.cpp` (blit and blit-cancel handlers) and `PresentMon/CsvOutput.cpp`.
- `python -B work/experiments/renderer-l2/check_l2.py`: 173 finalizer cases pass.

## 5. Attempts so far
Review `20261009T160651Z-220450f6` found L2-TEARING-001, and the disposition is `adopt`. This is the first verification round.

## 6. Constraints and owned files
- Owned files: the five files of section 1.
- Not a critical path; fast tier.

## 7. Required return format
One JSON object matching `schemas/review-result.schema.json`, with ID, severity, evidence (`path:line` with file digest), counterexample, suggested experiment and verification status for each finding.
