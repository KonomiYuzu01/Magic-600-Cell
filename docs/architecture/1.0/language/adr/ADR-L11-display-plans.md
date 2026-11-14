# ADR-L11: display plans and user stages

Status: **Proposed.** Not accepted. The owner accepts or rejects it ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the
owner").

Rests on: partition INV-4, INV-6, INV-8, INV-9, section 8.5 and LB4; [chapter
10](../architecture/10-renderer-platform.md); the owner's answers of
[6 November 2026](../../../../wiki/decisions/owner-decisions-2026-11-06.md). Round table: DT-L1 E5, M4, R2-E2, R4-M43.

## Context

Programs have to show things: objects, relations, charts and overlays. The renderer is not selected yet, and the
renderer line closes its experiments in parallel with this work. So nothing in the language can depend on a renderer
candidate.

Drawing also carries a mathematical risk. Everything on screen is approximate, and it is tempting to read a decision off
it. And one workload is heavy. During a four-dimensional camera rotation or a turn animation every vertex gets new input
every frame, which at full detail is 18,288,000 vertex evaluations per frame (RPI.31). A chart evaluated once per
revision cannot serve that.

The owner's answers of 6 November 2026 widen the work: users write shader stages under a safety contract, and free
views are allowed everywhere.

## Decision

- **ADR-L11.1** No language feature depends on the renderer candidate. A program publishes view-model content and
  display plans, and it never calls a renderer API or receives a renderer object (RPI.1). Drawing changes no mechanical
  state, identity, legality, protection or evidence (RPI.3).
- **ADR-L11.2** The renderer adopts each engine revision bundle without waiting for program output (RPI.2). An engine
  bundle and a program publication are separate atomic installations (RPI.9, RPI.10). RM.20 to RM.23 now draft
  publication, replacement and device loss. Installation returns `completed(view_published)` and does not prove
  presentation. A failure after installation changes display availability, never that completed receipt.
  H15.8 keeps a scheduled publication's final validation failure on its original invocation.
- **ADR-L11.3** `ViewNode` and `DisplayPlan` are declarative (RPI.5, RPI.6). A derived state is displayed only by a plan
  naming its derivation and base revision (RPI.7). A value imported with no session ancestor states lineage `imported`,
  and its plan base is a comparison base that claims nothing (EV3.12, ONE.21). Content shown in a view is state data, a
  claim with its key, or program content, and a word of EVD.32 reaches a display only through `word()` (RPI.65).
- **ADR-L11.4** A view does work only when visible and only for changed data (RPI.11). It exposes its base revision and
  status (RPI.13), and it has no workspace-write effect (RPI.14).
- **ADR-L11.5** Display primitives are a stated inventory (RPI.15). A backend reports support or a typed refusal for
  each, and an unsupported primitive never silently becomes a different one (RPI.17). A headless run produces plans as
  test data (RPI.19). The renderer feature matrix now supplies draft payload/support rows and RM.14 to RM.26's
  portable adapter operations, immutable leases, cancellation and safe resource retirement. Those contracts grant
  no backend support claim without the required implementation and evidence.
- **ADR-L11.6** Picking resolves a display instance through a source map bound to its plan generation and base revision
  (RPI.20 to RPI.24, ADR-L04.9). A stale reply is discarded or returns its typed reason, and it never retries a
  state-changing command (RPI.25). RM.10 consumes EV1.10's source ranges. H15.10 checks pick bindings with
  `EV3:stale.display_pick` before checking a later source-digest mismatch with `EV3:stale.source_link`.
- **ADR-L11.7** A `Chart` declares its domain, map, inverse and loss (RPI.28). A float evaluation decides no exact
  predicate (RPI.29). Declared properties are checked exactly at exact camera poses (RPI.30). Where a chart has no
  finite value, the result carries missing coverage and a reason (RPI.37).
- **ADR-L11.8** Per-object sheets, atlases, trees and state graphs are evaluated on the CPU per revision (RPI.38). No
  projection work moves into a per-revision CPU path that claims to meet the moving-camera workload without evidence
  (RPI.40).
- **ADR-L11.9** Display content keeps the policy and provenance of its producing profile (RPI.45). A chart that fails a
  check cannot supply a checked display stage (RPI.48).
- **ADR-L11.10** The platform rules stand: no Windows-only code in the engine, store, command layer or view model,
  platform code behind narrow interfaces, and shaders written once in portable HLSL (RPI.51 to RPI.53).
- **ADR-L11.11** Global and Local stay visible together and linked. Further views are allowed in every profile under a
  stated resource budget (RPI.42, RPI.81 to RPI.83; owner, 6 November 2026, LQ6). The renderer stays unselected, and
  this record leaves its selection gate unchanged (RPI.60).
- **ADR-L11.12** Users write shader stages (owner, 6 November 2026, LQ7). A stage is a checked module that the
  project's own pipeline compiles to portable HLSL, and no raw shader upload exists. Every frame with a user stage runs
  a raster bound before any fragment work, a stage output is display only, and a device fault quarantines the chain's user stages
  and falls back to the built-in chain (RPI.68 to RPI.80).
