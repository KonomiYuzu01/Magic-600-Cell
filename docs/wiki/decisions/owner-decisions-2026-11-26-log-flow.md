---
id: owner-decisions-2026-11-26-log-flow
type: decision
status: verified
visibility: public
summary: Owner decision of 26 November 2026 - the log and document flow becomes the sixth flow of stage 2's usable acceptance, beside the five flows of 22 November 2026; LP6 starts when the ADR-D03 and K2 lines start, not before.
related: [owner-decisions-2026-11-22-design-art-path, owner-decisions-2026-11-22-adr-line-evening, owner-decisions-2026-11-23-a7-freeze, owner-decisions-2026-11-26-lq2]
supersedes: []
claims:
  - {id: log-flow-sixth, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: lp6-start-with-d03-k2, evidence_kind: decision, checked_at: 2026-11-26}
---

# Owner decision, 26 November 2026: the log flow and LP6's start

**How the decision came about.**
- The owner asked whether the ADRs plan the 1.0 log file's front end.
- The integrator answered from ADR-D01.79 to .84, which A7 froze on 23 November 2026. The plan has three parts:
  - the signature tabs, the compile lamp, the Problems tab, the overflow list and the split compare;
  - ADR-D03 builds the tabs on sample data;
  - K2's greybox stage wires the rows to the backend.
- LP4 built the backend (`c600-log` and the document service in `c600-session`).
- Three gaps were found:
  - ADR-D03 has not started;
  - the five flows of stage 2's usable acceptance ([22 November 2026](owner-decisions-2026-11-22-design-art-path.md))
    do not include the log, so no check asks that K2 wires it;
  - some backend parts are not built (below).
- The integrator recommended adding the log as a sixth flow. It also described LP6 and recommended starting LP6's
  importer now.
- The owner answered in chat (in Chinese), translated: "Agreed. LP6's start is not urgent; do it once the ADR-D03 and
  K2 lines have started."

## Decisions

1. **The log and document flow is stage 2's sixth flow.**
   - It joins the five flows that stage 2's usable acceptance runs end to end in the product shell, each control
     invoking a command-table row:
     - one cap twist with preview and commit;
     - building one block orbit-first under protection;
     - writing and running a `.6cs` macro;
     - the Jumble repair flow;
     - the research output panel.
   - The sixth flow: create a log or a script, save it, compile it, open a saved log, and make it current through the
     `file.log.import` commit. It runs through the `file.document.*` and `file.log.*` rows with their gates, in the
     document tabs of ADR-D01.83.
   - This adds a check and relaxes none.
2. **LP6 starts when the ADR-D03 and K2 lines start, not before.**
   - LP6's start conditions are met since LQ2's joint setting
     ([26 November 2026](owner-decisions-2026-11-26-lq2.md)).
   - LP6's products are the path B importer, packaging, complete documentation, the boundary policy as decided
     (policy table version 1) and the owner-machine gates.
   - The owner placed the importer in LP6 by this answer: its CT halves are indexed with it.

## What follows

- **K2.** The greybox stage takes the sixth flow into its acceptance. The flow needs backend parts that are not built:
  - converting a session with Jumble on into a log (`KN5:input.row_not_built` today);
  - restoring open documents at start with a changed-on-disk mark;
  - a host that installs the document service in the product shell.
  These are indexed with K2's greybox stage. Applying a log's recorded view, layout and workspace stays recorded
  values (`rust/README.md`, LP4-12 and LP4-14).
- **ADR-D03** builds the document tabs on sample data, as ADR-D01.84 says.
- **LP6** waits. The integrator proposes its start when the ADR-D03 and K2 lines start.
- **Briefing.** The briefing records both decisions. Its pin refresh under FP.1 is recorded on the
  [LQ2 page](owner-decisions-2026-11-26-lq2.md).

## What this does not do

- It moves no bar, relaxes no gate, and accepts no ADR.
- It edits no frozen text: ADR-D01.84 already assigns the tabs and the wiring.
