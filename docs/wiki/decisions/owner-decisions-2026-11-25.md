---
id: owner-decisions-2026-11-25
type: decision
status: verified
visibility: public
summary: Owner decisions of 25 November 2026 - ADR-D02's A4 may be prepared and then starts; the human tests' form is set as recommended, with the owner running CT-LB6-04 and CT-EV5-05 alone on self-run sheets and CT-EV5-03 waiting for non-authors; the owner-machine sitting runs on 25 November 2026 and the lattice replay waits for the owner's word; Fable writes the ADR-D02 and ADR-D03 build frameworks and Opus builds, in the subscription with no API; and a correction to the 24 November page's review sentence.
related: [owner-decisions-2026-11-24-lp4-lp5-exit, owner-decisions-2026-11-23-a7-freeze, owner-decisions-2026-11-16-time-saving]
supersedes: []
claims:
  - {id: a4-preparation-allowed, evidence_kind: decision, checked_at: 2026-11-25}
  - {id: human-test-form-set, evidence_kind: decision, checked_at: 2026-11-25}
  - {id: owner-sitting-2026-11-25, evidence_kind: decision, checked_at: 2026-11-25}
  - {id: lattice-replay-held, evidence_kind: decision, checked_at: 2026-11-25}
  - {id: d02-d03-builders, evidence_kind: decision, checked_at: 2026-11-25}
---

# Owner decisions, 25 November 2026: human tests, the sitting, and ADR-D02

On 25 November 2026 the owner answered the integrator in chat: every recommended option, with the changes below. The
owner gave the ADR line two more answers the same day (item 5). This page records them in English. It moves no bar
and edits no frozen text.

## 1. ADR-D02's A4 may be prepared

The owner lifted the hold on A4 of 24 November 2026. The ADR-D02 session prepares A4's pick page and the data its
three waiting picks need, with no API. Later the same day the owner told the ADR line that A4 starts as soon as its
preparation is done. The picks stay the owner's, put to the owner as one card.

## 2. The human tests' form is set

The integrator's one-page draft at minimal scale ([16 November 2026](owner-decisions-2026-11-16-time-saving.md)) is
set as recommended, with two changes:
- **CT-LB6-04 and CT-EV5-05:** the owner runs them alone, on self-run sheets with sealed hints. The draft's
  alternative for the moderator becomes the form. The answer keys stay private until the sheets are done.
- **CT-EV5-03:** waits until non-authors are available. Its ID stays waiting; nothing counts it as passed.

The rest stands as recommended:
- the help ladder (0 to 3) and the pass at help level 0 or 1 with no critical mistake;
- the list of critical mistakes;
- no step-count ceiling (LB6.2) until these results report;
- the mathematician checks the form, the keys and the critical-mistake list beforehand, and classifies each wrong
  answer afterwards. This is review, not human evidence.

Raw sheets stay private. A sanitized table goes to the LP4 human-test page, which LP4 adds with the form. The owner's
two sheets take about 45 minutes.

## 3. The owner-machine sitting runs on 25 November 2026

The attended sitting of [24 November 2026](owner-decisions-2026-11-24-lp4-lp5-exit.md), item 6, runs on 25 November
2026: the stutter fix's device re-run, reading (a)'s owner-machine halves, LX-4's editor bars on LP4's candidate,
CT-EV4-05's Windows half and the greybox flow. Its results land in their own pull request.

## 4. The lattice replay waits for the owner's word

LP5's lattice replay does not run after the sitting. It starts when the owner says so. Until then CT-KN6-03 stays
part, as the 24 November reading says. The replay calls no model; its cost is about half a day of the machine's
processor time, so it runs unattended overnight.

## 5. Who builds the ADR-D02 and ADR-D03 shells

In the ADR line the owner answered the open item of the [A7 freeze](owner-decisions-2026-11-23-a7-freeze.md):
- For each of ADR-D02 and ADR-D03, Fable writes the full build-direction framework, Opus builds, and Fable can watch
  at any time.
- Channel: the subscription, with no API. Fable runs as a subscription subagent: it writes the framework, reviews at
  each checkpoint and reviews on demand. There is no paid spend.
- The owner also wants to watch a live Look Lab window while the shells are built and checked.

The ADR line's other answers of 25 November 2026 (the icon tiers, and open research as ADR-D02's core) are recorded
with ADR-D02's pull request.

## 6. Correction to the 24 November page

The [24 November page](owner-decisions-2026-11-24-lp4-lp5-exit.md) said the mathematician reviewed the glossary, the
learning path and the principle records row by row. That review, on 24 November 2026, covered the glossary's rows and
the learning path's seven steps and 27 keys. The principle records had the mathematician's pass of 23 November 2026
(`c152350f`) with its re-check (`0af6fa70`), and a scoped check on 25 November 2026 that also brought their review
fields up to date (pull request 437). No blocker or major is open, so the exit's conclusion stands. The sentence on the
24 November page is corrected in place.

## What follows

- LP4 lands the form, the owner's two self-run sheets and the human-test page in one small pull request.
- The sitting's pull request records its results.
- The briefing's 23 and 24 November bullets say the builders, the form, the sitting and ADR-D02's start wait for the
  owner. Its new bullet records these answers, with an FP.1 pin refresh of the format packet.

## What this does not do

- It moves no bar or gate and accepts no ADR.
- It counts none of the waiting tests, the lattice replay or the sitting's halves as passed.
