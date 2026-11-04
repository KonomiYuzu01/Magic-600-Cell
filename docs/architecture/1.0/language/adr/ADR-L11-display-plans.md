# ADR-L11: display plans and user charts

Status: **Proposed.** Not accepted. The owner accepts or rejects it ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the
owner").

Rests on: partition INV-4, INV-6, INV-8, INV-9, section 8.5 and LB4; [chapter
10](../architecture/10-renderer-platform.md). Round table: DT-L1 E5, M4, R2-E2.

## Context

Programs have to show things: objects, relations, charts and overlays. The renderer is not selected yet, and the
renderer line closes its experiments in parallel with this work. So nothing in the language can depend on a renderer
candidate.

Drawing also carries a mathematical risk. Everything on screen is approximate, and it is tempting to read a decision off
it. And one workload is heavy. During a four-dimensional camera rotation or a turn animation every vertex gets new input
every frame, which at full detail is 18,288,000 vertex evaluations per frame (RPI.31). A chart evaluated once per
revision cannot serve that.

## Decision

- **ADR-L11.1** No language feature depends on the renderer candidate. A program publishes view-model content and
  display plans, and it never calls a renderer API or receives a renderer object (RPI.1). Drawing changes no mechanical
  state, identity, legality, protection or evidence (RPI.3).
- **ADR-L11.2** The renderer adopts each engine revision bundle without waiting for program output (RPI.2). An engine
  bundle and a program publication are separate atomic installations (RPI.9, RPI.10).
- **ADR-L11.3** `ViewNode` and `DisplayPlan` are declarative (RPI.5, RPI.6). A derived state is displayed only by a plan
  naming its derivation and base revision (RPI.7).
- **ADR-L11.4** A view does work only when visible and only for changed data (RPI.11). It exposes its base revision and
  status (RPI.13), and it has no workspace-write effect (RPI.14).
- **ADR-L11.5** Display primitives are a stated inventory (RPI.15). A backend reports support or a typed refusal for
  each, and an unsupported primitive never silently becomes a different one (RPI.17). A headless run produces plans as
  test data (RPI.19).
- **ADR-L11.6** Picking resolves a display instance through a source map bound to its plan generation and base revision
  (RPI.20 to RPI.24, ADR-L04.9). A stale reply is discarded or returns its typed reason, and it never retries a
  state-changing command (RPI.25).
- **ADR-L11.7** A `Chart` declares its domain, map, inverse and loss (RPI.28). A float evaluation decides no exact
  predicate (RPI.29). Declared properties are checked exactly at exact camera poses (RPI.30). Where a chart has no
  finite value, the result carries missing coverage and a reason (RPI.37).
- **ADR-L11.8** Per-object sheets, atlases, trees and state graphs are evaluated on the CPU per revision (RPI.38). No
  option moves projection work into a per-revision CPU path that claims to meet the moving-camera workload without
  evidence (RPI.40).
- **ADR-L11.9** Display content keeps the policy and provenance of its producing profile (RPI.45). A chart that fails a
  check cannot supply a checked display stage (RPI.48).
- **ADR-L11.10** The platform rules stand: no Windows-only code in the engine, store, command layer or view model,
  platform code behind narrow interfaces, and shaders written once in portable HLSL (RPI.51 to RPI.53).
- **ADR-L11.11** Charter D8 stays in force unless the owner changes it (RPI.42). The renderer stays unselected, and this
  record leaves its selection gate unchanged (RPI.60).

## What this record does not decide

Open whatever the owner answers:

- Exposed-face geometry and cut-cell data, which the render contract does not supply yet (RPI.18).
- Multi-layer pick disambiguation, a product decision for the design line (RPI.27).
- Which chart properties have exact checkers (RPI.34), and the precision rule (RPI.36).
- Plan limits and the backend interface's signatures (RPI.50, RPI.55).
- How a derived value imported from raw data, with no session ancestor, is displayed (EV3.12).

Conditional on an owner question:

- **LQ7:** the recommended chart subset is pure and total, uses float32 and bounded loops, and compiles to portable HLSL
  with the CPU interpreter as reference (RPI.32, RPI.33). User charts are enabled only when LX-7 passes (RPI.41). The
  alternatives are built-in parameterised charts only, or a shader language for users, which has no safety contract in
  the sources (RPI.39). Every option keeps what RPI.40 lists.
- **LQ6:** where views live. The recommendation keeps solving-profile charts inside Global and Local and lets research
  views open in a research workspace (RPI.43). The tool-authoring profile has no rule yet (RPI.44).
- **LQ20:** whether the handoff to the renderer line happens, and with what (RPI.57 to RPI.59).

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
| Built-in charts only (LQ7) | Simplest and each is measured in H-06, but a researcher cannot add a chart. It stays open as LQ7's alternative |
| A shader language for users (LQ7) | No safety contract exists (RPI.39). It stays open as LQ7's alternative |
| Picking by pixels or colour | Screens are not identities (INV-6) |

## Consequences

- The renderer feature list has to carry these primitives, or plans have to fit the frozen renderer as it is. That is
  what LQ20 decides.
- Programs cannot do custom drawing outside the inventory, and each new primitive is a contract change.
- Exact property checks limit which charts can claim a property until checkers exist (RPI.34).
- Active program views enter the end-to-end E2 test, so a view cannot be judged by a short timer (RPI.62).

## Evidence

Classes: source for the contract and the workload count. The figure of 18,288,000 is counted from `assets/mesh.json`,
30,480 base vertices times 600 cells. It is a workload count, not measured throughput. No Windows, DirectX or
performance result supports this record. LX-7 has not run, and PresentMon figures count only from the owner's machine
(RPI.61). CT-RPI-01 to CT-RPI-26 are requirements only.

## What the owner accepts

Accepting this record accepts ADR-L11.1 to ADR-L11.11 and the [D] clauses they cite. It answers none of LQ6, LQ7 or
LQ20, selects no renderer and changes no gate.
