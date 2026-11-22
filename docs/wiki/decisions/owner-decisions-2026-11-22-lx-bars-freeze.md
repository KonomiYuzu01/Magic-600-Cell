---
id: owner-decisions-2026-11-22-lx-bars-freeze
type: decision
status: verified
visibility: public
summary: Owner answers of 22 November 2026 after the C9 sitting, all on the integrator's recommended option - LX-2's and LX-4's latency bars move to 100 ms, LX-4 counts a sample without a confirmed frame as E2 counts a key and cites U4's W3 summary as its gate reference, alloc_metered's kernel_cost limit end counts, the renderer gate is not required in the fault windows where the runtime is lost, alloc_ceiling's 84 echo samples are accepted, LP3 and LP2 exit on their existing candidates with no new sitting, kernel_cost stays 2^40, LP2's exit readings take their recommended options, and the product hosts keep no per-operation job rows; after the re-analysis, a sample without a confirmed frame is counted apart as a known limit (the confirmation gap) and the bars are judged on confirmed samples, so LQ9 does not reopen.
related: [owner-decisions-2026-11-21-ceilings-e3, owner-decisions-2026-11-20-lp3-exit, owner-decisions-2026-11-16-time-saving, owner-decisions-2026-11-14-language-route, c9-sitting-results]
supersedes: []
claims:
  - {id: lx-bars-100-ms, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: lx4-counting-and-gate-reference, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: lx2-kernel-cost-limit-end, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: lx2-gate-under-runtime-loss, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: lx2-alloc-ceiling-samples, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: exits-on-existing-candidates, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: kernel-cost-kept, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: lp2-exit-readings, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: product-hosts-no-job-rows, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: lx-confirmation-gap-apart, evidence_kind: decision, checked_at: 2026-11-22}
---

# Owner decision, 22 November 2026: LX bars at 100 ms and the exits on the existing candidates

The C9 sitting ran once on the frozen candidate `013900ff` on 21 and 22 November 2026
([results](../evidence/c9-sitting-results.md)). E3, E2 and every renderer gate summary were met. LX-2 was not met:
echo took 76 to 93 ms against 50 ms in every fault, and the renderer gate dropped to 39 to 57 fps while the runtime was
lost. LX-4 was invalid under its draft rule, because 16 of 10,800 samples had no confirmed frame. Under RTE.21 a failing
LX-2 or LX-4 run reopens LQ9.

The integrator sent the owner one card. The owner then said that there is no time now for long measurements, that LP2
and LP3 should freeze soon, and that the bars may be relaxed to 100 ms, and asked for a new recommendation. The
integrator recommended eight items, R1 to R8, with no new measurement. The owner answered in chat in Chinese that every
item takes the new recommendation and that the work starts once pull request 404 has merged. This page records the
answers in English. It edits no frozen text and accepts no ADR.

## Decisions

1. **Latency bars at 100 ms (R1).** This is the owner's bar move, which RTE.53 reserves to the owner. Every p95 and
   p99 bar below 100 ms in LX-2 (RTE.42 condition 3) and LX-4 (RTE.49 and RTE.53) becomes 100 ms, the bar of charter
   E2 (RTE.43).

   | Interval (RTE.49) | Sizes | Draft p95 / p99 (ms) | Now (ms) |
   | --- | --- | --- | --- |
   | Typed echo, also LX-2's echo bar | S, M, L | 50 / 67 | 100 / 100 |
   | Syntax feedback | S, M | 67 / 100 | 100 / 100 |

   Every other bar in RTE.49 already had a p95 of at least 100 ms and is unchanged. Syntax feedback at L (100 / 150)
   is unchanged.
2. **LX-4's counting and gate reference (R2).**
   - A sample without a confirmed frame counts as one failed sample, and the run stays valid. This is the rule that
     charter E2 uses for a key without a confirmed frame (owner, 20 November 2026, item 2). Item 10 replaced this
     point after the re-analysis.
   - U8's LX-4 runs may cite U4's W3 summary as their renderer gate reference. It comes from the same build on the
     same night, and it was met. This answers the open item in
     [owner-decisions-2026-11-21-ceilings-e3](owner-decisions-2026-11-21-ceilings-e3.md) ("What follows").
   - The recorded U8 data is analysed again under these rules. No new run is made.
3. **alloc_metered's limit end (R3).** In the shell, alloc_metered's view job runs under the product's own view
   limits. All 61 of its limit windows ended at `KN7:budget.kernel_cost`. `lx2.py` counts that code as a limit end of
   this fault, beside `KN7:budget.memory` and the 60 s safety stop. The recorded U7 data is analysed again.
4. **The renderer gate under runtime loss (R4).** In LX-2's condition 3, the renderer gate is not required inside the
   fault windows of the four faults in which the runtime process dies and is restarted:
   - forced_kill;
   - alloc_ceiling;
   - recursion_native;
   - loop_native.

   The gate's figures there (39 to 57 fps, p99 52 to 59 ms) are recorded as a known limit. The echo bar and conditions
   1, 2 and 4 still apply in those windows. The gate still applies in the metered faults, where it held at 60 fps, and
   everywhere outside fault windows. Removing the stutter is product work after the freeze.
5. **alloc_ceiling's echo samples (R5, option 1).** This fault has 84 timed echo samples, so the record reads
   `insufficient`. They are accepted for this fault: p95 92.0 ms and p99 92.8 ms, both under 100 ms. No U7-only rerun
   is made.
