# ADR-D02 build: Astra review round 1

The first of the two Astra rounds that come before D02's freeze. The owner asked for both rounds: build framework,
"Refinement, after C6, then the freeze", in `docs/progress/1.0/adr-d02-build-framework.md`. Round 1 reviews the whole
build. Round 2 re-checks the fixes.

**Status (27 November 2026, 23:10 UTC): round 1 done. D02's freeze is held.**
- Part 1 found 5 blockers, 16 majors and 4 minors.
- Part 2 confirmed 7 of them in the product window and found 8 new majors.
- The fix round starts after the weekly quota reset (30 November 2026, 14:00 UTC). Round 2 re-checks it.

## The calls
- **Part 1, the static review.**
  - Wrapper call `20261127T220428Z-83c606d5`. Its record is under `work/reviews/`, owned by the wrapper and ignored
    by Git.
  - Model gpt-6.2-astra at effort xhigh, fast tier, read-only sandbox.
  - Head `10da864b`, scope `d18b908a87..10da864b`: 260 commits, 453 files.
  - Ran 22:04 to 22:13 UTC. The record is valid; verdict `findings`.
- **Part 2, the product window with Computer Use.**
  - The owner runs it in the Codex app: gpt-6.2-astra, Extra, Standard tier, "Ask for approval".
  - It runs in a throwaway clone at `10da864b`, with no remote, holding the gate sitting's product build
    `c9-product-1a6a967326354a35` and the session fixtures. Limits: 2 hours and 40 screenshots.
  - The packet sends it the same forbidden reads as part 1, plus every file named `defects.json` in the clone. The
    launch checks every file of the build, so the build's copy stayed in place unopened.
  - It ran about 22:20 to 23:06 UTC: 39 minutes of product testing and 15 screenshots. Its result validates against
    `schemas/review-result.schema.json`; verdict `findings`.
  - It stopped early under the packet's safety rule. After Alt+F4, Windows reported the deep-2 window as not
    responding (D02-CU-01); the process then exited with code 0, and no black screen was seen.
  - The System log from 22:15 to 23:08 UTC holds no nvlddmkm, display or LiveKernelEvent entry. The Application log
    holds no Windows Error Reporting hang report.
  - The result and the run folders stay in the clone (`work/astra-d02/repo/work/astra-review/`, ignored by Git) until
    the fix round has read them.

## Part 1's findings and their dispositions
- Every finding is accepted on Astra's evidence. Astra marked each one verified: the source and an in-memory
  counterexample, with no product window.
- D02-05 is also confirmed here: the kept build holds `shell/project/samples/jumble/defects.json`. No C# source of the
  shell or the chrome library reads that file.
- Each fix lands with a test that reproduces the counterexample before the fix.

| Id | Severity | Finding | Disposition |
| --- | --- | --- | --- |
| D02-01 | blocker | A whole-only repair commits as separate durable twists (`ShellPreview.cs:300`). | The single-transaction word commit that ADR-D01 rev6 requires needs a command-layer row, which D02 does not patch: row request to the top integrator. In the fix round, the live repair preview stays readable and its Commit is disabled until the row lands. |
| D02-02 | blocker | The executed witness can differ from the picked row (`RepairList.cs:201`). | Fix round: derive the witness from the picked tuple; check supplemental records against it. |
| D02-03 | blocker | A fixture-writer refusal deletes the directory it refused to replace (`build_d02_fixtures.py:146`). | Fix round: build in a uniquely owned staging folder, verify, then publish and keep the last good copy. |
| D02-04 | blocker | The reopen check deletes a pre-existing sibling folder (`c600-shell-session-fixture.rs:120`). | Fix round: create an exclusive scratch folder and remove only that one. |
| D02-05 | blocker | The normal product build packages Track J's defect list (`build_shell.py:73`). | Fix round: exclude harness-only files at both staging points and check the manifest. Also check whether any J0 run's agents could read the product folder. If they could, those J0 results are void and J0 runs again. |
| D02-06 | major | A protected piece stays movable by a live repair. | Fix round: one protection source for the palette, the status drawing and the repair check. |
| D02-07 | major | A second pick appends to the first repair's witness. | Fix round: a pick replaces the pending live repair. |
| D02-08 | major | Repair results are not matched to the requested cap and revision. | Fix round: compare input digest, cap, menu pin, tie rule and revision with the active request. |
| D02-09 | major | Holding a seat creates twelve twist trials the solver did not release. | Fix round: the twelve-event rhythm stays in the test driver only. |
| D02-10 | major | The log tab reports ν coordinates as orbit residuals. | Fix round: the 35 residual levels in rail order, as rev6 specifies; correct the console test. |
| D02-11 | major | The word panel shows reachability without reachability evidence. | Fix round: pass the verified reachability result separately. |
| D02-12 | major | The occlusion rescore can count an old selection as the current click's result. | Checker fix, then rescore from the saved observations if they suffice, otherwise rerun at the next owner sitting. |
| D02-13 | major | The rescore can validate an incomplete grid under the wrong preset. | As D02-12, with a per-preset grid check. |
| D02-14 | major | A stall across the measurement boundary disappears from the frame rate. | Checker fix; recompute the rows from their saved frame records. |
| D02-15 | major | A performance row can pass without a VRAM reading. | Checker fix: a row needs an in-window VRAM reading. Rows without one rerun. |
| D02-16 | major | G3 measures the certificate count without drawing the glyphs. | G3 is recorded as count-only until a glyph-drawing step exists. The step is added and G3 reruns. |
| D02-17 | major | G8's documented command leaves the core simulation matrix off. | Set the core switch for G8, check the matrix, rerun G8. |
| D02-18 | major | G1 and G9 can pass while the contact tint is gone. | Validity checks the tint across the sweep; G1 and G9 rerun. |
| D02-19 | major | J1's 56-of-56 denominator omits a registered D02 command. | Fix round: count every registered section; J1 reruns. |
| D02-20 | major | J0 can report PASS before its researcher checks run. | Fix round: the combined result stays incomplete until the researcher's checks run. |
| D02-21 | major | J0 credits an unrelated free-text explanation as a caught defect. | Fix round: free-text matches stay pending until checked. |
| D02-22 | minor | Local auto-framing uses the cell from an earlier pick revision. | Fix round. |
| D02-23 | minor | The gate record omits the build identity. | Fix round: carry the build identity and run id; the rescorer checks them. |
| D02-24 | minor | The window checks cannot show the shell was never topmost. | The results page's wording becomes "not topmost at any check". |
| D02-25 | minor | A correction multiplicity can overflow. | Fix round: widen before the absolute value; validate the total. |

