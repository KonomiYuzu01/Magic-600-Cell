---
id: owner-decisions-2026-11-15-freeze
type: decision
status: verified
visibility: public
summary: Owner decisions of 15 November 2026 on the language - after both Astra freeze rulings went go, the owner confirmed understanding and signed LP1. ADR-L01 to ADR-L12 are accepted, the language architecture and the migration format are frozen, the 6 November charter amendment is recorded so that LP2 to LP5 may start, the design-system ADR is named as outstanding, and the frozen portions of H8, H9, H10 and H12 are accepted. The owner also accepted state-contract amendment A6.
related: [owner-decisions-2026-11-14-language-route, owner-decisions-2026-11-06, owner-decisions-2026-11-03, owner-decisions-2026-11-14]
supersedes: []
claims:
  - {id: lp1-understanding-confirmed, evidence_kind: decision, checked_at: 2026-11-15}
  - {id: language-adrs-accepted, evidence_kind: decision, checked_at: 2026-11-15}
  - {id: architecture-frozen, evidence_kind: decision, checked_at: 2026-11-15}
  - {id: migration-format-frozen, evidence_kind: decision, checked_at: 2026-11-15}
  - {id: charter-amendment-recorded, evidence_kind: decision, checked_at: 2026-11-15}
  - {id: a6-accepted, evidence_kind: decision, checked_at: 2026-11-15}
---

# Owner decisions, 15 November 2026: LP1 sign-off and the language freeze

On 15 November 2026 both Astra freeze rulings stood at go. The architecture-freeze ruling of 14 November 2026 had four
findings; all were fixed, and its scoped re-check passed at `37dd0079`. The migration-format-freeze ruling of the same
day had three findings and its re-check two more; all were fixed, and a second call that the owner asked for passed at
`1bd7f3fe` with no finding. Between those two commits only the format packet and two paragraphs of chapter 05 changed,
and the format ruling read them.

The integrator then put two questions to the owner in plain words: what freezes, what does not, what stays open and
which evidence classes the candidate rests on. The owner answered in Chinese and picked the recommended option in both.
This page records the answers in English.

## Decisions

| Question | Answer | Follows the recommendation |
| --- | --- | --- |
| LP1 understanding and sign-off (chapter 12, PHS.5, PHS.51, PHS.57) | Understood; sign everything: accept ADR-L01 to ADR-L12, freeze the language architecture and the migration format, record the 6 November charter amendment, and accept the frozen portions of H8, H9, H10 and H12 | Yes |
| State-contract amendment A6, exact rotation requests (chapter 12, PHS.53, EN1 row) | Accept it now, together with the sign-off | Yes |

1. **Understanding.** The owner's answer is the recorded response that PHS.57 asks for. It is not inferred from silence
   or from agreement between models.
2. **ADRs.** [ADR-L01 to ADR-L12](../../architecture/1.0/language/adr/README.md) are accepted as written at the freeze.
   Their open lists and the alternatives the owner may overrule stay in each record.
3. **Architecture.** The [language architecture](../../architecture/1.0/language/README.md) is frozen as the candidate
   the architecture-freeze ruling read: the chapters, the ADRs, the annexes, EN4's
   [layer interfaces](../../progress/1.0/layer-interfaces.md) and the renderer feature list.
4. **Migration format.** The [format packet](../../architecture/1.0/language/format-packet/README.md) is frozen: PS2.10's
   language-neutral data core and the facts payload (PHS.49, FP.2). Its `candidate.json` records `frozen: true`, with
   every pin refreshed at the freeze (FP.1).
5. **Charter amendment.** The 6 November amendment (PHS.44, LQ19) is recorded: LP2 to LP5 may start. The design-system
   ADR is outstanding. It conditions LP4's editor look and LP6, and the stage 2.5 exit stays incomplete until it is
   accepted.
6. **H8, H9, H10 and H12.** The format packet consumes four working forms: H8 for text and output accounting, H9 for
   no-run provenance, H10 for flat menu words and H12 for aliases. Their portions inside the frozen core are accepted.
7. **A6.** EN1's Astra check of A6 was part of the architecture-freeze ruling, which found the text correct and ready
   for the owner; the re-check kept that finding. With the owner's acceptance A6 is in force and, under the state
   contract's amendment rule, holds over A1 where they differ. It replaces A1's input map only.

## Freeze identity

| Item | Value |
| --- | --- |
| Architecture candidate | The language architecture at `37dd0079`, with the format changes read at `1bd7f3fe` |
| Format candidate | `en4-data-core-draft-1` at `1bd7f3fe` |
| Freeze commit | `5ad74a96`: `candidate.json` records `frozen: true`, with every pin refreshed against input commit `a5b24fe9`, the sign-off's status commit |
| Final inventory digest | `candidate_sha256` `661e01ebc197b9eedf0246f41b7d5b317bd95059937a0efcb5dd434da1726952` |

## Integrator readings (the owner may change them)

- The freeze is the conditional freeze of PHS.52: a changed format, layer boundary or renderer feature follows its
  recorded revision, review and owner acceptance. A change to the frozen format core also follows FP.1's pin rules.
- The status lines and pins changed at the sign-off are part of the sign-off itself, not such a change. The `status`
  strings inside the annexes' data files and the closure records keep the wording the rulings read.
- [O] clauses stay open until the decision they name, and [P LQ13] clauses stay conditional on LQ13. Syntax and the IR
  freeze later (PHS.49), and the command table freezes at LP3 (PHS.53).
- EN3's [jumbling interfaces](../../progress/1.0/jumbling-interfaces.md) keep their own review and approval: a linked
  draft of another line records a supplied specification, not that line's acceptance (PHS.53).
- The architecture ruling was not repeated at `1bd7f3fe`, because the only changes after `37dd0079` were the format
  fixes that the format ruling read.
- A product dependency, a big-integer crate included, still needs the owner's approval.

## What this sign-off does not settle

- The design-system ADR, and with it LP4's editor look, LP6 and the stage 2.5 exit.
- LQ13, which waits for LX-8 before LP2's parser, and LQ2's option set, which waits for the joint setting. Until then
  built-in tools keep the 2 October boundary and the 17 October repair-list exception, and no solver-tool solving
  preview is enabled (PHS.7).
- The LP3 acceptance runs: LX-1, LX-2 and LX-4 keep their bars and a failing one reopens LQ9; a failing LX-3 check goes
  to the owner.
- The renderer selection, the 1.0 importer and the design line's hold.
- Any Windows, DirectX, long-session or performance claim. The candidate rests on source, fixture and synthetic
  evidence; the rulings are reviews.

## Effect

- Accepted: ADR-L01 to ADR-L12 and state-contract amendment A6 in the
  [state contract](../../../research/jumbling/state-contract.md).
- Frozen: the language architecture and the migration format.
- Superseded in part: the closing sentences of the [6 November](owner-decisions-2026-11-06.md) and
  [14 November](owner-decisions-2026-11-14-language-route.md) pages that no ADR is accepted and nothing is frozen.
- Unchanged: the model identities and mathematical contracts, the protection rules, the renderer selection gate, the
  human-solve boundary for built-in tools, the design-line hold and the Codex minimum.
- Next: LP2 to LP5. The briefing and the status lines of the language documents carry this page.
