# Packet: milestone audit of the jumbling mid-stage (senior reviewer, review kind)

Run read-only as a milestone audit by the senior reviewer, with effort `max` and speed tier `fast`. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: the audit that plan section 2 of `docs/progress/1.0/jumbling-plan.md` requires before the two mid-stage packets go to the owner. Check:
  1. Does each row of `docs/progress/1.0/jumbling-midstage.md` link its deliverable to its contract revision, its exact menu identity, a replayable fixture and its result, and does the repository support each entry? Spot-check the identities and counts against the files they cite.
  2. Does each row meet the condition plan section 2 sets for it (J1: items 1–9 pass and its Astra review is valid; J2: certified J1 sequences with blocked grips and certificates; J4: orbit data for the candidate menus; J3: verified, conjectural and open conclusions listed after one Astra verification)? A row that does not meet it must say so.
  3. Do the two packets carry the content of plan sections 3 and 3b, and is every factual statement in them supported by the cited evidence? Packets: `docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md` and `docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md`.
  4. Does either packet claim Windows, Direct3D 12, input or performance evidence, relax the renderer gate, widen an authorization, or break the human-solve boundary or the platform rule?
  5. Can the fixtures in `research/jumbling/fixtures/` serve both packets as written: are the array conventions, the swept-twist family and the W-J trace well defined and consistent with J1 and with `work/experiments/renderer-sb/SPEC.md` section 3?
- Acceptance: a schema-valid result. Each finding gives `path:line` evidence; a `blocker` or `major` gives a counterexample.
- Out of scope:
  - the J1 code and the theory proofs, which have their own reviews;
  - the menu choice and every other owner decision;
  - `minor` and `nit` issues of wording.

## 2. Actual problem and reproduction
- Reproduce the fixtures: `python research/jumbling/fixtures/wj.py check` (about 20 minutes). The renderer packet lint: `python tools/perf/check_renderer_packets.py`.

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, current head. Contract revision `state-contract 2026-10-09 A1-A4`. Evidence kind: source, fixture and synthetic geometry.

## 4. Necessary source and evidence
- `docs/progress/1.0/jumbling-plan.md` sections 2, 3, 3b and 5; `docs/progress/1.0/jumbling-midstage.md`.
- The two packets above, `docs/progress/1.0/packets/renderer/README.md` (the gate) and `work/experiments/renderer-sb/SPEC.md`, `RESULT.md`.
- `research/jumbling/fixtures/` (`wj.py`, `README.md`, the three fixtures).
- `research/jumbling/sim/README.md`, `acceptance.json`; `research/jumbling/viewer/README.md`, `scene-s4.json`; `research/jumbling/explorer/README.md`, `explorer-results.json`; `research/jumbling/theory/theory-draft.md`.
- `research/jumbling/state-contract.md` and `docs/progress/1.0/command-table.md` section 1.

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | J1 review and verification | passed (`20261009T213956Z-8426390f`) |
| 2 | J2 S4 scene, browser check | 105 of 105 assertions |
| 3 | J4 group menus and J3 verification | see the table |

## 6. Constraints and owned files
- Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Finding IDs use the prefix `MSA`.
- Review only; do not perform follow-up work.
