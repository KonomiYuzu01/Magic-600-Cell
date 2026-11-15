# Architecture decision records for the language

Status: **all twelve records are accepted** (owner, 15 November 2026;
[decision](../../../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)).

The owner answered LQ1 to LQ21 on [6 November 2026](../../../../wiki/decisions/owner-decisions-2026-11-06.md) and
chose LQ8, LQ9 and LQ21 on [14 November 2026](../../../../wiki/decisions/owner-decisions-2026-11-14-language-route.md):
the engine and the interpreter's host in Rust, route RT1 with the runtime in a process of its own, and capability
separation. The answers are reflected in the decisions. Those answers accepted no ADR and froze nothing; the owner
accepted the twelve records with the LP1 sign-off of 15 November 2026, after both freeze rulings.

These records name the decisions the [architecture document](../architecture/00-overview.md) rests on. The list is
[partition section 17](../ecosystem-partition.md)'s. The chapters hold the contracts. A record says what is decided,
why, what was turned down and what it costs, and it cites clauses instead of restating them. Where a record and a clause
differ, the clause is the text to read and the difference is a defect to report.

## What acceptance does

- Only the owner accepts or rejects a record ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the owner"). A passed review
  and agreement between models accept nothing.
- Until then a record binds nobody. What binds is what the chapters tag [B], which restates a source already in force.
- An accepted record ranks where `AGENTS.md` puts accepted ADRs: below owner decisions and the rule files, above source
  evidence and the wiki. That order governs decisions and instructions. It cannot turn an unrun check into evidence.
- Accepting a record accepts its Decision section and the [D] clauses it cites. It answers no owner question. The
  remaining proposal tag, [P LQ13], stays conditional on the waiting answer; an [O] clause stays open.
- Accepting a record freezes nothing. The architecture freeze and the migration format freeze are separate gates
  (chapter 12, section 11).
- Each record ends with the additions the owner may overrule. Overruling one changes the clause it names and the record.
- After acceptance a record's decision does not change. A later change is a new record that supersedes it.

## Index

| Record | Decision | Status | Questions it leaves open inside it |
| --- | --- | --- | --- |
| [ADR-L01](ADR-L01-one-state.md) | One mechanics and one authoritative state | Accepted 15 November 2026 | Preview replacement waits for the design line |
| [ADR-L02](ADR-L02-runtime-as-client.md) | The runtime as a client of the command layer, off the critical path | Accepted 15 November 2026 | None from the owner's list; LX-2's decisive run is an LP3 acceptance check |
| [ADR-L03](ADR-L03-exactness.md) | Exactness, the numeric tower and the algebras | Accepted 15 November 2026 | The big-integer implementation, chosen in LP2 on LX-5's rows |
| [ADR-L04](ADR-L04-canonical-identity.md) | Canonical identity, the identity tower and source maps | Accepted 15 November 2026 | None from the owner's list |
| [ADR-L05](ADR-L05-evidence.md) | The evidence model and certificates | Accepted 15 November 2026 | The "proved" threshold; invalid pairs and absent-format records |
| [ADR-L06](ADR-L06-effects-and-flow.md) | Effects, capabilities and flow control; the form is capability separation | Accepted 15 November 2026 | LQ2's option set |
| [ADR-L07](ADR-L07-typed-ir.md) | The typed IR as the stable contract, under route RT1 | Accepted 15 November 2026 | LQ13 |
| [ADR-L08](ADR-L08-jobs-and-budgets.md) | Jobs and deterministic budgets | Accepted 15 November 2026 | Production tariffs and conformance |
| [ADR-L09](ADR-L09-boundary-mechanisms.md) | Boundary mechanisms, with the policy left to the joint setting | Accepted 15 November 2026 | LQ2's option set |
| [ADR-L10](ADR-L10-persistence.md) | Persistence formats, the data core and editions | Accepted 15 November 2026 | Format review, implementation and evidence; no unanswered question from the owner's list |
| [ADR-L11](ADR-L11-display-plans.md) | Display plans and user stages | Accepted 15 November 2026 | LQ2's display sinks; precision rules and gate evidence |
| [ADR-L12](ADR-L12-route.md) | Implementation route and runtime placement: Rust, RT1, a process of its own | Accepted 15 November 2026 | LQ13; a failing LP3 acceptance run reopens LQ9 |

[Chapter 12](../architecture/12-phases-and-conformance.md), PHS.10, PHS.13 and PHS.15, sets the timing: at LP0 every draft
is reviewed and ready, and the owner accepts ADRs after both LP1 rulings. The owner chose the engine language, the
route and placement and the form of flow control in ADR-L06, ADR-L07 and ADR-L12 on 14 November 2026 (LQ8, LQ9,
LQ21). ADR-L05's Decision is the draft mechanism for the model chosen in LQ3 (owner, 6 November 2026, LQ3); that
answer did not accept the ADR.

The freeze rulings needed LQ8, LQ9 and LQ21 first (owner, 6 November 2026), and the owner answered them on
14 November 2026. LQ13 waits for LX-8 before LP2's parser, and LQ2's option set waits for the joint setting. Both
freeze rulings sit at LP1's exit; LP2 to LP5 follow both rulings and the owner's sign-off, and the design-system ADR
conditions LP4's editor look and LP6 (owner, 6 November 2026, decision 7). No gate is relaxed. Both rulings went go
and the owner signed on 15 November 2026, so LP2 to LP5 may start; the design-system ADR is still outstanding.

## Format

Each record has the same sections: Context, Decision, What this record does not decide, Why, Alternatives considered,
Consequences, Evidence, and What the owner accepts.

Decision items are numbered ADR-Lnn.m, because the charter's bars already use D1 to D8. Clause IDs, status tags and
evidence classes are those of [the overview](../architecture/00-overview.md), section 3.

A record's status is one of Proposed, Accepted, Rejected or Superseded. When the owner decides, the integrator records
the decision under `docs/wiki/decisions/`, and the record's status line links it.

The explanations the kickoff asks for, in plain words and for the owner, are separate from these records. Each record
names the round-table explainer that covers it where one exists.
