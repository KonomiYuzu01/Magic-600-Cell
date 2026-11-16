---
id: lg3-ambient-read-code
type: question
status: draft
visibility: public
summary: Contradiction - LX-3's frozen corpus expects `LG3:capability.ambient_read` for an ungranted clock, file or network read (P01 to P03), while chapter 13 keeps that row as not yet used and the LP2 checker refuses those reads with `LG3:capability.not_granted`; the code needs a decision.
tags: [contradiction]
related: [owner-decisions-2026-11-14-language-route, owner-decisions-2026-11-15-freeze]
supersedes: []
claims:
  - {id: registry-row-not-yet-used, evidence_kind: source, path: docs/architecture/1.0/language/architecture/13-errors.md, line: 396, checked_at: 2026-11-16}
  - {id: not-granted-rule-source, evidence_kind: source, path: docs/architecture/1.0/language/architecture/13-errors.md, line: 395, checked_at: 2026-11-16}
  - {id: codes-frozen-with-corpus, evidence_kind: source, path: docs/architecture/1.0/language/architecture/11-route-and-spikes.md, line: 486, checked_at: 2026-11-16}
  - {id: corpus-expects-ambient-read, evidence_kind: fixture, path: work/experiments/language-lx/lx3-sandbox-flow/corpus/planted.json, checked_at: 2026-11-16}
  - {id: rt1-run-gave-ambient-read, evidence_kind: source, path: work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt1-own/RESULT.md, checked_at: 2026-11-16}
  - {id: checker-gives-not-granted, evidence_kind: source, path: rust/c600-module/tests/ct_rte_07_full.rs, checked_at: 2026-11-16}
  - {id: lx3-checks-are-lp3-acceptance, evidence_kind: decision, checked_at: 2026-11-14}
---

# Which code refuses an ungranted clock, file or network read?

Status: **open.** Found while the LP2 checker was extended on 16 November 2026. This page records the conflict; it
changes no rule and accepts no decision.

## The conflict

- Chapter 13 registers `LG3:capability.ambient_read` for a clock, file or network read requested without a grant
  (rule source INV-10, default 2 and LX-3), and its Used-by cell reads "Round 3; not yet used as a concrete code"
  ([13-errors.md](../../architecture/1.0/language/architecture/13-errors.md), line 396). Under ERR.23 that note keeps
  an entry that no chapter emits yet; the row is neither reserved nor retired.
- The row above it, `LG3:capability.not_granted`, names LG3.8 and LG3.9 in its rule source (line 395). LG3.8 makes
  files and the clock explicitly granted capabilities and grants no network or process access in 1.0; LG3.9 makes a
  missing grant a typed refusal.
- LX-3's frozen corpus expects `LG3:capability.ambient_read` for the planted clock, file and network probes P01 to P03
  and `LG3:capability.not_granted` for the process probe P04
  ([planted.json](../../../work/experiments/language-lx/lx3-sandbox-flow/corpus/planted.json)). RTE.45 freezes the
  expected registry codes with the corpus, and no case is excused after observing its result
  ([11-route-and-spikes.md](../../architecture/1.0/language/architecture/11-route-and-spikes.md), line 486). The RT1
  candidate's LX-3 run refused P01 to P03 with `ambient_read` from its shared policy layer
  ([RESULT.md](../../../work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt1-own/RESULT.md)).
- `c600-check` refuses the same reads with `LG3:capability.not_granted`: LG3.8 for network access, LG3.9 for a clock
  or file capability the run does not grant. CT-RTE-07 on 600-cell-Full lists P01 to P03 as keys whose code differs
  from the corpus's (`WRONG_CODES` in [ct_rte_07_full.rs](../../../rust/c600-module/tests/ct_rte_07_full.rs)). The
  refusals hold: approval refuses each probe before any trap is called, and no planted data appears.

## What needs deciding

1. Whether the checker raises `LG3:capability.ambient_read` for an ungranted clock, file or network read. P01 to P03
   would then match the frozen corpus, and chapter 13's Used-by note would be out of date; a change to frozen text
   follows the revision, review and owner acceptance process of PHS.52.
2. Or whether `LG3:capability.not_granted` stays the code. CT-RTE-07's P01 to P03 keys then miss the corpus's frozen
   codes. The remaining LX-3 checks are LP3 acceptance checks with unchanged bars, and a failing LX-3 check goes to the
   owner ([14 November 2026](../decisions/owner-decisions-2026-11-14-language-route.md)).

Either way only the code differs; the refusal and its effect on the session stay the same.
