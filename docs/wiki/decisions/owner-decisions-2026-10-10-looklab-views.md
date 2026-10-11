---
id: owner-decisions-2026-10-10-looklab-views
type: decision
status: verified
visibility: public
summary: Owner decision of 10 October 2026 - the Look Lab gets two default greybox layouts that show Global and Local together and linked, a left-right split and a main view with a picture-in-picture inset; the two existing layouts stay; the layouts are revised when the engineering and design tracks join.
related: [owner-decisions-2026-10-02-design]
supersedes: []
claims:
  - {id: two-view-default-layouts, evidence_kind: decision, checked_at: 2026-10-10}
  - {id: revised-when-tracks-join, evidence_kind: decision, checked_at: 2026-10-10}
---

# Owner decision, 10 October 2026: Look Lab Global and Local layouts

The owner gave this decision in a chat message in Chinese. This page records it in English.

## Decision

1. **Build the default layouts now.** The Look Lab gets two greybox layouts that show the two 1.0 views, Global and Local, at the same time: a left-right split (`split-views`) and a main Global view with a Local picture-in-picture inset (`focus-inset`).
2. **Revise them later.** These are defaults for the G1 flows. They are revised when the engineering and design tracks join the Look Lab work.

## Requirements

- **The views follow the design decision.** Global and Local are visible together and linked, sharing one context and revision, with no third view ([owner-decisions-2026-10-02-design](owner-decisions-2026-10-02-design.md)).
- **The existing layouts stay.** `central-stage` and `docked-workbench` and their behaviour do not change. With the two new layouts the Look Lab has four layout structures; before this decision its plan named two or three.
- **Process.** The layout format, the second view and the framing are behaviour changes. They get a plan check before implementation and one review of the finished candidate. No performance claim is made.
