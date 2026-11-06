---
id: owner-decisions-2026-11-06
type: decision
status: verified
visibility: public
summary: Owner decisions of 6 November 2026 - the owner answered the language questions LQ1 to LQ21 on the questions page. Sixteen answers take the recommendation. Five do not - free views everywhere, which lifts charter D8's limit to two views; a shader language for users; unattended playback of view and workspace steps in a recording; export of proof obligations to Lean; and the name 600 Cell Script with the extension .6cs. The answer to LQ19 amends the charter so that LP2 to LP5 may start after both freeze rulings. LQ8, LQ9, LQ13 and LQ21 wait for their evidence, so the freeze rulings follow that evidence. No ADR is accepted and nothing is frozen by these answers. Follow-up answers of the same day choose exact requests for the A1 input question, confirm that a live twist has no preview, set who writes evidence fields and where tool words are refused, and allow a big-integer crate and Linux tools in WSL for the experiments.
related: [owner-decisions-2026-11-05, owner-decisions-2026-11-03, owner-decisions-2026-10-22-dt-m1, owner-decisions-2026-10-03, owner-decisions-2026-10-02-design, owner-decisions-2026-10-02-scope, renderer-candidates]
supersedes: []
claims:
  - {id: language-questions-answered, evidence_kind: decision, checked_at: 2026-11-06}
  - {id: free-views-everywhere, evidence_kind: decision, checked_at: 2026-11-06}
  - {id: shader-language-for-users, evidence_kind: decision, checked_at: 2026-11-06}
  - {id: recording-unattended-view-workspace, evidence_kind: decision, checked_at: 2026-11-06}
  - {id: proof-obligations-export-to-lean, evidence_kind: decision, checked_at: 2026-11-06}
  - {id: language-name, evidence_kind: decision, checked_at: 2026-11-06}
  - {id: theory-research-in-repository, evidence_kind: decision, checked_at: 2026-11-06}
  - {id: charter-order-amended, evidence_kind: decision, checked_at: 2026-11-06}
  - {id: a1-exact-request-direction, evidence_kind: decision, checked_at: 2026-11-06}
  - {id: live-twist-no-preview, evidence_kind: decision, checked_at: 2026-11-06}
  - {id: evidence-field-writers, evidence_kind: decision, checked_at: 2026-11-06}
  - {id: tool-words-refused-at-preview, evidence_kind: decision, checked_at: 2026-11-06}
  - {id: experiment-crate-and-wsl-allowance, evidence_kind: decision, checked_at: 2026-11-06}
---

# Owner decisions, 6 November 2026: the answers to the language questions

The owner answered the questions page and said so in a chat message in Chinese on 6 November 2026. The questions are
LQ1 to LQ21 of the [ecosystem partition](../../architecture/1.0/language/ecosystem-partition.md), section 15. This page
records the answers in English.

## Decisions

| Question | Answer | Follows the recommendation |
| --- | --- | --- |
| LQ1 One language or several | One language with three profiles | Yes |
| LQ2 The boundary for tools a solver develops | Build the mechanisms now; choose the option set together later | Yes |
| LQ3 Evidence model | Separate fields; the five words are computed from them | Yes |
| LQ4 Branches and research runs | Engine-owned journal branches; pinned jobs finish | Yes |
| LQ5 Built-in tools in the language | Every tool above the kernel is written in the language | Yes |
| LQ6 Views and charter D8 | **Free views everywhere** | No |
| LQ7 User charts and the renderer | **A shader language for users** | No |
| LQ8 Engine language | Decide after LX-5 and LX-6 | Yes |
| LQ9 Route and runtime placement | Decide after LX-1 to LX-5, together with LQ21 | Yes |
| LQ10 Persistence and the migration format freeze | Freeze only the data core; the envelope stores facts | Yes |
| LQ11 Recording | **Allow unattended playback of view and workspace steps; commits stay manual** | No |
| LQ12 Sharing modules in 1.0 | Local only, through files | Yes |
| LQ13 Syntax style | Decide after LX-8, before LP2 builds the parser | Yes |
| LQ14 Learning gates | Soft disclosure: guide, never lock | Yes |
| LQ15 Formal proofs | **Export proof obligations to Lean** | No |
| LQ16 The 1.0 cut | All five layers, with the note in decision 5 | Yes |
| LQ17 Name | **Name it now: 600 Cell Script, extension `.6cs`** | No |
| LQ18 Evidence spikes | Allow LX-1 to LX-9 as experiment code | Yes |
| LQ19 The freeze gates and the charter's stage 2.5 exit | Amend the charter: LP2 to LP5 may start after both rulings | Yes |
| LQ20 A handoff to the renderer line | Add the handoff row now | Yes |
| LQ21 Flow control in the solving profile | Decide after LX-1 and LX-3, together with LQ9 | Yes |