## Part 2's verdicts on part 1's findings
- **Confirmed in the window (7):**
  - D02-01: the run's journal holds three separate transactions for one repair.
  - D02-05: by path only.
  - D02-06: 3,600 protected pieces, then 36 conflicting movers, and the preview still offered Commit.
  - D02-07: a second pick grew the preview from three letters to six.
  - D02-09: one seat hold made twelve trials.
  - D02-16: G3's fallback counts certificates and defers the glyphs.
  - D02-18: the contact tint is lost on adoption.
- **Not observable in the window (18):** the other findings concern fixture writers, the gate scorers, Track J or
  altered inputs. None was refuted.

## Part 2's new findings

| Id | Severity | Finding | Disposition |
| --- | --- | --- | --- |
| D02-CU-01 | major | The deep-2 window stopped responding after Close. | Fix round: reproduce the shutdown at a deep state with the host's own timing, then fix what blocks the close. |
| D02-CU-02 | major | Solve staged an unrelated sample word, with Commit available, when given a copied research reference that Research says solving previews refuse. | Fix round: Solve refuses a research reference, as Research says; a sample word never stages from one. |
| D02-CU-03 | major | A script's ring annotation draws over Local's controls. | Fix round: script annotations are clipped to the view they name. |
| D02-CU-04 | major | Some listed repairs have no inspectable moving set for their second step. | Fix round: every listed row's steps carry their moving set. A row without one says why. |
| D02-CU-05 | major | Opening Research at 1280×720 collapses Global to zero height. | Fix round: one layout pass for 1280×720 with minimum view sizes, together with D02-CU-06. |
| D02-CU-06 | major | Staging a repair pushes an open filter dock past the right edge. | Fix round, in the same layout pass. |
| D02-CU-07 | major | A research run cannot open its derived state in a free view (ADR-D02.45; CR-17 refusal). | The framework records this as deferred. The freeze card puts it before the owner: build it in the fix round, or carry it to K2 with the refusal. |
| D02-CU-08 | major | The checker trace cannot be opened from Research (CR-17 refusal). | The framework records the debugger path as deferred. The freeze card handles it as D02-CU-07. |

Part 2 found these clean, on the named fixtures and actions only:
- cap naming and the repair list's neutral order (129 rows, ascending (x, d), unsorted ΔN, no preview until a pick);
- the repair preview's scrub, overlay, Escape and settle controls;
- the focus layer;
- the view overlays under a filter;
- the cut slice;
- the raw and antipodal views;
- the ν corrections;
- research runs with a declared budget, and their obligations;
- the research claims;
- tuning save and reset;
- the documents surface;
- the cap map, threshold and prompt;
- the cap covers;
- the deep charts.

Across every view-only replay, the revision and digest stayed put.

## What this changes elsewhere
- **The gate results** (`docs/progress/1.0/adr-d02-gate-results.md`): D02-12 to D02-18, D02-23 and D02-24 question
  what the 27 November records establish. Until the checkers are fixed and the rows rescored or rerun, the 11-of-11
  result is recorded but not final.
- **Track J**: D02-19 to D02-21 question J1's 56 of 56 and J0's combined result, and D02-05 may void J0's runs. Both
  stay under review until the fix round.
- **Row request**: D02-01 needs a command-layer row for the single-transaction commit of a whole-only word. It goes
  to the top integrator.
