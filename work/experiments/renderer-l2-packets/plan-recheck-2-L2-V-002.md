# Second scoped plan re-check packet: L2-V-002 plan, revision 3

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: the second scoped re-check of `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md`, after re-check `20261009T163456Z-b3c5367e`. That re-check closed L2-P-002-02 and -03 and kept L2-P-002-01 open for one startup case. Check only:
  1. whether L2-P-002-01 is now closed;
  2. whether the change introduces a new `blocker` or `major`.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. The verdict is `pass` if no `blocker` or `major` finding remains.
- Out of scope: every other part of the plan; code; findings below `major`.

## 2. Actual problem and reproduction
The open case: a module is created after `ready`, then loaded, called, unloaded and deleted before `main` registers the observer. The unload history then holds a name that matches no current file, and revision 2 accepted that history.

## 3. Environment and versions
Branch `claude/renderer-l2-followup`; Windows 11, Qt 6.10.3.

## 4. Necessary source and evidence
- The plan, section 5 ("Before `main`" and "After the seal"), and section 7 (observer test program, case 1).
- The changed text of revision 3:
  > **Before `main`** (process start until registration):
  > - Modules loaded before `main` that are still loaded are in the snapshot below.
  > - Modules loaded and unloaded before `main` are in the loader's unload history, `RtlGetUnloadEventTraceEx`. Its entries keep only a base name of up to 32 characters and no directory, so a name cannot show that a module was out of scope.
  > - At registration, the observer therefore refuses (fail closed) when the history holds any entry at all. A module that was created, loaded, unloaded and deleted before registration is refused this way too (L2-P-002-01, re-check).
  > - Stop point: after the first build of G2, Claude runs the real `sd_smoke.exe` once with an invalid level 2 argument. That is the usage path: every static import loads, but there is no window and no GPU work, and it writes a harness record. Claude then reads the unload-history result from that record. If a normal Qt startup unloads any module before `main`, every run would refuse. Implementation then stops and the plan returns to Astra; the rule is never weakened silently.
  > 
  > - The callback reserves its slot (step 1) before it reads `sealed`, and `write()` sets `sealed` before it reads the count. With sequentially consistent atomics in that order, every in-scope load is either counted before the seal or sees the seal. Unloads after the seal change nothing, because a loaded module is already in the union.
  >   1. the probe is loaded, called and unloaded by a global constructor before `main`: refusal. This holds also when the probe file is deleted before `main`, so no current file matches its name;
- The previous re-check: `work/reviews/20261009T163456Z-b3c5367e/review.json` (disposition: adopt).

## 5. Attempts so far
Revision 2 matched the history against current file names. Revision 3 follows the reviewer's simplest option: it refuses every nonempty startup history. It also adds a stop point, in case normal Qt startup unloads modules. It states the reserve-before-read order that the reviewer required.

## 6. Constraints and owned files
Owned: the plan and this packet. No code. Not a critical path; Astra on the fast tier. This is the second and last scoped round for the plan.

## 7. Required return format
One JSON object matching `schemas/review-result.schema.json`, with ID, severity, evidence (`path:line` with file digest), counterexample, suggested experiment and verification status for each finding.
