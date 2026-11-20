---
id: owner-decisions-2026-11-20-lp3-exit
type: decision
status: verified
visibility: public
summary: Owner answers of 20 November 2026 to the nine items of LP3's exit card - eight take the recommended option, and the E2 item takes option 2 (a key without a confirmed frame counts as one failed key, and its run stays valid). B12 is recorded cold only; CT-RTE-04's five form-2 traces and its three IR amendment candidates are amended under PHS.52 at the freeze; push keeps its per-element price; the five later draft codes are registered in chapter 13 with the six of 16 November; `replay` and `protection` keep one name each, typed by puzzle and level; E3 stays 3 s and startup is made faster; route B's blind seconds come from the core's own frame statistics, cross-checked against PresentMon, since PresentMon cannot be kept as the source (pull request 385).
related: [owner-decisions-2026-11-16-phs52, owner-decisions-2026-11-18, owner-decisions-2026-11-18-route-b, owner-decisions-2026-11-16-time-saving]
supersedes: []
claims:
  - {id: lp3-exit-b12-cold, evidence_kind: decision, checked_at: 2026-11-20}
  - {id: lp3-exit-e2-missing-frame, evidence_kind: decision, checked_at: 2026-11-20}
  - {id: lp3-exit-form2-traces, evidence_kind: decision, checked_at: 2026-11-20}
  - {id: lp3-exit-ir-amendments, evidence_kind: decision, checked_at: 2026-11-20}
  - {id: lp3-exit-push-tariff, evidence_kind: decision, checked_at: 2026-11-20}
  - {id: lp3-exit-draft-codes, evidence_kind: decision, checked_at: 2026-11-20}
  - {id: lp3-exit-bridge-names, evidence_kind: decision, checked_at: 2026-11-20}
  - {id: lp3-exit-e3-startup, evidence_kind: decision, checked_at: 2026-11-20}
  - {id: lp3-exit-route-b-source, evidence_kind: decision, checked_at: 2026-11-20}
---

# Owner decision, 20 November 2026: LP3's exit card

After the C9 rehearsal of 20 November 2026, the integrator sent the owner one card with LP3's open exit items, each
with a recommended option (owner, 16 November 2026: one card per milestone). The owner answered in chat in Chinese:
every item takes the recommended option except item 2, which takes option 2. This page records the answers in
English. It edits no frozen text; the PHS.52 amendments below are written in LP3's command-table freeze pull request,
which cites this page.

## Decisions

1. **B12 (CT-RTE-07).** 1.0's host keeps no row-answer cache, so every read is answered fresh (KN7.8). B12 is
   recorded as run cold, and its other five cache states (warm, evicted, poisoned, shared, relocated) are not
   applicable while no cache exists. A cache, with all six states, is built only if E2 or LX-4 on the owner's machine
   shows a need. The draft row cache of closed pull request 368 stays on its branch.
2. **Charter E2's counting rule (option 2).** A key whose frame is not confirmed counts as one failed key, with a time
   above 100 ms, in its run's and in the pooled p95, and its run stays valid. The rest of the draft rule in
   `tools/perf/c9/README.md` stands: nearest-rank p95 of at most 100 ms over at least 100 confirmed keys of valid runs
   of one build, met by the pooled p95 and by every run's p95. A run that breaks PHS.64's protocol is still invalid.
3. **CT-RTE-04's five form-2 traces.** They need an executor for `lp1-exec-region-draft-1`, which LP2 does not build
   (stated to the owner on 18 November 2026). They are amended under PHS.52 at the freeze: out of LP3's run until
   form 2 is built, and kept on record in DET.65's vectors.
4. **CT-RTE-04's three amendment candidates.** `generator-nested-same-name` (a call frame without a slot that binds
   two names, while the IR binds a name with `let`, which takes a slot, DET.57), `text-comparison` (the IR draft has
   no text order, DET.62) and `abstract-trace-charges` (a flow check, which only the unselected flow-label alternative
   has, BND.41) are amended under PHS.52 at the freeze, with the reasons of LP2's pull request 234.
