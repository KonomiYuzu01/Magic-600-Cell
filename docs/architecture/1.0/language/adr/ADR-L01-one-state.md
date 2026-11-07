# ADR-L01: one mechanics and one authoritative state

Status: **Proposed.** Not accepted. The owner accepts or rejects it ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the
owner").

Rests on: [chapter 08](../architecture/08-one-state.md), [chapter 01](../architecture/01-kernel.md) KN2, KN5; partition
INV-1 to INV-3 and section 8.3. Round table: DT-L1 E1, E2, R2-E2.

## Context

Tools have to reason about states the session has not reached: what a word would do, what a search would find, what an
imported pose table holds. Solvers have to see a word's effect before it counts. Both need something that looks like a
puzzle state and is not the session's state.

0.4 already had one such thing, the preview, and its screening required that later changes by a caller cannot reach what
is committed (R-10). A language multiplies the callers. So the project needs one answer to three questions: what the
state is, what else may look like a state, and how any of it can become real.

## Decision

- **ADR-L01.1** Each session has exactly one state of record: the state at the journal head of its active line, labels
  with Jumble off and poses with Jumble on. The engine owns it and the store records it (ONE.1). Exit validity across
  history follows the reconciled working transitions in closure H11 and JI.27/JI.28; equal labels cannot revive a
  void exit. Those clauses own the transition facts; this record creates no separate lineage rule.
  Experiment branches are journal branches the engine owns, with exactly one active line per session, as KN4.6 and
  ONE.73 to ONE.77 describe (owner, 6 November 2026, LQ4).
- **ADR-L01.2** Only rows of kind `commit` change it, through the command layer and its gates (ONE.2). No such row is
  available to the runtime in any profile or placement (ONE.3). Every one reaches the command layer with a user-action
  token from the shell (ONE.4, KN5.14).
- **ADR-L01.3** Eight things can look like a state, and only the state of record is one (ONE.5). A program may copy what
  it reads. Such a copy has no standing, and it can enter a kernel call only through KN2.14, as a derived state (ONE.6).
- **ADR-L01.4** A derived state is an immutable value computed by the same exact kernels. It has no session revision, it
  lives in the value store by digest, and it is charged to the program's memory budget (KN2.11 to KN2.15, ONE.20 to
  ONE.30). It is never adopted. It becomes real only when the solver plays its word through `propose` and the engine
  replays that word at the current revision (ONE.26). A derived state with no word cannot be proposed. It can still be
  read, compared, displayed and derived from, and what is derived from it keeps its reachability claim (ONE.27).
- **ADR-L01.5** The preview is the single exception: the one state-like value a commit can adopt.
  - The engine builds it and owns its immutable witness (KN5.9, ONE.31).
  - A proposal never carries a state or a digest for the engine to adopt (KN5.13).
  - At most one preview is pending (KN5.11, ONE.38).
  - Freshness uses KN0.8's dependency predicate. An unrelated workspace edit preserves it; a changed state,
    protection or resolved draft dependency invalidates it (ONE.39). When the state revision moves the engine
    discards the preview, and the draft stays (ONE.40).
  - The token binds to the preview of the newest frame reported as presented (ONE.41). Only the built-in chain draws
    a frame with that presented-preview identity; a user-stage display carries none (RPI.76).
  - At step 9 the engine adopts its own object, so that step needs no trust in the language (ONE.44).
  - A preview is not durable (ONE.43).
- **ADR-L01.6** Every result names the revision or derived state it read, and it is never applied to or shown against
  another (KN0.6, ONE.7, ONE.11). KN5.21/KN5.23 and LI.27 to LI.30 now draft the result's invocation binding,
  terminal receipt and outcome lookup. A lost notification or released result payload does not undo a durable
  success, and a completed preview receipt does not certify that the preview is still pending or fresh.
- **ADR-L01.7** The view model holds no puzzle copy. An adopted bundle is a read-only image of one revision, replaced
  whole, and nothing decides from it (ONE.59 to ONE.61). The renderer draws no uncommitted state as the session state,
  with two named exceptions, each drawn as what it is (ONE.62).
  Further views are allowed in every profile (owner, 6 November 2026, LQ6). Global and Local stay visible together and
  linked; each further view shows one revision and keeps instance identity and source linkage (RPI.42, RPI.81).
  Resource budgets and their unchanged gate govern the supported workload (RPI.60, RPI.82, RM.39).
- **ADR-L01.8** A live twist is a commit with no preview: an immediate complete operation that the solver's key
  names and the active protection gates (ONE.46, ONE.47). This reads charter S4 with solving workflow 4.1 and DT-M1 C4,
  as the owner confirmed in ONE.78 (owner, 6 November 2026, decision 10). The other no-preview paths are LI.36's
  crossing (ONE.80) and undo and redo (LI.26).

