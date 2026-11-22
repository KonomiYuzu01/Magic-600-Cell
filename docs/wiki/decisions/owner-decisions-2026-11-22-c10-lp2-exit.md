---
id: owner-decisions-2026-11-22-c10-lp2-exit
type: decision
status: verified
visibility: public
summary: Owner sign-off of 22 November 2026 on one card for both exits - LP3's exit (C10) on 013900ff and LP2's exit on 0260e6b1 are signed, and the four readings of the tests that did not run whole take the recommended options - (a) the on-device and owner-machine halves move under PHS.52 to the owner-machine sitting of the post-freeze on-screen line, (b) BM5's review halves are indexed where the tools ship, (c) B12's five warm cache states are not applicable while no cache exists, (d) PHS.23 item 1's open halves are indexed with the work that builds them.
related: [owner-decisions-2026-11-22-lx-bars-freeze, owner-decisions-2026-11-22-adr-d01-overrules, owner-decisions-2026-11-20-lp3-exit, c9-sitting-results]
supersedes: []
claims:
  - {id: c10-signed, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: lp2-exit-signed, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: exit-readings-a-to-d, evidence_kind: decision, checked_at: 2026-11-22}
---

# Owner decision, 22 November 2026: LP3's exit (C10) and LP2's exit

On 22 November 2026 the integrator sent the owner one card for both exits. The card came after LP3's single pull
request (417) reproduced C10's index at a comment-only commit on the frozen candidate. The owner answered in chat: sign,
with every reading as recommended. This page records the answer in English. It moves no bar and edits no frozen text.

## What was signed

1. **LP3's exit (C10) on `013900ff`**, the merge of pull request 393. The exit is judged on this candidate under
   [item 6 of the LX decision](owner-decisions-2026-11-22-lx-bars-freeze.md); the
   [LP3 plan](../../progress/1.0/language-lp3-plan.md), sections 9 and 11, has the detail.
   - H-08 is frozen with 175 commands (PHS.53).
   - PHS.23's six acceptance items have evidence on `013900ff`. The [C9 sitting's records](../evidence/c9-sitting-results.md)
     and pull request 411's analysis under items 1 to 5 and 10 give:
     - E2 and E3 met;
     - LX-2 and LX-4 met on the confirmed samples, with the confirmation gap published as a known limit;
     - the renderer gate met outside the four fault windows where the runtime is lost.
   - CI:
     - Linux: push run 44850916378 on `013900ff`.
     - Windows: pull-request run 44847327728, whose checkout has `013900ff`'s tree (`3995eb5e`).
   - The index is read at `798dc877a768e67bc5b9562a979df997c50cb10f`, a commit on `013900ff` that adds only comment
     lines (PHS.62 markers) and `docs/progress/1.0/lp3-ct-index-notes.md`.
     `python tools/language/ct_index.py --phase LP3` there gives 82 IDs: 61 whole (19 by union), 11 part, 0 named
     only and 10 missing. The readings below cover the 21 tests that are not whole.
2. **LP2's exit on `0260e6b1`.** The exit record is section 12 of the
   [LP2 plan](../../progress/1.0/language-lp2-plan.md), from pull request 410:
   - lp2-full run 44871666684 passed;
   - PHS.19's items have evidence;
   - the index has 248 IDs: 126 whole (32 by union), 7 not applicable, 8 waiting and 107 indexed elsewhere.

## The readings of the tests that did not run whole

| Reading | Tests | Decided |
| --- | --- | --- |
| (a) | LP3: 16 tests whose device half needs a harness that `013900ff` lacks (CT-ONE-26, CT-RPI-13, CT-RPI-21, CT-RPI-24, CT-RPI-31, CT-RPI-33, CT-RPI-34, CT-RM-05, CT-RM-06, CT-RM-10, CT-BND-20, and the device halves of CT-EV3-08, CT-RPI-08, CT-RM-09, CT-LB4-04 and CT-RPI-32); CT-ONE-25 and CT-RPI-26, whose E2 runs had no tool views open. LP2: reading 17's owner-machine IDs CT-KN4-01, CT-KN5-05, CT-LG4-08, CT-PS1-05, CT-ONE-20, CT-DET-05, CT-DET-07 and CT-DET-16 | An index change under PHS.52. These halves move to the owner-machine sitting of the line that builds the on-screen contract after the freeze. C9's parts stay on record. Nothing in them counts as passed in LP3 or LP2 |
| (b) | CT-BND-15 and CT-LB6-03 | BM5's review halves are indexed where the tools ship, as LP2's readings 9 to 16 do |
| (c) | The five B12 cache states of CT-RTE-07 other than cold | Not applicable while the decision of [20 November 2026](owner-decisions-2026-11-20-lp3-exit.md) stands (no row-answer cache). This is no pass (PHS.63), and the states reopen if a cache is built |
| (d) | PHS.23 item 1's open halves: the three tools' entries through the key route on Windows, and the jumbling tool's entries through the command and source routes with their program-job route | Each half is indexed with the work that builds it |

The owner did not take either alternative:
- Against (a): build the harnesses and RPI.77's quarantine, then make a new candidate and hold a short sitting. That
  would have reversed item 6 of the LX decision.
- Against (d): write headless tests for the jumbling tool's entries before signing.

## What follows

- LP3's single pull request (417) merges once, with a merge commit, so `798dc877` stays reachable. Before it merges, it
  carries:
  - readings (a) to (d) as one notes row per ID;
  - LP2's eight IDs in LP2's notes;
  - CT-LG3-03 and CT-LB7-01 as whole by union with reading 18's undo fix, a post-exit index update that leaves LP2's
    count at `0260e6b1` unchanged;
  - the stutter fix under runtime loss (item 4 of the LX decision).
- ADR-D01's commit rows start ([ADR-D01 decision](owner-decisions-2026-11-22-adr-d01-overrules.md), item 4), each
  with KN5 boundary attack tests before it lands.
- Reading (a)'s halves wait for the post-freeze on-screen line. That line builds the device harnesses and RPI.77's
  quarantine, then runs the halves in one owner-machine sitting.
- The progress items for LP2's M8 and LP3's C5 to C10 are ticked with this page, the candidate and the C9 records as
  evidence.

## What this does not do

- It moves no bar or gate.
- It accepts no ADR. The design-system ADR stays open until A7.
- It changes neither candidate and no frozen text.
- It counts none of reading (a)'s halves as passed.
