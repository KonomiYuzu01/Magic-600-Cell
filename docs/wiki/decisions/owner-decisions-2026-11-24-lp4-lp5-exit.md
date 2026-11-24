---
id: owner-decisions-2026-11-24-lp4-lp5-exit
type: decision
status: verified
visibility: public
summary: Owner sign-off of 24 November 2026 on one card for both exits, every recommended option - LP4's exit on 0208e78d and LP5's exit on 503736cc are signed with their readings; LP5's three drafts (R8, R10, R1) are ratified; ONE.52's strict reading stays until a concrete flow needs ADR-D01.2's workspace rows; the integrator drafts the human tests' one-page form; the owner-machine sitting (with the lattice replay) is agreed and waits for a date the owner sets; pull request 433's merge under an owner exception is recorded.
related: [owner-decisions-2026-11-23-lp4-lp5-start, owner-decisions-2026-11-22-c10-lp2-exit, owner-decisions-2026-11-22-adr-d01-overrules, owner-decisions-2026-11-23-a7-freeze]
supersedes: []
claims:
  - {id: lp4-exit-signed, evidence_kind: decision, checked_at: 2026-11-24}
  - {id: lp5-exit-signed, evidence_kind: decision, checked_at: 2026-11-24}
  - {id: lp5-drafts-ratified, evidence_kind: decision, checked_at: 2026-11-24}
  - {id: one52-strict-kept, evidence_kind: decision, checked_at: 2026-11-24}
  - {id: owner-sitting-deferred, evidence_kind: decision, checked_at: 2026-11-24}
  - {id: pr433-owner-exception, evidence_kind: decision, checked_at: 2026-11-24}
---

# Owner decision, 24 November 2026: LP4's exit and LP5's exit

On 24 November 2026 the integrator sent the owner one card for both exits, after LP4's line pull request (435) and
LP5's pull requests (433, 434) had merged. Each item carried a recommended option. The owner answered in chat: every
recommended option. The owner also said that the owner-machine sitting and the lattice replay do not run that day.
This page records the answers in English. It moves no bar and edits no frozen text.

## What was signed

1. **LP4's exit on `0208e78d`**, the head of pull request 435 (merged as `fc472469`). Phase I landed earlier as pull
   request 431 (`b01099e3`). The [LP4 plan](../../progress/1.0/language-lp4-plan.md) is the starting point, and the
   pull request's body names the checks that ran.
   - LP4-1 to LP4-15 are merged.
   - The conformance index for LP4 reads 43 of 51 whole (10 of them by union), 3 part, 0 missing and 5 waiting
     ([index notes](../../progress/1.0/lp4-ct-index-notes.md)).
   - Reviews: fresh smaller-model reads of the command layer and the session store, and the mathematician's
     row-by-row review of the glossary, the learning path and the principle records. No blocker or major is open.
2. **LP5's exit on `503736cc`**, the head of pull request 433 (merged as `944ac454`), with pull request 434's CI-only
   follow-up (`0cff06a6`). The exit record is section 12 of the [LP5 plan](../../progress/1.0/language-lp5-plan.md).
   - Preflight answers 286 claim groups `ready` and 224 `unavailable`. Of the 287 checks of the ready groups, 286 pass
     and one, the lattice replay, is not run.
   - The in-place replays of stage 18's gates 2 and 3 pass in release.
   - The conformance index for LP5 reads 23 IDs: 14 whole and 9 part ([index notes](../../progress/1.0/lp5-ct-index-notes.md)).

## The readings

LP5's readings, as the exit record recommends:
- CT-KN6-01, CT-LB5-01 and CT-LB5-04 are whole: they lack only ignored tests, which ran and passed in release.
- CT-LB5-05 is whole: the in-place gate 2 and gate 3 replays ran and passed.
- CT-KN6-03 is part. The lattice replay runs once at the owner-machine sitting. Stage 19's seven type-lex records and
  the cap-noninteraction claims stay `unavailable` until the archive defines the irregular-piece vector and delta_Q5.
- CT-LB5-07, CT-LB5-08, CT-ERR-11 and CT-EVD-12, and LP2's halves CT-LG3-05 and CT-LG5-02, are indexed with the work
  that builds the parts they need.