- **ADR-L11.13** The renderer line receives the display-plan primitives, the user-stage contract and the free-view
  workload as handoff row H-11 before its feature freeze (RPI.57, RPI.58; owner, 6 November 2026, LQ20).

## What this record does not decide

The owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the
interpreter's host in Rust, route RT1 with the runtime in a process of its own, and capability separation (RTE.17,
RTE.21, RTE.24). LQ13 waits for its evidence, and LQ2's option set waits for the joint setting.

Open mechanisms:

- Certified exposed-face geometry and its size measurements remain owed (RPI.18). JI.41 now drafts the named
  producers and payloads, including cut-cell/off-grip data; missing certified geometry or a size profile is explicitly
  unavailable. The renderer matrix must reconcile those bindings and obtain actual producer/backend evidence.
- Multi-layer pick disambiguation, a product decision for the design line (RPI.27).
- Which chart properties have exact checkers (RPI.34), and the precision rule (RPI.36).
- Numeric production plan ceilings, tariffs, resource measurements and backend implementations remain owed
  (RPI.50, RPI.55). RM.11 to RM.13 now draft fixed ceiling fields/counts plus the existing output allowance, and
  RM.14 to RM.26 draft the portable signatures and lifetimes. The separate plan-budget code stays reserved (ERR.25).
  Final ABI/transport, support and presentation evidence still require their own integration and review.
- Stage numerics and ceilings: the operation tiers' error rule, the weights and the stage and view values all come
  from measurement (RPI.36, RPI.70, RPI.79, RM.38, RM.39).
- Which display sinks flow control checks stays in LQ2's option set, and its form is capability separation (owner,
  14 November 2026, LQ21; RTE.24; RPI.45).
- A stage's source syntax follows LQ13 (LG5.12).
- Whether a run-time stage compiler ships is an owner item (RPI.80).

## Why

- The language must survive any renderer outcome, so the contract is data with a stated meaning.
- Keeping the bundle apart from program output means a slow or failed view can never hold back a turn (RPI.2).
- Declaring domain and loss makes a chart's limits a checkable statement. The exact checks then run at exact poses, so
  nothing depends on what a float evaluation happened to give.
- A support report or typed refusal is better than silent substitution, which would show one thing and mean another.

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| A renderer API for programs | Ties the language to a candidate, against INV-8, and a tool could stall or crash a device |
| Charts evaluated per revision on the CPU, for projections too | Cannot serve the moving-camera workload (RPI.31) |
| Built-in charts only (LQ7) | Rejected alternative of LQ7 (owner, 6 November 2026). Built-in parameterised charts stay, but a researcher could not add a chart |
| A compiled chart subset only (LQ7) | Rejected alternative of LQ7 (owner, 6 November 2026) |
| Raw shader upload | No path: a program's shader text cannot be checked or bounded (RPI.68) |
| User charts only inside Global and Local, research views in a research workspace (LQ6) | Rejected alternative of LQ6 (owner, 6 November 2026) |
| Plans that fit a renderer already frozen (LQ20) | Rejected alternative of LQ20 (owner, 6 November 2026) |
| Picking by pixels or colour | Screens are not identities (INV-6) |

## Consequences

- The renderer feature list has to carry these primitives, the stage contract and the free-view workload (H-11). A
  candidate with no run-time compile path or no way to skip a stopped frame fails that row (RPI.80).
- Programs cannot do custom drawing outside the inventory and the stage contract, and each new primitive is a contract
  change.
- The raster bound runs in every frame with a user stage, and its cost counts toward the unchanged gate (RPI.72,
  RPI.79).
- Shipping a stage compiler is an owner item, because DXC is not on the toolchain allowlist (RPI.80).
- Exact property checks limit which charts can claim a property until checkers exist (RPI.34).
- Active program views enter the end-to-end E2 test, so a view cannot be judged by a short timer (RPI.62).

## Evidence

Classes: source for the contract and the workload count. The figure of 18,288,000 is counted from `assets/mesh.json`,
30,480 base vertices times 600 cells. It is a workload count, not measured throughput. LX-7's frozen `lx7-r1` result
covers a compute subset on the owner's RTX 4070 Laptop GPU: 4 accepted and 18 refused sources and 496 readback records.
It does not establish drawing-pipeline or fragment stages, the raster bound, W3 performance, arbitrary-chart
correctness or renderer selection. PresentMon figures count only from the owner's machine (RPI.61). CT-RPI-01 to
CT-RPI-38 and CT-RM-01 to CT-RM-10 are requirements only; the feature matrix's source vectors do not establish native
support, recovery, input or performance for this candidate.

## What the owner accepts

Accepting this record accepts ADR-L11.1 to ADR-L11.13 and the [D] clauses they cite. It answers no owner question,
selects no renderer and changes no gate. LQ6, LQ7 and LQ20 were answered on 6 November 2026; those answers accepted no
ADR and froze nothing.