1. **Free views everywhere.** The owner chose the option that reopens their own design decision. Charter D8's "They are
   the only two views" and "There is no third view" no longer bind.
2. **A shader language for users.** Users write display programs for the graphics card. The architecture has to give
   this a language and safety rules, and the exact checks and the gate tests become harder.
3. **Recording.** A recording may play back view and workspace steps unattended. A commit never plays unattended.
4. **Lean export.** The language stays as it is, and a separate export sends proof obligations to Lean. The evidence
   model gains a check category and persistence gains an export format.
5. **The 1.0 cut.** 1.0 ships all five layers, including the research library and the checkers for the research archives.
   The owner's note on this answer, translated: "Theory research does not go into the software, but the repository has to
   hold what relates to it."
6. **The name.** The language is named 600 Cell Script and its source files use the extension `.6cs`. "C600 Script" and
   `.c6s` were working names.
7. **The charter's order.** Both freeze rulings sit at LP1's exit. The charter is amended so that LP2 to LP5 may start
   after the two rulings. The language's development counts as complete when LP2 to LP5 are done apart from LP4's editor
   look. The design line resumes at that point, and its design-system ADR is the condition for LP4's editor look and for
   LP6.

## Integrator readings (the owner may change them)

- **Decision 1** lifts the limit on the number of views and nothing else in D8: Global and Local are still redesigned, each
  still answers its own question, and the two are still visible together and linked to one context and revision. A
  further view shows one revision and keeps its identity and source linkage. The renderer selection gate is unrelaxed, so
  the view workload needs a stated resource budget and its own gate run. How views look and where they sit stays with the
  design line, which remains on hold.
- **Decision 2** reads "shader language" as a language that the project's own pipeline checks and compiles to the one
  portable HLSL source, and not as the upload of raw shader code. Display values never reach an exact path (INV-4). No
  global TDR or security setting changes. The safety contract is written before any user stage is enabled, and the
  earlier LX-7 results apply to the subset they tested.
- **Decision 3:** a step counts as a view or workspace step by its command row and lane, so a command that uses the commit
  lane stays manual whatever its effect is called.
- **Decision 4 and LQ16's option text.** LQ16's option says formal proofs come after 1.0, and LQ15 asks for the export. The
  integrator reads LQ15 as the specific answer. The export of proof obligations is in 1.0. Lean is neither shipped nor a
  runtime dependency, a Lean result enters only as a record with its named check, and a failed check is still no
  refutation. Lean is not on the allowlist, so a project check that runs it needs the owner.
- **Decision 5's note:** the theory book, proofs and stage research records stay out of the shipped software and live in the
  repository. The research library and the archive checkers stay in 1.0, as the chosen option says. The note also answers
  EV5.17: the theory book does not extend to the language's libraries.
- **Decision 6:** the name changes no module identity bytes. The `c600-` prefix of draft format identifiers names the
  project's data, not the language, and it stays until the format freeze fixes the final spelling.
