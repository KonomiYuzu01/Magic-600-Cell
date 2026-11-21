---
id: owner-decisions-2026-11-21-ceilings-e3
type: decision
status: verified
visibility: public
summary: Owner answers of 21 November 2026 to the card sent after the command-table freeze, all on the recommended option - one product budget ceiling for all three profiles (steps 2^21, memory 2^27, output 2^24 from LP2's measurement, kernel_cost kept at the draft 2^40 until the rows are priced and then measured again), the draft E3 cold definition with one untimed warm-up launch before the three timed runs, and the C9 sitting rerun once on the frozen candidate with its unattended part overnight.
related: [owner-decisions-2026-11-20-lp3-exit, owner-decisions-2026-11-18, owner-decisions-2026-11-16-time-saving]
supersedes: []
claims:
  - {id: ceilings-one-for-all-profiles, evidence_kind: decision, checked_at: 2026-11-21}
  - {id: ceilings-kernel-cost-draft, evidence_kind: decision, checked_at: 2026-11-21}
  - {id: e3-cold-definition, evidence_kind: decision, checked_at: 2026-11-21}
  - {id: c9-sitting-frozen-candidate, evidence_kind: decision, checked_at: 2026-11-21}
---

# Owner decision, 21 November 2026: product budget ceilings, E3's cold start and the C9 sitting

LP3's command-table freeze pull request (393) merged on 21 November 2026 at 01:33 UTC. Its merge commit, main
`013900ffc013d4c63f0dd8c39d83836de2b7c443`, is the frozen candidate. The integrator then sent the owner one card with
three items, each with a recommended option (owner, 16 November 2026: one card per milestone). The owner answered in
chat in Chinese that every item takes the recommended option. This page records the answers in English. It edits no
frozen text and accepts no ADR.

## Decisions

1. **Product budget ceilings (option 1).** Every session profile (solving, tool authoring and research) admits a
   program job against one ceiling (KN7.7, BM3). CT-RTE-07's keys B13 and B14, a missing budget and a budget over the
   host's ceiling, are tested against it. Until now `Profile::budget_ceiling` returned the draft
   `Budget::DRAFT_CEILING`: steps 2^20, kernel_cost 2^40, memory 2^32 and output 2^26.

   | Dimension | Measured peak (solving profile) | Ceiling |
   | --- | --- | --- |
   | steps | 359,678 | 2^21 = 2,097,152 |
   | kernel_cost | 2,533,004,695 under draft tariffs | 2^40, the draft value, kept |
   | memory | 16,889,436 | 2^27 = 134,217,728 |
   | output | 2,901,606 | 2^24 = 16,777,216 |

   - **Measurement.** LP2 measured the peaks headless, from the deterministic meter (DET.27), over every completed run
     of the shipped library, jumbling-tool, representation and 600-cell-tool entries in the Rust workspace's tests.
     Each ceiling is the peak times four, rounded up to a power of two.
   - **Limits of the evidence.** This is source and fixture evidence only. The peaks come from single entries run on
     the tests' inputs, which are not proved worst cases. Tool authoring and research run no jobs yet, and they take
     the same values.
   - **kernel_cost.** It stays at 2^40 until the command table's rows are priced. Then it is measured again with the
     same method and goes to the owner.
2. **E3's cold start.** The C9 tools' draft definition of a cold run is accepted:
   - a new product process, with no product process running at launch;
   - a fresh copy of the session fixture;
   - at least 20 s since the previous product process exited;
   - no reboot and no flush of the OS file cache.

   One untimed launch of the same build comes before the three timed runs. The first launch of new binaries also pays
   Windows' first-run checks (pull request 387). That launch's time is recorded apart and is never counted. A
   reboot-cold reading, which would be stricter, is not used. E3's bar is unchanged: 3 s to the first correct frame in
   each of three cold runs.
3. **The C9 sitting.**
   - The owner-machine runs are made once, on the frozen candidate `013900ff` and the builds made from it, with no
     rehearsal before them (PHS.23, PHS.64).
   - The sitting starts on the evening of 21 November 2026, with about 40 minutes attended. The unattended part,
     about 7 h 30 min, runs overnight.
   - The machine stays in light mode from 20:00 UTC until the run ends.

## What follows

- LP2 makes one pull request for item 1:
  - `Profile::budget_ceiling` returns the values;
  - the shell, the bridge's default budget, the propose path and the tests declare the profile's ceiling instead of
    `DRAFT_CEILING`.
- The frozen candidate and the C9 sitting keep `DRAFT_CEILING`. U7's evidence binds to the C9 build at admission only:
  the job then runs under the trial's limits.
- The C9 records cite this page for E3's cold definition. The C9 tools' "DRAFT" mark on it is lifted in a later pull
  request that does not touch the frozen candidate's build.
- After the sitting, the owner rules on whether U8's reference to U4's W3 summary may stand in for LX-4's gate
  reference. Until then LX-4's verdict stays `insufficient`.

## What this does not do

- No bar moves: E2 stays a p95 of at most 100 ms, and E3 stays 3 s.
- No frozen text is edited, and no ADR is accepted.
- The C10 sign-off, LP2's exit, LQ2's option set and the kernel_cost ceiling stay open.
