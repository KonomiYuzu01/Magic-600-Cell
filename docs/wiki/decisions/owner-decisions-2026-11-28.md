---
id: owner-decisions-2026-11-28
type: decision
status: verified
visibility: public
summary: Owner decisions of 27 and 28 November 2026 on the owner-machine chain - every result of the 28 November chain and the lattice replay counts as passed, including the s1128 W3 summary as U8's gate reference; a test that shows a problem needing a fix and a rerun is skipped together with the tests that depend on it, with no repeat testing later; and LP6 starts in parallel with the IR line's remainder when ADR-D02 freezes. The 26 November order of work is otherwise unchanged, and K2 starts when ADR-D03 and LP6 are both done.
related: [owner-decisions-2026-11-26-release-order, owner-decisions-2026-11-25, owner-decisions-2026-11-26-lq2, owner-decisions-2026-11-22-lx-bars-freeze, owner-sitting-2026-11-28-results]
supersedes: []
claims:
  - {id: chain-results-accepted, evidence_kind: decision, checked_at: 2026-11-28}
  - {id: skip-dependent-tests-no-repeat, evidence_kind: decision, checked_at: 2026-11-28}
  - {id: lp6-parallel-with-ir-remainder, evidence_kind: decision, checked_at: 2026-11-28}
  - {id: gpu-overclock-off, evidence_kind: decision, checked_at: 2026-11-28}
---

# Owner decisions, 27 and 28 November 2026: the chain's results and the order of LP6

The owner answered the integrator in chat on 27 and 28 November 2026. This page records the answers in English with their
readings. It moves no bar and edits no frozen text. The results are in
[owner-sitting-2026-11-28-results](../evidence/owner-sitting-2026-11-28-results.md).

## 1. Every result counts as passed (28 November 2026, about 13:50 UTC)

Owner (said in Chinese; public pages stay in English): put up the pull request and merge it now; count everything as passed; next week I will fix D02 and open the
IR line, LP6, D03 and K2.

Reading:
- The owner accepts every result of the 28 November chain (U4, U7, U8 and GAP) and the lattice replay as passed.
- The accepted U4 W3 summary of 28 November (`w3-summary-s1128.json`) is U8's gate reference. LX-4's harness counts a
  reference only for the summary and build that the owner accepted (`ACCEPTED_GATE_REFERENCES` in
  `tools/perf/c9/lx4.py`, [LX bars decision](owner-decisions-2026-11-22-lx-bars-freeze.md), item 2). The entry is made
  with this page, and the analysis reads `all-bars` with it.
- GAP carries no bar, so there is nothing to pass there; its share figures are recorded as measured.
- The [26 November order of work](owner-decisions-2026-11-26-release-order.md) is unchanged. K2 starts when ADR-D03 and
  LP6 are both done.

## 2. A problem that needs a fix and a rerun skips its dependents (27 November 2026, about 23:00 UTC)

Owner (said in Chinese): if a test shows a problem that needs a fix and a rerun, skip the tests that depend on it; no repeat testing
later.

Reading: the rule applies to the chain's remaining steps and to later sittings of this kind. Nothing needed it on
28 November 2026: every step ran and read met.

## 3. LP6 starts in parallel with the IR line's remainder (27 November 2026, about 21:24 UTC)

Owner (said in Chinese): start in parallel.

Reading: when ADR-D02 freezes, LP6 starts in parallel with the IR line's remainder instead of after it. The first LP6
work is the importer, packaging and the OV-5 policy. LP6's documents and the language-product freeze wait for the IR
freeze card. ADR-D03's place in the order is unchanged.

## 4. The GPU overclock was off (27 November 2026, about 19:47 UTC)

The owner turned the Vantage GPU overclock off. In the 30-second clock log of the 28 November chain, no memory clock
reading is above the stock maximum of 8001 MHz (see the results page).
