# ADR-L02: the runtime as a client of the command layer, off the critical path

Status: **Proposed.** Not accepted. The owner accepts or rejects it ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the
owner").

Rests on: partition sections 2 and 9; [chapter 08](../architecture/08-one-state.md) ONE.67 to ONE.69; [chapter
01](../architecture/01-kernel.md) KN0.7, KN5, KN7; [chapter 02](../architecture/02-language-core.md) LG4. Round table:
DT-L1 E1, E4; explainer P-E6.

## Context

The language has to reach everything a tool needs: the state, the journal, the workspace, the view. The charter's E2 bar
counts every millisecond from a key to its frame, and a commit must stay durable whatever a program does. If programs
sit on that path, every key inherits their faults and their latency. If they sit too far from it, they cannot help the
solver.

The runtime therefore needs a defined place: which layers it may use, how, and what it may never touch.

## Decision

- **ADR-L02.1** The runtime is a client. It uses each layer only as ONE.67's table says. It reads through snapshots and
  query rows (KN2.2, KN4.3), and it changes things only through rows of the kinds open to it.
- **ADR-L02.2** The runtime is off the critical path. A turn's interval runs from the key through the command layer, the
  engine's apply, the store's durable transaction and the bundle's adoption to the frame, and no interpreter stands on
  it (ONE.68, ONE.48). A key bound to a kernel row dispatches without an interpreter step or a program job (LG4.13).
- **ADR-L02.3** A revision bundle never waits for program output (RPI.2, ONE.64). Program views skip to the newest
  revision, so a burst of twists queues no view work (LG4.11, RPI.12).
- **ADR-L02.4** A command the language defines is an overlay row of kind `view`, `workspace` or `query`, or `file` with
  a granted capability, never `commit` or `app` (KN5.3). It reaches the engine as data (KN5.5). Its availability comes
  from two stored tables, the static verdict and kernel-row availability, so answering it runs no language code (KN5.7).
- **ADR-L02.5** The kernel never calls the language. No kernel operation takes a callback, and no table the kernel reads
  holds code (KN0.7).
- **ADR-L02.6** A running job never delays a commit, an undo or a key bound to a kernel row (KN7.12). A commit goes
  ahead at once and cancels the jobs that read the replaced revision (KN5.18, ONE.45). A runtime that dies ends its jobs
  as `error`, the journal and state hash stay unchanged and the shell stays responsive (KN7.13, DET.43).
- **ADR-L02.7** Placement stays open. The runtime may run in the engine process, in a process of its own or in the shell
  process, and every clause above must hold in each. The shell process is admitted only if it passes LX-2's
  fault-injection run (ONE.69).

## What this record does not decide

Open whatever the owner answers:

- The availability protocol between the command layer and the engine, which needs its own Astra plan check before the
  architecture freeze (KN5.8), including the form of the generation event (KN5.25).
- The binding between runtime and engine, including zero-copy snapshot access (RTE.7).

Conditional on an owner question:

- **LQ9:** the placement itself, decided with the route on LX-1 to LX-5 under the recommendation (RTE.21, RTE.23). That
  decision is recorded in [ADR-L12](ADR-L12-route.md).
- **LQ21:** what a program may read while it prepares a preview, under the two forms of flow control
  ([ADR-L06](ADR-L06-effects-and-flow.md)).

## Why

- Charter E2 is measured from key to frame. A layer that is not on the path cannot spend from that interval.
- A fault in user code is expected, not exceptional. Keeping the path free of programs means a fault can end a tool
  without touching a key, a commit or an undo (KN7.13).
- R-05 and R-07 require one permission check and no commit outside the normal path. A client of the command layer gets
  those gates without any new rule, because it has no other route.
- A callback from the kernel would put an unbounded program inside a transaction, and the budget rules of
  [ADR-L08](ADR-L08-jobs-and-budgets.md) could not then promise that a commit goes ahead at once.

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| Every key dispatches through the language | Puts interpreter time and interpreter faults on the path E2 measures |
| Callbacks from the engine into programs | Runs user code inside engine work; against KN0.7 and KN7.12 |
| Direct reads of the store by programs | Against KN4.3: it skips the gates and makes the store's layout a public contract |
| A program view that holds a revision until it is ready | A slow view would delay every frame; RPI.2 refuses this |

## Consequences

- A key bound to a language-defined command can at most reach a preview (ONE.48). It pays for the job that computes the
  word, and that time is not inside E2's interval.
- A view can lag. It then shows its designed stale state and names its base revision (ONE.12, RPI.13).
- A display refresh can fail after a commit. The report says "applied, display out of date", and it never invites a
  retry (KN5.22, R-02).
- Two new lanes, program and view, are needed. EN4 must confirm them (KN7.2).
- Tools see only what query rows and snapshots offer. A need the rows do not meet becomes a new row and a review of it,
  not a side door.

## Evidence

Class: source. Nothing has been measured. No program runs today, so this record has no latency figure, and the charter's
E2 test stays the decisive check. Active program views also enter that test (RPI.62). LX-2 tests faults and
responsiveness on the owner's machine (RTE.42), and it has not run.

## What the owner accepts

Accepting this record accepts ADR-L02.1 to ADR-L02.7 and the [D] clauses they cite. It does not choose a placement.

The owner may overrule these integrator additions: the two new lanes of KN7.2, which EN4 also has to confirm, and the
availability generation of KN5.25.