## What this record does not decide

The owner answered LQ1 to LQ21 on 6 November 2026. LQ8, LQ9, LQ13 and LQ21 wait for their evidence, and LQ2's option set
waits for the joint setting.

Open mechanisms and waiting questions:

- Whether an invoked command may replace a preview already on screen (KN5.12, LG3.7, ONE.42). The token keeps the commit
  correct either way, and the choice waits for the design line.
- Where the runtime runs (ONE.69, LQ9). LX-1 to LX-5 feed the choice with LQ21. Every clause above must hold in each
  placement.
- Review and implementation of the drafted freshness, receipts, history transitions and presented-frame protocol
  (KN0.8, LI.26 to LI.30, closure H11 and RM.26). Their descriptions here accept none of those drafts.

## Why

- A state does not witness its own reachability. A solved state with one centre turned by a third-turn of A4₀ passes the
  sticker test and is unreachable in J(A4₀) (KN1.17, ONE.28). So no check of a state's form can justify adopting it.
  Only replaying a word through the gates can.
- Keeping commit rows away from the runtime is what gives the trust table its first line: a bug in the language can make
  a tool lie, and it cannot corrupt a session or commit a word the solver did not see (overview, section 7).
- Reads through snapshots give a program the same data as a mirror would, with no copy to keep in step (KN2.2, KN2.3).
- Binding the token to what was presented closes the gap between a preview arriving and the solver seeing it (ONE.41).

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| A mirror of the state held in the language | A second state that can drift, against INV-1, for no data a snapshot does not give |
| The engine adopts a state a program computed | Against KN5.13. The state would carry no journal path, so nothing witnesses its reachability |
| Adoption after a check of the state | A check establishes form, not reachability: KN1.17's case passes the sticker test |
| A preview object owned by the language | Against R-10: a caller could change what is committed, and the token would bind something the engine does not hold |

## Consequences

- A tool can hand over only a word. An imported state with no word stays an object of comparison.
- A word computed against an older state can end elsewhere or be refused at the current revision. The preview shows what
  it does now (ONE.26).
- A program cannot commit, restore or undo. It can offer the solver a command (ONE.3, ONE.51).
- A commit never waits for a reader, and it cancels the jobs that read the replaced revision (KN5.18, ONE.15, ONE.45).
  Digest-pinned research jobs finish; their results reach a newer revision only by a re-run or a lemma-justified exact
  transport under KN7.5 and ONE.19 (owner, 6 November 2026, LQ4).
- Derived states cost a program memory budget, and it holds them only by digest across evictions (ONE.29, ONE.30).
- LI.26 now drafts the single `session.undo` entry without `not_busy`; H15.1/H15.2 supply its reconciled row and
  ActionHint binding. RM.26 drafts the presented-frame report required by ONE.41. The renderer handoff carries the
  display-plan primitives, the user-stage contract and the free-view workload of RPI.57 and RPI.58
  (owner, 6 November 2026, LQ20).
  [H-08 CM.1](../../../../progress/1.0/command-table.md) supplies the integrated final table candidate with
  174 commands and `final: true`; that flag records the supplied delta, not approval. Current-candidate review,
  adapter implementation and matching evidence remain owed.

## Evidence

Class: source. No product conformance is established. CT-ONE-01 to CT-ONE-30 are requirements, and none of them
exists yet (overview, section 3.4). The preview-swap test of partition 8.3 is LP2 acceptance, and it has not run. The
counterexample above comes from note N1, which stands at (theorem, producer) until a model review reads it (chapter 06).

## What the owner accepts

Accepting this record accepts ADR-L01.1 to ADR-L01.8 and the [D] clauses they cite.

New integrator rules the owner may overrule (chapter 08, section 17):

- ONE.3: no commit row for the runtime at all, which is wider than KN5.3.
- ONE.4: a token on every commit row, not only on previews.
- ONE.34: previews only inside a command the solver invokes.
- ONE.40: the engine discards a preview when the state revision moves.
- ONE.41: on screen means presented.
- ONE.27: a wordless import can be the base of `derive`, and what is derived keeps its reachability claim.

Three clauses read the sources, and the owner may read them differently: ONE.12 ("shows the stale state"), ONE.21 with
EV3.12 (the base of an imported value) and ONE.61 ("holds no puzzle copy"). ONE.78's reading of charter S4 for a live
twist was confirmed (owner, 6 November 2026, decision 10); this accepts no ADR.
