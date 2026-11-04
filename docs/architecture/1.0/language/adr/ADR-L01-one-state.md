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
  with Jumble off and poses with Jumble on. The engine owns it and the store records it (ONE.1).
- **ADR-L01.2** Only rows of kind `commit` change it, through the command layer and its gates (ONE.2). No such row is
  available to the runtime in any profile or placement (ONE.3). Every one reaches the command layer with a user-action
  token from the shell (ONE.4, KN5.14).
- **ADR-L01.3** Eight things can look like a state, and only the state of record is one (ONE.5). A program may copy what
  it reads. Such a copy has no standing, and it can enter a kernel call only through KN2.14, as a derived state (ONE.6).
- **ADR-L01.4** A derived state is an immutable value computed by the same exact kernels. It has no session revision, it
  lives in the value store by digest, and it is charged to the program's memory budget (KN2.11 to KN2.15, ONE.20 to
  ONE.30). It is never adopted. It becomes real only when the solver plays its word through `propose` and the engine
  replays that word at the current revision (ONE.26). A derived state with no word can be compared and displayed, and
  nothing more (ONE.27).
- **ADR-L01.5** The preview is the single exception: the one state-like value a commit can adopt.
  - The engine builds it and owns its immutable witness (KN5.9, ONE.31).
  - A proposal never carries a state or a digest for the engine to adopt (KN5.13).
  - At most one preview is pending (KN5.11, ONE.38).
  - When the state revision moves the engine discards the preview, and the draft stays (ONE.40).
  - The token binds to the preview of the newest frame reported as presented (ONE.41).
  - At step 9 the engine adopts its own object, so that step needs no trust in the language (ONE.44).
  - A preview is not durable (ONE.43).
- **ADR-L01.6** Every result names the revision or derived state it read, and it is never applied to or shown against
  another (KN0.6, ONE.7, ONE.11).
- **ADR-L01.7** The view model holds no puzzle copy. An adopted bundle is a read-only image of one revision, replaced
  whole, and nothing decides from it (ONE.59 to ONE.61). The renderer draws no uncommitted state as the session state,
  with two named exceptions, each drawn as what it is (ONE.62).
- **ADR-L01.8** A live twist is the one commit with no preview: an immediate complete operation that the solver's key
  names and the active protection gates (ONE.46, ONE.47). This reads charter S4 with solving workflow 4.1 and DT-M1 C4,
  and the owner confirms it (ONE.78).

## What this record does not decide

Open whatever the owner answers:

- Whether an invoked command may replace a preview already on screen (KN5.12, LG3.7, ONE.42). The token keeps the commit
  correct either way, and the choice waits for the design line.
- Where the runtime runs (ONE.69, LQ9). Every clause above must hold in each placement.

Conditional on an owner question:

- **LQ4:** experiment branches and pinned jobs (KN4.6, KN7.5, ONE.19, ONE.73 to ONE.77). Under LQ4's recommendation a
  branch is a journal branch the engine owns, and exactly one line is active.
- **LQ6:** where a derived state may appear inside Global and Local (ONE.65).

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
- Derived states cost a program memory budget, and it holds them only by digest across evictions (ONE.29, ONE.30).
- Two asks go to other lines. EN4 must remove `session.undo`'s `not_busy` gate before the runtime binds it (KN5.20). The
  renderer adapter must report which frame was presented, which is a new item for the renderer line (ONE.41, LQ20).

## Evidence

Class: source. Nothing in this design has been executed. CT-ONE-01 to CT-ONE-27 are requirements, and none of them
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

Four clauses read the sources, and the owner may read them differently: ONE.12 ("shows the stale state"), ONE.21 with
EV3.12 (the base of an imported value), ONE.61 ("holds no puzzle copy") and ONE.78 (charter S4 for a live twist).
