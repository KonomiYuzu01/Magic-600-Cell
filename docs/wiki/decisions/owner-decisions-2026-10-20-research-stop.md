---
id: owner-decisions-2026-10-20-research-stop
type: decision
status: verified
visibility: public
summary: Owner decision of 20 October 2026 - the jumbling research stops once the work in flight is done, because it was progressing too slowly; the tutorial is not accepted, none of the 21 acceptance scrambles has run and packets P1 to P5 stay unapplied; on 21 October the owner asked for the handoff entries of the other lines, which do not restart the research.
related: [owner-decisions-2026-10-20-tutorial-acceptance, owner-decisions-2026-10-18-machine-tutorial, owner-decisions-2026-10-18-bprime-goal, owner-decisions-2026-10-17-jumbling-pr]
supersedes: []
claims:
  - {id: research-stop, evidence_kind: decision, checked_at: 2026-10-20}
  - {id: research-stop-state, evidence_kind: source, checked_at: 2026-10-21}
  - {id: research-stop-handoff, evidence_kind: decision, checked_at: 2026-10-21}
---

# Owner decision, 20 October 2026: the jumbling research stops

The owner gave this decision in a chat message in Chinese; this page records it in English.

## Decision

Once the work in flight is done, every task of the jumbling research stops. The owner's reason: the research was
progressing too slowly.

## The state at the stop

The work in flight was finished and recorded on the branch of draft pull request #112:
- the R22D-25 kernel with D19's program, run at six height-4 stops: both were stuck at all six, with no step
  ([stage 12](../../../research/jumbling/restoration/12-low-height/README.md), section 11);
- packets P1 to P5 of the tutorial runner, implemented by Codex and reviewed by the integrator; one `major` finding,
  P3-falsepass-01, is open, and the patches are kept, not applied
  (stage 13, section 8: `research/jumbling/restoration/13-tutorial-runner/README.md`, on #112's branch only).

Not started: I1, the integration of the five packets, and with it every run of the runner with the existing rules
connected (stage 13, section 8). None of the 21 scrambles of the
[acceptance test](owner-decisions-2026-10-20-tutorial-acceptance.md) has run, so the tutorial is not accepted. No rule
yet always takes height 4 to height 2, and the coverage of B′(2) is open, its round 9 held
([record](owner-decisions-2026-10-18-machine-tutorial.md)). The recurring check for new senior-review material
([record](owner-decisions-2026-10-18-bprime-goal.md), item 4) no longer runs, and no research round, run or scheduled
check-in remains.

## Not decided

Asked afterwards what comes next, the integrator proposed three steps: fix P3-falsepass-01, carry out I1 and run the
shallow acceptance batches; a search at the six stuck height-4 stops, limited to two working days; then an owner
choice between limiting the depth of the acceptance test and pausing the jumbling work to return to the 1.0 main line.
The owner has not chosen, and none of it has started.

## The handoff entries (owner, 21 October 2026)

The owner then asked for the handoff entries for the other lines, written to a finished standard: the entries dated
21 October 2026 on the five pages of `docs/progress/1.0/jumbling-handoff/`, which record the work of 18 to 20 October
and the state after the stop. Writing them does not restart the research.

## Effect

- Nothing of the research restarts until the owner says so.
- Draft pull request #112 stays a draft: its merge conditions ([record](owner-decisions-2026-10-17-jumbling-pr.md))
  include a tutorial that has passed the owner's acceptance, so its pages stay on its branch and not on `main`.
- Integrator reading for the handoff pages: a line's own stage 2 work does not wait on the research unless its page
  says why.
