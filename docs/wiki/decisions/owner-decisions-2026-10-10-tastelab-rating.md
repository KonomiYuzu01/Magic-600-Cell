---
id: owner-decisions-2026-10-10-tastelab-rating
type: decision
status: verified
visibility: public
summary: Owner decision of 10 October 2026 - look tuning keeps pairwise comparisons; image ratings gain a third level, love (Up), next to like (Right) and dislike (Left); existing ratings and exports stay valid unchanged; every fitting step treats love as like; love puts candidates first for annotated references, nexus cards and pair notes; class B rules unchanged.
related: [owner-decisions-2026-10-03-taste-lab, owner-decisions-2026-10-03-image-library]
supersedes: []
claims:
  - {id: looks-stay-pairwise, evidence_kind: decision, checked_at: 2026-10-10}
  - {id: three-level-image-rating, evidence_kind: decision, checked_at: 2026-10-10}
  - {id: love-counts-as-like, evidence_kind: decision, checked_at: 2026-10-10}
  - {id: class-b-unchanged, evidence_kind: decision, checked_at: 2026-10-10}
---

# Owner decision, 10 October 2026: Taste Lab rating levels

The owner gave this decision in a chat message in Chinese. This page records it in English.

## Decision

1. **Look tuning stays pairwise.** Phase 1a keeps its pairwise comparisons, and its learner does not change. An acceptability model stays a later option and is not built now.
2. **Image ratings get three levels.** In the page's Images tab and in the local rating window (`rate.py`, `Rate.qml`), the ratings are Left dislike, Right like and Up love. Down skip, the note key and undo do not change. There is no 1-to-5 star scale.

## Requirements

- **Existing ratings stay valid.** No rating is converted or asked again. Earlier export files and the ratings already stored in the page are read as before.
- **Love counts as like.** Every existing fitting step treats love as like, so current model behaviour does not change. These are the page's logistic regression, `learn.py`, the taste map, the term ranking and the proposals. A higher weight for love may be proposed in the plan.
- **Love sets priorities.** Love is also used to put candidates first for annotated references, nexus cards and pair notes.
- **Class B rules are unchanged.** No class B rating, love included, enters any fitting step, and the canary tests cover the new value. The page and the export bundles still accept class A only.
- **Process.** The new value, the export and import, and the read-back of the page's stored ratings are behaviour and contract changes. They get a plan check before implementation and one review of the finished candidate. Separable parts go to Codex.
- **Checks.**
  - `tests/test_tastelab_images.py --require`: all 11 modules pass, and the exit code is checked.
  - `tests/test_tastelab.py`, including the page's node tests.
  - The wiki lint.
- **Running work.** The running image refill and audit are not interrupted. The automatic bundle export does not load half-changed files.
- **Publishing.** The owner is asked before the updated Taste Lab page is published.
