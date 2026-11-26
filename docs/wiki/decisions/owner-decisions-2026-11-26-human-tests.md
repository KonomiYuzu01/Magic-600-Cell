---
id: owner-decisions-2026-11-26-human-tests
type: decision
status: verified
visibility: public
summary: Owner decision of 26 November 2026 on the LP4 human tests' results - the mathematician's post-run ruling M24 counts, so the human half of CT-EV5-05 passes; the material findings are fixed without a rerun, so CT-LB6-04 stays not met; the human-test line ends when its results pull request merges.
related: [owner-decisions-2026-11-25, owner-decisions-2026-11-24-lp4-lp5-exit, owner-decisions-2026-11-23-lp4-lp5-start]
supersedes: []
claims:
  - {id: m24-counts, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: no-rerun, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: line-ends-at-merge, evidence_kind: decision, checked_at: 2026-11-26}
---

# Owner decision, 26 November 2026: the LP4 human tests' results

**How the decision came about.**
- The owner ran the self-run sheet, version 2, on 26 November 2026, and LP4's line main graded it under form version 2
  ([LP4 human tests](../../progress/1.0/lp4-human-tests.md)).
- The mathematician graded it independently and made ruling M24 after the run: in `full_600_cell`'s part (d), the clause
  that a preview is not an execution counts as a limit. Under the key's rule M3 as written, that part is only partly
  met, and the human half of CT-EV5-05 is not met; under M24 it passes.
- A post-run ruling that turns a not-met into a pass, on the owner's own result, is the owner's call, so the results
  page first recorded that verdict as open.
- The owner answered in LP4's line session (in Chinese): count M24 as the mathematician ruled, a pass; after the fixes,
  no retest; this line ends once its pull request merges. The owner then asked for these choices and the fixed data to
  go into the same pull request.

**What this page does not do.**
- It moves no gate and accepts no ADR.
- It changes no bar of the form: form version 2 stands, and the results stay human evidence only (EV5.4, EV5.16).

## Decisions

1. **M24 counts.** `full_600_cell`'s part (d) is yes, and the human half of CT-EV5-05 passes on the run of 26 November
   2026. CT-EV5-05 as a whole stays part until the shared grammar of EV5.11 is reviewed with the theory book.
2. **Fixes without a rerun.** The post-run findings that concern the product's records are fixed: M25 in
   `jumble_checker` and M30 in `representations`. The findings that concern only the sheet (M21, M23, M24's wording,
   M27, M29) go to any future sheet, and no sheet runs again now. CT-LB6-04 stays not met on the owner's run.
3. **The line ends.** The human-test line closes when the results pull request merges. CT-EV5-03 keeps its form and
   still waits for non-authors.
