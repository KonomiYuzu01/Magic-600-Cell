# Architecture decision records for the language

Status: **all twelve records are proposed. None is accepted.**

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
- Accepting a record accepts its Decision section and the [D] clauses it cites. It answers no owner question: a [P LQn]
  clause stays conditional on LQn, and an [O] clause stays open.
- Accepting a record freezes nothing. The architecture freeze and the migration format freeze are separate gates
  (chapter 12, section 11).
- Each record ends with the additions the owner may overrule. Overruling one changes the clause it names and the record.
- After acceptance a record's decision does not change. A later change is a new record that supersedes it.

## Index

| Record | Decision | When the owner can accept it | Questions it leaves open inside it |
| --- | --- | --- | --- |
| [ADR-L01](ADR-L01-one-state.md) | One mechanics and one authoritative state | LP0 | LQ4, LQ6 |
| [ADR-L02](ADR-L02-runtime-as-client.md) | The runtime as a client of the command layer, off the critical path | LP0 | LQ9 |
| [ADR-L03](ADR-L03-exactness.md) | Exactness, the numeric tower and the algebras | LP0 | LX-5, LX-9 |
| [ADR-L04](ADR-L04-canonical-identity.md) | Canonical identity, the identity tower and source maps | LP0 | None from the owner's list |
| [ADR-L05](ADR-L05-evidence.md) | The evidence model and certificates | LP0, with LQ3 | LQ10, LQ15 |
| [ADR-L06](ADR-L06-effects-and-flow.md) | Effects, capabilities and flow control | LP0, without the form of flow control | LQ21, which holds that part for LP1; LQ2 |
| [ADR-L07](ADR-L07-typed-ir.md) | The typed IR as the stable contract | LP1: it follows the route | LQ9 |
| [ADR-L08](ADR-L08-jobs-and-budgets.md) | Jobs and deterministic budgets | LP0 | LQ4 |
| [ADR-L09](ADR-L09-boundary-mechanisms.md) | Boundary mechanisms, with the policy left to the joint setting | LP0 | LQ2, LQ14, LQ21 |
| [ADR-L10](ADR-L10-persistence.md) | Persistence formats, the data core and editions | LP0 | LQ10, LQ11, LQ12, LQ19 |
| [ADR-L11](ADR-L11-display-plans.md) | Display plans and user charts | LP0 | LQ6, LQ7, LQ20 |
| [ADR-L12](ADR-L12-route.md) | Implementation route and runtime placement | LP1: it follows the evidence | LQ8, LQ9, LQ15, LQ18, LQ21 |

[Chapter 12](../architecture/12-phases-and-conformance.md), PHS.9 and PHS.10, sets the timing: at LP0 the owner accepts
every draft except the form of flow control in ADR-L06, ADR-L07 and ADR-L12, which stay for LP1. ADR-L05's Decision is
itself a proposal under LQ3, so accepting it and answering LQ3 are one step.

## Format

Each record has the same sections: Context, Decision, What this record does not decide, Why, Alternatives considered,
Consequences, Evidence, and What the owner accepts.

Decision items are numbered ADR-Lnn.m, because the charter's bars already use D1 to D8. Clause IDs, status tags and
evidence classes are those of [the overview](../architecture/00-overview.md), section 3.

A record's status is one of Proposed, Accepted, Rejected or Superseded. When the owner decides, the integrator records
the decision under `docs/wiki/decisions/`, and the record's status line links it.

The explanations the kickoff asks for, in plain words and for the owner, are separate from these records. Each record
names the round-table explainer that covers it where one exists.
