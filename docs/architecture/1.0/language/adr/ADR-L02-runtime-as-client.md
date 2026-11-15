# ADR-L02: the runtime as a client of the command layer, off the critical path

Status: **Accepted** (owner, 15 November 2026;
[decision](../../../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)).

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
  from two stored tables, the static verdict and kernel-row availability, so answering it runs no language code
  (KN5.7, LI.18 to LI.21). H15.6 distinguishes static argument bindings from computed arguments' row-level hints;
  invocation still checks actual arguments. The availability generation is advisory, never a grant or a lane reservation.
- **ADR-L02.5** The kernel never calls the language. No kernel operation takes a callback, and no table the kernel reads
  holds code (KN0.7).
- **ADR-L02.6** A running job never delays a commit, an undo or a key bound to a kernel row (KN7.12). A commit goes
  ahead at once and cancels the jobs that read the replaced revision (KN5.18, ONE.45), except digest-pinned research
  jobs, which finish under KN7.5 (owner, 6 November 2026, LQ4). A runtime that dies ends its jobs
  as `error`, the journal and state hash stay unchanged and the shell stays responsive (KN7.13, DET.43). LI.28 owns
  the job-to-receipt mapping; LI.29/LI.30 own payload loss and outcome lookup. No refresh failure rewrites a terminal
  success, and missing terminal proof is not inferred from a frame.
- **ADR-L02.7** The runtime runs in a process of its own (owner, 14 November 2026, LQ9; RTE.21). The engine-process and
  shell-process placements are not admitted, and every clause above holds in that placement. LX-2's fault-injection
  run is an LP3 acceptance check with its bar unchanged; if it fails, LQ9 reopens (ONE.69).

## What this record does not decide

The owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the
interpreter's host in Rust, route RT1 with the runtime in a process of its own, and capability separation (RTE.17,
RTE.21, RTE.24). LQ13 waits for its evidence, and LQ2's option set waits for the joint setting.

Open mechanisms:

- Review and implementation of the availability protocol now drafted in KN5.8/KN5.25 and LI.18 to LI.21,
  including `availability_changed` and full-table refresh after a gap or epoch change. The required Astra plan check
  and final contract review remain freeze inputs; a written protocol is not a frozen transport.
- The binding between runtime and engine, including zero-copy snapshot access (RTE.7).

Answered owner questions:

- **LQ9:** the placement itself, a process of its own, with route RT1 (owner, 14 November 2026, LQ9; RTE.21). The
  decision is recorded in [ADR-L12](ADR-L12-route.md).
- **LQ21:** what a program may read while it prepares a preview, under capability separation (owner, 14 November 2026,
  LQ21; RTE.24; [ADR-L06](ADR-L06-effects-and-flow.md)).

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
- KN7.2 and LI.22 to LI.25 now draft six lane admissions, including program and view. H15.5 supplies the reconciled
  target binding for view/latest-wins work. [H-08 CM.1](../../../../progress/1.0/command-table.md) supplies the
  integrated final table candidate with 174 commands and `final: true`, including its lane targets. That flag
  records the supplied delta, not approval. Current-candidate review, runtime/engine binding implementation and
  matching evidence remain owed.
- Tools see only what query rows and snapshots offer. A need the rows do not meet becomes a new row and a review of it,
  not a side door.

## Evidence

Classes: source and synthetic. LX-2's headless slice ran; its record is
`work/experiments/language-lx/lx2-placement/RESULT.md`. It establishes no complete LX-2 pass, product latency or
placement decision (RTE.82). The charter's E2 test stays the decisive check. Active program views also enter that test
(RPI.62). The decisive LX-2 responsiveness run is an LP3 acceptance check with its bar unchanged (owner,
14 November 2026, LQ9; RTE.21).

## What the owner accepts

Accepting this record accepts ADR-L02.1 to ADR-L02.7 and the [D] clauses they cite. It does not itself choose a
placement; ADR-L02.7 restates the owner's answer (LQ9; RTE.21).

The owner may overrule these integrator additions: the program and view lanes of KN7.2 and the availability generation
of KN5.25. EN4's definitions are now drafted; neither this description nor that draft accepts them.
