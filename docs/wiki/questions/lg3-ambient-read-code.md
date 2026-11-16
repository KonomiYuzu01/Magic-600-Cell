---
id: lg3-ambient-read-code
type: question
status: verified
visibility: public
summary: Answered on 16 November 2026 - an ungranted clock read, file read or network request is `LG3:capability.ambient_read`, the code LX-3's frozen corpus expects for P01 to P03; process access, session-file access, randomness and a file that is only written keep `LG3:capability.not_granted`; chapter 13's Used-by note changes in the PHS.52 amendment at LP3's command-table freeze.
tags: [contradiction]
related: [owner-decisions-2026-11-14-language-route, owner-decisions-2026-11-15-freeze, owner-decisions-2026-11-16-phs52]
supersedes: []
claims:
  - {id: registry-row-not-yet-used, evidence_kind: source, path: docs/architecture/1.0/language/architecture/13-errors.md, line: 396, checked_at: 2026-11-16}
  - {id: not-granted-rule-source, evidence_kind: source, path: docs/architecture/1.0/language/architecture/13-errors.md, line: 395, checked_at: 2026-11-16}
  - {id: codes-frozen-with-corpus, evidence_kind: source, path: docs/architecture/1.0/language/architecture/11-route-and-spikes.md, line: 486, checked_at: 2026-11-16}
  - {id: corpus-expects-ambient-read, evidence_kind: fixture, path: work/experiments/language-lx/lx3-sandbox-flow/corpus/planted.json, checked_at: 2026-11-16}
  - {id: rt1-run-gave-ambient-read, evidence_kind: source, path: work/experiments/language-lx/lx3-sandbox-flow/part-b/candidates/rt1-own/RESULT.md, checked_at: 2026-11-16}
  - {id: lx3-checks-are-lp3-acceptance, evidence_kind: decision, checked_at: 2026-11-14}
  - {id: phs52-ambient-read, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: checker-gives-ambient-read, evidence_kind: source, path: rust/c600-check/src/lib.rs, checked_at: 2026-11-16}
  - {id: ct-rte-07-gives-corpus-codes, evidence_kind: source, path: rust/c600-module/tests/ct_rte_07_full.rs, checked_at: 2026-11-16}
---

# Which code refuses an ungranted clock, file or network read?

Status: **answered.** On 16 November 2026 the owner chose `LG3:capability.ambient_read`, the code LX-3's frozen corpus
expects, as one of the PHS.52 readings
([owner-decisions-2026-11-16-phs52](../decisions/owner-decisions-2026-11-16-phs52.md), section 3). The conflict was
found while the LP2 checker was extended the same day.

## The answer as carried out

- `c600-check` refuses with `LG3:capability.ambient_read` a clock or network capability the run does not hold, and a
  file capability the run does not hold where the entry may read a file (a file read, or a call of a callable whose
  effect bound holds the file capability). A module manifest's network request gets the same code. Process access,
  session-file access, randomness and a file the entry only writes keep `LG3:capability.not_granted` (LG3.8, LG3.9).
  One mapping serves the checker's admission, its built-in column and LP3's approval of a built-in tool
  ([lib.rs](../../../rust/c600-check/src/lib.rs)).
- CT-RTE-07 on 600-cell-Full gives the corpus's code for every refused key, P01 to P03 included
  ([ct_rte_07_full.rs](../../../rust/c600-module/tests/ct_rte_07_full.rs)). The refusals are unchanged: approval
  refuses each probe before any trap is called, and no planted data appears.
- Chapter 13's Used-by cell of the row still reads "not yet used as a concrete code". The PHS.52 amendment at LP3's
  command-table freeze names the checker there; the frozen text is not edited before then.

## The conflict, as found

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
- `c600-check` refused the same reads with `LG3:capability.not_granted`, and CT-RTE-07 listed P01 to P03 as keys whose
  code differed from the corpus's. The remaining LX-3 checks are LP3 acceptance checks with unchanged bars
  ([14 November 2026](../decisions/owner-decisions-2026-11-14-language-route.md)).
