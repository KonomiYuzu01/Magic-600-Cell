---
id: owner-decisions-2026-11-23-a7-freeze
type: decision
status: verified
visibility: public
summary: The owner froze A7 on 23 November 2026 - ADR-D01, the design system, is accepted at revision 6 as UI/UX freeze round 1 (its framework items and slot names, not its values), after A5's Look Lab build, the A6 Astra review with Computer Use and one scoped re-check; the Look Lab fixes land with it, and the build's full acceptance moves to the stage 2 greybox check.
related: [owner-decisions-2026-11-22-adr-line-evening, owner-decisions-2026-11-22-adr-d01-overrules, owner-decisions-2026-11-22-design-art-path]
supersedes: []
claims:
  - {id: a7-frozen, evidence_kind: decision, checked_at: 2026-11-23}
  - {id: a6-reviews, evidence_kind: decision, checked_at: 2026-11-23}
  - {id: build-acceptance-at-greybox, evidence_kind: decision, checked_at: 2026-11-23}
  - {id: one-pr-merged-now, evidence_kind: decision, checked_at: 2026-11-23}
---

# Owner decisions, 23 November 2026: A7, the design system frozen

The owner gave these answers in the ADR line's sessions on 23 November 2026. This page records them in English.

What this page does not do:
- It freezes no default, tuned value or open item of ADR-D01. Those come in freeze rounds 2 to 4.
- It edits no frozen language text. Where ADR-D01 conflicts with a frozen clause, the
  [overrules](owner-decisions-2026-11-22-adr-d01-overrules.md) of 22 November 2026 apply.
- It moves no gate or bar.

## Decisions

1. **A7 is frozen: ADR-D01 is accepted.**
   - The owner's words, about 04:05 UTC: "freeze A7, open the PR now and merge it at once".
   - What is frozen. This is UI/UX freeze round 1 of the [design and art path](owner-decisions-2026-11-22-design-art-path.md):
     - the items tagged (framework);
     - revision 5's document forms;
     - every slot name.
   - Revision 6 wins where the revisions differ.
   - Where the text is: [docs/architecture/1.0/design/adr/](../../architecture/1.0/design/adr/README.md), in four
     revisions (3a, 4, 5 and 6) and the dependency list.
   - Why it is published now. The drafts stayed private until A7. They are published unchanged in substance.

2. **The A6 review and its re-check.**
   - **The review.** Codex Astra (`gpt-6.2-astra`, effort xhigh) reviewed the record and A5's Look Lab build in the
     Codex desktop app, with Computer Use on Look Lab.
     - It was the line's one Codex call under the Codex minimum, owner-invoked, on 23 November 2026.
     - It returned 29 findings: 3 blocker, 18 major and 8 minor.
   - **The fixes.** The owner asked for the fixes at once, then one more round without Computer Use, then the freeze.
   - **The re-check.** It returned 1 blocker and 15 major findings, and every one was answered `adopt`:
     - the record items in revision 6, read by a fresh Claude subagent;
     - the build items in Look Lab, with a test for each.
   - **No further round.** The owner asked for no further Codex round.
   - **Cost.** The first run used up the Codex five-hour window, then owner-approved ChatGPT credits. The amounts are
     in the private ledger.

3. **The build's full acceptance moves to the stage 2 greybox check.**
   - The owner: the complete first version is checked when stage 2 is fully frozen, which means:
     - the greybox wiring in K2 is done;
     - the language is finished;
     - the ADRs are fully frozen.
   - So A7 did not wait for the Look Lab build.
   - The Look Lab fixes still land now. Their checks pass: LookLab.Tests 149 of 149, and `check.py --godot`.

4. **One pull request, merged at once.** It carries:
   - the Look Lab fixes;
   - the published ADR-D01;
   - this record.

## What follows

- **For the integrator.**
  - The briefing still lists the design-system ADR as open, with its 22 November bullet. Both need an edit with
    the FP.1 pin refresh.
  - Revision 4's public-line edits are still open (the solving workflow and the glossary).
  - So are the command-table rows that the revisions name: `session.branch.replay`, `preview.side.*`, the document
    rows of ADR-D01.84, and the rows Look Lab proposes (`library.branch.toggle`, `layout.roll`, `history.show`,
    `history.entry.toggle`). They are added at the command-table freeze.
- **The rest of the path.**
  - ADR-D02, the Jumble and research surfaces, is drafted next.
  - K2 opens after A7 and ADR-D02.
  - The look-cost envelope goes into the core as one PHS.52 revision.
- **Still open for the owner.** Who builds the ADR-D02 and ADR-D03 shells, and on what budget.