LP4's readings, as its line pull request records them:
1. **Preview route.** LG3.16 and BND.56 stay the interim human-solve boundary until the joint setting (LQ2). The
   program-prepared preview of ADR-D01.8 and ADR-D01.58 opens only with that setting. ADR-D01's precedence does not
   reach the human-solve boundary.
2. **ONE.52.** A run reaches no workspace row (item 4 below).
3. **EV1 codes.** They are also emitted on a route that names a digest. A grant revoked at activation answers
   `LG3:capability.not_granted`.
4. **Log import.** Import is one ordinary retained move under `file.log.import`, so neither the frozen journal format
   nor the migration format changes. The cost: FP.32's voids are not written, and a partial undo can step back inside
   the imported word. Every playback stops at import, the log's preferences are recorded and never applied, an export
   covers the whole session, and Jumble sessions are not converted.
5. **Soft disclosure.** The disclosure state belongs to the shell. The command layer never reads it, and every row
   stays invocable in every disclosure state.

The overlay records OV-1 to OV-4 in [language-overlays.md](../../progress/1.0/language-overlays.md) stand as written:
- OV-1 is chapter 05's part of EV2.9's revision.
- OV-2 is the 1.0 log file's record, `c600-log-draft` version 1.
- OV-3 is the learning path's step order.
- OV-4 says where recordings and playback states are kept.

DET.7 in chapter 09 is amended under PHS.52, as item 2.3 of the
[LP4 and LP5 start](owner-decisions-2026-11-23-lp4-lp5-start.md) authorized.

## The other items

3. **LP5's drafts are ratified.**
   - R8's cost rule, metering rule, report format and edition `6cs-draft-1`.
   - R10's row IDs, kinds, lanes and argument types, and the research-profile serve.
   - R1's binding rule: besides the matrix's required files, a binding names the stored inputs its claim checker reads,
     each a matrix file with its pin.

   Their names keep `-draft` until their own packet. This freezes no format.
4. **ONE.52's strict reading stays.** ADR-D01.2's workspace rows for an invoked entry point are not admitted now. When
   a concrete flow needs one, the integrator names a safe subset on that milestone's card.
5. **The human tests' form.** CT-EV5-03, CT-EV5-05 and CT-LB6-04 wait for it. The integrator drafts a one-page form at
   minimal scale ([16 November 2026](owner-decisions-2026-11-16-time-saving.md)), and the owner sets it.
6. **One owner-machine sitting, at a date the owner sets.**
   - It runs about half an hour attended:
     - the stutter fix's device re-run;
     - the owner-machine halves of reading (a) of the [C10 and LP2 exit](owner-decisions-2026-11-22-c10-lp2-exit.md);
     - LX-4's editor bars on LP4's candidate;
     - CT-EV4-05's Windows half;
     - the greybox flow.
   - LP5's lattice replay (about 13 hours in release) runs unattended once, overnight after the sitting.
   - The owner said it does not run on 24 November 2026.
7. **Pull request 433 merged under an owner exception.** On 24 November 2026 the owner told the integrator to merge
   LP5's pull request without another CI round. Every check passed except Windows shard 1. That shard stopped at its
   3,500 s limit after 3,505 s, cut off inside `c600-tools`. Every Rust test that ran before the stop passed, and the
   full Rust workspace passed on Linux on the same head. Pull request 434 split `c600-checkers`' tests into a check of
   their own, and every shard passed again.

## What follows

- The progress items `1.0-lp4 line-pr` and `1.0-lp5 exit` are ticked with pull request 435 and this page as evidence,
  and both steps are marked done.
- The briefing records both exits. The briefing is an input of the frozen migration format packet, so its pin is
  refreshed under FP.1 in a separate commit.
- The integrator drafts the human tests' form and puts it to the owner at the next milestone.
- The owner-machine sitting waits for the owner's date.

## What this does not do

- It moves no bar or gate.
- It accepts no ADR.
- It edits no frozen text beyond DET.7, which item 2.3 of 23 November 2026 had already authorized.
- It counts none of the waiting tests, the lattice replay or the owner-machine halves as passed.