- **Decision 7** keeps "no gate is relaxed": the stage 2.5 exit keeps all three parts, and only their order changes for the
  language line.
- **LQ20 with LQ7's answer:** the handoff carries the user shader stage, the free-view workload and the safety contract
  in place of the generated projection stage.
- **LQ18:** the allowance of 5 November stands as LQ18's answer.

## Follow-up answers, the same day

The integrator put seven questions and two defaults to the owner in chat. The owner answered in one message: option B for
the first question, and agreement with the rest.

8. **The A1 request form (LX-9): option B.** A fresh rotation request becomes an exact request read by its bytes: a dyadic
   Cayley vector, an explicit half-turn form or a bound menu letter. A menu drag compares its candidates exactly in
   Q(√5) under a recorded menu order. The owner chose the direction; state contract A1 changes through its own process,
   EN1's Astra check of the amendment text and then the owner.
9. **The readings of decisions 4 and 5 are confirmed.** The theory book, proofs and research records stay out of the
   software and live in the repository, and the research library and the archive checkers stay in 1.0. The export of
   proof obligations is in 1.0, Lean does not ship with the software, and formal proofs themselves come after 1.0.
10. **Live twists (ONE.78).** Charter S4's preview applies to a turn entered as a draft step, to a macro and to any word.
    A live twist shows nothing first, as solving workflow 4.1 and DT-M1 C4 say, so `twist.apply` gains no preview.
11. **Who writes evidence records: option A.** Each field has a named writer. Only the kernel writes the independent
    checker value. A theorem basis cites a record pinned to a commit. Model and human review cite a digested review
    record. A user program writes at most exact computation with the check producer, and anything higher comes only
    through a certificate the kernel can check.
12. **Tool words through macros: option A.** The check sits at the point of use, as in BND.61. In a solving session a word
    whose record block or any part says that the solver's tool computed it is refused at preview, whatever route it
    took. The bytes stay readable, and hand-entered words and imported 0.4 macros (`producer: import`) are not affected.
13. **Big-integer crates.** LX-5 may fetch a `num-bigint` class crate once the integrator has checked its licence. This is
    experiment code, and a copyleft licence still needs the owner.
14. **Linux tools in WSL.** One installation of Rust, Node and .NET in WSL is allowed for the Linux legs of LX-2, LX-5,
    LX-6 and the LX-9 follow-up.
15. **Decisive timing and foreground windows.** The owner says when the machine is free. LX-2 and LX-4 run on the existing
    renderer candidates first and run again if the renderer changes.

## What these answers do not settle

- LQ8, LQ9, LQ13 and LQ21 chose to wait for their evidence, and LQ2's option set waits for the joint setting. The freeze
  rulings need LQ8, LQ9 and LQ21 first, so the freeze follows the decisive experiment runs.
- Option B leaves two things for the amendment text: what free mode does with an exact request (the rotation R(ω) as
  named, or the nearest point of A1's rational grid), and what produces the request bytes from a gesture.
- The "proved" threshold stays open, and so do the owner's windows for decisive timing and foreground runs.
- No ADR is accepted, nothing is frozen and no renderer is selected. The owner accepts ADRs and signs off after both
  rulings.

## Effect

- Superseded in part: charter D8 and section 5's limit to two views, with the matching line of the
  [2 October design decisions](owner-decisions-2026-10-02-design.md); the charter's stage 2.5 exit order for the language
  line; the [5 November](owner-decisions-2026-11-05.md) reading that keeps the whole exit in front of implementation until
  LQ19 is decided.
- Unchanged: the model identities and mathematical contracts, the protection rules, the review rules, the evidence kinds,
  the renderer selection gate, the human-solve boundary for built-in tools and the design-line hold.
- Superseded in part: ONE.78's wait for the owner's reading of charter S4, which decision 10 answers.
- Next: the architecture text is finished from these answers and reviewed as one final candidate.