6. **The exits on the existing candidates (R6).**
   - LP3's exit (C10) is judged on `013900ff`, and LP2's exit on `0260e6b1`. There is no new candidate and no new
     sitting.
   - Three changes land after the exits as follow-ups and are part of neither exit candidate:
     - pull request 404 (item 9 below);
     - reading 18's undo fix (item 8 below);
     - the shell's stutter fix under runtime loss (item 4).
   - CT-LG3-03's and CT-LB7-01's undo halves move to reading 18's fix. That is an index change under PHS.52, which this
     answer approves.
7. **kernel_cost (R7).** The product ceiling for kernel_cost stays at 2^40 until the command table's rows are priced
   (DET.55 to DET.59). Then it is measured again and goes to the owner, as decided on 21 November 2026.
8. **LP2's exit readings (R8).** LP2's exit candidate is `0260e6b1`:
   - lp2-full run 44871666684 passed on it, including the comparison of the Windows and Linux transcripts;
   - PHS.19's items have evidence;
   - the CT index has 248 IDs: 126 whole, 121 part and 1 waiting (CT-LI-14).

   The readings of LP2's exit list take their recommended options. Reading 11 was closed by pull request 401.

   | Readings | What is missing | Recommended reading, now decided |
   | --- | --- | --- |
   | 1 to 4 | A second route, ONE.32's fields in two profiles, a second edition, a budgeted research search | Not applicable while the recorded decision stands (PHS.63); each reopens if that decision changes |
   | 5 | LQ2's joint setting and the BND.37 class review | Waiting; the tests run when those land |
   | 6, 7 | LP4's editor services; LP5's checkers | The missing halves are indexed in LP4 and LP5 |
   | 8 | Form 2 (LG2.21), which has no executor | Amended under PHS.52: form 2 stays out of LP2 until it is built |
   | 9, 10, 12 to 16 | LP3's host, shell, display and renderer; the host serving frozen H-08 rows; M6's store follow-up; the importer; IR constructs; chapter 03 operations; kernel and jumbling calls | Each missing half is indexed with the work that builds its piece |
   | 17 | Runs that only the owner's machine or another platform can give | The owner-machine halves join LP3's owner-machine batch; other platforms come after 1.0 |
   | 18 | Is a change to a language-defined workspace row undoable? | Option 1: LG3.5 and LB7.2 stay. A language-defined row's completion gets an undo entry that writes back what each record held, as `filter.apply`'s does. LP3's command layer makes the fix after the exit (item 6) |

9. **Job rows in the product hosts.** The owner answered this item earlier on 22 November in LP3's session, on the
   recommended option. The tool's two product hosts, the process host (`job_under`) and the in-process host (`run_under`),
   keep no per-operation rows (`record: false`). The counters and stops of the run stay. Pull request 404 makes the change.

10. **Samples without a confirmed frame (answered after the re-analysis).** The C9 tools analysed U7 and U8 again
    under items 1 to 5 (pull request 411). Every timed p95 and p99 was within 100 ms. LX-2 and LX-4 still read not met,
    only because some samples had no confirmed frame: the frame was drawn, but the core received no confirmation that
    it was displayed.
    - LX-4: 16 of 10,800 samples. Counted as E2 counts them, they put two cells' p99 on a failed sample: Jumble on,
      series 1, syntax at S; and Jumble off, series 2, syntax at L.
    - LX-2's echo: 8 of 110 samples in forced_kill, 26 of 110 in alloc_ceiling and 13 of 201 in loop_native, all
      faults that lose the runtime; and 1 of 212 in recursion_metered.

    The integrator offered three options:
    1. count these samples apart;
    2. count them as E2 does, so that LQ9 reopens;
    3. fix the gap and run U7 and U8 again.

    The owner chose option 1:
    - In LX-2 and LX-4, a sample without a confirmed frame is counted apart. It is published as a known limit, the
      confirmation gap, with its count per fault, cell and run. The latency bars are judged on the samples that have
      a confirmed frame. This replaces item 2's first point and also applies to LX-2's echo bar.
    - The gap does not reopen LQ9, and RT1 stays. The integrator's reading, an inference, is that the gap lies in the
      display confirmation path and not in the route. It also appears with no fault and no program running: 1 of 602
      keys in U8 and 77 of 32,254 UI frames in U3.
    - The cost is on record: in alloc_ceiling about a quarter of the echo samples are not judged.
    - Closing the gap is product work after the freeze, together with the stutter fix (item 4).
    - Charter E2 keeps its own rule: a key without a confirmed frame counts as a failed key.

## What follows

- The C9 tools take items 1 to 5 and item 10, and the recorded U7 and U8 data is analysed again:
  - `lx2.py` and `lx4.py` use the 100 ms bars;
  - `lx4.py` uses U4's W3 summary;
  - both count samples without a confirmed frame apart (item 10);
  - `lx2.py` adds the kernel_cost limit end and drops the gate inside the four faults' windows.

  The results page is updated. If a 100 ms bar is still missed on the confirmed samples, LX-2 or LX-4 fails, LQ9
  reopens (RTE.21) and the numbers go to the owner.
- Chapter 11's RTE.42, RTE.49 and RTE.53 receive a PHS.52 amendment that cites this page, item 10 included, with the format packet's
  pin refresh (FP.1) if the chapter is pinned.
- The LP2 line writes LP2's exit record on `0260e6b1` with the index changes of item 8.
- LP3 checks C10's list on `013900ff` and prepares C10's sign-off.
- The owner then signs LP2's exit and C10 on one card (owner, 16 November 2026: one card per milestone).

## What this does not do

- Charter E2 stays a p95 of at most 100 ms, and E3 stays 3 s.
- The renderer gate is unchanged outside the four faults' windows.
- Every bar in RTE.49 that is not named in item 1 is unchanged.
- No frozen text is edited, and no ADR is accepted.
- It is not LP2's exit sign-off or C10's sign-off.
- LQ2's option set and the design-system ADR stay open.
