---
id: owner-decisions-2026-11-03
type: decision
status: verified
visibility: public
summary: Owner decisions of 3 November 2026 - the results of the restoration research the owner restarted on 2 November enter the repository; the engineering line develops the project's own language first, with the round table's mathematician and engineer assisting, while UI/UX and the design line wait until it is complete; the owner closes and freezes the renderer selection in parallel; a tool a solver develops through the built-in base tools while understanding its principle does not count as the program outputting a solution, with the exact boundary set jointly; the language and what the research needs of it are in 1.0's scope; the earlier schedule is set aside.
related: [owner-decisions-2026-10-27-jumbling-handoff, owner-decisions-2026-10-27-freeze, owner-decisions-2026-10-22-dt-m1, owner-decisions-2026-10-22, owner-decisions-2026-10-02, owner-decisions-2026-10-02-scope, renderer-candidates]
supersedes: []
claims:
  - {id: research-results-to-repository, evidence_kind: decision, checked_at: 2026-11-03}
  - {id: language-first, evidence_kind: decision, checked_at: 2026-11-03}
  - {id: round-table-for-the-language, evidence_kind: decision, checked_at: 2026-11-03}
  - {id: design-line-waits, evidence_kind: decision, checked_at: 2026-11-03}
  - {id: renderer-close-and-freeze, evidence_kind: decision, checked_at: 2026-11-03}
  - {id: self-developed-tools, evidence_kind: decision, checked_at: 2026-11-03}
  - {id: language-in-scope, evidence_kind: decision, checked_at: 2026-11-03}
  - {id: schedule-set-aside, evidence_kind: decision, checked_at: 2026-11-03}
---

# Owner decisions, 3 November 2026: the language first

The owner gave these decisions in one chat message in Chinese on 3 November 2026, after reading the integrator's
summary of three handoff documents: two exported local conversations of 2 November 2026 (a language for 1.0's
components, and projection schemes for the jumbling puzzle) and a combined reference of 3 November. The documents are
private and not in the repository. This page records the decisions in English.

## Context

- On 2 November 2026 the owner restarted the jumbling restoration research in a local session. Under the
  [27 October freeze](owner-decisions-2026-10-27-freeze.md) reopening the research is an owner decision, and this was
  one. The work stopped the same day at the owner's instruction, and its automatic continuation is paused.
- The same sessions proposed a project-specific language (working name C600 Script), an application kernel, a
  programmable research workbench and a projection baseline for the jumbling puzzle, with a browser prototype.

## Decisions

1. **The research results enter the repository.** The records, proofs and evidence of the 2 November research come to
   `main`.
2. **The engineering line develops the language first.** After the research results are merged, the owner starts the
   language's development in a local session, beginning with its complete ecosystem and architecture.
3. **The round table continues, and the mathematician and the engineer assist the language.** The owner calls the
   language with its tools and its graphical UI a "Skill": learned over time until using it is efficient.
4. **UI/UX and the design line wait** until the language's development is complete.
5. **The renderer.** The owner closes the renderer experiments soon, in parallel, then selects a renderer and freezes
   it.
6. **Tools a solver develops.** If a solver develops a tool themselves, through the built-in base tools and while
   understanding its principle, and the tool assists solving, this does not count as the program outputting a solution
   by itself. The owner and the integrator set the exact boundary together.
7. **The handoff's proposals are references only.** The specific schemes in the handoff documents (syntax sketches,
   example tools, the projection baseline and its prototype) are references, not decisions. The objects they name are
   a small part of what the language needs: complete restoration and the jumbling research use many more.
8. **Scope.** The language, its ecosystem and what restoration and the jumbling research need of it are in 1.0.
9. **Schedule.** The earlier schedule is set aside for now; other matters kept the owner from continuing development.

## Integrator readings (the owner may change them)

- Decision 1 brings the research to the repository as records with their review status: no proof of it has had a
  human review, and its checks are replays and model reviews. Bringing it in restarts nothing; the research stays
  stopped until the owner says otherwise.
- Decisions 3 and 4: the round table resumes for the language only, with the mathematician and the engineer advisor.
  The front-end designer and the interaction artist, design-table topics DT-J2 to DT-J4, and the design line's Look Lab
  and Taste Lab work wait until the language's development is complete. DT-J1 and DT-M1 keep their answers of
  22 October.
- Decision 5: the selection gate of [renderer-candidates](renderer-candidates.md) still decides, on the owner's
  machine.
- Decision 6 adds to the [2 October](owner-decisions-2026-10-02.md) human-solve boundary for tools that solvers
  develop. Until the exact boundary is set, the built-in tools themselves keep the 2 October boundary and its
  [17 October exception](owner-decisions-2026-10-17-repair-pairs.md).
- Decision 8 adds to the [2 October scope](owner-decisions-2026-10-02-scope.md): design stays in 1.0 and follows the
  language.
- Decision 9: stage day counts and target days, the 2.5 freeze at about stage day 34
  ([22 October](owner-decisions-2026-10-22.md)) among them, do not bind until the owner sets a new schedule. No gate is
  relaxed.

## Effect

- Superseded in part: the hold of the round table ([22 October](owner-decisions-2026-10-22-dt-m1.md)), which resumes
  for the language while the design line stays on hold; and the schedule of [22 October](owner-decisions-2026-10-22.md),
  which is set aside.
- Unchanged: the model identities and mathematical contracts of 600-cell-Full and of the jumbling puzzle, the
  protection rules, the review rules, the evidence kinds and the renderer selection gate.
- Next: the research records come to `main` in their own pull request; then the owner's local session starts the
  language's ecosystem and architecture.
