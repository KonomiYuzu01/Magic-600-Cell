---
id: owner-decisions-2026-11-16-en3-engine-half
type: decision
status: verified
visibility: public
summary: Owner decision of 16 November 2026 - EN3's jumbling interfaces are approved for their engine half (JI.1 to JI.16, JI.52 and JI.53, two engine codes) as the input of LP2's M5, after a fresh Claude read, a scoped re-check and their fixes; the contract pin is state-contract 2026-11-15 A1-A6, and J1's public API is one added module. M5 may start; the store, command and view halves stay drafts.
related: [owner-decisions-2026-11-15-freeze, owner-decisions-2026-11-14-language-route, owner-decisions-2026-11-06, owner-decisions-2026-11-16-time-saving]
supersedes: []
claims:
  - {id: en3-engine-half-approved, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: en3-interfaces-text, evidence_kind: source, path: docs/progress/1.0/jumbling-interfaces.md, checked_at: 2026-11-16}
  - {id: domain-key-reference, evidence_kind: source, path: research/jumbling/differential/references/observations.py, checked_at: 2026-11-16}
---

# Owner decision, 16 November 2026: EN3's jumbling interfaces, engine half

The owner gave this decision in chat in Chinese on 16 November 2026 ("approve"). Before deciding, the owner asked for
a summary of exactly what the approval covers. This page records the decision in English.

## Question

M5 of LP2, the jumbling half of the kernel API, waited for EN3's review and approval of the jumbling interfaces
(PHS.53; LP2 plan, section 2). On 16 November the owner asked for that start condition to be resolved now.

M5's one question card asked the owner to approve the engine half of the reviewed draft as M5's input. The basis was a
fresh Claude read (`AGENTS.md` "Codex minimum"). The other options were:

- an Astra review first, which is a Codex call the owner would ask for;
- no approval yet.

The card recommended approval.

## Decision

Approved: the engine half of [EN3's jumbling interfaces](../../progress/1.0/jumbling-interfaces.md) as the input of
LP2's M5, with the scope that JI.54 states. The engine half is:

- JI.1 to JI.16;
- JI.52, the public J1 API;
- JI.53, the domain ID;
- JI.45's codes `KN1:input.unsupported_dimension` and `KN2:input.named_object_invalid`.

With it:

- The contract pin is `state-contract 2026-11-15 A1-A6`, and the state contract's header now records it. J1
  (`state-contract 2026-10-09 A1-A4`) stays the reference for sections 1 to 4 and for menu letters. Requests are
  checked against KN3.7 to KN3.9.
- J1's public API is one added module, `research/jumbling/sim/api.py`, that changes no accepted J1 file. Writing it
  is oracle engineering for the engineering line, not research; the research stays stopped.

Not approved by this decision:

- the session-store half (JI.17 to JI.31), which stays a draft, bound where it consumes the frozen migration format
  packet;
- the command layer, the view model and the command delta (JI.32 to JI.43, `command-rows-jumbling.json`), which stay
  drafts until LP3 and LP4 (PHS.53).

No frozen text changed. The format packet's pin of the draft was refreshed under FP.1, as recorded below.

## What the approval rests on

A fresh Claude subagent on a smaller model read the draft against the frozen chapters. No Codex call was made. It
found four major problems, which commits `e643109a` and `37c9f2f2` fix:

| Finding | Fix |
| --- | --- |
| R1: the contract pin was still `state-contract 2026-10-22 A1-A5`, though the owner accepted A6 on 15 November 2026 | JI.1 and JI.3 pin `state-contract 2026-11-15 A1-A6`. The state contract's header records the revision in place |
| R2: JI.35 described rotation requests in pre-A6 terms, against KN3.7 and KN3.8 | JI.35 states A6's request and binding and the kernel's exact drag scoring. Free mode stays research-only |
| R3: the public J1 API that default 12 and LP2's products require was not specified | JI.52 names a public entry point for each private member of J1 or the theory code that the harness's bindings use |
| R4: "domain key" had no definition, while the chapters say "domain ID" | JI.53 defines the domain ID as the reference `domain_key`, the exact key of the coset g K+ for every pose, with CT-KN2-02's vector |

The read's minor points are fixed in JI.1, JI.6, JI.14, JI.46, JI.48, JI.50 and JI.51. Three are deferred:

- JI.18's "nothing is frozen yet" goes to LP3.
- The command delta's twist rows and live-preview alternative go to LP3.
- The relationship-strip fields go to LP4.

A scoped re-check of the first fixes confirmed R1, R2 and R4 and found R3 incomplete:

- The table missed the state copy and the fixture replay.
- The source scan reached lead scripts that are records, not bindings.

Commit `37c9f2f2` completes JI.52 with `copy_state`, `adopt` and `fixture_state`. It also limits the scan to the
harness's bindings, which are its adapters, `candidate/` and `references/` with the lead modules they import.

The frozen chapters still call A6 "proposed" in several clauses. That is no contradiction:
`docs/architecture/1.0/language/architecture/00-overview.md` reads every such clause as A6 in force.

## Format packet pin refresh (FP.1)

The draft is an input of the frozen migration format packet, so each edit needs a recorded pin refresh (FP.1).

The first refresh, in commit `7a87a75e`, follows the review's fixes:

| Pin in `candidate.json` | Old | New |
| --- | --- | --- |
| Draft SHA-256 | `ee44fdd4ebbda2db88061c488e85fe2ea08791452eb4ffc956d6f81b5fdd4f69` | `1fa64e69ddd4f91c02e7c584eaa34850b20fd9bc7d30205ce36e7ba9e03e8589` |
| `input_commit` | `34ea6dfb5cd41d32de4c925fcf6c5549098a5c16` | `37c9f2f28979955cfd202cc16f9b89d6a305c28b`, the last commit of the fixes |

The second refresh follows this decision. It changes the status line and JI.54:

| Pin in `candidate.json` | Old | New |
| --- | --- | --- |
| Draft SHA-256 | `1fa64e69ddd4f91c02e7c584eaa34850b20fd9bc7d30205ce36e7ba9e03e8589` | `a81cbf90eac8bb09a7d61de060a8b0d652697a80f995949f61efb8a82c333114` |
| `input_commit` | `37c9f2f28979955cfd202cc16f9b89d6a305c28b` | `21ebadc42450f3588b58bbe218bec6193f2658f9`, the commit of this change that edits the draft |

No frozen clause, vector or dependency meaning changed:

- The packet cites only JI.21, JI.22, JI.23, JI.25 and JI.27. Section 2 of the draft, which holds JI.17 to JI.31, is
  byte-identical before and after both edits.
- The candidate files and `candidate_sha256` are unchanged.
- `check_vectors.py` verifies every other candidate and input pin against the same bytes as before.

## What follows

- M5 may start. The LP2 plan's section 2 and its M5 row say so, and CT-JI-03 joins M5's tests (JI.46).
- M5's pull request records what JI.54 leaves to it:
  - what a Net-protection refusal reports;
  - the identity blocks of the S4 and I_b fixtures;
  - the batch budgets and chunk sizes;
  - the internal free-mode entry;
  - whether JI.16's vector holds as written.
- No progress item is ticked for the approval. M5's item stays open until M5's pull request.
- This week's governance share is above 30%. This governance and documentation work was done at the owner's word
  ("resolve it now").
