# ADR-L08: jobs and deterministic budgets

Status: **Proposed.** Not accepted. The owner accepts or rejects it ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the
owner").

Rests on: partition INV-10, section 8.4 and default 6; [chapter 01](../architecture/01-kernel.md) KN7; [chapter
09](../architecture/09-determinism.md). Round table: DT-L1 E3, M1; explainer P-E7.

## Context

Searches and checks can run for a long time, and the solver keeps playing while they do. A result also has to mean
something afterwards. "Nothing found" after ten minutes on a busy laptop says nothing that anyone can reproduce or build
on. If a budget is counted in seconds, the same run on the same input can end at a different point on another day, and
any negative it reports has no reproducible scope.

So long work needs its own rules for admission and cancellation, and a budget unit that does not depend on the machine.

## Decision

- **ADR-L08.1** Long work runs as a job, never on the UI thread. Admission is atomic per lane and keyed by the durable
  command ID (KN7.1). A lane is a named admission rule (KN7.2). LI.22 to LI.25 now draft its capacity and
  replacement rules; H15.5 supplies the reconciled target binding for view/latest-wins work.
- **ADR-L08.2** A job ends as completed, cancelled, cut off or error. Each result is bound to its input digest,
  parameters and coverage (KN7.3). A job that did not complete yields no negative claim, and a witness it found counts
  once it is checked (KN7.4). Cancellation is cooperative and phase-aware (KN7.6). LI.28 is the sole mapping from
  job outcome to invocation receipt: deterministic budget cut-off gives `cancelled(budget)`, while an error gives
  `failed(Error)`. A partial query is completed only when its result type declares that coverage a normal outcome
  within the grant (H15.7); exhausting the budget cannot become completion.
  A research job with digest-pinned input finishes after a commit; its result reaches a newer revision only by a re-run
  or a lemma-justified exact transport under KN7.5 and ONE.19 (owner, 6 November 2026, LQ4).
- **ADR-L08.3** Every job has an explicit deterministic budget of steps, kernel cost, memory and output. Seconds are a
  display unit, never a budget unit (KN7.7, DET.18). A wall-clock safety stop may exist as a machine limit. It is never
  in the reproducibility key, and it has its own code, because it is not a budget (KN7.10).
- **ADR-L08.4** Every kernel call declares its cost before admission, as a function of its inputs only. A cache hit is
  charged as a miss (KN7.8, DET.23). Memory is counted in logical units (KN7.9). DET.54 to DET.60 now draft the
  operation schedule, admission order, ownership lifetimes, canonical output accounting and serial child meters.
  Native rows still need their declared tariffs and bounds; the reference vectors use test tariffs only.
- **ADR-L08.5** "Empty", "not found within budget" and "not run" are distinct results (KN7.11, DET.30).
- **ADR-L08.6** `ReproKey` names the complete fixed input of a run, including the cost table revision and the kernel and
  checker identities (DET.1 to DET.4). For a run that completes or reaches a deterministic cut-off, the same key gives
  the same exact result bytes on Windows and on Linux (DET.5). DET.6's recorded counters, outcome, code and
  coverage stay in that exact payload. H6 and DET.64 retain predicate/size agreement for display accounting; no
  later exact continuation may read those observations or reclaim the display region's remainder.
- **ADR-L08.7** One run has one execution thread, traversal uses canonical keys, no ambient input is read, seeds are
  sealed and cache state is unobservable (DET.8 to DET.11, DET.16).
- **ADR-L08.8** A deterministic cut-off is reproducible. Solver cancellation and a wall-clock safety stop are not, and a
  record keeps which one happened (DET.17, DET.29).
- **ADR-L08.9** A running job never delays a commit, an undo or a key bound to a kernel row (KN7.12).

## What this record does not decide

The owner answered LQ1 to LQ21 on 6 November 2026. LQ8, LQ9, LQ13 and LQ21 wait for their evidence, and LQ2's option set
waits for the joint setting.

Open mechanisms, as LP1 review and delivery obligations:

- Review and conformance of the proposed one-step semantic operations and admission schedule
  (`lp1-exec-draft-1`, LG4.8, DET.54 to DET.56). The weighted-table alternative remains recorded with a different
  execution identity and compatibility treatment; neither this ADR nor the test tariffs select a production schedule.
- Review and implementation of the drafted logical ownership/lifetime rules (DET.57/DET.58) and canonical
  uncompressed output bytes with a separate terminal reserve (H8, DET.59). Physical memory and GPU retirement
  remain distinct, and production reserve capacity is still owed.
- Review and implementation of DET.60's serial child accounting, including ancestor debits and child versus root
  exhaustion. No executor evidence establishes those rules yet.
- Budget presets, numeric profile ceilings, production row tariffs and a recursion guard (DET.28). The draft
  resource rules do not supply measured product defaults.
- Review and implementation of `sha256-counter-draft-1` and sealed stream paths (DET.61), plus scalar Text equality
  and ordering without normalization (DET.62). These are now explicit draft semantics, not accepted contracts.

Waiting owner questions:

- **LQ9:** whether RT2 can map its executor onto the same abstract charges (DET.21, DET.22).

## Why

- A negative is a statement about a declared set. Only a deterministic budget lets someone else re-run the same search
  to the same stopping point and see what it covered.
- The language needs the distinction anyway. A wall-clock stop reads as "incomplete" and a budget stop as "not found
  within budget" (EVD.32, EVD.35). Mixing them would let a busy machine create a claim.
- Charging a cache hit as a miss keeps a run's stopping point independent of what happened to be cached (DET.16).
- Declaring cost before admission lets a run refuse work it cannot pay for, instead of stopping partway through a kernel
  call.

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| Wall-clock budgets | A negative would have no reproducible scope, and the same key could end differently on another machine |
| A VM's fuel as the unit | It ties budgets to one executor, and a route change would move stopping points. A route may implement the abstract charges with fuel if it preserves them (DET.21) |
| No budget, only cancellation | "I stopped it" says nothing about coverage |
| Parallel evaluation inside a run | Observable order would depend on scheduling. Parallelism stays between jobs (LG4.3, DET.8) |

## Consequences

- Every kernel row has to publish a cost function, and a change to it is a new cost table revision.
- Budget figures have no natural feel. Seconds stay available for display, so a solver sees an estimate and the record
  keeps the units.
- One thread per run gives up speed inside a run. First-pass experiments do not establish that cost for a product
  search (RTE.82).
- Cold caches and warm caches cost a run the same, so a warm cache saves real time without changing a charge.
- Reproducing a run needs its whole dependency closure, so archives have to keep those inputs by digest.

## Evidence

Classes: source and synthetic. LX-3's first-pass records are under `work/experiments/language-lx/lx3-sandbox-flow/`.
They establish no product execution contract, and their evidence classes and limits stay separate (RTE.82). Repeating
keys on Windows and headless Linux remains required (DET.49, RTE.46). CT-DET-01 to CT-DET-32 are requirements only.
`contract-vectors/` supplies abstract reference fixtures for the new accounting draft, not product execution evidence.
Existing research runs do not all
meet these rules: stage 13's README retains its clock use, and DET.48 says these are requirements on new runs.

## What the owner accepts

Accepting this record accepts ADR-L08.1 to ADR-L08.9 and the [D] clauses they cite.

The owner may overrule these integrator additions: the cost table revision and the kernel and checker identities in the
key (DET.2, DET.3), the rule that the key fixes every input a program can consult (DET.4), and the program and view
lanes now drafted in KN7.2. Their draft descriptions do not replace final EN4 integration or owner acceptance.
