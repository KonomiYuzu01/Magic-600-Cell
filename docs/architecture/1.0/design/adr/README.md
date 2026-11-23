# ADR-D01: the design system

Status: **Accepted** (owner, 23 November 2026, at A7; [freeze record](../../../../wiki/decisions/owner-decisions-2026-11-23-a7-freeze.md)).
A7 is UI/UX freeze round 1 of the [design and art path](../../../../wiki/decisions/owner-decisions-2026-11-22-design-art-path.md).

What is frozen:
- Every item tagged (framework).
- Revision 5's document forms, ADR-D01.79 to .84, which carry no tags.
- Every slot name.

What is not frozen:
- Defaults, tuned values and open items. They are set in freeze rounds 2 to 4.

## The record

ADR-D01 is one record in four revisions, plus its dependency list:

| File | Revision | Items |
| --- | --- | --- |
| [ADR-D01-design-system.md](ADR-D01-design-system.md) | 3a: the frame, the default set, the owner overrules | ADR-D01.1 to .60 |
| [ADR-D01-rev4-solving-surface.md](ADR-D01-rev4-solving-surface.md) | 4: the solving surface (section 15) | ADR-D01.61 to .78 |
| [ADR-D01-rev5-documents.md](ADR-D01-rev5-documents.md) | 5: documents and the log file | ADR-D01.79 to .84 |
| [ADR-D01-rev6-a6-fixes.md](ADR-D01-rev6-a6-fixes.md) | 6: fixes from the A6 review and its re-check | amends .1 to .84 |
| [ADR-D01-dependencies.md](ADR-D01-dependencies.md) | the product dependencies the owner approved on 22 November 2026 | |

Where two revisions differ, the later one wins: 6 over 5, 5 over 4, and 4 over 3a. Earlier files were not edited in
place. They were written as private drafts and are published at A7 unchanged in substance. Only two kinds of edit
were made for publication:
- The status lines were updated.
- References to private working files were removed, and the owner's words are given in English.

## How it was built and reviewed

- **A5.** Fable built the default set in Look Lab (`tools/looklab/`, `renderer/shell/`; pull request 424).
- **A6.** One owner-invoked review by Codex Astra (`gpt-6.2-astra`, effort xhigh) in the Codex desktop app.
  - It used Computer Use on Look Lab, on 23 November 2026, call `20261123T000053Z-01a205b4`.
  - It returned 29 findings: 3 blocker, 18 major and 8 minor.
- **Re-check.** A scoped re-check without Computer Use, call `20261123T024528Z-01a2064b`.
  - It returned 1 blocker and 15 major findings, and every one was answered `adopt`.
  - The record fixes are revision 6, checked by a fresh Claude subagent.
  - The build fixes are in Look Lab, one test per finding (`tools/looklab/tests/A6FixTests.cs`, README section
    "A6 scoped re-check: build fixes").
- The review records are private (`work/reviews/`).

The Look Lab build is a reference build, not the product. Its full acceptance moves to the stage 2 greybox check in
K2 (owner, 23 November 2026).