5. **The push tariff.** `push` keeps its price of `LIST_PER_ELEMENT` per element of the new list, although it now
   extends a list in place when no other handle sees it (LP2's pull request 364). No pinned budget number moves.
6. **The five later draft codes.** `KN5:consistency.unadmitted_journal_action`, `KN5:capability.preview_shown`,
   `KN5:capability.outside_solver_command`, `KN5:input.saved_set_exists` and `KN4:busy.session_locked` are registered
   in chapter 13 together with the six codes of [16 November](owner-decisions-2026-11-16-phs52.md), in the one
   amendment at LP3's command-table freeze.
7. **The names of the bridge read rows.** `replay` and `protection` keep one name each, typed by puzzle and level, as
   KN2.4 [D] already lists them (`protection`: Full at labels, jumbling at poses; `replay`: by puzzle). A program's
   mode belongs to a revision, and each bridge refuses the other mode, so a mismatch gives a refusal, never a wrong
   answer. No frozen file changes for this.
8. **Charter E3.** The bar stays at 3 s, as set on [18 November](owner-decisions-2026-11-18.md). The rehearsal of
   20 November ran on a build from before the freeze and did not meet it; it is no evidence (PHS.23, PHS.64). Startup
   is made faster before the frozen candidate, and E3 is measured again in the owner's sitting on that candidate.
9. **Route B's blind-second evidence.** Only if the measurement fix cannot keep PresentMon as the source (PresentMon
   marked every present of the core's composition chain as dropped in the rehearsal): the blind seconds come from the
   core's own frame statistics (`GetFrameStatistics`), cross-checked against PresentMon. That change goes in its own
   pull request, which names the gate text it changes ([route B](owner-decisions-2026-11-18-route-b.md)).

   The condition was met: in the rehearsal PresentMon reported all 11,761 presents of the core's composition chain as
   dropped, with no display time, so it cannot stay the source. Pull request 385 applies the option. Route B's
   `blind-seconds` check needs, in each checked second, a traced frame that the core's frame statistics confirm
   displayed (`presented.jsonl`) and a displayed present of the window's chain, and a new `presented` check has
   PresentMon find each confirmed frame's own present, in order, no later than its display. For the same reason, the
   PresentMon cross-check of E2 and LX-2, a draft rule, now checks present order (the first present of the declared
   swap chain after a frame's submission comes no later than its confirmed display) instead of a 2 ms window. No bar
   moves, and the renderer gate's frame times still come from PresentMon's time between the core chain's presents.

## Format packet pin refresh (FP.1)

The layer interfaces are an input of the frozen migration format packet. LP3's freeze pull request amends LI.5 and
LI.33 in them under PHS.52, as items 5 and 6 of the [16 November](owner-decisions-2026-11-16-phs52.md) amendment
list decided, so their pin is refreshed under FP.1.

| Pin in `candidate.json` | Old | New |
| --- | --- | --- |
| Layer interfaces SHA-256 | `9007f33cece174e434d43ab0367edd1cba6cf02068403e41e2aaa1da11982a7e` | `d0b3a7f7c2fe838d69251d1db770ae588de54df46185af8704ef660c32df49a6` |
| `input_commit` | `21ebadc42450f3588b58bbe218bec6193f2658f9` | `b7cb2bd625ec93fddf3ee1acc9376d407e59d777`, the commit of the freeze pull request that edits the draft |

- The change is confined to the draft's status line, LI.5, LI.33 and a new section 10 that records the amendment.
- The packet cites LI.9 and LI.30, which are unchanged, and LI.5 in FP.36. It does not cite LI.33.
- LI.5's change reaches a dependency meaning: the request digest now covers the caller that the host stamps from the
  channel. FP.36 encodes LI.5's resolved request in a durable admission, says that the request hashes those bytes and
  lists no caller. FP.36 is not edited here, and how a durable admission records the caller stays open.
- The candidate files and `candidate_sha256` are unchanged, and `check_vectors.py` verifies every other candidate and
  input pin against the same bytes as before.

## What follows

- LP3's freeze pull request carries items 3, 4, 6 and 7, with H-08's content, the FP.1 pin refresh and a C10 section
  in the LP3 plan, and records items 1 and 5.
- The C9 measurement tools take item 2 (`key_to_frame.py` and the "Bars" section, pull request 380) and item 9
  (pull request 385).
- Item 8 is product work on the shell's startup path before the frozen candidate.

## What this does not do

No bar moves: E2 stays a p95 of at most 100 ms and E3 stays 3 s; item 2 changes only how a key without a confirmed
frame is counted. No frozen text is edited by this page, and no ADR is accepted. LQ2's option set, B13 and B14's
product ceiling values and the C10 sign-off stay open.
